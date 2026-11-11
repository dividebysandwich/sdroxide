//! Meisei RS-11G and iMS-100 decoder.
//!
//! The signal: FSK at 2400 symbols/s carrying a biphase-S code, so 1200 bits
//! per second: a bit is 1 when its two symbols are equal, 0 when they differ.
//! The bits make sub-frames of 300 bits, four a second, alternately starting
//! with the header `049DCE` and `FB6230` (24 bits each). After a header come
//! six blocks of 46 bits: two 16-bit words, each followed by an odd-parity bit
//! (1 when the word has an even number of ones), then 12 BCH check bits of a
//! (63, 51) code shortened to (46, 34), generator
//! x^12 + x^10 + x^8 + x^5 + x^4 + x^3 + 1, which corrects two wrong bits.
//!
//! A sub-frame holds twelve 16-bit words. The first one of a `049DCE`
//! sub-frame is a counter (one a half second). Words 2 and 3 hold one
//! configuration value, a 32-bit float, indexed by the counter modulo 64:
//! index 0, 16, 32 and 48 are the serial number, the temperature table and
//! the humidity polynomial follow in the other indexes. The `FB6230`
//! sub-frame of an even counter holds the position and velocity. Two
//! variants exist, told apart by the low byte of word 7 of the first sub-frame
//! (below `C0`: RS-11G, from `C0` on: iMS-100):
//!
//! * RS-11G: latitude and longitude as 32-bit integers in 1e-7 degrees,
//!   altitude 32 bits in centimetres, speed, heading and climb 16 bits in
//!   1/100 units; its float is an IEEE single with the exponent field shifted
//!   by one bit.
//! * iMS-100: latitude and longitude as NMEA `ddmm.mmmm` times 1e4, 32-bit
//!   signed, altitude 24 bits in centimetres, heading in 1/100 degree, speed
//!   in 1/194.384 m/s (knots times 100); the climb is in the odd counter.
//!   The twelfth word of the second sub-frame is the 16-bit sum of the words
//!   10 and 11 of the first and 0 to 10 of the second one.
//!
//! Provenance: the layout is the public description of the Meisei telemetry
//! by rs1729/RS (`meisei/meisei-rs.txt`, `demod/mod/meisei100mod.c`). No code
//! of auto_rx, SDRangel or rs1729 (GPL) is used. Checked on the sample words
//! of that description (both variants: serial, place, altitude, velocities,
//! the iMS-100 checksum on a flight recorded as words); the BCH and the
//! biphase layer follow the description and are tested on synthetic bursts,
//! not on a radio recording.

use std::sync::OnceLock;

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 2400.0;
/// The two headers as raw symbols (biphase-S encoded), 48 of them each.
const RAW_HEADER_A: u64 = 0b101010101011_010100101011_001101001100_101011001101;
const RAW_HEADER_B: u64 = 0b110011001101_001101001101_010100101010_110010101010;
const RAW_MASK: u64 = (1 << 48) - 1;
const HEADER_TOLERANCE: u32 = 4;
/// Bits of a sub-frame after its header, and the raw symbols they take.
const PAYLOAD_BITS: usize = 6 * 46;
const PAYLOAD_RAW: usize = 2 * PAYLOAD_BITS;
/// The BCH generator.
const GEN: u32 = 0x1539;

// ---- BCH (46, 34) ---------------------------------------------------------

/// The 12-bit remainder of a 46-bit word (the first bit received is the
/// highest power) divided by the generator.
fn remainder(mut cw: u64) -> u16 {
    for i in (12..46).rev() {
        if cw >> i & 1 == 1 {
            cw ^= u64::from(GEN) << (i - 12);
        }
    }
    (cw & 0xFFF) as u16
}

/// For each non-zero remainder, the one or two bit powers that produce it.
fn syndrome_table() -> &'static [(u8, u8); 4096] {
    static TABLE: OnceLock<[(u8, u8); 4096]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [(0xFFu8, 0xFFu8); 4096];
        let s: Vec<u16> = (0..46).map(|i| remainder(1u64 << i)).collect();
        for i in 0..46 {
            t[usize::from(s[i])] = (i as u8, 0xFF);
        }
        for i in 0..46 {
            for j in i + 1..46 {
                let e = &mut t[usize::from(s[i] ^ s[j])];
                if *e == (0xFF, 0xFF) {
                    *e = (i as u8, j as u8);
                }
            }
        }
        t
    })
}

/// Repairs up to two wrong bits of a block; returns the word and how many
/// bits were wrong, or `None` when it cannot be repaired.
fn bch_repair(cw: u64) -> Option<(u64, u32)> {
    let syn = remainder(cw);
    if syn == 0 {
        return Some((cw, 0));
    }
    let (a, b) = syndrome_table()[usize::from(syn)];
    if a == 0xFF {
        return None;
    }
    let mut fixed = cw ^ (1u64 << a);
    let mut n = 1;
    if b != 0xFF {
        fixed ^= 1u64 << b;
        n = 2;
    }
    Some((fixed, n))
}

/// The twelve words of a sub-frame from its 276 payload bits: the BCH repair
/// and the parity bits checked. Also the number of repaired bits.
fn words_of(bits: &[u8]) -> Option<([u16; 12], u32)> {
    let mut words = [0u16; 12];
    let mut fixed_bits = 0;
    for b in 0..6 {
        let blk = &bits[46 * b..46 * b + 46];
        let cw = blk.iter().fold(0u64, |a, &x| (a << 1) | u64::from(x));
        let (cw, n) = bch_repair(cw)?;
        fixed_bits += n;
        let bit = |j: usize| (cw >> (45 - j) & 1) as u16;
        for half in 0..2 {
            let at = 17 * half;
            let w = (0..16).fold(0u16, |a, k| (a << 1) | bit(at + k));
            let parity = bit(at + 16);
            // The parity bit is 1 when the word has an even number of ones.
            if parity != (w.count_ones() as u16 & 1) ^ 1 {
                return None;
            }
            words[2 * b + half] = w;
        }
    }
    Some((words, fixed_bits))
}

// ---- Telemetry ------------------------------------------------------------

/// The float of the RS-11G: an IEEE single whose exponent field sits one bit
/// lower, divided by four.
fn f32_rs11g(num: u32) -> f32 {
    let v = (num & 0x80_0000) << 8 | (num >> 1) & 0x7F80_0000 | num & 0x7F_FFFF;
    f32::from_bits(v) / 4.0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Variant {
    Rs11g,
    Ims100,
}

impl Variant {
    fn prefix(self) -> &'static str {
        match self {
            Variant::Rs11g => "RS11G",
            Variant::Ims100 => "IMS100",
        }
    }
}

/// Configuration values sent one a half second, 64 of them in turn.
struct Config {
    value: [f32; 64],
    valid: u64,
    f_ref: u16,
    serial: Option<f32>,
}

impl Config {
    fn new() -> Self {
        Self { value: [0.0; 64], valid: 0, f_ref: 0, serial: None }
    }

    fn has(&self, range: std::ops::RangeInclusive<usize>) -> bool {
        range.into_iter().all(|i| self.valid >> i & 1 == 1)
    }

    /// Temperature from the raw reading: a frequency ratio turned into a
    /// resistance, then interpolated in the table of the sonde.
    fn temperature(&self, v: Variant, raw: u16) -> Option<f32> {
        // (offset of the four resistance terms, of the table of resistances,
        // table length) in the configuration.
        let (c0, r0, n) = match v {
            Variant::Rs11g => (33, 37, 11),
            Variant::Ims100 => (53, 33, 12),
        };
        let ok = self.has(17..=17 + n - 1)
            && self.has(r0..=r0 + n - 1)
            && self.has(c0..=c0 + 3)
            && self.f_ref != 0;
        if !ok {
            return None;
        }
        let r = &self.value[r0..r0 + n];
        let t = &self.value[17..17 + n];
        // Resistances must increase and temperatures decrease.
        if !r.windows(2).all(|w| w[0] < w[1]) || r[0] <= 0.0 || !t.windows(2).all(|w| w[0] > w[1]) {
            return None;
        }
        let f = f32::from(raw) / f32::from(self.f_ref) * 4.0;
        if f <= 1.0 {
            return None;
        }
        let f = 1.0 / (f - 1.0);
        let res = self.value[c0] + self.value[c0 + 1] * f + self.value[c0 + 2] * f * f
            - self.value[c0 + 3];
        let temp = if res <= r[0] {
            t[0]
        } else if res >= r[n - 1] {
            t[n - 1]
        } else {
            let j = (0..n - 1).find(|&j| res < r[j + 1])?;
            let x = (res.ln() - r[j].ln()) / (r[j + 1].ln() - r[j].ln());
            t[j] - x * (t[j] - t[j + 1])
        };
        temp.is_finite().then_some(temp)
    }

    fn humidity(&self, raw: u16) -> Option<f32> {
        if !self.has(49..=52) || self.f_ref == 0 {
            return None;
        }
        let f = f32::from(raw) / f32::from(self.f_ref) * 4.0;
        let rh = self.value[49]
            + self.value[50] * f
            + self.value[51] * f * f
            + self.value[52] * f * f * f;
        rh.is_finite().then(|| rh.clamp(0.0, 100.0))
    }
}

/// The first sub-frame of a pair, kept until the second one arrives.
struct First {
    words: [u16; 12],
    variant: Variant,
    meteo: Meteo,
}

fn word32(hi: u16, lo: u16) -> u32 {
    u32::from(hi) << 16 | u32::from(lo)
}

/// The position of a second sub-frame of an even counter.
fn position(v: Variant, b: &[u16; 12]) -> Option<GpsPos> {
    let (lat, lon, alt, speed, course, climb) = match v {
        Variant::Rs11g => (
            f64::from(word32(b[1], b[2]) as i32) / 1e7,
            f64::from(word32(b[3], b[4]) as i32) / 1e7,
            f64::from(word32(b[5], b[6])) / 100.0,
            f64::from(b[7]) / 100.0,
            f64::from(b[8]) / 100.0,
            f64::from(b[9] as i16) / 100.0,
        ),
        Variant::Ims100 => {
            let nmea = |v: i32| {
                let deg = (f64::from(v) / 1e6).trunc();
                deg + (f64::from(v) / 1e6 - deg) * 100.0 / 60.0
            };
            (
                nmea(word32(b[1], b[2]) as i32),
                nmea(word32(b[3], b[4]) as i32),
                f64::from(u32::from(b[5]) << 8 | u32::from(b[6] >> 8)) / 100.0,
                f64::from(b[10]) / 194.384,
                f64::from(b[9]) / 100.0,
                0.0,
            )
        }
    };
    let ok = (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
        && (-500.0..60_000.0).contains(&alt)
        && (0.0..=360.0).contains(&course)
        && (lat != 0.0 || lon != 0.0);
    ok.then_some(GpsPos {
        lat,
        lon,
        alt_m: alt,
        speed_ms: speed,
        course_deg: course,
        climb_ms: climb,
        num_sv: 0,
    })
}

// ---- Decoder --------------------------------------------------------------

struct Collect {
    raw: Vec<u8>,
    second: bool,
}

pub struct MeiseiDecoder {
    slicer: FskSlicer,
    sr: u64,
    frame: Option<Collect>,
    config: Config,
    /// The first sub-frame just before, with its counter.
    first: Option<(u16, First)>,
    /// The climb rate of the last odd counter (iMS-100).
    climb: Option<f64>,
}

impl MeiseiDecoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        Ok(Self {
            slicer: FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Mean)?,
            sr: 0,
            frame: None,
            config: Config::new(),
            first: None,
            climb: None,
        })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        if let Some(f) = self.frame.as_mut() {
            f.raw.push(r);
            if f.raw.len() == PAYLOAD_RAW {
                let f = self.frame.take().unwrap();
                let bits: Vec<u8> = f.raw.chunks(2).map(|p| u8::from(p[0] == p[1])).collect();
                if let Some((words, fixed)) = words_of(&bits) {
                    self.subframe(f.second, words, fixed, out);
                }
            }
            return;
        }
        self.sr = ((self.sr << 1) | u64::from(r)) & RAW_MASK;
        let near = |h: u64| {
            let d = (self.sr ^ h).count_ones();
            d.min(48 - d) <= HEADER_TOLERANCE
        };
        let (a, b) = (near(RAW_HEADER_A), near(RAW_HEADER_B));
        if a || b {
            self.frame = Some(Collect { raw: Vec::with_capacity(PAYLOAD_RAW), second: !a });
        }
    }

    fn subframe(&mut self, second: bool, w: [u16; 12], fixed: u32, out: &mut Vec<SondeFix>) {
        if !second {
            let counter = w[0];
            let variant = if w[7] & 0xFF >= 0xC0 { Variant::Ims100 } else { Variant::Rs11g };
            if fixed == 0 {
                let value = match variant {
                    Variant::Ims100 => f32::from_bits(word32(w[3], w[2])),
                    Variant::Rs11g => f32_rs11g(word32(w[3].swap_bytes(), w[2].swap_bytes())),
                };
                let i = usize::from(counter % 64);
                if value.is_finite() {
                    self.config.value[i] = value;
                    self.config.valid |= 1 << i;
                    if counter.is_multiple_of(16) && value > 0.0 && value < 1e9 {
                        self.config.serial = Some(value);
                    }
                }
                match (variant, counter % 4) {
                    (_, 0) => self.config.f_ref = w[1],
                    (Variant::Ims100, 3) => self.config.f_ref = w[6],
                    _ => {}
                }
            }
            let mut meteo = Meteo::default();
            if counter.is_multiple_of(2) {
                let (t_raw, u_raw) = (w[5], w[6]);
                meteo.temp_c = self.config.temperature(variant, t_raw);
                meteo.humidity_pct = self.config.humidity(u_raw);
            }
            self.first = Some((counter, First { words: w, variant, meteo }));
            return;
        }
        let Some((counter, first)) = self.first.take() else { return };
        let v = first.variant;
        if v == Variant::Ims100 {
            // The checksum of the position block.
            let sum =
                first.words[10..12].iter().chain(&w[..11]).fold(0u16, |a, &x| a.wrapping_add(x));
            if sum != w[11] {
                return;
            }
            if !counter.is_multiple_of(2) {
                self.climb = (w[1] != 0).then(|| f64::from(w[1] as i16) / 19.4384);
                return;
            }
        } else if !counter.is_multiple_of(2) {
            return;
        }
        let Some(mut pos) = position(v, &w) else { return };
        if v == Variant::Ims100 {
            pos.climb_ms = self.climb.unwrap_or(0.0);
        }
        let Some(sn) = self.config.serial else { return };
        out.push(SondeFix {
            model: SondeModel::Meisei,
            serial: format!("{}-{sn:.0}", v.prefix()),
            frame_num: u32::from(counter),
            battery_v: None,
            meteo: first.meteo,
            pos: Some(pos),
        });
    }
}

impl ModelDecoder for MeiseiDecoder {
    fn model(&self) -> SondeModel {
        SondeModel::Meisei
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

    fn arr(s: &str) -> [u16; 12] {
        let v: Vec<u16> =
            s.split_whitespace().map(|x| u16::from_str_radix(x, 16).unwrap()).collect();
        v.try_into().unwrap()
    }

    /// Words of the RS-11G samples of the description: an even counter and
    /// its position sub-frame.
    const RS_A0: &str = "1C66 7FDD 8F53 7C81 0000 4F20 7646 30A2 0000 0000 1004 0436";
    const RS_B0: &str = "8080 1F39 5FFE 08A7 6540 000F E01D 0C2C 1E75 0250 06DE 0A07";

    /// A real iMS-100 flight (Indonesia, 2019), the words of the sub-frames:
    /// first, second, first, second of each second.
    const IMS: &[&str] = &[
        "241E 800A 0000 0000 0000 28D1 6111 30C1 0000 0000 55F0 0018",
        "2F25 FF7E FDED 06DB B388 1AB2 1A1C C462 2480 3F8E 0789 A1C2",
        "241F 2D78 0000 0000 0000 28CF 8F37 31C1 0000 0000 5515 0800",
        "0700 006C 0300 0103 080B 0E10 1216 171A 1B1F 20C1 C2C3 A672",
        "2420 8F37 F386 4AD5 0000 28D3 6111 30C1 0000 0044 59D8 0018",
        "2F25 FF7E FDC5 06DB B390 1AB4 721C C462 2480 402E 06FE FDA1",
        "2421 0000 AFF7 3F2C 0000 28DF 6111 31C1 F526 0000 5915 0800",
        "0700 006E 0301 161A 4626 260B 2821 1F02 351A 1232 1F2E 9B6C",
        "2422 800B 0A13 3FA5 0000 28E3 6110 30C1 0000 0000 5DC0 0018",
        "2F25 FF7E FD9B 06DB B3A8 1AB6 841C C462 2480 3FB6 068F 1292",
        "2423 2D7F 9DF1 3FC4 0000 28E1 8F37 31C1 0000 0000 5D15 0800",
        "0700 006C 0302 657A 8C67 4703 5B6E 9F0F 0327 484E 0D12 FB6B",
    ];

    fn ims(i: usize) -> [u16; 12] {
        arr(IMS[i])
    }

    #[test]
    fn the_generator_is_the_product_of_the_two_factors() {
        // (x^6+x^4+x^2+x+1)(x^6+x+1)
        let (a, b) = (0b101_0111u32, 0b100_0011u32);
        assert_eq!(
            GEN,
            (0..7).fold(0, |acc, i| if b >> i & 1 == 1 { acc ^ (a << i) } else { acc })
        );
    }

    #[test]
    fn up_to_two_wrong_bits_of_a_block_are_repaired() {
        let msg = 0x2_3456_789Au64;
        let cw = msg << 12 | u64::from(remainder(msg << 12));
        assert_eq!(remainder(cw), 0);
        assert_eq!(bch_repair(cw), Some((cw, 0)));
        assert_eq!(bch_repair(cw ^ (1 << 20)), Some((cw, 1)));
        assert_eq!(bch_repair(cw ^ (1 << 20) ^ (1 << 3)), Some((cw, 2)));
        // Three wrong bits are either refused or repaired into another word.
        assert_ne!(bch_repair(cw ^ 0b1_0010_0100), Some((cw, 3)));
    }

    #[test]
    fn the_rs11g_float_has_its_exponent_one_bit_lower() {
        // 1.0 is 3F800000; stored (times four) its exponent sits one bit lower.
        let v = (1.0f32 * 4.0).to_bits();
        let num = (v & 0x7F_FFFF) | (v >> 31) << 23 | (v >> 23 & 0xFF) << 24;
        assert!((f32_rs11g(num) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_rs11g_position_block_gives_the_flight_values() {
        let p = position(Variant::Rs11g, &arr(RS_B0)).expect("a place");
        assert!((p.lat - 52.4).abs() < 0.1 && (p.lon - 14.5).abs() < 0.1, "{} {}", p.lat, p.lon);
        assert!((p.alt_m - 10_404.13).abs() < 1.0, "{}", p.alt_m);
        assert!(
            (p.speed_ms - 31.16).abs() < 0.01
                && (p.course_deg - 77.97).abs() < 0.01
                && (p.climb_ms - 5.92).abs() < 0.01
        );
    }

    #[test]
    fn the_variant_is_told_by_word_seven() {
        assert_eq!(arr(RS_A0)[7] & 0xFF, 0xA2);
        assert!(ims(0)[7] & 0xFF >= 0xC0 && ims(2)[7] & 0xFF >= 0xC0);
    }

    #[test]
    fn the_checksum_of_the_real_ims100_position_block_matches() {
        for k in 0..3 {
            let (a, b) = (ims(4 * k), ims(4 * k + 1));
            let sum = a[10..12].iter().chain(&b[..11]).fold(0u16, |s, &x| s.wrapping_add(x));
            assert_eq!(sum, b[11], "pair {k}");
        }
    }

    #[test]
    fn a_real_ims100_position_block_gives_a_flight_over_bali() {
        let p = position(Variant::Ims100, &ims(1)).expect("a place");
        assert!(
            (p.lat + 8.7578).abs() < 0.001 && (p.lon - 115.2).abs() < 1.0,
            "{} {}",
            p.lat,
            p.lon
        );
        assert!((p.alt_m - 17_495.3).abs() < 1.0, "{}", p.alt_m);
        assert!(
            (p.course_deg - 162.7).abs() < 0.01 && (p.speed_ms - 9.92).abs() < 0.1,
            "{} {}",
            p.course_deg,
            p.speed_ms
        );
    }

    /// The serial of the real flight, in the float of the counter 0x2420.
    #[test]
    fn the_serial_of_the_real_ims100_is_a_seven_digit_float() {
        let sn = f32::from_bits(word32(ims(4)[3], ims(4)[2]));
        assert!((6.9e6..7.1e6).contains(&sn), "{sn}");
    }

    /// The calibration of the real flight (cfg 17..28 temperatures, 33..44
    /// resistances, 49..56 humidity and thermistor terms) and one reading at
    /// 17.7 km, where the air is about -81 C and dry.
    #[test]
    fn the_real_ims100_calibration_gives_a_stratospheric_temperature() {
        let mut c = Config::new();
        let t =
            [60.0, 40.04, 35.05, 29.98, 20.07, 0.0, -10.06, -20.03, -29.99, -39.98, -64.96, -84.92];
        let r = [
            0.67456, 1.28937, 1.53607, 1.84266, 2.67276, 6.07194, 9.52729, 15.33, 25.3537, 43.5006,
            195.855, 727.169,
        ];
        let k = [-2029.53, 3581.07, -1750.91, 260.8, -36.49026, 99.65264, 0.757661, -2.908408];
        c.value[17..29].copy_from_slice(&t);
        c.value[33..45].copy_from_slice(&r);
        c.value[49..57].copy_from_slice(&k);
        c.valid = u64::MAX;
        c.f_ref = 36662;
        let temp = c.temperature(Variant::Ims100, 10782).expect("a temperature");
        assert!((temp + 80.8).abs() < 0.3, "{temp}");
        let rh = c.humidity(24813).expect("a humidity");
        assert!((rh - 7.34).abs() < 0.1, "{rh}");
        // Without the reference frequency nothing is read.
        c.f_ref = 0;
        assert!(c.temperature(Variant::Ims100, 10782).is_none() && c.humidity(24813).is_none());
    }

    // ---- synthetic bursts ----

    /// The 46 bits of a block from two words (each followed by its parity).
    fn block(w1: u16, w2: u16) -> Vec<u8> {
        let p = |w: u16| u64::from((w.count_ones() as u16 & 1) ^ 1);
        let msg = u64::from(w1) << 18 | p(w1) << 17 | u64::from(w2) << 1 | p(w2);
        let cw = msg << 12 | u64::from(remainder(msg << 12));
        (0..46).map(|j| (cw >> (45 - j) & 1) as u8).collect()
    }

    /// The raw symbols of one sub-frame: header, then the biphase-S payload.
    fn raw(words: &[u16; 12], second: bool, level: &mut u8) -> Vec<u8> {
        let h = if second { RAW_HEADER_B } else { RAW_HEADER_A };
        let mut out: Vec<u8> = (0..48).rev().map(|k| (h >> k & 1) as u8).collect();
        *level = *out.last().unwrap();
        for b in 0..6 {
            for bit in block(words[2 * b], words[2 * b + 1]) {
                let first = *level ^ 1;
                let last = if bit == 1 { first } else { first ^ 1 };
                out.extend([first, last]);
                *level = last;
            }
        }
        out
    }

    fn modulate(sym: &[u8], rate: f64, invert: bool) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if (r == 1) != invert { 2000.0 } else { -2000.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    fn run(
        d: &mut MeiseiDecoder,
        subframes: &[([u16; 12], bool)],
        rate: f64,
        invert: bool,
    ) -> Vec<SondeFix> {
        let mut sym: Vec<u8> = (0..400).map(|k| (k % 2) as u8).collect();
        let mut level = 0;
        for (w, second) in subframes {
            sym.extend(raw(w, *second, &mut level));
        }
        sym.extend((0..200).map(|k| (k % 2) as u8));
        let mut iq = vec![Complex32::new(0.0, 0.0); 2000];
        iq.extend(modulate(&sym, rate, invert));
        let mut fixes = Vec::new();
        for chunk in iq.chunks(1000) {
            fixes.extend(d.process(chunk));
        }
        fixes
    }

    #[test]
    fn a_real_ims100_flight_is_decoded_from_a_synthetic_signal() {
        for rate in [48_000.0, 96_000.0] {
            for invert in [false, true] {
                let subs: Vec<([u16; 12], bool)> = (0..12).map(|i| (ims(i), i % 2 == 1)).collect();
                let mut d = MeiseiDecoder::new(rate, 0.0).unwrap();
                let fixes = run(&mut d, &subs, rate, invert);
                assert!(!fixes.is_empty(), "no fix at {rate}, invert {invert}");
                assert!(fixes[0].serial.starts_with("IMS100-70"), "{}", fixes[0].serial);
                assert_eq!(fixes[0].frame_num, 0x2420);
                let p = fixes[0].pos.as_ref().unwrap();
                assert!((p.alt_m - 17_501.3).abs() < 1.0, "{}", p.alt_m);
            }
        }
    }

    #[test]
    fn a_rs11g_burst_gives_a_fix_once_the_serial_is_known() {
        // A first sub-frame at a counter multiple of 16 carries the serial;
        // 12345 as the float of the RS-11G.
        let v = (12345.0f32 * 4.0).to_bits();
        let num = (v & 0x7F_FFFF) | (v >> 31) << 23 | (v >> 23 & 0xFF) << 24;
        let mut a = arr(RS_A0);
        a[0] = 0x1C60;
        a[3] = ((num >> 16) as u16).swap_bytes();
        a[2] = (num as u16).swap_bytes();
        let subs = [(a, false), (arr(RS_B0), true)];
        let mut d = MeiseiDecoder::new(48_000.0, 0.0).unwrap();
        let fixes = run(&mut d, &subs, 48_000.0, false);
        assert_eq!(fixes.len(), 1, "{fixes:?}");
        assert_eq!(fixes[0].serial, "RS11G-12345");
        assert!((fixes[0].pos.as_ref().unwrap().alt_m - 10_404.13).abs() < 1.0);
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = MeiseiDecoder::new(48_000.0, 0.0).unwrap();
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
