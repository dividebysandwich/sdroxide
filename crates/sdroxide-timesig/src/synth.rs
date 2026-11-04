//! A time-signal station in a box: the audio a receiver would hand the
//! decoder, for a chosen moment, at a chosen signal-to-noise ratio.
//!
//! The symbols come from each format's own encoder, so the generator and the
//! decoder meet only at the specification. The long-wave keying is a clean
//! step softened over a millisecond, symmetric so the half-way point of every
//! edge is exactly on the tenth of a second it belongs to — which is what lets
//! a test say the decoder was right to a fraction of a millisecond. WWVB's
//! carrier is also inverted for each 1 of its phase code, from 100 ms into
//! the second, with the extended symbols sent at XX:10 and XX:40 as the real
//! station sends them. ALS162's carrier keeps its amplitude and swings its
//! phase instead: an element or two at the start of every second, and over
//! the rest of it the pseudo-random code the station adds, which is there to
//! be read past.
//!
//! DCF77's carrier carries its phase code too, from 200 ms into each second
//! (see [`crate::format::dcf77_phase`]). BPC and RWM are keyed carriers like
//! the others — RWM's switched right off between its pulses, and sending its
//! one pulse a second throughout, without the ten-a-second and carrier-only
//! parts of its hour.
//!
//! The short-wave stations are an AM broadcast as a USB receiver hears it:
//! the carrier at the tone and each part of the modulation as a tone above
//! it — the 100 Hz subcarrier at −15 dBc while high and −30 dBc while low, the
//! 5 ms ticks and the minute's 800 ms tone, and the 500 or 600 Hz tone of
//! alternate minutes kept out of the 40 ms protected zone around each second.
//! The other station on the same frequency can be added, later and weaker, and
//! the whole signal can fade. BPM is the same kind of broadcast with only its
//! ticks on it, 20 ms early as it sends them, and its half hour as it runs:
//! UTC seconds, carrier alone, UTC again, then UT1 seconds — late or early by
//! [`Synth::dut1_s`] — and a minute for its call sign.
//!
//! The SNR is quoted in a 3 kHz bandwidth against the full carrier, like the
//! other decoders' harnesses. The carrier is narrow and the decoder's own
//! bandwidth is a few tens of hertz, so add about 18 dB to compare with an SNR
//! in 50 Hz: −20 dB here is the carrier 2 dB below the noise in 50 Hz. On
//! short wave the time code is 21 dB below the carrier to begin with, so the
//! same figure means a much weaker code.

use sdroxide_types::TimeStation;

use crate::format::{Format, Sym, als162, dcf77_phase, wwvb_phase};

/// What a station's reduced carrier is, against its full carrier.
fn low_level(station: TimeStation) -> f32 {
    match station {
        // 15 % — PTB.
        TimeStation::Dcf77 => 0.15,
        // Off — NPL.
        TimeStation::Msf => 0.0,
        // 17 dB down — NIST.
        TimeStation::Wwvb => 0.1413,
        // 10 % — NICT.
        TimeStation::Jjy40 | TimeStation::Jjy60 => 0.1,
        // Not published; taken to be JJY's.
        TimeStation::Bpc => 0.1,
        // Off between pulses.
        TimeStation::Rwm => 0.0,
        // Unused: the short-wave carrier and ALS162's are never reduced.
        TimeStation::Wwv | TimeStation::Wwvh | TimeStation::Bpm | TimeStation::Als162 => 1.0,
    }
}

/// The short-wave subcarrier's modulation depth, high and low: −15 and
/// −30 dBc.
const SUB_HIGH: f64 = 0.178;
const SUB_LOW: f64 = 0.0316;
/// The standard tones' modulation depth.
const TONE_DEPTH: f64 = 0.5;

/// Clicks on top of the noise — static, or a switching supply nearby: each a
/// millisecond and a half of noise far louder than the rest.
#[derive(Debug, Clone, Copy)]
pub struct Impulses {
    /// How many a second, on average, at random.
    pub per_s: f64,
    /// Each one's level against the carrier, dB.
    pub level_db: f64,
}

/// The other short-wave station, heard on the same frequency.
#[derive(Debug, Clone, Copy)]
pub struct Sibling {
    /// How much later its signal arrives, seconds: the difference of the two
    /// paths.
    pub delay_s: f64,
    /// Its carrier against this station's.
    pub level: f64,
}

pub struct Synth {
    pub station: TimeStation,
    pub rate: f64,
    /// Where the carrier lands in the audio, Hz.
    pub tone_hz: f64,
    /// Signal-to-noise ratio in 3 kHz; `None` for no noise at all.
    pub snr_db: Option<f64>,
    /// How far the receiver's sample clock is from true, parts per million:
    /// positive means it runs fast, so a true second spans more samples.
    pub rate_error_ppm: f64,
    pub seed: u64,
    /// A flat fade at this rate, Hz: the signal swings between full and a
    /// tenth (20 dB down) and back.
    pub fade_hz: Option<f64>,
    /// Short wave only: the other station too.
    pub sibling: Option<Sibling>,
    /// The carrier alone, with nothing on it: a transmitter on the air whose
    /// time code has stopped, which must decode as nothing.
    pub dead_air: bool,
    pub impulses: Option<Impulses>,
    /// BPM only: UT1 − UTC, seconds, which its UT1 minutes are sent by.
    pub dut1_s: f64,
}

impl Synth {
    pub fn new(station: TimeStation, rate: f64) -> Self {
        Synth {
            station,
            rate,
            tone_hz: f64::from(sdroxide_types::TIMESIG_TONE_HZ),
            snr_db: None,
            rate_error_ppm: 0.0,
            seed: 1,
            fade_hz: None,
            sibling: None,
            dead_air: false,
            impulses: None,
            dut1_s: -0.3,
        }
    }

    /// `secs` of audio whose first sample is the UTC instant `start` (Unix
    /// seconds, fractional).
    pub fn render(&self, start: f64, secs: f64) -> Vec<f32> {
        let amp = 0.3f64;
        let true_rate = self.rate * (1.0 + self.rate_error_ppm * 1e-6);
        let sigma = self.snr_db.map(|snr| {
            let carrier_power = amp * amp / 2.0;
            (carrier_power / 10f64.powf(snr / 10.0) * self.rate / 6000.0).sqrt()
        });
        let mut rng = Rng(self.seed.max(1));
        let mut lf = Lf::new(self.station);
        let mut hf = [
            Hf::new(self.station, self.dut1_s),
            Hf::new(self.station.sibling().unwrap_or(self.station), self.dut1_s),
        ];
        // RWM keys its carrier, as the long-wave stations do.
        let ticks = self.station.is_hf() && self.station.tick_hz().is_some();
        let n = (secs * self.rate) as usize;
        let mut out = Vec::with_capacity(n);
        // Clicks: a chance per sample, and how many samples one lasts.
        let click_len = (0.0015 * self.rate).ceil() as usize;
        let mut click_rng = Rng(self.seed.wrapping_mul(0x9e37_79b9).max(1));
        let mut click_left = 0usize;
        for i in 0..n {
            let t = start + i as f64 / true_rate;
            let wc = std::f64::consts::TAU * self.tone_hz * i as f64 / self.rate;
            let fade = self
                .fade_hz
                .map_or(1.0, |f| 0.55 + 0.45 * (std::f64::consts::TAU * f * (t - start)).cos());
            let signal = if self.dead_air {
                wc.cos()
            } else if ticks {
                let mut x = hf[0].sample(t, wc);
                if let Some(s) = self.sibling {
                    // A fixed phase between the two carriers: they are on the
                    // same frequency to a part in 10¹², so they do not beat
                    // over a test's length.
                    x += s.level * hf[1].sample(t - s.delay_s, wc + 1.0);
                }
                x
            } else {
                let (a, phase) = lf.sample(t);
                a * (wc + phase).cos()
            };
            let mut noise = sigma.map_or(0.0, |s| s * rng.gauss());
            if let Some(imp) = self.impulses {
                if click_left == 0 && click_rng.next_f64() < imp.per_s / self.rate {
                    click_left = click_len;
                }
                if click_left > 0 {
                    click_left -= 1;
                    let level = amp * 10f64.powf(imp.level_db / 20.0);
                    noise += level * click_rng.gauss();
                }
            }
            out.push((amp * fade * signal + noise) as f32);
        }
        out
    }
}

/// The keyed carriers: the long-wave stations and RWM, and the phase codes
/// of WWVB and DCF77.
struct Lf {
    station: TimeStation,
    fmt: &'static Format,
    minute: Option<(i64, [Sym; 60])>,
    phase_bit: Option<(i64, bool)>,
    chips: [bool; dcf77_phase::CHIPS],
}

impl Lf {
    fn new(station: TimeStation) -> Self {
        Lf {
            station,
            fmt: Format::for_station(station),
            minute: None,
            phase_bit: None,
            chips: dcf77_phase::chips(),
        }
    }

    /// DCF77's phase at `t`: the code, the right way up for the second's 0
    /// and inverted for its 1. The minute's marker second sends it as a 0.
    fn dcf77_phase(&mut self, t: f64) -> f64 {
        let sec = t.floor() as i64;
        let m = sec.div_euclid(60) * 60;
        if self.minute.is_none_or(|(at, _)| at != m) {
            self.minute = Some((m, (self.fmt.encode)(m)));
        }
        let sym = self.minute.expect("just set").1[(sec - m) as usize];
        dcf77_phase::phase(t - sec as f64, sym.a && !sym.marker, &self.chips)
    }

    fn level_at(&mut self, t: f64) -> f32 {
        let sec = t.floor() as i64;
        let m = sec.div_euclid(60) * 60;
        if self.minute.is_none_or(|(at, _)| at != m) {
            self.minute = Some((m, (self.fmt.encode)(m)));
        }
        let sym = self.minute.expect("just set").1[(sec - m) as usize];
        let slot = ((t - sec as f64) * 10.0) as usize;
        let reduced = self.fmt.template(sym).is_some_and(|tpl| tpl.low[slot.min(9)]);
        if reduced { low_level(self.station) } else { 1.0 }
    }

    /// The carrier's amplitude and phase at `t`.
    fn sample(&mut self, t: f64) -> (f64, f64) {
        let half_ramp = 0.0005;
        let a =
            (self.level_at(t - half_ramp) + self.level_at(t) + self.level_at(t + half_ramp)) / 3.0;
        let phase = match self.station {
            TimeStation::Wwvb => {
                // Each phase bit starts 100 ms into its second.
                let sec = (t - 0.1).floor() as i64;
                if self.phase_bit.is_none_or(|(at, _)| at != sec) {
                    self.phase_bit = Some((sec, wwvb_phase::bit_at(sec)));
                }
                if self.phase_bit.expect("just set").1 { std::f64::consts::PI } else { 0.0 }
            }
            TimeStation::Als162 => self.als162_phase(t),
            TimeStation::Dcf77 => self.dcf77_phase(t),
            _ => 0.0,
        };
        (f64::from(a), phase)
    }

    /// ALS162's phase at `t`: the elements of the second that `t` is nearest
    /// the start of, and the position code of whichever second it is in.
    fn als162_phase(&mut self, t: f64) -> f64 {
        // An element starts 50 ms before its second, so everything from
        // −50 ms on belongs to the second that follows.
        let sec = (t + 0.05).floor() as i64;
        let into = t - sec as f64;
        let m = sec.div_euclid(60) * 60;
        if self.minute.is_none_or(|(at, _)| at != m) {
            self.minute = Some((m, (self.fmt.encode)(m)));
        }
        let s_of_m = (sec - m) as usize;
        let sym = self.minute.expect("just set").1[s_of_m];
        let elements = (0..als162::elements(sym))
            .map(|k| als162::element(into - k as f64 * als162::ELEMENT_S))
            .sum::<f64>();
        // The position code's 25 ms slices are slopes, in units of a radian
        // a slice, from 150 ms to 950 ms; integrated, it is back at zero by
        // the end of each.
        let mut code = 0.0;
        let from = 0.15;
        if into > from {
            let slices = &ALS162_POSITION[s_of_m];
            let x = (into - from) / 0.025;
            let whole = (x as usize).min(slices.len());
            code = slices[..whole].iter().map(|&d| f64::from(d)).sum::<f64>();
            if whole < slices.len() {
                code += f64::from(slices[whole]) * x.fract();
            }
        }
        elements + code
    }
}

/// ALS162's pseudo-random phase code as reverse-engineered from the air by
/// Henning Maier for gr_ALS162_Receiver (GPL-3.0, github.com/henningM1r,
/// `python/ALS162_codes.py`), one row a second of the minute: each entry is
/// the slope of the phase over one 25 ms slice, radians a slice. Not used to
/// decode — LNE-SYRTE does not publish it — only to put on the synthetic
/// carrier what a receiver has to read the elements past.
#[rustfmt::skip]
static ALS162_POSITION: [[i8; 32]; 60] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, -1, 0, 0, 0, 1, -1, 0, 0, 0, 0, 0, 0, 0, -1, 0, 2, 0, -2, 0, 2, 0, -2, 0, 1, 0, 0, 0, 0, 0, 0],
    [1, 0, -2, 0, 2, -1, -1, 1, 1, -1, -1, 1, 0, 1, 0, -2, 1, 0, 0, 1, -2, 0, 1, 0, 1, -1, -1, 1, 0, 0, 0, 0],
    [1, -1, 0, 0, -1, 2, 0, -1, -1, 0, 2, -1, -1, 2, 0, -1, -1, 0, 2, -1, -1, 2, 0, -1, -1, 0, 1, 0, 0, 0, 0, 0],
    [1, -1, -1, 1, 1, 0, -1, -1, 1, 0, -1, 1, 0, 0, 0, 1, 0, -2, 1, 1, -1, -1, 0, 1, 1, -1, -1, 1, 0, 0, 0, 0],
    [1, -1, 0, 1, -2, 0, 1, 0, 1, 0, -1, -1, 0, 2, 0, -1, -1, 0, 1, 1, -1, -1, 2, -1, -1, 1, 0, 0, 0, 0, 0, 0],
    [1, -1, -1, 1, 1, 0, -1, -1, 0, 1, 0, 1, 0, -2, 0, 2, 0, -2, 1, 1, -1, -1, 0, 2, -1, -1, 1, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 1, 0, -1, 0, -1, 0, 1, 1, 0, -1, -1, 1, 0, 0, 0, -1, 1, 1, 0, -1, -1, 0, 1, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, -1, 0, 0, 0, 0, -1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, -1, -1, 2, -1, -1, 1, 0, 0, 0, 0],
    [1, -1, 0, 1, -2, 1, 0, -1, 1, 1, -1, -1, 1, 1, 0, -1, -1, 0, 1, 1, 0, -1, -1, 0, 1, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, -1, -1, 0, 1, 1, 0, -1, -1, 0, 2, -1, -1, 1, 1, 0, -1, -1, 1, 1, -2, 0, 1, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, -1, -1, 1, 1, -2, 1, 0, -1, 1, 0, 1, -1, 0, 1, -1, 0, -1, 0, 2, -1, 0, 0, -1, 1, 0, 0, 0, 0, 0],
    [0, 1, 0, -1, 0, 0, -1, 0, 2, -1, -1, 1, 1, 0, -1, 0, -1, 0, 1, 0, 0, 0, 0, 0, 1, -1, -1, 1, 0, 0, 0, 0],
    [1, -1, -1, 2, 0, -1, -1, 0, 1, 1, -1, 0, 1, -2, 0, 1, 1, -1, -1, 1, 0, 0, 1, 0, -1, -1, 0, 1, 0, 0, 0, 0],
    [1, 0, -2, 1, 1, -2, 1, 0, -1, 2, 0, -2, 0, 1, 1, -1, 0, 0, -1, 1, 1, -1, -1, 2, 0, -2, 0, 1, 0, 0, 0, 0],
    [0, 1, -1, -1, 1, 1, 0, -1, 0, -1, 0, 2, 0, -1, 0, -1, 0, 2, -1, 0, 1, -2, 0, 2, -1, -1, 1, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, -2, 1, 1, -1, 0, 0, -1, 1, 0, -1, 1, 0, 0, 0, 0, 1, 0, -2, 1, 1, -1, -1, 0, 1, 0, 0, 0, 0],
    [1, 0, -2, 0, 2, -1, 0, 1, -1, 0, 0, -1, 1, 1, -1, 0, 0, -1, 1, 1, -2, 1, 1, -2, 0, 1, 0, 0, 0, 0, 0, 0],
    [1, -1, 0, 1, -2, 0, 2, -1, 0, 1, -1, 0, 0, -1, 0, 2, 0, -1, 0, -1, 1, 0, -1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 0, -1, 0, -1, 0, 1, 1, -1, 0, 1, -2, 0, 2, -1, -1, 1, 0, 0, 1, -1, -1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, -1, -1, 1, 0, 1, 0, -2, 0, 1, 0, 1, -1, 0, 1, -2, 0, 2, -1, 0, 1, -2, 0, 2, -1, -1, 1, 0, 0, 0, 0, 0],
    [0, 0, 1, -1, -1, 2, -1, 0, 1, -1, 0, -1, 0, 1, 1, -1, 0, 1, -2, 1, 1, -1, 0, -1, 0, 1, 0, 0, 0, 0, 0, 0],
    [1, -1, -1, 2, -1, 0, 0, -1, 1, 0, 1, 0, -1, -1, 0, 2, 0, -2, 0, 2, 0, -1, 0, -1, 1, 0, -1, 1, 0, 0, 0, 0],
    [0, 0, 0, 0, 1, 0, -1, 0, 0, 0, 0, 0, -1, 1, 1, -2, 1, 0, 0, 1, -1, -1, 0, 2, 0, -2, 0, 1, 0, 0, 0, 0],
    [0, 0, 0, 0, 1, -1, 0, 0, 0, 1, -2, 1, 0, -1, 2, 0, -1, -1, 1, 1, -2, 1, 1, -1, 0, -1, 0, 1, 0, 0, 0, 0],
    [1, 0, -2, 0, 1, 0, 0, 0, 1, 0, -2, 0, 2, 0, -1, 0, 0, -1, 1, 0, 0, 1, -2, 1, 1, -2, 0, 1, 0, 0, 0, 0],
    [1, -1, 0, 1, -2, 0, 2, -1, 0, 0, 0, 1, -2, 0, 2, -1, -1, 1, 0, 1, -1, 0, 1, -1, -1, 0, 1, 0, 0, 0, 0, 0],
    [1, 0, -2, 0, 1, 1, -1, -1, 1, 0, 1, 0, -2, 0, 2, 0, -1, -1, 1, 0, -1, 1, 1, 0, -2, 0, 1, 0, 0, 0, 0, 0],
    [1, 0, -1, -1, 1, 1, -2, 1, 0, 0, 1, -2, 0, 1, 0, 1, -1, -1, 2, 0, -1, 0, -1, 1, 1, -2, 0, 1, 0, 0, 0, 0],
    [0, 1, -1, -1, 2, 0, -2, 0, 2, 0, -1, 0, 0, 0, 0, -1, 0, 2, -1, -1, 1, 0, 1, -1, -1, 1, 0, 0, 0, 0, 0, 0],
    [1, 0, -2, 0, 2, -1, -1, 1, 0, 1, 0, -2, 0, 2, 0, -2, 1, 0, 0, 1, -2, 0, 1, 1, 0, -2, 0, 1, 0, 0, 0, 0],
    [0, 1, -1, 0, 1, -2, 0, 1, 1, 0, -2, 1, 1, -1, 0, -1, 0, 2, 0, -1, -1, 0, 2, -1, -1, 1, 0, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, -2, 1, 1, -1, -1, 0, 1, 1, 0, -2, 0, 1, 0, 0, 0, 1, 0, -2, 1, 0, 0, 0, -1, 1, 0, 0, 0, 0],
    [1, 0, -1, 0, -1, 1, 0, -1, 1, 0, 0, 1, -1, 0, 1, -1, 0, 0, 0, -1, 0, 1, 1, 0, -2, 0, 1, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 1, 0, -2, 0, 2, -1, -1, 2, 0, -1, 0, 0, 0, -1, 1, 1, -1, 0, 0, 0, -1, 0, 1, 0, 0, 0, 0, 0],
    [0, 0, 0, 1, 0, -1, 0, 0, -1, 0, 1, 0, 1, 0, -2, 1, 0, 0, 1, -1, 0, -1, 0, 1, 1, -1, -1, 1, 0, 0, 0, 0],
    [1, -1, 0, 1, -1, 0, 0, 0, -1, 0, 2, -1, 0, 0, 0, 0, -1, 1, 0, 0, 0, 1, -1, 0, 0, -1, 1, 0, 0, 0, 0, 0],
    [1, -1, -1, 1, 0, 1, -1, -1, 1, 1, -1, -1, 1, 0, 1, 0, -2, 1, 0, 0, 1, -1, -1, 0, 1, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, -2, 1, 1, -1, 0, 0, 0, -1, 0, 2, -1, -1, 1, 1, 0, -2, 0, 1, 1, -1, -1, 1, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, -1, -1, 1, 1, -2, 0, 2, -1, -1, 1, 1, -1, 0, 1, -1, 0, -1, 0, 2, -1, -1, 1, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, -1, 0, 0, -1, 0, 2, -1, -1, 1, 1, -1, -1, 1, 1, 0, -1, 0, 0, 0, 0, 0, -1, 0, 1, 0, 0, 0, 0, 0],
    [0, 1, 0, -1, -1, 1, 1, -2, 1, 1, -1, 0, -1, 0, 2, -1, -1, 1, 0, 1, 0, -2, 1, 0, -1, 1, 0, 0, 0, 0, 0, 0],
    [1, 0, -1, -1, 1, 0, -1, 2, 0, -1, -1, 1, 1, -2, 0, 2, -1, -1, 2, 0, -2, 0, 2, 0, -1, -1, 0, 1, 0, 0, 0, 0],
    [1, 0, -1, 0, -1, 0, 2, 0, -2, 0, 2, -1, 0, 1, -1, 0, 0, 0, 0, -1, 1, 0, -1, 1, 1, -1, -1, 1, 0, 0, 0, 0],
    [0, 1, -1, -1, 2, 0, -2, 0, 2, -1, 0, 0, 0, 0, 0, 1, -2, 1, 0, -1, 1, 0, 0, 0, 1, -1, -1, 1, 0, 0, 0, 0],
    [0, 1, 0, -1, 0, 0, -1, 1, 1, -2, 1, 0, -1, 2, 0, -2, 0, 1, 0, 0, 0, 0, 0, 1, -1, -1, 1, 0, 0, 0, 0, 0],
    [1, -1, -1, 1, 1, -1, -1, 2, 0, -2, 0, 2, -1, -1, 1, 1, 0, -2, 1, 1, -2, 0, 2, 0, -2, 0, 1, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, -2, 0, 1, 0, 1, 0, -2, 1, 0, -1, 2, -1, 0, 1, -1, -1, 1, 1, -2, 1, 1, -2, 0, 1, 0, 0, 0, 0],
    [1, 0, -1, -1, 1, 0, 0, 1, -2, 1, 0, -1, 2, 0, -2, 1, 0, -1, 2, 0, -2, 1, 0, 0, 1, -2, 0, 1, 0, 0, 0, 0],
    [1, -1, -1, 2, -1, -1, 2, 0, -2, 0, 2, 0, -1, 0, 0, 0, -1, 1, 1, -2, 1, 0, -1, 1, 1, -1, -1, 1, 0, 0, 0, 0],
    [0, 0, 1, 0, -2, 0, 1, 0, 1, -1, 0, 0, 0, 0, 0, 1, -1, 0, 0, -1, 1, 1, -1, 0, -1, 0, 1, 0, 0, 0, 0, 0],
    [1, -1, -1, 1, 0, 0, 0, 1, -1, 0, 1, -1, -1, 0, 2, 0, -2, 1, 0, 0, 0, -1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 0, -1, -1, 1, 1, -1, 0, 0, 0, 0, -1, 1, 0, -1, 2, -1, -1, 2, 0, -1, -1, 1, 1, -1, -1, 0, 1, 0, 0, 0, 0],
    [0, 1, 0, -1, -1, 0, 2, 0, -2, 0, 1, 1, -1, -1, 1, 1, 0, -1, 0, -1, 1, 0, -1, 1, 1, -1, -1, 1, 0, 0, 0, 0],
    [1, 0, -1, -1, 0, 1, 1, 0, -2, 1, 0, 0, 0, 0, 0, 0, 0, -1, 2, -1, 0, 0, 0, 0, -1, 1, 0, 0, 0, 0, 0, 0],
    [0, 0, 1, -1, 0, 1, -2, 0, 2, -1, -1, 2, -1, 0, 0, 0, 0, -1, 2, 0, -1, 0, -1, 1, 0, -1, 1, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 1, -1, -1, 2, 0, -2, 0, 1, 0, 0, 1, -1, 0, 0, -1, 2, -1, 0, 0, -1, 1, 0, 0, 0, 0],
    [0, 0, 1, 0, -2, 1, 0, 0, 1, -1, -1, 1, 0, 0, 1, -2, 1, 1, -1, -1, 0, 1, 0, 1, 0, -2, 0, 1, 0, 0, 0, 0],
    [1, 0, -1, 0, -1, 1, 0, -1, 2, -1, -1, 2, -1, -1, 2, -1, 0, 1, -1, -1, 0, 1, 0, 0, 1, -1, -1, 1, 0, 0, 0, 0],
    [1, 0, -2, 0, 2, 0, -2, 0, 2, 0, -2, 0, 2, 0, -2, 0, 2, 0, -2, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];

/// One short-wave station's broadcast, as USB audio relative to its carrier.
struct Hf {
    station: TimeStation,
    fmt: &'static Format,
    minute: Option<(i64, [Sym; 60])>,
    dut1_s: f64,
}

impl Hf {
    fn new(station: TimeStation, dut1_s: f64) -> Self {
        Hf { station, fmt: Format::for_station(station), minute: None, dut1_s }
    }

    /// BPM at true time `t`: its ticks alone, sent ahead of UTC, in the part
    /// of its half hour `t` falls in.
    fn bpm(&mut self, t: f64, wc: f64) -> f64 {
        let tau = std::f64::consts::TAU;
        let mut x = wc.cos();
        let sent = t + self.station.advance_s();
        let minute_of_hour = (sent.div_euclid(60.0) as i64).rem_euclid(60) as u32;
        let (clock, second_len) = match minute_of_hour % 30 {
            0..10 | 15..25 => (sent, 0.010),
            // UT1 seconds, 100 ms long.
            25..29 => (sent + self.dut1_s, 0.100),
            // Carrier alone, or the call sign, which is not modelled.
            _ => return x,
        };
        let sec = clock.floor();
        let into = clock - sec;
        let len = if (sec as i64).rem_euclid(60) == 0 { 0.300 } else { second_len };
        if into < len {
            x += 0.5 * (wc + tau * 1000.0 * into).cos();
        }
        x
    }

    /// The audio at true time `t` with the carrier's phase `wc`, carrier
    /// amplitude 1. Each modulating tone of depth μ is a sideband of μ/2.
    fn sample(&mut self, t: f64, wc: f64) -> f64 {
        if self.station == TimeStation::Bpm {
            return self.bpm(t, wc);
        }
        let sec = t.floor() as i64;
        let into = t - sec as f64;
        let m = sec.div_euclid(60) * 60;
        if self.minute.is_none_or(|(at, _)| at != m) {
            self.minute = Some((m, (self.fmt.encode)(m)));
        }
        let s_of_m = (sec - m) as usize;
        let sym = self.minute.expect("just set").1[s_of_m];
        let tau = std::f64::consts::TAU;
        let side = |depth: f64, hz: f64, since: f64| depth / 2.0 * (wc + tau * hz * since).cos();
        let mut x = wc.cos();

        // The subcarrier: nothing in the tick's first 30 ms, nor in the hole.
        if into >= 0.03 && !sym.hole {
            let slot = ((into * 10.0) as usize).min(9);
            let high = self.fmt.template(sym).is_some_and(|tpl| !tpl.low[slot]);
            // A sine in phase with the second, so a positive-going zero
            // crossing lands at 30 ms: a cosine a quarter period late.
            x += side(if high { SUB_HIGH } else { SUB_LOW }, 100.0, into - 0.0025);
        }

        // The seconds pulse: the minute's 800 ms tone (1500 Hz at the top of
        // the hour), a 5 ms tick otherwise, nothing in seconds 29 and 59.
        let tick = self.station.tick_hz().unwrap_or(1000.0);
        let (pulse_hz, pulse_len) = match s_of_m {
            0 if sec.rem_euclid(3600) == 0 => (1500.0, 0.8),
            0 => (tick, 0.8),
            29 | 59 => (0.0, 0.0),
            _ => (tick, 0.005),
        };
        if into < pulse_len {
            x += side(1.0, pulse_hz, into);
        }

        // The standard tone of alternate minutes, clear of the protected
        // zone from 10 ms before each second to 30 ms after it, and of the
        // minute's own pulse.
        let protected = !(0.03..0.99).contains(&into) || s_of_m == 0;
        if !protected {
            let hz = if (m / 60) % 2 == 0 { 500.0 } else { 600.0 };
            x += side(TONE_DEPTH, hz, t - m as f64);
        }
        x
    }
}

/// xorshift64* with a Box–Muller pair, so the crate needs no RNG dependency
/// and a seed always means the same noise.
struct Rng(u64);

impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_f491_4f6c_dd1d);
        ((x >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    fn gauss(&mut self) -> f64 {
        let (u, v) = (self.next_f64(), self.next_f64());
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}
