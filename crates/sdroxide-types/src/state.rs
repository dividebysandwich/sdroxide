use serde::{Deserialize, Serialize};

use crate::{AgcMode, Band, Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Vfo {
    A,
    B,
}

/// Receiver slot: the main receiver or the sub receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RxId {
    Main,
    Sub,
}

impl RxId {
    pub fn index(self) -> usize {
        match self {
            RxId::Main => 0,
            RxId::Sub => 1,
        }
    }
}

/// RIT/XIT style offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OffsetState {
    pub enabled: bool,
    pub hz: i32,
}

impl OffsetState {
    pub fn effective_hz(self) -> f64 {
        if self.enabled { self.hz as f64 } else { 0.0 }
    }
}

/// Squelch fully open (slider minimum).
pub const SQUELCH_OPEN_DB: f32 = -150.0;

/// Per-receiver settings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RxState {
    pub mode: Mode,
    /// Passband edges in Hz relative to the VFO frequency.
    pub filter_lo: f32,
    pub filter_hi: f32,
    pub agc: AgcMode,
    pub agc_max_gain_db: f32,
    /// 0.0..=1.0
    pub volume: f32,
    pub muted: bool,
    /// Audio gates closed below this post-filter power (dBFS).
    /// [`SQUELCH_OPEN_DB`] = always open.
    pub squelch_db: f32,
}

impl RxState {
    pub fn with_mode(mode: Mode) -> Self {
        let (filter_lo, filter_hi) = mode.default_filter();
        RxState {
            mode,
            filter_lo,
            filter_hi,
            agc: AgcMode::Med,
            agc_max_gain_db: 90.0,
            volume: 0.5,
            muted: false,
            squelch_db: SQUELCH_OPEN_DB,
        }
    }
}

/// Transmit-side settings and status.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TxState {
    pub ptt: bool,
    pub tune: bool,
    /// 0.0..=1.0 fraction of maximum drive.
    pub drive: f32,
    /// Drive used while `tune` is active.
    pub tune_drive: f32,
    /// 0.0..=1.0
    pub mic_gain: f32,
}

/// Complete radio state snapshot. Kept small (~300 bytes serialized) so full
/// snapshots — never deltas — travel on every change, latest-wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadioState {
    pub vfo_a_hz: f64,
    pub vfo_b_hz: f64,
    pub active_vfo: Vfo,
    pub split: bool,

    /// SDR hardware center frequency.
    pub center_hz: f64,
    /// SDR hardware sample rate.
    pub sample_rate: f64,

    /// Indexed by [`RxId::index`]: main, sub.
    pub rx: [RxState; 2],
    pub sub_rx_enabled: bool,

    pub rit: OffsetState,
    pub xit: OffsetState,

    pub tx: TxState,
    pub band: Band,
    /// Impulse noise blanker on the raw IQ stream.
    pub noise_blanker: bool,

    /// SoapySDR RX gain elements: (name, dB).
    pub gains: Vec<(String, f64)>,
    /// SoapySDR TX gain elements: (name, dB). Default all-minimum (safety).
    pub tx_gains: Vec<(String, f64)>,
    pub antenna_rx: String,
    pub antenna_tx: String,
}

impl Default for RadioState {
    fn default() -> Self {
        let (freq, mode) = Band::M20.default_entry();
        RadioState {
            vfo_a_hz: freq,
            vfo_b_hz: freq,
            active_vfo: Vfo::A,
            split: false,
            center_hz: freq,
            sample_rate: 1_536_000.0,
            rx: [RxState::with_mode(mode), RxState::with_mode(mode)],
            sub_rx_enabled: false,
            rit: OffsetState::default(),
            xit: OffsetState::default(),
            // Low drive defaults: digital amplitude stays far from full
            // scale until the operator raises it deliberately.
            tx: TxState {
                drive: 0.1,
                tune_drive: 0.05,
                mic_gain: 0.5,
                ..TxState::default()
            },
            band: Band::M20,
            noise_blanker: false,
            gains: Vec::new(),
            tx_gains: Vec::new(),
            antenna_rx: String::new(),
            antenna_tx: String::new(),
        }
    }
}

impl RadioState {
    /// Frequency of the currently active VFO.
    pub fn active_freq_hz(&self) -> f64 {
        match self.active_vfo {
            Vfo::A => self.vfo_a_hz,
            Vfo::B => self.vfo_b_hz,
        }
    }

    /// Receive frequency including RIT.
    pub fn rx_freq_hz(&self) -> f64 {
        self.active_freq_hz() + self.rit.effective_hz()
    }

    /// Transmit frequency including XIT and split.
    pub fn tx_freq_hz(&self) -> f64 {
        let base = if self.split {
            match self.active_vfo {
                Vfo::A => self.vfo_b_hz,
                Vfo::B => self.vfo_a_hz,
            }
        } else {
            self.active_freq_hz()
        };
        base + self.xit.effective_hz()
    }
}
