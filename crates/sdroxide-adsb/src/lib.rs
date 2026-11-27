//! The 1090 MHz ADS-B / Mode S decoder: reads the surveillance downlink every
//! civil aircraft transmits and reports one row per aircraft, with its identity,
//! altitude, velocity and position (issue #160).
//!
//! Native-only, like the skimmers and the ISM decoder: it runs in the engine off
//! the raw I/Q and reaches the UI as [`sdroxide_types::AdsbStatus`] over the
//! ordinary event path.
//!
//! # Shape of the thing
//!
//! ```text
//! 2+ Msps I/Q  ──►  envelope  ──►  preamble correlator  ──►  chip slicer
//!                                                                 │ 7 or 14 bytes
//!                       aircraft table ◄── CRC / address match ◄───┘
//! ```
//!
//! [`demod`] finds and slices messages; [`crc`] and [`frame`] decide which of
//! them may be believed, and on what grounds; [`message`] says what a believed
//! one contains; [`cpr`] turns the position fields into coordinates; [`track`]
//! folds everything into one entry per aircraft. `controller` wraps the lot in
//! a worker thread.
//!
//! # Provenance
//!
//! All of it is written from the published standards — ICAO Annex 10 Volume IV
//! (the downlink formats, the field layouts and the check sequence), RTCA
//! DO-260B Appendix A (the extended-squitter payloads) and ICAO Doc 9871
//! D.2.4.7 (the position algorithms) — and cited per module. It was
//! cross-checked against `dump1090`'s behaviour and against the frames the
//! standards publish, but no code came from it, which also keeps that project's
//! GPL-2.0 out of the tree.
//!
//! `rsadsb`'s `adsb_deku` was the intended dependency for the message layer and
//! turned out not to build: its last release is pinned by construction to a
//! version of `deku` that has since been yanked, and the one that replaced it
//! rejects the macro attributes `adsb_deku` uses. See this crate's `Cargo.toml`.
//!
//! # The receiver this needs
//!
//! Mode S is one megabit a second and every bit is two half-microsecond chips,
//! so what matters is how many samples land inside a chip. The stream has to be
//! at least [`sdroxide_types::ADSB_MIN_RATE_HZ`] and centred near
//! [`sdroxide_types::ADSB_FREQ_HZ`]; neither can be manufactured downstream,
//! because the engine's downconverter decimates and does not interpolate.
//!
//! Between that floor and [`sdroxide_types::ADSB_GOOD_RATE_HZ`] the decoder
//! runs but is *degraded*, and not through any shortcoming of the
//! implementation: at 2 Msps a chip and a sample are the same width, so a burst
//! arriving out of step with the sample clock has its chips split equally
//! between two samples that then read identically. Strong aircraft decode and
//! the rest are lost. Above the good rate every arrival phase decodes — see
//! [`demod`], which measures both.
//!
//! Where a receiver cannot do it at all, the answer is
//! [`sdroxide_types::AdsbStatus::unavailable`] — a sentence the operator can act
//! on — rather than a decoder that runs and finds nothing. Where it can run but
//! only just, it is
//! [`sdroxide_types::AdsbStatus::degraded`], for the same reason.

pub mod beast;
pub mod beast_net;
mod controller;
pub mod cpr;
pub mod crc;
pub mod demod;
pub mod frame;
pub mod message;
pub mod prior;
pub mod track;

pub use controller::{AdsbAction, AdsbController};
pub use demod::{Candidate, Demod, modulate, modulate_at};
pub use frame::{Accepted, Rejected, accept};
pub use message::{Body, Es, Message};
pub use track::Tracker;

/// Whether a stream of this rate, centred here, can carry the decoder.
///
/// The centre test is generous on purpose: the window only has to *contain*
/// 1090 MHz with a megahertz either side, and a wide front end tuned somewhere
/// else entirely may still cover it.
pub fn window_covers(center_hz: f64, rate_hz: f64) -> bool {
    if rate_hz < sdroxide_types::ADSB_MIN_RATE_HZ {
        return false;
    }
    let half = rate_hz / 2.0;
    let want = sdroxide_types::ADSB_FREQ_HZ;
    center_hz - half <= want && want <= center_hz + half
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two ways a receiver can fail to carry ADS-B are different problems
    /// with different fixes, and both have to be recognised.
    #[test]
    fn a_window_has_to_be_both_wide_enough_and_in_the_right_place() {
        assert!(window_covers(1_090_000_000.0, 2_400_000.0));
        assert!(!window_covers(1_090_000_000.0, 1_024_000.0), "too narrow to slice a chip");
        assert!(!window_covers(868_880_000.0, 2_400_000.0), "the right rate in the wrong place");
        // A wide front end parked elsewhere may still reach it.
        assert!(window_covers(1_080_000_000.0, 32_000_000.0));
    }
}
