//! The QPSK demodulator for Meteor LRPT: 72 ksym/s, root-raised-cosine with a
//! roll-off of 0.6, on a carrier that moves with the satellite's Doppler.
//!
//! Three things have to be followed, and a real pass moves all three:
//!
//! * the **carrier**: a coarse frequency from the fourth power of the signal (a
//!   QPSK signal raised to the fourth power has a line at four times the
//!   carrier offset), refreshed every third of a second, then a Costas loop for
//!   the phase;
//! * the **symbol clock**: the recorder's or the receiver's clock is a few ppm
//!   from the satellite's, and the Doppler moves the symbol rate too, so the
//!   sampling instants drift by about a sample in ten seconds. The clock is read
//!   from the line at the symbol rate in `|y|^2` and a straight line is fitted
//!   through the last few seconds of it. A clock that does not follow this loses
//!   the signal;
//! * the **level**, kept at one.
//!
//! The lane runs at three samples per symbol. The output is one pair of soft
//! values per symbol, the carrier removed, with the constellation on the
//! diagonals: which of the eight ways round it is left to the decoder, which
//! tries them.

use std::collections::VecDeque;
use std::sync::Arc;

use rustfft::num_complex::Complex32 as FftC;
use rustfft::{Fft, FftPlanner};
use sdroxide_dsp::{Complex32, ComplexFir, Nco};

/// The lane rate: three samples per symbol.
pub const LRPT_LANE_RATE_HZ: f64 = 216_000.0;
pub const SYMBOL_RATE_HZ: f64 = 72_000.0;
const SPS: f64 = 3.0;
const ROLL_OFF: f64 = 0.6;
/// Root-raised-cosine span, in symbols each side.
const RRC_SPAN: usize = 6;

/// Samples per block of the clock estimator (1024 symbols).
const CLOCK_BLOCK: usize = 3 * 1024;
/// Blocks the clock's straight line is fitted through.
const CLOCK_WINDOW: usize = 128;
/// Samples the coarse carrier estimate looks at, and how often it is refreshed.
const COARSE_N: usize = 65_536;
const COARSE_STEP: usize = 32_768;
/// The coarse search runs over this many Hz either side of the NCO.
const COARSE_SEARCH_HZ: f64 = 6_000.0;
/// Peak over median of the fourth-power spectrum for a line to be believed.
const COARSE_CONFIDENCE: f32 = 6.0;

/// The root-raised-cosine pulse, `sps` samples per symbol, unit energy.
fn rrc_taps(beta: f64, sps: f64, span: usize) -> Vec<f32> {
    let n = (2.0 * span as f64 * sps) as isize;
    let mut h: Vec<f64> = (0..=n)
        .map(|i| {
            let t = (i - n / 2) as f64 / sps;
            let pi = std::f64::consts::PI;
            if t.abs() < 1e-9 {
                1.0 - beta + 4.0 * beta / pi
            } else if (t.abs() - 1.0 / (4.0 * beta)).abs() < 1e-9 {
                beta / 2f64.sqrt()
                    * ((1.0 + 2.0 / pi) * (pi / (4.0 * beta)).sin()
                        + (1.0 - 2.0 / pi) * (pi / (4.0 * beta)).cos())
            } else {
                ((pi * t * (1.0 - beta)).sin() + 4.0 * beta * t * (pi * t * (1.0 + beta)).cos())
                    / (pi * t * (1.0 - (4.0 * beta * t).powi(2)))
            }
        })
        .collect();
    let e: f64 = h.iter().map(|v| v * v).sum::<f64>().sqrt();
    h.iter_mut().for_each(|v| *v /= e);
    h.into_iter().map(|v| v as f32).collect()
}

/// Cubic (Catmull-Rom) value of `y` at fractional index `pos`.
fn cubic(y: &[Complex32], pos: f64) -> Complex32 {
    let i = pos.floor() as usize;
    let f = (pos - pos.floor()) as f32;
    let (y0, y1, y2, y3) = (y[i - 1], y[i], y[i + 1], y[i + 2]);
    let a = y0 * -0.5 + y1 * 1.5 - y2 * 1.5 + y3 * 0.5;
    let b = y0 - y1 * 2.5 + y2 * 2.0 - y3 * 0.5;
    let c = y0 * -0.5 + y2 * 0.5;
    ((a * f + b) * f + c) * f + y1
}

/// One soft symbol: carrier removed, constellation on the diagonals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Symbol {
    pub i: f32,
    pub q: f32,
}

pub struct QpskDemod {
    nco: Nco,
    /// Frequency the NCO removes, Hz.
    f_nco: f64,
    rrc: ComplexFir,
    /// Matched-filter output not yet turned into symbols, and the index in the
    /// whole stream of its first sample.
    y: Vec<Complex32>,
    y_start: usize,
    // coarse carrier
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    coarse: Vec<Complex32>,
    have_carrier: bool,
    /// The carrier was found by the call in progress.
    just_found: bool,
    // clock
    next_block: usize,
    taus: VecDeque<(f64, f64)>,
    /// The latest unwrapped clock phase, in samples.
    tau_last: Option<f64>,
    /// Index of the next symbol.
    m: i64,
    // Costas
    theta: f32,
    omega: f32,
    alpha: f32,
    beta: f32,
    amp: f32,
    /// Smoothed squared distance from the nearest diagonal point.
    evm2: f32,
}

impl Default for QpskDemod {
    fn default() -> Self {
        Self::new()
    }
}

impl QpskDemod {
    pub fn new() -> Self {
        let taps: Vec<Complex32> =
            rrc_taps(ROLL_OFF, SPS, RRC_SPAN).into_iter().map(|t| Complex32::new(t, 0.0)).collect();
        let fft = FftPlanner::new().plan_fft_forward(COARSE_N);
        let window = (0..COARSE_N)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / COARSE_N as f32).cos())
            .collect();
        Self {
            nco: Nco::new(0.0, LRPT_LANE_RATE_HZ),
            f_nco: 0.0,
            rrc: ComplexFir::new(taps),
            y: Vec::new(),
            y_start: 0,
            fft,
            window,
            coarse: Vec::new(),
            have_carrier: false,
            just_found: false,
            next_block: 0,
            taus: VecDeque::new(),
            tau_last: None,
            m: 0,
            theta: 0.0,
            omega: 0.0,
            alpha: 0.012,
            beta: 0.012 * 0.012 / 4.0,
            amp: 1.0,
            evm2: 1.0,
        }
    }

    /// The carrier offset being removed, Hz.
    pub fn carrier_hz(&self) -> f64 {
        self.f_nco
    }

    /// Whether a carrier has been found.
    pub fn has_carrier(&self) -> bool {
        self.have_carrier
    }

    /// Root-mean-square distance of the symbols from their nearest ideal point,
    /// for a unit-amplitude constellation: about 0.2 on a good pass, about 1 on
    /// noise.
    pub fn evm(&self) -> f32 {
        self.evm2.sqrt()
    }

    /// The fourth-power line, refreshed from the matched-filter output.
    fn coarse_update(&mut self) {
        let n = COARSE_N;
        let start = self.coarse.len() - n;
        let mut buf: Vec<FftC> = self.coarse[start..]
            .iter()
            .zip(self.window.iter())
            .map(|(&v, &w)| {
                let s = v * v;
                let p = s * s * w;
                FftC::new(p.re, p.im)
            })
            .collect();
        self.fft.process(&mut buf);
        let bin_hz = LRPT_LANE_RATE_HZ / n as f64;
        // The line sits at four times the offset.
        let span = ((4.0 * COARSE_SEARCH_HZ) / bin_hz) as isize;
        let idx = |k: isize| k.rem_euclid(n as isize) as usize;
        let mag: Vec<f32> = (-span..=span).map(|k| buf[idx(k)].norm()).collect();
        let (kpk, &pk) = mag.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
        let mut sorted = mag.clone();
        sorted.sort_by(f32::total_cmp);
        let median = sorted[sorted.len() / 2].max(1e-12);
        if pk / median < COARSE_CONFIDENCE {
            return;
        }
        // Parabolic interpolation on the three bins around the peak.
        let frac = if kpk > 0 && kpk + 1 < mag.len() {
            let (a, b, c) = (mag[kpk - 1].ln(), mag[kpk].ln(), mag[kpk + 1].ln());
            let d = a - 2.0 * b + c;
            if d.abs() > 1e-9 { 0.5 * (a - c) / d } else { 0.0 }
        } else {
            0.0
        };
        let f4 = (kpk as f64 - span as f64 + f64::from(frac)) * bin_hz;
        let residual = f4 / 4.0;
        self.f_nco += residual;
        self.nco.set_freq(-self.f_nco, LRPT_LANE_RATE_HZ);
        if !self.have_carrier {
            self.just_found = true;
        }
        self.have_carrier = true;
        // The samples held for the estimate were taken with the old frequency.
        self.coarse.clear();
    }

    /// The clock phase of one block, in samples, from the line at the symbol
    /// rate in `|y|^2`.
    fn block_tau(&self, from: usize) -> f64 {
        let w = [
            Complex32::new(1.0, 0.0),
            Complex32::new(-0.5, -0.866_025_4),
            Complex32::new(-0.5, 0.866_025_4),
        ];
        let mut acc = Complex32::new(0.0, 0.0);
        for (k, v) in self.y[from..from + CLOCK_BLOCK].iter().enumerate() {
            acc += w[(self.y_start + from + k) % 3] * v.norm_sqr();
        }
        -f64::from(acc.arg()) / (2.0 * std::f64::consts::PI) * SPS
    }

    /// The straight line through the recent clock phases: its value at `t0`
    /// and its slope (samples per sample).
    fn clock_fit(&self) -> Option<(f64, f64)> {
        let n = self.taus.len();
        if n < 8 {
            return None;
        }
        let t0 = self.taus.back().unwrap().0;
        let (mut st, mut sy, mut stt, mut sty) = (0.0, 0.0, 0.0, 0.0);
        for &(t, y) in &self.taus {
            let x = t - t0;
            st += x;
            sy += y;
            stt += x * x;
            sty += x * y;
        }
        let nf = n as f64;
        let den = nf * stt - st * st;
        if den.abs() < 1e-9 {
            return None;
        }
        let slope = (nf * sty - st * sy) / den;
        let c0 = (sy - slope * st) / nf;
        Some((c0, slope))
    }

    /// Feed lane-rate samples; soft symbols are appended to `out`.
    pub fn process(&mut self, iq: &[Complex32], out: &mut Vec<Symbol>) {
        let mut x = Vec::with_capacity(iq.len());
        self.nco.mix(iq, &mut x);
        let mut y = Vec::with_capacity(iq.len());
        self.rrc.process(&x, &mut y);
        self.coarse.extend_from_slice(&y);
        self.y.extend_from_slice(&y);
        if self.coarse.len() >= COARSE_N + COARSE_STEP {
            // Keep the newest window; the older part is spent.
            let drop = self.coarse.len() - COARSE_N;
            self.coarse.drain(..drop);
        }
        if self.coarse.len() >= COARSE_N {
            self.coarse_update();
        }
        if !self.have_carrier {
            // Nothing to measure the clock on yet: keep the stream moving.
            self.y_start += self.y.len();
            self.y.clear();
            return;
        }
        if self.just_found {
            // What was filtered before the first estimate was rotating at the
            // wrong frequency: the clock starts from what comes next.
            self.just_found = false;
            self.y_start += self.y.len();
            self.y.clear();
            self.next_block = self.y_start;
            return;
        }
        // One clock phase per whole block.
        while self.y.len() >= self.next_block.saturating_sub(self.y_start) + CLOCK_BLOCK + 8
            && self.next_block >= self.y_start
        {
            let from = self.next_block - self.y_start;
            let mut tau = self.block_tau(from);
            if let Some(prev) = self.tau_last {
                // Keep the phase continuous: the nearest of its aliases.
                tau += SPS * ((prev - tau) / SPS).round();
            }
            self.tau_last = Some(tau);
            let centre = (self.next_block + CLOCK_BLOCK / 2) as f64;
            self.taus.push_back((centre, tau));
            if self.taus.len() > CLOCK_WINDOW {
                self.taus.pop_front();
            }
            self.next_block += CLOCK_BLOCK;
            self.symbols_up_to(self.next_block, out);
        }
        // Keep only the samples a later symbol can still need.
        let keep_from = self.next_block.saturating_sub(8).max(self.y_start);
        if keep_from > self.y_start {
            self.y.drain(..keep_from - self.y_start);
            self.y_start = keep_from;
        }
    }

    /// Symbols whose sampling instant lies before `limit` (a sample index).
    fn symbols_up_to(&mut self, limit: usize, out: &mut Vec<Symbol>) {
        let Some((c0, slope)) = self.clock_fit() else { return };
        let t_ref = self.taus.back().unwrap().0;
        loop {
            // p = 3m + tau(p), tau(t) = c0 + slope * (t - t_ref).
            let p = (SPS * self.m as f64 + c0 - slope * t_ref) / (1.0 - slope);
            if p >= limit as f64 {
                break;
            }
            if p < (self.y_start + 2) as f64 {
                // Before what is still held (the first block, or a lost one).
                self.m += 1;
                continue;
            }
            let local = p - self.y_start as f64;
            if local as usize + 3 >= self.y.len() {
                break;
            }
            let s = cubic(&self.y, local);
            self.m += 1;
            out.push(self.costas(s));
        }
    }

    /// One symbol through the level control and the phase loop.
    fn costas(&mut self, s: Complex32) -> Symbol {
        let (sn, cs) = self.theta.sin_cos();
        let z = s * Complex32::new(cs, -sn);
        self.amp += (z.norm() - self.amp) * 0.001;
        let z = z / self.amp.max(1e-9);
        let e = z.re.signum() * z.im - z.im.signum() * z.re;
        self.omega += self.beta * e;
        self.theta += self.omega + self.alpha * e;
        if self.theta > std::f32::consts::PI {
            self.theta -= 2.0 * std::f32::consts::PI;
        } else if self.theta < -std::f32::consts::PI {
            self.theta += 2.0 * std::f32::consts::PI;
        }
        let r = std::f32::consts::FRAC_1_SQRT_2;
        let ideal = Complex32::new(z.re.signum() * r, z.im.signum() * r);
        self.evm2 += ((z - ideal).norm_sqr() - self.evm2) * 0.001;
        Symbol { i: z.re, q: z.im }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pulse_is_a_unit_energy_root_raised_cosine() {
        let h = rrc_taps(0.6, 3.0, 6);
        assert_eq!(h.len(), 37);
        let e: f32 = h.iter().map(|v| v * v).sum();
        assert!((e - 1.0).abs() < 1e-4);
        // Symmetric, peak in the middle.
        assert!((h[0] - h[36]).abs() < 1e-6);
        assert!(h.iter().all(|&v| v <= h[18] + 1e-6));
    }
}
