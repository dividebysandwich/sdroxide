//! STANAG 4285 — the receive side, uncoded rates.
//!
//! Transcribed from the standard itself (`STANAG4285-EDITION1.md`) rather than
//! from somebody's description of it, and **receive-only**: no modulator, and the
//! coded rates (Annex E) are not built.
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
//! ## Acquisition, and why it is a fold and not a peak
//!
//! The synchronisation sequence repeats every frame, so the correlation of the
//! received differential stream against the differentially encoded PN31 is a
//! **periodic** statistic: the right question is not "is there a peak here" but
//! "at which of the 256 symbol phases does this average over *many* frames stand
//! up". Measured on the night's captures: a single peak reaches 0.86 and so does
//! a strong PSK carrier of another mode (0.86 against 0.14 for the control when
//! the correlation is **folded over twelve frames**), while one frame's maximum
//! over every offset reaches 0.738 on white noise.
//!
//! Two more things have to be measured rather than assumed, and both were the
//! difference between 0.28 and 0.70 on the reference blocks:
//!
//! * **the sampling instant.** A receiver that resamples at a fixed phase lands
//!   anywhere inside the symbol. The fold is therefore searched over ten
//!   sub-symbol phases, and each frame refines its own timing again.
//! * **the carrier offset inside a block.** 16 symbols is 6.7 ms, so 150 Hz turns
//!   a whole circle inside one reference block and a perfectly good frame
//!   measures 0.3 instead of 0.9. It is corrected per block by a search, and the
//!   same correction is applied to the data.
//!
//! ## What this deliberately does not claim
//!
//! The three uncoded rates are 1200 BPSK, 2400 QPSK and 3600 8PSK, and **each of
//! those constellations is also used by coded rates** (1200 BPSK carries the
//! 75–600 bps rates at 1/16…1/2, QPSK carries 1200 at 1/2, 8PSK carries 2400 at
//! 2/3). A constellation test cannot tell them apart, so the rate is reported as
//! "uncoded, or a coded rate" and never as a claim. The coded payload needs
//! Annex E's FEC and interleaver, which are not transcribed, so the text of a
//! coded transmission is not readable here — and the payload bits are exposed so
//! that shows as bits rather than as silence.

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

/// Frames folded together to acquire, and to keep saying the lock is still there.
const ACQ_FRAMES: usize = 12;
/// Sub-symbol phases the acquisition searches. A tenth of a symbol is 42 µs,
/// and on a real capture the two best phases differed by exactly that.
const PHASES: usize = 10;

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
    fn lerp(self, o: C, t: f32) -> C {
        C::new(self.re + (o.re - self.re) * t, self.im + (o.im - self.im) * t)
    }
}

/// The data rate a frame carries, as far as its constellation can say.
///
/// Uncoded 3600 and **every coded rate** use all eight phases, so they cannot be
/// told apart at this layer; nor can 1200 BPSK from the 75–600 bps coded rates,
/// nor QPSK from 1200 at 1/2. The report says both rather than guessing, and that
/// is the honest ceiling of a constellation test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S4285Rate {
    /// 1200 bps, one bit per symbol, 2PSK — or a coded BPSK rate (75–600).
    Bps1200,
    /// 2400 bps, a dibit per symbol, QPSK — or 1200 at rate 1/2.
    Bps2400,
    /// 3600 bps uncoded (8PSK) — or 2400 at rate 2/3.
    Bps3600OrCoded,
    /// Not yet decided — a frame whose references did not collapse.
    Unknown,
}

impl S4285Rate {
    pub fn label(self) -> &'static str {
        match self {
            S4285Rate::Bps1200 => "1200 BPSK uncoded, or a coded BPSK rate",
            S4285Rate::Bps2400 => "2400 QPSK uncoded, or 1200 QPSK rate 1/2",
            S4285Rate::Bps3600OrCoded => "3600 8PSK uncoded, or a coded rate",
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
    /// hides exactly that.
    pub reference: [f32; 3],
    /// Carrier offset the reference blocks agree on, in hertz.
    pub carrier_offset_hz: f32,
    /// Which scrambler sign the frame needed.
    pub reversed: bool,
    /// What the descrambled constellation says.
    pub rate: S4285Rate,
    /// The distinct phases the descrambled data symbols used, 0..8.
    pub phases_used: usize,
    /// How tightly the data symbols sit on the 2-, 4- and 8-point lattices. Each
    /// is `|mean e^(jnφ)|`, which is independent of a rotation. A clean BPSK frame
    /// gives ~1, ~1, ~1; QPSK ~0, ~1, ~1; 8PSK ~0, ~0, ~1. Noise lowers all three,
    /// so the verdict compares them against each other rather than an absolute.
    pub constellation: [f32; 3],
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
    /// The folded differential sync correlation at the lock, 0..1. Folded over
    /// [`ACQ_FRAMES`] frames, which is what makes it a measurement: one frame's
    /// best peak over every offset reaches 0.74 on white noise.
    pub sync_quality: f32,
    /// The same correlation's median over the 256 phases: the null it is read
    /// against. On the night's captures the signal reads 0.85 and the control
    /// 0.14.
    pub correlation_floor: f32,
    /// The frame-period autocorrelation: how much of a frame repeats one frame
    /// later. A correct 4285 measures about **0.31-0.60** — the 80 sync symbols
    /// repeat — while a strong PSK carrier that is a different mode measures about
    /// **0.03**. Measured off the air, and the reason a sync correlation alone
    /// cannot tell the two apart.
    pub frame_acf: f32,
    /// Carrier offset measured from the reference blocks, in hertz.
    pub carrier_offset_hz: f32,
    /// Frames demodulated since the last reset.
    pub frames: u64,
    /// The last frame's rate verdict.
    pub rate: S4285Rate,
    /// The last frame's three reference concentrations.
    pub reference: [f32; 3],
    /// The last frame's 2-, 4- and 8-point constellation concentrations.
    pub constellation: [f32; 3],
    /// The sampling phase being tracked, in symbols (0..1).
    pub timing: f32,
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
            constellation: [0.0; 3],
            timing: 0.0,
        }
    }
}

/// A Fibonacci LFSR, least-significant bit first, as both of the standard's
/// generators are specified: the output is `x₀`, the feedback is the XOR of the
/// tapped stages, and the register shifts one position per output bit.
///
/// The stage count is the polynomial's degree, **not** one more than its highest
/// tap. `x⁵ + x² + 1` is five stages with taps {0, 2} and `x⁹ + x⁴ + 1` is nine
/// with taps {0, 4}; giving either a stage too many makes the register empty out
/// and stay at zero, which produced a sync sequence of period 16 and a scrambler
/// that was a constant — and the references then could not collapse, which read
/// as "no frame". `the_generators_are_maximal_and_match_the_standard` pins it.
fn lfsr(stages: usize, taps: &[usize], n: usize, init: u32) -> Vec<u8> {
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

/// Where the frame head sits, once acquisition has found it.
#[derive(Debug, Clone, Copy)]
struct Lock {
    /// Absolute index, in baseband samples, of the next frame's first symbol.
    next: u64,
}

/// The receiver. Feed it audio at `rate_hz`; it acquires on a fold of the
/// synchronisation, emits a frame per frame once locked, and text as the
/// character layer decodes.
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
    /// Baseband at [`SPS`] samples per symbol, newest last.
    bb: Vec<C>,
    /// Absolute index of `bb[0]`, in baseband samples.
    base: u64,
    /// Frames searched at each acquisition attempt, so a hopeless channel is not
    /// re-folded on every audio block.
    acq_wait: usize,
    lock: Option<Lock>,
    /// The tracked sampling phase, in symbols.
    timing: f32,
    scrambling: [C; SCRAMBLED],
    sync_diff: [f32; SYNC - 1],
    baudot: BaudotRx,
    bit_run: Vec<u8>,
    /// Characters decoded but not yet handed out.
    events_text: String,
    report: S4285Report,
}

/// How many baseband samples the acquisition window spans.
const ACQ_SPAN: u64 = (ACQ_FRAMES as u64 + 1) * FRAME as u64 * SPS as u64;

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
            bb: Vec::new(),
            base: 0,
            acq_wait: 0,
            lock: None,
            timing: 0.0,
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
        self.bb.clear();
        self.base = 0;
        self.acq_wait = 0;
        self.lock = None;
        self.timing = 0.0;
        self.bit_run.clear();
        self.report = S4285Report::default();
    }

    /// Feed audio; returns whatever it produced.
    pub fn process(&mut self, audio: &[f32]) -> Vec<S4285Event> {
        let mut events = Vec::new();
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
                self.bb.push(self.prev_bb.lerp(z, self.frac as f32));
                self.frac += self.step;
            }
            self.frac -= 1.0;
            self.prev_bb = z;
        }

        match self.lock {
            None => {
                if self.bb.len() as u64 >= ACQ_SPAN + SPS as u64 {
                    self.acquire();
                }
            }
            Some(_) => self.drain_frames(&mut events),
        }
        self.trim();
        events
    }

    /// A symbol at a fractional sample position, linearly interpolated. The
    /// baseband is band-limited to 1.5 kHz and sampled at 12 kHz, so a straight
    /// line between two samples is four times oversampled.
    fn at(&self, pos: f64) -> C {
        let i = pos.floor();
        if i < 0.0 {
            return C::default();
        }
        let i = i as usize;
        if i + 1 >= self.bb.len() {
            return self.bb.last().copied().unwrap_or_default();
        }
        self.bb[i].lerp(self.bb[i + 1], (pos - i as f64) as f32)
    }

    /// `FRAME` symbols from a frame head at `start` (a baseband sample index) and
    /// a sampling phase in symbols.
    fn frame_symbols(&self, start: u64, phase: f32) -> [C; FRAME] {
        let mut out = [C::default(); FRAME];
        for (i, o) in out.iter_mut().enumerate() {
            let pos = start as f64 - self.base as f64 + (i as f32 + phase) as f64 * SPS as f64;
            *o = self.at(pos);
        }
        out
    }

    /// The frame-period autocorrelation of the baseband: how much of a frame
    /// repeats one frame later. Independent of the sampling phase, which is why it
    /// can be reported before acquisition has found one.
    fn frame_acf(&self) -> f32 {
        let lag = FRAME * SPS;
        let n = self.bb.len();
        if n < lag + FRAME * SPS {
            return 0.0;
        }
        let take = (n - lag).min(FRAME * SPS);
        let mut acc = C::default();
        let (mut ea, mut eb) = (0.0f32, 0.0f32);
        for k in 0..take {
            let a = self.bb[n - take - lag + k];
            let b = self.bb[n - take + k];
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

    /// Fold the differential sync correlation over every frame phase, at each
    /// sub-symbol sampling phase, and take the standing-up one.
    fn acquire(&mut self) {
        if self.acq_wait > 0 {
            self.acq_wait -= 1;
            return;
        }
        let n = self.bb.len();
        if n < ACQ_SPAN as usize {
            return;
        }
        let start = self.base + (n as u64 - ACQ_SPAN);
        let acf = self.frame_acf();
        self.report.frame_acf = acf;
        if acf < MIN_FRAME_ACF {
            self.report.locked = false;
            self.report.correlation_floor = 0.0;
            self.report.sync_quality = 0.0;
            self.acq_wait = ACQ_FRAMES;
            return;
        }

        let count = ACQ_SPAN as usize / SPS;
        let mut best = (0.0f32, 0.0f32, 0usize, 0.0f32);
        for p in 0..PHASES {
            let phase = p as f32 / PHASES as f32;
            // Symbols over the whole window, at this sampling phase.
            let mut sym = Vec::with_capacity(count);
            for k in 0..count {
                let pos = start as f64 - self.base as f64 + (k as f32 + phase) as f64 * SPS as f64;
                sym.push(self.at(pos));
            }
            // Differential products, then the correlation over the sync window.
            let mut corr = vec![0.0f32; count];
            let n = self.sync_diff.len();
            for k in n..count {
                let mut acc = C::default();
                let mut mag = 0.0f32;
                for (j, &w) in self.sync_diff.iter().enumerate() {
                    let i = k - n + j;
                    let d = sym[i + 1].mul(sym[i].conj());
                    acc = acc.add(d.scale(w));
                    mag += d.norm();
                }
                if mag > 1e-12 {
                    corr[k] = acc.norm() / mag;
                }
            }
            // Fold: mean per phase index, skipping the head where no window fits.
            // `corr[k]` covers symbols `k - n ..= k`, so the window's **first**
            // symbol — the frame head — is `k - n`, and that is what is folded.
            let mut sum = [0.0f32; FRAME];
            let mut num = [0u32; FRAME];
            for (k, &c) in corr.iter().enumerate().skip(n) {
                sum[(k - n) % FRAME] += c;
                num[(k - n) % FRAME] += 1;
            }
            let folded: Vec<f32> = sum
                .iter()
                .zip(&num)
                .map(|(s, m)| if *m > 0 { *s / *m as f32 } else { 0.0 })
                .collect();
            let mut sorted = folded.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = sorted[FRAME / 2];
            let (o, &score) = folded
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((0, &0.0));
            if score > best.0 {
                best = (score, median, o, phase);
            }
        }

        let (score, median, offset, phase) = best;
        self.report.correlation_floor = median;
        self.report.sync_quality = score;
        // A folded correlation has to stand well clear of its own null. Measured:
        // 0.85 on the signal, 0.14 on the control, so 2.2x is the middle with room.
        if score < MIN_SYNC || score < median * 2.2 {
            self.report.locked = false;
            self.acq_wait = ACQ_FRAMES;
            return;
        }
        // The fold says where the frame head is; from here every frame is taken at
        // a fixed stride and refines its own timing.
        self.lock = Some(Lock { next: start + (offset * SPS) as u64 });
        self.timing = phase;
        self.report.locked = true;
    }

    fn drain_frames(&mut self, events: &mut Vec<S4285Event>) {
        loop {
            let Some(lock) = self.lock else { return };
            if lock.next + (FRAME * SPS) as u64 > self.base + self.bb.len() as u64 {
                return;
            }
            let frame = self.take_frame(lock.next);
            match frame {
                Some(f) => {
                    events.push(S4285Event::Frame(f.clone()));
                    // Text only from a frame whose constellation is resolved: an
                    // unresolved one produces letters that look plausible and mean
                    // nothing, which is the one failure a listener cannot see. The
                    // bits are still on the event and in `bits()`.
                    if f.rate != S4285Rate::Unknown {
                        for value in f.bits.chunks(5) {
                            let mut v = 0u8;
                            for (i, b) in value.iter().enumerate() {
                                v |= b << i;
                            }
                            if let Some(ch) = self.baudot.decode(v) {
                                self.events_text.push(ch);
                            }
                        }
                    }
                    if !self.events_text.is_empty() {
                        let t: String = self.events_text.drain(..).collect();
                        events.push(S4285Event::Text(t));
                    }
                    if let Some(l) = self.lock.as_mut() {
                        l.next += (FRAME * SPS) as u64;
                    }
                }
                None => {
                    // The frame did not stand up: let go and acquire again from
                    // what is still buffered rather than demodulating a lie.
                    self.report.locked = false;
                    self.lock = None;
                    self.acq_wait = 0;
                    return;
                }
            }
        }
    }

    /// Demodulate the frame whose head is at absolute sample `start`.
    fn take_frame(&mut self, start: u64) -> Option<S4285Frame> {
        let mut best: Option<(f32, f32, bool, f32)> = None;
        for step in -4i32..=4 {
            let phase = self.timing + step as f32 * 0.05;
            let frame = self.frame_symbols(start, phase);
            for &sign in &[1.0f32, -1.0f32] {
                let mut z = [C::default(); SCRAMBLED];
                for k in 0..SCRAMBLED {
                    let s = if sign > 0.0 { self.scrambling[k] } else { self.scrambling[k].conj() };
                    z[k] = frame[SYNC + k].mul(s.conj());
                }
                // Coarse ramp, then a fine one round the best. A 16-symbol block
                // is 6.67 ms, so 150 Hz turns a whole circle inside it.
                let mut coarse = (0.0f32, 0.0f32);
                let mut hz = -150.0f32;
                while hz <= 150.0 {
                    let c = ramp_concentrations(&z, hz);
                    let r = (c[0] + c[1] + c[2]) / 3.0;
                    if r > coarse.0 {
                        coarse = (r, hz);
                    }
                    hz += 5.0;
                }
                let mut fine = coarse;
                let mut hz = coarse.1 - 5.0;
                while hz <= coarse.1 + 5.0 {
                    let c = ramp_concentrations(&z, hz);
                    let r = (c[0] + c[1] + c[2]) / 3.0;
                    if r > fine.0 {
                        fine = (r, hz);
                    }
                    hz += 1.0;
                }
                if best.as_ref().is_none_or(|b| fine.0 > b.0) {
                    best = Some((fine.0, phase, sign < 0.0, fine.1));
                }
            }
        }
        let (score, phase, reversed, offset) = best?;
        if score < MIN_REFERENCE {
            return None;
        }
        // The carrier offset has to come off the symbols themselves, not just off
        // the score: the channel phase is read from the reference blocks, and a
        // ramp left in place turns a whole circle between two of them, so the
        // interpolation across the frame would be nonsense.
        let frame = self.frame_symbols(start, phase);
        let mut z = [C::default(); SCRAMBLED];
        for k in 0..SCRAMBLED {
            let s = if reversed { self.scrambling[k].conj() } else { self.scrambling[k] };
            let t = k as f32 / BAUD as f32;
            z[k] = frame[SYNC + k].mul(s.conj()).mul(C::from_phase(-offset * TAU * t));
        }
        let reference = ramp_concentrations(&z, 0.0);
        // Track the sampling phase slowly: a per-frame jump is a measurement, a
        // moving average is what keeps a fading capture's eye open.
        let delta = phase - self.timing;
        self.timing += (delta * 0.25).clamp(-0.05, 0.05);
        if self.timing < 0.0 {
            self.timing += 1.0;
        } else if self.timing >= 1.0 {
            self.timing -= 1.0;
        }

        // The channel inside the frame: the three reference blocks give its phase
        // at three points, and the data blocks between them interpolate.
        let centres = [
            (REF_BLOCKS[0].0 + REF_BLOCKS[0].1 - 1) as f32 / 2.0,
            (REF_BLOCKS[1].0 + REF_BLOCKS[1].1 - 1) as f32 / 2.0,
            (REF_BLOCKS[2].0 + REF_BLOCKS[2].1 - 1) as f32 / 2.0,
        ];
        let mut ph = [0.0f32; 3];
        for (i, &(lo, hi)) in REF_BLOCKS.iter().enumerate() {
            let mut acc = C::default();
            for k in lo..hi {
                acc = acc.add(z[k]);
            }
            ph[i] = acc.arg();
        }
        // Least-squares slope through the three centres.
        let m = ((ph[1] - ph[0]) * (centres[1] - centres[0])
            + (ph[2] - ph[1]) * (centres[2] - centres[1]))
            / ((centres[1] - centres[0]).powi(2) + (centres[2] - centres[1]).powi(2));
        let b = (ph[0] + ph[1] + ph[2]) / 3.0 - m * (centres[0] + centres[1] + centres[2]) / 3.0;

        let mut used = [false; 8];
        let mut data: Vec<C> = Vec::with_capacity(128);
        for &(lo, hi) in &DATA_BLOCKS {
            for k in lo..hi {
                let c = z[k].mul(C::from_phase(-(m * k as f32 + b)));
                let idx = (c.arg() / std::f32::consts::FRAC_PI_4).round() as i32;
                used[idx.rem_euclid(8) as usize] = true;
                data.push(c);
            }
        }
        let phases_used = used.iter().filter(|u| **u).count();

        // The constellation, as concentrations that a rotation cannot change.
        let (mut c2, mut c4, mut c8) = (C::default(), C::default(), C::default());
        for c in &data {
            c2 = c2.add(C::from_phase(2.0 * c.arg()));
            c4 = c4.add(C::from_phase(4.0 * c.arg()));
            c8 = c8.add(C::from_phase(8.0 * c.arg()));
        }
        let n = data.len().max(1) as f32;
        let constellation = [c2.norm() / n, c4.norm() / n, c8.norm() / n];

        // A 2PSK frame leaves the even symbol numbers only; a dibit frame the
        // multiples of two; a tribit frame all eight. Noise smears each of those,
        // so the classes are told apart by which concentration stands up, not by
        // counting phases that noise has already spread over all eight. A leading
        // concentration under 0.2 is not a resolved constellation and is reported
        // as such rather than guessed — a rate with no evidence behind it is how a
        // wrong demodulation looks right.
        let (c2, c4, c8) = (constellation[0], constellation[1], constellation[2]);
        let leader = c2.max(c4).max(c8);
        let runner = if leader == c2 {
            c4.max(c8)
        } else if leader == c4 {
            c2.max(c8)
        } else {
            c2.max(c4)
        };
        let rate = if leader < 0.20 || leader < runner * 1.5 {
            S4285Rate::Unknown
        } else if leader == c2 {
            S4285Rate::Bps1200
        } else if leader == c4 {
            S4285Rate::Bps2400
        } else {
            S4285Rate::Bps3600OrCoded
        };
        let bits = transcode(&data, rate);

        self.report.frames += 1;
        self.report.rate = rate;
        self.report.reference = reference;
        self.report.constellation = constellation;
        self.report.carrier_offset_hz = offset;
        self.report.timing = self.timing;
        self.bit_run.extend_from_slice(&bits);
        if self.bit_run.len() > 8192 {
            let cut = self.bit_run.len() - 8192;
            self.bit_run.drain(..cut);
        }
        Some(S4285Frame {
            reference,
            carrier_offset_hz: offset,
            reversed,
            rate,
            phases_used,
            constellation,
            bits,
        })
    }

    /// Keep only what the next frame and the acquisition window need.
    fn trim(&mut self) {
        let need = match self.lock {
            Some(l) => l.next.saturating_sub(self.base),
            None => 0,
        };
        let keep_from = need.min(self.bb.len() as u64) as usize;
        let drop = match self.lock {
            Some(_) => keep_from,
            None => self.bb.len().saturating_sub(ACQ_SPAN as usize + SPS),
        };
        if drop > 4096 {
            self.bb.drain(..drop);
            self.base += drop as u64;
        }
    }

    /// Characters decoded but not yet handed out.
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.events_text)
    }
}

/// The mean reference concentration a block needs before it is called a frame.
///
/// Sixteen symbols of noise give `1/sqrt(16)` = 0.25; a frame whose blocks have
/// collapsed measured 0.65–0.81 across the night's captures. 0.45 sits between
/// them and is deliberately nearer the noise floor than the signal: a frame that
/// is wrongly accepted is visible in the three numbers, while one wrongly rejected
/// is silence.
const MIN_REFERENCE: f32 = 0.45;

/// The folded sync correlation a lock needs, and the margin over its own null.
///
/// A correct 4285 measures 0.85 folded over twelve frames; the strong PSK carrier
/// that is a different mode measures 0.14 on the same statistic, and a single
/// frame's maximum over every offset reaches 0.738 on white noise — which is why
/// the peak alone was never a detector.
const MIN_SYNC: f32 = 0.40;

/// The frame-period autocorrelation a lock needs.
///
/// A correct 4285 measures 0.31-0.60 (its 80 sync symbols repeat out of 256) and a
/// strong PSK carrier of another mode measures ~0.03, both measured. 0.12 sits
/// between them with room for a fading capture.
const MIN_FRAME_ACF: f32 = 0.12;

/// The three reference blocks' concentrations with a carrier offset removed
/// inside each block.
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

    /// Both generators are the standard's, and both got wrong once: a register
    /// given one stage too many empties out and stays at zero, which made the
    /// sync sequence periodic with 16 and the scrambler a constant. Nothing
    /// failed loudly — the sync correlated *better* against the degenerate
    /// pattern and the references simply never collapsed, which read as "no frame
    /// in this file".
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

    /// Is this scrambled offset a reference symbol?
    fn is_reference(k: usize) -> bool {
        REF_BLOCKS.iter().any(|&(lo, hi)| k >= lo && k < hi)
    }

    /// A frame-accurate 4285 BPSK transmitter, for the regression test only.
    ///
    /// Raised-cosine pulses with 30 % excess bandwidth — the standard's 3000 Hz
    /// occupied bandwidth at 2400 baud — so the loopback is a fair signal and not
    /// a rectangular-pulse spectrum the receiver's own filter would have to
    /// truncate. Frame `n` carries `bits[n*128 ..]`, so the payload is a known
    /// sequence rather than noise.
    fn modulate(bits: &[u8], frames: usize, rate: f64) -> Vec<f32> {
        let sync = sync_symbols();
        let scr = scrambling();
        // Symbol phases on a 5-sample grid, then pulse-shaped.
        let mut sym = Vec::with_capacity(frames * FRAME * 5);
        for f in 0..frames {
            let mut frame = [0.0f32; FRAME];
            for (i, s) in sync.iter().enumerate() {
                frame[i] = if *s > 0.0 { 0.0 } else { 4.0 };
            }
            let mut d = 0;
            for k in 0..SCRAMBLED {
                let sym_phase = if is_reference(k) {
                    0.0
                } else {
                    let b = bits[(f * 128 + d) % bits.len().max(1)];
                    d += 1;
                    if b == 1 { 4.0 } else { 0.0 }
                };
                frame[SYNC + k] =
                    (sym_phase + scr[k].arg() / std::f32::consts::FRAC_PI_4).rem_euclid(8.0);
            }
            for v in frame {
                sym.push(C::from_phase(v * std::f32::consts::FRAC_PI_4));
                for _ in 1..5 {
                    sym.push(C::default());
                }
            }
        }
        // Raised cosine, alpha 0.3, five symbols either side.
        let alpha = 0.3f32;
        let taps: Vec<f32> = (-20i32..=20)
            .map(|i| {
                let x = i as f32 / 5.0;
                let sinc = if x.abs() < 1e-6 {
                    1.0
                } else {
                    (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x)
                };
                let d = 1.0 - (2.0 * alpha * x).powi(2);
                let rc = if d.abs() < 1e-6 {
                    std::f32::consts::FRAC_PI_4 * (std::f32::consts::PI * x / 2.0).sin()
                } else {
                    (std::f32::consts::PI * alpha * x).cos() / d
                };
                sinc * rc
            })
            .collect();
        let mut bb = vec![C::default(); sym.len()];
        for (i, s) in sym.iter().enumerate() {
            if s.norm() < 1e-9 {
                continue;
            }
            for (j, t) in taps.iter().enumerate() {
                let k = i as isize + j as isize - 20;
                if k >= 0 && (k as usize) < bb.len() {
                    bb[k as usize] = bb[k as usize].add(s.scale(*t));
                }
            }
        }
        bb.iter()
            .enumerate()
            .map(|(i, z)| {
                // The phase is accumulated in f64 and reduced: `TAU * f * i` in
                // f32 loses its fractional part within a second and the carrier
                // lands on the wrong frequency entirely.
                let ph = (TAU as f64 * 1800.0 * i as f64 / rate).rem_euclid(TAU as f64) as f32;
                0.5 * z.mul(C::from_phase(ph)).re
            })
            .collect()
    }

    #[test]
    fn a_synthetic_frame_locks_and_its_references_collapse() {
        // A pseudo-random payload, so the data cannot pass for references. A
        // plain multiplicative hash has period 256 here and makes frame *n+1*
        // the exact complement of frame *n*, which cancels the sync in the frame
        // ACF — that fixture was the thing that was wrong, not the receiver.
        let mut seed = 0x2545_f491u32;
        let bits: Vec<u8> = (0..4096)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed & 1) as u8
            })
            .collect();
        let audio = modulate(&bits, 18, 12000.0);
        let mut rx = S4285Receiver::new(12000.0, 1800.0);
        for chunk in audio.chunks(1024) {
            rx.process(chunk);
        }
        let r = rx.report();
        assert!(r.locked, "a clean synthetic frame did not lock: {:?}", r);
        assert!(r.frames > 8, "only {} frames demodulated", r.frames);
        for (i, c) in r.reference.iter().enumerate() {
            assert!(*c > 0.85, "reference block {i} read {c:.3}, and it should collapse");
        }
        assert!(r.frame_acf > 0.25, "frame ACF {:.3}", r.frame_acf);
    }

    /// The negative that makes the lock a detector rather than a lamp: noise must
    /// not lock, and neither must a strong carrier of a different shape.
    #[test]
    fn noise_and_a_plain_carrier_do_not_lock() {
        let mut seed = 0x1234_5678u32;
        let mut noise = Vec::new();
        for _ in 0..12 * 12000 {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            noise.push(((seed >> 16) as f32 / 32768.0 - 1.0) * 0.2);
        }
        let mut rx = S4285Receiver::new(12000.0, 1800.0);
        for chunk in noise.chunks(1024) {
            rx.process(chunk);
        }
        assert!(!rx.report().locked, "white noise locked: {:?}", rx.report());

        // A single steady tone at the carrier: a strong carrier is not a frame.
        let tone: Vec<f32> = (0..12 * 12000)
            .map(|i| 0.5 * (TAU as f32 * 1800.0 * i as f32 / 12000.0).cos())
            .collect();
        let mut rx = S4285Receiver::new(12000.0, 1800.0);
        for chunk in tone.chunks(1024) {
            rx.process(chunk);
        }
        assert!(!rx.report().locked, "a plain carrier locked: {:?}", rx.report());
    }

    /// The material this was built against: a real station off the air. Not in
    /// the tree — the capture is the operator's — so it runs only when pointed at
    /// one, exactly as the other off-air gates here do.
    ///
    ///     SDROXIDE_S4285_SAMPLE=~/s4285-captures/6348.000kHz-20261022-045545Z.wav \
    ///       cargo test -p sdroxide-dsp --release -- --ignored --nocapture an_off_air_capture
    #[test]
    #[ignore]
    fn an_off_air_capture_locks_and_its_references_collapse() {
        let path = std::env::var("SDROXIDE_S4285_SAMPLE").expect("set SDROXIDE_S4285_SAMPLE");
        let mut reader = hound::WavReader::open(&path).expect("open the wav");
        let spec = reader.spec();
        let audio: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32_768.0).collect()
            }
        };
        let mut rx = S4285Receiver::new(spec.sample_rate as f64, 1800.0);
        let mut refs = [0.0f64; 3];
        let mut n = 0.0f64;
        for chunk in audio.chunks(4096) {
            for ev in rx.process(chunk) {
                if let S4285Event::Frame(f) = ev {
                    for i in 0..3 {
                        refs[i] += f.reference[i] as f64;
                    }
                    n += 1.0;
                }
            }
        }
        let r = rx.report();
        println!("{:?}", r);
        assert!(n > 50.0, "only {n} frames demodulated");
        let mean = [refs[0] / n, refs[1] / n, refs[2] / n];
        println!("mean references {mean:?}");
        for (i, c) in mean.iter().enumerate() {
            assert!(*c > 0.55, "reference block {i} averaged {c:.3} over {n} frames");
        }
    }
}
