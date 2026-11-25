//! Two receive antennas, one output: null a local noise source, or combine the
//! pair for the better signal-to-noise ratio of the two.
//!
//! # What "diversity" means here
//!
//! Two things, which is why this has a mode switch. Both start from the same
//! place — a second receiver, coherent with the first because the two chains
//! share one synthesiser and one sample clock — and they differ only in what
//! is done with the answer.
//!
//! * **[`DiversityMode::Cancel`]** is the QRM/QRN killer, the DSP form of what
//!   an MFJ-1026 or an X-Phase does with a knob: point a second aerial at the
//!   noise, and subtract what it hears from what the main aerial hears. The
//!   filter below finds the gain, phase and delay that make the two versions of
//!   the noise line up, and what is left is the band without it. What it costs
//!   is that anything *both* aerials hear equally goes with it, so the second
//!   one wants to be the one that hears the interference and not much else — a
//!   short whip by the offending switched-mode supply, a loop pointed at it, or
//!   simply the noisier of two aerials.
//!
//! * **[`DiversityMode::Combine`]** is diversity reception proper: two aerials
//!   on the same signal, added in the phase that makes them reinforce, weighted
//!   so the one with the better signal counts for more. On HF, where the two
//!   fade independently, this fills in the fades; the theoretical gain over the
//!   better branch alone is `1 + |r|²`, where `r` is the amplitude ratio
//!   between them — 3 dB for two equal aerials, and much more when one of them
//!   is momentarily in a null.
//!
//! # One filter, two arithmetics
//!
//! Both modes run the same adaptive filter and differ in one line at the end.
//!
//! The filter is a normalised LMS transversal filter `W` fed by the auxiliary
//! channel and driven by the error against the main one:
//!
//! ```text
//! y[n] = Σ_k w[k] · aux[n−k]
//! e[n] = main[n] − y[n]
//! w[k] += mu · e[n] · conj(aux[n−k]) / (Σ|aux|² + ε)
//! ```
//!
//! It converges on the Wiener solution: `W` becomes the transfer function that
//! turns the auxiliary channel into whatever part of the main channel can be
//! predicted from it — which is exactly the common signal, whether that is a
//! noise source or a station.
//!
//! **Multi-tap on purpose.** A single complex weight is a gain and a phase, and
//! that is all a knob-and-potentiometer canceller has; it nulls at one
//! frequency and gets steadily worse either side of it. Over the megahertz of
//! span an SDR shows, two aerials with different feedlines differ in *delay*
//! too, and a delay is a phase that turns with frequency. Each tap buys one
//! sample period of the difference the filter can equalise, so a handful of
//! them is the difference between a notch and a band that is quiet all the way
//! across. They are not free — see [`Diversity::cost_note`].
//!
//! Then:
//!
//! * `Cancel` outputs the error itself, `main − W·aux`: the common part
//!   removed.
//! * `Combine` outputs `(‖W‖²·main + W·aux) / (1 + ‖W‖²)`. That is
//!   maximal-ratio combining written in terms of the same filter: `W·aux` is
//!   the auxiliary channel referred to the main one's scale, and weighting the
//!   two branches by the inverse of their noise powers — which is what the
//!   `‖W‖²` and the `1` are — is what makes the sum optimal rather than merely
//!   in phase. It assumes the two branches have **comparable noise floors**,
//!   so the auxiliary chain's gain is a real setting and not a convenience:
//!   set it so the band noise reads about the same on both.
//!
//! # What it cannot do
//!
//! Nothing here separates a wanted signal from an unwanted one; the filter only
//! knows what the two channels have in common. Point both aerials at the same
//! thing in `Cancel` and it will dutifully cancel the station you are listening
//! to. The honest workflow is to watch the waterfall while it converges, and
//! [`Diversity::set_frozen`] the moment it has: a converged null holds while
//! the band changes around it, and a filter left adapting will re-aim itself at
//! whatever is loudest.

use crate::Complex32;

/// What to do with the two channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiversityMode {
    /// Subtract what the auxiliary aerial hears: kill the noise source.
    Cancel,
    /// Add the two in phase, weighted by branch quality: diversity reception.
    Combine,
}

/// Slowest and fastest normalised step size the rate control reaches.
///
/// The bottom is a filter that takes tens of seconds to settle and then sits
/// still — what a stationary noise source wants. The top converges inside a
/// fraction of a second and visibly hunts, which is what makes it useful for
/// *finding* the null before freezing it. Above about 1.0 an NLMS filter is no
/// longer guaranteed stable, so the ceiling is well short of it.
const MU_MIN: f32 = 1.0e-4;
const MU_MAX: f32 = 0.5;

/// Keeps the normalisation finite on a dead auxiliary channel.
const EPS: f32 = 1.0e-12;

/// A weight vector longer than the signal it is fitting will wander; a
/// divergent one is worse, because it turns the output into noise louder than
/// the input. Anything past this is a reset rather than a slow recovery.
const MAX_WEIGHT_ENERGY: f32 = 1.0e6;

/// One over the time constant, in samples, of the input-power average that
/// normalises the step size. About four thousand samples: long enough to
/// average out a modulation envelope, short enough to follow a gain change.
const POWER_TAU_INV: f32 = 1.0 / 4096.0;

/// How often, in samples, the combining weight `‖W‖²` is recomputed inside a
/// block. Short enough that a filter converging mid-block is followed, long
/// enough that the recomputation is not part of the per-sample cost.
const WNORM_INTERVAL: usize = 256;

/// How much of the previous block's power estimate survives into the next one.
/// Only the reported depth depends on this, not the filter.
const POWER_DECAY: f32 = 0.9;

pub struct Diversity {
    mode: DiversityMode,
    mu: f32,
    frozen: bool,
    /// The filter, `w[0]` multiplying the newest auxiliary sample.
    w: Vec<Complex32>,
    /// The auxiliary delay line, written at [`Self::pos`].
    hist: Vec<Complex32>,
    pos: usize,
    /// `Σ|aux|²` over the delay line, maintained incrementally: recomputing it
    /// per sample would double the work.
    energy: f32,
    /// [`Self::energy`] smoothed over [`POWER_TAU`] samples, and the actual
    /// step-size normaliser — see the note on the update in [`Self::process`].
    pow: f32,
    in_pow: f32,
    out_pow: f32,
}

impl Diversity {
    /// `taps` is clamped to 1..=[`Self::MAX_TAPS`]; `rate` is the 0..1 control,
    /// not a step size — see [`Self::mu_for_rate`].
    pub fn new(mode: DiversityMode, taps: usize, rate: f32) -> Diversity {
        let taps = taps.clamp(1, Self::MAX_TAPS);
        Diversity {
            mode,
            mu: Self::mu_for_rate(rate),
            frozen: false,
            w: vec![Complex32::new(0.0, 0.0); taps],
            hist: vec![Complex32::new(0.0, 0.0); taps],
            pos: 0,
            energy: 0.0,
            pow: 0.0,
            in_pow: 0.0,
            out_pow: 0.0,
        }
    }

    /// The longest filter this will build. At the top sample rates it is
    /// already more arithmetic than the rest of the receive chain put
    /// together, and a filter this long is equalising a path difference no
    /// pair of aerials on one site has. Kept in step with
    /// `sdroxide_types::LimeAuxConfig::MAX_TAPS`, which is what the settings
    /// panel bounds its control by; this one is the backstop.
    pub const MAX_TAPS: usize = 64;

    /// The 0..1 rate control mapped to a normalised step size, logarithmically
    /// — the useful range spans three and a half decades, and a linear slider
    /// would spend nine tenths of its travel in the part that hunts.
    pub fn mu_for_rate(rate: f32) -> f32 {
        let r = rate.clamp(0.0, 1.0);
        MU_MIN * (MU_MAX / MU_MIN).powf(r)
    }

    pub fn mode(&self) -> DiversityMode {
        self.mode
    }

    /// Changing mode keeps the filter: it is the same estimate either way, and
    /// throwing away a converged one to answer a switch would cost the operator
    /// the convergence they just watched happen.
    pub fn set_mode(&mut self, mode: DiversityMode) {
        self.mode = mode;
    }

    pub fn taps(&self) -> usize {
        self.w.len()
    }

    /// Resize the filter, which necessarily starts it again — the taps mean
    /// different delays now.
    pub fn set_taps(&mut self, taps: usize) {
        let taps = taps.clamp(1, Self::MAX_TAPS);
        if taps == self.w.len() {
            return;
        }
        self.w = vec![Complex32::new(0.0, 0.0); taps];
        self.hist = vec![Complex32::new(0.0, 0.0); taps];
        self.pos = 0;
        self.energy = 0.0;
        self.pow = 0.0;
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.mu = Self::mu_for_rate(rate);
    }

    /// Stop adapting, holding the filter where it is. The whole point of the
    /// control: a null found while the band was quiet is worth keeping when it
    /// is not.
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    pub fn frozen(&self) -> bool {
        self.frozen
    }

    /// Zero the filter and start again — the answer when the aerials, the band
    /// or the noise source have changed under a frozen null.
    pub fn reset(&mut self) {
        self.w.fill(Complex32::new(0.0, 0.0));
        self.hist.fill(Complex32::new(0.0, 0.0));
        self.pos = 0;
        self.energy = 0.0;
        self.pow = 0.0;
        self.in_pow = 0.0;
        self.out_pow = 0.0;
    }

    /// How much quieter the output is than the main channel was, in dB.
    ///
    /// In `Cancel` this is the null depth, and it is the number to watch: a
    /// converged canceller on a real noise source reads 15–30 dB, and one
    /// reading a fraction of a decibel is one whose auxiliary aerial cannot
    /// hear what it is being asked to subtract. In `Combine` it is not a
    /// figure of merit at all — the output is meant to be about as loud as the
    /// input — so it is reported only for the one mode that means something.
    pub fn depth_db(&self) -> Option<f32> {
        if self.mode != DiversityMode::Cancel || self.in_pow <= 0.0 || self.out_pow <= 0.0 {
            return None;
        }
        Some(10.0 * (self.in_pow / self.out_pow).log10())
    }

    /// Combine one block. `main` is replaced by the result; `aux` must be the
    /// **same samples in time** from the second chain, which is the caller's
    /// side of the bargain — a block pair that is not sample-aligned produces a
    /// filter that fits a delay that is not there.
    ///
    /// A short `aux` processes only the overlap and leaves the rest of `main`
    /// as it came, which is the right answer for a chain that has stalled: no
    /// cancellation beats cancellation against the wrong samples.
    pub fn process(&mut self, main: &mut [Complex32], aux: &[Complex32]) {
        let n = main.len().min(aux.len());
        if n == 0 {
            return;
        }
        let taps = self.w.len();
        let mut wnorm = 0.0f32;
        let mut in_acc = 0.0f32;
        let mut out_acc = 0.0f32;

        let mut pos = self.pos;
        for i in 0..n {
            // `‖W‖²` is only needed by the combining arithmetic, and it moves
            // at the adaptation's pace — thousands of samples slow. Refreshing
            // it every [`WNORM_INTERVAL`] samples rather than every one keeps
            // it a rounding error on the cost, while still tracking a filter
            // that converges inside a block. Doing it once per *block* would
            // not: a caller that hands over a whole second at a time would
            // combine the entire first block against a weight of zero.
            if i % WNORM_INTERVAL == 0 {
                wnorm = self.w.iter().map(|w| w.norm_sqr()).sum();
                if !wnorm.is_finite() || wnorm > MAX_WEIGHT_ENERGY {
                    // Diverged. The output would be louder than the input and
                    // made of nothing, so start again and let the rest of the
                    // block through untouched — silently, because a DSP block
                    // on the sample path has no business logging, and the
                    // caller is watching [`Self::depth_db`] anyway.
                    self.reset();
                    return;
                }
                // The running `Σ|aux|²` below is kept by adding and
                // subtracting in f32, and is only repaired when it goes
                // negative. After a burst near full scale — the rig keying
                // beside the aerial — the rounding left over from those large
                // terms can sit far above a quiet band's true energy, and the
                // step size divides by it: the filter then barely adapts until
                // a Restart. Recomputing it here costs `taps` multiplies every
                // few hundred samples.
                self.energy = self.hist.iter().map(|h| h.norm_sqr()).sum();
            }
            let x = aux[i];
            let m = main[i];
            // Slide the delay line one sample on, keeping its energy with it.
            let dropped = self.hist[pos];
            self.hist[pos] = x;
            self.energy += x.norm_sqr() - dropped.norm_sqr();
            if self.energy < 0.0 {
                // Rounding, over millions of samples of adds and subtracts.
                self.energy = self.hist.iter().map(|h| h.norm_sqr()).sum();
            }
            let newest = pos;
            pos = if pos + 1 == taps { 0 } else { pos + 1 };

            let mut y = Complex32::new(0.0, 0.0);
            for k in 0..taps {
                let idx = (newest + taps - k) % taps;
                y += self.w[k] * self.hist[idx];
            }
            let e = m - y;

            // Normalised by the *smoothed* input power rather than this
            // sample's, and that is not a refinement — it is what makes the
            // filter converge on the right answer. Dividing by the
            // instantaneous energy, as textbook NLMS does, weights every
            // sample by `1/|aux|²`, so the quiet samples dominate the average
            // and the filter settles somewhere other than the least-squares
            // fit; on two aerials hearing one station that showed up as a
            // combining weight half again too big. The `max` keeps the step
            // no larger than plain NLMS would have taken, so a burst cannot
            // make it unstable.
            self.pow += (self.energy - self.pow) * POWER_TAU_INV;
            if !self.frozen {
                let step = self.mu / (self.pow.max(self.energy) + EPS);
                for k in 0..taps {
                    let idx = (newest + taps - k) % taps;
                    self.w[k] += e * self.hist[idx].conj() * step;
                }
            }

            main[i] = match self.mode {
                DiversityMode::Cancel => e,
                // Maximal ratio: each branch weighted by the inverse of its
                // noise power, with `W` carrying the scale between them.
                DiversityMode::Combine => (m * wnorm + y) / (1.0 + wnorm),
            };
            in_acc += m.norm_sqr();
            out_acc += main[i].norm_sqr();
        }

        self.pos = pos;
        let inv = 1.0 / n as f32;
        self.in_pow = self.in_pow * POWER_DECAY + in_acc * inv * (1.0 - POWER_DECAY);
        self.out_pow = self.out_pow * POWER_DECAY + out_acc * inv * (1.0 - POWER_DECAY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic complex noise source, so a failure is reproducible.
    fn noise(n: usize, seed: u64) -> Vec<Complex32> {
        let mut s = seed | 1;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            ((s >> 40) as f32 / 8_388_608.0) - 1.0
        };
        (0..n).map(|_| Complex32::new(next(), next())).collect()
    }

    fn power(x: &[Complex32]) -> f32 {
        x.iter().map(|s| s.norm_sqr()).sum::<f32>() / x.len() as f32
    }

    /// The case the mode exists for: one noise source reaching two aerials
    /// through different paths — a complex gain *and* a delay, which is what a
    /// single-weight canceller cannot follow across a span.
    ///
    /// Measured on the tail of the run, after convergence, and on the
    /// interference alone: what matters is how much of the noise is left, not
    /// how much total power went away.
    #[test]
    fn a_delayed_and_rotated_noise_source_is_nulled() {
        let n = 200_000;
        let qrm = noise(n, 0x1234);
        // The path from the noise to the main aerial: 3 samples further and
        // 0.6 of the amplitude, rotated 40°.
        let h = Complex32::from_polar(0.6, 0.7);
        let delay = 3usize;
        let mut main: Vec<Complex32> = (0..n)
            .map(|i| if i >= delay { qrm[i - delay] * h } else { Complex32::new(0.0, 0.0) })
            .collect();

        let mut d = Diversity::new(DiversityMode::Cancel, 8, 0.8);
        d.process(&mut main, &qrm);

        let tail = &main[n - 20_000..];
        let before: Vec<Complex32> = (n - 20_000..n).map(|i| qrm[i - delay] * h).collect();
        let depth = 10.0 * (power(&before) / power(tail)).log10();
        assert!(depth > 30.0, "only {depth:.1} dB of the noise source was removed");
    }

    /// And the wanted signal survives it, as long as the auxiliary aerial
    /// cannot hear it — which is the whole instruction for using this mode.
    #[test]
    fn a_signal_only_the_main_aerial_hears_survives_the_null() {
        let n = 200_000;
        let qrm = noise(n, 0xbeef);
        let h = Complex32::from_polar(0.8, -1.2);
        // A tone at a twelfth of the sample rate, on the main aerial only.
        let want: Vec<Complex32> = (0..n)
            .map(|i| Complex32::from_polar(0.3, std::f32::consts::TAU * i as f32 / 12.0))
            .collect();
        let mut main: Vec<Complex32> = (0..n).map(|i| want[i] + qrm[i] * h).collect();

        let mut d = Diversity::new(DiversityMode::Cancel, 8, 0.8);
        d.process(&mut main, &qrm);

        // Correlate the tail against the tone: how much of it came through.
        let tail = n - 20_000..n;
        let corr: Complex32 = tail
            .clone()
            .map(|i| main[i] * want[i].conj())
            .fold(Complex32::new(0.0, 0.0), |a, b| a + b);
        let kept = corr.norm() / (tail.len() as f32 * 0.3 * 0.3);
        assert!(kept > 0.9, "the wanted tone came through at {kept:.3} of its amplitude");
    }

    /// Two aerials on the same signal with independent noise: the textbook
    /// 3 dB, which is what says the branch weighting is right and not merely
    /// in phase.
    #[test]
    fn two_equal_branches_combine_for_about_three_decibels() {
        let n = 400_000;
        let sig = noise(n, 0xa11ce);
        let n1 = noise(n, 0x1111);
        let n2 = noise(n, 0x2222);
        // Equal amplitude, different phase — two aerials at different distances
        // from the same station.
        let h1 = Complex32::from_polar(1.0, 0.3);
        let h2 = Complex32::from_polar(1.0, -2.1);
        // About 10 dB per branch: enough that the filter's estimate of the
        // ratio between them is the ratio and not half of it. The formula's
        // 3 dB is the high-SNR limit, and a branch at 0 dB is a branch whose
        // ratio cannot be measured to better than a factor of two.
        let noise_amp = 0.316f32;
        let mut main: Vec<Complex32> = (0..n).map(|i| sig[i] * h1 + n1[i] * noise_amp).collect();
        let aux: Vec<Complex32> = (0..n).map(|i| sig[i] * h2 + n2[i] * noise_amp).collect();

        let mut d = Diversity::new(DiversityMode::Combine, 1, 0.6);
        d.process(&mut main, &aux);

        // Measure on the tail: project the output onto the signal, and call
        // what is left the noise.
        let tail = n - 100_000..n;
        let refs: Vec<Complex32> = tail.clone().map(|i| sig[i] * h1).collect();
        let out: Vec<Complex32> = tail.map(|i| main[i]).collect();
        let g: Complex32 = out
            .iter()
            .zip(&refs)
            .map(|(o, r)| o * r.conj())
            .fold(Complex32::new(0.0, 0.0), |a, b| a + b)
            / refs.iter().map(|r| r.norm_sqr()).sum::<f32>();
        let resid: Vec<Complex32> = out.iter().zip(&refs).map(|(o, r)| o - r * g).collect();
        let snr_out = 10.0 * ((g.norm_sqr() * power(&refs)) / power(&resid)).log10();
        // The same measurement on the main branch alone: its residual after
        // the signal is projected out is exactly its own noise.
        let own: Vec<Complex32> = (n - 100_000..n).map(|i| n1[i] * noise_amp).collect();
        let snr_in = 10.0 * (power(&refs) / power(&own)).log10();
        let gain = snr_out - snr_in;
        assert!(
            (2.5..3.5).contains(&gain),
            "combining two equal branches gained {gain:.2} dB, not about 3"
        );
    }

    /// Freezing holds the filter where it is, which is what makes a null found
    /// on a quiet band survive a loud one.
    #[test]
    fn freezing_stops_the_filter_moving() {
        let n = 40_000;
        let qrm = noise(n, 7);
        let mut main: Vec<Complex32> = qrm.iter().map(|q| q * 0.5).collect();
        let mut d = Diversity::new(DiversityMode::Cancel, 4, 0.9);
        d.process(&mut main, &qrm);
        let held: Vec<Complex32> = d.w.clone();

        d.set_frozen(true);
        let other = noise(n, 99);
        let mut main2: Vec<Complex32> = other.iter().map(|q| q * 3.0).collect();
        d.process(&mut main2, &other);
        assert_eq!(d.w, held, "the filter moved while frozen");
    }

    /// [`Diversity::reset`] zeroes the filter but, deliberately, never
    /// touches [`Diversity::frozen`] — its own doc comment says it is meant
    /// to be usable "under a frozen null". Left frozen, that leaves the
    /// filter permanently parked at zero: `Cancel` becomes indistinguishable
    /// from `Combine`, since nothing after `reset` ever adapts again. This
    /// is what a real Fobos capture reproduced (a 33 dB null vanishing the
    /// instant Restart was pressed with HOLD selected, silently, with
    /// nothing on screen to say why) — a caller reaching for `reset` while
    /// frozen, meaning to "find the null again" per the RESTART control's
    /// own tooltip, must also call `set_frozen(false)` itself, which is
    /// exactly the two-call sequence `src/fobos_source.rs`'s
    /// `DIV_RESET_ELEMENT` handler now performs.
    #[test]
    fn reset_alone_leaves_a_frozen_filter_permanently_zeroed() {
        let n = 40_000;
        let qrm = noise(n, 7);
        let mut main: Vec<Complex32> = qrm.iter().map(|q| q * 0.5).collect();
        let mut d = Diversity::new(DiversityMode::Cancel, 4, 0.9);
        d.process(&mut main, &qrm);
        assert_ne!(d.w, vec![Complex32::new(0.0, 0.0); 4], "should have converged to something");

        d.set_frozen(true);
        d.reset();
        assert_eq!(d.w, vec![Complex32::new(0.0, 0.0); 4], "reset should zero the taps");
        assert!(d.frozen(), "reset alone must not clear frozen — that's the bug");

        // Frozen and zeroed: further samples change nothing, exactly the
        // silent no-cancellation state the bug report captured.
        let mut main2: Vec<Complex32> = qrm.iter().map(|q| q * 0.5).collect();
        d.process(&mut main2, &qrm);
        assert_eq!(d.w, vec![Complex32::new(0.0, 0.0); 4], "still zeroed while frozen");

        // The fix: also clearing `frozen` after `reset` lets it actually
        // "find the null again", the way RESTART's own tooltip promises.
        d.set_frozen(false);
        let mut main3: Vec<Complex32> = qrm.iter().map(|q| q * 0.5).collect();
        d.process(&mut main3, &qrm);
        assert_ne!(
            d.w,
            vec![Complex32::new(0.0, 0.0); 4],
            "clearing frozen after reset should let the filter adapt again"
        );
    }

    /// The rate control spans the documented range and is monotonic, because a
    /// slider that goes the wrong way at one end is worse than no slider.
    #[test]
    fn the_rate_control_spans_its_range_in_order() {
        assert!((Diversity::mu_for_rate(0.0) - MU_MIN).abs() < 1e-9);
        assert!((Diversity::mu_for_rate(1.0) - MU_MAX).abs() < 1e-6);
        let mut last = 0.0;
        for i in 0..=10 {
            let mu = Diversity::mu_for_rate(i as f32 / 10.0);
            assert!(mu > last, "rate {i} went backwards");
            last = mu;
        }
        // Out of range is clamped rather than extrapolated.
        assert_eq!(Diversity::mu_for_rate(-1.0), Diversity::mu_for_rate(0.0));
        assert_eq!(Diversity::mu_for_rate(9.0), Diversity::mu_for_rate(1.0));
    }

    /// A stalled auxiliary chain hands over fewer samples than the main one;
    /// the rest of the block must come through untouched rather than be
    /// cancelled against silence.
    #[test]
    fn a_short_auxiliary_block_passes_the_remainder_through() {
        let mut main: Vec<Complex32> = (0..16).map(|i| Complex32::new(i as f32, 0.0)).collect();
        let aux = vec![Complex32::new(1.0, 0.0); 4];
        let mut d = Diversity::new(DiversityMode::Cancel, 2, 0.5);
        d.process(&mut main, &aux);
        for i in 4..16 {
            assert_eq!(main[i], Complex32::new(i as f32, 0.0), "sample {i} was touched");
        }
    }
}
