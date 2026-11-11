//! NOAA APT (Automatic Picture Transmission) decoder.
//!
//! The satellite sends two lines a second. A line is 2080 "pixels" at 4160 per
//! second: sync A (39), space A (47), image A (909), telemetry A (45), then
//! sync B, space B, image B, telemetry B. The pixels are the envelope of a
//! 2400 Hz subcarrier, itself frequency-modulated onto the 137 MHz carrier.
//!
//! The chain, all from the public description: FM discriminator on the complex
//! baseband, mix the subcarrier to DC, low-pass, take the magnitude (the AM
//! envelope), resample to 4160 per second, then find each line with the sync A
//! pattern (seven cycles of 1040 Hz) and scale it by that pattern's own high
//! and low levels, so a fading signal does not change the brightness.

use sdroxide_dsp::Complex32;

/// Pixels in one APT line (both channels, sync, space and telemetry).
pub const LINE_PIXELS: usize = 2080;
/// Pixels per second.
pub const PIXEL_RATE_HZ: f64 = 4160.0;
/// Subcarrier frequency, Hz.
pub const SUBCARRIER_HZ: f64 = 2400.0;
/// Length of the sync A pattern in pixels.
pub const SYNC_LEN: usize = 39;
/// Where channel A's image starts in a line (after sync and space).
pub const IMAGE_A_START: usize = 86;
/// Where channel B's image starts in a line.
pub const IMAGE_B_START: usize = 1040 + 86;
/// Pixels of image per channel.
pub const IMAGE_WIDTH: usize = 909;

/// Sync A: 4 low, seven cycles of (2 high, 2 low), then low up to 39 pixels.
pub fn sync_a() -> [f32; SYNC_LEN] {
    let mut s = [0.0f32; SYNC_LEN];
    for c in 0..7 {
        s[4 + c * 4] = 1.0;
        s[5 + c * 4] = 1.0;
    }
    s
}

/// One decoded line.
#[derive(Debug, Clone)]
pub struct AptLine {
    /// 2080 pixels, 0 = black, 255 = white, scaled by the line's sync.
    pub pixels: Vec<u8>,
    /// Correlation with the sync pattern, 0..1 (about 1 on a clean signal).
    pub quality: f32,
    /// False when the line was placed by the clock because the sync was lost.
    pub synced: bool,
}

/// Real FIR low-pass (windowed sinc) run on a complex stream.
struct ComplexFir {
    taps: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    pos: usize,
}

impl ComplexFir {
    fn lowpass(cutoff_hz: f64, rate: f64, n: usize) -> Self {
        let fc = cutoff_hz / rate;
        let m = (n - 1) as f64 / 2.0;
        let mut taps: Vec<f32> = (0..n)
            .map(|i| {
                let x = i as f64 - m;
                let sinc = if x == 0.0 {
                    2.0 * fc
                } else {
                    (2.0 * std::f64::consts::PI * fc * x).sin() / (std::f64::consts::PI * x)
                };
                let w =
                    0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos();
                (sinc * w) as f32
            })
            .collect();
        let sum: f32 = taps.iter().sum();
        for t in taps.iter_mut() {
            *t /= sum;
        }
        Self { taps, re: vec![0.0; n], im: vec![0.0; n], pos: 0 }
    }

    fn push(&mut self, re: f32, im: f32) -> (f32, f32) {
        let n = self.taps.len();
        self.re[self.pos] = re;
        self.im[self.pos] = im;
        let (mut a, mut b) = (0.0f32, 0.0f32);
        let mut j = self.pos;
        for &t in &self.taps {
            a += t * self.re[j];
            b += t * self.im[j];
            j = if j == 0 { n - 1 } else { j - 1 };
        }
        self.pos = (self.pos + 1) % n;
        (a, b)
    }
}

/// From complex baseband at `rate` to the 4160 per second pixel stream.
pub struct AptDemod {
    rate: f64,
    prev: Complex32,
    phase: f64,
    step: f64,
    fir: ComplexFir,
    acc: f32,
    n: u32,
    t: f64,
    /// Running mean power of the input, for a level meter.
    power: f32,
}

impl AptDemod {
    pub fn new(rate: f64) -> Self {
        Self {
            rate,
            prev: Complex32::new(1.0, 0.0),
            phase: 0.0,
            step: 2.0 * std::f64::consts::PI * SUBCARRIER_HZ / rate,
            fir: ComplexFir::lowpass(2000.0, rate, 63),
            acc: 0.0,
            n: 0,
            t: 0.0,
            power: 0.0,
        }
    }

    /// Mean signal level in dB relative to full scale.
    pub fn level_dbfs(&self) -> f32 {
        10.0 * self.power.max(1e-12).log10()
    }

    /// Feed baseband samples; envelope pixels are appended to `out`.
    pub fn process(&mut self, iq: &[Complex32], out: &mut Vec<f32>) {
        let per_pixel = PIXEL_RATE_HZ / self.rate;
        for &x in iq {
            self.power += (x.norm_sqr() - self.power) * 1e-4;
            // FM discriminator: the angle between successive samples.
            let d = x * self.prev.conj();
            self.prev = x;
            let f = d.im.atan2(d.re);
            // Mix the subcarrier down and keep what is below ~2 kHz.
            let (s, c) = (self.phase.sin() as f32, self.phase.cos() as f32);
            self.phase += self.step;
            if self.phase > std::f64::consts::TAU {
                self.phase -= std::f64::consts::TAU;
            }
            let (a, b) = self.fir.push(f * c, -f * s);
            self.acc += (a * a + b * b).sqrt();
            self.n += 1;
            self.t += per_pixel;
            if self.t >= 1.0 {
                self.t -= 1.0;
                out.push(self.acc / self.n as f32);
                self.acc = 0.0;
                self.n = 0;
            }
        }
    }
}

/// Pixels → lines: finds the sync, follows it, scales each line.
pub struct AptDecoder {
    buf: Vec<f32>,
    template: [f32; SYNC_LEN],
    template_norm: f32,
    /// Index in `buf` where the next line is expected, once locked.
    expect: Option<usize>,
    misses: u32,
    lines: u64,
}

/// Projection of the filtered sync on the template over its true swing, for
/// the demodulator's low-pass (measured on a synthetic line, 0.708).
const SYNC_GAIN: f32 = 0.708;
/// Share of the 39 sync pixels that are high (14 of 39).
const SYNC_HIGH_FRACTION: f32 = 14.0 / 39.0;
/// The sync pulses sit at about 92% of full white: 235 of 255.
const SYNC_LEVEL: f32 = 235.0;

/// How far from the expected place a line's sync is looked for, pixels.
const SEARCH: usize = 6;
/// Samples kept before the expected start, so the search can look back.
const KEEP_BEHIND: usize = SEARCH + 2;
/// Correlation needed to accept a sync while hunting for the first one.
const HUNT_THRESHOLD: f32 = 0.55;
/// Correlation needed at the next line's place to confirm that first sync
/// (noise alone passes the first test somewhere in every line's worth).
const CONFIRM_THRESHOLD: f32 = 0.5;
/// Correlation needed to accept a sync once locked.
const TRACK_THRESHOLD: f32 = 0.4;
/// Lines without a sync in a row before the lock is dropped.
const MAX_MISSES: u32 = 10;

impl Default for AptDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl AptDecoder {
    pub fn new() -> Self {
        let raw = sync_a();
        let mean = raw.iter().sum::<f32>() / SYNC_LEN as f32;
        let mut template = [0.0f32; SYNC_LEN];
        for (t, r) in template.iter_mut().zip(raw) {
            *t = r - mean;
        }
        let template_norm = template.iter().map(|t| t * t).sum::<f32>().sqrt();
        Self { buf: Vec::new(), template, template_norm, expect: None, misses: 0, lines: 0 }
    }

    /// Lines emitted since the decoder was made.
    pub fn lines(&self) -> u64 {
        self.lines
    }

    /// True while a sync is being followed.
    pub fn locked(&self) -> bool {
        self.expect.is_some()
    }

    /// Normalised correlation of the sync pattern with `buf[start..]`.
    fn score(&self, start: usize) -> f32 {
        let w = &self.buf[start..start + SYNC_LEN];
        let mean = w.iter().sum::<f32>() / SYNC_LEN as f32;
        let mut dot = 0.0f32;
        let mut var = 0.0f32;
        for (x, t) in w.iter().zip(&self.template) {
            let v = x - mean;
            dot += v * t;
            var += v * v;
        }
        if var <= 1e-12 { 0.0 } else { dot / (var.sqrt() * self.template_norm) }
    }

    fn best_in(&self, from: usize, to: usize) -> (usize, f32) {
        let mut best = (from, f32::MIN);
        for i in from..=to {
            let s = self.score(i);
            if s > best.1 {
                best = (i, s);
            }
        }
        best
    }

    /// Scale a line by the swing of its own sync pattern.
    ///
    /// The 2 kHz low-pass rounds the 2-pixel pulses, so their peaks and valleys
    /// understate the true levels. The pattern's projection on the template
    /// does not: it is `SYNC_GAIN` times the true swing for this filter, and
    /// the mean of the 39 pixels is kept by the filter, which gives the low
    /// level back.
    fn line_at(&self, start: usize, quality: f32, synced: bool) -> AptLine {
        let w = &self.buf[start..start + SYNC_LEN];
        let mean = w.iter().sum::<f32>() / SYNC_LEN as f32;
        let proj: f32 = w.iter().zip(&self.template).map(|(x, t)| (x - mean) * t).sum::<f32>()
            / (self.template_norm * self.template_norm);
        let swing = (proj / SYNC_GAIN).max(1e-9);
        let lo = mean - swing * SYNC_HIGH_FRACTION;
        let pixels = self.buf[start..start + LINE_PIXELS]
            .iter()
            .map(|&v| (((v - lo) / swing) * SYNC_LEVEL).clamp(0.0, 255.0) as u8)
            .collect();
        AptLine { pixels, quality, synced }
    }

    /// Feed envelope pixels; finished lines are appended to `out`.
    pub fn process(&mut self, pixels: &[f32], out: &mut Vec<AptLine>) {
        self.buf.extend_from_slice(pixels);
        loop {
            match self.expect {
                None => {
                    // Hunting: a whole line in view, so every offset is tried.
                    if self.buf.len() < 2 * LINE_PIXELS + SYNC_LEN + SEARCH {
                        return;
                    }
                    let (at, s) = self.best_in(0, LINE_PIXELS - 1);
                    let next = LINE_PIXELS + at;
                    let confirmed = s >= HUNT_THRESHOLD
                        && self.best_in(next - 2, next + 2).1 >= CONFIRM_THRESHOLD;
                    if confirmed {
                        // Keep the sync where it is: the tracking branch picks it up.
                        self.buf.drain(..at.saturating_sub(KEEP_BEHIND));
                        self.expect = Some(at.min(KEEP_BEHIND));
                        self.misses = 0;
                    } else {
                        self.buf.drain(..LINE_PIXELS);
                    }
                }
                Some(expect) => {
                    if self.buf.len() < expect + SEARCH + LINE_PIXELS {
                        return;
                    }
                    let from = expect.saturating_sub(SEARCH);
                    let (at, s) = self.best_in(from, expect + SEARCH);
                    let (start, synced) = if s >= TRACK_THRESHOLD {
                        self.misses = 0;
                        (at, true)
                    } else {
                        self.misses += 1;
                        (expect, false)
                    };
                    out.push(self.line_at(start, s.max(0.0), synced));
                    self.lines += 1;
                    self.buf.drain(..start + LINE_PIXELS - KEEP_BEHIND);
                    self.expect = if self.misses >= MAX_MISSES { None } else { Some(KEEP_BEHIND) };
                }
            }
        }
    }
}

/// A test transmitter: lines of pixels to complex baseband, the way a NOAA
/// satellite sends them. Public so the engine's tests can use it too.
pub mod synth {
    use super::*;

    /// Build one 2080-pixel line: sync, the two image strips, nothing else.
    pub fn line(image_a: &[u8], image_b: &[u8]) -> Vec<u8> {
        let mut l = vec![0u8; LINE_PIXELS];
        for (i, &s) in sync_a().iter().enumerate() {
            l[i] = if s > 0.5 { 235 } else { 0 };
        }
        // Sync B: seven cycles at 832 Hz, five pixels a cycle.
        for c in 0..7 {
            for k in 0..3 {
                l[1040 + 4 + c * 5 + k] = 235;
            }
        }
        l[IMAGE_A_START..IMAGE_A_START + IMAGE_WIDTH].copy_from_slice(image_a);
        l[IMAGE_B_START..IMAGE_B_START + IMAGE_WIDTH].copy_from_slice(image_b);
        l
    }

    /// Modulate lines to complex baseband at `rate`: pixels held over their
    /// time, AM on 2400 Hz, FM with 17 kHz of deviation. `amplitude` scales
    /// the whole signal (a weaker pass); the envelope reaches 1 at pixel 255.
    pub fn modulate(lines: &[Vec<u8>], rate: f64, amplitude: f32) -> Vec<Complex32> {
        let mut out = Vec::new();
        let (mut sub, mut ph) = (0.0f64, 0.0f64);
        let mut t = 0.0f64;
        for l in lines {
            for &p in l {
                let env = 0.15 + 0.85 * f64::from(p) / 255.0;
                t += rate / PIXEL_RATE_HZ;
                while t >= 1.0 {
                    t -= 1.0;
                    sub += std::f64::consts::TAU * SUBCARRIER_HZ / rate;
                    let audio = env * sub.cos();
                    ph += std::f64::consts::TAU * 17_000.0 * audio / rate;
                    out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32) * amplitude);
                }
            }
        }
        out
    }

    /// Deterministic white noise, `sigma` per axis.
    pub fn add_noise(iq: &mut [Complex32], sigma: f32, mut seed: u64) {
        let mut next = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 40) as f32 / (1u64 << 24) as f32) - 0.5
        };
        for x in iq.iter_mut() {
            // Sum of uniforms is near enough to Gaussian for a test.
            let g = |n: &mut dyn FnMut() -> f32| (0..12).map(|_| n()).sum::<f32>();
            *x += Complex32::new(g(&mut next) * sigma, g(&mut next) * sigma);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WXSAT_LANE_RATE_HZ;

    fn ramp(up: bool) -> Vec<u8> {
        (0..IMAGE_WIDTH)
            .map(|i| {
                let v = (i * 255 / (IMAGE_WIDTH - 1)) as u8;
                if up { v } else { 255 - v }
            })
            .collect()
    }

    fn run(iq: &[Complex32]) -> Vec<AptLine> {
        let mut dem = AptDemod::new(WXSAT_LANE_RATE_HZ);
        let mut dec = AptDecoder::new();
        let (mut px, mut lines) = (Vec::new(), Vec::new());
        for chunk in iq.chunks(2048) {
            px.clear();
            dem.process(chunk, &mut px);
            dec.process(&px, &mut lines);
        }
        lines
    }

    fn mean_error(line: &AptLine, a: &[u8], b: &[u8]) -> f32 {
        let mut e = 0.0f32;
        for i in 20..IMAGE_WIDTH - 20 {
            e += (f32::from(line.pixels[IMAGE_A_START + i]) - f32::from(a[i])).abs();
            e += (f32::from(line.pixels[IMAGE_B_START + i]) - f32::from(b[i])).abs();
        }
        e / (2.0 * (IMAGE_WIDTH - 40) as f32)
    }

    #[test]
    fn clean_signal_gives_every_line_with_the_right_picture() {
        let (a, b) = (ramp(true), ramp(false));
        let lines: Vec<Vec<u8>> = (0..12).map(|_| synth::line(&a, &b)).collect();
        let iq = synth::modulate(&lines, WXSAT_LANE_RATE_HZ, 1.0);
        let got = run(&iq);
        assert!(got.len() >= 9, "got {} lines of 12", got.len());
        for l in &got[2..] {
            assert!(l.synced);
            assert!(l.quality > 0.8, "quality {}", l.quality);
            let e = mean_error(l, &a, &b);
            assert!(e < 20.0, "mean error {e}");
        }
    }

    #[test]
    fn a_weak_pass_gives_the_same_brightness() {
        let (a, b) = (ramp(true), ramp(false));
        let lines: Vec<Vec<u8>> = (0..10).map(|_| synth::line(&a, &b)).collect();
        let strong = run(&synth::modulate(&lines, WXSAT_LANE_RATE_HZ, 1.0));
        let weak = run(&synth::modulate(&lines, WXSAT_LANE_RATE_HZ, 0.05));
        assert!(!weak.is_empty());
        let l = weak.len().min(strong.len()) - 1;
        assert!(mean_error(&weak[l], &a, &b) < 20.0);
    }

    #[test]
    fn noise_and_a_late_start_still_lock() {
        let (a, b) = (ramp(true), ramp(false));
        let lines: Vec<Vec<u8>> = (0..16).map(|_| synth::line(&a, &b)).collect();
        let mut iq = synth::modulate(&lines, WXSAT_LANE_RATE_HZ, 1.0);
        // Start in the middle of a line, with noise on top.
        iq.drain(..12_345);
        synth::add_noise(&mut iq, 0.05, 7);
        let got = run(&iq);
        assert!(got.len() >= 10, "got {} lines", got.len());
        let good = got.iter().filter(|l| l.synced && mean_error(l, &a, &b) < 30.0).count();
        assert!(good >= 8, "only {good} good lines of {}", got.len());
    }

    #[test]
    fn noise_alone_does_not_lock() {
        let mut iq = vec![Complex32::new(0.0, 0.0); 48_000 * 6];
        synth::add_noise(&mut iq, 0.2, 3);
        let got = run(&iq);
        assert!(got.len() <= 2, "{} lines from noise", got.len());
    }

    #[test]
    fn a_dropout_is_bridged_by_the_clock() {
        let (a, b) = (ramp(true), ramp(false));
        let lines: Vec<Vec<u8>> = (0..16).map(|_| synth::line(&a, &b)).collect();
        let mut iq = synth::modulate(&lines, WXSAT_LANE_RATE_HZ, 1.0);
        // Two lines of silence in the middle.
        let one = (48_000.0 / 2.0) as usize;
        for x in iq[one * 8..one * 10].iter_mut() {
            *x = Complex32::new(0.0, 0.0);
        }
        let got = run(&iq);
        let last = got.last().expect("lines");
        assert!(last.synced, "should relock after the dropout");
        assert!(got.len() >= 13);
    }
}
