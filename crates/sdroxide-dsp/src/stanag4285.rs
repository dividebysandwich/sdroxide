//! STANAG 4285 — the receive side, uncoded rates.
//!
//! Transcribed from the standard itself (`STANAG4285-EDITION1.md`) rather than
//! from somebody's description of it, and **receive-only**: no modulator, no
//! `Mode` variant, no wire, no `PROTO_VERSION`. The three uncoded data rates —
//! 1200 BPSK, 2400 QPSK, 3600 8PSK — are what this decodes. The coded rates are
//! Annex E and are not built, which is said here so that "all eight phases" is
//! never read as a claim to decode them.
//!
//! The frame, in full, because every one of these matters:
//!
//! * **256 symbols at 2400 baud** = 106.67 ms: **80 synchronisation** symbols,
//!   then 176 scrambled ones — three groups of `32 data + 16 reference`, then a
//!   final 32 data symbols with no reference after them.
//! * The synchronisation is **2PSK**, PN31 (two periods plus the first 18
//!   symbols), generator `x⁵ + x² + 1`, register set to `11010` at the start of
//!   every frame, **first symbol the LSB**, then 79 clocks.
//! * The 176 reference and data symbols are **scrambled** by `x⁹ + x⁴ + 1`, set
//!   to 1 each frame, **three bits per symbol**: `n = 4x₂ + 2x₁ + x₀` and
//!   `Bk = e^(jnπ/4)`, the register advancing three positions per symbol.
//! * The reference symbols are **all symbol 0 before scrambling**, which is the
//!   handle the whole receiver hangs on.
//!
//! Why the detector is differential, and this is the one design decision that was
//! measured rather than derived: an 80-symbol coherent integration is 33 ms, and
//! the standard allows ±75 Hz of carrier error — 30 Hz turns a whole circle
//! inside that window. Correlating the *products* of adjacent sync symbols
//! removes the unknown offset before the correlation ever sees it. Measured at
//! 0.55–0.61 against a coherent 0.40, which is the sequence's own bias.
//!
//! What this deliberately does not do: a single peak is never a lock. The
//! correlation is a maximum over every offset, so on 60 s of white noise it
//! reaches 0.738 — measured — and reading that as a detection is how an empty
//! channel produced a 0.900. A frame is accepted only when the peak **recurs one
//! frame period later**, and every threshold is read against a running noise
//! estimate rather than an absolute.

use crate::rtty::BaudotRx;
use std::f32::consts::TAU;

/// Symbol rate, fixed by the standard.
pub const BAUD: f64 = 2400.0;
/// Symbols per frame.
pub const FRAME: usize = 256;
/// Synchronisation symbols at the head of every frame.
pub const SYNC: usize = 80;
/// Scrambled symbols: 176 = 3 × (32 data + 16 reference) + 32 data.
pub const SCRAMBLED: usize = FRAME - SYNC;
/// Samples per symbol at the modem rate.
pub const SPS: usize = 5;
/// The modem's internal sample rate: five samples per symbol.
pub const MODEM_RATE: f64 = BAUD * SPS as f64;
/// The reference blocks, as offsets into the 176 scrambled symbols.
pub const REF_BLOCKS: [(usize, usize); 3] = [(32, 48), (80, 96), (128, 144)];
/// The data blocks, as offsets into the 176 scrambled symbols.
pub const DATA_BLOCKS: [(usize, usize); 4] = [(0, 32), (48, 80), (96, 128), (144, 176)];

/// A complex value as a pair, so this module needs no extra dependency.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct C {
    pub re: f32,
    pub im: f32,
}

impl C {
    pub fn new(re: f32, im: f32) -> Self {
        C { re, im }
    }
    pub fn mul(self, o: C) -> C {
        C::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    pub fn conj(self) -> C {
        C::new(self.re, -self.im)
    }
    pub fn norm(self) -> f32 {
        (self.re * self.re + self.im * self.im).sqrt()
    }
    pub fn scale(self, k: f32) -> C {
        C::new(self.re * k, self.im * k)
    }
    pub fn add(self, o: C) -> C {
        C::new(self.re + o.re, self.im + o.im)
    }
    pub fn sub(self, o: C) -> C {
        C::new(self.re - o.re, self.im - o.im)
    }
    pub fn from_phase(a: f32) -> C {
        C::new(a.cos(), a.sin())
    }
    pub fn arg(self) -> f32 {
        self.im.atan2(self.re)
    }
}

/// The data rate a frame carries, as far as its constellation can say.
///
/// Uncoded 3600 and **every coded rate** use all eight phases, so they cannot be
/// told apart at this layer. The report says both rather than guessing, and that
/// is the honest ceiling of a constellation test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S4285Rate {
    /// 1200 bps, one bit per symbol, 2PSK: symbols {0, 4}.
    Bps1200,
    /// 2400 bps, a dibit per symbol, QPSK: symbols {0, 2, 4, 6}.
    Bps2400,
    /// 3600 bps uncoded (8PSK), or any coded rate.
    Bps3600OrCoded,
    /// Not yet decided — a frame whose references did not collapse.
    Unknown,
}

impl S4285Rate {
    pub fn label(self) -> &'static str {
        match self {
            S4285Rate::Bps1200 => "1200 BPSK (uncoded)",
            S4285Rate::Bps2400 => "2400 QPSK (uncoded)",
            S4285Rate::Bps3600OrCoded => "3600 8PSK (uncoded) or a coded rate",
            S4285Rate::Unknown => "unknown",
        }
    }

    /// Bits carried per symbol.
    pub fn bits_per_symbol(self) -> usize {
        match self {
            S4285Rate::Bps1200 => 1,
            S4285Rate::Bps2400 => 2,
            _ => 3,
        }
    }
}

/// One demodulated frame.
#[derive(Debug, Clone, PartialEq)]
pub struct S4285Frame {
    /// The three reference blocks' concentrations after descrambling, in order.
    /// **Reported separately and never averaged**: a channel that rotates within
    /// one frame separates the blocks while leaving each coherent, and an average
    /// hides exactly that — it is why an earlier single-point test declined files
    /// that do carry a frame.
    pub reference: [f32; 3],
    /// Carrier offset the reference blocks agree on, in hertz.
    pub carrier_offset_hz: f32,
    /// Which scrambler sign the frame needed.
    pub reversed: bool,
    /// What the descrambled constellation says.
    pub rate: S4285Rate,
    /// The distinct phases the descrambled data symbols used, 0..8. The verdict
    /// is this count and nothing else: 2 → 1200, 4 → 2400, 8 → 3600/coded.
    pub phases_used: usize,
    /// Payload bits of this frame, oldest first.
    pub bits: Vec<u8>,
}

/// Something the receiver produced.
#[derive(Debug, Clone, PartialEq)]
pub enum S4285Event {
    /// A frame was demodulated.
    Frame(S4285Frame),
    /// Text from the character layer, ITA2 5N1. The standard's payload is a
    /// synchronous bit stream, so this is a best reading of it — the raw bits are
    /// on the frame event for anything that is not ITA2.
    Text(String),
}

/// The receiver's standing state, for a panel or a probe.
#[derive(Debug, Clone, PartialEq)]
pub struct S4285Report {
    /// A frame is being tracked.
    pub locked: bool,
    /// The differential sync correlation at the lock, 0..1.
    pub sync_quality: f32,
    /// The running noise estimate the correlation is judged against.
    pub correlation_floor: f32,
    /// The frame-period autocorrelation at the lock: how much of a frame repeats
    /// one frame later. **This is the detector's real discriminator.** A correct
    /// 4285 measures about **0.31-0.35** — only the 80 sync symbols repeat, the
    /// other 176 are random — while a strong PSK carrier that is a different mode
    /// measures about **0.03**. Measured off the air, and the reason a sync
    /// correlation alone cannot tell the two apart.
    pub frame_acf: f32,
    /// Carrier offset measured from the reference blocks, in hertz.
    pub carrier_offset_hz: f32,
    /// Frames demodulated since the last reset.
    pub frames: u64,
    /// The last frame's rate verdict.
    pub rate: S4285Rate,
    /// The last frame's three reference concentrations.
    pub reference: [f32; 3],
}

impl Default for S4285Report {
    fn default() -> Self {
        S4285Report {
            locked: false,
            sync_quality: 0.0,
            correlation_floor: 0.0,
            frame_acf: 0.0,
            carrier_offset_hz: 0.0,
            frames: 0,
            rate: S4285Rate::Unknown,
            reference: [0.0; 3],
        }
    }
}

/// A Fibonacci LFSR, least-significant bit first, as both of the standard's
/// generators are specified: the output is `x₀`, the feedback is the XOR of the
/// tapped stages, and the register shifts one position per output bit.
fn lfsr(stages: usize, taps: &[usize], n: usize, init: u32) -> Vec<u8> {
    // The stage count is the polynomial's degree, **not** one more than its
    // highest tap. `x⁵ + x² + 1` is five stages with taps {0, 2} and `x⁹ + x⁴ + 1`
    // is nine with taps {0, 4}; giving either a stage too many makes the register
    // empty out and stay at zero, which produced a sync sequence of period 16 and
    // a scrambler that was a constant — and the references then could not
    // collapse, which read as "no frame".
    let mut state: Vec<u8> = (0..stages).map(|i| ((init >> i) & 1) as u8).collect();
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(state[0]);
        let mut feedback = 0;
        for &t in taps {
            feedback ^= state[t];
        }
        state.remove(0);
        state.push(feedback);
    }
    out
}

/// The 80-symbol synchronisation sequence as ±1, PN31 clocked from `11010`.
pub fn sync_symbols() -> [f32; SYNC] {
    let bits = lfsr(5, &[0, 2], SYNC, 0b11010);
    let mut out = [0.0f32; SYNC];
    for (o, b) in out.iter_mut().zip(&bits) {
        *o = if *b == 1 { 1.0 } else { -1.0 };
    }
    out
}

/// The products of adjacent sync symbols: what a differential detector produces,
/// so this is what the correlator has to be matched to.
pub fn sync_differential() -> [f32; SYNC - 1] {
    let s = sync_symbols();
    let mut out = [0.0f32; SYNC - 1];
    for i in 0..SYNC - 1 {
        out[i] = s[i + 1] * s[i];
    }
    out
}

/// The 176-symbol scrambling sequence, `Bk = e^(jnπ/4)` with
/// `n = 4x₂ + 2x₁ + x₀`, the register advancing three positions per symbol.
pub fn scrambling() -> [C; SCRAMBLED] {
    let bits = lfsr(9, &[0, 4], SCRAMBLED * 3, 1);
    let mut out = [C::default(); SCRAMBLED];
    for (k, o) in out.iter_mut().enumerate() {
        let n = 4 * bits[3 * k + 2] as i32 + 2 * bits[3 * k + 1] as i32 + bits[3 * k] as i32;
        *o = C::from_phase(n as f32 * std::f32::consts::FRAC_PI_4);
    }
    out
}

/// A windowed-sinc low-pass at the input rate, applied **before** any resampling.
/// Filtering first is not tidiness: interpolating a full-band signal folds the
/// rest of the band into the channel.
struct Lowpass {
    taps: Vec<f32>,
    buf: Vec<C>,
    pos: usize,
}

impl Lowpass {
    fn new(rate: f64, cutoff_hz: f64, len: usize) -> Self {
        let mid = (len / 2) as isize;
        let fc = (cutoff_hz / rate) as f32;
        let mut taps = Vec::with_capacity(len);
        let mut sum = 0.0f32;
        for i in 0..len {
            let x = i as isize - mid;
            let sinc = if x == 0 {
                2.0 * fc
            } else {
                (TAU as f32 * fc * x as f32).sin() / (std::f32::consts::PI * x as f32)
            };
            let w = 0.54 - 0.46 * ((TAU as f32) * i as f32 / (len - 1) as f32).cos();
            taps.push(sinc * w);
            sum += sinc * w;
        }
        for t in taps.iter_mut() {
            *t /= sum;
        }
        Lowpass { taps, buf: vec![C::default(); len], pos: 0 }
    }

    fn push(&mut self, z: C) -> C {
        let n = self.taps.len();
        self.buf[self.pos] = z;
        let mut acc = C::default();
        let mut k = self.pos;
        for &t in &self.taps {
            acc = acc.add(self.buf[k].scale(t));
            k = if k + 1 == n { 0 } else { k + 1 };
        }
        self.pos = if self.pos + 1 == n { 0 } else { self.pos + 1 };
        acc
    }
}

/// The receiver. Feed it audio at `rate_hz`; it emits a frame per frame once it
/// has locked, and text as the character layer decodes.
pub struct S4285Receiver {
    rate: f64,
    carrier: f64,
    nco: f32,
    nco_inc: f32,
    lpf: Lowpass,
    step: f64,
    frac: f64,
    prev_bb: C,
    have_prev: bool,
    /// Symbols at the modem rate, the newest last, capped at four frames.
    syms: Vec<C>,
    /// Differential symbols, aligned with `syms` (one fewer at the head).
    diff: Vec<C>,
    /// The differential correlation at each symbol, aligned with `diff`.
    corr: Vec<f32>,
    /// A creeping floor the correlation is read against.
    floor: f32,
    /// Symbols still to wait before the next frame head, once locked.
    countdown: Option<usize>,
    scrambling: [C; SCRAMBLED],
    sync_diff: [f32; SYNC - 1],
    baudot: BaudotRx,
    bit_run: Vec<u8>,
    /// Characters decoded but not yet handed out.
    events_text: String,
    report: S4285Report,
}

impl S4285Receiver {
    /// `rate_hz` is the audio tap rate; `carrier_hz` is the audio frequency of the
    /// suppressed carrier — 1800 Hz when the dial is on it, but a fixed-frequency
    /// log entry can be a kilohertz out, so it is a parameter.
    pub fn new(rate_hz: f64, carrier_hz: f64) -> Self {
        S4285Receiver {
            rate: rate_hz,
            carrier: carrier_hz,
            nco: 0.0,
            nco_inc: (TAU as f32) * carrier_hz as f32 / rate_hz as f32,
            lpf: Lowpass::new(rate_hz, 1500.0, 129),
            step: rate_hz / MODEM_RATE,
            frac: 0.0,
            prev_bb: C::default(),
            have_prev: false,
            syms: Vec::with_capacity(4 * FRAME),
            diff: Vec::with_capacity(4 * FRAME),
            corr: Vec::with_capacity(4 * FRAME),
            floor: 0.0,
            countdown: None,
            scrambling: scrambling(),
            sync_diff: sync_differential(),
            baudot: BaudotRx::new(),
            bit_run: Vec::new(),
            events_text: String::new(),
            report: S4285Report::default(),
        }
    }

    pub fn report(&self) -> &S4285Report {
        &self.report
    }

    /// The raw payload bits recovered so far, oldest first — exposed because a
    /// transmission that is not ITA2 must be visible as bits, not as silence.
    pub fn bits(&self) -> &[u8] {
        &self.bit_run
    }

    pub fn carrier_hz(&self) -> f64 {
        self.carrier
    }

    /// Retune the suppressed carrier and start again: acquisition cannot survive a
    /// jump, and a stale lock on a new frequency would be a lie.
    pub fn set_carrier(&mut self, carrier_hz: f64) {
        self.carrier = carrier_hz;
        self.nco_inc = (TAU as f32) * carrier_hz as f32 / self.rate as f32;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.nco = 0.0;
        self.frac = 0.0;
        self.have_prev = false;
        self.syms.clear();
        self.diff.clear();
        self.corr.clear();
        self.floor = 0.0;
        self.countdown = None;
        self.bit_run.clear();
        self.report = S4285Report::default();
    }

    /// Feed audio; returns whatever it produced.
    pub fn process(&mut self, audio: &[f32]) -> Vec<S4285Event> {
        let mut events = Vec::new();
        let mut bb: Vec<C> = Vec::with_capacity((audio.len() as f64 / self.step) as usize + 2);
        for &s in audio {
            let lo = C::from_phase(-self.nco);
            self.nco += self.nco_inc;
            if self.nco > TAU as f32 {
                self.nco -= TAU as f32;
            }
            let z = self.lpf.push(C::new(s, 0.0).mul(lo));
            if !self.have_prev {
                self.prev_bb = z;
                self.have_prev = true;
                continue;
            }
            while self.frac < 1.0 {
                let t = self.frac as f32;
                bb.push(self.prev_bb.scale(1.0 - t).add(z.scale(t)));
                self.frac += self.step;
            }
            self.frac -= 1.0;
            self.prev_bb = z;
        }

        for chunk in bb.chunks_exact(SPS) {
            let sym = chunk[0];
            if let Some(prev) = self.syms.last().copied() {
                self.diff.push(sym.mul(prev.conj()));
                self.correlate();
            }
            self.syms.push(sym);
            while self.syms.len() > 4 * FRAME {
                self.syms.remove(0);
            }
            while self.diff.len() > 4 * FRAME {
                self.diff.remove(0);
                self.corr.remove(0);
            }
            if let Some(n) = self.countdown {
                if n == 0 {
                    if let Some(frame) = self.take_frame() {
                        events.push(S4285Event::Frame(frame.clone()));
                        for value in frame.bits.chunks(5) {
                            let mut v = 0u8;
                            for (i, b) in value.iter().enumerate() {
                                v |= b << i;
                            }
                            if let Some(ch) = self.baudot.decode(v) {
                                self.events_text.push(ch);
                            }
                        }
                        if !self.events_text.is_empty() {
                            let t: String = self.events_text.drain(..).collect();
                            events.push(S4285Event::Text(t));
                        }
                    }
                    self.countdown = Some(FRAME - 1);
                } else {
                    self.countdown = Some(n - 1);
                }
            } else {
                self.try_lock();
            }
        }
        events
    }

    /// One differential symbol against the sequence, and the recurrence test that
    /// makes a peak mean anything.
    fn correlate(&mut self) {
        let n = self.sync_diff.len();
        let len = self.diff.len();
        let mut score = 0.0f32;
        if len >= n {
            let start = len - n;
            let mut acc = C::default();
            let mut mag = 0.0f32;
            for (k, &w) in self.sync_diff.iter().enumerate() {
                let z = self.diff[start + k];
                acc = acc.add(z.scale(w));
                mag += z.norm();
            }
            if mag > 1e-12 {
                score = acc.norm() / mag;
            }
        }
        self.corr.push(score);
        // A creeping floor, not a fixed threshold: the sequence's own bias is
        // ~0.2-0.4 and a maximum over every offset on noise reaches 0.738.
        self.floor = if self.floor == 0.0 { score } else { self.floor * 0.9995 + score * 0.0005 };
        self.report.correlation_floor = self.floor;
        // The ACF is the number that says whether a *frame* is there, so it is
        // reported whether or not a lock was accepted — a file that does not lock
        // has to say why, and "ACF 0.03" is a different answer from "no carrier".
        if self.diff.len() % 16 == 0 {
            self.report.frame_acf = self.frame_acf();
        }
    }

    /// How much of the last two frames repeats one frame later.
    ///
    /// Normalised so it is independent of level: `|sum s[k] conj(s[k-256])|` over
    /// the geometric mean of the two energies. A correct 4285 repeats only its 80
    /// sync symbols, so the ceiling is about 80/256 = 0.31 plus the correlation of
    /// the random part; a different mode repeats nothing and sits near zero.
    fn frame_acf(&self) -> f32 {
        let n = self.syms.len();
        if n < FRAME + 1 {
            return 0.0;
        }
        let take = (n - FRAME).min(FRAME);
        let mut acc = C::default();
        let (mut ea, mut eb) = (0.0f32, 0.0f32);
        for k in 0..take {
            let a = self.syms[n - take - FRAME + k];
            let b = self.syms[n - take + k];
            acc = acc.add(b.mul(a.conj()));
            ea += a.norm() * a.norm();
            eb += b.norm() * b.norm();
        }
        let denom = (ea * eb).sqrt();
        if denom <= 1e-20 {
            return 0.0;
        }
        (acc.norm() / denom).min(1.0)
    }

    fn try_lock(&mut self) {
        let l = self.corr.len();
        if l < FRAME + 1 {
            return;
        }
        let now = self.corr[l - 1];
        let before = self.corr[l - 1 - FRAME];
        let floor = self.floor.max(0.15);
        if now > 0.45 && now > floor * 1.6 && before > floor * 1.4 {
            // ...but a correlation that recurs is still not proof. The frame
            // period repeats only because 80 of its 256 symbols are the sync
            // sequence, which is 0.31-0.35 of it; a different mode with a strong
            // carrier measures ~0.03. Gate on that, not on the correlation.
            let acf = self.frame_acf();
            self.report.frame_acf = acf;
            if acf < MIN_FRAME_ACF {
                return;
            }
            self.report.locked = true;
            self.report.sync_quality = now;
            // The correlation's window ends on the last sync symbol, so the next
            // frame head is 176 symbols after this one.
            self.countdown = Some(FRAME - SYNC - 1);
        }
    }

    /// Take the frame that ends at the newest symbol and demodulate it.
    fn take_frame(&mut self) -> Option<S4285Frame> {
        if self.syms.len() < FRAME {
            return None;
        }
        let start = self.syms.len() - FRAME;
        let frame: Vec<C> = self.syms[start..].to_vec();
        // The scrambled part, with both scrambler signs tried: a receiver on the
        // opposite sideband sees the conjugate sequence, and picking the sign that
        // collapses the references costs nothing.
        let mut best: Option<(f32, bool, f32, [C; SCRAMBLED])> = None;
        for &sign in &[1.0f32, -1.0f32] {
            let mut z = [C::default(); SCRAMBLED];
            for k in 0..SCRAMBLED {
                let s = self.scrambling[k];
                let s = if sign > 0.0 { s } else { s.conj() };
                z[k] = frame[SYNC + k].mul(s.conj());
            }
            // The residual carrier offset has to be taken out **inside** each
            // block before the concentration can mean anything. A ramp spreads a
            // 16-symbol block across the circle, which is why three numbers that
            // should read 0.7-0.9 read 0.3 on a perfectly good capture — the trap
            // the probe's README recorded and this reproduces exactly.
            let mut hz = -150.0f32;
            while hz <= 150.0 {
                let c = ramp_concentrations(&z, hz);
                let r = (c[0] + c[1] + c[2]) / 3.0;
                if best.as_ref().is_none_or(|(b, _, _, _)| r > *b) {
                    best = Some((r, sign < 0.0, hz, z));
                }
                hz += 1.0;
            }
        }
        let (score, reversed, offset, z) = best?;
        // A frame whose references do not collapse is not a frame. This is the
        // gate that keeps a mistuned or mis-scrambled block from being reported
        // as a demodulation; the three blocks are printed whatever it decides.
        if score < MIN_REFERENCE {
            return None;
        }
        let reference = ramp_concentrations(&z, offset);

        // Rate from the descrambled data symbols only — scrambling makes them
        // uniform otherwise, which would read as 8PSK on every frame.
        let mut used = [false; 8];
        let mut data: Vec<C> = Vec::with_capacity(128);
        for &(lo, hi) in &DATA_BLOCKS {
            for k in lo..hi {
                let s = z[k].mul(C::from_phase(-offset * TAU * k as f32 / BAUD as f32));
                let s = &s;
                let idx = (s.arg() / std::f32::consts::FRAC_PI_4).round() as i32;
                let idx = idx.rem_euclid(8) as usize;
                used[idx] = true;
                data.push(*s);
            }
        }
        let phases_used = used.iter().filter(|u| **u).count();
        let rate = match phases_used {
            0 | 1 => S4285Rate::Unknown,
            2 => S4285Rate::Bps1200,
            3 | 4 => S4285Rate::Bps2400,
            _ => S4285Rate::Bps3600OrCoded,
        };
        let bits = transcode(&data, rate);

        self.report.frames += 1;
        self.report.rate = rate;
        self.report.reference = reference;
        self.report.carrier_offset_hz = offset;
        self.bit_run.extend_from_slice(&bits);
        if self.bit_run.len() > 8192 {
            let cut = self.bit_run.len() - 8192;
            self.bit_run.drain(..cut);
        }
        Some(S4285Frame { reference, carrier_offset_hz: offset, reversed, rate, phases_used, bits })
    }

    /// Characters decoded but not yet handed out.
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.events_text)
    }
}

/// The mean reference concentration a block needs before it is called a frame.
///
/// Sixteen symbols of noise give `1/sqrt(16)` = 0.25, and a real frame measured
/// off the air gives 0.7-0.9 per block. 0.35 sits between them and is deliberately
/// nearer the noise floor than the signal: a frame that is wrongly accepted is
/// visible in the three numbers, while one wrongly rejected is silence.
const MIN_REFERENCE: f32 = 0.35;

/// The frame-period autocorrelation a lock needs.
///
/// A correct 4285 measures 0.31-0.35 (only its 80 sync symbols repeat out of 256)
/// and a strong PSK carrier of another mode measures ~0.03, both measured. 0.12
/// sits between them with room for a fading capture: the night-time file that
/// started this work measures 0.347 across its whole 36 s.
const MIN_FRAME_ACF: f32 = 0.12;

/// The three reference blocks' concentrations with a carrier offset removed
/// inside each block, and their mean in slot 0 of the return? No — the mean is
/// returned separately by the caller; this returns the three.
fn ramp_concentrations(z: &[C], hz: f32) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (i, &(lo, hi)) in REF_BLOCKS.iter().enumerate() {
        let mut sum = C::default();
        let mut mag = 0.0f32;
        for k in lo..hi {
            let t = k as f32 / BAUD as f32;
            let c = z[k].mul(C::from_phase(-hz * TAU * t));
            sum = sum.add(c);
            mag += c.norm();
        }
        out[i] = if mag > 1e-12 { (sum.norm() / mag).min(1.0) } else { 0.0 };
    }
    out
}

/// Symbol phases to payload bits, oldest first, by Annex A's tables.
///
/// The symbols are descrambled and quantised to the nearest of eight phases, and
/// the table is chosen by the rate the constellation implied. At
/// [`S4285Rate::Bps3600OrCoded`] the 8PSK table is used, which is right for the
/// uncoded 3600 rate and is a best-effort tribit split for the coded ones — the
/// raw bits are exposed so that a coded frame is visible as wrong bits rather
/// than as silence.
fn transcode(data: &[C], rate: S4285Rate) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data.len() * 3);
    for z in data {
        let idx = ((z.arg() / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as u8;
        match rate {
            S4285Rate::Bps1200 => bits.push(u8::from(idx != 0)),
            S4285Rate::Bps2400 => match idx {
                0 => bits.extend_from_slice(&[0, 0]),
                2 => bits.extend_from_slice(&[0, 1]),
                6 => bits.extend_from_slice(&[1, 0]),
                _ => bits.extend_from_slice(&[1, 1]),
            },
            _ => match idx {
                1 => bits.extend_from_slice(&[0, 0, 0]),
                0 => bits.extend_from_slice(&[0, 0, 1]),
                2 => bits.extend_from_slice(&[0, 1, 0]),
                3 => bits.extend_from_slice(&[0, 1, 1]),
                6 => bits.extend_from_slice(&[1, 0, 0]),
                7 => bits.extend_from_slice(&[1, 0, 1]),
                5 => bits.extend_from_slice(&[1, 1, 0]),
                _ => bits.extend_from_slice(&[1, 1, 1]),
            },
        }
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both generators are the standard's, and both were got wrong once: a
    /// register given one stage too many empties out and stays at zero, which
    /// made the sync sequence periodic with 16 and the scrambler a constant.
    /// Nothing failed loudly — the sync correlated *better* against the
    /// degenerate pattern and the references simply never collapsed, which read
    /// as "no frame in this file".
    #[test]
    fn the_generators_are_maximal_and_match_the_standard() {
        let sync = sync_symbols();
        // PN31: two periods plus the first 18, and the period really is 31.
        let bits: Vec<i8> = sync.iter().map(|v| if *v > 0.0 { 1 } else { 0 }).collect();
        assert_eq!(&bits[..31], &bits[31..62], "the sync is not periodic with 31");
        assert_ne!(&bits[..31], &bits[16..47], "the sync has a shorter period than 31");
        // The first ten symbols, from the register `11010` clocked as specified.
        assert_eq!(&bits[..10], &[0, 1, 0, 1, 1, 0, 0, 1, 1, 1], "sync head");

        let scr = scrambling();
        // A 511-length maximal sequence: over one frame's worth of symbols all
        // eight phases appear, and the sequence is not constant.
        let mut seen = [false; 8];
        for z in scr {
            let n = ((z.arg() / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as usize;
            seen[n] = true;
        }
        assert!(seen.iter().filter(|s| **s).count() >= 7, "the scrambler is not maximal");
        // The first symbol of the sequence: the register is all zeros but its
        // lowest stage, so n = 1 and the phase is one step.
        assert_eq!(((scr[0].arg() / std::f32::consts::FRAC_PI_4).round() as i32) & 7, 1);
    }
}
