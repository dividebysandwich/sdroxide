//! Lockheed Martin Sippican LMS6-1680 / Mk2a decoder.
//!
//! The signal: FSK at 9600 baud, 8N1 characters (a start bit 0, eight data
//! bits least significant first, a stop bit 1), no error correction. A burst
//! starts with the bytes `CA CA CA 24 52`, then a sub-frame type: `4D` (68
//! bytes) or `54` (174 bytes), each ending with a CRC-16 (CCITT, start value
//! 0, over everything before it, the `24 52` included, big-endian). Both carry
//! the 16-bit sonde id and a frame counter; `54` has the GPS time of week in
//! milliseconds, latitude and longitude as 32-bit binary angles (2^32/360 per
//! degree), altitude in millimetres and three 24-bit velocities (east, north,
//! up) in mm/s; `4D` may carry the full 32-bit id.
//!
//! Provenance: the layout is the public description of the Mk2a telemetry by
//! rs1729/RS (`mk2a/mk2a.txt`, `demod/mod/mk2a1680mod.c`). No code of
//! auto_rx, SDRangel or rs1729 (GPL) is used. Checked on the real sample
//! sub-frames of that description (CRC, id, place, altitude and velocities
//! read sensibly); not on a radio recording, the signal is at 1680 MHz with a
//! wide deviation.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 9616.0;
/// `CA CA CA 24 52` as 8N1 characters, in the order sent.
const HEADER: u64 = 0b0010100111_0010100111_0010100111_0001001001_0010010101;
const HEADER_BITS: u32 = 50;
const HEADER_MASK: u64 = (1 << HEADER_BITS) - 1;
const HEADER_TOLERANCE: u32 = 5;
/// Bytes of the `4D` and `54` sub-frames, the CRC included.
const LEN_4D: usize = 68;
const LEN_54: usize = 174;
/// Binary angle: 2^32 / 360 per degree, truncated as the sonde does.
const B60B60: f64 = 0xB6_0B60 as f64;

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

fn be16(b: &[u8]) -> u16 {
    u16::from(b[0]) << 8 | u16::from(b[1])
}

fn be32(b: &[u8]) -> i32 {
    (u32::from(b[0]) << 24 | u32::from(b[1]) << 16 | u32::from(b[2]) << 8 | u32::from(b[3])) as i32
}

fn be24(b: &[u8]) -> i32 {
    let v = i32::from(b[0]) << 16 | i32::from(b[1]) << 8 | i32::from(b[2]);
    if v > 0x7F_FFFF { v - 0x100_0000 } else { v }
}

/// What a sub-frame says. `frame` starts with `24 52`.
enum Sub {
    /// The short id and, when the sub-frame carries it, the full 32-bit id.
    Id(u16, Option<u32>),
    Fix(SondeFix),
}

fn parse(frame: &[u8], full_id: Option<(u16, u32)>) -> Option<Sub> {
    let kind = *frame.get(2)?;
    let len = match kind {
        0x4D => LEN_4D,
        0x54 => LEN_54,
        _ => return None,
    };
    let frame = frame.get(..len)?;
    if be16(&frame[len - 2..]) != crc16(&frame[..len - 2]) {
        return None;
    }
    let short = be16(&frame[4..6]);
    if kind == 0x4D {
        // The id repeated, then its two upper bytes; blank (0F 0F) when unknown.
        let full = (be16(&frame[50..52]) == short && be16(&frame[52..54]) != 0x0F0F)
            .then(|| u32::from(frame[52]) << 24 | u32::from(frame[53]) << 16 | u32::from(short));
        return Some(Sub::Id(short, full));
    }
    let lat = f64::from(be32(&frame[18..22])) / B60B60;
    let lon = f64::from(be32(&frame[22..26])) / B60B60;
    let alt = f64::from(be32(&frame[26..30])) / 1000.0;
    let ve = f64::from(be24(&frame[30..33])) / 1000.0;
    let vn = f64::from(be24(&frame[33..36])) / 1000.0;
    let vu = f64::from(be24(&frame[36..39])) / 1000.0;
    let ok = (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
        && (-500.0..60_000.0).contains(&alt);
    let pos = ok.then(|| GpsPos {
        lat,
        lon,
        alt_m: alt,
        speed_ms: ve.hypot(vn),
        course_deg: (ve.atan2(vn).to_degrees() + 360.0) % 360.0,
        climb_ms: vu,
        num_sv: 0,
    });
    let serial = match full_id {
        Some((s, id)) if s == short => format!("LMS6-{id}"),
        _ => format!("Mk2a-{short}"),
    };
    Some(Sub::Fix(SondeFix {
        model: SondeModel::Mk2a,
        serial,
        frame_num: u32::from(be16(&frame[6..8])),
        battery_v: None,
        meteo: Meteo::default(),
        pos,
    }))
}

/// The byte of one 8N1 character (`bits[0]` is the start bit).
fn byte_of(bits: &[u8]) -> u8 {
    (0..8).fold(0u8, |a, k| a | (bits[1 + k] << k))
}

struct Collect {
    bits: Vec<u8>,
    invert: bool,
}

pub struct Mk2aDecoder {
    slicer: FskSlicer,
    sr: u64,
    frame: Option<Collect>,
    /// The latest full id read from a `4D` sub-frame: (short id, id).
    full_id: Option<(u16, u32)>,
}

impl Mk2aDecoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        let slicer = FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?;
        Ok(Self { slicer, sr: 0, frame: None, full_id: None })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        if let Some(f) = self.frame.as_mut() {
            f.bits.push(r ^ u8::from(f.invert));
            // The first character after the header is the sub-frame type.
            if f.bits.len() >= 10 {
                let total = match byte_of(&f.bits[..10]) {
                    0x4D => LEN_4D,
                    0x54 => LEN_54,
                    _ => 0,
                };
                if total == 0 {
                    self.frame = None;
                    return;
                }
                if f.bits.len() == (total - 2) * 10 {
                    let f = self.frame.take().unwrap();
                    let mut bytes = vec![0x24, 0x52];
                    bytes.extend(f.bits.chunks(10).map(byte_of));
                    match parse(&bytes, self.full_id) {
                        Some(Sub::Id(short, Some(id))) => self.full_id = Some((short, id)),
                        Some(Sub::Id(..)) | None => {}
                        Some(Sub::Fix(fix)) => out.push(fix),
                    }
                }
            }
            return;
        }
        self.sr = ((self.sr << 1) | u64::from(r)) & HEADER_MASK;
        let straight = (self.sr ^ HEADER).count_ones() <= HEADER_TOLERANCE;
        let inverted = (self.sr ^ (!HEADER & HEADER_MASK)).count_ones() <= HEADER_TOLERANCE;
        if straight || inverted {
            self.frame = Some(Collect { bits: Vec::with_capacity(LEN_54 * 10), invert: !straight });
        }
    }
}

impl ModelDecoder for Mk2aDecoder {
    fn model(&self) -> SondeModel {
        SondeModel::Mk2a
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

    /// Two real sub-frames of the rs1729 description.
    const SUB_4D: &str = "24524D32EC6405D700380D03B9B019FA6711D5EC19FB711FF8570FFFFD1DFD7511F64D03B9C203B9CD03B9C603B9C803B9BF29E20F0F0F0F0F0F0F0F0F0F44FFA77B2E20";
    const SUB_54: &str = "24525432EC6405D700A2199437BBFFEE849221416082B857DAEF001BFD3E0007BDFFF9A0000EE800300A10280F140084355F471CFB328E072E0F14008307E0E21CECF44D0A2B0F14008BEE8C091CF61BC806260F140083AF54271CF308C308290F140091E740261CEAF7A20D2B0F140079F5EBD41CF4CEC813240F140089DB4BF11CEA7864172B0F14007EA015A51CF9BAC403270F1400806F4F861CEEA78A05260F1400986025991CEDEE47B4B5";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn the_crc_of_the_real_sub_frames_matches() {
        let a = unhex(SUB_4D);
        let b = unhex(SUB_54);
        assert_eq!((a.len(), b.len()), (LEN_4D, LEN_54));
        assert_eq!(crc16(&a[..LEN_4D - 2]), be16(&a[LEN_4D - 2..]));
        assert_eq!(crc16(&b[..LEN_54 - 2]), be16(&b[LEN_54 - 2..]));
    }

    #[test]
    fn a_real_position_sub_frame_gives_the_flight_values() {
        let Some(Sub::Fix(f)) = parse(&unhex(SUB_54), None) else { panic!("a fix") };
        assert_eq!(f.serial, "Mk2a-60516");
        assert_eq!(f.frame_num, 1495);
        let p = f.pos.expect("a place");
        assert!(
            (p.lat - 46.765379).abs() < 1e-6 && (p.lon + 100.767403).abs() < 1e-6,
            "{} {}",
            p.lat,
            p.lon
        );
        assert!((p.alt_m - 1834.302).abs() < 1e-9);
        assert!(p.speed_ms > 0.0 && (0.0..360.0).contains(&p.course_deg));
    }

    #[test]
    fn a_flipped_bit_fails_the_crc() {
        let mut b = unhex(SUB_54);
        b[20] ^= 1;
        assert!(parse(&b, None).is_none());
    }

    #[test]
    fn an_id_sub_frame_without_a_full_id_gives_nothing_to_remember() {
        assert!(matches!(parse(&unhex(SUB_4D), None), Some(Sub::Id(0xEC64, None))));
    }

    /// The characters of `bytes`, 8N1, least significant bit first.
    fn symbols(bytes: &[u8]) -> Vec<u8> {
        bytes
            .iter()
            .flat_map(|&b| {
                std::iter::once(0).chain((0..8).map(move |k| b >> k & 1)).chain(std::iter::once(1))
            })
            .collect()
    }

    fn modulate(sym: &[u8], rate: f64, invert: bool) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if (r == 1) != invert { 3500.0 } else { -3500.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    #[test]
    fn a_synthetic_burst_is_decoded_at_both_rates_and_both_polarities() {
        for rate in [48_000.0, 96_000.0] {
            for invert in [false, true] {
                let mut bytes = vec![0xCA; 4];
                bytes.extend([0xCA, 0xCA, 0xCA]);
                bytes.extend(unhex(SUB_4D));
                bytes.extend([0xCA; 4]);
                for _ in 0..3 {
                    bytes.extend([0xCA, 0xCA, 0xCA]);
                    bytes.extend(unhex(SUB_54));
                    bytes.extend([0xCA; 3]);
                }
                let mut iq = vec![Complex32::new(0.0, 0.0); 3000];
                iq.extend(modulate(&symbols(&bytes), rate, invert));
                iq.extend(vec![Complex32::new(0.0, 0.0); 3000]);
                let mut d = Mk2aDecoder::new(rate, 0.0).unwrap();
                let mut fixes = Vec::new();
                for chunk in iq.chunks(1000) {
                    fixes.extend(d.process(chunk));
                }
                assert!(fixes.len() >= 2, "{} fixes at {rate}, invert {invert}", fixes.len());
                assert_eq!(fixes[0].serial, "Mk2a-60516");
                assert!((fixes[0].pos.as_ref().unwrap().alt_m - 1834.302).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = Mk2aDecoder::new(48_000.0, 0.0).unwrap();
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
