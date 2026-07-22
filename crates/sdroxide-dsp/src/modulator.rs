//! Modulators: 48 kHz mic/line audio → complex baseband, carrier at DC.

use sdroxide_types::Mode;

use crate::Complex32;
use crate::fir::{ComplexFir, RealFir, bandpass_taps};

const TX_TAPS: usize = 331;

pub trait Modulator: Send {
    /// Consume audio, append complex baseband samples (same rate).
    fn process(&mut self, audio: &[f32], out: &mut Vec<Complex32>);
}

/// Modulator for a mode. `None` = mode has no audio-driven TX (SPEC, WFM
/// broadcast, and CW until the keyer exists — the engine transmits a plain
/// carrier for those when keyed).
pub fn make_modulator(mode: Mode, rate: f64) -> Option<Box<dyn Modulator>> {
    let (lo, hi) = mode.default_filter();
    match mode {
        // FT8/FT4 modulate as USB: the synthesized 12 kHz audio (resampled to
        // 48 k, injected as "mic") is USB-modulated exactly like a real radio.
        // PSK/RTTY and SSTV ride the same USB path.
        Mode::Lsb | Mode::Usb | Mode::Digu | Mode::Digl | Mode::Ft8 | Mode::Ft4
        | Mode::Psk | Mode::Rtty | Mode::Sstv => Some(Box::new(SsbMod::new(rate, lo, hi))),
        Mode::Am | Mode::Sam | Mode::Dsb => Some(Box::new(AmMod::new(rate))),
        Mode::Nfm => Some(Box::new(FmMod::new(rate))),
        Mode::Cw | Mode::Wfm | Mode::Spec => None,
    }
}

/// Filter-method SSB: audio as a real (analytic-less) signal through a
/// complex band-pass selects one sideband.
pub struct SsbMod {
    fir: ComplexFir,
    complex_in: Vec<Complex32>,
}

impl SsbMod {
    pub fn new(rate: f64, lo: f32, hi: f32) -> Self {
        SsbMod {
            fir: ComplexFir::new(bandpass_taps(TX_TAPS, lo as f64, hi as f64, rate)),
            complex_in: Vec::new(),
        }
    }
}

impl Modulator for SsbMod {
    fn process(&mut self, audio: &[f32], out: &mut Vec<Complex32>) {
        self.complex_in.clear();
        self.complex_in
            .extend(audio.iter().map(|&a| Complex32::new(a.clamp(-1.0, 1.0), 0.0)));
        let before = out.len();
        self.fir.process(&self.complex_in, out);
        // A real tone splits half its amplitude into each sideband.
        for z in &mut out[before..] {
            *z *= 2.0;
        }
    }
}

/// Full-carrier AM: 0.5·(1 + audio).
pub struct AmMod {
    lpf: RealFir,
    filtered: Vec<f32>,
}

impl AmMod {
    pub fn new(rate: f64) -> Self {
        AmMod { lpf: RealFir::lowpass(129, 4_500.0, rate), filtered: Vec::new() }
    }
}

impl Modulator for AmMod {
    fn process(&mut self, audio: &[f32], out: &mut Vec<Complex32>) {
        self.filtered.clear();
        self.lpf.process(audio, &mut self.filtered);
        out.extend(
            self.filtered
                .iter()
                .map(|&a| Complex32::new(0.5 * (1.0 + a.clamp(-1.0, 1.0)), 0.0)),
        );
    }
}

/// NFM: phase integrator, ±5 kHz deviation.
pub struct FmMod {
    lpf: RealFir,
    filtered: Vec<f32>,
    phase: f64,
    dev_step: f64,
}

impl FmMod {
    pub fn new(rate: f64) -> Self {
        FmMod {
            lpf: RealFir::lowpass(129, 3_000.0, rate),
            filtered: Vec::new(),
            phase: 0.0,
            dev_step: std::f64::consts::TAU * 5_000.0 / rate,
        }
    }
}

impl Modulator for FmMod {
    fn process(&mut self, audio: &[f32], out: &mut Vec<Complex32>) {
        self.filtered.clear();
        self.lpf.process(audio, &mut self.filtered);
        for &a in &self.filtered {
            self.phase = (self.phase + self.dev_step * a.clamp(-1.0, 1.0) as f64)
                % std::f64::consts::TAU;
            out.push(Complex32::new(
                0.9 * self.phase.cos() as f32,
                0.9 * self.phase.sin() as f32,
            ));
        }
    }
}
