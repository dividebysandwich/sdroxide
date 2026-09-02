//! The handles the rest of the program holds, and the accounting behind them.
//!
//! Same shape as the RTL-SDR and RX-888 backends: a control thread owns the
//! device session, control goes in over a crossbeam channel, samples come
//! back out through an `rtrb` ring of interleaved `f32`. The one structural
//! difference is that samples are not pulled off a USB endpoint here — the
//! vendor service pushes them into a callback on a thread it owns.
//!
//! # One device, one or two streams
//!
//! An RSPduo running both tuners is one API session with two of everything
//! downstream of the ADC, and the two can be separate radios on separate
//! frequencies (issue #165). So the session and the stream are two objects:
//! [`SdrPlayDevice`] owns the thread and the device, and each
//! [`SdrPlayHandle`] is one tuner's stream on it — its own ring, its own
//! dial, its own gains. The device stays open while any stream holds it and
//! closes when the last one goes, which is what lets one radio be reopened
//! (Settings → Apply) without disturbing the other.
//!
//! Every control therefore names a tuner. The exception is the handful the
//! hardware has one of — the reference trim, and on an RSPdx the HDR path —
//! which are marked where they appear.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use rtrb::{Consumer, Producer, RingBuffer};
use sdroxide_types::SdrPlayModel;

use crate::error::{Error, Result};

/// How often the callback emits a throughput line.
const STATS_INTERVAL: Duration = Duration::from_secs(5);

/// Which tuner a control is for: an index into the per-tuner arrays, 0 for
/// tuner 1 (the API's tuner A) and 1 for tuner 2 (tuner B).
pub(crate) type TunerIdx = usize;

/// How many tuners any RSP has.
pub(crate) const TUNERS: usize = 2;

/// A control message for the session thread. Everything that belongs to one
/// tuner carries its index; the rest is the device's.
#[derive(Debug)]
pub(crate) enum Ctrl {
    /// Start delivering this tuner's samples into `ring`, from `center_hz`.
    Attach {
        tuner: TunerIdx,
        ring: Producer<f32>,
        center_hz: f64,
    },
    /// Stop delivering them: the radio that was reading them has gone. The
    /// tuner keeps running — the session may still be serving the other one,
    /// and an RSPduo's tuners are only ever switched on together.
    Detach(TunerIdx),
    Center(TunerIdx, f64),
    /// IF gain reduction in dB (the RSP's native unit, 20..=59).
    IfGr(TunerIdx, i32),
    Lna(TunerIdx, u8),
    /// [`sdroxide_types::SdrPlayAgc::code`] value.
    Agc(TunerIdx, i32),
    AgcSetpoint(TunerIdx, i32),
    BiasTee(TunerIdx, bool),
    RfNotch(TunerIdx, bool),
    DabNotch(TunerIdx, bool),
    Antenna(TunerIdx, String),
    /// Reference trim: the device's own, not a tuner's.
    Ppm(f64),
    /// The RSPdx's high-dynamic-range path, likewise device-wide.
    Hdr(bool),
    /// The HDR path's analog filter, as an `sdrplay_api_RspDx_HdrModeBwT`
    /// code. Device-wide for the same reason as the switch above.
    HdrBw(i32),
    /// Whether the IF gain reduction may go below the API's normal 20 dB
    /// floor. Per tuner, like the reduction it bounds.
    ExtendedIfGr(TunerIdx, bool),
    Shutdown,
}

/// One tuner's share of the control accumulated over a pass of the session
/// loop.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct TunerPending {
    pub center: Option<f64>,
    pub if_gr: Option<i32>,
    pub lna: Option<u8>,
    pub agc: Option<i32>,
    pub agc_setpoint: Option<i32>,
    pub extended_if_gr: Option<bool>,
    pub bias_tee: Option<bool>,
    pub rf_notch: Option<bool>,
    pub dab_notch: Option<bool>,
    pub antenna: Option<String>,
}

/// Control messages accumulated over one pass of the session loop. Dragging
/// the dial or a slider emits far more messages than `sdrplay_api_Update`
/// round-trips per second; last value wins, which is the right semantics for
/// every one of these.
///
/// The stream attachments are the exception and are *not* coalesced: each one
/// carries a ring the callbacks must be given, and a pass that saw two of them
/// has two radios waiting.
#[derive(Debug, Default)]
pub(crate) struct Pending {
    pub t: [TunerPending; TUNERS],
    pub attach: Vec<(TunerIdx, Producer<f32>, f64)>,
    pub detach: [bool; TUNERS],
    pub ppm: Option<f64>,
    pub hdr: Option<bool>,
    pub hdr_bw: Option<i32>,
    pub shutdown: bool,
}

impl Pending {
    pub(crate) fn absorb(&mut self, c: Ctrl) {
        // A message for a tuner this device does not have is dropped rather
        // than panicking an index: the sender is another thread, and a stale
        // handle is not worth a crash.
        let ok = |t: TunerIdx| t < TUNERS;
        match c {
            Ctrl::Attach { tuner, ring, center_hz } if ok(tuner) => {
                self.detach[tuner] = false;
                self.attach.push((tuner, ring, center_hz));
            }
            Ctrl::Detach(t) if ok(t) => {
                self.attach.retain(|(i, _, _)| *i != t);
                self.detach[t] = true;
            }
            Ctrl::Center(t, v) if ok(t) => self.t[t].center = Some(v),
            Ctrl::IfGr(t, v) if ok(t) => self.t[t].if_gr = Some(v),
            Ctrl::Lna(t, v) if ok(t) => self.t[t].lna = Some(v),
            Ctrl::Agc(t, v) if ok(t) => self.t[t].agc = Some(v),
            Ctrl::AgcSetpoint(t, v) if ok(t) => self.t[t].agc_setpoint = Some(v),
            Ctrl::BiasTee(t, v) if ok(t) => self.t[t].bias_tee = Some(v),
            Ctrl::RfNotch(t, v) if ok(t) => self.t[t].rf_notch = Some(v),
            Ctrl::DabNotch(t, v) if ok(t) => self.t[t].dab_notch = Some(v),
            Ctrl::Antenna(t, v) if ok(t) => self.t[t].antenna = Some(v),
            Ctrl::Ppm(v) => self.ppm = Some(v),
            Ctrl::Hdr(v) => self.hdr = Some(v),
            Ctrl::HdrBw(v) => self.hdr_bw = Some(v),
            Ctrl::ExtendedIfGr(t, v) if ok(t) => self.t[t].extended_if_gr = Some(v),
            Ctrl::Shutdown => self.shutdown = true,
            _ => {}
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.t.iter().all(|t| *t == TunerPending::default())
            && self.attach.is_empty()
            && !self.detach.iter().any(|d| *d)
            && self.ppm.is_none()
            && self.hdr.is_none()
            && self.hdr_bw.is_none()
            && !self.shutdown
    }
}

/// What one tuner's callbacks and control publish for its handle.
pub(crate) struct TunerShared {
    /// Timestamp of the newest samples, in ms since the session epoch — a
    /// timestamp, not an age; see [`SdrPlayHandle::silent_for`].
    pub last_rx_ms: AtomicU64,
    /// Samples the ring could not take because the consumer fell behind.
    pub dropped: AtomicU64,
    /// What the GainChange event last reported, `i64::MIN` for "not yet".
    /// Written by the event callback on every gain update — including the
    /// AGC's own — which is what keeps `current_gains()` honest while the
    /// hardware, not the operator, is moving the gain.
    pub ev_gr_db: AtomicI64,
    pub ev_lna_gr_db: AtomicI64,
    /// System gain from the same event, in milli-dB.
    ///
    /// The event announces a gain the samples in flight were *not* taken with:
    /// the service raises it when it programs the tuner, and the USB pipe
    /// behind it is still full of blocks from before. Subtracting this from a
    /// measurement straight away puts a step in the S-meter the antenna never
    /// saw, in the wrong direction, for as long as the pipe takes to drain. So
    /// this is only the announcement; [`Self::rx_curr_gain_milli_db`] is the
    /// one to measure against.
    pub ev_curr_gain_milli_db: AtomicI64,
    /// The system gain the samples now arriving were actually taken with, in
    /// milli-dB, `i64::MIN` for "not yet". This is the one to measure against.
    ///
    /// Promoted from the announcement above by `settle_gain` in the stream
    /// callback, on the block the service flags with `grChanged` or, where it
    /// never flags one, once the announcement has stood long enough that the
    /// pipe cannot still be holding samples from before it. Both paths matter
    /// and the reasoning for each is on that function.
    pub rx_curr_gain_milli_db: AtomicI64,
    /// The announcement `settle_gain` is currently waiting on, and the moment
    /// it arrived (ms since the session epoch). Separate from the two above
    /// because the wait is per *announcement*: a loop still hunting replaces
    /// this every few milliseconds and only the value it settles on ever ages
    /// into place.
    pub pending_gain_milli_db: AtomicI64,
    pub pending_gain_since_ms: AtomicU64,
    /// The LNA state actually programmed, after any per-band clamp.
    pub lna_state: AtomicU8,
    /// Set while the engine reading this tuner is transmitting and therefore
    /// not reading it — see `IqSource::set_rx_paused`. Read by the stream
    /// thread on every block so a ring that fills during an over is accounted
    /// for as the cost of transmitting rather than as an overrun. Per tuner
    /// because the two can be different radios, and only one of them is keyed.
    pub rx_paused: AtomicBool,
}

impl TunerShared {
    fn new() -> TunerShared {
        TunerShared {
            last_rx_ms: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            ev_gr_db: AtomicI64::new(i64::MIN),
            ev_lna_gr_db: AtomicI64::new(i64::MIN),
            ev_curr_gain_milli_db: AtomicI64::new(i64::MIN),
            rx_curr_gain_milli_db: AtomicI64::new(i64::MIN),
            pending_gain_milli_db: AtomicI64::new(i64::MIN),
            pending_gain_since_ms: AtomicU64::new(0),
            lna_state: AtomicU8::new(0),
            rx_paused: AtomicBool::new(false),
        }
    }
}

/// State the session thread and the API's callbacks publish for the handles.
pub(crate) struct Shared {
    pub alive: AtomicBool,
    /// The service said the ADC is overloaded (and has not yet said corrected).
    pub overload: AtomicBool,
    /// Which tuners have an overload message waiting for the mandatory
    /// acknowledgement, which must come from the session thread rather than
    /// the callback: bit 0 is tuner A, bit 1 is tuner B. A mask rather than a
    /// flag because the acknowledgement names a tuner, and one sent for a
    /// tuner that never reported an overload is one the service refuses.
    pub overload_ack_pending: AtomicU8,
    /// The device disappeared mid-session (unplug or service failure).
    pub removed: AtomicBool,
    /// Per tuner, in tuner order: A then B.
    pub t: [TunerShared; TUNERS],
    /// The second tuner has stopped delivering and the first is going through
    /// unpaired — see [`crate::pair`]. Diversity only.
    pub aux_stalled: AtomicBool,
    /// How many times the two tuners had to be paired up again from scratch.
    pub pair_slips: AtomicU64,
    /// Whether the service fills in the sample numbers the pairing prefers, or
    /// the pairing has fallen back to arrival order.
    pub pair_stamped: AtomicBool,
}

impl Shared {
    pub(crate) fn new() -> Shared {
        Shared {
            alive: AtomicBool::new(true),
            overload: AtomicBool::new(false),
            overload_ack_pending: AtomicU8::new(0),
            removed: AtomicBool::new(false),
            t: [TunerShared::new(), TunerShared::new()],
            aux_stalled: AtomicBool::new(false),
            pair_slips: AtomicU64::new(0),
            pair_stamped: AtomicBool::new(true),
        }
    }
}

/// Throughput and health accounting, ticked from the stream callback.
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

    /// The nominal rate this was opened against, so a re-attached stream can
    /// start its accounting again without being told the rate twice.
    pub(crate) fn nominal_hz(&self) -> f64 {
        self.nominal_hz
    }

    /// `named` says the board is running two tuners as separate radios, where
    /// a line that did not say which one would be half the diagnosis.
    pub(crate) fn tick(&mut self, tuner: usize, named: bool) {
        let elapsed = self.since.elapsed();
        if elapsed < STATS_INTERVAL {
            return;
        }
        let who =
            if named { format!("SDRplay tuner {}", tuner + 1) } else { "SDRplay".to_string() };
        let secs = elapsed.as_secs_f64();
        let rate = self.win_samples as f64 / secs;
        if self.win_dropped > 0 {
            tracing::warn!(
                "{who}: {:.3} Msps ({:.1}% of nominal), {} samples dropped{}",
                rate / 1e6,
                100.0 * rate / self.nominal_hz,
                self.win_dropped,
                self.keyed_note(),
            );
        } else {
            tracing::debug!(
                "{who}: {:.3} Msps ({:.1}% of nominal)",
                rate / 1e6,
                100.0 * rate / self.nominal_hz
            );
        }
        self.since = Instant::now();
        self.win_samples = 0;
        self.win_dropped = 0;
        self.win_keyed = 0;
    }
}

/// Push interleaved complex samples into the ring, counting what will not
/// fit. Dropping beats blocking: the thread being served here belongs to the
/// vendor service, and stalling it stalls the hardware.
pub(crate) fn push_iq(
    ring: &mut Producer<f32>,
    data: &[f32],
    stats: &mut RxStats,
    paused: bool,
    stride: usize,
) -> usize {
    let mut written = 0;
    // Rounded down to a whole number of samples — a pair with one tuner
    // running, a quadruple with two. `slots()` is a count of floats and is
    // under no obligation to be a multiple of anything, so taking it at face
    // value would eventually commit a lone I with its Q dropped — and from
    // then on every sample in the ring is one float out of step, which swaps I
    // with Q for the rest of the session and mirrors the spectrum. That reads
    // like a driver bug rather than the overrun it is.
    let free = ring.slots();
    let room = free - free % stride;
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
    stats.on_iq(written / stride);
    let short = (data.len() - written) / stride;
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

/// Ring sized for half a second of interleaved `f32` at `rate`, rounded up to
/// a power of two. `stride` is the floats per sample: two with one tuner
/// running, four with an RSPduo's pair interleaved.
pub(crate) fn ring_for(rate: f64, stride: usize) -> (Producer<f32>, Consumer<f32>) {
    let slots = ((rate * stride as f64 * 0.5) as usize).next_power_of_two().clamp(1 << 14, 1 << 24);
    RingBuffer::new(slots)
}

/// A running RSP: one API session, one thread, and the one or two tuner
/// streams hung off it.
///
/// Held by an `Arc` per stream, and by the binary's device registry as a
/// `Weak` — so the session lives exactly as long as some radio is reading it
/// and closes when the last one lets go. See [`Self::attach`].
pub struct SdrPlayDevice {
    ctrl: Sender<Ctrl>,
    shared: Arc<Shared>,
    join: Mutex<Option<JoinHandle<()>>>,
    /// Which tuners already have a stream. A tuner is one radio's or nobody's.
    taken: Mutex<[bool; TUNERS]>,
    serial: String,
    model: SdrPlayModel,
    /// The tuner the session was opened for, and — with both running — the one
    /// carrying the main aerial.
    main: TunerIdx,
    /// Whether both tuners are running, and if so what for.
    duo: DuoMode,
    /// Delivered complex rate. One number, not two: both tuners come off one
    /// ADC clock through one decimator setting, so whichever radio opened the
    /// session set it and the other adopts it.
    out_rate_hz: f64,
    analog_bw_hz: f64,
    /// The low IF the API's downconverter worked from, in kHz, or zero for
    /// plain zero-IF. Dual-tuner operation forces a low IF, which is what
    /// spares it the DC spike the zero-IF path needs an LO offset to dodge.
    low_if_khz: i32,
}

/// What a session is doing with the second tuner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuoMode {
    /// Only one tuner is running.
    Single,
    /// Both, combined into the main tuner's stream: diversity (issue #153).
    Paired,
    /// Both, as two independent radios (issue #165).
    Split,
}

impl SdrPlayDevice {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        ctrl: Sender<Ctrl>,
        shared: Arc<Shared>,
        join: JoinHandle<()>,
        serial: String,
        model: SdrPlayModel,
        main: TunerIdx,
        duo: DuoMode,
        out_rate_hz: f64,
        analog_bw_hz: f64,
        low_if_khz: i32,
    ) -> SdrPlayDevice {
        SdrPlayDevice {
            ctrl,
            shared,
            join: Mutex::new(Some(join)),
            taken: Mutex::new([false; TUNERS]),
            serial,
            model,
            main,
            duo,
            out_rate_hz,
            analog_bw_hz,
            low_if_khz,
        }
    }

    pub fn serial(&self) -> &str {
        &self.serial
    }

    pub fn model(&self) -> SdrPlayModel {
        self.model
    }

    pub fn duo_mode(&self) -> DuoMode {
        self.duo
    }

    /// The tuner this session opened for — the only one running unless
    /// [`Self::duo_mode`] says both are.
    pub fn main_tuner(&self) -> TunerIdx {
        self.main
    }

    /// The effective complex rate delivered, after any decimation.
    pub fn out_rate_hz(&self) -> f64 {
        self.out_rate_hz
    }

    /// Whether the session thread is still running.
    pub fn is_alive(&self) -> bool {
        self.shared.alive.load(Ordering::Relaxed)
    }

    /// Claim `tuner`'s stream, tuned to `center_hz`.
    ///
    /// Fails rather than sharing: a tuner belongs to one radio at a time, and
    /// two engines reading one ring would each get half the samples. The
    /// message says which case it is, because all three are things an operator
    /// can act on — pick the other tuner, turn the second radio off, or stop
    /// asking a single-tuner session for a tuner it never opened.
    pub fn attach(self: &Arc<Self>, tuner: TunerIdx, center_hz: f64) -> Result<SdrPlayHandle> {
        if tuner >= TUNERS {
            return Err(Error::NotFound(format!("tuner {} does not exist", tuner + 1)));
        }
        match self.duo {
            DuoMode::Split => {}
            DuoMode::Single if tuner == self.main => {}
            DuoMode::Paired if tuner == self.main => {}
            DuoMode::Paired => {
                return Err(Error::InUse(format!(
                    "the RSPduo's tuner {} is being combined into tuner {}'s stream for \
                     diversity — set that radio's second tuner to \"a second radio\" to run \
                     them apart",
                    tuner + 1,
                    self.main + 1
                )));
            }
            DuoMode::Single => {
                return Err(Error::InUse(format!(
                    "the receiver is open on tuner {} alone — both radios have to be set to \
                     run both tuners before either can have one",
                    self.main + 1
                )));
            }
        }
        {
            let mut taken = self.taken.lock().expect("sdrplay taken lock");
            if taken[tuner] {
                return Err(Error::InUse(format!(
                    "tuner {} is already running as another radio",
                    tuner + 1
                )));
            }
            taken[tuner] = true;
        }
        let stride = if self.duo == DuoMode::Paired { crate::pair::QUAD } else { 2 };
        let (prod, cons) = ring_for(self.out_rate_hz, stride);
        if self.ctrl.send(Ctrl::Attach { tuner, ring: prod, center_hz }).is_err() {
            self.taken.lock().expect("sdrplay taken lock")[tuner] = false;
            return Err(Error::Api {
                call: "attach",
                text: "the SDRplay session has stopped".into(),
            });
        }
        Ok(SdrPlayHandle {
            rx: cons,
            dev: Arc::clone(self),
            tuner,
            center: center_hz,
            opened_at: Instant::now(),
            released: false,
        })
    }

    /// Stop the session and hand the device back to the service.
    ///
    /// Idempotent. Called when the last stream is given back, and again from
    /// `Drop`: a session nobody is reading is one whose *configuration* has
    /// stopped being true — the rate, the bandwidth and the tuner arrangement
    /// are all fixed when the board is opened, so a radio that comes back
    /// after an Apply has to find a closed session and open a fresh one rather
    /// than silently adopt the old shape.
    pub(crate) fn shutdown(&self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.lock().expect("sdrplay join lock").take() {
            let _ = j.join();
        }
        self.shared.alive.store(false, Ordering::Relaxed);
    }
}

impl Drop for SdrPlayDevice {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// One tuner's stream: what a radio reads and drives.
pub struct SdrPlayHandle {
    rx: Consumer<f32>,
    dev: Arc<SdrPlayDevice>,
    tuner: TunerIdx,
    center: f64,
    opened_at: Instant,
    released: bool,
}

impl SdrPlayHandle {
    fn shared(&self) -> &TunerShared {
        &self.dev.shared.t[self.tuner]
    }

    /// Tell the stream thread that the engine has stopped reading for an over,
    /// and then that it has started again — see `IqSource::set_rx_paused`. The
    /// receiver itself is left running: the samples keep arriving and keep
    /// being offered, this only decides whether the ones that no longer fit are
    /// reported as a fault.
    pub fn set_rx_paused(&self, paused: bool) {
        self.shared().rx_paused.store(paused, Ordering::Relaxed);
    }

    /// The session this stream belongs to, which may be serving another radio
    /// as well.
    pub fn device(&self) -> &Arc<SdrPlayDevice> {
        &self.dev
    }

    /// Which tuner this stream is, counted from zero.
    pub fn tuner(&self) -> TunerIdx {
        self.tuner
    }

    /// A label for this stream: the model, the serial, the rate, and — when
    /// the board is running both tuners — which one this is.
    pub fn label(&self) -> String {
        let mut s = format!(
            "SDRplay {} (serial {}) @ {:.3} Msps",
            self.dev.model.label(),
            self.dev.serial,
            self.dev.out_rate_hz / 1e6
        );
        match self.dev.duo {
            DuoMode::Single => {}
            DuoMode::Paired => s.push_str(", both tuners"),
            DuoMode::Split => s.push_str(&format!(", tuner {}", self.tuner + 1)),
        }
        s
    }

    pub fn serial(&self) -> &str {
        &self.dev.serial
    }

    pub fn model(&self) -> SdrPlayModel {
        self.dev.model
    }

    /// The effective complex rate delivered, after any decimation.
    pub fn out_rate_hz(&self) -> f64 {
        self.dev.out_rate_hz
    }

    /// The programmed analog IF bandwidth, for the LO-offset calculation.
    pub fn analog_bw_hz(&self) -> f64 {
        self.dev.analog_bw_hz
    }

    /// Whether this stream carries both tuners, combined.
    pub fn dual_tuner(&self) -> bool {
        self.dev.duo == DuoMode::Paired
    }

    /// Whether the session is running both tuners as separate radios.
    pub fn split_tuner(&self) -> bool {
        self.dev.duo == DuoMode::Split
    }

    /// The low IF the service downconverted from, in kHz — zero for the
    /// ordinary zero-IF path. A low IF puts the converter's DC offset outside
    /// the span by construction, so the source needs no LO offset to hide it.
    pub fn low_if_khz(&self) -> i32 {
        self.dev.low_if_khz
    }

    /// The second tuner has stopped delivering, so blocks are going through
    /// unpaired and the diversity filter has nothing to work with.
    pub fn aux_stalled(&self) -> bool {
        self.dev.shared.aux_stalled.load(Ordering::Relaxed)
    }

    /// How many times the two tuners had to be paired up again from scratch.
    pub fn pair_slips(&self) -> u64 {
        self.dev.shared.pair_slips.load(Ordering::Relaxed)
    }

    /// Whether the pairing is working from the service's sample numbers, or
    /// has fallen back to arrival order because they do not advance.
    pub fn pair_stamped(&self) -> bool {
        self.dev.shared.pair_stamped.load(Ordering::Relaxed)
    }

    /// The second tuner's LNA state, after any per-band clamp.
    pub fn aux_lna_state(&self) -> u8 {
        self.dev.shared.t[1 - self.tuner].lna_state.load(Ordering::Relaxed)
    }

    /// Whether the session thread is still running.
    pub fn is_alive(&self) -> bool {
        !self.released && self.dev.is_alive()
    }

    /// The service reported the device gone (unplugged, or the service died).
    pub fn removed(&self) -> bool {
        self.dev.shared.removed.load(Ordering::Relaxed)
    }

    /// The ADC is currently overloaded — gain is set too high for the input.
    pub fn overloaded(&self) -> bool {
        self.dev.shared.overload.load(Ordering::Relaxed)
    }

    pub fn dropped(&self) -> u64 {
        self.shared().dropped.load(Ordering::Relaxed)
    }

    /// How long the receiver has delivered nothing. `last_rx_ms` is a
    /// timestamp from the session epoch, not an age — subtract, don't read.
    pub fn silent_for(&self) -> Duration {
        let since_open = self.opened_at.elapsed();
        let last = Duration::from_millis(self.shared().last_rx_ms.load(Ordering::Relaxed));
        since_open.saturating_sub(last)
    }

    /// IF gain reduction the hardware last reported, if it has reported one.
    /// Under AGC this moves on its own, and it is the honest answer.
    pub fn effective_if_gr_db(&self) -> Option<i32> {
        match self.shared().ev_gr_db.load(Ordering::Relaxed) {
            i64::MIN => None,
            v => Some(v as i32),
        }
    }

    /// The LNA state actually programmed, after any per-band clamp.
    pub fn lna_state(&self) -> u8 {
        self.shared().lna_state.load(Ordering::Relaxed)
    }

    /// Absolute system gain — LNA plus mixer plus IF, everything between the
    /// antenna socket and the converter — in dB, as the hardware reports it
    /// for the samples now arriving. `None` until the first block taken with a
    /// known gain has been delivered.
    ///
    /// This is the number that turns a level in dBFS into a level at the
    /// antenna, and the only one that does: an RSP's front end has two knobs
    /// and neither is in dB (the LNA is a step index whose value depends on
    /// the band, the IF is a *reduction*), so nothing derivable from the
    /// control settings on this side is the whole gain. The API works it out
    /// and publishes it as `gainParams.currGain`, which is what this is.
    ///
    /// It moves on its own whenever the AGC is running, which is precisely
    /// when it matters: without it an S-meter reads the loop's activity
    /// instead of the signal.
    pub fn system_gain_db(&self) -> Option<f64> {
        match self.shared().rx_curr_gain_milli_db.load(Ordering::Relaxed) {
            i64::MIN => None,
            v => Some(v as f64 / 1000.0),
        }
    }

    /// Read interleaved complex samples. Returns how many `f32` were taken.
    ///
    /// With both tuners combined the ring holds quadruples rather than pairs,
    /// so this is the wrong door — use [`Self::read_pair`], which is what the
    /// source calls either way.
    pub fn read(&mut self, out: &mut [f32]) -> usize {
        let stride = self.stride();
        let n = out.len().min(self.rx.slots());
        // Keep a sample's floats together: a partial one would swap I with Q
        // for good.
        let n = n - (n % stride);
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

    /// Floats per sample in the ring: I and Q, or both tuners' I and Q.
    fn stride(&self) -> usize {
        if self.dual_tuner() { crate::pair::QUAD } else { 2 }
    }

    /// Read both tuners at once, as interleaved complex samples, and return
    /// how many **complex samples** landed in each.
    ///
    /// The pair is sample-aligned by construction: the two tuners are put
    /// together in the callbacks and travel through one ring, so there is no
    /// way for them to come apart on the way here — see [`crate::pair`]. Where
    /// this stream carries one tuner `aux` is left untouched and this is the
    /// ordinary read.
    pub fn read_pair(&mut self, main: &mut [f32], aux: &mut [f32]) -> usize {
        if !self.dual_tuner() {
            return self.read(main) / 2;
        }
        let quad = crate::pair::QUAD;
        let want = (main.len() / 2).min(aux.len() / 2).min(self.rx.slots() / quad);
        if want == 0 {
            return 0;
        }
        let Ok(chunk) = self.rx.read_chunk(want * quad) else {
            return 0;
        };
        let (head, tail) = chunk.as_slices();
        for (idx, &v) in head.iter().chain(tail.iter()).enumerate() {
            let (sample, lane) = (idx / quad, idx % quad);
            match lane {
                0 => main[2 * sample] = v,
                1 => main[2 * sample + 1] = v,
                2 => aux[2 * sample] = v,
                _ => aux[2 * sample + 1] = v,
            }
        }
        chunk.commit_all();
        want
    }

    fn send(&self, c: Ctrl) {
        let _ = self.dev.ctrl.send(c);
    }

    pub fn center_hz(&self) -> f64 {
        self.center
    }

    pub fn set_center_hz(&mut self, hz: f64) {
        self.center = hz;
        self.send(Ctrl::Center(self.tuner, hz));
    }

    pub fn set_if_gr_db(&self, gr: i32) {
        self.send(Ctrl::IfGr(self.tuner, gr));
    }

    pub fn set_lna_state(&self, state: u8) {
        self.send(Ctrl::Lna(self.tuner, state));
    }

    /// The *other* tuner's gains, which one radio owns only while it is
    /// combining the pair: in diversity the second aerial has no radio of its
    /// own to set them from.
    pub fn set_aux_if_gr_db(&self, gr: i32) {
        self.send(Ctrl::IfGr(1 - self.tuner, gr));
    }

    pub fn set_aux_lna_state(&self, state: u8) {
        self.send(Ctrl::Lna(1 - self.tuner, state));
    }

    pub fn set_agc(&self, code: i32) {
        self.send(Ctrl::Agc(self.tuner, code));
        // A diversity pair wants both branches gained the same way, or the
        // filter spends its time chasing one AGC against the other.
        if self.dual_tuner() {
            self.send(Ctrl::Agc(1 - self.tuner, code));
        }
    }

    pub fn set_agc_setpoint(&self, dbfs: i32) {
        self.send(Ctrl::AgcSetpoint(self.tuner, dbfs));
        if self.dual_tuner() {
            self.send(Ctrl::AgcSetpoint(1 - self.tuner, dbfs));
        }
    }

    /// The reference trim, which belongs to the board rather than to a tuner:
    /// on a split RSPduo either radio's setting moves both.
    pub fn set_ppm(&self, ppm: f64) {
        self.send(Ctrl::Ppm(ppm));
    }

    pub fn set_bias_tee(&self, on: bool) {
        self.send(Ctrl::BiasTee(self.tuner, on));
    }

    pub fn set_rf_notch(&self, on: bool) {
        self.send(Ctrl::RfNotch(self.tuner, on));
        // Both branches of a diversity pair through the same filters, for the
        // same reason as the AGC.
        if self.dual_tuner() {
            self.send(Ctrl::RfNotch(1 - self.tuner, on));
        }
    }

    pub fn set_dab_notch(&self, on: bool) {
        self.send(Ctrl::DabNotch(self.tuner, on));
        if self.dual_tuner() {
            self.send(Ctrl::DabNotch(1 - self.tuner, on));
        }
    }

    /// The RSPdx's HDR path — a device-wide switch, and a model that has no
    /// second tuner anyway.
    pub fn set_hdr(&self, on: bool) {
        self.send(Ctrl::Hdr(on));
    }

    /// The HDR path's analog filter. Only does anything with the path itself
    /// switched on, and only at the centres that filter is built for — see
    /// [`sdroxide_types::SdrPlayHdrBw::works_at`], which is what the source
    /// warns the operator from.
    pub fn set_hdr_bw(&self, code: i32) {
        self.send(Ctrl::HdrBw(code));
    }

    /// Open the last 20 dB of IF gain, below the API's normal floor.
    pub fn set_extended_if_gr(&self, on: bool) {
        self.send(Ctrl::ExtendedIfGr(self.tuner, on));
        if self.dual_tuner() {
            self.send(Ctrl::ExtendedIfGr(1 - self.tuner, on));
        }
    }

    pub fn set_antenna(&self, name: &str) {
        self.send(Ctrl::Antenna(self.tuner, name.to_string()));
    }

    /// Give this tuner's stream back.
    ///
    /// Idempotent, and leaves the handle callable but inert — the engine's
    /// reopen path requires exactly that, because a replacement is built only
    /// after the outgoing source has stood down.
    ///
    /// The *session* stays open while another radio still holds a tuner on it,
    /// which is what lets one radio be reopened without interrupting the
    /// other. The last stream to go closes it, rather than leaving it for the
    /// next open to adopt: everything the session's shape is made of — the
    /// rate, the bandwidth, which tuners run and what for — is chosen when the
    /// board is opened, so a live session outliving its last radio would make
    /// an Apply that changes any of them do nothing at all.
    pub fn release(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        self.send(Ctrl::Detach(self.tuner));
        let last = {
            let mut taken = self.dev.taken.lock().expect("sdrplay taken lock");
            taken[self.tuner] = false;
            !taken.iter().any(|t| *t)
        };
        if last {
            self.dev.shutdown();
        }
    }
}

impl Drop for SdrPlayHandle {
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
        p.absorb(Ctrl::Center(0, 7_000_000.0));
        p.absorb(Ctrl::Center(0, 14_000_000.0));
        p.absorb(Ctrl::IfGr(0, 30));
        p.absorb(Ctrl::Antenna(0, "Antenna A".into()));
        p.absorb(Ctrl::Antenna(0, "Antenna B".into()));
        assert_eq!(p.t[0].center, Some(14_000_000.0));
        assert_eq!(p.t[0].if_gr, Some(30));
        assert_eq!(p.t[0].antenna.as_deref(), Some("Antenna B"));
        assert_eq!(p.t[0].lna, None);
        assert!(!p.is_empty());
    }

    /// Two radios on one board are two dials, two gain sets and two antenna
    /// choices — and a control for one of them must never land on the other.
    #[test]
    fn each_tuner_keeps_its_own_control() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Center(0, 7_100_000.0));
        p.absorb(Ctrl::Center(1, 145_500_000.0));
        p.absorb(Ctrl::Lna(1, 3));
        assert_eq!(p.t[0].center, Some(7_100_000.0));
        assert_eq!(p.t[1].center, Some(145_500_000.0));
        assert_eq!(p.t[0].lna, None);
        assert_eq!(p.t[1].lna, Some(3));
        // A message naming a tuner no RSP has is dropped, not indexed.
        p.absorb(Ctrl::Center(9, 1.0));
        assert_eq!(p.t[0].center, Some(7_100_000.0));
    }

    /// A detach cancels an attach the same pass brought, and the other way
    /// round: what a radio last said is what it meant.
    #[test]
    fn attach_and_detach_cancel_each_other_out() {
        let mut p = Pending::default();
        let (prod, _cons) = RingBuffer::<f32>::new(16);
        p.absorb(Ctrl::Attach { tuner: 1, ring: prod, center_hz: 1.0 });
        assert_eq!(p.attach.len(), 1);
        p.absorb(Ctrl::Detach(1));
        assert!(p.attach.is_empty(), "the ring goes with the radio that asked for it");
        assert!(p.detach[1]);
        let (prod, _cons) = RingBuffer::<f32>::new(16);
        p.absorb(Ctrl::Attach { tuner: 1, ring: prod, center_hz: 2.0 });
        assert!(!p.detach[1], "re-attaching cancels the detach");
        assert_eq!(p.attach.len(), 1);
    }

    #[test]
    fn shutdown_survives_later_messages() {
        let mut p = Pending::default();
        p.absorb(Ctrl::Shutdown);
        p.absorb(Ctrl::Center(0, 1.0));
        assert!(p.shutdown, "a shutdown must not be forgotten by a later retune");
    }

    #[test]
    fn the_ring_holds_about_half_a_second() {
        let (p, _c) = ring_for(2_000_000.0, 2);
        assert!(p.slots() >= 2_000_000);
        assert!(p.slots().is_power_of_two());
        // Two tuners are twice the floats for the same half second.
        let (p, _c) = ring_for(2_000_000.0, 4);
        assert!(p.slots() >= 4_000_000);
    }

    fn test_handle(cons: Consumer<f32>) -> SdrPlayHandle {
        test_handle_with(cons, DuoMode::Single, 0)
    }

    /// A stream on a session with no thread behind it: everything but the
    /// device itself, which is what the accounting and the reads are made of.
    fn test_handle_with(cons: Consumer<f32>, duo: DuoMode, tuner: TunerIdx) -> SdrPlayHandle {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let dev = Arc::new(SdrPlayDevice::from_parts(
            tx,
            Arc::new(Shared::new()),
            std::thread::spawn(|| {}),
            "0000000000".into(),
            if duo == DuoMode::Single { SdrPlayModel::Rsp1b } else { SdrPlayModel::RspDuo },
            0,
            duo,
            2_000_000.0,
            1_536_000.0,
            if duo == DuoMode::Single { 0 } else { 1620 },
        ));
        SdrPlayHandle {
            rx: cons,
            dev,
            tuner,
            center: 14_100_000.0,
            opened_at: Instant::now(),
            released: false,
        }
    }

    #[test]
    fn reads_never_split_an_iq_pair() {
        let (mut prod, cons) = RingBuffer::<f32>::new(64);
        for i in 0..9 {
            prod.push(i as f32).unwrap();
        }
        let mut h = test_handle(cons);
        let mut buf = [0f32; 32];
        // Nine samples are available; an even count must come back.
        let n = h.read(&mut buf);
        assert_eq!(n % 2, 0, "read {n} f32, which splits an I/Q pair");
        assert_eq!(n, 8);
        h.released = true; // do not try to join in Drop
    }

    #[test]
    fn silence_is_an_age_not_a_timestamp() {
        let (_p, cons) = RingBuffer::<f32>::new(16);
        let mut h = test_handle(cons);
        let age = h.opened_at.elapsed();
        h.shared().last_rx_ms.store(age.as_millis() as u64, Ordering::Relaxed);
        assert!(h.silent_for() < Duration::from_millis(50), "{:?}", h.silent_for());
        h.released = true;
    }

    #[test]
    fn effective_gain_is_unknown_until_an_event_reports_one() {
        let (_p, cons) = RingBuffer::<f32>::new(16);
        let mut h = test_handle(cons);
        assert_eq!(h.effective_if_gr_db(), None);
        h.shared().ev_gr_db.store(42, Ordering::Relaxed);
        assert_eq!(h.effective_if_gr_db(), Some(42));
        h.released = true;
    }

    /// The announcement alone must not move the meter: an event is the service
    /// saying it has programmed a gain, not that the samples in the pipe were
    /// taken with it.
    #[test]
    fn system_gain_follows_the_samples_not_the_announcement() {
        let (_p, cons) = RingBuffer::<f32>::new(16);
        let mut h = test_handle(cons);
        assert_eq!(h.system_gain_db(), None);
        h.shared().ev_curr_gain_milli_db.store(43_500, Ordering::Relaxed);
        assert_eq!(h.system_gain_db(), None, "an announced gain is not a delivered one");
        h.shared().rx_curr_gain_milli_db.store(43_500, Ordering::Relaxed);
        assert_eq!(h.system_gain_db(), Some(43.5));
        h.released = true;
    }

    /// A split RSPduo's two radios each read their own front end. The gain is
    /// per tuner for the same reason the IF reduction is: they are two
    /// receivers, and one operator's attenuator is not the other's.
    #[test]
    fn system_gain_is_per_tuner() {
        let (_p, cons) = RingBuffer::<f32>::new(16);
        let mut h = test_handle_with(cons, DuoMode::Split, 1);
        h.dev.shared.t[0].rx_curr_gain_milli_db.store(20_000, Ordering::Relaxed);
        assert_eq!(h.system_gain_db(), None, "tuner 1 read tuner 0's gain");
        h.dev.shared.t[1].rx_curr_gain_milli_db.store(31_000, Ordering::Relaxed);
        assert_eq!(h.system_gain_db(), Some(31.0));
        h.released = true;
    }

    /// The partial writer must never commit a lone I with its Q left behind.
    /// `slots()` is a count of floats and is under no obligation to be even, so
    /// taking it at face value eventually puts the ring one float out of step —
    /// and from then on every pair is swapped and the spectrum is mirrored for
    /// the rest of the session.
    #[test]
    fn a_partial_write_still_lands_on_a_pair_boundary() {
        let (mut prod, cons) = RingBuffer::<f32>::new(8);
        let mut stats = RxStats::new(2.0e6);
        // Leave an odd three slots free, then offer four floats.
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            prod.push(v).expect("room");
        }
        let dropped = push_iq(&mut prod, &[6.0, 7.0, 8.0, 9.0], &mut stats, false, 2);
        assert_eq!(cons.slots(), 7, "only one whole pair fitted the three free slots");
        assert_eq!(dropped, 1, "the pair that did not fit is one complex sample");
    }

    /// The paired read must split the ring's quadruples back into two streams
    /// the right way round — an I and a Q of one tuner, then of the other.
    #[test]
    fn a_paired_read_splits_the_two_tuners_apart() {
        let (mut prod, cons) = RingBuffer::<f32>::new(64);
        let mut stats = RxStats::new(2.0e6);
        // Two samples: main (1,2) then (5,6); aux (3,4) then (7,8).
        push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], &mut stats, false, 4);
        let mut h = test_handle_with(cons, DuoMode::Paired, 0);
        let (mut main, mut aux) = ([0f32; 8], [0f32; 8]);
        let n = h.read_pair(&mut main, &mut aux);
        assert_eq!(n, 2, "two complex samples per tuner");
        assert_eq!(&main[..4], &[1.0, 2.0, 5.0, 6.0]);
        assert_eq!(&aux[..4], &[3.0, 4.0, 7.0, 8.0]);
        h.released = true;
    }

    /// A ring that fills because the engine stopped reading for an over is not
    /// the DSP thread falling behind, and must not reach the fault counters —
    /// nor the count this hands back for the handle's `dropped()` total.
    #[test]
    fn a_full_ring_while_paused_is_not_an_overrun() {
        let (mut prod, cons) = RingBuffer::<f32>::new(4);
        let mut stats = RxStats::new(2.0e6);

        push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0], &mut stats, true, 2);
        let dropped = push_iq(&mut prod, &[5.0, 6.0], &mut stats, true, 2);
        assert_eq!(dropped, 0, "a paused receiver reports no overruns to its caller");
        assert_eq!(stats.win_dropped, 0);
        assert_eq!(stats.win_keyed, 1, "the discarded pair is accounted for as keyed");

        // Unpaused, the very same full ring is a genuine overrun again.
        let dropped = push_iq(&mut prod, &[7.0, 8.0], &mut stats, false, 2);
        assert_eq!(dropped, 1);
        assert_eq!(stats.win_dropped, 1);
        assert_eq!(stats.win_keyed, 1);
        assert_eq!(cons.slots() % 2, 0);
    }
}
