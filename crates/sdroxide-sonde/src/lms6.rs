//! Lockheed Martin Sippican LMS6-400 decoder.
//!
//! The signal: FSK at 4800 symbols/s carrying a rate 1/2, constraint 7
//! convolutional code (polynomials 0x4f and 0x6d, the second output
//! inverted). Decoded, the bits form blocks of 260 bytes that start with the
//! CCSDS sync word `1A CF FC 1D`; the bytes of a block are sent least
//! significant bit first. A block holds a 223-byte message (then 32 bytes of
//! Reed-Solomon parity, not used here); the messages of consecutive blocks,
//! joined, hold the telemetry frames, one a second, each starting `24 54 00
//! 00 00`: a 3-byte serial, a frame counter, GPS time of week in
//! milliseconds, latitude and longitude as 32-bit binary angles (2^32/360 per
//! degree), altitude in millimetres, three 24-bit velocities in mm/s, and a
//! CRC-16 (CCITT, start value 0) over the first 221 bytes.
//!
//! Provenance: the layout is the public description of the LMS6 by
//! rs1729/RS; the code polynomials, polarity and bit order were checked on a
//! real recording (projecthorus sample set): the Viterbi decoder finishes
//! with a path metric of 0 and every frame found passes its CRC. Hard
//! decisions, then Reed-Solomon (255, 223) repairs up to 16 wrong bytes of a
//! block (`rs255`); a block it cannot repair is still tried, the frame CRC
//! decides. No code of auto_rx, SDRangel or rs1729 (GPL) is used.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};
use crate::rs255::{self, Rs};

const BAUD: f64 = 4800.0;
const POLY_A: u32 = 0x4f;
const POLY_B: u32 = 0x6d;
/// The sync word, as the decoded bit stream carries it (most significant bit first).
const ASM: u32 = 0x1ACF_FC1D;
/// Coded symbols per block: 260 bytes at rate 1/2.
const BLOCK_SYMBOLS: usize = 260 * 8 * 2;
/// A few bits past the block, so the last ones decode well.
const TAIL_BITS: usize = 24;
/// Mismatches allowed when looking for the sync word in the coded stream.
const SYNC_TOLERANCE: u32 = 7;
/// Message bytes of a block.
const MSG_BYTES: usize = 223;
/// A frame: 221 bytes and its CRC.
const FRAME_BYTES: usize = 223;
/// Symbols are kept this long; more than a block and its lead-in.
const KEEP_SYMBOLS: usize = BLOCK_SYMBOLS + 4096;

fn parity(x: u32) -> u32 {
    x.count_ones() & 1
}

/// The two coded bits of one input bit, given the six before it, and the
/// next state.
fn encode_step(state: u32, bit: u32) -> (u32, u32, u32) {
    let reg = bit | (state << 1);
    (parity(reg & POLY_A), parity(reg & POLY_B) ^ 1, reg & 63)
}

/// The coded form of the last 26 bits of the sync word, which do not depend
/// on what was sent before it: 52 symbols, the first of them oldest.
fn sync_symbols() -> u64 {
    let mut state = 0u32;
    let mut out = 0u64;
    for i in (0..32).rev() {
        let bit = (ASM >> i) & 1;
        let (a, b, next) = encode_step(state, bit);
        state = next;
        if i < 26 {
            out = (out << 2) | u64::from(a << 1 | b);
        }
    }
    out
}

/// Hard-decision Viterbi decoding of `symbols` (pairs), no assumed start state.
fn viterbi(symbols: &[u8]) -> Vec<u8> {
    const INF: u32 = u32::MAX / 2;
    let steps = symbols.len() / 2;
    let mut metric = [0u32; 64];
    // For each step, which of the two predecessors each state came from.
    let mut from = Vec::with_capacity(steps);
    for t in 0..steps {
        let (ra, rb) = (u32::from(symbols[2 * t]), u32::from(symbols[2 * t + 1]));
        let mut next = [INF; 64];
        let mut choice = 0u64;
        for s_new in 0..64u32 {
            let bit = s_new & 1;
            for k in 0..2u32 {
                // The state before: the new one shifted back, plus the bit that fell off.
                let s_old = (s_new >> 1) | (k << 5);
                let (a, b, _) = encode_step(s_old, bit);
                let d = u32::from(a != ra) + u32::from(b != rb);
                let m = metric[s_old as usize] + d;
                if m < next[s_new as usize] {
                    next[s_new as usize] = m;
                    if k == 1 {
                        choice |= 1 << s_new;
                    } else {
                        choice &= !(1 << s_new);
                    }
                }
            }
        }
        metric = next;
        from.push(choice);
    }
    let mut s = (0..64usize).min_by_key(|&i| metric[i]).unwrap_or(0) as u32;
    let mut bits = vec![0u8; steps];
    for t in (0..steps).rev() {
        bits[t] = (s & 1) as u8;
        let k = ((from[t] >> s) & 1) as u32;
        s = (s >> 1) | (k << 5);
    }
    bits
}

fn crc16(data: &[u8]) -> u16 {
    let mut c = 0u16;
    for &b in data {
        c ^= u16::from(b) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
    }
    c
}

fn be_i32(b: &[u8], o: usize) -> i32 {
    i32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn be_i24(b: &[u8], o: usize) -> i32 {
    let v = (i32::from(b[o]) << 16) | (i32::from(b[o + 1]) << 8) | i32::from(b[o + 2]);
    if v >> 23 == 1 { v - (1 << 24) } else { v }
}

const DEG_PER_COUNT: f64 = 360.0 / 4_294_967_296.0;

/// One frame (starting at `24 54`), when its CRC holds.
fn decode_frame(f: &[u8]) -> Option<SondeFix> {
    if f.len() < FRAME_BYTES || f[0] != 0x24 || f[1] != 0x54 {
        return None;
    }
    if crc16(&f[..FRAME_BYTES - 2]) != u16::from_be_bytes([f[FRAME_BYTES - 2], f[FRAME_BYTES - 1]])
    {
        return None;
    }
    let serial = (u32::from(f[5]) << 16) | (u32::from(f[6]) << 8) | u32::from(f[7]);
    let counter = u32::from(u16::from_be_bytes([f[8], f[9]]));
    let lat = f64::from(be_i32(f, 18)) * DEG_PER_COUNT;
    let lon = f64::from(be_i32(f, 22)) * DEG_PER_COUNT;
    let alt = f64::from(be_i32(f, 26)) / 1000.0;
    let ve = f64::from(be_i24(f, 30)) / 1000.0;
    let vn = f64::from(be_i24(f, 33)) / 1000.0;
    let vu = f64::from(be_i24(f, 36)) / 1000.0;
    let ok = lat.abs() < 89.9
        && lon.abs() <= 180.0
        && !(lat.abs() < 1e-6 && lon.abs() < 1e-6)
        && (-500.0..60_000.0).contains(&alt);
    Some(SondeFix {
        model: SondeModel::Lms6,
        serial: format!("LMS6-{serial}"),
        frame_num: counter,
        battery_v: None,
        meteo: Meteo::default(),
        pos: ok.then(|| GpsPos {
            lat,
            lon,
            alt_m: alt,
            speed_ms: ve.hypot(vn),
            course_deg: ve.atan2(vn).to_degrees().rem_euclid(360.0),
            climb_ms: vu,
            num_sv: 0,
        }),
    })
}

pub struct Lms6Decoder {
    slicer: FskSlicer,
    sync: u64,
    window: u64,
    seen: usize,
    /// Symbols, oldest first; `base` is the absolute index of the first.
    symbols: Vec<u8>,
    base: usize,
    /// Absolute index of the start of the next block to decode.
    pending: Option<usize>,
    /// Absolute index where the block that was decoded last ends.
    last_end: Option<usize>,
    /// Messages of consecutive blocks, joined.
    payload: Vec<u8>,
    rs: Rs,
    /// Whether the Reed-Solomon repair is applied (tests switch it off).
    repair: bool,
    /// Bytes the repair has set right, for the log and the tests.
    pub repaired_bytes: usize,
}

impl Lms6Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        Ok(Self {
            slicer: FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?,
            sync: sync_symbols(),
            window: 0,
            seen: 0,
            symbols: Vec::new(),
            base: 0,
            pending: None,
            last_end: None,
            payload: Vec::new(),
            rs: Rs::new(),
            repair: true,
            repaired_bytes: 0,
        })
    }

    fn push_symbol(&mut self, s: u8, out: &mut Vec<SondeFix>) {
        self.symbols.push(s);
        self.window = (self.window << 1) | u64::from(s);
        self.seen += 1;
        // One past this symbol, as an absolute index.
        let abs = self.base + self.symbols.len();
        if self.pending.is_none() && self.seen >= 64 {
            let w = self.window & ((1u64 << 52) - 1);
            let d = (w ^ self.sync).count_ones();
            if d <= SYNC_TOLERANCE || d >= 52 - SYNC_TOLERANCE {
                // The 52 symbols end 12 + 52 symbols after the first bit of the sync word.
                let start = abs.saturating_sub(52 + 12);
                let fits = start >= self.base;
                if fits && self.last_end.is_none_or(|e| start + 100 >= e) {
                    self.pending = Some(start);
                }
            }
        }
        if let Some(start) = self.pending
            && abs >= start + BLOCK_SYMBOLS + 2 * TAIL_BITS
        {
            self.pending = None;
            self.block(start, out);
        }
        if self.symbols.len() > KEEP_SYMBOLS {
            let drop = self.symbols.len() - KEEP_SYMBOLS;
            self.symbols.drain(..drop);
            self.base += drop;
        }
    }

    fn block(&mut self, start: usize, out: &mut Vec<SondeFix>) {
        let from = start - self.base;
        let seg = &self.symbols[from..from + BLOCK_SYMBOLS + 2 * TAIL_BITS];
        // Either polarity: the sync match says which.
        let mut w = 0u64;
        for &s in &seg[12..64] {
            w = (w << 1) | u64::from(s);
        }
        let inverted = (w ^ self.sync).count_ones() > 26;
        let coded: Vec<u8> = seg.iter().map(|&s| s ^ u8::from(inverted)).collect();
        let bits = viterbi(&coded);
        let word = bits[..32].iter().fold(0u32, |a, &b| (a << 1) | u32::from(b));
        if (word ^ ASM).count_ones() > 3 {
            return;
        }
        // Bytes are sent least significant bit first.
        let mut bytes: Vec<u8> = bits[..BLOCK_SYMBOLS / 2]
            .chunks(8)
            .map(|c| c.iter().enumerate().fold(0u8, |a, (i, &b)| a | (b << i)))
            .collect();
        // The 255 bytes after the sync word are a Reed-Solomon codeword.
        if self.repair
            && let Some(n) = self.rs.correct(&mut bytes[4..4 + rs255::BLOCK])
        {
            self.repaired_bytes += n;
        }
        // Messages of consecutive blocks join; a gap starts afresh.
        if self.last_end != Some(start) {
            self.payload.clear();
        }
        self.last_end = Some(start + BLOCK_SYMBOLS);
        self.payload.extend_from_slice(&bytes[4..4 + MSG_BYTES]);
        let mut i = 0;
        let mut consumed = 0;
        while i + FRAME_BYTES <= self.payload.len() {
            if self.payload[i] == 0x24
                && self.payload[i + 1] == 0x54
                && let Some(fix) = decode_frame(&self.payload[i..i + FRAME_BYTES])
            {
                out.push(fix);
                i += FRAME_BYTES;
                consumed = i;
                continue;
            }
            i += 1;
        }
        // Keep what may still be the start of a frame; drop what cannot be.
        let keep_from = consumed.max(self.payload.len().saturating_sub(FRAME_BYTES - 1));
        self.payload.drain(..keep_from);
    }
}

impl ModelDecoder for Lms6Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::Lms6
    }

    fn process(&mut self, input: &[Complex32]) -> Vec<SondeFix> {
        let mut out = Vec::new();
        let mut syms = Vec::new();
        self.slicer.process(input, |b| syms.push(b));
        for s in syms {
            self.push_symbol(s, &mut out);
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

    /// A frame of the real recording (223 bytes: 221 and the CRC).
    const REAL: &str = "24540000007b8d8c01940fee0c74ffe1c8df1add4434c8cfa99700013899ffffbcfffeb700002e007014291f7d1d3e2a1cefe63a18251f7ef636fc1cf212f3151a1f812abbc21cf60e3e0f221f86acb9541cfb8f6e201a1f91c058f11cebad540a201f879824211cecf57c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003b54400000019fdf209ea160000000000001df9ce11f25b03b56103b55d03b55803b565000000000603e8001e000007a708";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn a_real_frame_passes_its_crc_and_has_a_place() {
        let f = unhex(REAL);
        assert_eq!(f.len(), FRAME_BYTES);
        let fix = decode_frame(&f).expect("a real frame");
        assert_eq!(fix.serial, "LMS6-8097164");
        assert_eq!(fix.frame_num, 404);
        let p = fix.pos.expect("a position");
        assert!((p.lat - 37.7780).abs() < 1e-3 && (p.lon + 77.6093).abs() < 1e-3);
        assert!((p.alt_m - 80.0).abs() < 0.1);
        assert!(p.speed_ms < 1.0);
    }

    #[test]
    fn a_flipped_bit_fails_the_crc() {
        let mut f = unhex(REAL);
        f[20] ^= 0x01;
        assert!(decode_frame(&f).is_none());
    }

    fn encode(bits: &[u8]) -> Vec<u8> {
        let mut state = 0;
        let mut out = Vec::new();
        for &b in bits {
            let (a, c, next) = encode_step(state, u32::from(b));
            state = next;
            out.push(a as u8);
            out.push(c as u8);
        }
        out
    }

    #[test]
    fn the_viterbi_decoder_repairs_a_few_wrong_symbols() {
        let mut x = 99u32;
        let bits: Vec<u8> = (0..600)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                (x >> 24) as u8 & 1
            })
            .collect();
        let mut coded = encode(&bits);
        for i in [40usize, 300, 301, 700, 1100] {
            coded[i] ^= 1;
        }
        let back = viterbi(&coded);
        assert_eq!(&back[..590], &bits[..590]);
    }

    /// Blocks as on air: the sync word, the messages, zero parity.
    fn blocks(frames: &[Vec<u8>]) -> Vec<u8> {
        // Messages are laid end to end; the first frame starts 29 bytes in.
        let mut stream = vec![0xCAu8; 29];
        for f in frames {
            stream.extend_from_slice(f);
        }
        while !stream.len().is_multiple_of(MSG_BYTES) {
            stream.push(0xCA);
        }
        let mut bits = vec![0u8; 24];
        for msg in stream.chunks(MSG_BYTES) {
            for i in (0..32).rev() {
                bits.push(((ASM >> i) & 1) as u8);
            }
            let mut block = msg.to_vec();
            block.extend_from_slice(&Rs::new().parity(msg));
            block.resize(256, 0);
            for b in block {
                for i in 0..8 {
                    bits.push((b >> i) & 1);
                }
            }
        }
        bits.extend(vec![0u8; 40]);
        encode(&bits)
    }

    fn modulate(sym: &[u8], rate: f64, invert: bool) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = vec![Complex32::new(0.0, 0.0); 2000];
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let up = (r == 1) != invert;
            let f = if up { 2400.0 } else { -2400.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out.extend(vec![Complex32::new(0.0, 0.0); 2000]);
        out
    }

    #[test]
    fn a_synthetic_signal_gives_the_frames_in_both_polarities() {
        let f1 = unhex(REAL);
        let mut f2 = f1.clone();
        f2[9] = f2[9].wrapping_add(1);
        let crc = crc16(&f2[..FRAME_BYTES - 2]).to_be_bytes();
        f2[FRAME_BYTES - 2..].copy_from_slice(&crc);
        let sym = blocks(&[f1, f2]);
        for rate in [48_000.0, 96_000.0] {
            for invert in [false, true] {
                let mut d = Lms6Decoder::new(rate, 0.0).unwrap();
                let mut fixes = Vec::new();
                for chunk in modulate(&sym, rate, invert).chunks(1000) {
                    fixes.extend(d.process(chunk));
                }
                let counters: Vec<u32> = fixes.iter().map(|f| f.frame_num).collect();
                assert_eq!(counters, vec![404, 405], "rate {rate} invert {invert}");
            }
        }
    }

    #[test]
    fn a_burst_of_wrong_symbols_is_repaired_by_reed_solomon() {
        let f1 = unhex(REAL);
        let mut f2 = f1.clone();
        f2[9] = f2[9].wrapping_add(1);
        let crc = crc16(&f2[..FRAME_BYTES - 2]).to_be_bytes();
        f2[FRAME_BYTES - 2..].copy_from_slice(&crc);
        let mut sym = blocks(&[f1, f2]);
        // 80 symbols of a block turned over: the Viterbi path wanders and leaves wrong bytes.
        for s in &mut sym[1500..1580] {
            *s ^= 1;
        }
        let run = |repair: bool| {
            let mut d = Lms6Decoder::new(48_000.0, 0.0).unwrap();
            d.repair = repair;
            let mut fixes = Vec::new();
            for chunk in modulate(&sym, 48_000.0, false).chunks(1000) {
                fixes.extend(d.process(chunk));
            }
            (fixes.len(), d.repaired_bytes)
        };
        let (without, _) = run(false);
        let (with, repaired) = run(true);
        assert_eq!(with, 2, "both frames come back");
        assert!(
            repaired > 0 && without < with,
            "without repair {without}, repaired {repaired} bytes"
        );
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = Lms6Decoder::new(48_000.0, 0.0).unwrap();
        let mut x = 4242u32;
        let noise: Vec<Complex32> = (0..48_000 * 6)
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
