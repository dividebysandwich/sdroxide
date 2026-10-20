//! RTTY modem: streaming decode + encode of Baudot (ITA2) FSK.
//!
//! Standard amateur RTTY: 45.45 baud, 170 Hz shift, 5-bit Baudot with
//! LTRS/FIGS shift, 1 start bit (space) + 5 data (LSB first) + 1.5 stop (mark),
//! idle = mark. USB-side: mark is the higher tone (`center + shift/2`), space
//! the lower. Baud, shift and mark/space sense are configurable.
//!
//! RX down-converts to complex baseband at the operator's center tone and runs
//! the textbook non-coherent FSK detector: one integrate-and-dump matched
//! filter per tone, sliced on the difference of their envelopes. A bit clock
//! locked to the observed transitions strobes at each bit centre, and a UART
//! framer that *validates* the start and stop bits assembles characters. TX
//! renders phase-continuous FSK.
//!
//! Four details are what make this read a real signal rather than a loopback
//! (issue #23):
//!
//! * The matched filters, not an FM discriminator, make the bit decision. A
//!   moving average of one bit has an equivalent noise bandwidth of exactly the
//!   baud rate, so it collects the full matched-filter processing gain — worth
//!   about 6 dB against slicing the instantaneous frequency sample by sample.
//! * Framing is checked, not assumed, and output is gated on it. The old slicer
//!   resynchronised on any mark→space edge and emitted whatever five bits
//!   followed, so noise during the 1.5-bit stop period manufactured a character
//!   every time — a receiver sitting on an empty channel produced several
//!   characters a second of hash. Requiring a space start bit and a mark stop
//!   bit, and holding output until a few frames in a row have validated,
//!   silences that without costing any sensitivity.
//! * Automatic threshold correction refers each tone to its own recent level.
//!   HF paths are frequency selective, so mark and space regularly arrive tens
//!   of dB apart; comparing raw envelopes then decides on the fade instead of
//!   on the data.
//! * AFC tracks tuning error, which the matched filters are far less forgiving
//!   of than the old wideband discriminator was — and which is exactly what the
//!   reporter had to switch on in FLdigi to read the same signal.

use std::collections::VecDeque;
use std::f64::consts::TAU;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};

// ── Baudot / ITA2 (US-TTY figures) ──────────────────────────────────────
const LTRS: u8 = 31;
const FIGS: u8 = 27;
const BAUDOT_SPACE: u8 = 4;
const BAUDOT_CR: u8 = 2;
const BAUDOT_LF: u8 = 8;

fn letter_char(v: u8) -> Option<char> {
    Some(match v {
        3 => 'A',
        25 => 'B',
        14 => 'C',
        9 => 'D',
        1 => 'E',
        13 => 'F',
        26 => 'G',
        20 => 'H',
        6 => 'I',
        11 => 'J',
        15 => 'K',
        18 => 'L',
        28 => 'M',
        12 => 'N',
        24 => 'O',
        22 => 'P',
        23 => 'Q',
        10 => 'R',
        5 => 'S',
        16 => 'T',
        7 => 'U',
        30 => 'V',
        19 => 'W',
        29 => 'X',
        21 => 'Y',
        17 => 'Z',
        _ => return None,
    })
}

fn figure_char(v: u8) -> Option<char> {
    Some(match v {
        1 => '3',
        3 => '-',
        6 => '8',
        7 => '7',
        9 => '$',
        10 => '4',
        11 => '\'',
        12 => ',',
        13 => '!',
        14 => ':',
        15 => '(',
        16 => '5',
        17 => '"',
        18 => ')',
        19 => '2',
        20 => '#',
        21 => '6',
        22 => '0',
        23 => '1',
        24 => '9',
        25 => '?',
        26 => '&',
        28 => '.',
        29 => '/',
        30 => ';',
        _ => return None, // 5 (S) = BELL: ignored
    })
}

fn letter_code(c: char) -> Option<u8> {
    ('A'..='Z')
        .contains(&c)
        .then(|| ())
        .and_then(|_| (0u8..32).find(|&v| letter_char(v) == Some(c)))
}
fn figure_code(c: char) -> Option<u8> {
    (0u8..32).find(|&v| figure_char(v) == Some(c))
}

/// Baudot value → character with LTRS/FIGS shift state. Shared by the audio
/// [`RttyRx`] and the wideband skimmer (both frame characters themselves and
/// hand the 5-bit value here).
#[derive(Default)]
pub struct BaudotRx {
    figs: bool,
}

impl BaudotRx {
    pub fn new() -> Self {
        BaudotRx { figs: false }
    }

    /// Decode a 5-bit Baudot value; returns the character, or `None` for shift
    /// codes and ignored controls.
    pub fn decode(&mut self, value: u8) -> Option<char> {
        match value {
            LTRS => {
                self.figs = false;
                None
            }
            FIGS => {
                self.figs = true;
                None
            }
            BAUDOT_SPACE => Some(' '),
            BAUDOT_LF => Some('\n'),
            BAUDOT_CR => None, // ignore CR; LF drives newlines
            0 => None,
            v => {
                if self.figs {
                    figure_char(v)
                } else {
                    letter_char(v)
                }
            }
        }
    }
}

// ─────────────────────────────── transmit ───────────────────────────────

struct TxBit {
    mark: bool,
    char_done: Option<usize>,
}

/// Streaming RTTY transmitter. Feed text with [`RttyTx::push_text`]; pull audio
/// with [`RttyTx::next_block`]. Idle output is a steady mark tone.
pub struct RttyTx {
    rate: f64,
    mark_hz: f64,
    space_hz: f64,
    spb: usize, // samples per bit
    ph: f64,
    q: VecDeque<TxBit>,
    figs: bool,
    total_chars: usize,
    sent_chars: usize,
    // current bit render
    cur_left: usize,
    cur_mark: bool,
    cur_done: Option<usize>,
}

impl RttyTx {
    pub fn new(rate: f64, center_hz: f64, baud: f64, shift_hz: f64) -> Self {
        RttyTx {
            rate,
            mark_hz: center_hz + shift_hz / 2.0,
            space_hz: center_hz - shift_hz / 2.0,
            spb: (rate / baud).round().max(1.0) as usize,
            ph: 0.0,
            q: VecDeque::new(),
            figs: false,
            total_chars: 0,
            sent_chars: 0,
            cur_left: 0,
            cur_mark: true,
            cur_done: None,
        }
    }

    pub fn set_tuning(&mut self, center_hz: f64, shift_hz: f64) {
        self.mark_hz = center_hz + shift_hz / 2.0;
        self.space_hz = center_hz - shift_hz / 2.0;
    }

    fn frame(&mut self, value: u8, char_done: Option<usize>) {
        // start (space), 5 data LSB-first (mark=1), 1.5 stop → 2 mark bits.
        self.q.push_back(TxBit { mark: false, char_done: None });
        for i in 0..5 {
            self.q.push_back(TxBit { mark: (value >> i) & 1 == 1, char_done: None });
        }
        self.q.push_back(TxBit { mark: true, char_done: None });
        self.q.push_back(TxBit { mark: true, char_done });
    }

    /// Queue text (uppercased; unknown characters dropped). Emits LTRS/FIGS
    /// shift codes as needed.
    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let ci = self.total_chars;
            self.total_chars += 1;
            let c = ch.to_ascii_uppercase();
            match c {
                ' ' => self.frame(BAUDOT_SPACE, Some(ci)),
                '\n' => self.frame(BAUDOT_LF, Some(ci)),
                '\r' => self.frame(BAUDOT_CR, Some(ci)),
                _ => {
                    if let Some(v) = letter_code(c) {
                        if self.figs {
                            self.figs = false;
                            self.frame(LTRS, None);
                        }
                        self.frame(v, Some(ci));
                    } else if let Some(v) = figure_code(c) {
                        if !self.figs {
                            self.figs = true;
                            self.frame(FIGS, None);
                        }
                        self.frame(v, Some(ci));
                    } else {
                        // Unrepresentable: still count it as "sent" so the UI
                        // cursor doesn't stall.
                        self.sent_chars = self.sent_chars.max(ci + 1);
                    }
                }
            }
        }
    }

    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }
    pub fn total_chars(&self) -> usize {
        self.total_chars
    }
    pub fn drained(&self) -> bool {
        self.q.is_empty() && self.cur_left == 0
    }
    pub fn clear(&mut self) {
        self.q.clear();
        self.cur_left = 0;
        self.figs = false;
        self.total_chars = 0;
        self.sent_chars = 0;
    }

    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        for s in out.iter_mut() {
            if self.cur_left == 0 {
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                match self.q.pop_front() {
                    Some(b) => {
                        self.cur_mark = b.mark;
                        self.cur_done = b.char_done;
                    }
                    None => {
                        self.cur_mark = true; // idle mark
                        self.cur_done = None;
                    }
                }
                self.cur_left = self.spb;
            }
            let f = if self.cur_mark { self.mark_hz } else { self.space_hz };
            self.ph += TAU * f / self.rate;
            if self.ph > TAU {
                self.ph -= TAU;
            }
            *s = (self.ph.sin() as f32) * 0.5;
            self.cur_left -= 1;
        }
        out.len()
    }
}

// ─────────────────────────────── receive ───────────────────────────────

/// Sliding integrate-and-dump over one bit — the matched filter for a
/// rectangular FSK symbol. A moving average of `len` samples has an equivalent
/// noise bandwidth of exactly `rate/len` (the baud rate here), sidelobes and
/// all, so this collects the full matched-filter gain at O(1) per sample.
pub(crate) struct Integrator {
    ring: Vec<Complex32>,
    pos: usize,
    // f64 so the running sum can't drift over an hours-long stream.
    sum_re: f64,
    sum_im: f64,
}

impl Integrator {
    pub(crate) fn new(len: usize) -> Self {
        Integrator {
            ring: vec![Complex32::new(0.0, 0.0); len.max(1)],
            pos: 0,
            sum_re: 0.0,
            sum_im: 0.0,
        }
    }

    pub(crate) fn resize(&mut self, len: usize) {
        self.ring.clear();
        self.ring.resize(len.max(1), Complex32::new(0.0, 0.0));
        self.pos = 0;
        self.sum_re = 0.0;
        self.sum_im = 0.0;
    }

    /// Push one sample; returns the window mean.
    pub(crate) fn push(&mut self, z: Complex32) -> Complex32 {
        let old = std::mem::replace(&mut self.ring[self.pos], z);
        self.pos = (self.pos + 1) % self.ring.len();
        self.sum_re += (z.re - old.re) as f64;
        self.sum_im += (z.im - old.im) as f64;
        let k = 1.0 / self.ring.len() as f64;
        Complex32::new((self.sum_re * k) as f32, (self.sum_im * k) as f32)
    }
}

/// UART framer state. `Start` and `Stop` exist so the framing can be *checked*;
/// the old decoder had neither and paid for it in noise.
#[derive(PartialEq, Clone, Copy)]
enum RxState {
    /// Hunting for the mark→space edge that opens a character.
    Hunt,
    /// Strobe lands on the start bit; it must read space.
    Start,
    /// Collecting the five data bits, LSB first.
    Data,
    /// Strobe lands on the stop bit; it must read mark.
    Stop,
}

/// How hard a transition inside a character pulls the bit clock. Deliberately
/// gentle: acquisition is a hard reset on the start edge, so this only has to
/// absorb a small baud-rate mismatch, and a loose loop is what stops the clock
/// from chasing noise.
const CLOCK_TRACK_GAIN: f32 = 0.06;
/// Smoothing for the branch-separation confidence (≈0.25 s at 8 kHz). Reported
/// for display only — gating output on it would mute the decoder through every
/// fade, so that job belongs to [`RttyRx::lock`] instead.
const CONF_SMOOTH: f32 = 1.0 / 2000.0;
/// AFC loop gain per sample, and the widest tuning error it will chase.
const AFC_GAIN: f32 = 3.0e-4;
const AFC_LIMIT_HZ: f32 = 90.0;
/// AFC needs the integrator to be settled on a single tone: its phase is
/// meaningless while the window straddles a transition. Requiring the winning
/// branch to have held for a quarter of a bit excludes exactly that, and
/// unlike a magnitude threshold it keeps working when mistuning has knocked
/// both branches far down the matched filter's skirt.
const AFC_SETTLE: f32 = 0.25;
/// Floor on branch separation for AFC, low enough not to limit pull-in and
/// only there to keep the loop out of pure noise.
const AFC_SEP_MIN: f32 = 0.2;

/// Automatic threshold correction. Two-path HF fading is frequency selective:
/// with a 1 ms delay spread the channel has a null every kHz, so mark and space
/// — only a couple of hundred Hz apart — routinely arrive at very different
/// levels. Comparing raw envelopes then decides on the fade rather than on the
/// data. Each branch is instead referred to its own recent level *while that
/// tone was the one present*, which is a conditional average: it does not decay
/// away while its tone is absent, and it needs no absolute threshold.
const ATC_SMOOTH: f32 = 8.0e-3;
/// Rate at which a *stalled* branch's reference relaxes towards the active one.
///
/// A conditional average alone latches: once a tone fades far enough to stop
/// winning, its reference freezes at the pre-fade level and it can never win
/// again. Relaxing towards the winning branch is the way out, and it also stops
/// a long idle-mark run from leaving the space reference stale and biasing the
/// first character after it.
///
/// It must not run continuously, though. Doing so caps how far apart the two
/// references can get, and a static selective-fade notch — the 2 ms delay
/// spread case, where one tone sits permanently 20 dB down — needs exactly that
/// large a standing correction. Gating it on a branch having been quiet for
/// longer than any legitimate run of same-tone bits keeps the deep correction
/// available during data and still guarantees an escape.
const ATC_RELAX: f32 = ATC_SMOOTH / 4.0;
/// Bits a branch must go without winning before its reference starts relaxing.
/// A Baudot character can legitimately hold one tone for about eight bits
/// (`LTRS` plus its stop bits), so wait longer than that.
const ATC_IDLE_BITS: f32 = 12.0;
/// Bound the correction to about 16 dB; a deeper split is unreadable anyway.
const ATC_MIN_RATIO: f32 = 0.15;

/// Frame-validation lock. Every character whose start bit read space and whose
/// stop bit read mark counts up; every framing failure counts down harder.
///
/// This is the noise gate. Random noise clears the start check about half the
/// time and the stop check about half of what is left, so the counter drifts
/// firmly downwards and never reaches [`LOCK_EMIT`]; a real signal validates
/// essentially every frame and saturates within a few characters. Unlike a
/// level or confidence threshold it costs nothing in sensitivity, and it rides
/// through a fade instead of muting.
const LOCK_MAX: i32 = 6;
const LOCK_EMIT: i32 = 3;
/// Ceiling on characters held back waiting for lock. Without it a signal that
/// hovers just under the threshold would buffer for ever.
const PENDING_CAP: usize = 32;

/// Streaming RTTY receiver: per-tone matched filters, a transition-locked bit
/// clock, and a framing-validated UART.
pub struct RttyRx {
    rate: f64,
    center_hz: f64,
    baud: f64,
    shift_hz: f64,
    /// Swap the sense of the tones (the "Reverse"/RV control other RTTY
    /// programs offer). Needed whenever the signal is received on the opposite
    /// sideband from the one it was sent on, which flips mark and space.
    reverse: bool,
    afc_enabled: bool,

    // Front end: mix to the operator's centre tone, then band-limit.
    ph: f32,
    ph_inc: f32,
    lpf: ComplexFir,

    // Tone branches: rotate ±shift/2 to DC, integrate over one bit.
    tone_ph: f32,
    tone_inc: f32,
    mark_int: Integrator,
    space_int: Integrator,

    // Bit clock: a free-running NCO nudged by observed transitions.
    spb: f32,
    clk: f32,
    last_bit: bool,
    have_last: bool,

    // Framing.
    state: RxState,
    nbits: u8,
    value: u8,
    lock: i32,
    /// Characters decoded before the lock counter was convinced. Held rather
    /// than dropped, so a transmission that starts cleanly loses none of its
    /// opening characters while the gate makes up its mind.
    pending: String,

    // Quality / tracking.
    conf: f32,
    afc_hz: f32,
    prev_win: Complex32,
    prev_win_mark: bool,
    have_win: bool,
    /// Samples the current branch winner has held — AFC's settle gate.
    win_run: f32,
    /// Per-tone reference levels for automatic threshold correction, and how
    /// long each tone has gone without being the one present.
    ref_mark: f32,
    ref_space: f32,
    mark_idle: f32,
    space_idle: f32,
    mag: f32,
    baudot: BaudotRx,
}

impl RttyRx {
    pub fn new(rate: f64, center_hz: f64, baud: f64, shift_hz: f64) -> Self {
        let mut rx = RttyRx {
            rate,
            center_hz,
            baud,
            shift_hz,
            reverse: false,
            afc_enabled: true,
            ph: 0.0,
            ph_inc: 0.0,
            lpf: ComplexFir::new(Vec::new()),
            tone_ph: 0.0,
            tone_inc: 0.0,
            mark_int: Integrator::new(1),
            space_int: Integrator::new(1),
            spb: 1.0,
            clk: 0.0,
            last_bit: true,
            have_last: false,
            state: RxState::Hunt,
            nbits: 0,
            value: 0,
            lock: 0,
            pending: String::new(),
            conf: 0.0,
            afc_hz: 0.0,
            prev_win: Complex32::new(0.0, 0.0),
            prev_win_mark: false,
            have_win: false,
            win_run: 0.0,
            ref_mark: 0.0,
            ref_space: 0.0,
            mark_idle: 0.0,
            space_idle: 0.0,
            mag: 0.0,
            baudot: BaudotRx::new(),
        };
        rx.configure();
        rx
    }

    /// (Re)derive every rate-dependent quantity from centre/baud/shift.
    fn configure(&mut self) {
        self.spb = (self.rate / self.baud) as f32;
        let len = self.spb.round().max(1.0) as usize;
        self.mark_int.resize(len);
        self.space_int.resize(len);
        self.tone_inc = (TAU * (self.shift_hz / 2.0) / self.rate) as f32;
        self.tone_ph = 0.0;
        // Pass both tones plus the AFC pull-in range; the matched filters do the
        // real noise rejection, so this only has to keep neighbours out.
        let bw = (self.shift_hz / 2.0 + 2.0 * self.baud + AFC_LIMIT_HZ as f64 + 20.0).max(120.0);
        self.lpf.set_taps(bandpass_taps(129, -bw, bw, self.rate));
        self.lpf.reset();
        self.apply_afc();
        self.state = RxState::Hunt;
        self.have_last = false;
        self.have_win = false;
        self.win_run = 0.0;
        self.ref_mark = 0.0;
        self.ref_space = 0.0;
        self.mark_idle = 0.0;
        self.space_idle = 0.0;
        self.conf = 0.0;
        self.lock = 0;
        self.pending.clear();
    }

    fn apply_afc(&mut self) {
        let hz = self.center_hz + self.afc_hz as f64;
        self.ph_inc = (TAU * hz / self.rate) as f32;
    }

    pub fn set_tuning(&mut self, center_hz: f64, baud: f64, shift_hz: f64) {
        self.center_hz = center_hz;
        self.baud = baud;
        self.shift_hz = shift_hz;
        self.afc_hz = 0.0;
        self.configure();
    }

    /// Swap mark and space. A signal received on the wrong sideband arrives
    /// inverted, and without this it decodes as unbroken nonsense.
    pub fn set_reverse(&mut self, reverse: bool) {
        if self.reverse != reverse {
            self.reverse = reverse;
            self.state = RxState::Hunt;
            self.conf = 0.0;
        }
    }

    pub fn reverse(&self) -> bool {
        self.reverse
    }

    /// Enable automatic frequency correction. On by default; turning it off
    /// pins the decoder to exactly where the operator put the cursor.
    pub fn set_afc(&mut self, on: bool) {
        self.afc_enabled = on;
        if !on {
            self.afc_hz = 0.0;
            self.apply_afc();
        }
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    /// Current AFC pull in Hz — how far off the cursor the signal actually is.
    pub fn afc_offset_hz(&self) -> f32 {
        self.afc_hz
    }

    /// Smoothed tone separation in [0, 1]: how convincingly one branch is
    /// winning. Purely a display figure — see [`RttyRx::lock`] for the gate.
    pub fn confidence(&self) -> f32 {
        self.conf
    }

    /// Frame-lock confidence in [0, 1]. Output is suppressed until the framing
    /// has validated a few characters in a row, which is what keeps a receiver
    /// parked on noise silent.
    pub fn lock(&self) -> f32 {
        (self.lock as f32 / LOCK_EMIT as f32).min(1.0)
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            let z = Complex32::new(a * self.ph.cos(), -a * self.ph.sin());
            self.ph += self.ph_inc;
            if self.ph > std::f32::consts::TAU {
                self.ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        let hz_per_rad = (self.rate / TAU) as f32;

        for z in bb {
            // Rotate the two tones down to DC and integrate each over a bit.
            let (s, c) = (self.tone_ph.sin(), self.tone_ph.cos());
            self.tone_ph += self.tone_inc;
            if self.tone_ph > std::f32::consts::TAU {
                self.tone_ph -= std::f32::consts::TAU;
            }
            let m = self.mark_int.push(z * Complex32::new(c, -s));
            let sp = self.space_int.push(z * Complex32::new(c, s));
            let (em, es) = (m.norm(), sp.norm());

            let total = em + es;
            self.mag += 0.02 * (total - self.mag);
            let sep = if total > 1e-20 { (em - es).abs() / total } else { 0.0 };
            self.conf += CONF_SMOOTH * (sep - self.conf);

            // Automatic threshold correction: compare each branch against its
            // own recent level rather than against the other branch directly,
            // so a selective fade that leaves one tone 10 dB down still decodes.
            // Seed both references equal on the first sample carrying energy —
            // starting them at zero costs a character or two of transient while
            // they converge, which lands squarely on the front of a message.
            if self.ref_mark <= 0.0 && self.ref_space <= 0.0 && total > 1e-20 {
                self.ref_mark = total * 0.5;
                self.ref_space = total * 0.5;
            }
            let (dm, ds) = if self.ref_mark > 1e-20 && self.ref_space > 1e-20 {
                (em / self.ref_mark, es / self.ref_space)
            } else {
                (em, es)
            };

            // `phys_mark` is which tone is actually present (AFC cares about
            // that); `bit` is what it means after the reverse setting.
            let phys_mark = dm > ds;
            let bit = phys_mark != self.reverse;

            let idle_limit = ATC_IDLE_BITS * self.spb;
            if phys_mark {
                self.ref_mark += ATC_SMOOTH * (em - self.ref_mark);
                self.space_idle += 1.0;
                self.mark_idle = 0.0;
                if self.space_idle > idle_limit {
                    self.ref_space += ATC_RELAX * (self.ref_mark - self.ref_space);
                }
            } else {
                self.ref_space += ATC_SMOOTH * (es - self.ref_space);
                self.mark_idle += 1.0;
                self.space_idle = 0.0;
                if self.mark_idle > idle_limit {
                    self.ref_mark += ATC_RELAX * (self.ref_space - self.ref_mark);
                }
            }
            let floor = self.ref_mark.max(self.ref_space) * ATC_MIN_RATIO;
            self.ref_mark = self.ref_mark.max(floor);
            self.ref_space = self.ref_space.max(floor);

            if self.have_win && self.prev_win_mark == phys_mark {
                self.win_run += 1.0;
            } else {
                self.win_run = 0.0;
            }

            // AFC. The winning branch's integrator output is a phasor turning
            // at exactly the residual tuning error, already averaged over a
            // bit, so its phase advance *is* the error in Hz — no separate
            // discriminator, and nothing to misalign against the bit decision.
            let win = if phys_mark { m } else { sp };
            if self.afc_enabled && sep > AFC_SEP_MIN && self.win_run > AFC_SETTLE * self.spb {
                let d = win * self.prev_win.conj();
                if d.norm_sqr() > 1e-24 {
                    let err = (d.arg() * hz_per_rad).clamp(-AFC_LIMIT_HZ, AFC_LIMIT_HZ);
                    self.afc_hz = (self.afc_hz + AFC_GAIN * err).clamp(-AFC_LIMIT_HZ, AFC_LIMIT_HZ);
                    self.apply_afc();
                }
            }
            self.prev_win = win;
            self.prev_win_mark = phys_mark;
            self.have_win = true;

            self.advance_clock(bit, &mut out);
        }
        out
    }

    /// Charge a framing failure against the lock counter, discarding anything
    /// still held back once confidence in the framing is gone.
    fn fail_lock(&mut self, cost: i32) {
        self.lock = (self.lock - cost).max(0);
        if self.lock == 0 {
            self.pending.clear();
        }
    }

    /// One sample of the bit clock: track transitions, then act on a strobe.
    ///
    /// The matched filter's output crosses over half a bit after the signal's
    /// transition, and its window is exactly full of the new symbol half a bit
    /// after that — so an observed transition should sit at clock phase 0.5,
    /// and the strobe that follows lands on the bit centre.
    fn advance_clock(&mut self, bit: bool, out: &mut String) {
        if self.have_last && bit != self.last_bit {
            if self.state == RxState::Hunt {
                if self.last_bit && !bit {
                    // Mark→space: a candidate start bit. Acquire hard.
                    self.clk = 0.5;
                    self.state = RxState::Start;
                    self.nbits = 0;
                    self.value = 0;
                }
            } else {
                let mut e = 0.5 - self.clk;
                if e > 0.5 {
                    e -= 1.0;
                } else if e < -0.5 {
                    e += 1.0;
                }
                self.clk = (self.clk + CLOCK_TRACK_GAIN * e).rem_euclid(1.0);
            }
        }
        self.last_bit = bit;
        self.have_last = true;

        if self.state == RxState::Hunt {
            return;
        }
        self.clk += 1.0 / self.spb;
        if self.clk < 1.0 {
            return;
        }
        self.clk -= 1.0;

        match self.state {
            RxState::Hunt => {}
            RxState::Start => {
                // A real start bit is still space at its centre; a noise edge
                // usually is not.
                if bit {
                    self.fail_lock(1);
                    self.state = RxState::Hunt;
                } else {
                    self.state = RxState::Data;
                }
            }
            RxState::Data => {
                if bit {
                    self.value |= 1 << self.nbits;
                }
                self.nbits += 1;
                if self.nbits >= 5 {
                    self.state = RxState::Stop;
                }
            }
            RxState::Stop => {
                // Mark stop bit or the character never happened. This check is
                // what stops noise during the 1.5-bit stop period from
                // manufacturing a character every time.
                if bit {
                    self.lock = (self.lock + 1).min(LOCK_MAX);
                    if let Some(c) = self.baudot.decode(self.value) {
                        if self.lock >= LOCK_EMIT {
                            out.push_str(&self.pending);
                            self.pending.clear();
                            out.push(c);
                        } else {
                            if self.pending.chars().count() >= PENDING_CAP {
                                self.pending.remove(0);
                            }
                            self.pending.push(c);
                        }
                    }
                } else {
                    self.fail_lock(2);
                }
                self.state = RxState::Hunt;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baudot_roundtrip() {
        for c in 'A'..='Z' {
            let v = letter_code(c).unwrap();
            assert_eq!(letter_char(v), Some(c));
        }
        for (v, _) in (0u8..32).filter_map(|v| figure_char(v).map(|c| (v, c))) {
            let c = figure_char(v).unwrap();
            assert_eq!(figure_code(c), Some(v));
        }
    }

    #[test]
    fn loopback() {
        let rate = 8000.0;
        let center = 1000.0;
        let (baud, shift) = (45.45, 170.0);
        let mut tx = RttyTx::new(rate, center, baud, shift);
        let mut audio = Vec::new();
        // Idle mark preamble.
        let mut pre = [0.0f32; 2000];
        tx.next_block(&mut pre);
        audio.extend_from_slice(&pre);

        let msg = "CQ CQ DE AB1CD K";
        tx.push_text(msg);
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 4000 {
            let mut b = [0.0f32; 2000];
            tx.next_block(&mut b);
            audio.extend_from_slice(&b);
            guard += 1;
        }
        let mut tail = [0.0f32; 2000];
        tx.next_block(&mut tail);
        audio.extend_from_slice(&tail);

        let mut rx = RttyRx::new(rate, center, baud, shift);
        let mut decoded = String::new();
        for chunk in audio.chunks(400) {
            decoded.push_str(&rx.process(chunk));
        }
        assert!(decoded.contains(msg), "decoded {decoded:?} did not contain {msg:?}");
        assert_eq!(tx.sent_chars(), tx.total_chars());
    }
    /// What the decoder makes of a real transmission.
    ///
    /// `SDROXIDE_RTTY_SAMPLE` is a mono 8 kHz WAV of an RTTY signal;
    /// `SDROXIDE_RTTY_CENTRE` (default 1700 Hz), `_BAUD` (45.45) and `_SHIFT`
    /// (170 Hz) describe it, and `_TEXT` asserts on the result when set.
    ///
    /// Run it with
    /// `SDROXIDE_RTTY_SAMPLE=... cargo test -p sdroxide-dsp --release --lib \\
    ///   -- --ignored --nocapture an_off_air_capture_decodes`.
    #[test]
    #[ignore]
    fn an_off_air_capture_decodes() {
        let Ok(path) = std::env::var("SDROXIDE_RTTY_SAMPLE") else {
            eprintln!("SDROXIDE_RTTY_SAMPLE unset; skipping");
            return;
        };
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 8000, "the sample must be at 8 kHz");
        assert_eq!(spec.channels, 1, "the sample must be mono");
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect();
        let num = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        let centre = num("SDROXIDE_RTTY_CENTRE", 1700.0);
        let baud = num("SDROXIDE_RTTY_BAUD", 45.45);
        let shift = num("SDROXIDE_RTTY_SHIFT", 170.0);
        let mut rx = RttyRx::new(8000.0, centre, baud, shift);
        if std::env::var("SDROXIDE_RTTY_REVERSE").is_ok() {
            rx.set_reverse(true);
        }
        let mut out = String::new();
        for chunk in audio.chunks(512) {
            out.push_str(&rx.process(chunk));
        }
        eprintln!(
            "decoded {} chars at {baud} baud, {shift} Hz shift, {centre} Hz: {out:?}",
            out.len()
        );
        if let Ok(want) = std::env::var("SDROXIDE_RTTY_TEXT") {
            for w in want.split_whitespace() {
                assert!(out.contains(w), "off-air RTTY did not recover {w:?}: {out:?}");
            }
        }
    }
}
