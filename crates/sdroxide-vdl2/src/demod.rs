//! D8PSK: the pulse shapes, the interpolating matched filter, and the symbol
//! decision.
//!
//! VDL Mode 2 transmits 10 500 symbols a second, three bits each, as *changes*
//! in carrier phase rather than absolute ones. Eight phase changes, an eighth
//! of a turn apart, Gray-coded so the two symbols either side of a decision
//! boundary differ in one bit — which is what makes the difference between a
//! marginal frame and a lost one, because almost every symbol error is a
//! neighbour error.
//!
//! # Why the symbol position is a fraction
//!
//! A downconverter can only decimate by a whole number, so the samples per
//! symbol are whatever the front end's rate divides down to — 9.14 on an
//! RTL-SDR at 2.4 Msps, 9.64 on an RX-888 — and never a round figure. Rounding
//! 9.64 to 10 slips a symbol every twenty-eight, which over the longest burst is
//! two hundred symbols of drift.
//!
//! And even at an exact ten samples per symbol it would still be a fraction: a
//! transmitter has no idea where this receiver's sample instants are, so a burst
//! starts at a uniformly random sub-sample phase. An interpolating filter is
//! needed either way, and once there is one, a non-integer step costs one
//! addition per symbol. [`Rrc`] is that filter — a bank of sub-sample phases of
//! the same prototype, one evaluation per symbol at wherever the symbol
//! actually lands.
//!
//! # The carrier budget is why a frequency estimate is not optional
//!
//! A decision sector is 45° wide, so a differential decision has ±22.5° of
//! margin. A residual carrier offset of `f` hertz spends `360·f/10500` degrees
//! of it on *every* symbol, so the whole margin goes at
//!
//! ```text
//! 22.5 × 10500 / 360 = 656 Hz
//! ```
//!
//! which at 136.8 MHz is 4.8 parts per million. Avionics are specified at about
//! five; an uncalibrated RTL-SDR is twenty to fifty, up to ten times the entire
//! budget. So the per-burst estimate from the synchronisation word is what makes
//! this work at all, and the decision-directed tracker here is what keeps it
//! working over a long frame.
//!
//! Source: ETSI EN 301 841-1 §4, the VDL Mode 2 physical layer.

use std::f32::consts::{FRAC_PI_4, PI};

use sdroxide_dsp::Complex32;

/// Symbols a second.
pub const SYMBOL_RATE: f64 = 10_500.0;
/// Raised-cosine roll-off, nominal.
pub const ALPHA: f64 = 0.6;
/// Bits a symbol.
pub const BITS_PER_SYMBOL: usize = 3;

/// Phase-change index to the three bits it carries.
///
/// A Gray code: adjacent entries differ in exactly one bit, so a symbol
/// mistaken for its neighbour — which is nearly every symbol error there is —
/// costs one bit rather than up to three.
pub const GRAY: [u8; 8] = [0, 1, 3, 2, 6, 7, 5, 4];
/// The inverse of [`GRAY`]: three bits to the phase-change index.
pub const UNGRAY: [u8; 8] = [0, 1, 3, 2, 7, 6, 4, 5];

/// How hard the decision-directed frequency tracker pulls, per symbol.
///
/// Small on purpose. The estimate from the synchronisation word is already good
/// to a couple of hundred hertz, and this only has to walk out the rest over a
/// few hundred symbols; a fast loop on a noisy constellation would chase its own
/// decision errors round the circle.
const TRACK_GAIN: f32 = 0.02;

/// Root-raised-cosine impulse response, `t` in symbol periods.
pub fn rrc_impulse(t: f64, alpha: f64) -> f64 {
    let eps = 1e-8;
    if t.abs() < eps {
        return 1.0 - alpha + 4.0 * alpha / std::f64::consts::PI;
    }
    if alpha > 0.0 && (t.abs() - 1.0 / (4.0 * alpha)).abs() < eps {
        let a = std::f64::consts::PI / (4.0 * alpha);
        return alpha / 2f64.sqrt()
            * ((1.0 + 2.0 / std::f64::consts::PI) * a.sin()
                + (1.0 - 2.0 / std::f64::consts::PI) * a.cos());
    }
    let pt = std::f64::consts::PI * t;
    let num = (pt * (1.0 - alpha)).sin() + 4.0 * alpha * t * (pt * (1.0 + alpha)).cos();
    let den = pt * (1.0 - (4.0 * alpha * t).powi(2));
    num / den
}

/// Raised-cosine impulse response, `t` in symbol periods.
///
/// The shape the standard names for the transmitter. Kept beside the root form
/// because which of the two a transmitter applies is not something a receiver
/// gets to assume, and `crate::tx` produces both so the decoder is made to cope
/// with either.
pub fn rc_impulse(t: f64, alpha: f64) -> f64 {
    let eps = 1e-8;
    let d = 1.0 - (2.0 * alpha * t).powi(2);
    if alpha > 0.0 && d.abs() < eps {
        // The removable singularity at t = ±1/(2α).
        return std::f64::consts::FRAC_PI_4 * sinc(1.0 / (2.0 * alpha));
    }
    sinc(t) * (std::f64::consts::PI * alpha * t).cos() / d
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x) }
}

/// Cutoff of the receive filter, in symbol rates.
///
/// Wider than the signal, which is why it is a *band-limiting* filter and not a
/// matched one. See [`ChannelFilter`].
pub const CUTOFF_SYMS: f64 = 1.2;

/// The interpolating receive filter: band-limiting, and a bank of sub-sample
/// phases so a symbol can be read wherever it actually lands.
///
/// # Why this is not a root-raised-cosine matched filter
///
/// The standard puts the *whole* raised cosine at the transmitter, and a raised
/// cosine is already a Nyquist pulse: sampled at the symbol instants it has no
/// inter-symbol interference at all. A root-raised-cosine receive filter would
/// disturb that — measured against a compliant transmitter it leaves 25 %
/// inter-symbol interference, where a flat filter over the signal band leaves
/// under 1 %. The matched filter is the right answer only when the root is
/// split across the two ends, which is not what this standard asks for.
///
/// A transmitter that does split it is still decoded, with about a quarter of
/// the decision margin spent on the mismatch instead of on noise. That is the
/// trade this filter makes, deliberately and in the direction the standard
/// points.
///
/// # It is also the channel filter
///
/// The downconverter's own decimating low-pass is designed against its output
/// rate, which is ten times a channel wide, so a neighbouring VDL2 channel is
/// well inside it. This is what puts it outside: the plan is the whole 25 kHz
/// raster, a neighbour sits two and a third symbol rates away — its own
/// shoulder a symbol and a half away — and both are far into this filter's
/// stopband.
///
/// The cutoff is wider than the signal on purpose. A receiver whose clock is a
/// few tens of parts per million out — which an uncalibrated one is — slides the
/// whole signal sideways, and a filter cut close to the signal's own shoulder
/// would take a slice off it. [`CUTOFF_SYMS`] buys about ±3 kHz of that at the
/// cost of admitting a little more noise; past it the operator has to set a
/// frequency correction, which is why the measured offset is reported per frame.
pub struct ChannelFilter {
    nphase: usize,
    ntaps: usize,
    half: isize,
    /// `nphase` banks of `ntaps`, phase-major.
    taps: Vec<f32>,
}

impl ChannelFilter {
    /// A filter spanning `span_syms` symbols at `sps` samples per symbol, with
    /// `nphase` sub-sample positions.
    pub fn new(sps: f64, span_syms: usize, nphase: usize) -> ChannelFilter {
        assert!(sps > 1.0 && nphase >= 1);
        let ntaps = ((span_syms as f64 * sps).round() as usize).max(3) | 1;
        let half = (ntaps / 2) as isize;
        let mut taps = vec![0f32; nphase * ntaps];
        for p in 0..nphase {
            let frac = p as f64 / nphase as f64;
            let mut row: Vec<f64> = (0..ntaps)
                .map(|k| {
                    let t = (k as f64 - half as f64 - frac) / sps;
                    // A Blackman-Harris taper on the truncation. Without it the
                    // abrupt cut puts sidelobes where the neighbouring channel
                    // is, which is the one place they must not be.
                    lowpass_impulse(t, CUTOFF_SYMS) * blackman_harris(ntaps, k)
                })
                .collect();
            let sum: f64 = row.iter().sum();
            if sum.abs() > 1e-12 {
                row.iter_mut().for_each(|t| *t /= sum);
            }
            for (k, v) in row.into_iter().enumerate() {
                taps[p * ntaps + k] = v as f32;
            }
        }
        ChannelFilter { nphase, ntaps, half, taps }
    }

    pub fn ntaps(&self) -> usize {
        self.ntaps
    }

    /// Samples of run-up the filter needs either side of a position.
    pub fn half(&self) -> usize {
        self.half as usize
    }

    /// The filtered signal at fractional sample position `x`, or zero where the
    /// filter would reach outside the buffer.
    pub fn at(&self, iq: &[Complex32], x: f64) -> Complex32 {
        let mut i0 = x.floor() as isize;
        let frac = x - i0 as f64;
        let mut p = (frac * self.nphase as f64).round() as isize;
        if p >= self.nphase as isize {
            p -= self.nphase as isize;
            i0 += 1;
        }
        let base = i0 - self.half;
        if base < 0 || base as usize + self.ntaps > iq.len() {
            return Complex32::default();
        }
        let base = base as usize;
        let row = &self.taps[p as usize * self.ntaps..][..self.ntaps];
        let mut acc = Complex32::default();
        for (k, &t) in row.iter().enumerate() {
            acc += iq[base + k] * t;
        }
        acc
    }

    /// Run the filter across a whole buffer at whole-sample positions.
    ///
    /// The coarse synchronisation search reads this rather than evaluating the
    /// filter sixteen times per candidate offset: one pass over the burst
    /// instead of one pass per trial position, and the fractional refinement
    /// afterwards only touches the neighbourhood of the peak.
    pub fn run(&self, iq: &[Complex32], out: &mut Vec<Complex32>) {
        out.clear();
        out.reserve(iq.len());
        let row = &self.taps[..self.ntaps];
        for i in 0..iq.len() {
            let base = i as isize - self.half;
            if base < 0 || base as usize + self.ntaps > iq.len() {
                out.push(Complex32::default());
                continue;
            }
            let base = base as usize;
            let mut acc = Complex32::default();
            for (k, &t) in row.iter().enumerate() {
                acc += iq[base + k] * t;
            }
            out.push(acc);
        }
    }
}

/// A windowed-sinc low-pass, `t` in symbol periods, `fc` in symbol rates.
fn lowpass_impulse(t: f64, fc: f64) -> f64 {
    2.0 * fc * sinc(2.0 * fc * t)
}

fn blackman_harris(n: usize, i: usize) -> f64 {
    let x = 2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64;
    0.35875 - 0.48829 * x.cos() + 0.14128 * (2.0 * x).cos() - 0.01168 * (3.0 * x).cos()
}

/// Walks a burst one symbol at a time from a synchronisation lock.
///
/// It does not take a length: the frame's length is in the header, which is the
/// first thing read out of it.
pub struct SymbolReader<'a> {
    iq: &'a [Complex32],
    rrc: &'a ChannelFilter,
    pos: f64,
    sps: f64,
    /// Residual carrier, radians per symbol, tracked as the burst is read.
    w: f32,
    prev: Complex32,
    invert: bool,
    err_acc: f32,
    err_n: u32,
    mag_acc: f32,
}

impl<'a> SymbolReader<'a> {
    /// Start reading immediately after a lock's last synchronisation symbol,
    /// which is the reference the first header symbol is differenced against.
    pub fn new(
        iq: &'a [Complex32],
        rrc: &'a ChannelFilter,
        lock: &crate::sync::Lock,
    ) -> SymbolReader<'a> {
        let mut prev = rrc.at(iq, lock.at);
        if lock.inverted {
            prev = prev.conj();
        }
        SymbolReader {
            iq,
            rrc,
            pos: lock.at,
            sps: lock.sps,
            w: lock.w,
            prev,
            invert: lock.inverted,
            err_acc: 0.0,
            err_n: 0,
            mag_acc: 0.0,
        }
    }

    /// The next symbol's three bits, **most significant first**, or `None`
    /// past the end of what the buffer can be filtered over.
    ///
    /// Which end of the symbol goes first is not a detail: it decides every bit
    /// of the header and every octet of the frame, and reading it backwards
    /// produces a bit stream that is perfectly self-consistent and means
    /// nothing. It was backwards until issue #265 — the transmitter in
    /// `crate::tx` packed symbols the same way round, so the two agreed with
    /// each other and with nothing on the air. Measured against a recording:
    /// most-significant-first makes the header's parity check out with **no
    /// correction** on every burst in it, and least-significant-first needs a
    /// repair on every one and yields lengths that are nonsense.
    pub fn next_symbol(&mut self) -> Option<[u8; 3]> {
        self.pos += self.sps;
        if self.pos.floor() as usize + self.rrc.half() + 1 >= self.iq.len() {
            return None;
        }
        let mut z = self.rrc.at(self.iq, self.pos);
        if self.invert {
            z = z.conj();
        }
        let d = z * self.prev.conj();
        self.prev = z;
        if d.norm_sqr() <= 0.0 {
            return None;
        }
        // Take out the residual carrier the tracker has learned so far, then
        // decide, then feed the leftover back in.
        let ang = wrap_pi(d.arg() - self.w);
        let kr = (ang / FRAC_PI_4).round();
        let err = ang - kr * FRAC_PI_4;
        self.w += TRACK_GAIN * err;
        self.err_acc += err.abs();
        self.err_n += 1;
        self.mag_acc += d.norm_sqr().sqrt();

        let k = (kr as i32 & 7) as usize;
        let v = GRAY[k];
        Some([(v >> 2) & 1, (v >> 1) & 1, v & 1])
    }

    /// Read `n` symbols' worth of bits, stopping early at the end of the burst.
    pub fn read_bits(&mut self, n: usize, out: &mut Vec<u8>) -> bool {
        while out.len() < n {
            match self.next_symbol() {
                Some(b) => out.extend_from_slice(&b),
                None => return false,
            }
        }
        true
    }

    /// Mean residual phase error after the decision, degrees. A decision sector
    /// is 45° wide, so anything over about 15 is a frame that only just arrived.
    pub fn evm_deg(&self) -> f32 {
        if self.err_n == 0 { 0.0 } else { self.err_acc / self.err_n as f32 * 180.0 / PI }
    }

    /// The carrier offset the tracker settled on, hertz.
    pub fn freq_hz(&self) -> f32 {
        self.w / (2.0 * PI) * SYMBOL_RATE as f32
    }

    /// Mean symbol magnitude, for the level report.
    pub fn magnitude(&self) -> f32 {
        if self.err_n == 0 { 0.0 } else { self.mag_acc / self.err_n as f32 }
    }

    /// Fractional sample position just past the last symbol read, so a search
    /// for a second transmission can resume after this one.
    pub fn pos(&self) -> f64 {
        self.pos
    }
}

/// Wrap into (-π, π].
pub fn wrap_pi(mut a: f32) -> f32 {
    while a > PI {
        a -= 2.0 * PI;
    }
    while a <= -PI {
        a += 2.0 * PI;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The map really is a Gray code.
    ///
    /// A mistyped entry fails this immediately. A round trip through
    /// [`GRAY`] and [`UNGRAY`] would not — it passes for any permutation, Gray
    /// or not, and the cost of a non-Gray map is invisible until a weak signal
    /// turns one symbol error into three bit errors.
    #[test]
    fn the_symbol_map_is_a_gray_code() {
        for i in 0..8 {
            let d = GRAY[i] ^ GRAY[(i + 1) % 8];
            assert_eq!(d.count_ones(), 1, "{i} to {} differs in {d:b}", (i + 1) % 8);
        }
        for i in 0..8u8 {
            assert_eq!(UNGRAY[GRAY[i as usize] as usize], i);
            assert_eq!(GRAY[UNGRAY[i as usize] as usize], i);
        }
        let mut seen = [false; 8];
        for &g in &GRAY {
            assert!(!seen[g as usize], "{g} appears twice");
            seen[g as usize] = true;
        }
    }

    /// The receive filter passes the whole signal and stops a neighbouring VDL2
    /// channel, which sits two and a third symbol rates away.
    ///
    /// Measured rather than assumed: the cutoff, the tap count and the taper are
    /// numbers that have to earn their place, and this is where they do it.
    #[test]
    fn the_receive_filter_rejects_the_neighbouring_channel() {
        let sps = 9.6;
        let rrc = ChannelFilter::new(sps, 8, 16);
        let row = &rrc.taps[..rrc.ntaps];
        let response = |f_sym: f64| -> f64 {
            // Frequency in symbol rates, so the sample rate is `sps` of them.
            let mut re = 0.0;
            let mut im = 0.0;
            for (k, &t) in row.iter().enumerate() {
                let ph = -2.0 * std::f64::consts::PI * f_sym / sps * k as f64;
                re += f64::from(t) * ph.cos();
                im += f64::from(t) * ph.sin();
            }
            (re * re + im * im).sqrt()
        };
        let dc = response(0.0);
        let db = |f| 20.0 * (response(f) / dc).log10();

        assert!(db(0.0) > -0.1, "DC is {} dB", db(0.0));
        // Flat across the whole transmitted signal, whose raised-cosine
        // spectrum ends at (1 + alpha)/2 symbol rates. Anything less than flat
        // here is inter-symbol interference the transmitter did not put there.
        assert!(db(0.5 * (1.0 + ALPHA)) > -1.0, "the signal's own edge is {} dB", db(0.8));
        // ...and past the cutoff it goes, so a mistuned receiver still has the
        // signal inside the passband but the noise does not come with it.
        assert!(db(CUTOFF_SYMS + 0.6) < -20.0, "the stopband is only {} dB", db(1.8));
        // A VDL2 neighbour is 25 kHz away, and the part of it that comes
        // nearest is its own shoulder, (1 + alpha)/2 symbol rates below its
        // centre. This is what keeps a strong station on the next channel out
        // of the decision, since the downconverter ahead of it is designed
        // against its own output rate rather than against the channel raster.
        let neighbour = 25_000.0 / SYMBOL_RATE;
        assert!(db(neighbour) < -60.0, "the neighbour is only {} dB down", db(neighbour));
        let shoulder = neighbour - 0.5 * (1.0 + ALPHA);
        assert!(db(shoulder) < -40.0, "the neighbour's shoulder is only {} dB down", db(shoulder));
    }

    /// The filter's phases really are sub-sample positions of one prototype:
    /// interpolating a signal that is a straight line has to give the line back.
    #[test]
    fn the_phase_bank_interpolates() {
        let rrc = ChannelFilter::new(10.0, 6, 32);
        // A tone well inside the passband, sampled; reading it back at
        // fractional positions has to follow it.
        let n = 400;
        let f = 0.01;
        let iq: Vec<Complex32> = (0..n)
            .map(|i| {
                let p = 2.0 * std::f64::consts::PI * f * i as f64;
                Complex32::new(p.cos() as f32, p.sin() as f32)
            })
            .collect();
        for step in 0..20 {
            let x = 200.0 + step as f64 * 0.05;
            let got = rrc.at(&iq, x);
            let p = 2.0 * std::f64::consts::PI * f * x;
            let want = Complex32::new(p.cos() as f32, p.sin() as f32);
            let err = (got - want).norm_sqr().sqrt();
            assert!(err < 0.05, "at {x}: {got:?} against {want:?}");
        }
    }

    /// Reading outside the buffer is zero, not a panic and not garbage from the
    /// end of the burst.
    #[test]
    fn the_filter_does_not_read_past_the_burst() {
        let rrc = ChannelFilter::new(10.0, 8, 16);
        let iq = vec![Complex32::new(1.0, 0.0); 50];
        assert_eq!(rrc.at(&iq, -5.0), Complex32::default());
        assert_eq!(rrc.at(&iq, 500.0), Complex32::default());
        assert_eq!(rrc.at(&iq, 2.0), Complex32::default(), "too near the start to filter");
    }

    /// Wrapping lands inside one turn and changes the angle only by whole
    /// turns — the two things every phase comparison downstream assumes.
    #[test]
    fn phases_wrap_into_one_turn() {
        for k in -6..=6 {
            for &f in &[0.0f32, 0.3, 1.0, -0.7, 3.1] {
                let a = f + k as f32 * 2.0 * PI;
                let w = wrap_pi(a);
                assert!(w.abs() <= PI + 1e-5, "{a} wrapped to {w}");
                let turns = (a - w) / (2.0 * PI);
                assert!((turns - turns.round()).abs() < 1e-3, "{a} moved by {turns} turns");
            }
        }
        assert!(wrap_pi(0.0).abs() < 1e-9);
    }
}
