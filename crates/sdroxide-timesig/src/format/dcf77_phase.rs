//! DCF77's phase code: each second's bit sent a second time, in the carrier's
//! phase.
//!
//! After the amplitude drop is over, PTB swings DCF77's carrier phase by a
//! few degrees either way, chip by chip, following a fixed pseudo-random
//! sequence of 512 chips — sent as it is for a 0 and inverted for a 1, the
//! same bit the amplitude code carries that second. The swing averages to
//! nothing, so receivers that only read the amplitude never notice it.
//!
//! # Learnt, not built in
//!
//! The decoder does not use the sequence below. It learns the shape the code
//! takes after the receiver's own filters by watching it: every second whose
//! amplitude bit was read clearly says which way up the code was sent, and
//! the second's phase, turned the right way up, is averaged into a template.
//! Each second is then read by correlating it with that template, and the
//! sign says the bit. So what the decoder relies on is only what makes the
//! code a code — the same waveform every second, its sign the second's bit,
//! and nothing of it in the first 200 ms, where the amplitude code is — and
//! not the generator's polynomial, the chip rate or the size of the swing.
//! Those are what [`crate::synth`] needs to put a code on a test carrier, and
//! are here for that:
//!
//! * a chip is 120 carrier cycles, 1.548 ms; 512 chips, 793 ms in all;
//! * the sequence starts 200 ms into the second;
//! * a 9-stage shift register, x⁹ + x⁵ + 1, makes 511 chips of maximal length,
//!   and one more 0 rounds them to 512;
//! * the swing is ±15.6°.
//!
//! These are as PTB has described the service (Hetzel, 1988), written down
//! from that description rather than checked against it here; the decoder
//! is built not to care.

/// When the code begins, seconds into the second.
pub const START_S: f64 = 0.2;
/// One chip: 120 cycles of 77.5 kHz.
pub const CHIP_S: f64 = 120.0 / 77_500.0;
/// Chips a second.
pub const CHIPS: usize = 512;
/// The phase swing, radians either way.
pub const DEVIATION_RAD: f64 = 15.6 * std::f64::consts::PI / 180.0;

/// The 512 chips, `true` for a 1.
pub fn chips() -> [bool; CHIPS] {
    let mut out = [false; CHIPS];
    let mut sr: u16 = 0x1ff;
    for c in out.iter_mut().take(CHIPS - 1) {
        *c = sr & 1 != 0;
        let fb = (sr ^ (sr >> 4)) & 1;
        sr = (sr >> 1) | (fb << 8);
    }
    out
}

/// The phase the code puts on the carrier `into` seconds into a second whose
/// bit is `bit`, radians.
pub fn phase(into: f64, bit: bool, chips: &[bool; CHIPS]) -> f64 {
    let k = (into - START_S) / CHIP_S;
    if k < 0.0 || k >= CHIPS as f64 {
        return 0.0;
    }
    let up = chips[k as usize] != bit;
    if up { DEVIATION_RAD } else { -DEVIATION_RAD }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sequence_is_maximal_and_balanced() {
        let c = chips();
        // A maximal-length 9-stage sequence: 256 ones and 255 zeros in its
        // 511, and the appended chip a 0.
        assert_eq!(c[..511].iter().filter(|&&x| x).count(), 256);
        assert!(!c[511]);
        // Two-valued autocorrelation: −1 at every shift but none.
        let pm: Vec<i32> = c[..511].iter().map(|&x| if x { -1 } else { 1 }).collect();
        for shift in 1..511 {
            let r: i32 = (0..511).map(|i| pm[i] * pm[(i + shift) % 511]).sum();
            assert_eq!(r, -1, "shift {shift}");
        }
    }

    #[test]
    fn the_code_fits_between_the_amplitude_drop_and_the_next_second() {
        assert!(START_S + CHIPS as f64 * CHIP_S < 1.0);
        let c = chips();
        assert_eq!(phase(0.1, false, &c), 0.0);
        assert_eq!(phase(0.999, true, &c), 0.0);
        // A 1 is the 0 inverted.
        for t in [0.2005, 0.5, 0.9] {
            assert_eq!(phase(t, true, &c), -phase(t, false, &c));
        }
    }
}
