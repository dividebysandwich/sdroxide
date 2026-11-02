use crate::demap::demapper;
use num_complex::Complex64;

pub(crate) fn diff(c: &[Vec<Complex64>], rows: usize, nb: usize, rot: Complex64) -> Vec<Vec<Complex64>> {
    (1..rows).map(|r| (0..nb).map(|b| c[r][b] * c[r - 1][b].conj() * rot).collect()).collect()
}

pub(crate) fn dpsk_nvar(d: &mut [Vec<Complex64>], eps: f64, marks: impl Iterator<Item = (usize, usize)>) -> f64 {
    let n: usize = d.iter().map(Vec::len).sum();
    let g = d.iter().flatten().map(|v| v.norm()).sum::<f64>() / n as f64 + eps;
    for v in d.iter_mut().flatten() {
        *v /= g;
    }
    let mk: Vec<f64> = marks.map(|(i, j)| (d[i][j] - d[i][j].norm()).norm_sqr()).collect();
    (mk.iter().sum::<f64>() / mk.len() as f64).max(1e-3)
}

pub(crate) fn derotate(c: &mut [Vec<Complex64>], thd: f64) {
    for (row, cr) in c.iter_mut().enumerate() {
        let r = Complex64::from_polar(1.0, -thd * row as f64);
        for v in cr.iter_mut() {
            *v *= r;
        }
    }
}

#[derive(Default)]
pub(crate) struct Eq {
    zs: Vec<Complex64>,
    ws: Vec<f64>,
}

impl Eq {
    pub fn push(&mut self, y: Complex64, x: f64, xs: &[usize], ys: &[Complex64]) {
        let h = interp(x, xs, ys);
        let z = y / h;
        self.zs.push(if z.re.is_finite() && z.im.is_finite() { z } else { Complex64::default() });
        self.ws.push(h.norm_sqr());
    }

    pub fn llrs(&self, cst: &[Complex64], floor: f64, llr: &mut Vec<f64>) -> f64 {
        let mw = self.ws.iter().sum::<f64>() / self.ws.len().max(1) as f64 + 1e-30;
        let wn: Vec<f64> = self.ws.iter().map(|w| w / mw).collect();
        let dm = demapper(cst);
        let err: f64 = self.zs.iter().zip(&wn).map(|(&z, &w)| w * dm.nearest(z).1).sum();
        let nvar = (err / self.zs.len().max(1) as f64).max(floor);
        dm.soft_bits(&self.zs, nvar, Some(&wn), llr);
        nvar
    }
}

fn interp(x: f64, xs: &[usize], ys: &[Complex64]) -> Complex64 {
    if xs.is_empty() {
        return Complex64::new(1.0, 0.0);
    }
    if x <= xs[0] as f64 {
        return ys[0];
    }
    if x >= xs[xs.len() - 1] as f64 {
        return ys[ys.len() - 1];
    }
    let i = xs.partition_point(|&v| (v as f64) <= x) - 1;
    let (x0, x1) = (xs[i] as f64, xs[i + 1] as f64);
    let f = (x - x0) / (x1 - x0);
    ys[i] * (1.0 - f) + ys[i + 1] * f
}
