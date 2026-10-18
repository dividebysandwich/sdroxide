//! The envelope to seconds, the seconds to symbols, the symbols to minutes.
//!
//! # Finding the second
//!
//! Every second of every code here starts with the same edge — the carrier
//! going down (DCF77, MSF, WWVB) or coming up (JJY) — while what follows it
//! changes from second to second. So the envelope is *folded*: each sample is
//! added into one of a thousand bins by where it falls within its second, and
//! averaged over the last several seconds. The edge that every second shares
//! builds up in the fold; the bits, which differ, average out to steps of
//! their own at 100 ms and beyond; and noise averages towards its mean. The
//! sharpest step of the right sign is where the second begins. A fold works
//! at signal levels where no single edge could be seen, which is the reason
//! for it.
//!
//! Whether that step is real is a question about noise, so it is answered
//! against the fold's own: the scatter between bins too far apart to share a
//! filter's smoothing, which the keying's few steps barely touch. A step many
//! times that scatter is an edge; noise alone does not make one. The edge is
//! found with a wide detector, which sees furthest into the noise, and then
//! placed with a narrow one, which places it most precisely.
//!
//! The fold is over the input's own sample clock, not the computer's: a
//! receiver whose sample rate is fifty parts per million out drifts by a
//! millisecond in twenty seconds, which the fold's memory is short enough to
//! follow.
//!
//! # Reading a second
//!
//! Every length any of these codes keys is a whole number of tenths, so a
//! second is measured as ten 100 ms slots — the middle 80 ms of each, clear
//! of the edges — normalised between the carrier's full and reduced levels,
//! and compared with each pattern the station sends.
//!
//! The two levels start from the fold — its top and bottom are full and
//! reduced carrier in every code here — and then follow the seconds that were
//! read, each slot counted towards the level its pattern says it was at.
//! (The highest and lowest slot of each second would need no read to have
//! succeeded, but in noise they sit a standard deviation and more outside the
//! levels they stand for.) A run of seconds that will not read reseeds them
//! from the fold, so a bad start cannot hold the reader shut for good. The closest wins, if it
//! wins clearly; otherwise the second is left unread, which costs that minute
//! and protects every minute after it.
//!
//! # Reading a minute
//!
//! When a marker arrives the last minute's worth of symbols is handed to the
//! station's [`Format`], which decides whether it is a minute and what it
//! says. The instant it names is tied to the input sample its second began
//! on — and that sample, not the computer's clock, is what leaves here.

use std::collections::VecDeque;

use sdroxide_types::{TIMESIG_TONE_HZ, TimeFrame, TimeStation};

use crate::format::{Context, Format, FrameError, Sym};
use crate::frontend::{EnvSample, FrontEnd};

/// Bins in the fold: one a millisecond.
const NB: usize = 1000;
/// How quickly the fold forgets: each bin moves this far towards each new
/// sample, which is a memory of about sixteen seconds — a sample clock fifty
/// parts per million out moves the edge less than a millisecond in that.
const FOLD_ALPHA: f32 = 1.0 / 16.0;
/// The wide edge detector's reach either side of a candidate edge, in bins.
/// 80 ms is inside the part of the second every symbol of every code here
/// shares: full carrier for the last 100 ms before a falling edge and reduced
/// for the first 100 ms after it (JJY the other way about).
const EDGE_WIDE: usize = 80;
/// The narrow detector that places the edge, and how far either side of the
/// wide one's answer it looks.
const EDGE_NARROW: usize = 10;
const EDGE_REFINE: usize = 20;
/// How many bins apart two fold bins must be for the envelope smoothing not
/// to correlate them (it spans five).
const NOISE_LAG: usize = 8;
/// Fold bins the smoothing makes one independent sample out of.
const CORR_BINS: f64 = 5.0;
/// How many standard deviations of noise the wide detector's step has to
/// stand at to be an edge. Its largest excursion on noise alone, over the
/// fold's couple of hundred independent positions, is about three; and a
/// lock on noise would still have to read a minute that passes its checks,
/// twice, before anything believed it.
const MIN_EDGE_Z: f32 = 6.0;
/// Seconds of envelope folded before the fold is asked anything.
const WARMUP_S: f64 = 6.0;
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
/// How fast the full and reduced carrier levels follow the signal.
const LEVEL_BETA: f32 = 0.15;
/// Consecutive unread seconds after which the levels are reseeded from the
/// fold.
const RESEED_AFTER: u32 = 8;
/// Symbols kept: two minutes and a little over.
const SYMS_KEPT: usize = 130;

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
    Frame { pos: f64, frame: TimeFrame },
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
    /// The fold, rotated to start at the second's start, 100 points, 0–255.
    pub scope: Vec<u8>,
}

pub struct Decoder {
    fmt: &'static Format,
    rate: f64,
    fe: FrontEnd,
    scratch: Vec<EnvSample>,
    env: VecDeque<EnvSample>,
    fold: Vec<f32>,
    fold_seen: Vec<bool>,
    /// The time, seconds of input, the fold started and was last examined.
    fold_start: Option<f64>,
    examined: f64,
    /// Where each second starts, as a fraction of a second of input time.
    phase: Option<f64>,
    agree: u32,
    disagree: u32,
    edge_z: f32,
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
}

impl Decoder {
    pub fn new(station: TimeStation, rate: f64) -> Self {
        Decoder {
            fmt: Format::for_station(station),
            rate,
            // WWVB keys its carrier's phase as well: see the front end.
            fe: FrontEnd::new(rate, f64::from(TIMESIG_TONE_HZ), station != TimeStation::Wwvb),
            scratch: Vec::new(),
            env: VecDeque::new(),
            fold: vec![0.0; NB],
            fold_seen: vec![false; NB],
            fold_start: None,
            examined: 0.0,
            phase: None,
            agree: 0,
            disagree: 0,
            edge_z: 0.0,
            locked: false,
            fe_moves: 0,
            next: None,
            hi: 0.0,
            lo: 0.0,
            misfit: 1.0,
            unread: 0,
            syms: VecDeque::new(),
            ctx: Context::default(),
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
        if self.fe.moves() != self.fe_moves {
            // Everything folded so far was measured off the carrier.
            self.fe_moves = self.fe.moves();
            self.restart(events);
        }
        for &e in &scratch {
            self.envelope(e, events);
        }
        self.scratch = scratch;
    }

    fn envelope(&mut self, e: EnvSample, events: &mut Vec<Event>) {
        let t = e.pos / self.rate;
        let bin = ((t.rem_euclid(1.0) * NB as f64) as usize).min(NB - 1);
        if self.fold_seen[bin] {
            self.fold[bin] += FOLD_ALPHA * (e.mag - self.fold[bin]);
        } else {
            self.fold[bin] = e.mag;
            self.fold_seen[bin] = true;
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
            self.next = self.phase.map(|p| snap(start + 1.0, p));
        }
    }

    /// Starts again from nothing: an empty fold, no lock, no symbols.
    fn restart(&mut self, events: &mut Vec<Event>) {
        if self.locked {
            events.push(Event::LockLost);
        }
        self.fold_seen.iter_mut().for_each(|b| *b = false);
        self.fold_start = None;
        self.phase = None;
        self.agree = 0;
        self.disagree = 0;
        self.locked = false;
        self.next = None;
        self.syms.clear();
    }

    /// Looks for the second's start in the fold.
    fn examine(&mut self, now: f64, events: &mut Vec<Event>) {
        let (edge, z) = self.find_edge();
        self.edge_z = z;
        let credible = z >= MIN_EDGE_Z;
        match self.phase {
            Some(p) if credible && circ(edge - p).abs() < SAME_EDGE_S => {
                // Follow it slowly: the estimate is noisy, the clock it is
                // tracking is not.
                self.phase = Some((p + 0.25 * circ(edge - p)).rem_euclid(1.0));
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
                    self.phase = credible.then_some(edge);
                }
            }
        }
        if !self.locked && self.agree >= LOCK_AFTER {
            let p = self.phase.expect("agreeing estimates imply a phase");
            self.locked = true;
            self.disagree = 0;
            self.seed_levels();
            // The first whole second still in the history.
            let oldest = self.env.front().map_or(now, |e| e.pos / self.rate);
            let mut first = snap(oldest + SLOT_GUARD_S, p);
            if first < oldest + SLOT_GUARD_S {
                first += 1.0;
            }
            self.next = Some(first);
            events.push(Event::Locked);
        }
    }

    /// The fold's sharpest step of the station's sign: where, as a fraction of
    /// a second, and how many standard deviations of the fold's noise it is.
    fn find_edge(&self) -> (f64, f32) {
        // Circular prefix sums so a window may run across the end.
        let mut pre = vec![0.0f64; 2 * NB + 1];
        for i in 0..2 * NB {
            pre[i + 1] = pre[i] + f64::from(self.fold[i % NB]);
        }
        let mean = |from: usize, len: usize| (pre[from + len] - pre[from]) / len as f64;
        // Bins b−w … b−1 against b … b+w−1; the first window is read a lap
        // later so it never starts below zero.
        let step = |b: usize, w: usize| {
            let b = b % NB;
            let before = mean(b + NB - w, w);
            let after = mean(b, w);
            if self.fmt.rising_edge { after - before } else { before - after }
        };
        let (wide_at, wide) = (0..NB)
            .map(|b| (b, step(b, EDGE_WIDE)))
            .fold((0, f64::MIN), |best, x| if x.1 > best.1 { x } else { best });
        // The noise, from differences between bins the smoothing does not
        // join: a robust spread, so the keying's few steps do not count.
        let mut d: Vec<f64> = (0..NB)
            .map(|i| f64::from(self.fold[i] - self.fold[(i + NB - NOISE_LAG) % NB]).abs())
            .collect();
        d.sort_by(f64::total_cmp);
        let sigma_bin = 1.4826 * d[NB / 2] / std::f64::consts::SQRT_2;
        let sigma_step = sigma_bin * (2.0 * CORR_BINS / EDGE_WIDE as f64).sqrt();
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

    /// Starting levels for the carrier, from the fold: its top and bottom
    /// twentieths are full carrier and reduced carrier in every code here.
    fn seed_levels(&mut self) {
        let mut v = self.fold.clone();
        v.sort_by(f32::total_cmp);
        let n = NB / 20;
        self.lo = v[..n].iter().sum::<f32>() / n as f32;
        self.hi = v[NB - n..].iter().sum::<f32>() / n as f32;
    }

    fn read_second(&mut self, start: f64, events: &mut Vec<Event>) {
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
        let read = if n.iter().all(|&c| c > 0) && self.hi > self.lo {
            let span = self.hi - self.lo;
            let y: Vec<f32> = (0..10).map(|k| (sum[k] / n[k] as f32 - self.lo) / span).collect();
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
                    (c, i)
                })
                .collect();
            costs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (best, idx) = costs[0];
            let margin = costs.get(1).map_or(f32::MAX, |c| c.0) - best;
            let t = &self.fmt.templates[idx];
            if margin >= MIN_MARGIN && best <= MAX_COST {
                // The slots the winning pattern says were at full carrier say
                // what full carrier is now, and likewise the reduced ones.
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
                Some((t.sym, margin))
            } else {
                None
            }
        } else {
            None
        };
        self.unread = if read.is_some() { 0 } else { self.unread + 1 };
        if self.unread >= RESEED_AFTER {
            self.unread = 0;
            self.seed_levels();
        }
        let second = Second {
            pos: start * self.rate,
            sym: read.map(|(s, _)| s),
            margin: read.map_or(0.0, |(_, m)| m),
        };
        events.push(Event::Second(second));
        if self.syms.len() == SYMS_KEPT {
            self.syms.pop_front();
        }
        self.syms.push_back(second);
        if second.sym.is_some_and(|s| s.marker) {
            self.try_frame(events);
        }
    }

    fn try_frame(&mut self, events: &mut Vec<Event>) {
        let len = self.fmt.frame_len;
        if self.syms.len() < len {
            return;
        }
        let window: Vec<Second> = self.syms.iter().skip(self.syms.len() - len).copied().collect();
        // One second apart each, or the window is not a minute of air.
        let contiguous = window.windows(2).all(|p| {
            let gap = (p[1].pos - p[0].pos) / self.rate;
            (gap - 1.0).abs() < 0.01
        });
        if !contiguous {
            return;
        }
        let Some(syms) = window.iter().map(|s| s.sym).collect::<Option<Vec<Sym>>>() else {
            // A second could not be read; nothing to check.
            return;
        };
        let last = window[len - 1].pos;
        match (self.fmt.decode)(&syms, &self.ctx) {
            Ok(frame) => {
                let pos = if self.fmt.time_index < len {
                    window[self.fmt.time_index].pos
                } else {
                    last + self.rate
                };
                self.ctx.last_utc = Some(frame.utc);
                events.push(Event::Frame { pos, frame });
            }
            // Not a minute: WWVB and JJY send a marker every ten seconds, and
            // only the one ending a minute frames. Nothing worth reporting.
            Err(FrameError::Framing) => {}
            Err(error) => events.push(Event::BadFrame { pos: last, error }),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let level = 2.0 * self.hi.max(1e-9);
        let scope = match self.phase {
            Some(p) => {
                let start = (p * NB as f64) as usize;
                let pts: Vec<f32> = (0..100)
                    .map(|i| {
                        (0..10).map(|j| self.fold[(start + i * 10 + j) % NB]).sum::<f32>() / 10.0
                    })
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
        }
    }

    /// How far the fold's sharpest edge stands out of its noise, in standard
    /// deviations — the quantity [`MIN_EDGE_Z`] is a threshold on, for tests
    /// and tuning.
    pub fn edge_z(&self) -> f32 {
        self.edge_z
    }
}

/// The second boundary nearest `t` for a second starting at fraction `phase`.
fn snap(t: f64, phase: f64) -> f64 {
    (t - phase).round() + phase
}

/// A difference of two fractions of a second, wrapped to ±½.
fn circ(d: f64) -> f64 {
    d - d.round()
}
