//! Graw DFM-06 / DFM-09 / DFM-17 decoder.
//!
//! The signal: FSK at 2500 baud, Manchester coded (1250 data bit/s), frames of
//! 280 data bits: a 16-bit header `0x45CF`, then three blocks of
//! [8,4] extended Hamming codewords, interleaved by columns: CFG (7 words),
//! DAT1 and DAT2 (13 words each). A DAT block is 12 nibbles of payload plus a
//! packet id in the last nibble; the GPS fix arrives spread over several ids.
//!
//! Provenance: the frame layout and the GPS field scalings are the public
//! description of the DFM telemetry by rs1729/RS (`dfm06.txt`). The three
//! things that text leaves open (the Hamming parity rows, the interleaver
//! order and the polarity) were found from a real DFM-09 recording, not copied
//! from any program: with the column interleave exactly 16 distinct codewords
//! appear in 100 % of the words. No code of auto_rx or SDRangel (GPL) is used.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 2500.0;
const HEADER: u32 = 0x45CF;
const HEADER_INV: u32 = 0xBA30;
/// Data bits after the header: CFG 56 + DAT1 104 + DAT2 104.
const BODY_BITS: usize = 264;
/// Manchester violations tolerated inside one frame before it is dropped.
const MAX_VIOLATIONS: u32 = 10;
/// A sonde that has sent no channel A in this many frames (4 s) is a DFM-06.
const DFM06_AFTER_FRAMES: u32 = 20;

/// The 16 codewords of the extended [8,4] Hamming code: 4 data bits first,
/// then 4 parity bits (rows 0111, 1011, 1101, 1110 for d0..d3).
fn codeword(n: u8) -> u8 {
    const ROWS: [u8; 4] = [0b0111, 0b1011, 0b1101, 0b1110];
    let mut p = 0u8;
    for (i, row) in ROWS.iter().enumerate() {
        if n >> (3 - i) & 1 == 1 {
            p ^= row;
        }
    }
    (n << 4) | p
}

/// The data nibble of the codeword nearest to `w`, if it is at most one bit
/// away (the code has distance 4: one error is corrected, two are detected).
fn nibble_of(w: u8) -> Option<u8> {
    (0..16u8).find(|&n| (codeword(n) ^ w).count_ones() <= 1)
}

/// De-interleave a block of `n` codewords (bit `i` of word `j` is bit
/// `i * n + j` of the block) and decode them into nibbles.
fn decode_block(bits: &[u8], n: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(n);
    for j in 0..n {
        let mut w = 0u8;
        for i in 0..8 {
            w = (w << 1) | (bits[i * n + j] & 1);
        }
        out.push(nibble_of(w)?);
    }
    Some(out)
}

fn value(nibbles: &[u8]) -> u64 {
    nibbles.iter().fold(0u64, |a, &q| (a << 4) | u64::from(q))
}

/// What has been heard so far of the sonde, folded across frames.
#[derive(Default)]
struct Assembler {
    serial_hi: Option<u16>,
    serial_lo: Option<u16>,
    serial_dfm06: Option<u32>,
    /// Frames seen so far: channel 6 is the DFM-06 serial but a plain
    /// measurement on a DFM-09, whose serial (channel A) comes every 8 frames.
    frames: u32,
    counter: u8,
    num_sv: u8,
    lat: Option<f64>,
    lon: Option<f64>,
    hspeed: f64,
    course: f64,
    /// lat and lon received since the last fix went out.
    fresh: u8,
}

impl Assembler {
    fn serial(&self) -> Option<u32> {
        match (self.serial_hi, self.serial_lo) {
            (Some(h), Some(l)) => Some((u32::from(h) << 16) | u32::from(l)),
            (None, None) if self.frames >= DFM06_AFTER_FRAMES => self.serial_dfm06,
            _ => None,
        }
    }

    fn cfg(&mut self, n: &[u8]) {
        match n[0] {
            // DFM-09 / DFM-17: `A c aaaa b`, b = 0 the high half, 1 the low.
            0xA => {
                let half = value(&n[2..6]) as u16;
                match n[6] {
                    0 => self.serial_hi = Some(half),
                    1 => self.serial_lo = Some(half),
                    _ => {}
                }
            }
            // DFM-06: `6 dddddd`, the six digits are decimal.
            6 if n[1..7].iter().all(|&d| d <= 9) => {
                self.serial_dfm06 = Some(n[1..7].iter().fold(0u32, |a, &d| a * 10 + u32::from(d)));
            }
            _ => {}
        }
    }

    /// One DAT block: 12 payload nibbles and the packet id.
    fn packet(&mut self, n: &[u8], out: &mut Vec<SondeFix>) {
        let id = n[12];
        let p = value(&n[..12]); // 48 bits
        let hi = (p >> 16) as u32;
        let lo = (p & 0xFFFF) as u16;
        match id {
            0 => self.counter = ((p >> 16) & 0xFF) as u8,
            1 => self.num_sv = hi.count_ones().min(255) as u8,
            2 => {
                self.lat = Some(f64::from(hi as i32) / 1e7);
                self.hspeed = f64::from(lo) / 100.0;
                self.fresh |= 1;
            }
            3 => {
                self.lon = Some(f64::from(hi as i32) / 1e7);
                self.course = f64::from(lo) / 100.0;
                self.fresh |= 2;
            }
            4 if self.fresh == 3 => {
                if let (Some(serial), Some(lat), Some(lon)) = (self.serial(), self.lat, self.lon) {
                    out.push(SondeFix {
                        model: SondeModel::Dfm,
                        serial: format!("DFM-{serial}"),
                        frame_num: u32::from(self.counter),
                        battery_v: None,
                        meteo: Meteo::default(),
                        pos: Some(GpsPos {
                            lat,
                            lon,
                            alt_m: f64::from(hi as i32) / 100.0,
                            speed_ms: self.hspeed,
                            course_deg: self.course,
                            climb_ms: f64::from(lo as i16) / 100.0,
                            num_sv: self.num_sv,
                        }),
                    });
                }
                self.fresh = 0;
            }
            _ => {}
        }
    }
}

/// Collects the data bits that follow a header found on one Manchester
/// alignment.
struct Frame {
    bits: Vec<u8>,
    invert: bool,
    violations: u32,
}

pub struct DfmDecoder {
    slicer: FskSlicer,
    // Raw bit history, for the two Manchester alignments.
    raw_n: u64,
    last_raw: u8,
    sr: [u32; 2],
    frames: [Option<Frame>; 2],
    asm: Assembler,
}

impl DfmDecoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        Ok(Self {
            slicer: FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Mean)?,
            raw_n: 0,
            last_raw: 0,
            sr: [0; 2],
            frames: [None, None],
            asm: Assembler::default(),
        })
    }

    fn raw_bit(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        self.raw_n += 1;
        if self.raw_n >= 2 {
            // The pair (last_raw, r) starts at index raw_n - 2: its alignment.
            let q = ((self.raw_n - 2) & 1) as usize;
            let violation = self.last_raw == r;
            // Data bit: the first bit of the pair, inverted.
            let d = (self.last_raw ^ 1) & 1;
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
        let w = self.sr[q] & 0xFFFF;
        if w == HEADER || w == HEADER_INV {
            self.frames[q] = Some(Frame {
                bits: Vec::with_capacity(BODY_BITS),
                invert: w == HEADER_INV,
                violations: 0,
            });
        }
    }

    fn finish(&mut self, bits: &[u8], out: &mut Vec<SondeFix>) {
        self.asm.frames = self.asm.frames.saturating_add(1);
        if let Some(cfg) = decode_block(&bits[..56], 7) {
            self.asm.cfg(&cfg);
        }
        for blk in [&bits[56..160], &bits[160..264]] {
            if let Some(n) = decode_block(blk, 13) {
                self.asm.packet(&n, out);
            }
        }
    }
}

impl ModelDecoder for DfmDecoder {
    fn model(&self) -> SondeModel {
        SondeModel::Dfm
    }

    fn process(&mut self, input: &[Complex32]) -> Vec<SondeFix> {
        let mut out = Vec::new();
        let mut bits = Vec::new();
        self.slicer.process(input, |b| bits.push(b));
        for b in bits {
            self.raw_bit(b, &mut out);
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

    fn encode_word(n: u8) -> [u8; 8] {
        let w = codeword(n);
        std::array::from_fn(|i| (w >> (7 - i)) & 1)
    }

    fn encode_block(nibbles: &[u8]) -> Vec<u8> {
        let n = nibbles.len();
        let mut bits = vec![0u8; 8 * n];
        for (j, &q) in nibbles.iter().enumerate() {
            let w = encode_word(q);
            for i in 0..8 {
                bits[i * n + j] = w[i];
            }
        }
        bits
    }

    fn nibbles_of(v: u64, n: usize) -> Vec<u8> {
        (0..n).map(|i| ((v >> (4 * (n - 1 - i))) & 0xF) as u8).collect()
    }

    fn dat(id: u8, payload: u64) -> Vec<u8> {
        let mut n = nibbles_of(payload, 12);
        n.push(id);
        n
    }

    fn frame_bits(cfg: &[u8], d1: &[u8], d2: &[u8]) -> Vec<u8> {
        let mut b: Vec<u8> = (0..16).map(|i| ((HEADER >> (15 - i)) & 1) as u8).collect();
        b.extend(encode_block(cfg));
        b.extend(encode_block(d1));
        b.extend(encode_block(d2));
        assert_eq!(b.len(), 280);
        b
    }

    /// Manchester (data bit d is sent as the pair (!d, d)), FSK at +-1.2 kHz.
    fn modulate(bits: &[u8], rate: f64) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut raw = Vec::new();
        for &d in bits {
            raw.push(d ^ 1);
            raw.push(d);
        }
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in raw.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if r == 1 { 1200.0 } else { -1200.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    fn s32(x: f64) -> u64 {
        (x as i64 as u32) as u64
    }

    /// A short flight: the serial in two halves, then fixes.
    fn stream(rate: f64) -> Vec<Complex32> {
        let serial: u32 = 0x0009_BB65;
        let mut iq = vec![Complex32::new(0.0, 0.0); 2000];
        iq.extend(modulate(&[0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1], rate));
        for k in 0..12u8 {
            // CFG: channel 0, 1, 2 then A (low and high halves alternating).
            let cfg: Vec<u8> = match k % 4 {
                3 => {
                    let (half, b) =
                        if k % 8 == 3 { (serial & 0xFFFF, 1u8) } else { (serial >> 16, 0u8) };
                    let mut v = vec![0xA, 0xC];
                    v.extend(nibbles_of(u64::from(half), 4));
                    v.push(b);
                    v
                }
                c => vec![c, 0, 0, 0, 0, 0, 0],
            };
            let lat = -34.7207068 + f64::from(k) * 1e-4;
            let (d1, d2) = match k % 3 {
                0 => (dat(0, u64::from(k) << 16), dat(1, 0x1234_5678_0000)),
                1 => (dat(2, (s32(lat * 1e7) << 16) | 4), dat(3, (s32(138.692769e7) << 16) | 684)),
                _ => (dat(4, (s32(10335.0) << 16) | 0xFFFF), dat(5, 0)),
            };
            iq.extend(modulate(&frame_bits(&cfg, &d1, &d2), rate));
        }
        iq.extend(vec![Complex32::new(0.0, 0.0); 2000]);
        iq
    }

    #[test]
    fn the_hamming_code_corrects_one_error_and_flags_two() {
        for n in 0..16u8 {
            let w = codeword(n);
            assert_eq!(nibble_of(w), Some(n));
            for b in 0..8 {
                assert_eq!(nibble_of(w ^ (1 << b)), Some(n), "one flipped bit is corrected");
            }
            assert_eq!(
                nibble_of(w ^ 0b11),
                None,
                "two flipped bits are detected, not mis-corrected"
            );
        }
    }

    #[test]
    fn the_codewords_are_the_ones_heard_on_a_real_sonde() {
        // Seen in a real DFM-09 recording: all 16, nothing else.
        let heard: [u8; 16] = [
            0x00, 0x87, 0x2D, 0x99, 0x55, 0x66, 0x4B, 0xAA, 0xE1, 0xCC, 0xB4, 0x78, 0x1E, 0xFF,
            0x33, 0xD2,
        ];
        let mut mine: Vec<u8> = (0..16).map(codeword).collect();
        mine.sort_unstable();
        let mut want = heard.to_vec();
        want.sort_unstable();
        assert_eq!(mine, want);
    }

    #[test]
    fn a_synthetic_flight_gives_serial_place_and_height() {
        for rate in [48_000.0, 96_000.0] {
            let mut d = DfmDecoder::new(rate, 0.0).unwrap();
            let mut fixes = Vec::new();
            for chunk in stream(rate).chunks(1000) {
                fixes.extend(d.process(chunk));
            }
            assert!(!fixes.is_empty(), "no fix at {rate}");
            let f = &fixes[0];
            assert_eq!(f.model, SondeModel::Dfm);
            assert_eq!(f.serial, "DFM-637797");
            let p = f.pos.as_ref().unwrap();
            assert!((p.lon - 138.692769).abs() < 1e-6);
            assert!((p.lat + 34.72).abs() < 0.01);
            assert!((p.alt_m - 103.35).abs() < 1e-6);
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = DfmDecoder::new(48_000.0, 0.0).unwrap();
        let mut x = 12345u32;
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
