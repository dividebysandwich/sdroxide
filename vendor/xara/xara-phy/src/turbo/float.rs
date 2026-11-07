use super::engine::Kernel;
use super::{NEXT, PAR};
use std::borrow::Cow;

const NEG: f64 = -1e9;

const fn gi(s: usize, u: usize) -> usize {
    (u << 1) | PAR[s][u] as usize
}

type Edges = [[(usize, usize); 2]; 8];

const fn tables() -> ([[usize; 2]; 8], Edges) {
    let mut gis = [[0; 2]; 8];
    let mut edge = [[(0, 0); 2]; 8];
    let mut s = 0;
    while s < 8 {
        gis[s] = [gi(s, 0), gi(s, 1)];
        let mut u = 0;
        while u < 2 {
            let n = NEXT[s][u];
            edge[n][s >> 2] = (s, gi(s, u));
            u += 1;
        }
        s += 1;
    }
    (gis, edge)
}

const TABLES: ([[usize; 2]; 8], Edges) = tables();
const GI: [[usize; 2]; 8] = TABLES.0;
const EDGE: Edges = TABLES.1;

#[inline(always)]
fn mx(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

#[inline(always)]
fn gam(x: f64, p: f64) -> [f64; 4] {
    let (a, b) = (0.5 * (x + p), 0.5 * (x - p));
    [a, b, -b, -a]
}

pub(crate) struct Float {
    pub(crate) scale: f64,
}

impl Kernel for Float {
    type V = f64;
    type S = [f64; 8];
    type C = [f64; 8];

    fn term(&self) -> [f64; 8] {
        [0.0, NEG, NEG, NEG, NEG, NEG, NEG, NEG]
    }

    fn zero(&self) -> [f64; 8] {
        [0.0; 8]
    }

    fn cell(&self) -> [f64; 8] {
        [0.0; 8]
    }

    fn quant<'a>(&self, v: &'a [f64]) -> Cow<'a, [f64]> {
        Cow::Borrowed(v)
    }

    fn add(&self, a: f64, b: f64) -> f64 {
        a + b
    }

    fn alpha(&self, x: &[f64], p: &[f64], mut a: [f64; 8], cells: &mut [[f64; 8]]) -> [f64; 8] {
        for (t, c) in cells.iter_mut().enumerate() {
            *c = a;
            let g = gam(x[t], p[t]);
            let n: [f64; 8] = std::array::from_fn(|d| {
                let [(s0, g0), (s1, g1)] = EDGE[d];
                mx(a[s0] + g[g0], a[s1] + g[g1])
            });
            a = n.map(|v| v - n[0]);
        }
        a
    }

    fn beta(&self, x: &[f64], p: &[f64], mut b: [f64; 8], cells: &[[f64; 8]], out: &mut [f64]) -> [f64; 8] {
        for t in (0..cells.len()).rev() {
            let g = gam(x[t], p[t]);
            let bs: [[f64; 2]; 8] = std::array::from_fn(|s| [b[NEXT[s][0]] + g[GI[s][0]], b[NEXT[s][1]] + g[GI[s][1]]]);
            if t < out.len() {
                let a = &cells[t];
                let (mut m0, mut m1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
                for s in 0..8 {
                    m0 = mx(m0, a[s] + bs[s][0]);
                    m1 = mx(m1, a[s] + bs[s][1]);
                }
                out[t] = self.scale * ((m0 - m1) - x[t]);
            }
            let n: [f64; 8] = std::array::from_fn(|s| mx(bs[s][0], bs[s][1]));
            b = n.map(|v| v - n[0]);
        }
        b
    }
}
