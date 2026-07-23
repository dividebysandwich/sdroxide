//! A simple adaptive squelch for the keyboard-mode decoders: it tracks the
//! noise floor of a decoder's signal-magnitude estimate and opens only when the
//! current magnitude rises a tunable factor above that floor. This stops the
//! modems from "decoding" pure noise when no signal is present.

/// Adaptive noise-floor gate. `threshold` is 0..1 (0 = always open).
#[derive(Default)]
pub struct Squelch {
    floor: f32,
    seeded: bool,
}

impl Squelch {
    pub fn new() -> Self {
        Squelch::default()
    }

    /// Update the floor with `mag` and return whether the gate is open for the
    /// given `threshold` (0 disables the gate). The floor is a smoothed tracker
    /// of the ambient level: it follows quieter magnitudes fairly quickly and
    /// louder ones slowly, and it is frozen while the gate is open so a steady
    /// signal can't slowly squelch itself. Tracking a smoothed level (rather than
    /// the running minimum) keeps noise crest peaks from false-opening the gate.
    pub fn open(&mut self, mag: f32, threshold: f32) -> bool {
        if !self.seeded {
            self.floor = mag;
            self.seeded = true;
        }
        if threshold <= 0.01 {
            // Off: still track the floor so it's warm when the gate is enabled.
            self.floor += (mag - self.floor) * 0.05;
            return true;
        }
        // threshold 0..1 → require ~1.4× … 6× the noise floor.
        let ratio = 1.0 + threshold * 5.0;
        let open = mag > self.floor * ratio;
        if mag < self.floor {
            self.floor += (mag - self.floor) * 0.05; // attack down toward quiet
        } else if !open {
            self.floor += (mag - self.floor) * 0.02; // release up while squelched
        }
        open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A cheap deterministic pseudo-random in 0..1.
    fn rng(state: &mut u32) -> f32 {
        *state ^= *state << 13;
        *state ^= *state >> 17;
        *state ^= *state << 5;
        (*state >> 8) as f32 / (1u32 << 24) as f32
    }

    #[test]
    fn noise_stays_closed_signal_opens() {
        let mut sq = Squelch::new();
        let mut st = 0x1234_5678u32;
        // Fluctuating noise around ~0.1 must not open the gate.
        let mut noise_opens = 0;
        for _ in 0..2000 {
            let mag = 0.06 + 0.08 * rng(&mut st); // 0.06..0.14
            if sq.open(mag, 0.35) {
                noise_opens += 1;
            }
        }
        assert_eq!(noise_opens, 0, "fluctuating noise must keep the squelch closed");
        // A signal well above the floor opens it.
        assert!(sq.open(0.8, 0.35), "a strong signal should open the squelch");
    }

    #[test]
    fn threshold_zero_is_always_open() {
        let mut sq = Squelch::new();
        for _ in 0..100 {
            assert!(sq.open(0.001, 0.0));
        }
    }
}
