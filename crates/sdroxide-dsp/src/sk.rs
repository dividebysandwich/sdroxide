//! SK post-filter — filter by HDZ (Hans-Dieter Zucht): the idea, the
//! open/close and width controls and the four presets are his, tuned on air.
//!
//! SK filter for the listener audio, switched on from the NR menu: a filter
//! curve the signal cuts for itself, from the spectral kurtosis of each FFT
//! bin.
//!
//! Gaussian noise read as power has a kurtosis `m2 / m1² − 1` of 1; speech,
//! which comes and goes in a bin, reads far higher (7–10 in the voice band of
//! a weak 80 m LSB signal, against 1.5–2 in the noise above it). So each bin
//! keeps its two moments as exponential means over [`SkFilter::tau_s`], and its
//! gain runs smoothly from [`FLOOR_DB`] at a kurtosis of [`SK_NOISE`] to unity
//! at [`SK_VOICE`], smoothed over `±width` bins. It is the panadapter's SK guide
//! turned into a mask, on the audio.
//!
//! With a long `tau_s` (seconds) the curve settles into the shape of the
//! channel and changes no faster than that, so it cuts the noise-only stretch
//! of the passband without the warble of a frame-by-frame denoiser. With a
//! short one (0.1 s) it opens and closes with the syllables and acts as a gate
//! as well. STFT of about 21 ms, a quarter of it per hop, Hann in and out; the
//! audio is delayed by one frame ([`SkFilter::delay_samples`]).
//!
//! On top of the curve, each bin's gain follows how active the bin is *now* —
//! its power this frame against its own mean — opening with
//! [`DEFAULT_OPEN_S`]-style speed and closing with its own, both set from the
//! NR menu. With a long close time a syllable leaves a tail in the bins it
//! used, which HDZ heard as a cave around the voice; a short one ends it.
//
// Why: HDZ 2026-11-10: "basierend auf unserem SK-Filter … der Filter mit
//      unserem tau ermittelt doch quasi das slow Frequenzspektrum, kannst du das
//      mal um in der FFT eine scharfe Filterkurve zu schneiden, quasi ein SK-
//      Autofilter" and "tau einstellbar, 0.1 ist gut". Bench, 80 m LSB: at
//      tau 2 s the mains rhythm went from 0.39 to 0.01 for 0.8 dB off the voice
//      core; at 0.1 s the pauses went from 5.8 to 12.9 dB below the speech.

use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use rustfft::num_complex::Complex32;
use std::sync::Arc;

/// Kurtosis at and below which a bin is noise and gets [`FLOOR_DB`].
pub const SK_NOISE: f32 = 1.3;
/// Kurtosis at and above which a bin is passed untouched.
pub const SK_VOICE: f32 = 3.0;
/// How far a noise bin is pulled down.
pub const FLOOR_DB: f32 = -20.0;
/// The range the NR menu offers for it.
pub const TAU_RANGE_S: (f32, f32) = (0.05, 10.0);
/// How fast a bin opens when it becomes active, in seconds, to start with.
pub const DEFAULT_OPEN_S: f32 = 0.01;
/// How fast it closes again once it is no longer active, to start with: long,
/// so the filter starts out sounding as it did before it had the control.
pub const DEFAULT_CLOSE_S: f32 = 1.0;
/// The ranges the NR menu offers for the two.
pub const OPEN_RANGE_S: (f32, f32) = (0.005, 1.0);
pub const CLOSE_RANGE_S: (f32, f32) = (0.01, 5.0);
/// How many bins either side the mask is smoothed over, to start with (±1:
/// three bins, about 140 Hz at 48 kHz), and the most the NR menu offers.
/// Wider takes the narrow open peaks — resonators the noise rings — out of
/// the curve, at the price of a coarser one.
pub const DEFAULT_WIDTH: usize = 1;
pub const MAX_WIDTH: usize = 16;
/// A bin this far below its own mean power counts as inactive (closed), and
/// at its mean as fully active; a smooth step in between.
const ACTIVE_BELOW_DB: f32 = -12.0;

pub struct SkFilter {
    rate: f32,
    tau_s: f32,
    open_s: f32,
    close_s: f32,
    width: usize,
    n: usize,
    hop: usize,
    window: Vec<f32>,
    fwd: Arc<dyn RealToComplex<f32>>,
    inv: Arc<dyn ComplexToReal<f32>>,
    frame: Vec<f32>,
    spec: Vec<Complex32>,
    input: Vec<f32>,
    pending: Vec<f32>,
    acc: Vec<f32>,
    out: std::collections::VecDeque<f32>,
    m1: Vec<f32>,
    m2: Vec<f32>,
    gain: Vec<f32>,
    /// Per bin: power this frame, and how active the bin is (0–1), smoothed
    /// with the open and close times.
    power: Vec<f32>,
    act: Vec<f32>,
    /// Per bin: the kurtosis's voice weight this frame (scratch, kept so a hop
    /// does not allocate on the audio thread).
    voice: Vec<f32>,
    frames: usize,
}

impl SkFilter {
    pub fn new(rate: f32, tau_s: f32) -> Self {
        let n = ((0.0213 * rate).round() as usize).next_power_of_two().max(256);
        let hop = n / 4;
        let mut planner = RealFftPlanner::<f32>::new();
        let fwd = planner.plan_fft_forward(n);
        let inv = planner.plan_fft_inverse(n);
        let bins = n / 2 + 1;
        // Periodic Hann, in and out: at a quarter-frame hop the squares sum to
        // exactly 1.5, so the overlap-add puts back what came in.
        let window = (0..n)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos())
            .collect();
        SkFilter {
            rate,
            tau_s: tau_s.clamp(TAU_RANGE_S.0, TAU_RANGE_S.1),
            n,
            hop,
            window,
            frame: fwd.make_input_vec(),
            spec: fwd.make_output_vec(),
            fwd,
            inv,
            input: vec![0.0; n],
            pending: Vec::with_capacity(hop),
            acc: vec![0.0; n],
            out: std::iter::repeat_n(0.0, hop).collect(),
            m1: vec![0.0; bins],
            m2: vec![0.0; bins],
            gain: vec![1.0; bins],
            power: vec![0.0; bins],
            act: vec![1.0; bins],
            voice: vec![1.0; bins],
            open_s: DEFAULT_OPEN_S,
            width: DEFAULT_WIDTH,
            close_s: DEFAULT_CLOSE_S,
            frames: 0,
        }
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn tau_s(&self) -> f32 {
        self.tau_s
    }

    /// Change the time constant in place, keeping what the moments know.
    pub fn set_tau(&mut self, tau_s: f32) {
        self.tau_s = tau_s.clamp(TAU_RANGE_S.0, TAU_RANGE_S.1);
    }

    /// How fast a bin opens and closes, in seconds — see [`Self::frame_step`].
    pub fn set_open_close(&mut self, open_s: f32, close_s: f32) {
        self.open_s = open_s.clamp(OPEN_RANGE_S.0, OPEN_RANGE_S.1);
        self.close_s = close_s.clamp(CLOSE_RANGE_S.0, CLOSE_RANGE_S.1);
    }

    pub fn open_close_s(&self) -> (f32, f32) {
        (self.open_s, self.close_s)
    }

    /// Smooth the mask over `width` bins either side (0: none).
    pub fn set_width(&mut self, width: usize) {
        self.width = width.min(MAX_WIDTH);
    }

    pub fn width(&self) -> usize {
        self.width
    }

    /// Width of one bin in Hz, for showing [`Self::width`] as a bandwidth.
    pub fn bin_hz(&self) -> f32 {
        self.rate / self.n as f32
    }

    /// What the filter adds to the audio's delay.
    pub fn delay_samples(&self) -> usize {
        self.n
    }

    pub fn process(&mut self, audio: &mut [f32]) {
        for s in audio.iter_mut() {
            self.pending.push(*s);
            if self.pending.len() == self.hop {
                self.input.drain(..self.hop);
                self.input.append(&mut self.pending);
                self.frame_step();
            }
            *s = self.out.pop_front().unwrap_or(0.0);
        }
    }

    fn frame_step(&mut self) {
        for ((f, &x), &w) in self.frame.iter_mut().zip(&self.input).zip(&self.window) {
            *f = x * w;
        }
        if self.fwd.process(&mut self.frame, &mut self.spec).is_err() {
            return;
        }
        let a = 1.0 - (-(self.hop as f32) / (self.rate * self.tau_s)).exp();
        let first = self.frames == 0;
        let mut top = 0.0f32;
        for (k, c) in self.spec.iter().enumerate() {
            let p = c.norm_sqr();
            self.power[k] = p;
            if first {
                (self.m1[k], self.m2[k]) = (p, p * p);
            } else {
                self.m1[k] += a * (p - self.m1[k]);
                self.m2[k] += a * (p * p - self.m2[k]);
            }
            top = top.max(self.m1[k]);
        }
        self.frames += 1;
        // Until the moments have seen a few time constants, a kurtosis is a
        // guess; the filter passes everything rather than act on one.
        let settled = self.frames as f32 * self.hop as f32 >= 3.0 * self.tau_s * self.rate
            && self.frames >= 20;
        let floor = 10f32.powf(FLOOR_DB / 20.0);
        let bins = self.spec.len();
        // `m1²` must stay a normal float: through a long digital silence `m1`
        // and `top` decay together, and an underflowed square would make the
        // kurtosis 0/0.
        let quiet = (1e-9 * top).max(1e-15);
        let mut u = std::mem::take(&mut self.voice);
        u.fill(1.0);
        if settled {
            for (k, uk) in u.iter_mut().enumerate() {
                // A bin with next to nothing in it reads as anything at all;
                // there is nothing there to cut either.
                if self.m1[k] <= quiet {
                    continue;
                }
                let sk = self.m2[k] / (self.m1[k] * self.m1[k]) - 1.0;
                let t = ((sk - SK_NOISE) / (SK_VOICE - SK_NOISE)).clamp(0.0, 1.0);
                *uk = t * t * (3.0 - 2.0 * t);
            }
        }
        // How active each bin is *now*: its power this frame against its own
        // mean. The kurtosis says where speech lives and changes only over
        // tau; this says whether it is speaking, and follows the open time up
        // and the close time down. A short close time ends the tail a syllable
        // leaves in the bins it used — the cave the filter otherwise builds
        // around the voice.
        // Why: HDZ 2026-11-10: "es klingt wie in der Tropfsteinhöhle … Öffnen
        //      und Schließen mit eigenen Reglern versehen".
        let hop_s = self.hop as f32 / self.rate;
        let (up, down) = (1.0 - (-hop_s / self.open_s).exp(), 1.0 - (-hop_s / self.close_s).exp());
        for k in 0..bins {
            let target = if settled && self.m1[k] > quiet {
                let db = 10.0 * (self.power[k] / self.m1[k]).max(1e-12).log10();
                let t = ((db - ACTIVE_BELOW_DB) / -ACTIVE_BELOW_DB).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            } else {
                1.0
            };
            let rate = if target > self.act[k] { up } else { down };
            self.act[k] += rate * (target - self.act[k]);
        }
        for k in 0..bins {
            let lo = k.saturating_sub(self.width);
            let hi = (k + self.width).min(bins - 1);
            let mean = u[lo..=hi].iter().sum::<f32>() / (hi - lo + 1) as f32;
            let act = self.act[lo..=hi].iter().sum::<f32>() / (hi - lo + 1) as f32;
            self.gain[k] = floor + (1.0 - floor) * mean * act;
        }
        self.voice = u;
        for (c, &g) in self.spec.iter_mut().zip(&self.gain) {
            *c *= g;
        }
        // The DC and Nyquist bins of a real signal must stay real.
        self.spec[0].im = 0.0;
        if let Some(last) = self.spec.last_mut() {
            last.im = 0.0;
        }
        if self.inv.process(&mut self.spec, &mut self.frame).is_err() {
            return;
        }
        let scale = 1.0 / (self.n as f32 * 1.5);
        for ((acc, &y), &w) in self.acc.iter_mut().zip(&self.frame).zip(&self.window) {
            *acc += y * w * scale;
        }
        self.out.extend(self.acc.drain(..self.hop));
        self.acc.extend(std::iter::repeat_n(0.0, self.hop));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn noise(n: usize, amp: f32, seed: u32) -> Vec<f32> {
        let mut r = seed;
        (0..n)
            .map(|_| {
                // Sum of four uniforms: close enough to Gaussian for a kurtosis.
                let mut s = 0.0;
                for _ in 0..4 {
                    r ^= r << 13;
                    r ^= r >> 17;
                    r ^= r << 5;
                    s += r as f32 / u32::MAX as f32 - 0.5;
                }
                amp * s
            })
            .collect()
    }

    fn band_power(x: &[f32], lo: f32, hi: f32) -> f32 {
        let n = 4096;
        let mut planner = RealFftPlanner::<f32>::new();
        let f = planner.plan_fft_forward(n);
        let mut total = 0.0;
        for c in x.chunks_exact(n) {
            let mut buf = c.to_vec();
            let mut sp = f.make_output_vec();
            f.process(&mut buf, &mut sp).unwrap();
            for (k, v) in sp.iter().enumerate() {
                let hz = k as f32 * RATE / n as f32;
                if (lo..hi).contains(&hz) {
                    total += v.norm_sqr();
                }
            }
        }
        total
    }

    /// Proves: before the moments have settled the filter passes the audio
    /// exactly, only delayed — the overlap-add puts back what came in.
    #[test]
    fn the_overlap_add_puts_back_what_came_in() {
        let x = noise(4_000, 0.1, 3);
        let mut y = x.clone();
        let mut f = SkFilter::new(RATE, 10.0);
        f.process(&mut y);
        let d = f.delay_samples();
        for i in d + 1_024..x.len() {
            assert!((y[i] - x[i - d]).abs() < 1e-4, "at {i}: {} vs {}", y[i], x[i - d]);
        }
    }

    /// Proves: a band where something comes and goes — sparse bursts of a
    /// tone, like speech — is passed, and a band of steady Gaussian noise beside it is
    /// pulled down towards the floor.
    #[test]
    fn noise_is_cut_and_bursts_are_kept() {
        let n = 8 * RATE as usize;
        let hiss = noise(n, 0.05, 11);
        // Steady noise above 2 kHz only: a crude high-pass of the white noise.
        let mut steady = vec![0.0; n];
        for i in 2..n {
            steady[i] = hiss[i] - 1.6 * hiss[i - 1] + 0.64 * hiss[i - 2];
        }
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / RATE;
                // 600 Hz on for 30 ms of every 200 ms: sparse like speech in its bins,
                // a kurtosis of about 1 / 0.15 − 1 ≈ 5.7 read as power.
                let on = (t * 5.0).fract() < 0.15;
                steady[i] + if on { 0.2 * (std::f32::consts::TAU * 600.0 * t).sin() } else { 0.0 }
            })
            .collect();
        let mut y = x.clone();
        let mut f = SkFilter::new(RATE, 1.0);
        f.process(&mut y);
        let d = f.delay_samples();
        let tail = 3 * RATE as usize + 2 * RATE as usize / 2;
        let (xi, yi) = (&x[tail..n - d], &y[tail + d..n]);
        let keep = 10.0 * (band_power(yi, 500.0, 700.0) / band_power(xi, 500.0, 700.0)).log10();
        let cut =
            10.0 * (band_power(yi, 6_000.0, 12_000.0) / band_power(xi, 6_000.0, 12_000.0)).log10();
        assert!(keep > -3.0, "the bursts lost {keep:.1} dB");
        assert!(cut < -10.0, "the steady noise lost only {cut:.1} dB");
    }

    /// Proves: the close time decides how long a syllable's bins stay open
    /// after it — the "cave". White noise under 30 ms bursts of a 600 Hz tone
    /// every 200 ms: in the gaps, the noise around 600 Hz comes through a long
    /// close time and is shut out by a short one.
    #[test]
    fn a_short_close_time_ends_the_tail() {
        let n = 8 * RATE as usize;
        let hiss = noise(n, 0.01, 23);
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / RATE;
                let on = (t * 5.0).fract() < 0.15;
                hiss[i] + if on { 0.2 * (std::f32::consts::TAU * 600.0 * t).sin() } else { 0.0 }
            })
            .collect();
        let tail = |close_s: f32| {
            let mut f = SkFilter::new(RATE, 1.0);
            f.set_open_close(DEFAULT_OPEN_S, close_s);
            let mut y = x.clone();
            f.process(&mut y);
            let d = f.delay_samples();
            // The gaps from 80 to 180 ms into each 200 ms period, last 3 s.
            let mut gap = Vec::new();
            let period = RATE as usize / 5;
            for start in (5 * RATE as usize..n - d - period).step_by(period) {
                let a = start + period * 2 / 5;
                gap.extend_from_slice(&y[a + d..a + d + period / 2]);
            }
            band_power(&gap, 500.0, 700.0)
        };
        let db = 10.0 * (tail(0.02) / tail(1.0)).log10();
        assert!(db < -6.0, "a short close time took only {db:.1} dB off the tail");
    }

    /// Proves: a wider smoothing takes the narrow peaks out of the mask — the
    /// largest step between neighbouring bins shrinks.
    #[test]
    fn a_wider_width_flattens_the_mask() {
        let n = 4 * RATE as usize;
        let hiss = noise(n, 0.02, 5);
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / RATE;
                let on = (t * 5.0).fract() < 0.15;
                let tones = (1..6)
                    .map(|h| (std::f32::consts::TAU * 180.0 * h as f32 * t).sin())
                    .sum::<f32>();
                hiss[i] + if on { 0.05 * tones } else { 0.0 }
            })
            .collect();
        let step = |width: usize| {
            let mut f = SkFilter::new(RATE, 0.5);
            f.set_width(width);
            let mut y = x.clone();
            f.process(&mut y);
            f.gain.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max)
        };
        let (narrow, wide) = (step(0), step(8));
        assert!(wide < narrow * 0.5, "width 8 left a step of {wide:.3} against {narrow:.3}");
    }
}
