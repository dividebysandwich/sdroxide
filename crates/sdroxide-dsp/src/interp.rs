//! Interpolation: the TX mirror of the decimators — 48 kHz complex baseband
//! up to the device rate via a rational resampler plus ×2 half-band stages.

use crate::Complex32;
use crate::decim::lowpass_taps;
use crate::resample::ComplexResampler;

/// ×2 interpolator: zero-stuff then half-band low-pass (gain-compensated).
pub struct HalfbandInterp {
    taps: Vec<f32>,
    buf: Vec<Complex32>,
    stuffed: Vec<Complex32>,
}

impl HalfbandInterp {
    pub fn new() -> Self {
        // Cutoff 0.25 of the OUTPUT rate; ×2 restores the zero-stuffing loss.
        let taps = lowpass_taps(23, 0.25).into_iter().map(|t| t * 2.0).collect();
        HalfbandInterp { taps, buf: Vec::new(), stuffed: Vec::new() }
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.stuffed.clear();
        self.stuffed.reserve(input.len() * 2);
        for &x in input {
            self.stuffed.push(x);
            self.stuffed.push(Complex32::default());
        }

        self.buf.extend_from_slice(&self.stuffed);
        let n = self.taps.len();
        if self.buf.len() < n {
            return;
        }
        let count = self.buf.len() - n + 1;
        out.reserve(count);
        for o in 0..count {
            let mut acc = Complex32::default();
            for (k, &t) in self.taps.iter().enumerate() {
                acc += self.buf[o + k] * t;
            }
            out.push(acc);
        }
        self.buf.drain(..count);
    }
}

/// Digital up-converter: complex baseband at `in_rate` → `out_rate`.
pub struct Duc {
    resampler: Option<ComplexResampler>,
    stages: Vec<HalfbandInterp>,
    out_rate: f64,
    tmp_a: Vec<Complex32>,
    tmp_b: Vec<Complex32>,
}

impl Duc {
    pub fn new(in_rate: f64, out_rate: f64) -> Self {
        // out_rate = base · 2^k with base ∈ [in_rate, 2·in_rate).
        let mut k = 0u32;
        while out_rate / 2f64.powi(k as i32 + 1) >= in_rate {
            k += 1;
        }
        let base = out_rate / 2f64.powi(k as i32);
        Duc {
            resampler: ComplexResampler::new(in_rate, base),
            stages: (0..k).map(|_| HalfbandInterp::new()).collect(),
            out_rate,
            tmp_a: Vec::new(),
            tmp_b: Vec::new(),
        }
    }

    pub fn out_rate(&self) -> f64 {
        self.out_rate
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.tmp_a.clear();
        match &mut self.resampler {
            Some(r) => r.push(input, &mut self.tmp_a),
            None => self.tmp_a.extend_from_slice(input),
        }
        for stage in &mut self.stages {
            self.tmp_b.clear();
            stage.process(&self.tmp_a, &mut self.tmp_b);
            std::mem::swap(&mut self.tmp_a, &mut self.tmp_b);
        }
        out.extend_from_slice(&self.tmp_a);
    }
}
