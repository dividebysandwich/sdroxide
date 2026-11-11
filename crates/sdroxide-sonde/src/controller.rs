//! Threading wrapper around the `xng_mode_sonde` model decoders, mirroring
//! `sdroxide_hfdl::HfdlController`: the realtime engine thread ships I/Q
//! blocks to a worker over a bounded channel and drains status updates
//! non-blocking via [`SondeController::poll`]. All the DSP runs on the worker.
//!
//! The lane is a fixed [`crate::SONDE_LANE_RATE_HZ`] baseband centred on the
//! channel, so the decoder is built with a zero offset and no downconverter of
//! its own. A dropped block costs a fraction of one frame (a sonde sends one a
//! second), so the bounded queue is the simple version: `try_send`, drop the
//! whole block.

use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, bounded, select, unbounded};
use sdroxide_dsp::Complex32;
use sdroxide_types::{SondeSettings, SondeStatus};

use crate::model::{SondeModel, decoder_for};
use crate::tracks::TrackBook;

/// Realtime data, dropped on backpressure.
struct Iq {
    samples: Vec<Complex32>,
}

/// Control traffic, never dropped.
enum Ctl {
    Config(SondeSettings),
    Stop,
}

/// Bounded I/Q queue depth: a couple of seconds of 48 kS/s lane at a typical
/// device read.
const IQ_QUEUE_DEPTH: usize = 256;

/// How often a status goes out when nothing has decoded: once every N blocks
/// (about once a second at a normal device read), so the panel's level meter
/// keeps moving without flooding the event channel.
const IDLE_TICK: u64 = 250;

pub struct SondeController {
    iq_tx: Sender<Iq>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<SondeStatus>,
    worker: Option<JoinHandle<()>>,
}

impl SondeController {
    pub fn new(cfg: SondeSettings) -> Self {
        let (iq_tx, iq_rx) = bounded::<Iq>(IQ_QUEUE_DEPTH);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<SondeStatus>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-sonde".into())
            .spawn(move || {
                let mut cfg = cfg;
                // One decoder per model, all fed the same lane.
                let mut decoders = Vec::new();
                for &m in SondeModel::ALL {
                    match decoder_for(m, crate::SONDE_LANE_RATE_HZ, 0.0) {
                        Ok(d) => decoders.push(d),
                        Err(e) => {
                            tracing::error!(%e, model = m.name(), "radiosonde decoder could not start");
                        }
                    }
                }
                if decoders.is_empty() {
                    return;
                }
                let mut book = TrackBook::default();
                let mut frames = 0u64;
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
                                    let now = now_unix();
                                    for dec in decoders.iter_mut() {
                                        for fix in dec.process(&samples) {
                                            book.push(&fix, cfg.frequency_hz, now);
                                            frames += 1;
                                            emitted = true;
                                        }
                                    }
                                }
                                if emitted || tick.is_multiple_of(IDLE_TICK) || tick == 1 {
                                    let _ = res_tx.send(SondeStatus {
                                        running: cfg.enabled,
                                        level_dbfs: decoders.iter().map(|d| d.level_dbfs()).fold(f32::MIN, f32::max),
                                        frames,
                                        sondes: book.snapshot(),
                                    });
                                }
                            }
                            Err(_) => break,
                        },
                    }
                }
            })
            .expect("spawn sonde worker");
        SondeController { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of lane-rate I/Q to the worker. Non-blocking.
    pub fn on_rx_iq(&self, iq: &[Complex32]) {
        let _ = self.iq_tx.try_send(Iq { samples: iq.to_vec() });
    }

    /// Apply new settings to the running worker.
    pub fn set_config(&self, cfg: SondeSettings) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    /// Drain the latest status, non-blocking. Only the newest matters.
    pub fn poll(&self) -> Option<SondeStatus> {
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

impl Drop for SondeController {
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
    use xng_mode_sonde::{CHANNEL_RATE, modulate, whitening};

    /// rs1729/RS `rs41.txt` example 1: a clean 320-byte frame from sonde
    /// K1930293, in its de-whitened form (the same vector xng tests against).
    const FRAME1: &str = "8635f44093df1a602c87e0fa0521e8943d9cef4c7a67393f6d39fb546461f2111b6447ab79a746c80350cda5344157f8c0c12234f46902220f792816174b313933303239331a00000300000a00002f0007322ce53e31991abf12dada3eb68468c16755d51c7a2a15310216060245f302000d08a31607821e08bb210219060243f302000000000000000000000000000000220d7c1e0807d03cdc071fd81ddb19d70a8d0eb602b60cb518d40692ff00ff00ff001c277d59b8d83301ff0f881f0f38f4fe18b283038735ff000000003eb8ff4947201e6e3aff55415f13fc6e005440440cf100009e9f7406f85800832b631719d70010bebc172a8b00000000000000000000000000000000000000000000a48b7b15366181193ef05d07e1245b1be0f721f801f60804107b0b76110000000000000000000000000000000000ecc7";

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    fn wait_for(c: &SondeController, pred: impl Fn(&SondeStatus) -> bool) -> Option<SondeStatus> {
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

    fn frame_iq() -> Vec<Complex32> {
        let mut on_air = hex(FRAME1);
        whitening::xor_mask(&mut on_air, 0);
        let mut iq = modulate::burst_iq(&on_air, CHANNEL_RATE, 0.0, 1.0);
        // Quiet air after the frame, so the decoder sees it end.
        iq.extend(std::iter::repeat_n(Complex32::new(0.0, 0.0), 4800));
        iq
    }

    #[test]
    fn a_frame_on_the_lane_becomes_a_sonde_with_a_place() {
        let c = SondeController::new(SondeSettings { enabled: true, ..Default::default() });
        for chunk in frame_iq().chunks(2048) {
            c.on_rx_iq(chunk);
        }
        let s = wait_for(&c, |s| s.frames >= 1).expect("a status should arrive");
        assert_eq!(s.frames, 1);
        assert_eq!(s.sondes.len(), 1);
        let t = &s.sondes[0];
        assert_eq!(t.model, "RS41");
        assert_eq!(t.serial, "K1930293");
        assert_eq!(t.frame, 5910);
        assert!((t.lat.unwrap() - 46.050_263).abs() < 1e-4);
        assert!((t.lon.unwrap() - 16.110_771).abs() < 1e-4);
        assert!((f64::from(t.alt_m.unwrap()) - 28_410.0).abs() < 1.0);
        assert_eq!(t.track.len(), 1);
    }

    #[test]
    fn a_disabled_lane_decodes_nothing() {
        let c = SondeController::new(SondeSettings::default());
        for chunk in frame_iq().chunks(2048) {
            c.on_rx_iq(chunk);
        }
        let s = wait_for(&c, |s| s.frames >= 1).expect("an idle snapshot arrives");
        assert!(!s.running);
        assert_eq!(s.frames, 0);
    }

    #[test]
    fn pure_noise_never_decodes() {
        let c = SondeController::new(SondeSettings { enabled: true, ..Default::default() });
        let noise: Vec<Complex32> = (0..48_000 * 2)
            .map(|i| Complex32::new((i as f32 * 0.7).sin(), (i as f32 * 1.9 + 1.0).sin()))
            .collect();
        for chunk in noise.chunks(2048) {
            c.on_rx_iq(chunk);
        }
        let s = wait_for(&c, |s| s.running).expect("a snapshot");
        assert_eq!(s.frames, 0);
    }
}
