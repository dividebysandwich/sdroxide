//! The handle the rest of the program holds, and the accounting behind it.
//!
//! Same shape as [`sdroxide_airspyhf::AirspyHfHandle`]: one blocking thread owns
//! the device, control goes in over a crossbeam channel, samples come back out
//! through an `rtrb` ring of interleaved `f32`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use rtrb::{Consumer, Producer, RingBuffer};
use sdroxide_types::EladConfig;

use crate::error::Result;
use crate::protocol::Model;
use crate::trace::Trace;

/// How often the stream thread emits a throughput line.
const STATS_INTERVAL: Duration = Duration::from_secs(2);

/// How long the stream must run before the measured rate is worth reporting.
/// Below this the host's own scheduling and the ring filling up dominate.
const RATE_SETTLE: Duration = Duration::from_secs(2);

/// How far the measured rate may differ from the configured one before the
/// operator is told. Wide enough that jitter and a busy machine cannot trip it,
/// narrow enough that the gaps between the real rates — each is twice the last
/// — are all far outside it.
const RATE_TOLERANCE: f64 = 0.10;

/// A control message for the stream thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ctrl {
    Center(f64),
    Attenuator(bool),
    Preselector(bool),
    /// An ASCII CAT command for the FDM-DUO, through the USB gateway.
    Cat(String),
    Shutdown,
}

/// Control messages accumulated over one pass of the thread loop.
///
/// Dragging the panadapter emits hundreds of `Center` messages a second and a
/// retune is a control transfer plus — on a DUO — a CAT write with a busy-wait
/// in front of it. Applying each in turn would put the thread permanently
/// behind the operator's hand *and* starve the completion drain, so the whole
/// channel is collapsed into this and each field applied once, last value wins.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Pending {
    pub center: Option<f64>,
    pub attenuator: Option<bool>,
    pub preselector: Option<bool>,
    /// **Not** coalesced, with one exception. Each CAT command is its own
    /// instruction to the radio and they are not interchangeable — collapsing a
    /// queue of them to the last would drop everything but the most recent. The
    /// exception is a run of dial commands, which are a *value*: see
    /// [`Pending::absorb`].
    pub cat: Vec<String>,
    pub shutdown: bool,
}

impl Pending {
    pub(crate) fn absorb(&mut self, c: Ctrl) {
        match c {
            Ctrl::Center(v) => self.center = Some(v),
            Ctrl::Attenuator(v) => self.attenuator = Some(v),
            Ctrl::Preselector(v) => self.preselector = Some(v),
            // A dial command *is* a value, and only the last of a run of them
            // can matter — so consecutive ones replace each other rather than
            // queueing, exactly as `center` does. Everything else queues.
            //
            // This is not a nicety. On a DUO reached through the USB gateway the
            // dial is commanded with `FA`, and dragging the panadapter emits one
            // per UI frame; each costs a control transfer plus a busy-wait on
            // the radio's CAT buffer, on the same thread that has to keep the
            // sample transfers submitted. A queue of them is a stalled stream.
            //
            // Only *consecutive* ones, so the order of everything else is left
            // exactly as the operator's instructions arrived: `FA` then `TX;` is
            // a frequency and then a key-down, and it still is.
            Ctrl::Cat(s) => match self.cat.last_mut() {
                Some(last) if is_dial_frame(last) && is_dial_frame(&s) => *last = s,
                _ => self.cat.push(s),
            },
            Ctrl::Shutdown => self.shutdown = true,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        *self == Pending::default()
    }
}

/// Whether an ELAD CAT frame is a bare "put the dial here" — the one command
/// that arrives in floods and whose earlier values mean nothing.
///
/// `FA` and `FB` set VFO A and VFO B; the query forms (`FA;`) ask instead of
/// setting and carry no value to collapse, so they are left alone.
fn is_dial_frame(frame: &str) -> bool {
    matches!(frame.get(..2), Some("FA" | "FB")) && frame.len() > 3
}

/// Throughput and health accounting.
pub(crate) struct RxStats {
    nominal_hz: f64,
    /// Which ELAD this is, for the one message whose remedy depends on it: a
    /// sampler's rate is an FPGA image sdroxide loads, and the transceiver's is
    /// whatever mode it was left in.
    model: Model,
    /// When the first sample arrived, and the sample count since then.
    ///
    /// Deliberately *not* the thread start: identifying and configuring the
    /// device is a dozen control transfers before the first sample appears, and
    /// counting that dead time against the sample total biases the measured
    /// rate low — which here would be read as the configured rate being wrong.
    first_iq: Option<Instant>,
    since: Instant,
    win_samples: u64,
    win_dropped: u64,
    /// Discarded while the engine was not reading this receiver because the
    /// station was transmitting. Counted apart from `win_dropped` because it
    /// is not a fault: see [`RxStats::on_dropped_keyed`].
    win_keyed: u64,
    win_errors: u64,
    total_samples: u64,
    total_dropped: u64,
    total_keyed: u64,
    total_errors: u64,
    stalls: u64,
    /// Whether the rate check has already been made. Once only: it is a
    /// statement about the device, not a running condition.
    rate_checked: bool,
}

/// Bytes per second an ELAD puts on the wire at `rate_hz`.
fn wire_bps_of(rate_hz: u32) -> f64 {
    rate_hz as f64 * crate::protocol::sample_bytes(rate_hz) as f64
}

/// Every rate whose wire throughput is within [`RATE_TOLERANCE`] of
/// `wire_bps`, lowest first. Usually one; 3072 and 6144 kHz always come
/// together, because the 16-bit samples at the top rate make its byte rate the
/// same as the one below it.
pub(crate) fn rates_for_wire_bps(wire_bps: f64) -> Vec<u32> {
    sdroxide_types::ELAD_SAMPLE_RATES
        .iter()
        .copied()
        .filter(|&r| (wire_bps / wire_bps_of(r) - 1.0).abs() <= RATE_TOLERANCE)
        .collect()
}

impl RxStats {
    pub(crate) fn new(nominal_hz: f64, model: Model) -> RxStats {
        RxStats {
            nominal_hz,
            model,
            first_iq: None,
            since: Instant::now(),
            win_samples: 0,
            win_dropped: 0,
            win_keyed: 0,
            win_errors: 0,
            total_samples: 0,
            total_dropped: 0,
            total_keyed: 0,
            total_errors: 0,
            stalls: 0,
            rate_checked: false,
        }
    }

    pub(crate) fn on_iq(&mut self, pairs: usize) {
        self.win_samples += pairs as u64;
        match self.first_iq {
            // Start the clock at the first block and do not count that block:
            // it spans an unknown interval reaching back into device setup.
            None => self.first_iq = Some(Instant::now()),
            Some(_) => self.total_samples += pairs as u64,
        }
    }

    pub(crate) fn on_dropped(&mut self, pairs: usize) {
        self.win_dropped += pairs as u64;
        self.total_dropped += pairs as u64;
    }

    /// Record `pairs` complex samples discarded because the ring was full
    /// while the station was transmitting.
    ///
    /// The engine does not read a half-duplex source for the length of an over
    /// and empties the ring on unkey, but this receiver need not be the
    /// transmitter — it may be a separate SDR lent to a rig as a panadapter —
    /// and it carries on streaming throughout. So the ring fills within its own
    /// depth of key-down and everything after that is discarded: expected, at
    /// exactly the sample rate, for as long as the operator transmits. Counting
    /// it as an overrun turns an ordinary over into a warning that blames the
    /// DSP thread and advises a lower sample rate, and leaves the running total
    /// reading as transmit time. See `IqSource::set_rx_paused`, which is what
    /// tells this side which of the two it is looking at.
    pub(crate) fn on_dropped_keyed(&mut self, pairs: usize) {
        self.win_keyed += pairs as u64;
        self.total_keyed += pairs as u64;
    }

    /// What this receiver threw away while the station was transmitting,
    /// phrased so it cannot be read as a fault. Empty when the operator did not
    /// key up, so it costs nothing on a receive-only session.
    fn keyed_note(&self) -> String {
        if self.win_keyed == 0 {
            return String::new();
        }
        format!(
            "; {} sample(s) discarded while keyed (expected — this receiver is not read \
             during an over); {} discarded while keyed in total",
            self.win_keyed, self.total_keyed,
        )
    }

    pub(crate) fn on_error(&mut self) {
        self.win_errors += 1;
        self.total_errors += 1;
    }

    pub(crate) fn on_stall(&mut self) {
        self.stalls += 1;
        self.on_error();
    }

    /// The rate the stream is actually arriving at, once enough of it has.
    pub(crate) fn measured_hz(&self) -> Option<f64> {
        let dt = self.first_iq?.elapsed();
        if dt < RATE_SETTLE || self.total_samples == 0 {
            return None;
        }
        Some(self.total_samples as f64 / dt.as_secs_f64())
    }

    /// Check the stream's real rate against the one it is being read as, once.
    ///
    /// The throughput is the only evidence there is either way, and a mismatch
    /// is not subtle — the rates are octaves apart — but it *is* silent,
    /// because a stream read at the wrong rate is still a perfectly
    /// healthy-looking panadapter of the wrong width.
    ///
    /// What a disagreement means depends on the model. On a sampler the rate is
    /// which FPGA image [`crate::fpga`] loaded, so the two can only differ if
    /// the load did not happen or did not take. An FDM-DUO streams 192 kHz on
    /// its own image and every wider rate on one [`crate::remote`] uploads, so
    /// a DUO read at 192 kHz but arriving wider was left on a wide image by
    /// another program, and one read wider did not take its image.
    ///
    /// Returns a sentence for the operator when the two disagree.
    pub(crate) fn check_rate(&mut self, trace: &Trace) -> Option<String> {
        if self.rate_checked {
            return None;
        }
        let measured = self.measured_hz()?;
        self.rate_checked = true;
        trace.measured_rate(self.nominal_hz, measured);
        if self.nominal_hz <= 0.0 {
            return None;
        }
        // The count is of samples *as decoded*, and the decode depends on the
        // rate being read as: below 6144 kHz a sample is eight bytes on the
        // wire, at 6144 kHz it is four. So the count is first turned back into
        // what was actually measured — bytes per second — and only then
        // compared with what each real rate would put on the wire. Comparing
        // sample counts directly sent a DUO left at 6144 kHz and read as
        // 192 kHz to "3072 kHz", which then read its 16-bit words as 32-bit
        // ones and agreed with itself.
        let nominal = self.nominal_hz.round() as u32;
        let wire_bps = measured * crate::protocol::sample_bytes(nominal) as f64;
        let fits = rates_for_wire_bps(wire_bps);
        if fits.contains(&nominal) {
            return None;
        }
        // What the stream is in. Usually one rate; 3072 and 6144 kHz put the
        // same number of bytes on the wire, so then both are named. Nothing
        // within tolerance names the closest by byte rate, still the best guess.
        let fits = if fits.is_empty() {
            sdroxide_types::ELAD_SAMPLE_RATES
                .iter()
                .copied()
                .min_by(|a, b| {
                    let d = |r: u32| (wire_bps_of(r) - wire_bps).abs();
                    d(*a).partial_cmp(&d(*b)).unwrap_or(std::cmp::Ordering::Equal)
                })
                .into_iter()
                .collect()
        } else {
            fits
        };
        let mode = fits
            .iter()
            .map(|r| format!("{:.0} kHz", *r as f64 / 1000.0))
            .collect::<Vec<_>>()
            .join(" or ");
        let remedy = if self.model.needs_fpga_load() {
            format!(
                "the rate is decided by which FPGA image is loaded, so either ELAD's \
                 `{}` loader did not run or it loaded a different one — set the sample \
                 rate in Settings → Radio to {mode} to match what is in the device, \
                 or check the loader.",
                crate::fpga::LOADER,
            )
        } else if nominal == crate::remote::STANDALONE_RATE_HZ {
            // A DUO read at its stand-alone rate but streaming wider: another
            // program (FDM-SW2, SDR Console) left it on a wide image. The stream
            // thread asks for a reopen that restores stand-alone.
            "the FDM-DUO was left on a wide image by another program (FDM-SW2 or SDR \
             Console)."
                .to_string()
        } else {
            format!(
                "the FDM-DUO's {:.0} kHz image did not take. Power-cycle the DUO, then \
                 reopen it.",
                self.nominal_hz / 1000.0,
            )
        };
        Some(format!(
            "the stream is arriving at {:.1} MB/s but is being read as {:.0} kHz — \
             the device is in its {mode} mode. {remedy}",
            wire_bps / 1e6,
            self.nominal_hz / 1000.0,
        ))
    }

    pub(crate) fn summary(&self) -> String {
        let rate = match self.measured_hz() {
            Some(hz) => format!("{hz:.0} sps measured against {:.0} configured", self.nominal_hz),
            None => "rate not measured".to_string(),
        };
        format!(
            "{} samples, {} dropped, {} transfer errors, {} endpoint stalls; {rate}",
            self.total_samples, self.total_dropped, self.total_errors, self.stalls,
        )
    }

    pub(crate) fn tick(&mut self, trace: &Trace) {
        let dt = self.since.elapsed();
        if dt < STATS_INTERVAL {
            return;
        }
        let ksps = self.win_samples as f64 / dt.as_secs_f64() / 1000.0;
        if self.win_dropped > 0 || self.win_errors > 0 {
            let line = format!(
                "ELAD RX: {} samples ({ksps:.1} ksps) over {:.2}s; \
                 {} sample(s) DROPPED (RX ring full — the DSP thread is not keeping up; \
                 try a lower sample rate), {} transfer error(s); \
                 totals {} dropped / {} errors{}",
                self.win_samples,
                dt.as_secs_f64(),
                self.win_dropped,
                self.win_errors,
                self.total_dropped,
                self.total_errors,
                self.keyed_note(),
            );
            tracing::warn!("{line}");
            trace.note(line);
        } else {
            tracing::debug!(
                "ELAD RX: {} samples ({ksps:.1} ksps) over {:.2}s; total {}{}",
                self.win_samples,
                dt.as_secs_f64(),
                self.total_samples,
                self.keyed_note(),
            );
        }
        self.since = Instant::now();
        self.win_samples = 0;
        self.win_dropped = 0;
        self.win_keyed = 0;
        self.win_errors = 0;
    }
}

/// Push interleaved I/Q into the RX ring, keeping I and Q paired.
///
/// All-or-nothing: if the ring cannot take the whole block it is dropped whole.
/// Pushing what fits would leave the ring one float out of step, swapping I with
/// Q for the rest of the session — a mirrored, unusable spectrum that reads like
/// a driver bug rather than the overrun it is.
///
/// `paused` says whether the engine has stopped reading for an over, which
/// decides how a full ring is accounted for — a fault, or the normal cost of
/// transmitting. It is deliberately not a reason to skip the push: the samples
/// are still offered, and it is the reader's business whether it wants them.
pub(crate) fn push_iq(rx: &mut Producer<f32>, iq: &[f32], stats: &mut RxStats, paused: bool) {
    let Ok(mut chunk) = rx.write_chunk(iq.len()) else {
        if paused {
            stats.on_dropped_keyed(iq.len() / 2);
        } else {
            stats.on_dropped(iq.len() / 2);
        }
        return;
    };
    let (head, tail) = chunk.as_mut_slices();
    head.copy_from_slice(&iq[..head.len()]);
    tail.copy_from_slice(&iq[head.len()..]);
    chunk.commit_all();
}

/// Shared state the stream thread publishes and the handle reads.
pub(crate) struct Shared {
    pub alive: AtomicBool,
    /// Milliseconds since the thread started, at the last sample delivered.
    pub last_rx_ms: AtomicU64,
    /// Set while the engine is transmitting and therefore not reading this
    /// receiver — see `IqSource::set_rx_paused`. Read by the stream thread on
    /// every block so a ring that fills during an over is accounted for as the
    /// cost of transmitting rather than as an overrun.
    pub rx_paused: AtomicBool,
    /// What an FDM-DUO last said it was tuned to, in whole Hz, or zero before
    /// the first answer and on every other model.
    ///
    /// The radio's own knob moves this window, so it is read back rather than
    /// assumed — see `Device::read_tuned`. Whole Hz because the radio reports
    /// whole Hz, which also makes an atomic the whole of the plumbing.
    pub duo_tuned_hz: AtomicU64,
    /// Whether the stream thread asks the radio for its dial at all.
    ///
    /// Off until the owner turns it on, because only one owner needs it: a
    /// DUO driven through its USB gateway, which has no other way to see the
    /// knob. With a CAT serial port the dial already comes back over serial,
    /// and four control transfers a second on the streaming interface would
    /// buy nothing but a chance to stall it.
    pub read_dial: AtomicBool,
    /// Set when a DUO read at 192 kHz turned out to be streaming at another
    /// rate — left on a wide image by another program — and a restore to
    /// stand-alone has been asked for: the source should reopen, and the
    /// reopen does it (see `crate::remote::ask_restore`).
    pub reopen_wanted: AtomicBool,
    /// Set when the handle is dropped rather than released: the radio is being
    /// closed for good, not reopened. See `Device::shutdown`.
    pub leaving: AtomicBool,
}

impl Shared {
    pub(crate) fn new() -> Shared {
        Shared {
            alive: AtomicBool::new(true),
            last_rx_ms: AtomicU64::new(0),
            rx_paused: AtomicBool::new(false),
            duo_tuned_hz: AtomicU64::new(0),
            read_dial: AtomicBool::new(false),
            reopen_wanted: AtomicBool::new(false),
            leaving: AtomicBool::new(false),
        }
    }
}

/// An open ELAD device.
pub struct EladHandle {
    rx: Consumer<f32>,
    ctrl: Sender<Ctrl>,
    shared: Arc<Shared>,
    opened_at: Instant,
    join: Option<JoinHandle<()>>,
    trace: Trace,
    /// Warnings raised after the open handshake — currently the sample-rate
    /// mismatch, which cannot be known until samples have been flowing for a
    /// couple of seconds.
    late_warning: Arc<std::sync::Mutex<Option<String>>>,

    /// Description for logs and the UI, filled in by the thread at open time.
    pub label: String,
    pub model: Model,
    pub serial: Option<String>,
    pub hw_version: Option<(u8, u8)>,
    pub firmware: Option<(u8, u8)>,
    pub sample_rate_hz: f64,
    /// Warnings gathered while opening, for `IqSource::open_status`.
    pub warnings: Vec<String>,
}

impl EladHandle {
    /// Tell the stream thread that the engine has stopped reading for an over,
    /// and then that it has started again — see `IqSource::set_rx_paused`. The
    /// receiver itself is left running: the samples keep arriving and keep
    /// being offered, this only decides whether the ones that no longer fit are
    /// reported as a fault.
    pub fn set_rx_paused(&self, paused: bool) {
        self.shared.rx_paused.store(paused, Ordering::Relaxed);
    }

    /// Open a device and start streaming.
    ///
    /// The device is opened and configured on the stream thread, not here, so
    /// that every control transfer in the process happens on one thread — see
    /// the invariant in [`crate::usb`]. This call blocks until that has either
    /// succeeded or failed.
    pub fn open(cfg: &EladConfig, center_hz: f64) -> Result<EladHandle> {
        crate::stream::spawn(cfg, center_hz)
    }

    /// Whether the stream thread is still running.
    pub fn is_alive(&self) -> bool {
        self.shared.alive.load(Ordering::Relaxed)
    }

    /// How long the device has gone without delivering samples, measured from
    /// the last block or — if none ever arrived — from when it was opened.
    ///
    /// A stream that never starts matters as much as one that stops, so it has
    /// to age the same way.
    pub fn silent_for(&self) -> Duration {
        let since_open = self.opened_at.elapsed();
        let last = Duration::from_millis(self.shared.last_rx_ms.load(Ordering::Relaxed));
        since_open.saturating_sub(last)
    }

    /// Whether the stream was found in another mode than it is read as and a
    /// reopen has been asked for to put the radio right. See
    /// `crate::remote::ask_restore`.
    pub fn wants_reopen(&self) -> bool {
        self.shared.reopen_wanted.load(Ordering::Relaxed)
    }

    /// A warning the stream thread raised after the open returned, taken once.
    pub fn take_late_warning(&self) -> Option<String> {
        self.late_warning.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    /// Drain interleaved I,Q floats into `out`. Always returns an even count,
    /// so the stream can never come out of alignment. Zero means nothing is
    /// available yet.
    pub fn rx_read(&mut self, out: &mut [f32]) -> usize {
        let take = self.rx.slots().min(out.len()) & !1;
        let mut n = 0;
        while n < take {
            match self.rx.pop() {
                Ok(v) => {
                    out[n] = v;
                    n += 1;
                }
                Err(_) => break,
            }
        }
        n
    }

    /// Drop whatever the stream thread queued while the engine was not reading.
    /// The DDC streams right through an over — the PTT line it is keyed with
    /// runs over a different interface entirely — so `rx_read` would otherwise
    /// replay a whole transmission's worth of stale I/Q as fresh receive.
    pub fn discard_pending_rx(&mut self) {
        while self.rx.pop().is_ok() {}
    }

    fn send(&self, c: Ctrl) {
        // A closed channel means the thread has exited; `needs_reopen` will
        // pick that up from `is_alive`, so there is nothing useful to do here.
        let _ = self.ctrl.send(c);
    }

    /// Where the radio itself says the window is, if it has said.
    ///
    /// Only an FDM-DUO answers: its receive window is its VFO, so the
    /// front-panel knob moves it and this is how that becomes visible without a
    /// CAT serial port. `None` until the first read comes back, and on every
    /// model that has no VFO to turn.
    ///
    /// Nothing is asked until [`Self::follow_radio_dial`] turns the read on.
    pub fn tuned_hz(&self) -> Option<f64> {
        match self.shared.duo_tuned_hz.load(Ordering::Relaxed) {
            0 => None,
            hz => Some(hz as f64),
        }
    }

    /// Have the stream thread read the radio's own dial back (see
    /// [`Self::tuned_hz`]), or stop it. For a DUO with no CAT serial port,
    /// which has no other way to see its knob turned; everywhere else the read
    /// is traffic on the streaming interface that nothing uses.
    pub fn follow_radio_dial(&self, on: bool) {
        self.shared.read_dial.store(on, Ordering::Relaxed);
    }

    pub fn set_center_hz(&self, hz: f64) {
        self.send(Ctrl::Center(hz));
    }

    pub fn set_attenuator(&self, on: bool) {
        self.send(Ctrl::Attenuator(on));
    }

    pub fn set_preselector(&self, on: bool) {
        self.send(Ctrl::Preselector(on));
    }

    /// Send an ASCII CAT command to an FDM-DUO through the USB gateway.
    ///
    /// Ignored on an S1 or S2, which have no such command set. This is the path
    /// that works with no serial cable plugged in; where one is, the ordinary
    /// `sdroxide_cat` link is richer — it can *read*, which this cannot.
    pub fn send_cat(&self, cmd: impl Into<String>) {
        if self.model == Model::Duo {
            self.send(Ctrl::Cat(cmd.into()));
        }
    }

    /// Stop the stream thread and let the device go, without dropping the
    /// handle.
    ///
    /// The engine needs this before it can build a replacement front-end: the
    /// USB interface is claimed exclusively and a second claim is refused even
    /// from this same process, so a device that has not let go is one that
    /// cannot be reopened. Blocks until the thread has closed the device.
    ///
    /// Afterwards the handle is inert rather than invalid: [`Self::rx_read`]
    /// drains what is left in the ring and then returns nothing, control
    /// messages go nowhere, and [`Self::is_alive`] is false — which is what
    /// makes `EladSource::needs_reopen` true. Idempotent.
    pub fn release(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }

    pub(crate) fn from_parts(
        rx: Consumer<f32>,
        ctrl: Sender<Ctrl>,
        shared: Arc<Shared>,
        join: JoinHandle<()>,
        trace: Trace,
        late_warning: Arc<std::sync::Mutex<Option<String>>>,
        info: crate::stream::DeviceInfo,
    ) -> EladHandle {
        EladHandle {
            rx,
            ctrl,
            shared,
            opened_at: Instant::now(),
            join: Some(join),
            trace,
            late_warning,
            label: info.label,
            model: info.model,
            serial: info.serial,
            hw_version: info.hw_version,
            firmware: info.firmware,
            sample_rate_hz: info.sample_rate_hz,
            warnings: info.warnings,
        }
    }
}

impl Drop for EladHandle {
    /// Dropped without [`Self::release`] first is a close for good — the
    /// engine releases a source before it reopens one — so a DUO on a wide
    /// image is put back in stand-alone on the way out.
    fn drop(&mut self) {
        self.shared.leaving.store(true, Ordering::Relaxed);
        self.release();
    }
}

/// Size the RX ring for a sample rate — half a second of interleaved floats,
/// rounded up to a power of two, capped so the top rate does not reserve an
/// absurd block. Same formula as the RTL-SDR and Airspy HF+ backends.
pub(crate) fn ring_for(rate_hz: f64) -> (Producer<f32>, Consumer<f32>) {
    let cap = ((rate_hz * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
    RingBuffer::<f32>::new(cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dragging the dial emits hundreds of messages a second and a retune costs
    /// a control transfer plus a CAT write. Only the last value can matter.
    #[test]
    fn pending_keeps_only_the_last_value_of_each_field() {
        let mut p = Pending::default();
        assert!(p.is_empty());
        for hz in [7_000_000.0, 7_050_000.0, 7_074_000.0] {
            p.absorb(Ctrl::Center(hz));
        }
        p.absorb(Ctrl::Attenuator(true));
        p.absorb(Ctrl::Attenuator(false));
        assert_eq!(p.center, Some(7_074_000.0));
        assert_eq!(p.attenuator, Some(false));
        assert!(!p.is_empty());
        // Fields nobody set stay unset, so `apply` does not touch the hardware
        // for settings that did not change.
        assert_eq!(p.preselector, None);
    }

    /// CAT commands are instructions, not a value: each one says something
    /// different and collapsing the queue would drop all but the last.
    #[test]
    fn cat_commands_queue_rather_than_coalesce() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Cat("MD2;".into()));
        p.absorb(Ctrl::Cat("FA00014074000;".into()));
        p.absorb(Ctrl::Cat("TX;".into()));
        assert_eq!(p.cat, vec!["MD2;", "FA00014074000;", "TX;"]);
    }

    /// Except the dial, which is a value like any other. A drag emits one of
    /// these per UI frame and each costs a control transfer plus a busy-wait on
    /// the radio's CAT buffer — on the thread that has to keep the sample
    /// stream fed. Only the frequency the drag ended on can matter.
    #[test]
    fn a_run_of_dial_commands_keeps_only_the_last() {
        let mut p = Pending::default();
        for hz in [14_074_000u64, 14_075_000, 14_076_000] {
            p.absorb(Ctrl::Cat(format!("FA{hz:011};")));
        }
        assert_eq!(p.cat, vec!["FA00014076000;"]);
    }

    /// And only a *consecutive* run: an instruction between two dial commands
    /// is a fence, because the order of the operator's instructions against the
    /// dial is the whole meaning of a key-down.
    #[test]
    fn a_dial_command_after_an_instruction_does_not_swallow_the_one_before_it() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Cat("FA00014074000;".into()));
        p.absorb(Ctrl::Cat("TX;".into()));
        p.absorb(Ctrl::Cat("FA00014200000;".into()));
        assert_eq!(p.cat, vec!["FA00014074000;", "TX;", "FA00014200000;"]);
    }

    /// A read is not a value. `FA;` asks the radio where it is, and two of them
    /// are two questions — though nothing on the gateway can hear the answer,
    /// which is why this only has to not make things worse.
    #[test]
    fn a_dial_query_is_not_collapsed_into_a_dial_command() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Cat("FA;".into()));
        p.absorb(Ctrl::Cat("FA00014074000;".into()));
        assert_eq!(p.cat, vec!["FA;", "FA00014074000;"]);
    }

    /// Shutdown must survive anything that arrives after it in the same batch,
    /// or a busy dial could keep the thread alive past a release.
    #[test]
    fn shutdown_is_sticky() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Shutdown);
        p.absorb(Ctrl::Center(7_074_000.0));
        p.absorb(Ctrl::Attenuator(true));
        assert!(p.shutdown);
    }

    /// An odd ring capacity would eventually split an I/Q pair across the wrap.
    #[test]
    fn the_ring_holds_at_least_half_a_second_and_an_even_number_of_floats() {
        for rate in sdroxide_types::ELAD_SAMPLE_RATES {
            let (p, _c) = ring_for(rate as f64);
            let cap = p.buffer().capacity();
            assert_eq!(cap % 2, 0, "{rate}");
            assert!(cap as f64 >= rate as f64 * 2.0 * 0.5, "{rate}: {cap} floats");
        }
    }

    /// A partial push would leave the ring one float out of step and swap I
    /// with Q for the rest of the session.
    #[test]
    fn push_iq_drops_whole_blocks_rather_than_splitting_a_pair() {
        let (mut prod, mut cons) = RingBuffer::<f32>::new(8);
        let mut stats = RxStats::new(192_000.0, Model::Duo);
        push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0], &mut stats, false);
        assert_eq!(cons.slots(), 4);
        // Six more floats into four free slots: nothing goes in.
        push_iq(&mut prod, &[0.0; 6], &mut stats, false);
        assert_eq!(cons.slots(), 4);
        assert_eq!(stats.total_dropped, 3);
        assert_eq!(cons.pop(), Ok(1.0));
    }

    /// On a transceiver the rate cannot be commanded at all, so this check is
    /// the only thing standing between an operator and a panadapter that is
    /// quietly the wrong width.
    #[test]
    fn a_rate_that_disagrees_with_the_stream_is_reported_once_and_named() {
        let mut s = RxStats::new(192_000.0, Model::Duo);
        // Nothing to say before any samples have arrived.
        let t = Trace::new();
        assert_eq!(s.check_rate(&t), None);

        // Pretend two seconds of 384 kHz went by while reading as 192 kHz.
        s.first_iq = Some(Instant::now() - Duration::from_secs(4));
        s.total_samples = 384_000 * 4;
        let msg = s.check_rate(&t).expect("a doubled rate has to be reported");
        assert!(msg.contains("384 kHz"), "{msg}");
        assert!(msg.contains("192 kHz"), "{msg}");
        // Once only: it is a statement about the device, not a running
        // condition, and repeating it would bury everything else.
        assert_eq!(s.check_rate(&t), None);
    }

    /// A sampler's rate *is* a command — it is which FPGA image was loaded — so
    /// it must not be sent after ELAD's Windows software the way the
    /// transceiver's is.
    #[test]
    fn a_sampler_is_pointed_at_the_loader_rather_than_at_windows() {
        let mut s = RxStats::new(192_000.0, Model::S2);
        s.first_iq = Some(Instant::now() - Duration::from_secs(4));
        s.total_samples = 384_000 * 4;
        let msg = s.check_rate(&Trace::new()).expect("a doubled rate has to be reported");
        assert!(msg.contains(crate::fpga::LOADER), "{msg}");

        let mut duo = RxStats::new(192_000.0, Model::Duo);
        duo.first_iq = Some(Instant::now() - Duration::from_secs(4));
        duo.total_samples = 384_000 * 4;
        let msg = duo.check_rate(&Trace::new()).expect("a doubled rate has to be reported");
        assert!(!msg.contains(crate::fpga::LOADER), "{msg}");
    }

    /// The radio left at 6144 kHz and read as 192 kHz: its 16-bit samples are
    /// decoded as 32-bit ones, so the decoded count runs at half the real rate.
    /// That count alone points at 3072 kHz, which reads the same bytes the same
    /// wrong way and would then agree with itself. Measured in bytes, the
    /// stream is either of the top two, and both have to be named.
    #[test]
    fn a_duo_left_at_the_top_rate_is_not_sent_to_the_one_below_it() {
        let mut s = RxStats::new(192_000.0, Model::Duo);
        s.first_iq = Some(Instant::now() - Duration::from_secs(4));
        // 6144 kHz × 4 bytes, decoded at 8 bytes a sample.
        s.total_samples = 3_072_000 * 4;
        let msg = s.check_rate(&Trace::new()).expect("a wrong rate has to be reported");
        assert!(msg.contains("3072 kHz or 6144 kHz"), "{msg}");
        assert!(msg.contains("left on a wide image"), "{msg}");
    }

    /// The other direction: a 192 kHz stream read as 6144 kHz decodes four
    /// bytes a sample instead of eight and so counts twice as many.
    #[test]
    fn a_stream_read_as_the_top_rate_is_measured_in_bytes() {
        let mut s = RxStats::new(6_144_000.0, Model::Duo);
        s.first_iq = Some(Instant::now() - Duration::from_secs(4));
        s.total_samples = 384_000 * 4;
        let msg = s.check_rate(&Trace::new()).expect("a wrong rate has to be reported");
        assert!(msg.contains("its 192 kHz mode"), "{msg}");
        assert!(msg.contains("did not take"), "{msg}");
    }

    /// 3072 and 6144 kHz put the same bytes on the wire, so reading one as
    /// the other cannot be caught by throughput and must not be reported as
    /// a mismatch either.
    #[test]
    fn the_top_two_rates_cannot_be_told_apart_by_throughput() {
        assert_eq!(rates_for_wire_bps(24_576_000.0), vec![3_072_000, 6_144_000]);
        for r in [192_000u32, 384_000, 768_000, 1_536_000] {
            assert_eq!(rates_for_wire_bps(r as f64 * 8.0), vec![r]);
        }
    }

    #[test]
    fn a_rate_that_agrees_within_tolerance_says_nothing() {
        let mut s = RxStats::new(192_000.0, Model::Duo);
        s.first_iq = Some(Instant::now() - Duration::from_secs(4));
        // 3% slow: a busy machine, not a wrong setting.
        s.total_samples = (192_000.0 * 4.0 * 0.97) as u64;
        assert_eq!(s.check_rate(&Trace::new()), None);
    }

    /// A ring that fills because the engine stopped reading for an over is not
    /// the DSP thread falling behind, and must not reach the fault counters:
    /// that is what turned a healthy station into a warning per two seconds of
    /// transmit and a running total that only measured time on the air.
    #[test]
    fn a_full_ring_while_paused_is_not_an_overrun() {
        let (mut prod, mut cons) = RingBuffer::<f32>::new(4);
        let mut stats = RxStats::new(192_000.0, Model::Duo);

        push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0], &mut stats, true);
        // The over: nobody is draining, so everything after this is discarded.
        push_iq(&mut prod, &[5.0, 6.0], &mut stats, true);
        assert_eq!(stats.total_dropped, 0, "a paused receiver reports no overruns");
        assert_eq!(stats.total_keyed, 1, "the discarded pair is accounted for as keyed");

        // Unpaused, the very same full ring is a genuine overrun again.
        push_iq(&mut prod, &[7.0, 8.0], &mut stats, false);
        assert_eq!(stats.total_dropped, 1);
        assert_eq!(stats.total_keyed, 1);

        // And nothing was let into the ring out of pair alignment on the way.
        while cons.pop().is_ok() {}
        push_iq(&mut prod, &[9.0, 10.0], &mut stats, false);
        assert_eq!(cons.slots() % 2, 0);
    }
}
