use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Rx,
    Tx,
}

/// One adjustable gain stage exposed by the device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GainElement {
    pub name: String,
    pub direction: Direction,
    pub min_db: f64,
    pub max_db: f64,
    pub step_db: f64,
}

/// Device capabilities probed once at open time. Drives all UI adaptation
/// (e.g. `tx_channels == 0` hides every TX control).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DeviceCaps {
    pub driver: String,
    pub label: String,

    pub rx_channels: usize,
    pub tx_channels: usize,
    /// Whether RX keeps running during TX. Conservative default: false.
    pub full_duplex: bool,
    /// The source delivers already-demodulated real audio (a CAT rig on a
    /// sound card), so the engine bypasses the DDC/demod chain and shows a
    /// narrow audio-band panadapter. Sound-card *IQ* leaves this `false` and
    /// runs the normal wideband path.
    pub audio_mode: bool,

    /// Tunable ranges in Hz: (min, max).
    pub freq_ranges_rx: Vec<(f64, f64)>,
    pub freq_ranges_tx: Vec<(f64, f64)>,

    /// Discrete supported rates, if the device reports any.
    pub sample_rates: Vec<f64>,
    /// Continuous rate ranges: (min, max).
    pub rate_ranges: Vec<(f64, f64)>,

    pub gains: Vec<GainElement>,
    pub antennas_rx: Vec<String>,
    pub antennas_tx: Vec<String>,

    /// Sensor names from the SoapySDR sensor API (device- and channel-level).
    pub sensors: Vec<String>,
    pub has_swr_sensor: bool,
    pub has_fwd_power_sensor: bool,
}

impl DeviceCaps {
    pub fn is_transmit_capable(&self) -> bool {
        self.tx_channels > 0
    }

    pub fn can_rx_hz(&self, hz: f64) -> bool {
        self.freq_ranges_rx.iter().any(|&(lo, hi)| hz >= lo && hz <= hi)
    }

    pub fn can_tx_hz(&self, hz: f64) -> bool {
        self.freq_ranges_tx.iter().any(|&(lo, hi)| hz >= lo && hz <= hi)
    }
}
