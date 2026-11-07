#![allow(clippy::needless_range_loop)]

use crate::crc::crc_ok;

mod engine;
mod fixed;
mod float;
mod simd;

const fn trellis() -> ([[usize; 2]; 8], [[bool; 2]; 8]) {
    let mut next = [[0; 2]; 8];
    let mut par = [[false; 2]; 8];
    let mut s = 0;
    while s < 8 {
        let (r0, r1, r2) = ((s >> 2) & 1, (s >> 1) & 1, s & 1);
        let mut u = 0;
        while u < 2 {
            let a = u ^ r0 ^ r1;
            par[s][u] = a ^ r2 ^ r0 == 1;
            next[s][u] = (r1 << 2) | (r2 << 1) | a;
            u += 1;
        }
        s += 1;
    }
    (next, par)
}

const TRELLIS: ([[usize; 2]; 8], [[bool; 2]; 8]) = trellis();
pub(crate) const NEXT: [[usize; 2]; 8] = TRELLIS.0;
pub(crate) const PAR: [[bool; 2]; 8] = TRELLIS.1;

pub struct Split {
    pub ls: Vec<f64>,
    pub(crate) lp1: Vec<f64>,
    pub(crate) ls2: Vec<f64>,
    pub(crate) lp2: Vec<f64>,
}

pub fn used_len(k: usize, p: usize) -> usize {
    crate::fec::punctured_len(k as u32, p as u32) as usize
}

pub fn split(llr: &[f64], perm: &[u32], p: usize) -> Split {
    let k = perm.len();
    let t_len = k + 3;
    let mut s = Split { ls: vec![0.0; t_len], lp1: vec![0.0; t_len], ls2: vec![0.0; t_len], lp2: vec![0.0; t_len] };
    let mut it = llr.iter().map(|&v| if v.is_finite() { v } else { 0.0 });
    let mut next = || it.next().expect("too few LLRs");
    for t in 0..k {
        s.ls[t] = next();
        if p == 1 || t % p == 0 {
            s.lp1[t] = next();
        }
        if p == 1 || t % p == p / 2 {
            s.lp2[t] = next();
        }
    }
    for t in k..t_len {
        s.ls[t] = next();
        s.lp1[t] = next();
        s.ls2[t] = next();
        s.lp2[t] = next();
    }
    for (i, &j) in perm.iter().enumerate() {
        s.ls2[i] = s.ls[j as usize];
    }
    s
}

pub struct Decoded {
    pub bits: Vec<u8>,
    pub iterations: usize,
}

pub struct CrcCheck<'a> {
    pub white: &'a [u8],
    pub nbytes: usize,
}

impl CrcCheck<'_> {
    pub fn bytes(&self, bits: &[u8]) -> Vec<u8> {
        bits[..self.nbytes * 8]
            .chunks(8)
            .zip(self.white.chunks(8))
            .map(|(b, w)| b.iter().zip(w).fold(0u8, |v, (&x, &y)| (v << 1) | ((x ^ y) & 1)))
            .collect()
    }

    pub fn ok(&self, bits: &[u8]) -> bool {
        crc_ok(&self.bytes(bits))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Float,
    Fixed,
}

impl Kind {
    pub const DEFAULT: Kind = if cfg!(fixed_turbo) { Kind::Fixed } else { Kind::Float };
}

pub struct Decoder {
    pub kind: Kind,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new(Kind::DEFAULT)
    }
}

impl Decoder {
    pub fn new(kind: Kind) -> Self {
        Self { kind }
    }

    pub fn decode(
        &self,
        sp: &Split,
        perm: &[u32],
        iters: usize,
        scale: f64,
        early: Option<(usize, &CrcCheck)>,
    ) -> Decoded {
        match self.kind {
            Kind::Float => engine::run(&float::Float { scale }, sp, perm, iters, early),
            Kind::Fixed => engine::run(&fixed::Fixed::<simd::Native>::new(sp, scale), sp, perm, iters, early),
        }
    }
}
