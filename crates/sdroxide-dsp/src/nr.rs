//! Spectral audio noise reduction — pulls voice out of static and white noise.
//!
//! A streaming short-time Fourier transform (Hann window, 75 % overlap, weighted
//! overlap-add) applies a per-bin suppression gain. Stationary noise (static,
//! hiss) sits at a slowly-varying floor while speech is transient and rises above
//! it, so a per-bin noise floor is tracked with a minimum-follower and each bin's
//! gain follows its a-priori SNR (decision-directed, Ephraim-Malah–style Wiener),
//! which keeps musical-noise artefacts low. The intensity (Low/Med/High) sets a
//! noise over-estimation factor and a minimum gain floor.
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

    // Per-bin frequency-domain state (bins 0..=N/2).
    frame: Vec<Complex32>,
    noise: Vec<f32>,      // tracked noise magnitude floor
    smag: Vec<f32>,       // smoothed magnitude (for the min-follower)
    prev_clean: Vec<f32>, // previous frame clean power (a-priori SNR)
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
            noise: vec![0.0; N / 2 + 1],
            smag: vec![0.0; N / 2 + 1],
            prev_clean: vec![0.0; N / 2 + 1],
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
        self.noise.iter_mut().for_each(|x| *x = 0.0);
        self.smag.iter_mut().for_each(|x| *x = 0.0);
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
            let mag = self.frame[k].norm();
            // Smoothed magnitude, then a minimum-follower for the noise floor:
            // dip down fast to whatever floor a voice pause reveals, creep up
            // very slowly so a rising noise floor is eventually re-learned.
            let sm = if self.learned { 0.7 * self.smag[k] + 0.3 * mag } else { mag };
            self.smag[k] = sm;
            if !self.learned || sm < self.noise[k] {
                self.noise[k] = sm;
            } else {
                self.noise[k] += (sm - self.noise[k]) * 0.0008;
            }
            let np = (self.noise[k] * self.noise[k] * self.over).max(1e-12);
            let post = (mag * mag) / np; // a-posteriori SNR
            // Decision-directed a-priori SNR → Wiener gain; floored to keep some
            // ambience and avoid musical noise.
            let prio = 0.98 * (self.prev_clean[k] / np) + 0.02 * (post - 1.0).max(0.0);
            let gain = (prio / (1.0 + prio)).clamp(self.floor, 1.0);
            let clean = gain * mag;
            self.prev_clean[k] = clean * clean;
            self.frame[k] *= gain;
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
        nr.set_params(2.6, 0.06); // High
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
        nr.set_params(1.7, 0.15); // Medium
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
}
