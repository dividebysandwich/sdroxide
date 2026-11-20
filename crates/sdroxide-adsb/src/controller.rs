//! Threading wrapper around the decoder, mirroring
//! [`sdroxide_ism::IsmController`]: the realtime engine thread ships window-rate
//! I/Q over a bounded channel (dropping on backpressure) and drains results
//! non-blocking through [`AdsbController::poll`]. The correlator, the slicer and
//! the aircraft table all run on the worker.
//!
//! Control traffic rides a separate unbounded channel for the same reason it
//! does in the skimmer and the ISM decoder: a retune or an off-switch must never
//! be dropped behind a backed-up I/Q queue, which is exactly when it is most
//! likely to be sent.
//!
//! # Why the whole table goes out every time
//!
//! Two hundred and forty samples a second reach this decoder per aircraft in
//! view, and a status message every half second carries all of them at once.
//! Forwarding decodes individually would be a message per squitter — several
//! hundred a second on a busy sector — each of which the panel would have to
//! fold into a table it is already keeping. The snapshot does that folding once,
//! on the worker, and has the property every snapshot has: a dropped one costs
//! nothing, because the next carries the same information.
//!
//! # The stream clock
//!
//! The worker counts samples rather than reading the wall clock for anything
//! that has to be monotonic and fine-grained — the CPR pairing window and the
//! derived turn rate. At a known sample rate that count *is* a clock, it cannot
//! jump, it does not care how the engine is scheduling blocks, and it makes the
//! decoder's arithmetic exactly reproducible from a recording.

use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, bounded, select, unbounded};
use sdroxide_dsp::Complex32 as C32;
use sdroxide_types::{AdsbSettings, AdsbStatus};
use tracing::info;

use crate::demod::Demod;
use crate::frame::{self, Rejected};
use crate::track::Tracker;

/// How often a snapshot goes out.
///
/// Twice a second: an airliner's position updates about that often, so a faster
/// tick would re-send a table that had not moved, and a slower one would make a
/// target visibly step rather than track.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

/// What the engine drains each tick.
pub enum AdsbAction {
    Status(Box<AdsbStatus>),
}

/// Realtime data, dropped on backpressure.
struct Iq(Vec<C32>);

/// Control traffic, never dropped.
enum Ctl {
    Window {
        center_hz: f64,
        rate_hz: f64,
    },
    Config(AdsbSettings),
    /// Where the receiver is, for the surface-position reference.
    Home(Option<(f64, f64)>),
    Stop,
}

pub struct AdsbController {
    iq_tx: Sender<Iq>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<AdsbAction>,
    worker: Option<JoinHandle<()>>,
}

impl AdsbController {
    /// `window_rate_hz` is the rate of the I/Q the engine will feed, and
    /// `window_center_hz` the absolute RF frequency it is centred on.
    pub fn new(window_center_hz: f64, window_rate_hz: f64, cfg: AdsbSettings) -> AdsbController {
        let (iq_tx, iq_rx) = bounded::<Iq>(64);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<AdsbAction>();

        let worker = std::thread::Builder::new()
            .name("sdroxide-adsb".into())
            .spawn(move || {
                let mut w = Worker::new(window_center_hz, window_rate_hz, cfg);
                let mut last_emit = Instant::now();
                loop {
                    select! {
                        recv(ctl_rx) -> msg => match msg {
                            Ok(Ctl::Window { center_hz, rate_hz }) => {
                                w.set_window(center_hz, rate_hz);
                            }
                            Ok(Ctl::Config(next)) => w.set_config(next),
                            Ok(Ctl::Home(h)) => w.set_home(h),
                            Ok(Ctl::Stop) | Err(_) => break,
                        },
                        recv(iq_rx) -> msg => match msg {
                            Ok(Iq(iq)) => {
                                w.process(&iq);
                                if last_emit.elapsed() >= EMIT_INTERVAL {
                                    last_emit = Instant::now();
                                    if res_tx.send(AdsbAction::Status(Box::new(w.status()))).is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        },
                    }
                }
            })
            .expect("spawn adsb worker");

        AdsbController { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of window-rate I/Q to the worker.
    /// Non-blocking; drops the block if the worker is behind.
    pub fn on_rx_iq(&self, iq: &[C32]) {
        let _ = self.iq_tx.try_send(Iq(iq.to_vec()));
    }

    /// The window moved — the front end retuned, or changed rate.
    ///
    /// The aircraft table survives: a receiver nudged a hundred kilohertz is
    /// still looking at the same sky, and throwing away every target for it
    /// would be a worse answer than a second of missed frames.
    pub fn set_window(&self, center_hz: f64, rate_hz: f64) {
        let _ = self.ctl_tx.send(Ctl::Window { center_hz, rate_hz });
    }

    pub fn set_config(&self, cfg: AdsbSettings) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    /// Tell the decoder where the receiver is, so a surface position has a
    /// reference to be decoded against.
    pub fn set_home(&self, home: Option<(f64, f64)>) {
        let _ = self.ctl_tx.send(Ctl::Home(home));
    }

    /// Drain whatever the worker has produced since the last poll. Non-blocking.
    pub fn poll(&self) -> Vec<AdsbAction> {
        let mut out = Vec::new();
        while let Ok(a) = self.res_rx.try_recv() {
            out.push(a);
        }
        out
    }
}

impl Drop for AdsbController {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

struct Worker {
    demod: Demod,
    tracker: Tracker,
    center_hz: f64,
    rate_hz: f64,
    /// Samples seen, which at a known rate is the stream clock.
    samples: u64,
    frames: u64,
    bad_crc: u64,
    unmatched: u64,
    /// Scratch, kept so a block does not allocate.
    cand: Vec<crate::demod::Candidate>,
}

impl Worker {
    fn new(center_hz: f64, rate_hz: f64, cfg: AdsbSettings) -> Worker {
        info!(rate = rate_hz, center = center_hz, "ADS-B decoder started");
        Worker {
            demod: Demod::new(rate_hz),
            tracker: Tracker::new(cfg),
            center_hz,
            rate_hz,
            samples: 0,
            frames: 0,
            bad_crc: 0,
            unmatched: 0,
            cand: Vec::new(),
        }
    }

    fn set_window(&mut self, center_hz: f64, rate_hz: f64) {
        self.center_hz = center_hz;
        if (rate_hz - self.rate_hz).abs() > 1.0 {
            // The demodulator's whole geometry is derived from the rate, so a
            // new rate is a new demodulator. The aircraft table is not touched.
            self.demod = Demod::new(rate_hz);
            self.rate_hz = rate_hz;
            info!(rate = rate_hz, center = center_hz, "ADS-B window rebuilt");
        }
    }

    fn set_config(&mut self, cfg: AdsbSettings) {
        self.tracker.set_config(cfg);
    }

    fn set_home(&mut self, home: Option<(f64, f64)>) {
        self.tracker.set_home(home);
    }

    fn process(&mut self, iq: &[C32]) {
        self.cand.clear();
        self.demod.push(iq, &mut self.cand);
        let now = unix_now();
        let mono = self.samples as f64 / self.rate_hz.max(1.0);
        self.samples += iq.len() as u64;

        // `cand` is taken so the tracker can be borrowed mutably alongside it;
        // it goes straight back, so the buffer is still reused.
        let cand = std::mem::take(&mut self.cand);
        for c in &cand {
            match frame::accept(&c.bytes, |icao| self.tracker.knows(icao, now)) {
                Ok(acc) => {
                    self.frames += 1;
                    let carrier =
                        self.demod.carrier_of(c).map(|off| self.center_hz + f64::from(off));
                    self.tracker.absorb(&acc, &c.bytes, c.rssi_dbfs, carrier, now, mono);
                }
                Err(Rejected::BadCrc | Rejected::Malformed) => self.bad_crc += 1,
                Err(Rejected::Unmatched) => self.unmatched += 1,
                Err(Rejected::Unsupported) => {}
            }
        }
        self.cand = cand;
    }

    fn status(&mut self) -> AdsbStatus {
        let now = unix_now();
        self.tracker.expire(now);
        AdsbStatus {
            aircraft: self.tracker.snapshot(),
            unavailable: None,
            degraded: None,
            suggest_center_hz: None,
            window_center_hz: self.center_hz,
            window_rate_hz: self.rate_hz,
            preambles: self.demod.preambles,
            frames: self.frames,
            bad_crc: self.bad_crc,
            unmatched: self.unmatched,
        }
    }
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
