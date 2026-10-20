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
use crate::fec::{ConvCode, ConvEncoder, ViterbiStream};
use crate::fir::{ComplexFir, bandpass_taps};

/// The rate PSK31 is defined at, and the default for every constructor that
/// does not say otherwise.
pub const PSK31_BAUD: f64 = 31.25;
/// Decimated baseband rate: an integer number of samples per symbol.
pub const SPS: usize = 16;

/// The decimated baseband rate for a baud rate: `SPS` samples per symbol, so a
/// decoder always sees the same oversampling whatever the mode's speed.
fn dec_rate(baud: f64) -> f64 {
    baud * SPS as f64
}

/// Varicode for a printable ASCII byte, or `""` if unmapped. Bits are sent
/// most-significant first; a `00` separator is appended between characters.
/// Shared with the THOR modem, which streams the same varicode through its FEC.
pub(crate) fn varicode(b: u8) -> &'static str {
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
    baud: f64,
    sps: usize,
    cph: f32,
    cph_inc: f32,
    amp: f32,
    /// The carrier's phase as a unit phasor: BPSK only ever uses ±1 on the
    /// real axis, QPSK rotates it by the encoder's pair, so one field carries
    /// both.
    last_c: (f32, f32),
    /// Where the symbol in progress started, for the raised-cosine blend.
    prev_c: (f32, f32),
    qpsk: bool,
    enc: ConvEncoder,
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
        Self::new_baud(rate, carrier_hz, PSK31_BAUD)
    }

    /// The same modulator at any of PSK31's speeds: they differ in the symbol
    /// rate and in nothing else.
    pub fn new_baud(rate: f64, carrier_hz: f64, baud: f64) -> Self {
        PskTx {
            baud,
            sps: (rate / baud).round().max(1.0) as usize,
            cph: 0.0,
            cph_inc: (std::f64::consts::TAU * carrier_hz / rate) as f32,
            amp: 0.5,
            last_c: (1.0, 0.0),
            prev_c: (1.0, 0.0),
            qpsk: false,
            enc: ConvEncoder::new(ConvCode::PSK_QPSK),
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

    /// Switch between BPSK and QPSK. The encoder restarts and the carrier
    /// returns to phase zero, so a transmission never straddles the two.
    pub fn set_qpsk(&mut self, on: bool) {
        if self.qpsk == on {
            return;
        }
        self.qpsk = on;
        self.enc.reset();
        self.last_c = (1.0, 0.0);
        self.prev_c = (1.0, 0.0);
    }

    /// Change the symbol rate. The symbol in progress is finished first, so a
    /// speed change never lands mid-symbol.
    pub fn set_baud(&mut self, baud: f64, rate: f64) {
        if (baud - self.baud).abs() < 1e-9 {
            return;
        }
        self.baud = baud;
        self.sps = (rate / baud).round().max(1.0) as usize;
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
        let (mut ci, mut cq) = self.last_c;
        if self.qpsk {
            // A QPSK symbol *is* the encoder's output pair, so one queued bit
            // becomes one phase step of 90 degrees times that pair. The text
            // rate is unchanged from BPSK: two channel bits per symbol at rate
            // 1/2 is one data bit.
            let sym = self.enc.push(steady as u8) & 3;
            for _ in 0..sym {
                let t = ci;
                ci = -cq;
                cq = t;
            }
        } else if !steady {
            ci = -ci;
            cq = -cq;
        }
        let (pi, pq) = self.prev_c;
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = done;
        for i in 0..self.sps {
            // Blend the *phasor*: on BPSK that is the amplitude dip between
            // reversals, and on QPSK it is the same dip with the phase turning.
            let blend = 0.5 * (1.0 + (PI * i as f32 / self.sps as f32).cos()); // 1→0
            let ai = pi * blend + ci * (1.0 - blend);
            let aq = pq * blend + cq * (1.0 - blend);
            let s = (ai * self.cph.cos() - aq * self.cph.sin()) * self.amp;
            self.cph += self.cph_inc;
            if self.cph > 2.0 * PI {
                self.cph -= 2.0 * PI;
            }
            self.cur.push(s);
        }
        self.prev_c = (ci, cq);
        self.last_c = (ci, cq);
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
        // A Varicode always starts with '1'; drop leading/idle zeros so a stray
        // pre-character '0' can't corrupt the next code (e.g. after idle).
        if self.rx_word.is_empty() && bit == 0 {
            return None;
        }
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

/// Second-order loop coefficients for a normalised bandwidth and 0.707 damping.
/// Written out rather than hand-picked because the frequency term is what sets
/// how much carrier offset the Costas loop can pull in, and eyeballing it is how
/// a loop ends up taking half a minute to acquire a few Hz.
pub(crate) fn loop_gains(bw: f32) -> (f32, f32) {
    let damp = 0.707f32;
    let theta = bw / (damp + 0.25 / damp);
    let denom = 1.0 + 2.0 * damp * theta + theta * theta;
    (4.0 * damp * theta / denom, 4.0 * theta * theta / denom)
}

/// Costas/Gardner loop bandwidth as a fraction of the baseband sample rate.
const LOOP_BW: f32 = 0.006;
/// Gardner timing-loop gain, applied to a magnitude-normalised error.
const TIMING_GAIN: f32 = 0.05;

/// Carrier-coherence noise gate.
///
/// Varicode has no framing to check — every bit pattern ending in `00` is some
/// character — so a receiver parked on an empty channel prints a steady stream
/// of plausible-looking letters. What does separate signal from noise is where
/// the differential product lands: locked BPSK puts it on the real axis, while
/// isotropic noise spreads it around the circle and averages |cos| = 2/π ≈ 0.64.
/// Gating on that costs nothing in sensitivity, because by the time coherence
/// falls this far the text is unreadable anyway.
const COHERENCE_SMOOTH: f32 = 1.0 / 32.0;
const COHERENCE_GATE: f32 = 0.72;

/// Streaming BPSK31 symbol decoder over *complex baseband* at `sps` samples per
/// symbol: a matched filter collects the symbol energy, a Costas loop removes
/// any residual carrier offset, a Gardner loop recovers symbol timing,
/// differential detection yields bits, and Varicode yields text. Shared by the
/// audio [`PskRx`] (well-oversampled) and the skimmer (a coarse ~6 sps
/// filterbank tap, where the Costas loop is essential).
///
/// The matched filter is what makes this a weak-signal decoder. Deciding from a
/// single baseband sample per symbol throws away the rest of the symbol's
/// energy and leaves the front-end filter's whole bandwidth of noise in the
/// decision — worth about 6 dB on a mode whose entire appeal is reading signals
/// well below the noise floor.
///
/// Both loop errors are normalised by signal magnitude. Without that the loop
/// bandwidths scale with the audio level, so the same signal 40 dB quieter gets
/// a loop 100× slower and never acquires.
pub struct BpskCore {
    sps: f32,
    ph: f32,
    freq: f32,
    alpha: f32,
    beta: f32,
    acc: f32,
    /// Raised-cosine matched filter over one symbol, and its ring buffer.
    mf_taps: Vec<f32>,
    mf_buf: Vec<Complex32>,
    mf_pos: usize,
    hist: VecDeque<Complex32>,
    prev_sym: Complex32,
    vrx: VaricodeRx,
    mag: f32,
    coherence: f32,
    gate: bool,
    /// QPSK rather than BPSK: two channel bits per symbol, carried by a
    /// rate-1/2 convolutional code, so one *data* bit per symbol.
    qpsk: bool,
    /// The code the QPSK modes carry. Unused in BPSK, where a symbol is a bit.
    fec: ViterbiStream,
}

impl BpskCore {
    pub fn new(sps: f32) -> Self {
        let (alpha, beta) = loop_gains(LOOP_BW);
        let n = (sps.round() as usize).max(3);
        // Hann over one symbol: matched to the raised-cosine envelope the
        // transmitter blends reversals with, and it weights the symbol centre
        // (where the phase is clean) over the edges (where it is in transit).
        let mut taps: Vec<f32> = (0..n)
            .map(|i| 0.5 * (1.0 - (std::f32::consts::TAU * (i + 1) as f32 / (n + 1) as f32).cos()))
            .collect();
        let sum: f32 = taps.iter().sum();
        taps.iter_mut().for_each(|t| *t /= sum);

        let mut hist = VecDeque::new();
        hist.extend(std::iter::repeat(Complex32::new(0.0, 0.0)).take(sps as usize + 2));
        BpskCore {
            sps,
            ph: 0.0,
            freq: 0.0,
            alpha,
            beta,
            acc: 0.0,
            mf_taps: taps,
            mf_buf: vec![Complex32::new(0.0, 0.0); n],
            mf_pos: 0,
            hist,
            prev_sym: Complex32::new(1.0, 0.0),
            vrx: VaricodeRx::new(),
            mag: 0.0,
            coherence: 1.0,
            gate: true,
            qpsk: false,
            fec: ViterbiStream::new(ConvCode::PSK_QPSK),
        }
    }

    /// Switch between BPSK and QPSK.
    ///
    /// The two are the same demodulator with a different slice at the end: QPSK
    /// carries two channel bits per symbol and the convolutional code turns
    /// them into one data bit, which is why the text rate does not double.
    pub fn set_qpsk(&mut self, on: bool) {
        if self.qpsk == on {
            return;
        }
        self.qpsk = on;
        self.fec.reset();
    }

    pub fn qpsk(&self) -> bool {
        self.qpsk
    }


    /// Turn the carrier-coherence noise gate off (the skimmer scores its own
    /// candidates and would rather see everything).
    pub fn set_gate(&mut self, on: bool) {
        self.gate = on;
    }

    /// Smoothed carrier coherence in [0, 1]: 1 is a cleanly locked signal,
    /// about 0.64 is pure noise.
    pub fn coherence(&self) -> f32 {
        self.coherence
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    /// Residual carrier offset the Costas loop is holding, in fractions of the
    /// baseband sample rate.
    pub fn freq_offset(&self) -> f32 {
        self.freq / std::f32::consts::TAU
    }

    fn matched(&mut self, y: Complex32) -> Complex32 {
        self.mf_buf[self.mf_pos] = y;
        self.mf_pos = (self.mf_pos + 1) % self.mf_buf.len();
        let n = self.mf_buf.len();
        let mut acc = Complex32::new(0.0, 0.0);
        for (k, &t) in self.mf_taps.iter().enumerate() {
            acc += self.mf_buf[(self.mf_pos + k) % n] * t;
        }
        acc
    }

    /// Feed one complex baseband sample; push any decoded characters to `out`.
    pub fn push(&mut self, z: Complex32, out: &mut String) {
        // Derotate by the tracked phase first — the matched filter is a
        // lowpass, so the signal has to be centred before it is applied.
        let rot = Complex32::new(self.ph.cos(), -self.ph.sin());
        let y = self.matched(z * rot);

        // Decision-directed BPSK phase error, normalised to radians so the loop
        // bandwidth does not depend on how loud the signal is.
        let n = y.norm();
        let e = if n > 1e-20 { (if y.re >= 0.0 { y.im } else { -y.im }) / n } else { 0.0 };
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
            // Gardner timing (sample at symbol center + half-symbol prior),
            // again normalised so the correction is in symbol fractions.
            let midi = self.hist.len().saturating_sub(self.sps as usize / 2 + 1);
            let mid = *self.hist.get(midi).unwrap_or(&curr);
            let et = (mid.conj() * (curr - self.prev_sym)).re;
            let scale = mid.norm() * curr.norm().max(self.prev_sym.norm());
            if scale > 1e-20 {
                self.acc += (TIMING_GAIN * et / scale).clamp(-0.5, 0.5);
            }
            // Differential detection (immune to the Costas ± ambiguity).
            let d = curr * self.prev_sym.conj();
            let dn = d.norm();
            self.mag += 0.05 * (curr.norm() - self.mag);
            self.prev_sym = curr;
            let bit = if self.qpsk {
                // A QPSK symbol *is* the encoder's output pair, so it carries
                // one data bit rather than two — the FEC is what halves it.
                //
                // The quadrant is taken as `(4 - s) & 3`, which is the mode's
                // convention and not an accident: inverting the received value
                // is how an all-zero message still produces transitions for the
                // receiver's timing to hold on to.
                let s = if dn > 1e-20 {
                    ((d.arg() / std::f32::consts::FRAC_PI_2).round() as i32).rem_euclid(4) as u8
                } else {
                    0
                };
                let bits = (4 - s) & 3;
                // The inversion the mode applies to its second output lives in
                // the *code* (`ConvCode::PSK_QPSK`), not here: it belongs to the
                // encoding, and putting it in both places would cancel it out.
                let a = if bits & 1 != 0 { 1.0 } else { -1.0 };
                let b = if bits & 2 != 0 { 1.0 } else { -1.0 };
                if dn > 1e-20 {
                    // A clean QPSK point sits on a diagonal, so `|re|` alone is
                    // only ever 0.707 — below the coherence gate, which would
                    // have rejected every good symbol.
                    let coh = d.re.abs().max(d.im.abs()) / dn;
                    self.coherence += COHERENCE_SMOOTH * (coh - self.coherence);
                }
                self.fec.push(a, b)
            } else {
                if dn > 1e-20 {
                    self.coherence += COHERENCE_SMOOTH * (d.re.abs() / dn - self.coherence);
                }
                Some(if d.re >= 0.0 { 1u8 } else { 0 })
            };
            if let Some(bit) = bit
                && let Some(c) = self.vrx.push_bit(bit)
                && (!self.gate || self.coherence >= COHERENCE_GATE)
            {
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
    baud: f64,
    ph: f32,
    ph_inc: f32,
    lpf: ComplexFir,
    decim: usize,
    dcount: usize,
    core: BpskCore,
}

impl PskRx {
    pub fn new(rate: f64, carrier_hz: f64) -> Self {
        Self::new_baud(rate, carrier_hz, PSK31_BAUD)
    }

    /// The same receiver at any of PSK31's speeds.
    ///
    /// The passband has to follow the baud rate, not just the decimation: PSK31
    /// is about as wide as its symbol rate, so a filter sized for 31 baud throws
    /// away most of a 250-baud signal while a filter sized for 250 lets the
    /// neighbourhood into a 31-baud one.
    pub fn new_baud(rate: f64, carrier_hz: f64, baud: f64) -> Self {
        let decim = (rate / dec_rate(baud)).round().max(1.0) as usize;
        let half = (2.5 * baud).max(80.0);
        let taps = bandpass_taps(129, -half, half, rate);
        PskRx {
            rate,
            carrier: carrier_hz,
            baud,
            ph: 0.0,
            ph_inc: (std::f64::consts::TAU * carrier_hz / rate) as f32,
            lpf: ComplexFir::new(taps),
            decim,
            dcount: 0,
            // The core's loop is in samples per symbol at the decimated rate,
            // which is only exactly `SPS` when the decimation landed exactly.
            core: BpskCore::new((rate / decim as f64 / baud) as f32),
        }
    }

    pub fn set_carrier(&mut self, carrier_hz: f64) {
        self.carrier = carrier_hz;
        self.ph_inc = (std::f64::consts::TAU * carrier_hz / self.rate) as f32;
        self.lpf.reset();
    }

    /// Change the symbol rate: the decimation, the passband and the timing loop
    /// are all derived from it, so all three are rebuilt and the partial symbol
    /// is dropped.
    pub fn set_baud(&mut self, baud: f64) {
        if (baud - self.baud).abs() < 1e-9 {
            return;
        }
        self.baud = baud;
        self.decim = (self.rate / dec_rate(baud)).round().max(1.0) as usize;
        let half = (2.5 * baud).max(80.0);
        self.lpf = ComplexFir::new(bandpass_taps(129, -half, half, self.rate));
        self.core = BpskCore::new((self.rate / self.decim as f64 / baud) as f32);
        self.dcount = 0;
    }

    pub fn baud(&self) -> f64 {
        self.baud
    }

    /// Switch between BPSK and QPSK. The slicing is the core's, so this is a
    /// passthrough — the two modes share every stage above it.
    pub fn set_qpsk(&mut self, on: bool) {
        self.core.set_qpsk(on);
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
        assert!(decoded.contains(msg), "decoded {decoded:?} did not contain {msg:?}");
        // Sent-char progress reached the whole message.
        assert_eq!(tx.sent_chars(), tx.total_chars());
    }

    /// The four speeds are one modem, so each must read its own transmission.
    ///
    /// This is the guard for `psk_speed`. The rate used to be a compile-time
    /// constant, and that failure was silent: a decoder tuned to 31 baud hears
    /// nothing at all from a 250-baud signal and reports nothing wrong, so the
    /// fault can only be caught by transmitting at the rate and reading it back.
    #[test]
    fn every_speed_reads_back() {
        for baud in [31.25f64, 62.5, 125.0, 250.0] {
            let rate = 8000.0;
            let carrier = 1000.0;
            let mut tx = PskTx::new_baud(rate, carrier, baud);
            let mut audio = Vec::new();
            let mut blk = [0.0f32; 4000]; // idle reversals, to acquire timing
            tx.next_block(&mut blk);
            audio.extend_from_slice(&blk);

            let msg = "CQ CQ DE AB1CD K";
            tx.push_text(msg);
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

            let mut rx = PskRx::new_baud(rate, carrier, baud);
            let mut decoded = String::new();
            for chunk in audio.chunks(512) {
                decoded.push_str(&rx.process(chunk));
            }
            assert!(
                decoded.contains(msg),
                "{baud} baud decoded {decoded:?}, which does not contain {msg:?}"
            );
        }
    }

    /// QPSK against a real recording, because a loopback can only prove the two
    /// halves agree with each other.
    ///
    /// Point `SDROXIDE_QPSK_SAMPLE` at a mono WAV and, if it is not 31.25 baud
    /// or not centred on 1000 Hz, say so with `SDROXIDE_QPSK_BAUD` and
    /// `SDROXIDE_QPSK_HZ`. MultiPSK's `QPSK31-1000_Hz.WAV` works unmodified.
    ///
    /// The assertion is deliberately weak — text rather than a known sentence.
    /// The sample's content is somebody else's recording and pinning a word
    /// from it would make the test a fixture of that file rather than of the
    /// decoder; what matters here is that a real symbol stream comes back as
    /// readable text at all.
    #[test]
    #[ignore = "needs a recording: SDROXIDE_QPSK_SAMPLE=/path/file.wav"]
    fn an_off_air_qpsk_recording_decodes() {
        let Ok(path) = std::env::var("SDROXIDE_QPSK_SAMPLE") else {
            return;
        };
        let baud: f64 = std::env::var("SDROXIDE_QPSK_BAUD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(31.25);
        let hz: f64 = std::env::var("SDROXIDE_QPSK_HZ")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000.0);
        let mut r = hound::WavReader::open(&path).expect("open the recording");
        let spec = r.spec();
        let scale = (1i64 << (spec.bits_per_sample.max(1) - 1)) as f32;
        let mut audio: Vec<f32> = if spec.bits_per_sample <= 8 {
            r.samples::<i8>().map(|s| s.unwrap() as f32 / scale).collect()
        } else {
            r.samples::<i16>().map(|s| s.unwrap() as f32 / scale).collect()
        };
        if spec.channels > 1 {
            audio = audio.chunks(spec.channels as usize).map(|c| c[0]).collect();
        }
        let mut rx = PskRx::new_baud(spec.sample_rate as f64, hz, baud);
        rx.set_qpsk(true);
        let mut text = String::new();
        for chunk in audio.chunks(512) {
            text.push_str(&rx.process(chunk));
        }
        println!("first 240 characters: {:?}", text.chars().take(240).collect::<String>());
        let letters = text.chars().filter(|c| c.is_ascii_alphabetic()).count();
        let spaces = text.chars().filter(|c| *c == ' ').count();
        assert!(
            letters > 40 && spaces > 5,
            "no text came out: {letters} letters, {spaces} spaces from {} characters",
            text.len()
        );
    }

}
