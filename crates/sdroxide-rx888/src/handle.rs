//! The handle the rest of the program holds, and the accounting behind it.
//!
//! Same shape as the RTL-SDR and HPSDR backends: blocking threads own the
//! device, control goes in over a crossbeam channel, samples come back out
//! through an `rtrb` ring of interleaved `f32`.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use rtrb::{Consumer, Producer, RingBuffer};

/// How often the stream thread emits a throughput line.
const STATS_INTERVAL: Duration = Duration::from_secs(5);

/// A control message for the stream thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ctrl {
    Center(f64),
    Vga(f64),
    VgaRange(crate::device::VgaRange),
    Att(f64),
    Dither(bool),
    BiasTee(bool),
    Pga(bool),
    TunerGain(f64),
    TunerAgc(bool),
    BiasTeeVhf(bool),
    Shutdown,
}

/// Control messages accumulated over one pass of the thread loop.
///
/// Dragging the panadapter emits hundreds of `Center` messages a second. On HF
/// retuning is nearly free — a change of FFT bin, not an I2C transaction — but
/// coalescing still matters, because applying a hundred retunes per pass would
/// reset the downconverter's block phase a hundred times.
///
/// Above the VHF crossover it stops being free at all: a large jump reprograms
/// the tuner's PLL and waits for lock. So this is load-bearing rather than
/// merely tidy, and `stream::pump` paces the hardware half on top of it. Last
/// value wins, which is the right semantics for a dial.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Pending {
    pub center: Option<f64>,
    pub vga: Option<f64>,
    pub vga_range: Option<crate::device::VgaRange>,
    pub att: Option<f64>,
    pub dither: Option<bool>,
    pub bias_tee: Option<bool>,
    pub pga: Option<bool>,
    pub tuner_gain: Option<f64>,
    pub tuner_agc: Option<bool>,
    pub bias_tee_vhf: Option<bool>,
    pub shutdown: bool,
}

impl Pending {
    pub(crate) fn absorb(&mut self, c: Ctrl) {
        match c {
            Ctrl::Center(v) => self.center = Some(v),
            Ctrl::Vga(v) => self.vga = Some(v),
            Ctrl::VgaRange(v) => self.vga_range = Some(v),
            Ctrl::Att(v) => self.att = Some(v),
            Ctrl::Dither(v) => self.dither = Some(v),
            Ctrl::BiasTee(v) => self.bias_tee = Some(v),
            Ctrl::Pga(v) => self.pga = Some(v),
            Ctrl::TunerGain(v) => self.tuner_gain = Some(v),
            Ctrl::TunerAgc(v) => self.tuner_agc = Some(v),
            Ctrl::BiasTeeVhf(v) => self.bias_tee_vhf = Some(v),
            Ctrl::Shutdown => self.shutdown = true,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        *self == Pending::default()
    }
}

/// State the stream thread publishes for the handle to read.
pub(crate) struct Shared {
    pub alive: AtomicBool,
    pub last_rx_ms: AtomicU64,
    /// Achieved output rate, in millihertz, so the adapter can report what the
    /// downconverter actually produces rather than what was asked for.
    pub out_rate_milli_hz: AtomicU64,
    /// Gains the hardware settled on, in tenths of a dB; `i64::MIN` for unset.
    pub vga_tenth_db: AtomicI64,
    pub att_tenth_db: AtomicI64,
    /// Samples the ring could not take because the consumer fell behind.
    pub dropped: AtomicU64,
    /// Latest full-band spectrum frame, with the axis it belongs on.
    ///
    /// Latest-wins rather than queued: a display that falls behind wants the
    /// newest picture, not a backlog of stale ones. A mutex is fine here — it is
    /// taken twenty times a second, not per sample.
    pub wide: Mutex<Option<WideFrame>>,
    /// Set while the engine is transmitting and therefore not reading this
    /// receiver — see `IqSource::set_rx_paused`. Read by the stream thread on
    /// every block so a ring that fills during an over is accounted for as the
    /// cost of transmitting rather than as an overrun.
    pub rx_paused: AtomicBool,
}

/// A full-band frame and the axis it sits on.
///
/// The axis travels with the bins rather than being asked for separately: on
/// VHF it moves with the tuner, and a frame captured before a band change must
/// not be drawn on the axis that came after it.
pub struct WideFrame {
    /// dBFS, ascending in RF.
    pub bins: Vec<f32>,
    pub center_hz: f64,
    pub span_hz: f64,
}

/// Throughput and health accounting.
pub(crate) struct RxStats {
    nominal_hz: f64,
    since: Instant,
    win_samples: u64,
    win_dropped: u64,
    /// Discarded while the engine was not reading this receiver because the
    /// station was transmitting. Counted apart from `win_dropped` because it
    /// is not a fault: see [`RxStats::on_dropped_keyed`].
    win_keyed: u64,
}

impl RxStats {
    pub(crate) fn new(nominal_hz: f64) -> RxStats {
        RxStats { nominal_hz, since: Instant::now(), win_samples: 0, win_dropped: 0, win_keyed: 0 }
    }

    pub(crate) fn on_iq(&mut self, samples: usize) {
        self.win_samples += samples as u64;
    }

    pub(crate) fn on_dropped(&mut self, samples: usize) {
        self.win_dropped += samples as u64;
    }

    /// Record `samples` complex samples discarded because the ring was full
    /// while the station was transmitting.
    ///
    /// This receiver does not transmit, so this is the case where it is
    /// somebody else's panadapter: the engine stops reading a half-duplex
    /// source for the length of an over and empties the ring on unkey, while
    /// this one carries on streaming throughout. The ring fills within its own
    /// depth of key-down and everything after that is discarded — expected, at
    /// exactly the sample rate, for as long as the operator transmits. Counting
    /// it as an overrun turns an ordinary over into a warning that blames the
    /// DSP thread. See `IqSource::set_rx_paused`.
    pub(crate) fn on_dropped_keyed(&mut self, samples: usize) {
        self.win_keyed += samples as u64;
    }

    /// What this receiver threw away while the station was transmitting,
    /// phrased so it cannot be read as a fault. Empty when the operator did not
    /// key up, so it costs nothing on a receive-only session.
    fn keyed_note(&self) -> String {
        if self.win_keyed == 0 {
            return String::new();
        }
        format!(
            ", {} discarded while keyed (expected — this receiver is not read during \
             an over)",
            self.win_keyed,
        )
    }

    /// Emit a periodic line and reset the window.
    pub(crate) fn tick(&mut self) {
        let elapsed = self.since.elapsed();
        if elapsed < STATS_INTERVAL {
            return;
        }
        let secs = elapsed.as_secs_f64();
        let rate = self.win_samples as f64 / secs;
        if self.win_dropped > 0 {
            tracing::warn!(
                "RX-888: {:.3} Msps out ({:.1}% of nominal), {} samples dropped{}",
                rate / 1e6,
                100.0 * rate / self.nominal_hz,
                self.win_dropped,
                self.keyed_note(),
            );
        } else {
            tracing::debug!(
                "RX-888: {:.3} Msps out ({:.1}% of nominal){}",
                rate / 1e6,
                100.0 * rate / self.nominal_hz,
                self.keyed_note(),
            );
        }
        self.since = Instant::now();
        self.win_samples = 0;
        self.win_dropped = 0;
        self.win_keyed = 0;
    }
}

/// Push interleaved complex samples into the ring, counting what will not fit.
///
/// Dropping is the right failure here: the alternative is blocking the thread
/// that is servicing USB, which turns a brief consumer stall into a hardware
/// overrun and loses far more than the samples that would not fit.
pub(crate) fn push_iq(
    ring: &mut Producer<f32>,
    data: &[f32],
    stats: &mut RxStats,
    paused: bool,
) -> usize {
    let mut written = 0;
    // Rounded down to a whole number of I/Q pairs. `slots()` is a count of
    // floats and is under no obligation to be even, so taking it at face value
    // would eventually commit a lone I with its Q dropped — and from then on
    // every pair in the ring is one float out of step, which swaps I with Q for
    // the rest of the session and mirrors the spectrum. That reads like a
    // driver bug rather than the overrun it is.
    let room = ring.slots() & !1;
    if let Ok(mut chunk) = ring.write_chunk_uninit(data.len().min(room)) {
        let (a, b) = chunk.as_mut_slices();
        let split = a.len();
        for (dst, src) in a.iter_mut().zip(&data[..split.min(data.len())]) {
            dst.write(*src);
        }
        if data.len() > split {
            for (dst, src) in b.iter_mut().zip(&data[split..]) {
                dst.write(*src);
            }
        }
        written = a.len() + b.len();
        unsafe { chunk.commit_all() };
    }
    stats.on_iq(written / 2);
    let short = (data.len() - written) / 2;
    if short == 0 {
        return 0;
    }
    // While the engine is transmitting it is not reading this receiver at all,
    // so a full ring is the ordinary cost of an over rather than a host that
    // fell behind — and the caller's fault counter must not collect it either.
    // See `IqSource::set_rx_paused`.
    if paused {
        stats.on_dropped_keyed(short);
        return 0;
    }
    stats.on_dropped(short);
    short
}

/// Ring sized for half a second of interleaved `f32` at `rate`, rounded up to a
/// power of two.
pub(crate) fn ring_for(rate: f64) -> (Producer<f32>, Consumer<f32>) {
    let slots = ((rate * 2.0 * 0.5) as usize).next_power_of_two().clamp(1 << 14, 1 << 24);
    RingBuffer::new(slots)
}

/// A running RX-888.
pub struct Rx888Handle {
    rx: Consumer<f32>,
    ctrl: Sender<Ctrl>,
    shared: Arc<Shared>,
    join: Option<JoinHandle<()>>,
    label: String,
    serial: Option<String>,
    adc_rate_hz: f64,
    out_rate_hz: f64,
    bin_hz: f64,
    warning: Option<String>,
    /// Whether this receiver has an R828D and an ADC clock with room for its
    /// IF. Fixed at open: neither can change without a re-open.
    vhf_capable: bool,
    opened_at: Instant,
    released: bool,
}

impl Rx888Handle {
    /// Tell the stream thread that the engine has stopped reading for an over,
    /// and then that it has started again — see `IqSource::set_rx_paused`. The
    /// receiver itself is left running: the samples keep arriving and keep
    /// being offered, this only decides whether the ones that no longer fit are
    /// reported as a fault.
    pub fn set_rx_paused(&self, paused: bool) {
        self.shared.rx_paused.store(paused, Ordering::Relaxed);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        rx: Consumer<f32>,
        ctrl: Sender<Ctrl>,
        shared: Arc<Shared>,
        join: JoinHandle<()>,
        label: String,
        serial: Option<String>,
        adc_rate_hz: f64,
        out_rate_hz: f64,
        bin_hz: f64,
        warning: Option<String>,
        vhf_capable: bool,
    ) -> Rx888Handle {
        Rx888Handle {
            rx,
            ctrl,
            shared,
            join: Some(join),
            label,
            serial,
            adc_rate_hz,
            out_rate_hz,
            bin_hz,
            warning,
            vhf_capable,
            opened_at: Instant::now(),
            released: false,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn serial(&self) -> Option<&str> {
        self.serial.as_deref()
    }

    /// The ADC clock, i.e. the real-sample rate on the wire.
    pub fn adc_rate_hz(&self) -> f64 {
        self.adc_rate_hz
    }

    /// The complex baseband rate delivered to the caller.
    pub fn out_rate_hz(&self) -> f64 {
        let milli = self.shared.out_rate_milli_hz.load(Ordering::Relaxed);
        if milli > 0 { milli as f64 / 1000.0 } else { self.out_rate_hz }
    }

    /// Coarse tuning grid, in Hz.
    pub fn bin_hz(&self) -> f64 {
        self.bin_hz
    }

    /// Centre and span the full-band spectrum covers while the receiver is on
    /// HF: a real ADC stream runs DC to Nyquist, so a quarter and a half of the
    /// ADC clock.
    ///
    /// Only true on HF, which is why it is no longer what the source reports —
    /// above the crossover the analyser is looking at the tuner's IF and the
    /// axis moves with the tuner. Each frame carries its own; see
    /// [`Rx888Handle::take_wide_spectrum`]. Kept for the bring-up examples,
    /// which only ever run on HF.
    pub fn wide_span_hz(&self) -> (f64, f64) {
        (self.adc_rate_hz / 4.0, self.adc_rate_hz / 2.0)
    }

    /// Take the newest full-band spectrum frame, if one is waiting, with the
    /// axis it belongs on.
    pub fn take_wide_spectrum(&self) -> Option<WideFrame> {
        self.shared.wide.lock().ok().and_then(|mut g| g.take())
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    /// Whether the stream thread is still running.
    pub fn is_alive(&self) -> bool {
        !self.released && self.shared.alive.load(Ordering::Relaxed)
    }

    /// Complex samples the ring could not take because the consumer fell
    /// behind. Non-zero here means the DSP thread is not keeping up, which is a
    /// different fault from the converter falling behind the USB endpoint.
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    /// How long the receiver has delivered nothing.
    ///
    /// `last_rx_ms` is a *timestamp* measured from when the stream thread
    /// started, not an age, so it has to be subtracted from the elapsed time
    /// rather than used directly — reading it as an age reports a perfectly
    /// healthy receiver as dead a few seconds after it starts.
    pub fn silent_for(&self) -> Duration {
        let since_open = self.opened_at.elapsed();
        let last = Duration::from_millis(self.shared.last_rx_ms.load(Ordering::Relaxed));
        since_open.saturating_sub(last)
    }

    /// The gain the hardware settled on, if it has reported one yet.
    pub fn effective_vga_db(&self) -> Option<f64> {
        match self.shared.vga_tenth_db.load(Ordering::Relaxed) {
            i64::MIN => None,
            v => Some(v as f64 / 10.0),
        }
    }

    pub fn effective_att_db(&self) -> Option<f64> {
        match self.shared.att_tenth_db.load(Ordering::Relaxed) {
            i64::MIN => None,
            v => Some(v as f64 / 10.0),
        }
    }

    /// Read interleaved complex samples. Returns how many `f32` were taken.
    pub fn read(&mut self, out: &mut [f32]) -> usize {
        let n = out.len().min(self.rx.slots());
        // Keep I and Q together: an odd count would swap them for good.
        let n = n - (n % 2);
        if n == 0 {
            return 0;
        }
        match self.rx.read_chunk(n) {
            Ok(chunk) => {
                let (a, b) = chunk.as_slices();
                out[..a.len()].copy_from_slice(a);
                out[a.len()..a.len() + b.len()].copy_from_slice(b);
                chunk.commit_all();
                n
            }
            Err(_) => 0,
        }
    }

    pub fn set_center_hz(&self, hz: f64) {
        let _ = self.ctrl.send(Ctrl::Center(hz));
    }

    pub fn set_vga_db(&self, db: f64) {
        let _ = self.ctrl.send(Ctrl::Vga(db));
    }

    pub fn set_att_db(&self, db: f64) {
        let _ = self.ctrl.send(Ctrl::Att(db));
    }

    /// Choose the VGA's range — see [`crate::device::VgaRange`].
    pub fn set_vga_range(&self, range: crate::device::VgaRange) {
        let _ = self.ctrl.send(Ctrl::VgaRange(range));
    }

    pub fn set_dither(&self, on: bool) {
        let _ = self.ctrl.send(Ctrl::Dither(on));
    }

    pub fn set_bias_tee(&self, on: bool) {
        let _ = self.ctrl.send(Ctrl::BiasTee(on));
    }

    pub fn set_pga(&self, on: bool) {
        let _ = self.ctrl.send(Ctrl::Pga(on));
    }

    /// R828D RF gain, in dB. Has no effect below the VHF crossover.
    pub fn set_tuner_gain_db(&self, db: f64) {
        let _ = self.ctrl.send(Ctrl::TunerGain(db));
    }

    /// Let the tuner run its own LNA and mixer loops.
    pub fn set_tuner_agc(&self, on: bool) {
        let _ = self.ctrl.send(Ctrl::TunerAgc(on));
    }

    /// Power the VHF antenna port.
    pub fn set_bias_tee_vhf(&self, on: bool) {
        let _ = self.ctrl.send(Ctrl::BiasTeeVhf(on));
    }

    /// Whether this receiver has a usable VHF front end.
    pub fn vhf_capable(&self) -> bool {
        self.vhf_capable
    }

    /// Stop the thread and let go of the device.
    ///
    /// Idempotent, and leaves the handle callable but inert — the engine's
    /// reopen path requires exactly that, because a replacement is built only
    /// after the outgoing source has stood down.
    pub fn release(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        self.shared.alive.store(false, Ordering::Relaxed);
    }
}

impl Drop for Rx888Handle {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_keeps_only_the_last_value_per_field() {
        let mut p = Pending::default();
        assert!(p.is_empty());
        p.absorb(Ctrl::Center(1.0));
        p.absorb(Ctrl::Center(2.0));
        p.absorb(Ctrl::Vga(10.0));
        assert_eq!(p.center, Some(2.0));
        assert_eq!(p.vga, Some(10.0));
        assert_eq!(p.att, None);
        assert!(!p.is_empty());
    }

    #[test]
    fn shutdown_survives_later_messages() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Shutdown);
        p.absorb(Ctrl::Center(1.0));
        assert!(p.shutdown, "a shutdown must not be forgotten by a later retune");
    }

    #[test]
    fn the_ring_holds_about_half_a_second() {
        let (p, _c) = ring_for(2_025_000.0);
        // Half a second of interleaved f32 is 2.025 M slots; the next power of
        // two is 4 Mi, and it must be comfortably more than a single block.
        assert!(p.slots() >= 2_025_000);
        assert!(p.slots().is_power_of_two());
    }

    #[test]
    fn the_ring_is_clamped_for_absurd_rates() {
        let (p, _c) = ring_for(1e12);
        assert!(p.slots() <= 1 << 24);
        let (p, _c) = ring_for(1.0);
        assert!(p.slots() >= 1 << 14);
    }

    #[test]
    fn silence_is_an_age_not_a_timestamp() {
        let (_p, cons) = RingBuffer::<f32>::new(16);
        let (tx, _rx) = crossbeam_channel::unbounded();
        let shared = Arc::new(Shared {
            alive: AtomicBool::new(true),
            last_rx_ms: AtomicU64::new(0),
            out_rate_milli_hz: AtomicU64::new(0),
            vga_tenth_db: AtomicI64::new(i64::MIN),
            att_tenth_db: AtomicI64::new(i64::MIN),
            dropped: AtomicU64::new(0),
            wide: Mutex::new(None),
            rx_paused: AtomicBool::new(false),
        });
        let join = std::thread::spawn(|| {});
        let mut h = Rx888Handle::from_parts(
            cons,
            tx,
            shared,
            join,
            "test".into(),
            None,
            64.8e6,
            2.025e6,
            7910.0,
            None,
            false,
        );
        // A receiver that delivered a sample "just now" has near-zero silence,
        // however long the handle has been open. Reading the raw timestamp
        // instead would grow without bound and trip the reopen threshold.
        let age = h.opened_at.elapsed();
        h.shared.last_rx_ms.store(age.as_millis() as u64, Ordering::Relaxed);
        assert!(h.silent_for() < Duration::from_millis(50), "{:?}", h.silent_for());
        h.released = true;
    }

    #[test]
    fn reads_never_split_an_iq_pair() {
        let (mut prod, cons) = RingBuffer::<f32>::new(64);
        for i in 0..9 {
            prod.push(i as f32).unwrap();
        }
        let (tx, _rx) = crossbeam_channel::unbounded();
        let shared = Arc::new(Shared {
            alive: AtomicBool::new(true),
            last_rx_ms: AtomicU64::new(0),
            out_rate_milli_hz: AtomicU64::new(0),
            vga_tenth_db: AtomicI64::new(i64::MIN),
            att_tenth_db: AtomicI64::new(i64::MIN),
            dropped: AtomicU64::new(0),
            wide: Mutex::new(None),
            rx_paused: AtomicBool::new(false),
        });
        let join = std::thread::spawn(|| {});
        let mut h = Rx888Handle::from_parts(
            cons,
            tx,
            shared,
            join,
            "test".into(),
            None,
            64.8e6,
            2.025e6,
            7910.0,
            None,
            false,
        );
        let mut buf = [0f32; 32];
        // Nine samples are available; an even count must come back.
        let n = h.read(&mut buf);
        assert_eq!(n % 2, 0, "read {n} f32, which splits an I/Q pair");
        assert_eq!(n, 8);
        h.released = true; // do not try to join in Drop
    }

    /// The partial writer must never commit a lone I with its Q left behind.
    /// `slots()` is a count of floats and is under no obligation to be even, so
    /// taking it at face value eventually puts the ring one float out of step —
    /// and from then on every pair is swapped and the spectrum is mirrored for
    /// the rest of the session.
    #[test]
    fn a_partial_write_still_lands_on_a_pair_boundary() {
        let (mut prod, cons) = RingBuffer::<f32>::new(8);
        let mut stats = RxStats::new(64.0e6);
        // Leave an odd three slots free, then offer four floats.
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            prod.push(v).expect("room");
        }
        let dropped = push_iq(&mut prod, &[6.0, 7.0, 8.0, 9.0], &mut stats, false);
        assert_eq!(cons.slots(), 7, "only one whole pair fitted the three free slots");
        assert_eq!(dropped, 1, "the pair that did not fit is one complex sample");
    }

    /// A ring that fills because the engine stopped reading for an over is not
    /// the DSP thread falling behind, and must not reach the fault counters —
    /// nor the count this hands back for the handle's `dropped()` total.
    #[test]
    fn a_full_ring_while_paused_is_not_an_overrun() {
        let (mut prod, cons) = RingBuffer::<f32>::new(4);
        let mut stats = RxStats::new(64.0e6);

        push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0], &mut stats, true);
        let dropped = push_iq(&mut prod, &[5.0, 6.0], &mut stats, true);
        assert_eq!(dropped, 0, "a paused receiver reports no overruns to its caller");
        assert_eq!(stats.win_dropped, 0);
        assert_eq!(stats.win_keyed, 1, "the discarded pair is accounted for as keyed");

        // Unpaused, the very same full ring is a genuine overrun again.
        let dropped = push_iq(&mut prod, &[7.0, 8.0], &mut stats, false);
        assert_eq!(dropped, 1);
        assert_eq!(stats.win_dropped, 1);
        assert_eq!(stats.win_keyed, 1);
        assert_eq!(cons.slots() % 2, 0);
    }
}
