//! Threading wrapper around one or more rs1729 `*mod` subprocesses.
//!
//! Same shape as the AIS / ADS-B controllers: bounded I/Q that drops under
//! backpressure, unbounded control that must never be dropped, and a whole
//! sonde-table snapshot twice a second. The worker runs a [`ChannelBank`] so
//! two carriers in the same lane each get their own helper.

use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, bounded, select, unbounded};
use sdroxide_dsp::Complex32;
use sdroxide_types::{Rs41Settings, Rs41Status, SondeKind};
use tracing::{info, warn};

use crate::channels::ChannelBank;
use crate::modproc;
use crate::track::Tracker;

const EMIT_INTERVAL: Duration = Duration::from_millis(500);

pub enum Rs41Action {
    Status(Box<Rs41Status>),
}

struct Iq(Vec<Complex32>);

enum Ctl {
    Window { center_hz: f64, rate_hz: f64 },
    Config(Rs41Settings),
    Stop,
}

pub struct Rs41Controller {
    iq_tx: Sender<Iq>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<Rs41Action>,
    worker: Option<JoinHandle<()>>,
}

impl Rs41Controller {
    pub fn new(window_center_hz: f64, window_rate_hz: f64, cfg: Rs41Settings) -> Rs41Controller {
        let (iq_tx, iq_rx) = bounded::<Iq>(64);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<Rs41Action>();

        let worker = std::thread::Builder::new()
            .name("sdroxide-rs41".into())
            .spawn(move || {
                worker_loop(window_center_hz, window_rate_hz, cfg, iq_rx, ctl_rx, res_tx);
            })
            .expect("spawn rs41 worker");

        Rs41Controller { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    pub fn on_rx_iq(&self, iq: &[Complex32]) {
        let _ = self.iq_tx.try_send(Iq(iq.to_vec()));
    }

    pub fn set_window(&self, center_hz: f64, rate_hz: f64) {
        let _ = self.ctl_tx.send(Ctl::Window { center_hz, rate_hz });
    }

    pub fn set_config(&self, cfg: Rs41Settings) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    pub fn poll(&self) -> Option<Rs41Action> {
        let mut out = None;
        while let Ok(a) = self.res_rx.try_recv() {
            out = Some(a);
        }
        out
    }
}

impl Drop for Rs41Controller {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

fn worker_loop(
    window_center_hz: f64,
    window_rate_hz: f64,
    cfg: Rs41Settings,
    iq_rx: Receiver<Iq>,
    ctl_rx: Receiver<Ctl>,
    res_tx: Sender<Rs41Action>,
) {
    let mut track = Tracker::new(window_center_hz, window_rate_hz, cfg);
    let mut center = window_center_hz;
    let mut rate = window_rate_hz;
    let mut cfg = cfg.sane();
    let mut last_emit = Instant::now();
    let mut kind = cfg.kind;
    let mut bank: Option<ChannelBank> = None;

    match open_bank(kind, rate) {
        Ok(b) => {
            track.set_unavailable(None);
            bank = Some(b);
        }
        Err(msg) => {
            warn!(%msg, "sonde *mod unavailable");
            track.set_unavailable(Some(msg));
        }
    }

    loop {
        select! {
            recv(ctl_rx) -> msg => match msg {
                Ok(Ctl::Window { center_hz, rate_hz }) => {
                    center = center_hz;
                    let rate_changed = (rate_hz - rate).abs() >= 1.0;
                    rate = rate_hz;
                    track.set_window(center_hz, rate_hz);
                    if rate_changed {
                        bank = None;
                        match open_bank(kind, rate) {
                            Ok(b) => {
                                track.set_unavailable(None);
                                bank = Some(b);
                            }
                            Err(msg) => track.set_unavailable(Some(msg)),
                        }
                    }
                }
                Ok(Ctl::Config(next)) => {
                    let next = next.sane();
                    let kind_changed = next.kind != kind;
                    cfg = next;
                    track.set_config(next);
                    if kind_changed {
                        kind = next.kind;
                        rate = next.kind.lane_rate_hz();
                        track.set_window(center, rate);
                        bank = None;
                        match open_bank(kind, rate) {
                            Ok(b) => {
                                track.set_unavailable(None);
                                bank = Some(b);
                            }
                            Err(msg) => track.set_unavailable(Some(msg)),
                        }
                    }
                }
                Ok(Ctl::Stop) | Err(_) => break,
            },
            recv(iq_rx) -> msg => match msg {
                Ok(Iq(iq)) => {
                    if cfg.enabled {
                        if let Some(b) = bank.as_mut() {
                            let now = now_unix();
                            let pkgs = b.on_iq(&iq);
                            for (offset_hz, pkg) in pkgs {
                                // Absolute RF: lane centre + channel offset.
                                track.ingest(pkg, center + offset_hz, -40.0, now);
                            }
                            let (frames, bad) = b.frame_counters();
                            track.note_counters(frames, bad);
                            track.age_out(now);
                            if b.len() == 0 {
                                let msg = format!(
                                    "{} exited — reinstall rs1729 *mod on PATH",
                                    kind.binary()
                                );
                                track.set_unavailable(Some(msg));
                                bank = None;
                            }
                        }
                    }
                    if last_emit.elapsed() >= EMIT_INTERVAL {
                        last_emit = Instant::now();
                        if res_tx
                            .send(Rs41Action::Status(Box::new(track.status())))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                Err(_) => break,
            },
        }
    }
    drop(bank);
    info!("sonde worker stopped");
}

fn open_bank(kind: SondeKind, rate_hz: f64) -> Result<ChannelBank, String> {
    if !modproc::binary_on_path(kind) {
        return Err(format!("install {} on PATH (rs1729/RS demod/mod)", kind.binary()));
    }
    ChannelBank::open(kind, rate_hz)
}

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
