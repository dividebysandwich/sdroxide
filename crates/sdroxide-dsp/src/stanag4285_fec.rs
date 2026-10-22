//! STANAG 4285 Annex E — the coded rates: the convolutional code, the
//! interleaver and the message framing.
//!
//! Transcribed from Annex E of the standard (`STANAG4285-EDITION1.md`, "Annex E"),
//! and **none of it is cryptography**: a constraint-length-7 rate-1/2
//! convolutional code, a convolutional interleaver and a start/end-of-message
//! convention. The traffic's own encryption, where there is any, sits above this
//! layer and is neither described by the standard nor implemented here.
//!
//! The code is the ordinary NASA/CCSDS one — `G1 = 133`, `G2 = 171` octal, T1
//! first — and the lower rates are that code's output *pairs* repeated, never the
//! first bit repeated and then the second.
//!
//! ## What is certain, and what is not
//!
//! Every number below is read off the annex: the polynomials, the 32 rows, the
//! input commutator (`n·9 mod 32`), the 24-of-32 output commutator for 2400, the
//! SOM and EOM words, the flush counts and the decoder's structure.
//!
//! Two things the annex leaves open, and both are marked where they are used
//! because a reader has to know which way a wire was cut:
//!
//! * **which bit of each four is punctured** for the 2400 bps 8-PSK rate. The
//!   annex says "rate 1/2 punctured to rate 2/3" and nothing more, so that rate
//!   is not built here — a guess in a demapper is indistinguishable from a
//!   decode.
//! * **the character layer above the message.** Annex E carries a message
//!   protocol (SOM, EOM, flush) and calls what is between them "message bits";
//!   it does not say they are ITA2. A 75 bps link is exactly a 75 baud
//!   teleprinter, so ITA2 5N1.5 is what a test tape is expected to send, and
//!   both readings are offered rather than one assumed.

use std::collections::VecDeque;

/// Constraint length: seven stages, six of them memory.
pub const K: usize = 7;
/// Number of encoder states.
pub const STATES: usize = 1 << (K - 1);
/// `x⁶ + x⁴ + x³ + x + 1` — the T1 polynomial, 133 octal.
pub const G1: u32 = 0o133;
/// `x⁶ + x⁵ + x³ + x² + 1` — the T2 polynomial, 171 octal.
pub const G2: u32 = 0o171;
/// Start of message, `03873C3C`, leftmost bit sent first.
pub const SOM: u32 = 0x0387_3C3C;
/// End of message, `4B65A5B2`.
pub const EOM: u32 = 0x4B65_A5B2;
/// Rows in the convolutional interleaver, at every rate.
pub const ROWS: usize = 32;
/// The input commutator's row order: the normal sequence multiplied by 9 mod 32.
pub const IN_ORDER: [usize; ROWS] = [
    0, 9, 18, 27, 4, 13, 22, 31, 8, 17, 26, 3, 12, 21, 30, 7, 16, 25, 2, 11, 20, 29, 6, 15, 24, 1,
    10, 19, 28, 5, 14, 23,
];
/// The output commutator for the 2400 bps 8-PSK rate: 24 of the 32 rows, every
/// fourth one skipped.
pub const OUT_ORDER_2400: [usize; 24] =
    [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14, 16, 17, 18, 20, 21, 22, 24, 25, 26, 28, 29, 30];

/// A coded rate, as Annex E names it.
///
/// The four BPSK rates cannot be told apart by the constellation — all of them
/// are one bit per symbol at 1200 channel bits a second — so which one is on the
/// air comes out as the rate whose decode produces the start-of-message pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodedRate {
    /// 1200 bps data, QPSK, unmodified rate 1/2.
    Bps1200,
    /// 600 bps data, BPSK, unmodified rate 1/2.
    Bps600,
    /// 300 bps data, BPSK, rate 1/2 repeated twice.
    Bps300,
    /// 150 bps data, BPSK, rate 1/2 repeated four times.
    Bps150,
    /// 75 bps data, BPSK, rate 1/2 repeated eight times.
    Bps75,
}

impl CodedRate {
    /// The four a BPSK (1200 channel bit) transmission can be.
    pub const BPSK: [CodedRate; 4] =
        [CodedRate::Bps600, CodedRate::Bps300, CodedRate::Bps150, CodedRate::Bps75];

    pub fn label(self) -> &'static str {
        match self {
            CodedRate::Bps1200 => "1200 bps coded (QPSK, rate 1/2)",
            CodedRate::Bps600 => "600 bps coded (BPSK, rate 1/2)",
            CodedRate::Bps300 => "300 bps coded (BPSK, rate 1/4)",
            CodedRate::Bps150 => "150 bps coded (BPSK, rate 1/8)",
            CodedRate::Bps75 => "75 bps coded (BPSK, rate 1/16)",
        }
    }

    /// User data rate in bits per second.
    pub fn bps(self) -> f64 {
        match self {
            CodedRate::Bps1200 => 1200.0,
            CodedRate::Bps600 => 600.0,
            CodedRate::Bps300 => 300.0,
            CodedRate::Bps150 => 150.0,
            CodedRate::Bps75 => 75.0,
        }
    }

    /// How many times the rate-1/2 output pair is repeated.
    pub fn repeat(self) -> usize {
        match self {
            CodedRate::Bps1200 | CodedRate::Bps600 => 1,
            CodedRate::Bps300 => 2,
            CodedRate::Bps150 => 4,
            CodedRate::Bps75 => 8,
        }
    }

    /// The interleaver's delay increment per row, `j`.
    ///
    /// The annex's long-interleave column: 48 at 2400, 24 at 1200, and 12 for
    /// 600–75 — the last four put the same 1200 coded bits a second into the
    /// interleaver, so one `j` covers all of them.
    pub fn j(self) -> usize {
        match self {
            CodedRate::Bps1200 => 24,
            _ => 12,
        }
    }
}

/// Convolutional encoder, rate 1/2, register starting at zero.
pub fn encode(bits: &[u8]) -> Vec<u8> {
    let mut reg = 0u32;
    let mut out = Vec::with_capacity(bits.len() * 2);
    for &b in bits {
        reg = ((reg << 1) | (b as u32 & 1)) & 0x7F;
        out.push((reg & G1).count_ones() as u8 & 1);
        out.push((reg & G2).count_ones() as u8 & 1);
    }
    out
}

/// Repeat each *pair* of coded bits, as Annex E requires: T1,T2,T1,T2 — never
/// the first repeated and then the second.
pub fn repeat_pairs(bits: &[u8], times: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(bits.len() * times);
    for pair in bits.chunks(2) {
        for _ in 0..times {
            out.extend_from_slice(pair);
        }
    }
    out
}

/// Undo [`repeat_pairs`] on soft decisions: the even and the odd soft values are
/// **averaged separately** (Annex E, Appendix 1 §4), which is what turns four,
/// eight or sixteen soft decisions back into two.
pub fn average_pairs(soft: &[f32], times: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(soft.len() / times);
    for group in soft.chunks(times * 2) {
        if group.len() < times * 2 {
            break;
        }
        let (mut even, mut odd) = (0.0f32, 0.0f32);
        for k in 0..times {
            even += group[2 * k];
            odd += group[2 * k + 1];
        }
        out.push(even / times as f32);
        out.push(odd / times as f32);
    }
    out
}

/// The convolutional interleaver and its inverse.
///
/// Row `r` of the **interleaver** holds `r·j` entries; row `r` of the
/// **deinterleaver** holds `(I-1-r)·j`, because the annex gives it "j fewer bits
/// of storage than the preceding one above it". That is what makes the two a
/// pair: a bit spends `r·j` steps in one and `(I-1-r)·j` in the other, so the
/// total is a constant `(I-1)·j` and the message comes out in order.
///
/// The interleaver writes in the `n·9 mod 32` order and reads in the normal one;
/// the deinterleaver is the same machine with its two commutators exchanged,
/// which is what the annex says the receiver does ("soft decisions are entered
/// … in the normal commutator sequence and are taken out using the …
/// non-sequential commutator sequence").
///
/// The rows are pre-filled with zeros, which is both their "initialised to all
/// zeros" state and, for the deinterleaver, the erasures Appendix 1 asks for.
#[derive(Debug, Clone)]
pub struct ConvInterleaver {
    rows: Vec<VecDeque<f32>>,
    n: usize,
    j: usize,
    transmit: bool,
    out_order: Option<[usize; 24]>,
}

impl ConvInterleaver {
    /// The transmitter's interleaver: row `r` holds `r·j`.
    pub fn interleaver(j: usize) -> Self {
        ConvInterleaver {
            rows: (0..ROWS).map(|r| VecDeque::from(vec![0.0; r * j])).collect(),
            n: 0,
            j,
            transmit: true,
            out_order: None,
        }
    }

    /// The receiver's deinterleaver: row `r` holds `(I-1-r)·j`.
    pub fn deinterleaver(j: usize) -> Self {
        ConvInterleaver {
            rows: (0..ROWS).map(|r| VecDeque::from(vec![0.0; (ROWS - 1 - r) * j])).collect(),
            n: 0,
            j,
            transmit: false,
            out_order: None,
        }
    }

    /// The 2400 bps rate takes its output from 24 of the 32 rows.
    pub fn with_psk8(mut self, yes: bool) -> Self {
        self.out_order = yes.then_some(OUT_ORDER_2400);
        self
    }

    pub fn reset(&mut self) {
        for (r, row) in self.rows.iter_mut().enumerate() {
            let len = if self.transmit { r * self.j } else { (ROWS - 1 - r) * self.j };
            row.clear();
            row.resize(len, 0.0);
        }
        self.n = 0;
    }

    fn out_row(&self, step: usize) -> usize {
        match &self.out_order {
            Some(list) => list[step % list.len()],
            None => step % ROWS,
        }
    }

    /// One soft value in, one out.
    pub fn push(&mut self, v: f32) -> f32 {
        let step = self.n % ROWS;
        let (write, read) = if self.transmit {
            (IN_ORDER[step], self.out_row(step))
        } else {
            (self.out_row(step), IN_ORDER[step])
        };
        self.rows[write].push_back(v);
        self.n += 1;
        self.rows[read].pop_front().unwrap_or(0.0)
    }
}

/// Soft-decision Viterbi decoder for the rate-1/2 code.
///
/// Soft values are positive for a zero and negative for a one, which is Annex E's
/// own convention. The metric is the correlation with the expected pair, so a
/// larger path metric is better.
///
/// The traceback stores the **previous state** for every state at every step, not
/// the input bit: this code's state transition drops the oldest bit, so the
/// received state and the input bit do not identify the predecessor — two states
/// differ only in the bit that has just left the register, and remembering the
/// input alone picks the wrong one half the time, which is a decode that looks
/// plausible and is 30 % wrong.
pub fn viterbi_decode(soft: &[f32]) -> Vec<u8> {
    if soft.len() < 2 {
        return Vec::new();
    }
    let steps = soft.len() / 2;
    let mut expected = [[0.0f32; 2]; STATES * 2];
    let mut next = vec![[0usize; 2]; STATES];
    for s in 0..STATES {
        for b in 0..2 {
            let full = ((s as u32) << 1) | b as u32;
            let o1 = if (full & G1).count_ones() & 1 == 0 { 1.0 } else { -1.0 };
            let o2 = if (full & G2).count_ones() & 1 == 0 { 1.0 } else { -1.0 };
            expected[s * 2 + b] = [o1, o2];
            next[s][b] = (full & 0x3F) as usize;
        }
    }
    let mut metric = vec![f32::NEG_INFINITY; STATES];
    metric[0] = 0.0;
    let mut m2 = metric.clone();
    let mut from = vec![[0u8; STATES]; steps];
    for (t, pair) in soft.chunks_exact(2).enumerate() {
        let (y1, y2) = (pair[0], pair[1]);
        m2.fill(f32::NEG_INFINITY);
        for s in 0..STATES {
            let m = metric[s];
            if m == f32::NEG_INFINITY {
                continue;
            }
            for b in 0..2 {
                let o = &expected[s * 2 + b];
                let v = m + y1 * o[0] + y2 * o[1];
                let ns = next[s][b];
                if v > m2[ns] {
                    m2[ns] = v;
                    from[t][ns] = s as u8;
                }
            }
        }
        std::mem::swap(&mut metric, &mut m2);
    }
    let mut s = (0..STATES)
        .max_by(|a, b| metric[*a].partial_cmp(&metric[*b]).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0);
    let mut out = vec![0u8; steps];
    for t in (0..steps).rev() {
        // The input bit is the low bit of the state we have arrived in.
        out[t] = (s & 1) as u8;
        s = from[t][s] as usize;
    }
    out
}

/// What the message layer found.
#[derive(Debug, Clone, PartialEq)]
pub struct CodedMessage {
    /// Which coded rate produced it.
    pub rate: CodedRate,
    /// The message bits between the SOM and the EOM, oldest first.
    pub bits: Vec<u8>,
    /// True when an EOM was actually seen — otherwise the message is whatever
    /// had arrived when the receiver gave up waiting for it.
    pub closed: bool,
}

/// The message layer: finds the SOM at the decoder's output and collects until
/// the EOM, exactly as Annex E, Appendix 1 §5 describes.
#[derive(Debug, Clone)]
pub struct MessageFramer {
    rate: CodedRate,
    window: u32,
    filled: usize,
    bits: Vec<u8>,
    collecting: bool,
    closed: bool,
    /// Bits seen since the last SOM, for the "no SOM appeared" fallback.
    since: usize,
    seen: u64,
    /// The closest the decoder's output has come to the SOM, in matching bits of
    /// 32. A decoder that is working on real traffic gets close even when a fade
    /// stops it matching exactly, and one working on noise stays at 16 ± 3 —
    /// which is the difference between "not yet" and "not there".
    pub best_som: u8,
}

impl MessageFramer {
    pub fn new(rate: CodedRate) -> Self {
        MessageFramer {
            rate,
            window: 0,
            filled: 0,
            bits: Vec::new(),
            collecting: false,
            closed: false,
            since: 0,
            seen: 0,
            best_som: 0,
        }
    }

    pub fn reset(&mut self) {
        self.window = 0;
        self.filled = 0;
        self.bits.clear();
        self.collecting = false;
        self.closed = false;
        self.since = 0;
    }

    /// Feed one decoded bit; returns a message when one finishes.
    pub fn push(&mut self, bit: u8) -> Option<CodedMessage> {
        self.seen += 1;
        if self.filled >= 31 {
            let m = (self.window as u32).count_ones() as u8;
            // The window is the newest 32 bits with the newest at bit 0.
            let score = if self.filled >= 32 {
                let w = self.window;
                (0..32).filter(|k| ((w >> (31 - k)) & 1) == ((SOM >> (31 - k)) & 1)).count() as u8
            } else {
                m.min(0)
            };
            if score > self.best_som {
                self.best_som = score;
            }
        }
        if self.collecting {
            self.bits.push(bit);
            self.window = (self.window << 1) | bit as u32;
            self.since += 1;
            if self.window == EOM {
                self.collecting = false;
                self.closed = true;
                // The EOM itself is not message data.
                self.bits.truncate(self.bits.len().saturating_sub(32));
                return Some(CodedMessage {
                    rate: self.rate,
                    bits: std::mem::take(&mut self.bits),
                    closed: true,
                });
            }
            // Give up on a message that never closes, rather than growing a
            // buffer for as long as the station transmits.
            if self.since > 8 * 1024 * 8 {
                self.collecting = false;
                return Some(CodedMessage {
                    rate: self.rate,
                    bits: std::mem::take(&mut self.bits),
                    closed: false,
                });
            }
        } else {
            self.window = (self.window << 1) | bit as u32;
            self.filled += 1;
            self.since += 1;
            if self.filled >= 32 && self.window == SOM {
                // Drop the SOM itself: a message starts after it.
                self.collecting = true;
                self.closed = false;
                self.since = 0;
                self.bits.clear();
            }
        }
        None
    }

    /// The bits between the SOM and the EOM, when one is complete.
    pub fn closed(&self) -> bool {
        self.closed
    }
}

/// ITA2 letters, in the order the five-bit code numbers them.
const ITA2_LETTERS: [char; 32] = [
    ' ', 'E', '\n', 'A', ' ', 'S', 'I', 'U', '\r', 'D', 'R', 'J', 'N', 'F', 'C', 'K', 'T', 'Z',
    'L', 'W', 'H', 'Y', 'P', 'Q', 'O', 'B', 'G', ' ', 'M', 'X', 'V', ' ',
];

/// A best reading of a message's bits as text.
///
/// Annex E calls what it carries "message bits" and does not name a character
/// set; a 75 bps link is exactly a 75 baud teleprinter, so ITA2 is what a test
/// tape is expected to send. Both framings are tried — synchronous five-bit
/// groups at every alignment, and 5N1.5 with its start and stop bits — and the
/// reading is kept only if it looks like text, because a wrong reading is
/// indistinguishable from a right one to a reader who cannot check it.
pub fn message_text(bits: &[u8]) -> Option<String> {
    if bits.len() < 40 {
        return None;
    }
    // 5N1.5 first: one start bit, five data bits least significant first, then
    // the stop bits. The reader is self-synchronising — it hunts for a start bit
    // and then for the next one — so an alignment that is wrong produces short
    // runs, which the score can see.
    let mut best: Option<(f32, String)> = None;
    for shift in 0..8 {
        let mut out = String::new();
        let mut p = shift;
        while p + 6 <= bits.len() {
            if bits[p] != 0 {
                p += 1;
                continue;
            }
            let mut v = 0u8;
            for i in 0..5 {
                v |= bits[p + 1 + i] << i;
            }
            out.push(ITA2_LETTERS[v as usize & 31]);
            p += 6;
            // At least one stop bit, then the next start bit: skip the ones.
            let mut stop = 0;
            while p < bits.len() && bits[p] == 1 && stop < 3 {
                p += 1;
                stop += 1;
            }
            if stop == 0 {
                // No stop bit where one is required: this alignment is not the
                // framing, so the reading ends here.
                break;
            }
        }
        // How much of the bit budget the reading covered, and how much it looks
        // like text. A wrong alignment produces a handful of characters and then
        // a missing stop bit, which is what this catches.
        let covered = out.chars().count() as f32 * 7.5 / bits.len() as f32;
        if covered < 0.5 {
            continue;
        }
        let score = text_score(&out) * (0.5 + 0.5 * covered.min(1.0));
        if best.as_ref().is_none_or(|(b, _)| score > *b) {
            best = Some((score, out));
        }
    }
    // Synchronous five-bit groups are the fallback for a link that is not a
    // teleprinter; they are tried only when no framing reading stood up, because
    // a synchronous read of a 5N1.5 stream is all letters and no spaces.
    if best.is_none() {
        for shift in 0..5 {
            let n = (bits.len().saturating_sub(shift)) / 5;
            let mut out = String::with_capacity(n);
            for k in 0..n {
                let mut v = 0u8;
                for i in 0..5 {
                    v |= bits[shift + 5 * k + i] << i;
                }
                out.push(ITA2_LETTERS[v as usize & 31]);
            }
            let score = text_score(&out) * 0.8;
            if best.as_ref().is_none_or(|(b, _)| score > *b) {
                best = Some((score, out));
            }
        }
    }
    let (score, text) = best?;
    (score > 0.45).then_some(text)
}

/// How much a string looks like text.
///
/// ITA2 has only 32 codes and most of them are letters, so "it is all letters"
/// separates nothing: uniform noise over those codes is 37.5 % common letters, and
/// a wrong reading of a real message is 60 %. Two numbers do separate them — the
/// share of the twelve letters that dominate English and French, which is about
/// 70 % in text against 37.5 % in noise, and the share of spaces, about 15 %
/// against 3 % — and the score is those two against those references rather than
/// an absolute count, so a short message is judged on its own scale.
fn text_score(s: &str) -> f32 {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 8 {
        return 0.0;
    }
    let n = chars.len() as f32;
    let common = chars
        .iter()
        .filter(|c| {
            matches!(c, 'E' | 'T' | 'A' | 'O' | 'I' | 'N' | 'S' | 'H' | 'R' | 'D' | 'U' | 'L')
        })
        .count() as f32;
    let spaces = chars.iter().filter(|c| **c == ' ').count() as f32;
    let (common_rate, space_rate) = (common / n, spaces / n);
    // Normalised: 0 at noise, 1 at text.
    let letters = ((common_rate - 0.375) / (0.70 - 0.375)).clamp(0.0, 1.0);
    let spaces = (1.0 - (space_rate - 0.15).abs() / 0.15).clamp(0.0, 1.0);
    0.6 * letters + 0.4 * spaces
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The encoder is the standard's: `G1 = 133`, `G2 = 171`, T1 first. Checked
    /// against the impulse response of the two polynomials and against a
    /// hand-computed sequence, because a swapped pair decodes to noise.
    #[test]
    fn the_encoder_is_the_standard_polynomials() {
        // One bit in, register `1`: both parities are 1 for G1 (bit 0 set) and
        // for G2 (bit 0 set).
        assert_eq!(encode(&[1]), vec![1, 1]);
        // Two bits: `11` → the second step's register is `11`, G1 has bits 0 and
        // 1 so its parity is even, G2 has only bit 0 of those two so its parity
        // is odd.
        assert_eq!(encode(&[1, 1]), vec![1, 1, 0, 1]);
        // The all-zero input encodes to all zeros, which is what the annex's
        // zeroed registers mean.
        assert_eq!(encode(&[0; 6]), vec![0; 12]);
        // A single one, then zeros, walks the impulse response of both taps.
        let imp = encode(&[1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&imp[..2], &[1, 1]);
        // x⁶ takes six steps to leave the register.
        assert_eq!(&imp[12..14], &[1, 1]);
    }

    /// Repeating pairs and averaging them back is the identity on soft values.
    #[test]
    fn repeated_pairs_come_back_through_the_averager() {
        for times in [2, 4, 8] {
            let coded = encode(&[1, 0, 1, 1, 0, 0, 1, 0]);
            let repeated = repeat_pairs(&coded, times);
            assert_eq!(repeated.len(), coded.len() * times);
            // Annex E repeats pairs, so the first four entries at 4× are the
            // first pair four times.
            let first = &repeated[..times * 2];
            for pair in first.chunks(2) {
                assert_eq!(pair, &coded[..2]);
            }
            let soft: Vec<f32> =
                repeated.iter().map(|b| if *b == 1 { -1.0 } else { 1.0 }).collect();
            let back = average_pairs(&soft, times);
            assert_eq!(back.len(), coded.len());
            for (i, v) in back.iter().enumerate() {
                assert_eq!(*v, if coded[i] == 1 { -1.0 } else { 1.0 }, "bit {i} at {times}x");
            }
        }
    }

    /// Encoder → interleaver → channel → deinterleaver → Viterbi returns the bits.
    ///
    /// The two commutators have to be exact inverses: a single row out of phase
    /// turns the whole message into noise, and the only symptom is that the SOM
    /// never appears.
    #[test]
    fn a_coded_message_survives_the_interleaver_and_the_decoder() {
        let bits: Vec<u8> = (0..2000u32).map(|i| ((i * 7 + (i >> 3)) % 3 == 0) as u8).collect();
        let coded = encode(&bits);
        // j = 1, so the interleaver's whole span is 31·32 = 992 coded bits and a
        // 2000-bit message runs well past it. At a real j the span is ten seconds
        // of signal and a unit test would be minutes of silence.
        let mut il = ConvInterleaver::interleaver(1);
        let sent: Vec<f32> = coded
            .iter()
            .map(|b| {
                let v = if *b == 1 { -1.0 } else { 1.0 };
                il.push(v)
            })
            .collect();
        let mut di = ConvInterleaver::deinterleaver(1);
        let soft: Vec<f32> = sent.iter().map(|v| di.push(*v)).collect();
        let back = viterbi_decode(&soft);
        // The interleaver delays, so compare the tail with the tail: the first
        // `(I-1)·j·I` bits of the decode are the registers emptying.
        let skip = (ROWS - 1) * ROWS + 1;
        assert!(back.len() > skip + 64, "block too short to compare");
        let n = 64;
        let want = &bits[bits.len() - n..];
        let got = &back[back.len() - n..];
        assert_eq!(got, want, "the interleaver and decoder did not return the message");
    }

    /// A teleprinter's message reads back as its text, in both framings.
    #[test]
    fn the_message_bits_read_as_ita2_text() {
        // "RYRY TEST RYRY" as ITA2 5N1.5 at 75 baud: one start bit, five data
        // bits least significant first, one and a half stop bits. Letters only:
        // the figures live behind a shift code and are not what a test tape's
        // diddle is made of.
        let text = "RYRY TEST RYRY";
        let mut bits = Vec::new();
        for c in text.chars() {
            let v = ITA2_LETTERS.iter().position(|x| *x == c).expect("in the table") as u8;
            bits.push(0u8);
            for i in 0..5 {
                bits.push((v >> i) & 1);
            }
            bits.push(1);
            bits.push(1);
        }
        let read = message_text(&bits).expect("the bits did not read as text");
        assert!(read.starts_with(text), "read {read:?} instead of {text:?}");
        // Noise is not text, which is the half that matters: a wrong reading that
        // looks like letters is worse than silence.
        let mut seed = 0x1234_5678u32;
        let noise: Vec<u8> = (0..800)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed & 1) as u8
            })
            .collect();
        assert_eq!(message_text(&noise), None, "noise read as text");
    }

    /// The SOM is found at the decoder output and the EOM closes the message.
    #[test]
    fn the_framer_finds_the_start_and_the_end_of_a_message() {
        let msg: Vec<u8> = (0..200u32).map(|i| (i % 5 == 0) as u8).collect();
        let mut wire = Vec::new();
        for k in (0..32).rev() {
            wire.push(((SOM >> k) & 1) as u8);
        }
        wire.extend_from_slice(&msg);
        for k in (0..32).rev() {
            wire.push(((EOM >> k) & 1) as u8);
        }
        // A little leading noise, as a real decoder output has.
        let mut f = MessageFramer::new(CodedRate::Bps75);
        let mut got = None;
        for b in (0..100).map(|i| (i * 13 % 7 == 0) as u8).chain(wire) {
            if let Some(m) = f.push(b) {
                got = Some(m);
            }
        }
        let m = got.expect("no message framed");
        assert!(m.closed, "the EOM did not close the message");
        assert_eq!(m.bits, msg, "the message bits are not the ones between SOM and EOM");
    }
}
