//! Receive audio to the carrier's envelope, a thousand times a second.
//!
//! The carrier arrives as a tone near [`sdroxide_types::TIMESIG_TONE_HZ`]. It
//! is mixed to zero, averaged down to about 1 kHz, found exactly by a
//! spectrum of a few seconds of that, mixed the rest of the way, and smoothed
//! over 5 ms. What leaves is the carrier's amplitude, which is the whole of
//! what an amplitude-keyed time code says.
//!
//! # Coherent, where the station allows it
//!
//! The amplitude is read *coherently* — as the part of the signal in phase
//! with the carrier — wherever the station's carrier keeps its phase, which
//! is every one of them but WWVB. The carrier's phase is followed by averaging
//! the signal itself over half a second: the keying changes its size, never
//! its direction. Against simply taking the magnitude, that leaves out the
//! half of the noise in the other quadrature, and it stays linear when the
//! noise is as strong as the carrier, where a magnitude flattens the keying
//! towards the noise's own level. Both matter most exactly where a weak
//! station needs them.
//!
//! WWVB has since 2012 also keyed its carrier's phase by 180° once a second,
//! so its "in phase" flips sign with the phase code; it is read by magnitude.
//!
//! # Where each sample is
//!
//! Every filter here is a plain average — symmetric, so its delay is exactly
//! half its length and nothing else. That matters more than its shape: the
//! point of the exercise is to know *when* the carrier dropped, and a filter
//! whose delay depended on frequency or level would move the answer. Each
//! output carries `pos`, the input sample (fractional) it stands for once
//! both averages' delays are taken off.

use std::sync::Arc;

use num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use sdroxide_types::TIMESIG_SEARCH_HZ;

/// The rate the envelope is wanted at. The decimation is the nearest whole
/// number to the input rate over this, so the real rate is close to it rather
/// than equal — 1002.27 Hz from 44.1 kHz — and everything downstream works in
/// seconds rather than in envelope samples.
const ENV_RATE: f64 = 1000.0;
/// Envelope smoothing, in envelope samples: 5 ms. Short against the 100 ms
/// the keying is made of, long enough to take most of the noise off the edge.
const SMOOTH: usize = 5;
/// The carrier search's transform: about four seconds at the envelope rate,
/// a quarter of a hertz a bin.
const FFT_N: usize = 4096;
/// How far a carrier has to stand above the median of the search range to be
/// believed, as a power ratio: 15 dB.
const CARRIER_MIN_RATIO: f32 = 31.6;
/// The carrier-phase average's time constant, in envelope samples: half a
/// second. Long against the noise, short against what is left of the
/// frequency error after the search (a few hundredths of a hertz).
const PHASE_TC: f32 = 500.0;

/// One envelope sample.
#[derive(Debug, Clone, Copy)]
pub struct EnvSample {
    /// The input sample this stands for, counted from the first sample the
    /// front end was given.
    pub pos: f64,
    /// The carrier's amplitude.
    pub mag: f32,
}

pub struct FrontEnd {
    rate: f64,
    /// Input samples per envelope sample.
    m: usize,
    tone_hz: f64,
    /// The coarse mixer's phase, radians, and its step per input sample.
    phase: f64,
    step: f64,
    acc: Complex32,
    acc_n: usize,
    /// Decimated samples produced so far.
    blocks: u64,
    /// The fine mixer, at the envelope rate.
    fine_phase: f64,
    ring: [Complex32; SMOOTH],
    /// Read the in-phase part rather than the magnitude.
    coherent: bool,
    /// The carrier's phase reference: a slow average of the signal.
    reference: Complex32,
    fft: Arc<dyn Fft<f32>>,
    fft_buf: Vec<Complex32>,
    window: Vec<f32>,
    spectrum: Vec<f32>,
    have_spectrum: bool,
    /// The carrier's offset from the tone, Hz, once found.
    offset_hz: Option<f64>,
    /// A different carrier seen once, waiting to be seen again before the
    /// search moves to it — so one burst of interference cannot take it away
    /// from the station.
    candidate: Option<f64>,
    /// How many times the carrier has been found somewhere new. Whatever was
    /// measured before a move was measured on the wrong frequency.
    moves: u32,
}

impl FrontEnd {
    /// `coherent` reads the carrier's in-phase part rather than its magnitude
    /// — right for every station whose carrier phase is not keyed.
    pub fn new(rate: f64, tone_hz: f64, coherent: bool) -> Self {
        let m = (rate / ENV_RATE).round().max(1.0) as usize;
        let fft = FftPlanner::new().plan_fft_forward(FFT_N);
        let window = (0..FFT_N)
            .map(|i| {
                let x = std::f32::consts::PI * i as f32 / FFT_N as f32;
                x.sin() * x.sin()
            })
            .collect();
        FrontEnd {
            rate,
            m,
            tone_hz,
            phase: 0.0,
            step: std::f64::consts::TAU * tone_hz / rate,
            acc: Complex32::new(0.0, 0.0),
            acc_n: 0,
            blocks: 0,
            fine_phase: 0.0,
            ring: [Complex32::new(0.0, 0.0); SMOOTH],
            coherent,
            reference: Complex32::new(0.0, 0.0),
            fft,
            fft_buf: Vec::with_capacity(FFT_N),
            window,
            spectrum: vec![0.0; FFT_N],
            have_spectrum: false,
            offset_hz: None,
            candidate: None,
            moves: 0,
        }
    }

    /// The envelope's sample rate.
    pub fn env_rate(&self) -> f64 {
        self.rate / self.m as f64
    }

    /// Counts each time the carrier was found somewhere new; see
    /// [`FrontEnd::moves`]'s field.
    pub fn moves(&self) -> u32 {
        self.moves
    }

    /// Where the carrier is in the audio, once found.
    pub fn carrier_hz(&self) -> Option<f64> {
        self.offset_hz.map(|o| self.tone_hz + o)
    }

    pub fn process(&mut self, audio: &[f32], out: &mut Vec<EnvSample>) {
        for &x in audio {
            let (s, c) = self.phase.sin_cos();
            self.acc += Complex32::new(x * c as f32, -x * s as f32);
            self.phase += self.step;
            if self.phase > std::f64::consts::PI {
                self.phase -= std::f64::consts::TAU;
            }
            self.acc_n += 1;
            if self.acc_n == self.m {
                let y = self.acc / self.m as f32;
                self.acc = Complex32::new(0.0, 0.0);
                self.acc_n = 0;
                self.block(y, out);
            }
        }
    }

    fn block(&mut self, y: Complex32, out: &mut Vec<EnvSample>) {
        self.fft_buf.push(y);
        if self.fft_buf.len() == FFT_N {
            self.search();
            self.fft_buf.clear();
        }
        let env_rate = self.env_rate();
        if let Some(off) = self.offset_hz {
            self.fine_phase += std::f64::consts::TAU * off / env_rate;
            if self.fine_phase > std::f64::consts::PI {
                self.fine_phase -= std::f64::consts::TAU;
            } else if self.fine_phase < -std::f64::consts::PI {
                self.fine_phase += std::f64::consts::TAU;
            }
        }
        let (s, c) = self.fine_phase.sin_cos();
        let yf = y * Complex32::new(c as f32, -s as f32);
        let k = self.blocks;
        self.ring[(k % SMOOTH as u64) as usize] = yf;
        self.blocks += 1;
        if self.blocks >= SMOOTH as u64 {
            let sum: Complex32 = self.ring.iter().sum();
            // Block k averages input samples k·m … k·m + m − 1, centred on
            // k·m + (m − 1)/2; the smoothing is centred two blocks back.
            let centre = k as f64 - (SMOOTH as f64 - 1.0) / 2.0;
            let pos = centre * self.m as f64 + (self.m as f64 - 1.0) / 2.0;
            let smooth = sum / SMOOTH as f32;
            let mag = if self.coherent {
                // The reference follows the newest samples and the smoothed
                // value is centred two behind; half a second's average has
                // not turned by anything measurable in two milliseconds.
                self.reference += (smooth - self.reference) / PHASE_TC;
                let r = self.reference.norm();
                if r > 0.0 { (smooth * self.reference.conj()).re / r } else { smooth.norm() }
            } else {
                smooth.norm()
            };
            out.push(EnvSample { pos, mag });
        }
    }

    /// Finds the carrier in the last few seconds' spectrum.
    fn search(&mut self) {
        let mut buf: Vec<Complex32> =
            self.fft_buf.iter().zip(&self.window).map(|(z, w)| z * *w).collect();
        self.fft.process(&mut buf);
        for (acc, z) in self.spectrum.iter_mut().zip(&buf) {
            let p = z.norm_sqr();
            *acc = if self.have_spectrum { 0.5 * *acc + 0.5 * p } else { p };
        }
        self.have_spectrum = true;

        let env_rate = self.env_rate() as f32;
        let bin_hz = env_rate / FFT_N as f32;
        let reach = (TIMESIG_SEARCH_HZ / bin_hz) as isize;
        let at = |i: isize| self.spectrum[i.rem_euclid(FFT_N as isize) as usize];
        let mut range: Vec<f32> = (-reach..=reach).map(at).collect();
        let (peak_i, peak) = (-reach..=reach)
            .map(|i| (i, at(i)))
            .fold((0, f32::MIN), |best, x| if x.1 > best.1 { x } else { best });
        range.sort_by(f32::total_cmp);
        let median = range[range.len() / 2].max(f32::MIN_POSITIVE);
        if peak / median < CARRIER_MIN_RATIO {
            return;
        }
        // Parabolic interpolation on the log spectrum.
        let (l, c, r) = (at(peak_i - 1).ln(), peak.ln(), at(peak_i + 1).ln());
        let den = l - 2.0 * c + r;
        let frac = if den.abs() > 1e-12 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 };
        let found = f64::from((peak_i as f32 + frac) * bin_hz);
        // The envelope filter is ±100 Hz wide, so before the first find a
        // carrier within a few hertz of the tone was already being measured
        // properly and finding it there is not a move.
        let moved = |from: f64| (found - from).abs() >= 2.0;
        match self.offset_hz {
            None => {
                self.offset_hz = Some(found);
                self.moves += u32::from(moved(0.0));
            }
            Some(off) if !moved(off) => {
                self.offset_hz = Some(off + 0.5 * (found - off));
                self.candidate = None;
            }
            Some(_) => match self.candidate {
                Some(c) if (found - c).abs() < 2.0 => {
                    self.offset_hz = Some(found);
                    self.candidate = None;
                    self.moves += 1;
                }
                _ => self.candidate = Some(found),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: f64, hz: f64, secs: f64) -> Vec<f32> {
        (0..(rate * secs) as usize)
            .map(|n| (0.3 * (std::f64::consts::TAU * hz * n as f64 / rate).cos()) as f32)
            .collect()
    }

    #[test]
    fn finds_a_carrier_off_the_tone() {
        for (rate, off) in [(12_000.0, 0.0), (48_000.0, 173.4), (44_100.0, -251.0)] {
            let mut fe = FrontEnd::new(rate, 1000.0, true);
            let mut out = Vec::new();
            fe.process(&tone(rate, 1000.0 + off, 10.0), &mut out);
            let found = fe.carrier_hz().expect("carrier") - 1000.0;
            assert!((found - off).abs() < 0.2, "{rate}: {found} for {off}");
            // Once found, the envelope is the carrier's amplitude, steady: a
            // cosine of 0.3 mixes to 0.15, less the first average's droop a
            // couple of hundred hertz off the tone (10 % at 250 Hz). What is
            // left of the tone's image ripples it by a percent or two, at a
            // few hundred hertz the slot averages take out entirely.
            let tail = &out[out.len() - 100..];
            for e in tail {
                assert!(e.mag > 0.13 && e.mag < 0.153, "{rate}: {}", e.mag);
            }
        }
    }

    #[test]
    fn an_edge_lands_where_it_was_made() {
        // A carrier that drops to nothing at exactly 2.5 s: the envelope's half
        // way point must sit at 2.5 s whatever the rate.
        for rate in [8_000.0, 12_000.0, 44_100.0, 48_000.0] {
            let mut x = tone(rate, 1000.0, 4.0);
            let cut = (2.5 * rate) as usize;
            for s in &mut x[cut..] {
                *s = 0.0;
            }
            let mut fe = FrontEnd::new(rate, 1000.0, true);
            let mut out = Vec::new();
            fe.process(&x, &mut out);
            let half = 0.075;
            let i = out.iter().position(|e| e.pos > rate && e.mag < half).expect("edge");
            // Interpolate between the samples either side of the crossing.
            let (a, b) = (out[i - 1], out[i]);
            let pos = a.pos + (b.pos - a.pos) * f64::from((a.mag - half) / (a.mag - b.mag));
            let err_ms = (pos - cut as f64) / rate * 1000.0;
            assert!(err_ms.abs() < 0.6, "{rate}: edge off by {err_ms} ms");
        }
    }
}
