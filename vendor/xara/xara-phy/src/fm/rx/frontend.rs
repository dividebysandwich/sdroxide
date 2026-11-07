use super::DEC;
use crate::dsp::{Track, kaiser_lowpass};
use crate::ofdm::FS;
use num_complex::Complex64;
use std::f64::consts::PI;

pub const TAPS: usize = 481;
const D: i64 = (TAPS as i64 - 1) / 2;

pub struct FrontEnd {
    h: Vec<Complex64>,
    x: Track<f64>,
    out_next: i64,
}

impl Default for FrontEnd {
    fn default() -> Self {
        Self::new()
    }
}

impl FrontEnd {
    pub fn new() -> Self {
        let lp = kaiser_lowpass(TAPS, 3000.0, FS, 6.76);
        let h = lp
            .iter()
            .enumerate()
            .map(|(i, &v)| v * Complex64::from_polar(1.0, 2.0 * PI * 3000.0 * (i as f64 - D as f64) / FS))
            .collect();
        Self { h, x: Track::new(4 * TAPS), out_next: 0 }
    }

    pub fn process(&mut self, x: &[f64]) -> (i64, Vec<Complex64>) {
        let end = self.x.end();
        self.x.append(end, x.iter().copied());
        let m0 = self.out_next;
        let m_hi = (self.x.end() - 1 - D).div_euclid(DEC);
        let out: Vec<Complex64> = (m0..=m_hi)
            .map(|m| {
                let c = DEC * m + D;
                let mut acc = Complex64::default();
                for (i, &h) in self.h.iter().enumerate() {
                    let n = c - i as i64;
                    if n >= self.x.base {
                        acc += h * self.x.buf[(n - self.x.base) as usize];
                    }
                }
                acc
            })
            .collect();
        self.out_next = m0 + out.len() as i64;
        (m0, out)
    }
}
