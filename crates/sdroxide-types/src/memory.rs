use serde::{Deserialize, Serialize};

use crate::Mode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryChannel {
    pub id: u32,
    pub name: String,
    pub freq_hz: f64,
    pub mode: Mode,
    pub filter_lo: f32,
    pub filter_hi: f32,
}

/// One entry of a band-stack register (PowerSDR-style: up to 3 per band,
/// pressing the band button again cycles them).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BandStackEntry {
    pub freq_hz: f64,
    pub mode: Mode,
    pub filter_lo: f32,
    pub filter_hi: f32,
}
