//! BPSK31 (PSK31) modem: streaming decode + encode over an audio carrier.
//!
//! PSK31 is differential BPSK at 31.25 baud with a raised-cosine amplitude
//! envelope and Varicode text coding (each character a run of 1s/0s containing
//! no `00`; characters separated by `00`). A `0` bit is a phase reversal, a `1`
//! bit is no reversal, so the idle stream (continuous reversals) is all-zero.
//!
//! RX: down-convert the audio to complex baseband at the carrier, low-pass and
//! decimate to a few samples/symbol, recover symbol timing (Gardner), detect
//! bits differentially, and Varicode-decode. TX: Varicode-encode text into a
//! bit queue and stream cosine-blended BPSK symbols on the carrier, reporting
//! how many source characters have been fully transmitted (for the UI).

use std::collections::HashMap;
use std::collections::VecDeque;
use std::f32::consts::PI;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};

const BAUD: f64 = 31.25;
/// Decimated baseband rate: an integer number of samples per symbol.
const SPS: usize = 16;
const DEC_RATE: f64 = BAUD * SPS as f64; // 500 Hz

/// Varicode for a printable ASCII byte, or `""` if unmapped. Bits are sent
/// most-significant first; a `00` separator is appended between characters.
fn varicode(b: u8) -> &'static str {
    match b {
        b' ' => "1",
        b'!' => "111111111",
        b'"' => "101011111",
        b'#' => "111110101",
        b'$' => "111011011",
        b'%' => "1011010101",
        b'&' => "1010111011",
        b'\'' => "101111111",
        b'(' => "11111011",
        b')' => "11110111",
        b'*' => "101101111",
        b'+' => "111011111",
        b',' => "1110101",
        b'-' => "110101",
        b'.' => "1010111",
        b'/' => "110101111",
        b'0' => "10110111",
        b'1' => "10111101",
        b'2' => "11101101",
        b'3' => "11111111",
        b'4' => "101110111",
        b'5' => "101011011",
        b'6' => "101101011",
        b'7' => "110101101",
        b'8' => "110101011",
        b'9' => "110110111",
        b':' => "11110101",
        b';' => "110111101",
        b'<' => "111101101",
        b'=' => "1010101",
        b'>' => "111010111",
        b'?' => "1010101111",
        b'@' => "1010111101",
        b'A' => "1111101",
        b'B' => "11101011",
        b'C' => "10101101",
        b'D' => "10110101",
        b'E' => "1110111",
        b'F' => "11011011",
        b'G' => "11111101",
        b'H' => "101010101",
        b'I' => "1111111",
        b'J' => "111111101",
        b'K' => "101111101",
        b'L' => "11010111",
        b'M' => "10111011",
        b'N' => "11011101",
        b'O' => "10101011",
        b'P' => "11010101",
        b'Q' => "111011101",
        b'R' => "10101111",
        b'S' => "1101111",
        b'T' => "1101101",
        b'U' => "101010111",
        b'V' => "110110101",
        b'W' => "101011101",
        b'X' => "101110101",
        b'Y' => "101111011",
        b'Z' => "1010101101",
        b'[' => "111110111",
        b'\\' => "111101111",
        b']' => "111111011",
        b'^' => "1010111111",
        b'_' => "101101101",
        b'`' => "1011011111",
        b'a' => "1011",
        b'b' => "1011111",
        b'c' => "101111",
        b'd' => "101101",
        b'e' => "11",
        b'f' => "111101",
        b'g' => "1011011",
        b'h' => "101011",
        b'i' => "1101",
        b'j' => "111101011",
        b'k' => "10111111",
        b'l' => "11011",
        b'm' => "111011",
        b'n' => "1111",
        b'o' => "111",
        b'p' => "111111",
        b'q' => "110111111",
        b'r' => "10101",
        b's' => "10111",
        b't' => "101",
        b'u' => "110111",
        b'v' => "1111011",
        b'w' => "1101011",
        b'x' => "11011111",
        b'y' => "1011101",
        b'z' => "111010101",
        b'{' => "1010110111",
        b'|' => "110111011",
        b'}' => "1010110101",
        b'~' => "1011010111",
        b'\n' => "11101",
        b'\r' => "11111",
        _ => "",
    }
}

/// Reverse Varicode map (bit string → character), for decoding.
fn revmap() -> HashMap<&'static str, char> {
    let mut m = HashMap::new();
    for b in 0u8..=127 {
        let code = varicode(b);
        if !code.is_empty() {
            m.entry(code).or_insert(b as char);
        }
    }
    m
}

// ─────────────────────────────── transmit ───────────────────────────────

struct TxBit {
    /// `false` = phase reversal (Varicode 0), `true` = no reversal (Varicode 1).
    steady: bool,
    /// Set on the final separator bit of a source character (its 0-based index).
    char_done: Option<usize>,
}

/// Streaming BPSK31 transmitter. Feed text with [`PskTx::push_text`]; pull audio
/// with [`PskTx::next_block`]. When the queue is empty it emits idle reversals.
pub struct PskTx {
    sps: usize,
    cph: f32,
    cph_inc: f32,
    amp: f32,
    last_sym: f32,
    q: VecDeque<TxBit>,
    total_chars: usize,
    sent_chars: usize,
    // current symbol being rendered
    cur: Vec<f32>,
    cur_pos: usize,
    cur_done: Option<usize>,
}

impl PskTx {
    pub fn new(rate: f64, carrier_hz: f64) -> Self {
        PskTx {
            sps: (rate / BAUD).round() as usize,
            cph: 0.0,
            cph_inc: (std::f64::consts::TAU * carrier_hz / rate) as f32,
            amp: 0.5,
            last_sym: 1.0,
            q: VecDeque::new(),
            total_chars: 0,
            sent_chars: 0,
            cur: Vec::new(),
            cur_pos: 0,
            cur_done: None,
        }
    }

    pub fn set_carrier(&mut self, carrier_hz: f64, rate: f64) {
        self.cph_inc = (std::f64::consts::TAU * carrier_hz / rate) as f32;
    }

    /// Queue text for transmission (appends; does not reset progress counters).
    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let ci = self.total_chars;
            let byte = if ch.is_ascii() { ch as u8 } else { b'?' };
            let code = varicode(byte);
            let code = if code.is_empty() { varicode(b'?') } else { code };
            for c in code.chars() {
                self.q.push_back(TxBit { steady: c == '1', char_done: None });
            }
            // Two-bit separator; tag the last one as the character boundary.
            self.q.push_back(TxBit { steady: false, char_done: None });
            self.q.push_back(TxBit { steady: false, char_done: Some(ci) });
            self.total_chars += 1;
        }
    }

    /// Characters fully transmitted so far (for the green "sent" indicator).
    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }
    pub fn total_chars(&self) -> usize {
        self.total_chars
    }
    /// True once every queued character has been sent and nothing is pending.
    pub fn drained(&self) -> bool {
        self.q.is_empty() && self.cur_pos >= self.cur.len()
    }

    /// Reset all queued text + counters (e.g. leaving TX).
    pub fn clear(&mut self) {
        self.q.clear();
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = None;
        self.total_chars = 0;
        self.sent_chars = 0;
    }

    fn render_symbol(&mut self, steady: bool, done: Option<usize>) {
        let s_new = if steady { self.last_sym } else { -self.last_sym };
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = done;
        for i in 0..self.sps {
            let blend = 0.5 * (1.0 + (PI * i as f32 / self.sps as f32).cos()); // 1→0
            let a = self.last_sym * blend + s_new * (1.0 - blend);
            let s = a * self.cph.cos() * self.amp;
            self.cph += self.cph_inc;
            if self.cph > 2.0 * PI {
                self.cph -= 2.0 * PI;
            }
            self.cur.push(s);
        }
        self.last_sym = s_new;
    }

    /// Fill `out` with the next audio samples. Emits idle reversals when the
    /// queue is empty. Returns the number of samples written (== out.len()).
    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.cur_pos >= self.cur.len() {
                // Symbol finished: mark its character done, then start the next.
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                match self.q.pop_front() {
                    Some(b) => self.render_symbol(b.steady, b.char_done),
                    None => self.render_symbol(false, None), // idle reversal
                }
            }
            out[n] = self.cur[self.cur_pos];
            self.cur_pos += 1;
            n += 1;
        }
        n
    }
}

// ─────────────────────────────── receive ───────────────────────────────

/// Varicode bit-stream decoder: accumulates bits and yields a character on each
/// `00` separator. Shared by the audio [`PskRx`] and the wideband skimmer.
pub struct VaricodeRx {
    rx_word: String,
    map: HashMap<&'static str, char>,
}

impl VaricodeRx {
    pub fn new() -> Self {
        VaricodeRx { rx_word: String::new(), map: revmap() }
    }

    /// Feed one differential bit; returns the decoded character at a `00`
    /// boundary (or `None` while a code is still accumulating).
    pub fn push_bit(&mut self, bit: u8) -> Option<char> {
        self.rx_word.push(if bit == 1 { '1' } else { '0' });
        if self.rx_word.ends_with("00") {
            let code = &self.rx_word[..self.rx_word.len() - 2];
            let c = if code.is_empty() { None } else { self.map.get(code).copied() };
            self.rx_word.clear();
            c
        } else {
            if self.rx_word.len() > 20 {
                self.rx_word.clear(); // no terminator in a plausible length — resync
            }
            None
        }
    }
}

impl Default for VaricodeRx {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming BPSK31 symbol decoder over *complex baseband* at `sps` samples per
/// symbol: a Costas loop removes any residual carrier offset, a Gardner loop
/// recovers symbol timing, differential detection yields bits, and Varicode
/// yields text. Shared by the audio [`PskRx`] (well-oversampled) and the
/// skimmer (a coarse ~6 sps filterbank tap, where the Costas loop is essential).
pub struct BpskCore {
    sps: f32,
    ph: f32,
    freq: f32,
    alpha: f32,
    beta: f32,
    acc: f32,
    hist: VecDeque<Complex32>,
    prev_sym: Complex32,
    vrx: VaricodeRx,
    mag: f32,
}

impl BpskCore {
    pub fn new(sps: f32) -> Self {
        // Costas loop gains for a modest tracking bandwidth (~1% of symbol rate).
        let bw = 0.008f32;
        let mut hist = VecDeque::new();
        hist.extend(std::iter::repeat(Complex32::new(0.0, 0.0)).take(sps as usize + 2));
        BpskCore {
            sps,
            ph: 0.0,
            freq: 0.0,
            alpha: 2.0 * bw,
            beta: bw * bw,
            acc: 0.0,
            hist,
            prev_sym: Complex32::new(1.0, 0.0),
            vrx: VaricodeRx::new(),
            mag: 0.0,
        }
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    /// Feed one complex baseband sample; push any decoded characters to `out`.
    pub fn push(&mut self, z: Complex32, out: &mut String) {
        // Costas loop: derotate by the tracked phase, then a decision-directed
        // BPSK error nudges phase/frequency to null any residual carrier offset.
        let rot = Complex32::new(self.ph.cos(), -self.ph.sin());
        let y = z * rot;
        let e = (if y.re >= 0.0 { y.im } else { -y.im }).clamp(-1.0, 1.0);
        self.freq = (self.freq + self.beta * e).clamp(-0.4, 0.4);
        self.ph += self.freq + self.alpha * e;
        if self.ph > std::f32::consts::PI {
            self.ph -= std::f32::consts::TAU;
        } else if self.ph < -std::f32::consts::PI {
            self.ph += std::f32::consts::TAU;
        }

        self.hist.push_back(y);
        if self.hist.len() > self.sps as usize + 2 {
            self.hist.pop_front();
        }
        self.acc += 1.0;
        if self.acc >= self.sps {
            self.acc -= self.sps;
            let curr = y;
            // Gardner timing (sample at symbol center + half-symbol prior).
            let midi = self.hist.len().saturating_sub(self.sps as usize / 2 + 1);
            let mid = *self.hist.get(midi).unwrap_or(&curr);
            let et = (mid.conj() * (curr - self.prev_sym)).re;
            self.acc += (-0.02 * et).clamp(-1.0, 1.0);
            // Differential BPSK detection (immune to the Costas ± ambiguity).
            let d = curr * self.prev_sym.conj();
            let bit = if d.re >= 0.0 { 1u8 } else { 0 };
            self.mag += 0.05 * (curr.norm() - self.mag);
            self.prev_sym = curr;
            if let Some(c) = self.vrx.push_bit(bit) {
                out.push(c);
            }
        }
    }
}

/// Streaming BPSK31 receiver over *real audio*. Feed audio with
/// [`PskRx::process`]; it returns any newly decoded text. Down-converts at
/// `carrier_hz`, so retune by [`PskRx::set_carrier`] when the operator moves the
/// tuning line.
pub struct PskRx {
    rate: f64,
    carrier: f64,
    ph: f32,
    ph_inc: f32,
    lpf: ComplexFir,
    decim: usize,
    dcount: usize,
    core: BpskCore,
}

impl PskRx {
    pub fn new(rate: f64, carrier_hz: f64) -> Self {
        let decim = (rate / DEC_RATE).round().max(1.0) as usize;
        // Complex low-pass ~±80 Hz around the carrier (PSK31 is ~±16 Hz wide;
        // extra margin tolerates mistuning).
        let taps = bandpass_taps(129, -80.0, 80.0, rate);
        PskRx {
            rate,
            carrier: carrier_hz,
            ph: 0.0,
            ph_inc: (std::f64::consts::TAU * carrier_hz / rate) as f32,
            lpf: ComplexFir::new(taps),
            decim,
            dcount: 0,
            core: BpskCore::new(SPS as f32),
        }
    }

    pub fn set_carrier(&mut self, carrier_hz: f64) {
        self.carrier = carrier_hz;
        self.ph_inc = (std::f64::consts::TAU * carrier_hz / self.rate) as f32;
        self.lpf.reset();
    }

    /// Rough tuning/quality magnitude (average symbol amplitude).
    pub fn magnitude(&self) -> f32 {
        self.core.magnitude()
    }

    /// Feed a block of real audio (at the constructor's `rate`); returns any
    /// newly decoded characters.
    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            let z = Complex32::new(a * self.ph.cos(), -a * self.ph.sin());
            self.ph += self.ph_inc;
            if self.ph > 2.0 * PI {
                self.ph -= 2.0 * PI;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);
        for z in bb {
            self.dcount += 1;
            if self.dcount < self.decim {
                continue;
            }
            self.dcount = 0;
            self.core.push(z, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varicode_well_formed_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for b in 32u8..=126 {
            let c = varicode(b);
            assert!(!c.is_empty(), "missing varicode for {}", b as char);
            assert!(c.starts_with('1') && c.ends_with('1'), "{}: {c}", b as char);
            assert!(!c.contains("00"), "{}: {c} contains 00", b as char);
            assert!(seen.insert(c), "duplicate varicode {c} for {}", b as char);
        }
    }

    #[test]
    fn loopback() {
        let rate = 8000.0;
        let carrier = 1000.0;
        let mut tx = PskTx::new(rate, carrier);
        // Idle preamble lets the RX acquire symbol timing before the message.
        let mut audio = Vec::new();
        let mut blk = [0.0f32; 4000]; // 0.5 s of idle reversals
        tx.next_block(&mut blk);
        audio.extend_from_slice(&blk);

        let msg = "CQ CQ DE AB1CD K";
        tx.push_text(msg);
        // Bounded on sent-char progress (never on block/symbol alignment).
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 4000 {
            let mut b = [0.0f32; 2000];
            tx.next_block(&mut b);
            audio.extend_from_slice(&b);
            guard += 1;
        }
        let mut tail = [0.0f32; 4000];
        tx.next_block(&mut tail);
        audio.extend_from_slice(&tail);

        let mut rx = PskRx::new(rate, carrier);
        let mut decoded = String::new();
        for chunk in audio.chunks(512) {
            decoded.push_str(&rx.process(chunk));
        }
        assert!(
            decoded.contains(msg),
            "decoded {decoded:?} did not contain {msg:?}"
        );
        // Sent-char progress reached the whole message.
        assert_eq!(tx.sent_chars(), tx.total_chars());
    }
}
