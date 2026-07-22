//! Persisted radio-backend configuration (`radio.json`): choose between a
//! SoapySDR device and a CAT-controlled rig whose audio arrives over a USB
//! sound card. Serde-only — no I/O, safe in the wasm client (the settings UI
//! is shared, even though the CAT machinery is native-only).

use serde::{Deserialize, Serialize};

/// Which radio backend to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Backend {
    /// Legacy "SoapySDR if present, else CAT" auto-detect. No longer offered in
    /// the UI, but kept so older `radio.json` files still deserialize.
    Auto,
    #[default]
    Soapy,
    Cat,
    /// OpenHPSDR ethernet SDR (Protocol 2), discovered/reached over the LAN.
    Hpsdr,
}

impl Backend {
    pub const ALL: [Backend; 4] = [Backend::Auto, Backend::Soapy, Backend::Cat, Backend::Hpsdr];
    pub fn label(self) -> &'static str {
        match self {
            Backend::Auto => "Auto-detect (SoapySDR / CAT)",
            Backend::Soapy => "SoapySDR",
            Backend::Cat => "CAT / Audio",
            Backend::Hpsdr => "HPSDR (network)",
        }
    }
}

/// CAT protocol family. Only `Xiegu` is hardware-verified so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CatFamily {
    #[default]
    Xiegu,
    Icom,
    Yaesu,
}

impl CatFamily {
    pub const ALL: [CatFamily; 3] = [CatFamily::Xiegu, CatFamily::Icom, CatFamily::Yaesu];
    pub fn label(self) -> &'static str {
        match self {
            CatFamily::Xiegu => "Xiegu",
            CatFamily::Icom => "Icom",
            CatFamily::Yaesu => "Yaesu",
        }
    }
}

/// How the radio's audio is carried over the sound card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SoundFormat {
    /// Stereo L=I, R=Q complex baseband → normal wideband engine path.
    Iq,
    /// Mono already-demodulated audio → audio-band panadapter (engine bypass).
    #[default]
    DemodAudio,
}

impl SoundFormat {
    pub const ALL: [SoundFormat; 2] = [SoundFormat::DemodAudio, SoundFormat::Iq];
    pub fn label(self) -> &'static str {
        match self {
            SoundFormat::Iq => "IQ (stereo)",
            SoundFormat::DemodAudio => "Demod audio",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Parity {
    #[default]
    None,
    Even,
    Odd,
}

impl Parity {
    pub const ALL: [Parity; 3] = [Parity::None, Parity::Even, Parity::Odd];
    pub fn label(self) -> &'static str {
        match self {
            Parity::None => "None",
            Parity::Even => "Even",
            Parity::Odd => "Odd",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StopBits {
    #[default]
    One,
    Two,
}

impl StopBits {
    pub const ALL: [StopBits; 2] = [StopBits::One, StopBits::Two];
    pub fn label(self) -> &'static str {
        match self {
            StopBits::One => "1",
            StopBits::Two => "2",
        }
    }
}

/// A serial control line forced to a fixed level while the port is open (some
/// rigs need DTR/RTS held high to enable CAT). `None` = leave as-is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineState {
    #[default]
    None,
    High,
    Low,
}

impl LineState {
    pub const ALL: [LineState; 3] = [LineState::None, LineState::High, LineState::Low];
    pub fn label(self) -> &'static str {
        match self {
            LineState::None => "None",
            LineState::High => "High",
            LineState::Low => "Low",
        }
    }
}

/// How to key the transmitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PttMethod {
    /// Rig keys itself from TX audio; software just routes audio.
    Vox,
    Dtr,
    Rts,
    /// A CAT command keys the rig.
    #[default]
    Cat,
}

impl PttMethod {
    pub const ALL: [PttMethod; 4] = [PttMethod::Cat, PttMethod::Dtr, PttMethod::Rts, PttMethod::Vox];
    pub fn label(self) -> &'static str {
        match self {
            PttMethod::Vox => "VOX",
            PttMethod::Dtr => "DTR",
            PttMethod::Rts => "RTS",
            PttMethod::Cat => "CAT",
        }
    }
}

/// Who drives the rig's mode for ordinary modes (USB/LSB/CW/AM/FM/DIGU/DIGL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ModeControl {
    /// The app commands the rig's mode over CAT to match the selected mode.
    #[default]
    Cat,
    /// The operator sets the mode on the radio; the app just follows it.
    Radio,
}

impl ModeControl {
    pub const ALL: [ModeControl; 2] = [ModeControl::Cat, ModeControl::Radio];
    pub fn label(self) -> &'static str {
        match self {
            ModeControl::Cat => "CAT",
            ModeControl::Radio => "Radio controlled",
        }
    }
}

/// What mode the rig should be in for the FT8/FT4 digital engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DigiMode {
    /// Force the rig to USB.
    #[default]
    Usb,
    /// Force the rig to its DATA/PKT (USB-D) mode.
    Data,
    /// Leave the rig's mode as the operator set it.
    Radio,
}

impl DigiMode {
    pub const ALL: [DigiMode; 3] = [DigiMode::Usb, DigiMode::Data, DigiMode::Radio];
    pub fn label(self) -> &'static str {
        match self {
            DigiMode::Usb => "USB",
            DigiMode::Data => "DIGI",
            DigiMode::Radio => "Radio controlled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SerialConfig {
    /// Serial device path (Linux/mac `/dev/tty…`, Windows `COMx`).
    pub path: String,
    pub baud: u32,
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub force_rts: LineState,
    pub force_dtr: LineState,
}

impl Default for SerialConfig {
    fn default() -> Self {
        SerialConfig {
            path: String::new(),
            baud: 19200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            force_rts: LineState::None,
            force_dtr: LineState::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatConfig {
    pub family: CatFamily,
    pub serial: SerialConfig,
    pub ptt: PttMethod,
    /// How often to poll the rig for its dial/mode (Hz).
    pub poll_hz: f32,
    /// Who controls the rig's mode for ordinary modes.
    pub mode_control: ModeControl,
    /// What mode the rig uses for the FT8/FT4 engine.
    pub digi_mode: DigiMode,
    /// Icom CI-V transceiver address (hex byte), e.g. 0x70 for many rigs.
    pub icom_radio_id: u8,
    pub format: SoundFormat,
    /// Displayed panadapter bandwidth for demod-audio mode (Hz).
    pub audio_bw_hz: f64,
}

impl Default for CatConfig {
    fn default() -> Self {
        CatConfig {
            family: CatFamily::default(),
            serial: SerialConfig::default(),
            ptt: PttMethod::default(),
            poll_hz: 5.0,
            mode_control: ModeControl::default(),
            digi_mode: DigiMode::default(),
            icom_radio_id: 0x70,
            format: SoundFormat::default(),
            audio_bw_hz: 4000.0,
        }
    }
}

/// OpenHPSDR (ethernet SDR) backend configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HpsdrConfig {
    /// Explicit target IP (e.g. "192.168.1.50"). When set, connect directly and
    /// skip discovery/selection.
    pub manual_ip: Option<String>,
    /// IP of the device picked from a discovery scan (persisted selection).
    pub selected_ip: Option<String>,
    /// DDC sample rate in Hz (48k, 96k, 192k, 384k, 768k, 1536k).
    pub sample_rate_hz: f64,
}

impl Default for HpsdrConfig {
    fn default() -> Self {
        HpsdrConfig { manual_ip: None, selected_ip: None, sample_rate_hz: 1_536_000.0 }
    }
}

impl HpsdrConfig {
    /// Supported DDC sample rates (Hz) for Protocol 2 boards.
    pub const SAMPLE_RATES: [f64; 6] =
        [48_000.0, 96_000.0, 192_000.0, 384_000.0, 768_000.0, 1_536_000.0];

    /// Protocol 1 (Metis) boards top out at 384 kHz.
    pub const P1_SAMPLE_RATES: [f64; 4] = [48_000.0, 96_000.0, 192_000.0, 384_000.0];

    /// The sample rates valid for a given protocol (1 or 2).
    pub fn rates_for(protocol: u8) -> &'static [f64] {
        if protocol == 1 {
            &Self::P1_SAMPLE_RATES
        } else {
            &Self::SAMPLE_RATES
        }
    }

    /// Resolve the IP to connect to: manual override, else the persisted pick.
    /// `None` means "discover and use the first responder".
    pub fn target_ip(&self) -> Option<&str> {
        self.manual_ip
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(self.selected_ip.as_deref())
    }
}

/// One HPSDR device found by a discovery scan. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HpsdrDevice {
    pub ip: String,
    pub mac: String,
    /// Board name, e.g. "Hermes", "Saturn", "Hermes-Lite 2".
    pub board: String,
    /// OpenHPSDR protocol the board speaks (1 or 2).
    pub protocol: u8,
    /// Whether the board reports it is already in use by another host.
    pub in_use: bool,
}

impl HpsdrDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        let mut s = format!("{}  {}  (P{})", self.board, self.ip, self.protocol);
        if self.in_use {
            s.push_str("  [in use]");
        }
        if !self.supported() {
            s.push_str("  [unsupported protocol]");
        }
        s
    }

    /// Whether this device can be driven by the current implementation
    /// (Protocol 1 and Protocol 2 are both supported).
    pub fn supported(&self) -> bool {
        matches!(self.protocol, 1 | 2)
    }
}

/// Persisted backend configuration (`radio.json`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioConfig {
    pub backend: Backend,
    /// Sound-card device (cpal name) carrying the radio's RX audio → PC.
    pub radio_audio_in: Option<String>,
    /// Sound-card device (cpal name) carrying the TX audio PC → radio.
    pub radio_audio_out: Option<String>,
    pub cat: CatConfig,
    pub hpsdr: HpsdrConfig,
}
