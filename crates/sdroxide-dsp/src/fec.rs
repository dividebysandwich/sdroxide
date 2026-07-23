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
    let mut state = if terminated {
        0
    } else {
        (0..NSTATES).min_by_key(|&s| pm[s]).unwrap_or(0)
    };
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

#[cfg(test)]
mod tests {
    use super::*;

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
