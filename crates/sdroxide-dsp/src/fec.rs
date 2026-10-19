//! Constraint-length-7, rate-1/2 convolutional FEC (the NASA/`0155,0117` polys)
//! with a hard-decision Viterbi decoder. Used by the THOR modem; written as a
//! standalone block so it can be unit-tested independently.

const K: usize = 7;
/// Generator polynomials 0o155 and 0o117 (the standard K=7 r=1/2 code).
const G1: u32 = 0o155;
const G2: u32 = 0o117;
const NSTATES: usize = 1 << (K - 1); // 64

fn parity(x: u32) -> u8 {
    (x.count_ones() & 1) as u8
}

/// The two output bits for input `b` (0/1) from state `s` (6 bits).
fn outputs(s: usize, b: u8) -> (u8, u8) {
    let reg = ((s as u32) << 1) | b as u32; // 7-bit register value
    (parity(reg & G1), parity(reg & G2))
}

/// Streaming rate-1/2 convolutional encoder (same code as [`conv_encode`]).
#[derive(Default)]
pub struct ConvEnc {
    s: usize,
}

impl ConvEnc {
    pub fn new() -> Self {
        ConvEnc { s: 0 }
    }
    /// Encode one message bit into its two coded bits.
    pub fn encode_bit(&mut self, b: u8) -> (u8, u8) {
        let b = b & 1;
        let out = outputs(self.s, b);
        self.s = ((self.s << 1) | b as usize) & (NSTATES - 1);
        out
    }
}

/// Encode a message bitstream, appending `K-1` flush bits so the trellis
/// terminates in state 0. Output length = `(bits.len() + K - 1) * 2`. Used by the
/// FEC unit tests (streaming TX uses [`ConvEnc`]).
#[cfg(test)]
pub fn conv_encode(bits: &[u8]) -> Vec<u8> {
    let mut s = 0usize;
    let mut out = Vec::with_capacity((bits.len() + K - 1) * 2);
    for &b in bits.iter().chain(std::iter::repeat(&0u8).take(K - 1)) {
        let b = b & 1;
        let (o1, o2) = outputs(s, b);
        out.push(o1);
        out.push(o2);
        s = (((s << 1) | b as usize) & (NSTATES - 1)) as usize;
    }
    out
}

/// Hard-decision Viterbi decode. With `terminated = true` the traceback starts
/// from state 0 (a flushed block) and the `K-1` tail bits are dropped; otherwise
/// it starts from the best surviving state (a streaming prefix) and returns all
/// `coded.len()/2` bits.
pub fn viterbi_decode(coded: &[u8], terminated: bool) -> Vec<u8> {
    let nsteps = coded.len() / 2;
    if nsteps == 0 {
        return Vec::new();
    }
    // A terminated block starts (and ends) in state 0; a streaming prefix has an
    // unknown start state, so all states begin equally likely.
    let mut pm = if terminated {
        let mut p = vec![u32::MAX / 2; NSTATES];
        p[0] = 0;
        p
    } else {
        vec![0u32; NSTATES]
    };
    // trace[step][state] = the input bit on the surviving branch into `state`.
    let mut trace = vec![0u8; nsteps * NSTATES];
    let mut prev = vec![0usize; nsteps * NSTATES];
    for step in 0..nsteps {
        let r1 = coded[2 * step];
        let r2 = coded[2 * step + 1];
        let mut npm = vec![u32::MAX / 2; NSTATES];
        for s in 0..NSTATES {
            if pm[s] >= u32::MAX / 2 {
                continue;
            }
            for b in 0u8..2 {
                let (o1, o2) = outputs(s, b);
                let bm = (o1 ^ r1) as u32 + (o2 ^ r2) as u32;
                let ns = ((s << 1) | b as usize) & (NSTATES - 1);
                let cand = pm[s] + bm;
                if cand < npm[ns] {
                    npm[ns] = cand;
                    trace[step * NSTATES + ns] = b;
                    prev[step * NSTATES + ns] = s;
                }
            }
        }
        pm = npm;
    }
    // Choose the traceback start state.
    let mut state = if terminated { 0 } else { (0..NSTATES).min_by_key(|&s| pm[s]).unwrap_or(0) };
    let mut bits = vec![0u8; nsteps];
    for step in (0..nsteps).rev() {
        bits[step] = trace[step * NSTATES + state];
        state = prev[step * NSTATES + state];
    }
    if terminated {
        bits.truncate(nsteps.saturating_sub(K - 1));
    }
    bits
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

    /// PSK31's **QPSK** code: constraint length 5, polynomials `0x17` and
    /// `0x19`. Same construction as [`ConvCode::THOR`] at two fewer bits, which
    /// is why the rest of this module serves it with no new machinery.
    pub const PSK_QPSK: ConvCode = ConvCode { poly_a: 0x17, poly_b: 0x19, invert_b: false };

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

    /// The reason for carrying soft values at all: given the same channel, the
    /// soft decoder survives noise that defeats the hard one.
    #[test]
    fn soft_decisions_beat_hard_ones_on_the_same_noisy_channel() {
        let msg: Vec<u8> = (0..400).map(|i| ((i * 29 + 7) & 1) as u8).collect();
        let coded = encode_with(&msg, ConvCode::THOR);

        // A plain AWGN channel: ±1 plus Gaussian noise, sliced for the hard
        // decoder and passed through whole for the soft one.
        let mut st = 0x1234_5678_9abc_def0u64;
        let mut norm = || {
            // Two uniforms into one approximately Gaussian sample.
            let mut u = || {
                st ^= st << 13;
                st ^= st >> 7;
                st ^= st << 17;
                (st >> 11) as f32 / (1u64 << 53) as f32
            };
            let (a, b): (f32, f32) = (u().max(1e-9), u());
            (-2.0 * a.ln()).sqrt() * (std::f32::consts::TAU * b).cos()
        };
        let sigma = 1.1;
        let rx: Vec<f32> =
            coded.iter().map(|&b| if b == 1 { 1.0 } else { -1.0 } + sigma * norm()).collect();
        let hard: Vec<u8> = rx.iter().map(|&v| u8::from(v > 0.0)).collect();

        let errs = |d: &[u8]| d.iter().zip(&msg).filter(|(a, b)| a != b).count();
        let soft_errs = errs(&viterbi_soft(&rx, ConvCode::THOR, true));
        let hard_errs = errs(&viterbi_decode(&hard, true));
        assert!(
            soft_errs < hard_errs,
            "soft {soft_errs} errors vs hard {hard_errs} — soft decisions should win"
        );
    }

    #[test]
    fn conv_roundtrip_clean() {
        let msg: Vec<u8> = (0..200).map(|i| ((i * 37 + 11) & 1) as u8).collect();
        let coded = conv_encode(&msg);
        let dec = viterbi_decode(&coded, true);
        assert_eq!(dec, msg);
    }

    #[test]
    fn conv_corrects_errors() {
        let msg: Vec<u8> = (0..120).map(|i| ((i * 13 + 5) & 1) as u8).collect();
        let mut coded = conv_encode(&msg);
        // Flip a handful of well-separated coded bits.
        for &i in &[3usize, 20, 45, 80, 140] {
            coded[i] ^= 1;
        }
        let dec = viterbi_decode(&coded, true);
        assert_eq!(dec, msg, "Viterbi should correct sparse bit errors");
    }
}
