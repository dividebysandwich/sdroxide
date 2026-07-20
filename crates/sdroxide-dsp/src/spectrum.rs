use std::sync::Arc;

use rustfft::{Fft, FftPlanner};
use sdroxide_types::SpectrumFrame;

use crate::{Complex32, window::blackman_harris};

/// Overlapped windowed-FFT power spectrum with exponential averaging.
///
/// Feed raw IQ with [`process`](Self::process); it runs an FFT every
/// `fft_size / 2` samples (50 % overlap) and folds the result into a running
/// power average. Emit display frames at any rate with
/// [`make_frame`](Self::make_frame).
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    fft_size: usize,
    hop: usize,
    window: Vec<f32>,
    /// Normalization so a full-scale coherent sine reads ~0 dBFS.
    coherent_gain: f32,

    pending: Vec<Complex32>,
    scratch: Vec<Complex32>,
    work: Vec<Complex32>,

    /// Averaged linear power per bin, natural FFT order.
    avg_power: Vec<f32>,
    alpha: f32,
    primed: bool,
    peak_abs: f32,
    seq: u32,
    /// Hide the hardware DC/LO-leakage spike in emitted frames.
    dc_suppress: bool,
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize, sample_rate: f64, avg_tc_secs: f32) -> Self {
        let fft = FftPlanner::new().plan_fft_forward(fft_size);
        let window = blackman_harris(fft_size);
        let coherent_gain: f32 = window.iter().sum();
        let hop = fft_size / 2;
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];

        let mut analyzer = SpectrumAnalyzer {
            fft,
            fft_size,
            hop,
            window,
            coherent_gain,
            pending: Vec::with_capacity(fft_size * 2),
            scratch,
            work: vec![Complex32::default(); fft_size],
            avg_power: vec![0.0; fft_size],
            alpha: 1.0,
            primed: false,
            peak_abs: 0.0,
            seq: 0,
            dc_suppress: true,
        };
        analyzer.set_avg_tc(avg_tc_secs, sample_rate);
        analyzer
    }

    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Clear the overlap/averaging state (e.g. across a TX→RX transition so
    /// transmit samples don't contaminate the first receive frames).
    pub fn reset(&mut self) {
        self.pending.clear();
        self.avg_power.iter_mut().for_each(|p| *p = 0.0);
        self.primed = false;
        self.peak_abs = 0.0;
    }

    pub fn set_avg_tc(&mut self, tc_secs: f32, sample_rate: f64) {
        let hop_time = self.hop as f32 / sample_rate as f32;
        self.alpha = if tc_secs <= 0.0 {
            1.0
        } else {
            1.0 - (-hop_time / tc_secs).exp()
        };
    }

    /// Consume IQ samples, running as many overlapped FFTs as fit.
    pub fn process(&mut self, iq: &[Complex32]) {
        for s in iq {
            let a = s.norm_sqr();
            if a > self.peak_abs {
                self.peak_abs = a;
            }
        }
        self.pending.extend_from_slice(iq);

        while self.pending.len() >= self.fft_size {
            for (w, (x, win)) in self
                .work
                .iter_mut()
                .zip(self.pending.iter().zip(&self.window))
            {
                *w = x * win;
            }
            self.fft.process_with_scratch(&mut self.work, &mut self.scratch);

            let norm = 1.0 / (self.coherent_gain * self.coherent_gain);
            if self.primed {
                for (avg, x) in self.avg_power.iter_mut().zip(&self.work) {
                    let p = x.norm_sqr() * norm;
                    *avg += self.alpha * (p - *avg);
                }
            } else {
                for (avg, x) in self.avg_power.iter_mut().zip(&self.work) {
                    *avg = x.norm_sqr() * norm;
                }
                self.primed = true;
            }
            self.pending.drain(..self.hop);
        }
    }

    /// Peak input magnitude (dBFS) since the last call; resets on read.
    pub fn take_peak_dbfs(&mut self) -> f32 {
        let p = self.peak_abs;
        self.peak_abs = 0.0;
        10.0 * (p + 1e-20).log10()
    }

    /// Averaged spectrum in dBFS, frequency-ascending (DC centered).
    pub fn spectrum_db(&self, out: &mut Vec<f32>) {
        out.clear();
        out.reserve(self.fft_size);
        let half = self.fft_size / 2;
        for &p in self.avg_power[half..].iter().chain(&self.avg_power[..half]) {
            out.push(10.0 * (p + 1e-20).log10());
        }
    }

    /// Build a display frame with `out_bins` bins (max-pooled), u8-mapped
    /// over `[db_floor, db_ceil]`. With `viewport = Some((lo_hz, hi_hz))`
    /// only that sub-span is extracted (zoomed display); the frame's
    /// center/span then describe the viewport.
    pub fn make_frame(
        &mut self,
        center_hz: f64,
        span_hz: f64,
        db_floor: f32,
        db_ceil: f32,
        out_bins: usize,
        viewport: Option<(f64, f64)>,
    ) -> SpectrumFrame {
        let n = self.fft_size;
        let half = n / 2;
        let out_bins = out_bins.clamp(1, n * 4);
        let scale = 255.0 / (db_ceil - db_floor).max(1e-3);

        let (frac_lo, frac_hi, out_center, out_span) = match viewport {
            Some((lo, hi)) if hi > lo && span_hz > 0.0 => {
                let full_lo = center_hz - span_hz / 2.0;
                let flo = ((lo - full_lo) / span_hz).clamp(0.0, 0.998);
                let fhi = ((hi - full_lo) / span_hz).clamp(flo + 0.002, 1.0);
                (
                    flo,
                    fhi,
                    full_lo + (flo + fhi) / 2.0 * span_hz,
                    (fhi - flo) * span_hz,
                )
            }
            _ => (0.0, 1.0, center_hz, span_hz),
        };

        // DC spike suppression (hardware LO leakage): read the ±2 bins
        // around DC as the average of their neighbors. Patch at read time so
        // the running average stays uncontaminated.
        let dc_repl = if self.dc_suppress && n > 16 {
            let mut acc = 0.0f32;
            for d in 3..=6 {
                acc += self.avg_power[d] + self.avg_power[n - d];
            }
            Some(acc / 8.0)
        } else {
            None
        };
        let shifted = |i: usize| {
            let nat = (i + half) % n;
            if let Some(repl) = dc_repl {
                if nat.min(n - nat) <= 2 {
                    return repl;
                }
            }
            self.avg_power[nat]
        };
        let lo_bin = frac_lo * n as f64;
        let bin_range = (frac_hi - frac_lo) * n as f64;

        let mut bins = Vec::with_capacity(out_bins);
        for b in 0..out_bins {
            let lo = (lo_bin + b as f64 * bin_range / out_bins as f64) as usize;
            let hi = ((lo_bin + (b + 1) as f64 * bin_range / out_bins as f64) as usize)
                .clamp(lo + 1, n);
            let mut max_p = 0.0f32;
            for i in lo..hi.max(lo + 1).min(n) {
                max_p = max_p.max(shifted(i));
            }
            let db = 10.0 * (max_p + 1e-20).log10();
            bins.push(((db - db_floor) * scale).clamp(0.0, 255.0) as u8);
        }

        self.seq = self.seq.wrapping_add(1);
        SpectrumFrame {
            seq: self.seq,
            center_hz: out_center,
            span_hz: out_span,
            db_floor,
            db_ceil,
            bins,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn tone_lands_in_the_right_bin_at_the_right_level() {
        let fs = 1_000_000.0;
        let n = 1024;
        let tone_hz = 250_000.0f32; // exactly bin n*0.25 above DC
        let mut an = SpectrumAnalyzer::new(n, fs, 0.0);

        let iq: Vec<Complex32> = (0..n * 4)
            .map(|i| {
                let ph = TAU * tone_hz * i as f32 / fs as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();
        an.process(&iq);

        let mut db = Vec::new();
        an.spectrum_db(&mut db);

        // DC-centered ordering: +fs/4 sits at 3/4 of the display.
        let expect = n * 3 / 4;
        let peak_bin = db
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(peak_bin, expect);
        // Full-scale coherent tone should read close to 0 dBFS.
        assert!(db[peak_bin] > -1.0 && db[peak_bin] < 1.0, "{}", db[peak_bin]);
        // Blackman-Harris sidelobes: far bins well below -80 dB.
        assert!(db[n / 4] < -80.0, "{}", db[n / 4]);
    }
}
