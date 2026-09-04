//! The handle the rest of the program holds.
//!
//! Same shape as [`sdroxide_spyserver::SpyServerHandle`], and for the same
//! reason: two output lanes with very different needs. The I/Q rides an `rtrb`
//! ring, because every sample matters and the reader is a DSP chain. The
//! waterfall rides a mutex-guarded slot where the newest frame overwrites an
//! unread one, because a six-frame-old picture of the band is worth nothing
//! beside a current one.
//!
//! What is different here is that the two lanes are two *sockets*, on two
//! threads. The receiver treats them as one session — they share a timestamp in
//! their URLs — but nothing else couples them, and the waterfall may fail to
//! open without costing the operator their receiver.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU8, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use rtrb::{Consumer, Producer};

/// One finished full-band frame from the receiver's waterfall.
///
/// The centre and span travel with the bins rather than in separate accessors,
/// for the reason the SpyServer's `FftFrame` does: a frame decoded either side
/// of a retune would otherwise be labelled with the wrong frequency, silently.
/// Here the window does not actually move — this client only ever asks for
/// zoom 0, the receiver's whole band — but the invariant is worth keeping in
/// the type rather than in a comment.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterfallFrame {
    pub center_hz: f64,
    pub span_hz: f64,
    /// dBm on the receiver's own calibration, ascending from the low edge.
    pub bins: Vec<f32>,
}

/// What the receiver said about itself in its opening burst.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KiwiInfo {
    /// The I/Q rate it will actually deliver — 11998.874997 Hz on the receiver
    /// this was measured against, and *not* the round 12 kHz it also reports as
    /// `audio_rate`. The resampler has to use this one.
    pub sample_rate_hz: f64,
    /// Centre and width of the receiver's whole band, which is what the
    /// waterfall covers at zoom 0.
    pub center_hz: f64,
    pub bandwidth_hz: f64,
    /// Firmware, as `(major, minor)` — 1 and 902 for v1.902.
    pub version: (u32, u32),
    /// User channels this receiver has.
    pub rx_chans: u32,
    /// The operator's waterfall calibration, applied on this side.
    pub wf_cal: i32,
}

impl KiwiInfo {
    /// One line for a log or a Test button.
    pub fn describe(&self) -> String {
        format!(
            "KiwiSDR v{}.{} — {:.0}–{:.0} kHz, {} channels, {:.1} kHz I/Q",
            self.version.0,
            self.version.1,
            (self.center_hz - self.bandwidth_hz / 2.0) / 1e3,
            (self.center_hz + self.bandwidth_hz / 2.0) / 1e3,
            self.rx_chans,
            self.sample_rate_hz / 1e3,
        )
    }
}

/// A control message for the audio thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ctrl {
    Center(f64),
    Agc(bool, u8),
    Shutdown,
}

/// Control messages accumulated over one pass of the thread loop.
///
/// Dragging the dial emits hundreds of `Center` messages a second and the
/// receiver will accept a handful; applying each in turn would put the thread
/// permanently behind the operator's hand. Last value wins, which is the right
/// semantics for a dial.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Pending {
    pub center: Option<f64>,
    pub agc: Option<(bool, u8)>,
    pub shutdown: bool,
}

impl Pending {
    pub(crate) fn absorb(&mut self, c: Ctrl) {
        match c {
            Ctrl::Center(v) => self.center = Some(v),
            Ctrl::Agc(on, g) => self.agc = Some((on, g)),
            Ctrl::Shutdown => self.shutdown = true,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        *self == Pending::default()
    }
}

/// Shared state the two threads publish and the handle reads.
pub(crate) struct Shared {
    /// The audio thread is running. The waterfall thread's health is
    /// deliberately not part of this: losing the band view is a degradation,
    /// not a dead receiver.
    pub alive: AtomicBool,
    /// Set once by whichever thread hit a refusal, so the source can report it
    /// and — crucially — decline to reconnect. Guarded rather than atomic
    /// because it is a sentence.
    pub refusal: std::sync::Mutex<Option<String>>,
    /// Milliseconds since the thread started, at the last block delivered.
    pub last_rx_ms: AtomicU64,
    /// The newest waterfall frame, or none since it was last taken.
    pub wf: std::sync::Mutex<Option<WaterfallFrame>>,
    /// The receiver's own S-meter from the last audio frame, in hundredths of
    /// a dBm so an integer atomic can carry it.
    pub smeter_centi_dbm: AtomicI32,
    /// Set while the receiver's ADC was clipping on the last frame.
    pub adc_overflow: AtomicBool,
    /// Where the audio channel is tuned, in millihertz.
    pub center_milli_hz: AtomicI64,
    /// Waterfall speed the operator has asked for, read by the waterfall
    /// thread between frames.
    pub wf_speed: AtomicU8,
    /// How far the operator has asked the receiver's waterfall to be zoomed in,
    /// as a power of two of its whole band — see
    /// [`sdroxide_types::KiwiConfig::wf_zoom`]. Read by the waterfall thread
    /// between frames, alongside [`Self::center_milli_hz`], which is what the
    /// window is centred on once it is narrower than the band.
    pub wf_zoom: AtomicU8,
    /// Both threads stand down when this is set.
    pub stop: AtomicBool,
}

/// An open session with a KiwiSDR.
pub struct KiwiHandle {
    rx: Consumer<f32>,
    ctrl: Sender<Ctrl>,
    pub(crate) shared: Arc<Shared>,
    opened_at: Instant,
    threads: Vec<JoinHandle<()>>,
    /// What the receiver said about itself at connect.
    pub info: KiwiInfo,
    /// Description for logs and the UI.
    pub label: String,
}

impl KiwiHandle {
    pub(crate) fn from_parts(
        rx: Consumer<f32>,
        ctrl: Sender<Ctrl>,
        shared: Arc<Shared>,
        threads: Vec<JoinHandle<()>>,
        info: KiwiInfo,
        label: String,
    ) -> KiwiHandle {
        KiwiHandle { rx, ctrl, shared, opened_at: Instant::now(), threads, info, label }
    }

    /// Whether the audio thread is still running.
    pub fn is_alive(&self) -> bool {
        self.shared.alive.load(Ordering::Relaxed)
    }

    /// The receiver's own words for why it declined, if it did.
    ///
    /// A session that ended this way must **not** be reconnected: the receiver
    /// is working and has said no. See [`crate::Error::is_retryable`].
    pub fn refusal(&self) -> Option<String> {
        self.shared.refusal.lock().ok().and_then(|g| g.clone())
    }

    /// How long the receiver has gone without delivering samples, measured
    /// from the last block or — if none ever arrived — from when it was
    /// opened.
    pub fn silent_for(&self) -> Duration {
        let since_open = self.opened_at.elapsed();
        let last = Duration::from_millis(self.shared.last_rx_ms.load(Ordering::Relaxed));
        since_open.saturating_sub(last)
    }

    /// Take the newest waterfall frame, if one has arrived since the last call.
    pub fn take_waterfall(&self) -> Option<WaterfallFrame> {
        self.shared.wf.lock().ok().and_then(|mut g| g.take())
    }

    /// The receiver's own S-meter reading, in dBm.
    ///
    /// Not derived from the samples on purpose: with the receiver's AGC ahead
    /// of the I/Q, the sample amplitude measures the AGC as much as the signal.
    pub fn smeter_dbm(&self) -> f32 {
        self.shared.smeter_centi_dbm.load(Ordering::Relaxed) as f32 / 100.0
    }

    /// Whether the far end's ADC was clipping on the last frame — the one
    /// front-end fault visible from this side of the link.
    pub fn adc_overflow(&self) -> bool {
        self.shared.adc_overflow.load(Ordering::Relaxed)
    }

    pub fn center_hz(&self) -> f64 {
        self.shared.center_milli_hz.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Drain interleaved I,Q floats into `out`. Always an even count, so the
    /// stream can never come out of alignment; zero means nothing yet.
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

    fn send(&self, c: Ctrl) {
        // A closed channel means the thread has exited; `is_alive` is where
        // that is noticed, so there is nothing useful to do here.
        let _ = self.ctrl.send(c);
    }

    pub fn set_center_hz(&self, hz: f64) {
        self.send(Ctrl::Center(hz));
    }

    pub fn set_agc(&self, on: bool, man_gain: u8) {
        self.send(Ctrl::Agc(on, man_gain));
    }

    pub fn set_wf_speed(&self, speed: u8) {
        self.shared.wf_speed.store(speed.clamp(1, 4), Ordering::Relaxed);
    }

    /// How far in the receiver's waterfall is zoomed. The window follows the
    /// dial from there; see [`sdroxide_types::KiwiConfig::wf_zoom`].
    pub fn set_wf_zoom(&self, zoom: u8) {
        self.shared
            .wf_zoom
            .store(zoom.min(sdroxide_types::KiwiConfig::WF_ZOOM_MAX), Ordering::Relaxed);
    }

    /// The span the receiver's waterfall currently covers, in Hz.
    pub fn wf_span_hz(&self) -> f64 {
        let z = self.shared.wf_zoom.load(Ordering::Relaxed);
        self.info.bandwidth_hz / f64::from(1u32 << z.min(sdroxide_types::KiwiConfig::WF_ZOOM_MAX))
    }

    /// Close both sockets and stop both threads, without dropping the handle.
    ///
    /// Idempotent, and it *waits*: a KiwiSDR has four or eight user channels
    /// and holding one open after the operator closed the radio is taking it
    /// from somebody. This is the one place in this backend where blocking on a
    /// join is the right thing to do.
    pub fn release(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        let _ = self.ctrl.send(Ctrl::Shutdown);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        self.shared.alive.store(false, Ordering::Relaxed);
    }
}

impl Drop for KiwiHandle {
    fn drop(&mut self) {
        self.release();
    }
}

/// Push interleaved I/Q into the ring, keeping I and Q paired.
///
/// All-or-nothing: a block the ring cannot take whole is dropped whole.
/// Pushing what fits would leave the ring one float out of step and swap I with
/// Q for the rest of the session — a mirrored, unusable spectrum that reads
/// like a driver bug rather than the overrun it is.
pub(crate) fn push_iq(rx: &mut Producer<f32>, iq: &[f32]) -> bool {
    let Ok(mut chunk) = rx.write_chunk(iq.len()) else {
        return false;
    };
    let (head, tail) = chunk.as_mut_slices();
    head.copy_from_slice(&iq[..head.len()]);
    tail.copy_from_slice(&iq[head.len()..]);
    chunk.commit_all();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dial_drag_collapses_to_its_last_position() {
        let mut p = Pending::default();
        assert!(p.is_empty());
        for hz in [7_000_000.0, 7_001_000.0, 7_002_000.0] {
            p.absorb(Ctrl::Center(hz));
        }
        p.absorb(Ctrl::Agc(false, 40));
        assert_eq!(p.center, Some(7_002_000.0));
        assert_eq!(p.agc, Some((false, 40)));
        assert!(!p.shutdown);
        p.absorb(Ctrl::Shutdown);
        assert!(p.shutdown);
    }

    #[test]
    fn a_block_the_ring_cannot_take_whole_is_dropped_whole() {
        let (mut prod, mut cons) = rtrb::RingBuffer::<f32>::new(8);
        assert!(push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0]));
        // Four slots left, six offered: refused, so I and Q stay paired.
        assert!(!push_iq(&mut prod, &[5.0; 6]));
        let mut out = [0.0f32; 8];
        let mut n = 0;
        while let Ok(v) = cons.pop() {
            out[n] = v;
            n += 1;
        }
        assert_eq!(&out[..n], &[1.0, 2.0, 3.0, 4.0]);
    }
}
