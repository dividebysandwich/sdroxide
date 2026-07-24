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
        Mode::Usb
        | Mode::Digu
        | Mode::Ft8
        | Mode::Ft4
        | Mode::Psk
        | Mode::Rtty
        | Mode::Sstv
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq
        | Mode::RfPaint => 0x01,
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
/// Read the SWR meter (Icom cmd `0x15` sub `0x12`). Only meaningful while
/// transmitting; the rig answers with a 0..255 reading (see [`swr_from_reading`]).
pub fn read_swr_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x15, &[0x12])
}

/// Decode Icom's 2-byte BCD meter reading (`0000..0255`) to a plain integer.
/// `data` is the payload after the meter sub-command byte.
fn decode_meter(data: &[u8]) -> Option<u32> {
    let bcd = |b: u8| -> Option<u32> {
        let (hi, lo) = ((b >> 4) as u32, (b & 0x0f) as u32);
        (hi <= 9 && lo <= 9).then_some(hi * 10 + lo)
    };
    let (a, b) = (data.first()?, data.get(1)?);
    Some(bcd(*a)? * 100 + bcd(*b)?)
}

/// Map an Icom SWR-meter reading (`0..255`) to an SWR ratio via piecewise-linear
/// interpolation over the standard calibration breakpoints (matching Hamlib's
/// Icom SWR curve: 0→1.0, 48→1.5, 80→2.0, 120→3.0), extended past 3:1 for the
/// rare high-SWR case. Clamped to the table ends.
fn swr_from_reading(reading: u32) -> f32 {
    const CAL: &[(f32, f32)] =
        &[(0.0, 1.0), (48.0, 1.5), (80.0, 2.0), (120.0, 3.0), (160.0, 5.0), (255.0, 10.0)];
    let r = reading as f32;
    if r <= CAL[0].0 {
        return CAL[0].1;
    }
    for w in CAL.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if r <= x1 {
            return y0 + (y1 - y0) * (r - x0) / (x1 - x0);
        }
    }
    CAL[CAL.len() - 1].1
}

/// Parse an SWR-meter reply payload (Icom cmd `0x15`): the sub-command byte
/// followed by the BCD reading. Returns the SWR ratio, or `None` if the reply
/// isn't the SWR meter (`0x12`) or is malformed.
pub fn parse_swr_reply(data: &[u8]) -> Option<f32> {
    if data.first() != Some(&0x12) {
        return None;
    }
    Some(swr_from_reading(decode_meter(&data[1..])?))
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
    fn swr_meter_decodes_and_scales() {
        // Reply payload = sub-command 0x12 followed by the 2-byte BCD reading.
        // Calibration breakpoints map exactly: 0→1.0, 48→1.5, 80→2.0, 120→3.0.
        let swr = |reading_bcd: [u8; 2]| parse_swr_reply(&[0x12, reading_bcd[0], reading_bcd[1]]);
        assert_eq!(swr([0x00, 0x00]), Some(1.0)); // reading 0
        assert_eq!(swr([0x00, 0x48]), Some(1.5)); // reading 48
        assert_eq!(swr([0x00, 0x80]), Some(2.0)); // reading 80
        assert_eq!(swr([0x01, 0x20]), Some(3.0)); // reading 120
        // Midpoint of the 0..48 segment interpolates linearly to 1.25.
        assert_eq!(swr([0x00, 0x24]), Some(1.25)); // reading 24
        // A malformed reading (bad BCD nibble) yields None, not a bogus SWR.
        assert_eq!(swr([0x00, 0x0f]), None);
        // The wrong meter sub-command is ignored (we only read SWR / 0x12).
        assert_eq!(parse_swr_reply(&[0x11, 0x00, 0x50]), None);
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
