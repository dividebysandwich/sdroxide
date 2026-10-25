//! Adaptive predistortion: linearise a power amplifier by sending it the
//! inverse of its own distortion, measured from a sample of what it actually
//! emitted.
//!
//! This is the technique openHPSDR calls **PureSignal**, and the arrangement is
//! the same: a directional coupler on the amplifier's output feeds a second
//! receiver tuned to the transmit frequency, and the transmitter compares what
//! came back with what it meant to send. On a LimeSDR the second receiver is
//! the board's other chain (issue #98), which is why this lives beside
//! `sdroxide_lime`'s auxiliary chain.
//!
//! # Why it is worth the trouble
//!
//! Every amplifier compresses near its ceiling, and compression on a
//! multi-tone signal — which is what SSB and every digital mode are — is
//! intermodulation: energy landing either side of your own transmission, on
//! other people's QSOs. Backing off is the traditional answer and costs most of
//! the amplifier. Predistortion instead sends a deliberately *wrong* signal,
//! bent by exactly the inverse of the amplifier's curve, so that what comes out
//! is right. Twenty-odd decibels of IMD improvement is the usual figure, and the
//! amplifier keeps its power.
//!
//! # What is modelled
//!
//! A complex gain indexed by *amplitude*: `out = x · G(|x|)`, with `G` a table
//! interpolated between [`PureSignal::bins`] entries across the full-scale
//! range. `|G|` carries the AM/AM curve — how the gain sags as drive rises —
//! and `arg G` the AM/PM curve, the phase shift that comes with it. This is a
//! **memoryless** model, and deliberately: memory effects need a polynomial
//! with delay terms and far more feedback than an HF amplifier's curve does,
//! and the memoryless part is where nearly all of the amateur-band IMD is.
//!
//! # The loop
//!
//! 1. [`PureSignal::predistort`] bends the outgoing block and keeps a copy of
//!    what was *wanted* — the reference.
//! 2. The feedback arrives later, by however long the transmit FIFO, the
//!    converters, the amplifier and the receive FIFO take.
//!    [`PureSignal::feed_back`] finds that delay by correlating envelopes, and
//!    then holds it: at a fixed sample rate the two streams differ by a
//!    constant, so it is measured once per over and re-checked, not searched
//!    for every block.
//! 3. Each aligned pair updates the table towards the inverse of what was
//!    measured, damped by the adaptation rate so noise on one block cannot move
//!    it far.
//!
//! # It cannot make the transmitter louder
//!
//! Two properties hold whatever the feedback says, because the failure mode of
//! a correction loop fed rubbish must not be an over-driven amplifier:
//!
//! * The table is **normalised at the top** — the largest amplitude keeps unit
//!   gain and everything below it is pulled *down* — so a compressing amplifier
//!   is linearised by reducing small-signal gain rather than by asking for more
//!   than full scale.
//! * Every entry is then clamped so that `|x| · |G(|x|)|` cannot exceed full
//!   scale, and so that no entry departs from unity by more than
//!   [`MAX_CORRECTION`].
//!
//! And the table starts at unity, so a coupler that is not connected, a
//! receiver that is deaf, or an alignment that never locks all leave the
//! transmitter exactly as it would have been.
//!
//! # Not verified against hardware
//!
//! No amplifier has been on the end of this. The tests below drive it through a
//! simulated compressing amplifier with a delay, a frequency offset and noise,
//! which says the arithmetic converges — not that a real feedback path is what
//! this expects.

use crate::Complex32;

/// How far an entry may depart from unity gain, as a ratio. A correction
/// beyond this is not a nonlinearity, it is a broken feedback path.
pub const MAX_CORRECTION: f32 = 4.0;

/// Envelope decimation for the coarse delay search. The envelope of a
/// modulated signal has nothing in it above a few tens of kilohertz, so
/// throwing away 63 of every 64 samples costs the correlation nothing and
/// makes searching a hundred thousand lags cheap.
const ENV_DECIM: usize = 64;

/// The shortest feedback block worth trying to align or learn from.
const MIN_BLOCK: usize = 1024;

/// How well the aligned envelopes must correlate for the alignment to be
/// believed. Two unrelated signals score near zero; the same signal through an
/// amplifier and a receiver scores well above this.
const LOCK_THRESHOLD: f32 = 0.5;

/// How many samples of full-rate refinement either side of the coarse answer.
const REFINE: usize = ENV_DECIM;

/// A bin needs this many samples in a block before that block is allowed to
/// move it. Amplitude histograms of speech are steep: the top bins see very
/// few samples, and learning a bin from three of them is learning noise.
const MIN_BIN_HITS: u32 = 32;

/// The predistorter, its reference history, and the alignment between them.
pub struct PureSignal {
    /// The correction table: complex gain against `|x|` from 0 to full scale.
    table: Vec<Complex32>,
    /// How hard each block moves the table, 0..1.
    alpha: f32,
    frozen: bool,

    /// Everything recently asked for, oldest first. `base` is the absolute
    /// index of `refs[0]`.
    refs: Vec<Complex32>,
    /// The decimated envelope of `refs`, one entry per [`ENV_DECIM`] samples.
    env: Vec<f32>,
    base: u64,
    /// Total samples ever handed to [`Self::predistort`].
    ref_total: u64,
    /// Total feedback samples ever consumed.
    fb_total: u64,
    /// How far the reference index runs ahead of the feedback index. Constant
    /// while neither stream drops anything, which is what makes the search a
    /// once-per-over cost.
    offset: Option<i64>,
    /// The last alignment score, for [`Self::locked`] and the log.
    score: f32,
    /// Feedback samples since the alignment was last checked.
    since_check: u64,
    /// How often to check it, in samples.
    check_every: u64,
    /// How much history to keep, which bounds the delay that can be found.
    keep: usize,
    /// Feedback samples since the last search for the alignment while there
    /// is none, and how many must pass between searches. A search costs the
    /// whole history times the block, so searching on *every* block while
    /// unlocked — fine over a tenth of a second of history — is a gigaflop a
    /// second over the second of history a networked radio needs.
    since_locate: u64,
    locate_every: u64,

    /// De-rotation and envelope scratch, kept to avoid allocating per block.
    scratch: Vec<Complex32>,
    fenv: Vec<f32>,
}

impl PureSignal {
    /// `bins` is the table resolution and `rate` the 0..1 adaptation control.
    /// `sample_rate_hz` sizes the reference history: the loop delay is mostly
    /// the transmit FIFO, so the history has to span it.
    pub fn new(bins: usize, rate: f32, sample_rate_hz: f64) -> PureSignal {
        // A tenth of a second of history, bounded so a fast device does not
        // cost eight megabytes: the transmit FIFO is what the delay is made
        // of, and the engine paces the over to keep it at tens of
        // milliseconds.
        let keep = ((sample_rate_hz * 0.1) as usize).clamp(1 << 16, 1 << 19);
        Self::with_keep(bins, rate, sample_rate_hz, keep)
    }

    /// The same, for a radio whose loop delay is longer than a tenth of a
    /// second — one reached over a network, where the samples pass through a
    /// host ring, the device's own DMA buffers and back again before the
    /// feedback arrives (a Pluto, issue #525). `history_s` is how far back the
    /// alignment may look; past what it covers the loop simply never locks.
    pub fn with_history(bins: usize, rate: f32, sample_rate_hz: f64, history_s: f64) -> PureSignal {
        let keep = ((sample_rate_hz * history_s.max(0.1)) as usize).clamp(1 << 16, 1 << 22);
        Self::with_keep(bins, rate, sample_rate_hz, keep)
    }

    fn with_keep(bins: usize, rate: f32, sample_rate_hz: f64, keep: usize) -> PureSignal {
        let bins = bins.clamp(4, 256);
        let check_every = (sample_rate_hz * 0.5) as u64;
        // Eight tries a second while unlocked: quick enough that an over is
        // found within its first syllable, rare enough to cost nothing.
        let locate_every = (check_every / 4).max(1);
        PureSignal {
            table: vec![Complex32::new(1.0, 0.0); bins],
            alpha: Self::alpha_for_rate(rate),
            frozen: false,
            refs: Vec::new(),
            env: Vec::new(),
            base: 0,
            ref_total: 0,
            fb_total: 0,
            offset: None,
            score: 0.0,
            since_check: 0,
            // Twice a second at any rate people transmit at.
            check_every,
            keep,
            // So that the very first block is searched at once.
            since_locate: locate_every,
            locate_every,
            scratch: Vec::new(),
            fenv: Vec::new(),
        }
    }

    /// The 0..1 adaptation control as a per-block damping factor. Slow enough
    /// at the bottom to average a whole over, quick enough at the top to
    /// converge inside a syllable.
    pub fn alpha_for_rate(rate: f32) -> f32 {
        0.01 * (50.0f32).powf(rate.clamp(0.0, 1.0))
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.alpha = Self::alpha_for_rate(rate);
    }

    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    pub fn frozen(&self) -> bool {
        self.frozen
    }

    pub fn bins(&self) -> usize {
        self.table.len()
    }

    /// Whether the feedback has been aligned with the reference. Until it has,
    /// the table is not touched and the transmitter is exactly as it was.
    pub fn locked(&self) -> bool {
        self.offset.is_some() && self.score >= LOCK_THRESHOLD
    }

    /// How well the last aligned pair correlated, 0 to 1.
    pub fn score(&self) -> f32 {
        self.score
    }

    /// The correction table, for a display: complex gain against amplitude.
    pub fn table(&self) -> &[Complex32] {
        &self.table
    }

    /// How far the table has departed from flat, in dB — the amount of
    /// compression it is undoing. Around zero means the amplifier is already
    /// linear over the range being driven, or that nothing has been learned.
    pub fn correction_db(&self) -> f32 {
        let (mut lo, mut hi) = (f32::MAX, 0.0f32);
        for g in &self.table {
            let m = g.norm();
            lo = lo.min(m);
            hi = hi.max(m);
        }
        if lo <= 0.0 || hi <= 0.0 { 0.0 } else { 20.0 * (hi / lo).log10() }
    }

    /// Forget the table and the alignment. What a new over does, and what the
    /// operator's Restart does.
    pub fn reset(&mut self) {
        self.table.fill(Complex32::new(1.0, 0.0));
        self.unlock();
    }

    /// Forget the alignment but keep the table — what starting an over does:
    /// the amplifier's curve has not changed since the last one, but where the
    /// feedback sits relative to the reference has.
    pub fn unlock(&mut self) {
        self.refs.clear();
        self.env.clear();
        self.base = 0;
        self.ref_total = 0;
        self.fb_total = 0;
        self.offset = None;
        self.score = 0.0;
        self.since_check = 0;
        // Search on the first feedback block of the new over, not a slot later.
        self.since_locate = self.locate_every;
    }

    /// Bend one block on its way to the transmitter, and keep what was wanted.
    ///
    /// In place: `x` arrives as the wanted baseband and leaves as what to
    /// actually send. The reference kept is the *wanted* signal, because that
    /// is what the amplifier's output is supposed to look like.
    pub fn predistort(&mut self, x: &mut [Complex32]) {
        if x.is_empty() {
            return;
        }
        self.push_reference(x);
        for s in x.iter_mut() {
            *s *= self.gain_at(s.norm());
        }
    }

    /// The table, interpolated. `a` is an amplitude from 0 to full scale.
    fn gain_at(&self, a: f32) -> Complex32 {
        let b = self.table.len();
        // Bin centres at (i + 0.5)/b, so the ends are held rather than
        // extrapolated into a region no sample reached.
        let pos = a.clamp(0.0, 1.0) * b as f32 - 0.5;
        if pos <= 0.0 {
            return self.table[0];
        }
        let i = pos.floor() as usize;
        if i + 1 >= b {
            return self.table[b - 1];
        }
        let f = pos - i as f32;
        self.table[i] * (1.0 - f) + self.table[i + 1] * f
    }

    fn push_reference(&mut self, x: &[Complex32]) {
        // Compact when the history has grown past twice what is kept, so the
        // memmove happens once per `keep` samples rather than once per block.
        if self.refs.len() + x.len() > 2 * self.keep {
            let drop = self.refs.len().saturating_sub(self.keep);
            let drop = drop - drop % ENV_DECIM; // keep the envelope in step
            self.refs.drain(..drop);
            self.env.drain(..drop / ENV_DECIM);
            self.base += drop as u64;
        }
        self.refs.extend_from_slice(x);
        // The envelope holds one entry per whole group, so it is always
        // exactly `refs.len() / ENV_DECIM` long and picks up where it left
        // off. A block that ends mid-group leaves the tail for the next one —
        // which is why the compaction above only ever drops whole groups.
        let mut g = self.env.len() * ENV_DECIM;
        while g + ENV_DECIM <= self.refs.len() {
            let sum: f32 = self.refs[g..g + ENV_DECIM].iter().map(|s| s.norm()).sum();
            self.env.push(sum / ENV_DECIM as f32);
            g += ENV_DECIM;
        }
        self.ref_total += x.len() as u64;
    }

    /// Hand over one block of what came back from the coupler.
    ///
    /// `offset_hz` is the difference between the transmit and receive local
    /// oscillators, which on a zero-IF radio is not zero: the feedback arrives
    /// that far from the centre of the receiver's span and has to be spun back
    /// before it can be compared with anything. It is known exactly — both
    /// synthesisers were commanded — so this is arithmetic, not a search. The
    /// constant phase left over is absorbed by the per-block gain estimate.
    ///
    /// Returns whether this block moved the table.
    pub fn feed_back(&mut self, y: &[Complex32], offset_hz: f64, sample_rate_hz: f64) -> bool {
        if y.len() < MIN_BLOCK || self.refs.len() < MIN_BLOCK {
            self.fb_total += y.len() as u64;
            return false;
        }
        // Spin out the known LO difference.
        self.scratch.clear();
        self.scratch.reserve(y.len());
        let step = -std::f64::consts::TAU * offset_hz / sample_rate_hz;
        for (n, s) in y.iter().enumerate() {
            let ph = (step * n as f64) as f32;
            self.scratch.push(*s * Complex32::from_polar(1.0, ph));
        }
        let m = self.scratch.len();
        let start = self.fb_total;
        self.fb_total += m as u64;

        self.since_check += m as u64;
        self.since_locate += m as u64;
        let recheck = self.since_check >= self.check_every;
        let searching = self.offset.is_none() && self.since_locate >= self.locate_every;
        if searching || recheck {
            self.since_check = 0;
            self.since_locate = 0;
            self.locate(start, m);
        }
        let Some(off) = self.offset else { return false };

        // Where this block's reference sits in the history.
        let idx = start as i64 + off - self.base as i64;
        if idx < 0 || idx as usize + m > self.refs.len() {
            // The history no longer covers it — the alignment is stale.
            self.offset = None;
            self.score = 0.0;
            return false;
        }
        let idx = idx as usize;
        let score = envelope_score(&self.refs[idx..idx + m], &self.scratch[..m]);
        self.score = score;
        if score < LOCK_THRESHOLD {
            self.offset = None;
            return false;
        }
        if self.frozen {
            return false;
        }
        self.learn(idx, m)
    }

    /// Find where this feedback block sits in the reference history.
    ///
    /// Coarse over the decimated envelopes, then refined at the full rate.
    /// Envelopes throughout, never the complex samples: the phase between the
    /// two chains is unknown and irrelevant, and an envelope correlation does
    /// not care about it.
    fn locate(&mut self, fb_start: u64, m: usize) {
        // The feedback envelope.
        self.fenv.clear();
        let mut g = 0;
        while g + ENV_DECIM <= m {
            let sum: f32 = self.scratch[g..g + ENV_DECIM].iter().map(|s| s.norm()).sum();
            self.fenv.push(sum / ENV_DECIM as f32);
            g += ENV_DECIM;
        }
        if self.fenv.len() < 8 || self.env.len() <= self.fenv.len() {
            return;
        }
        let mut best = (0.0f32, 0usize);
        let span = self.env.len() - self.fenv.len();
        for p in 0..=span {
            let s = correlate(&self.env[p..p + self.fenv.len()], &self.fenv);
            if s > best.0 {
                best = (s, p);
            }
        }
        if best.0 < LOCK_THRESHOLD {
            self.offset = None;
            self.score = best.0;
            return;
        }
        // Refine at the full rate, either side of the coarse answer.
        let coarse = best.1 * ENV_DECIM;
        let lo = coarse.saturating_sub(REFINE);
        let hi = (coarse + REFINE).min(self.refs.len().saturating_sub(m));
        // A short window is enough for the refinement and keeps it cheap.
        let win = m.min(4096);
        let mut fine = (0.0f32, coarse);
        for p in lo..=hi {
            let s = envelope_score(&self.refs[p..p + win], &self.scratch[..win]);
            if s > fine.0 {
                fine = (s, p);
            }
        }
        self.score = fine.0;
        if fine.0 < LOCK_THRESHOLD {
            self.offset = None;
            return;
        }
        // Stored as the constant difference between the two streams' sample
        // counts, which is what stays true from one block to the next.
        self.offset = Some(self.base as i64 + fine.1 as i64 - fb_start as i64);
    }

    /// One update of the table from an aligned pair.
    fn learn(&mut self, idx: usize, m: usize) -> bool {
        let b = self.table.len();
        let mut acc = vec![Complex32::new(0.0, 0.0); b];
        let mut pw = vec![0.0f32; b];
        let mut hits = vec![0u32; b];

        // The overall complex gain of the whole loop — coupler attenuation,
        // cable length, receiver gain and the phase between the two
        // synthesisers, all of it. Estimated per block and divided out, so
        // what is left is the *shape* of the amplifier's curve, which is the
        // only part worth learning.
        let mut num = Complex32::new(0.0, 0.0);
        let mut den = 0.0f32;
        for n in 0..m {
            let x = self.refs[idx + n];
            num += self.scratch[n] * x.conj();
            den += x.norm_sqr();
        }
        if den <= 0.0 || num.norm() <= 0.0 {
            return false;
        }
        let k = num / den;

        for n in 0..m {
            let x = self.refs[idx + n];
            let a = x.norm();
            if a <= 0.0 {
                continue;
            }
            let i = ((a.clamp(0.0, 1.0) * b as f32) as usize).min(b - 1);
            // The feedback referred back to the reference's own scale.
            acc[i] += (self.scratch[n] / k) * x.conj();
            pw[i] += a * a;
            hits[i] += 1;
        }

        let mut moved = false;
        for i in 0..b {
            if hits[i] < MIN_BIN_HITS || pw[i] <= 0.0 {
                continue;
            }
            // What the cascade actually does at this amplitude, relative to
            // unity. One means already linear here.
            let r = acc[i] / pw[i];
            if r.norm() <= 1e-6 {
                continue;
            }
            // Move the table towards its own inverse: fixed-point iteration on
            // `G · g(|x·G|) = 1`, damped so one noisy block cannot swing it.
            let want = self.table[i] / r;
            self.table[i] = self.table[i] * (1.0 - self.alpha) + want * self.alpha;
            moved = true;
        }
        if !moved {
            return false;
        }
        self.normalise();
        true
    }

    /// Pin the top of the table at unity and hold everything inside the safe
    /// range. This is what makes the loop unable to ask for more drive than it
    /// was given — see the module doc.
    fn normalise(&mut self) {
        let b = self.table.len();
        let top = self.table[b - 1];
        if top.norm() > 1e-6 {
            for g in self.table.iter_mut() {
                *g /= top;
            }
        }
        for (i, g) in self.table.iter_mut().enumerate() {
            // The amplitude this entry is reached by, at the top of its bin:
            // the product of the two is what actually leaves the DAC.
            let a = (i + 1) as f32 / b as f32;
            let ceiling = MAX_CORRECTION.min(1.0 / a.max(1e-3));
            let m = g.norm();
            if !m.is_finite() || m <= 0.0 {
                *g = Complex32::new(1.0, 0.0);
            } else if m > ceiling {
                *g *= ceiling / m;
            } else if m < 1.0 / MAX_CORRECTION {
                *g *= (1.0 / MAX_CORRECTION) / m;
            }
        }
    }
}

/// Normalised correlation of two non-negative sequences, with their means
/// removed — the Pearson coefficient, which is 1 for the same envelope at any
/// gain and near 0 for unrelated ones.
fn correlate(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let inv = 1.0 / n as f32;
    let ma: f32 = a[..n].iter().sum::<f32>() * inv;
    let mb: f32 = b[..n].iter().sum::<f32>() * inv;
    let (mut num, mut da, mut db) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..n {
        let x = a[i] - ma;
        let y = b[i] - mb;
        num += x * y;
        da += x * x;
        db += y * y;
    }
    if da <= 0.0 || db <= 0.0 { 0.0 } else { num / (da * db).sqrt() }
}

/// The same, straight from two complex blocks' envelopes.
fn envelope_score(a: &[Complex32], b: &[Complex32]) -> f32 {
    let n = a.len().min(b.len());
    let ea: Vec<f32> = a[..n].iter().map(|s| s.norm()).collect();
    let eb: Vec<f32> = b[..n].iter().map(|s| s.norm()).collect();
    correlate(&ea, &eb)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A compressing amplifier, of the shape every real one has: linear at the
    /// bottom, sagging towards a ceiling, with a phase that turns as it does.
    fn amplifier(u: Complex32) -> Complex32 {
        let a = u.norm();
        if a <= 0.0 {
            return u;
        }
        // Soft limiter: gain falls away as the drive approaches full scale.
        let g = 1.0 / (1.0 + (a / 0.75).powi(3)).powf(1.0 / 3.0);
        // AM/PM: a few degrees of phase shift by the time it is compressing.
        let phi = -0.35 * a * a;
        u * Complex32::from_polar(g, phi)
    }

    fn source(n: usize, seed: u64) -> Vec<Complex32> {
        let mut s = seed | 1;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            ((s >> 40) as f32 / 8_388_608.0) - 1.0
        };
        // Band-limited-ish noise at a realistic crest factor, scaled so the
        // amplifier is worked into compression but not clipped.
        let mut out = Vec::with_capacity(n);
        let (mut i, mut q) = (0.0f32, 0.0f32);
        for _ in 0..n {
            i = 0.97 * i + 0.03 * next() * 8.0;
            q = 0.97 * q + 0.03 * next() * 8.0;
            let s = Complex32::new(i, q);
            let a = s.norm();
            out.push(if a > 0.95 { s * (0.95 / a) } else { s });
        }
        out
    }

    /// Intermodulation, measured the way it matters: how much of the output
    /// is not a scaled copy of the input.
    fn distortion_db(want: &[Complex32], got: &[Complex32]) -> f32 {
        let mut num = Complex32::new(0.0, 0.0);
        let mut den = 0.0f32;
        for (w, g) in want.iter().zip(got) {
            num += g * w.conj();
            den += w.norm_sqr();
        }
        let k = num / den;
        let err: f32 = want.iter().zip(got).map(|(w, g)| (g - w * k).norm_sqr()).sum();
        let sig: f32 = want.iter().map(|w| (w * k).norm_sqr()).sum();
        10.0 * (err / sig).log10()
    }

    /// The whole point: run the loop against a compressing amplifier with a
    /// delay and a local-oscillator offset in the feedback path, and the
    /// residual distortion should fall by a long way.
    #[test]
    fn a_compressing_amplifier_is_linearised() {
        let rate = 1.0e6;
        let offset_hz = 250_000.0;
        let delay = 3000usize;
        let mut ps = PureSignal::new(32, 0.9, rate);

        // The amplifier's own distortion, before any of this.
        let probe = source(20_000, 5);
        let raw: Vec<Complex32> = probe.iter().map(|x| amplifier(*x)).collect();
        let before = distortion_db(&probe, &raw);

        // The feedback path: a delay line, an unknown gain and phase, the LO
        // difference the receiver sees it at, and the receiver's own noise —
        // about 30 dB down, which is a coupler and an attenuator chosen so the
        // second chain is not driven into compression by its own job.
        let mut line: Vec<Complex32> = vec![Complex32::new(0.0, 0.0); delay];
        let coupler = Complex32::from_polar(0.11, 2.0);
        let mut phase = 0usize;
        let mut hiss = source(8192 * 41, 4242).into_iter();

        for round in 0..40 {
            let mut block = source(8192, 100 + round);
            let wanted = block.clone();
            ps.predistort(&mut block);
            // Through the amplifier, the coupler and the second receiver.
            for s in &block {
                let n = hiss.next().unwrap_or(Complex32::new(0.0, 0.0));
                line.push(amplifier(*s) * coupler + n * 0.004);
            }
            let fb: Vec<Complex32> = line
                .drain(..8192)
                .enumerate()
                .map(|(i, s)| {
                    let ph =
                        (std::f64::consts::TAU * offset_hz * ((phase + i) as f64) / rate) as f32;
                    s * Complex32::from_polar(1.0, ph)
                })
                .collect();
            phase += 8192;
            ps.feed_back(&fb, offset_hz, rate);
            let _ = wanted;
        }
        assert!(ps.locked(), "the feedback never aligned (score {:.2})", ps.score());

        // Now measure what the cascade does with the learned table.
        let mut test = probe.clone();
        ps.predistort(&mut test);
        let out: Vec<Complex32> = test.iter().map(|x| amplifier(*x)).collect();
        let after = distortion_db(&probe, &out);
        assert!(
            after < before - 15.0,
            "distortion went from {before:.1} dB to {after:.1} dB — not an improvement worth \
             the trouble"
        );
    }

    /// A networked radio's loop delay — a Pluto over Ethernet, issue #525 —
    /// is hundreds of milliseconds, far past the default history: the
    /// default never locks on it, and a history long enough does. Run at a
    /// low rate so the test stays quick; what matters is delay against
    /// history, not the rate.
    #[test]
    fn a_long_loop_delay_locks_with_a_long_enough_history() {
        let rate = 200_000.0;
        // 0.8 s. The default keeps at least 65536 samples and holds up to
        // twice that, so at this low rate it reaches 0.65 s; this is past
        // that and inside the one-second history.
        let delay = 160_000usize;
        let run = |mut ps: PureSignal| {
            let mut line: Vec<Complex32> = vec![Complex32::new(0.0, 0.0); delay];
            let coupler = Complex32::from_polar(0.11, 1.0);
            for round in 0..70 {
                let mut block = source(8192, 300 + round);
                ps.predistort(&mut block);
                line.extend(block.iter().map(|s| amplifier(*s) * coupler));
                let fb: Vec<Complex32> = line.drain(..8192).collect();
                ps.feed_back(&fb, 0.0, rate);
            }
            ps
        };
        let short = run(PureSignal::new(32, 0.9, rate));
        assert!(!short.locked(), "the default history should not reach a 0.8 s delay");
        let long = run(PureSignal::with_history(32, 0.9, rate, 1.0));
        assert!(long.locked(), "a one-second history never found it (score {:.2})", long.score());
        assert!(long.correction_db() > 0.5, "locked, but learned nothing");
    }

    /// Feedback that is not the transmission — an unconnected coupler, a
    /// receiver hearing the band instead — never locks, and an unlocked loop
    /// leaves the transmitter exactly as it was.
    #[test]
    fn unrelated_feedback_never_touches_the_transmitter() {
        let mut ps = PureSignal::new(32, 0.9, 1.0e6);
        for round in 0..20 {
            let mut block = source(8192, 200 + round);
            let before = block.clone();
            ps.predistort(&mut block);
            assert_eq!(block, before, "the table moved without a lock");
            let junk = source(8192, 900 + round);
            assert!(!ps.feed_back(&junk, 0.0, 1.0e6));
        }
        assert!(!ps.locked());
        for g in ps.table() {
            assert_eq!(*g, Complex32::new(1.0, 0.0));
        }
    }

    /// However the table is learned, it can never ask the converter for more
    /// than full scale — the property that makes a broken feedback path safe.
    #[test]
    fn the_table_can_never_raise_the_peak() {
        let mut ps = PureSignal::new(16, 1.0, 1.0e6);
        // Force a wild table, as a feedback path reading nonsense would.
        for (i, g) in ps.table.iter_mut().enumerate() {
            *g = Complex32::from_polar(100.0 * (i + 1) as f32, i as f32);
        }
        ps.normalise();
        for (i, g) in ps.table().iter().enumerate() {
            let a = (i + 1) as f32 / 16.0;
            assert!(
                a * g.norm() <= 1.0001,
                "bin {i} would drive the converter to {:.3} of full scale",
                a * g.norm()
            );
            assert!(g.norm() <= MAX_CORRECTION + 1e-3);
        }
    }

    /// Holding the table stops it moving, which is what lets an operator keep
    /// a correction learned on a clean over.
    #[test]
    fn holding_the_table_stops_it_learning() {
        let rate = 1.0e6;
        let mut ps = PureSignal::new(32, 0.9, rate);
        let mut line: Vec<Complex32> = vec![Complex32::new(0.0, 0.0); 2048];
        for round in 0..20 {
            let mut block = source(8192, 300 + round);
            ps.predistort(&mut block);
            for s in &block {
                line.push(amplifier(*s) * 0.2);
            }
            let fb: Vec<Complex32> = line.drain(..8192).collect();
            ps.feed_back(&fb, 0.0, rate);
        }
        assert!(ps.locked());
        let held: Vec<Complex32> = ps.table().to_vec();
        ps.set_frozen(true);
        for round in 0..10 {
            let mut block = source(8192, 700 + round);
            ps.predistort(&mut block);
            for s in &block {
                line.push(amplifier(*s) * 0.9);
            }
            let fb: Vec<Complex32> = line.drain(..8192).collect();
            ps.feed_back(&fb, 0.0, rate);
        }
        assert_eq!(ps.table(), &held[..], "the table moved while held");
    }

    /// The coarse search runs on a decimated envelope of the reference, and
    /// every position it reports is multiplied back up by the decimation — so
    /// an envelope that has drifted out of step with the samples it describes
    /// puts the alignment out by a whole group. Blocks do not arrive in
    /// multiples of the decimation, so that is the case to pin.
    #[test]
    fn the_envelope_stays_in_step_with_awkward_block_sizes() {
        let mut ps = PureSignal::new(16, 0.5, 1.0e6);
        let mut all: Vec<Complex32> = Vec::new();
        // Deliberately not multiples of ENV_DECIM, and not all the same.
        for (i, len) in [1000usize, 37, 4097, 64, 1, 9999].iter().enumerate() {
            let mut block = source(*len, 50 + i as u64);
            all.extend_from_slice(&block);
            ps.predistort(&mut block);
        }
        assert_eq!(ps.env.len(), all.len() / ENV_DECIM, "wrong number of envelope entries");
        for (g, e) in ps.env.iter().enumerate() {
            let want: f32 =
                all[g * ENV_DECIM..(g + 1) * ENV_DECIM].iter().map(|s| s.norm()).sum::<f32>()
                    / ENV_DECIM as f32;
            assert!(
                (e - want).abs() < 1e-4,
                "envelope group {g} is {e}, but its samples average {want}"
            );
        }
    }

    /// The interpolation holds the ends rather than running off them, and is
    /// continuous in between.
    #[test]
    fn the_table_is_interpolated_and_its_ends_are_held() {
        let mut ps = PureSignal::new(4, 0.5, 1.0e6);
        ps.table = vec![
            Complex32::new(0.5, 0.0),
            Complex32::new(0.75, 0.0),
            Complex32::new(1.0, 0.0),
            Complex32::new(1.25, 0.0),
        ];
        assert_eq!(ps.gain_at(0.0), Complex32::new(0.5, 0.0));
        assert_eq!(ps.gain_at(2.0), Complex32::new(1.25, 0.0));
        // Half way between the first two bin centres.
        let mid = ps.gain_at(0.25);
        assert!((mid.re - 0.625).abs() < 1e-5, "{mid:?}");
    }
}
