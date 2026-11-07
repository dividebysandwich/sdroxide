use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use std::f64::consts::PI;
use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug)]
pub struct Track<T> {
    pub base: i64,
    pub buf: Vec<T>,
    keep: usize,
}

impl<T: Clone + Default> Track<T> {
    pub fn new(keep: usize) -> Self {
        Self { base: 0, buf: Vec::new(), keep }
    }

    pub fn end(&self) -> i64 {
        self.base + self.buf.len() as i64
    }

    pub fn restart(&mut self, base: i64) {
        self.base = base;
        self.buf.clear();
    }

    pub fn append(&mut self, m0: i64, v: impl IntoIterator<Item = T>) {
        if m0 != self.end() {
            self.restart(m0);
        }
        self.buf.extend(v);
        if self.buf.len() > 2 * self.keep {
            let drop = self.buf.len() - self.keep;
            self.buf.drain(..drop);
            self.base += drop as i64;
        }
    }

    pub fn at(&self, i: i64) -> &T {
        &self.buf[(i - self.base) as usize]
    }

    pub fn get(&self, a: i64, b: i64) -> Vec<T> {
        let mut v = Vec::new();
        self.fill(a, b, &mut v);
        v
    }

    pub fn fill(&self, a: i64, b: i64, out: &mut Vec<T>) {
        out.clear();
        out.extend((a..b.max(a)).map(|i| {
            if i >= self.base && i < self.end() { self.buf[(i - self.base) as usize].clone() } else { T::default() }
        }));
    }
}

pub struct Window {
    fft: Arc<dyn Fft<f64>>,
    buf: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

impl Window {
    pub fn new(n: usize) -> Self {
        let fft = FftPlanner::new().plan_fft_forward(n);
        let scratch = vec![Complex64::default(); fft.get_inplace_scratch_len()];
        Self { fft, buf: Vec::with_capacity(n), scratch }
    }

    pub fn at(&mut self, track: &Track<Complex64>, a: i64, len: usize) -> &[Complex64] {
        track.fill(a, a + len as i64, &mut self.buf);
        self.buf.resize(self.fft.len(), Complex64::default());
        self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
        &self.buf
    }
}

pub const CHI_MIN: f64 = 20.0;

pub fn quantise(x: f64) -> f64 {
    const ULP_ONE: f64 = 4_503_599_627_370_496.0;
    let y = x.clamp(-1.0, 1.0) * 32767.0;
    ((y.abs() + ULP_ONE) - ULP_ONE).copysign(y) / 32767.0
}

pub(crate) fn sine(k: u32, period: usize) -> &'static [f64] {
    static TABLES: [[OnceLock<Vec<f64>>; 128]; 3] = [const { [const { OnceLock::new() }; 128] }; 3];
    let p = match period {
        512 => 0,
        1024 => 1,
        2048 => 2,
        _ => panic!("no sine table for period {period}"),
    };
    TABLES[p][k as usize]
        .get_or_init(|| (0..=period).map(|n| (2.0 * PI * f64::from(k) * n as f64 / period as f64).sin()).collect())
}

pub fn db_amp(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

pub struct Hold<X: Copy> {
    hold: i64,
    dead: i64,
    best: Option<(i64, f64, X)>,
    block: i64,
}

impl<X: Copy> Hold<X> {
    pub fn new(hold: i64, dead: i64) -> Self {
        Self { hold, dead, best: None, block: i64::MIN / 2 }
    }

    fn push(&mut self, pos: i64, metric: f64, extra: X) -> Option<(i64, f64, X)> {
        let out = self.flush_before(pos);
        if out.is_none() && pos >= self.block && self.best.is_none_or(|b| metric > b.1) {
            self.best = Some((pos, metric, extra));
        }
        out
    }

    fn flush_before(&mut self, pos: i64) -> Option<(i64, f64, X)> {
        self.best.take_if(|b| pos - b.0 > self.hold)
    }

    pub fn retain(&mut self, f: impl FnOnce(&mut X) -> bool) {
        self.best.take_if(|b| !f(&mut b.2));
    }

    pub fn offer(&mut self, pos: i64, candidate: Option<(f64, X)>) -> Option<(i64, f64, X)> {
        let out = match candidate {
            Some((metric, extra)) => self.push(pos, metric, extra),
            None => self.flush_before(pos),
        };
        if let Some(b) = out {
            self.block = b.0 + self.dead;
        }
        out
    }
}

pub fn median(v: &[f64]) -> f64 {
    let mut a = v.to_vec();
    a.sort_by(f64::total_cmp);
    let n = a.len();
    assert!(n > 0, "median of nothing");
    if n % 2 == 1 { a[n / 2] } else { 0.5 * (a[n / 2 - 1] + a[n / 2]) }
}

pub fn peak_mid(vals: &[f64], i: usize, rel: f64) -> usize {
    let tol = if vals.len() > 2 { rel * (vals[i] - median(vals)) } else { 0.0 };
    let (mut j0, mut j1) = (i, i);
    while j0 > 0 && vals[j0 - 1] >= vals[i] - tol {
        j0 -= 1;
    }
    while j1 + 1 < vals.len() && vals[j1 + 1] >= vals[i] - tol {
        j1 += 1;
    }
    (j0 + j1) / 2
}

pub fn unit(z: Complex64) -> Complex64 {
    z / (z.norm() + 1e-30)
}

pub fn argmax(v: &[f64]) -> usize {
    let mut b = 0;
    for (i, &x) in v.iter().enumerate() {
        if x > v[b] {
            b = i;
        }
    }
    b
}

pub fn thin<T: Copy>(v: Vec<T>, max: usize) -> Vec<T> {
    if v.len() <= max {
        return v;
    }
    (0..max).map(|i| v[i * v.len() / max]).collect()
}

fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term, mut k) = (1.0, 1.0, 1.0);
    while term > 1e-17 * sum {
        term *= (x / (2.0 * k)).powi(2);
        sum += term;
        k += 1.0;
    }
    sum
}

pub fn kaiser_lowpass(n: usize, cutoff: f64, fs: f64, beta: f64) -> Vec<f64> {
    let fc = cutoff / (fs / 2.0);
    let mid = (n - 1) as f64 / 2.0;
    let h: Vec<f64> = (0..n)
        .map(|i| {
            let m = i as f64 - mid;
            let sinc = if m == 0.0 { 1.0 } else { (PI * fc * m).sin() / (PI * fc * m) };
            let r = 2.0 * i as f64 / (n - 1) as f64 - 1.0;
            fc * sinc * bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / bessel_i0(beta)
        })
        .collect();
    let s: f64 = h.iter().sum();
    h.iter().map(|v| v / s).collect()
}

pub(crate) fn par_map<T: Sync, R: Send + Default>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let mut out: Vec<R> = items.iter().map(|_| R::default()).collect();
    let per = items.len().div_ceil(cores()).max(1);
    let f = &f;
    std::thread::scope(|s| {
        let mut parts = out.chunks_mut(per).zip(items.chunks(per));
        let first = parts.next();
        for (o, x) in parts {
            s.spawn(move || o.iter_mut().zip(x).for_each(|(o, x)| *o = f(x)));
        }
        if let Some((o, x)) = first {
            o.iter_mut().zip(x).for_each(|(o, x)| *o = f(x));
        }
    });
    out
}

pub(crate) fn cores() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantise_matches_round_ties_even() {
        let edges = [0.0, -0.0, 1e-300, -1e-300, 1.0, -1.0, 3.0, -3.0, f64::MAX, f64::MIN];
        let grid = (-140_000..=140_000)
            .flat_map(|i| [0.0, 1e-9, 0.25, 0.499_999_999, 0.5, 0.75].map(|f| (f64::from(i) + f) / 65_534.0));
        for x in edges.into_iter().chain(grid).flat_map(|x| [x, -x]) {
            let want = (x.clamp(-1.0, 1.0) * 32767.0).round_ties_even() / 32767.0;
            assert_eq!(quantise(x).to_bits(), want.to_bits(), "{x}");
        }
    }
}
