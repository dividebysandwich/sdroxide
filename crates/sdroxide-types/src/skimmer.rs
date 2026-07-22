//! Skimmer domain types, shared by the native engine, the wire protocol, and
//! the UI (native + WASM). Pure data + serde — the actual decoding lives in the
//! native `sdroxide-skimmer` crate. Designed to be skimmer-kind-agnostic so
//! future skimmers (RTTY/PSK/…) reuse the same event, wire, and overlay path.

use serde::{Deserialize, Serialize};

/// What kind of skimmer produced a spot. The wire event, UI overlay, and
/// engine seam are generic over this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkimmerKind {
    Cw,
    Psk,
    Rtty,
}

impl SkimmerKind {
    /// The operating mode a spot of this kind tunes to on click.
    pub fn mode(self) -> crate::Mode {
        match self {
            SkimmerKind::Cw => crate::Mode::Cw,
            SkimmerKind::Psk => crate::Mode::Psk,
            SkimmerKind::Rtty => crate::Mode::Rtty,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SkimmerKind::Cw => "CW",
            SkimmerKind::Psk => "PSK",
            SkimmerKind::Rtty => "RTTY",
        }
    }
}

/// One decoded signal from a skimmer: a station heard at a frequency, with a
/// (possibly not-yet-known) callsign and a rolling tail of decoded text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkimmerSpot {
    /// Stable id for this track, so the UI can update a box in place (and keep
    /// its message scrolling) rather than recreating it each update.
    pub id: u64,
    pub kind: SkimmerKind,
    /// Absolute RF frequency of the signal (Hz).
    pub freq_hz: f64,
    /// Best-guess callsign extracted from the decoded text, if any.
    pub callsign: Option<String>,
    /// Rolling tail of decoded text (most recent characters).
    pub text: String,
    /// Signal-to-noise estimate (dB).
    pub snr_db: i16,
    /// Estimated speed in words per minute.
    pub wpm: u16,
    /// True while the signal is currently keying (recently active).
    pub active: bool,
}
