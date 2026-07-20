//! Simple impulse noise blanker: zero out samples whose magnitude spikes
//! far above the running average (ignition noise, power-line crackle).

use crate::Complex32;

const THRESHOLD_FACTOR: f32 = 5.0;
/// Samples of running-average settling before blanking starts.
const WARMUP: u32 = 8_192;

pub struct NoiseBlanker {
    mean: f32,
    seen: u32,
}

impl NoiseBlanker {
    pub fn new() -> Self {
        NoiseBlanker { mean: 0.0, seen: 0 }
    }

    pub fn process(&mut self, samples: &mut [Complex32]) {
        for s in samples {
            let mag = s.norm();
            // Track the mean of ordinary samples only, so impulses don't
            // raise their own threshold.
            if self.seen < WARMUP || mag < THRESHOLD_FACTOR * self.mean {
                self.mean += 5e-4 * (mag - self.mean);
                self.seen = self.seen.saturating_add(1);
            } else {
                *s = Complex32::default();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_impulses_keeps_signal() {
        let mut nb = NoiseBlanker::new();
        let n = 60_000;
        let mut buf: Vec<Complex32> = (0..n)
            .map(|i| {
                let ph = std::f32::consts::TAU * 0.01 * i as f32;
                let mut z = Complex32::new(0.1 * ph.cos(), 0.1 * ph.sin());
                if i > WARMUP as usize && i % 10_000 == 0 {
                    z *= 100.0; // impulse
                }
                z
            })
            .collect();
        nb.process(&mut buf);

        let peak_after = buf[WARMUP as usize..]
            .iter()
            .fold(0.0f32, |a, z| a.max(z.norm()));
        assert!(peak_after < 0.2, "impulse survived: {peak_after}");
        // Ordinary signal untouched.
        let idx = WARMUP as usize + 5_000;
        assert!((buf[idx].norm() - 0.1).abs() < 0.01);
    }
}
