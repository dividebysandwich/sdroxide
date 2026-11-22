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

/// The rate the constants below were designed and tested at. Every length is
/// kept the same in *time* at any other rate: at 48 kHz, where both the SDR
/// chain and a rig's own audio run, 64 taps span only 1.3 ms, which makes each
/// notch ~750 Hz wide, and a 24-sample delay is short enough that band-limited
/// speech is predictable across it. The filter then cancels the voice itself,
/// which is what made ANC "very distorted" (#901).
const DESIGN_RATE: f64 = 12_000.0;
/// Adaptive FIR length at [`DESIGN_RATE`]. Longer notches more/narrower tones.
const TAPS: usize = 64;
/// Decorrelation delay (samples) at [`DESIGN_RATE`]: long enough that
/// broadband audio is uncorrelated across it, so only persistent tones are
/// predicted.
const DELAY: usize = 24;

/// How long the filter takes to learn a tone, in seconds. See `mu` in
/// [`AutoNotch::new`].
const ADAPT_S: f64 = 0.5;

pub struct AutoNotch {
    rate: f64,
    taps: usize,
    delay: usize,
    w: Vec<f32>,    // adaptive taps (predict the tonal component)
    hist: Vec<f32>, // input history ring (delay + taps long)
    pos: usize,
    mu: f32,   // NLMS step size
    leak: f32, // tap leakage (keeps weights from drifting)
}

impl AutoNotch {
    /// A notch for audio at `rate` samples per second.
    pub fn new(rate: f64) -> Self {
        let scale = (rate / DESIGN_RATE).max(0.25);
        let taps = ((TAPS as f64 * scale).round() as usize).max(16);
        let delay = ((DELAY as f64 * scale).round() as usize).max(4);
        AutoNotch {
            rate,
            taps,
            delay,
            w: vec![0.0; taps],
            hist: vec![0.0; delay + taps],
            pos: 0,
            // Normalised by the whole window's energy, NLMS converges in about
            // taps/mu samples. That is set to ADAPT_S whatever the rate: slow
            // enough that a syllable is gone before the filter has learned its
            // spectrum, so only what holds still for longer (a heterodyne, a
            // carrier) is cancelled. The old fixed 0.08 adapted in ~67 ms at
            // 12 kHz, which is fast enough to eat speech.
            mu: (taps as f64 / (ADAPT_S * rate)).min(0.1) as f32,
            leak: (1e-5 / scale) as f32,
        }
    }

    /// Rebuild for another sample rate; a no-op when it has not changed.
    pub fn set_rate(&mut self, rate: f64) {
        if (rate - self.rate).abs() > 0.5 {
            *self = AutoNotch::new(rate);
        }
    }

    /// Clear the adaptive state (call when switching the notch on).
    pub fn reset(&mut self) {
        self.w.iter_mut().for_each(|x| *x = 0.0);
        self.hist.iter_mut().for_each(|x| *x = 0.0);
        self.pos = 0;
    }

    /// Remove predictable tones from `audio` in place.
    pub fn process(&mut self, audio: &mut [f32]) {
        let len = self.hist.len();
        let (taps, delay) = (self.taps, self.delay);
        for s in audio.iter_mut() {
            let x = *s;
            self.hist[self.pos] = x;
            // Predict the current sample from the delayed reference window
            // (x[n-delay .. n-delay-taps+1]) — the periodic/tonal component.
            // The window's energy comes out of the same pass: normalising by
            // it, not by a slow average of the input, is what keeps the step
            // stable at a syllable's onset after a quiet gap.
            let mut y = 0.0;
            let mut energy = 0.0;
            for k in 0..taps {
                let u = self.hist[(self.pos + len - delay - k) % len];
                y += self.w[k] * u;
                energy += u * u;
            }
            let e = x - y; // residual: tones removed, voice + noise kept
            // Normalized-LMS tap update, with leakage.
            let norm = self.mu / (1e-6 + energy);
            for k in 0..taps {
                let idx = (self.pos + len - delay - k) % len;
                self.w[k] = self.w[k] * (1.0 - self.leak) + norm * e * self.hist[idx];
            }
            self.pos = (self.pos + 1) % len;
            *s = e;
        }
    }
}

impl Default for AutoNotch {
    /// 48 kHz, the rate both the receive chain and a rig's audio run at.
    fn default() -> Self {
        Self::new(48_000.0)
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
        let rate = 12_000.0f32;
        let mut nf = AutoNotch::new(f64::from(rate));
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
        let mut nf = AutoNotch::new(12_000.0);
        let mut buf = noise(48_000, 0.2, 0xBEEF);
        let before = rms(&buf);
        nf.process(&mut buf);
        // Unpredictable noise is left essentially untouched.
        let after = rms(&buf[24_000..]);
        assert!(after > 0.8 * before, "noise over-suppressed: {before} -> {after}");
    }

    /// Voice-band noise (300–2700 Hz), syllabic on/off: what SSB speech
    /// looks like to a predictor. Band-limited, so strongly correlated over a
    /// fraction of a millisecond.
    fn speechlike(n: usize, rate: f32) -> Vec<f32> {
        let w = noise(n, 0.3, 0x5EED);
        let biquad = |x: &[f32], f0: f32, q: f32, high: bool| {
            let om = std::f32::consts::TAU * f0 / rate;
            let (sn, cs) = om.sin_cos();
            let a = sn / (2.0 * q);
            let (b0, b1, b2) = if high {
                ((1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0)
            } else {
                ((1.0 - cs) / 2.0, 1.0 - cs, (1.0 - cs) / 2.0)
            };
            let (a0, a1, a2) = (1.0 + a, -2.0 * cs, 1.0 - a);
            let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
            x.iter()
                .map(|&v| {
                    let y = (b0 * v + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
                    (x2, x1, y2, y1) = (x1, v, y1, y);
                    y
                })
                .collect::<Vec<f32>>()
        };
        let mut v = biquad(&biquad(&w, 300.0, 0.707, true), 2700.0, 0.707, false);
        v = biquad(&biquad(&v, 300.0, 0.707, true), 2700.0, 0.707, false);
        // Syllables: 150 ms on, 100 ms off.
        let syl = (0.25 * rate) as usize;
        for (i, s) in v.iter_mut().enumerate() {
            if i % syl >= (0.15 * rate) as usize {
                *s = 0.0;
            }
        }
        v
    }

    /// At 48 kHz, where the receive chain and a rig's audio both run, speech
    /// comes through nearly as it went in (#901). The 12 kHz constants used
    /// here unscaled cancelled the voice itself.
    #[test]
    fn passes_speech_at_48k() {
        let rate = 48_000.0;
        let input = speechlike(4 * 48_000, rate as f32);
        let mut out = input.clone();
        AutoNotch::new(rate).process(&mut out);
        let tail = 48_000..input.len();
        let diff: Vec<f32> = tail.clone().map(|i| out[i] - input[i]).collect();
        let (sig, err) = (rms(&input[tail]), rms(&diff));
        assert!(err < 0.35 * sig, "speech distorted: error {err} against {sig}");
    }

    /// And a heterodyne under the speech at 48 kHz still goes.
    #[test]
    fn cancels_a_tone_under_speech_at_48k() {
        let rate = 48_000.0f32;
        let speech = speechlike(6 * 48_000, rate);
        let tone: Vec<f32> = (0..speech.len())
            .map(|i| 0.3 * (std::f32::consts::TAU * 1000.0 * i as f32 / rate).sin())
            .collect();
        let mut buf: Vec<f32> = speech.iter().zip(&tone).map(|(a, b)| a + b).collect();
        AutoNotch::new(f64::from(rate)).process(&mut buf);
        let tail = 3 * 48_000..buf.len();
        let left: Vec<f32> = tail.clone().map(|i| buf[i] - speech[i]).collect();
        let (t, l) = (rms(&tone[tail]), rms(&left));
        assert!(l < 0.5 * t, "tone not notched under speech: {t} -> {l}");
    }
}
