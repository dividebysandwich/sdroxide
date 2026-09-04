//! The worker thread the engine talks to.
//!
//! Same shape as the ADS-B and ISM controllers: a bounded channel for I/Q that
//! drops blocks rather than stalling the audio thread, an unbounded one for
//! control that must never be dropped behind a backed-up queue, and a whole
//! snapshot of everything twice a second.
//!
//! Inside, the window is split into one downconverter and one
//! [`crate::channel::ChannelRx`] per channel of the plan the window can reach —
//! the ISM decoder's arrangement, because VDL2 is a channel plan and not a
//! single frequency.
//!
//! # Why the whole thing, twice a second
//!
//! The alternative is to send each decoded frame as it arrives. That is fewer
//! bytes and worse in every other way: a remote client that connects mid-session
//! would see nothing until the next transmission, a dropped message would leave
//! a hole in the log forever, and the station table would have to be rebuilt
//! from a stream that had already been thinned. A snapshot has none of those
//! problems, and a dropped one costs nothing because the next carries the same
//! information.

use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, bounded, select, unbounded};
use sdroxide_dsp::{Complex32, Ddc};
use sdroxide_types::{Vdl2ChannelStatus, Vdl2Settings, Vdl2Status};
use tracing::info;

use crate::channel::{ChannelRx, Decoded};
use crate::plan;
use crate::station::Tracker;

/// How often a snapshot goes out.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

/// What the engine drains each tick.
pub enum Vdl2Action {
    Status(Box<Vdl2Status>),
}

struct Iq(Vec<Complex32>);

enum Ctl {
    Window { center_hz: f64, rate_hz: f64 },
    Config(Vdl2Settings),
    Stop,
}

pub struct Vdl2Controller {
    iq_tx: Sender<Iq>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<Vdl2Action>,
    worker: Option<JoinHandle<()>>,
}

impl Vdl2Controller {
    /// `window_rate_hz` is the rate of the I/Q the engine will feed, and
    /// `window_center_hz` the absolute RF frequency it is centred on.
    pub fn new(window_center_hz: f64, window_rate_hz: f64, cfg: Vdl2Settings) -> Vdl2Controller {
        let (iq_tx, iq_rx) = bounded::<Iq>(64);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<Vdl2Action>();

        let worker = std::thread::Builder::new()
            .name("sdroxide-vdl2".into())
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
                            Ok(Ctl::Stop) | Err(_) => break,
                        },
                        recv(iq_rx) -> msg => match msg {
                            Ok(Iq(iq)) => {
                                w.process(&iq);
                                if last_emit.elapsed() >= EMIT_INTERVAL {
                                    last_emit = Instant::now();
                                    if res_tx
                                        .send(Vdl2Action::Status(Box::new(w.status())))
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
            })
            .expect("spawn vdl2 worker");

        Vdl2Controller { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of window-rate I/Q to the worker.
    /// Non-blocking; drops the block if the worker is behind.
    pub fn on_rx_iq(&self, iq: &[Complex32]) {
        let _ = self.iq_tx.try_send(Iq(iq.to_vec()));
    }

    /// The window moved — the front end retuned, or changed rate.
    ///
    /// The log and the station table survive: a receiver nudged a hundred
    /// kilohertz is still listening to the same aeroplanes, and throwing away
    /// an hour of messages for it would be a worse answer than the second of
    /// silence while the chains are rebuilt.
    pub fn set_window(&self, center_hz: f64, rate_hz: f64) {
        let _ = self.ctl_tx.send(Ctl::Window { center_hz, rate_hz });
    }

    pub fn set_config(&self, cfg: Vdl2Settings) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    /// Drain whatever the worker has produced since the last poll. Non-blocking.
    pub fn poll(&self) -> Vec<Vdl2Action> {
        let mut out = Vec::new();
        while let Ok(a) = self.res_rx.try_recv() {
            out.push(a);
        }
        out
    }
}

impl Drop for Vdl2Controller {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

/// One channel of the plan that is actually being decoded.
struct Chan {
    /// Index into [`plan::CHANNELS`].
    index: usize,
    ddc: Ddc,
    rx: ChannelRx,
    buf: Vec<Complex32>,
}

struct Worker {
    window_center_hz: f64,
    window_rate_hz: f64,
    cfg: Vdl2Settings,
    chans: Vec<Chan>,
    tracker: Tracker,
    decoded: Vec<Decoded>,
    /// Samples seen, which at a known rate is the stream clock — used to expire
    /// the station table without asking the operating system the time on every
    /// block.
    samples: u64,
    last_expire: u64,
}

impl Worker {
    fn new(window_center_hz: f64, window_rate_hz: f64, cfg: Vdl2Settings) -> Worker {
        let cfg = cfg.sane();
        let mut w = Worker {
            window_center_hz,
            window_rate_hz,
            cfg,
            chans: Vec::new(),
            tracker: Tracker::new(cfg),
            decoded: Vec::new(),
            samples: 0,
            last_expire: 0,
        };
        w.rebuild();
        w
    }

    /// Open a downconverter and a receiver on every channel the window reaches
    /// and the operator has left switched on.
    ///
    /// The two reasons a channel is dark are recorded separately, because
    /// "outside the receiver's window" and "you turned it off" produce the same
    /// empty column and want completely different answers.
    fn rebuild(&mut self) {
        self.chans.clear();
        for (i, ch) in plan::CHANNELS.iter().enumerate() {
            if !self.cfg.channel_enabled(i)
                || !plan::fits(ch.center_hz, self.window_center_hz, self.window_rate_hz)
            {
                continue;
            }
            let mut ddc = Ddc::new(self.window_rate_hz, plan::CHANNEL_TARGET_RATE_HZ);
            ddc.set_offset_hz(ch.center_hz - self.window_center_hz);
            let rate = ddc.out_rate();
            let rx = ChannelRx::new(ch.center_hz, rate, f32::from(self.cfg.threshold_db));
            self.chans.push(Chan { index: i, ddc, rx, buf: Vec::new() });
        }
        info!(
            channels = self.chans.len(),
            center = self.window_center_hz,
            rate = self.window_rate_hz,
            "VDL2 channels opened"
        );
    }

    fn set_window(&mut self, center_hz: f64, rate_hz: f64) {
        if (center_hz - self.window_center_hz).abs() < 1.0
            && (rate_hz - self.window_rate_hz).abs() < 1.0
        {
            return;
        }
        self.window_center_hz = center_hz;
        self.window_rate_hz = rate_hz;
        self.rebuild();
    }

    fn set_config(&mut self, cfg: Vdl2Settings) {
        let cfg = cfg.sane();
        let channels_changed = cfg.channels != self.cfg.channels;
        self.cfg = cfg;
        self.tracker.set_config(cfg);
        if channels_changed {
            self.rebuild();
            return;
        }
        // A threshold change goes straight through: rebuilding would throw away
        // every channel's learned noise floor, and the noise did not move
        // because the operator dragged a slider.
        for c in &mut self.chans {
            c.rx.set_threshold_db(f32::from(cfg.threshold_db));
        }
    }

    fn process(&mut self, iq: &[Complex32]) {
        self.samples += iq.len() as u64;
        let now = unix_now();
        let mut decoded = std::mem::take(&mut self.decoded);
        for c in &mut self.chans {
            c.buf.clear();
            c.ddc.process(iq, &mut c.buf);
            decoded.clear();
            c.rx.push(&c.buf, &mut decoded);
            for d in &decoded {
                self.tracker.absorb(d, now);
            }
        }
        decoded.clear();
        self.decoded = decoded;

        // Expiring is a scan of the table; once a second is plenty for a list
        // whose window is half an hour.
        if self.samples.saturating_sub(self.last_expire) as f64 >= self.window_rate_hz {
            self.last_expire = self.samples;
            self.tracker.expire(now);
        }
    }

    fn status(&self) -> Vdl2Status {
        let mut st = Vdl2Status {
            window_center_hz: self.window_center_hz,
            window_rate_hz: self.window_rate_hz,
            stations: self.tracker.stations(),
            messages: self.tracker.messages(),
            ..Vdl2Status::default()
        };
        for (i, ch) in plan::CHANNELS.iter().enumerate() {
            let live = self.chans.iter().find(|c| c.index == i);
            let reason = if live.is_some() {
                None
            } else if !self.cfg.channel_enabled(i) {
                Some("switched off".to_string())
            } else {
                Some("outside the receiver's window".to_string())
            };
            let (bursts, frames, floor) = match live {
                Some(c) => {
                    let n = c.rx.counters();
                    (n.bursts, n.frames, c.rx.floor_dbfs())
                }
                None => (0, 0, 0.0),
            };
            st.channels.push(Vdl2ChannelStatus {
                freq_hz: ch.center_hz,
                live: live.is_some(),
                reason,
                bursts,
                frames,
                floor_dbfs: floor,
            });
        }
        for c in &self.chans {
            let n = c.rx.counters();
            st.bursts += n.bursts;
            st.syncs += n.syncs;
            st.headers += n.header_ok;
            st.header_bad += n.header_bad + n.length_insane;
            st.rs_fail += n.rs_fail;
            st.rs_corrected += n.rs_corrected;
            st.hdlc_bad += n.hdlc_bad;
            st.fec_bypassed += n.fec_bypassed;
            st.fcs_bad += n.fcs_bad;
            st.frames += n.frames;
            st.multiblock += n.multiblock;
            st.multiblock_ok += n.multiblock_ok;
        }
        st.samples_per_symbol =
            self.chans.first().map_or(0.0, |c| c.rx.samples_per_symbol() as f32);
        st
    }
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{Vdl2AddrKind, Vdl2Frame, Vdl2Payload};

    /// A frame transmitted into the window on one channel comes out of the
    /// controller — the whole crate, through the thread the engine talks to.
    #[test]
    fn a_frame_on_one_channel_reaches_the_snapshot() {
        let window_rate = 500_000.0;
        let window_center = plan::ideal_center_hz();
        // The Common Signalling Channel, offset from the window centre.
        let ch = sdroxide_types::VDL2_CSC_HZ;

        let frame = crate::avlc::build(
            crate::avlc::Address { addr: 0x10_A1_B2, kind: Vdl2AddrKind::GroundAdmin, cr: true },
            crate::avlc::Address { addr: 0x44_0F_31, kind: Vdl2AddrKind::Aircraft, cr: false },
            crate::avlc::control_octet(Vdl2Frame::Ui { pf: false }),
            &crate::acars::build(
                &sdroxide_types::Vdl2Acars {
                    mode: '2',
                    registration: "OE-LWA".to_string(),
                    label: "H1".to_string(),
                    block_id: '1',
                    msn: "M01A".to_string(),
                    flight: "AUA123".to_string(),
                    text: "OVER THE WINDOW".to_string(),
                    ..sdroxide_types::Vdl2Acars::default()
                },
                true,
            ),
        );

        // Modulate at the window rate, offset onto the channel.
        let p = crate::tx::TxParams {
            sample_rate: window_rate,
            freq_offset_hz: ch - window_center,
            amplitude: 0.5,
            ..crate::tx::TxParams::default()
        };
        let mut burst = crate::tx::modulate(&frame, &p, 4000.0);
        crate::tx::Noise::new(0x5eed).add(&mut burst, 0.004);

        let c = Vdl2Controller::new(window_center, window_rate, Vdl2Settings::default());
        // Quiet first, so every channel's floor is learned before anything
        // arrives — the same order a real receiver starts in.
        let mut quiet = vec![Complex32::default(); 100_000];
        crate::tx::Noise::new(7).add(&mut quiet, 0.004);
        let mut tail = vec![Complex32::default(); 100_000];
        crate::tx::Noise::new(11).add(&mut tail, 0.004);

        // Paced, because the I/Q channel drops blocks rather than blocking the
        // caller — which is right on the audio thread and would silently throw
        // the transmission away here.
        let mut last: Option<Box<Vdl2Status>> = None;
        let feed = |c: &Vdl2Controller, buf: &[Complex32], last: &mut Option<Box<Vdl2Status>>| {
            for block in buf.chunks(8192) {
                c.on_rx_iq(block);
                std::thread::sleep(Duration::from_millis(2));
                for a in c.poll() {
                    let Vdl2Action::Status(st) = a;
                    *last = Some(st);
                }
            }
        };
        feed(&c, &quiet, &mut last);
        feed(&c, &burst, &mut last);
        feed(&c, &tail, &mut last);

        // The worker emits twice a second, and only when something arrives, so
        // keep it fed until a snapshot carrying the frame comes back.
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && last.as_ref().is_none_or(|s| s.frames == 0) {
            feed(&c, &tail[..8192], &mut last);
        }
        let st = last.expect("no snapshot arrived");
        assert_eq!(st.channels.len(), plan::CHANNELS.len());
        assert!(st.channels.iter().all(|c| c.live), "not every channel opened: {:?}", st.channels);
        assert_eq!(st.frames, 1, "counters: bursts {} syncs {}", st.bursts, st.syncs);
        assert_eq!(st.messages.len(), 1);
        let m = &st.messages[0];
        assert_eq!(m.freq_hz, ch);
        assert_eq!(m.src, 0x44_0F_31);
        match &m.payload {
            Vdl2Payload::Acars(a) => {
                assert_eq!(a.flight, "AUA123");
                assert_eq!(a.text, "OVER THE WINDOW");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(st.stations.len(), 1);
    }

    /// A channel the operator switched off, and one the window cannot reach,
    /// are both dark — and the panel is told which is which.
    #[test]
    fn a_dark_channel_says_why_it_is_dark() {
        // The CSC alone, which is the top of the plan.
        let csc = plan::CHANNELS.len() - 1;
        let cfg = Vdl2Settings { channels: 1 << csc, ..Vdl2Settings::default() };
        let c = Vdl2Controller::new(plan::ideal_center_hz(), 500_000.0, cfg);
        c.on_rx_iq(&vec![Complex32::default(); 4096]);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut st = None;
        while Instant::now() < deadline && st.is_none() {
            for a in c.poll() {
                let Vdl2Action::Status(s) = a;
                st = Some(s);
            }
            if st.is_none() {
                c.on_rx_iq(&vec![Complex32::default(); 4096]);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        let st = st.expect("no snapshot");
        assert!(st.channels[csc].live, "the CSC should be the one left on");
        assert_eq!(st.channels[0].reason.as_deref(), Some("switched off"));

        // A window too narrow to reach the outer channels.
        let c = Vdl2Controller::new(sdroxide_types::VDL2_CSC_HZ, 60_000.0, Vdl2Settings::default());
        c.on_rx_iq(&vec![Complex32::default(); 4096]);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut st = None;
        while Instant::now() < deadline && st.is_none() {
            for a in c.poll() {
                let Vdl2Action::Status(s) = a;
                st = Some(s);
            }
            if st.is_none() {
                c.on_rx_iq(&vec![Complex32::default(); 4096]);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        let st = st.expect("no snapshot");
        assert!(st.channels[csc].live);
        assert_eq!(st.channels[0].reason.as_deref(), Some("outside the receiver's window"));
    }
}
