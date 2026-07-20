//! Streaming FIR filters at channel rate.

use std::f64::consts::TAU;

use crate::Complex32;
use crate::decim::lowpass_taps;

/// Complex band-pass taps with passband [lo_hz, hi_hz] relative to DC
/// (either or both edges may be negative — that selects the sideband).
pub fn bandpass_taps(ntaps: usize, lo_hz: f64, hi_hz: f64, sample_rate: f64) -> Vec<Complex32> {
    let bw = (hi_hz - lo_hz).abs().max(50.0);
    let center = (lo_hz + hi_hz) / 2.0;
    let lp = lowpass_taps(ntaps, (bw / 2.0) / sample_rate);
    let mid = (ntaps - 1) as f64 / 2.0;
    // Negative exponent: ComplexFir applies taps in correlation orientation
    // (no time reversal), which mirrors the shift.
    lp.iter()
        .enumerate()
        .map(|(i, &t)| {
            let ph = -TAU * center * (i as f64 - mid) / sample_rate;
            Complex32::new((ph.cos() * t as f64) as f32, (ph.sin() * t as f64) as f32)
        })
        .collect()
}

/// Streaming complex FIR (complex in, complex out).
pub struct ComplexFir {
    taps: Vec<Complex32>,
    buf: Vec<Complex32>,
}

impl ComplexFir {
    pub fn new(taps: Vec<Complex32>) -> Self {
        ComplexFir { taps, buf: Vec::new() }
    }

    pub fn set_taps(&mut self, taps: Vec<Complex32>) {
        self.taps = taps;
        // Keep history; length mismatch just causes one transient block.
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.buf.extend_from_slice(input);
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

    pub fn reset(&mut self) {
        self.buf.clear();
    }
}

/// Streaming real FIR (f32 in/out), used for post-discriminator audio filtering.
pub struct RealFir {
    taps: Vec<f32>,
    buf: Vec<f32>,
}

impl RealFir {
    pub fn lowpass(ntaps: usize, cutoff_hz: f64, sample_rate: f64) -> Self {
        RealFir { taps: lowpass_taps(ntaps, cutoff_hz / sample_rate), buf: Vec::new() }
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.buf.extend_from_slice(input);
        let n = self.taps.len();
        if self.buf.len() < n {
            return;
        }
        let count = self.buf.len() - n + 1;
        out.reserve(count);
        for o in 0..count {
            let mut acc = 0.0f32;
            for (k, &t) in self.taps.iter().enumerate() {
                acc += self.buf[o + k] * t;
            }
            out.push(acc);
        }
        self.buf.drain(..count);
    }
}
