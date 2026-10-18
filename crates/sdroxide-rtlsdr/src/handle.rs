//! The handle the rest of the program holds, and the accounting behind it.
//!
//! Same shape as [`sdroxide_hpsdr::HpsdrHandle`]: one blocking thread owns the
//! device, control goes in over a crossbeam channel, samples come back out
//! through an `rtrb` ring of interleaved `f32`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use rtrb::{Consumer, Producer, RingBuffer};
use sdroxide_types::{RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode};

use crate::error::Result;

/// How often the stream thread emits a throughput line.
const STATS_INTERVAL: Duration = Duration::from_secs(2);

/// A control message for the stream thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ctrl {
    Center(f64),
    Gain(f64),
    Agc(RtlSdrAgc),
    HfMode(RtlSdrHfMode),
    Ppm(i32),
    BiasTee(bool),
    /// One `rsp_tcp` control. Carried as an opaque `(opcode, argument)` pair
    /// rather than a variant apiece because the whole set is one five-byte
    /// write with no reply and no interaction between them — and because the
    /// USB backend, which shares this enum, has no use for any of it.
    Rsp(crate::tcp::proto::RspCmd, u32),
    Shutdown,
}

/// Control messages accumulated over one pass of the thread loop.
///
/// A retune costs ~25 ms of I2C, and dragging the panadapter emits hundreds of
/// `Center` messages a second. Applying each in turn would put the thread
/// permanently behind the operator's hand *and* starve the completion drain, so
/// the whole channel is collapsed into this and each field applied once, last
/// value wins — which is exactly the right semantics for a dial.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Pending {
    pub center: Option<f64>,
    pub gain: Option<f64>,
    /// `rsp_tcp` controls, last value wins per opcode. A `Vec` rather than a
    /// field apiece: they are pass-through writes, and a dial drag never
    /// touches them.
    pub rsp: Vec<(crate::tcp::proto::RspCmd, u32)>,
    pub agc: Option<RtlSdrAgc>,
    pub hf_mode: Option<RtlSdrHfMode>,
    pub ppm: Option<i32>,
    pub bias_tee: Option<bool>,
    pub shutdown: bool,
}

impl Pending {
    pub(crate) fn absorb(&mut self, c: Ctrl) {
        match c {
            Ctrl::Center(v) => self.center = Some(v),
            Ctrl::Gain(v) => self.gain = Some(v),
            Ctrl::Agc(v) => self.agc = Some(v),
            Ctrl::HfMode(v) => self.hf_mode = Some(v),
            Ctrl::Ppm(v) => self.ppm = Some(v),
            Ctrl::BiasTee(v) => self.bias_tee = Some(v),
            Ctrl::Rsp(op, arg) => match self.rsp.iter_mut().find(|(o, _)| *o == op) {
                Some(slot) => slot.1 = arg,
                None => self.rsp.push((op, arg)),
            },
            Ctrl::Shutdown => self.shutdown = true,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        *self == Pending::default()
    }
}

/// Throughput and health accounting, mirroring the HPSDR backend's.
pub(crate) struct RxStats {
    nominal_hz: f64,
    /// When the first sample arrived, and the sample count since then.
    ///
    /// Deliberately *not* the thread start: configuring the device is a few
    /// hundred milliseconds of I2C before the first sample appears, and
    /// counting that dead time against the sample total biases the clock
    /// estimate low — by over 100 ppm at first, converging so slowly that the
    /// figure is still visibly drifting a minute in. Since the whole point of
    /// the estimate is to hand the operator a number to type into the ppm
    /// field, a number that keeps moving is worse than none.
    first_iq: Option<Instant>,
    since: Instant,
    /// Whether the samples are paced by the dongle's own clock, which is what
    /// makes the clock estimate mean anything. See [`RxStats::network`].
    hw_paced: bool,
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
}

impl RxStats {
    pub(crate) fn new(nominal_hz: f64) -> RxStats {
        RxStats {
            nominal_hz,
            first_iq: None,
            since: Instant::now(),
            hw_paced: true,
            win_samples: 0,
            win_dropped: 0,
            win_keyed: 0,
            win_errors: 0,
            total_samples: 0,
            total_dropped: 0,
            total_keyed: 0,
            total_errors: 0,
        }
    }

    /// Accounting for samples that arrived over a socket, where the clock
    /// estimate is withheld rather than reported.
    ///
    /// The measurement counts samples against elapsed time here, and over a
    /// network what it measures is the *buffering*, not the crystal: the
    /// difference between the two is however many samples are sitting in the
    /// server's queue, the kernel's, and the ring at the moment of the reading,
    /// and that moves by tens of milliseconds from one reading to the next.
    /// Measured against a real `rtl_tcp` on loopback — the most favourable link
    /// there is — successive readings 25 seconds in ranged from -770 to
    /// +2600 ppm, and the error only decays as 1/t: reaching the few ppm the
    /// figure is meant to resolve would take hours.
    ///
    /// So it is not shown. A number that looks like a measurement and is off by
    /// three orders of magnitude is worse than no number, and this one came
    /// with "set this as the ppm correction" beside it.
    pub(crate) fn network(nominal_hz: f64) -> RxStats {
        RxStats { hw_paced: false, ..RxStats::new(nominal_hz) }
    }

    pub(crate) fn on_iq(&mut self, pairs: usize) {
        self.win_samples += pairs as u64;
        match self.first_iq {
            // Start the clock at the first block and do not count that block:
            // it spans an unknown interval reaching back into device setup.
            // Everything after it maps exactly onto measured elapsed time.
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

    /// The dongle's sample clock measured against the host's, in ppm.
    ///
    /// This is a free crystal-error estimate, because the device clocks samples
    /// out at `xtal / resampler_ratio` — the same oscillator the tuner PLL runs
    /// from. Whatever this reports is very nearly the number to type into the
    /// ppm setting, which turns an otherwise fiddly calibration into reading a
    /// log line.
    ///
    /// Only over a link that the dongle paces, though — see
    /// [`RxStats::network`], where this measures the buffering instead and is
    /// withheld.
    fn clock_error(&self) -> String {
        if !self.hw_paced {
            return "clock: not measurable over a network link".to_string();
        }
        let dt = self.first_iq.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
        // Below ~20 s the host's own scheduling jitter dominates.
        if dt < 20.0 || self.total_samples == 0 || self.nominal_hz <= 0.0 {
            return "clock: measuring".to_string();
        }
        let measured = self.total_samples as f64 / dt;
        let ppm = (measured / self.nominal_hz - 1.0) * 1e6;
        format!("clock: {measured:.0} sps, {ppm:+.1} ppm — set this as the ppm correction")
    }

    pub(crate) fn tick(&mut self) {
        let dt = self.since.elapsed();
        if dt < STATS_INTERVAL {
            return;
        }
        let ksps = self.win_samples as f64 / dt.as_secs_f64() / 1000.0;
        if self.win_dropped > 0 || self.win_errors > 0 {
            tracing::warn!(
                "RTL-SDR RX: {} samples ({ksps:.1} ksps) over {:.2}s; \
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
        } else {
            tracing::debug!(
                "RTL-SDR RX: {} samples ({ksps:.1} ksps) over {:.2}s; total {}; {}{}",
                self.win_samples,
                dt.as_secs_f64(),
                self.total_samples,
                self.clock_error(),
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
/// Pushing what fits would leave the ring one float out of step, swapping I
/// with Q for the rest of the session — a mirrored, unusable spectrum that
/// reads like a driver bug rather than the overrun it is.
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
    /// Achieved sample rate, published once the device is configured.
    pub rate_milli_hz: AtomicU64,
    /// Gain actually in effect, in tenths of a dB, or -1 while the tuner's own
    /// AGC is driving it. Published by the stream thread after every change so
    /// the UI can settle on a value the hardware can really produce rather than
    /// on the one that was asked for.
    pub effective_gain_tenth_db: AtomicI64,
    /// Set while the engine is transmitting and therefore not reading this
    /// receiver — see `IqSource::set_rx_paused`. Read by the stream thread on
    /// every block so a ring that fills during an over is accounted for as the
    /// cost of transmitting rather than as an overrun.
    pub rx_paused: AtomicBool,
}

/// An open RTL-SDR dongle.
pub struct RtlSdrHandle {
    rx: Consumer<f32>,
    ctrl: Sender<Ctrl>,
    shared: Arc<Shared>,
    opened_at: Instant,
    join: Option<JoinHandle<()>>,
    /// Description for logs and the UI, filled in by the thread at open time.
    pub label: String,
    pub tuner: String,
    pub is_blog_v4: bool,
    pub hf_capable: bool,
    pub sample_rate_hz: f64,
    /// Gain values the tuner can actually produce, in dB.
    pub gains_db: Vec<f64>,
}

impl RtlSdrHandle {
    /// Tell the stream thread that the engine has stopped reading for an over,
    /// and then that it has started again — see `IqSource::set_rx_paused`. The
    /// receiver itself is left running: the samples keep arriving and keep
    /// being offered, this only decides whether the ones that no longer fit are
    /// reported as a fault.
    pub fn set_rx_paused(&self, paused: bool) {
        self.shared.rx_paused.store(paused, Ordering::Relaxed);
    }

    /// Open a dongle and start streaming.
    ///
    /// The device is opened and configured on the stream thread, not here, so
    /// that every register access in the process happens on one thread — see
    /// the invariant in [`crate::usb`]. This call blocks until that has either
    /// succeeded or failed.
    pub fn open(cfg: &RtlSdrConfig, center_hz: f64) -> Result<RtlSdrHandle> {
        // Refused before the stream thread takes the dongle: the range check
        // is the first thing `Device::set_center` does, and by then the USB
        // interface has been claimed and the tuner initialised for nothing.
        // Each refused open of an unreachable start frequency paid that cost
        // again, which is part of the delay before a radio came up (#72).
        crate::device::check_tunable_hz(center_hz)?;
        crate::stream::spawn(cfg, center_hz)
    }

    /// Connect to a dongle published by an `rtl_tcp` server, and start
    /// streaming.
    ///
    /// The same handle as [`Self::open`], because nothing in it was ever
    /// specific to USB: the control setters below become five-byte commands
    /// on a socket, and the samples arrive through the same ring. Blocks until
    /// the connection is up and the far end configured, or has failed.
    ///
    /// What differs is what can be *known*. The protocol carries no replies,
    /// so [`Self::effective_gain_db`] reports the gain that was asked for
    /// rather than one snapped to a step the far end has, [`Self::gains_db`]
    /// is empty because the values are never sent, and the sample rate is the
    /// requested one.
    pub fn connect(cfg: &sdroxide_types::RtlTcpConfig, center_hz: f64) -> Result<RtlSdrHandle> {
        crate::tcp::spawn(cfg, center_hz)
    }

    pub(crate) fn from_parts(
        rx: Consumer<f32>,
        ctrl: Sender<Ctrl>,
        shared: Arc<Shared>,
        join: JoinHandle<()>,
        label: String,
        tuner: String,
        is_blog_v4: bool,
        hf_capable: bool,
        sample_rate_hz: f64,
        gains_db: Vec<f64>,
    ) -> RtlSdrHandle {
        RtlSdrHandle {
            rx,
            ctrl,
            shared,
            opened_at: Instant::now(),
            join: Some(join),
            label,
            tuner,
            is_blog_v4,
            hf_capable,
            sample_rate_hz,
            gains_db,
        }
    }

    /// Whether the stream thread is still running.
    pub fn is_alive(&self) -> bool {
        self.shared.alive.load(Ordering::Relaxed)
    }

    /// How long the dongle has gone without delivering samples, measured from
    /// the last block or — if none ever arrived — from when it was opened.
    ///
    /// A stream that never starts matters as much as one that stops, so it has
    /// to age the same way.
    pub fn silent_for(&self) -> Duration {
        let since_open = self.opened_at.elapsed();
        let last = Duration::from_millis(self.shared.last_rx_ms.load(Ordering::Relaxed));
        since_open.saturating_sub(last)
    }

    /// The gain the tuner actually settled on, in dB. `None` while its own AGC
    /// is in charge, in which case there is no single figure to report.
    pub fn effective_gain_db(&self) -> Option<f64> {
        match self.shared.effective_gain_tenth_db.load(Ordering::Relaxed) {
            n if n < 0 => None,
            n => Some(n as f64 / 10.0),
        }
    }

    /// Drain interleaved I,Q floats into `out`. Always returns an even count,
    /// so the stream can never come out of alignment. Zero means nothing is
    /// available yet.
    ///
    /// One chunk, not a value at a time. `Consumer::pop` publishes the read
    /// index with a release store on **every element**, so draining a block of
    /// sixteen thousand complex samples was thirty-two thousand stores to a
    /// cache line the producer thread also holds — for what is otherwise a
    /// `memcpy`. Measured at 0.75 % of everything the program was doing on an
    /// RTL-SDR at 2.4 Msps, which is a tenth of the DSP thread that the
    /// overrun message blames. Same shape as the RX-888 backend's drain.
    pub fn rx_read(&mut self, out: &mut [f32]) -> usize {
        let n = self.rx.slots().min(out.len()) & !1;
        if n == 0 {
            return 0;
        }
        match self.rx.read_chunk(n) {
            Ok(chunk) => {
                // Two slices where the ring wraps, one where it does not.
                let (a, b) = chunk.as_slices();
                out[..a.len()].copy_from_slice(a);
                out[a.len()..a.len() + b.len()].copy_from_slice(b);
                chunk.commit_all();
                n
            }
            Err(_) => 0,
        }
    }

    fn send(&self, c: Ctrl) {
        // A closed channel means the thread has exited; `needs_reopen` will
        // pick that up from `is_alive`, so there is nothing useful to do here.
        let _ = self.ctrl.send(c);
    }

    pub fn set_center_hz(&self, hz: f64) {
        self.send(Ctrl::Center(hz));
    }

    pub fn set_gain_db(&self, db: f64) {
        self.send(Ctrl::Gain(db));
    }

    pub fn set_agc(&self, agc: RtlSdrAgc) {
        self.send(Ctrl::Agc(agc));
    }

    pub fn set_hf_mode(&self, mode: RtlSdrHfMode) {
        self.send(Ctrl::HfMode(mode));
    }

    pub fn set_ppm(&self, ppm: i32) {
        self.send(Ctrl::Ppm(ppm));
    }

    pub fn set_bias_tee(&self, on: bool) {
        self.send(Ctrl::BiasTee(on));
    }

    /// Send one `rsp_tcp` control.
    ///
    /// Only meaningful against an SDRplay server; on a plain `rtl_tcp` one the
    /// opcode is unknown and is ignored in silence, which is what this protocol
    /// does with everything it does not recognise. The settings tab is what
    /// keeps these out of reach unless a capability block arrived.
    pub fn set_rsp(&self, cmd: crate::tcp::proto::RspCmd, arg: u32) {
        self.send(Ctrl::Rsp(cmd, arg));
    }

    /// Stop the stream thread and let the dongle go, without dropping the
    /// handle.
    ///
    /// The engine needs this before it can build a replacement front-end: the
    /// USB interface is claimed exclusively and a second claim is refused even
    /// from this same process, so a dongle that has not let go is a dongle that
    /// cannot be reopened. Blocks until the thread has closed the device.
    ///
    /// Afterwards the handle is inert rather than invalid: [`Self::rx_read`]
    /// drains what is left in the ring and then returns nothing, control
    /// messages go nowhere, and [`Self::is_alive`] is false — which is what
    /// makes `RtlSdrSource::needs_reopen` true. Idempotent.
    pub fn release(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for RtlSdrHandle {
    fn drop(&mut self) {
        self.release();
    }
}

/// Size the RX ring for a sample rate — half a second of interleaved floats,
/// rounded up to a power of two. Same formula as the HPSDR and TCI backends.
pub(crate) fn ring_for(rate_hz: f64) -> (Producer<f32>, Consumer<f32>) {
    let cap = ((rate_hz * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
    RingBuffer::<f32>::new(cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_keeps_only_the_last_value_per_field() {
        let mut p = Pending::default();
        // A panadapter drag: hundreds of centres, one retune.
        for hz in [100e6, 100.1e6, 100.2e6, 100.3e6] {
            p.absorb(Ctrl::Center(hz));
        }
        p.absorb(Ctrl::Gain(20.0));
        p.absorb(Ctrl::Gain(30.0));
        assert_eq!(p.center, Some(100.3e6));
        assert_eq!(p.gain, Some(30.0));
        assert!(!p.shutdown);
    }

    #[test]
    fn pending_shutdown_is_sticky() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Shutdown);
        p.absorb(Ctrl::Center(100e6));
        assert!(p.shutdown, "a later message must not cancel a shutdown");
    }

    /// The clock estimate is a property of a hardware-paced link, and saying
    /// so is the whole point: over a socket the same arithmetic measures the
    /// buffering and lands thousands of ppm out.
    #[test]
    fn the_clock_estimate_is_withheld_over_a_network_link() {
        let mut net = RxStats::network(1_024_000.0);
        net.on_iq(1_024_000);
        assert!(net.clock_error().contains("not measurable"), "{}", net.clock_error());

        let mut usb = RxStats::new(2_400_000.0);
        usb.on_iq(2_400_000);
        // Hardware-paced, but not for long enough yet — which is a different
        // answer from "never".
        assert_eq!(usb.clock_error(), "clock: measuring");
    }

    #[test]
    fn empty_pending_is_detected() {
        let mut p = Pending::default();
        assert!(p.is_empty());
        p.absorb(Ctrl::BiasTee(false));
        assert!(!p.is_empty(), "even a false bias-tee request is a request");
    }

    /// The ring must hold an even number of floats, or the very first wrap
    /// would swap I and Q.
    #[test]
    fn ring_capacity_is_even_and_at_least_half_a_second() {
        for rate in [250_000.0, 1_536_000.0, 2_400_000.0, 3_200_000.0] {
            let (p, _c) = ring_for(rate);
            let cap = p.buffer().capacity();
            assert_eq!(cap % 2, 0, "odd ring capacity at {rate}");
            assert!(cap as f64 >= rate, "ring holds {cap} floats, less than 0.5 s of {rate} sps");
        }
    }

    #[test]
    fn push_iq_drops_whole_blocks_rather_than_splitting_pairs() {
        let (mut prod, cons) = RingBuffer::<f32>::new(8);
        let mut stats = RxStats::new(1000.0);

        // Fits.
        push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0], &mut stats, false);
        assert_eq!(cons.slots(), 4);

        // Does not fit — must be dropped whole, leaving the ring aligned.
        push_iq(&mut prod, &[5.0, 6.0, 7.0, 8.0, 9.0, 10.0], &mut stats, false);
        assert_eq!(cons.slots(), 4, "a partial write would desynchronise I and Q");
        assert_eq!(stats.total_dropped, 3, "three complex samples were dropped");

        // The ring still holds an even count, so pairs are intact.
        assert_eq!(cons.slots() % 2, 0);
    }

    /// A ring that fills because the engine stopped reading for an over is not
    /// the DSP thread falling behind, and must not reach the fault counters:
    /// that is what turned a healthy station into a warning per two seconds of
    /// transmit and a running total that only measured time on the air.
    #[test]
    fn a_full_ring_while_paused_is_not_an_overrun() {
        let (mut prod, mut cons) = RingBuffer::<f32>::new(4);
        let mut stats = RxStats::new(1000.0);

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
