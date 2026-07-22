//! Demodulators operating on channel-rate (≈48 kHz) complex baseband.
//!
//! The wanted signal's carrier sits at DC; the passband filter edges are in
//! Hz relative to that carrier (negative = lower sideband).

use sdroxide_types::Mode;

use crate::Complex32;
use crate::fir::{ComplexFir, RealFir, bandpass_taps};

const PASSBAND_TAPS: usize = 331;

pub trait Demodulator: Send {
    /// Consume channel-rate IQ, append audio samples at [`Self::audio_rate`].
    fn process(&mut self, iq: &[Complex32], out: &mut Vec<f32>);
    fn set_filter(&mut self, lo_hz: f32, hi_hz: f32);
    /// Rate of the produced audio (equals the channel rate except WFM,
    /// which decimates by 4 after the discriminator).
    fn audio_rate(&self) -> f64;
    /// Post-filter, pre-AGC signal power (dBFS) for the S-meter.
    fn power_dbfs(&self) -> f32;
}

/// The channel rate a mode's demodulator wants from the DDC.
pub fn channel_target(mode: Mode) -> f64 {
    match mode {
        // Generous rate for WFM: the discriminator wraps when the composite
        // deviation exceeds ±fs/2, so ±128 kHz of margin keeps broadcast
        // peaks (±75 kHz nominal) well clear of click territory.
        Mode::Wfm => 256_000.0,
        _ => 48_000.0,
    }
}

/// Demodulator for a mode (`None` = no audio, e.g. SPEC).
pub fn make_demod(mode: Mode, channel_rate: f64) -> Option<Box<dyn Demodulator>> {
    let (lo, hi) = mode.default_filter();
    match mode {
        // FT8/FT4 and PSK/RTTY demodulate as USB; the digi engine taps this audio.
        Mode::Lsb | Mode::Usb | Mode::Cw | Mode::Digu | Mode::Digl | Mode::Dsb
        | Mode::Ft8 | Mode::Ft4 | Mode::Psk | Mode::Rtty => {
            Some(Box::new(SsbDemod::new(channel_rate, lo, hi)))
        }
        Mode::Am => Some(Box::new(AmDemod::new(channel_rate, lo, hi))),
        Mode::Sam => Some(Box::new(SamDemod::new(channel_rate, lo, hi))),
        Mode::Nfm => Some(Box::new(FmDemod::new(channel_rate, lo, hi))),
        Mode::Wfm => Some(Box::new(WfmDemod::new(channel_rate))),
        Mode::Spec => None,
    }
}

/// Smoothed power tracker shared by all demods.
struct PowerMeter {
    mean_sq: f32,
}

impl PowerMeter {
    fn new() -> Self {
        PowerMeter { mean_sq: 0.0 }
    }

    fn update(&mut self, filtered: &[Complex32]) {
        if filtered.is_empty() {
            return;
        }
        let p: f32 = filtered.iter().map(|z| z.norm_sqr()).sum::<f32>() / filtered.len() as f32;
        self.mean_sq += 0.3 * (p - self.mean_sq);
    }

    fn dbfs(&self) -> f32 {
        10.0 * (self.mean_sq + 1e-20).log10()
    }
}

/// SSB/CW/digital: complex band-pass, take the real part.
pub struct SsbDemod {
    rate: f64,
    fir: ComplexFir,
    filtered: Vec<Complex32>,
    power: PowerMeter,
}

impl SsbDemod {
    pub fn new(rate: f64, lo: f32, hi: f32) -> Self {
        SsbDemod {
            rate,
            fir: ComplexFir::new(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, rate)),
            filtered: Vec::new(),
            power: PowerMeter::new(),
        }
    }
}

impl Demodulator for SsbDemod {
    fn process(&mut self, iq: &[Complex32], out: &mut Vec<f32>) {
        self.filtered.clear();
        self.fir.process(iq, &mut self.filtered);
        self.power.update(&self.filtered);
        out.extend(self.filtered.iter().map(|z| z.re * 2.0));
    }

    fn set_filter(&mut self, lo: f32, hi: f32) {
        self.fir.set_taps(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, self.rate));
    }

    fn audio_rate(&self) -> f64 {
        self.rate
    }

    fn power_dbfs(&self) -> f32 {
        self.power.dbfs()
    }
}

/// Single-pole DC blocker with a rate-aware corner frequency.
pub struct DcBlock {
    r: f32,
    x1: f32,
    y1: f32,
}

impl DcBlock {
    pub fn new(cutoff_hz: f64, sample_rate: f64) -> Self {
        let r = (1.0 - std::f64::consts::TAU * cutoff_hz / sample_rate).clamp(0.9, 0.999_999);
        DcBlock { r: r as f32, x1: 0.0, y1: 0.0 }
    }

    #[inline]
    pub fn run(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

/// AM: envelope detector after the band-pass, DC blocked.
pub struct AmDemod {
    rate: f64,
    fir: ComplexFir,
    dc: DcBlock,
    filtered: Vec<Complex32>,
    power: PowerMeter,
}

impl AmDemod {
    pub fn new(rate: f64, lo: f32, hi: f32) -> Self {
        AmDemod {
            rate,
            fir: ComplexFir::new(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, rate)),
            dc: DcBlock::new(20.0, rate),
            filtered: Vec::new(),
            power: PowerMeter::new(),
        }
    }
}

impl Demodulator for AmDemod {
    fn process(&mut self, iq: &[Complex32], out: &mut Vec<f32>) {
        self.filtered.clear();
        self.fir.process(iq, &mut self.filtered);
        self.power.update(&self.filtered);
        out.extend(self.filtered.iter().map(|z| self.dc.run(z.norm())));
    }

    fn set_filter(&mut self, lo: f32, hi: f32) {
        self.fir.set_taps(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, self.rate));
    }

    fn audio_rate(&self) -> f64 {
        self.rate
    }

    fn power_dbfs(&self) -> f32 {
        self.power.dbfs()
    }
}

/// Synchronous AM: a 2nd-order PLL locks the carrier, then coherent
/// detection (real part of the de-rotated signal).
pub struct SamDemod {
    rate: f64,
    fir: ComplexFir,
    dc: DcBlock,
    phase: f64,
    freq: f64,
    alpha: f64,
    beta: f64,
    max_freq: f64,
    filtered: Vec<Complex32>,
    power: PowerMeter,
}

impl SamDemod {
    pub fn new(rate: f64, lo: f32, hi: f32) -> Self {
        // Loop natural frequency 100 Hz, damping 0.707.
        let wn = std::f64::consts::TAU * 100.0 / rate;
        SamDemod {
            rate,
            fir: ComplexFir::new(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, rate)),
            dc: DcBlock::new(20.0, rate),
            phase: 0.0,
            freq: 0.0,
            alpha: 2.0 * 0.707 * wn,
            beta: wn * wn,
            max_freq: std::f64::consts::TAU * 1_000.0 / rate,
            filtered: Vec::new(),
            power: PowerMeter::new(),
        }
    }
}

impl Demodulator for SamDemod {
    fn process(&mut self, iq: &[Complex32], out: &mut Vec<f32>) {
        self.filtered.clear();
        self.fir.process(iq, &mut self.filtered);
        self.power.update(&self.filtered);

        for &z in &self.filtered {
            let r = Complex32::new(self.phase.cos() as f32, -(self.phase.sin() as f32));
            let v = z * r;
            let err = (v.im as f64).atan2((v.re as f64).abs().max(1e-12));
            self.freq = (self.freq + self.beta * err).clamp(-self.max_freq, self.max_freq);
            self.phase += self.freq + self.alpha * err;
            self.phase %= std::f64::consts::TAU;
            out.push(self.dc.run(v.re));
        }
    }

    fn set_filter(&mut self, lo: f32, hi: f32) {
        self.fir.set_taps(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, self.rate));
    }

    fn audio_rate(&self) -> f64 {
        self.rate
    }

    fn power_dbfs(&self) -> f32 {
        self.power.dbfs()
    }
}

/// NFM: quadrature discriminator, scaled for ±5 kHz deviation, DC blocked
/// (off-tune carrier offset), audio low-pass.
pub struct FmDemod {
    rate: f64,
    fir: ComplexFir,
    lpf: RealFir,
    dc: DcBlock,
    prev: Complex32,
    scale: f32,
    filtered: Vec<Complex32>,
    raw_audio: Vec<f32>,
    power: PowerMeter,
}

impl FmDemod {
    pub fn new(rate: f64, lo: f32, hi: f32) -> Self {
        FmDemod {
            rate,
            fir: ComplexFir::new(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, rate)),
            lpf: RealFir::lowpass(63, 3600.0, rate),
            dc: DcBlock::new(20.0, rate),
            prev: Complex32::new(1.0, 0.0),
            scale: (rate / (std::f64::consts::TAU * 5_000.0)) as f32,
            filtered: Vec::new(),
            raw_audio: Vec::new(),
            power: PowerMeter::new(),
        }
    }
}

impl Demodulator for FmDemod {
    fn process(&mut self, iq: &[Complex32], out: &mut Vec<f32>) {
        self.filtered.clear();
        self.fir.process(iq, &mut self.filtered);
        self.power.update(&self.filtered);

        self.raw_audio.clear();
        for &z in &self.filtered {
            let d = z * self.prev.conj();
            self.prev = z;
            self.raw_audio.push(self.dc.run(d.arg() * self.scale));
        }
        self.lpf.process(&self.raw_audio, out);
    }

    fn set_filter(&mut self, lo: f32, hi: f32) {
        self.fir.set_taps(bandpass_taps(PASSBAND_TAPS, lo as f64, hi as f64, self.rate));
    }

    fn audio_rate(&self) -> f64 {
        self.rate
    }

    fn power_dbfs(&self) -> f32 {
        self.power.dbfs()
    }
}

/// WFM broadcast: wide discriminator at the ~256 kHz channel rate, ±75 kHz
/// deviation, DC blocked (off-tune offset), 50 µs de-emphasis, sharp 15 kHz
/// low-pass (pilot/subcarrier rejection), then decimate by 4. Mono.
pub struct WfmDemod {
    rate: f64,
    fir: ComplexFir,
    lpf: RealFir,
    dc: DcBlock,
    prev: Complex32,
    scale: f32,
    deemph_state: f32,
    deemph_alpha: f32,
    decim_phase: usize,
    filtered: Vec<Complex32>,
    raw_audio: Vec<f32>,
    lp_audio: Vec<f32>,
    power: PowerMeter,
}

/// Leave headroom below full scale: broadcast processing regularly pushes
/// peaks to (and past) nominal deviation.
const WFM_HEADROOM: f32 = 0.7;

impl WfmDemod {
    pub fn new(rate: f64) -> Self {
        let bw = (rate * 0.45).min(110_000.0);
        WfmDemod {
            rate,
            // Fewer taps: the passband is nearly the whole channel.
            fir: ComplexFir::new(bandpass_taps(63, -bw, bw, rate)),
            // 255 taps → transition ≈ 8 kHz at 256 k: the 19 kHz pilot and
            // 38 kHz subcarrier are gone before decimation.
            lpf: RealFir::lowpass(255, 15_000.0, rate),
            dc: DcBlock::new(5.0, rate),
            prev: Complex32::new(1.0, 0.0),
            scale: (rate / (std::f64::consts::TAU * 75_000.0)) as f32,
            deemph_state: 0.0,
            deemph_alpha: 1.0 - (-1.0 / (rate * 50e-6)).exp() as f32,
            decim_phase: 0,
            filtered: Vec::new(),
            raw_audio: Vec::new(),
            lp_audio: Vec::new(),
            power: PowerMeter::new(),
        }
    }
}

impl Demodulator for WfmDemod {
    fn process(&mut self, iq: &[Complex32], out: &mut Vec<f32>) {
        self.filtered.clear();
        self.fir.process(iq, &mut self.filtered);
        self.power.update(&self.filtered);

        self.raw_audio.clear();
        for &z in &self.filtered {
            let d = z * self.prev.conj();
            self.prev = z;
            let audio = self.dc.run(d.arg() * self.scale) * WFM_HEADROOM;
            self.deemph_state += self.deemph_alpha * (audio - self.deemph_state);
            self.raw_audio.push(self.deemph_state);
        }

        self.lp_audio.clear();
        self.lpf.process(&self.raw_audio, &mut self.lp_audio);
        // Decimate by 4, phase-continuous across blocks.
        let mut i = self.decim_phase;
        while i < self.lp_audio.len() {
            out.push(self.lp_audio[i]);
            i += 4;
        }
        self.decim_phase = i - self.lp_audio.len();
    }

    fn set_filter(&mut self, _lo: f32, _hi: f32) {
        // WFM bandwidth is fixed by the broadcast standard.
    }

    fn audio_rate(&self) -> f64 {
        self.rate / 4.0
    }

    fn power_dbfs(&self) -> f32 {
        self.power.dbfs()
    }
}
