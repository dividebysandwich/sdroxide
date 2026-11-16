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
    HFDL_ACARS_DEPTH, HFDL_LOG_DEPTH, HfdlAcars, HfdlChannelStatus, HfdlDecode, HfdlFix,
    HfdlSettings, HfdlStation, HfdlStatus,
};
use xng_mode_hfdl::HfdlChannelDecoder;
use xng_mode_hfdl::pdu::{HfdlEvent, gs_name};

use crate::aircraft::AircraftBook;
use crate::stations::StationBook;

/// Realtime data, dropped on backpressure.
struct Iq {
    samples: Vec<Complex32>,
}

/// One channel of the lane: which, and where it sits relative to the lane's
/// centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneChannel {
    pub khz: u32,
    /// The channel's (suppressed-carrier) frequency minus the lane's centre, Hz.
    /// The +1440 Hz subcarrier is the decoder's own business on top of this.
    pub offset_hz: f64,
}

/// A channel decoder and its counts.
struct Chan {
    khz: u32,
    decoder: HfdlChannelDecoder,
    bursts: u64,
    decodes: u64,
}

/// Build the decoders for a lane at `rate_hz`. A channel whose decoder will
/// not build (a rate it cannot take) is skipped with a log line rather than
/// taking the others down with it.
fn build_chans(rate_hz: f64, lane: &[LaneChannel]) -> Vec<Chan> {
    lane.iter()
        .filter_map(|c| match HfdlChannelDecoder::new(rate_hz, c.offset_hz) {
            Ok(decoder) => Some(Chan { khz: c.khz, decoder, bursts: 0, decodes: 0 }),
            Err(e) => {
                tracing::error!(%e, khz = c.khz, "HFDL channel decoder could not start");
                None
            }
        })
        .collect()
}

/// Control traffic, never dropped.
enum Ctl {
    Config(HfdlSettings),
    /// A new lane: its rate and channels. The decoders are rebuilt (each bakes
    /// its offset in); the logs and the station and aircraft books carry on.
    Lane(f64, Vec<LaneChannel>),
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
    /// One channel, the lane centred on it: `rate_hz` is the rate the engine is
    /// feeding (the DDC's actual output, [`sdroxide_types::HFDL_LANE_RATE_HZ`]
    /// or the nearest the DDC can make), and the decoder is built with a zero
    /// offset — the +1440 Hz subcarrier is the decoder's own business.
    pub fn new(rate_hz: f64, cfg: HfdlSettings) -> Self {
        Self::with_lane(rate_hz, vec![LaneChannel { khz: cfg.primary_khz(), offset_hz: 0.0 }], cfg)
    }

    /// Several channels sharing one lane at `rate_hz`, each at its offset from
    /// the lane's centre — every channel decoder sees the same samples and
    /// mixes its own channel out of them.
    pub fn with_lane(rate_hz: f64, lane: Vec<LaneChannel>, cfg: HfdlSettings) -> Self {
        let (iq_tx, iq_rx) = bounded::<Iq>(IQ_QUEUE_DEPTH);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<HfdlStatus>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-hfdl".into())
            .spawn({
                move || {
                    let mut cfg = cfg;
                    let mut chans = build_chans(rate_hz, &lane);
                    if chans.is_empty() {
                        tracing::error!("HFDL decoder could not start");
                        return;
                    }
                    let mut log: VecDeque<HfdlDecode> = VecDeque::with_capacity(HFDL_LOG_DEPTH);
                    // The ACARS messages among them, kept longer than the log
                    // keeps anything: squitters fill the log's 64 entries in
                    // half an hour and a message should outlast that.
                    let mut acars: VecDeque<HfdlDecode> = VecDeque::new();
                    let mut tick = 0u64;
                    let mut book = StationBook::new();
                    let mut fleet = AircraftBook::new();
                    // Set by anything that changes the status without a decode
                    // (a seeded table), so the next block carries it out rather
                    // than the panel waiting for the idle tick.
                    let mut force = false;
                    loop {
                        select! {
                            recv(ctl_rx) -> msg => match msg {
                                Ok(Ctl::Config(next)) => cfg = next,
                                Ok(Ctl::Lane(rate, lane)) => {
                                    let next = build_chans(rate, &lane);
                                    if !next.is_empty() {
                                        chans = next;
                                    }
                                    force = true;
                                }
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
                                        let mut batch: Vec<(usize, HfdlEvent)> = Vec::new();
                                        for (i, ch) in chans.iter_mut().enumerate() {
                                            batch.extend(ch.decoder.process(&samples).into_iter().map(|e| (i, e)));
                                        }
                                        let batch = keep_strongest(batch);
                                        let now = now_unix();
                                        let mut hit = vec![false; chans.len()];
                                        for (i, e) in batch {
                                            let ch = &mut chans[i];
                                            hit[i] = true;
                                            ch.decodes += 1;
                                            book.observe(&e, ch.khz, now);
                                            fleet.observe(&e, ch.khz, now);
                                            record(&mut log, &mut acars, mk_decode(ch.khz, &e, now));
                                            emitted = true;
                                        }
                                        for (ch, h) in chans.iter_mut().zip(hit) {
                                            ch.bursts += u64::from(h);
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
                                        let aircraft = fleet.snapshot(now_unix());
                                        let channels: Vec<HfdlChannelStatus> = chans
                                            .iter()
                                            .map(|c| HfdlChannelStatus {
                                                khz: c.khz,
                                                level_dbfs: c.decoder.level_dbfs(),
                                                bursts: c.bursts,
                                                decodes: c.decodes,
                                            })
                                            .collect();
                                        let _ = res_tx.send(HfdlStatus {
                                            running: cfg.enabled,
                                            level_dbfs: channels[0].level_dbfs,
                                            bursts: channels.iter().map(|c| c.bursts).sum(),
                                            decodes: channels.iter().map(|c| c.decodes).sum(),
                                            log: log.iter().cloned().collect(),
                                            acars: acars.iter().cloned().collect(),
                                            aircraft,
                                            stations,
                                            table_version,
                                            channels,
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

    /// Move to a new lane: rate and channels. See [`Self::with_lane`].
    pub fn set_lane(&self, rate_hz: f64, lane: Vec<LaneChannel>) {
        let _ = self.ctl_tx.send(Ctl::Lane(rate_hz, lane));
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
fn mk_decode(khz: u32, e: &HfdlEvent, unix: i64) -> HfdlDecode {
    HfdlDecode {
        unix,
        kind: e.kind.clone(),
        gs: gs_of(e),
        freq_khz: khz,
        snr_db: e.snr_db,
        freq_skew_hz: e.freq_skew_hz,
        fec_corrected: e.fec_corrected,
        details: serde_json::to_string(&e.details).unwrap_or_default(),
        position: fix_of(e),
        acars: acars_of(e),
    }
}

/// Drop the copies of a frame that decoded on more than one channel of the
/// lane from the same samples, keeping the strongest.
///
/// A very strong burst leaks into a neighbouring channel's decoder — at a lane
/// spacing that folds it onto the neighbour's own frequency it can decode
/// there too, some 45 dB down (seen with a synthesised squitter) — and would
/// then be filed under the wrong channel, marking a station heard where it is
/// not. The same bytes cannot have been sent on two channels in the same
/// instant, so the copy with the best SNR is the one that was on air there.
/// Events with no raw bytes (a reassembled system table) are kept as they are.
fn keep_strongest(batch: Vec<(usize, HfdlEvent)>) -> Vec<(usize, HfdlEvent)> {
    let snr = |e: &HfdlEvent| e.snr_db.unwrap_or(f32::NEG_INFINITY);
    let mut out: Vec<(usize, HfdlEvent)> = Vec::with_capacity(batch.len());
    for (i, e) in batch {
        let dup = if e.raw.is_empty() {
            None
        } else {
            out.iter().position(|(j, o)| *j != i && o.kind == e.kind && o.raw == e.raw)
        };
        match dup {
            Some(k) if snr(&e) > snr(&out[k].1) => out[k] = (i, e),
            Some(_) => {}
            None => out.push((i, e)),
        }
    }
    out
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
pub(crate) fn acars_of(e: &HfdlEvent) -> Option<HfdlAcars> {
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
            HfdlSettings { enabled: true, frequency_hz: 21_931_000.0, ..Default::default() },
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
        let d = mk_decode(21_931, &e, 1_000);
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
        let mut log = VecDeque::new();
        let mut acars = VecDeque::new();
        let msg = |unix| mk_decode(21_931, &acars_event("H1", '4', Some("UAL012"), "WORDS"), unix);
        record(&mut log, &mut acars, msg(1));
        for i in 0..(HFDL_LOG_DEPTH as i64 + 5) {
            let sq = mk_decode(21_931, &event("squitter", serde_json::json!({})), 2 + i);
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
            HfdlSettings { enabled: true, frequency_hz: 21_931_000.0, ..Default::default() },
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

    /// A squitter synthesised at the lane rate, `offset_hz` from the lane's
    /// centre (plus the +1440 Hz subcarrier), with a little noise.
    fn squitter_at(rate: f64, offset_hz: f64, gs: u8) -> Vec<Complex32> {
        use xng_mode_hfdl::fec::SETTINGS;
        use xng_mode_hfdl::modulate::{burst_symbols, modulate};
        let syms = burst_symbols(&xng_mode_hfdl::pdu::build_spdu(gs, 1234, 52), &SETTINGS[0]);
        let mut iq = vec![Complex32::new(0.0, 0.0); (rate * 0.3) as usize];
        iq.extend(modulate(&syms, rate, offset_hz + 1440.0, 0.5));
        iq.extend(vec![Complex32::new(0.0, 0.0); (rate * 0.3) as usize]);
        let mut n = noise(iq.len());
        for (x, w) in iq.iter_mut().zip(n.drain(..)) {
            *x += w * 0.01;
        }
        iq
    }

    /// Two channels share one lane: a squitter on the upper one decodes, and it
    /// is filed under that channel — not the other, and not the lane's centre.
    ///
    /// The spacing here (36 kHz in a 72 kS/s lane) puts the burst on the other
    /// channel's Nyquist fold, where it leaks into that decoder strongly enough
    /// to decode too; `keep_strongest` is what keeps it to one channel.
    #[test]
    fn a_two_channel_lane_decodes_each_channel_as_itself() {
        let rate = 72_000.0;
        let lane = vec![
            LaneChannel { khz: 13_276, offset_hz: -18_000.0 },
            LaneChannel { khz: 13_312, offset_hz: 18_000.0 },
        ];
        let c = HfdlController::with_lane(
            rate,
            lane,
            HfdlSettings { enabled: true, frequency_hz: 13_276_000.0, ..Default::default() },
        );
        for chunk in squitter_at(rate, 18_000.0, 7).chunks(4096) {
            c.on_rx_iq(chunk);
        }
        let s = wait_for(&c, |s| s.decodes > 0).expect("the squitter should decode");
        let sq = s.log.iter().find(|d| d.kind == "squitter").expect("a squitter in the log");
        assert_eq!(sq.freq_khz, 13_312, "filed under the channel it was on");
        assert_eq!(s.channels.len(), 2);
        assert_eq!(s.channels[0].khz, 13_276);
        assert_eq!(s.channels[0].decodes, 0, "nothing on the other channel");
        assert!(s.channels[1].decodes > 0);
        // The station book hears it on that channel too.
        let gs = s.stations.iter().find(|st| st.gs_id == 7).expect("Shannon heard");
        assert_eq!(gs.heard_khz, 13_312);
    }

    #[test]
    fn a_frame_decoded_on_two_channels_is_kept_once_at_the_stronger() {
        let ev = |snr: f32, raw: &[u8]| HfdlEvent {
            kind: "squitter".into(),
            details: serde_json::json!({}),
            acars: None,
            fec_corrected: None,
            freq_skew_hz: None,
            snr_db: Some(snr),
            raw: raw.to_vec(),
        };
        let kept =
            keep_strongest(vec![(0, ev(1.4, &[1, 2])), (1, ev(46.0, &[1, 2])), (0, ev(9.0, &[3]))]);
        assert_eq!(kept.len(), 2);
        assert_eq!(
            (kept[0].0, kept[0].1.snr_db),
            (1, Some(46.0)),
            "the strong copy, on its channel"
        );
        assert_eq!(kept[1].1.raw, vec![3], "a different frame is not a duplicate");
        // Two frames with the same bytes on the same channel are two
        // transmissions (a repeated squitter), not a leak.
        assert_eq!(keep_strongest(vec![(0, ev(5.0, &[1])), (0, ev(5.0, &[1]))]).len(), 2);
    }

    /// A new lane rebuilds the decoders but keeps what has been heard.
    #[test]
    fn moving_the_lane_keeps_the_log() {
        let rate = 24_000.0;
        let c = HfdlController::new(
            rate,
            HfdlSettings { enabled: true, frequency_hz: 8_912_000.0, ..Default::default() },
        );
        for chunk in squitter_at(rate, 0.0, 4).chunks(4096) {
            c.on_rx_iq(chunk);
        }
        wait_for(&c, |s| s.decodes > 0).expect("the squitter should decode");
        c.set_lane(rate, vec![LaneChannel { khz: 5_652, offset_hz: 0.0 }]);
        c.on_rx_iq(&noise(4096));
        let s = wait_for(&c, |s| s.channels.first().is_some_and(|ch| ch.khz == 5_652))
            .expect("the new lane reports");
        assert!(
            s.log.iter().any(|d| d.kind == "squitter" && d.freq_khz == 8_912),
            "the log carries on"
        );
        assert!(
            s.stations.iter().any(|st| st.gs_id == 4 && st.heard_at > 0),
            "so does the station book"
        );
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
            HfdlSettings { enabled: true, frequency_hz: 21_931_000.0, ..Default::default() },
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
