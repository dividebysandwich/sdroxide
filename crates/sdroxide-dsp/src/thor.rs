//! THOR modem (DominoEX family): streaming decode + encode.
//!
//! THOR is MFSK over 18 tones using **incremental frequency keying** (IFK+): the
//! data is carried by the *difference* between successive tones, so the decoder
//! needs no absolute frequency reference. Each symbol carries one nibble (4 coded
//! bits); the text is Varicode-coded and protected by a K=7 rate-1/2
//! convolutional FEC (soft/​hard-decision Viterbi on receive). Submodes select
//! the symbol rate (THOR4 … THOR32).
//!
//! Interop note: the IFK+ increment, FEC polynomials and Varicode follow the
//! DominoEX/THOR design, but exact interleaving is not yet bit-matched to fldigi
//! (tracked for live validation). TX↔RX here is internally consistent and
//! unit-tested by loopback.

use std::collections::VecDeque;

use crate::fec::{ConvEnc, viterbi_decode};
use crate::mfsk::{MfskClock, ToneGen};
use crate::psk::{VaricodeRx, varicode};

const OUT_AMP: f32 = 0.5;
/// THOR/DominoEX tone count.
const NUMTONES: usize = 18;
/// Minimum IFK tone increment (blocks a repeated/adjacent tone).
const IFK_BASE: usize = 2;
/// Drop this many trailing decoded bits each pass — the Viterbi traceback tail
/// is unreliable until more coded bits arrive to confirm it.
const TAIL_BITS: usize = 48;
/// Bound the streaming-decode buffer.
const CODED_CAP: usize = 40_000;

struct Geom {
    spacing: f64,
    sps: usize,
    base_hz: f64,
}

impl Geom {
    fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        let sps = (rate / baud).round().max(16.0) as usize;
        let spacing = rate / sps as f64; // exact bin spacing → orthogonal tones
        let bw = NUMTONES as f64 * spacing;
        let base_hz = audio_hz - bw / 2.0 + spacing / 2.0;
        Geom { spacing, sps, base_hz }
    }

    fn tone_hz(&self, k: usize) -> f64 {
        self.base_hz + k as f64 * self.spacing
    }
}

// ─────────────────────────────── transmit ───────────────────────────────

pub struct ThorTx {
    rate: f64,
    g: Geom,
    tonegen: ToneGen,
    enc: ConvEnc,
    prev_tone: usize,
    /// Message (Varicode) bits with a source-char index on the last bit.
    bitq: VecDeque<(u8, Option<usize>)>,
    total_chars: usize,
    sent_chars: usize,
    cur: Vec<f32>,
    cur_pos: usize,
    cur_done: Option<usize>,
}

impl ThorTx {
    pub fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        ThorTx {
            rate,
            g: Geom::new(rate, audio_hz, baud),
            tonegen: ToneGen::new(rate),
            enc: ConvEnc::new(),
            prev_tone: 0,
            bitq: VecDeque::new(),
            total_chars: 0,
            sent_chars: 0,
            cur: Vec::new(),
            cur_pos: 0,
            cur_done: None,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, baud: f64) {
        self.g = Geom::new(self.rate, audio_hz, baud);
    }

    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let byte = if ch.is_ascii() { ch as u8 } else { b'?' };
            let code = varicode(byte);
            let code = if code.is_empty() { varicode(b'?') } else { code };
            for c in code.chars() {
                self.bitq.push_back(((c == '1') as u8, None));
            }
            // "00" inter-character separator; tag the last with the char index.
            self.bitq.push_back((0, None));
            self.bitq.push_back((0, Some(self.total_chars)));
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
        self.bitq.is_empty() && self.cur_pos >= self.cur.len()
    }

    pub fn clear(&mut self) {
        self.bitq.clear();
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = None;
        self.total_chars = 0;
        self.sent_chars = 0;
    }

    fn build_symbol(&mut self) {
        let mut nib = 0u8;
        let mut done = None;
        for _ in 0..2 {
            let (mb, tag) = self.bitq.pop_front().unwrap_or((0, None));
            if let Some(i) = tag {
                done = Some(done.map_or(i, |d: usize| d.max(i)));
            }
            let (o1, o2) = self.enc.encode_bit(mb);
            nib = (nib << 1) | o1;
            nib = (nib << 1) | o2;
        }
        let tone = (self.prev_tone + nib as usize + IFK_BASE) % NUMTONES;
        self.prev_tone = tone;
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = done;
        self.tonegen.emit(self.g.tone_hz(tone), self.g.sps, OUT_AMP, &mut self.cur);
    }

    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.cur_pos >= self.cur.len() {
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                self.build_symbol();
            }
            out[n] = self.cur[self.cur_pos];
            self.cur_pos += 1;
            n += 1;
        }
        n
    }
}

// ─────────────────────────────── receive ───────────────────────────────

pub struct ThorRx {
    rate: f64,
    clock: MfskClock,
    prev_tone: Option<usize>,
    coded: Vec<u8>,
    text: String,
    emitted: usize,
}

impl ThorRx {
    pub fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        let g = Geom::new(rate, audio_hz, baud);
        ThorRx {
            rate,
            clock: MfskClock::new(rate, g.base_hz, g.spacing, NUMTONES, g.sps),
            prev_tone: None,
            coded: Vec::new(),
            text: String::new(),
            emitted: 0,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, baud: f64) {
        let g = Geom::new(self.rate, audio_hz, baud);
        self.clock = MfskClock::new(self.rate, g.base_hz, g.spacing, NUMTONES, g.sps);
        self.prev_tone = None;
        self.coded.clear();
        self.text.clear();
        self.emitted = 0;
    }

    pub fn magnitude(&self) -> f32 {
        self.clock.peak_mag()
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        let syms = self.clock.feed(audio);
        for mags in syms {
            let tone = mags
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            if let Some(prev) = self.prev_tone {
                // Add 2·NUMTONES before subtracting so a downward tone step (e.g.
                // tone 0 after tone 17) can't underflow the unsigned arithmetic.
                let nib = ((tone + 2 * NUMTONES - prev - IFK_BASE) % NUMTONES) as u8 & 0x0F;
                for shift in (0..4).rev() {
                    self.coded.push((nib >> shift) & 1);
                }
            }
            self.prev_tone = Some(tone);
        }
        if !self.coded.is_empty() {
            self.decode(&mut out);
        }
        out
    }

    fn decode(&mut self, out: &mut String) {
        if self.coded.len() > CODED_CAP {
            let drop = self.coded.len() - CODED_CAP / 2;
            self.coded.drain(0..drop);
            // Re-baseline: whatever we decode now is a fresh prefix.
            self.text.clear();
            self.emitted = 0;
        }
        let mut bits = viterbi_decode(&self.coded, false);
        // The trailing bits are the unreliable Viterbi tail; drop them and decode
        // only the confirmed prefix. Trailing idle keeps extending the buffer, so
        // the real message bits are always well inside the confirmed region.
        bits.truncate(bits.len().saturating_sub(TAIL_BITS));
        let mut vr = VaricodeRx::new();
        let mut text = String::new();
        for b in bits {
            if let Some(c) = vr.push_bit(b) {
                text.push(c);
            }
        }
        let count = text.chars().count();
        if count > self.emitted {
            let add: String = text.chars().skip(self.emitted).collect();
            out.push_str(&add);
            self.emitted = count;
        }
        self.text = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(baud: f64, msg: &str) -> String {
        let rate = 8000.0;
        let audio = 1500.0;
        let mut tx = ThorTx::new(rate, audio, baud);
        let mut sig = Vec::new();
        let mut warm = vec![0.0f32; tx.g.sps * 40];
        tx.next_block(&mut warm);
        sig.extend_from_slice(&warm);

        tx.push_text(msg);
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 200_000 {
            let mut b = [0.0f32; 2048];
            tx.next_block(&mut b);
            sig.extend_from_slice(&b);
            guard += 1;
        }
        // Trailing idle so the last chars pass the hold-back.
        let mut tail = vec![0.0f32; tx.g.sps * 60];
        tx.next_block(&mut tail);
        sig.extend_from_slice(&tail);

        let mut rx = ThorRx::new(rate, audio, baud);
        let mut decoded = String::new();
        for chunk in sig.chunks(512) {
            decoded.push_str(&rx.process(chunk));
        }
        decoded
    }

    #[test]
    fn loopback_thor16() {
        let msg = "CQ DE AB1CD THOR";
        let got = run(15.625, msg);
        assert!(got.contains(msg), "decoded {got:?} did not contain {msg:?}");
    }

    #[test]
    fn noise_does_not_panic() {
        // Garbage tone transitions (e.g. a downward step tone 0 after tone 17)
        // must not underflow the IFK arithmetic — this used to panic on mode
        // switches. Feed a deterministic pseudo-random signal.
        let mut rx = ThorRx::new(8000.0, 1500.0, 15.625);
        let mut lfsr: u32 = 0xACE1;
        for _ in 0..200 {
            let mut chunk = [0.0f32; 512];
            for s in chunk.iter_mut() {
                lfsr ^= lfsr << 13;
                lfsr ^= lfsr >> 17;
                lfsr ^= lfsr << 5;
                *s = (lfsr as f32 / u32::MAX as f32) * 2.0 - 1.0;
            }
            let _ = rx.process(&chunk);
        }
    }
}
