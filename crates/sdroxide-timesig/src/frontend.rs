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
//! so its "in phase" flips sign with the phase code; its amplitude is read by
//! magnitude instead.
//!
//! # ALS162's phase
//!
//! ALS162 never moves its carrier's amplitude, and swings its phase by up to
//! a radian either way — symmetrically, so the swings average to nothing and
//! the half-second average above still points along the unmodulated carrier.
//! What leaves is the part of the signal *across* that average: the carrier's
//! amplitude times the sine of its phase swing, which for a swing of a radian
//! is close to the swing itself. Like the in-phase reading, it is linear in
//! the noise, and only the noise in its own quadrature reaches it.
//!
//! The average is a different one, though: two seconds centred on the sample,
//! rather than half a second behind it. A trailing average swings with each
//! element as it goes past — towards its first lobe, then back — and moves
//! the element's zero crossing, which is the time, by half a millisecond. A
//! centred one holds the whole element throughout, and an element sums to
//! nothing; what enters at one end and leaves at the other is the same part
//! of the seconds either side, and cancels too.
//!
//! # WWVB's phase
//!
//! For the phase code itself the carrier's phase is followed by a loop on the
//! squared signal — squaring takes the keying off, so the loop sees a plain
//! carrier at twice the angle — and the signal is turned by half the loop's
//! angle. What leaves is the carrier with its keying on: its real part is
//! the bit times the amplitude, its imaginary part is noise. The loop's
//! angle is only known modulo half a turn, so the bits may come out upside
//! down; the phase code's sync word sorts that out.
//!
//! # Clicks
//!
//! Long wave is full of them — static, and every switching supply and LED
//! driver nearby — and everything here is an average, which a click a
//! hundred times the noise's size moves as far as a hundred samples of
//! honest noise would. So before anything else each millisecond's sample is
//! held against the usual size of one, and one far larger than that is
//! blanked: set to nothing, which takes its share of carrier with it but
//! nothing else.
//!
//! The usual size is a quantile of the samples' sizes, which clicks cannot
//! drag up until they are as common as the samples above it. Which quantile
//! depends on the station: it has to sit below the full carrier, or a clean
//! signal's full carrier would look like a click against its reduced one. So
//! it is the one half the full-carrier samples lie above — the median for a
//! carrier that is never reduced (ALS162), higher for one that spends much
//! of each second reduced (WWVB, JJY). The limit is then set where Gaussian
//! noise alone reaches once in a million samples or so, and the carrier
//! never does; a click does every time.
//!
//! # Where each sample is
//!
//! Every filter here is a plain average — symmetric, so its delay is exactly
//! half its length and nothing else. That matters more than its shape: the
//! point of the exercise is to know *when* the carrier dropped, and a filter
//! whose delay depended on frequency or level would move the answer. Each
//! output carries `pos`, the input sample (fractional) it stands for once
//! both averages' delays are taken off.

use std::collections::VecDeque;

use num_complex::Complex32;

use crate::search::CarrierSearch;

/// The rate the envelope is wanted at. The decimation is the nearest whole
/// number to the input rate over this, so the real rate is close to it rather
/// than equal — 1002.27 Hz from 44.1 kHz — and everything downstream works in
/// seconds rather than in envelope samples.
pub(crate) const ENV_RATE: f64 = 1000.0;
/// Envelope smoothing, in envelope samples: 5 ms. Short against the 100 ms
/// the keying is made of, long enough to take most of the noise off the edge.
const SMOOTH: usize = 5;
/// The carrier-phase average's time constant, in envelope samples: half a
/// second. Long against the noise, short against what is left of the
/// frequency error after the search (a few hundredths of a hertz).
const PHASE_TC: f32 = 500.0;
/// WWVB's phase loop: a second-order loop of 0.15 Hz noise bandwidth,
/// critically damped. Narrow enough to average a couple of seconds of noise,
/// wide enough to pull in what the search leaves of the frequency error.
const PLL_BN_HZ: f64 = 0.15;
const PLL_ZETA: f64 = 0.707;
/// Envelope samples of squared signal averaged to seed the loop's angle.
const PLL_SEED: u32 = 500;
/// The power average the loop's error is normalised by: a second.
const POWER_TC: f32 = 1000.0;
/// A decimated sample larger than this many standard deviations of the
/// noise in one of its quadratures is a click: Gaussian noise alone is that
/// size once in a few million samples.
const BLANK_SIGMAS: f64 = 5.5;
/// How far the usual size moves for each sample above or below it, as a
/// fraction of itself: slow enough to average over a second's keying, and a
/// step of 6 dB is still followed in a few seconds.
const BLANK_STEP: f32 = 0.002;
/// Samples whose quantile seeds the usual size.
const BLANK_SEED: usize = 101;
/// Consecutive samples blanked, 100 ms, after which it is not clicks but the
/// signal that has grown — audio that started silent, a receiver unmuted —
/// and the usual size is seeded again.
const BLANK_RUN_MAX: u32 = 100;

/// What a station keys, and so what the front end reads off its carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keying {
    /// The amplitude alone: read in phase with the carrier.
    Amplitude,
    /// The amplitude, and the carrier's sign as a second code (WWVB): the
    /// amplitude by magnitude, the sign through the phase loop into
    /// [`EnvSample::iq`].
    AmplitudeAndSign,
    /// The phase alone (ALS162): the part of the signal across the carrier.
    Phase,
}

/// One envelope sample.
///
/// The decoder reads two things off the signal: when each second starts (the
/// *timing* channel) and what each second says (the *level* channel). On long
/// wave both are the carrier's envelope and the two pairs of fields are the
/// same; the short-wave front end fills them from different parts of the
/// broadcast.
#[derive(Debug, Clone, Copy, Default)]
pub struct EnvSample {
    /// The input sample this stands for, counted from the first sample the
    /// front end was given.
    pub pos: f64,
    /// The level the second's symbol is read from.
    pub mag: f32,
    /// The input sample the timing value stands for.
    pub tpos: f64,
    /// The value the start of the second is found from.
    pub timing: f32,
    /// The other station's ticks, at `tpos` — short wave only.
    pub sibling: f32,
    /// The carrier turned onto its own phase, keying left on: WWVB only, and
    /// zero until the phase loop is running.
    pub iq: Complex32,
}

/// A second-order phase-locked loop on the squared carrier.
struct PhaseLoop {
    /// Half the squared carrier's angle: the carrier's own, modulo π.
    psi: f64,
    /// The loop's frequency, radians per envelope sample.
    omega: f64,
    k1: f64,
    k2: f64,
    power: f32,
    seed: Complex32,
    seeded: u32,
}

impl PhaseLoop {
    fn new(env_rate: f64) -> Self {
        let wn = 2.0 * PLL_BN_HZ / (PLL_ZETA + 1.0 / (4.0 * PLL_ZETA));
        let t = 1.0 / env_rate;
        PhaseLoop {
            psi: 0.0,
            omega: 0.0,
            k1: 2.0 * PLL_ZETA * wn * t,
            k2: (wn * t) * (wn * t),
            power: 0.0,
            seed: Complex32::new(0.0, 0.0),
            seeded: 0,
        }
    }

    /// Takes one smoothed sample; returns it turned onto the carrier's phase,
    /// once the loop has an angle.
    fn step(&mut self, z: Complex32) -> Option<Complex32> {
        self.power += (z.norm_sqr() - self.power) / POWER_TC;
        if self.seeded < PLL_SEED {
            self.seed += z * z;
            self.seeded += 1;
            if self.seeded < PLL_SEED {
                return None;
            }
            self.psi = f64::from(self.seed.arg()) / 2.0;
        }
        let (s, c) = self.psi.sin_cos();
        let turn = Complex32::new(c as f32, -s as f32);
        let q = z * z * turn * turn;
        // sin 2Δ, halved and normalised: Δ radians for a small error.
        let err = f64::from(q.im / (2.0 * self.power.max(f32::MIN_POSITIVE)));
        self.omega += self.k2 * err;
        self.psi += self.omega + self.k1 * err;
        if self.psi > std::f64::consts::PI {
            self.psi -= std::f64::consts::TAU;
        } else if self.psi < -std::f64::consts::PI {
            self.psi += std::f64::consts::TAU;
        }
        Some(z * turn)
    }
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
    keying: Keying,
    /// The carrier's phase reference: a slow average of the signal.
    reference: Complex32,
    /// ALS162: the smoothed samples either side of the one being read, and
    /// their sum — its phase reference.
    centred: VecDeque<(f64, Complex32)>,
    centred_sum: Complex32,
    search: CarrierSearch,
    /// WWVB's phase loop, for the stations that key their carrier's phase.
    pll: Option<PhaseLoop>,
    moves_seen: u32,
    moved_at: f64,
    /// The usual size of a decimated sample — a running quantile of them —
    /// the quantile, the first few samples, which seed it, and the multiple
    /// of it that is a click.
    usual: f32,
    quantile: f32,
    seed: Vec<f32>,
    blank_ratio: f32,
    blank_run: u32,
}

impl FrontEnd {
    pub fn new(rate: f64, tone_hz: f64, keying: Keying) -> Self {
        let m = (rate / ENV_RATE).round().max(1.0) as usize;
        let env_rate = rate / m as f64;
        let phase_code = keying == Keying::AmplitudeAndSign;
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
            keying,
            reference: Complex32::new(0.0, 0.0),
            centred: VecDeque::new(),
            centred_sum: Complex32::new(0.0, 0.0),
            search: CarrierSearch::new(env_rate, phase_code),
            pll: phase_code.then(|| PhaseLoop::new(env_rate)),
            moves_seen: 0,
            moved_at: 0.0,
            usual: 0.0,
            quantile: 0.5,
            seed: Vec::with_capacity(BLANK_SEED),
            blank_ratio: blank_ratio(0.5),
            blank_run: 0,
        }
    }

    /// Sets the click blanker for a carrier at full level `full_share` of
    /// the time, 0 to 1.
    pub fn with_full_share(mut self, full_share: f64) -> Self {
        let q = (1.0 - full_share.clamp(0.0, 1.0) / 2.0).clamp(0.5, 0.9);
        self.quantile = q as f32;
        self.blank_ratio = blank_ratio(q);
        self
    }

    /// Blanks `y` if it is a click. The usual size steps up for every sample
    /// above it and down for every one below, in the proportion that settles
    /// it on its quantile however large the samples above it are.
    fn blank(&mut self, y: Complex32) -> Complex32 {
        let a = y.norm();
        if self.seed.len() < BLANK_SEED {
            self.seed.push(a);
            if self.seed.len() == BLANK_SEED {
                self.seed.sort_by(f32::total_cmp);
                self.usual = self.seed[(self.quantile * (BLANK_SEED - 1) as f32) as usize];
            }
            return y;
        }
        let blank = a > self.blank_ratio * self.usual;
        self.blank_run = if blank { self.blank_run + 1 } else { 0 };
        if self.blank_run > BLANK_RUN_MAX {
            self.blank_run = 0;
            self.seed.clear();
        }
        self.usual *= if a > self.usual {
            1.0 + BLANK_STEP * self.quantile
        } else {
            1.0 - BLANK_STEP * (1.0 - self.quantile)
        };
        if blank { Complex32::new(0.0, 0.0) } else { y }
    }

    /// The input sample the carrier was last found somewhere new at: nothing
    /// before it was measured on the right frequency.
    pub fn moved_at(&self) -> f64 {
        self.moved_at
    }

    /// The unmodulated carrier's amplitude as received, for the stations
    /// read against it.
    pub fn carrier_level(&self) -> f32 {
        self.reference.norm()
    }

    /// The envelope's sample rate.
    pub fn env_rate(&self) -> f64 {
        self.rate / self.m as f64
    }

    /// Counts each time the carrier was found somewhere new: whatever was
    /// measured before a move was measured on the wrong frequency.
    pub fn moves(&self) -> u32 {
        self.search.moves()
    }

    /// Where the carrier is in the audio, once found.
    pub fn carrier_hz(&self) -> Option<f64> {
        self.search.offset_hz().map(|o| self.tone_hz + o)
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
                let y = self.blank(self.acc / self.m as f32);
                self.acc = Complex32::new(0.0, 0.0);
                self.acc_n = 0;
                self.block(y, out);
            }
        }
    }

    fn block(&mut self, y: Complex32, out: &mut Vec<EnvSample>) {
        self.search.push(y);
        if self.search.moves() != self.moves_seen {
            self.moves_seen = self.search.moves();
            self.moved_at = self.blocks as f64 * self.m as f64;
            // The phase loop was following whatever was at the old frequency,
            // and the centred average holds it.
            if let Some(pll) = &mut self.pll {
                *pll = PhaseLoop::new(self.rate / self.m as f64);
            }
            self.centred.clear();
            self.centred_sum = Complex32::new(0.0, 0.0);
        }
        let env_rate = self.env_rate();
        let found = self.search.offset_hz();
        if let Some(off) = found {
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
            if self.keying == Keying::Phase {
                self.across(pos, smooth, out);
                return;
            }
            let mag = if self.keying == Keying::AmplitudeAndSign {
                smooth.norm()
            } else {
                // The reference follows the newest samples and the smoothed
                // value is centred two behind; half a second's average has
                // not turned by anything measurable in two milliseconds.
                self.reference += (smooth - self.reference) / PHASE_TC;
                let r = self.reference.norm();
                if r > 0.0 { (smooth * self.reference.conj()).re / r } else { smooth.norm() }
            };
            // The loop runs only on a carrier the search has found: before
            // that it would be locking to the tone's neighbourhood.
            let iq = match (&mut self.pll, found) {
                (Some(pll), Some(_)) => pll.step(smooth).unwrap_or_default(),
                _ => Complex32::new(0.0, 0.0),
            };
            out.push(EnvSample { pos, mag, tpos: pos, timing: mag, sibling: 0.0, iq });
        }
    }

    /// ALS162: queues a smoothed sample, and reads the one a second back
    /// across the average of the two seconds around it.
    fn across(&mut self, pos: f64, smooth: Complex32, out: &mut Vec<EnvSample>) {
        let half = self.env_rate().round() as usize;
        self.centred.push_back((pos, smooth));
        self.centred_sum += smooth;
        if self.centred.len() < 2 * half + 1 {
            return;
        }
        let (pos, z) = self.centred[half];
        self.reference = self.centred_sum / self.centred.len() as f32;
        let r = self.reference.norm();
        let mag = if r > 0.0 { (z * self.reference.conj()).im / r } else { 0.0 };
        let zero = Complex32::new(0.0, 0.0);
        out.push(EnvSample { pos, mag, tpos: pos, timing: mag, sibling: 0.0, iq: zero });
        let (_, gone) = self.centred.pop_front().expect("full");
        self.centred_sum -= gone;
    }
}

/// The click limit as a multiple of quantile `q` of the samples' sizes, for
/// noise alone: complex Gaussian noise's size is Rayleigh distributed, whose
/// quantile `q` is σ √(−2 ln(1 − q)).
fn blank_ratio(q: f64) -> f32 {
    (BLANK_SIGMAS / (-2.0 * (1.0 - q).ln()).sqrt()) as f32
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
            let mut fe = FrontEnd::new(rate, 1000.0, Keying::Amplitude);
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

    /// Audio that starts silent, then a carrier: the carrier is not taken
    /// for a click for longer than it takes to measure it again.
    #[test]
    fn a_signal_after_silence_is_not_blanked() {
        let rate = 12_000.0;
        let mut x = vec![0.0f32; (2.0 * rate) as usize];
        x.extend(tone(rate, 1000.0, 10.0));
        let mut fe = FrontEnd::new(rate, 1000.0, Keying::Amplitude);
        let mut out = Vec::new();
        fe.process(&x, &mut out);
        let tail = &out[out.len() - 100..];
        assert!(tail.iter().all(|e| e.mag > 0.13), "the carrier is still blanked");
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
            let mut fe = FrontEnd::new(rate, 1000.0, Keying::Amplitude);
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
