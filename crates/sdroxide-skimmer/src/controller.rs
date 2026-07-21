//! Threading wrapper around [`CwSkimmer`], mirroring `DigiController`: the
//! realtime engine thread ships IQ blocks to a worker over a bounded channel
//! (dropping on backpressure) and drains emitted spots non-blocking via
//! [`SkimmerController::poll`]. All the heavy DSP runs on the worker thread.

use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use sdroxide_dsp::Complex32 as C32;
use sdroxide_types::SkimmerSpot;

use crate::cw::CwSkimmer;

/// Actions drained from the skimmer each engine tick.
pub enum SkimmerAction {
    Spots(Vec<SkimmerSpot>),
}

enum Job {
    Iq(Vec<C32>),
    Center(f64),
    Stop,
}

pub struct SkimmerController {
    job_tx: Sender<Job>,
    res_rx: Receiver<Vec<SkimmerSpot>>,
    worker: Option<JoinHandle<()>>,
}

impl SkimmerController {
    pub fn new(skim_rate: f64, skim_center_hz: f64) -> Self {
        let (job_tx, job_rx) = bounded::<Job>(64);
        let (res_tx, res_rx) = unbounded::<Vec<SkimmerSpot>>();
        // Emit a fresh spot snapshot roughly 4×/second.
        let emit_every = (skim_rate * 0.25) as usize;
        let worker = std::thread::Builder::new()
            .name("sdroxide-cw-skimmer".into())
            .spawn(move || {
                let mut sk = CwSkimmer::new(skim_rate, skim_center_hz);
                let mut since = 0usize;
                for job in job_rx {
                    match job {
                        Job::Iq(iq) => {
                            since += iq.len();
                            sk.process(&iq);
                            if since >= emit_every {
                                since = 0;
                                let _ = res_tx.send(sk.spots());
                            }
                        }
                        Job::Center(hz) => sk.set_center(hz),
                        Job::Stop => break,
                    }
                }
            })
            .expect("spawn cw-skimmer worker");
        SkimmerController { job_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of skim-rate IQ to the worker. Non-blocking;
    /// drops the block if the worker is behind (backpressure).
    pub fn on_rx_iq(&self, iq: &[C32]) {
        let _ = self.job_tx.try_send(Job::Iq(iq.to_vec()));
    }

    /// Re-center the skim window (band/center change); clears tracks.
    pub fn set_center(&self, center_hz: f64) {
        let _ = self.job_tx.try_send(Job::Center(center_hz));
    }

    /// Drain any spot snapshots produced since the last poll. Non-blocking.
    pub fn poll(&self) -> Vec<SkimmerAction> {
        let mut out = Vec::new();
        while let Ok(spots) = self.res_rx.try_recv() {
            out.push(SkimmerAction::Spots(spots));
        }
        out
    }
}

impl Drop for SkimmerController {
    fn drop(&mut self) {
        let _ = self.job_tx.send(Job::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}
