//! Spectral audio noise reduction — pulls voice out of static and white noise
//! with minimal "musical noise" (the random chirping/birdie artefacts that
//! naive spectral subtraction leaves behind).
//!
//! A streaming short-time Fourier transform (Hann window, 75 % overlap, weighted
//! overlap-add) applies a per-bin suppression gain. The gain rule is the
//! established low-artefact combination used by OM-LSA-class enhancers:
//!
//!  * **Noise estimate — MCRA** (minima-controlled recursive averaging, Cohen
//!    2002): the per-bin noise *power* is a recursive average taken only while
//!    speech is absent, detected by comparing the smoothed power to its tracked
//!    minimum (Doblinger continuous minimum tracking). Unlike a bare
//!    minimum-follower this estimates the noise *mean* rather than its minimum,
//!    so it doesn't under-estimate the floor — the primary cause of musical noise.
//!  * **A-priori SNR — decision-directed** (Ephraim & Malah 1984): ξ is smoothed
//!    across frames, which is what actually suppresses musical noise, and is
//!    floored (Cappé 1994) so the gain can't collapse to zero and pop back.
//!  * **Gain — log-MMSE** (Ephraim & Malah 1985): the log-spectral-amplitude
//!    estimator, renowned for far less musical noise than a plain Wiener gain.
//!  * A light **SNR-weighted frequency smoothing** of the gain removes the last
//!    isolated single-bin spikes in noise-dominated regions while leaving
//!    speech-dominated bins (which span several bins) untouched.
//!
//! The intensity (Low/Med/High) sets a noise over-estimation factor and a minimum
//! gain floor.
//!
//! Pure Rust (rustfft), wasm-clean. In-place, same length in and out, with a
//! fixed one-frame latency.

use std::collections::VecDeque;
use std::sync::Arc;

use rustfft::{Fft, FftPlanner};

use crate::Complex32;

/// FFT size (samples). Frame length; independent of the audio sample rate.
const N: usize = 512;
/// Hop between frames — 75 % overlap gives smooth reconstruction and low
/// musical noise.
const HOP: usize = 128;

/// Decision-directed a-priori-SNR smoothing (Ephraim-Malah). High → strong
/// cross-frame smoothing, the main musical-noise suppressor.
const ALPHA_DD: f32 = 0.98;
/// A-priori-SNR floor (~ -30 dB). Keeps the gain from collapsing to zero and
/// popping back up (Cappé 1994) without audibly changing the noise floor.
const XI_MIN: f32 = 1e-3;
/// Smoothing of the per-bin power used for noise tracking.
const ALPHA_S: f32 = 0.8;
/// Doblinger continuous minimum tracking of the smoothed power.
const GAMMA_MIN: f32 = 0.998;
const BETA_MIN: f32 = 0.96;
/// smoothed-power / tracked-minimum above this ⇒ speech present in the bin.
const DELTA_SPP: f32 = 5.0;
/// Speech-presence-probability smoothing.
const ALPHA_P: f32 = 0.2;
/// Base noise recursive-averaging factor (used where speech is absent).
const ALPHA_N: f32 = 0.85;
/// Clamp on the a-posteriori SNR for numerical safety.
const GAMMA_MAX: f32 = 1000.0;

/// Exponential integral `E1(x) = ∫_x^∞ e^-t / t dt`, `x > 0`.
/// Abramowitz & Stegun 5.1.53 (x<1) / 5.1.56 (x≥1); error < 3e-6 over the range
/// that matters here. Used by the log-MMSE gain.
fn exp_int_e1(x: f32) -> f32 {
    let x = x.max(1e-8); // ν→0 only for empty bins (output is 0 either way)
    if x < 1.0 {
        -x.ln() - 0.577_215_66
            + x * (0.999_991_93
                + x * (-0.249_910_55
                    + x * (0.055_199_68 + x * (-0.009_760_04 + x * 0.001_078_57))))
    } else {
        let num = x * x + 2.334_733 * x + 0.250_621;
        let den = x * x + 3.330_657 * x + 1.681_534;
        (-x).exp() / x * (num / den)
    }
}

pub struct SpectralNr {
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex32>,
    window: Vec<f32>,

    // Sliding input ring: the most recent N samples, in write order.
    in_ring: Vec<f32>,
    write: usize,
    filled: usize,    // grows to N while priming
    since_hop: usize, // samples toward the next frame

    // Weighted overlap-add accumulators: signal and window energy.
    ola_sig: Vec<f32>,
    ola_win: Vec<f32>,
    out: VecDeque<f32>, // finalized output samples awaiting emit

    // Frequency-domain frame (full N, conjugate-symmetric).
    frame: Vec<Complex32>,

    // Per-bin scratch and state (bins 0..=N/2).
    mag: Vec<f32>,        // current-frame magnitude (scratch)
    gain: Vec<f32>,       // per-bin gain before frequency smoothing (scratch)
    weight: Vec<f32>,     // speech weight ξ/(1+ξ), for smoothing blend (scratch)
    noise_pow: Vec<f32>,  // MCRA noise power estimate
    s_smooth: Vec<f32>,   // smoothed power S
    s_min: Vec<f32>,      // tracked minimum of S (Doblinger)
    s_prev: Vec<f32>,     // previous S (Doblinger)
    p_present: Vec<f32>,  // speech-presence probability
    prev_clean: Vec<f32>, // previous frame clean power (decision-directed)
    learned: bool,

    // Tuning from the NR level.
    over: f32,
    floor: f32,
}

impl SpectralNr {
    pub fn new() -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(N);
        let ifft = planner.plan_fft_inverse(N);
        let scratch_len =
            fft.get_inplace_scratch_len().max(ifft.get_inplace_scratch_len());
        // Periodic Hann window.
        let window: Vec<f32> = (0..N)
            .map(|n| 0.5 - 0.5 * (std::f32::consts::TAU * n as f32 / N as f32).cos())
            .collect();
        let half = N / 2 + 1;
        SpectralNr {
            fft,
            ifft,
            scratch: vec![Complex32::default(); scratch_len],
            window,
            in_ring: vec![0.0; N],
            write: 0,
            filled: 0,
            since_hop: 0,
            ola_sig: vec![0.0; N],
            ola_win: vec![0.0; N],
            out: VecDeque::with_capacity(N),
            frame: vec![Complex32::default(); N],
            mag: vec![0.0; half],
            gain: vec![0.0; half],
            weight: vec![0.0; half],
            noise_pow: vec![0.0; half],
            s_smooth: vec![0.0; half],
            s_min: vec![0.0; half],
            s_prev: vec![0.0; half],
            p_present: vec![0.0; half],
            prev_clean: vec![0.0; half],
            learned: false,
            over: 1.0,
            floor: 1.0,
        }
    }

    /// Set the intensity: `over` = noise over-estimation factor (more removes
    /// more), `floor` = minimum per-bin gain (lower is more aggressive).
    pub fn set_params(&mut self, over: f32, floor: f32) {
        self.over = over;
        self.floor = floor;
    }

    /// Clear all internal state (call when turning NR on so a stale ring doesn't
    /// glitch, and to re-learn the noise floor).
    pub fn reset(&mut self) {
        self.write = 0;
        self.filled = 0;
        self.since_hop = 0;
        self.in_ring.iter_mut().for_each(|x| *x = 0.0);
        self.ola_sig.iter_mut().for_each(|x| *x = 0.0);
        self.ola_win.iter_mut().for_each(|x| *x = 0.0);
        self.out.clear();
        self.noise_pow.iter_mut().for_each(|x| *x = 0.0);
        self.s_smooth.iter_mut().for_each(|x| *x = 0.0);
        self.s_min.iter_mut().for_each(|x| *x = 0.0);
        self.s_prev.iter_mut().for_each(|x| *x = 0.0);
        self.p_present.iter_mut().for_each(|x| *x = 0.0);
        self.prev_clean.iter_mut().for_each(|x| *x = 0.0);
        self.learned = false;
    }

    /// Reduce noise in `audio` in place. Same length in and out; introduces a
    /// fixed latency of one FFT frame (filled with reconstructed signal).
    pub fn process(&mut self, audio: &mut [f32]) {
        for s in audio.iter_mut() {
            self.in_ring[self.write] = *s;
            self.write = (self.write + 1) % N;
            if self.filled < N {
                self.filled += 1;
            }
            self.since_hop += 1;
            if self.filled >= N && self.since_hop >= HOP {
                self.since_hop = 0;
                self.run_frame();
            }
            *s = self.out.pop_front().unwrap_or(0.0);
        }
    }

    fn run_frame(&mut self) {
        // Gather the last N samples in time order (write points at the oldest),
        // apply the analysis window.
        for i in 0..N {
            let x = self.in_ring[(self.write + i) % N];
            self.frame[i] = Complex32::new(x * self.window[i], 0.0);
        }
        self.fft.process_with_scratch(&mut self.frame, &mut self.scratch);

        let half = N / 2;
        for k in 0..=half {
            self.mag[k] = self.frame[k].norm();
        }

        // --- Noise estimate (MCRA) ---------------------------------------
        for k in 0..=half {
            let power = self.mag[k] * self.mag[k];
            // Frequency-smoothed instantaneous power (reduces the variance that
            // would otherwise leak into the speech-presence decision).
            let pf = if k == 0 {
                0.5 * (self.mag[0].powi(2) + self.mag[1].powi(2))
            } else if k == half {
                0.5 * (self.mag[half].powi(2) + self.mag[half - 1].powi(2))
            } else {
                0.25 * self.mag[k - 1].powi(2)
                    + 0.5 * power
                    + 0.25 * self.mag[k + 1].powi(2)
            };

            if !self.learned {
                self.s_smooth[k] = pf;
                self.s_min[k] = pf;
                self.s_prev[k] = pf;
                self.noise_pow[k] = pf;
                self.p_present[k] = 0.0;
                continue;
            }

            self.s_smooth[k] = ALPHA_S * self.s_smooth[k] + (1.0 - ALPHA_S) * pf;
            // Doblinger continuous minimum tracking: dip to the smoothed power
            // instantly, creep back up slowly so a rising floor is re-learned.
            if self.s_min[k] < self.s_smooth[k] {
                self.s_min[k] = GAMMA_MIN * self.s_min[k]
                    + ((1.0 - GAMMA_MIN) / (1.0 - BETA_MIN))
                        * (self.s_smooth[k] - BETA_MIN * self.s_prev[k]);
            } else {
                self.s_min[k] = self.s_smooth[k];
            }
            self.s_prev[k] = self.s_smooth[k];

            // Speech present when the smoothed power rides well above its
            // minimum; update a smoothed presence probability, then average the
            // noise power only in proportion to speech *absence*.
            let sr = self.s_smooth[k] / self.s_min[k].max(1e-12);
            let ind = if sr > DELTA_SPP { 1.0 } else { 0.0 };
            self.p_present[k] = ALPHA_P * self.p_present[k] + (1.0 - ALPHA_P) * ind;
            let a = ALPHA_N + (1.0 - ALPHA_N) * self.p_present[k];
            self.noise_pow[k] = a * self.noise_pow[k] + (1.0 - a) * power;
        }

        // --- Gains (decision-directed a-priori SNR → log-MMSE) -----------
        for k in 0..=half {
            let np = (self.noise_pow[k] * self.over).max(1e-12);
            let power = self.mag[k] * self.mag[k];
            let gamma = (power / np).min(GAMMA_MAX); // a-posteriori SNR
            let xi = (ALPHA_DD * (self.prev_clean[k] / np)
                + (1.0 - ALPHA_DD) * (gamma - 1.0).max(0.0))
            .max(XI_MIN); // a-priori SNR (decision-directed, floored)
            let ratio = xi / (1.0 + xi);
            let nu = ratio * gamma;
            // Log-spectral-amplitude (log-MMSE) gain.
            let g = (ratio * (0.5 * exp_int_e1(nu)).exp()).clamp(self.floor, 1.0);
            self.gain[k] = g;
            self.weight[k] = ratio; // ≈1 where speech dominates, ≈0 in noise
        }

        // --- Apply: SNR-weighted frequency smoothing + reconstruct -------
        // In noise-dominated bins blend toward a [0.25,0.5,0.25]-smoothed gain
        // (kills isolated single-bin chirps); in speech bins keep the raw gain
        // (preserves formant detail, which spans several bins).
        for k in 0..=half {
            let gs = if k == 0 || k == half {
                self.gain[k]
            } else {
                0.25 * self.gain[k - 1] + 0.5 * self.gain[k] + 0.25 * self.gain[k + 1]
            };
            let w = self.weight[k];
            let g = w * self.gain[k] + (1.0 - w) * gs;
            let clean = g * self.mag[k];
            self.prev_clean[k] = clean * clean;
            self.frame[k] *= g;
            // Mirror the (real) gain onto the conjugate-symmetric upper half so
            // the inverse transform stays real.
            if k > 0 && k < half {
                self.frame[N - k] = self.frame[k].conj();
            }
        }
        self.learned = true;

        self.ifft.process_with_scratch(&mut self.frame, &mut self.scratch);

        // Weighted overlap-add: synthesis window + running window-energy so the
        // division normalizes exactly regardless of edge effects.
        let inv = 1.0 / N as f32;
        for i in 0..N {
            let y = self.frame[i].re * inv;
            self.ola_sig[i] += self.window[i] * y;
            self.ola_win[i] += self.window[i] * self.window[i];
        }
        // The oldest HOP samples are complete now → emit them and shift.
        for i in 0..HOP {
            let d = self.ola_win[i];
            self.out.push_back(if d > 1e-6 { self.ola_sig[i] / d } else { 0.0 });
        }
        self.ola_sig.copy_within(HOP.., 0);
        self.ola_win.copy_within(HOP.., 0);
        for i in (N - HOP)..N {
            self.ola_sig[i] = 0.0;
            self.ola_win[i] = 0.0;
        }
    }
}

impl Default for SpectralNr {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|&s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    // Deterministic pseudo-random noise.
    fn noise(n: usize, amp: f32, seed: u64) -> Vec<f32> {
        let mut r = seed;
        (0..n)
            .map(|_| {
                r ^= r << 13;
                r ^= r >> 7;
                r ^= r << 17;
                (((r >> 40) as i32) as f32 / (1 << 23) as f32 - 1.0) * amp
            })
            .collect()
    }

    #[test]
    fn reduces_broadband_noise() {
        let mut nr = SpectralNr::new();
        nr.set_params(2.0, 0.07); // High
        let mut buf = noise(60_000, 0.2, 0x1234);
        let before = rms(&buf);
        nr.process(&mut buf);
        // Skip the priming latency at the start.
        let after = rms(&buf[N * 4..]);
        assert!(after < 0.6 * before, "noise not reduced: {before} -> {after}");
    }

    #[test]
    fn preserves_modulated_signal() {
        let mut nr = SpectralNr::new();
        nr.set_params(1.4, 0.14); // Medium
        // A non-stationary signal (on/off tone bursts, like speech syllables or
        // keyed CW) should largely survive: the gaps reveal the noise floor, so
        // the bursts read as high-SNR and pass through.
        let rate = 12_000.0;
        let f = 1000.0;
        let mut buf: Vec<f32> = (0..96_000)
            .map(|i| {
                let on = (i / 4000) % 2 == 0; // 4000 on, 4000 off
                if on { 0.3 * (std::f32::consts::TAU * f * i as f32 / rate).sin() } else { 0.0 }
            })
            .collect();
        let before = rms(&buf);
        nr.process(&mut buf);
        let after = rms(&buf[N * 4..]);
        assert!(after > 0.7 * before, "signal over-suppressed: {before} -> {after}");
    }

    #[test]
    fn low_musical_noise_on_stationary_input() {
        // Musical noise manifests as a bursty, fluctuating residual: random
        // tonal peaks that flit between frames. On a stationary input a good
        // suppressor leaves a smooth residual, so the per-hop envelope stays
        // steady — a low temporal coefficient of variation. (Naive spectral
        // subtraction lands well above this bound.)
        let mut nr = SpectralNr::new();
        nr.set_params(2.0, 0.07); // High
        let mut buf = noise(240_000, 0.2, 0xBEEF);
        nr.process(&mut buf);
        let tail = &buf[N * 16..];
        let env: Vec<f32> = tail.chunks(HOP).map(rms).collect();
        let mean = env.iter().sum::<f32>() / env.len() as f32;
        let var = env.iter().map(|&e| (e - mean).powi(2)).sum::<f32>() / env.len() as f32;
        let cv = var.sqrt() / mean.max(1e-9);
        assert!(cv < 0.5, "residual too bursty (musical noise): cv={cv}");
    }
}
