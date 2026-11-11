//! The TNC modems' vocabulary (Mercury, Data2G) shared by the engine, the wire
//! protocol and the panel.
//!
//! **Interface facts only.** The modems themselves are native libraries behind
//! `sdroxide-mercury` and `sdroxide-data2g`; what is here is what the panel and
//! the wire need to show a link and drive it, so this crate stays wasm-clean.

use serde::{Deserialize, Serialize};

use crate::Mode;

/// Transcript lines kept for the panel.
pub const TNC_TRANSCRIPT_MAX: usize = 500;

/// The width a TNC modem may use, as the `BW` host command names it. Mercury
/// offers 500 and 2300; Data2G 500, 1200 and 2300 (its 2400 Hz band).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TncBw {
    Hz500,
    Hz1200,
    #[default]
    Hz2300,
}

impl TncBw {
    /// The widths `mode` offers, narrowest first.
    pub fn offered(mode: Mode) -> &'static [TncBw] {
        match mode {
            Mode::Data2g => &[TncBw::Hz500, TncBw::Hz1200, TncBw::Hz2300],
            _ => &[TncBw::Hz500, TncBw::Hz2300],
        }
    }

    pub fn hz(self) -> u32 {
        match self {
            TncBw::Hz500 => 500,
            TncBw::Hz1200 => 1200,
            TncBw::Hz2300 => 2300,
        }
    }

    /// The host command that sets it.
    pub fn command(self) -> String {
        format!("BW{}", self.hz())
    }

    /// The audio the signal occupies at this width, (low, high) Hz, with a
    /// little room. Mercury centres on 1500 Hz (its MFSK floor mode spans
    /// 1015–1984 Hz even at 500); Data2G on 1525 Hz (`config.py` BANDS).
    pub fn passband(self, mode: Mode) -> (f32, f32) {
        match (mode, self) {
            (Mode::Data2g, TncBw::Hz500) => (1200.0, 1850.0),
            (Mode::Data2g, TncBw::Hz1200) => (850.0, 2200.0),
            (Mode::Data2g, TncBw::Hz2300) => (300.0, 2750.0),
            (_, TncBw::Hz500) => (950.0, 2050.0),
            (_, _) => (300.0, 2700.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TncBw::Hz500 => "500",
            TncBw::Hz1200 => "1200",
            TncBw::Hz2300 => "2300",
        }
    }
}

/// Where a transcript line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TncLineKind {
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
pub struct TncLine {
    pub at_unix: i64,
    pub kind: TncLineKind,
    pub text: String,
}

/// Who is driving the modem. One at a time.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TncHolder {
    #[default]
    Nobody,
    /// The operator's own panel.
    Panel,
    /// The MAIL window's forwarding session.
    Winlink,
    /// A program on the TCP host ports, by its address.
    Tcp(String),
}

/// A TNC modem's live state, for the panel and the wire.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TncStatus {
    /// The modem and its release, e.g. `Data2G 0.4.3 (protocol 15)`.
    pub version: String,
    /// Why the modem is not running, when it is not (a failed start, a build
    /// without it).
    pub unavailable: Option<String>,
    /// True while a link is up.
    pub connected: bool,
    /// An incoming call is being answered.
    pub pending: bool,
    /// The far station, while a link is up or being called.
    pub peer: String,
    /// The width the modem is set to.
    pub bw: TncBw,
    /// The speed the modem last reported: Mercury's level and net bit rate,
    /// Data2G's submode name.
    pub level: u8,
    pub bps: u32,
    pub submode: String,
    /// Signal-to-noise of the far station, dB, as last reported.
    pub snr_db: Option<f32>,
    /// The channel is occupied, so a call would collide.
    pub busy: bool,
    /// The modem wants the transmitter keyed.
    pub ptt: bool,
    /// A tune tone is going out.
    pub tuning: bool,
    /// Bytes queued for sending.
    pub buffered: u32,
    /// Who holds the modem.
    pub holder: TncHolder,
    /// Where the TCP host ports are listening, when they are — `127.0.0.1:8300`.
    pub host_listening: Option<String>,
    /// Why the TCP host ports are not listening, when they should be.
    pub host_error: Option<String>,
    /// The panel listens for incoming calls.
    pub listening: bool,
    /// Bytes moved on the current link, each way.
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub transcript: Vec<TncLine>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_modem_offers_its_own_widths() {
        assert_eq!(TncBw::offered(Mode::Mercury), &[TncBw::Hz500, TncBw::Hz2300]);
        assert_eq!(TncBw::offered(Mode::Data2g).len(), 3);
        assert_eq!(TncBw::Hz1200.command(), "BW1200");
        for m in [Mode::Mercury, Mode::Data2g] {
            for bw in TncBw::offered(m) {
                let (lo, hi) = bw.passband(m);
                assert!(lo < 1500.0 && hi > 1550.0 && hi - lo >= bw.hz() as f32, "{m:?} {bw:?}");
            }
        }
    }
}
