//! Mono audio resampler (rubato) for the small channel-rate → audio-rate
//! ratio corrections (e.g. 50 kHz → 48 kHz).

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};

use crate::Complex32;

const CHUNK: usize = 1024;

pub struct MonoResampler {
    inner: Async<f32>,
    pending: Vec<f32>,
}

impl MonoResampler {
    /// `None` when the rates already match (within 0.01 Hz).
    pub fn new(in_rate: f64, out_rate: f64) -> Option<Self> {
        if (in_rate - out_rate).abs() < 0.01 {
            return None;
        }
        let inner = Async::new_poly(
            out_rate / in_rate,
            1.1,
            PolynomialDegree::Septic,
            CHUNK,
            1,
            FixedAsync::Input,
        )
        .expect("resampler construction");
        Some(MonoResampler { inner, pending: Vec::new() })
    }

    /// Feed input samples; appends resampled output to `out`.
    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.pending.extend_from_slice(input);
        while self.pending.len() >= CHUNK {
            let adapter = InterleavedSlice::new(&self.pending[..CHUNK], 1, CHUNK)
                .expect("adapter");
            let produced = self.inner.process(&adapter, None).expect("resample");
            out.extend_from_slice(&produced.take_data());
            self.pending.drain(..CHUNK);
        }
    }
}

/// Complex-valued resampler: I/Q as a 2-channel interleaved stream so both
/// components share exact timing.
pub struct ComplexResampler {
    inner: Async<f32>,
    pending: Vec<f32>, // interleaved re,im
}

impl ComplexResampler {
    /// `None` when the rates already match (within 0.01 Hz).
    pub fn new(in_rate: f64, out_rate: f64) -> Option<Self> {
        if (in_rate - out_rate).abs() < 0.01 {
            return None;
        }
        let inner = Async::new_poly(
            out_rate / in_rate,
            4.0,
            PolynomialDegree::Septic,
            CHUNK,
            2,
            FixedAsync::Input,
        )
        .expect("resampler construction");
        Some(ComplexResampler { inner, pending: Vec::new() })
    }

    pub fn push(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.pending.reserve(input.len() * 2);
        for z in input {
            self.pending.push(z.re);
            self.pending.push(z.im);
        }
        while self.pending.len() >= CHUNK * 2 {
            let adapter = InterleavedSlice::new(&self.pending[..CHUNK * 2], 2, CHUNK)
                .expect("adapter");
            let produced = self.inner.process(&adapter, None).expect("resample");
            let data = produced.take_data();
            out.extend(
                data.chunks_exact(2)
                    .map(|p| Complex32::new(p[0], p[1])),
            );
            self.pending.drain(..CHUNK * 2);
        }
    }
}
