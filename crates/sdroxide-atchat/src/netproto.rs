//! A port of `netproto.py` — AtCHAT's shared protocol constants, frame types
//! and line-based JSON wire framing.
//!
//! This crate represents two distinct layers:
//!   1. **Wire messages** (`ClientMsg` / `ServerMsg`): the outer envelope sent
//!      over TCP between the client and the channel server. Field names match
//!      `channel_server.py` / `client.py` exactly (Python ↔ Rust interop).
//!   2. **Protocol frames** (`Frame`): the JSON INSIDE the modulated audio
//!      payload. Matches `handle_frame` in `client.py` exactly.

use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

// --- PHY-layer parameters (netproto.py) -----------------------------------

/// Effective bytes/sec (the design table). For `airtime()`.
pub fn rate_for(mode: &str) -> f64 {
    match mode {
        "BPSK" => 125.0,
        "QPSK" => 250.0,
        "16QAM" => 500.0,
        _ => 250.0, // Python: RATE_TABLE["QPSK"]
    }
}

/// Seconds — the fixed sync+header cost of every transmission.
pub const PREAMBLE_OVERHEAD: f64 = 0.3;

/// A frame's "time on air" (seconds). `netproto.airtime`.
pub fn airtime(size_bytes: usize, mode: &str) -> f64 {
    PREAMBLE_OVERHEAD + size_bytes as f64 / rate_for(mode)
}

// --- Super-frame / NET timing parameters --------------------------------

pub const BEACON_INTERVAL: f64 = 8.0;
pub const BEACON_TIMEOUT: f64 = BEACON_INTERVAL * 3.0; // 24 s
pub const LOST_TIMEOUT: f64 = 30.0;
pub const REMOVE_TIMEOUT: f64 = 120.0;
pub const BLOCK_SIZE: usize = 220;

pub const SAMPLE_RATE: u32 = 8000;

// --- CRC -----------------------------------------------------------------

/// Exactly `zlib.crc32(data) & 0xFFFFFFFF` (IEEE CRC-32).
pub fn crc32(data: &[u8]) -> u32 {
    let mut h = crc32fast::Hasher::new();
    h.update(data);
    h.finalize()
}

// --- base64 (netproto.b64e / b64d) -------------------------------------

use base64::Engine as _;
const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

pub fn b64e(data: &[u8]) -> String {
    B64.encode(data)
}

pub fn b64d(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    B64.decode(s.as_bytes())
}

// --- Modulation mode -------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename = "BPSK")]
    Bpsk,
    #[serde(rename = "QPSK")]
    Qpsk,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Bpsk => "BPSK",
            Mode::Qpsk => "QPSK",
        }
    }
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "BPSK" => Some(Mode::Bpsk),
            "QPSK" => Some(Mode::Qpsk),
            _ => None,
        }
    }
}

// --- Wire messages: client -> channel server ---------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum ClientMsg {
    #[serde(rename = "HELLO")]
    Hello { callsign: String },
    #[serde(rename = "TRANSMIT_AUDIO")]
    TransmitAudio { audio_b64: String },
}

// --- Wire messages: channel server -> client ---------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    #[serde(rename = "TX_GRANTED")]
    TxGranted { duration: f64 },
    #[serde(rename = "CHANNEL_BUSY")]
    ChannelBusy { retry_after: f64 },
    #[serde(rename = "RX_AUDIO")]
    RxAudio { audio_b64: String },
}

// --- Protocol frames (the JSON inside the modulated payload) --------------
//
// Field names match `handle_frame` in `client.py` exactly. An unknown frame
// type falls through to `Unknown` (the Python side also silently ignores
// unknown ones).

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Frame {
    #[serde(rename = "JOIN_REQUEST")]
    JoinRequest { src: String, dst: String },

    #[serde(rename = "BEACON")]
    Beacon {
        src: String,
        dst: String,
        #[serde(default)]
        backup: Option<String>,
        #[serde(default)]
        roster: Vec<String>,
    },

    #[serde(rename = "CHAT")]
    Chat { src: String, dst: String, text: String },

    #[serde(rename = "BULK_META")]
    BulkMeta {
        src: String,
        dst: String,
        transfer_id: String,
        filename: String,
        total_blocks: usize,
        total_size: usize,
    },

    #[serde(rename = "BULK_BLOCK")]
    BulkBlock {
        src: String,
        dst: String,
        transfer_id: String,
        seq: usize,
        data: String, // base64
        crc: u32,
    },

    #[serde(rename = "BULK_END")]
    BulkEnd { src: String, dst: String, transfer_id: String },

    #[serde(rename = "BULK_STATUS")]
    BulkStatus { src: String, dst: String, transfer_id: String, missing: Vec<usize> },

    #[serde(other)]
    Unknown,
}

impl Frame {
    /// `frame.get("src")` — a Python convenience.
    pub fn src(&self) -> Option<&str> {
        match self {
            Frame::JoinRequest { src, .. }
            | Frame::Beacon { src, .. }
            | Frame::Chat { src, .. }
            | Frame::BulkMeta { src, .. }
            | Frame::BulkBlock { src, .. }
            | Frame::BulkEnd { src, .. }
            | Frame::BulkStatus { src, .. } => Some(src),
            Frame::Unknown => None,
        }
    }

    /// `frame.get("dst", "ALL")`.
    pub fn dst(&self) -> &str {
        match self {
            Frame::JoinRequest { dst, .. }
            | Frame::Beacon { dst, .. }
            | Frame::Chat { dst, .. }
            | Frame::BulkMeta { dst, .. }
            | Frame::BulkBlock { dst, .. }
            | Frame::BulkEnd { dst, .. }
            | Frame::BulkStatus { dst, .. } => dst,
            Frame::Unknown => "ALL",
        }
    }

    /// A short wire-name tag, for logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Frame::JoinRequest { .. } => "JOIN_REQUEST",
            Frame::Beacon { .. } => "BEACON",
            Frame::Chat { .. } => "CHAT",
            Frame::BulkMeta { .. } => "BULK_META",
            Frame::BulkBlock { .. } => "BULK_BLOCK",
            Frame::BulkEnd { .. } => "BULK_END",
            Frame::BulkStatus { .. } => "BULK_STATUS",
            Frame::Unknown => "UNKNOWN",
        }
    }

    pub fn to_json_bytes(&self) -> Vec<u8> {
        // The equivalent of `json.dumps(frame, ensure_ascii=False)`.
        serde_json::to_vec(self).expect("frame serialize")
    }

    pub fn from_json_bytes(b: &[u8]) -> Option<Frame> {
        serde_json::from_slice(b).ok()
    }
}

// --- Line-based JSON framing (netproto.send_json / read_json) ------------

/// `send_json`: one line of JSON + `\n`, then flush.
pub async fn write_json<W, T>(w: &mut W, obj: &T) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let mut line = serde_json::to_vec(obj).map_err(io::Error::other)?;
    line.push(b'\n');
    w.write_all(&line).await?;
    w.flush().await
}

/// `read_json`: read one line. `Ok(None)` at EOF.
pub async fn read_json<R>(r: &mut R) -> io::Result<Option<Value>>
where
    R: AsyncBufReadExt + Unpin,
{
    let mut line = String::new();
    let n = r.read_line(&mut line).await?;
    if n == 0 {
        return Ok(None);
    }
    let v = serde_json::from_str(line.trim_end()).map_err(io::Error::other)?;
    Ok(Some(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_zlib() {
        // zlib.crc32(b"123456789") == 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn airtime_matches_python() {
        assert!((airtime(250, "QPSK") - 1.3).abs() < 1e-9);
        assert!((airtime(125, "BPSK") - 1.3).abs() < 1e-9);
    }

    #[test]
    fn frame_roundtrip_field_names() {
        let f = Frame::Beacon {
            src: "TA1ABC".into(),
            dst: "ALL".into(),
            backup: Some("TA2DEF".into()),
            roster: vec!["TA1ABC".into(), "TA2DEF".into()],
        };
        let s = String::from_utf8(f.to_json_bytes()).unwrap();
        assert!(s.contains(r#""type":"BEACON""#));
        assert!(s.contains(r#""backup":"TA2DEF""#));
        let back = Frame::from_json_bytes(s.as_bytes()).unwrap();
        assert_eq!(back.src(), Some("TA1ABC"));
    }

    #[test]
    fn unknown_frame_does_not_error() {
        let v = br#"{"type":"SOMETHING_NEW","src":"X"}"#;
        assert!(matches!(Frame::from_json_bytes(v), Some(Frame::Unknown)));
    }

    #[test]
    fn wire_msg_tags() {
        let m = ClientMsg::Hello { callsign: "TA1ABC".into() };
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains(r#""cmd":"HELLO""#));
        let r: ServerMsg = serde_json::from_str(r#"{"type":"TX_GRANTED","duration":1.5}"#).unwrap();
        assert!(matches!(r, ServerMsg::TxGranted { duration } if (duration - 1.5).abs() < 1e-9));
    }
}
