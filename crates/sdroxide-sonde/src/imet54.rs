//! InterMet iMet-54 decoder.
//!
//! The signal: FSK at 4798 baud carrying 8N1 characters. A frame starts with
//! the characters `00 AA 24 24 24 24 42` (the first four are the correlation
//! header, the last three still belong to the sync) and holds 2200 bits, that
//! is 220 characters. Dropping the start and stop bits leaves 1760 data bits;
//! after the three sync characters they are taken in blocks of 64 bits, each
//! block transposed as an 8 x 8 matrix, and every 8 bits are an extended
//! Hamming [8,4] word that gives one nibble. Two nibbles make a byte, so a
//! frame is 108 bytes, big-endian numbers: serial (0x00), time of day as
//! `hhmmssmmm` in decimal (0x04), latitude and longitude as `ddmm.mmmmmm` in
//! millionths (0x08, 0x0C), altitude in decimetres (0x10), the air temperature
//! as a float (0x1C) and a status word (0x2A) whose bits 4 and 5 tell the
//! position is valid. The check is a 32-bit sum over the first 108 bytes, the
//! two 16-bit halves of it stored at 100 and 106.
//!
//! Provenance: the layout is the public decoder by rs1729/RS
//! (`demod/mod/imet54mod.c`). No code of auto_rx, SDRangel or rs1729 (GPL) is
//! used. Checked on a real recording (a sonde on the ground near Johannesburg):
//! the frames that pass the check give a steady place, a counting clock and a
//! steady temperature. The humidity fields are not read: the recording gives
//! unusable values in them.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 4798.0;
/// `00 AA 24 24` as 8N1 characters.
const HEADER: &str = "0000000001010101010100010010010001001001";
const HEADER_TOLERANCE: u32 = 4;
const FRAME_BITS: usize = 2200;
const FRAME_BYTES: usize = 108;
/// Bits dropped after the header: the characters `24 24 42`.
const SYNC_BITS: usize = 24;
const MARKER_NO_VALUE: u32 = 0x4E6E_6B28;

/// The extended Hamming [8,4] words, by nibble.
const HAM: [u8; 16] = [
    0x00, 0x87, 0x99, 0x1E, 0xAA, 0x2D, 0x33, 0xB4, 0x4B, 0xCC, 0xD2, 0x55, 0xE1, 0x66, 0x78, 0xFF,
];

fn header_bits() -> u64 {
    HEADER.bytes().fold(0u64, |a, c| (a << 1) | u64::from(c - b'0'))
}

/// The nibble of a word, repairing one wrong bit.
fn nibble(word: u8) -> Option<u8> {
    HAM.iter().position(|&h| (h ^ word).count_ones() <= 1).map(|n| n as u8)
}

/// 8N1 characters, the data bits only.
fn drop_framing(bits: &[u8]) -> Vec<u8> {
    bits.iter().enumerate().filter(|(i, _)| matches!(i % 10, 1..=8)).map(|(_, &b)| b).collect()
}

/// Each block of 64 bits transposed as an 8 x 8 matrix (a transposition is its
/// own inverse).
fn transpose64(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bits.len()];
    for (n, block) in bits.as_chunks::<64>().0.iter().enumerate() {
        for i in 0..8 {
            for j in 0..8 {
                out[64 * n + 8 * j + i] = block[8 * i + j];
            }
        }
    }
    out
}

/// The bytes of a frame from its 2200 bits after the header.
fn bytes_of(bits: &[u8]) -> Option<[u8; FRAME_BYTES]> {
    if bits.len() < FRAME_BITS {
        return None;
    }
    let data = drop_framing(&bits[..FRAME_BITS]);
    let data = &data[SYNC_BITS..];
    let data = transpose64(&data[..data.len() / 64 * 64]);
    let mut out = [0u8; FRAME_BYTES];
    for (k, byte) in out.iter_mut().enumerate() {
        let mut v = 0u8;
        for h in 0..2 {
            let w = &data[8 * (2 * k + h)..8 * (2 * k + h) + 8];
            let word = w.iter().enumerate().fold(0u8, |a, (b, &x)| a | (x << b));
            v = (v << 4) | nibble(word)?;
        }
        *byte = v;
    }
    Some(out)
}

fn checksum_ok(b: &[u8; FRAME_BYTES]) -> bool {
    let (poly0, poly1) = (0x0EDBu32, 0x8260u32);
    let (mut c0, mut c1) = (0x48EBu32, 0x1ACAu32);
    let (mut crc0, mut crc1) = (0u32, 0u32);
    let data0 = u32::from(b[100]) << 8 | u32::from(b[101]);
    let data1 = u32::from(b[106]) << 8 | u32::from(b[107]);
    let (mut n, mut bit) = (104usize, 0u32);
    loop {
        if (n < 100 || (n > 101 && n < 106)) && (b[n] >> bit) & 1 == 1 {
            crc0 ^= c0;
            crc1 ^= c1;
        }
        let (mut nx0, mut nx1) = (c0, c1);
        if c1 & 0x8000 != 0 {
            nx0 ^= poly0;
            nx1 ^= poly1;
        }
        nx0 <<= 1;
        nx1 <<= 1;
        if c1 & 0x8000 != 0 {
            nx0 |= 1;
        }
        if (c1 ^ c0) & 0x8000 != 0 {
            nx1 |= 1;
        }
        c0 = nx0 & 0xFFFF;
        c1 = nx1;
        if bit < 7 {
            bit += 1;
        } else {
            bit = 0;
            if n % 4 == 3 {
                match n.checked_sub(7) {
                    Some(m) => n = m,
                    None => break,
                }
            } else {
                n += 1;
            }
        }
    }
    crc0 ^= data0 ^ 0x5000;
    crc1 ^= data1 ^ 0x1DAD;
    crc1 == 0 && (crc0 & 0xF000) == 0
}

fn be_i32(b: &[u8], o: usize) -> i32 {
    i32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Degrees from `ddmm.mmmmmm` in millionths.
fn degrees(v: i32) -> f64 {
    let a = i64::from(v).unsigned_abs();
    let deg = (a / 1_000_000) as f64;
    let min = (a % 1_000_000) as f64 / 10_000.0;
    let d = deg + min / 60.0;
    if v < 0 { -d } else { d }
}

/// A checked frame.
fn fix_of(b: &[u8; FRAME_BYTES]) -> Option<SondeFix> {
    let sn = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let t = be_i32(b, 0x04);
    if !(0..=235_959_999).contains(&t) {
        return None;
    }
    let frame_num = (t / 10_000_000 * 3600 + t / 100_000 % 100 * 60 + t / 1000 % 100) as u32;
    let status = u16::from_be_bytes([b[0x2A], b[0x2B]]);
    let (lat, lon, alt) =
        (degrees(be_i32(b, 0x08)), degrees(be_i32(b, 0x0C)), f64::from(be_i32(b, 0x10)) / 10.0);
    let place_ok = (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
        && (-400.0..60_000.0).contains(&alt);
    let pos =
        (place_ok && status & 0x30 == 0x30 && !(lat == 0.0 && lon == 0.0)).then_some(GpsPos {
            lat,
            lon,
            alt_m: alt,
            speed_ms: 0.0,
            course_deg: 0.0,
            climb_ms: 0.0,
            num_sv: 0,
        });
    let raw_t = u32::from_be_bytes([b[0x1C], b[0x1D], b[0x1E], b[0x1F]]);
    let temp = f32::from_bits(raw_t);
    let temp_c = (raw_t != MARKER_NO_VALUE && temp > -120.0 && temp < 80.0).then_some(temp);
    Some(SondeFix {
        model: SondeModel::Imet54,
        serial: format!("iMet54-{sn}"),
        frame_num,
        battery_v: None,
        meteo: Meteo { temp_c, ..Meteo::default() },
        pos,
    })
}

struct Collect {
    bits: Vec<u8>,
    invert: bool,
}

pub struct Imet54Decoder {
    slicer: FskSlicer,
    header: u64,
    sr: u64,
    frame: Option<Collect>,
}

impl Imet54Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        let slicer = FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?
            .with_channel_lowpass(5000.0, input_rate)
            .with_hold(20.0)
            .with_smoothing(8.0 / 96_000.0, input_rate);
        Ok(Self { slicer, header: header_bits(), sr: 0, frame: None })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        if let Some(f) = self.frame.as_mut() {
            f.bits.push(r ^ u8::from(f.invert));
            if f.bits.len() == FRAME_BITS {
                let f = self.frame.take().unwrap();
                if let Some(bytes) = bytes_of(&f.bits)
                    && checksum_ok(&bytes)
                {
                    out.extend(fix_of(&bytes));
                }
            }
            return;
        }
        self.sr = ((self.sr << 1) | u64::from(r)) & 0xFF_FFFF_FFFF;
        let straight = (self.sr ^ self.header).count_ones() <= HEADER_TOLERANCE;
        let inverted = (self.sr ^ !self.header & 0xFF_FFFF_FFFF).count_ones() <= HEADER_TOLERANCE;
        if straight || inverted {
            self.frame = Some(Collect { bits: Vec::with_capacity(FRAME_BITS), invert: !straight });
        }
    }
}

impl ModelDecoder for Imet54Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::Imet54
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

    /// A frame of the real recording.
    const REAL: &str = "034835fe0e0e05c0fe7a431e01ae4fda00003b4a00000000000000004184c41742c80000c23cb2145c08003eb40073f06800093f000008355c0002cf1300084d120000001600000019000000a40000009b00f806c3a0000000000042060a010000ff738cc2ee075b400016f9";

    fn unhex(s: &str) -> [u8; FRAME_BYTES] {
        let v: Vec<u8> = (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
            .collect();
        v.try_into().unwrap()
    }

    #[test]
    fn a_real_frame_passes_its_check_and_a_flipped_bit_does_not() {
        let mut f = unhex(REAL);
        assert!(checksum_ok(&f));
        f[20] ^= 0x04;
        assert!(!checksum_ok(&f));
    }

    #[test]
    fn a_real_frame_gives_serial_clock_place_and_temperature() {
        let f = fix_of(&unhex(REAL)).expect("a fix");
        assert_eq!(f.serial, "iMet54-55064062");
        assert_eq!(f.frame_num, 23 * 3600 + 58 * 60);
        let p = f.pos.expect("a place");
        // 25 degrees 54.1858 minutes south, 28 degrees 20.0922 minutes east.
        assert!(
            (p.lat + 25.903097).abs() < 1e-5 && (p.lon - 28.334870).abs() < 1e-5,
            "{} {}",
            p.lat,
            p.lon
        );
        assert!((p.alt_m - 1517.8).abs() < 1e-6);
        assert!((f.meteo.temp_c.unwrap() - 16.5957).abs() < 1e-3);
    }

    /// The reverse of the receiver: bytes to nibbles to Hamming words, the
    /// 64-bit blocks transposed, the three sync characters, 8N1 framing.
    fn encode(frame: &[u8; FRAME_BYTES]) -> Vec<u8> {
        let mut words = Vec::new();
        for &byte in frame {
            for nib in [byte >> 4, byte & 15] {
                let w = HAM[usize::from(nib)];
                words.extend((0..8).map(|b| w >> b & 1));
            }
        }
        let data = transpose64(&words);
        let mut chars: Vec<u8> = Vec::new();
        for c in [0x24u8, 0x24, 0x42] {
            chars.push(c);
        }
        let mut bits: Vec<u8> = Vec::new();
        for c in chars {
            bits.extend((0..8).map(|b| c >> b & 1));
        }
        bits.extend(data);
        // The frame holds 220 characters: pad with zero bytes.
        let mut out = Vec::new();
        for c in bits.chunks(8) {
            out.push(0);
            out.extend(c);
            out.push(1);
        }
        while out.len() < FRAME_BITS {
            out.extend([0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        }
        out.truncate(FRAME_BITS);
        out
    }

    #[test]
    fn the_bits_of_a_frame_come_back_as_its_bytes() {
        let f = unhex(REAL);
        let bits = encode(&f);
        assert_eq!(bits.len(), FRAME_BITS);
        assert_eq!(bytes_of(&bits), Some(f));
    }

    #[test]
    fn one_wrong_bit_in_a_word_is_repaired() {
        let f = unhex(REAL);
        let mut bits = encode(&f);
        bits[100] ^= 1;
        assert_eq!(bytes_of(&bits), Some(f));
    }

    fn modulate(sym: &[u8], rate: f64, invert: bool) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if (r == 1) != invert { 2400.0 } else { -2400.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    #[test]
    fn a_synthetic_burst_is_decoded_at_both_rates_and_both_polarities() {
        let head: Vec<u8> = HEADER.bytes().map(|c| c - b'0').collect();
        for rate in [48_000.0, 96_000.0] {
            for invert in [false, true] {
                let mut sym: Vec<u8> = Vec::new();
                for _ in 0..40 {
                    sym.extend([0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1]);
                }
                for _ in 0..3 {
                    sym.extend(&head);
                    sym.extend(encode(&unhex(REAL)));
                }
                let mut iq = vec![Complex32::new(0.0, 0.0); 3000];
                iq.extend(modulate(&sym, rate, invert));
                iq.extend(vec![Complex32::new(0.0, 0.0); 3000]);
                let mut d = Imet54Decoder::new(rate, 0.0).unwrap();
                let mut fixes = Vec::new();
                for chunk in iq.chunks(1000) {
                    fixes.extend(d.process(chunk));
                }
                assert!(fixes.len() >= 2, "{} fixes at {rate}, invert {invert}", fixes.len());
                assert_eq!(fixes[0].serial, "iMet54-55064062");
            }
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = Imet54Decoder::new(48_000.0, 0.0).unwrap();
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
