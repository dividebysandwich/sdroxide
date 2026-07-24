//! "RF Paint" (Spectrum Painting) transmit synthesizer.
//!
//! Turns a `w`×`h` intensity mask into an audio stream that paints the picture
//! directly onto a receiver's waterfall (no decoder — the picture *is* the
//! signal). Each image **column** maps to an audio-frequency bin spread across
//! the painting band; each image **row** is one time-slice of fixed dwell.
//! Within a slice, every lit pixel contributes a sine at its column's frequency,
//! and the whole chord is summed.
//!
//! Rows are emitted **bottom-first** so the picture stands upright on a
//! "newest-row-at-the-top" waterfall (the app's own convention): the last row
//! sent — the top of the image — lands at the top of the display.
//!
//! The band is kept to 3 kHz (300..3300 Hz audio, USB), comfortably inside the
//! 4 kHz limit while leaving the tones far enough apart to be legible on a
//! zoomed waterfall.
//!
//! Samples are generated **on demand** in [`SpectrumPaintTx::next_block`], so a
//! slow, long transmission costs `O(bins)` memory rather than a multi-hundred-MB
//! precomputed buffer.

use std::f64::consts::TAU;

/// Painting band, audio Hz (USB). 300..3300 Hz = 3 kHz wide (≤ the 4 kHz cap).
pub const BAND_LO_HZ: f64 = 300.0;
pub const BAND_HI_HZ: f64 = 3300.0;
/// Band centre — the mode's nominal "audio tone" used for markers / zoom.
pub const CENTER_HZ: f32 = 1800.0;
/// Base dwell per image row (one waterfall time-slice) at speed 1.0, seconds.
/// The transmit-speed control divides this (lower speed ⇒ longer dwell).
pub const ROW_SECS: f64 = 0.055;

/// Safety caps so a pathological image can't sum an unreasonable number of
/// simultaneous tones or run for an absurd number of rows. The UI bounds its
/// bitmaps well below these; anything larger is sub-sampled here.
const MAX_BINS: usize = 256; // frequency columns
const MAX_ROWS: usize = 2048; // time rows (long painted-text banners can be wide)

/// A streaming spectrum-painting burst. Built from an intensity mask, then
/// pulled block-by-block by the controller's `fill_tx_block`.
pub struct SpectrumPaintTx {
    gray: Vec<u8>,
    w: usize,
    h: usize,
    bins: usize,
    rows: usize,
    dwell: usize,
    /// Per-bin phase increment (rad/sample); phases advance continuously across
    /// rows so a bin that stays lit never clicks at a row boundary.
    dphase: Vec<f64>,
    phase: Vec<f64>,
    /// Output gain, chosen so the brightest row can't clip the modulator.
    gain: f32,
    /// Current time-row (0..rows) and sample within it (0..dwell).
    t: usize,
    k: usize,
    total: u64,
    emitted: u64,
}

impl SpectrumPaintTx {
    /// Build the burst for a `w`×`h` intensity mask (`gray[y * w + x]`, 0..=255,
    /// row 0 = top of the picture) at sample rate `rate`. `speed` is a fraction
    /// of the base scan rate (1.0 = fastest, 0.25 = default / 4× slower).
    pub fn new(gray: &[u8], w: usize, h: usize, rate: f64, speed: f32) -> Self {
        if w == 0 || h == 0 || gray.len() < w * h {
            return SpectrumPaintTx {
                gray: Vec::new(),
                w: 0,
                h: 0,
                bins: 0,
                rows: 0,
                dwell: 0,
                dphase: Vec::new(),
                phase: Vec::new(),
                gain: 1.0,
                t: 0,
                k: 0,
                total: 0,
                emitted: 0,
            };
        }
        let bins = w.clamp(1, MAX_BINS);
        let rows = h.clamp(1, MAX_ROWS);
        let speed = speed.clamp(0.02, 4.0) as f64;
        let dwell = (ROW_SECS * rate / speed).round().max(1.0) as usize;

        let dphase: Vec<f64> = (0..bins)
            .map(|ix| {
                let f = if bins > 1 {
                    BAND_LO_HZ + (ix as f64 / (bins - 1) as f64) * (BAND_HI_HZ - BAND_LO_HZ)
                } else {
                    (BAND_LO_HZ + BAND_HI_HZ) * 0.5
                };
                TAU * f / rate
            })
            .collect();

        // Conservative gain: the brightest row's summed tone weight bounds the
        // worst-case peak (all phases aligned), so scaling by 0.85 / that keeps
        // headroom for the modulator without a full audio pre-pass.
        let mut max_sum = 0.0f64;
        for t in 0..rows {
            let iy = row_index(t, rows, h);
            let mut s = 0.0;
            for ix in 0..bins {
                let sx = ix * w / bins;
                s += gray[iy * w + sx] as f64 / 255.0;
            }
            max_sum = max_sum.max(s);
        }
        let gain = if max_sum > 1e-9 { (0.85 / max_sum) as f32 } else { 1.0 };

        SpectrumPaintTx {
            gray: gray.to_vec(),
            w,
            h,
            bins,
            rows,
            dwell,
            dphase,
            phase: vec![0.0; bins],
            gain,
            t: 0,
            k: 0,
            total: rows as u64 * dwell as u64,
            emitted: 0,
        }
    }

    /// Fill `out` with the next block, zero-filling any tail past the end.
    /// Returns the count of real (non-padding) samples written.
    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for o in out.iter_mut() {
            if self.emitted >= self.total {
                *o = 0.0;
                continue;
            }
            let iy = row_index(self.t, self.rows, self.h);
            let mut s = 0.0f64;
            for ix in 0..self.bins {
                self.phase[ix] += self.dphase[ix];
                if self.phase[ix] > TAU {
                    self.phase[ix] -= TAU;
                }
                let sx = ix * self.w / self.bins;
                let v = self.gray[iy * self.w + sx];
                if v > 0 {
                    s += (v as f64 / 255.0) * self.phase[ix].sin();
                }
            }
            *o = s as f32 * self.gain;
            self.emitted += 1;
            written += 1;
            self.k += 1;
            if self.k >= self.dwell {
                self.k = 0;
                self.t += 1;
            }
        }
        written
    }

    /// True once the whole burst has been emitted.
    pub fn done(&self) -> bool {
        self.emitted >= self.total
    }

    /// Total planned samples (for timing / progress).
    pub fn total_samples(&self) -> u64 {
        self.total
    }

    /// Fraction transmitted so far, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            (self.emitted as f32 / self.total as f32).clamp(0.0, 1.0)
        }
    }
}

/// The source image row painted at time-slice `t`. Bottom-first (so the picture
/// is upright on a newest-at-top waterfall); sub-sampled when `h > rows`.
fn row_index(t: usize, rows: usize, h: usize) -> usize {
    (rows - 1 - t) * h / rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_done() {
        let tx = SpectrumPaintTx::new(&[], 0, 0, 48_000.0, 1.0);
        assert!(tx.done());
        assert_eq!(tx.total_samples(), 0);
    }

    #[test]
    fn length_scales_with_speed() {
        let gray = vec![255u8; 4 * 3];
        let base = SpectrumPaintTx::new(&gray, 4, 3, 48_000.0, 1.0).total_samples();
        let slow = SpectrumPaintTx::new(&gray, 4, 3, 48_000.0, 0.25).total_samples();
        // Quarter speed ⇒ ~4× the samples.
        assert!((slow as f64 / base as f64 - 4.0).abs() < 0.05, "slow {slow} base {base}");
    }

    #[test]
    fn drains_and_stays_within_range() {
        let gray = vec![255u8; 8 * 4];
        let mut tx = SpectrumPaintTx::new(&gray, 8, 4, 48_000.0, 1.0);
        let mut peak = 0.0f32;
        let mut block = [0.0f32; 480];
        while !tx.done() {
            tx.next_block(&mut block);
            for &s in &block {
                peak = peak.max(s.abs());
            }
        }
        assert!(peak <= 0.86, "peak {peak} exceeds the headroom ceiling");
        assert!(peak > 0.05, "all-lit mask should produce signal (peak {peak})");
    }
}
