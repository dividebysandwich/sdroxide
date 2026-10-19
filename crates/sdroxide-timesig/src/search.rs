//! Finding a station's carrier in a few seconds of spectrum.
//!
//! Both front ends hand this the carrier channel at about 1 kHz — mixed down
//! from the tone it should be at — and it answers how far from the tone the
//! carrier really is. A spectrum of four seconds gives a quarter of a hertz a
//! bin; the peak, interpolated, is good to a few hundredths.
//!
//! # A carrier that is phase keyed
//!
//! WWVB inverts its carrier for every 1 of its phase code, and a carrier
//! flipping at random once a second has no line in its spectrum to find —
//! only a hump a couple of hertz wide where the line was. Squaring the signal
//! takes the keying off (either sign squares to the same), leaving a clean
//! line at twice the offset. Twice the offset folds over at the edge of the
//! decimated band, so the squared spectrum is read for the fine answer and
//! the plain one, hump and all, decides which of the folded images it was.
//!
//! # Moving
//!
//! A carrier found somewhere new is believed only when it is found there
//! twice running, so one burst of interference cannot take the search away
//! from the station; and every move is counted, because whatever was
//! measured before it was measured on the wrong frequency.

use std::sync::Arc;

use num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use sdroxide_types::TIMESIG_SEARCH_HZ;

/// About four seconds at the envelope rate: a quarter of a hertz a bin.
const FFT_N: usize = 4096;
/// How far a carrier has to stand above the median of the search range to be
/// believed, as a power ratio: 15 dB.
const CARRIER_MIN_RATIO: f32 = 31.6;
/// How far a new find may be from the last and still be the same carrier,
/// Hz. The plain spectrum's hump on a phase-keyed carrier is good to about
/// half a hertz; anything past this is somewhere else.
const SAME_CARRIER_HZ: f64 = 2.0;

pub(crate) struct CarrierSearch {
    env_rate: f64,
    squared: bool,
    fft: Arc<dyn Fft<f32>>,
    buf: Vec<Complex32>,
    window: Vec<f32>,
    spectrum: Vec<f32>,
    spectrum_sq: Vec<f32>,
    have_spectrum: bool,
    offset_hz: Option<f64>,
    candidate: Option<f64>,
    moves: u32,
}

impl CarrierSearch {
    /// `squared` reads the carrier off the squared signal, for a station that
    /// keys its carrier's phase.
    pub fn new(env_rate: f64, squared: bool) -> Self {
        let window = (0..FFT_N)
            .map(|i| {
                let x = std::f32::consts::PI * i as f32 / FFT_N as f32;
                x.sin() * x.sin()
            })
            .collect();
        CarrierSearch {
            env_rate,
            squared,
            fft: FftPlanner::new().plan_fft_forward(FFT_N),
            buf: Vec::with_capacity(FFT_N),
            window,
            spectrum: vec![0.0; FFT_N],
            spectrum_sq: vec![0.0; if squared { FFT_N } else { 0 }],
            have_spectrum: false,
            offset_hz: None,
            candidate: None,
            moves: 0,
        }
    }

    /// The carrier's offset from the tone, Hz, once found.
    pub fn offset_hz(&self) -> Option<f64> {
        self.offset_hz
    }

    /// How many times the carrier has been found somewhere new.
    pub fn moves(&self) -> u32 {
        self.moves
    }

    pub fn push(&mut self, y: Complex32) {
        self.buf.push(y);
        if self.buf.len() == FFT_N {
            self.search();
            self.buf.clear();
        }
    }

    fn transform(&self, square: bool) -> Vec<f32> {
        let mut b: Vec<Complex32> = self
            .buf
            .iter()
            .zip(&self.window)
            .map(|(z, w)| if square { z * z * *w } else { z * *w })
            .collect();
        self.fft.process(&mut b);
        b.iter().map(|z| z.norm_sqr()).collect()
    }

    fn search(&mut self) {
        let fresh = self.transform(false);
        let fresh_sq = if self.squared { self.transform(true) } else { Vec::new() };
        let first = !self.have_spectrum;
        for (acc, p) in self.spectrum.iter_mut().zip(&fresh) {
            *acc = if first { *p } else { 0.5 * *acc + 0.5 * p };
        }
        for (acc, p) in self.spectrum_sq.iter_mut().zip(&fresh_sq) {
            *acc = if first { *p } else { 0.5 * *acc + 0.5 * p };
        }
        self.have_spectrum = true;

        let bin_hz = self.env_rate / FFT_N as f64;
        let reach = (f64::from(TIMESIG_SEARCH_HZ) / bin_hz) as isize;
        let plain = peak(&self.spectrum, -reach..=reach);
        let found = if self.squared {
            // The squared line is at twice the offset, wherever that folds
            // to; look across the whole band for it.
            let half = FFT_N as isize / 2;
            match peak(&self.spectrum_sq, -half + 1..=half - 1) {
                Some(line) => {
                    let base = line * bin_hz / 2.0;
                    let fold = self.env_rate / 2.0;
                    // Of the offsets that square to that line, the one the
                    // plain spectrum has the most power near.
                    let near = |f: f64| {
                        let c = (f / bin_hz).round() as isize;
                        (c - 4..=c + 4).map(|i| at(&self.spectrum, i)).sum::<f32>()
                    };
                    [base - fold, base, base + fold]
                        .into_iter()
                        .filter(|f| f.abs() <= f64::from(TIMESIG_SEARCH_HZ))
                        .max_by(|a, b| near(*a).total_cmp(&near(*b)))
                }
                // Too weak to square cleanly: the plain hump still says
                // roughly where the carrier is, which is all the amplitude
                // code needs.
                None => plain.map(|p| p * bin_hz),
            }
        } else {
            plain.map(|p| p * bin_hz)
        };
        let Some(found) = found else { return };
        // The envelope filter is ±100 Hz wide, so before the first find a
        // carrier within a couple of hertz of the tone was already being
        // measured properly and finding it there is not a move.
        let moved = |from: f64| (found - from).abs() >= SAME_CARRIER_HZ;
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
                Some(c) if (found - c).abs() < SAME_CARRIER_HZ => {
                    self.offset_hz = Some(found);
                    self.candidate = None;
                    self.moves += 1;
                }
                _ => self.candidate = Some(found),
            },
        }
    }
}

fn at(spectrum: &[f32], i: isize) -> f32 {
    spectrum[i.rem_euclid(spectrum.len() as isize) as usize]
}

/// The strongest bin in `range`, interpolated, if it stands clear of the
/// range's median.
fn peak(spectrum: &[f32], range: std::ops::RangeInclusive<isize>) -> Option<f64> {
    let mut v: Vec<f32> = range.clone().map(|i| at(spectrum, i)).collect();
    let (i, p) = range
        .map(|i| (i, at(spectrum, i)))
        .fold((0, f32::MIN), |best, x| if x.1 > best.1 { x } else { best });
    v.sort_by(f32::total_cmp);
    let median = v[v.len() / 2].max(f32::MIN_POSITIVE);
    if p / median < CARRIER_MIN_RATIO {
        return None;
    }
    // Parabolic interpolation on the log spectrum.
    let (l, c, r) = (at(spectrum, i - 1).ln(), p.ln(), at(spectrum, i + 1).ln());
    let den = l - 2.0 * c + r;
    let frac = if den.abs() > 1e-12 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 };
    Some(f64::from(i as f32 + frac))
}
