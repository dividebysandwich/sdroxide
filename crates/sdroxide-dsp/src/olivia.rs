//! Olivia MFSK modem: streaming decode + encode, bit-compatible with fldigi /
//! MultiPSK (Pawel Jalocha's reference, `src/include/jalocha/pj_mfsk.h` in
//! fldigi).
//!
//! A block of 64 MFSK symbols carries `BitsPerSymbol = log2(tones)` 7-bit
//! characters. Each character becomes one 64-chip Walsh function (the inverse
//! fast Hadamard transform of a ±1 impulse at `char & 63`, sign from bit 6),
//! which is then
//!
//! * **scrambled** by XOR with the 64-bit sequence `0xE257E6D0291574EC`, the
//!   character in slot `j` of the block using the sequence rotated right by
//!   `13*j` bits; and
//! * **interleaved**: chip `t` of character `j` goes into bit `(j + t) mod
//!   BitsPerSymbol` of symbol `t` (so character 0 uses bit 0 of symbol 0, bit 1
//!   of symbol 1, …).
//!
//! The resulting symbol value is Gray-coded and sent as tone
//! `FirstCarrier + 2*gray` on an FFT grid of `SymbolLen = 2·rate/spacing` bins
//! (tone spacing = two bins = the baud rate). Each symbol is a `SymbolLen`-long
//! raised-cosine (`1 - cos`) burst, overlap-added every `SymbolLen/2` samples,
//! and the carrier phase jumps by a random ±90° between symbols.
//!
//! The receiver mirrors fldigi's `MFSK_Receiver`: a raised-cosine windowed DFT
//! every half symbol, soft bits from the squared tone energies, and a bank of
//! soft Walsh decoders over every half-symbol time phase within the block (128)
//! and every half-tone frequency offset within ±`SYNC_MARGIN` bins. The
//! scrambler is what makes the right phase stand out: a mis-aligned block
//! descrambles to noise.
//!
//! The internal sample rate is expected to be 8 kHz (as in fldigi); the
//! geometry is derived from `rate` so other rates work, but only 8 kHz gives
//! fldigi's exact power-of-two `SymbolLen` and carrier grid.
//!
//! **Contestia** is the same engine with `bContestia` set in `pj_mfsk.h`: six
//! bits to a character, so a 32-symbol block and a 32-chip Walsh function; the
//! scrambling sequence `0xEDB88320` rotated 5 bits per slot instead of 13; its
//! own upper-case character map (`EncodeCharacter`/`DecodeCharacter`); and the
//! tone set placed with `fc_offset = bandwidth / 2` (`contestia.cxx`), half a
//! tone lower than Olivia's. [`OliviaTx::contestia`] and
//! [`OliviaRx::contestia`] build it.

use std::collections::VecDeque;

use crate::mfsk::{gray, ungray};

const OUT_AMP: f32 = 0.5;
/// Symbols per Olivia block: `1 << (BitsPerCharacter - 1)` with 7-bit chars.
/// The largest block; Contestia's is half of it ([`CONTESTIA_BLOCK`]).
const BLOCK: usize = 64;
/// Symbols per Contestia block: six-bit characters.
const CONTESTIA_BLOCK: usize = 32;
/// Contestia's scrambling sequence (`ScramblingCodeContestia`), the CRC-32
/// polynomial.
const SCRAMBLE_CONTESTIA: u64 = 0xEDB8_8320;
/// Its rotation per character slot (`nShift = bContestia ? 5 : 13`).
const SCRAMBLE_SHIFT_CONTESTIA: usize = 5;
/// Olivia's Walsh scrambling sequence (`MFSK_Encoder::ScramblingCodeOlivia`).
const SCRAMBLE: u64 = 0xE257_E6D0_2915_74EC;
/// Rotation of the scrambling sequence per character slot in the block.
const SCRAMBLE_SHIFT: usize = 13;
/// Spectra (time slices) per symbol, as fldigi's `SpectraPerSymbol`.
const SLICES: usize = 2;
/// Frequency search margin in FFT bins (half tone spacings) each side of the
/// nominal carrier grid (fldigi's default `SyncMargin`).
const SYNC_MARGIN: usize = 8;
/// Integration length of the sync signal/noise filters, in blocks (fldigi's
/// default `SyncIntegLen`); it is also the decode pipeline delay.
const SYNC_INTEG: usize = 4;
/// Integrated S/N needed to print a block (fldigi clamps its threshold to ≥3).
const SYNC_THRESHOLD: f32 = 3.0;
/// A block whose own S/N clears this prints even after the integrated S/N has
/// decayed — keeps the last blocks of an over from being swallowed by the
/// pipeline delay once the carrier drops.
const BLOCK_THRESHOLD: f32 = 5.0;
/// Max characters per block (`BitsPerSymbol` ≤ 8 for 256 tones).
const MAX_BPS: usize = 8;

/// fldigi's forward FHT (`pj_fht.h::FHT`). Note the butterfly is
/// `(a, b) -> (a + b, b - a)`, not the textbook `(a + b, a - b)`; the sign of
/// the decoded peak depends on it.
fn fht(data: &mut [f32]) {
    let len = data.len();
    let mut step = 1;
    while step < len {
        let mut p = 0;
        while p < len {
            for q in p..p + step {
                let (b1, b2) = (data[q], data[q + step]);
                data[q] = b2 + b1;
                data[q + step] = b2 - b1;
            }
            p += 2 * step;
        }
        step *= 2;
    }
}

/// Chip `t` of the Walsh function for character code `ch` in a `block`-chip
/// block, before scrambling: `true` = negative chip. Closed form of fldigi's
/// `IFHT` of an impulse at `ch & (block-1)` (+1, or −1 when `ch & block`):
/// `(-1)^popcount(c & !t)`.
fn walsh_chip_negative(ch: u8, t: usize, block: usize) -> bool {
    let c = ch as usize & (block - 1);
    let neg = (c & !t & (block - 1)).count_ones() & 1 == 1;
    neg ^ (ch as usize & block != 0)
}

/// `true` when the scrambler flips chip `t` of the character in slot `slot`.
fn scramble_flip(slot: usize, t: usize, contestia: bool) -> bool {
    if contestia {
        let bit = (slot * SCRAMBLE_SHIFT_CONTESTIA + t) & (CONTESTIA_BLOCK - 1);
        (SCRAMBLE_CONTESTIA >> bit) & 1 == 1
    } else {
        let bit = (slot * SCRAMBLE_SHIFT + t) & (BLOCK - 1);
        (SCRAMBLE >> bit) & 1 == 1
    }
}

/// `MFSK_Encoder::EncodeBlock`: `bps` character codes → one block of binary
/// symbol values (before Gray coding), 64 for Olivia and 32 for Contestia.
fn encode_block(chars: &[u8], bps: usize, contestia: bool) -> Vec<u8> {
    let block = if contestia { CONTESTIA_BLOCK } else { BLOCK };
    let mut out = vec![0u8; block];
    for (slot, &ch) in chars.iter().enumerate().take(bps) {
        let ch = ch & (2 * block - 1) as u8;
        for (t, sym) in out.iter_mut().enumerate() {
            if walsh_chip_negative(ch, t, block) ^ scramble_flip(slot, t, contestia) {
                *sym |= 1 << ((slot + t) % bps);
            }
        }
    }
    out
}

/// Contestia's `EncodeCharacter`: upper case only, space and CR moved to the
/// top of the six-bit range, LF sent as NUL, printable ASCII counted from 33.
fn contestia_encode(b: u8) -> u8 {
    match b {
        b'a'..=b'z' => b - b'a' + b'A' - 32,
        b' ' => 59,
        b'\r' => 60,
        b'\n' => 0,
        33..=90 => b - 32,
        8 => 61,
        0 => 0,
        _ => b'?' - 32,
    }
}

/// Contestia's `DecodeCharacter`, the inverse.
fn contestia_decode(c: u8) -> u8 {
    match c {
        59 => b' ',
        60 => b'\r',
        61 => 8,
        0 => 0,
        c => c + 32,
    }
}

/// Resolved Olivia geometry for a (tones, bandwidth) pair at a sample rate,
/// following `MFSK_Transmitter::Preset` and `olivia.cxx`.
#[derive(Clone, Copy)]
struct Geom {
    /// Contestia rather than Olivia (see the module notes).
    contestia: bool,
    /// Symbols per block: 64 for Olivia, 32 for Contestia.
    block: usize,
    tones: usize,
    bps: usize,
    /// FFT / symbol window length (`SymbolLen`), = 2 symbol periods.
    n: usize,
    /// Samples per symbol (`SymbolSepar` = n/2).
    sps: usize,
    /// FFT bin of tone 0 (`FirstCarrier`); tone k is at `first + 2k`.
    first: usize,
    /// Usable search margin in bins (≤ SYNC_MARGIN, clipped at the band edges).
    margin: usize,
}

impl Geom {
    fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64, contestia: bool) -> Self {
        // Contestia's narrowest is four tones.
        let min_tones = if contestia { 4 } else { 2 };
        let bps = (tones.clamp(min_tones, 256) as u32).ilog2() as usize;
        let tones = 1usize << bps;
        // Bandwidth is quantised to 125·2^k Hz, 125..=2000.
        let lb = ((bw.clamp(125.0, 2000.0) / 125.0) as u32).ilog2();
        let bw = 125.0 * f64::from(1u32 << lb);
        let spacing = bw / tones as f64;
        let n = ((2.0 * rate / spacing).round() as usize).max(8) & !3; // n/4 integral
        let bin = rate / n as f64;
        // olivia.cxx: FirstCarrierMultiplier = (f - fc_offset) / 500, then
        // FirstCarrier = (SymbolLen/16)·multiplier + 1 — at 8 kHz that is
        // floor((f - fc_offset) / bin) + 1. Centres the tone set on `audio_hz`
        // rounded to the bin grid.
        // contestia.cxx puts tone 0 on the band edge instead: half a tone lower.
        let fc_offset = if contestia { bw / 2.0 } else { bw * (1.0 - 0.5 / tones as f64) / 2.0 };
        let max_first = (n / 2).saturating_sub(2 * tones + 1).max(1);
        let first =
            (((audio_hz - fc_offset) / bin).floor() + 1.0).clamp(1.0, max_first as f64) as usize;
        let top = first + 2 * (tones - 1);
        let margin = SYNC_MARGIN.min(first).min((n / 2).saturating_sub(top + 1));
        let block = if contestia { CONTESTIA_BLOCK } else { BLOCK };
        Geom { contestia, block, tones, bps, n, sps: n / 2, first, margin }
    }

    fn offsets(&self) -> usize {
        2 * self.margin + 1
    }

    /// Block phases searched: every half symbol within the block.
    fn phases(&self) -> usize {
        SLICES * self.block
    }
}

// ─────────────────────────────── transmit ───────────────────────────────

/// `MFSK_Modulator`: overlap-added raised-cosine tone bursts on the FFT grid,
/// with fldigi's integer phase bookkeeping and a random ±90° jump per symbol.
struct Modulator {
    n: usize,
    sep: usize,
    first: usize,
    cos: Vec<f32>,
    tap: Vec<f32>,
    tap_ptr: usize,
    phase: i64,
    rng: u32,
}

impl Modulator {
    fn new(g: &Geom) -> Self {
        let n = g.n;
        Modulator {
            n,
            sep: g.sps,
            first: g.first,
            cos: (0..n)
                .map(|i| (std::f64::consts::TAU * i as f64 / n as f64).cos() as f32)
                .collect(),
            tap: vec![0.0; n],
            tap_ptr: 0,
            phase: 0,
            rng: 0x2545_F491,
        }
    }

    fn reset(&mut self) {
        self.tap.iter_mut().for_each(|v| *v = 0.0);
        self.tap_ptr = 0;
        self.phase = 0;
    }

    fn coin(&mut self) -> bool {
        // xorshift32 — stands in for fldigi's rand(); only the ±90° choice.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng & 1 == 1
    }

    /// `MFSK_Modulator::Send` + `Output`: queue binary symbol `sym` and append
    /// one symbol period (`sep` samples) of audio to `out`.
    fn send(&mut self, sym: u8, out: &mut Vec<f32>) {
        let n = self.n as i64;
        let freq = (self.first + 2 * gray(u32::from(sym)) as usize) as i64;
        let half = self.sep as i64 / 2;
        self.phase = (self.phase + freq * (half - n / 2)).rem_euclid(n);
        let mut ph = self.phase as usize;
        let mut p = self.tap_ptr;
        for t in 0..self.n {
            self.tap[p] += self.cos[ph] * (1.0 - self.cos[t]);
            ph += freq as usize;
            if ph >= self.n {
                ph -= self.n;
            }
            p += 1;
            if p == self.n {
                p = 0;
            }
        }
        self.phase = (self.phase + freq * (half + n / 2)).rem_euclid(n);
        let jump = if self.coin() { -n / 4 } else { n / 4 };
        self.phase = (self.phase + jump).rem_euclid(n);
        // Two overlapping (1 - cos) bursts peak at 2.
        for _ in 0..self.sep {
            out.push(self.tap[self.tap_ptr] * (OUT_AMP * 0.5));
            self.tap[self.tap_ptr] = 0.0;
            self.tap_ptr += 1;
            if self.tap_ptr == self.n {
                self.tap_ptr = 0;
            }
        }
    }
}

pub struct OliviaTx {
    rate: f64,
    g: Geom,
    modu: Modulator,
    /// Queued characters with their source index (`None` = NUL idle fill).
    q: VecDeque<(u8, Option<usize>)>,
    total_chars: usize,
    sent_chars: usize,
    cur: Vec<f32>,
    cur_pos: usize,
    cur_done: Option<usize>,
    /// Idle blocks still to send after the last character (see [`Self::finish`]).
    tail: usize,
    closing: bool,
}

/// Idle blocks after the last character. The receiver prints a block only once
/// it has integrated a few more behind it, so an over that stops on its last
/// character loses that character's block to the decode pipeline.
const TAIL_BLOCKS: usize = 2;

impl OliviaTx {
    pub fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        Self::build(rate, audio_hz, tones, bw, false)
    }

    /// A Contestia transmitter: Olivia's engine with Contestia's block,
    /// scrambler, character map and tone placement.
    pub fn contestia(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        Self::build(rate, audio_hz, tones, bw, true)
    }

    fn build(rate: f64, audio_hz: f64, tones: usize, bw: f64, contestia: bool) -> Self {
        let g = Geom::new(rate, audio_hz, tones, bw, contestia);
        OliviaTx {
            rate,
            modu: Modulator::new(&g),
            g,
            q: VecDeque::new(),
            total_chars: 0,
            sent_chars: 0,
            cur: Vec::new(),
            cur_pos: 0,
            cur_done: None,
            tail: 0,
            closing: false,
        }
    }

    /// Close the over once the queue has gone out, with a tail of idle blocks.
    pub fn finish(&mut self) {
        if !self.closing {
            self.closing = true;
            self.tail = TAIL_BLOCKS;
        }
    }

    /// True while a closing over is still playing its tail.
    pub fn tail_pending(&self) -> bool {
        self.closing && self.sent_chars > 0 && (self.tail > 0 || self.cur_pos < self.cur.len())
    }

    pub fn set_params(&mut self, audio_hz: f64, tones: usize, bw: f64) {
        self.g = Geom::new(self.rate, audio_hz, tones, bw, self.g.contestia);
        self.modu = Modulator::new(&self.g);
    }

    pub fn push_text(&mut self, text: &str) {
        if !text.is_empty() {
            self.closing = false;
        }
        for ch in text.chars() {
            // fldigi sends '.' for anything outside 7-bit ASCII (8-bit escape off).
            let byte = if ch.is_ascii() { ch as u8 } else { b'.' };
            let byte = if self.g.contestia { contestia_encode(byte) } else { byte };
            self.q.push_back((byte, Some(self.total_chars)));
            self.total_chars += 1;
        }
    }

    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }
    pub fn total_chars(&self) -> usize {
        self.total_chars
    }
    pub fn drained(&self) -> bool {
        self.q.is_empty() && self.cur_pos >= self.cur.len()
    }

    pub fn clear(&mut self) {
        self.q.clear();
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = None;
        self.total_chars = 0;
        self.sent_chars = 0;
        self.closing = false;
        self.tail = 0;
        self.modu.reset();
    }

    fn build_block(&mut self) {
        if self.closing && self.q.is_empty() {
            self.tail = self.tail.saturating_sub(1);
        }
        // Take up to `bps` characters; pad with NUL idle fill.
        let mut chars = [0u8; MAX_BPS];
        let mut done: Option<usize> = None;
        for slot in chars.iter_mut().take(self.g.bps) {
            if let Some((b, idx)) = self.q.pop_front() {
                *slot = b;
                if let Some(i) = idx {
                    done = Some(done.map_or(i, |d| d.max(i)));
                }
            }
        }
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = done;
        for sym in encode_block(&chars[..self.g.bps], self.g.bps, self.g.contestia) {
            self.modu.send(sym, &mut self.cur);
        }
    }

    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.cur_pos >= self.cur.len() {
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                self.build_block();
            }
            out[n] = self.cur[self.cur_pos];
            self.cur_pos += 1;
            n += 1;
        }
        n
    }
}

// ─────────────────────────────── receive ───────────────────────────────

/// Three-pole IIR integrator (`pj_lowpass3.h::LowPass3_Filter`).
#[derive(Clone, Copy, Default)]
struct LowPass3 {
    o1: f32,
    o2: f32,
    out: f32,
}

impl LowPass3 {
    fn process(&mut self, inp: f32, weight: f32) {
        let w = 2.0 * weight;
        let d1 = (inp - self.o1) * w;
        let d12 = (self.o1 - self.o2) * w;
        let d23 = (self.o2 - self.out) * w;
        self.o1 += d1;
        self.o2 += d12;
        self.out += d23;
        self.o2 += d23 * 0.1;
    }
}

/// One decoded block hypothesis.
#[derive(Clone, Copy, Default)]
struct Decoded {
    chars: [u8; MAX_BPS],
    /// This block's own peak / rms-noise ratio.
    snr: f32,
}

/// `MFSK_SoftDecoder`: a 64-symbol ring of soft bits, descrambled and
/// de-interleaved into `bps` soft Walsh decodes.
struct SoftDecoder {
    bps: usize,
    block: usize,
    contestia: bool,
    ring: Vec<f32>,
    ptr: usize,
}

impl SoftDecoder {
    fn new(bps: usize, contestia: bool) -> Self {
        let block = if contestia { CONTESTIA_BLOCK } else { BLOCK };
        SoftDecoder { bps, block, contestia, ring: vec![0.0; block * bps], ptr: 0 }
    }

    fn input(&mut self, soft: &[f32; MAX_BPS]) {
        self.ring[self.ptr..self.ptr + self.bps].copy_from_slice(&soft[..self.bps]);
        self.ptr += self.bps;
        if self.ptr >= self.ring.len() {
            self.ptr = 0;
        }
    }

    /// Decode the most recent block of symbols. Returns
    /// (signal, noise energy, decoded) with fldigi's `Signal`/`NoiseEnergy`.
    fn process(&self) -> (f32, f32, Decoded) {
        let len = self.ring.len();
        let mut signal = 0.0f32;
        let mut noise = 0.0f32;
        let mut dec = Decoded::default();
        for slot in 0..self.bps {
            let mut whole = [0.0f32; BLOCK];
            let buf = &mut whole[..self.block];
            // `ptr` is the oldest symbol: time bit 0 of the block.
            let mut p = self.ptr;
            let mut rot = slot;
            for (t, cell) in buf.iter_mut().enumerate() {
                let b = self.ring[p + rot];
                *cell = if scramble_flip(slot, t, self.contestia) { -b } else { b };
                rot += 1;
                if rot == self.bps {
                    rot = 0;
                }
                p += self.bps;
                if p >= len {
                    p -= len;
                }
            }
            fht(buf);
            let (mut peak, mut pos, mut sq) = (0.0f32, 0usize, 0.0f32);
            for (i, &v) in buf.iter().enumerate() {
                sq += v * v;
                if v.abs() > peak.abs() {
                    peak = v;
                    pos = i;
                }
            }
            let ch = pos as u8 + if peak < 0.0 { self.block as u8 } else { 0 };
            dec.chars[slot] = if self.contestia { contestia_decode(ch) } else { ch };
            let n_e = (sq - peak * peak) / (self.block - 1) as f32;
            noise += n_e;
            signal += peak.abs();
        }
        signal /= self.bps as f32;
        noise /= self.bps as f32;
        dec.snr = if noise > 0.0 { signal / noise.sqrt() } else { 0.0 };
        (signal, noise, dec)
    }
}

pub struct OliviaRx {
    rate: f64,
    g: Geom,
    /// The last `n` input samples (oldest first), zero-primed.
    hist: VecDeque<f32>,
    /// New samples still needed before the next half-symbol spectrum.
    pending: usize,
    /// Window × cos/sin tables for the `n`-point DFT.
    win: Vec<f32>,
    cos: Vec<f32>,
    sin: Vec<f32>,
    /// Bin energies for bins `first - margin ..= first + 2(tones-1) + margin`.
    energy: Vec<f32>,
    /// Decoders indexed `[slice * offsets + offset]`.
    decoders: Vec<SoftDecoder>,
    /// Integrated sync signal / noise, `[phase * offsets + offset]`.
    sync_signal: Vec<LowPass3>,
    sync_noise: Vec<LowPass3>,
    /// Decode pipeline, `[(phase * offsets + offset) * SYNC_INTEG + k]`.
    pipe: Vec<Decoded>,
    pipe_ptr: Vec<usize>,
    block_phase: usize,
    best_signal: f32,
    best_phase: usize,
    best_offset: usize,
    snr: f32,
    mag: f32,
}

impl OliviaRx {
    pub fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        Self::build(rate, audio_hz, tones, bw, false)
    }

    /// A Contestia receiver — see [`OliviaTx::contestia`].
    pub fn contestia(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        Self::build(rate, audio_hz, tones, bw, true)
    }

    fn build(rate: f64, audio_hz: f64, tones: usize, bw: f64, contestia: bool) -> Self {
        let g = Geom::new(rate, audio_hz, tones, bw, contestia);
        let n = g.n;
        let offs = g.offsets();
        let phases = g.phases();
        let tau = std::f64::consts::TAU;
        OliviaRx {
            rate,
            hist: std::iter::repeat_n(0.0, n).collect(),
            pending: n / 4,
            win: (0..n)
                .map(|t| ((1.0 - (tau * t as f64 / n as f64).cos()) / n as f64) as f32)
                .collect(),
            cos: (0..n).map(|i| (tau * i as f64 / n as f64).cos() as f32).collect(),
            sin: (0..n).map(|i| (tau * i as f64 / n as f64).sin() as f32).collect(),
            energy: vec![0.0; 2 * (g.tones - 1) + 1 + 2 * g.margin],
            decoders: (0..SLICES * offs).map(|_| SoftDecoder::new(g.bps, contestia)).collect(),
            sync_signal: vec![LowPass3::default(); phases * offs],
            sync_noise: vec![LowPass3::default(); phases * offs],
            pipe: vec![Decoded::default(); phases * offs * SYNC_INTEG],
            pipe_ptr: vec![0; phases],
            block_phase: 0,
            best_signal: 0.0,
            best_phase: 0,
            best_offset: 0,
            snr: 0.0,
            mag: 0.0,
            g,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, tones: usize, bw: f64) {
        *self = OliviaRx::build(self.rate, audio_hz, tones, bw, self.g.contestia);
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        for &s in audio {
            self.hist.pop_front();
            self.hist.push_back(s);
            self.pending -= 1;
            if self.pending == 0 {
                self.pending = self.g.n / 4;
                self.spectrum();
                self.slice(&mut out);
            }
        }
        out
    }

    /// Raised-cosine windowed DFT of the last `n` samples at the bins of
    /// interest (fldigi's `MFSK_Demodulator::Process`, one slice).
    fn spectrum(&mut self) {
        let n = self.g.n;
        let lo = self.g.first - self.g.margin;
        let (a, b) = self.hist.as_slices();
        let mut peak = 0.0f32;
        for (i, e) in self.energy.iter_mut().enumerate() {
            let bin = lo + i;
            let (mut re, mut im) = (0.0f32, 0.0f32);
            let mut idx = 0usize;
            for (t, &x) in a.iter().chain(b.iter()).enumerate() {
                let v = x * self.win[t];
                re += v * self.cos[idx];
                im -= v * self.sin[idx];
                idx += bin;
                if idx >= n {
                    idx -= n;
                }
            }
            *e = re * re + im * im;
            peak = peak.max(*e);
        }
        self.mag += 0.02 * (peak.sqrt() - self.mag);
    }

    /// `MFSK_Demodulator::SoftDecode` at frequency offset `off` (0..offsets).
    fn soft_decode(&self, off: usize) -> [f32; MAX_BPS] {
        let mut sym = [0.0f32; MAX_BPS];
        let mut total = 0.0f32;
        for k in 0..self.g.tones {
            let e = self.energy[off + 2 * k];
            let e = e * e;
            total += e;
            let v = ungray(k as u32);
            for (bit, s) in sym.iter_mut().enumerate().take(self.g.bps) {
                if (v >> bit) & 1 == 1 {
                    *s -= e;
                } else {
                    *s += e;
                }
            }
        }
        if total > 0.0 {
            sym.iter_mut().for_each(|s| *s /= total);
        }
        sym
    }

    /// One half-symbol step of `MFSK_Receiver::ProcessSymbol`.
    fn slice(&mut self, out: &mut String) {
        let offs = self.g.offsets();
        let bp = self.block_phase;
        let slice = bp % SLICES;
        let w = 1.0 / SYNC_INTEG as f32;
        let wp = self.pipe_ptr[bp];
        let (mut best_sig, mut best_off) = (0.0f32, 0usize);
        for o in 0..offs {
            let soft = self.soft_decode(o);
            let dec = &mut self.decoders[slice * offs + o];
            dec.input(&soft);
            let (signal, noise, decoded) = dec.process();
            let cell = bp * offs + o;
            self.sync_noise[cell].process(noise, w);
            self.sync_signal[cell].process(signal, w);
            let s = self.sync_signal[cell].out;
            if s > best_sig {
                best_sig = s;
                best_off = o;
            }
            self.pipe[cell * SYNC_INTEG + wp] = decoded;
        }
        self.pipe_ptr[bp] = (wp + 1) % SYNC_INTEG;

        if bp == self.best_phase || best_sig > self.best_signal {
            self.best_signal = best_sig;
            self.best_phase = bp;
            self.best_offset = best_off;
        }

        // Half a block after the best phase, print its oldest pipelined block.
        let phases = self.g.phases();
        let dist = (bp + phases - self.best_phase) % phases;
        if dist == phases / 2 {
            let cell = self.best_phase * offs + self.best_offset;
            let noise = self.sync_noise[cell].out.max(0.0).sqrt();
            self.snr = if noise > 0.0 { self.best_signal / noise } else { 0.0 };
            let d = self.pipe[cell * SYNC_INTEG + self.pipe_ptr[self.best_phase]];
            if self.snr >= SYNC_THRESHOLD || d.snr >= BLOCK_THRESHOLD {
                for &c in d.chars.iter().take(self.g.bps) {
                    // fldigi prints everything above code 7 (BS, TAB, LF, CR…).
                    if c > 7 && c < 127 {
                        out.push(c as char);
                    }
                }
            }
        }

        self.block_phase = (bp + 1) % self.g.phases();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tone indices (after Gray coding) of one block, produced by fldigi's own
    /// `MFSK_Encoder` from `pj_mfsk.h` (compiled unmodified) — not by this file.
    const FLDIGI_32_CQ_DE: [u8; 64] = [
        11, 6, 19, 27, 28, 8, 9, 23, 16, 9, 14, 16, 13, 10, 17, 1, 7, 23, 14, 21, 12, 31, 3, 10, 6,
        30, 6, 21, 18, 17, 30, 4, 7, 24, 12, 8, 21, 23, 20, 1, 28, 16, 9, 14, 29, 31, 29, 2, 25,
        30, 4, 8, 24, 3, 22, 22, 8, 5, 25, 21, 21, 3, 28, 6,
    ];
    const FLDIGI_8_ABC: [u8; 64] = [
        5, 4, 7, 0, 1, 1, 2, 1, 3, 1, 4, 5, 1, 1, 6, 2, 5, 6, 2, 5, 6, 7, 7, 6, 1, 5, 3, 5, 6, 1,
        3, 5, 2, 2, 0, 6, 2, 4, 7, 6, 0, 2, 2, 5, 1, 6, 5, 2, 2, 6, 5, 2, 3, 7, 7, 4, 1, 3, 1, 4,
        6, 6, 4, 6,
    ];
    const FLDIGI_16_TEST: [u8; 64] = [
        1, 4, 13, 2, 0, 15, 4, 10, 2, 15, 12, 13, 7, 4, 11, 4, 1, 0, 9, 0, 12, 8, 7, 12, 14, 6, 1,
        9, 2, 1, 6, 13, 1, 8, 4, 14, 4, 1, 4, 10, 8, 6, 14, 15, 5, 4, 7, 13, 1, 4, 7, 0, 0, 2, 13,
        8, 0, 5, 1, 3, 11, 0, 2, 3,
    ];
    /// 4 tones, idle block (two NULs).
    const FLDIGI_4_IDLE: [u8; 64] = [
        3, 1, 1, 2, 0, 2, 1, 2, 0, 0, 1, 1, 1, 3, 2, 0, 2, 0, 1, 0, 1, 0, 0, 1, 1, 1, 3, 3, 3, 2,
        0, 0, 3, 1, 3, 1, 2, 1, 1, 2, 0, 2, 1, 0, 3, 3, 1, 3, 2, 2, 2, 0, 1, 1, 2, 0, 3, 2, 3, 0,
        0, 2, 1, 2,
    ];

    fn tones_of(text: &[u8], bps: usize) -> Vec<u8> {
        let mut chars = [0u8; MAX_BPS];
        chars[..text.len()].copy_from_slice(text);
        encode_block(&chars[..bps], bps, false).iter().map(|&s| gray(u32::from(s)) as u8).collect()
    }

    #[test]
    fn encoder_matches_fldigi() {
        assert_eq!(tones_of(b"CQ DE", 5), FLDIGI_32_CQ_DE);
        assert_eq!(tones_of(b"ABC", 3), FLDIGI_8_ABC);
        assert_eq!(tones_of(b"TEST", 4), FLDIGI_16_TEST);
        assert_eq!(tones_of(b"", 2), FLDIGI_4_IDLE);
    }

    /// The soft decoder recovers fldigi's tone sequence (hard ±1 soft bits),
    /// and only at the right block phase.
    #[test]
    fn decoder_reads_fldigi_block() {
        let run = |tones: &[u8; 64], bps: usize, shift: usize| {
            let mut d = SoftDecoder::new(bps, false);
            for t in 0..BLOCK {
                let v = ungray(u32::from(tones[(t + shift) % BLOCK]));
                let mut s = [0.0f32; MAX_BPS];
                for (b, x) in s.iter_mut().enumerate().take(bps) {
                    *x = if (v >> b) & 1 == 1 { -1.0 } else { 1.0 };
                }
                d.input(&s);
            }
            d.process().2.chars[..bps].to_vec()
        };
        assert_eq!(run(&FLDIGI_32_CQ_DE, 5, 0), b"CQ DE");
        assert_eq!(run(&FLDIGI_8_ABC, 3, 0), b"ABC");
        for shift in 1..BLOCK {
            assert_ne!(run(&FLDIGI_32_CQ_DE, 5, shift), b"CQ DE", "shift {shift}");
        }
    }

    #[test]
    fn geometry_matches_fldigi() {
        // fldigi PrintParameters at 1500 Hz: 32/1000 → SymbolLen 512, tone 0 at
        // bin 65 (1015.625 Hz), 31.25 baud.
        let g = Geom::new(8000.0, 1500.0, 32, 1000.0, false);
        assert_eq!((g.n, g.sps, g.first, g.bps), (512, 256, 65, 5));
        let g = Geom::new(8000.0, 1500.0, 8, 250.0, false);
        assert_eq!((g.n, g.first), (512, 89));
        let g = Geom::new(8000.0, 1500.0, 4, 250.0, false);
        assert_eq!(g.n, 256);
    }

    fn run(tones: usize, bw: f64, msg: &str) -> String {
        run_kind(tones, bw, msg, false)
    }

    fn run_kind(tones: usize, bw: f64, msg: &str, contestia: bool) -> String {
        let rate = 8000.0;
        let audio = 1500.0;
        let mut tx = if contestia {
            OliviaTx::contestia(rate, audio, tones, bw)
        } else {
            OliviaTx::new(rate, audio, tones, bw)
        };
        let blk = tx.g.sps * tx.g.block;
        let mut sig = vec![0.0f32; blk];
        let mut warm = vec![0.0f32; blk * 2];
        tx.next_block(&mut warm);
        sig.extend_from_slice(&warm);
        tx.push_text(msg);
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 40_000 {
            let mut b = [0.0f32; 2048];
            tx.next_block(&mut b);
            sig.extend_from_slice(&b);
            guard += 1;
        }
        let mut tail = vec![0.0f32; blk * 2];
        tx.next_block(&mut tail);
        sig.extend_from_slice(&tail);
        // Let the decode pipeline drain.
        sig.extend(std::iter::repeat_n(0.0, blk * (SYNC_INTEG + 2)));
        let mut rx = if contestia {
            OliviaRx::contestia(rate, audio, tones, bw)
        } else {
            OliviaRx::new(rate, audio, tones, bw)
        };
        let mut decoded = String::new();
        for chunk in sig.chunks(512) {
            decoded.push_str(&rx.process(chunk));
        }
        decoded
    }

    #[test]
    fn loopback_32_1000() {
        let msg = "CQ DE AB1CD";
        let got = run(32, 1000.0, msg);
        assert!(got.contains(msg), "decoded {got:?} did not contain {msg:?}");
    }

    #[test]
    fn loopback_8_250() {
        let msg = "TEST OLIVIA";
        let got = run(8, 250.0, msg);
        assert!(got.contains(msg), "decoded {got:?} did not contain {msg:?}");
    }

    /// Off-tune by 10 Hz (one FFT bin, a third of a tone): the frequency search
    /// must still lock.
    #[test]
    fn loopback_mistuned() {
        let rate = 8000.0;
        let mut tx = OliviaTx::new(rate, 1510.0, 32, 1000.0);
        let blk = tx.g.sps * BLOCK;
        tx.push_text("MISTUNED OK");
        let mut sig = vec![0.0f32; blk * 6];
        tx.next_block(&mut sig);
        sig.extend(std::iter::repeat_n(0.0, blk * (SYNC_INTEG + 2)));
        let mut rx = OliviaRx::new(rate, 1500.0, 32, 1000.0);
        let got: String = sig.chunks(700).map(|c| rx.process(c)).collect();
        assert!(got.contains("MISTUNED OK"), "decoded {got:?}");
    }

    /// Contestia's block: 32 symbols, the 0xEDB88320 scrambler at five bits a
    /// slot, and its own character map — a character survives the round trip
    /// through the encoder and the decoder, upper-cased.
    #[test]
    fn contestia_character_map_round_trips() {
        for &b in b"ABCXYZ 0189?/.\r" {
            assert_eq!(contestia_decode(contestia_encode(b)), b, "{:?}", b as char);
        }
        assert_eq!(contestia_decode(contestia_encode(b'q')), b'Q');
        let g = Geom::new(8000.0, 1500.0, 8, 250.0, true);
        assert_eq!(g.block, 32);
        // Tone 0 a quarter of a spacing lower than Olivia's before the bin
        // grid rounds it: the same bin at 1500 Hz, one bin lower at 1510.
        let o = Geom::new(8000.0, 1500.0, 8, 250.0, false);
        assert_eq!(o.first, g.first);
        let o = Geom::new(8000.0, 1510.0, 8, 250.0, false);
        let g = Geom::new(8000.0, 1510.0, 8, 250.0, true);
        assert_eq!(o.first - g.first, 1, "olivia {} contestia {}", o.first, g.first);
    }

    /// Contestia through its own receiver at the common speeds.
    #[test]
    fn loopback_contestia() {
        for (tones, bw) in [(4, 250.0), (8, 250.0), (16, 500.0), (32, 1000.0), (8, 500.0)] {
            let msg = "CQ DE AB1CD";
            let got = run_kind(tones, bw, msg, true);
            assert!(got.contains(msg), "{tones}/{bw:.0} decoded {got:?}");
        }
    }

    /// fldigi's own Contestia 4/125 recording, from sdroxide-brown's corpus
    /// (`CONTESTIA.wav`, centred near 1718 Hz).
    ///
    /// **The corpus is spectrally inverted**: its Olivia recordings read through
    /// this file's (fldigi-exact) Olivia receiver only once mirrored, too. So the
    /// audio is mirrored about the centre first; with that, the known text reads
    /// whole — measured on the way in, `AAA TEST TEST THE EAGLE HAS LANDED 12345
    /// SDRBROWN ZZZ`.
    #[test]
    #[ignore = "needs fldigi's recordings in samples/fldigi, kept in sdroxide-brown, not in this tree"]
    fn the_contestia_corpus_sample_decodes() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/CONTESTIA.wav");
        let mut reader = hound::WavReader::open(path).expect("the Contestia sample");
        let spec = reader.spec();
        let rate = spec.sample_rate as f64;
        // Mirror about 1718 Hz: multiplying by a carrier at twice the centre
        // folds f to 2·fc − f; the sum frequency lands far outside the bank.
        let fold = std::f64::consts::TAU * 2.0 * 1718.0 / rate;
        let audio: Vec<f32> = reader
            .samples::<i16>()
            .enumerate()
            .map(|(i, s)| 2.0 * (s.expect("s") as f64 / 32_768.0 * (fold * i as f64).cos()) as f32)
            .collect();
        let (tones, bw) = (4, 125.0);
        let mut best = String::new();
        for hz in (1650..=1800).step_by(10) {
            let mut rx = OliviaRx::contestia(rate, hz as f64, tones, bw);
            let got: String = audio.chunks(512).map(|c| rx.process(c)).collect();
            if got.contains("EAGLE") {
                best = got;
                break;
            }
            if got.len() > best.len() {
                best = got;
            }
        }
        eprintln!("contestia corpus: {best:?}");
        assert!(best.contains("EAGLE"), "read {best:?}");
    }
}
