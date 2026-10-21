//! The envelope to seconds, the seconds to symbols, the symbols to minutes.
//!
//! # Finding the second
//!
//! Every second of every code here starts with the same event, while what
//! follows it changes from second to second. On long wave the event is an
//! edge — the carrier going down (DCF77, MSF, WWVB) or coming up (JJY); on
//! short wave it is the tick. So the timing channel is *folded*: each sample
//! is added into one of a thousand bins by where it falls within its second,
//! and averaged over the last several seconds. What every second shares builds
//! up in the fold; the bits, which differ, average out to steps of their own
//! at 100 ms and beyond; and noise averages towards its mean. The sharpest
//! step of the right sign — or the tallest peak, for a tick — is where the
//! second begins. A fold works at signal levels where no single edge or tick
//! could be seen, which is the reason for it.
//!
//! Whether that step is real is a question about noise, so it is answered
//! against the fold's own: the scatter between bins too far apart to share a
//! filter's smoothing, which the keying's few steps barely touch. A step many
//! times that scatter is an edge; noise alone does not make one. The edge is
//! found with a wide detector, which sees furthest into the noise, and then
//! placed with a narrow one, which places it most precisely. A tick is short
//! enough that one detector does both: the fold's peak against the quiet zone
//! either side of it, placed between bins by the shape of the peak.
//!
//! ALS162's seconds are neither: each is marked by an element, a swing of
//! the carrier's phase up, down through zero on the second, and back. The
//! timing channel is the phase, and the fold is searched with the element's
//! own shape — a matched filter. The element averages to nothing, so a phase
//! offset the front end leaves does not move the answer; and the filter's
//! output is symmetric about its peak, so the peak is placed between bins the
//! way a tick is. What else matches an element in the fold — the second
//! element of every 1, a tenth of a second later, and what is left of the
//! station's pseudo-random code after sixteen seconds' averaging — can stand
//! half as tall as the real one, but unlike it, it changes from second to
//! second; so the filter's output is folded with its square as well, and
//! where it matches is held to how steadily it does.
//!
//! The fold is over the input's own sample clock, not the computer's: a
//! receiver whose sample rate is fifty parts per million out drifts by a
//! millisecond in twenty seconds, which the fold's memory is short enough to
//! follow.
//!
//! # Reading a second
//!
//! Every length any of these codes keys ends on a whole tenth, so a second is
//! measured as ten 100 ms slots — the middle 80 ms of each, clear of the
//! edges — normalised between the full and reduced levels, and compared with
//! each pattern the station sends.
//!
//! The two levels start from a fold of the level channel — its top and bottom
//! are full and reduced level in every code here — and then follow the
//! seconds that were read, each slot counted towards the level its pattern
//! says it was at. (The highest and lowest slot of each second would need no
//! read to have succeeded, but in noise they sit a standard deviation and more
//! outside the levels they stand for.) A run of seconds that will not read
//! reseeds them from the fold, so a bad start cannot hold the reader shut for
//! good. The closest pattern wins, if it wins clearly; otherwise the second
//! is left unread, which costs that minute and protects every minute after it.
//!
//! ALS162's seconds are read the same way with the element in place of the
//! slots: how much of an element there is on the second, and how much a
//! tenth of a second later, against the size of one. None, one and two are
//! the marker, a 0 and a 1.
//!
//! WWVB's seconds are also read for their phase: the carrier, turned onto its
//! own phase by the front end, summed over the part of the second at full
//! power — the part the amplitude pattern just read says is full, or where
//! it usually is when the pattern could not be read. The sum against the
//! noise in the other quadrature is a soft bit.
//!
//! # Reading a minute
//!
//! When a marker arrives the last minute's worth of symbols is handed to the
//! station's [`Format`], which decides whether it is a minute and what it
//! says. A second or two that could not be read lose the minute, except in
//! the codes whose parity can stand in for them (DCF77 and ALS162), where the
//! format fills them in first. WWVB's phase bits are tried every second, since a minute of them is
//! found by its sync word rather than by a marker; the two codes name the
//! same minute and are merged into one frame, so nothing downstream ever sees
//! a minute twice. DCF77 and ALS162 also hand every second, read or not, to
//! the integrator ([`crate::integrate`]) as likelihoods, which names the
//! minutes too weak to read one at a time from several together; where the
//! two both name a minute they have to agree. The instant a frame names is
//! tied to the input sample its second began on — and that sample, not the
//! computer's clock, is what leaves here.

use std::collections::VecDeque;

use sdroxide_types::{TIMESIG_TONE_HZ, TimeCode, TimeFrame, TimeStation};

use crate::format::{Context, Format, FrameError, Sym, Template, als162, wwvb_phase};
use crate::frontend::{EnvSample, FrontEnd, Keying};
use crate::hf::HfFrontEnd;
use crate::integrate::{Integrator, Soft};

/// Bins in the fold: one a millisecond.
const NB: usize = 1000;
/// How quickly the fold forgets: each bin moves this far towards each new
/// sample, which is a memory of about sixteen seconds — a sample clock fifty
/// parts per million out moves the edge less than a millisecond in that.
const FOLD_ALPHA: f32 = 1.0 / 16.0;
/// The wide edge detector's reach either side of a candidate edge, in bins.
/// 80 ms is inside the part of the second every symbol of every long-wave
/// code shares: full carrier for the last 100 ms before a falling edge and
/// reduced for the first 100 ms after it (JJY the other way about).
const EDGE_WIDE: usize = 80;
/// The narrow detector that places the edge, and how far either side of the
/// wide one's answer it looks.
const EDGE_NARROW: usize = 10;
const EDGE_REFINE: usize = 20;
/// The quiet either side of a tick's peak the peak is measured against, bins
/// from the peak: past the tick's own 11 ms response and inside the 40 ms the
/// stations keep clear around it.
const TICK_QUIET_FROM: usize = 8;
const TICK_QUIET_TO: usize = 20;
/// Where in the tick its matched filter peaks, seconds after the second: the
/// middle of the 5 ms burst.
const TICK_CENTRE_S: f64 = 0.0025;
/// Where around this station's tick the other station's is not looked for,
/// bins from the tick's middle. A 5 ms average nulls a steady tone 200 Hz
/// away but not a 5 ms burst it only half overlaps: this station's own tick
/// shows in the other's detector at a third of its height, 2.5 ms either side.
/// No wider, since the other station's ticks are as likely as not to arrive a
/// few milliseconds from these ones; the little else that leaks across — the
/// standard tones' gating at each end of the protected zone — is held off by
/// [`SIBLING_MIN_RATIO`].
const SIBLING_MASK: [(isize, isize); 1] = [(-7, 7)];
/// How tall the other station's peak has to be against this one's to be
/// counted as heard: what leaks across from this station's own gating is a
/// small fraction of its tick, and a second station is a tick of its own.
const SIBLING_MIN_RATIO: f64 = 0.2;
/// How many bins apart two fold bins must be for the envelope smoothing not
/// to correlate them (it spans five, on the timing channel of either band).
const NOISE_LAG: usize = 8;
/// Fold bins the smoothing makes one independent sample out of.
const CORR_BINS: f64 = 5.0;
/// How many standard deviations of noise the timing detector has to stand at
/// to be an edge or a tick. Its largest excursion on noise alone, over the
/// fold's couple of hundred independent positions, is about three; and a
/// lock on noise would still have to read a minute that passes its checks,
/// twice, before anything believed it.
const MIN_EDGE_Z: f32 = 6.0;
/// ALS162: how much before the second its reading begins, seconds.
const ELEMENT_LEAD_S: f64 = 0.051;
/// ALS162: the largest a second's misfit may be, in squared units of one
/// element, and how many times noise alone it may be, where that is more —
/// two readings' worth of noise variance, which a second fits within seven
/// times over all but once in a thousand.
const ELEMENT_MAX_COST: f32 = 0.5;
const ELEMENT_MAX_COST_NOISE: f32 = 7.0;
/// Seconds of envelope folded before the fold is asked anything.
const WARMUP_S: f64 = 6.0;
/// How old, on average, what the fold holds is, seconds: an average that
/// moves a sixteenth of the way each second remembers fifteen seconds back.
/// A sample clock off by tens of parts per million moves the second's start
/// by that much times its drift between when the fold saw it and now.
const FOLD_AGE_S: f64 = (1.0 - FOLD_ALPHA as f64) / FOLD_ALPHA as f64;
/// The estimates the drift is fitted over, and how many it needs first: a
/// minute of looks, so the fit's own noise times [`FOLD_AGE_S`] is a few
/// hundredths of a millisecond.
const DRIFT_KEPT: usize = 60;
const DRIFT_MIN: usize = 20;
/// The largest drift believed, seconds per second: a sample clock 200 ppm
/// out is a broken one.
const DRIFT_MAX: f64 = 200e-6;
/// How far a new estimate of the second's start may be from the one in hand
/// and still be the same edge, seconds. A weak edge is placed to within a few
/// milliseconds by any one look, and the estimate in hand averages them.
const SAME_EDGE_S: f64 = 0.006;
/// Consecutive agreeing estimates before the second is believed.
const LOCK_AFTER: u32 = 3;
/// Consecutive disagreeing estimates before it is given up.
const UNLOCK_AFTER: u32 = 4;
/// Envelope history kept, seconds — enough for a second being read plus the
/// slack of a late block.
const HISTORY_S: f64 = 3.0;
/// Each slot is read over its middle, this far in from either edge, seconds.
const SLOT_GUARD_S: f64 = 0.01;
/// The smallest margin, in squared normalised units, by which the best pattern
/// has to beat the next before a second counts as read. Two patterns differ
/// by at least one whole slot, which is a margin of 1 on a clean signal.
///
/// Low on purpose. An unread second loses its whole minute, while a misread
/// one is what the parity bits, the range checks and the agreement of
/// consecutive minutes are there to catch; only a true coin toss is left
/// unread.
const MIN_MARGIN: f32 = 0.1;
/// A second no pattern fits better than this is not any of them — a burst of
/// static, a fade — and is left unread.
const MAX_COST: f32 = 2.5;
/// …or than this many times what noise alone costs the right pattern, where
/// that is more: ten slots of the noise variance as read, which a second fits
/// within three times over all but once in a thousand. A fixed limit alone
/// would turn a weak but steady signal's every noisy second into an unread
/// one.
const MAX_COST_NOISE: f32 = 3.0;
/// How much likelier any other second is than WWV's hole, as a log: one second
/// in sixty is the hole. A pattern's cost is a log-likelihood in units of twice
/// the slots' noise variance, so the hole pays this times that variance —
/// when the second is clean, next to nothing; when it is not, a 0 that looks
/// a little like a hole is still read as the 0 it almost certainly is.
const HOLE_PRIOR: f32 = 4.08;
/// How fast the full and reduced levels follow the signal.
const LEVEL_BETA: f32 = 0.15;
/// Consecutive unread seconds after which the levels are reseeded from the
/// fold.
const RESEED_AFTER: u32 = 8;
/// How long after the last frame a minute with seconds filled in may still
/// be checked against it, seconds: an hour, over which even a sample clock
/// [`DRIFT_MAX`] out moves less than a second.
const FILL_CONTEXT_S: f64 = 3600.0;
/// Symbols kept: two minutes and a little over.
const SYMS_KEPT: usize = 130;
/// How likely each slot of a WWVB second is to be at full power when its
/// amplitude pattern could not be read: slots 2–4 only for a 0, 5–7 for
/// anything but a marker, 8–9 always. Slots 0 and 1 are never read for the
/// phase — the bit changes 100 ms into the second, and slot 1 is reduced in
/// every pattern.
const PHASE_WEIGHT: [f32; 10] = [0.0, 0.0, 0.65, 0.65, 0.65, 0.88, 0.88, 0.88, 1.0, 1.0];
/// How fast the phase reading's noise estimate follows, per second.
const PHASE_NOISE_BETA: f32 = 0.05;
/// Seconds of phase read before any is believed: the noise estimate needs a
/// few to mean anything.
const PHASE_WARMUP: u32 = 5;
/// How fast the noise the integrator's ratios are scaled by follows, per
/// second, and how many seconds of it are measured before any ratio is
/// handed over.
const PILOT_BETA: f32 = 0.05;
const PILOT_WARMUP: u32 = 10;

/// One second, read.
#[derive(Debug, Clone, Copy)]
pub struct Second {
    /// The input sample the second began on.
    pub pos: f64,
    /// What it said, or `None` if it could not be read.
    pub sym: Option<Sym>,
    /// How clearly: the best pattern's lead over the next, in squared
    /// normalised units.
    pub margin: f32,
    /// WWVB only: the phase bit as a soft value, in standard deviations of
    /// noise, its sign meaning nothing until a minute's sync has said which
    /// way up the bits are.
    pub phase: Option<f32>,
    /// The phase bit itself, once the sync has said which way up — and only
    /// when the soft value is clear enough to call.
    pub phase_bit: Option<bool>,
}

/// What the decoder found in a block of audio.
#[derive(Debug, Clone)]
pub enum Event {
    /// The start of each second is now known.
    Locked,
    /// The start of each second has been lost.
    LockLost,
    /// A second was read.
    Second(Second),
    /// A minute decoded: `frame.utc` is the instant input sample `pos` began.
    Frame { pos: f64, frame: TimeFrame, code: TimeCode },
    /// A minute was read in full and failed its checks.
    BadFrame { pos: f64, error: FrameError },
}

/// A picture of the receiver for the panel.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub carrier_hz: Option<f32>,
    pub level_db: f32,
    pub snr_db: f32,
    pub locked: bool,
    /// The level channel's fold, rotated to start at the second's start (a
    /// tenth of a second before it for ALS162), 100 points, 0–255.
    pub scope: Vec<u8>,
    /// WWVB: how cleanly the phase bits read, dB.
    pub phase_snr_db: Option<f32>,
    /// How far the timing detector's best stands out of the noise, standard
    /// deviations — whether this station's seconds are being heard at all.
    pub timing_z: f32,
    /// The same for the other station on these frequencies, short wave only.
    pub sibling_z: Option<f32>,
    /// Whether that other station's ticks are clearly there: a peak of their
    /// own, not this station's gating seen through the other's detector.
    pub sibling_heard: bool,
}

/// How the start of a second is found.
#[derive(Debug, Clone, Copy)]
enum Timing {
    /// A step in the carrier: up (`rising`) or down.
    Edge { rising: bool },
    /// A tick: a short burst whose middle is [`TICK_CENTRE_S`] after the
    /// second.
    Tick,
    /// ALS162's element: a swing of phase through zero on the second.
    Element,
}

// One per decoder, held for its whole life: the size of the smaller variant
// is not worth an indirection on every block.
#[allow(clippy::large_enum_variant)]
enum Fe {
    Lf(FrontEnd),
    Hf(HfFrontEnd),
}

impl Fe {
    fn process(&mut self, audio: &[f32], out: &mut Vec<EnvSample>) {
        match self {
            Fe::Lf(f) => f.process(audio, out),
            Fe::Hf(f) => f.process(audio, out),
        }
    }

    fn moves(&self) -> u32 {
        match self {
            Fe::Lf(f) => f.moves(),
            Fe::Hf(f) => f.moves(),
        }
    }

    fn carrier_hz(&self) -> Option<f64> {
        match self {
            Fe::Lf(f) => f.carrier_hz(),
            Fe::Hf(f) => f.carrier_hz(),
        }
    }

    fn moved_at(&self) -> f64 {
        match self {
            Fe::Lf(f) => f.moved_at(),
            Fe::Hf(f) => f.moved_at(),
        }
    }
}

/// A thousand bins of one channel, each the recent average of the samples
/// that fell at that point of the second.
struct Fold {
    v: Vec<f32>,
    /// Where in its bin, on average, each bin's samples fell, 0 to 1. Where
    /// the samples land depends on the input rate — at 12 kHz every one is
    /// nine tenths of the way through its millisecond — and a tick is placed
    /// to a fraction of a bin, so it has to be known.
    off: Vec<f32>,
    seen: Vec<bool>,
}

impl Fold {
    fn new() -> Self {
        Fold { v: vec![0.0; NB], off: vec![0.5; NB], seen: vec![false; NB] }
    }

    fn add(&mut self, t: f64, x: f32) {
        let at = t.rem_euclid(1.0) * NB as f64;
        let bin = (at as usize).min(NB - 1);
        let off = (at - bin as f64) as f32;
        if self.seen[bin] {
            self.v[bin] += FOLD_ALPHA * (x - self.v[bin]);
            self.off[bin] += FOLD_ALPHA * (off - self.off[bin]);
        } else {
            self.v[bin] = x;
            self.off[bin] = off;
            self.seen[bin] = true;
        }
    }

    fn clear(&mut self) {
        self.seen.iter_mut().for_each(|b| *b = false);
    }

    /// One bin's noise, from differences between bins the smoothing does not
    /// join: a robust spread, so the keying's few steps and a tick's one peak
    /// do not count.
    fn sigma(&self) -> f64 {
        let mut d: Vec<f64> = (0..NB)
            .map(|i| f64::from(self.v[i] - self.v[(i + NB - NOISE_LAG) % NB]).abs())
            .collect();
        d.sort_by(f64::total_cmp);
        1.4826 * d[NB / 2] / std::f64::consts::SQRT_2
    }

    /// The fold's sharpest step of the given sign: where, as a fraction of a
    /// second, and how many standard deviations of its noise it is.
    fn edge(&self, rising: bool) -> (f64, f32) {
        // Circular prefix sums so a window may run across the end.
        let mut pre = vec![0.0f64; 2 * NB + 1];
        for i in 0..2 * NB {
            pre[i + 1] = pre[i] + f64::from(self.v[i % NB]);
        }
        let mean = |from: usize, len: usize| (pre[from + len] - pre[from]) / len as f64;
        // Bins b−w … b−1 against b … b+w−1; the first window is read a lap
        // later so it never starts below zero.
        let step = |b: usize, w: usize| {
            let b = b % NB;
            let before = mean(b + NB - w, w);
            let after = mean(b, w);
            if rising { after - before } else { before - after }
        };
        let (wide_at, wide) = (0..NB)
            .map(|b| (b, step(b, EDGE_WIDE)))
            .fold((0, f64::MIN), |best, x| if x.1 > best.1 { x } else { best });
        let sigma_step = self.sigma() * (2.0 * CORR_BINS / EDGE_WIDE as f64).sqrt();
        let z = (wide / sigma_step.max(1e-12)) as f32;

        let (mut at, mut best) = (wide_at, f64::MIN);
        for b in wide_at + NB - EDGE_REFINE..=wide_at + NB + EDGE_REFINE {
            let v = step(b, EDGE_NARROW);
            if v > best {
                best = v;
                at = b % NB;
            }
        }
        let (l, r) = (step(at + NB - 1, EDGE_NARROW), step(at + 1, EDGE_NARROW));
        let den = l - 2.0 * best + r;
        let frac = if den.abs() > 1e-12 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 };
        (((at as f64 + frac) / NB as f64).rem_euclid(1.0), z)
    }

    /// The fold's tallest peak above the quiet either side of it: where the
    /// second starts, as a fraction of a second, how many standard deviations
    /// of its noise the peak stands, and its height. Peaks within
    /// [`SIBLING_MASK`] of the second starting at `not_near` are passed over.
    fn tick(&self, not_near: Option<f64>) -> (f64, f32, f64) {
        let at = |i: isize| f64::from(self.v[i.rem_euclid(NB as isize) as usize]);
        let quiet = |b: isize| {
            let (from, to) = (TICK_QUIET_FROM as isize, TICK_QUIET_TO as isize);
            let n = 2 * (to - from + 1);
            ((from..=to).map(|d| at(b - d) + at(b + d)).sum::<f64>()) / n as f64
        };
        let masked = |b: isize| {
            not_near.is_some_and(|p| {
                let centre = ((p + TICK_CENTRE_S) * NB as f64).round() as isize;
                // From the tick's middle, wrapped to ±half a second.
                let d = (b - centre + NB as isize / 2).rem_euclid(NB as isize) - NB as isize / 2;
                SIBLING_MASK.iter().any(|&(lo, hi)| (lo..=hi).contains(&d))
            })
        };
        let (peak_at, peak) = (0..NB as isize)
            .filter(|&b| !masked(b))
            .map(|b| (b, at(b) - quiet(b)))
            .fold((0, f64::MIN), |best, x| if x.1 > best.1 { x } else { best });
        let n_quiet = 2.0 * (TICK_QUIET_TO - TICK_QUIET_FROM + 1) as f64;
        let sigma_peak = self.sigma() * (1.0 + CORR_BINS / n_quiet).sqrt();
        let z = (peak / sigma_peak.max(1e-12)) as f32;
        // A tick's matched filter rises and falls in straight lines with a
        // rounded top, symmetric about the tick's middle. The two points where
        // it crosses half its height are on the straight parts, where
        // interpolating between bins is exact, and the middle is between them.
        let half = quiet(peak_at) + peak / 2.0;
        // Where bin b's samples are, in bins, unwrapped around the peak.
        let pos = |b: isize| b as f64 + f64::from(self.off[b.rem_euclid(NB as isize) as usize]);
        let cross = |dir: isize| {
            let mut b = peak_at;
            for _ in 0..TICK_QUIET_FROM {
                let next = b + dir;
                if at(next) < half {
                    let (v0, v1) = (at(b), at(next));
                    return pos(b) + (pos(next) - pos(b)) * (v0 - half) / (v0 - v1);
                }
                b = next;
            }
            pos(b)
        };
        let middle = (cross(-1) + cross(1)) / 2.0;
        ((middle / NB as f64 - TICK_CENTRE_S).rem_euclid(1.0), z, peak)
    }
}

/// ALS162's timing channel run through a filter matched to the element, and
/// folded twice: the output, and its square.
///
/// The one fold says how well each point of the second matches an element on
/// average; the two together say how steadily it does, second after second.
/// Every second but the marker carries the same element, so on the second
/// the output barely varies beyond the noise. Everything else that matches an
/// element in the average — the second element of a 1, which only some
/// seconds send, and what the station's pseudo-random code leaves behind —
/// changes from second to second, and is told apart by it.
///
/// Each bin is a plain mean of what it has seen until it has seen
/// 1/[`FOLD_ALPHA`] seconds, and the usual moving average after: its spread is
/// measured from the first few seconds, when an average that started from
/// the first sample would still be mostly that one.
struct MatchedFold {
    /// The element at the envelope's sample spacing, centred.
    taps: Vec<f32>,
    energy: f32,
    /// The last `taps.len()` timing samples, seconds of input and value.
    ring: VecDeque<(f64, f32)>,
    mean: Vec<f32>,
    square: Vec<f32>,
    /// The sum of the squared weights each bin's mean is made of: how much of
    /// one sample's variance its mean carries.
    weight2: Vec<f32>,
    seen: Vec<u32>,
    off: Vec<f32>,
}

impl MatchedFold {
    fn new(env_rate: f64) -> Self {
        let half = (als162::ELEMENT_S / 2.0 * env_rate).ceil() as isize;
        let taps: Vec<f32> =
            (-half..=half).map(|k| als162::element(k as f64 / env_rate) as f32).collect();
        MatchedFold {
            energy: taps.iter().map(|w| w * w).sum(),
            taps,
            ring: VecDeque::new(),
            mean: vec![0.0; NB],
            square: vec![0.0; NB],
            weight2: vec![1.0; NB],
            seen: vec![0; NB],
            off: vec![0.5; NB],
        }
    }

    fn clear(&mut self) {
        self.ring.clear();
        self.seen.iter_mut().for_each(|n| *n = 0);
    }

    fn add(&mut self, t: f64, x: f32) {
        self.ring.push_back((t, x));
        if self.ring.len() < self.taps.len() {
            return;
        }
        let y: f32 = self.ring.iter().zip(&self.taps).map(|(&(_, x), w)| w * x).sum();
        let t = self.ring[self.taps.len() / 2].0;
        self.ring.pop_front();
        let at = t.rem_euclid(1.0) * NB as f64;
        let bin = (at as usize).min(NB - 1);
        let off = (at - bin as f64) as f32;
        self.seen[bin] = self.seen[bin].saturating_add(1);
        let a = (1.0 / self.seen[bin] as f32).max(FOLD_ALPHA);
        if self.seen[bin] == 1 {
            self.mean[bin] = y;
            self.square[bin] = y * y;
            self.weight2[bin] = 1.0;
            self.off[bin] = off;
        } else {
            self.mean[bin] += a * (y - self.mean[bin]);
            self.square[bin] += a * (y * y - self.square[bin]);
            self.weight2[bin] = (1.0 - a) * (1.0 - a) * self.weight2[bin] + a * a;
            self.off[bin] += a * (off - self.off[bin]);
        }
    }

    /// Where the element is: as a fraction of a second, how many standard
    /// deviations of noise it stands, and its height in units of one element.
    /// `sigma_bin` is one bin's noise in the plain fold of the same channel.
    ///
    /// Each point is held to two standards: its mean against the fold's
    /// noise, as every other detector here is, and against its own spread from
    /// second to second. Noise fails the first; the second element and the
    /// pseudo-random code, which are strong but unsteady, fail the second.
    /// A point's score is the lesser of the two.
    fn find(&self, sigma_bin: f64) -> (f64, f32, f64) {
        let sigma = (sigma_bin * (CORR_BINS * f64::from(self.energy)).sqrt()).max(1e-12);
        let score = |b: usize| {
            let (m, w2) = (f64::from(self.mean[b]), f64::from(self.weight2[b]));
            if self.seen[b] < 2 || w2 >= 1.0 {
                return f64::MIN;
            }
            // One output's spread, unbiased for the weights it was averaged
            // with, times how much of it the mean keeps.
            let var = (f64::from(self.square[b]) - m * m).max(0.0) / (1.0 - w2) * w2;
            (m / sigma).min(m / var.sqrt().max(1e-12))
        };
        let best = (0..NB).max_by(|&a, &b| score(a).total_cmp(&score(b))).unwrap_or(0);
        let z = score(best);
        // The best score is near the top of its peak, not necessarily on it:
        // climb to the top of the mean, and place it between bins by its
        // curvature.
        let at = |i: isize| f64::from(self.mean[i.rem_euclid(NB as isize) as usize]);
        let mut peak = best as isize;
        for _ in 0..(als162::ELEMENT_S * NB as f64 / 2.0) as usize {
            let up = if at(peak + 1) > at(peak) {
                1
            } else if at(peak - 1) > at(peak) {
                -1
            } else {
                break;
            };
            peak += up;
        }
        let (l, c, r) = (at(peak - 1), at(peak), at(peak + 1));
        let den = l - 2.0 * c + r;
        let frac = if den.abs() > 1e-12 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 };
        let p = peak.rem_euclid(NB as isize) as usize;
        let pos = (peak as f64 + frac + f64::from(self.off[p])) / NB as f64;
        (pos.rem_euclid(1.0), z as f32, c / f64::from(self.energy))
    }
}

pub struct Decoder {
    fmt: &'static Format,
    rate: f64,
    fe: Fe,
    timing: Timing,
    scratch: Vec<EnvSample>,
    env: VecDeque<EnvSample>,
    /// The timing channel folded, which the second is found in.
    timing_fold: Fold,
    /// The level channel folded, which the levels are seeded from and the
    /// scope drawn from. The same as the timing fold on long wave.
    level_fold: Fold,
    /// The other short-wave station's ticks, folded.
    sibling_fold: Option<Fold>,
    /// ALS162's timing channel through its element's matched filter.
    matched: Option<MatchedFold>,
    /// The height of ALS162's element in the fold, at the last look.
    element_height: f64,
    /// The time, seconds of input, the fold started and was last examined.
    fold_start: Option<f64>,
    examined: f64,
    /// Where each second starts, as a fraction of a second of input time.
    second_phase: Option<f64>,
    /// The fold's recent estimates of it, unwrapped, against input time —
    /// what the sample clock's drift is fitted from.
    looks: VecDeque<(f64, f64)>,
    /// When the fold was last looked at, seconds of input.
    last_look: Option<f64>,
    agree: u32,
    disagree: u32,
    edge_z: f32,
    sibling_z: Option<f32>,
    /// The other station's ticks are there to be locked to.
    sibling_heard: bool,
    locked: bool,
    /// The front end's move count when the fold was started.
    fe_moves: u32,
    /// The start of the next second to read, seconds of input time.
    next: Option<f64>,
    hi: f32,
    lo: f32,
    /// Running mean square of a read second's misfit, per slot.
    misfit: f32,
    unread: u32,
    syms: VecDeque<Second>,
    ctx: Context,
    /// The input sample the last frame named, and the instant it named.
    last_frame: Option<(f64, i64)>,
    /// WWVB's phase code is read.
    phase_code: bool,
    /// The phase reading's noise: the mean square of the other quadrature's
    /// weighted sum, per unit of squared weight.
    phase_noise: f32,
    phase_seconds: u32,
    /// The recent mean of a phase bit's size, for the panel.
    phase_level: f32,
    /// Which way up the phase bits are, from the last minute's sync.
    polarity: Option<f32>,
    /// DCF77 and ALS162: minutes read several at a time, for the ones too
    /// weak to read alone.
    integ: Option<Integrator>,
    /// The level each slot is at in every pattern the station sends, where
    /// they all agree — what the noise is measured on.
    pilot: [Option<f32>; 10],
    /// The noise in one normalised reading, as a variance, and how many
    /// seconds it has been measured over.
    pilot_var: f32,
    pilot_seconds: u32,
}

impl Decoder {
    pub fn new(station: TimeStation, rate: f64) -> Self {
        let fmt = Format::for_station(station);
        let tone = f64::from(TIMESIG_TONE_HZ);
        let full = full_share(fmt);
        let (fe, timing) = match station {
            _ if station.is_hf() => (Fe::Hf(HfFrontEnd::new(station, rate, tone)), Timing::Tick),
            TimeStation::Als162 => (
                Fe::Lf(FrontEnd::new(rate, tone, Keying::Phase).with_full_share(full)),
                Timing::Element,
            ),
            // WWVB keys its carrier's phase as well: see the front end.
            TimeStation::Wwvb => (
                Fe::Lf(FrontEnd::new(rate, tone, Keying::AmplitudeAndSign).with_full_share(full)),
                Timing::Edge { rising: fmt.rising_edge },
            ),
            _ => (
                Fe::Lf(FrontEnd::new(rate, tone, Keying::Amplitude).with_full_share(full)),
                Timing::Edge { rising: fmt.rising_edge },
            ),
        };
        let matched = match (&fe, timing) {
            (Fe::Lf(f), Timing::Element) => Some(MatchedFold::new(f.env_rate())),
            _ => None,
        };
        let integ = matches!(station, TimeStation::Dcf77 | TimeStation::Als162)
            .then(|| Integrator::new(fmt, rate));
        let mut pilot = [None; 10];
        for (k, p) in pilot.iter_mut().enumerate() {
            let low = fmt.templates[0].low[k];
            if fmt.templates.iter().all(|t| t.low[k] == low) {
                *p = Some(if low { 0.0 } else { 1.0 });
            }
        }
        Decoder {
            fmt,
            rate,
            fe,
            timing,
            scratch: Vec::new(),
            env: VecDeque::new(),
            timing_fold: Fold::new(),
            level_fold: Fold::new(),
            sibling_fold: station.sibling().map(|_| Fold::new()),
            matched,
            element_height: 0.0,
            fold_start: None,
            examined: 0.0,
            second_phase: None,
            looks: VecDeque::new(),
            last_look: None,
            agree: 0,
            disagree: 0,
            edge_z: 0.0,
            sibling_z: None,
            sibling_heard: false,
            locked: false,
            fe_moves: 0,
            next: None,
            hi: 0.0,
            lo: 0.0,
            misfit: 1.0,
            unread: 0,
            syms: VecDeque::new(),
            ctx: Context::default(),
            last_frame: None,
            phase_code: station == TimeStation::Wwvb,
            phase_noise: 0.0,
            phase_seconds: 0,
            phase_level: 0.0,
            polarity: None,
            integ,
            pilot,
            pilot_var: 0.0,
            pilot_seconds: 0,
        }
    }

    pub fn station(&self) -> TimeStation {
        self.fmt.station
    }

    pub fn rate(&self) -> f64 {
        self.rate
    }

    pub fn process(&mut self, audio: &[f32], events: &mut Vec<Event>) {
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        self.fe.process(audio, &mut scratch);
        let mut from = f64::MIN;
        if self.fe.moves() != self.fe_moves {
            // Everything folded so far was measured off the carrier, and so
            // was whatever this block made before the carrier moved.
            self.fe_moves = self.fe.moves();
            self.restart(events);
            from = self.fe.moved_at();
        }
        for &e in scratch.iter().filter(|e| e.pos >= from) {
            self.envelope(e, events);
        }
        self.scratch = scratch;
    }

    fn envelope(&mut self, e: EnvSample, events: &mut Vec<Event>) {
        let t = e.pos / self.rate;
        let tt = e.tpos / self.rate;
        self.timing_fold.add(tt, e.timing);
        self.level_fold.add(t, e.mag);
        if let Some(f) = &mut self.sibling_fold {
            f.add(tt, e.sibling);
        }
        if let Some(f) = &mut self.matched {
            f.add(tt, e.timing);
        }
        let start = *self.fold_start.get_or_insert(t);
        self.env.push_back(e);
        while self.env.front().is_some_and(|f| f.pos / self.rate < t - HISTORY_S) {
            self.env.pop_front();
        }
        if t - start >= WARMUP_S && t - self.examined >= 1.0 {
            self.examined = t;
            self.examine(t, events);
        }
        while let Some(start) = self.next {
            if t < start + 1.0 + SLOT_GUARD_S {
                break;
            }
            self.read_second(start, events);
            self.next = self.second_phase.map(|p| snap(start + 1.0, p));
        }
    }

    /// Starts again from nothing: empty folds, no lock, no symbols.
    fn restart(&mut self, events: &mut Vec<Event>) {
        if self.locked {
            events.push(Event::LockLost);
        }
        self.timing_fold.clear();
        self.level_fold.clear();
        if let Some(f) = &mut self.sibling_fold {
            f.clear();
        }
        if let Some(f) = &mut self.matched {
            f.clear();
        }
        self.fold_start = None;
        self.second_phase = None;
        self.looks.clear();
        self.agree = 0;
        self.disagree = 0;
        self.locked = false;
        self.next = None;
        self.syms.clear();
        self.polarity = None;
        self.phase_seconds = 0;
        if let Some(it) = &mut self.integ {
            it.reset();
        }
        self.pilot_seconds = 0;
    }

    /// The sample clock's drift against the station, seconds per second, from
    /// a straight line through the fold's recent estimates.
    fn drift(&self) -> f64 {
        let n = self.looks.len();
        if n < DRIFT_MIN {
            return 0.0;
        }
        let (mt, me) = self.looks.iter().fold((0.0, 0.0), |(a, b), &(t, e)| (a + t, b + e));
        let (mt, me) = (mt / n as f64, me / n as f64);
        let (sxy, sxx) = self.looks.iter().fold((0.0, 0.0), |(xy, xx), &(t, e)| {
            (xy + (t - mt) * (e - me), xx + (t - mt) * (t - mt))
        });
        if sxx <= 0.0 { 0.0 } else { (sxy / sxx).clamp(-DRIFT_MAX, DRIFT_MAX) }
    }

    /// Looks for the second's start in the fold.
    fn examine(&mut self, now: f64, events: &mut Vec<Event>) {
        let (raw, z, height) = match self.timing {
            Timing::Edge { rising } => {
                let (e, z) = self.timing_fold.edge(rising);
                (e, z, 0.0)
            }
            Timing::Tick => self.timing_fold.tick(None),
            Timing::Element => {
                let sigma = self.timing_fold.sigma();
                let (e, z, h) = self.matched.as_ref().expect("made for Element").find(sigma);
                self.element_height = h;
                (e, z, 0.0)
            }
        };
        self.edge_z = z;
        let credible = z >= MIN_EDGE_Z;
        // The fold says where the second started some fifteen seconds ago,
        // on average; carried forward by the drift, where it starts now.
        if credible {
            let unwrapped = match self.looks.back() {
                Some(&(_, last)) => last + circ(raw - last),
                None => raw,
            };
            self.looks.push_back((now, unwrapped));
            if self.looks.len() > DRIFT_KEPT {
                self.looks.pop_front();
            }
        }
        let drift = self.drift();
        let edge = (raw + drift * FOLD_AGE_S).rem_euclid(1.0);
        let own = credible.then_some(raw);
        let sibling = self.sibling_fold.as_ref().map(|f| f.tick(own));
        self.sibling_z = sibling.map(|(_, z, _)| z);
        self.sibling_heard = sibling.is_some_and(|(_, sz, sh)| {
            sz >= MIN_EDGE_Z && (!credible || sh >= SIBLING_MIN_RATIO * height)
        });
        let since = now - self.last_look.unwrap_or(now);
        self.last_look = Some(now);
        match self.second_phase {
            Some(p) if credible && circ(edge - p).abs() < SAME_EDGE_S => {
                // Follow it slowly: the estimate is noisy, the clock it is
                // tracking is not — and it is where the drift has taken it
                // since the last look, not where it was then.
                let p = p + drift * since;
                self.second_phase = Some((p + 0.25 * circ(edge - p)).rem_euclid(1.0));
                self.agree += 1;
                self.disagree = 0;
            }
            _ => {
                self.disagree += 1;
                self.agree = 0;
                if !self.locked || self.disagree >= UNLOCK_AFTER {
                    if self.locked {
                        self.locked = false;
                        self.next = None;
                        self.syms.clear();
                        events.push(Event::LockLost);
                    }
                    if !credible || self.second_phase.is_some() {
                        // Whatever the drift was fitted to, it was not this.
                        self.looks.clear();
                    }
                    self.second_phase = credible.then_some(edge);
                }
            }
        }
        if !self.locked && self.agree >= LOCK_AFTER {
            let p = self.second_phase.expect("agreeing estimates imply a phase");
            self.locked = true;
            self.disagree = 0;
            self.seed_levels();
            // The first whole second still in the history — with its element
            // from 50 ms before it, for ALS162.
            let lead =
                if matches!(self.timing, Timing::Element) { ELEMENT_LEAD_S } else { SLOT_GUARD_S };
            let oldest = self.env.front().map_or(now, |e| e.pos / self.rate);
            let mut first = snap(oldest + lead, p);
            if first < oldest + lead {
                first += 1.0;
            }
            self.next = Some(first);
            events.push(Event::Locked);
        }
    }

    /// Starting levels, from the level fold: its top and bottom twentieths
    /// are full and reduced level in every code here. ALS162's are one
    /// element and none, and the size of an element is what the fold's match
    /// to one says.
    fn seed_levels(&mut self) {
        if matches!(self.timing, Timing::Element) {
            self.lo = 0.0;
            self.hi = self.element_height as f32;
            return;
        }
        let mut v = self.level_fold.v.clone();
        v.sort_by(f32::total_cmp);
        let n = NB / 20;
        self.lo = v[..n].iter().sum::<f32>() / n as f32;
        self.hi = v[NB - n..].iter().sum::<f32>() / n as f32;
    }

    /// Each slot's sum and count over its middle, for the second starting at
    /// `start` seconds of input.
    fn slots(&self, start: f64) -> ([f32; 10], [u32; 10]) {
        let mut sum = [0.0f32; 10];
        let mut n = [0u32; 10];
        for e in &self.env {
            let r = e.pos / self.rate - start;
            if !(0.0..1.0).contains(&r) {
                continue;
            }
            let k = ((r * 10.0) as usize).min(9);
            let into = r - k as f64 * 0.1;
            if (SLOT_GUARD_S..0.1 - SLOT_GUARD_S).contains(&into) {
                sum[k] += e.mag;
                n[k] += 1;
            }
        }
        (sum, n)
    }

    /// ALS162's second starting at `start`: how much of an element there is
    /// on the second and a tenth of a second after it, in units of one, to
    /// the number of elements the second carried.
    fn read_elements(&mut self, start: f64) -> (Option<(Sym, f32)>, Option<Soft>) {
        let (mut a, mut w2) = ([0.0f32; 2], [0.0f32; 2]);
        for e in &self.env {
            let r = e.pos / self.rate - start;
            for (k, (a, w2)) in a.iter_mut().zip(&mut w2).enumerate() {
                let w = als162::element(r - k as f64 * als162::ELEMENT_S) as f32;
                *a += w * e.mag;
                *w2 += w * w;
            }
        }
        if w2.contains(&0.0) || self.hi <= 0.0 {
            return (None, None);
        }
        let y = [a[0] / w2[0] / self.hi, a[1] / w2[1] / self.hi];
        let fit = |sym: Sym| {
            let n = als162::elements(sym);
            (0..2)
                .map(|k| {
                    let want = if k < n { 1.0 } else { 0.0 };
                    (y[k] - want) * (y[k] - want)
                })
                .sum::<f32>()
        };
        // The first element is there in every second but the marker: how far
        // it is from one is the noise.
        let pilot = (fit(Sym::MARKER) > fit(Sym::ZERO).min(fit(Sym::ONE)))
            .then_some((y[0] - 1.0) * (y[0] - 1.0));
        let soft = self.soft(|sym| Some(fit(sym)), pilot);
        let mut costs: Vec<(f32, f32, Sym)> = als162::SYMBOLS
            .iter()
            .map(|&(sym, n)| {
                let c = (0..2)
                    .map(|k| {
                        let want = if k < n { 1.0 } else { 0.0 };
                        (y[k] - want) * (y[k] - want)
                    })
                    .sum::<f32>();
                // One second in sixty is the marker, as one in sixty is
                // WWV's hole; see `HOLE_PRIOR`.
                let prior = if sym.marker { 2.0 * HOLE_PRIOR * self.misfit } else { 0.0 };
                (c + prior, c, sym)
            })
            .collect();
        costs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (best, fit, sym) = costs[0];
        let margin = costs[1].0 - best;
        let max_cost = ELEMENT_MAX_COST.max(ELEMENT_MAX_COST_NOISE * 2.0 * self.misfit);
        if margin < MIN_MARGIN || fit > max_cost {
            return (None, soft);
        }
        // Each element read says what the size of one is now.
        let n = als162::elements(sym);
        if n > 0 {
            let seen = y[..n].iter().sum::<f32>() / n as f32 * self.hi;
            self.hi += LEVEL_BETA * (seen - self.hi);
        }
        self.misfit += 0.1 * (fit / 2.0 - self.misfit);
        (Some((sym, margin)), soft)
    }

    /// What the integrator is handed for a second: each pattern's misfit
    /// (`cost`, squared normalised units) as a log-likelihood, against the
    /// noise measured where every pattern agrees — `pilot`, this second's
    /// squared deviation there, if it has such a place.
    ///
    /// Measured on the pilot rather than taken from how well the chosen
    /// pattern fits: on a weak signal the best fit is partly the noise's own
    /// doing, and the seconds that fit worst are the ones not read at all, so
    /// the fit says the noise is less than it is — and ratios scaled by it
    /// claim more than they know.
    fn soft(&mut self, cost: impl Fn(Sym) -> Option<f32>, pilot: Option<f32>) -> Option<Soft> {
        self.integ.as_ref()?;
        if let Some(r) = pilot {
            self.pilot_var = if self.pilot_seconds == 0 {
                r
            } else {
                self.pilot_var + PILOT_BETA * (r - self.pilot_var)
            };
            self.pilot_seconds += 1;
        }
        if self.pilot_seconds < PILOT_WARMUP {
            return None;
        }
        let scale = 0.5 / self.pilot_var.max(1e-4);
        let l = |sym: Sym| cost(sym).map(|c| -c * scale);
        let (zero, one, marker) = (l(Sym::ZERO)?, l(Sym::ONE)?, l(Sym::MARKER)?);
        Some(Soft { bit: one - zero, marker: marker - zero.max(one) })
    }

    fn read_second(&mut self, start: f64, events: &mut Vec<Event>) {
        if matches!(self.timing, Timing::Element) {
            let (read, soft) = self.read_elements(start);
            let named = self.integ.as_mut().and_then(|it| it.push(start * self.rate, soft));
            self.unread = if read.is_some() { 0 } else { self.unread + 1 };
            if self.unread >= RESEED_AFTER {
                self.unread = 0;
                self.seed_levels();
            }
            let second = Second {
                pos: start * self.rate,
                sym: read.map(|(s, _)| s),
                margin: read.map_or(0.0, |(_, m)| m),
                phase: None,
                phase_bit: None,
            };
            self.push_second(second, named, events);
            return;
        }
        let (sum, n) = self.slots(start);
        let mut soft = None;
        let read = if n.iter().all(|&c| c > 0) && self.hi > self.lo {
            let span = self.hi - self.lo;
            let y: Vec<f32> = (0..10).map(|k| (sum[k] / n[k] as f32 - self.lo) / span).collect();
            let fmt = self.fmt;
            let fit = |sym: Sym| {
                fmt.template(sym).map(|t| {
                    (0..10)
                        .map(|k| {
                            let want = if t.low[k] { 0.0 } else { 1.0 };
                            (y[k] - want) * (y[k] - want)
                        })
                        .sum::<f32>()
                })
            };
            let pilot: Vec<f32> = (0..10)
                .filter_map(|k| self.pilot[k].map(|want| (y[k] - want) * (y[k] - want)))
                .collect();
            let pilot = (!pilot.is_empty()).then(|| pilot.iter().sum::<f32>() / pilot.len() as f32);
            soft = self.soft(fit, pilot);
            let mut costs: Vec<(f32, usize)> = self
                .fmt
                .templates
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let c = (0..10)
                        .map(|k| {
                            let want = if t.low[k] { 0.0 } else { 1.0 };
                            (y[k] - want) * (y[k] - want)
                        })
                        .sum::<f32>();
                    // `misfit` is the slots' noise variance as read.
                    let prior = if t.sym.hole { 2.0 * HOLE_PRIOR * self.misfit } else { 0.0 };
                    (c + prior, i)
                })
                .collect();
            costs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (best, idx) = costs[0];
            let margin = costs.get(1).map_or(f32::MAX, |c| c.0) - best;
            let t = &self.fmt.templates[idx];
            let max_cost = MAX_COST.max(MAX_COST_NOISE * 10.0 * self.misfit);
            if margin >= MIN_MARGIN && best <= max_cost {
                // The slots the winning pattern says were at full level say
                // what full level is now, and likewise the reduced ones.
                let level = |low: bool| {
                    let ks: Vec<usize> = (0..10).filter(|&k| t.low[k] == low).collect();
                    (!ks.is_empty()).then(|| {
                        ks.iter().map(|&k| sum[k] / n[k] as f32).sum::<f32>() / ks.len() as f32
                    })
                };
                if let Some(h) = level(false) {
                    self.hi += LEVEL_BETA * (h - self.hi);
                }
                if let Some(l) = level(true) {
                    self.lo += LEVEL_BETA * (l - self.lo);
                }
                self.misfit += 0.1 * (best / 10.0 - self.misfit);
                Some((t, margin))
            } else {
                None
            }
        } else {
            None
        };
        let named = self.integ.as_mut().and_then(|it| it.push(start * self.rate, soft));
        self.unread = if read.is_some() { 0 } else { self.unread + 1 };
        if self.unread >= RESEED_AFTER {
            self.unread = 0;
            self.seed_levels();
        }
        let phase =
            if self.phase_code { self.read_phase(start, read.map(|(t, _)| t)) } else { None };
        let second = Second {
            pos: start * self.rate,
            sym: read.map(|(t, _)| t.sym),
            margin: read.map_or(0.0, |(_, m)| m),
            phase,
            phase_bit: phase.zip(self.polarity).and_then(|(s, p)| wwvb_phase::bit(s, p)),
        };
        self.push_second(second, named, events);
    }

    /// A second read: reported, kept, and — if it ends a minute — decoded.
    /// `named` is the minute the integrator says starts after it.
    fn push_second(&mut self, second: Second, named: Option<i64>, events: &mut Vec<Event>) {
        events.push(Event::Second(second));
        if self.syms.len() == SYMS_KEPT {
            self.syms.pop_front();
        }
        self.syms.push_back(second);
        let amplitude =
            if second.sym.is_some_and(|s| s.marker) { self.amplitude_frame() } else { None };
        let phase = if self.phase_code { self.phase_frame() } else { None };
        // The integrator names a minute by its bits as the station sends
        // them, so the format reads it back the same way any minute is read.
        let integrated = named.and_then(|utc| {
            let sent = (self.fmt.encode)(utc - 60);
            let frame = (self.fmt.decode)(&sent, &self.ctx).ok()?;
            (frame.utc == utc).then_some((second.pos + self.rate, frame))
        });
        self.report(amplitude, phase, integrated, events);
    }

    /// The phase bit of the second starting at `start`, as a soft value: the
    /// carrier's in-phase part summed over the full-power part of the second,
    /// against the noise in the other quadrature.
    fn read_phase(&mut self, start: f64, read: Option<&Template>) -> Option<f32> {
        let (mut re, mut im, mut w2) = (0.0f32, 0.0f32, 0.0f32);
        let mut live = false;
        for e in &self.env {
            let r = e.pos / self.rate - start;
            if !(0.0..1.0).contains(&r) {
                continue;
            }
            let k = ((r * 10.0) as usize).min(9);
            let into = r - k as f64 * 0.1;
            if !(SLOT_GUARD_S..0.1 - SLOT_GUARD_S).contains(&into) {
                continue;
            }
            // NIST's advice: read the phase where the power is full, which
            // the pattern just read says, or where it usually is.
            // Slots 0 and 1 never, as for `PHASE_WEIGHT`.
            let w = match read {
                Some(t) => {
                    if k >= 2 && !t.low[k] {
                        1.0
                    } else {
                        0.0
                    }
                }
                None => PHASE_WEIGHT[k],
            };
            re += w * e.iq.re;
            im += w * e.iq.im;
            w2 += w * w;
            live |= e.iq.re != 0.0 || e.iq.im != 0.0;
        }
        if !live || w2 == 0.0 {
            return None;
        }
        let noise = im * im / w2;
        self.phase_noise = if self.phase_seconds == 0 {
            noise
        } else {
            self.phase_noise + PHASE_NOISE_BETA * (noise - self.phase_noise)
        };
        self.phase_seconds += 1;
        if self.phase_seconds < PHASE_WARMUP {
            return None;
        }
        let soft = re / (self.phase_noise * w2).sqrt().max(f32::MIN_POSITIVE);
        self.phase_level += 0.1 * (soft.abs() - self.phase_level);
        Some(soft)
    }

    /// The last `len` seconds, if they are one second apart each — a minute
    /// of air with nothing missing.
    fn window(&self, len: usize) -> Option<Vec<Second>> {
        if self.syms.len() < len {
            return None;
        }
        let window: Vec<Second> = self.syms.iter().skip(self.syms.len() - len).copied().collect();
        let contiguous = window.windows(2).all(|p| {
            let gap = (p[1].pos - p[0].pos) / self.rate;
            (gap - 1.0).abs() < 0.01
        });
        contiguous.then_some(window)
    }

    /// The minute the amplitude code says, now its trigger marker is in.
    /// `None` when there is nothing to report: a gap, an unread second, or a
    /// marker that does not end a minute.
    ///
    /// A minute with seconds filled in ([`Format::fill`]) has used up some of
    /// its checks doing it, and two misread bits in one parity group pass as
    /// none. So it is believed only if it names exactly the minute the input's
    /// own clock says has come since the last frame — which a wrong one would
    /// have to hit by chance — and a filled minute that fails is not counted
    /// as a bad one, since it was never read in full.
    fn amplitude_frame(&self) -> Option<(f64, Result<TimeFrame, FrameError>)> {
        let len = self.fmt.frame_len;
        let window = self.window(len)?;
        let read: Vec<Option<Sym>> = window.iter().map(|s| s.sym).collect();
        let (syms, filled) = match read.iter().copied().collect::<Option<Vec<Sym>>>() {
            Some(syms) => (syms, false),
            // A second could not be read: worked out from the rest where the
            // code's checks allow, and otherwise there is nothing to check.
            None => {
                let margin: Vec<f32> = window.iter().map(|s| s.margin).collect();
                ((self.fmt.fill?)(&read, &margin)?, true)
            }
        };
        let last = window[len - 1].pos;
        match (self.fmt.decode)(&syms, &self.ctx) {
            Ok(frame) => {
                let pos = if self.fmt.time_index < len {
                    window[self.fmt.time_index].pos
                } else {
                    last + self.rate
                };
                if filled {
                    let (at, utc) = self.last_frame?;
                    let since = (pos - at) / self.rate;
                    if since > FILL_CONTEXT_S || frame.utc != utc + since.round() as i64 {
                        return None;
                    }
                }
                Some((pos, Ok(frame)))
            }
            // Not a minute: WWVB, WWV and JJY send a marker every ten
            // seconds, and only the one ending a minute frames.
            Err(FrameError::Framing) => None,
            Err(_) if filled => None,
            Err(e) => Some((last, Err(e))),
        }
    }

    /// The minute WWVB's phase code says, if the last sixty seconds are one.
    fn phase_frame(&mut self) -> Option<(f64, Result<TimeFrame, FrameError>)> {
        let window = self.window(60)?;
        let soft: Vec<f32> = window.iter().map(|s| s.phase).collect::<Option<_>>()?;
        let soft: [f32; 60] = soft.try_into().ok()?;
        match wwvb_phase::decode(&soft) {
            Ok((frame, reading)) => {
                self.polarity = Some(reading.polarity);
                Some((window[0].pos, Ok(frame)))
            }
            // No sync: not where a minute starts, or an extended symbol.
            Err(FrameError::Framing) => None,
            Err(e) => Some((window[59].pos, Err(e))),
        }
    }

    /// One minute's verdict from whatever codes read it, and from the
    /// integrator.
    fn report(
        &mut self,
        amplitude: Option<(f64, Result<TimeFrame, FrameError>)>,
        phase: Option<(f64, Result<TimeFrame, FrameError>)>,
        integrated: Option<(f64, TimeFrame)>,
        events: &mut Vec<Event>,
    ) {
        let found = match (amplitude, phase) {
            (Some((pos, Ok(a))), Some((_, Ok(p)))) if a.utc == p.utc => {
                // The amplitude frame carries DUT1, which the phase code does
                // not; the phase frame says nothing the other contradicts.
                Ok((pos, a, TimeCode::Both))
            }
            (Some((pos, Ok(_))), Some((_, Ok(_)))) => {
                Err((pos, FrameError::Range("the amplitude and phase codes disagree")))
            }
            (Some((pos, Ok(a))), _) => Ok((pos, a, TimeCode::Amplitude)),
            // The phase code is protected by its Hamming code and a sync word;
            // where it reads and the amplitude code does not, it is believed.
            (_, Some((pos, Ok(p)))) => Ok((pos, p, TimeCode::Phase)),
            (Some((pos, Err(e))), _) | (None, Some((pos, Err(e)))) => Err((pos, e)),
            (None, None) => match &integrated {
                Some((pos, frame)) => Ok((*pos, frame.clone(), TimeCode::Amplitude)),
                None => return,
            },
        };
        let found = match (found, integrated) {
            // Read in full and integrated, to different minutes: one of them
            // is wrong, and nothing says which.
            (Ok((pos, read, _)), Some((_, i))) if read.utc != i.utc => {
                Err((pos, FrameError::Range("the minute read and the minutes integrated disagree")))
            }
            // A minute that did not read is still named by the minutes
            // around it. Its flags are the encoder's, from the rules — a
            // leap second's announcement is the one thing it cannot know.
            (Err(_), Some((pos, frame))) => Ok((pos, frame, TimeCode::Amplitude)),
            (found, _) => found,
        };
        match found {
            Ok((pos, frame, code)) => {
                self.ctx.last_utc = Some(frame.utc);
                self.last_frame = Some((pos, frame.utc));
                events.push(Event::Frame { pos, frame, code });
            }
            Err((pos, error)) => events.push(Event::BadFrame { pos, error }),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let level = match &self.fe {
            // A cosine of amplitude A mixes to A/2.
            Fe::Lf(f) if matches!(self.timing, Timing::Element) => {
                2.0 * f.carrier_level().max(1e-9)
            }
            Fe::Lf(_) => 2.0 * self.hi.max(1e-9),
            Fe::Hf(f) => 2.0 * f.level().max(1e-9),
        };
        let scope = match self.second_phase {
            Some(p) => {
                // ALS162's element starts before its second.
                let lead = if matches!(self.timing, Timing::Element) { 0.1 } else { 0.0 };
                let start = ((p - lead).rem_euclid(1.0) * NB as f64) as usize;
                let v = &self.level_fold.v;
                let pts: Vec<f32> = (0..100)
                    .map(|i| (0..10).map(|j| v[(start + i * 10 + j) % NB]).sum::<f32>() / 10.0)
                    .collect();
                let (lo, hi) =
                    pts.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
                let span = (hi - lo).max(f32::MIN_POSITIVE);
                pts.iter().map(|v| ((v - lo) / span * 255.0).round() as u8).collect()
            }
            None => Vec::new(),
        };
        Snapshot {
            carrier_hz: self.fe.carrier_hz().map(|f| f as f32),
            level_db: 20.0 * level.log10(),
            snr_db: if self.locked { -10.0 * self.misfit.max(1e-6).log10() } else { 0.0 },
            locked: self.locked,
            scope,
            phase_snr_db: (self.phase_seconds >= PHASE_WARMUP)
                .then(|| 20.0 * self.phase_level.max(1e-3).log10()),
            timing_z: self.edge_z,
            sibling_z: self.sibling_z,
            sibling_heard: self.sibling_heard,
        }
    }

    /// How far the fold's sharpest edge stands out of its noise, in standard
    /// deviations — the quantity [`MIN_EDGE_Z`] is a threshold on, for tests
    /// and tuning.
    pub fn edge_z(&self) -> f32 {
        self.edge_z
    }

    /// Whether a timing detector's z-score is one the decoder would lock to.
    pub fn credible(z: f32) -> bool {
        z >= MIN_EDGE_Z
    }
}

/// How much of a typical minute the station's carrier is at full level: the
/// share of slots its encoder's patterns hold high over one minute.
fn full_share(fmt: &Format) -> f64 {
    let sent = (fmt.encode)(1_800_000_000);
    let high: usize = sent
        .iter()
        .filter_map(|&sym| fmt.template(sym))
        .map(|t| t.low.iter().filter(|&&low| !low).count())
        .sum();
    high as f64 / (10 * sent.len()) as f64
}

/// The second boundary nearest `t` for a second starting at fraction `phase`.
fn snap(t: f64, phase: f64) -> f64 {
    (t - phase).round() + phase
}

/// A difference of two fractions of a second, wrapped to ±½.
fn circ(d: f64) -> f64 {
    d - d.round()
}
