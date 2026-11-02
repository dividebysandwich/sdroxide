use num_complex::Complex64;
use std::sync::{Arc, Mutex};

const CAND: usize = 4;

pub struct Demapper {
    cst: Vec<Complex64>,
    bps: u32,
    cand: Vec<[u8; CAND]>,
}

impl Demapper {
    fn new(cst: &[Complex64]) -> Self {
        let bps = cst.len().trailing_zeros();
        let mut cand = Vec::with_capacity(cst.len() * bps as usize);
        for h in 0..cst.len() {
            for b in 0..bps {
                let sh = bps - 1 - b;
                let mut c: Vec<usize> = (0..cst.len()).filter(|&j| ((j ^ h) >> sh) & 1 == 1).collect();
                c.sort_by(|&x, &y| (cst[x] - cst[h]).norm_sqr().total_cmp(&(cst[y] - cst[h]).norm_sqr()));
                cand.push(std::array::from_fn(|i| c[i.min(c.len() - 1)] as u8));
            }
        }
        Self { cst: cst.to_vec(), bps, cand }
    }

    pub fn nearest(&self, z: Complex64) -> (usize, f64) {
        let mut best = (0, f64::INFINITY);
        for (i, &c) in self.cst.iter().enumerate() {
            let d = (z - c).norm_sqr();
            if d < best.1 {
                best = (i, d);
            }
        }
        best
    }

    pub fn soft_bits(&self, z: &[Complex64], nvar: f64, w: Option<&[f64]>, out: &mut Vec<f64>) {
        let bps = self.bps as usize;
        for (i, &zi) in z.iter().enumerate() {
            let (h, dh) = self.nearest(zi);
            let s = w.map_or(1.0, |w| w[i]) / nvar;
            for (b, cand) in self.cand[h * bps..(h + 1) * bps].iter().enumerate() {
                let dalt = cand.iter().map(|&j| (zi - self.cst[j as usize]).norm_sqr()).fold(f64::INFINITY, f64::min);
                let v = (dalt - dh) * s;
                out.push(if (h >> (bps - 1 - b)) & 1 == 0 { v } else { -v });
            }
        }
    }
}

pub fn demapper(cst: &[Complex64]) -> Arc<Demapper> {
    static CACHE: Mutex<Vec<Arc<Demapper>>> = Mutex::new(Vec::new());
    let mut c = CACHE.lock().unwrap();
    if let Some(d) = c.iter().find(|d| d.cst == cst) {
        return d.clone();
    }
    let d = Arc::new(Demapper::new(cst));
    c.push(d.clone());
    d
}

pub fn soft_bits(z: &[Complex64], cst: &[Complex64], nvar: f64, out: &mut Vec<f64>) {
    if cst.len() > 2 * CAND {
        return demapper(cst).soft_bits(z, nvar, None, out);
    }
    let bps = cst.len().trailing_zeros();
    let mut d = vec![0.0; cst.len()];
    for &zi in z {
        for (dv, &c) in d.iter_mut().zip(cst) {
            *dv = -(zi - c).norm_sqr() / nvar;
        }
        energy_llrs(&d, bps, out);
    }
}

pub fn energy_llrs(m: &[f64], bps: u32, out: &mut Vec<f64>) {
    for k in 0..bps {
        let sh = bps - 1 - k;
        let (mut m0, mut m1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (idx, &v) in m.iter().enumerate() {
            if (idx >> sh) & 1 == 0 {
                m0 = m0.max(v);
            } else {
                m1 = m1.max(v);
            }
        }
        out.push(m0 - m1);
    }
}
