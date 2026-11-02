use super::Split;
use super::engine::Kernel;
use super::simd::Lanes;
use std::borrow::Cow;
use std::marker::PhantomData;

const NEG: i16 = -8192;
const TARGET: f64 = 24.0;
const CLAMP: i64 = 127;

pub(crate) struct Fixed<B> {
    sc: f64,
    es: i32,
    _b: PhantomData<B>,
}

impl<B> Fixed<B> {
    pub fn new(sp: &Split, scale: f64) -> Self {
        let n = sp.ls.len() + sp.lp1.len() + sp.lp2.len();
        let sum: f64 = sp.ls.iter().chain(&sp.lp1).chain(&sp.lp2).map(|v| v.abs()).sum();
        let mean = sum / n.max(1) as f64 + 1e-12;
        Self { sc: TARGET / mean, es: (scale * 64.0).round() as i32, _b: PhantomData }
    }

    #[inline(always)]
    fn ext(&self, d: i32, x: i16) -> i16 {
        (((i64::from(d) - 2 * i64::from(x)) * i64::from(self.es)) >> 7).clamp(-CLAMP, CLAMP) as i16
    }
}

impl<B: Lanes + Sync> Kernel for Fixed<B> {
    type V = i16;
    type S = [i16; 8];
    type C = (B, B);

    fn term(&self) -> [i16; 8] {
        [0, NEG, NEG, NEG, NEG, NEG, NEG, NEG]
    }

    fn zero(&self) -> [i16; 8] {
        [0; 8]
    }

    fn cell(&self) -> (B, B) {
        (B::load([0; 8]), B::load([0; 8]))
    }

    fn quant<'a>(&self, v: &'a [f64]) -> Cow<'a, [i16]> {
        v.iter().map(|&x| (x * self.sc).round().clamp(-127.0, 127.0) as i16).collect()
    }

    fn add(&self, a: i16, b: i16) -> i16 {
        a + b
    }

    fn alpha(&self, x: &[i16], p: &[i16], a: [i16; 8], cells: &mut [(B, B)]) -> [i16; 8] {
        let mut a = B::load(a);
        for (t, c) in cells.iter_mut().enumerate() {
            let g = B::gamma(x[t] + p[t], x[t] - p[t]);
            *c = (a, g);
            a = a.zip_lo().adds(g).max(a.zip_hi().subs(g)).norm();
        }
        a.store()
    }

    fn beta(&self, x: &[i16], _p: &[i16], b: [i16; 8], cells: &[(B, B)], out: &mut [i16]) -> [i16; 8] {
        let mut b = B::load(b);
        for t in (out.len()..cells.len()).rev() {
            let g = cells[t].1;
            b = b.adds(g).pairmax(b.subs(g)).norm();
        }
        for t in (0..out.len()).rev() {
            let (a, g) = cells[t];
            let (s0, s1) = (b.adds(g), b.subs(g));
            out[t] = self.ext(B::llr(a.zip_lo().adds(s0), a.zip_hi().adds(s1)), x[t]);
            b = s0.pairmax(s1).norm();
        }
        b.store()
    }
}

#[cfg(test)]
mod tests {
    use super::super::simd::{Native, SIGN_A, SIGN_B, Scalar, U1};
    use super::super::{CrcCheck, NEXT, PAR, Split, engine};
    use super::*;

    #[test]
    fn lane_tables_match_trellis() {
        for s in 0..8 {
            for u in 0..2 {
                let (n, r0) = (NEXT[s][u], s >> 2);
                assert_eq!(s, (r0 << 2) | (n >> 1));
                let sign = if r0 == 0 { 1 } else { -1 };
                let (x, p) = (1 - 2 * u as i16, 1 - 2 * PAR[s][u] as i16);
                let g = |a: i16, b: i16| sign * (SIGN_A[n] * a + SIGN_B[n] * b);
                assert_eq!(g(3 + 5, 3 - 5), x * 3 + p * 5, "s {s} u {u}");
                assert_eq!(U1[n] ^ (r0 == 1), u == 1);
            }
        }
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn gauss(&mut self) -> f64 {
            let u1 = ((self.next() >> 11) as f64 + 1.0) / (1u64 << 53) as f64;
            let u2 = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
            (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
        }
    }

    #[test]
    fn native_matches_scalar() {
        let mut r = Rng(7);
        for (k, amp) in [(500, 0.5), (5000, 2.0), (20000, 6.0), (20000, 40.0)] {
            let mut perm: Vec<u32> = (0..k as u32).collect();
            for i in (1..k).rev() {
                perm.swap(i, (r.next() % (i as u64 + 1)) as usize);
            }
            let mut v = || (0..k + 3).map(|_| amp + 2.0 * r.gauss()).collect::<Vec<f64>>();
            let sp = Split { ls: v(), lp1: v(), ls2: v(), lp2: v() };
            let white = vec![0u8; k];
            let check = CrcCheck { white: &white, nbytes: k / 8 };
            let a = engine::run(&Fixed::<Native>::new(&sp, 0.7), &sp, &perm, 6, Some((2, &check)));
            let b = engine::run(&Fixed::<Scalar>::new(&sp, 0.7), &sp, &perm, 6, Some((2, &check)));
            assert_eq!(a.bits, b.bits, "k {k}");
            assert_eq!(a.iterations, b.iterations, "k {k}");
        }
    }
}
