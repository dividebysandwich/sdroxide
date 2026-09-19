//! HF weather facsimile (WEFAX / radiofax) receiver.
//!
//! The charts the meteorological services broadcast on short wave — surface
//! analyses, wave heights, ice edges, satellite composites — are sent as
//! frequency-modulated facsimile inside an SSB channel, to a scheme that has
//! not changed since the 1970s and is documented in ITU-R Recommendation
//! M.1170 and in every navy's radio-facsimile schedule.
//!
//! # The signal
//!
//! The picture is a subcarrier that swings between two frequencies: **1500 Hz
//! is black, 2300 Hz is white**, centred on 1900 Hz with ±400 Hz of deviation.
//! That is the same discriminator geometry as SSTV, which is why this module
//! and [`crate::sstv`] demodulate identically and diverge only afterwards.
//!
//! What arrives is a continuous raster, not a framed image:
//!
//! * a **start tone** — the black/white subcarrier alternating at 300 Hz (for
//!   [`Ioc::I576`]) or 675 Hz ([`Ioc::I288`]) for about five seconds, which is
//!   what says a transmission is beginning and at which index of cooperation;
//! * a **phasing signal** — around thirty seconds of lines that are black
//!   except for a white pulse occupying the last 5 % of each. The pulse is the
//!   only thing that says where a line *starts*; without it the chart arrives
//!   correctly but cut vertically and wrapped, which is the classic radiofax
//!   failure;
//! * the **picture**, at 60, 90, 120, 180 or 240 lines per minute — 120 is what
//!   nearly every service uses for charts;
//! * a **stop tone**, the subcarrier alternating at 450 Hz for five seconds.
//!
//! Line length comes from the *index of cooperation*: pixels per line is
//! `IOC × π`, so IOC 576 gives 1809 and IOC 288 gives 904. Nothing in the
//! signal states the height — a chart runs for as long as the station keeps
//! sending, which is why the receiver ends on the stop tone or on the
//! operator's say-so rather than at a known row count.
//!
//! # Why the timing is the hard part
//!
//! A line at 120 LPM is half a second, which at a 12 kHz tap is six thousand
//! samples for 1809 pixels. An error of one part in ten thousand in the assumed
//! sample rate — well within a sound card's tolerance — walks the image sideways
//! by most of a line over a fifteen-minute chart. That is what [`WefaxRx::set_slant_ppm`]
//! trims, and why the pixel clock here is accumulated in fractional samples and
//! never by adding a rounded per-line integer.

use std::f64::consts::{PI, TAU};

use num_complex::Complex32;

use crate::fir::{ComplexFir, bandpass_taps};

/// Subcarrier centre, Hz. Black is 400 Hz below it and white 400 Hz above.
pub const CENTRE_HZ: f64 = 1900.0;
/// Black frequency, Hz.
pub const BLACK_HZ: f64 = 1500.0;
/// White frequency, Hz.
pub const WHITE_HZ: f64 = 2300.0;

/// Index of cooperation: the drum-facsimile parameter that survives as the
/// definition of how many pixels there are in a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ioc {
    /// 576 — every weather chart. 1809 pixels per line.
    #[default]
    I576,
    /// 288 — the half-resolution mode, and what the older satellite-imagery
    /// schedules use. 904 pixels per line.
    I288,
}

impl Ioc {
    pub const ALL: [Ioc; 2] = [Ioc::I576, Ioc::I288];

    pub fn value(self) -> u32 {
        match self {
            Ioc::I576 => 576,
            Ioc::I288 => 288,
        }
    }

    /// Pixels per line, `IOC × π` truncated.
    ///
    /// Truncated rather than rounded because that is what the figures every
    /// schedule and every other decoder quotes come from: 576 π is 1809.56 and
    /// the line is 1809 pixels, 288 π is 904.78 and the line is 904.
    pub fn width(self) -> u16 {
        (self.value() as f64 * PI) as u16
    }

    /// The start tone that announces this index of cooperation, Hz.
    pub fn start_tone_hz(self) -> f64 {
        match self {
            Ioc::I576 => 300.0,
            Ioc::I288 => 675.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Ioc::I576 => "IOC 576",
            Ioc::I288 => "IOC 288",
        }
    }
}

/// Lines per minute. 120 is what the weather services broadcast charts at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lpm {
    L60,
    L90,
    #[default]
    L120,
    L180,
    L240,
}

impl Lpm {
    pub const ALL: [Lpm; 5] = [Lpm::L60, Lpm::L90, Lpm::L120, Lpm::L180, Lpm::L240];

    pub fn value(self) -> f64 {
        match self {
            Lpm::L60 => 60.0,
            Lpm::L90 => 90.0,
            Lpm::L120 => 120.0,
            Lpm::L180 => 180.0,
            Lpm::L240 => 240.0,
        }
    }

    /// Seconds per scan line.
    pub fn line_secs(self) -> f64 {
        60.0 / self.value()
    }

    pub fn label(self) -> &'static str {
        match self {
            Lpm::L60 => "60 LPM",
            Lpm::L90 => "90 LPM",
            Lpm::L120 => "120 LPM",
            Lpm::L180 => "180 LPM",
            Lpm::L240 => "240 LPM",
        }
    }
}

/// The stop tone, Hz — the same whatever the index of cooperation.
pub const STOP_TONE_HZ: f64 = 450.0;

/// Longest chart the receiver will build before ending it itself.
///
/// A stop tone is what normally ends a transmission, but it is the first thing
/// lost to a fade, and a receiver that kept appending rows to a dead carrier
/// would grow an image without limit. Twenty minutes at 120 LPM is 2400 lines,
/// comfortably longer than any scheduled chart.
pub const MAX_LINES: u16 = 2600;

/// What the receiver reports as it goes.
#[derive(Debug, Clone, PartialEq)]
pub enum WefaxEvent {
    /// A transmission has begun; a new image starts. `auto` distinguishes a
    /// start tone from the operator pressing START.
    Start { ioc: Ioc, lpm: Lpm, auto: bool },
    /// The phasing pulse has been located, `skew` samples into the line.
    Phased { skew: u32 },
    /// A finished scan line: `gray` is [`Ioc::width`] bytes, 0 black … 255 white.
    Line { y: u16, gray: Vec<u8> },
    /// The transmission ended. `by_tone` distinguishes a stop tone from the
    /// operator, a fade, or the length limit.
    Complete { lines: u16, by_tone: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Not receiving. Listening for a start tone if auto-start is on.
    Idle,
    /// Receiving, but still looking for the phasing pulse that says where a
    /// line begins.
    Phasing,
    /// Building the picture.
    Image,
}

/// Detector for the alternating black/white tones that bracket a transmission.
///
/// The start and stop signals are not audio tones — they are the *picture*
/// alternating between black and white at 300, 675 or 450 Hz. So they are
/// measured after the discriminator, on the demodulated brightness, by counting
/// how often it crosses mid-grey. A 300 Hz square wave crosses 600 times a
/// second; nothing in a weather chart does that for seconds on end.
#[derive(Debug, Default)]
struct ToneDetect {
    /// Crossings seen in the window so far, and how long the window has run.
    crossings: u32,
    samples: u32,
    above: bool,
    /// Consecutive windows whose rate matched, in units of the window length.
    matched: f32,
}

impl ToneDetect {
    /// Feed one demodulated sample; returns the measured crossing rate once a
    /// window completes.
    fn push(&mut self, level: f32, window: u32) -> Option<f64> {
        let above = level > 0.5;
        // Hysteresis-free is fine here: the start signal is full-swing black to
        // white, and mid-grey noise between windows costs at most one crossing.
        if above != self.above {
            self.crossings += 1;
            self.above = above;
        }
        self.samples += 1;
        if self.samples < window {
            return None;
        }
        let rate = self.crossings as f64;
        self.crossings = 0;
        self.samples = 0;
        Some(rate)
    }

    fn reset(&mut self) {
        self.crossings = 0;
        self.samples = 0;
        self.matched = 0.0;
    }
}

/// The weather-fax receiver.
pub struct WefaxRx {
    rate: f64,

    // Discriminator, identical in shape to the SSTV one: down-mix to complex
    // baseband at the subcarrier centre, low-pass, then differentiate the phase.
    mix_ph: f32,
    mix_inc: f32,
    lpf: ComplexFir,
    prev: Complex32,
    have_prev: bool,
    inst_hz: f64,
    in_level: f32,

    // Settings.
    ioc: Ioc,
    lpm: Lpm,
    auto_start: bool,
    auto_stop: bool,
    /// Sample-clock trim in parts per million. Positive stretches the line,
    /// which corrects an image slanting to the left.
    slant_ppm: f64,

    phase: Phase,
    /// Demodulated brightness of the line being built, one entry per input
    /// sample since the line began.
    line_buf: Vec<f32>,
    /// Fractional sample position where the current line started, so the line
    /// clock never accumulates rounding error.
    line_start: f64,
    line_no: u16,
    sample_idx: u64,

    // Phasing.
    /// Phasing lines examined so far, and the running skew estimate in samples.
    phase_lines: u32,
    phase_skew: f64,
    /// Samples spent phasing, so a transmission whose phasing signal was missed
    /// still produces a (misaligned) chart rather than nothing.
    phase_samples: u64,
    /// Consecutive completed lines that looked like a phasing signal *while a
    /// picture was being built*, and where the last one's pulse sat. See
    /// [`WefaxRx::note_phasing_line`].
    rephase_lines: u32,
    rephase_skew: f64,

    start_det: ToneDetect,
    stop_det: ToneDetect,
    /// Consecutive matching windows, for start and stop respectively.
    start_run: u32,
    stop_run: u32,
    /// Which IOC the start tone that armed us implied.
    start_ioc: Ioc,
}

/// How long a tone-detector window is, in seconds. Long enough that a chart's
/// own black/white detail cannot fill one, short enough that several fit in the
/// five-second start signal.
const TONE_WINDOW_S: f64 = 0.5;
/// Matching windows needed to accept a start signal while *idle* (≈1.5 s).
const TONE_RUN: u32 = 3;
/// Matching windows needed to accept a start signal heard *while a picture is
/// being drawn* (≈3 s).
///
/// A real start signal is five seconds long and is the second way to learn that
/// a page is over — the stop tone, the first way, is the first thing a fade
/// takes. But a chart's own high-contrast detail can hold a start-tone-looking
/// rate for a window or two, and each false accept cuts a healthy picture in
/// two (issue #496). Asking for twice the evidence mid-picture still catches
/// the real five-second signal with two seconds to spare, and a fading chart
/// is far less likely to fake three seconds than one and a half.
const START_RUN_RECEIVING: u32 = 6;
/// Matching windows needed to accept a stop signal (≈3.5 s). A stop tone is
/// five seconds long; requiring most of it keeps a noise burst or a fade from
/// ending a chart whose signal is merely dipping (issue #496).
const STOP_RUN: u32 = 7;
/// How far a measured crossing rate may be from the nominal one, as a fraction.
const TONE_TOL: f64 = 0.18;
/// Give up looking for the phasing pulse after this long and start the picture
/// anyway — a misaligned chart beats no chart.
const PHASE_TIMEOUT_S: f64 = 35.0;
/// Phasing lines to average the skew over before locking.
const PHASE_LINES: u32 = 5;
/// Phasing-looking lines in a row, in the middle of a picture, that mean the
/// next transmission has begun on top of it.
///
/// Eight is four seconds at 120 LPM — short enough to lose almost none of the
/// new chart's thirty-second phasing signal, long enough that a chart's own
/// graphic has to hold the same shape for four seconds to fake it. Four was
/// not (issue #496): a dark image with one bright feature in a fixed place
/// passes the shape test line after line, and two seconds of it was cutting
/// pictures up.
const REPHASE_LINES: u32 = 8;
/// How far two phasing lines' pulses may sit apart, as a fraction of a line,
/// and still be read as the same phasing signal. The line clock drifts by parts
/// per million between lines; a picture that happened to look like phasing twice
/// running would not put its white patch in the same place twice.
const REPHASE_TOL: f64 = 0.02;

impl WefaxRx {
    pub fn new(rate: f64) -> Self {
        WefaxRx {
            rate,
            mix_ph: 0.0,
            mix_inc: (TAU as f32) * CENTRE_HZ as f32 / rate as f32,
            // ±1100 Hz passes the ±400 Hz deviation with room for a badly
            // tuned receiver, and rejects the rest of the SSB channel.
            lpf: ComplexFir::new(bandpass_taps(129, -1100.0, 1100.0, rate)),
            prev: Complex32::new(0.0, 0.0),
            have_prev: false,
            inst_hz: CENTRE_HZ,
            in_level: 0.0,
            ioc: Ioc::default(),
            lpm: Lpm::default(),
            auto_start: true,
            auto_stop: true,
            slant_ppm: 0.0,
            phase: Phase::Idle,
            line_buf: Vec::new(),
            line_start: 0.0,
            line_no: 0,
            sample_idx: 0,
            phase_lines: 0,
            phase_skew: 0.0,
            phase_samples: 0,
            rephase_lines: 0,
            rephase_skew: 0.0,
            start_det: ToneDetect::default(),
            stop_det: ToneDetect::default(),
            start_run: 0,
            stop_run: 0,
            start_ioc: Ioc::default(),
        }
    }

    pub fn ioc(&self) -> Ioc {
        self.ioc
    }

    pub fn lpm(&self) -> Lpm {
        self.lpm
    }

    /// Set the index of cooperation. Ignored mid-picture: changing the line
    /// length halfway would leave the top and the bottom of one image at
    /// different widths.
    pub fn set_ioc(&mut self, ioc: Ioc) {
        if self.phase == Phase::Idle {
            self.ioc = ioc;
        }
    }

    pub fn set_lpm(&mut self, lpm: Lpm) {
        if self.phase == Phase::Idle {
            self.lpm = lpm;
        }
    }

    /// Whether a start tone begins a picture on its own.
    pub fn set_auto_start(&mut self, on: bool) {
        self.auto_start = on;
    }

    /// Whether a stop tone ends one.
    pub fn set_auto_stop(&mut self, on: bool) {
        self.auto_stop = on;
    }

    /// Sample-clock trim, parts per million.
    pub fn set_slant_ppm(&mut self, ppm: f32) {
        self.slant_ppm = ppm as f64;
    }

    pub fn slant_ppm(&self) -> f32 {
        self.slant_ppm as f32
    }

    /// Smoothed input level, for the panel's tuning meter.
    pub fn level(&self) -> f32 {
        self.in_level
    }

    /// Instantaneous subcarrier frequency, for the tuning indicator: a
    /// correctly tuned receiver puts a black-and-white chart's excursions
    /// symmetrically about 1900 Hz.
    pub fn subcarrier_hz(&self) -> f64 {
        self.inst_hz
    }

    pub fn receiving(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// True while still hunting for the phasing pulse.
    pub fn phasing(&self) -> bool {
        self.phase == Phase::Phasing
    }

    /// Lines emitted for the picture in progress.
    pub fn lines(&self) -> u16 {
        self.line_no
    }

    /// Begin a picture now, without waiting for a start tone.
    ///
    /// The usual way to catch a chart that was already running when you tuned
    /// to it — which, on a fifteen-minute transmission, is most of the time.
    pub fn start_manual(&mut self, out: &mut Vec<WefaxEvent>) {
        if self.phase != Phase::Idle {
            return;
        }
        self.begin(self.ioc, false, out);
    }

    /// End the picture in progress, keeping what has arrived so far.
    pub fn stop_manual(&mut self, out: &mut Vec<WefaxEvent>) {
        self.finish(false, out);
    }

    /// Nudge the line alignment by `pixels`, for a chart whose phasing pulse
    /// was missed or misread. Positive moves the image right.
    pub fn nudge_phase(&mut self, pixels: i32) {
        let spp = self.samples_per_line() / self.ioc.width() as f64;
        self.line_start -= pixels as f64 * spp;
    }

    /// Samples in one scan line, including the slant trim.
    fn samples_per_line(&self) -> f64 {
        self.rate * self.lpm.line_secs() * (1.0 + self.slant_ppm * 1e-6)
    }

    fn begin(&mut self, ioc: Ioc, auto: bool, out: &mut Vec<WefaxEvent>) {
        self.ioc = ioc;
        self.phase = Phase::Phasing;
        self.line_buf.clear();
        self.line_start = self.sample_idx as f64;
        self.line_no = 0;
        self.phase_lines = 0;
        self.phase_skew = 0.0;
        self.phase_samples = 0;
        self.rephase_lines = 0;
        self.start_det.reset();
        self.start_run = 0;
        self.stop_det.reset();
        self.stop_run = 0;
        out.push(WefaxEvent::Start { ioc, lpm: self.lpm, auto });
    }

    fn finish(&mut self, by_tone: bool, out: &mut Vec<WefaxEvent>) {
        if self.phase == Phase::Idle {
            return;
        }
        let lines = self.line_no;
        self.phase = Phase::Idle;
        self.line_buf.clear();
        self.rephase_lines = 0;
        self.start_det.reset();
        self.stop_det.reset();
        self.start_run = 0;
        self.stop_run = 0;
        out.push(WefaxEvent::Complete { lines, by_tone });
    }

    /// Feed audio; push any decoded events.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<WefaxEvent>) {
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            self.in_level += 0.001 * (a.abs() - self.in_level);
            let z = Complex32::new(a * self.mix_ph.cos(), -a * self.mix_ph.sin());
            self.mix_ph += self.mix_inc;
            if self.mix_ph > std::f32::consts::TAU {
                self.mix_ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        for z in bb {
            let raw_hz = if self.have_prev {
                let d = z * self.prev.conj();
                CENTRE_HZ + (d.arg() as f64) * self.rate / TAU
            } else {
                CENTRE_HZ
            };
            self.prev = z;
            self.have_prev = true;
            // Light smoothing: enough to take the edge off the discriminator's
            // noise without rounding the black/white transitions the phasing
            // pulse is found by.
            self.inst_hz += 0.5 * (raw_hz - self.inst_hz);

            let level = brightness(self.inst_hz);
            self.sample_idx += 1;

            match self.phase {
                Phase::Idle => self.step_idle(level, out),
                Phase::Phasing | Phase::Image => self.step_receiving(level, out),
            }
        }
    }

    /// Listening for a start tone.
    fn step_idle(&mut self, level: f32, out: &mut Vec<WefaxEvent>) {
        if !self.auto_start {
            return;
        }
        if let Some(ioc) = self.start_tone(level, TONE_RUN) {
            self.begin(ioc, true, out);
        }
    }

    /// Feed the start-signal detector one sample; `Some(ioc)` once a whole
    /// start signal has been heard. `required` matching windows are demanded,
    /// because how much evidence a start signal needs depends on what else is
    /// on the line: see [`TONE_RUN`] and [`START_RUN_RECEIVING`].
    ///
    /// Run while receiving as well as while idle: the next transmission's start
    /// signal is the second way to learn that this picture is over, and the
    /// stop tone — the first way — is also the first thing a fade takes.
    fn start_tone(&mut self, level: f32, required: u32) -> Option<Ioc> {
        let window = (self.rate * TONE_WINDOW_S) as u32;
        let rate = self.start_det.push(level, window)?;
        // Crossings per window → crossings per second → the alternation rate,
        // which is half the crossing rate.
        let alt_hz = rate / TONE_WINDOW_S / 2.0;
        let hit = Ioc::ALL.into_iter().find(|i| {
            let want = i.start_tone_hz();
            (alt_hz - want).abs() <= want * TONE_TOL
        });
        match hit {
            Some(ioc) if ioc == self.start_ioc => {
                self.start_run += 1;
                if self.start_run >= required {
                    self.start_run = 0;
                    return Some(ioc);
                }
            }
            Some(ioc) => {
                self.start_ioc = ioc;
                self.start_run = 1;
            }
            None => self.start_run = 0,
        }
        None
    }

    /// Whether `line` continues a run of phasing lines long enough to mean the
    /// next transmission has started on top of the picture being built.
    ///
    /// The pulses have to line up. One dark line with a bright patch in it is
    /// something a satellite image or a heavily inked chart can produce; four
    /// in a row with the patch in the same place, to within the line clock's own
    /// drift, is a phasing signal.
    fn note_phasing_line(&mut self, line: &[f32]) -> bool {
        // The strict test: mid-picture, a line that is merely "mostly black
        // with a white patch" is a chart's own graphic, not a phasing pulse.
        let Some(skew) = phasing_skew_strict(line) else {
            self.rephase_lines = 0;
            return false;
        };
        let (skew, n) = (skew as f64, line.len() as f64);
        // Circular: a pulse sitting on the line boundary reads as ≈0 on one
        // line and ≈n on the next, and that is the same place.
        let d = (skew - self.rephase_skew).abs();
        let moved = d.min(n - d) > n * REPHASE_TOL;
        self.rephase_lines =
            if self.rephase_lines > 0 && moved { 1 } else { self.rephase_lines + 1 };
        self.rephase_skew = skew;
        self.rephase_lines >= REPHASE_LINES
    }

    /// Receiving: accumulate the line, watch for the stop tone — and for the
    /// next transmission arriving on top of this one.
    fn step_receiving(&mut self, level: f32, out: &mut Vec<WefaxEvent>) {
        self.line_buf.push(level);

        // The next chart's start signal, heard while this one is still being
        // drawn. A stop tone is how a transmission normally ends, and a
        // receiver that misses one — a fade takes five seconds of tone whole,
        // and not every station sends the same one — used to go on appending
        // the *next* chart to the old page on the old line phase, which is a
        // picture that can never come out straight (issue #276). Finish the
        // page here instead and phase the new one properly.
        if self.auto_start
            && let Some(ioc) = self.start_tone(level, START_RUN_RECEIVING)
        {
            self.finish(false, out);
            self.begin(ioc, true, out);
            return;
        }

        if self.auto_stop {
            let window = (self.rate * TONE_WINDOW_S) as u32;
            if let Some(rate) = self.stop_det.push(level, window) {
                let alt_hz = rate / TONE_WINDOW_S / 2.0;
                if (alt_hz - STOP_TONE_HZ).abs() <= STOP_TONE_HZ * TONE_TOL {
                    self.stop_run += 1;
                    if self.stop_run >= STOP_RUN {
                        // The stop tone is already in the last line or two;
                        // they are dropped rather than left as a barcode at the
                        // bottom of the chart.
                        self.line_no = self.line_no.saturating_sub(2);
                        self.finish(true, out);
                        return;
                    }
                } else {
                    self.stop_run = 0;
                }
            }
        }

        let spl = self.samples_per_line();
        let end = self.line_start + spl;
        if (self.sample_idx as f64) < end {
            return;
        }

        // One line's worth of brightness is in hand.
        let line: Vec<f32> = std::mem::take(&mut self.line_buf);
        self.line_start = end;

        match self.phase {
            Phase::Phasing => {
                self.phase_samples += line.len() as u64;
                if let Some(skew) = phasing_skew(&line) {
                    self.phase_skew += skew as f64;
                    self.phase_lines += 1;
                    if self.phase_lines >= PHASE_LINES {
                        // The pulse sits at the *end* of a phasing line, so the
                        // picture starts just after it.
                        let mean = (self.phase_skew / self.phase_lines as f64).round();
                        self.line_start += mean;
                        self.phase = Phase::Image;
                        out.push(WefaxEvent::Phased { skew: mean.max(0.0) as u32 });
                    }
                }
                if self.phase == Phase::Phasing
                    && self.phase_samples as f64 > PHASE_TIMEOUT_S * self.rate
                {
                    // No usable pulse: start anyway rather than sit on a chart
                    // that is already halfway through.
                    self.phase = Phase::Image;
                    out.push(WefaxEvent::Phased { skew: 0 });
                }
            }
            Phase::Image => {
                // The other half of the same rule: the start signal is five
                // seconds long and a fade can swallow it whole, while the
                // phasing signal that follows it runs for thirty. A run of
                // phasing lines in the middle of a picture is the next
                // transmission, whether its start tone arrived or not.
                if self.auto_start && self.note_phasing_line(&line) {
                    self.finish(false, out);
                    self.begin(self.ioc, true, out);
                    return;
                }
                let gray = resample_line(&line, self.ioc.width() as usize);
                out.push(WefaxEvent::Line { y: self.line_no, gray });
                self.line_no = self.line_no.saturating_add(1);
                if self.line_no >= MAX_LINES {
                    self.finish(false, out);
                }
            }
            Phase::Idle => {}
        }
    }
}

/// Map a subcarrier frequency to brightness, 0.0 black … 1.0 white.
fn brightness(hz: f64) -> f32 {
    (((hz - BLACK_HZ) / (WHITE_HZ - BLACK_HZ)) as f32).clamp(0.0, 1.0)
}

/// Average a line's samples down to `width` pixels.
///
/// A box average rather than a pick: at 120 LPM and a 12 kHz tap there are
/// three or four samples per pixel, and taking one of them throws away most of
/// the signal-to-noise the wide-band discriminator just earned.
fn resample_line(line: &[f32], width: usize) -> Vec<u8> {
    let mut out = vec![0u8; width];
    if line.is_empty() {
        return out;
    }
    let step = line.len() as f64 / width as f64;
    for (x, px) in out.iter_mut().enumerate() {
        let a = (x as f64 * step) as usize;
        let b = (((x + 1) as f64 * step) as usize).max(a + 1).min(line.len());
        let sum: f32 = line[a..b].iter().sum();
        let mean = sum / (b - a) as f32;
        *px = (mean * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Where the phasing pulse ends, in samples from the start of `line`.
///
/// A phasing line is black apart from a white pulse occupying about 5 % of it.
/// Correlating against that shape — rather than hunting for an edge — is what
/// makes this survive a noisy line: a single spike cannot move a correlation
/// taken over a twentieth of the line, and the returned offset is the amount
/// the line clock has to be advanced to put the pulse at the end where it
/// belongs.
///
/// `None` when the line does not look like a phasing line at all, which is how
/// a picture that is already under way falls through to the timeout.
fn phasing_skew(line: &[f32]) -> Option<u32> {
    phasing_skew_with(line, false)
}

/// The same, with the extra evidence a *mid-picture* rephase has to demand.
///
/// A chart's own graphic can be mostly black with one bright patch, and a patch
/// that does not move is in the same place line after line — which is the whole
/// of what the lenient test looks for, and enough to cut a picture in two
/// (issue #496). So the strict test also requires the line to be almost black,
/// the patch to be nearly white, and — the discriminator a chart cannot fake
/// without a border — the patch to sit at the *end* of the line, where a real
/// phasing pulse belongs.
///
/// Used only for the mid-picture rephase. Phasing proper uses the lenient test,
/// because there the pulse is expected and a noisy line should still align.
fn phasing_skew_strict(line: &[f32]) -> Option<u32> {
    phasing_skew_with(line, true)
}

fn phasing_skew_with(line: &[f32], strict: bool) -> Option<u32> {
    let n = line.len();
    let pulse = (n as f64 * 0.05).round() as usize;
    if n < 32 || pulse == 0 {
        return None;
    }
    // A phasing line is mostly black. If it is not, this is picture.
    let mean: f32 = line.iter().sum::<f32>() / n as f32;
    let max_mean = if strict { 0.20 } else { 0.35 };
    if mean > max_mean {
        return None;
    }
    // Running sum over a pulse-wide window, wrapping, so a pulse split across
    // the line boundary is still found in one piece.
    let mut best = (f32::MIN, 0usize);
    let mut acc: f32 = line[..pulse].iter().sum();
    for start in 0..n {
        if acc > best.0 {
            best = (acc, start);
        }
        acc -= line[start];
        acc += line[(start + pulse) % n];
    }
    // The window has to actually be white, or there was no pulse to find.
    let min_pulse = if strict { 0.75 } else { 0.55 };
    if best.0 / pulse as f32 <= min_pulse {
        return None;
    }
    // Mid-picture, the pulse has to be where a real one is: in the last few per
    // cent of the line. A chart's bright feature can be anywhere, and this is
    // what tells the two apart.
    if strict && best.1 + 2 * pulse < n {
        return None;
    }
    // The picture begins just after the pulse.
    Some(((best.1 + pulse) % n) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 12_000.0;

    /// Synthesise a WEFAX subcarrier from `levels` (0.0 black … 1.0 white), one
    /// entry per sample.
    ///
    /// Built from the *standard's* description — 1500 Hz black, 2300 Hz white,
    /// phase-continuous FM — and not from anything the decoder does, so a
    /// decoder that agreed only with itself would fail these.
    fn modulate(levels: &[f32]) -> Vec<f32> {
        let mut ph = 0.0f64;
        levels
            .iter()
            .map(|&l| {
                let hz = BLACK_HZ + (WHITE_HZ - BLACK_HZ) * l as f64;
                ph += TAU * hz / RATE;
                if ph > TAU {
                    ph -= TAU;
                }
                (ph.sin() * 0.5) as f32
            })
            .collect()
    }

    /// `secs` of the subcarrier alternating black/white at `alt_hz` — the shape
    /// of the start and stop signals.
    fn tone(alt_hz: f64, secs: f64) -> Vec<f32> {
        let n = (RATE * secs) as usize;
        let levels: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f64 / RATE;
                if (t * alt_hz).fract() < 0.5 { 1.0 } else { 0.0 }
            })
            .collect();
        modulate(&levels)
    }

    /// `count` phasing lines: black, with a white pulse in the last 5 %.
    fn phasing(lpm: Lpm, count: usize) -> Vec<f32> {
        let per = (RATE * lpm.line_secs()) as usize;
        let pulse = (per as f64 * 0.05) as usize;
        let mut levels = Vec::new();
        for _ in 0..count {
            levels.extend(std::iter::repeat_n(0.0f32, per - pulse));
            levels.extend(std::iter::repeat_n(1.0f32, pulse));
        }
        modulate(&levels)
    }

    /// A picture whose every line is a horizontal ramp black → white, which is
    /// the pattern that makes a misalignment obvious: a correctly phased decode
    /// is monotonic across the row and a wrapped one has a cliff in it.
    fn ramp_picture(lpm: Lpm, lines: usize) -> Vec<f32> {
        let per = (RATE * lpm.line_secs()) as usize;
        let mut levels = Vec::new();
        for _ in 0..lines {
            for i in 0..per {
                levels.push(i as f32 / per as f32);
            }
        }
        modulate(&levels)
    }

    /// A dark chart: every line almost entirely black with one bright patch at
    /// `x_frac` of the line — the night side of an infrared satellite image,
    /// or any chart with one heavy graphic on an otherwise black field.
    ///
    /// This is the shape that used to cut pictures up: it passes the old
    /// "mostly black with a white 5 %" test for a phasing line, and a patch
    /// fixed in place does so line after line.
    fn dark_chart(lpm: Lpm, lines: usize, x_frac: f64) -> Vec<f32> {
        let per = (RATE * lpm.line_secs()) as usize;
        let band = (per as f64 * 0.05) as usize;
        let x = (per as f64 * x_frac) as usize;
        let mut levels = Vec::new();
        for _ in 0..lines {
            let mut line = vec![0.0f32; per];
            for v in line.iter_mut().skip(x).take(band) {
                *v = 1.0;
            }
            levels.extend(line);
        }
        modulate(&levels)
    }

    /// Drive `rx` from idle into `Phase::Image`, so a test can feed it picture.
    fn into_image(rx: &mut WefaxRx, lpm: Lpm) -> Vec<WefaxEvent> {
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        let ev = drain(rx, &phasing(lpm, 8));
        assert!(rx.receiving() && !rx.phasing(), "did not reach the picture: {ev:?}");
        out
    }

    fn drain(rx: &mut WefaxRx, audio: &[f32]) -> Vec<WefaxEvent> {
        let mut out = Vec::new();
        // In blocks, the way the engine feeds it, so nothing may depend on
        // seeing a whole transmission in one call.
        for chunk in audio.chunks(1024) {
            rx.process(chunk, &mut out);
        }
        out
    }

    #[test]
    fn the_index_of_cooperation_sets_the_line_length() {
        // IOC × π, which is the definition.
        assert_eq!(Ioc::I576.width(), 1809);
        assert_eq!(Ioc::I288.width(), 904);
        assert_eq!(Ioc::I576.start_tone_hz(), 300.0);
        assert_eq!(Ioc::I288.start_tone_hz(), 675.0);
        // 120 LPM is half a second a line.
        assert_eq!(Lpm::L120.line_secs(), 0.5);
        assert_eq!(Lpm::L60.line_secs(), 1.0);
        assert!((Lpm::L240.line_secs() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn black_and_white_land_where_the_standard_puts_them() {
        assert_eq!(brightness(BLACK_HZ), 0.0);
        assert_eq!(brightness(WHITE_HZ), 1.0);
        assert!((brightness(CENTRE_HZ) - 0.5).abs() < 1e-6);
        // A badly tuned receiver is clamped rather than wrapped.
        assert_eq!(brightness(1000.0), 0.0);
        assert_eq!(brightness(3000.0), 1.0);
    }

    /// The 300 Hz start signal has to arm the receiver, and the 675 Hz one has
    /// to arm it at the other index of cooperation.
    #[test]
    fn a_start_tone_begins_a_picture_at_the_index_it_announces() {
        for ioc in Ioc::ALL {
            let mut rx = WefaxRx::new(RATE);
            let ev = drain(&mut rx, &tone(ioc.start_tone_hz(), 5.0));
            let started = ev.iter().find_map(|e| match e {
                WefaxEvent::Start { ioc, auto, .. } => Some((*ioc, *auto)),
                _ => None,
            });
            assert_eq!(started, Some((ioc, true)), "{ioc:?} start tone was not recognised");
            assert!(rx.receiving());
            assert_eq!(rx.ioc(), ioc);
        }
    }

    /// ...and ordinary picture must not.
    #[test]
    fn a_picture_is_not_mistaken_for_a_start_tone() {
        let mut rx = WefaxRx::new(RATE);
        let ev = drain(&mut rx, &ramp_picture(Lpm::L120, 20));
        assert!(!ev.iter().any(|e| matches!(e, WefaxEvent::Start { .. })), "{ev:?}");
        assert!(!rx.receiving());
        // Nor may silence.
        let mut rx = WefaxRx::new(RATE);
        let ev = drain(&mut rx, &vec![0.0f32; (RATE * 6.0) as usize]);
        assert!(!ev.iter().any(|e| matches!(e, WefaxEvent::Start { .. })), "{ev:?}");
    }

    #[test]
    fn a_stop_tone_ends_the_picture() {
        let mut rx = WefaxRx::new(RATE);
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        assert!(rx.receiving());
        let ev = drain(&mut rx, &tone(STOP_TONE_HZ, 5.0));
        let done = ev.iter().find_map(|e| match e {
            WefaxEvent::Complete { by_tone, .. } => Some(*by_tone),
            _ => None,
        });
        assert_eq!(done, Some(true), "the stop tone did not end the picture: {ev:?}");
        assert!(!rx.receiving());

        // With auto-stop off it keeps going, which is what an operator who is
        // capturing a continuous transmission wants.
        let mut rx = WefaxRx::new(RATE);
        rx.set_auto_stop(false);
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        let ev = drain(&mut rx, &tone(STOP_TONE_HZ, 5.0));
        assert!(!ev.iter().any(|e| matches!(e, WefaxEvent::Complete { .. })));
        assert!(rx.receiving());
    }

    /// The whole point of the phasing signal: the decoded rows have to come out
    /// aligned to the line the transmitter sent, not to whenever we tuned in.
    #[test]
    fn the_phasing_pulse_aligns_the_picture() {
        let lpm = Lpm::L120;
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        let mut out = Vec::new();
        rx.start_manual(&mut out);

        // Start mid-line, so an unphased decode would be visibly wrapped.
        let per = (RATE * lpm.line_secs()) as usize;
        let mut audio = modulate(&vec![0.0f32; per / 3]);
        audio.extend(phasing(lpm, 8));
        audio.extend(ramp_picture(lpm, 6));
        let ev = drain(&mut rx, &audio);

        assert!(
            ev.iter().any(|e| matches!(e, WefaxEvent::Phased { .. })),
            "never phased: {:?}",
            ev.iter().take(4).collect::<Vec<_>>()
        );
        let lines: Vec<&Vec<u8>> = ev
            .iter()
            .filter_map(|e| match e {
                WefaxEvent::Line { gray, .. } => Some(gray),
                _ => None,
            })
            .collect();
        assert!(lines.len() >= 3, "only {} lines", lines.len());

        // A correctly phased ramp rises monotonically across the row. Check the
        // middle of a settled line, away from the ends where the discriminator's
        // filter delay and the pulse itself blur things.
        let row = lines[lines.len() - 2];
        let w = row.len();
        assert_eq!(w, Ioc::I576.width() as usize);
        let (a, b) = (row[w / 4] as i32, row[3 * w / 4] as i32);
        assert!(b - a > 80, "the row is not a ramp: {a} → {b}");
        // ...and has no cliff in the middle, which is what a wrapped line is.
        let biggest_step = row[w / 8..7 * w / 8]
            .windows(2)
            .map(|p| (p[1] as i32 - p[0] as i32).abs())
            .max()
            .unwrap_or(0);
        assert!(biggest_step < 60, "the row wraps: biggest step {biggest_step}");
    }

    /// Issue #276: the next chart begins and the stop tone never arrives.
    ///
    /// A receiver that just keeps appending draws the new transmission onto the
    /// old page, on the old line phase — the picture is shifted and no amount
    /// of waiting straightens it. The phasing signal at the head of the new
    /// transmission is thirty seconds long and unmistakable, so it is enough on
    /// its own: the page ends there and the next one is phased properly.
    #[test]
    fn a_new_transmission_ends_the_page_and_re_phases_without_a_stop_tone() {
        let lpm = Lpm::L120;
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        // Auto-stop on, and no stop tone anywhere in what follows: this is the
        // fade that takes the tone, not an operator who switched it off.
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        let mut audio = phasing(lpm, 8);
        audio.extend(ramp_picture(lpm, 6));
        // The next transmission, arriving with nothing but its phasing signal.
        // A real one runs about thirty seconds — sixty lines at 120 LPM — and
        // the rephase spends eight of them recognising it before the new page
        // starts phasing afresh, so the fixture has to leave enough to phase
        // the second page from.
        audio.extend(phasing(lpm, 16));
        audio.extend(ramp_picture(lpm, 6));
        let ev = drain(&mut rx, &audio);

        let complete = ev.iter().filter(|e| matches!(e, WefaxEvent::Complete { .. })).count();
        assert_eq!(complete, 1, "the first page should have been finished: {ev:?}");
        let starts = ev.iter().filter(|e| matches!(e, WefaxEvent::Start { .. })).count();
        assert_eq!(starts, 1, "and a second page started (the manual one is not an event)");
        let phased = ev.iter().filter(|e| matches!(e, WefaxEvent::Phased { .. })).count();
        assert_eq!(phased, 2, "both pages have to be phased: {ev:?}");

        // The second page's rows are numbered from the top again, and its ramp
        // comes out unwrapped — which is the whole complaint.
        let after: Vec<&Vec<u8>> = ev
            .iter()
            .skip_while(|e| !matches!(e, WefaxEvent::Complete { .. }))
            .filter_map(|e| match e {
                WefaxEvent::Line { gray, .. } => Some(gray),
                _ => None,
            })
            .collect();
        assert!(after.len() >= 3, "only {} lines on the second page", after.len());
        let row = after[after.len() - 2];
        let w = row.len();
        let (a, b) = (row[w / 4] as i32, row[3 * w / 4] as i32);
        assert!(b - a > 80, "the second page is not a ramp: {a} → {b}");
        let biggest_step = row[w / 8..7 * w / 8]
            .windows(2)
            .map(|p| (p[1] as i32 - p[0] as i32).abs())
            .max()
            .unwrap_or(0);
        assert!(biggest_step < 60, "the second page wraps: biggest step {biggest_step}");
    }

    /// The same, the ordinary way round: the new transmission's *start* signal
    /// arrives while the old page is still being drawn.
    #[test]
    fn a_start_tone_during_a_picture_starts_the_next_one() {
        let lpm = Lpm::L120;
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        let mut audio = phasing(lpm, 8);
        audio.extend(ramp_picture(lpm, 6));
        audio.extend(tone(Ioc::I576.start_tone_hz(), 5.0));
        let ev = drain(&mut rx, &audio);
        assert!(
            ev.iter().any(|e| matches!(e, WefaxEvent::Complete { by_tone: false, .. })),
            "the page should have ended on the next start signal: {ev:?}"
        );
        assert!(rx.receiving() && rx.phasing(), "and the next one should be phasing");
    }

    /// ...but only when the operator asked for automatic starts. Someone
    /// capturing a continuous transmission with AUTO START off gets one page,
    /// however many charts go by.
    #[test]
    fn a_manual_capture_is_never_cut_in_two() {
        let lpm = Lpm::L120;
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        rx.set_auto_start(false);
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        let mut audio = phasing(lpm, 8);
        audio.extend(ramp_picture(lpm, 4));
        audio.extend(phasing(lpm, 10));
        let ev = drain(&mut rx, &audio);
        assert!(!ev.iter().any(|e| matches!(e, WefaxEvent::Complete { .. })), "{ev:?}");
        assert!(rx.receiving());
    }

    /// Issue #496: a chart's own high-contrast detail must not be read as the
    /// next transmission's phasing signal.
    ///
    /// `dark_chart` is mostly black with one bright patch per line, in a fixed
    /// place — a satellite image's night side, or a chart with a heavy graphic.
    /// It passes the shape test for a phasing line, and a patch that does not
    /// move passes it line after line, so the old four-line run cut the picture
    /// into fragments. The patch is in the *middle* of the line; a real phasing
    /// pulse is at the end, which is the extra evidence a mid-picture rephase
    /// has to demand.
    #[test]
    fn a_dark_chart_with_a_bright_feature_is_not_a_new_transmission() {
        let lpm = Lpm::L120;
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        let mut out = into_image(&mut rx, lpm);
        // Six seconds of it, which is what a chart produces.
        out.extend(drain(&mut rx, &dark_chart(lpm, 12, 0.5)));
        assert!(
            !out.iter().any(|e| matches!(e, WefaxEvent::Complete { .. })),
            "a chart's own graphic ended the page: {out:?}"
        );
        assert!(rx.receiving());
    }

    /// Issue #496: a brief burst at the stop tone's rate is a noise burst or a
    /// fade, not five seconds of stop tone. It must not end a healthy chart.
    #[test]
    fn a_brief_stop_tone_burst_does_not_end_the_picture() {
        let lpm = Lpm::L120;
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        let mut out = into_image(&mut rx, lpm);
        let mut audio = ramp_picture(lpm, 2);
        audio.extend(tone(STOP_TONE_HZ, 1.5));
        audio.extend(ramp_picture(lpm, 2));
        out.extend(drain(&mut rx, &audio));
        assert!(
            !out.iter().any(|e| matches!(e, WefaxEvent::Complete { .. })),
            "a 1.5 s burst ended the page: {out:?}"
        );
        assert!(rx.receiving());
    }

    /// ...and the same for a brief burst at the start tone's rate mid-picture.
    #[test]
    fn a_brief_start_tone_burst_does_not_cut_the_page() {
        let lpm = Lpm::L120;
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        let mut out = into_image(&mut rx, lpm);
        let mut audio = ramp_picture(lpm, 2);
        audio.extend(tone(Ioc::I576.start_tone_hz(), 1.5));
        audio.extend(ramp_picture(lpm, 2));
        out.extend(drain(&mut rx, &audio));
        assert!(
            !out.iter().any(|e| matches!(e, WefaxEvent::Complete { .. })),
            "a 1.5 s start burst cut the page: {out:?}"
        );
        assert!(rx.receiving());
    }

    /// A line has to come out at the width the index of cooperation demands,
    /// whatever the line rate — that is the only thing fixing the aspect ratio.
    #[test]
    fn every_line_rate_produces_a_full_width_line() {
        for lpm in Lpm::ALL {
            for ioc in Ioc::ALL {
                let mut rx = WefaxRx::new(RATE);
                rx.set_lpm(lpm);
                rx.set_ioc(ioc);
                let mut out = Vec::new();
                rx.start_manual(&mut out);
                let mut audio = phasing(lpm, 8);
                audio.extend(ramp_picture(lpm, 4));
                let ev = drain(&mut rx, &audio);
                let line = ev.iter().find_map(|e| match e {
                    WefaxEvent::Line { gray, .. } => Some(gray),
                    _ => None,
                });
                let line = line.unwrap_or_else(|| panic!("{lpm:?} {ioc:?} produced no line"));
                assert_eq!(line.len(), ioc.width() as usize, "{lpm:?} {ioc:?}");
            }
        }
    }

    /// Rows must be numbered from zero and consecutively — a gap would be an
    /// image with a band of nothing across it.
    #[test]
    fn rows_are_numbered_consecutively_from_the_top() {
        let lpm = Lpm::L240; // fastest, so the test stays short
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(lpm);
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        let mut audio = phasing(lpm, 8);
        audio.extend(ramp_picture(lpm, 12));
        let ev = drain(&mut rx, &audio);
        let ys: Vec<u16> = ev
            .iter()
            .filter_map(|e| match e {
                WefaxEvent::Line { y, .. } => Some(*y),
                _ => None,
            })
            .collect();
        assert!(ys.len() >= 8, "only {} lines", ys.len());
        assert_eq!(ys[0], 0);
        for (i, y) in ys.iter().enumerate() {
            assert_eq!(*y as usize, i, "row numbering jumped at {i}");
        }
        assert_eq!(rx.lines() as usize, ys.len());
    }

    /// The slant trim has to change the line length, which is the whole of what
    /// it does: a sound card a hundred parts per million fast walks the image
    /// sideways over a quarter-hour chart.
    #[test]
    fn the_slant_trim_stretches_the_line_clock() {
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(Lpm::L120);
        let nominal = rx.samples_per_line();
        assert!((nominal - 6000.0).abs() < 1e-6, "{nominal}");
        rx.set_slant_ppm(1000.0);
        assert!((rx.samples_per_line() - 6006.0).abs() < 1e-6);
        rx.set_slant_ppm(-1000.0);
        assert!((rx.samples_per_line() - 5994.0).abs() < 1e-6);
        assert_eq!(rx.slant_ppm(), -1000.0);
    }

    /// Nudging the alignment moves the picture by whole pixels, so the operator
    /// can straighten a chart whose phasing was missed.
    #[test]
    fn a_phase_nudge_moves_the_line_by_pixels() {
        let mut rx = WefaxRx::new(RATE);
        rx.set_lpm(Lpm::L120);
        let before = rx.line_start;
        rx.nudge_phase(100);
        let spp = rx.samples_per_line() / Ioc::I576.width() as f64;
        assert!(((before - rx.line_start) - 100.0 * spp).abs() < 1e-6);
        rx.nudge_phase(-100);
        assert!((rx.line_start - before).abs() < 1e-6);
    }

    /// Settings may not change under a picture in progress: a chart whose top
    /// half is 1809 pixels wide and whose bottom half is 904 is not an image.
    #[test]
    fn the_line_geometry_is_frozen_while_receiving() {
        let mut rx = WefaxRx::new(RATE);
        let mut out = Vec::new();
        rx.start_manual(&mut out);
        rx.set_ioc(Ioc::I288);
        rx.set_lpm(Lpm::L60);
        assert_eq!(rx.ioc(), Ioc::I576);
        assert_eq!(rx.lpm(), Lpm::L120);
        // Once it has finished they take effect.
        rx.stop_manual(&mut out);
        rx.set_ioc(Ioc::I288);
        assert_eq!(rx.ioc(), Ioc::I288);
        // Stopping twice is harmless and emits one completion.
        let before = out.len();
        rx.stop_manual(&mut out);
        assert_eq!(out.len(), before);
    }

    /// Picture must not be read as a phasing line, or a chart tuned into
    /// halfway would keep re-aligning to its own content.
    #[test]
    fn only_a_real_phasing_line_yields_a_skew() {
        let n = 6000;
        // Black with a white pulse at the end: a phasing line.
        let mut l = vec![0.0f32; n];
        let pulse = n / 20;
        for v in l.iter_mut().skip(n - pulse) {
            *v = 1.0;
        }
        let skew = phasing_skew(&l).expect("a phasing line must yield a skew");
        // The pulse ends at the end of the line, so the picture starts there.
        assert!(skew as usize == 0 || skew as usize >= n - 2, "skew {skew} of {n}");

        // The same pulse a third of the way in.
        let mut l = vec![0.0f32; n];
        for v in l.iter_mut().skip(n / 3).take(pulse) {
            *v = 1.0;
        }
        assert_eq!(phasing_skew(&l), Some((n / 3 + pulse) as u32));

        // A ramp is picture, not phasing.
        let ramp: Vec<f32> = (0..n).map(|i| i as f32 / n as f32).collect();
        assert_eq!(phasing_skew(&ramp), None);
        // All white is not phasing either.
        assert_eq!(phasing_skew(&vec![1.0; n]), None);
        // All black has no pulse to find.
        assert_eq!(phasing_skew(&vec![0.0; n]), None);
        // Degenerate input is refused rather than panicking.
        assert_eq!(phasing_skew(&[]), None);
        assert_eq!(phasing_skew(&[0.0; 8]), None);
    }

    #[test]
    fn a_line_is_averaged_down_to_the_requested_width() {
        // Two samples per pixel, so the box average is the mean of each pair.
        let line: Vec<f32> = vec![0.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        assert_eq!(resample_line(&line, 3), vec![128, 128, 255]);
        // Fewer samples than pixels still fills the row rather than panicking.
        assert_eq!(resample_line(&[1.0], 4).len(), 4);
        assert_eq!(resample_line(&[], 4), vec![0, 0, 0, 0]);
    }
}
