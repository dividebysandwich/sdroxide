//! Modulators: 48 kHz mic/line audio → complex baseband, carrier at DC.

use sdroxide_types::Mode;

use crate::Complex32;
use crate::fir::{ComplexFir, RealFir, bandpass_taps};

const TX_TAPS: usize = 331;

pub trait Modulator: Send {
    /// Consume audio, append complex baseband samples (same rate).
    fn process(&mut self, audio: &[f32], out: &mut Vec<Complex32>);

    /// Retune the transmit passband. No-op for modulators with a fixed
    /// bandwidth (AM, FM, CW, RIFP).
    fn set_filter(&mut self, _lo_hz: f32, _hi_hz: f32) {}
}

/// Modulator for a mode. `None` = the mode has no audio-driven transmit (SPEC,
/// WFM broadcast, and CW) — the engine transmits a plain carrier for those when
/// keyed.
///
/// `passband` is the SSB filter's band-pass edges for plain voice (USB/LSB),
/// so transmit bandwidth follows the operator's receive filter. Other modes
/// ignore it and use their own fixed passband.
pub fn make_modulator(mode: Mode, rate: f64, passband: (f32, f32)) -> Option<Box<dyn Modulator>> {
    let (lo, hi) = match mode {
        Mode::Lsb | Mode::Usb => passband,
        _ => mode.default_filter(),
    };
    match mode {
        // FT8/FT4 modulate as USB: the synthesized 12 kHz audio (resampled to
        // 48 k, injected as "mic") is USB-modulated exactly like a real radio.
        // PSK/RTTY, Olivia/Thor/FSQ, and SSTV ride the same USB path.
        Mode::Lsb
        | Mode::Usb
        | Mode::Digu
        | Mode::Digl
        | Mode::Ft8
        | Mode::Js8
        | Mode::Wspr
        | Mode::Ft4
        | Mode::Psk
        | Mode::Rtty
        | Mode::Sstv
        | Mode::Wefax
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq
        | Mode::Hell
        | Mode::RfPaint
        | Mode::Rade => Some(Box::new(SsbMod::new(rate, lo, hi))),
        Mode::Am | Mode::Sam | Mode::Dsb => Some(Box::new(AmMod::new(rate))),
        Mode::Nfm => Some(Box::new(FmMod::new(rate))),
        // RIFP keys the carrier itself rather than a sideband of it.
        Mode::Rifp => Some(Box::new(CpfskMod::new(rate))),
        // CW has none, deliberately. A manual PTT in CW keys a carrier — that
        // is what a straight key does — and modulating the microphone instead
        // would put speech on the air in a CW segment. The panel's keyer needs
        // single-sideband all the same, and asks for it by name
        // (`Engine::tx_block_digi`).
        Mode::Cw | Mode::Wfm | Mode::Spec => None,
    }
}

/// Filter-method SSB: audio as a real (analytic-less) signal through a
/// complex band-pass selects one sideband.
pub struct SsbMod {
    fir: ComplexFir,
    complex_in: Vec<Complex32>,
    rate: f64,
}

impl SsbMod {
    pub fn new(rate: f64, lo: f32, hi: f32) -> Self {
        SsbMod {
            fir: ComplexFir::new(bandpass_taps(TX_TAPS, lo as f64, hi as f64, rate)),
            complex_in: Vec::new(),
            rate,
        }
    }
}

impl Modulator for SsbMod {
    fn process(&mut self, audio: &[f32], out: &mut Vec<Complex32>) {
        self.complex_in.clear();
        self.complex_in.extend(audio.iter().map(|&a| Complex32::new(a.clamp(-1.0, 1.0), 0.0)));
        let before = out.len();
        self.fir.process(&self.complex_in, out);
        // A real tone splits half its amplitude into each sideband.
        for z in &mut out[before..] {
            *z *= 2.0;
        }
    }

    fn set_filter(&mut self, lo_hz: f32, hi_hz: f32) {
        self.fir.set_taps(bandpass_taps(TX_TAPS, lo_hz as f64, hi_hz as f64, self.rate));
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
            self.filtered.iter().map(|&a| Complex32::new(0.5 * (1.0 + a.clamp(-1.0, 1.0)), 0.0)),
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
            self.phase =
                (self.phase + self.dev_step * a.clamp(-1.0, 1.0) as f64) % std::f64::consts::TAU;
            out.push(Complex32::new(0.9 * self.phase.cos() as f32, 0.9 * self.phase.sin() as f32));
        }
    }
}

/// Continuous-phase FSK for the RIFP `rifp-cpfsk-4800` profile: the ±1 symbol
/// waveform from [`crate::rifp::RifpTx`] integrated straight into carrier
/// phase at ±4000 Hz. Deliberately unfiltered — rectangular NRZ into a phase
/// integrator is what the profile specifies, and what the reference
/// implementation transmits.
///
/// Between frames the modem feeds exactly zero, which this reads as "no
/// carrier" and ramps the envelope down over [`RAMP_S`] (and back up when the
/// next frame starts), so each burst has soft edges instead of the splatter a
/// hard key would make.
pub struct CpfskMod {
    phase: f64,
    dev_step: f64,
    /// Transmit envelope, 0 (idle) … 1 (keyed).
    env: f32,
    ramp_step: f32,
}

/// Envelope rise/fall time. Matches the reference implementation's default
/// 2 ms burst ramp; ten symbols of a 384-bit preamble is nothing to lose.
const RAMP_S: f64 = 0.002;

impl CpfskMod {
    pub fn new(rate: f64) -> Self {
        CpfskMod {
            phase: 0.0,
            dev_step: std::f64::consts::TAU * crate::rifp::DEVIATION_HZ / rate,
            env: 0.0,
            ramp_step: (1.0 / (RAMP_S * rate)) as f32,
        }
    }
}

impl Modulator for CpfskMod {
    fn process(&mut self, symbols: &[f32], out: &mut Vec<Complex32>) {
        for &s in symbols {
            let s = s.clamp(-1.0, 1.0);
            let target = if s == 0.0 { 0.0 } else { 1.0 };
            self.env = if self.env < target {
                (self.env + self.ramp_step).min(target)
            } else {
                (self.env - self.ramp_step).max(target)
            };
            // Phase advances only while keyed; an idle gap must not leave the
            // next burst starting from an arbitrary offset of nothing.
            self.phase = (self.phase + self.dev_step * s as f64) % std::f64::consts::TAU;
            // Raised-cosine amplitude over the linear envelope ramp.
            let a = 0.9 * (0.5 - 0.5 * (std::f32::consts::PI * self.env.clamp(0.0, 1.0)).cos());
            out.push(Complex32::new(a * self.phase.cos() as f32, a * self.phase.sin() as f32));
        }
    }
}
