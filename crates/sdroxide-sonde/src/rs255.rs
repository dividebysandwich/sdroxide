//! Reed-Solomon (255, 223) over GF(256), as the LMS6-400 uses it: field
//! polynomial 0x187, 32 roots `(α^11)^(112 + j)`, conventional (not dual)
//! basis. Up to 16 wrong bytes in a block are found and repaired.
//!
//! Textbook decoding: syndromes, Berlekamp-Massey for the error locator, a
//! search for its roots, Forney's formula for the error values. The first
//! consecutive root and the primitive element were found by trying them on a
//! real recording: all 32 syndromes of a decoded block are zero.

const FIELD_POLY: u16 = 0x187;
const PRIM: usize = 11;
const FIRST_ROOT: usize = 112;
const PARITY: usize = 32;
pub const BLOCK: usize = 255;

pub struct Rs {
    exp: [u8; 512],
    log: [u8; 256],
}

impl Default for Rs {
    fn default() -> Self {
        Self::new()
    }
}

impl Rs {
    #[allow(clippy::needless_range_loop)] // the index is the exponent
    pub fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x: u16 = 1;
        for i in 0..255 {
            exp[i] = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= FIELD_POLY;
            }
        }
        for i in 255..512 {
            exp[i] = exp[i - 255];
        }
        Self { exp, log }
    }

    fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            self.exp[usize::from(self.log[usize::from(a)]) + usize::from(self.log[usize::from(b)])]
        }
    }

    fn div(&self, a: u8, b: u8) -> u8 {
        debug_assert!(b != 0);
        if a == 0 {
            0
        } else {
            self.exp[usize::from(self.log[usize::from(a)]) + 255
                - usize::from(self.log[usize::from(b)])]
        }
    }

    /// `β^e` with `β = α^11`, for any integer `e`.
    fn beta(&self, e: i64) -> u8 {
        self.exp[(e * PRIM as i64).rem_euclid(255) as usize]
    }

    fn syndromes(&self, cw: &[u8]) -> [u8; PARITY] {
        let mut s = [0u8; PARITY];
        for (j, out) in s.iter_mut().enumerate() {
            let root = self.beta((FIRST_ROOT + j) as i64);
            *out = cw.iter().fold(0u8, |acc, &c| self.mul(acc, root) ^ c);
        }
        s
    }

    /// Repair `cw` (255 bytes) in place. The number of bytes repaired, or
    /// `None` when there are more errors than the code can repair.
    pub fn correct(&self, cw: &mut [u8]) -> Option<usize> {
        if cw.len() != BLOCK {
            return None;
        }
        let syn = self.syndromes(cw);
        if syn.iter().all(|&s| s == 0) {
            return Some(0);
        }
        // Berlekamp-Massey: the error locator Λ, lowest power first.
        let mut lambda = vec![1u8];
        let mut prev = vec![1u8];
        let (mut l, mut m, mut b) = (0usize, 1usize, 1u8);
        for n in 0..PARITY {
            let mut d = syn[n];
            for i in 1..=l.min(lambda.len() - 1) {
                d ^= self.mul(lambda[i], syn[n - i]);
            }
            if d == 0 {
                m += 1;
            } else if 2 * l <= n {
                let t = lambda.clone();
                let coef = self.div(d, b);
                if lambda.len() < prev.len() + m {
                    lambda.resize(prev.len() + m, 0);
                }
                for (i, &p) in prev.iter().enumerate() {
                    lambda[i + m] ^= self.mul(coef, p);
                }
                l = n + 1 - l;
                prev = t;
                b = d;
                m = 1;
            } else {
                let coef = self.div(d, b);
                if lambda.len() < prev.len() + m {
                    lambda.resize(prev.len() + m, 0);
                }
                for (i, &p) in prev.iter().enumerate() {
                    lambda[i + m] ^= self.mul(coef, p);
                }
                m += 1;
            }
        }
        if l == 0 || l > PARITY / 2 {
            return None;
        }
        lambda.resize(l + 1, 0);
        // Roots of Λ: the byte at distance `p` from the end is wrong when
        // Λ(β^-p) = 0.
        let eval = |poly: &[u8], x: u8| poly.iter().rev().fold(0u8, |acc, &c| self.mul(acc, x) ^ c);
        let mut positions = Vec::new();
        for p in 0..BLOCK {
            if eval(&lambda, self.beta(-(p as i64))) == 0 {
                positions.push(p);
            }
        }
        if positions.len() != l {
            return None;
        }
        // Ω = S·Λ mod x^32.
        let mut omega = vec![0u8; PARITY];
        for (i, o) in omega.iter_mut().enumerate() {
            for j in 0..=i.min(l) {
                *o ^= self.mul(lambda[j], syn[i - j]);
            }
        }
        // Λ', in characteristic 2 only the odd powers survive.
        let dlambda: Vec<u8> = (1..=l).map(|i| if i % 2 == 1 { lambda[i] } else { 0 }).collect();
        let mut fixed = cw.to_vec();
        for &p in &positions {
            let xinv = self.beta(-(p as i64));
            let num = eval(&omega, xinv);
            let den = eval(&dlambda, xinv);
            if den == 0 {
                return None;
            }
            // X^(1 - first root) · Ω(X^-1) / Λ'(X^-1)
            let scale = self.beta(p as i64 * (1 - FIRST_ROOT as i64));
            let e = self.mul(scale, self.div(num, den));
            fixed[BLOCK - 1 - p] ^= e;
        }
        if self.syndromes(&fixed).iter().any(|&s| s != 0) {
            return None;
        }
        cw.copy_from_slice(&fixed);
        Some(positions.len())
    }

    /// The 32 parity bytes of a 223-byte message (tests and tools).
    pub fn parity(&self, msg: &[u8]) -> [u8; PARITY] {
        assert_eq!(msg.len(), BLOCK - PARITY);
        // Generator: Π (x - β^(first + j)).
        let mut genp = vec![1u8];
        for j in 0..PARITY {
            let root = self.beta((FIRST_ROOT + j) as i64);
            let mut next = vec![0u8; genp.len() + 1];
            for (i, &c) in genp.iter().enumerate() {
                next[i] ^= self.mul(c, root);
                next[i + 1] ^= c;
            }
            genp = next;
        }
        // Long division, highest power first.
        let mut reg = [0u8; PARITY];
        for &byte in msg {
            let feedback = byte ^ reg[0];
            for i in 0..PARITY - 1 {
                reg[i] = reg[i + 1] ^ self.mul(feedback, genp[PARITY - 1 - i]);
            }
            reg[PARITY - 1] = self.mul(feedback, genp[0]);
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codeword(rs: &Rs, seed: u32) -> Vec<u8> {
        let mut x = seed;
        let mut cw: Vec<u8> = (0..BLOCK - PARITY)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                (x >> 24) as u8
            })
            .collect();
        let p = rs.parity(&cw);
        cw.extend_from_slice(&p);
        cw
    }

    #[test]
    fn a_clean_codeword_has_no_syndrome() {
        let rs = Rs::new();
        let mut cw = codeword(&rs, 1);
        assert_eq!(rs.correct(&mut cw), Some(0));
    }

    #[test]
    fn up_to_sixteen_wrong_bytes_are_repaired() {
        let rs = Rs::new();
        for (seed, n) in [(2u32, 1usize), (3, 5), (4, 16)] {
            let good = codeword(&rs, seed);
            let mut bad = good.clone();
            for k in 0..n {
                let at = (k * 37 + 11) % BLOCK;
                bad[at] ^= 0x5A ^ (k as u8).wrapping_mul(3) | 1;
            }
            assert_eq!(rs.correct(&mut bad), Some(n), "{n} errors");
            assert_eq!(bad, good);
        }
    }

    #[test]
    fn errors_in_the_parity_are_repaired_too() {
        let rs = Rs::new();
        let good = codeword(&rs, 9);
        let mut bad = good.clone();
        bad[254] ^= 0xFF;
        bad[230] ^= 0x01;
        bad[0] ^= 0x80;
        assert_eq!(rs.correct(&mut bad), Some(3));
        assert_eq!(bad, good);
    }

    #[test]
    fn too_many_errors_are_refused() {
        let rs = Rs::new();
        let good = codeword(&rs, 5);
        let mut bad = good.clone();
        for k in 0..40 {
            bad[(k * 6 + 3) % BLOCK] ^= 0x33 | 1;
        }
        let before = bad.clone();
        assert_eq!(rs.correct(&mut bad), None);
        assert_eq!(bad, before, "a failed repair leaves the block alone");
    }
}
