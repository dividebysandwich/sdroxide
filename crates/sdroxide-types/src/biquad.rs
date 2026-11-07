//! Second-order filter design shared by the transmit voice chain's DSP and
//! the UI that draws its curves.

/// The five normalized coefficients of one second-order section — the filter's
/// *shape*, with none of its memory. The DSP crate's `Biquad` runs them; the
/// UI evaluates them to draw a response curve. Here, rather than beside the
/// filter, because the UI builds for wasm32 without the DSP crate and the
/// curve it draws has to be the response the audio actually gets.
///
/// Coefficients follow the RBJ Audio EQ Cookbook, computed in `f64` and stored
/// as `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoeffs {
    /// The identity filter, used for a band whose gain is 0 dB, so a flat
    /// band costs no more than passing samples through unchanged.
    pub fn identity() -> Self {
        BiquadCoeffs { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 }
    }

    fn normalized(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> Self {
        BiquadCoeffs {
            b0: (b0 / a0) as f32,
            b1: (b1 / a0) as f32,
            b2: (b2 / a0) as f32,
            a1: (a1 / a0) as f32,
            a2: (a2 / a0) as f32,
        }
    }

    /// Peaking (bell) filter: boost/cut `gain_db` around `freq_hz`, `q` sets
    /// how narrow the bell is (higher = narrower).
    pub fn peaking(freq_hz: f64, gain_db: f64, q: f64, rate: f64) -> Self {
        if gain_db == 0.0 || freq_hz <= 0.0 || rate <= 0.0 {
            return Self::identity();
        }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = std::f64::consts::TAU * freq_hz / rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q.max(0.05));

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;
        Self::normalized(b0, b1, b2, a0, a1, a2)
    }

    /// Low shelf: boost/cut `gain_db` below `freq_hz`. `slope` is the RBJ
    /// cookbook's `S` (0 < S <= 1; 1 is the steepest monotonic shelf). That's
    /// what [`crate::TxEqBand::q`] means on a shelf band, distinct from a peaking
    /// band's Q.
    pub fn low_shelf(freq_hz: f64, gain_db: f64, slope: f64, rate: f64) -> Self {
        if gain_db == 0.0 || freq_hz <= 0.0 || rate <= 0.0 {
            return Self::identity();
        }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = std::f64::consts::TAU * freq_hz / rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let s = slope.clamp(0.1, 1.0);
        let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).sqrt();
        let sqrt_a_2alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + sqrt_a_2alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - sqrt_a_2alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + sqrt_a_2alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - sqrt_a_2alpha;
        Self::normalized(b0, b1, b2, a0, a1, a2)
    }

    /// High shelf: boost/cut `gain_db` above `freq_hz`. See [`Self::low_shelf`]
    /// for what `slope` means.
    pub fn high_shelf(freq_hz: f64, gain_db: f64, slope: f64, rate: f64) -> Self {
        if gain_db == 0.0 || freq_hz <= 0.0 || rate <= 0.0 {
            return Self::identity();
        }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = std::f64::consts::TAU * freq_hz / rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let s = slope.clamp(0.1, 1.0);
        let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).sqrt();
        let sqrt_a_2alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + sqrt_a_2alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - sqrt_a_2alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + sqrt_a_2alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - sqrt_a_2alpha;
        Self::normalized(b0, b1, b2, a0, a1, a2)
    }

    /// The `w0` and `alpha` every RBJ second-order section below starts from.
    fn w0_alpha(freq_hz: f64, q: f64, rate: f64) -> (f64, f64) {
        let w0 = std::f64::consts::TAU * freq_hz / rate;
        (w0, w0.sin() / (2.0 * q.max(0.05)))
    }

    /// Second-order high-pass; `q = 1/√2` is Butterworth.
    pub fn highpass(freq_hz: f64, q: f64, rate: f64) -> Self {
        if freq_hz <= 0.0 || rate <= 0.0 {
            return Self::identity();
        }
        let (w0, alpha) = Self::w0_alpha(freq_hz, q, rate);
        let c = w0.cos();
        Self::normalized(
            (1.0 + c) / 2.0,
            -(1.0 + c),
            (1.0 + c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    /// Second-order low-pass; `q = 1/√2` is Butterworth.
    pub fn lowpass(freq_hz: f64, q: f64, rate: f64) -> Self {
        if freq_hz <= 0.0 || rate <= 0.0 || freq_hz >= rate / 2.0 {
            return Self::identity();
        }
        let (w0, alpha) = Self::w0_alpha(freq_hz, q, rate);
        let c = w0.cos();
        Self::normalized(
            (1.0 - c) / 2.0,
            1.0 - c,
            (1.0 - c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    /// Band-pass with 0 dB at its centre. Its complement, `x - bandpass(x)`,
    /// is exactly the matching notch — which is what lets the de-esser split
    /// a voice into "the sibilant band" and "everything else" and put the two
    /// back together without a trace when it is not working.
    pub fn bandpass(freq_hz: f64, q: f64, rate: f64) -> Self {
        if freq_hz <= 0.0 || rate <= 0.0 || freq_hz >= rate / 2.0 {
            return Self { b0: 0.0, ..Self::identity() };
        }
        let (w0, alpha) = Self::w0_alpha(freq_hz, q, rate);
        Self::normalized(alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * w0.cos(), 1.0 - alpha)
    }

    /// Second-order all-pass: flat magnitude, phase turning through 360°
    /// around `freq_hz`.
    pub fn allpass(freq_hz: f64, q: f64, rate: f64) -> Self {
        if freq_hz <= 0.0 || rate <= 0.0 {
            return Self::identity();
        }
        let (w0, alpha) = Self::w0_alpha(freq_hz, q, rate);
        let c = w0.cos();
        Self::normalized(1.0 - alpha, -2.0 * c, 1.0 + alpha, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }

    /// The magnitude of this section's response at `freq_hz`, for drawing a
    /// response curve.
    pub fn magnitude(&self, freq_hz: f64, rate: f64) -> f64 {
        let w = std::f64::consts::TAU * freq_hz / rate;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        let (b0, b1, b2) = (self.b0 as f64, self.b1 as f64, self.b2 as f64);
        let (a1, a2) = (self.a1 as f64, self.a2 as f64);
        let nr = b0 + b1 * c1 + b2 * c2;
        let ni = -(b1 * s1 + b2 * s2);
        let dr = 1.0 + a1 * c1 + a2 * c2;
        let di = -(a1 * s1 + a2 * s2);
        ((nr * nr + ni * ni) / (dr * dr + di * di)).sqrt()
    }
}
