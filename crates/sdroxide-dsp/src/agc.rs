//! Hang AGC with lookahead delay: fast attack, configurable hang time,
//! slow recovery, capped maximum gain.
//!
//! Switched off it becomes a fixed manual gain rather than a bypass. A bypass
//! is unity gain on the demodulator's own output, and that output is whatever
//! the antenna delivered — an SSB signal sitting 60 dB below full scale stays
//! 60 dB below full scale, which no volume control can rescue. The tracker
//! keeps running while off so the manual level can be seeded from it and so
//! switching back on doesn't jump.

use std::collections::VecDeque;

use sdroxide_types::{AgcMode, MAX_MANUAL_GAIN_DB};

const TARGET: f32 = 0.35;
const LOOKAHEAD_S: f32 = 0.020;
const ATTACK_TC_S: f32 = 0.002;
const RECOVERY_TC_S: f32 = 0.250;
const ENV_DECAY_TC_S: f32 = 0.050;
/// The loudest thing the envelope tracker will believe: sixty decibels above
/// full scale.
///
/// Nothing a demodulator produces legitimately reaches it — the whole chain
/// works in units where 1.0 is full scale — so anything above this is a fault
/// upstream, and the only question is how long the receiver is deaf afterwards.
/// The envelope decays at [`ENV_DECAY_TC_S`], so from here it is back at the
/// target inside four hundred milliseconds; from `f32::MAX` it would be four
/// and a half seconds, and from an infinity it would never come back at all
/// (see [`Agc::step`]).
const ENV_CEILING: f32 = 1_000.0;
/// Manual gain before anyone has set one. Matches `RxState::manual_gain_db`'s
/// own default, so a fresh chain and a fresh receiver agree.
const DEFAULT_MANUAL_GAIN_DB: f32 = 20.0;

pub struct Agc {
    rate: f32,
    enabled: bool,
    max_gain: f32,
    /// Linear gain applied while the AGC is off.
    manual_gain: f32,
    gain: f32,
    env: f32,
    hang_samples: u32,
    hang_left: u32,
    attack_alpha: f32,
    recovery_alpha: f32,
    env_decay: f32,
    delay: VecDeque<f32>,
    /// Second lookahead line, used only by [`Agc::process_pair`] so the side
    /// channel comes out delayed by exactly as much as the main one.
    delay_b: VecDeque<f32>,
    delay_len: usize,
    /// Samples that were not numbers, since the count was last read. Kept
    /// because surviving one is not the same as it being all right that one
    /// arrived: something upstream computed it, and the count is the only
    /// evidence of that there will ever be — see [`Agc::take_impossible`].
    impossible: u64,
}

impl Agc {
    pub fn new(sample_rate: f64) -> Self {
        let rate = sample_rate as f32;
        let mut agc = Agc {
            rate,
            enabled: true,
            max_gain: 10f32.powf(90.0 / 20.0),
            manual_gain: 1.0,
            gain: 1.0,
            env: 0.0,
            hang_samples: 0,
            hang_left: 0,
            attack_alpha: 1.0 - (-1.0 / (rate * ATTACK_TC_S)).exp(),
            recovery_alpha: 1.0 - (-1.0 / (rate * RECOVERY_TC_S)).exp(),
            env_decay: (-1.0 / (rate * ENV_DECAY_TC_S)).exp(),
            delay: VecDeque::new(),
            delay_b: VecDeque::new(),
            delay_len: (rate * LOOKAHEAD_S) as usize,
            impossible: 0,
        };
        agc.set_mode(AgcMode::Med);
        // Only reached if something switches the AGC off before any audio has
        // run through it; anything that goes on to do so re-seeds this from the
        // level the tracker had settled at.
        agc.set_manual_gain_db(DEFAULT_MANUAL_GAIN_DB);
        agc
    }

    /// Switching between the two is glitch-free: the lookahead delay and the
    /// gain tracker run in both states, so nothing is flushed and re-enabling
    /// resumes at the level the signal already warrants.
    pub fn set_mode(&mut self, mode: AgcMode) {
        match mode.hang_ms() {
            Some(ms) => {
                self.enabled = true;
                self.hang_samples = (self.rate * ms / 1000.0) as u32;
            }
            None => self.enabled = false,
        }
    }

    pub fn set_max_gain_db(&mut self, db: f32) {
        self.max_gain = 10f32.powf(db.clamp(0.0, 120.0) / 20.0);
    }

    /// The fixed gain used while the AGC is off.
    pub fn set_manual_gain_db(&mut self, db: f32) {
        self.manual_gain = 10f32.powf(db.clamp(0.0, MAX_MANUAL_GAIN_DB) / 20.0);
    }

    /// What the tracker is applying right now, in dB. Seeds the manual gain
    /// when the operator switches the AGC off, so the level carries over
    /// instead of dropping to whatever was last set by hand.
    pub fn gain_db(&self) -> f32 {
        20.0 * self.gain.max(1e-9).log10()
    }

    /// Samples that were not numbers since this was last called, and reset.
    ///
    /// Nothing in a receive chain should ever produce one, and the AGC is the
    /// stage where it does the most damage, so it is also the stage best placed
    /// to say that one came through — see [`Agc::step`] for what it used to
    /// cost. The engine reports it once per chain; the count is what turns the
    /// next report of a receiver gone quiet into a diagnosis.
    pub fn take_impossible(&mut self) -> u64 {
        std::mem::replace(&mut self.impossible, 0)
    }

    /// A sample, or silence where there was no sample. Anything that is not a
    /// number is not audio: played it is a click at best and a muted stream at
    /// worst, and carried into the tracker it is worse than that.
    #[inline]
    fn finite(&mut self, x: f32) -> f32 {
        if x.is_finite() {
            x
        } else {
            self.impossible += 1;
            0.0
        }
    }

    /// In-place. Output is the delayed input scaled by the tracked gain — or by
    /// the manual gain while the AGC is off.
    pub fn process(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            let x = self.finite(*s);
            let (d, _) = self.push_delay(x, 0.0);
            *s = d * self.step(x.abs());
        }
    }

    /// Two channels sharing one gain trajectory and one lookahead delay.
    ///
    /// The same gain lands on both, so the ratio between them — which *is* the
    /// stereo image — survives untouched; letting each channel level itself
    /// would make the image pump. `side` rides a parallel delay line so the two
    /// stay sample-aligned.
    ///
    /// The envelope is driven by `|main| + |side|`, which is exactly
    /// `max(|L|, |R|)` after the matrix. Using `|main|` alone would let strongly
    /// anti-phase content (sum near zero, difference large) wind the gain up to
    /// its limit and clip both ears.
    pub fn process_pair(&mut self, main: &mut [f32], side: &mut [f32]) {
        debug_assert_eq!(main.len(), side.len(), "AGC channel pair must be equal length");
        for (m, sd) in main.iter_mut().zip(side.iter_mut()) {
            let (x, y) = (self.finite(*m), self.finite(*sd));
            let (dm, ds) = self.push_delay(x, y);
            let g = self.step(x.abs() + y.abs());
            *m = dm * g;
            *sd = ds * g;
        }
    }

    /// Advance both lookahead lines by one sample and return what falls out.
    ///
    /// Both are advanced on every path, including mono `process`, so they can
    /// never drift out of step: a mono interlude that fed only one of them would
    /// leave the other holding stale side audio, and the pair would come back
    /// misaligned by however many mono samples had run.
    #[inline]
    fn push_delay(&mut self, main: f32, side: f32) -> (f32, f32) {
        self.delay.push_back(main);
        self.delay_b.push_back(side);
        if self.delay.len() > self.delay_len {
            (self.delay.pop_front().unwrap(), self.delay_b.pop_front().unwrap())
        } else {
            (0.0, 0.0)
        }
    }

    /// Advance the envelope/gain tracker by one sample and return the gain to
    /// apply. Envelope: instant attack, exponential decay — it sees the sample
    /// `LOOKAHEAD_S` before that sample leaves the delay line.
    ///
    /// The tracker advances even while the AGC is off — the manual gain is what
    /// gets *applied* then, but keeping `gain` converged is what makes switching
    /// back on silent and gives [`Agc::gain_db`] something honest to report.
    ///
    /// The envelope is held finite and bounded, which is not housekeeping: it
    /// is multiplied by its own decay on every sample, so a value that cannot
    /// decay stays for ever. An infinity written into it makes `desired` zero
    /// permanently — the gain attacks to nothing and never recovers, and the
    /// receiver is silent for the rest of the session. Only the levelled path
    /// dies, because the manual gain is a constant: switching the AGC off
    /// brings the audio straight back, which is what makes this present as an
    /// AGC that has stopped working rather than as the one bad sample it is
    /// (issue #305). A merely enormous sample is survivable but not free, so it
    /// is trimmed to [`ENV_CEILING`] as well.
    fn step(&mut self, a: f32) -> f32 {
        // A sample the chain upstream could not compute says nothing about the
        // signal's level, so the envelope keeps the level it had rather than
        // taking a number that is not one.
        let a = if a.is_finite() { a.min(ENV_CEILING) } else { self.env };
        self.env = if a > self.env { a } else { self.env * self.env_decay };

        let desired = (TARGET / self.env.max(1e-9)).min(self.max_gain);
        if desired < self.gain {
            self.gain += (desired - self.gain) * self.attack_alpha;
            self.hang_left = self.hang_samples;
        } else if self.hang_left > 0 {
            self.hang_left -= 1;
        } else {
            self.gain += (desired - self.gain) * self.recovery_alpha;
        }
        if self.enabled { self.gain } else { self.manual_gain }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A steady tone at `amp`, `secs` of it.
    fn tone(amp: f32, secs: f32, rate: f32) -> Vec<f32> {
        let n = (rate * secs) as usize;
        (0..n).map(|k| amp * (std::f32::consts::TAU * 1000.0 * k as f32 / rate).sin()).collect()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    fn settled(rate: f64) -> Agc {
        let mut agc = Agc::new(rate);
        agc.set_mode(AgcMode::Med);
        agc.set_max_gain_db(90.0);
        agc.process(&mut tone(0.01, 1.0, rate as f32));
        agc
    }

    /// The whole of issue #305, and the only way this AGC can go quiet and stay
    /// quiet: one sample that is not a number.
    ///
    /// The envelope is multiplied by its own decay every sample, so an infinity
    /// in it is an infinity for ever — `TARGET / inf` is zero, the gain attacks
    /// to nothing, and nothing ever asks it back up. The receiver is deaf for
    /// the rest of the session, and only on the levelled path: the manual gain
    /// is a constant, so switching the AGC off restores the audio at once and
    /// the fault reads as "the AGC has stopped working".
    #[test]
    fn one_impossible_sample_does_not_leave_the_receiver_deaf() {
        for bad in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            let mut agc = settled(48_000.0);
            let before = agc.gain_db();
            assert!(before > 25.0, "the tracker should have wound a -40 dBFS tone up: {before}");

            agc.process(&mut vec![bad; 8]);

            // Half a second later — well inside the hang and the recovery — the
            // same tone must come back out at the same level as before.
            let mut after = tone(0.01, 0.5, 48_000.0);
            agc.process(&mut after);
            let out = peak(&after[24_000 - 4_800..]);
            assert!(
                (out - TARGET).abs() < 0.05,
                "{bad:?} left the AGC at {:.1} dB, holding the tone at {out:.4}",
                agc.gain_db()
            );
        }
    }

    /// And nothing that is not a number reaches the speakers either: a NaN
    /// played out is a click at best, and a stream some sound cards refuse to
    /// go on with at worst.
    #[test]
    fn nothing_impossible_reaches_the_output() {
        let mut agc = settled(48_000.0);
        let mut block = vec![f32::NAN; 64];
        block.extend(vec![f32::INFINITY; 64]);
        block.extend(tone(0.01, 0.1, 48_000.0));
        agc.process(&mut block);
        assert!(block.iter().all(|s| s.is_finite()), "a non-finite sample got through");
    }

    /// A merely enormous sample — an overload, not an arithmetic fault — is
    /// survivable, and must be survived in about the time the hang and the
    /// recovery take rather than in however long an unbounded envelope needs to
    /// decay back (from `f32::MAX` that alone is four and a half seconds).
    #[test]
    fn an_enormous_sample_costs_about_a_hang_and_a_recovery() {
        let mut agc = settled(48_000.0);
        agc.process(&mut vec![1e30f32; 8]);
        // Two seconds on: the envelope's own climb down from the ceiling, then
        // the 500 ms hang, then the 250 ms recovery.
        let mut after = tone(0.01, 2.0, 48_000.0);
        agc.process(&mut after);
        let out = peak(&after[96_000 - 4_800..]);
        assert!((out - TARGET).abs() < 0.05, "still at {:.1} dB, tone at {out:.4}", agc.gain_db());
    }

    /// The ordinary job, unchanged: a weak signal is brought up to the target
    /// and a strong one is brought down to it.
    #[test]
    fn the_level_is_the_target_either_way() {
        for amp in [0.001, 0.01, 0.2, 1.0] {
            let mut agc = Agc::new(48_000.0);
            agc.set_mode(AgcMode::Med);
            agc.set_max_gain_db(90.0);
            let mut x = tone(amp, 2.0, 48_000.0);
            agc.process(&mut x);
            let out = peak(&x[96_000 - 4_800..]);
            assert!((out - TARGET).abs() < 0.05, "{amp} levelled to {out:.4}");
        }
    }

    /// Switched off it is a fixed gain, and one that goes on being applied
    /// whatever the tracker is doing — which is exactly why a fault that kills
    /// the tracker is invisible here and obvious one click away.
    #[test]
    fn switched_off_it_is_the_manual_gain() {
        let mut agc = Agc::new(48_000.0);
        agc.set_mode(AgcMode::Off);
        agc.set_manual_gain_db(20.0);
        let mut x = tone(0.01, 0.5, 48_000.0);
        agc.process(&mut x);
        let out = peak(&x[24_000 - 4_800..]);
        assert!((out - 0.1).abs() < 0.005, "manual +20 dB on 0.01 gave {out:.4}");
    }
}
