//! UVPacket vocabulary shared by the engine, the wire protocol and the panel.
//!
//! **Interface facts only.** UVPacket's modem, LDPC code and framing live in
//! `sdroxide-digi`, which already carries the GPL obligation for mfsk-core.
//! What is here is what the UI and the wire need to talk about a decoded frame
//! at all: which sub-mode it was sent at, what the header said, and the raw
//! payload bytes. Nothing here is derived from mfsk-core source, so this crate
//! stays permissive-intent and wasm-clean.
//!
//! # What UVPacket is
//!
//! Unlike the WSJT-X family, UVPacket is a **packet** protocol: a short
//! π/4-DQPSK burst carrying an application byte pipe rather than a
//! `<to> <from> <grid>` message. Its header names an `app_type` (the
//! application's own tag), a `sequence` number and a payload block count; the
//! payload is 1–32 blocks of twelve bytes. The sub-mode is conveyed by the
//! preamble, so a receiver **detects** it rather than being told it — which is
//! why there is no operator sub-mode setting here, unlike Q65's or FST4's.
//!
//! It is an in-tree mfsk-core *applied example* for private amateur groups,
//! not a mainstream mode; this program decodes it and shows the frames.

use serde::{Deserialize, Serialize};

/// A UVPacket sub-mode: the puncturing posture the preamble names.
///
/// All four share the modem, the preamble layout and the `Ldpc240_101` mother
/// code; they differ only in how much of the parity survives puncture (and, for
/// `UltraRobust`, in running at half the symbol rate). The receiver detects
/// which one arrived, so this is a label on a decoded frame rather than a
/// setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum UvPacketMode {
    /// Robust — unpunctured rate 0.42, 1008 net bps. The weak-signal posture.
    #[default]
    Robust,
    /// Standard — punctured to rate 1/2, 1200 net bps.
    Standard,
    /// UltraRobust — unpunctured at half baud (600 baud), 504 net bps. The
    /// lowest-threshold sub-mode.
    UltraRobust,
    /// Express — punctured to rate 3/4, 1800 net bps. Strong signals only.
    Express,
}

impl UvPacketMode {
    /// Every sub-mode, in declaration order.
    pub const ALL: [UvPacketMode; 4] = [
        UvPacketMode::Robust,
        UvPacketMode::Standard,
        UvPacketMode::UltraRobust,
        UvPacketMode::Express,
    ];

    /// The name the panel shows.
    pub fn label(self) -> &'static str {
        match self {
            UvPacketMode::Robust => "Robust",
            UvPacketMode::Standard => "Standard",
            UvPacketMode::UltraRobust => "Ultra",
            UvPacketMode::Express => "Express",
        }
    }

    /// Net payload bit rate at the canonical 1200 baud (600 for UltraRobust).
    pub fn net_bps(self) -> u32 {
        match self {
            UvPacketMode::Robust => 1008,
            UvPacketMode::Standard => 1200,
            UvPacketMode::UltraRobust => 504,
            UvPacketMode::Express => 1800,
        }
    }
}

/// One decoded UVPacket frame, as the receiver filed it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UvPacketFrame {
    /// Unix seconds UTC when the frame was decoded — the receiver's clock, not
    /// anything the frame carries (it has no time of its own).
    pub at: i64,
    /// The sub-mode the preamble named.
    pub mode: UvPacketMode,
    /// The application's own 4-bit tag. UVPacket does not define what it means.
    pub app_type: u8,
    /// A 5-bit sequence number the application sets, for reassembling or
    /// de-duplicating its own traffic.
    pub sequence: u8,
    /// Payload blocks carried, 1–32.
    pub block_count: u8,
    /// WSJT-X-compatible SNR estimate (dB, 2.5 kHz reference bandwidth).
    pub snr_db: i16,
    /// The raw payload bytes, `block_count × 12` of them (zero-padded to the
    /// block boundary).
    pub payload: Vec<u8>,
}

impl UvPacketFrame {
    /// The payload as text when every byte is printable ASCII, with tab, CR and
    /// LF allowed; `None` when it is binary or empty.
    ///
    /// A private application is as likely to send a binary struct as a
    /// sentence, so the panel offers both readings rather than guessing.
    pub fn as_text(&self) -> Option<String> {
        if self.payload.is_empty() {
            return None;
        }
        let printable = self
            .payload
            .iter()
            .all(|&b| (0x20..=0x7e).contains(&b) || b == b'\t' || b == b'\r' || b == b'\n');
        printable.then(|| String::from_utf8_lossy(&self.payload).into_owned())
    }

    /// The payload as a lowercase hex string, no separators.
    pub fn as_hex(&self) -> String {
        let mut s = String::with_capacity(self.payload.len() * 2);
        for b in &self.payload {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

/// Audio-domain centre of the UVPacket carrier, in Hz above the dial. Fixed by
/// the modem — the four tones land at 800 / 1400 / 2000 / 2600 Hz around it.
pub const UVPACKET_AUDIO_CENTRE_HZ: f32 = 1700.0;

/// Most UVPacket frames kept. A packet channel is mostly quiet — bursts are
/// short and sporadic — so this is a long session's worth.
pub const UVPACKET_FRAME_MAX: usize = 200;

/// What the UVPacket receiver is doing.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct UvPacketStatus {
    /// Smoothed audio level, for a meter.
    pub level: f32,
    /// Frames received, newest last.
    pub frames: Vec<UvPacketFrame>,
    /// Frames decoded since the receiver started, including ones that have
    /// scrolled out of the rolling list.
    pub frames_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_payload_is_recognised_and_binary_is_not() {
        let f = |payload: Vec<u8>| UvPacketFrame {
            at: 0,
            mode: UvPacketMode::Robust,
            app_type: 0,
            sequence: 0,
            block_count: 1,
            snr_db: 0,
            payload,
        };
        assert_eq!(f(b"hello".to_vec()).as_text().as_deref(), Some("hello"));
        assert_eq!(f(vec![0x00, 0xff]).as_text(), None);
        assert_eq!(f(Vec::new()).as_text(), None);
        assert_eq!(f(vec![0x00, 0xab]).as_hex(), "00ab");
    }

    #[test]
    fn every_sub_mode_has_a_distinct_label() {
        for (i, a) in UvPacketMode::ALL.iter().enumerate() {
            for b in &UvPacketMode::ALL[i + 1..] {
                assert_ne!(a.label(), b.label());
            }
        }
    }
}
