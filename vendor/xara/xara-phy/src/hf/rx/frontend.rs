use crate::dsp::{Track, kaiser_lowpass};
use crate::ofdm::FS;
use num_complex::Complex64;
use std::f64::consts::PI;

pub struct FrontEnd {
    h1: Vec<f64>,
    h2: Vec<f64>,
    n_in: i64,
    z: Track<Complex64>,
    stage1_next: i64,
    u: Track<Complex64>,
    out_next: i64,
    mix: [Complex64; 32],
}

const N1: usize = 33;
const N2: usize = 273;
const D1: i64 = (N1 as i64 - 1) / 2;
const D2: i64 = (N2 as i64 - 1) / 2;

impl Default for FrontEnd {
    fn default() -> Self {
        Self::new()
    }
}

impl FrontEnd {
    pub fn new() -> Self {
        Self {
            h1: kaiser_lowpass(N1, 6000.0, FS, 5.65),
            h2: kaiser_lowpass(N2, 1520.0, FS / 4.0, 5.65),
            n_in: 0,
            z: Track::new(8192),
            stage1_next: 0,
            u: Track::new(8192),
            out_next: 0,
            mix: std::array::from_fn(|n| Complex64::from_polar(1.0, -PI * n as f64 / 16.0)),
        }
    }

    pub fn process(&mut self, x: &[f64]) -> (i64, Vec<Complex64>) {
        let n0 = self.n_in;
        self.n_in += x.len() as i64;
        let mix = self.mix;
        let z = x.iter().enumerate().map(|(i, &v)| mix[((n0 + i as i64) % 32) as usize] * v);
        let end = self.z.end();
        self.z.append(end, z);
        let k_hi = (self.n_in - 1 - D1).div_euclid(4);
        if k_hi >= self.stage1_next {
            let u: Vec<Complex64> = (self.stage1_next..=k_hi).map(|k| fir(&self.z, &self.h1, 4 * k - D1)).collect();
            self.u.append(self.stage1_next, u);
            self.stage1_next = k_hi + 1;
        }
        let m_hi = (self.stage1_next - 1 - D2).div_euclid(4);
        let m0 = self.out_next;
        if m_hi < m0 {
            return (m0, Vec::new());
        }
        let b3 =
            (m0..=m_hi).map(|m| fir(&self.u, &self.h2, 4 * m - D2) * if m % 2 == 1 { -2.0 } else { 2.0 }).collect();
        self.out_next = m_hi + 1;
        (m0, b3)
    }
}

fn fir(t: &Track<Complex64>, h: &[f64], start: i64) -> Complex64 {
    let mut acc = Complex64::default();
    for (i, &c) in h.iter().enumerate() {
        let n = start + i as i64;
        if n >= t.base {
            acc += t.buf[(n - t.base) as usize] * c;
        }
    }
    acc
}
