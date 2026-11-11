//! Aerospace Newsky CF-06-AH and Huayuntianyi HT03G / GTH3 decoder.
//!
//! The signal: FSK at 2400 baud, plain bits, least significant bit first, in
//! frames of 109 bytes that start with the preamble `AA AA AA AA AA`, the
//! word `2D D4 63` and two more bytes `7F FF`. The two sondes share this and
//! are told apart by the next two bytes (`AA AA`: CF-06-AH, otherwise
//! HT03G). Offsets below count from the `2D` (byte 5).
//!
//! * CF-06-AH: serial as four bytes, each taken modulo 100, at +7; GPS time
//!   of week in milliseconds (32 bits, little-endian) at +11; longitude and
//!   latitude as 32-bit integers in 1e-7 degrees at +15 and +19; altitude in
//!   millimetres at +23; east and north velocity and (negated) climb as 16
//!   bits in 1/100 m/s at +27, +29, +31. A CRC-16 (CCITT, start 0) over the
//!   40 bytes from +7 is stored big-endian at +47.
//! * HT03G: serial as 32 bits big-endian at +7; time at +12 (hour, minute,
//!   and tenths of seconds on 10 bits); longitude and latitude as float32 in
//!   radians at +16 and +20; altitude in decimetres at +24; north, east and
//!   up velocity at +28, +30, +32; temperature at +84 and humidity at +88 in
//!   1/100, pressure at +90 on 24 bits in 1/100 hPa; a counter at +97
//!   (big-endian). A reflected CRC-16 (0x8408, start 0) over the 97 bytes
//!   from +3 is stored little-endian at +100.
//!
//! Provenance: the layout is the public description of the CF-06-AH and
//! HT03G telemetry by rs1729/RS (`demod/mod/cf06ht03mod.c`, UAII2022
//! Lindenberg). No code of auto_rx, SDRangel or rs1729 (GPL) is used. Checked
//! on synthetic frames built from that description (both types, 48 and 96
//! kS/s, both polarities, CRC); not on a radio recording. The Reed-Solomon
//! repair of the CF-06-AH frames is not done: a damaged frame is dropped.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 2400.0;
const FRAME_BYTES: usize = 109;
const HEADER_BITS: usize = 64;
const HEADER_TOLERANCE: u32 = 4;
/// Offset of the `2D D4 63` word.
const OFS: usize = 5;
/// The header as a bit stream, `AA AA AA AA AA 2D D4 63` least significant bit first.
const HEADER: u64 = 0b01010101_01010101_01010101_01010101_01010101_10110100_00101011_11000110;

fn crc16(data: &[u8]) -> u16 {
    let mut c: u16 = 0;
    for &b in data {
        c ^= u16::from(b) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
    }
    c
}

fn crc16_rev(data: &[u8]) -> u16 {
    let mut c: u16 = 0;
    for &b in data {
        c ^= u16::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0x8408 } else { c >> 1 };
        }
    }
    c
}

fn le_i32(b: &[u8]) -> i32 {
    i32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn le_i16(b: &[u8]) -> i16 {
    i16::from_le_bytes([b[0], b[1]])
}

fn plausible(p: &GpsPos) -> bool {
    (-90.0..=90.0).contains(&p.lat)
        && (-180.0..=180.0).contains(&p.lon)
        && (-500.0..60_000.0).contains(&p.alt_m)
        && (p.lat != 0.0 || p.lon != 0.0)
}

fn cf06(f: &[u8; FRAME_BYTES]) -> Option<SondeFix> {
    let d = &f[OFS..];
    if u16::from(d[47]) << 8 | u16::from(d[48]) != crc16(&d[7..47]) {
        return None;
    }
    let sn = d[7..11].iter().fold(0u32, |a, &b| a * 100 + u32::from(b) % 100);
    let tow = u32::from_le_bytes([d[11], d[12], d[13], d[14]]);
    let ve = f64::from(le_i16(&d[27..])) / 100.0;
    let vn = f64::from(le_i16(&d[29..])) / 100.0;
    let vu = -f64::from(le_i16(&d[31..])) / 100.0;
    let pos = GpsPos {
        lon: f64::from(le_i32(&d[15..])) / 1e7,
        lat: f64::from(le_i32(&d[19..])) / 1e7,
        alt_m: f64::from(le_i32(&d[23..])) / 1e3,
        speed_ms: ve.hypot(vn),
        course_deg: (ve.atan2(vn).to_degrees() + 360.0) % 360.0,
        climb_ms: vu,
        num_sv: 0,
    };
    (plausible(&pos) && (sn != 0 || tow != 0)).then(|| SondeFix {
        model: SondeModel::Cf06,
        serial: format!("CF6-{sn:08}"),
        frame_num: (tow + 500) / 1000,
        battery_v: None,
        meteo: Meteo::default(),
        pos: Some(pos),
    })
}

fn ht03(f: &[u8; FRAME_BYTES]) -> Option<SondeFix> {
    let d = &f[OFS..];
    if u16::from(d[100]) | u16::from(d[101]) << 8 != crc16_rev(&d[3..100]) {
        return None;
    }
    let sn = u32::from_be_bytes([d[7], d[8], d[9], d[10]]);
    let counter = u32::from(d[97]) << 8 | u32::from(d[98]);
    let rad = |b: &[u8]| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])).to_degrees();
    let vn = f64::from(le_i16(&d[28..])) / 100.0;
    let ve = f64::from(le_i16(&d[30..])) / 100.0;
    let vu = f64::from(le_i16(&d[32..])) / 100.0;
    let pos = GpsPos {
        lon: rad(&d[16..]),
        lat: rad(&d[20..]),
        alt_m: f64::from(le_i32(&d[24..])) / 10.0,
        speed_ms: ve.hypot(vn),
        course_deg: (ve.atan2(vn).to_degrees() + 360.0) % 360.0,
        climb_ms: vu,
        num_sv: 0,
    };
    if !plausible(&pos) || (counter == 0 && sn == 0) {
        return None;
    }
    let t = f32::from(le_i16(&d[84..])) / 100.0;
    let rh = f32::from(le_i16(&d[88..])) / 100.0;
    let p = (u32::from(d[90]) | u32::from(d[91]) << 8 | u32::from(d[92]) << 16) as f32 / 100.0;
    Some(SondeFix {
        model: SondeModel::Cf06,
        serial: format!("GTH-{sn:08X}"),
        frame_num: counter,
        battery_v: None,
        meteo: Meteo {
            temp_c: (-120.0..80.0).contains(&t).then_some(t),
            humidity_pct: (-0.5..110.0).contains(&rh).then(|| rh.max(0.0)),
            pressure_hpa: (p > 0.0 && p < 1100.0).then_some(p),
        },
        pos: Some(pos),
    })
}

fn fix_of(f: &[u8; FRAME_BYTES]) -> Option<SondeFix> {
    if f[OFS + 5] == 0xAA && f[OFS + 6] == 0xAA { cf06(f) } else { ht03(f) }
}

struct Collect {
    bits: Vec<u8>,
    invert: bool,
}

pub struct Cf06Decoder {
    slicer: FskSlicer,
    sr: u64,
    frame: Option<Collect>,
}

impl Cf06Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        Ok(Self {
            slicer: FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?,
            sr: 0,
            frame: None,
        })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        if let Some(c) = self.frame.as_mut() {
            c.bits.push(r ^ u8::from(c.invert));
            if c.bits.len() == FRAME_BYTES * 8 - HEADER_BITS {
                let c = self.frame.take().unwrap();
                let mut f = [0u8; FRAME_BYTES];
                // The header, least significant bit first, is the first eight bytes.
                for (j, b) in f.iter_mut().enumerate().take(8) {
                    *b = (0..8)
                        .fold(0u8, |a, k| a | ((HEADER >> (63 - (8 * j + k)) & 1) as u8) << k);
                }
                for (j, byte) in c.bits.chunks(8).enumerate() {
                    f[8 + j] = byte.iter().enumerate().fold(0u8, |a, (k, &x)| a | x << k);
                }
                out.extend(fix_of(&f));
            }
            return;
        }
        self.sr = (self.sr << 1) | u64::from(r);
        let straight = (self.sr ^ HEADER).count_ones() <= HEADER_TOLERANCE;
        let inverted = (self.sr ^ !HEADER).count_ones() <= HEADER_TOLERANCE;
        if straight || inverted {
            self.frame =
                Some(Collect { bits: Vec::with_capacity(FRAME_BYTES * 8), invert: !straight });
        }
    }
}

impl ModelDecoder for Cf06Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::Cf06
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

    fn base(ht: bool) -> [u8; FRAME_BYTES] {
        let mut f = [0u8; FRAME_BYTES];
        f[..8].copy_from_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0x2D, 0xD4, 0x63]);
        f[8] = 0x7F;
        f[9] = 0xFF;
        let (a, b) = if ht { (0xFF, 0xFF) } else { (0xAA, 0xAA) };
        f[OFS + 5] = a;
        f[OFS + 6] = b;
        f
    }

    fn cf06_frame() -> [u8; FRAME_BYTES] {
        let mut f = base(false);
        let d = &mut f[OFS..];
        d[7..11].copy_from_slice(&[12, 34, 56, 78]);
        d[11..15].copy_from_slice(&(45_000_000u32 + 123).to_le_bytes()); // 12:30:00.123 on day 0
        d[15..19].copy_from_slice(&(1_013_000_000i32).to_le_bytes()); // 101.3 E
        d[19..23].copy_from_slice(&(26_500_000i32).to_le_bytes()); // 2.65 N
        d[23..27].copy_from_slice(&(12_345_678i32).to_le_bytes()); // 12345.678 m
        d[27..29].copy_from_slice(&300i16.to_le_bytes());
        d[29..31].copy_from_slice(&400i16.to_le_bytes());
        d[31..33].copy_from_slice(&(-500i16).to_le_bytes());
        let c = crc16(&d[7..47]);
        d[47] = (c >> 8) as u8;
        d[48] = c as u8;
        f
    }

    fn ht03_frame() -> [u8; FRAME_BYTES] {
        let mut f = base(true);
        let d = &mut f[OFS..];
        d[7..11].copy_from_slice(&0x1234ABCDu32.to_be_bytes());
        d[16..20].copy_from_slice(&(101.5f32.to_radians()).to_le_bytes());
        d[20..24].copy_from_slice(&(2.5f32.to_radians()).to_le_bytes());
        d[24..28].copy_from_slice(&(150_000i32).to_le_bytes()); // 15000.0 m
        d[28..30].copy_from_slice(&300i16.to_le_bytes());
        d[30..32].copy_from_slice(&400i16.to_le_bytes());
        d[32..34].copy_from_slice(&(-250i16).to_le_bytes());
        d[84..86].copy_from_slice(&(-5230i16).to_le_bytes());
        d[88..90].copy_from_slice(&(4550i16).to_le_bytes());
        d[90..93].copy_from_slice(&(12_345u32).to_le_bytes()[..3]);
        d[97] = 0x01;
        d[98] = 0x2C;
        let c = crc16_rev(&d[3..100]);
        d[100] = c as u8;
        d[101] = (c >> 8) as u8;
        f
    }

    #[test]
    fn a_cf06_frame_gives_serial_place_and_velocity() {
        let f = fix_of(&cf06_frame()).expect("a fix");
        assert_eq!(f.serial, "CF6-12345678");
        assert_eq!(f.frame_num, 45_000);
        let p = f.pos.unwrap();
        assert!(
            (p.lon - 101.3).abs() < 1e-9
                && (p.lat - 2.65).abs() < 1e-9
                && (p.alt_m - 12_345.678).abs() < 1e-9
        );
        assert!((p.speed_ms - 5.0).abs() < 1e-9 && (p.climb_ms - 5.0).abs() < 1e-9);
    }

    #[test]
    fn a_ht03_frame_gives_place_and_weather() {
        let f = fix_of(&ht03_frame()).expect("a fix");
        assert_eq!(f.serial, "GTH-1234ABCD");
        assert_eq!(f.frame_num, 300);
        let p = f.pos.unwrap();
        assert!(
            (p.lon - 101.5).abs() < 1e-4
                && (p.lat - 2.5).abs() < 1e-4
                && (p.alt_m - 15_000.0).abs() < 1e-9
        );
        assert!((f.meteo.temp_c.unwrap() + 52.3).abs() < 1e-4);
        assert!((f.meteo.humidity_pct.unwrap() - 45.5).abs() < 1e-4);
        assert!((f.meteo.pressure_hpa.unwrap() - 123.45).abs() < 1e-4);
    }

    #[test]
    fn a_flipped_bit_fails_the_crc() {
        let mut f = cf06_frame();
        f[30] ^= 1;
        assert!(fix_of(&f).is_none());
        let mut f = ht03_frame();
        f[40] ^= 1;
        assert!(fix_of(&f).is_none());
    }

    /// Least significant bit first.
    fn bits_of(bytes: &[u8]) -> Vec<u8> {
        bytes.iter().flat_map(|&b| (0..8).map(move |k| b >> k & 1)).collect()
    }

    fn modulate(sym: &[u8], rate: f64, invert: bool) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if (r == 1) != invert { 1500.0 } else { -1500.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    #[test]
    fn synthetic_bursts_are_decoded_for_both_types_rates_and_polarities() {
        for (frame, prefix) in [(cf06_frame(), "CF6-"), (ht03_frame(), "GTH-")] {
            for rate in [48_000.0, 96_000.0] {
                for invert in [false, true] {
                    let mut bytes = vec![0xAA; 4];
                    for _ in 0..3 {
                        bytes.extend(frame);
                        bytes.extend([0xAA; 3]);
                    }
                    let mut iq = vec![Complex32::new(0.0, 0.0); 2000];
                    iq.extend(modulate(&bits_of(&bytes), rate, invert));
                    iq.extend(vec![Complex32::new(0.0, 0.0); 2000]);
                    let mut d = Cf06Decoder::new(rate, 0.0).unwrap();
                    let mut fixes = Vec::new();
                    for chunk in iq.chunks(1000) {
                        fixes.extend(d.process(chunk));
                    }
                    assert!(
                        fixes.len() >= 2,
                        "{} fixes, {prefix} at {rate}, invert {invert}",
                        fixes.len()
                    );
                    assert!(fixes[0].serial.starts_with(prefix));
                }
            }
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = Cf06Decoder::new(48_000.0, 0.0).unwrap();
        let mut x = 31337u32;
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
