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
use sdroxide_types::{HFDL_LOG_DEPTH, HfdlDecode, HfdlFix, HfdlSettings, HfdlStatus};
use xng_mode_hfdl::HfdlChannelDecoder;
use xng_mode_hfdl::pdu::{HfdlEvent, gs_name};

/// Realtime data, dropped on backpressure.
struct Iq {
    samples: Vec<Complex32>,
}

/// Control traffic, never dropped.
enum Ctl {
    Config(HfdlSettings),
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
                    let (mut bursts, mut decodes) = (0u64, 0u64);
                    let mut tick = 0u64;
                    loop {
                        select! {
                            recv(ctl_rx) -> msg => match msg {
                                Ok(Ctl::Config(next)) => cfg = next,
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
                                            for e in events {
                                                if log.len() == HFDL_LOG_DEPTH {
                                                    log.pop_back();
                                                }
                                                log.push_front(mk_decode(&cfg, &e, now));
                                            }
                                            emitted = true;
                                        }
                                    }
                                    // A status always comes back once (tick 1)
                                    // so a fresh client sees `running` and the
                                    // level immediately, then periodically even
                                    // through silence, and immediately whenever
                                    // anything decoded.
                                    if emitted || tick % IDLE_TICK == 0 || tick == 1 {
                                        let _ = res_tx.send(HfdlStatus {
                                            running: cfg.enabled,
                                            level_dbfs: decoder.level_dbfs(),
                                            bursts,
                                            decodes,
                                            log: log.iter().cloned().collect(),
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
    }
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

    /// The decoder only works at the 24 000 Hz lane rate it was validated at:
    /// the same signal at a genuine 25 000 — what an ordinary 2.0 Msps front
    /// end's DDC produces — decodes nothing, and decodes again once it is
    /// resampled back onto 24 000 (issue #497).
    ///
    /// This is why the engine carries `hfdl_rs`: the lane is resampled to
    /// exactly [`sdroxide_types::HFDL_LANE_RATE_HZ`] before the decoder sees
    /// it, whatever the DDC happened to output. A station on real hardware
    /// showed a live waterfall, a noise level and no burst at all.
    #[test]
    #[ignore = "points at an off-air capture via SDROXIDE_HFDL_SAMPLE"]
    fn the_decoder_needs_the_exact_lane_rate() {
        let path = std::env::var("SDROXIDE_HFDL_SAMPLE")
            .expect("set SDROXIDE_HFDL_SAMPLE to the i16 interleaved complex capture");
        let raw = std::fs::read(&path).expect("read capture");
        let base: Vec<Complex32> = raw
            .chunks_exact(4)
            .map(|b| {
                Complex32::new(
                    i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0,
                    i16::from_le_bytes([b[2], b[3]]) as f32 / 32768.0,
                )
            })
            .collect();

        let events = |rate: f64, samples: &[Complex32]| {
            let mut dec = HfdlChannelDecoder::new(rate, 0.0).expect("decoder");
            samples.chunks(8_192).map(|c| dec.process(c).len()).sum::<usize>()
        };

        assert!(events(24_000.0, &base) >= 1, "the 24 kHz reference must decode");

        // A genuine 25 kHz lane: the capture interpolated onto 25 kHz, so the
        // sample spacing changes and the signal does not.
        let ratio = 25_000.0 / 24_000.0;
        let n = (base.len() as f64 / ratio) as usize;
        let up: Vec<Complex32> = (0..n)
            .map(|i| {
                let x = i as f64 * ratio;
                let i0 = x.floor() as usize;
                let f = (x - i0 as f64) as f32;
                let a = base[i0.min(base.len() - 1)];
                let b = base[(i0 + 1).min(base.len() - 1)];
                Complex32::new(a.re + (b.re - a.re) * f, a.im + (b.im - a.im) * f)
            })
            .collect();
        assert_eq!(events(25_000.0, &up), 0, "a plain 25 kHz lane must not decode");
    }
}
