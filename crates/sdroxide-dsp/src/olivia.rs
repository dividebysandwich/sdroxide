//! Olivia MFSK modem: streaming decode + encode.
//!
//! Olivia is a strong, slow MFSK chat mode. A block of 64 MFSK symbols carries
//! `log2(tones)` characters: each character (7-bit ASCII) is spread across the
//! 64 symbols with a (64,7) biorthogonal Walsh/Hadamard code, one character per
//! MFSK bit-plane. The heavy coding gain is what makes Olivia decode well below
//! the noise floor. A fixed per-position tone scrambler spreads the spectrum and
//! (crucially) gives the receiver an unambiguous block boundary even during the
//! constant idle stream.
//!
//! Parameters are the tone count (2..=64) and the bandwidth in Hz; tone spacing =
//! bandwidth/tones and the symbol rate equals the spacing. Common combinations
//! are 32/1000, 16/500 and 8/250.
//!
//! Interop note: the block/Walsh structure, MFSK/Gray mapping and scrambler
//! follow the Olivia design, but the scrambler constants are not yet bit-matched
//! to fldigi (tracked for live validation). TX↔RX here is internally consistent
//! and unit-tested by loopback.

use std::collections::VecDeque;

use crate::mfsk::{ToneGen, fwht, gray, hadamard_bit, tone_bank_mags, ungray};

const OUT_AMP: f32 = 0.5;
/// Symbols per Olivia block (fixed by the 64-length Walsh code).
const BLOCK: usize = 64;
/// Timing sub-phases searched per symbol.
const SUBPHASES: usize = 16;

/// A fixed, deterministic per-position tone offset (0..tones-1), 64 long. Varies
/// with position so a mis-aligned block descrambles to noise (block sync) and the
/// on-air spectrum is spread.
fn make_scramble(tones: usize) -> [u8; BLOCK] {
    let mask = (tones - 1) as u32;
    let mut s = [0u8; BLOCK];
    let mut lfsr: u32 = 0x1D5;
    for cell in s.iter_mut() {
        // xorshift step — deterministic, no RNG.
        lfsr ^= lfsr << 7;
        lfsr ^= lfsr >> 9;
        lfsr ^= lfsr << 8;
        *cell = (lfsr & mask) as u8;
    }
    s
}

/// Resolved Olivia geometry for a (tones, bandwidth) pair at a sample rate.
#[derive(Clone, Copy)]
struct Geom {
    tones: usize,
    planes: usize,
    spacing: f64,
    sps: usize,
    base_hz: f64,
}

impl Geom {
    fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        let tones = tones.clamp(2, 64).next_power_of_two();
        let planes = tones.trailing_zeros() as usize; // log2(tones)
        let spacing = bw / tones as f64;
        let sps = (rate / spacing).round().max(16.0) as usize;
        // Centre the tone bank on the audio frequency.
        let base_hz = audio_hz - bw / 2.0 + spacing / 2.0;
        Geom { tones, planes, spacing, sps, base_hz }
    }

    fn tone_hz(&self, k: usize) -> f64 {
        self.base_hz + k as f64 * self.spacing
    }
}

/// The 0/1 code bit for character `byte` at symbol `i`: the natural-order
/// Hadamard row `byte&63`, complemented when bit 6 (`byte&64`) is set.
fn code_bit(byte: u8, i: usize) -> u32 {
    let mut hb = hadamard_bit((byte & 63) as usize, i);
    if byte & 64 != 0 {
        hb = -hb;
    }
    (hb > 0) as u32
}

// ─────────────────────────────── transmit ───────────────────────────────

pub struct OliviaTx {
    rate: f64,
    g: Geom,
    scramble: [u8; BLOCK],
    tonegen: ToneGen,
    /// Queued characters with their source index (`None` = NUL idle fill).
    q: VecDeque<(u8, Option<usize>)>,
    total_chars: usize,
    sent_chars: usize,
    cur: Vec<f32>,
    cur_pos: usize,
    cur_done: Option<usize>,
}

impl OliviaTx {
    pub fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        let g = Geom::new(rate, audio_hz, tones, bw);
        OliviaTx {
            rate,
            scramble: make_scramble(g.tones),
            g,
            tonegen: ToneGen::new(rate),
            q: VecDeque::new(),
            total_chars: 0,
            sent_chars: 0,
            cur: Vec::new(),
            cur_pos: 0,
            cur_done: None,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, tones: usize, bw: f64) {
        self.g = Geom::new(self.rate, audio_hz, tones, bw);
        self.scramble = make_scramble(self.g.tones);
    }

    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let byte = if ch.is_ascii() { ch as u8 } else { b'?' };
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
    }

    fn build_block(&mut self) {
        // Take up to `planes` characters; pad with NUL idle fill.
        let mut chars = [0u8; 6];
        let mut done: Option<usize> = None;
        for slot in chars.iter_mut().take(self.g.planes) {
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
        for i in 0..BLOCK {
            let mut v = 0u32;
            for (p, &b) in chars.iter().enumerate().take(self.g.planes) {
                v |= code_bit(b, i) << p;
            }
            let tone = (gray(v) as usize ^ self.scramble[i] as usize) % self.g.tones;
            self.tonegen.emit(self.g.tone_hz(tone), self.g.sps, OUT_AMP, &mut self.cur);
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

pub struct OliviaRx {
    rate: f64,
    g: Geom,
    scramble: [u8; BLOCK],
    /// Recent audio, indexed by absolute sample count via `buf_start`.
    buf: Vec<f32>,
    buf_start: usize,
    next_hop: usize,
    hop: usize,
    /// Symbol-timing energy per sub-phase (decayed).
    sync: [f32; SUBPHASES],
    /// Held sampling sub-phase (updated once per symbol period).
    tphase: usize,
    /// Per-symbol tone magnitudes (one `[tones]` vector per decoded symbol).
    sbuf: VecDeque<Vec<f32>>,
    /// Absolute symbol index of `sbuf[0]`.
    sbuf_base: usize,
    scount: usize,
    /// Block-alignment confidence per phase (0..63), decayed.
    phase_conf: [f32; BLOCK],
    locked: bool,
    /// Absolute symbol index up to which blocks have been emitted.
    emitted_upto: usize,
    mag: f32,
}

impl OliviaRx {
    pub fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        let g = Geom::new(rate, audio_hz, tones, bw);
        let hop = (g.sps / SUBPHASES).max(1);
        OliviaRx {
            rate,
            scramble: make_scramble(g.tones),
            g,
            buf: Vec::new(),
            buf_start: 0,
            next_hop: g.sps,
            hop,
            sync: [0.0; SUBPHASES],
            tphase: 0,
            sbuf: VecDeque::new(),
            sbuf_base: 0,
            scount: 0,
            phase_conf: [0.0; BLOCK],
            locked: false,
            emitted_upto: 0,
            mag: 0.0,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, tones: usize, bw: f64) {
        *self = OliviaRx::new(self.rate, audio_hz, tones, bw);
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        self.buf.extend_from_slice(audio);
        let n_total = self.buf_start + self.buf.len();
        while self.next_hop <= n_total {
            let t = self.next_hop;
            if t >= self.g.sps {
                let a = t - self.g.sps - self.buf_start;
                let b = t - self.buf_start;
                let mags = tone_bank_mags(
                    &self.buf[a..b],
                    self.g.base_hz,
                    self.g.spacing,
                    self.g.tones,
                    self.rate,
                );
                self.on_window(t, mags, &mut out);
            }
            self.next_hop += self.hop;
        }
        let keep_from = n_total.saturating_sub(self.g.sps);
        if keep_from > self.buf_start {
            self.buf.drain(0..keep_from - self.buf_start);
            self.buf_start = keep_from;
        }
        out
    }

    fn on_window(&mut self, t: usize, mags: Vec<f32>, out: &mut String) {
        let peak = mags.iter().copied().fold(0.0f32, f32::max);
        self.mag += 0.02 * (peak - self.mag);
        let phase = (t / self.hop) % SUBPHASES;
        for (i, s) in self.sync.iter_mut().enumerate() {
            *s *= 0.995;
            if i == phase {
                *s += peak;
            }
        }
        // Re-estimate the sampling sub-phase once per symbol period, then hold it
        // so exactly one soft symbol is taken per period (keeps the symbol count
        // aligned to the transmit block grid).
        if phase == 0 {
            self.tphase = self
                .sync
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
        if phase != self.tphase {
            return;
        }
        // Sampling instant: keep the whole magnitude vector so decode_block can
        // descramble by block position.
        self.sbuf.push_back(mags);
        self.scount += 1;
        while self.sbuf.len() > 1024 {
            self.sbuf.pop_front();
            self.sbuf_base += 1;
        }
        self.try_decode(out);
    }

    /// Soft `planes` bits for the symbol at absolute index `abs`, descrambled for
    /// its block position `pos` (0..63).
    fn soft_bits(&self, abs: usize, pos: usize) -> [f32; 6] {
        let mags = &self.sbuf[abs - self.sbuf_base];
        let mut soft = [0.0f32; 6];
        let scr = self.scramble[pos] as usize;
        for (k, &m) in mags.iter().enumerate() {
            let v = ungray((k ^ scr) as u32);
            for (p, sp) in soft.iter_mut().enumerate().take(self.g.planes) {
                if (v >> p) & 1 == 1 {
                    *sp += m;
                } else {
                    *sp -= m;
                }
            }
        }
        soft
    }

    /// Confidence + decoded chars for the 64-symbol block starting at absolute
    /// index `start`.
    fn decode_block(&self, start: usize) -> (f32, [u8; 6]) {
        let mut planes_soft = [[0.0f32; BLOCK]; 6];
        for i in 0..BLOCK {
            let soft = self.soft_bits(start + i, i);
            for p in 0..self.g.planes {
                planes_soft[p][i] = soft[p];
            }
        }
        let mut conf = 0.0f32;
        let mut chars = [0u8; 6];
        for p in 0..self.g.planes {
            let mut sc = planes_soft[p];
            fwht(&mut sc);
            let (m, val) = sc
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, v)| (i, *v))
                .unwrap_or((0, 0.0));
            conf += val.abs();
            chars[p] = (m as u8 & 63) | if val < 0.0 { 64 } else { 0 };
        }
        (conf, chars)
    }

    fn try_decode(&mut self, out: &mut String) {
        // Need a full block available.
        if self.scount < self.sbuf_base + BLOCK {
            return;
        }
        // Score the block that just completed against its start phase.
        let start = self.scount - BLOCK;
        if start >= self.sbuf_base {
            let (conf, _) = self.decode_block(start);
            for c in self.phase_conf.iter_mut() {
                *c *= 0.997;
            }
            self.phase_conf[start % BLOCK] += conf;
        }
        // Lock once one phase clearly dominates.
        let (best_phase, best, second) = {
            let mut bp = 0;
            let mut b = -1.0f32;
            let mut s = -1.0f32;
            for (i, &c) in self.phase_conf.iter().enumerate() {
                if c > b {
                    s = b;
                    b = c;
                    bp = i;
                } else if c > s {
                    s = c;
                }
            }
            (bp, b, s)
        };
        if !self.locked {
            if self.scount < 2 * BLOCK || best < 1.4 * second.max(1e-6) {
                return;
            }
            self.locked = true;
            // Start emitting from the earliest buffered block at this phase.
            let base = self.sbuf_base;
            let first = base + ((best_phase + BLOCK - base % BLOCK) % BLOCK);
            self.emitted_upto = first;
        }
        // Emit any newly-complete aligned blocks (phase fixed once locked).
        let phase = best_phase;
        while self.emitted_upto >= self.sbuf_base
            && self.emitted_upto % BLOCK == phase % BLOCK
            && self.emitted_upto + BLOCK <= self.scount
        {
            let (_, chars) = self.decode_block(self.emitted_upto);
            for &b in chars.iter().take(self.g.planes) {
                if b != 0 {
                    out.push(b as char);
                }
            }
            self.emitted_upto += BLOCK;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(tones: usize, bw: f64, msg: &str) -> String {
        let rate = 8000.0;
        let audio = 1500.0;
        let mut tx = OliviaTx::new(rate, audio, tones, bw);
        let mut sig = Vec::new();
        // Idle runway so the RX acquires timing + block alignment.
        let mut warm = vec![0.0f32; tx.g.sps * BLOCK * 3];
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
        // Flush trailing blocks so the last message block is fully sent.
        let mut tail = vec![0.0f32; tx.g.sps * BLOCK * 2];
        tx.next_block(&mut tail);
        sig.extend_from_slice(&tail);

        let mut rx = OliviaRx::new(rate, audio, tones, bw);
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
}
