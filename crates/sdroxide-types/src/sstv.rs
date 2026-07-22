//! SSTV (slow-scan TV) sub-mode vocabulary shared by the engine, the wire
//! protocol, and the UI. The concrete tone timing lives in the native DSP crate;
//! this module only carries the identity, dimensions, and VIS codes so the UI
//! (native + wasm) can label modes and pick TX image sizes.

use serde::{Deserialize, Serialize};

/// One SSTV transmission mode. `Mode::Sstv` is the radio mode; this picks the
/// specific line format used for encode/decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SstvMode {
    Scottie1,
    Scottie2,
    ScottieDx,
    Martin1,
    Martin2,
    Robot72,
    Robot36,
}

impl Default for SstvMode {
    fn default() -> Self {
        SstvMode::Scottie1
    }
}

impl SstvMode {
    /// All modes, in a sensible menu order.
    pub const ALL: [SstvMode; 7] = [
        SstvMode::Scottie1,
        SstvMode::Scottie2,
        SstvMode::ScottieDx,
        SstvMode::Martin1,
        SstvMode::Martin2,
        SstvMode::Robot72,
        SstvMode::Robot36,
    ];

    /// Short human label for buttons/menus.
    pub fn label(self) -> &'static str {
        match self {
            SstvMode::Scottie1 => "Scottie 1",
            SstvMode::Scottie2 => "Scottie 2",
            SstvMode::ScottieDx => "Scottie DX",
            SstvMode::Martin1 => "Martin 1",
            SstvMode::Martin2 => "Martin 2",
            SstvMode::Robot72 => "Robot 72",
            SstvMode::Robot36 => "Robot 36",
        }
    }

    /// Transmitted image size in pixels, `(width, height)`.
    pub fn dimensions(self) -> (u16, u16) {
        match self {
            // Scottie/Martin are 320×256.
            SstvMode::Scottie1
            | SstvMode::Scottie2
            | SstvMode::ScottieDx
            | SstvMode::Martin1
            | SstvMode::Martin2 => (320, 256),
            // Robot modes are 320×240.
            SstvMode::Robot72 | SstvMode::Robot36 => (320, 240),
        }
    }

    /// The 7-bit VIS code identifying this mode in the calibration header.
    pub fn vis_code(self) -> u8 {
        match self {
            SstvMode::Robot36 => 8,
            SstvMode::Robot72 => 12,
            SstvMode::Martin2 => 40,
            SstvMode::Martin1 => 44,
            SstvMode::Scottie2 => 56,
            SstvMode::Scottie1 => 60,
            SstvMode::ScottieDx => 76,
        }
    }

    /// Map a decoded VIS code back to a mode, if recognised.
    pub fn from_vis(code: u8) -> Option<SstvMode> {
        SstvMode::ALL.into_iter().find(|m| m.vis_code() == code)
    }
}

/// Broadcast status for the SSTV panel: what's being sent/received and how far
/// along. Rides the wire as part of the digital-mode event stream.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SstvStatus {
    /// The mode selected for the next transmission.
    pub tx_mode: SstvMode,
    /// True while an image is being transmitted.
    pub tx_active: bool,
    /// True while a signal is being received/decoded.
    pub rx_active: bool,
    /// The mode detected from the incoming VIS header, if any.
    pub detected: Option<SstvMode>,
    /// Fraction of the current image completed, 0.0..=1.0 (RX while receiving,
    /// TX while transmitting).
    pub progress: f32,
    /// Smoothed in-band receive signal level (~0..1), for an activity meter so
    /// the operator can confirm audio is reaching the decoder.
    pub signal: f32,
}

impl Default for SstvStatus {
    fn default() -> Self {
        SstvStatus {
            tx_mode: SstvMode::default(),
            tx_active: false,
            rx_active: false,
            detected: None,
            progress: 0.0,
            signal: 0.0,
        }
    }
}
