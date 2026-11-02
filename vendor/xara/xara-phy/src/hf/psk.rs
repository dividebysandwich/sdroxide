use crate::constellation::{constellation, take_bits};
use crate::dat::marker_map;
use crate::dsp::par_map;
use crate::hf::level::{Level, Pulse, Waveform};
use num_complex::Complex64;
use std::f32::consts::TAU;
use std::f64::consts::PI;

pub const SPS: usize = 32;
pub const SCALE: f64 = 178.0;

fn f32r(x: f64) -> f64 {
    x as f32 as f64
}

pub fn pulse_taps(p: Pulse) -> Vec<f64> {
    match p {
        Pulse::Rc => {
            let t = (SPS / 2) as f64;
            (-16..=16)
                .map(|n: i32| {
                    let n = f64::from(n);
                    f32r(if n == 0.0 {
                        1.0
                    } else if n.abs() == t / 2.0 {
                        0.5 * (PI / 2.0).sin()
                    } else {
                        let x = n / t;
                        (PI * x).sin() * (PI * x).cos() / (x * PI * (1.0 - 4.0 * x * x))
                    })
                })
                .collect()
        }
        Pulse::Rrc { span } => {
            let t = SPS as f64;
            let half = (t * f64::from(span)).round() as i32;
            (-half..=half)
                .map(|n| {
                    let n = f64::from(n);
                    f32r(if n == 0.0 {
                        4.0 / PI
                    } else if n.abs() == t / 4.0 {
                        1.0 / 2f64.sqrt() * ((1.0 + 2.0 / PI) * (PI / 4.0).sin() + (1.0 - 2.0 / PI) * (PI / 4.0).cos())
                    } else {
                        let x = n / t;
                        4.0 * x * (PI * x * 2.0).cos() / (PI * x * (1.0 - (4.0 * x).powi(2)))
                    })
                })
                .collect()
        }
    }
}

pub fn tx_pulse_taps(p: Pulse) -> Vec<f64> {
    let mut h = pulse_taps(p);
    *h.last_mut().unwrap() = 0.0;
    h
}

pub fn interp16_taps() -> [f64; 32] {
    let f2 = 125.0 / 48000.0;
    std::array::from_fn(|i| {
        let m = i as f64 - 16.0;
        let h = if m == 0.0 { 2.0 * f2 } else { (2.0 * PI * f2 * m).sin() / (PI * m) };
        let w = f32r(0.54 - 0.46 * (2.0 * PI * i as f64 / 32.0).cos());
        f32r(h * w)
    })
}

pub const fn stagger(c: usize, nc: usize) -> usize {
    if nc == 3 {
        [0, 11, 21][c]
    } else if c % 2 == 1 {
        16
    } else {
        0
    }
}

pub fn nco_phase(fc: u32, n: usize) -> Vec<f32> {
    let step = (2.0 * PI * f64::from(fc) / 48000.0) as f32;
    let mut phi = 0.0f32;
    (0..n)
        .map(|_| {
            phi += step;
            if phi > TAU {
                phi -= TAU;
            }
            phi
        })
        .collect()
}

fn scatter(x: &[Complex64], stride: usize, offset: usize, h: &[f64]) -> Vec<Complex64> {
    let mut y = vec![Complex64::default(); (x.len() - 1) * stride + offset + 1 + h.len() - 1];
    for (i, &v) in x.iter().enumerate() {
        if v == Complex64::default() {
            continue;
        }
        let base = offset + i * stride;
        for (j, &t) in h.iter().enumerate() {
            y[base + j] += v * t;
        }
    }
    y
}

fn carrier_audio(symbols: &[Complex64], fc: u32, delay: usize, hp: &[f64], hi: &[f64]) -> Vec<f64> {
    let len3 = symbols.len() * SPS + delay;
    let mut y3 = scatter(symbols, SPS, delay, hp);
    y3.resize(len3 + hp.len() - 1, Complex64::default());
    let n = y3.len() * 16 + hi.len() - 1;
    nco_phase(fc, n)
        .into_iter()
        .enumerate()
        .map(|(t, p)| {
            let mut v = Complex64::default();
            for i in (t + 1).saturating_sub(hi.len()).div_ceil(16)..=(t / 16).min(y3.len() - 1) {
                if y3[i] != Complex64::default() {
                    v += y3[i] * hi[t - 16 * i];
                }
            }
            v.re * f64::from(p).cos() - v.im * f64::from(p).sin()
        })
        .collect()
}

pub fn symbol_map(l: &Level, other: &[u16]) -> Vec<Vec<bool>> {
    marker_map(other, l.nc() as usize, l.markers() as usize, l.spf() as usize - 1)
}

pub fn modulate(
    l: &Level,
    other: &[u16],
    phasors: &[Vec<Complex64>],
    txs: &[Vec<u8>],
    rand: &mut dyn FnMut(u32) -> u32,
) -> Vec<f64> {
    let Waveform::Psk { carriers_hz, pulse, .. } = l.waveform else { panic!("not a PSK level") };
    let cst = constellation(l.bps);
    let m = cst.len() as u32;
    let nsym = l.data_symbols() as usize;
    let map = symbol_map(l, other);
    let (hp, hi) = (tx_pulse_taps(pulse), interp16_taps());
    let nc = carriers_hz.len();
    let refs: Vec<Vec<Complex64>> = txs.iter().map(|_| (0..nc).map(|_| cst[rand(m) as usize]).collect()).collect();
    let lead: Vec<Vec<Complex64>> =
        (0..l.lead_in()).map(|_| (0..nc).map(|_| cst[rand(m) as usize]).collect()).collect();
    let mut ptrs = vec![0usize; txs.len()];
    let xs: Vec<(usize, Vec<Complex64>)> = (0..nc)
        .map(|c| {
            let mut x: Vec<Complex64> =
                lead.iter().enumerate().map(|(k, v)| v[c] * phasors[c][k].conj() * SCALE).collect();
            for ((tx, ptr), d0) in txs.iter().zip(ptrs.iter_mut()).zip(&refs) {
                let mut d = d0[c];
                x.push(d * phasors[c][0].conj() * SCALE);
                for s in 1..nsym {
                    if !(s >= 2 && map[c][s - 2]) {
                        d *= cst[take_bits(tx, ptr, l.bps)];
                    }
                    x.push(d * phasors[c][s].conj() * SCALE);
                }
            }
            x.push(Complex64::default());
            (c, x)
        })
        .collect();
    let sigs = par_map(&xs, |(c, x)| carrier_audio(x, carriers_hz[*c], stagger(*c, nc), &hp, &hi));
    let mut total: Vec<f64> = Vec::new();
    for sig in sigs {
        if sig.len() > total.len() {
            total.resize(sig.len(), 0.0);
        }
        for (t, s) in total.iter_mut().zip(sig) {
            *t += s;
        }
    }
    assert!(ptrs.iter().all(|&p| p == l.n() as usize));
    total
}
