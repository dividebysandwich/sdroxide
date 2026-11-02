//! VARA HF vocabulary shared by the engine, the wire protocol and the panel.
//!
//! **Interface facts only.** The modem itself is the vendored xara core behind
//! `sdroxide-vara`, which is native-only; what is here is what the panel and the
//! wire need to show a link and drive it, so this crate stays wasm-clean.

use serde::{Deserialize, Serialize};

/// Transcript lines kept for the panel.
pub const VARA_TRANSCRIPT_MAX: usize = 500;

/// The occupied bandwidth a VARA HF link runs at. All three are centred on
/// 1500 Hz of audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VaraBw {
    /// VARA HF "narrow", for crowded bands and weak paths.
    Hz500,
    /// The standard width, and what most RMS gateways run.
    #[default]
    Hz2300,
    /// The wide setting VARA calls "Tactical".
    Hz2750,
}

impl VaraBw {
    pub const ALL: [VaraBw; 3] = [VaraBw::Hz500, VaraBw::Hz2300, VaraBw::Hz2750];

    pub fn hz(self) -> u32 {
        match self {
            VaraBw::Hz500 => 500,
            VaraBw::Hz2300 => 2300,
            VaraBw::Hz2750 => 2750,
        }
    }

    /// The audio passband, (low, high) Hz.
    pub fn passband(self) -> (f32, f32) {
        let half = self.hz() as f32 / 2.0;
        (1500.0 - half, 1500.0 + half)
    }

    pub fn label(self) -> &'static str {
        match self {
            VaraBw::Hz500 => "500",
            VaraBw::Hz2300 => "2300",
            VaraBw::Hz2750 => "2750",
        }
    }
}

/// Where a transcript line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaraLineKind {
    /// Payload from the far station.
    Rx,
    /// Payload we sent.
    Tx,
    /// A modem event: connected, disconnected, speed change.
    Event,
    /// A host command or reply, shown when the host is an external program.
    Host,
    /// Something went wrong.
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaraLine {
    pub at_unix: i64,
    pub kind: VaraLineKind,
    pub text: String,
}

/// Who is driving the VARA modem. One at a time.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VaraHolder {
    #[default]
    Nobody,
    /// The operator's own panel.
    Panel,
    /// The MAIL window's forwarding session.
    Winlink,
    /// A program on the TCP host ports, by its address.
    Tcp(String),
}

/// VARA HF's live state, for the panel and the wire.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VaraStatus {
    /// The session state as the modem names it — `Listen`, `Calling`,
    /// `Sending`, `Receiving`, `Idle` and so on.
    pub state: String,
    /// True while a link is up.
    pub connected: bool,
    /// The far station, while a link is up or being called.
    pub peer: String,
    /// The bandwidth the modem is set to.
    pub bw: VaraBw,
    /// The speed level last reported, 1–17, and its net bit rate.
    pub level: u8,
    pub bps: u32,
    /// Signal-to-noise of the far station, dB, as last reported.
    pub snr_db: Option<f32>,
    /// The channel is occupied (the busy detector), so a call would collide.
    pub busy: bool,
    /// The modem wants the transmitter keyed.
    pub ptt: bool,
    /// A tune tone is going out.
    pub tuning: bool,
    /// Bytes queued for sending.
    pub buffered: u32,
    /// The receiver's frequency correction, Hz.
    pub afc_hz: Option<f32>,
    /// Connect attempts made on the current call.
    pub attempts: u32,
    /// Who holds the modem.
    pub holder: VaraHolder,
    /// Where the TCP host ports are listening, when they are — `127.0.0.1:8300`.
    pub host_listening: Option<String>,
    /// Why the TCP host ports are not listening, when they should be.
    pub host_error: Option<String>,
    /// The panel listens for incoming calls.
    pub listening: bool,
    /// Bytes moved on the current link, each way.
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub transcript: Vec<VaraLine>,
}
