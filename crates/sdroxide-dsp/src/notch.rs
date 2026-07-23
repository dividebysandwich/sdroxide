//! Adaptive auto-notch (ANC) — cancels constant tone elements (heterodynes,
//! carriers, tuner-uppers) while leaving voice and noise.
//!
//! An LMS adaptive line-canceller: a short FIR predicts the current sample from
//! a *delayed* copy of the input. Anything periodic and persistent (a steady
//! tone) is predictable and gets cancelled; broadband, non-stationary content
//! (speech, noise) is not predictable across the decorrelation delay and passes
//! through. The output is the prediction **error** — i.e. the input with its
//! predictable tones removed. Normalized-LMS keeps it stable across levels.
//!
//! Pure Rust, wasm-clean, in-place.

/// Adaptive FIR length. Longer notches more/narrower tones.
const TAPS: usize = 64;
/// Decorrelation delay (samples): long enough that broadband audio is
/// uncorrelated across it, so only persistent tones are predicted.
const DELAY: usize = 24;

pub struct AutoNotch {
    w: Vec<f32>,     // adaptive taps (predict the tonal component)
    hist: Vec<f32>,  // input history ring (DELAY + TAPS long)
    pos: usize,
    mu: f32,         // NLMS step size
    leak: f32,       // tap leakage (keeps weights from drifting)
    power: f32,      // running input power for NLMS normalization
}

impl AutoNotch {
    pub fn new() -> Self {
        AutoNotch {
            w: vec![0.0; TAPS],
            hist: vec![0.0; DELAY + TAPS],
            pos: 0,
            mu: 0.08,
            leak: 1e-5,
            power: 1e-3,
        }
    }

    /// Clear the adaptive state (call when switching the notch on).
    pub fn reset(&mut self) {
        self.w.iter_mut().for_each(|x| *x = 0.0);
        self.hist.iter_mut().for_each(|x| *x = 0.0);
        self.pos = 0;
        self.power = 1e-3;
    }

    /// Remove predictable tones from `audio` in place.
    pub fn process(&mut self, audio: &mut [f32]) {
        let len = self.hist.len();
        for s in audio.iter_mut() {
            let x = *s;
            self.hist[self.pos] = x;
            // Predict the current sample from the delayed reference window
            // (x[n-DELAY .. n-DELAY-TAPS+1]) — the periodic/tonal component.
            let mut y = 0.0;
            for k in 0..TAPS {
                let idx = (self.pos + len - DELAY - k) % len;
                y += self.w[k] * self.hist[idx];
            }
            let e = x - y; // residual: tones removed, voice + noise kept
            // Normalized-LMS tap update, with leakage.
            self.power = 0.999 * self.power + 0.001 * x * x;
            let norm = self.mu / (1e-6 + TAPS as f32 * self.power);
            for k in 0..TAPS {
                let idx = (self.pos + len - DELAY - k) % len;
                self.w[k] = self.w[k] * (1.0 - self.leak) + norm * e * self.hist[idx];
            }
            self.pos = (self.pos + 1) % len;
            *s = e;
        }
    }
}

impl Default for AutoNotch {
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
    fn cancels_steady_tone() {
        let mut nf = AutoNotch::new();
        let rate = 12_000.0;
        let f = 1500.0;
        let mut buf: Vec<f32> = (0..48_000)
            .map(|i| 0.3 * (std::f32::consts::TAU * f * i as f32 / rate).sin())
            .collect();
        let before = rms(&buf);
        nf.process(&mut buf);
        // After the filter converges, the steady tone should be strongly gone.
        let after = rms(&buf[24_000..]);
        assert!(after < 0.2 * before, "tone not notched: {before} -> {after}");
    }

    #[test]
    fn passes_broadband_noise() {
        let mut nf = AutoNotch::new();
        let mut buf = noise(48_000, 0.2, 0xBEEF);
        let before = rms(&buf);
        nf.process(&mut buf);
        // Unpredictable noise is left essentially untouched.
        let after = rms(&buf[24_000..]);
        assert!(after > 0.8 * before, "noise over-suppressed: {before} -> {after}");
    }
}
