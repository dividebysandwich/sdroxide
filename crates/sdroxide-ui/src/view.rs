//! Client-local display state (not part of the shared radio state).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewState {
    /// Visible frequency window; 0/0 means "fit the full device span".
    pub view_lo_hz: f64,
    pub view_hi_hz: f64,
    pub db_floor: f32,
    pub db_ceil: f32,
    pub fft_size: u32,
    /// Fraction of the panadapter height used by the spectrum line (rest = waterfall).
    pub spectrum_fraction: f32,
    /// Draw a decaying peak-hold trace over the spectrum.
    pub peak_hold: bool,
    /// Hide the spectrum line, showing only the waterfall (and, in FT8/FT4,
    /// giving the freed height to the operating panel).
    pub spectrum_collapsed: bool,
    /// Fraction of the FT8/FT4 layout height used by the operating panel (the
    /// decode list + QSO area); the rest is the waterfall. User-draggable.
    pub digi_panel_fraction: f32,
    /// Fraction of the FT8/FT4 panel width used by the decode table; the rest is
    /// the map/QSO area. User-draggable.
    pub digi_split_fraction: f32,
    /// Fraction of the QSO area's height given to the world map; the rest is the
    /// station card + transcript + buttons. User-draggable.
    pub digi_map_fraction: f32,
    /// Render the S-meter as an analog needle instrument instead of the bar.
    /// Toggled by clicking the meter.
    pub smeter_analog: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            view_lo_hz: 0.0,
            view_hi_hz: 0.0,
            db_floor: -120.0,
            db_ceil: -20.0,
            fft_size: 4096,
            spectrum_fraction: 0.35,
            peak_hold: false,
            spectrum_collapsed: false,
            digi_panel_fraction: 0.46,
            digi_split_fraction: 0.52,
            digi_map_fraction: 0.6,
            smeter_analog: false,
        }
    }
}

impl ViewState {
    /// Effective spectrum-height fraction (0 when collapsed).
    pub fn effective_spectrum_fraction(&self) -> f32 {
        if self.spectrum_collapsed { 0.0 } else { self.spectrum_fraction }
    }

    pub fn span(&self) -> f64 {
        self.view_hi_hz - self.view_lo_hz
    }

    pub fn is_unset(&self) -> bool {
        self.span() <= 0.0
    }

    /// Reset to show the whole device passband.
    pub fn fit(&mut self, center_hz: f64, span_hz: f64) {
        self.view_lo_hz = center_hz - span_hz / 2.0;
        self.view_hi_hz = center_hz + span_hz / 2.0;
    }

    /// Clamp the viewport inside the device passband, preserving width.
    pub fn clamp_to(&mut self, center_hz: f64, span_hz: f64) {
        let (lo, hi) = (center_hz - span_hz / 2.0, center_hz + span_hz / 2.0);
        let w = self.span().min(span_hz).max(span_hz / 1000.0);
        if self.view_lo_hz < lo {
            self.view_lo_hz = lo;
            self.view_hi_hz = lo + w;
        }
        if self.view_hi_hz > hi {
            self.view_hi_hz = hi;
            self.view_lo_hz = hi - w;
        }
    }

    pub fn freq_to_x(&self, hz: f64, rect: &eframe::egui::Rect) -> f32 {
        let frac = (hz - self.view_lo_hz) / self.span();
        rect.left() + rect.width() * frac as f32
    }

    pub fn x_to_freq(&self, x: f32, rect: &eframe::egui::Rect) -> f64 {
        let frac = ((x - rect.left()) / rect.width()) as f64;
        self.view_lo_hz + frac * self.span()
    }
}
