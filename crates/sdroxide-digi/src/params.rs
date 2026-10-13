//! Per-mode FT8/FT4 protocol timing.

use sdroxide_types::{Js8Speed, Mode, SlotTiming};

/// mfsk-core works entirely at this sample rate.
pub const DECODE_RATE: f64 = 12_000.0;

/// Frequency search window for decode/display (Hz within the passband).
pub const AUDIO_MIN_HZ: f32 = 100.0;
pub const AUDIO_MAX_HZ: f32 = 3300.0;

/// The sync score an FT8/FT4 candidate needs to be tried.
pub const SYNC_MIN: f32 = 1.5;
/// How many sync candidates a slot's decode is allowed to try.
///
/// The list is sorted by sync power and cut here, so on a quiet band this is
/// never reached and on a busy one it decides which of the weak signals are
/// never looked at — which is exactly the shape of issue #307, where the gap
/// against WSJT-X grew with the number of stations on the band.
///
/// Measured on a synthetic forty-signal slot: 120 candidates found thirteen
/// messages, 300 found fourteen and 600 found fifteen, for 9.6, 10.2 and
/// 11.8 milliseconds of decode. A slot is fifteen seconds long, so the whole
/// range is free and the only question is how many are looked at.
pub const MAX_CAND: usize = 600;

#[derive(Debug, Clone, Copy)]
pub struct DigiParams {
    pub mode: Mode,
    /// Slot length in seconds (FT8 15, FT4 7.5).
    pub slot_s: f64,
    /// Transmit start offset into the slot (FT8 0, FT4 0.5).
    pub tx_offset_s: f64,
    /// Nominal on-air burst length in seconds.
    pub burst_s: f64,
}

impl DigiParams {
    /// Timing for the modes whose slot length is fixed by the mode itself.
    ///
    /// The figures come from [`Mode::slot_timing`] rather than living here: the
    /// panels draw a turn's progress from the same numbers the scheduler keys
    /// against, and two copies of a slot length is two things to get wrong.
    ///
    /// The fallback arm used to rewrite `mode` to [`Mode::Ft8`], which combined
    /// with `make_digi`'s fall-through to `DigiController` meant any mode that
    /// forgot a branch quietly became FT8 — no error, no log line, just a
    /// receiver listening for the wrong protocol. It now keeps the mode it was
    /// given and complains in debug builds.
    pub fn for_mode(mode: Mode) -> Self {
        match mode.slot_timing() {
            Some(t) => DigiParams::from_timing(mode, t),
            None => {
                debug_assert!(
                    false,
                    "DigiParams::for_mode({mode:?}) has no slot timing — give the mode one in \
                     Mode::slot_timing rather than inheriting FT8's"
                );
                DigiParams::from_timing(mode, Mode::Ft8.slot_timing().expect("FT8 is slotted"))
            }
        }
    }

    /// Timing for JS8, whose slot length is a runtime setting rather than a
    /// distinct [`Mode`] — so [`DigiParams::for_mode`] cannot express it.
    pub fn for_js8(speed: Js8Speed) -> Self {
        DigiParams::from_timing(Mode::Js8, speed.slot_timing())
    }

    fn from_timing(mode: Mode, t: SlotTiming) -> Self {
        DigiParams { mode, slot_s: t.slot_s, tx_offset_s: t.tx_offset_s, burst_s: t.burst_s }
    }

    /// Samples of 12 kHz audio in one slot.
    pub fn slot_samples(&self) -> usize {
        (self.slot_s * DECODE_RATE) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digi_params_never_lies_about_the_mode() {
        // The regression this guards: the old fallback rewrote `mode` to
        // `Mode::Ft8`, so a caller could ask for one mode and silently get
        // another's timing *and* another's decoder.
        assert_eq!(DigiParams::for_mode(Mode::Ft8).mode, Mode::Ft8);
        assert_eq!(DigiParams::for_mode(Mode::Ft4).mode, Mode::Ft4);
        assert_eq!(DigiParams::for_mode(Mode::Ft2).mode, Mode::Ft2);
        assert_eq!(DigiParams::for_js8(Js8Speed::Normal).mode, Mode::Js8);
    }

    /// Every slotted mode's burst has to start and finish inside its slot, and
    /// its slot has to divide the minute or the two ends drift apart.
    #[test]
    fn slotted_timing_closes_on_the_minute() {
        for mode in [Mode::Ft8, Mode::Ft4, Mode::Ft2] {
            let p = DigiParams::for_mode(mode);
            assert!(p.tx_offset_s + p.burst_s < p.slot_s, "{mode:?} burst overruns its slot");
            assert!((60.0 / p.slot_s).fract() < 1e-9, "{mode:?} slot does not divide a minute");
        }
    }

    #[test]
    fn js8_timing_follows_the_speed() {
        for speed in Js8Speed::ALL {
            let p = DigiParams::for_js8(speed);
            assert_eq!(p.slot_s, speed.slot_s(), "{}", speed.label());
            assert_eq!(p.tx_offset_s, speed.start_delay_s(), "{}", speed.label());
            // The burst plus its start delay has to fit inside the slot.
            assert!(p.tx_offset_s + p.burst_s < p.slot_s, "{}", speed.label());
        }
    }
}
