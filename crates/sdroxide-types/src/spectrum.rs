use serde::{Deserialize, Serialize};

/// One panadapter frame. `bins` are magnitudes mapped to u8 over
/// `[db_floor, db_ceil]`, ordered from `center_hz - span_hz/2` upward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrumFrame {
    pub seq: u32,
    pub center_hz: f64,
    pub span_hz: f64,
    pub db_floor: f32,
    pub db_ceil: f32,
    pub bins: Vec<u8>,
}

impl SpectrumFrame {
    pub fn freq_at_bin(&self, bin: usize) -> f64 {
        let n = self.bins.len().max(1) as f64;
        self.center_hz - self.span_hz / 2.0 + (bin as f64 + 0.5) / n * self.span_hz
    }
}

/// Client-requested spectrum generation parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpectrumConfig {
    pub fft_size: u32,
    pub fps: u8,
    /// Exponential averaging time constant in seconds. 0 disables averaging.
    pub avg_tc: f32,
    /// u8 mapping range for emitted frames.
    pub db_floor: f32,
    pub db_ceil: f32,
    /// Visible sub-span (lo_hz, hi_hz); `None` = full device passband.
    pub viewport: Option<(f64, f64)>,
}

impl Default for SpectrumConfig {
    fn default() -> Self {
        SpectrumConfig {
            fft_size: 4096,
            fps: 30,
            avg_tc: 0.2,
            db_floor: -120.0,
            db_ceil: -20.0,
            viewport: None,
        }
    }
}
