//! Threading wrapper around the channel decoders, mirroring
//! `sdroxide_skimmer::SkimmerController`: the realtime engine thread ships
//! window-rate IQ blocks over a bounded channel (dropping on backpressure) and
//! drains results non-blocking via [`IsmController::poll`]. Every downconverter,
//! gate and slicer runs on the worker.
//!
//! Control traffic rides a separate unbounded channel for the same reason it does
//! in the skimmer: a retune or an off-switch must never be dropped behind a
//! backed-up IQ queue, which is exactly when it is most likely to be sent.
//!
//! # The device table
//!
//! The worker keeps one [`IsmReport`] per device and re-sends the whole table a
//! couple of times a second, rather than forwarding each decode as it happens.
//! See the note on [`sdroxide_types::IsmReport`] for why that is the shape the
//! panel wants; from this side it also bounds the message size on a busy band and
//! means a dropped result costs nothing, because the next snapshot carries the
//! same information.

use std::collections::HashMap;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, bounded, select, unbounded};
use sdroxide_dsp::{Complex32 as C32, Ddc};
use sdroxide_types::{IsmChannelStatus, IsmProtocol, IsmReport, IsmSettings, IsmStatus};
use tracing::info;

use crate::gate::{Burst, Gate};
use crate::plan;
use crate::proto::{self, Scratch};

/// The embedded rtl_433, its downconverter, and the scratch it is fed through.
#[cfg(feature = "rtl433")]
struct Rtl433Lane {
    inst: crate::rtl433::sys::Instance,
    band: &'static crate::rtl433::bands::Band,
    /// The window it is actually decoding, which is the width asked for rounded
    /// to what an integer decimation of the stream can give.
    rate_hz: f64,
    ddc: Ddc,
    /// Window IQ decimated to the band rate.
    buf: Vec<C32>,
    /// The same samples as interleaved cs16, which is what rtl_433 reads.
    cs16: Vec<i16>,
    decodes: u64,
}

/// How often a snapshot goes out. Fast enough that a sensor's reading appears
/// while the operator is still looking at the burst on the waterfall, slow enough
/// that the table is not rebuilt for nothing on a quiet band.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

/// Actions drained from the decoder each engine tick.
pub enum IsmAction {
    Reports(Vec<IsmReport>),
    Status(IsmStatus),
}

/// Realtime data, dropped on backpressure.
struct Iq(Vec<C32>);

/// Control traffic, never dropped.
enum Ctl {
    /// The window moved: the front end retuned, or changed rate.
    Window {
        center_hz: f64,
        rate_hz: f64,
    },
    Config(IsmSettings),
    /// A freshly read `rtl433_flex.conf`, sent when the operator asks for a
    /// reload after editing it.
    Flex(String),
    Stop,
}

pub struct IsmController {
    iq_tx: Sender<Iq>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<IsmAction>,
    worker: Option<JoinHandle<()>>,
}

impl IsmController {
    /// `window_rate_hz` is the rate of the IQ the engine will feed, and
    /// `window_center_hz` the absolute RF frequency that IQ is centred on.
    ///
    /// `flex_conf` is the operator's `rtl433_flex.conf`, verbatim. It is parsed
    /// and validated on the worker so that a spec which does not pass becomes a
    /// line in the panel rather than an error the caller has to route somewhere
    /// — and so there is exactly one place that decides what a valid spec is.
    pub fn new(
        window_center_hz: f64,
        window_rate_hz: f64,
        cfg: IsmSettings,
        flex_conf: String,
    ) -> IsmController {
        let (iq_tx, iq_rx) = bounded::<Iq>(64);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<IsmAction>();

        let worker = std::thread::Builder::new()
            .name("sdroxide-ism".into())
            .spawn(move || {
                let mut w = Worker::new(window_center_hz, window_rate_hz, cfg, flex_conf);
                let mut last_emit = Instant::now();
                loop {
                    select! {
                        recv(ctl_rx) -> msg => match msg {
                            Ok(Ctl::Window { center_hz, rate_hz }) => {
                                w.set_window(center_hz, rate_hz);
                            }
                            Ok(Ctl::Config(next)) => w.set_config(next),
                            Ok(Ctl::Flex(conf)) => w.set_flex_conf(conf),
                            Ok(Ctl::Stop) | Err(_) => break,
                        },
                        recv(iq_rx) -> msg => match msg {
                            Ok(Iq(iq)) => {
                                w.process(&iq);
                                if last_emit.elapsed() >= EMIT_INTERVAL {
                                    last_emit = Instant::now();
                                    if res_tx.send(IsmAction::Reports(w.snapshot())).is_err() {
                                        break;
                                    }
                                    if res_tx.send(IsmAction::Status(w.status())).is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        },
                    }
                }
            })
            .expect("spawn ism worker");

        IsmController { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of window-rate IQ to the worker. Non-blocking;
    /// drops the block if the worker is behind.
    pub fn on_rx_iq(&self, iq: &[C32]) {
        let _ = self.iq_tx.try_send(Iq(iq.to_vec()));
    }

    /// The window moved. Rebuilds the channel decoders and clears their gates —
    /// both the signal and the noise are about to be somebody else's.
    pub fn set_window(&self, center_hz: f64, rate_hz: f64) {
        let _ = self.ctl_tx.send(Ctl::Window { center_hz, rate_hz });
    }

    pub fn set_config(&self, cfg: IsmSettings) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    /// Hand over a freshly read `rtl433_flex.conf`.
    ///
    /// The device table survives: the operator adding a decoder should not cost
    /// them the sensors already on screen.
    pub fn set_flex_conf(&self, conf: String) {
        let _ = self.ctl_tx.send(Ctl::Flex(conf));
    }

    /// Drain whatever the worker has produced since the last poll. Non-blocking.
    pub fn poll(&self) -> Vec<IsmAction> {
        let mut out = Vec::new();
        while let Ok(a) = self.res_rx.try_recv() {
            out.push(a);
        }
        out
    }
}

impl Drop for IsmController {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

/// One channel being listened to.
struct Chan {
    ddc: Ddc,
    gate: Gate,
    buf: Vec<C32>,
}

struct Worker {
    window_center_hz: f64,
    window_rate_hz: f64,
    cfg: IsmSettings,
    chans: Vec<Chan>,
    /// Every channel in the plan and why it is or is not being listened to,
    /// rebuilt whenever the window or the settings change.
    status_chans: Vec<IsmChannelStatus>,
    devices: HashMap<u64, IsmReport>,
    scratch: Scratch,
    bursts_out: Vec<Burst>,
    decodes: u64,
    /// The operator's `rtl433_flex.conf`, verbatim. Kept so the lane can be
    /// rebuilt without going back to disk.
    flex_conf: String,
    #[cfg(feature = "rtl433")]
    rtl433: Option<Rtl433Lane>,
    /// Why the rtl_433 lane is not running, when it should be but is not.
    #[cfg(feature = "rtl433")]
    rtl433_unavailable: Option<String>,
    /// Flex specs the validator refused, kept for the panel.
    #[cfg(feature = "rtl433")]
    rtl433_errors: Vec<String>,
}

impl Worker {
    fn new(
        window_center_hz: f64,
        window_rate_hz: f64,
        cfg: IsmSettings,
        flex_conf: String,
    ) -> Worker {
        let mut w = Worker {
            window_center_hz,
            window_rate_hz,
            cfg,
            chans: Vec::new(),
            status_chans: Vec::new(),
            devices: HashMap::new(),
            scratch: Scratch::default(),
            bursts_out: Vec::new(),
            decodes: 0,
            flex_conf,
            #[cfg(feature = "rtl433")]
            rtl433: None,
            #[cfg(feature = "rtl433")]
            rtl433_unavailable: None,
            #[cfg(feature = "rtl433")]
            rtl433_errors: Vec::new(),
        };
        w.rebuild();
        w
    }

    fn set_window(&mut self, center_hz: f64, rate_hz: f64) {
        if (self.window_center_hz - center_hz).abs() < 1.0
            && (self.window_rate_hz - rate_hz).abs() < 1.0
        {
            return;
        }
        self.window_center_hz = center_hz;
        self.window_rate_hz = rate_hz;
        self.rebuild();
    }

    fn set_config(&mut self, next: IsmSettings) {
        let families_changed = next.families != self.cfg.families;
        let threshold_changed = next.threshold_db != self.cfg.threshold_db;
        let rtl433_changed = next.rtl433 != self.cfg.rtl433;
        self.cfg = next;
        if families_changed || rtl433_changed {
            // The rtl_433 lane decides which native decoders stand down, so a
            // change to either has to go through the same rebuild.
            self.rebuild();
        } else if threshold_changed {
            // Not a rebuild: the noise did not move because a slider did, and
            // throwing away every gate's learned floor would blind the decoder
            // for a second every time the operator nudged the threshold.
            for c in &mut self.chans {
                c.gate.set_threshold_db(f32::from(self.cfg.threshold_db));
            }
        }
        self.trim();
    }

    /// Replace the decoder file and restart the lane on it.
    fn set_flex_conf(&mut self, conf: String) {
        self.flex_conf = conf;
        #[cfg(feature = "rtl433")]
        self.rebuild_rtl433();
    }

    /// Whether the embedded rtl_433 is listening to `freq_hz` right now.
    ///
    /// The question the suppression rule turns on: a native decoder only stands
    /// down for a protocol rtl_433 reads *better*, and only while rtl_433 is
    /// actually in a position to hear it.
    #[cfg(feature = "rtl433")]
    fn rtl433_covers(&self, freq_hz: f64) -> bool {
        self.rtl433.as_ref().is_some_and(|l| (freq_hz - l.band.center_hz).abs() <= l.rate_hz / 2.0)
    }

    #[cfg(not(feature = "rtl433"))]
    fn rtl433_covers(&self, _freq_hz: f64) -> bool {
        false
    }

    /// Whether the native decoder for `p` should run on a burst from `freq_hz`.
    fn native_runs(&self, p: IsmProtocol, freq_hz: f64) -> bool {
        if !self.cfg.protocol_enabled(p) {
            return false;
        }
        #[cfg(feature = "rtl433")]
        if crate::rtl433::suppresses(p, self.rtl433_covers(freq_hz)) {
            return false;
        }
        let _ = freq_hz;
        true
    }

    /// Whether any registry protocol on `center_hz` is enabled and not left to
    /// rtl_433.
    fn any_protocol_on(&self, center_hz: f64) -> bool {
        proto::PROTOCOLS.iter().any(|p| {
            (p.channel_hz - center_hz).abs() < 10_000.0 && self.native_runs(p.id, center_hz)
        })
    }

    /// Native protocols standing down on this window because rtl_433 has them.
    #[cfg(feature = "rtl433")]
    fn superseded(&self) -> Vec<IsmProtocol> {
        let mut v: Vec<IsmProtocol> = proto::PROTOCOLS
            .iter()
            .filter(|p| {
                self.cfg.protocol_enabled(p.id)
                    && crate::rtl433::suppresses(p.id, self.rtl433_covers(p.channel_hz))
            })
            .map(|p| p.id)
            .collect();
        v.dedup();
        v
    }

    /// Whether any registry protocol exists on `center_hz` at all, enabled or not.
    fn any_protocol_implemented_on(center_hz: f64) -> bool {
        proto::PROTOCOLS.iter().any(|p| (p.channel_hz - center_hz).abs() < 10_000.0)
    }

    fn rebuild(&mut self) {
        self.chans.clear();
        self.status_chans.clear();

        // Before the native channels, because which of those run depends on what
        // rtl_433 ended up covering.
        #[cfg(feature = "rtl433")]
        self.rebuild_rtl433();

        for ch in plan::CHANNELS {
            let in_window = plan::fits(ch, self.window_center_hz, self.window_rate_hz);
            // The family switches alone, before rtl_433 is allowed an opinion.
            // The two have to be told apart: a channel the operator switched off
            // and a channel handed to the other lane look identical from here and
            // want completely different things done about them.
            let family_on = proto::PROTOCOLS.iter().any(|p| {
                (p.channel_hz - ch.center_hz).abs() < 10_000.0 && self.cfg.protocol_enabled(p.id)
            });
            let enabled = self.any_protocol_on(ch.center_hz);

            let reason = if !in_window {
                Some("outside the receiver's window".to_string())
            } else if !Self::any_protocol_implemented_on(ch.center_hz) {
                Some("not decoded yet".to_string())
            } else if !family_on {
                Some("switched off".to_string())
            } else if !enabled {
                // Switched on, and still not running: everything on this channel
                // is a protocol rtl_433 reads better, and it is listening here.
                Some("handled by rtl_433".to_string())
            } else {
                None
            };

            self.status_chans.push(IsmChannelStatus {
                freq_hz: ch.center_hz,
                label: ch.label.to_string(),
                live: reason.is_none(),
                reason,
            });

            if !in_window || !enabled {
                continue;
            }

            let mut ddc = Ddc::new(self.window_rate_hz, ch.rate_hz);
            ddc.set_offset_hz(ch.center_hz - self.window_center_hz);
            let rate = ddc.out_rate();
            self.chans.push(Chan {
                ddc,
                gate: Gate::new(rate, ch.center_hz, f32::from(self.cfg.threshold_db)),
                buf: Vec::new(),
            });
        }
    }

    /// Build, move or tear down the embedded rtl_433 to match the settings and
    /// the window.
    ///
    /// The instance is rebuilt rather than retuned when the band changes: its
    /// filters and pulse detector are sized for one sample rate, and a band
    /// change is usually a rate change too. The device table lives outside it and
    /// survives.
    #[cfg(feature = "rtl433")]
    fn rebuild_rtl433(&mut self) {
        use crate::rtl433::{bands, flex, sys};

        self.rtl433 = None;
        self.rtl433_unavailable = None;
        self.rtl433_errors.clear();

        if !self.cfg.rtl433_enabled() {
            return;
        }

        let Some(band) = bands::pick(&self.cfg.rtl433, self.window_center_hz, self.window_rate_hz)
        else {
            // Two ways to reach here and only one of them is about the dial: a
            // band that would have fitted at its own width and does not at the
            // one the operator chose is a bandwidth to turn down, not a
            // frequency to tune to.
            self.rtl433_unavailable = Some(
                if bands::fits_at_default(
                    &self.cfg.rtl433,
                    self.window_center_hz,
                    self.window_rate_hz,
                ) {
                    format!(
                        "the receiver's window is too narrow for {} kHz — choose a narrower \
                         bandwidth, or AUTO",
                        self.cfg.rtl433.bandwidth_hz / 1000
                    )
                } else {
                    "no enabled band is inside the receiver's window".to_string()
                },
            );
            return;
        };

        // `bands::pick` vetted this band at the width it asked for, so the lane
        // must not then be built narrower than that: a `Ddc` left to round its
        // own decimation hands 960 kHz back for a 1024 kHz band, which is both
        // narrower than the band that was chosen and under the 1.2 Msps
        // rtl_433's own wM-Bus mode C&T decoder asks for. More than asked for is
        // fine — it only means the lane hears past its band's edges — so the
        // width is rounded up. See `plan::width_at_least`.
        let want = bands::rate_for(band, &self.cfg.rtl433);
        let (target, _) = plan::width_at_least(self.window_rate_hz, want).unwrap_or((want, want));
        let mut ddc = Ddc::new(self.window_rate_hz, target);
        ddc.set_offset_hz(band.center_hz - self.window_center_hz);
        let rate = ddc.out_rate();

        sys::install_log_handler();
        let Some(mut inst) = sys::Instance::new(rate as u32, band.center_hz as u32) else {
            self.rtl433_unavailable = Some("rtl_433 could not start".to_string());
            return;
        };

        // The operator's own decoders. `parse_conf` validates as it goes —
        // rtl_433 exits the process on a spec it cannot parse — and hands back
        // the ones that did not pass, which become lines in the panel rather
        // than a silently shorter list of decoders.
        let (specs, problems) = flex::parse_conf(&self.flex_conf);
        for p in problems {
            self.rtl433_errors.push(format!("line {}: {}", p.line, p.message));
        }
        for spec in &specs {
            if let Err(e) = inst.register_flex(&spec.spec) {
                self.rtl433_errors.push(format!("line {}: {e}", spec.line));
            }
        }

        info!(
            band = band.label,
            rate,
            decoders = inst.decoder_count(),
            flex = inst.flex_count(),
            "rtl_433 decoders started"
        );

        self.rtl433 = Some(Rtl433Lane {
            inst,
            band,
            rate_hz: rate,
            ddc,
            buf: Vec::new(),
            cs16: Vec::new(),
            decodes: 0,
        });
    }

    /// Decimate the window to the band, convert, and let rtl_433 decode it.
    #[cfg(feature = "rtl433")]
    fn process_rtl433(&mut self, iq: &[C32], now: i64) {
        let Some(lane) = self.rtl433.as_mut() else { return };

        lane.buf.clear();
        lane.ddc.process(iq, &mut lane.buf);
        if lane.buf.is_empty() {
            return;
        }

        // rtl_433 reads interleaved signed 16-bit, which is its native path —
        // no conversion happens on its side and the full dynamic range of the
        // window survives.
        lane.cs16.clear();
        lane.cs16.reserve(lane.buf.len() * 2);
        for s in &lane.buf {
            lane.cs16.push(to_i16(s.re));
            lane.cs16.push(to_i16(s.im));
        }

        let events = lane.inst.feed(&lane.cs16);
        if events.is_empty() {
            return;
        }
        let center = lane.band.center_hz;
        lane.decodes += events.len() as u64;

        for kvs in events {
            let Some(mapped) = crate::rtl433::map::map_event(&kvs, center) else { continue };
            self.record_rtl433(mapped, now);
        }
        self.trim();
    }

    /// Fold one mapped rtl_433 decode into the shared device table.
    #[cfg(feature = "rtl433")]
    fn record_rtl433(&mut self, m: crate::rtl433::map::Mapped, now: i64) {
        let d = m.decoded;
        self.decodes += 1;

        // Two rtl_433 devices can report the same id under different models, so
        // the model is part of the identity as well as the display.
        let model = d.model.clone().unwrap_or_default();
        let id = device_id(d.protocol, &format!("{model}\u{0}{}", d.device));
        let snr = m.snr_db.unwrap_or(0.0).round().clamp(-99.0, 99.0) as i16;

        match self.devices.get_mut(&id) {
            Some(r) => {
                r.freq_hz = m.freq_hz;
                r.snr_db = snr;
                r.last_at = now;
                r.count = r.count.saturating_add(1);
                r.readings = d.readings;
                r.extra = d.extra;
                r.model = d.model;
                r.raw_hex = d.raw_hex;
            }
            None => {
                info!(
                    protocol = "rtl_433",
                    model = %model,
                    device = %d.device,
                    freq_hz = m.freq_hz,
                    snr_db = snr,
                    readings = %d
                        .readings
                        .iter()
                        .map(|r| r.fmt_labelled())
                        .collect::<Vec<_>>()
                        .join(", "),
                    "ISM device heard"
                );
                self.devices.insert(
                    id,
                    IsmReport {
                        id,
                        protocol: d.protocol,
                        model: d.model,
                        device: d.device,
                        freq_hz: m.freq_hz,
                        snr_db: snr,
                        first_at: now,
                        last_at: now,
                        count: 1,
                        readings: d.readings,
                        extra: d.extra,
                        encrypted: d.encrypted,
                        raw_hex: d.raw_hex,
                    },
                );
            }
        }
    }

    fn process(&mut self, iq: &[C32]) {
        let now = unix_now();

        #[cfg(feature = "rtl433")]
        self.process_rtl433(iq, now);

        if self.chans.is_empty() {
            return;
        }
        // Lifted out of `self` so the per-channel borrow can end before
        // `on_burst`, which needs the worker's scratch and device table.
        let mut bursts = std::mem::take(&mut self.bursts_out);
        for i in 0..self.chans.len() {
            bursts.clear();
            {
                let c = &mut self.chans[i];
                // `Ddc::process` appends, so the scratch has to be cleared.
                c.buf.clear();
                c.ddc.process(iq, &mut c.buf);
                let Chan { gate, buf, .. } = c;
                gate.push(buf, &mut bursts);
            }
            for b in &bursts {
                self.on_burst(b, now);
            }
        }
        self.bursts_out = bursts;
    }

    fn on_burst(&mut self, b: &Burst, now: i64) {
        // Which native decoders may run on this burst. Not simply the family
        // switches: a protocol rtl_433 covers stands down while rtl_433 is
        // listening here, and the saving is real because this runs before the
        // slicer rather than after it.
        let allowed: Vec<bool> =
            proto::PROTOCOLS.iter().map(|p| self.native_runs(p.id, b.center_hz)).collect();
        let enabled = |p: IsmProtocol| {
            proto::PROTOCOLS
                .iter()
                .position(|q| q.id == p)
                .is_some_and(|i| allowed.get(i).copied().unwrap_or(false))
        };
        let out = proto::decode(b, &mut self.scratch, &enabled, true);
        for d in out.decoded {
            self.decodes += 1;
            let id = device_id(d.protocol, &d.device);
            let snr = out.snr_db.round().clamp(-99.0, 99.0) as i16;
            let raw_hex = d.raw_hex;
            match self.devices.get_mut(&id) {
                Some(r) => {
                    r.freq_hz = out.freq_hz;
                    r.snr_db = snr;
                    r.last_at = now;
                    r.count = r.count.saturating_add(1);
                    r.readings = d.readings;
                    r.extra = d.extra;
                    r.model = d.model;
                    r.encrypted = d.encrypted;
                    r.raw_hex = raw_hex;
                }
                None => {
                    // Once per device, not per frame: the first time something is
                    // heard is worth a line on the console, and it is how an
                    // operator confirms the decoder is working without opening
                    // the window.
                    info!(
                        protocol = d.protocol.label(),
                        device = %d.device,
                        freq_hz = out.freq_hz,
                        snr_db = snr,
                        readings = %d
                            .readings
                            .iter()
                            .map(|r| r.fmt_labelled())
                            .collect::<Vec<_>>()
                            .join(", "),
                        "ISM device heard"
                    );
                    self.devices.insert(
                        id,
                        IsmReport {
                            id,
                            protocol: d.protocol,
                            model: d.model,
                            device: d.device,
                            freq_hz: out.freq_hz,
                            snr_db: snr,
                            first_at: now,
                            last_at: now,
                            count: 1,
                            readings: d.readings,
                            extra: d.extra,
                            encrypted: d.encrypted,
                            raw_hex,
                        },
                    );
                }
            }
        }
        self.trim();
    }

    /// Drop the least recently heard devices past the cap.
    fn trim(&mut self) {
        let cap = usize::from(self.cfg.max_devices).max(1);
        while self.devices.len() > cap {
            let Some(&oldest) = self.devices.iter().min_by_key(|(_, r)| r.last_at).map(|(k, _)| k)
            else {
                break;
            };
            self.devices.remove(&oldest);
        }
    }

    /// Newest first, which is the order the panel wants and the order that keeps
    /// a device that has just spoken at the top of a long list.
    fn snapshot(&self) -> Vec<IsmReport> {
        let mut v: Vec<IsmReport> = self.devices.values().cloned().collect();
        v.sort_by(|a, b| b.last_at.cmp(&a.last_at).then(a.device.cmp(&b.device)));
        v
    }

    fn status(&self) -> IsmStatus {
        // Only worth suggesting a retune when the window is what is in the way.
        let window_is_the_problem = self
            .status_chans
            .iter()
            .any(|c| c.reason.as_deref() == Some("outside the receiver's window"));
        // "Nothing is listening" is a claim about the whole decoder, not about
        // one of its two lanes. With rtl_433 running there is no problem to
        // report even when every native channel has stood down — which is the
        // usual case on a band rtl_433 covers, and saying otherwise would send
        // an operator looking for a fault while the panel fills with devices.
        #[cfg(feature = "rtl433")]
        let other_lane_running = self.rtl433.is_some();
        #[cfg(not(feature = "rtl433"))]
        let other_lane_running = false;

        IsmStatus {
            channels: self.status_chans.clone(),
            unavailable: (self.chans.is_empty() && !other_lane_running).then(|| {
                if window_is_the_problem {
                    "no ISM channel is inside the receiver's window".to_string()
                } else {
                    "no device family is switched on".to_string()
                }
            }),
            suggest_center_hz: window_is_the_problem.then(plan::ideal_center_hz),
            // Every time a gate opened, including the transmissions that ran too
            // long to be one of these protocols. A large count with no decodes is
            // a busy band full of devices sdroxide cannot read yet, and that is
            // worth showing rather than leaving the panel looking broken.
            bursts: self.chans.iter().map(|c| c.gate.opened).sum(),
            decodes: self.decodes,
            window_center_hz: self.window_center_hz,
            window_rate_hz: self.window_rate_hz,
            rtl433: self.rtl433_status(),
        }
    }

    #[cfg(feature = "rtl433")]
    fn rtl433_status(&self) -> Option<Box<sdroxide_types::Rtl433Status>> {
        use crate::rtl433::{bands, sys};

        if !self.cfg.rtl433_enabled() {
            return None;
        }

        // Every offered band, so a band that does not fit can still say where to
        // tune — the same one-click fix the native channel rows offer.
        let rows: Vec<IsmChannelStatus> = bands::all()
            .iter()
            .map(|b| {
                let on = self.cfg.rtl433.band_enabled(b.bit);
                let live = self.rtl433.as_ref().is_some_and(|l| l.band.bit == b.bit);
                let reason = if !on {
                    Some("switched off".to_string())
                } else if live {
                    None
                } else if !bands::fits(
                    b,
                    &self.cfg.rtl433,
                    self.window_center_hz,
                    self.window_rate_hz,
                ) {
                    // Same distinction the lane's own message draws: too far away
                    // and too wide are different faults with different fixes.
                    Some(
                        if bands::fits_at(b, b.rate_hz, self.window_center_hz, self.window_rate_hz)
                        {
                            "too wide for the receiver's window".to_string()
                        } else {
                            "outside the receiver's window".to_string()
                        },
                    )
                } else {
                    // Fits, enabled, but another band won: only one at a time.
                    Some("another band is live".to_string())
                };
                IsmChannelStatus { freq_hz: b.center_hz, label: b.label.to_string(), live, reason }
            })
            .collect();

        Some(Box::new(sdroxide_types::Rtl433Status {
            running: self.rtl433.is_some(),
            band: self.rtl433.as_ref().map(|l| l.band.label.to_string()).unwrap_or_default(),
            rate_hz: self.rtl433.as_ref().map(|l| l.rate_hz).unwrap_or(0.0),
            decoders: self.rtl433.as_ref().map(|l| l.inst.decoder_count()).unwrap_or(0),
            flex: self.rtl433.as_ref().map(|l| l.inst.flex_count()).unwrap_or(0),
            decodes: self.rtl433.as_ref().map(|l| l.decodes).unwrap_or(0),
            bands: rows,
            superseded: self.superseded(),
            errors: self.rtl433_errors.clone(),
            unavailable: self.rtl433_unavailable.clone(),
            version: sys::version(),
        }))
    }

    #[cfg(not(feature = "rtl433"))]
    fn rtl433_status(&self) -> Option<Box<sdroxide_types::Rtl433Status>> {
        None
    }
}

/// Scale a normalised sample to signed 16-bit, saturating rather than wrapping.
///
/// A clipped sample is a distorted burst; a wrapped one is a phantom edge in the
/// middle of a pulse, which is far worse for a pulse detector.
#[cfg(feature = "rtl433")]
fn to_i16(v: f32) -> i16 {
    (v * 32767.0).clamp(-32768.0, 32767.0) as i16
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// FNV-1a over the protocol and the device string.
///
/// Stable for as long as the process runs, which is all the UI needs, and it
/// costs no dependency. A device that changes its identity — a LaCrosse sensor
/// picks a new random ID when its batteries are replaced — is deliberately a new
/// row: it is, as far as anything on the air can tell, a different sensor.
fn device_id(p: IsmProtocol, device: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |b: u8| {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x1000_0000_01b3);
    };
    mix(p as u8);
    for b in device.as_bytes() {
        mix(*b);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::IsmFamily;

    fn on() -> IsmSettings {
        IsmSettings {
            enabled: true,
            threshold_db: 12,
            families: IsmFamily::Weather.bit(),
            max_devices: 200,
            // The native lane on its own: these tests are about the channel
            // plan and the device table, and an rtl_433 lane would stand the
            // weather decoders down underneath them.
            rtl433: sdroxide_types::Rtl433Settings::OFF,
        }
    }

    /// A window on the ISM band opens the channels that have an enabled decoder
    /// and no others, and says why for each one it skipped.
    ///
    /// The two reasons are not interchangeable and the test insists on which is
    /// which: 868.42 has a decoder that this configuration has switched off,
    /// while the metering channels have none at all, and an operator can only act
    /// on the first of those.
    #[test]
    fn the_worker_opens_only_the_channels_it_can_use() {
        let w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, on(), String::new());
        assert_eq!(w.chans.len(), 1, "only the weather channel is enabled here");
        assert_eq!(w.status_chans.len(), plan::CHANNELS.len());
        let live: Vec<f64> = w.status_chans.iter().filter(|c| c.live).map(|c| c.freq_hz).collect();
        assert_eq!(live, vec![868_300_000.0]);
        for c in w.status_chans.iter().filter(|c| !c.live) {
            let want = if c.freq_hz == 868_420_000.0 { "switched off" } else { "not decoded yet" };
            assert_eq!(
                c.reason.as_deref(),
                Some(want),
                "{:.3} MHz is in the window, so that is not why it is idle",
                c.freq_hz / 1e6
            );
        }
        assert!(w.status().unavailable.is_none());
        assert!(w.status().suggest_center_hz.is_none());
    }

    /// With rtl_433 listening on the same band, the native decoders it covers
    /// stand down — and the channel says so, rather than looking switched off by
    /// an operator who did no such thing.
    ///
    /// The reason matters more than the outcome here: "switched off" sends
    /// somebody hunting for a switch they never touched, and this is the one
    /// place the two lanes are visibly in each other's way.
    #[cfg(feature = "rtl433")]
    #[test]
    fn rtl433_takes_over_the_weather_channel_it_covers() {
        let mut cfg = on();
        cfg.rtl433.bands = crate::rtl433::bands::BANDS[1].bit; // 868 EU
        let w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, cfg, String::new());

        let weather = w
            .status_chans
            .iter()
            .find(|c| c.freq_hz == 868_300_000.0)
            .expect("the weather channel is in the plan");
        assert!(!weather.live, "the native weather decoder should have stood down");
        assert_eq!(weather.reason.as_deref(), Some("handled by rtl_433"));
        assert!(
            w.superseded().contains(&IsmProtocol::Bresser),
            "the panel needs the list to explain the changed protocol name"
        );

        // Z-Wave has no rtl_433 decoder at all, so it is never handed over —
        // whatever the lane is doing on this band.
        let mut cfg = cfg;
        cfg.set_family(IsmFamily::HomeAuto, true);
        let w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, cfg, String::new());
        let zwave = w.status_chans.iter().find(|c| c.freq_hz == 868_420_000.0).unwrap();
        assert!(zwave.live, "Z-Wave must keep its native decoder: rtl_433 has none");
    }

    /// A chosen bandwidth the receiver cannot deliver has to say so in those
    /// words (issue #141).
    ///
    /// "Outside the receiver's window" would be a lie here and an expensive one:
    /// it sends the operator to the dial, which is already on the band, instead
    /// of to the setting they just changed.
    #[cfg(feature = "rtl433")]
    #[test]
    fn a_width_the_receiver_cannot_give_says_so_rather_than_blaming_the_dial() {
        let band = &crate::rtl433::bands::BANDS[3]; // 315 MHz US, 250 kHz of its own
        let mut cfg = on();
        cfg.rtl433.bands = band.bit;

        // Its own width, through a window with room for it: running.
        let w = Worker::new(band.center_hz, 400_000.0, cfg, String::new());
        assert!(w.rtl433.is_some(), "the band fits at its own width");

        // The same window, with a megahertz asked for: not running, and the
        // reason names the bandwidth rather than the frequency.
        cfg.rtl433.bandwidth_hz = 1_024_000;
        let w = Worker::new(band.center_hz, 400_000.0, cfg, String::new());
        assert!(w.rtl433.is_none());
        let why = w.rtl433_unavailable.clone().expect("a reason");
        assert!(why.contains("1024 kHz"), "{why}");
        let rows = w.rtl433_status().expect("the lane is switched on").bands;
        let row = rows.iter().find(|r| r.freq_hz == band.center_hz).expect("the 315 MHz row");
        assert_eq!(row.reason.as_deref(), Some("too wide for the receiver's window"));

        // And tuned somewhere else entirely, the reason goes back to the dial.
        let w = Worker::new(145_000_000.0, 400_000.0, cfg, String::new());
        let why = w.rtl433_unavailable.clone().expect("a reason");
        assert_eq!(why, "no enabled band is inside the receiver's window");
    }

    /// A decoder spec that does not pass has to reach the panel.
    ///
    /// The whole point of validating rather than passing specs straight through
    /// is that rtl_433 would answer a bad one by ending the process; the trade is
    /// only worth anything if the operator is then told which line was refused
    /// and why. The good specs in the same file still load.
    #[cfg(feature = "rtl433")]
    #[test]
    fn a_refused_flex_spec_is_reported_and_the_others_still_load() {
        let mut cfg = on();
        cfg.rtl433.bands = crate::rtl433::bands::BANDS[1].bit;
        let conf = "\
decoder n=good,m=OOK_PWM,s=400,l=800,r=7000
decoder n=bad,m=OOK_PWM,s=400,l=800,r=7000,nonsense=1
";
        let w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, cfg, conf.to_string());
        let st = w.rtl433_status().expect("the lane is on");

        assert_eq!(st.flex, 1, "the good spec should have registered");
        assert_eq!(st.errors.len(), 1, "the bad one should be reported, got {:?}", st.errors);
        assert!(st.errors[0].contains("line 2"), "{:?} should name the line", st.errors);
        assert!(st.errors[0].contains("nonsense"), "{:?} should name the keyword", st.errors);
    }

    /// Point rtl_433 at another band and the native decoders come back. Standing
    /// down is about who is listening *here*, not a permanent handover.
    #[cfg(feature = "rtl433")]
    #[test]
    fn the_native_decoders_return_when_rtl433_looks_elsewhere() {
        let mut cfg = on();
        cfg.rtl433.bands = crate::rtl433::bands::BANDS[0].bit; // 433.92, far away
        let w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, cfg, String::new());
        let weather = w.status_chans.iter().find(|c| c.freq_hz == 868_300_000.0).unwrap();
        assert!(weather.live, "nothing is covering 868 here, so the native decoder must run");
        assert!(w.superseded().is_empty());
    }

    /// Turning the home-automation family on opens the Z-Wave channel, which is
    /// the switch an operator uses after seeing it reported as switched off.
    #[test]
    fn enabling_home_automation_opens_the_zwave_channel() {
        let mut cfg = on();
        cfg.set_family(IsmFamily::HomeAuto, true);
        let w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, cfg, String::new());
        let live: Vec<f64> = w.status_chans.iter().filter(|c| c.live).map(|c| c.freq_hz).collect();
        assert_eq!(live, vec![868_300_000.0, 868_420_000.0]);
    }

    /// A receiver tuned away from the band has to say so, and say where to go —
    /// otherwise an empty panel is indistinguishable from a quiet neighbourhood.
    #[test]
    fn a_receiver_off_the_band_reports_where_to_tune() {
        let w = Worker::new(866_000_000.0, 2_025_000.0, on(), String::new());
        assert!(w.chans.is_empty());
        let st = w.status();
        assert_eq!(
            st.unavailable.as_deref(),
            Some("no ISM channel is inside the receiver's window")
        );
        let want = st.suggest_center_hz.expect("no retune suggested");
        assert!((want - plan::ideal_center_hz()).abs() < 1.0);
        assert!(st.channels.iter().all(|c| !c.live));
    }

    /// Switching every family off is a different problem from being mistuned and
    /// must not be reported as one.
    #[test]
    fn everything_switched_off_is_not_reported_as_mistuned() {
        let mut cfg = on();
        cfg.families = 0;
        let w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, cfg, String::new());
        assert!(w.chans.is_empty());
        let st = w.status();
        assert_eq!(st.unavailable.as_deref(), Some("no device family is switched on"));
        assert!(st.suggest_center_hz.is_none(), "the receiver is tuned correctly");
    }

    /// Nudging the threshold must not rebuild the channels: a gate that has to
    /// relearn its floor is a gate that decodes nothing for a second, and the
    /// operator dragging the slider would see the band go dead.
    #[test]
    fn changing_the_threshold_keeps_the_gates() {
        let mut w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, on(), String::new());
        // Teach the gate a floor.
        let iq = vec![C32::new(0.001, 0.001); 8192];
        for _ in 0..20 {
            w.process(&iq);
        }
        let floor_before = w.chans[0].gate.floor_dbfs();
        assert!(floor_before.is_finite());

        let mut cfg = on();
        cfg.threshold_db = 20;
        w.set_config(cfg);
        assert_eq!(w.chans.len(), 1);
        assert_eq!(w.chans[0].gate.floor_dbfs(), floor_before, "the learned floor was thrown away");
    }

    /// The device table is capped, and it is the *stalest* entry that goes.
    #[test]
    fn the_device_table_evicts_the_least_recently_heard() {
        let mut cfg = on();
        cfg.max_devices = 3;
        let mut w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, cfg, String::new());

        for i in 0..5u8 {
            let id = device_id(IsmProtocol::LaCrosseIt, &i.to_string());
            w.devices.insert(
                id,
                IsmReport {
                    id,
                    protocol: IsmProtocol::LaCrosseIt,
                    model: None,
                    device: i.to_string(),
                    freq_hz: 868_300_000.0,
                    snr_db: 20,
                    first_at: i64::from(i),
                    last_at: i64::from(i),
                    count: 1,
                    readings: Vec::new(),
                    extra: Vec::new(),
                    encrypted: false,
                    raw_hex: String::new(),
                },
            );
            w.trim();
        }

        assert_eq!(w.devices.len(), 3);
        let kept: Vec<String> = w.snapshot().into_iter().map(|r| r.device).collect();
        assert_eq!(kept, vec!["4", "3", "2"], "newest first, and the oldest two dropped");
    }

    /// Two different devices must not collide into one row, and the same device
    /// must not split into two.
    #[test]
    fn device_ids_are_stable_and_distinct() {
        let a = device_id(IsmProtocol::LaCrosseIt, "10");
        assert_eq!(a, device_id(IsmProtocol::LaCrosseIt, "10"));
        assert_ne!(a, device_id(IsmProtocol::LaCrosseIt, "11"));
        // The protocol is mixed in as well as the identity, so two protocols
        // that both number their devices from zero cannot collide.
        assert_ne!(
            device_id(IsmProtocol::LaCrosseIt, ""),
            device_id(IsmProtocol::LaCrosseIt, "\0")
        );
    }

    /// Deterministic noise — no `rand` in this tree.
    fn noise_at(seed: &mut u64, sigma: f32) -> C32 {
        let mut u = || {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            ((*seed >> 11) as f64 / (1u64 << 53) as f64) as f32
        };
        // Box-Muller, so the noise really is Gaussian.
        let (u1, u2) = (u().max(1e-9), u());
        let r = (-2.0 * u1.ln()).sqrt() * sigma;
        C32::new(r * (std::f32::consts::TAU * u2).cos(), r * (std::f32::consts::TAU * u2).sin())
    }

    /// Modulate a byte sequence as the sensor would: preamble, sync word, then
    /// the payload, 2-FSK at `baud`, placed `offset_hz` from the window centre.
    ///
    /// Written from the physical layer's definition — a phase accumulator advanced
    /// at one of two frequencies — and *not* from anything in `demod` or `slice`,
    /// so agreeing with those is evidence rather than a tautology.
    fn modulate(payload: &[u8], rate_hz: f64, offset_hz: f64, amp: f32) -> Vec<C32> {
        let mut bits: Vec<u8> = (0..48).map(|i| (i % 2) as u8).collect();
        for i in (0..16).rev() {
            bits.push(((0x2DD4u32 >> i) & 1) as u8);
        }
        for &b in payload {
            for i in (0..8).rev() {
                bits.push((b >> i) & 1);
            }
        }

        let sps = rate_hz / 17_241.0;
        let dev = 50_000.0;
        let mut out = Vec::new();
        let mut phase = 0.0f64;
        for (i, &b) in bits.iter().enumerate() {
            let f = offset_hz + if b != 0 { dev } else { -dev };
            let n = (((i + 1) as f64 * sps).round() - (i as f64 * sps).round()) as usize;
            for _ in 0..n {
                phase += std::f64::consts::TAU * f / rate_hz;
                out.push(C32::new(amp * phase.cos() as f32, amp * phase.sin() as f32));
            }
        }
        out
    }

    /// The whole stack, end to end: a modulated LaCrosse frame in window-rate IQ
    /// on one side, a device with its readings on the other.
    ///
    /// Every unit test above checks one stage against its own inputs. This is the
    /// only one that checks they are wired together — that the channel `Ddc` lands
    /// the signal where the gate expects it, that the gate hands over enough of
    /// the preamble for the slicers, and that the measured carrier offset is what
    /// the slicing level is taken from.
    #[test]
    fn a_modulated_frame_becomes_a_device_with_readings() {
        const WINDOW: f64 = 2_025_000.0;
        let center = plan::ideal_center_hz();
        let mut w = Worker::new(center, WINDOW, on(), String::new());
        let mut seed = 0x1234_5678_9ABC_DEF0u64;

        // Let the gate learn a floor first. Around 100 ms is ample: the tracker's
        // time constant is a couple of milliseconds of channel time.
        let mut quiet = vec![C32::default(); 16_384];
        for _ in 0..16 {
            for z in quiet.iter_mut() {
                *z = noise_at(&mut seed, 0.002);
            }
            w.process(&quiet);
        }
        assert!(w.devices.is_empty(), "noise alone produced a device");

        // The published capture: 24.1 °C, 34 %RH, new battery, sensor id 26.
        let mut burst =
            modulate(&[0x96, 0xA6, 0x41, 0x22, 0x50], WINDOW, 868_300_000.0 - center, 0.05);
        for z in burst.iter_mut() {
            *z += noise_at(&mut seed, 0.002);
        }
        w.process(&burst);
        // Trailing quiet so the gate shuts and the burst is delivered.
        for _ in 0..4 {
            for z in quiet.iter_mut() {
                *z = noise_at(&mut seed, 0.002);
            }
            w.process(&quiet);
        }

        let reports = w.snapshot();
        assert_eq!(reports.len(), 1, "expected one device, got {}", reports.len());
        let r = &reports[0];
        assert_eq!(r.protocol, IsmProtocol::LaCrosseIt);
        assert_eq!(r.device, "26");
        assert_eq!(r.count, 1);
        assert_eq!(r.readings.len(), 2, "readings were {:?}", r.readings);
        assert!((r.readings[0].value - 24.1).abs() < 1e-9, "temp {}", r.readings[0].value);
        assert!((r.readings[1].value - 34.0).abs() < 1e-9, "humidity {}", r.readings[1].value);
        assert_eq!(r.extra, vec![("battery".to_string(), "new".to_string())]);
        assert_eq!(r.raw_hex, "96 a6 41 22 50", "the raw frame was not carried through");

        // The frequency has to come from the *signal*, not from the channel it was
        // found on: every device in the band would otherwise be reported on the
        // same four frequencies.
        assert!(
            (r.freq_hz - 868_300_000.0).abs() < 15_000.0,
            "reported {:.4} MHz for a transmitter on 868.3000",
            r.freq_hz / 1e6
        );
        assert!(r.snr_db > 12, "SNR came out {} dB", r.snr_db);

        // Heard twice, it is the same row with a higher count — not a second one.
        w.process(&burst);
        for _ in 0..4 {
            for z in quiet.iter_mut() {
                *z = noise_at(&mut seed, 0.002);
            }
            w.process(&quiet);
        }
        let reports = w.snapshot();
        assert_eq!(reports.len(), 1, "the same sensor produced a second row");
        assert_eq!(reports[0].count, 2);
        assert_eq!(reports[0].id, r.id, "the row's identity changed between frames");
    }

    /// Feeding the worker noise must not produce devices — the whole stack from
    /// gate to CRC has to hold together, not just its pieces.
    #[test]
    fn noise_through_the_whole_worker_produces_no_devices() {
        let mut w = Worker::new(plan::ideal_center_hz(), 2_025_000.0, on(), String::new());
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut buf = vec![C32::default(); 8192];
        for _ in 0..120 {
            for z in buf.iter_mut() {
                let mut r = || {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    ((seed >> 11) as f64 / (1u64 << 53) as f64) as f32 - 0.5
                };
                *z = C32::new(r() * 0.02, r() * 0.02);
            }
            w.process(&buf);
        }
        assert!(w.devices.is_empty(), "noise produced {} devices", w.devices.len());
    }
}
