//! Weathex WxR-301D decoder.
//!
//! The signal: a wide FSK carrier (about 64 kHz) at 4800 baud, or at 5000 baud
//! with the bytes whitened by the PN9 sequence of the CC1101 family. Plain
//! bits, most significant first. A frame is 69 bytes: the preamble `AA AA AA`
//! and a two-byte word (`2D D4` plain, `C1 94` whitened), then the data from
//! byte 6 (plain) or 8 (whitened): a 32-bit serial number, a 16-bit counter,
//! a frame id (1 or 2), and, in the id 2 frames, the time (decimal `hhmmss`
//! in the low 18 bits of 24), the altitude (19 bits, decimetres), the
//! latitude (25 bits) and the longitude (26 bits), both in 1e-5 degrees, bit
//! packed one behind the other. Two check bytes close the 53 data bytes: the
//! exclusive or and the sum of them.
//!
//! The checks are short, so a fix needs an id 1 and an id 2 frame, both with
//! a valid check, that carry the same serial number and counter.
//!
//! Provenance: the layout is the public description of the WxR-301D by
//! rs1729/RS (`weathex/`, `demod/mod/` is not used here). No code of
//! auto_rx, SDRangel or rs1729 (GPL) is used. Checked on synthetic frames
//! built from that description (both variants, 48 and 96 kS/s, both
//! polarities); not on a radio recording. A signal 64 kHz wide needs the
//! 96 kS/s lane.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const FRAME_BYTES: usize = 69;
const HEADER_BITS: usize = 40;
const HEADER_TOLERANCE: u32 = 2;
const PREAMBLE: u64 = 0xAAAAAA;
/// The two-byte word behind the preamble, plain and whitened.
const SYNC_PLAIN: u64 = 0x2DD4;
const SYNC_PN9: u64 = 0xC194;
const DATA_BYTES: usize = 53;

/// The whitening sequence.
const PN9: [u8; 64] = [
    0xFF, 0x87, 0xB8, 0x59, 0xB7, 0xA1, 0xCC, 0x24, 0x57, 0x5E, 0x4B, 0x9C, 0x0E, 0xE9, 0xEA, 0x50,
    0x2A, 0xBE, 0xB4, 0x1B, 0xB6, 0xB0, 0x5D, 0xF1, 0xE6, 0x9A, 0xE3, 0x45, 0xFD, 0x2C, 0x53, 0x18,
    0x0C, 0xCA, 0xC9, 0xFB, 0x49, 0x37, 0xE5, 0xA8, 0x51, 0x3B, 0x2F, 0x61, 0xAA, 0x72, 0x18, 0x84,
    0x02, 0x23, 0x23, 0xAB, 0x63, 0x89, 0x51, 0xB3, 0xE7, 0x8B, 0x72, 0x90, 0x4C, 0xE8, 0xFB, 0xC1,
];

fn check(data: &[u8]) -> u16 {
    let x = data.iter().fold(0u8, |a, &b| a ^ b);
    let s = data.iter().fold(0u8, |a, &b| a.wrapping_add(b));
    u16::from(x) << 8 | u16::from(s)
}

/// What a frame says, with its check verified.
#[derive(Clone, Copy)]
struct Frame {
    serial: u32,
    counter: u16,
    id: u8,
    /// Time of day in seconds, the place and the altitude (id 2 only).
    place: Option<(u32, f64, f64, f64)>,
}

/// `bytes` is the whole frame; `pn9` tells whether it is whitened.
fn parse(bytes: &[u8; FRAME_BYTES], pn9: bool) -> Option<Frame> {
    let ofs = if pn9 { 8 } else { 6 };
    let mut x = *bytes;
    if pn9 {
        for (j, b) in x.iter_mut().enumerate().skip(6) {
            *b ^= PN9[(j - 6) % 64];
        }
    }
    let d = &x[ofs..];
    if u16::from(d[DATA_BYTES]) << 8 | u16::from(d[DATA_BYTES + 1]) != check(&d[..DATA_BYTES]) {
        return None;
    }
    let serial = u32::from_le_bytes([d[0], d[1], d[2], d[3]]);
    let counter = u16::from_le_bytes([d[4], d[5]]);
    let id = d[6];
    let place = (id == 2).then(|| {
        let hms = (u32::from(d[7]) | u32::from(d[8]) << 8 | u32::from(d[9]) << 16) & 0x3FFFF;
        let secs = hms / 10_000 * 3600 + hms % 10_000 / 100 * 60 + hms % 100;
        let bits = d[13..23].iter().rev().fold(0u128, |a, &b| a << 8 | u128::from(b));
        let alt = (bits >> 4 & 0x7FFFF) as f64 / 10.0;
        let lat = (bits >> 23 & 0x1FF_FFFF) as f64 / 1e5;
        let lon = (bits >> 48 & 0x3FF_FFFF) as f64 / 1e5;
        (secs, lat, lon, alt)
    });
    Some(Frame { serial, counter, id, place })
}

fn pos_of(place: (u32, f64, f64, f64)) -> Option<GpsPos> {
    let (_, lat, lon, alt) = place;
    let ok = (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
        && (-500.0..60_000.0).contains(&alt)
        && (lat != 0.0 || lon != 0.0);
    ok.then_some(GpsPos {
        lat,
        lon,
        alt_m: alt,
        speed_ms: 0.0,
        course_deg: 0.0,
        climb_ms: 0.0,
        num_sv: 0,
    })
}

struct Collect {
    bits: Vec<u8>,
    invert: bool,
}

/// One demodulator and frame finder, for one baud rate.
struct Lane {
    slicer: FskSlicer,
    pn9: bool,
    sr: u64,
    frame: Option<Collect>,
}

pub struct WeathexDecoder {
    lanes: [Lane; 2],
    /// The latest valid id 1 frame.
    first: Option<Frame>,
}

impl WeathexDecoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        let lane = |baud: f64, pn9: bool| -> Result<Lane, String> {
            Ok(Lane {
                slicer: FskSlicer::new(input_rate, freq_offset_hz, baud, Threshold::Midrange)?,
                pn9,
                sr: 0,
                frame: None,
            })
        };
        Ok(Self { lanes: [lane(4800.0, false)?, lane(5000.0, true)?], first: None })
    }
}

fn symbol(lane: &mut Lane, r: u8) -> Option<Frame> {
    if let Some(f) = lane.frame.as_mut() {
        f.bits.push(r ^ u8::from(f.invert));
        if f.bits.len() == (FRAME_BYTES * 8) - HEADER_BITS {
            let f = lane.frame.take().unwrap();
            let sync = if lane.pn9 { SYNC_PN9 } else { SYNC_PLAIN };
            let mut bytes = [0u8; FRAME_BYTES];
            let head = PREAMBLE << 16 | sync;
            for (j, b) in bytes.iter_mut().enumerate().take(5) {
                *b = (head >> (8 * (4 - j))) as u8;
            }
            for (j, c) in f.bits.chunks(8).enumerate() {
                bytes[5 + j] = c.iter().fold(0u8, |a, &x| (a << 1) | x);
            }
            return parse(&bytes, lane.pn9);
        }
        return None;
    }
    lane.sr = ((lane.sr << 1) | u64::from(r)) & ((1 << HEADER_BITS) - 1);
    let sync = if lane.pn9 { SYNC_PN9 } else { SYNC_PLAIN };
    let head = PREAMBLE << 16 | sync;
    let mask = (1u64 << HEADER_BITS) - 1;
    let straight = (lane.sr ^ head).count_ones() <= HEADER_TOLERANCE;
    let inverted = (lane.sr ^ !head & mask).count_ones() <= HEADER_TOLERANCE;
    if straight || inverted {
        lane.frame = Some(Collect { bits: Vec::with_capacity(FRAME_BYTES * 8), invert: !straight });
    }
    None
}

impl ModelDecoder for WeathexDecoder {
    fn model(&self) -> SondeModel {
        SondeModel::Weathex
    }

    fn process(&mut self, input: &[Complex32]) -> Vec<SondeFix> {
        let mut out = Vec::new();
        for li in 0..2 {
            let mut syms = Vec::new();
            self.lanes[li].slicer.process(input, |b| syms.push(b));
            for s in syms {
                let Some(f) = symbol(&mut self.lanes[li], s) else { continue };
                if f.id == 1 {
                    self.first = Some(f);
                } else if let (Some(a), Some(place)) = (self.first, f.place)
                    && a.serial == f.serial
                    && a.counter == f.counter
                    && let Some(pos) = pos_of(place)
                {
                    out.push(SondeFix {
                        model: SondeModel::Weathex,
                        serial: format!("WXR-{}", f.serial),
                        frame_num: u32::from(f.counter),
                        battery_v: None,
                        meteo: Meteo::default(),
                        pos: Some(pos),
                    });
                }
            }
        }
        out
    }

    fn level_dbfs(&self) -> f32 {
        self.lanes[0].slicer.level_dbfs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame of the given variant: `id` 1 or 2, then the data.
    fn frame(pn9: bool, serial: u32, counter: u16, id: u8) -> [u8; FRAME_BYTES] {
        let ofs = if pn9 { 8 } else { 6 };
        let mut x = [0u8; FRAME_BYTES];
        let head = PREAMBLE << 16 | if pn9 { SYNC_PN9 } else { SYNC_PLAIN };
        for (j, b) in x.iter_mut().enumerate().take(5) {
            *b = (head >> (8 * (4 - j))) as u8;
        }
        x[ofs..ofs + 4].copy_from_slice(&serial.to_le_bytes());
        x[ofs + 4..ofs + 6].copy_from_slice(&counter.to_le_bytes());
        x[ofs + 6] = id;
        if id == 2 {
            // 12:20:37, 12616.6 m, 2.6785 N, 101.5827 E
            x[ofs + 7..ofs + 10].copy_from_slice(&122_037u32.to_le_bytes()[..3]);
            let bits: u128 = 126_166u128 << 4 | 267_850u128 << 23 | 10_158_270u128 << 48;
            for k in 0..10 {
                x[ofs + 13 + k] = (bits >> (8 * k)) as u8;
            }
        }
        let c = check(&x[ofs..ofs + DATA_BYTES]);
        x[ofs + DATA_BYTES] = (c >> 8) as u8;
        x[ofs + DATA_BYTES + 1] = c as u8;
        if pn9 {
            for (j, b) in x.iter_mut().enumerate().skip(6) {
                *b ^= PN9[(j - 6) % 64];
            }
        }
        x
    }

    #[test]
    fn a_frame_gives_serial_time_and_place() {
        for pn9 in [false, true] {
            let f = parse(&frame(pn9, 12_345_678, 4400, 2), pn9).expect("a frame");
            assert_eq!((f.serial, f.counter, f.id), (12_345_678, 4400, 2));
            let (secs, lat, lon, alt) = f.place.unwrap();
            assert_eq!(secs, 12 * 3600 + 20 * 60 + 37);
            assert!(
                (lat - 2.6785).abs() < 1e-9
                    && (lon - 101.5827).abs() < 1e-9
                    && (alt - 12_616.6).abs() < 1e-9
            );
        }
    }

    #[test]
    fn a_wrong_byte_fails_the_check() {
        let mut b = frame(false, 1, 1, 2);
        b[20] ^= 4;
        assert!(parse(&b, false).is_none());
    }

    fn bits_of(bytes: &[u8]) -> Vec<u8> {
        bytes.iter().flat_map(|&b| (0..8).rev().map(move |k| b >> k & 1)).collect()
    }

    fn modulate(sym: &[u8], baud: f64, rate: f64, invert: bool) -> Vec<Complex32> {
        let spb = rate / baud;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if (r == 1) != invert { 8000.0 } else { -8000.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    #[test]
    fn synthetic_bursts_are_decoded_in_both_variants_rates_and_polarities() {
        for (pn9, baud) in [(false, 4800.0), (true, 5000.0)] {
            for rate in [48_000.0, 96_000.0] {
                for invert in [false, true] {
                    let mut bytes = Vec::new();
                    for n in 0..3u16 {
                        bytes.extend([0xAA; 3]);
                        bytes.extend(frame(pn9, 12_345_678, 4400 + n, 1));
                        bytes.extend([0xAA; 3]);
                        bytes.extend(frame(pn9, 12_345_678, 4400 + n, 2));
                    }
                    let mut iq = vec![Complex32::new(0.0, 0.0); 2000];
                    iq.extend(modulate(&bits_of(&bytes), baud, rate, invert));
                    iq.extend(vec![Complex32::new(0.0, 0.0); 2000]);
                    let mut d = WeathexDecoder::new(rate, 0.0).unwrap();
                    let mut fixes = Vec::new();
                    for chunk in iq.chunks(1000) {
                        fixes.extend(d.process(chunk));
                    }
                    assert!(
                        fixes.len() >= 2,
                        "{} fixes, pn9 {pn9} at {rate}, invert {invert}",
                        fixes.len()
                    );
                    assert_eq!(fixes[0].serial, "WXR-12345678");
                    assert!((fixes[0].pos.as_ref().unwrap().alt_m - 12_616.6).abs() < 1e-9);
                }
            }
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = WeathexDecoder::new(96_000.0, 0.0).unwrap();
        let mut x = 31337u32;
        let noise: Vec<Complex32> = (0..96_000 * 4)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                let a = (x >> 8) as f32 / 8_388_608.0 - 1.0;
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                Complex32::new(a, (x >> 8) as f32 / 8_388_608.0 - 1.0)
            })
            .collect();
        assert!(d.process(&noise).is_empty());
    }
}
