//! Icom CI-V framing — also used by the Xiegu X6100, which speaks a CI-V
//! dialect. A frame is `FE FE <to> <from> <cmd> [data…] FD`.

use sdroxide_types::Mode;

pub const PREAMBLE: u8 = 0xFE;
pub const END: u8 = 0xFD;
/// Controller (this software) address — conventional default.
pub const CONTROLLER_ADDR: u8 = 0xE0;

/// Encode a frequency (Hz) as 5 little-endian BCD bytes (CI-V cmd 0x05/0x03).
pub fn encode_freq(hz: f64) -> [u8; 5] {
    let mut v = hz.round().max(0.0) as u64;
    let mut out = [0u8; 5];
    for b in out.iter_mut() {
        let lo = (v % 10) as u8;
        v /= 10;
        let hi = (v % 10) as u8;
        v /= 10;
        *b = (hi << 4) | lo;
    }
    out
}

/// Decode 5 little-endian BCD bytes back to a frequency in Hz.
pub fn decode_freq(bytes: &[u8]) -> Option<f64> {
    if bytes.len() < 5 {
        return None;
    }
    let mut hz: u64 = 0;
    for &b in bytes[..5].iter().rev() {
        let hi = (b >> 4) as u64;
        let lo = (b & 0x0f) as u64;
        if hi > 9 || lo > 9 {
            return None;
        }
        hz = hz * 100 + hi * 10 + lo;
    }
    Some(hz as f64)
}

/// The app's `Mode` → CI-V mode byte. Digital modes ride on their sideband.
pub fn mode_to_civ(m: Mode) -> u8 {
    match m {
        Mode::Lsb | Mode::Digl => 0x00,
        Mode::Usb | Mode::Digu | Mode::Ft8 | Mode::Ft4 => 0x01,
        Mode::Am | Mode::Sam | Mode::Dsb => 0x02,
        Mode::Cw => 0x03,
        Mode::Nfm | Mode::Wfm => 0x05,
        Mode::Spec => 0x01,
    }
}

/// CI-V mode byte → the app's `Mode`.
pub fn civ_to_mode(b: u8) -> Option<Mode> {
    Some(match b {
        0x00 => Mode::Lsb,
        0x01 => Mode::Usb,
        0x02 => Mode::Am,
        0x03 | 0x07 => Mode::Cw,
        0x05 | 0x06 => Mode::Nfm,
        _ => return None,
    })
}

/// Build a CI-V frame addressed to `radio`.
pub fn frame(radio: u8, cmd: u8, data: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(6 + data.len());
    f.extend_from_slice(&[PREAMBLE, PREAMBLE, radio, CONTROLLER_ADDR, cmd]);
    f.extend_from_slice(data);
    f.push(END);
    f
}

pub fn set_freq_frame(radio: u8, hz: f64) -> Vec<u8> {
    frame(radio, 0x05, &encode_freq(hz))
}
pub fn read_freq_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x03, &[])
}
pub fn set_mode_frame(radio: u8, m: Mode) -> Vec<u8> {
    // mode byte + filter 1; the X6100 accepts the two-byte form.
    frame(radio, 0x06, &[mode_to_civ(m), 0x01])
}
pub fn read_mode_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x04, &[])
}
pub fn ptt_frame(radio: u8, on: bool) -> Vec<u8> {
    frame(radio, 0x1C, &[0x00, on as u8])
}

/// A parsed reply from the rig (payload after `<cmd>`, addresses stripped).
#[derive(Debug, Clone, PartialEq)]
pub struct CivReply {
    pub from: u8,
    pub to: u8,
    pub cmd: u8,
    pub data: Vec<u8>,
}

/// Pull complete CI-V frames out of a rolling byte buffer, consuming them.
/// Tolerates junk between frames (CI-V is a shared bus with echoes).
pub fn parse_frames(buf: &mut Vec<u8>) -> Vec<CivReply> {
    let mut out = Vec::new();
    loop {
        // Find a preamble pair.
        let Some(start) = buf.windows(2).position(|w| w == [PREAMBLE, PREAMBLE]) else {
            // No frame start; keep at most the last byte (could be a lone FE).
            if buf.len() > 1 {
                let keep = buf.split_off(buf.len() - 1);
                buf.clear();
                buf.extend_from_slice(&keep);
            }
            break;
        };
        // Find the terminator after the preamble.
        let Some(rel_end) = buf[start + 2..].iter().position(|&b| b == END) else {
            // Incomplete frame — drop everything before `start` and wait.
            if start > 0 {
                buf.drain(..start);
            }
            break;
        };
        let end = start + 2 + rel_end;
        // Frame body is buf[start+2 ..= end]; need at least to,from,cmd.
        if end >= start + 5 {
            let to = buf[start + 2];
            let from = buf[start + 3];
            let cmd = buf[start + 4];
            let data = buf[start + 5..end].to_vec();
            out.push(CivReply { from, to, cmd, data });
        }
        buf.drain(..=end);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freq_roundtrips() {
        for hz in [14_074_000.0, 7_055_000.0, 28_500_000.0, 1_800_000.0, 145_500_000.0] {
            let b = encode_freq(hz);
            assert_eq!(decode_freq(&b), Some(hz), "freq {hz}");
        }
    }

    #[test]
    fn known_freq_bytes() {
        // 14.074000 MHz → little-endian BCD.
        assert_eq!(encode_freq(14_074_000.0), [0x00, 0x40, 0x07, 0x14, 0x00]);
    }

    #[test]
    fn set_freq_frame_shape() {
        let f = set_freq_frame(0x70, 14_074_000.0);
        assert_eq!(f, vec![0xFE, 0xFE, 0x70, 0xE0, 0x05, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD]);
    }

    #[test]
    fn parses_freq_reply_amid_echo() {
        // An echo of our own read request, then the rig's freq answer.
        let mut buf = Vec::new();
        buf.extend_from_slice(&read_freq_frame(0x70)); // echo (to=70,from=E0)
        buf.extend_from_slice(&frame(0x70, 0x03, &encode_freq(7_055_000.0))); // "reply"
        let frames = parse_frames(&mut buf);
        // Both parse; the freq one is cmd 0x03 with 5 data bytes.
        let freqs: Vec<f64> =
            frames.iter().filter(|f| f.cmd == 0x03 && f.data.len() >= 5).filter_map(|f| decode_freq(&f.data)).collect();
        assert_eq!(freqs, vec![7_055_000.0]);
        assert!(buf.is_empty());
    }

    #[test]
    fn handles_partial_frame() {
        let full = set_freq_frame(0x70, 14_074_000.0);
        let (head, tail) = full.split_at(6);
        let mut buf = head.to_vec();
        assert!(parse_frames(&mut buf).is_empty()); // incomplete, buffered
        buf.extend_from_slice(tail);
        assert_eq!(parse_frames(&mut buf).len(), 1);
    }
}
