//! Constraint-length-7, rate-1/2 convolutional FEC (the NASA/`0155,0117` polys)
//! with a hard-decision Viterbi decoder. Used by the THOR modem; written as a
//! standalone block so it can be unit-tested independently.

use std::collections::VecDeque;

/// Generator polynomials 0o155 and 0o117 (the standard K=7 r=1/2 code).
const G1: u32 = 0o155;
const G2: u32 = 0o117;

fn parity(x: u32) -> u8 {
    (x.count_ones() & 1) as u8
}

/// One rate-1/2, constraint-length-7 convolutional code, as the standard
/// leaves room for: two generator polynomials and whether the second output is
/// sent inverted.
///
/// The polynomials are in the convention this module uses throughout — the
/// newest input bit is the *low* bit of the 7-bit register — so the CCSDS pair
/// usually written `0o171`/`0o133` appears here bit-reversed, as
/// `0o117`/`0o155`. Nothing distinguishes the two conventions except which end
/// of the register the new bit enters, and a decoder built on the wrong one
/// produces confident nonsense, so state the polynomials you mean rather than
/// assuming a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvCode {
    pub poly_a: u32,
    pub poly_b: u32,
    /// Whether the second output symbol is transmitted inverted. Several
    /// standards do this to guarantee transitions on an all-zero message,
    /// which is what a receiver's symbol-timing recovery needs to stay locked
    /// through a quiet stretch.
    pub invert_b: bool,
}

impl ConvCode {
    /// The pair this module's own [`conv_encode`]/[`viterbi_decode`] use.
    pub const THOR: ConvCode = ConvCode { poly_a: G1, poly_b: G2, invert_b: false };

    /// The **PSK robust** code — the one BPSK63F and the whole `R`/`RC` family
    /// carry. fldigi's `PSKR_K`/`PSKR_POLY1`/`PSKR_POLY2` (`src/psk/psk.cxx`):
    /// constraint length 7, polynomials `0x6d` and `0x4f`.
    ///
    /// This is *not* [`ConvCode::THOR`]'s pair with a different name — the
    /// standard K=7 code is `0o155`/`0o117` and this one is another code
    /// entirely. Both outputs are sent as they are, so nothing is inverted.
    pub const PSK_R: ConvCode = ConvCode { poly_a: 0x6d, poly_b: 0x4f, invert_b: false };

    /// PSK31's **QPSK** code: constraint length 5, polynomials `0x17` and
    /// `0x19`. Same construction as [`ConvCode::THOR`] at two fewer bits, which
    /// is why the rest of this module serves it with no new machinery.
    ///
    /// The second output is **inverted**: PSK's receiver complements it before
    /// the decoder sees it, which is the same thing as the code emitting it
    /// inverted, and keeping it here means the transmitter and the receiver
    /// cannot disagree about which end applies it.
    pub const PSK_QPSK: ConvCode = ConvCode { poly_a: 0x17, poly_b: 0x19, invert_b: true };

    /// Constraint length, read off the polynomials rather than declared.
    ///
    /// The register is as wide as the highest set bit of either polynomial, and
    /// for both codes this tree carries that is exactly the constraint length:
    /// `0x17|0x19` is five bits, `0o155|0o117` is seven. One fewer field to get
    /// wrong, and a code that cannot disagree with itself about its own size.
    pub fn k(&self) -> u32 {
        32 - (self.poly_a | self.poly_b).leading_zeros()
    }

    /// Trellis states: `2^(k-1)`.
    pub fn states(&self) -> usize {
        1usize << (self.k() - 1)
    }

    /// The two output symbols for input `b` from state `s`.
    fn outputs(&self, s: usize, b: u8) -> (u8, u8) {
        let reg = ((s as u32) << 1) | b as u32;
        (parity(reg & self.poly_a), parity(reg & self.poly_b) ^ self.invert_b as u8)
    }
}

/// Soft-decision Viterbi decode.
///
/// `sym` carries two soft values per message bit, in transmission order, each
/// positive for a received `1` and negative for a `0`, with the magnitude
/// standing for how sure the demodulator is. That confidence is the whole
/// point: a hard-decision decoder throws it away at the slicer and pays about
/// 2 dB for it, which on a weak-signal link is the difference between decoding
/// and not.
///
/// With `terminated` the traceback starts from state 0 — the encoder was
/// flushed with `K-1` zero bits — and those tail bits are dropped from the
/// result. Otherwise it starts from whichever state ended best and every bit
/// is returned.
pub fn viterbi_soft(sym: &[f32], code: ConvCode, terminated: bool) -> Vec<u8> {
    let nsteps = sym.len() / 2;
    if nsteps == 0 {
        return Vec::new();
    }
    // Branch outputs for every (state, input), as ±1 signs to correlate with.
    let n = code.states();
    let mut sign = vec![[(0.0f32, 0.0f32); 2]; n];
    for (s, row) in sign.iter_mut().enumerate() {
        for (b, cell) in row.iter_mut().enumerate() {
            let (a, bb) = code.outputs(s, b as u8);
            *cell = (if a == 1 { 1.0 } else { -1.0 }, if bb == 1 { 1.0 } else { -1.0 });
        }
    }

    // The encoder starts in state 0, so at step 0 every other state is
    // impossible rather than merely unlikely.
    let mut metric = vec![f32::NEG_INFINITY; n];
    metric[0] = 0.0;
    let mut next = vec![f32::NEG_INFINITY; n];
    // One decision bit per state per step: which of the two predecessors won.
    let mut back = vec![0u64; nsteps];

    for (t, slot) in back.iter_mut().enumerate() {
        let (r0, r1) = (sym[2 * t], sym[2 * t + 1]);
        let mut best = f32::NEG_INFINITY;
        for (ns, slot_metric) in next.iter_mut().enumerate() {
            // `ns` is reached from `ns >> 1` or `(ns >> 1) | 32`, in both cases
            // by feeding in `ns & 1`.
            let b = ns & 1;
            let (p0, p1) = (ns >> 1, (ns >> 1) | (n / 2));
            let (a0, b0) = sign[p0][b];
            let (a1, b1) = sign[p1][b];
            let m0 = metric[p0] + a0 * r0 + b0 * r1;
            let m1 = metric[p1] + a1 * r0 + b1 * r1;
            let (m, from) = if m0 >= m1 { (m0, 0u64) } else { (m1, 1u64) };
            *slot |= from << ns;
            *slot_metric = m;
            if m > best {
                best = m;
            }
        }
        // Hold the metrics near zero. Only differences matter, and over a few
        // thousand steps the absolute values would otherwise run away.
        if best.is_finite() {
            for m in next.iter_mut() {
                *m -= best;
            }
        }
        metric = next;
        next = vec![f32::NEG_INFINITY; n];
    }

    let mut s = if terminated {
        0
    } else {
        (0..n).max_by(|&a, &b| metric[a].total_cmp(&metric[b])).unwrap_or(0)
    };
    let mut bits = vec![0u8; nsteps];
    for t in (0..nsteps).rev() {
        // The input bit that reached this state is its own low bit.
        bits[t] = (s & 1) as u8;
        let from = (back[t] >> s) & 1;
        s = (s >> 1) | ((from as usize) * (n / 2));
    }
    if terminated {
        bits.truncate(nsteps.saturating_sub(code.k() as usize - 1));
    }
    bits
}

/// A rate-1/2 encoder for an arbitrary [`ConvCode`], handing back the two
/// output bits packed as a symbol.
///
/// The packed form is what the transmitters want: PSK's QPSK maps the pair
/// straight onto a phase step of `sym * 90°`, and MFSK maps it onto a tone.
pub struct ConvEncoder {
    code: ConvCode,
    state: usize,
}

impl ConvEncoder {
    pub fn new(code: ConvCode) -> Self {
        ConvEncoder { code, state: 0 }
    }

    pub fn reset(&mut self) {
        self.state = 0;
    }

    /// Encode one bit into its two output bits, packed as `o1 | o2 << 1`.
    pub fn push(&mut self, bit: u8) -> u8 {
        let b = bit & 1;
        let (o1, o2) = self.code.outputs(self.state, b);
        self.state = ((self.state << 1) | b as usize) & (self.code.states() - 1);
        o1 | (o2 << 1)
    }
}

/// Streaming soft-decision Viterbi, for a mode that never stops.
///
/// [`viterbi_soft`] decodes a *finished block*: it starts its metrics at state 0
/// and traces back from whichever state ended best. A continuous mode has
/// neither of those — the trellis runs for the length of the over and there is no
/// final state to start from — so restarting it every block would put a
/// discontinuity in the middle of the message.
///
/// So the survivor history is kept in a ring and each bit is emitted once it is
/// `depth` steps old. By then the path has forgotten where it started, which is
/// the property the traceback depends on; the usual rule of five constraint
/// lengths is used, and it is nothing more than that rule.
///
/// Feed it two soft symbols per message bit — positive for a received `1` — and
/// [`ViterbiStream::flush`] at the end of the over to collect what is still in
/// the ring. A transmitter that sends the `k-1` zero tail bits will decode those
/// as zeros; they are the caller's to drop.
pub struct ViterbiStream {
    code: ConvCode,
    n: usize,
    metric: Vec<f32>,
    /// One survivor mask per step: bit `s` is the winning predecessor of state
    /// `s`. Fits in a `u64` because no constraint length here exceeds seven.
    ring: VecDeque<u64>,
    depth: usize,
    started: bool,
    /// Smoothed growth of the best path's metric, which is how well the incoming
    /// symbols fit *a* trellis path. Two decoders fed the same symbols in
    /// different pairings can therefore be told apart, which is what the PSK
    /// robust modes need to find their bit alignment.
    quality: f32,
}

impl ViterbiStream {
    pub fn new(code: ConvCode) -> Self {
        let n = code.states();
        assert!(n <= 64, "a survivor mask has to fit in a u64");
        ViterbiStream {
            code,
            n,
            metric: vec![0.0; n],
            ring: VecDeque::new(),
            depth: (5 * code.k()) as usize,
            started: false,
            quality: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.metric.iter_mut().for_each(|m| *m = 0.0);
        self.ring.clear();
        self.started = false;
        self.quality = 0.0;
    }

    /// How well the recent symbols fit a trellis path, in the units of one
    /// symbol pair's correlation (about `+2` for a clean aligned stream, about
    /// `0` for a pairing that carries no code).
    pub fn quality(&self) -> f32 {
        self.quality
    }

    fn best_state(&self) -> usize {
        (0..self.n).max_by(|&a, &b| self.metric[a].total_cmp(&self.metric[b])).unwrap_or(0)
    }

    /// Walk back through the ring from the best state and return the oldest bit.
    fn traceback_oldest(&self) -> u8 {
        let mut s = self.best_state();
        let mut bit = 0u8;
        for mask in self.ring.iter().rev() {
            bit = (s & 1) as u8;
            let from = (mask >> s) & 1;
            s = (s >> 1) | ((from as usize) * (self.n / 2));
        }
        bit
    }

    /// Feed the two soft symbols for one message bit; returns the bit that has
    /// fallen out the far end of the traceback window, once there is one.
    pub fn push(&mut self, a: f32, b: f32) -> Option<u8> {
        if !self.started {
            self.metric.iter_mut().for_each(|m| *m = f32::NEG_INFINITY);
            self.metric[0] = 0.0;
            self.started = true;
        }
        let mut next = vec![f32::NEG_INFINITY; self.n];
        let mut mask = 0u64;
        for ns in 0..self.n {
            let bit = ns & 1;
            let (p0, p1) = (ns >> 1, (ns >> 1) | (self.n / 2));
            let (s0a, s0b) = {
                let (o1, o2) = self.code.outputs(p0, bit as u8);
                (if o1 == 1 { 1.0 } else { -1.0 }, if o2 == 1 { 1.0 } else { -1.0 })
            };
            let (s1a, s1b) = {
                let (o1, o2) = self.code.outputs(p1, bit as u8);
                (if o1 == 1 { 1.0 } else { -1.0 }, if o2 == 1 { 1.0 } else { -1.0 })
            };
            let m0 = self.metric[p0] + s0a * a + s0b * b;
            let m1 = self.metric[p1] + s1a * a + s1b * b;
            let (m, from) = if m0 >= m1 { (m0, 0u64) } else { (m1, 1u64) };
            mask |= from << ns;
            next[ns] = m;
        }
        // Only differences matter, and the absolute values would run away.
        let best = next.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        if best.is_finite() {
            self.quality += 0.02 * (best - self.quality);
            next.iter_mut().for_each(|m| *m -= best);
        }
        self.metric = next;
        self.ring.push_back(mask);
        if self.ring.len() > self.depth {
            // Trace *before* dropping the oldest step: that step's mask is what
            // says which input bit reached each state, so popping first throws
            // away the very bit about to be emitted.
            let bit = self.traceback_oldest();
            self.ring.pop_front();
            Some(bit)
        } else {
            None
        }
    }

    /// Collect what is still inside the traceback window, oldest first.
    ///
    /// These are decided with less history behind them than a pushed bit, so a
    /// caller that cares about the last few should send the transmitter's tail
    /// bits and drop them.
    pub fn flush(&mut self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.ring.len());
        while !self.ring.is_empty() {
            out.push(self.traceback_oldest());
            self.ring.pop_front();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode with an arbitrary [`ConvCode`], for testing the soft decoder
    /// against codes this module does not otherwise transmit.
    fn encode_with(bits: &[u8], code: ConvCode) -> Vec<u8> {
        let mut s = 0usize;
        let k = code.k() as usize;
        let mut out = Vec::with_capacity((bits.len() + k - 1) * 2);
        for &b in bits.iter().chain(std::iter::repeat_n(&0u8, k - 1)) {
            let b = b & 1;
            let (o1, o2) = code.outputs(s, b);
            out.push(o1);
            out.push(o2);
            s = ((s << 1) | b as usize) & (code.states() - 1);
        }
        out
    }

    /// The stream decodes a message that never stops, which is the only shape
    /// QPSK and MFSK ever present: no final state, no block boundary.
    ///
    /// Both codes, because the state count is what the generalisation fixed and
    /// a stream is where a wrong one shows up as a discontinuity mid-message
    /// rather than as a single bad bit.
    #[test]
    fn the_stream_decodes_a_continuous_message() {
        for code in [ConvCode::PSK_QPSK, ConvCode::THOR] {
            let bits: Vec<u8> = (0..600u32).map(|i| ((i * 7 + i / 3) % 2) as u8).collect();
            let coded = encode_with(&bits, code);
            let softs = soft(&coded, 1.0);
            let mut v = ViterbiStream::new(code);
            let mut got = Vec::new();
            for pair in softs.chunks(2) {
                if let Some(b) = v.push(pair[0], pair[1]) {
                    got.push(b);
                }
            }
            got.extend(v.flush());
            assert_eq!(
                &got[..bits.len()],
                &bits[..],
                "k={} lost the message in the stream",
                code.k()
            );
        }
    }

    /// And it still corrects a channel, which is the whole reason it exists: a
    /// stream that only worked on clean input would decode nothing on the air.
    #[test]
    fn the_stream_corrects_a_noisy_channel() {
        let code = ConvCode::PSK_QPSK;
        let bits: Vec<u8> = (0..600u32).map(|i| ((i * 11 + 5) % 2) as u8).collect();
        let coded = encode_with(&bits, code);
        let mut softs = soft(&coded, 1.0);
        for (i, v) in softs.iter_mut().enumerate() {
            if i % 17 == 0 {
                *v = -*v;
            }
        }
        let mut v = ViterbiStream::new(code);
        let mut got = Vec::new();
        for pair in softs.chunks(2) {
            if let Some(b) = v.push(pair[0], pair[1]) {
                got.push(b);
            }
        }
        got.extend(v.flush());
        assert_eq!(&got[..bits.len()], &bits[..], "the stream did not correct the channel");
    }

    /// PSK31's QPSK code is the same machine two bits shorter: K=5, sixteen
    /// states, and it corrects a noisy channel just as the K=7 one does.
    ///
    /// This is the guard for the generalisation. The state count used to be a
    /// constant taken from the K=7 code, so a K=5 code would have been decoded
    /// against a 64-state trellis — confident nonsense, and no test would have
    /// noticed because nothing in the tree asked for K=5 yet.
    #[test]
    fn the_qpsk_code_is_the_same_machine_at_k5() {
        let code = ConvCode::PSK_QPSK;
        assert_eq!(code.k(), 5, "the constraint length is not read off the polynomials");
        assert_eq!(code.states(), 16);
        assert_eq!(ConvCode::THOR.k(), 7);
        assert_eq!(ConvCode::THOR.states(), 64);

        let bits: Vec<u8> = (0..200u32).map(|i| ((i * 7 + 3) % 2) as u8).collect();
        let coded = encode_with(&bits, code);
        assert_eq!(viterbi_soft(&soft(&coded, 1.0), code, true), bits, "a clean K=5 block");

        // And on a channel that flips one symbol in thirteen.
        let mut noisy = soft(&coded, 1.0);
        for (i, v) in noisy.iter_mut().enumerate() {
            if i % 13 == 0 {
                *v = -*v;
            }
        }
        assert_eq!(
            viterbi_soft(&noisy, code, true),
            bits,
            "the K=5 decoder did not correct the channel"
        );
    }

    /// Coded bits as ideal soft values: ±`amp`.
    fn soft(coded: &[u8], amp: f32) -> Vec<f32> {
        coded.iter().map(|&b| if b == 1 { amp } else { -amp }).collect()
    }

    #[test]
    fn the_soft_decoder_round_trips_a_clean_block() {
        let msg: Vec<u8> = (0..200).map(|i| ((i * 37 + 11) & 1) as u8).collect();
        let coded = encode_with(&msg, ConvCode::THOR);
        assert_eq!(viterbi_soft(&soft(&coded, 1.0), ConvCode::THOR, true), msg);
    }

    /// An inverted second output is a property of the *code*, not of the
    /// channel: decode with `invert_b` wrong and every bit is suspect.
    #[test]
    fn an_inverted_second_output_is_decoded_only_by_a_matching_code() {
        let inv = ConvCode { invert_b: true, ..ConvCode::THOR };
        let msg: Vec<u8> = (0..150).map(|i| ((i * 17 + 3) & 1) as u8).collect();
        let coded = encode_with(&msg, inv);
        assert_eq!(viterbi_soft(&soft(&coded, 1.0), inv, true), msg);

        let wrong = viterbi_soft(&soft(&coded, 1.0), ConvCode::THOR, true);
        let agree = wrong.iter().zip(&msg).filter(|(a, b)| a == b).count();
        assert!(agree < msg.len() * 3 / 4, "decoded {agree}/{} with the wrong code", msg.len());
    }
}
