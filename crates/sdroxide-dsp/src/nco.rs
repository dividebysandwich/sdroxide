use std::f64::consts::TAU;

use crate::Complex32;

/// Numerically controlled oscillator: multiplies a signal by e^{j2πft}.
/// Complex-recurrence in f64 with periodic renormalization — phase-continuous
/// across frequency changes, sub-0.01 Hz accuracy at Msps rates.
pub struct Nco {
    phasor: num_complex::Complex<f64>,
    step: num_complex::Complex<f64>,
    renorm: u32,
}

impl Nco {
    /// `freq_hz` may be negative. Positive shifts the signal up in frequency.
    pub fn new(freq_hz: f64, sample_rate: f64) -> Self {
        let mut nco = Nco {
            phasor: num_complex::Complex::new(1.0, 0.0),
            step: num_complex::Complex::new(1.0, 0.0),
            renorm: 0,
        };
        nco.set_freq(freq_hz, sample_rate);
        nco
    }

    /// Phase-continuous retune.
    pub fn set_freq(&mut self, freq_hz: f64, sample_rate: f64) {
        let w = TAU * freq_hz / sample_rate;
        self.step = num_complex::Complex::new(w.cos(), w.sin());
    }

    /// out[i] = input[i] * phasor (appends to `out`).
    pub fn mix(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        out.reserve(input.len());
        for &x in input {
            let p = Complex32::new(self.phasor.re as f32, self.phasor.im as f32);
            out.push(x * p);
            self.phasor *= self.step;
            self.renorm += 1;
            if self.renorm >= 1 << 16 {
                self.renorm = 0;
                self.phasor /= self.phasor.norm();
            }
        }
    }
}
