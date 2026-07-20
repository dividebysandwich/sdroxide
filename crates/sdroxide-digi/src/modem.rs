//! The mfsk-core adapter: FT8/FT4 decode and encode wrapped behind stable
//! sdroxide types. **This is the only file that touches raw mfsk-core
//! decode-result fields** — if the crate's field names change, only
//! `decode_slot` needs updating.

use mfsk_core::msg::wsjt77;
use sdroxide_types::{Decode, Mode};

use crate::params::{AUDIO_MAX_HZ, AUDIO_MIN_HZ};

const SYNC_MIN: f32 = 1.5;
const MAX_CAND: usize = 120;

/// Encode/decode engine for one digital mode.
pub struct Ft8Modem {
    mode: Mode,
}

impl Ft8Modem {
    pub fn new(mode: Mode) -> Self {
        Ft8Modem { mode }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Decode one full receive slot of 12 kHz mono i16 audio.
    ///
    /// FT8 and FT4 return different mfsk-core result types, so each is mapped
    /// to our stable [`Decode`] inside its own arm — the only place that
    /// reads raw mfsk-core fields.
    pub fn decode_slot(&self, audio_12k: &[i16], slot_utc: i64) -> Vec<Decode> {
        match self.mode {
            Mode::Ft4 => {
                let depth = mfsk_core::core::pipeline::DecodeDepth::BpAllOsd;
                mfsk_core::ft4::decode::decode_frame_with_options(
                    audio_12k, AUDIO_MIN_HZ, AUDIO_MAX_HZ, SYNC_MIN, None, depth, MAX_CAND,
                )
                .into_iter()
                .filter_map(|r| {
                    let bits: [u8; 77] = r.message77().try_into().ok()?;
                    build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc)
                })
                .collect()
            }
            _ => {
                let depth = mfsk_core::ft8::decode::DecodeDepth::BpAllOsd;
                mfsk_core::ft8::decode::decode_frame(
                    audio_12k, AUDIO_MIN_HZ, AUDIO_MAX_HZ, SYNC_MIN, None, depth, MAX_CAND,
                )
                .into_iter()
                .filter_map(|r| build_decode(&r.message77, r.snr_db, r.dt_sec, r.freq_hz, slot_utc))
                .collect()
            }
        }
    }

    /// Synthesize a message into 12 kHz mono f32 burst audio at tone offset
    /// `audio_hz`. Returns `None` if the message can't be packed.
    pub fn encode_burst_12k(&self, text: &str, audio_hz: f32, amplitude: f32) -> Option<Vec<f32>> {
        let (c1, c2, c3) = three_tokens(text);
        let msg77 = wsjt77::pack77(&c1, &c2, &c3)?;
        let audio = match self.mode {
            Mode::Ft4 => {
                let tones = mfsk_core::ft4::encode::message_to_tones(&msg77);
                mfsk_core::ft4::encode::tones_to_f32(&tones, audio_hz, amplitude)
            }
            _ => {
                let tones = mfsk_core::ft8::wave_gen::message_to_tones(&msg77);
                mfsk_core::ft8::wave_gen::tones_to_f32(&tones, audio_hz, amplitude)
            }
        };
        Some(audio)
    }
}

/// Unpack 77 message bits and build a [`Decode`], or `None` if unpacking fails.
fn build_decode(bits77: &[u8; 77], snr_db: f32, dt_sec: f32, freq_hz: f32, slot_utc: i64) -> Option<Decode> {
    let text = wsjt77::unpack77(bits77)?;
    let (to, from, grid, is_cq) = parse_message(&text);
    Some(Decode {
        slot_utc,
        snr_db: snr_db.round() as i16,
        dt: dt_sec,
        audio_hz: freq_hz,
        message: text,
        to,
        from,
        grid,
        is_cq,
    })
}

/// Split a message into three whitespace tokens for `pack77` (call1, call2,
/// payload); missing tokens become empty strings.
fn three_tokens(text: &str) -> (String, String, String) {
    let mut it = text.split_whitespace();
    (
        it.next().unwrap_or("").to_string(),
        it.next().unwrap_or("").to_string(),
        it.next().unwrap_or("").to_string(),
    )
}

/// Parse a standard `<to> <from> <payload>` message into its parts.
/// Returns (to, from, grid, is_cq). `to` is None for a CQ.
fn parse_message(text: &str) -> (Option<String>, Option<String>, Option<String>, bool) {
    let toks: Vec<&str> = text.split_whitespace().collect();
    if toks.is_empty() {
        return (None, None, None, false);
    }
    if toks[0] == "CQ" {
        // "CQ [DX|modifier] <from> [grid]"
        let from = toks.iter().skip(1).find(|t| is_callish(t)).map(|s| s.to_string());
        let grid = toks.last().filter(|t| is_grid(t)).map(|s| s.to_string());
        return (None, from, grid, true);
    }
    let to = Some(toks[0].to_string());
    let from = toks.get(1).map(|s| s.to_string());
    let grid = toks.get(2).filter(|t| is_grid(t)).map(|s| s.to_string());
    (to, from, grid, false)
}

fn is_grid(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 4
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_uppercase()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
}

fn is_callish(t: &str) -> bool {
    t.len() >= 3 && t.chars().any(|c| c.is_ascii_digit()) && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cq_and_qso_messages() {
        let (to, from, grid, cq) = parse_message("CQ AB1CD FN42");
        assert_eq!(to, None);
        assert_eq!(from.as_deref(), Some("AB1CD"));
        assert_eq!(grid.as_deref(), Some("FN42"));
        assert!(cq);

        let (to, from, grid, cq) = parse_message("W9XYZ AB1CD -13");
        assert_eq!(to.as_deref(), Some("W9XYZ"));
        assert_eq!(from.as_deref(), Some("AB1CD"));
        assert_eq!(grid, None);
        assert!(!cq);

        let (to, from, grid, _) = parse_message("AB1CD W9XYZ EM48");
        assert_eq!(to.as_deref(), Some("AB1CD"));
        assert_eq!(from.as_deref(), Some("W9XYZ"));
        assert_eq!(grid.as_deref(), Some("EM48"));
    }

    #[test]
    fn ft8_encode_decode_round_trip() {
        let modem = Ft8Modem::new(Mode::Ft8);
        let burst = modem.encode_burst_12k("CQ AB1CD FN42", 1500.0, 0.5).expect("encode");
        // Pad into a full 15 s slot: 0.5 s lead, burst, trailing silence.
        let mut slot = vec![0.0f32; (0.5 * 12_000.0) as usize];
        slot.extend_from_slice(&burst);
        slot.resize(15 * 12_000, 0.0);
        let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        let decodes = modem.decode_slot(&i16buf, 0);
        assert!(
            decodes.iter().any(|d| d.message == "CQ AB1CD FN42"),
            "got {:?}",
            decodes.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ft4_encode_decode_round_trip() {
        let modem = Ft8Modem::new(Mode::Ft4);
        let burst = modem.encode_burst_12k("CQ AB1CD FN42", 1500.0, 0.5).expect("encode");
        let mut slot = vec![0.0f32; (0.5 * 12_000.0) as usize];
        slot.extend_from_slice(&burst);
        slot.resize((7.5 * 12_000.0) as usize, 0.0);
        let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        let decodes = modem.decode_slot(&i16buf, 0);
        assert!(
            decodes.iter().any(|d| d.message == "CQ AB1CD FN42"),
            "got {:?}",
            decodes.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
