//! Meteo-Radiy MRZ (MP3-H1) decoder.
//!
//! The signal: FSK at 2399 symbols/s, Manchester coded (a pair `10` is a 1, a
//! pair `01` a 0). A frame starts with the bytes `AA BF 35` and is 50 bytes,
//! most significant bit first, little-endian numbers: sub-frame counter
//! (low nibble of byte 3), time of day as hour, minute, second (bytes 4 to 6),
//! the position as ECEF X, Y, Z in 1/100 m (bytes 8, 12, 16), the ECEF
//! velocity in 1/100 m/s (bytes 20, 22, 24), the satellite count (26), a
//! configuration word (44 to 47) whose meaning depends on the sub-frame, and
//! a CRC-16 (reflected polynomial 0xA001, start 0xFFFF) over bytes 3 to 47
//! stored at 48 and 49. Sub-frames 0xC and 0xD carry the two halves of the
//! serial number. Latitude, longitude and altitude are computed from ECEF.
//!
//! Provenance: the layout is the public description of the MRZ telemetry by
//! rs1729/RS. Checked on a real recording (projecthorus sample set, a flight
//! near Kaliningrad at 14 km, 19 satellites): the frames that pass the CRC
//! give the time, a steady position and speed, and the serial. No code of
//! auto_rx, SDRangel or rs1729 (GPL) is used.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::{GpsPos, ecef_to_geodetic};

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 2399.0;
const HEADER: u32 = 0x00AA_BF35;
const HEADER_TOLERANCE: u32 = 1;
const FRAME_BYTES: usize = 50;
/// Bits that follow the 24 header bits.
const BODY_BITS: usize = (FRAME_BYTES - 3) * 8;
const MAX_VIOLATIONS: u32 = 8;

fn crc16(data: &[u8]) -> u16 {
    let mut c: u16 = 0xFFFF;
    for &b in data {
        c ^= u16::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xA001 } else { c >> 1 };
        }
    }
    c
}

fn le_i32(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn le_i16(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}

/// Position and velocity from the ECEF fields of a frame.
fn position(b: &[u8]) -> Option<GpsPos> {
    let (x, y, z) = (
        f64::from(le_i32(b, 8)) / 100.0,
        f64::from(le_i32(b, 12)) / 100.0,
        f64::from(le_i32(b, 16)) / 100.0,
    );
    if x == 0.0 && y == 0.0 && z == 0.0 {
        return None;
    }
    let (lat, lon, alt) = ecef_to_geodetic(x, y, z);
    if !(lat.is_finite() && lon.is_finite() && alt.is_finite())
        || !(-1000.0..80_000.0).contains(&alt)
    {
        return None;
    }
    let (vx, vy, vz) = (
        f64::from(le_i16(b, 20)) / 100.0,
        f64::from(le_i16(b, 22)) / 100.0,
        f64::from(le_i16(b, 24)) / 100.0,
    );
    let (la, lo) = (lat.to_radians(), lon.to_radians());
    let ve = -lo.sin() * vx + lo.cos() * vy;
    let vn = -la.sin() * lo.cos() * vx - la.sin() * lo.sin() * vy + la.cos() * vz;
    let vu = la.cos() * lo.cos() * vx + la.cos() * lo.sin() * vy + la.sin() * vz;
    Some(GpsPos {
        lat,
        lon,
        alt_m: alt,
        speed_ms: ve.hypot(vn),
        course_deg: ve.atan2(vn).to_degrees().rem_euclid(360.0),
        climb_ms: vu,
        num_sv: b[26],
    })
}

/// What the frames said so far.
#[derive(Default)]
struct Assembler {
    sn_c: Option<u32>,
    sn_d: Option<u32>,
}

impl Assembler {
    /// A checked frame; a fix comes out once the serial is known (its two
    /// halves arrive in sub-frames 0xC and 0xD, about every 5 s).
    fn frame(&mut self, b: &[u8], out: &mut Vec<SondeFix>) {
        let word = u32::from_le_bytes([b[44], b[45], b[46], b[47]]);
        match b[3] & 0x0F {
            0xC => self.sn_c = Some(word),
            0xD => self.sn_d = Some(word),
            _ => {}
        }
        let (Some(c), Some(d)) = (self.sn_c, self.sn_d) else { return };
        out.push(SondeFix {
            model: SondeModel::Mrz,
            serial: format!("MRZ-{c}-{d}"),
            frame_num: u32::from(b[43]),
            battery_v: None,
            meteo: Meteo::default(),
            pos: position(b),
        });
    }
}

struct Collect {
    bits: Vec<u8>,
    invert: bool,
    violations: u32,
}

pub struct MrzDecoder {
    slicer: FskSlicer,
    raw_n: u64,
    last_raw: u8,
    sr: [u32; 2],
    frames: [Option<Collect>; 2],
    asm: Assembler,
}

impl MrzDecoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        // This signal is slow and noisy: smooth the discriminator more.
        let slicer = FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?
            .with_smoothing(7.0 / 96_000.0, input_rate);
        Ok(Self {
            slicer,
            raw_n: 0,
            last_raw: 0,
            sr: [0; 2],
            frames: [None, None],
            asm: Assembler::default(),
        })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        self.raw_n += 1;
        if self.raw_n >= 2 {
            let q = ((self.raw_n - 2) & 1) as usize;
            let violation = self.last_raw == r;
            // `10` is a 1, `01` a 0.
            let d = self.last_raw;
            self.data_bit(q, d, violation, out);
        }
        self.last_raw = r;
    }

    fn data_bit(&mut self, q: usize, d: u8, violation: bool, out: &mut Vec<SondeFix>) {
        if let Some(f) = self.frames[q].as_mut() {
            f.bits.push(d ^ u8::from(f.invert));
            f.violations += u32::from(violation);
            if f.violations > MAX_VIOLATIONS {
                self.frames[q] = None;
            } else if f.bits.len() == BODY_BITS {
                let f = self.frames[q].take().unwrap();
                self.finish(&f.bits, out);
            }
            return;
        }
        self.sr[q] = (self.sr[q] << 1) | u32::from(d);
        if violation {
            return;
        }
        let w = self.sr[q] & 0x00FF_FFFF;
        let inv = (w ^ HEADER).count_ones() > HEADER_TOLERANCE
            && (w ^ !HEADER & 0x00FF_FFFF).count_ones() <= HEADER_TOLERANCE;
        if (w ^ HEADER).count_ones() <= HEADER_TOLERANCE || inv {
            self.frames[q] =
                Some(Collect { bits: Vec::with_capacity(BODY_BITS), invert: inv, violations: 0 });
        }
    }

    fn finish(&mut self, bits: &[u8], out: &mut Vec<SondeFix>) {
        let mut b = [0u8; FRAME_BYTES];
        b[0] = 0xAA;
        b[1] = 0xBF;
        b[2] = 0x35;
        for i in 3..FRAME_BYTES {
            b[i] = bits[8 * (i - 3)..8 * (i - 2)].iter().fold(0u8, |a, &x| (a << 1) | x);
        }
        if crc16(&b[3..48]) != u16::from_le_bytes([b[48], b[49]]) {
            return;
        }
        self.asm.frame(&b, out);
    }
}

impl ModelDecoder for MrzDecoder {
    fn model(&self) -> SondeModel {
        SondeModel::Mrz
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

    /// Sub-frames 7, 12 and 13 of the real recording.
    const SUB7: &str = "aabf35870c1112001e1d941484a9fc07874af31ed7002c055b0013010c8be8fc05ffff22a12a005ba60d00084336773f5513";
    const SUB12: &str = "aabf358c0c111c00a02b9414e5dffc07034ff31e9a017302d20013010c7ce8fb05ffff80ae2a006da50d000d23160000f633";
    const SUB13: &str = "aabf358d0c111e00d52d9414c4e6fc075652f31e8d0045054e0113010c7fe8fc05ffffb4ab2a0007a70d000ef3980000aa5f";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn real_frames_pass_their_crc() {
        for h in [SUB7, SUB12, SUB13] {
            let f = unhex(h);
            assert_eq!(f.len(), FRAME_BYTES);
            assert_eq!(crc16(&f[3..48]), u16::from_le_bytes([f[48], f[49]]), "{h}");
        }
    }

    #[test]
    fn a_flipped_bit_fails_the_crc() {
        let mut f = unhex(SUB7);
        f[10] ^= 0x01;
        assert_ne!(crc16(&f[3..48]), u16::from_le_bytes([f[48], f[49]]));
    }

    #[test]
    fn real_frames_give_serial_and_a_place_once_both_halves_are_known() {
        let mut a = Assembler::default();
        let mut out = Vec::new();
        a.frame(&unhex(SUB7), &mut out);
        assert!(out.is_empty(), "no serial yet, no fix");
        a.frame(&unhex(SUB12), &mut out);
        a.frame(&unhex(SUB13), &mut out);
        a.frame(&unhex(SUB7), &mut out);
        let f = out.last().expect("a fix");
        assert_eq!(f.serial, "MRZ-5667-39155");
        let p = f.pos.as_ref().expect("a place");
        assert!(
            (p.lat - 54.684).abs() < 0.01 && (p.lon - 21.212).abs() < 0.01,
            "{} {}",
            p.lat,
            p.lon
        );
        assert!((p.alt_m - 14_038.0).abs() < 100.0);
        assert_eq!(p.num_sv, 19);
        assert!(p.speed_ms > 5.0 && p.speed_ms < 30.0);
    }

    fn with_crc(mut f: Vec<u8>) -> Vec<u8> {
        let c = crc16(&f[3..48]);
        f[48..50].copy_from_slice(&c.to_le_bytes());
        f
    }

    /// Manchester: a 1 is `10`, a 0 is `01`.
    fn symbols(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for &byte in bytes {
            for k in (0..8).rev() {
                let bit = byte >> k & 1;
                out.push(bit);
                out.push(bit ^ 1);
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
            let f = if r == 1 { 3000.0 } else { -3000.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    fn frame(sub: u8, word: u32) -> Vec<u8> {
        let mut f = unhex(SUB7);
        f[3] = 0x80 | sub;
        f[44..48].copy_from_slice(&word.to_le_bytes());
        with_crc(f)
    }

    #[test]
    fn a_synthetic_burst_gives_serial_and_place() {
        for rate in [48_000.0, 96_000.0] {
            let mut bytes = vec![0xAA, 0xAA, 0xAA, 0xAA];
            for (sub, word) in [(0xCu8, 5667u32), (0xD, 39_155), (7, 0), (7, 0)] {
                bytes.extend(frame(sub, word));
                bytes.extend([0xAA, 0xAA]);
            }
            let mut iq = vec![Complex32::new(0.0, 0.0); 3000];
            iq.extend(modulate(&symbols(&bytes), rate));
            iq.extend(vec![Complex32::new(0.0, 0.0); 3000]);
            let mut d = MrzDecoder::new(rate, 0.0).unwrap();
            let mut fixes = Vec::new();
            for chunk in iq.chunks(1000) {
                fixes.extend(d.process(chunk));
            }
            assert!(!fixes.is_empty(), "no fix at {rate}");
            assert_eq!(fixes[0].serial, "MRZ-5667-39155");
            let p = fixes[0].pos.as_ref().expect("a place");
            assert!((p.lat - 54.684).abs() < 0.01);
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = MrzDecoder::new(48_000.0, 0.0).unwrap();
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
