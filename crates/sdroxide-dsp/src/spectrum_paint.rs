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

use std::f64::consts::TAU;

/// Painting band, audio Hz (USB). 300..3300 Hz = 3 kHz wide (≤ the 4 kHz cap).
pub const BAND_LO_HZ: f64 = 300.0;
pub const BAND_HI_HZ: f64 = 3300.0;
/// Band centre — the mode's nominal "audio tone" used for markers / zoom.
pub const CENTER_HZ: f32 = 1800.0;
/// Dwell per image row (one waterfall time-slice), seconds.
pub const ROW_SECS: f64 = 0.055;

/// Safety caps so a pathological image can't allocate unbounded audio or sum an
/// unreasonable number of simultaneous tones. The UI bounds its bitmaps well
/// below these; anything larger is sub-sampled here.
const MAX_BINS: usize = 256; // frequency columns
const MAX_ROWS: usize = 2048; // time rows (long painted-text banners can be wide)

/// A pre-rendered spectrum-painting burst. Built once from an intensity mask,
/// then pulled block-by-block by the controller's `fill_tx_block`.
pub struct SpectrumPaintTx {
    buf: Vec<f32>,
    pos: usize,
}

impl SpectrumPaintTx {
    /// Build the audio for a `w`×`h` intensity mask (`gray[y * w + x]`, 0..=255,
    /// row 0 = top of the picture) at sample rate `rate`.
    pub fn new(gray: &[u8], w: usize, h: usize, rate: f64) -> Self {
        if w == 0 || h == 0 || gray.len() < w * h {
            return SpectrumPaintTx { buf: Vec::new(), pos: 0 };
        }
        let bins = w.clamp(1, MAX_BINS);
        let rows = h.clamp(1, MAX_ROWS);
        let dwell = (ROW_SECS * rate).round().max(1.0) as usize;

        // Per-bin phase increment: bin 0 → BAND_LO, bin (bins-1) → BAND_HI.
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
        // Phases advance continuously across rows so a bin that stays lit never
        // clicks at a row boundary.
        let mut phase = vec![0.0f64; bins];

        let mut buf: Vec<f32> = Vec::with_capacity(rows * dwell);
        for t in 0..rows {
            // Bottom-first: paint image row (rows-1-t), sub-sampled if h > rows.
            let img_row = rows - 1 - t;
            let iy = img_row * h / rows;
            for _ in 0..dwell {
                let mut s = 0.0f64;
                for (ix, dp) in dphase.iter().enumerate() {
                    phase[ix] += dp;
                    if phase[ix] > TAU {
                        phase[ix] -= TAU;
                    }
                    // Sub-sample columns if w > bins (normally w == bins).
                    let sx = ix * w / bins;
                    let v = gray[iy * w + sx];
                    if v > 0 {
                        s += (v as f64 / 255.0) * phase[ix].sin();
                    }
                }
                buf.push(s as f32);
            }
        }

        // Normalise to a safe peak: a bright row sums many tones, so scale the
        // whole burst to 0.8 full-scale to keep headroom for the modulator.
        let peak = buf.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        if peak > 1e-6 {
            let g = 0.8 / peak;
            for x in buf.iter_mut() {
                *x *= g;
            }
        }

        SpectrumPaintTx { buf, pos: 0 }
    }

    /// Fill `out` with the next block, zero-filling any tail past the end.
    /// Returns the count of real (non-padding) samples written.
    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let n = out.len().min(self.buf.len().saturating_sub(self.pos));
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        for s in out[n..].iter_mut() {
            *s = 0.0;
        }
        self.pos += n;
        n
    }

    /// True once the whole burst has been emitted.
    pub fn done(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// Total planned samples (for timing / progress).
    pub fn total_samples(&self) -> u64 {
        self.buf.len() as u64
    }

    /// Fraction transmitted so far, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.buf.is_empty() {
            1.0
        } else {
            (self.pos as f32 / self.buf.len() as f32).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_done() {
        let tx = SpectrumPaintTx::new(&[], 0, 0, 48_000.0);
        assert!(tx.done());
        assert_eq!(tx.total_samples(), 0);
    }

    #[test]
    fn length_matches_rows_times_dwell() {
        // 4×3 all-lit mask.
        let gray = vec![255u8; 4 * 3];
        let tx = SpectrumPaintTx::new(&gray, 4, 3, 48_000.0);
        let dwell = (ROW_SECS * 48_000.0).round() as u64;
        assert_eq!(tx.total_samples(), 3 * dwell);
    }

    #[test]
    fn drains_and_normalises_within_range() {
        let gray = vec![255u8; 8 * 4];
        let mut tx = SpectrumPaintTx::new(&gray, 8, 4, 48_000.0);
        let mut peak = 0.0f32;
        let mut block = [0.0f32; 480];
        while !tx.done() {
            tx.next_block(&mut block);
            for &s in &block {
                peak = peak.max(s.abs());
            }
        }
        assert!(peak <= 0.8001, "peak {peak} exceeds normalisation ceiling");
        assert!(peak > 0.1, "all-lit mask should produce signal (peak {peak})");
    }
}
