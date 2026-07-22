//! RTTY modem: streaming decode + encode of Baudot (ITA2) FSK.
//!
//! Standard amateur RTTY: 45.45 baud, 170 Hz shift, 5-bit Baudot with
//! LTRS/FIGS shift, 1 start bit (space) + 5 data (LSB first) + 1.5 stop (mark),
//! idle = mark. USB-side: mark is the higher tone (`center + shift/2`), space
//! the lower. Baud and shift are configurable.
//!
//! RX down-converts to complex baseband at the operator's center tone, and an
//! FM discriminator (sign of the instantaneous frequency) slices mark vs space;
//! a UART state machine frames characters. TX renders phase-continuous FSK.

use std::collections::VecDeque;
use std::f64::consts::TAU;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};

// ── Baudot / ITA2 (US-TTY figures) ──────────────────────────────────────
const LTRS: u8 = 31;
const FIGS: u8 = 27;
const BAUDOT_SPACE: u8 = 4;
const BAUDOT_CR: u8 = 2;
const BAUDOT_LF: u8 = 8;

fn letter_char(v: u8) -> Option<char> {
    Some(match v {
        3 => 'A', 25 => 'B', 14 => 'C', 9 => 'D', 1 => 'E', 13 => 'F', 26 => 'G',
        20 => 'H', 6 => 'I', 11 => 'J', 15 => 'K', 18 => 'L', 28 => 'M', 12 => 'N',
        24 => 'O', 22 => 'P', 23 => 'Q', 10 => 'R', 5 => 'S', 16 => 'T', 7 => 'U',
        30 => 'V', 19 => 'W', 29 => 'X', 21 => 'Y', 17 => 'Z',
        _ => return None,
    })
}

fn figure_char(v: u8) -> Option<char> {
    Some(match v {
        1 => '3', 3 => '-', 6 => '8', 7 => '7', 9 => '$', 10 => '4', 11 => '\'',
        12 => ',', 13 => '!', 14 => ':', 15 => '(', 16 => '5', 17 => '"', 18 => ')',
        19 => '2', 20 => '#', 21 => '6', 22 => '0', 23 => '1', 24 => '9', 25 => '?',
        26 => '&', 28 => '.', 29 => '/', 30 => ';',
        _ => return None, // 5 (S) = BELL: ignored
    })
}

fn letter_code(c: char) -> Option<u8> {
    ('A'..='Z').contains(&c).then(|| ()).and_then(|_| (0u8..32).find(|&v| letter_char(v) == Some(c)))
}
fn figure_code(c: char) -> Option<u8> {
    (0u8..32).find(|&v| figure_char(v) == Some(c))
}

// ─────────────────────────────── transmit ───────────────────────────────

struct TxBit {
    mark: bool,
    char_done: Option<usize>,
}

/// Streaming RTTY transmitter. Feed text with [`RttyTx::push_text`]; pull audio
/// with [`RttyTx::next_block`]. Idle output is a steady mark tone.
pub struct RttyTx {
    rate: f64,
    mark_hz: f64,
    space_hz: f64,
    spb: usize, // samples per bit
    ph: f64,
    q: VecDeque<TxBit>,
    figs: bool,
    total_chars: usize,
    sent_chars: usize,
    // current bit render
    cur_left: usize,
    cur_mark: bool,
    cur_done: Option<usize>,
}

impl RttyTx {
    pub fn new(rate: f64, center_hz: f64, baud: f64, shift_hz: f64) -> Self {
        RttyTx {
            rate,
            mark_hz: center_hz + shift_hz / 2.0,
            space_hz: center_hz - shift_hz / 2.0,
            spb: (rate / baud).round().max(1.0) as usize,
            ph: 0.0,
            q: VecDeque::new(),
            figs: false,
            total_chars: 0,
            sent_chars: 0,
            cur_left: 0,
            cur_mark: true,
            cur_done: None,
        }
    }

    pub fn set_tuning(&mut self, center_hz: f64, shift_hz: f64) {
        self.mark_hz = center_hz + shift_hz / 2.0;
        self.space_hz = center_hz - shift_hz / 2.0;
    }

    fn frame(&mut self, value: u8, char_done: Option<usize>) {
        // start (space), 5 data LSB-first (mark=1), 1.5 stop → 2 mark bits.
        self.q.push_back(TxBit { mark: false, char_done: None });
        for i in 0..5 {
            self.q.push_back(TxBit { mark: (value >> i) & 1 == 1, char_done: None });
        }
        self.q.push_back(TxBit { mark: true, char_done: None });
        self.q.push_back(TxBit { mark: true, char_done });
    }

    /// Queue text (uppercased; unknown characters dropped). Emits LTRS/FIGS
    /// shift codes as needed.
    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let ci = self.total_chars;
            self.total_chars += 1;
            let c = ch.to_ascii_uppercase();
            match c {
                ' ' => self.frame(BAUDOT_SPACE, Some(ci)),
                '\n' => self.frame(BAUDOT_LF, Some(ci)),
                '\r' => self.frame(BAUDOT_CR, Some(ci)),
                _ => {
                    if let Some(v) = letter_code(c) {
                        if self.figs {
                            self.figs = false;
                            self.frame(LTRS, None);
                        }
                        self.frame(v, Some(ci));
                    } else if let Some(v) = figure_code(c) {
                        if !self.figs {
                            self.figs = true;
                            self.frame(FIGS, None);
                        }
                        self.frame(v, Some(ci));
                    } else {
                        // Unrepresentable: still count it as "sent" so the UI
                        // cursor doesn't stall.
                        self.sent_chars = self.sent_chars.max(ci + 1);
                    }
                }
            }
        }
    }

    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }
    pub fn total_chars(&self) -> usize {
        self.total_chars
    }
    pub fn drained(&self) -> bool {
        self.q.is_empty() && self.cur_left == 0
    }
    pub fn clear(&mut self) {
        self.q.clear();
        self.cur_left = 0;
        self.figs = false;
        self.total_chars = 0;
        self.sent_chars = 0;
    }

    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        for s in out.iter_mut() {
            if self.cur_left == 0 {
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                match self.q.pop_front() {
                    Some(b) => {
                        self.cur_mark = b.mark;
                        self.cur_done = b.char_done;
                    }
                    None => {
                        self.cur_mark = true; // idle mark
                        self.cur_done = None;
                    }
                }
                self.cur_left = self.spb;
            }
            let f = if self.cur_mark { self.mark_hz } else { self.space_hz };
            self.ph += TAU * f / self.rate;
            if self.ph > TAU {
                self.ph -= TAU;
            }
            *s = (self.ph.sin() as f32) * 0.5;
            self.cur_left -= 1;
        }
        out.len()
    }
}

// ─────────────────────────────── receive ───────────────────────────────

#[derive(PartialEq)]
enum RxState {
    Idle,
    Data,
}

/// Streaming RTTY receiver (FM-discriminator bit slicer + UART framing).
pub struct RttyRx {
    rate: f64,
    ph: f32,
    ph_inc: f32,
    lpf: ComplexFir,
    prev: Complex32,
    disc: f32,
    bit_len: f32,
    state: RxState,
    clk: f32,
    nbits: u8,
    value: u8,
    last_mark: bool,
    figs: bool,
    mag: f32,
}

impl RttyRx {
    pub fn new(rate: f64, center_hz: f64, baud: f64, shift_hz: f64) -> Self {
        // LPF wide enough to pass both tones (±shift/2) with margin.
        let bw = (shift_hz / 2.0 + 60.0).max(120.0);
        let taps = bandpass_taps(129, -bw, bw, rate);
        RttyRx {
            rate,
            ph: 0.0,
            ph_inc: (TAU * center_hz / rate) as f32,
            lpf: ComplexFir::new(taps),
            prev: Complex32::new(0.0, 0.0),
            disc: 0.0,
            bit_len: (rate / baud) as f32,
            state: RxState::Idle,
            clk: 0.0,
            nbits: 0,
            value: 0,
            last_mark: true,
            figs: false,
            mag: 0.0,
        }
    }

    pub fn set_tuning(&mut self, center_hz: f64, baud: f64, shift_hz: f64) {
        self.ph_inc = (TAU * center_hz / self.rate) as f32;
        self.bit_len = (self.rate / baud) as f32;
        let bw = (shift_hz / 2.0 + 60.0).max(120.0);
        self.lpf.set_taps(bandpass_taps(129, -bw, bw, self.rate));
        self.lpf.reset();
        self.state = RxState::Idle;
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            let z = Complex32::new(a * self.ph.cos(), -a * self.ph.sin());
            self.ph += self.ph_inc;
            if self.ph > std::f32::consts::TAU {
                self.ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        for z in bb {
            let d = z * self.prev.conj();
            self.prev = z;
            // Instantaneous-frequency metric (positive = mark, the higher tone).
            self.disc += 0.15 * (d.im - self.disc);
            self.mag += 0.02 * (z.norm() - self.mag);
            let mark = self.disc > 0.0;

            match self.state {
                RxState::Idle => {
                    // Falling edge mark→space = start bit.
                    if self.last_mark && !mark {
                        self.state = RxState::Data;
                        self.clk = self.bit_len * 1.5; // to center of first data bit
                        self.nbits = 0;
                        self.value = 0;
                    }
                }
                RxState::Data => {
                    self.clk -= 1.0;
                    if self.clk <= 0.0 {
                        if mark {
                            self.value |= 1 << self.nbits;
                        }
                        self.nbits += 1;
                        if self.nbits >= 5 {
                            self.decode(self.value, &mut out);
                            self.state = RxState::Idle;
                        } else {
                            self.clk += self.bit_len;
                        }
                    }
                }
            }
            self.last_mark = mark;
        }
        out
    }

    fn decode(&mut self, value: u8, out: &mut String) {
        match value {
            LTRS => self.figs = false,
            FIGS => self.figs = true,
            BAUDOT_SPACE => out.push(' '),
            BAUDOT_LF => out.push('\n'),
            BAUDOT_CR => {} // ignore CR; LF drives newlines
            0 => {}
            v => {
                let c = if self.figs { figure_char(v) } else { letter_char(v) };
                if let Some(c) = c {
                    out.push(c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baudot_roundtrip() {
        for c in 'A'..='Z' {
            let v = letter_code(c).unwrap();
            assert_eq!(letter_char(v), Some(c));
        }
        for (v, _) in (0u8..32).filter_map(|v| figure_char(v).map(|c| (v, c))) {
            let c = figure_char(v).unwrap();
            assert_eq!(figure_code(c), Some(v));
        }
    }

    #[test]
    fn loopback() {
        let rate = 8000.0;
        let center = 1000.0;
        let (baud, shift) = (45.45, 170.0);
        let mut tx = RttyTx::new(rate, center, baud, shift);
        let mut audio = Vec::new();
        // Idle mark preamble.
        let mut pre = [0.0f32; 2000];
        tx.next_block(&mut pre);
        audio.extend_from_slice(&pre);

        let msg = "CQ CQ DE AB1CD K";
        tx.push_text(msg);
        while !tx.drained() {
            let mut b = [0.0f32; 2000];
            tx.next_block(&mut b);
            audio.extend_from_slice(&b);
        }
        let mut tail = [0.0f32; 2000];
        tx.next_block(&mut tail);
        audio.extend_from_slice(&tail);

        let mut rx = RttyRx::new(rate, center, baud, shift);
        let mut decoded = String::new();
        for chunk in audio.chunks(400) {
            decoded.push_str(&rx.process(chunk));
        }
        assert!(decoded.contains(msg), "decoded {decoded:?} did not contain {msg:?}");
        assert_eq!(tx.sent_chars(), tx.total_chars());
    }
}
