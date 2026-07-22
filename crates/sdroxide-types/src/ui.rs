//! UI / display preferences (persisted in `config.toml` under `[ui]`), plus the
//! coarse speed enum shared by the waterfall-scroll and spectrum-averaging
//! settings. Kept wasm-safe (no I/O) so the egui client can use it directly.

use serde::{Deserialize, Serialize};

/// Coarse speed setting for the waterfall scroll and the spectrum line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speed {
    Slow,
    Medium,
    Fast,
}

impl Speed {
    pub const ALL: [Speed; 3] = [Speed::Slow, Speed::Medium, Speed::Fast];

    pub fn label(self) -> &'static str {
        match self {
            Speed::Slow => "Slow",
            Speed::Medium => "Medium",
            Speed::Fast => "Fast",
        }
    }
}

/// User display preferences. All have defaults so a missing `[ui]` table loads.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// GUI repaint + spectrum frame rate, in frames per second.
    pub frame_rate_fps: u32,
    /// How fast the waterfall scrolls.
    pub waterfall_speed: Speed,
    /// How fast the spectrum line reacts (averaging; slower = smoother).
    pub spectrum_speed: Speed,
}

impl Default for UiSettings {
    fn default() -> Self {
        UiSettings {
            frame_rate_fps: 60,
            waterfall_speed: Speed::Medium,
            spectrum_speed: Speed::Medium,
        }
    }
}

impl UiSettings {
    /// Selectable frame rates for the UI combo.
    pub const FPS_OPTIONS: [u32; 3] = [30, 60, 90];

    /// Frame rate clamped to a sane range (guards a hand-edited config).
    pub fn fps(self) -> u32 {
        self.frame_rate_fps.clamp(10, 240)
    }

    /// Waterfall scroll rate in rows per second. Absolute (independent of the
    /// frame rate) so the time axis — and the 60-second gridlines — stay stable
    /// when the frame rate changes.
    pub fn waterfall_rows_per_sec(self) -> f32 {
        match self.waterfall_speed {
            Speed::Slow => 5.0,
            Speed::Medium => 12.0,
            Speed::Fast => 28.0,
        }
    }

    /// Exponential averaging time constant (seconds) for the spectrum line.
    /// Fast disables averaging (snappy); slower values smooth it out.
    pub fn spectrum_avg_tc(self) -> f32 {
        match self.spectrum_speed {
            Speed::Fast => 0.0,
            Speed::Medium => 0.1,
            Speed::Slow => 0.2,
        }
    }
}
