//! One decoded telemetry package from a `*mod` JSON line.

use sdroxide_types::SondeKind;

/// Fields common enough to fold into [`sdroxide_types::Rs41Sonde`].
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub serial: String,
    pub kind: SondeKind,
    pub subtype: Option<String>,
    pub manufacturer: String,
    pub frame: Option<u32>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub altitude_m: Option<f32>,
    pub ascent_rate_mps: Option<f32>,
    pub vel_h_mps: Option<f32>,
    pub heading_deg: Option<f32>,
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
    pub battery_v: Option<f32>,
    pub sats: Option<u8>,
    pub datetime: Option<String>,
}
