//! SAM-MRC: the two sidebands of a locked AM signal, combined frequency by
//! frequency in proportion to how well each one came through.
//!
//! # Why
//!
//! On HF the two paths of a skywave arrive a millisecond or so apart, and a
//! delay is a phase that turns with frequency: the channel becomes a comb that
//! drifts across the signal. A notch on the carrier makes the envelope
//! detector distort (the sidebands outgrow what is left of the carrier); a
//! notch at `fc + f` takes out the upper sideband at audio frequency `f` while
//! the lower one at `fc − f` usually still has it. Plain synchronous AM adds
//! the two with equal weight, so a notch in one costs that frequency half its
//! amplitude and the other sideband's full noise. Choosing one sideband for
//! the whole band, as the usual "SAM-U / SAM-L" switch does, trades one
//! notch for the other.
//!
//! # How
//!
//! After the carrier loop has de-rotated the signal, bin `k` of a short-time
//! spectrum carries the upper sideband as `a = X[+k]` and the lower one,
//! mirrored, as `b = conj X[−k]`. Both are the same message through different
//! channels: `a = h_u·M + n`, `b = h_l·M + n'`, with noise of equal power `N`
//! in both. The 2×2 covariance of `(a, b)`, smoothed over a quarter of a second
//! and ±2 bins, is rank one plus `N·I`: its smaller eigenvalue is the noise and
//! its principal eigenvector the maximal-ratio weights. The output is
//! `w₁*·a + w₂*·b`, with the eigenvector's free phase fixed so that `w₁ + w₂`
//! is real — on a flat channel that is `(a + b)/√2`, i.e. exactly what plain
//! SAM gives, scaled back by `1/√2` to the same level.
//!
//! # Prior art, and what is changed
//!
//! The weighted sum of the two sidebands, also per subband, is B. W. Kroeger,
//! *Coherent AM demodulator using a weighted LSB/USB sum for interference
//! mitigation*, US 7 127 008 B2 (iBiquity Digital, filed 2003-02-24, granted
//! 2006-10-24). There the weights are real, taken from each sideband's
//! estimated noise and interference, with the wanted signal assumed equally
//! strong in both; the target is adjacent-channel and IBOC interference.
//! Modified by Hans-Dieter Zucht for selective fading, where that assumption
//! fails: the signal itself differs between the sidebands in level *and*
//! phase, so the weights here are complex, from the full sideband covariance,
//! and line the two sidebands up in phase before they are added.
//!
//! # One sideband alone
//!
//! The same frames give SAM-USB and SAM-LSB for free: the output is `a` or `b`
//! instead of their combination, at the same level as plain SAM. That is the
//! answer to an interferer on one side, which the combination cannot tell
//! from signal (it assumes equal noise in both sidebands).
//!
//! The frame is 1024 samples at 48 kHz (47 Hz bins, 21 ms), hop a quarter of
//! it, square-root Hann in and out; the delay is one frame plus one hop.
//
// Why: HDZ 2026-11-16, 49 m AM in deep fading: "kannst du da was an der
//      Demodulation tun?"; after the A/B on 90 s of his own IQ (5.860 MHz,
//      16.11. 01:03): "das mrc klang gut", "machst du das als Toggle-Knopf".
//      Bench (`tools/fading-pruefstand.py` in HDZ's notes, noise taken from an
//      empty stretch of the same band through the same weights, the null
//      test's bias removed): audio S/N +1.1 dB over plain SAM overall, +3.7 dB
//      in the median time-frequency cell; one sideband per frame for the whole
//      band came out 0.8 dB *below* plain SAM.

use std::collections::VecDeque;
use std::sync::Arc;

use rustfft::{Fft, FftPlanner};

use crate::Complex32;

/// Frame length at 48 kHz; scaled with the rate to keep ~47 Hz bins.
const FRAME_AT_48K: usize = 1024;
/// Smoothing of the sideband covariance to start with ([`SidebandMrc::set_tau`]
/// changes it; the menu offers 0.2, 0.5 and 1 s). Long enough to average over
/// the programme's own spectral changes, short against the fades (their
/// carrier coherence time measured 1.1 s on the bench recording).
const COV_TAU_S: f64 = 0.2;
/// Bins either side averaged into the covariance: the comb's notches are
/// hundreds of hertz wide, a bin is 47.
const COV_BINS: usize = 2;
/// Below this the output is cut: the carrier's own slow fading lives here.
const LOW_CUT_HZ: f64 = 70.0;
/// The carrier is taken out before the frames are cut, as plain SAM's DC
/// blocker does: left in, a steady carrier leaks from bin 0 into the bins
/// above the cut, and the overlap-add turns that into a buzz at the hop rate.
const DC_CORNER_HZ: f64 = 20.0;

/// What the combiner puts out per frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// Maximal-ratio combination of both sidebands.
    Mrc,
    /// The upper sideband alone.
    Upper,
    /// The lower sideband alone.
    Lower,
}

pub struct SidebandMrc {
    pick: Pick,
    n: usize,
    hop: usize,
    win: Vec<f32>,
    /// Synthesis gain of the square-root Hann pair at this overlap.
    ola_gain: f32,
    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex32>,
    ring: Vec<Complex32>,
    pos: usize,
    since: usize,
    frame: Vec<Complex32>,
    saa: Vec<f32>,
    sbb: Vec<f32>,
    sab: Vec<Complex32>,
    alpha: f32,
    tau_s: f32,
    rate: f64,
    lo_bin: usize,
    ola: Vec<f32>,
    fifo: VecDeque<f32>,
    spec: Vec<Complex32>,
    dc: Complex32,
    dc_alpha: f32,
}

impl SidebandMrc {
    pub fn new(rate: f64, pick: Pick) -> Self {
        let n =
            ((FRAME_AT_48K as f64 * rate / 48_000.0).round() as usize).next_power_of_two().max(64);
        let hop = n / 4;
        let win: Vec<f32> = (0..n)
            .map(|i| {
                (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos()).sqrt() as f32
            })
            .collect();
        let mut planner = FftPlanner::<f32>::new();
        let fwd = planner.plan_fft_forward(n);
        let inv = planner.plan_fft_inverse(n);
        let scratch_len = fwd.get_inplace_scratch_len().max(inv.get_inplace_scratch_len());
        let half = n / 2 + 1;
        SidebandMrc {
            pick,
            n,
            hop,
            win,
            ola_gain: 1.0 / (0.5 * n as f32 / hop as f32),
            fwd,
            inv,
            scratch: vec![Complex32::new(0.0, 0.0); scratch_len],
            ring: vec![Complex32::new(0.0, 0.0); n],
            pos: 0,
            since: 0,
            frame: vec![Complex32::new(0.0, 0.0); n],
            saa: vec![0.0; half],
            sbb: vec![0.0; half],
            sab: vec![Complex32::new(0.0, 0.0); half],
            alpha: (1.0 - (-(hop as f64) / (rate * COV_TAU_S)).exp()) as f32,
            tau_s: COV_TAU_S as f32,
            rate,
            lo_bin: (LOW_CUT_HZ * n as f64 / rate).ceil() as usize,
            ola: vec![0.0; n],
            fifo: std::iter::repeat_n(0.0, hop).collect(),
            spec: vec![Complex32::new(0.0, 0.0); n / 2],
            dc: Complex32::new(0.0, 0.0),
            dc_alpha: (1.0 - (-std::f64::consts::TAU * DC_CORNER_HZ / rate).exp()) as f32,
        }
    }

    /// The covariance's averaging time in seconds; cheap when unchanged.
    pub fn set_tau(&mut self, tau_s: f32) {
        if tau_s == self.tau_s || !tau_s.is_finite() || tau_s <= 0.0 {
            return;
        }
        self.tau_s = tau_s;
        self.alpha = (1.0 - (-(self.hop as f64) / (self.rate * tau_s as f64)).exp()) as f32;
    }

    /// Switch between combining and one sideband; the covariance keeps
    /// running in every case, so a switch back to MRC starts settled.
    pub fn set_pick(&mut self, pick: Pick) {
        self.pick = pick;
    }

    /// One audio sample out for every de-rotated sample in, delayed by one
    /// frame plus one hop.
    pub fn process(&mut self, v: &[Complex32], out: &mut Vec<f32>) {
        for &z in v {
            self.dc += (z - self.dc) * self.dc_alpha;
            self.ring[self.pos] = z - self.dc;
            self.pos = (self.pos + 1) % self.n;
            self.since += 1;
            if self.since == self.hop {
                self.since = 0;
                self.frame();
            }
            out.push(self.fifo.pop_front().unwrap_or(0.0));
        }
    }

    fn frame(&mut self) {
        let n = self.n;
        for i in 0..n {
            self.frame[i] = self.ring[(self.pos + i) % n] * self.win[i];
        }
        self.fwd.process_with_scratch(&mut self.frame, &mut self.scratch);

        let half = n / 2;
        let al = self.alpha;
        for k in 1..half {
            let a = self.frame[k];
            let b = self.frame[n - k].conj();
            self.saa[k] += al * (a.norm_sqr() - self.saa[k]);
            self.sbb[k] += al * (b.norm_sqr() - self.sbb[k]);
            let ab = self.sab[k];
            self.sab[k] = ab + (a * b.conj() - ab) * al;
        }

        let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
        self.spec.iter_mut().for_each(|z| *z = Complex32::new(0.0, 0.0));
        // One sideband alone: `a` (or `b`) is half the amplitude of plain
        // SAM's `a + b` on a flat channel, and so is the combination below
        // after its 1/√2, so all three come out at plain SAM's level.
        if self.pick != Pick::Mrc {
            let upper = self.pick == Pick::Upper;
            for k in self.lo_bin.max(1)..half {
                self.spec[k] = if upper { self.frame[k] } else { self.frame[n - k].conj() };
            }
        }
        for k in self.lo_bin.max(1)..half {
            if self.pick != Pick::Mrc {
                break;
            }
            let lo = k.saturating_sub(COV_BINS).max(1);
            let hi = (k + COV_BINS).min(half - 1);
            let m = (hi - lo + 1) as f32;
            let (mut aa, mut bb, mut ab) = (0.0f32, 0.0f32, Complex32::new(0.0, 0.0));
            for j in lo..=hi {
                aa += self.saa[j];
                bb += self.sbb[j];
                ab += self.sab[j];
            }
            aa /= m;
            bb /= m;
            ab /= m;
            let disc = ((aa - bb) * (aa - bb) + 4.0 * ab.norm_sqr()).sqrt();
            let l1 = 0.5 * (aa + bb + disc);
            // Principal eigenvector (l1 − bb, ab*); falls back to plain SAM's
            // equal weights where the covariance has nothing to say.
            let mut w1 = Complex32::new(l1 - bb, 0.0);
            let mut w2 = ab.conj();
            let nrm = (w1.norm_sqr() + w2.norm_sqr()).sqrt();
            if nrm > 1e-20 && nrm.is_finite() {
                w1 /= nrm;
                w2 /= nrm;
                let p = w1 + w2;
                let pn = p.norm();
                if pn > 1e-12 {
                    let rot = p.conj() / pn;
                    w1 *= rot;
                    w2 *= rot;
                }
            } else {
                w1 = Complex32::new(inv_sqrt2, 0.0);
                w2 = Complex32::new(inv_sqrt2, 0.0);
            }
            let a = self.frame[k];
            let b = self.frame[n - k].conj();
            self.spec[k] = (w1.conj() * a + w2.conj() * b) * inv_sqrt2;
        }

        // A real signal whose positive half is the combined spectrum.
        self.frame.iter_mut().for_each(|z| *z = Complex32::new(0.0, 0.0));
        for k in 1..half {
            self.frame[k] = self.spec[k];
            self.frame[n - k] = self.spec[k].conj();
        }
        self.inv.process_with_scratch(&mut self.frame, &mut self.scratch);
        let scale = self.ola_gain / n as f32;
        for i in 0..n {
            self.ola[i] += self.frame[i].re * self.win[i] * scale;
        }
        self.fifo.extend(self.ola.drain(..self.hop));
        self.ola.extend(std::iter::repeat_n(0.0, self.hop));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// A tone at `f` as DSB around a locked carrier, with the upper and lower
    /// sideband scaled by `gu`, `gl`, plus noise of the same power in both.
    fn dsb(f: f64, gu: f32, gl: f32, noise: f32, len: usize, seed: u64) -> Vec<Complex32> {
        let mut s = seed;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s as f64 / u64::MAX as f64 - 0.5) as f32 * 3.464 // unit variance
        };
        (0..len)
            .map(|i| {
                let ph = std::f64::consts::TAU * f * i as f64 / RATE;
                let u = Complex32::new(ph.cos() as f32, ph.sin() as f32) * (0.5 * gu);
                let l = Complex32::new(ph.cos() as f32, -(ph.sin() as f32)) * (0.5 * gl);
                Complex32::new(1.0, 0.0) + u + l + Complex32::new(rnd(), rnd()) * noise
            })
            .collect()
    }

    fn tone_and_rest(x: &[f32], f: f64) -> (f64, f64) {
        // Project onto the tone, everything else is the rest.
        let (mut c, mut s) = (0.0f64, 0.0f64);
        for (i, &v) in x.iter().enumerate() {
            let ph = std::f64::consts::TAU * f * i as f64 / RATE;
            c += v as f64 * ph.cos();
            s += v as f64 * ph.sin();
        }
        let n = x.len() as f64;
        let amp2 = 4.0 * (c * c + s * s) / (n * n);
        let total: f64 = x.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / n;
        (amp2 / 2.0, (total - amp2 / 2.0).max(0.0))
    }

    fn run(x: &[Complex32]) -> Vec<f32> {
        run_pick(x, Pick::Mrc)
    }

    fn run_pick(x: &[Complex32], pick: Pick) -> Vec<f32> {
        let mut m = SidebandMrc::new(RATE, pick);
        let mut out = Vec::new();
        m.process(x, &mut out);
        out[RATE as usize..].to_vec() // let the covariance settle
    }

    #[test]
    fn a_flat_channel_gives_what_plain_sam_gives() {
        let x = dsb(1000.0, 1.0, 1.0, 0.0, 3 * RATE as usize, 1);
        let y = run(&x);
        let (tone, rest) = tone_and_rest(&y, 1000.0);
        // Plain SAM: Re{v} − carrier = cos(ωt), power 0.5.
        assert!((tone - 0.5).abs() < 0.03, "tone power {tone}");
        assert!(rest < 1e-3 * tone, "residue {rest}");
    }

    #[test]
    fn a_notched_sideband_is_carried_by_the_other() {
        // Upper sideband 30 dB down at the tone: plain SAM keeps about half the
        // amplitude and both noises; MRC leans on the lower sideband.
        let x = dsb(1500.0, 0.03, 1.0, 0.05, 4 * RATE as usize, 7);
        let y = run(&x);
        let (t_mrc, n_mrc) = tone_and_rest(&y, 1500.0);
        let sam: Vec<f32> = x[RATE as usize..].iter().map(|z| z.re - 1.0).collect();
        let (t_sam, n_sam) = tone_and_rest(&sam, 1500.0);
        // Compare in the tone's neighbourhood only: the wideband noise is the
        // same white floor for both, so the ratio over the whole band shows it.
        let gain_db = 10.0 * ((t_mrc / n_mrc) / (t_sam / n_sam)).log10();
        assert!(gain_db > 1.5, "MRC over SAM {gain_db:.2} dB");
    }

    #[test]
    fn one_sideband_alone_keeps_plain_sams_level_and_drops_the_other() {
        // An AM tone at 1 kHz in both sidebands, and an interferer at 2 kHz in
        // the upper one only.
        let len = 3 * RATE as usize;
        let x: Vec<Complex32> = (0..len)
            .map(|i| {
                let t = i as f64 / RATE;
                let m = std::f64::consts::TAU * 1000.0 * t;
                let q = std::f64::consts::TAU * 2000.0 * t;
                Complex32::new(1.0 + m.cos() as f32, 0.0)
                    + Complex32::new(q.cos() as f32, q.sin() as f32) * 0.5
            })
            .collect();
        for (pick, hears_interferer) in [(Pick::Upper, true), (Pick::Lower, false)] {
            let y = run_pick(&x, pick);
            let (tone, _) = tone_and_rest(&y, 1000.0);
            let (q, _) = tone_and_rest(&y, 2000.0);
            // Plain SAM gives the AM tone as cos(ωt), power 0.5; so does one
            // sideband alone.
            assert!((tone - 0.5).abs() < 0.03, "{pick:?}: tone {tone}");
            if hears_interferer {
                assert!(q > 0.1, "{pick:?}: interferer {q}");
            } else {
                assert!(q < 1e-4 * tone, "{pick:?}: interferer {q}");
            }
        }
    }

    #[test]
    fn one_sample_out_per_sample_in() {
        let mut m = SidebandMrc::new(RATE, Pick::Mrc);
        let mut out = Vec::new();
        for len in [1usize, 255, 256, 1000, 4097] {
            let before = out.len();
            m.process(&vec![Complex32::new(1.0, 0.0); len], &mut out);
            assert_eq!(out.len() - before, len);
        }
    }
}
