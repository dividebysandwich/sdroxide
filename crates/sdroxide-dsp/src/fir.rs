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

/// Independent running sums a filter window is walked with — see
/// [`crate::decim`], where the same reasoning and the same measurements apply.
/// A single accumulator makes the whole dot product one dependency chain: every
/// multiply-accumulate waits on the one before it, and a band-pass long enough
/// to shape an SSB channel is a chain some hundreds of adds deep.
const FIR_ACCUMULATORS: usize = 8;

crate::simd::kernel! {
    /// A complete complex-tap FIR: one dot product per output sample.
    fn complex_fir / complex_fir_portable / complex_fir_avx2 / complex_fir_avx512 (
        dst: &mut [Complex32],
        buf: &[Complex32],
        taps: &[Complex32],
    ) {
        let n = taps.len();
        for (o, d) in dst.iter_mut().enumerate() {
            let w = &buf[o..o + n];
            let mut acc = [Complex32::default(); FIR_ACCUMULATORS];
            let mut ws = w.chunks_exact(FIR_ACCUMULATORS);
            let mut ts = taps.chunks_exact(FIR_ACCUMULATORS);
            for (xs, tt) in (&mut ws).zip(&mut ts) {
                for i in 0..FIR_ACCUMULATORS {
                    acc[i] += xs[i] * tt[i];
                }
            }
            let mut sum = Complex32::default();
            for a in acc {
                sum += a;
            }
            for (x, &t) in ws.remainder().iter().zip(ts.remainder()) {
                sum += x * t;
            }
            *d = sum;
        }
    }
}

crate::simd::kernel! {
    /// [`complex_fir`]'s real-valued counterpart.
    fn real_fir / real_fir_portable / real_fir_avx2 / real_fir_avx512 (
        dst: &mut [f32],
        buf: &[f32],
        taps: &[f32],
    ) {
        let n = taps.len();
        for (o, d) in dst.iter_mut().enumerate() {
            let w = &buf[o..o + n];
            let mut acc = [0.0f32; FIR_ACCUMULATORS];
            let mut ws = w.chunks_exact(FIR_ACCUMULATORS);
            let mut ts = taps.chunks_exact(FIR_ACCUMULATORS);
            for (xs, tt) in (&mut ws).zip(&mut ts) {
                for i in 0..FIR_ACCUMULATORS {
                    acc[i] += xs[i] * tt[i];
                }
            }
            let mut sum = 0.0;
            for a in acc {
                sum += a;
            }
            for (x, &t) in ws.remainder().iter().zip(ts.remainder()) {
                sum += x * t;
            }
            *d = sum;
        }
    }
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
        // Keep history. When the length changes, the history is padded or
        // trimmed by the difference so the stream keeps its sample count: a
        // longer filter would otherwise swallow the extra taps' worth of
        // output, and a shorter one emit them twice.
        let (old, new) = (self.taps.len(), taps.len());
        if !self.buf.is_empty() {
            if new > old {
                self.buf.splice(0..0, std::iter::repeat_n(Complex32::default(), new - old));
            } else {
                let cut = (old - new).min(self.buf.len());
                self.buf.drain(..cut);
            }
        }
        self.taps = taps;
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.buf.extend_from_slice(input);
        let n = self.taps.len();
        if self.buf.len() < n {
            return;
        }
        let count = self.buf.len() - n + 1;
        let start = out.len();
        out.resize(start + count, Complex32::default());
        complex_fir(&mut out[start..], &self.buf, &self.taps);
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
    pub fn new(taps: Vec<f32>) -> Self {
        RealFir { taps, buf: Vec::new() }
    }

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
        let start = out.len();
        out.resize(start + count, 0.0);
        real_fir(&mut out[start..], &self.buf, &self.taps);
        self.buf.drain(..count);
    }
}
