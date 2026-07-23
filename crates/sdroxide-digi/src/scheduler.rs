//! UTC slot-boundary math for FT8/FT4. Slots are aligned to the Unix epoch
//! (which is aligned to UTC minutes), so FT8 transmits at wall-clock seconds
//! ≡ 0 mod 15 and FT4 at ≡ 0 mod 7.5.

use std::time::{SystemTime, UNIX_EPOCH};

use sdroxide_types::Mode;

use crate::params::DigiParams;

pub struct SlotScheduler {
    period_s: f64,
    tx_offset_s: f64,
}

impl SlotScheduler {
    pub fn for_mode(mode: Mode) -> Self {
        let p = DigiParams::for_mode(mode);
        SlotScheduler { period_s: p.slot_s, tx_offset_s: p.tx_offset_s }
    }

    /// Seconds since the Unix epoch as an f64.
    pub fn unix_now(now: SystemTime) -> f64 {
        now.duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
    }

    /// Index of the slot containing `now` (floor(unix / period)).
    pub fn slot_index(&self, now: SystemTime) -> i64 {
        (Self::unix_now(now) / self.period_s).floor() as i64
    }

    /// Index of the slot containing Unix time `unix` (floor(unix / period)).
    pub fn slot_index_unix(&self, unix: f64) -> i64 {
        (unix / self.period_s).floor() as i64
    }

    /// Unix seconds at the start of slot `idx`.
    pub fn slot_start_unix(&self, idx: i64) -> f64 {
        idx as f64 * self.period_s
    }

    /// Seconds elapsed into the current slot (0..period).
    pub fn secs_into_slot(&self, now: SystemTime) -> f64 {
        let u = Self::unix_now(now);
        u - (u / self.period_s).floor() * self.period_s
    }

    /// True if `idx` is an even period. FT8 alternates even/odd on 15 s;
    /// stations calling CQ pick one and reply in the other.
    pub fn is_even(&self, idx: i64) -> bool {
        idx.rem_euclid(2) == 0
    }

    /// The transmit start time within slot `idx` (slot start + tx offset).
    pub fn tx_start_unix(&self, idx: i64) -> f64 {
        self.slot_start_unix(idx) + self.tx_offset_s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(unix: f64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs_f64(unix)
    }

    #[test]
    fn ft8_slot_math() {
        let s = SlotScheduler::for_mode(Mode::Ft8);
        // 1609459200 = 2021-01-01 00:00:00, divisible by 15 → slot start.
        assert_eq!(s.slot_index(at(1_609_459_200.0)), 1_609_459_200 / 15);
        assert!(s.secs_into_slot(at(1_609_459_200.0)).abs() < 1e-6);
        assert!((s.secs_into_slot(at(1_609_459_207.0)) - 7.0).abs() < 1e-6);
        // Consecutive slots alternate even/odd.
        let idx = s.slot_index(at(1_609_459_200.0));
        assert_ne!(s.is_even(idx), s.is_even(idx + 1));
    }

    #[test]
    fn ft4_offset_and_period() {
        let s = SlotScheduler::for_mode(Mode::Ft4);
        // 7.5 s period: two slots per 15 s.
        assert_eq!(s.slot_index(at(1_609_459_207.0)), 1_609_459_200 * 2 / 15 + 0);
        // tx starts 0.5 s into the slot.
        let idx = s.slot_index(at(1_609_459_200.0));
        assert!((s.tx_start_unix(idx) - 1_609_459_200.5).abs() < 1e-6);
    }
}
