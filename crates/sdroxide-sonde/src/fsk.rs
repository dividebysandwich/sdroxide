//! The front end every FSK radiosonde model shares: mix the channel to
//! baseband, FM discriminator, slicer, and a bit clock that integrates over
//! each symbol and is nudged by the zero crossings.
//!
//! Models differ in baud rate, line coding and frame; they all start from the
//! raw symbol stream this produces.

use sdroxide_dsp::Complex32;

/// How the slicing threshold follows the signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Threshold {
    /// Slow mean of the symbols: right for a DC-balanced code (Manchester).
    Mean,
    /// Midway between the recent highest and lowest level: right when the
    /// stream is not balanced, or sits at one level between bursts.
    Midrange,
}

pub struct FskSlicer {
    spb: f64,
    osc: f64,
    osc_step: f64,
    prev: Complex32,
    y: f32,
    /// Smoothing of the discriminator output, per sample.
    alpha: f32,
    mode: Threshold,
    dc: f32,
    hi: f32,
    lo: f32,
    phase: f64,
    acc: f32,
    cnt: u32,
    last_above: bool,
    level: f32,
    /// Two one-pole low-pass stages on the complex baseband, ahead of the
    /// discriminator, for a narrow signal in a wide channel.
    lp: Option<(f32, [Complex32; 2])>,
    /// Midrange only: samples without a crossing after which the highest and
    /// lowest levels stop decaying (a long run of one symbol).
    hold: Option<u32>,
    since_cross: u32,
}

impl FskSlicer {
    /// `baud` raw symbols per second; the channel centre is `freq_offset_hz`
    /// from the middle of the input.
    pub fn new(
        input_rate: f64,
        freq_offset_hz: f64,
        baud: f64,
        mode: Threshold,
    ) -> Result<Self, String> {
        if input_rate < 4.0 * baud {
            return Err(format!("input rate {input_rate} too low for {baud} baud"));
        }
        Ok(Self {
            spb: input_rate / baud,
            osc: 0.0,
            osc_step: -2.0 * std::f64::consts::PI * freq_offset_hz / input_rate,
            prev: Complex32::new(1.0, 0.0),
            y: 0.0,
            alpha: 0.5,
            mode,
            dc: 0.0,
            hi: 0.0,
            lo: 0.0,
            phase: 0.0,
            acc: 0.0,
            cnt: 0,
            last_above: false,
            level: -120.0,
            lp: None,
            hold: None,
            since_cross: 0,
        })
    }

    /// Smooth the discriminator with a time constant of `tau_s` seconds
    /// (the default is about 40 microseconds at 48 kS/s). A slow, noisy
    /// model wants more.
    pub fn with_smoothing(mut self, tau_s: f64, input_rate: f64) -> Self {
        self.alpha = (1.0 / (input_rate * tau_s)).clamp(0.01, 1.0) as f32;
        self
    }

    /// Low-pass the channel (two one-pole stages, about `cutoff_hz`) before
    /// the discriminator. A narrow signal such as the 1200 baud MTS01 drowns
    /// in the wideband noise of the raw discriminator without it.
    pub fn with_channel_lowpass(mut self, cutoff_hz: f64, input_rate: f64) -> Self {
        let a = 1.0 - (-2.0 * std::f64::consts::PI * cutoff_hz / input_rate).exp();
        self.lp = Some((a as f32, [Complex32::new(0.0, 0.0); 2]));
        self
    }

    /// Midrange threshold: keep the highest and lowest levels when `bits`
    /// symbols go by without a crossing, so that a long run of the same
    /// symbol (the zero padding of an MTS01 frame) does not pull the
    /// threshold onto the signal.
    pub fn with_hold(mut self, bits: f64) -> Self {
        self.hold = Some((bits * self.spb) as u32);
        self
    }

    /// One sample through the mixer, the optional channel low-pass and the
    /// discriminator; returns the smoothed discriminator output (radians per
    /// sample).
    fn demod(&mut self, x: Complex32) -> f32 {
        let (s, c) = self.osc.sin_cos();
        self.osc += self.osc_step;
        if self.osc > std::f64::consts::TAU {
            self.osc -= std::f64::consts::TAU;
        } else if self.osc < -std::f64::consts::TAU {
            self.osc += std::f64::consts::TAU;
        }
        let mut z = x * Complex32::new(c as f32, s as f32);
        self.level += 0.0005 * (10.0 * (z.norm_sqr() + 1e-12).log10() - self.level);
        if let Some((a, st)) = self.lp.as_mut() {
            st[0] += (z - st[0]) * *a;
            st[1] += (st[0] - st[1]) * *a;
            z = st[1];
        }
        let d = (z * self.prev.conj()).arg();
        self.prev = z;
        self.y += self.alpha * (d - self.y);
        self.y
    }

    /// Feed I/Q; `out` is called with every discriminator sample (the audio
    /// of an FM signal, radians per sample), for models that carry tones.
    pub fn process_fm(&mut self, input: &[Complex32], mut out: impl FnMut(f32)) {
        for &x in input {
            let y = self.demod(x);
            out(y);
        }
    }

    /// Feed I/Q; `bit` is called with every raw symbol (0 or 1).
    pub fn process(&mut self, input: &[Complex32], mut bit: impl FnMut(u8)) {
        for &x in input {
            self.demod(x);

            if self.mode == Threshold::Midrange {
                self.since_cross = self.since_cross.saturating_add(1);
                let held = self.hold.is_some_and(|h| self.since_cross > h);
                if self.y > self.hi {
                    self.hi = self.y;
                } else if !held {
                    self.hi += 0.001 * (self.y - self.hi);
                }
                if self.y < self.lo {
                    self.lo = self.y;
                } else if !held {
                    self.lo += 0.001 * (self.y - self.lo);
                }
            }
            let thr = self.threshold();

            self.acc += self.y;
            self.cnt += 1;
            self.phase += 1.0;
            let above = self.y > thr;
            if above != self.last_above {
                self.since_cross = 0;
                let err =
                    if self.phase < self.spb / 2.0 { self.phase } else { self.phase - self.spb };
                self.phase -= 0.08 * err;
            }
            self.last_above = above;
            if self.phase >= self.spb {
                self.phase -= self.spb;
                let mean = self.acc / self.cnt.max(1) as f32;
                self.acc = 0.0;
                self.cnt = 0;
                let b = u8::from(mean > thr);
                if self.mode == Threshold::Mean {
                    self.dc += 0.02 * (mean - self.dc);
                }
                bit(b);
            }
        }
    }

    fn threshold(&self) -> f32 {
        match self.mode {
            Threshold::Mean => self.dc,
            Threshold::Midrange => 0.5 * (self.hi + self.lo),
        }
    }

    /// Smoothed channel power, dBFS.
    pub fn level_dbfs(&self) -> f32 {
        self.level
    }
}
