//! JTTY vocabulary shared by the engine, the wire protocol and the panel.
//!
//! **Interface facts only.** JTTY's modem, FEC and source grammar live in
//! `sdroxide-dsp` (which carries the GPL obligation for the WSJT-X port). What
//! is here is what the UI and the wire need to talk about a decoded message:
//! the rendered text and a couple of flags. Nothing here is derived from
//! WSJT-X source, so this crate stays permissive-intent and wasm-clean.
//!
//! # What JTTY is
//!
//! The WSJT-X 3.2 RTTY-like **asynchronous** text mode: a transmission can
//! start at any instant, so unlike FT8/FT4 there is no T/R slot and no
//! frame-aligned clock. Each ~1.888 s frame carries a short text or typed
//! contest atom over a narrow (≈127 Hz) 4-GFSK signal with forward error
//! correction, so it copies where 45.45-baud RTTY goes marginal. This program
//! decodes it and shows the messages received.

use serde::{Deserialize, Serialize};

/// The most received messages kept in the rolling list.
pub const JTTY_MESSAGE_MAX: usize = 200;

/// One decoded JTTY message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JttyMessage {
    /// When the message's last frame was heard, unix seconds.
    pub at_unix: i64,
    /// The rendered text, e.g. `CQ K1ABC CQ` or `K1ABC 599 123`.
    pub text: String,
    /// The signal's audio frequency, Hz.
    pub audio_hz: f32,
    /// An approximate SNR in dB.
    pub snr_db: i16,
    /// True when the end-of-message flag was seen, so the text is the whole
    /// message rather than a run cut off by the next transmission.
    pub complete: bool,
}

/// JTTY's live state, for the panel and the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JttyStatus {
    /// Smoothed audio level, for the meter.
    pub level: f32,
    /// Messages received, newest last.
    pub messages: Vec<JttyMessage>,
    /// Total decoded since entering the mode.
    pub total: u64,
}

impl Default for JttyStatus {
    fn default() -> Self {
        JttyStatus { level: 0.0, messages: Vec::new(), total: 0 }
    }
}
