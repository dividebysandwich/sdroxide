//! The level-dependent stages of the voice chain: gate, de-esser, compressor
//! and the output limiter. Each works on 48 kHz mono `f32` in place, keeps its
//! detector state across calls so the result does not depend on how the audio
//! was cut into blocks, and records the deepest gain reduction it applied for
//! the meters.

use sdroxide_types::{TxCompState, TxDeEssState, TxGateState, TxOutputState};

use crate::eq::{Biquad, Coeffs};

/// Below this a level is silence, as far as a detector is concerned — keeps
/// `log10` away from zero.
const FLOOR_LIN: f32 = 1e-6;

#[inline]
pub(super) fn lin_to_db(x: f32) -> f32 {
    20.0 * x.max(FLOOR_LIN).log10()
}

#[inline]
pub(super) fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// One-pole smoothing coefficient for a time constant of `ms` at `rate`.
pub(super) fn coef(ms: f32, rate: f64) -> f32 {
    let samples = (ms as f64 / 1000.0 * rate).max(1e-3);
    (1.0 - (-1.0 / samples).exp()) as f32
}

/// `ms` as a whole number of samples at `rate`.
fn samples(ms: f32, rate: f64) -> usize {
    (ms as f64 / 1000.0 * rate).round().max(0.0) as usize
}

/// A fixed delay line: what a look-ahead detector runs ahead of.
struct Delay {
    buf: Vec<f32>,
    pos: usize,
}

impl Delay {
    fn new() -> Self {
        Delay { buf: Vec::new(), pos: 0 }
    }

    /// Change the length, clearing the line only when it actually changes.
    fn set_len(&mut self, len: usize) {
        if len != self.buf.len() {
            self.buf = vec![0.0; len];
            self.pos = 0;
        }
    }

    fn clear(&mut self) {
        self.buf.fill(0.0);
        self.pos = 0;
    }

    #[inline]
    fn push(&mut self, x: f32) -> f32 {
        if self.buf.is_empty() {
            return x;
        }
        let out = std::mem::replace(&mut self.buf[self.pos], x);
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }
}

// ── Gate / expander ─────────────────────────────────────────────────────────

/// Detector times for the gate. Fast enough to see a syllable start, slow
/// enough that the level of a 100 Hz voice does not ripple through it.
const GATE_DETECT_ATTACK_MS: f32 = 0.5;
const GATE_DETECT_RELEASE_MS: f32 = 15.0;
/// How fast the gate opens. Quick, so a word is never chopped; the look-ahead
/// is what keeps this from clicking.
const GATE_OPEN_MS: f32 = 1.0;

pub(super) struct Gate {
    threshold_db: f32,
    close_db: f32,
    floor_db: f32,
    slope: f32,
    hold: usize,
    det_attack: f32,
    det_release: f32,
    open_coef: f32,
    close_coef: f32,
    delay: Delay,

    env: f32,
    open: bool,
    hold_left: usize,
    gain_db: f32,
    pub(super) gr_db: f32,
}

impl Gate {
    pub(super) fn new() -> Self {
        Gate {
            threshold_db: 0.0,
            close_db: 0.0,
            floor_db: 0.0,
            slope: 0.0,
            hold: 0,
            det_attack: 1.0,
            det_release: 1.0,
            open_coef: 1.0,
            close_coef: 1.0,
            delay: Delay::new(),
            env: 0.0,
            open: false,
            hold_left: 0,
            gain_db: 0.0,
            gr_db: 0.0,
        }
    }

    pub(super) fn configure(&mut self, cfg: &TxGateState, rate: f64) {
        self.threshold_db = cfg.threshold_db;
        self.close_db = cfg.threshold_db - cfg.return_db;
        self.floor_db = cfg.floor_db;
        self.slope = cfg.ratio - 1.0;
        self.hold = samples(cfg.hold_ms, rate);
        self.det_attack = coef(GATE_DETECT_ATTACK_MS, rate);
        self.det_release = coef(GATE_DETECT_RELEASE_MS, rate);
        self.open_coef = coef(GATE_OPEN_MS, rate);
        self.close_coef = coef(cfg.release_ms, rate);
        self.delay.set_len(samples(cfg.lookahead_ms, rate));
    }

    pub(super) fn latency(&self) -> usize {
        self.delay.buf.len()
    }

    pub(super) fn reset(&mut self) {
        self.delay.clear();
        self.env = 0.0;
        self.open = false;
        self.hold_left = 0;
        // Start closed: an over that opens on silence should not let the first
        // millisecond of room noise through at full level.
        self.gain_db = self.floor_db;
    }

    pub(super) fn process(&mut self, audio: &mut [f32]) {
        for s in audio.iter_mut() {
            let level = s.abs();
            let c = if level > self.env { self.det_attack } else { self.det_release };
            self.env += c * (level - self.env);
            let env_db = lin_to_db(self.env);

            if env_db >= self.threshold_db {
                self.open = true;
                self.hold_left = self.hold;
            } else if self.open {
                if env_db >= self.close_db {
                    // Inside the hysteresis band: stays open, and the hold
                    // only starts counting once the level has left it.
                    self.hold_left = self.hold;
                } else if self.hold_left > 0 {
                    self.hold_left -= 1;
                } else {
                    self.open = false;
                }
            }

            let target = if self.open {
                0.0
            } else {
                ((env_db - self.threshold_db) * self.slope).clamp(self.floor_db, 0.0)
            };
            let c = if target > self.gain_db { self.open_coef } else { self.close_coef };
            self.gain_db += c * (target - self.gain_db);
            self.gr_db = self.gr_db.min(self.gain_db);

            *s = self.delay.push(*s) * db_to_lin(self.gain_db);
        }
    }
}

// ── De-esser ────────────────────────────────────────────────────────────────

pub(super) struct DeEsser {
    band: Biquad,
    threshold_db: f32,
    amount_db: f32,
    attack: f32,
    release: f32,
    env: f32,
    pub(super) gr_db: f32,
}

impl DeEsser {
    pub(super) fn new() -> Self {
        DeEsser {
            band: Biquad::new(),
            threshold_db: 0.0,
            amount_db: 0.0,
            attack: 1.0,
            release: 1.0,
            env: 0.0,
            gr_db: 0.0,
        }
    }

    pub(super) fn configure(&mut self, cfg: &TxDeEssState, rate: f64) {
        self.band.set(Coeffs::bandpass(cfg.freq_hz as f64, cfg.q as f64, rate));
        self.threshold_db = cfg.threshold_db;
        self.amount_db = cfg.amount_db;
        self.attack = coef(cfg.attack_ms, rate);
        self.release = coef(cfg.release_ms, rate);
    }

    pub(super) fn reset(&mut self) {
        self.band.reset();
        self.env = 0.0;
    }

    pub(super) fn process(&mut self, audio: &mut [f32]) {
        for s in audio.iter_mut() {
            let band = self.band.process(*s);
            let level = band.abs();
            let c = if level > self.env { self.attack } else { self.release };
            self.env += c * (level - self.env);
            // Above the threshold the band is held at it — an infinite ratio,
            // which is what a de-esser is — but never turned down by more
            // than the operator allowed.
            let over = lin_to_db(self.env) - self.threshold_db;
            let gr = if over > 0.0 { (-over).max(self.amount_db) } else { 0.0 };
            self.gr_db = self.gr_db.min(gr);
            // `x - band` is the notch around the sibilant band, so this is
            // "everything else, untouched, plus the band at gain g": the
            // original sample exactly when g = 1.
            *s += (db_to_lin(gr) - 1.0) * band;
        }
    }
}

// ── Compressor ──────────────────────────────────────────────────────────────

/// Centres of the phase rotator's all-pass sections, lowest first. Low in the
/// voice band, where the fundamental and the first few harmonics of a voice
/// line up into the lopsided peaks the rotator exists to spread out.
const ROTATOR_HZ: [f64; 6] = [150.0, 210.0, 300.0, 420.0, 600.0, 850.0];
const ROTATOR_Q: f64 = 0.7;

pub(super) struct Compressor {
    cfg: TxCompState,
    drive: f32,
    attack: f32,
    release: f32,
    rotator: Vec<Biquad>,
    /// How many of `rotator` are in use.
    stages: usize,
    /// Smoothed gain reduction, dB, `<= 0`.
    gain_db: f32,
    pub(super) gr_db: f32,
}

impl Compressor {
    pub(super) fn new() -> Self {
        Compressor {
            cfg: TxCompState::default(),
            drive: 1.0,
            attack: 1.0,
            release: 1.0,
            rotator: ROTATOR_HZ.iter().map(|_| Biquad::new()).collect(),
            stages: 0,
            gain_db: 0.0,
            gr_db: 0.0,
        }
    }

    pub(super) fn configure(&mut self, cfg: &TxCompState, rate: f64) {
        self.cfg = *cfg;
        self.drive = db_to_lin(cfg.drive_db);
        self.attack = coef(cfg.attack_ms, rate);
        self.release = coef(cfg.release_ms, rate);
        for (b, &f) in self.rotator.iter_mut().zip(&ROTATOR_HZ) {
            b.set(Coeffs::allpass(f, ROTATOR_Q, rate));
        }
        self.stages = (cfg.phase_stages as usize).min(self.rotator.len());
    }

    pub(super) fn reset(&mut self) {
        self.rotator.iter_mut().for_each(Biquad::reset);
        self.gain_db = 0.0;
    }

    pub(super) fn process(&mut self, audio: &mut [f32]) {
        let makeup = db_to_lin(self.cfg.makeup_db);
        for s in audio.iter_mut() {
            let mut x = *s * self.drive;
            for b in &mut self.rotator[..self.stages] {
                x = b.process(x);
            }
            let target = self.cfg.static_gain_db(lin_to_db(x.abs()));
            let c = if target < self.gain_db { self.attack } else { self.release };
            self.gain_db += c * (target - self.gain_db);
            self.gr_db = self.gr_db.min(self.gain_db);
            *s = x * db_to_lin(self.gain_db) * makeup;
        }
    }
}

// ── Output: trim and look-ahead limiter ─────────────────────────────────────

const LIMITER_LOOKAHEAD_MS: f32 = 1.5;
const LIMITER_RELEASE_MS: f32 = 60.0;

/// Trim, then a brickwall limiter that never lets a sample past the ceiling.
///
/// The audio is delayed by `L` samples. For each incoming sample the gain it
/// would need is worked out; the gain applied is the minimum of that over the
/// last `L + 1` samples, released slowly, then averaged over another `L + 1`.
/// Every value in that average is at most the gain the sample leaving the
/// delay needs, so the average is too — the ceiling holds exactly, and the
/// averaging turns each gain change into a ramp `L` samples long instead of
/// the step that a hard clip is.
pub(super) struct Output {
    trim: f32,
    limiter: bool,
    ceiling: f32,
    release: f32,
    delay: Delay,
    need: Vec<f32>,
    held: Vec<f32>,
    pos: usize,
    released: f32,
    pub(super) gr_db: f32,
}

impl Output {
    pub(super) fn new() -> Self {
        Output {
            trim: 1.0,
            limiter: false,
            ceiling: 1.0,
            release: 1.0,
            delay: Delay::new(),
            need: Vec::new(),
            held: Vec::new(),
            pos: 0,
            released: 1.0,
            gr_db: 0.0,
        }
    }

    pub(super) fn configure(&mut self, cfg: &TxOutputState, rate: f64) {
        self.trim = db_to_lin(cfg.trim_db);
        self.limiter = cfg.limiter;
        self.ceiling = db_to_lin(cfg.ceiling_db);
        self.release = coef(LIMITER_RELEASE_MS, rate);
        let l = samples(LIMITER_LOOKAHEAD_MS, rate).max(1);
        if self.need.len() != l + 1 {
            self.delay.set_len(l);
            self.need = vec![1.0; l + 1];
            self.held = vec![1.0; l + 1];
            self.pos = 0;
        }
    }

    pub(super) fn latency(&self) -> usize {
        if self.limiter { self.delay.buf.len() } else { 0 }
    }

    pub(super) fn reset(&mut self) {
        self.delay.clear();
        self.need.fill(1.0);
        self.held.fill(1.0);
        self.pos = 0;
        self.released = 1.0;
    }

    pub(super) fn process(&mut self, audio: &mut [f32]) {
        if !self.limiter {
            if self.trim != 1.0 {
                audio.iter_mut().for_each(|s| *s *= self.trim);
            }
            return;
        }
        let n = self.need.len();
        for s in audio.iter_mut() {
            let x = *s * self.trim;
            let a = x.abs();
            self.need[self.pos] = if a > self.ceiling { self.ceiling / a } else { 1.0 };
            let min = self.need.iter().copied().fold(1.0f32, f32::min);
            // Down at once, back up slowly — and never above `min`, which is
            // what keeps the guarantee above intact.
            self.released = (self.released + self.release * (min - self.released)).min(min);
            self.held[self.pos] = self.released;
            self.pos = (self.pos + 1) % n;
            let g = self.held.iter().sum::<f32>() / n as f32;
            self.gr_db = self.gr_db.min(lin_to_db(g));
            *s = self.delay.push(x) * g;
        }
    }
}
