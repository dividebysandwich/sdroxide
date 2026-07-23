//! FSQ (Fast Simple QSO) modem: streaming decode + encode.
//!
//! FSQ is a 33-tone **incremental frequency keying** chat mode: each symbol's
//! tone is `(prev + code + 1) mod 33`, so it carries 5 bits and never repeats a
//! tone. Unlike THOR there is no forward error correction — robustness comes from
//! the tone spacing and (in the directed FSQCALL layer) message checksums. The
//! text rides the same streaming Varicode as PSK/THOR. Speeds: FSQ-2/3/4.5/6.
//!
//! Interop note: the IFK structure and Varicode framing follow FSQ, but the exact
//! tone spacing (this uses an orthogonal grid for clean DFT detection) and the
//! FSQCALL checksum are not yet bit-matched to fldigi (tracked for live
//! validation). TX↔RX here is internally consistent and unit-tested by loopback.

use std::collections::VecDeque;

use crate::mfsk::{MfskClock, ToneGen};
use crate::psk::{VaricodeRx, varicode};

const OUT_AMP: f32 = 0.5;
/// FSQ tone count.
const NUMTONES: usize = 33;
/// Bits carried per symbol (log2 rounded down of the usable increments).
const BITS_PER_SYM: usize = 5;

struct Geom {
    spacing: f64,
    sps: usize,
    base_hz: f64,
}

impl Geom {
    fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        let sps = (rate / baud).round().max(16.0) as usize;
        let spacing = rate / sps as f64;
        let bw = NUMTONES as f64 * spacing;
        let base_hz = audio_hz - bw / 2.0 + spacing / 2.0;
        Geom { spacing, sps, base_hz }
    }

    fn tone_hz(&self, k: usize) -> f64 {
        self.base_hz + k as f64 * self.spacing
    }
}

// ─────────────────────────────── transmit ───────────────────────────────

pub struct FsqTx {
    rate: f64,
    g: Geom,
    tonegen: ToneGen,
    prev_tone: usize,
    bitq: VecDeque<(u8, Option<usize>)>,
    total_chars: usize,
    sent_chars: usize,
    cur: Vec<f32>,
    cur_pos: usize,
    cur_done: Option<usize>,
}

impl FsqTx {
    pub fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        FsqTx {
            rate,
            g: Geom::new(rate, audio_hz, baud),
            tonegen: ToneGen::new(rate),
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
        let mut code = 0usize;
        let mut done = None;
        for _ in 0..BITS_PER_SYM {
            let (b, tag) = self.bitq.pop_front().unwrap_or((0, None));
            if let Some(i) = tag {
                done = Some(done.map_or(i, |d: usize| d.max(i)));
            }
            code = (code << 1) | b as usize;
        }
        let tone = (self.prev_tone + code + 1) % NUMTONES;
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

pub struct FsqRx {
    rate: f64,
    clock: MfskClock,
    prev_tone: Option<usize>,
    vrx: VaricodeRx,
}

impl FsqRx {
    pub fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        let g = Geom::new(rate, audio_hz, baud);
        FsqRx {
            rate,
            clock: MfskClock::new(rate, g.base_hz, g.spacing, NUMTONES, g.sps),
            prev_tone: None,
            vrx: VaricodeRx::new(),
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, baud: f64) {
        let g = Geom::new(self.rate, audio_hz, baud);
        self.clock = MfskClock::new(self.rate, g.base_hz, g.spacing, NUMTONES, g.sps);
        self.prev_tone = None;
        self.vrx = VaricodeRx::new();
    }

    pub fn magnitude(&self) -> f32 {
        self.clock.peak_mag()
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        for mags in self.clock.feed(audio) {
            let tone = mags
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            if let Some(prev) = self.prev_tone {
                // +2·NUMTONES guards the unsigned subtraction against a downward
                // tone step underflowing.
                let code = ((tone + 2 * NUMTONES - prev - 1) % NUMTONES) & ((1 << BITS_PER_SYM) - 1);
                for shift in (0..BITS_PER_SYM).rev() {
                    if let Some(c) = self.vrx.push_bit(((code >> shift) & 1) as u8) {
                        out.push(c);
                    }
                }
            }
            self.prev_tone = Some(tone);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(baud: f64, msg: &str) -> String {
        let rate = 8000.0;
        let audio = 1500.0;
        let mut tx = FsqTx::new(rate, audio, baud);
        let mut sig = Vec::new();
        let mut warm = vec![0.0f32; tx.g.sps * 30];
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
        let mut tail = vec![0.0f32; tx.g.sps * 40];
        tx.next_block(&mut tail);
        sig.extend_from_slice(&tail);

        let mut rx = FsqRx::new(rate, audio, baud);
        let mut decoded = String::new();
        for chunk in sig.chunks(512) {
            decoded.push_str(&rx.process(chunk));
        }
        decoded
    }

    #[test]
    fn loopback_fsq45() {
        let msg = "CQ DE AB1CD FSQ";
        let got = run(4.5, msg);
        assert!(got.contains(msg), "decoded {got:?} did not contain {msg:?}");
    }
}
