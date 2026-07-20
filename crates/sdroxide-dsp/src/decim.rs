//! Decimators: fast half-band /2 stages plus a generic windowed-sinc
//! FIR decimator for the residual integer factor.

use std::f64::consts::PI;

use crate::Complex32;

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 { 1.0 } else { (PI * x).sin() / (PI * x) }
}

fn blackman_harris_f64(n: usize, i: usize) -> f64 {
    const A: [f64; 4] = [0.35875, 0.48829, 0.14128, 0.01168];
    let x = std::f64::consts::TAU * i as f64 / (n as f64 - 1.0);
    A[0] - A[1] * x.cos() + A[2] * (2.0 * x).cos() - A[3] * (3.0 * x).cos()
}

/// Windowed-sinc lowpass, DC gain 1. `cutoff` is normalized to the input
/// sample rate (0.5 = Nyquist).
pub fn lowpass_taps(ntaps: usize, cutoff: f64) -> Vec<f32> {
    let center = (ntaps - 1) as f64 / 2.0;
    let mut taps: Vec<f64> = (0..ntaps)
        .map(|i| 2.0 * cutoff * sinc(2.0 * cutoff * (i as f64 - center)) * blackman_harris_f64(ntaps, i))
        .collect();
    let sum: f64 = taps.iter().sum();
    taps.iter_mut().for_each(|t| *t /= sum);
    taps.into_iter().map(|t| t as f32).collect()
}

/// Half-band decimator (factor 2). Every second tap is zero, so the dot
/// product touches only ~half the taps.
pub struct HalfbandDecim {
    taps: Vec<f32>, // full tap set; odd-index taps (except center) are ~0
    buf: Vec<Complex32>,
}

impl HalfbandDecim {
    pub fn new() -> Self {
        // 23-tap half-band: cutoff exactly 0.25 makes alternate taps zero.
        let taps = lowpass_taps(23, 0.25);
        HalfbandDecim { taps, buf: Vec::new() }
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.buf.extend_from_slice(input);
        let taps = &self.taps;
        let n = taps.len();
        if self.buf.len() < n {
            return;
        }
        let count = (self.buf.len() - n) / 2 + 1;
        out.reserve(count);
        let center = n / 2;
        for o in 0..count {
            let base = o * 2;
            let mut acc = self.buf[base + center] * taps[center];
            // Non-zero taps sit at even indices.
            let mut k = 0;
            while k < n {
                acc += self.buf[base + k] * taps[k];
                k += 2;
            }
            out.push(acc);
        }
        self.buf.drain(..count * 2);
    }
}

/// Generic FIR decimator by an integer factor.
pub struct FirDecim {
    taps: Vec<f32>,
    factor: usize,
    buf: Vec<Complex32>,
}

impl FirDecim {
    pub fn new(factor: usize) -> Self {
        assert!(factor >= 1);
        let ntaps = (12 * factor).clamp(24, 768) | 1; // odd
        let taps = lowpass_taps(ntaps, 0.45 / factor as f64);
        FirDecim { taps, factor, buf: Vec::new() }
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.buf.extend_from_slice(input);
        let n = self.taps.len();
        if self.buf.len() < n {
            return;
        }
        let count = (self.buf.len() - n) / self.factor + 1;
        out.reserve(count);
        for o in 0..count {
            let base = o * self.factor;
            let mut acc = Complex32::default();
            for (k, &t) in self.taps.iter().enumerate() {
                acc += self.buf[base + k] * t;
            }
            out.push(acc);
        }
        self.buf.drain(..count * self.factor);
    }
}
