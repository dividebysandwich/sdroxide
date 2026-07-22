//! TCI wire format: text command builders, status parsing, the binary stream
//! header, and Rust `Mode` ↔ TCI modulation mapping.
//!
//! Binary layout confirmed against wfview's `tciserver.h` (Expert TCI SDK): a
//! 64-byte header of 16 little-endian u32 then interleaved little-endian f32.

use sdroxide_types::Mode;

/// Binary stream type (`type` header field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Iq,       // 0: server→client, interleaved I,Q f32
    RxAudio,  // 1: server→client, stereo L,R f32 @ 48 kHz
    TxAudio,  // 2: client→server, stereo L,R f32 @ 48 kHz
    TxChrono, // 3: server→client, TX pacing
    Other(u32),
}

impl DataType {
    pub fn from_u32(v: u32) -> DataType {
        match v {
            0 => DataType::Iq,
            1 => DataType::RxAudio,
            2 => DataType::TxAudio,
            3 => DataType::TxChrono,
            other => DataType::Other(other),
        }
    }
    pub fn to_u32(self) -> u32 {
        match self {
            DataType::Iq => 0,
            DataType::RxAudio => 1,
            DataType::TxAudio => 2,
            DataType::TxChrono => 3,
            DataType::Other(v) => v,
        }
    }
}

/// float32 sample format id.
pub const FORMAT_FLOAT32: u32 = 3;
/// Binary header size in bytes (16 × u32).
pub const HEADER_LEN: usize = 64;

/// Parsed binary stream header. `sample_rate`/`format` are parsed for
/// completeness but not currently consulted.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub receiver: u32,
    pub sample_rate: u32,
    pub format: u32,
    /// Number of f32 VALUES in the payload (2 × frames for stereo/complex).
    pub length: u32,
    pub dtype: DataType,
}

fn u32_le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Parse the 64-byte header from the start of a binary message.
pub fn parse_header(msg: &[u8]) -> Option<Header> {
    if msg.len() < HEADER_LEN {
        return None;
    }
    Some(Header {
        receiver: u32_le(msg, 0),
        sample_rate: u32_le(msg, 4),
        format: u32_le(msg, 8),
        length: u32_le(msg, 20),
        dtype: DataType::from_u32(u32_le(msg, 24)),
    })
}

/// Decode the little-endian f32 payload of a binary message (after the header),
/// appending to `out`. Honors the header's `length` (float count) when sane.
pub fn decode_f32_payload(msg: &[u8], header: &Header, out: &mut Vec<f32>) {
    let payload = &msg[HEADER_LEN..];
    let max = payload.len() / 4;
    let n = if header.length as usize <= max { header.length as usize } else { max };
    for c in payload[..n * 4].chunks_exact(4) {
        out.push(f32::from_le_bytes([c[0], c[1], c[2], c[3]]));
    }
}

/// Build a `TxAudio` binary message from mono 48 kHz audio, duplicating each
/// sample to both stereo channels.
pub fn build_tx_audio(sample_rate: u32, receiver: u32, mono: &[f32]) -> Vec<u8> {
    let floats = mono.len() * 2; // stereo
    let mut buf = Vec::with_capacity(HEADER_LEN + floats * 4);
    let mut hdr = [0u32; 16];
    hdr[0] = receiver;
    hdr[1] = sample_rate;
    hdr[2] = FORMAT_FLOAT32;
    hdr[5] = floats as u32; // length = float count
    hdr[6] = DataType::TxAudio.to_u32();
    for w in hdr {
        buf.extend_from_slice(&w.to_le_bytes());
    }
    for &s in mono {
        buf.extend_from_slice(&s.to_le_bytes()); // L
        buf.extend_from_slice(&s.to_le_bytes()); // R
    }
    buf
}

// --- Text commands (lowercase, ';'-terminated) ---

pub fn dds(rx: u32, hz: f64) -> String {
    format!("dds:{rx},{};", hz.round() as i64)
}
pub fn if_offset(rx: u32, channel: u32, hz: f64) -> String {
    format!("if:{rx},{channel},{};", hz.round() as i64)
}
#[allow(dead_code)] // reserved: absolute-VFO tune (we use dds + if instead)
pub fn vfo(rx: u32, channel: u32, hz: f64) -> String {
    format!("vfo:{rx},{channel},{};", hz.round() as i64)
}
pub fn modulation(rx: u32, mode: &str) -> String {
    format!("modulation:{rx},{mode};")
}
pub fn trx(rx: u32, on: bool, tci_source: bool) -> String {
    if on && tci_source {
        format!("trx:{rx},true,tci;")
    } else {
        format!("trx:{rx},{on};")
    }
}
pub fn iq_samplerate(hz: u32) -> String {
    format!("iq_samplerate:{hz};")
}
pub fn iq_start(rx: u32) -> String {
    format!("iq_start:{rx};")
}
pub fn iq_stop(rx: u32) -> String {
    format!("iq_stop:{rx};")
}
#[allow(dead_code)] // reserved: demod-audio RX stream (we use wideband IQ)
pub fn audio_start(rx: u32) -> String {
    format!("audio_start:{rx};")
}
pub fn rx_enable(rx: u32, on: bool) -> String {
    format!("rx_enable:{rx},{on};")
}

/// Split a text frame into `(command, args)` pairs (command lowercased, args as
/// the raw remainder). Handles multiple `;`-separated commands per frame.
pub fn parse_status(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for part in text.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once(':') {
            Some((cmd, args)) => out.push((cmd.trim().to_lowercase(), args.trim().to_string())),
            None => out.push((part.to_lowercase(), String::new())),
        }
    }
    out
}

/// TCI modulation string for a Rust `Mode` (FT8/FT4 → digu).
pub fn mode_to_tci(mode: Mode) -> &'static str {
    match mode {
        Mode::Lsb => "lsb",
        Mode::Usb => "usb",
        Mode::Cw => "cw",
        Mode::Am => "am",
        Mode::Sam => "sam",
        Mode::Nfm => "nfm",
        Mode::Wfm => "wfm",
        Mode::Digu | Mode::Ft8 | Mode::Ft4 => "digu",
        Mode::Digl => "digl",
        Mode::Dsb => "dsb",
        Mode::Spec => "usb",
    }
}

/// Rust `Mode` for a TCI modulation string (used to follow rig mode changes).
pub fn tci_to_mode(s: &str) -> Option<Mode> {
    Some(match s.trim().to_lowercase().as_str() {
        "lsb" => Mode::Lsb,
        "usb" => Mode::Usb,
        "cw" => Mode::Cw,
        "am" => Mode::Am,
        "sam" => Mode::Sam,
        "nfm" | "fm" => Mode::Nfm,
        "wfm" => Mode::Wfm,
        "digu" => Mode::Digu,
        "digl" => Mode::Digl,
        "dsb" => Mode::Dsb,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands() {
        assert_eq!(dds(0, 14_074_000.0), "dds:0,14074000;");
        assert_eq!(if_offset(0, 0, -1500.0), "if:0,0,-1500;");
        assert_eq!(vfo(0, 1, 7_100_000.0), "vfo:0,1,7100000;");
        assert_eq!(modulation(0, "usb"), "modulation:0,usb;");
        assert_eq!(trx(0, true, true), "trx:0,true,tci;");
        assert_eq!(trx(0, false, true), "trx:0,false;");
        assert_eq!(iq_samplerate(192000), "iq_samplerate:192000;");
        assert_eq!(iq_start(0), "iq_start:0;");
    }

    #[test]
    fn status_parsing() {
        let s = parse_status("protocol:ExpertSDR3,1.8;device:SunSDR2PRO;ready;");
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], ("protocol".into(), "ExpertSDR3,1.8".into()));
        assert_eq!(s[1], ("device".into(), "SunSDR2PRO".into()));
        assert_eq!(s[2], ("ready".into(), "".into()));
    }

    #[test]
    fn header_and_payload_roundtrip() {
        // Build a TxAudio packet from mono, then parse it back as a stream.
        let mono = [0.25f32, -0.5, 0.75];
        let pkt = build_tx_audio(48_000, 0, &mono);
        let h = parse_header(&pkt).expect("header");
        assert_eq!(h.dtype, DataType::TxAudio);
        assert_eq!(h.sample_rate, 48_000);
        assert_eq!(h.format, FORMAT_FLOAT32);
        assert_eq!(h.length, 6); // 3 mono → 6 stereo floats
        let mut out = Vec::new();
        decode_f32_payload(&pkt, &h, &mut out);
        assert_eq!(out.len(), 6);
        // stereo-duplicated
        assert!((out[0] - 0.25).abs() < 1e-6 && (out[1] - 0.25).abs() < 1e-6);
        assert!((out[2] + 0.5).abs() < 1e-6 && (out[3] + 0.5).abs() < 1e-6);
    }

    #[test]
    fn datatype_roundtrip() {
        for v in 0..4u32 {
            assert_eq!(DataType::from_u32(v).to_u32(), v);
        }
    }

    #[test]
    fn mode_mapping() {
        assert_eq!(mode_to_tci(Mode::Usb), "usb");
        assert_eq!(mode_to_tci(Mode::Ft8), "digu");
        assert_eq!(tci_to_mode("LSB"), Some(Mode::Lsb));
        assert_eq!(tci_to_mode("digu"), Some(Mode::Digu));
        assert_eq!(tci_to_mode("bogus"), None);
    }
}
