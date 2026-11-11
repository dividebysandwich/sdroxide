//! Meteomodem M10 decoder.
//!
//! The signal: FSK at 9616 symbols/s, biphase-mark coded (a transition at the
//! start of every bit, another in the middle of a 1), so a data bit is a pair
//! of symbols that are equal (0) or different (1), and the polarity of the
//! channel does not matter. A frame is 102 bytes, most significant bit first:
//! `64 9F 20`, then telemetry, the sonde serial at 0x5D, a 16-bit checksum
//! at 0x63 over the 99 bytes before it. Several bytes are big-endian
//! integers (latitude and longitude in 2^32/360 degrees, altitude in mm,
//! velocities in 1/200 m/s).
//!
//! Provenance: the layout and the checksum are the public description of the
//! M10 telemetry by rs1729/RS. What is checked on air here is the signal
//! chain, the framing and the checksum: all 132 frames of a real 120 s
//! recording (projecthorus sample set) pass the checksum, with the header
//! and the serial number constant. That recording is a sonde with no GPS
//! fix, so the position fields are decoded as described but **not** seen with
//! a real position. No code of auto_rx, SDRangel or rs1729 (GPL) is used.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 9616.0;
/// 32 raw symbols that decode to the bytes `00 F8`, then the frame starts.
const SYNC: u32 = 0b1100_1100_1100_1100_1010_0110_0100_1100;
/// Bit errors tolerated in the sync word (the checksum is the real gate).
const SYNC_TOLERANCE: u32 = 2;
const FRAME_BYTES: usize = 102;
const CHECKED: usize = 0x63;

/// The frame checksum: one 16-bit update per byte, starting from 0.
pub fn checksum(data: &[u8]) -> u16 {
    let mut c: u16 = 0;
    for &x in data {
        let mut b = x.rotate_right(1);
        b ^= b >> 2;
        let bit = |n: u16| (c >> n) & 1;
        let t6 = bit(0) ^ bit(2) ^ bit(4);
        let t7 = bit(1) ^ bit(3) ^ bit(5);
        let t = (c & 0x3F) | (t6 << 6) | (t7 << 7);
        let mut s = ((c >> 7) & 0xFF) as u8;
        s ^= s >> 2;
        let c0 = u16::from(b ^ s) ^ t;
        c = ((c & 0xFF) << 8) | (c0 & 0xFF);
    }
    c
}

fn be_i32(b: &[u8], o: usize) -> i32 {
    i32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn be_i16(b: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([b[o], b[o + 1]])
}

/// The serial as printed on the sonde, digits only.
fn serial(b: &[u8]) -> String {
    let v = u16::from(b[0x60]) | (u16::from(b[0x61]) << 8);
    format!(
        "M10-{:X}{:02}{:X}{}{:04}",
        b[0x5F] >> 4,
        b[0x5F] & 0x0F,
        b[0x5D] & 0x0F,
        (v >> 13) & 7,
        v & 0x1FFF
    )
}

/// A position the telemetry can really mean: a sonde whose GPS has no fix
/// sends its defaults (latitude 90 degrees, longitude 0).
fn position(b: &[u8]) -> Option<GpsPos> {
    let scale = f64::from(0x00B6_0B60u32);
    let lat = f64::from(be_i32(b, 0x0E)) / scale;
    let lon = f64::from(be_i32(b, 0x12)) / scale;
    let alt = f64::from(be_i32(b, 0x16)) / 1000.0;
    let ok = lat.abs() < 89.9
        && lon.abs() <= 180.0
        && !(lat.abs() < 1e-6 && lon.abs() < 1e-6)
        && (-500.0..60_000.0).contains(&alt);
    if !ok {
        return None;
    }
    let ve = f64::from(be_i16(b, 0x04)) / 200.0;
    let vn = f64::from(be_i16(b, 0x06)) / 200.0;
    let vu = f64::from(be_i16(b, 0x08)) / 200.0;
    Some(GpsPos {
        lat,
        lon,
        alt_m: alt,
        speed_ms: ve.hypot(vn),
        course_deg: ve.atan2(vn).to_degrees().rem_euclid(360.0),
        climb_ms: vu,
        num_sv: 0,
    })
}

pub struct M10Decoder {
    slicer: FskSlicer,
    sr: u32,
    /// Raw symbols of the frame being collected.
    raw: Option<Vec<u8>>,
}

impl M10Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        Ok(Self {
            slicer: FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?,
            sr: 0,
            raw: None,
        })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        if let Some(raw) = self.raw.as_mut() {
            raw.push(r);
            if raw.len() == FRAME_BYTES * 16 {
                let raw = self.raw.take().unwrap();
                if let Some(fix) = decode_frame(&raw) {
                    out.push(fix);
                }
            }
            return;
        }
        self.sr = (self.sr << 1) | u32::from(r);
        if (self.sr ^ SYNC).count_ones() <= SYNC_TOLERANCE
            || (self.sr ^ !SYNC).count_ones() <= SYNC_TOLERANCE
        {
            self.raw = Some(Vec::with_capacity(FRAME_BYTES * 16));
        }
    }
}

/// Raw symbols (two per bit) to a checked frame.
fn decode_frame(raw: &[u8]) -> Option<SondeFix> {
    let mut b = [0u8; FRAME_BYTES];
    for (i, byte) in b.iter_mut().enumerate() {
        for k in 0..8 {
            let j = 16 * i + 2 * k;
            *byte = (*byte << 1) | u8::from(raw[j] != raw[j + 1]);
        }
    }
    let stored = u16::from_be_bytes([b[CHECKED], b[CHECKED + 1]]);
    if checksum(&b[..CHECKED]) != stored || b[0] != 0x64 {
        return None;
    }
    // 0x9F is the M10 with position; other types (the "-dop" variant) carry
    // other data and are not decoded.
    if b[1] != 0x9F {
        return None;
    }
    Some(SondeFix {
        model: SondeModel::M10,
        serial: serial(&b),
        frame_num: u32::from(b[0x62]),
        battery_v: None,
        meteo: Meteo::default(),
        pos: position(&b),
    })
}

impl ModelDecoder for M10Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::M10
    }

    fn process(&mut self, input: &[Complex32]) -> Vec<SondeFix> {
        let mut out = Vec::new();
        let mut syms = Vec::new();
        self.slicer.process(input, |b| syms.push(b));
        for s in syms {
            self.symbol(s, &mut out);
        }
        out
    }

    fn level_dbfs(&self) -> f32 {
        self.slicer.level_dbfs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frame 0 of the real recording (no GPS fix): checksum, header, serial.
    const REAL: &str = "649f2000000000000000000046503ffffff400000000000249f00000000500120800000000000000000000000000a77bc6719997092a8c09010027240000001aa5f209f2096303125a0b0100070013000000000000e50f1a008201ff00021483dc229e017c01";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn a_real_frame_passes_its_checksum() {
        let f = unhex(REAL);
        assert_eq!(f.len(), FRAME_BYTES);
        assert_eq!(checksum(&f[..CHECKED]), u16::from_be_bytes([f[CHECKED], f[CHECKED + 1]]));
    }

    #[test]
    fn a_flipped_bit_fails_the_checksum() {
        let mut f = unhex(REAL);
        f[0x20] ^= 0x10;
        assert_ne!(checksum(&f[..CHECKED]), u16::from_be_bytes([f[CHECKED], f[CHECKED + 1]]));
    }

    #[test]
    fn a_real_frame_gives_its_serial_and_no_position() {
        let f = unhex(REAL);
        assert_eq!(serial(&f), "M10-803210732");
        assert!(position(&f).is_none(), "defaults of a sonde without a GPS fix are not a place");
    }

    /// The sync word as it is on air, then the frame as biphase-mark: the
    /// level changes at the start of every bit and again in the middle of a 1
    /// (the data bit only needs the two symbols to be equal or different).
    fn symbols(bytes: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = (0..32).rev().map(|i| ((SYNC >> i) & 1) as u8).collect();
        let mut level = *out.last().unwrap();
        for &byte in bytes {
            for k in (0..8).rev() {
                level ^= 1;
                out.push(level);
                if byte >> k & 1 == 1 {
                    level ^= 1;
                }
                out.push(level);
            }
        }
        out
    }

    fn modulate(sym: &[u8], rate: f64) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if r == 1 { 5000.0 } else { -5000.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    fn frame_with_position(counter: u8) -> Vec<u8> {
        let mut f = vec![0u8; FRAME_BYTES];
        f[..3].copy_from_slice(&[0x64, 0x9F, 0x20]);
        let lat = (48.85 * f64::from(0x00B6_0B60u32)) as i32;
        let lon = (2.35 * f64::from(0x00B6_0B60u32)) as i32;
        f[0x0E..0x12].copy_from_slice(&lat.to_be_bytes());
        f[0x12..0x16].copy_from_slice(&lon.to_be_bytes());
        f[0x16..0x1A].copy_from_slice(&12_345_000i32.to_be_bytes());
        f[0x04..0x06].copy_from_slice(&((3.0f64 * 200.0) as i16).to_be_bytes());
        f[0x06..0x08].copy_from_slice(&((4.0f64 * 200.0) as i16).to_be_bytes());
        f[0x08..0x0A].copy_from_slice(&((5.0f64 * 200.0) as i16).to_be_bytes());
        f[0x5D..0x62].copy_from_slice(&[0x02, 0x14, 0x83, 0xDC, 0x22]);
        f[0x62] = counter;
        let c = checksum(&f[..CHECKED]);
        f[CHECKED..CHECKED + 2].copy_from_slice(&c.to_be_bytes());
        f
    }

    fn burst(counter: u8, rate: f64) -> Vec<Complex32> {
        let mut iq = vec![Complex32::new(0.0, 0.0); 3000];
        iq.extend(modulate(&symbols(&frame_with_position(counter)), rate));
        iq.extend(vec![Complex32::new(0.0, 0.0); 3000]);
        iq
    }

    #[test]
    fn a_synthetic_burst_gives_serial_and_position() {
        for rate in [48_000.0, 96_000.0] {
            let mut d = M10Decoder::new(rate, 0.0).unwrap();
            let mut fixes = Vec::new();
            for counter in 1..4u8 {
                for chunk in burst(counter, rate).chunks(1000) {
                    fixes.extend(d.process(chunk));
                }
            }
            assert!(!fixes.is_empty(), "no fix at {rate}");
            let f = &fixes[0];
            assert_eq!(f.model, SondeModel::M10);
            let p = f.pos.as_ref().expect("a position");
            assert!((p.lat - 48.85).abs() < 1e-5 && (p.lon - 2.35).abs() < 1e-5);
            assert!((p.alt_m - 12_345.0).abs() < 1e-6);
            assert!((p.speed_ms - 5.0).abs() < 1e-6, "3 east and 4 north make 5");
            assert!((p.climb_ms - 5.0).abs() < 1e-6);
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = M10Decoder::new(48_000.0, 0.0).unwrap();
        let mut x = 777u32;
        let noise: Vec<Complex32> = (0..48_000 * 4)
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
