//! Threading wrapper around `xng_mode_hfdl::HfdlChannelDecoder`, mirroring
//! `sdroxide_qo100::Qo100Controller`'s shape: the realtime engine thread ships
//! I/Q blocks to a worker over a bounded channel and drains status updates
//! non-blocking via [`HfdlController::poll`]. All the DSP runs on the worker.
//!
//! Simpler than the QO-100 wrapper in one way — the lane rate is fixed at
//! [`sdroxide_types::HFDL_LANE_RATE_HZ`] and the decoder carries no rolling
//! search window, so a dropped I/Q block costs a few milliseconds of a burst
//! rather than a whole frame, and there is nothing to do but restart. So the
//! bounded queue is the simple version (`try_send`, drop the whole block),
//! not the sequence-numbered restart the QO-100 worker needs.
//!
//! The decoder itself owns the HFDL state that must not be dropped — the LMS
//! equalizer's tap history, the decision-directed carrier loop, the
//! interleaver/Viterbi state — and that is exactly why it lives here rather
//! than being rebuilt per block.

use std::collections::VecDeque;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, bounded, select, unbounded};
use sdroxide_dsp::Complex32;
use sdroxide_types::{
    HFDL_ACARS_DEPTH, HFDL_LOG_DEPTH, HfdlAcars, HfdlDecode, HfdlFix, HfdlSettings, HfdlStation,
    HfdlStatus,
};
use xng_mode_hfdl::HfdlChannelDecoder;
use xng_mode_hfdl::pdu::{HfdlEvent, gs_name};

use crate::stations::StationBook;

/// Realtime data, dropped on backpressure.
struct Iq {
    samples: Vec<Complex32>,
}

/// Control traffic, never dropped.
enum Ctl {
    Config(HfdlSettings),
    /// A saved system table to fill the station book's channels from.
    Seed(Vec<HfdlStation>),
    Stop,
}

/// Bounded I/Q queue depth. Roughly a couple of seconds of channel-rate air
/// at a typical device read; an ordinary scheduling hiccup must not drop air,
/// and HFDL bursts are short enough that sustained backpressure past this can
/// only cost fragments, not whole frames.
const IQ_QUEUE_DEPTH: usize = 256;

/// How often a status snapshot goes out when nothing has decoded: once every
/// N blocks (about once a second at a normal device read), so the panel's
/// level meter keeps moving without flooding the event channel.
const IDLE_TICK: u64 = 250;

pub struct HfdlController {
    iq_tx: Sender<Iq>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<HfdlStatus>,
    worker: Option<JoinHandle<()>>,
}

impl HfdlController {
    /// `rate_hz` is the rate the engine is feeding (the DDC's actual output,
    /// [`sdroxide_types::HFDL_LANE_RATE_HZ`] or the nearest the DDC can make).
    /// The lane is centred on the channel, so the decoder is built with a
    /// zero offset — the +1440 Hz subcarrier is the decoder's own business.
    pub fn new(rate_hz: f64, cfg: HfdlSettings) -> Self {
        let (iq_tx, iq_rx) = bounded::<Iq>(IQ_QUEUE_DEPTH);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<HfdlStatus>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-hfdl".into())
            .spawn({
                move || {
                    let mut cfg = cfg;
                    let mut decoder = match HfdlChannelDecoder::new(rate_hz, 0.0) {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::error!(%e, "HFDL decoder could not start");
                            return;
                        }
                    };
                    let mut log: VecDeque<HfdlDecode> = VecDeque::with_capacity(HFDL_LOG_DEPTH);
                    // The ACARS messages among them, kept longer than the log
                    // keeps anything: squitters fill the log's 64 entries in
                    // half an hour and a message should outlast that.
                    let mut acars: VecDeque<HfdlDecode> = VecDeque::new();
                    let (mut bursts, mut decodes) = (0u64, 0u64);
                    let mut tick = 0u64;
                    let mut book = StationBook::new();
                    // Set by anything that changes the status without a decode
                    // (a seeded table), so the next block carries it out rather
                    // than the panel waiting for the idle tick.
                    let mut force = false;
                    loop {
                        select! {
                            recv(ctl_rx) -> msg => match msg {
                                Ok(Ctl::Config(next)) => cfg = next,
                                Ok(Ctl::Seed(saved)) => {
                                    book.seed(&saved);
                                    force = true;
                                }
                                Ok(Ctl::Stop) | Err(_) => break,
                            },
                            recv(iq_rx) -> msg => match msg {
                                Ok(Iq { samples }) => {
                                    tick += 1;
                                    let mut emitted = false;
                                    if cfg.enabled {
                                        let events: Vec<HfdlEvent> = decoder.process(&samples);
                                        bursts += u64::from(!events.is_empty());
                                        decodes += events.len() as u64;
                                        if !events.is_empty() {
                                            let now = now_unix();
                                            let khz = (cfg.frequency_hz / 1000.0).round() as u32;
                                            for e in events {
                                                book.observe(&e, khz, now);
                                                record(&mut log, &mut acars, mk_decode(&cfg, &e, now));
                                            }
                                            emitted = true;
                                        }
                                    }
                                    // A status always comes back once (tick 1)
                                    // so a fresh client sees `running` and the
                                    // level immediately, then periodically even
                                    // through silence, and immediately whenever
                                    // anything decoded.
                                    if emitted || force || tick % IDLE_TICK == 0 || tick == 1 {
                                        force = false;
                                        let (stations, table_version) = book.snapshot();
                                        let _ = res_tx.send(HfdlStatus {
                                            running: cfg.enabled,
                                            level_dbfs: decoder.level_dbfs(),
                                            bursts,
                                            decodes,
                                            log: log.iter().cloned().collect(),
                                            acars: acars.iter().cloned().collect(),
                                            stations,
                                            table_version,
                                        });
                                    }
                                }
                                Err(_) => break,
                            },
                        }
                    }
                }
            })
            .expect("spawn hfdl worker");
        HfdlController { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of lane-rate I/Q to the worker. Non-blocking.
    pub fn on_rx_iq(&self, iq: &[Complex32]) {
        let _ = self.iq_tx.try_send(Iq { samples: iq.to_vec() });
    }

    /// Apply new settings (currently just the channel and the on/off switch)
    /// to the running worker.
    pub fn set_config(&self, cfg: HfdlSettings) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    /// Hand the worker a saved system table (the stations' positions and
    /// channels), so a station's squitter bitmap can be read as frequencies
    /// before the network has broadcast the table again.
    pub fn seed_stations(&self, saved: Vec<HfdlStation>) {
        let _ = self.ctl_tx.send(Ctl::Seed(saved));
    }

    /// Drain the latest status, non-blocking. Only the newest matters.
    pub fn poll(&self) -> Option<HfdlStatus> {
        let mut out = None;
        while let Ok(s) = self.res_rx.try_recv() {
            out = Some(s);
        }
        out
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The ground station an event names, as `name (GS n)` — squitters and
/// frequency data always carry a `gs_id`; other kinds (aircraft data) do not.
fn gs_of(e: &HfdlEvent) -> Option<String> {
    let id = e.details.get("gs_id")?.as_u64()? as u8;
    let name = gs_name(id).unwrap_or("unknown");
    Some(format!("{name} (GS {id})"))
}

/// Map one decoded event onto the wire type's log entry.
fn mk_decode(cfg: &HfdlSettings, e: &HfdlEvent, unix: i64) -> HfdlDecode {
    HfdlDecode {
        unix,
        kind: e.kind.clone(),
        gs: gs_of(e),
        freq_khz: (cfg.frequency_hz / 1000.0).round() as u32,
        snr_db: e.snr_db,
        freq_skew_hz: e.freq_skew_hz,
        fec_corrected: e.fec_corrected,
        details: serde_json::to_string(&e.details).unwrap_or_default(),
        position: fix_of(e),
        acars: acars_of(e),
    }
}

/// File one decode: into the rolling log, and — if it carried an ACARS message
/// — into the longer ACARS list as well. Both are newest first and bounded.
fn record(log: &mut VecDeque<HfdlDecode>, acars: &mut VecDeque<HfdlDecode>, d: HfdlDecode) {
    if d.acars.is_some() {
        if acars.len() == HFDL_ACARS_DEPTH {
            acars.pop_back();
        }
        acars.push_front(d.clone());
    }
    if log.len() == HFDL_LOG_DEPTH {
        log.pop_back();
    }
    log.push_front(d);
}

/// The ACARS message an event carried, with what xng's application layer made
/// of it. `None` for every event that carried no ACARS block.
fn acars_of(e: &HfdlEvent) -> Option<HfdlAcars> {
    let b = e.acars.as_ref()?;
    let c = &b.core;
    let who = e.details.get("who");
    // The same application-layer pass the block parser ran, to get the typed
    // result back — `core.app` holds it only as JSON, and the one-line summary
    // is a function of the typed form.
    let dec = xng_acars::decode(&c.label, &c.text, b.downlink);
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = dec.app.as_ref().and_then(xng_acars::summary) {
        // Q-series summaries open with their own label, which the row prints.
        let s = s.strip_prefix(c.label.as_str()).map_or(s.as_str(), str::trim_start).to_string();
        parts.push(s);
    }
    if let Some(o) = &dec.oooi {
        let mut f = Vec::new();
        if let Some(a) = &o.depa {
            f.push(format!("from {a}"));
        }
        if let Some(a) = &o.dsta {
            f.push(format!("to {a}"));
        }
        for (name, t) in [
            ("OUT", &o.gtout),
            ("OFF", &o.wloff),
            ("ON", &o.wlin),
            ("IN", &o.gtin),
            ("ETA", &o.eta),
        ] {
            if let Some(t) = t {
                f.push(format!("{name} {t}"));
            }
        }
        if !f.is_empty() {
            parts.push(f.join(" "));
        }
    }
    if let Some(p) = &dec.position {
        parts.push(format!("POS {:.3}, {:.3}", p.latitude, p.longitude));
    }
    Some(HfdlAcars {
        downlink: b.downlink,
        registration: c.tail.clone().unwrap_or_default(),
        icao: who.and_then(|w| w.get("icao")).and_then(|v| v.as_str()).map(str::to_owned),
        flight: c.flight.clone().unwrap_or_default().trim().to_string(),
        label: c.label.clone(),
        sublabel: c.sublabel.clone(),
        mode: c.mode,
        block_id: c.block_id,
        msg_num: c.msg_num.clone().unwrap_or_default(),
        text: c.text.clone(),
        more: c.more_to_come,
        crc_ok: b.crc_ok,
        parity_errors: b.parity_errors,
        decoded: (!parts.is_empty()).then(|| parts.join(" · ")),
    })
}

/// The aircraft fix an event carried, if it carried one — xng's normalized
/// `details.position` object (see `HfdlFix`). Absent for every kind that is
/// not a performance-data or frequency-data record, and for a
/// position-bearing record whose fix was the all-zero placeholder (xng drops
/// that one before it reaches here).
fn fix_of(e: &HfdlEvent) -> Option<HfdlFix> {
    let p = e.details.get("position")?;
    let lat = p.get("lat")?.as_f64()?;
    let lon = p.get("lon")?.as_f64()?;
    Some(HfdlFix {
        lat,
        lon,
        aircraft_id: p.get("aircraft_id").and_then(|v| v.as_u64()).map(|v| v as u32),
        icao: p.get("icao").and_then(|v| v.as_str()).map(str::to_owned),
        flight: p
            .get("flight")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
    })
}

impl Drop for HfdlController {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::HfdlSettings;

    fn now_unix() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn wait_for(c: &HfdlController, pred: impl Fn(&HfdlStatus) -> bool) -> Option<HfdlStatus> {
        let mut latest = None;
        for _ in 0..500 {
            if let Some(s) = c.poll() {
                let hit = pred(&s);
                latest = Some(s);
                if hit {
                    return latest;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        latest
    }

    fn noise(len: usize) -> Vec<Complex32> {
        (0..len)
            .map(|i| {
                let (a, b) = ((i as f32 * 0.7).sin(), (i as f32 * 1.9 + 1.0).sin());
                Complex32::new(a, b)
            })
            .collect()
    }

    fn event(kind: &str, details: serde_json::Value) -> HfdlEvent {
        HfdlEvent {
            kind: kind.to_string(),
            details,
            acars: None,
            fec_corrected: None,
            freq_skew_hz: None,
            snr_db: None,
            raw: Vec::new(),
        }
    }

    /// A position-bearing record: xng's normalized `details.position` lifts out
    /// into the typed fix the panel and the map read.
    #[test]
    fn a_position_record_lifts_out_a_typed_fix() {
        let e = event(
            "performance-data",
            serde_json::json!({
                "position": {
                    "lat": 40.88, "lon": -72.64, "aircraft_id": 0x42,
                    "icao": "040087", "flight": "BAW123"
                }
            }),
        );
        let f = fix_of(&e).expect("a fix");
        assert!((f.lat - 40.88).abs() < 1e-9);
        assert!((f.lon + 72.64).abs() < 1e-9);
        assert_eq!(f.aircraft_id, Some(0x42));
        assert_eq!(f.icao.as_deref(), Some("040087"));
        assert_eq!(f.flight.as_deref(), Some("BAW123"));
        assert_eq!(f.label(), "BAW123");
        assert_eq!(f.key(), "icao:040087");
    }

    /// Everything that is not a position record — and a record whose fix xng
    /// dropped as the all-zero placeholder — carries no fix.
    #[test]
    fn a_record_without_a_position_lifts_nothing() {
        assert!(fix_of(&event("squitter", serde_json::json!({ "gs_id": 4 }))).is_none());
        assert!(
            fix_of(&event(
                "frequency-data",
                serde_json::json!({ "gs_id": 4, "position": serde_json::Value::Null }),
            ))
            .is_none()
        );
    }

    /// The key prefers the ICAO, then the alias, then the flight, then the
    /// position — so an aircraft that resolves its ICAO keeps one plot.
    #[test]
    fn the_fix_key_uses_the_most_stable_identity() {
        let base = HfdlFix {
            lat: 1.0,
            lon: 2.0,
            aircraft_id: Some(7),
            icao: None,
            flight: Some("KLM1".into()),
        };
        assert_eq!(base.key(), "ac:7");
        assert_eq!(base.label(), "KLM1");
        let resolved = HfdlFix { icao: Some("484123".into()), ..base.clone() };
        assert_eq!(resolved.key(), "icao:484123");
        assert_eq!(resolved.label(), "KLM1");
        let bare = HfdlFix { aircraft_id: None, flight: None, ..base };
        assert_eq!(bare.key(), "pos:1.00,2.00");
        assert_eq!(bare.label(), "#?");
    }

    #[test]
    fn the_worker_reports_running_and_survives_pure_noise() {
        let c = HfdlController::new(
            24_000.0,
            HfdlSettings { enabled: true, frequency_hz: 21_931_000.0 },
        );
        c.on_rx_iq(&noise(24_000 * 2));
        let s = wait_for(&c, |s| s.running).expect("an idle snapshot should arrive");
        assert!(s.running);
        assert_eq!(s.decodes, 0, "pure noise must never decode");
    }

    fn acars_event(label: &str, block_id: char, flight: Option<&str>, text: &str) -> HfdlEvent {
        let octets = xng_acars::block::build(
            '2',
            "N123AB",
            None,
            label,
            block_id,
            flight.map(|_| "M01A"),
            flight,
            text,
            false,
        );
        let block = xng_acars::block::parse(&octets).expect("the builder makes a parseable block");
        HfdlEvent {
            kind: "acars".into(),
            details: serde_json::json!({
                "who": { "dir": "downlink", "gs_id": 4, "aircraft_id": 12, "icao": "A1B2C3" },
                "bps": 600,
            }),
            acars: Some(block),
            fec_corrected: None,
            freq_skew_hz: None,
            snr_db: Some(11.0),
            raw: Vec::new(),
        }
    }

    /// The ACARS text and addressing come through — the log used to show only
    /// the link-layer envelope.
    #[test]
    fn an_acars_event_carries_its_message() {
        let e = acars_event("H1", '4', Some("UAL012"), "HELLO FROM THE OCEAN");
        let a = acars_of(&e).expect("an ACARS event yields a message");
        assert!(a.downlink);
        assert_eq!(a.registration, "N123AB");
        assert_eq!(a.flight, "UAL012");
        assert_eq!(a.icao.as_deref(), Some("A1B2C3"));
        assert_eq!(a.label, "H1");
        assert_eq!(a.msg_num, "M01A");
        assert!(a.text.contains("HELLO FROM THE OCEAN"), "{:?}", a.text);
        assert!(a.crc_ok);
        assert_eq!(a.parity_errors, 0);
        assert_eq!(a.who(), "N123AB");
        // The decode the log shows carries it, and an event with no ACARS does not.
        let d = mk_decode(&HfdlSettings::default(), &e, 1_000);
        assert!(d.acars.is_some());
        assert!(acars_of(&event("squitter", serde_json::json!({}))).is_none());
    }

    #[test]
    fn a_gate_report_is_decoded_into_a_line() {
        // Q2 is an ETA report: departure airport, then HHMM.
        let e = acars_event("Q2", '4', Some("UAL012"), "EGLL1530");
        let a = acars_of(&e).unwrap();
        let line = a.decoded.expect("an ETA report is recognised");
        assert!(line.contains("ETA Report"), "{line}");
        assert!(line.contains("ETA 1530"), "{line}");
        assert!(!line.starts_with("Q2"), "the label is the row's, not the summary's: {line}");
    }

    #[test]
    fn a_plain_message_has_no_decoded_line() {
        let e = acars_event("H1", '4', Some("UAL012"), "JUST WORDS");
        assert_eq!(acars_of(&e).unwrap().decoded, None);
    }

    /// ACARS messages are kept past the log's own window, in the same order
    /// (newest first) and within their own bound.
    #[test]
    fn the_acars_list_outlives_the_log() {
        let cfg = HfdlSettings::default();
        let mut log = VecDeque::new();
        let mut acars = VecDeque::new();
        let msg = |unix| mk_decode(&cfg, &acars_event("H1", '4', Some("UAL012"), "WORDS"), unix);
        record(&mut log, &mut acars, msg(1));
        for i in 0..(HFDL_LOG_DEPTH as i64 + 5) {
            let sq = mk_decode(&cfg, &event("squitter", serde_json::json!({})), 2 + i);
            record(&mut log, &mut acars, sq);
        }
        assert_eq!(log.len(), HFDL_LOG_DEPTH, "the log is bounded");
        assert!(log.iter().all(|d| d.acars.is_none()), "the log has rolled the message off");
        assert_eq!(acars.len(), 1, "squitters are not messages");
        assert_eq!(acars[0].unix, 1);
        // Newest first, and bounded at its own depth.
        for i in 0..(HFDL_ACARS_DEPTH as i64 + 3) {
            record(&mut log, &mut acars, msg(10 + i));
        }
        assert_eq!(acars.len(), HFDL_ACARS_DEPTH);
        assert_eq!(acars[0].unix, 10 + HFDL_ACARS_DEPTH as i64 + 2);
        assert!(acars.iter().all(|d| d.unix >= 10), "the oldest fell off the back");
    }

    /// A saved system table handed to the worker comes back out in the status,
    /// without waiting for a decode.
    #[test]
    fn a_seeded_table_reaches_the_status() {
        use sdroxide_types::HfdlStationFreq;
        let c = HfdlController::new(
            24_000.0,
            HfdlSettings { enabled: true, frequency_hz: 21_931_000.0 },
        );
        let mut riverhead = HfdlStation::new(4, Some("Riverhead, New York"));
        riverhead.freqs = vec![HfdlStationFreq { freq_hz: 21_931_000, slot: 2, active: false }];
        c.seed_stations(vec![riverhead]);
        // The seed forces the next block's status out; feed short blocks so
        // there is a next block whichever side of the seed the first lands on.
        for _ in 0..4 {
            c.on_rx_iq(&noise(2_400));
        }
        let s = wait_for(&c, |s| !s.stations.is_empty()).expect("the seeded station should arrive");
        assert_eq!(s.stations[0].gs_id, 4);
        assert_eq!(s.stations[0].freqs[0].khz(), 21_931);
        assert_eq!(s.table_version, None, "a saved table is not an over-the-air version");
    }

    /// End to end on the reference off-air capture: the worker, the bounded
    /// queue and the poll path hand a real 21 931 kHz Riverhead squitter back
    /// as a recognised decode — the same chain the engine drives.
    #[test]
    #[ignore = "points at an off-air capture via SDROXIDE_HFDL_SAMPLE"]
    fn an_off_air_capture_decodes_through_the_worker() {
        let path = std::env::var("SDROXIDE_HFDL_SAMPLE")
            .expect("set SDROXIDE_HFDL_SAMPLE to the i16 interleaved complex capture");
        let raw = std::fs::read(&path).expect("read capture");
        let samples: Vec<Complex32> = raw
            .chunks_exact(4)
            .map(|b| {
                Complex32::new(
                    i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0,
                    i16::from_le_bytes([b[2], b[3]]) as f32 / 32768.0,
                )
            })
            .collect();
        let c = HfdlController::new(
            24_000.0,
            HfdlSettings { enabled: true, frequency_hz: 21_931_000.0 },
        );
        let before = now_unix();
        for chunk in samples.chunks(65_536) {
            c.on_rx_iq(chunk);
        }
        let s = wait_for(&c, |s| s.decodes >= 1).expect("a decode should come back");
        let squitter = s.log.iter().find(|d| d.kind == "squitter");
        assert!(squitter.is_some(), "no squitter in {:?}", s.log);
        let sq = squitter.unwrap();
        assert_eq!(sq.freq_khz, 21_931);
        assert!(sq.details.contains("\"gs_id\":4"), "Riverhead: {}", sq.details);
        assert!(sq.unix >= before, "log entry is stamped with a real time");
    }
}
