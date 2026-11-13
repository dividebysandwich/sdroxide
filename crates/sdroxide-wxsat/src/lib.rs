//! Weather-satellite image decoders: the threading wrapper and the DSP behind
//! the WXSAT view. APT (NOAA-15/18/19, 137 MHz, FM with a 2400 Hz AM
//! subcarrier) lives in [`apt`]; Meteor LRPT (QPSK, digital image segments)
//! in [`lrpt`].

pub mod apt;
pub mod lane;
pub mod lrpt;
pub mod rs255;

pub use apt::{AptDecoder, AptLine, LINE_PIXELS};
pub use lane::{WxsatController, WxsatLane};

/// The rate of the complex baseband lane the engine feeds an APT decoder, Hz.
/// A NOAA signal is about 40 kHz wide (±17 kHz deviation plus the subcarrier).
pub const WXSAT_LANE_RATE_HZ: f64 = 48_000.0;

/// The lane rate of each decoder: 48 kS/s for APT, 216 kS/s (three samples per
/// symbol) for Meteor LRPT, whose signal is about 115 kHz wide.
pub fn lane_rate_hz(kind: sdroxide_types::WxsatKind) -> f64 {
    match kind {
        sdroxide_types::WxsatKind::Apt => WXSAT_LANE_RATE_HZ,
        sdroxide_types::WxsatKind::Lrpt => lrpt::demod::LRPT_LANE_RATE_HZ,
    }
}
