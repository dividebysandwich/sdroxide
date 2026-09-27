//! SSTV line timing: where each scan line really starts.
//!
//! A receiver that simply walks forward one nominal line period at a time
//! shears the picture by however far the two stations' sound cards disagree,
//! and one that snaps every line to its own sync pulse follows every pulse a
//! fade has smeared or noise has faked. Both are wrong in the same way: the
//! line starts of a real transmission lie on a straight line — the sender's
//! clock is constant for the length of a picture — so the thing to estimate is
//! that line, from all the sync pulses at once.
//!
//! Two estimates, used at two moments:
//!
//! * [`LineFit`] runs while the picture comes in. Every sync pulse that is
//!   clearly there becomes a point; a robust least-squares line through them
//!   (after Open-SSTV's `core/slant.py`, which does the same over a whole
//!   recording) places each new line, so one faded pulse costs nothing and a
//!   false one is outvoted.
//! * [`SyncMap`] runs once the picture is complete, over the frequency track of
//!   the whole of it. It searches every plausible (start, period) pair for the
//!   one along which the most sync-tone energy lies — the line through the
//!   picture's sync column, found the way slowrx finds it with its Hough
//!   transform — so it sees pulses too weak to count one at a time, and early
//!   lines get the benefit of the ones that came after them.

/// Instantaneous frequency below this is sync: midway between the 1200 Hz
/// sync tone and the 1500 Hz black level, the lowest thing a picture sends.
const SYNC_SLICE_HZ: f64 = 1350.0;

/// A line start's measured sync pulse has to be at least this full of sync
/// tone to be believed.
pub(super) const MIN_FILL: f64 = 0.6;

/// The fastest a sender's clock can run against the receiver's and still be
/// looked for, as a fraction of the line period: ±1 % is 10 000 ppm, far past
/// any sound card, with room for a mis-set resampler.
const MAX_DRIFT: f64 = 0.01;

/// Fewest points a line is fitted through, and the fewest line indices they
/// must span: two points always make a line, so it takes more than that
/// before one means anything.
const MIN_POINTS: usize = 4;
const MIN_SPAN: f64 = 3.0;

/// How many points, spread across the picture, are paired up to propose
/// candidate lines. Enough that a majority survives a long fade; few enough
/// that doing it every line costs nothing.
const SEED_POINTS: usize = 16;

/// Whether a frequency is sync tone.
fn sync_like(hz: f64) -> bool {
    hz < SYNC_SLICE_HZ
}

/// The line `start(k) = a + b·k`: line `k` starts at sample `a + b·k`, and `b`
/// is the real line period in receiver samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Line {
    pub a: f64,
    pub b: f64,
}

impl Line {
    pub fn at(&self, k: f64) -> f64 {
        self.a + self.b * k
    }
}

/// Find the sync pulse nearest `centre`: the position, within `half_win`
/// samples either side of it, where a window `len` samples long holds the most
/// sync tone. Returns the pulse's centre and how full of sync tone it is.
///
/// A box rather than a centroid of every low sample in the window, which is
/// what this receiver used to do: in noise the centroid is dragged towards
/// every stray sample below the slice, and the box is not.
pub(super) fn measure_sync(
    hz: impl Fn(i64) -> f64,
    centre: f64,
    half_win: f64,
    len: f64,
) -> Option<(f64, f64)> {
    let l = len.round().max(1.0) as usize;
    let lo = (centre - half_win - len / 2.0).floor() as i64;
    let hi = (centre + half_win + len / 2.0).ceil() as i64;
    let n = (hi - lo).max(0) as usize;
    if n < l {
        return None;
    }
    let mut pre = Vec::with_capacity(n + 1);
    pre.push(0u32);
    for i in 0..n {
        let s = sync_like(hz(lo + i as i64)) as u32;
        pre.push(pre[i] + s);
    }
    let count = |s: usize| pre[s + l] - pre[s];
    let starts = n - l + 1;
    let best = (0..starts).map(count).max()?;
    if best == 0 {
        return None;
    }
    // The first run of window positions that all reach the best count: a
    // clean pulse gives a run of one, a smeared one a plateau whose middle is
    // the pulse's middle.
    let first = (0..starts).find(|&s| count(s) == best)?;
    let last = (first..starts).take_while(|&s| count(s) == best).last()?;
    let mid = (first + last) as f64 / 2.0;
    Some((lo as f64 + mid + l as f64 / 2.0, best as f64 / l as f64))
}

/// Least squares through `pts`, centred for precision (the positions are
/// sample indices into a stream that may have run for hours).
fn least_squares(pts: &[(f64, f64)]) -> Option<Line> {
    let n = pts.len() as f64;
    if pts.len() < 2 {
        return None;
    }
    let mk = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let mm = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for &(k, m) in pts {
        sxx += (k - mk) * (k - mk);
        sxy += (k - mk) * (m - mm);
    }
    if sxx <= 0.0 {
        return None;
    }
    let b = sxy / sxx;
    Some(Line { a: mm - b * mk, b })
}

/// The points within `tol` of `line`, and their total distance from it.
fn inliers(pts: &[(f64, f64)], line: Line, tol: f64) -> (Vec<(f64, f64)>, f64) {
    let mut kept = Vec::new();
    let mut err = 0.0;
    for &p in pts {
        let r = (p.1 - line.at(p.0)).abs();
        if r <= tol {
            kept.push(p);
            err += r;
        }
    }
    (kept, err)
}

/// A line through `pts` that ignores the ones that are not on it.
///
/// Candidate lines come from `seed` (the previous answer) and from every pair
/// among a handful of points spread across the list; the one most points agree
/// with, to within `tol`, is refitted by least squares over just those points,
/// twice. `None` until enough points agree, spread over enough lines, on a
/// period within [`MAX_DRIFT`] of `nominal`.
pub(super) fn robust_line(
    pts: &[(f64, f64)],
    nominal: f64,
    tol: f64,
    seed: Option<Line>,
) -> Option<Line> {
    if pts.len() < MIN_POINTS {
        return None;
    }
    let plausible = |l: &Line| (l.b / nominal - 1.0).abs() <= 2.0 * MAX_DRIFT;
    let mut cands: Vec<Line> = seed.into_iter().collect();
    let m = pts.len().min(SEED_POINTS);
    let pick: Vec<(f64, f64)> = (0..m).map(|i| pts[i * (pts.len() - 1) / (m - 1).max(1)]).collect();
    for i in 0..pick.len() {
        for j in i + 1..pick.len() {
            let (p, q) = (pick[i], pick[j]);
            if q.0 == p.0 {
                continue;
            }
            let b = (q.1 - p.1) / (q.0 - p.0);
            let l = Line { a: p.1 - b * p.0, b };
            if plausible(&l) {
                cands.push(l);
            }
        }
    }
    let mut best: Option<(usize, f64, Line)> = None;
    for l in cands {
        let (kept, err) = inliers(pts, l, tol);
        let better = match best {
            None => true,
            Some((n, e, _)) => kept.len() > n || (kept.len() == n && err < e),
        };
        if better {
            best = Some((kept.len(), err, l));
        }
    }
    let mut line = best?.2;
    let mut kept = Vec::new();
    for _ in 0..2 {
        kept = inliers(pts, line, tol).0;
        line = least_squares(&kept)?;
    }
    let span = kept.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max)
        - kept.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    (kept.len() >= MIN_POINTS && span >= MIN_SPAN && plausible(&line) && line.b > 0.0)
        .then_some(line)
}

/// The running line-start estimate for the picture being received.
pub(super) struct LineFit {
    nominal: f64,
    /// How far from the line a sync pulse may sit and still be on it.
    pub tol: f64,
    pts: Vec<(f64, f64)>,
    line: Option<Line>,
}

impl LineFit {
    pub fn new(nominal: f64, tol: f64) -> Self {
        LineFit { nominal, tol, pts: Vec::new(), line: None }
    }

    /// Line `k`'s sync put its start at sample `m`.
    pub fn add(&mut self, k: f64, m: f64) {
        self.pts.push((k, m));
        self.line = robust_line(&self.pts, self.nominal, self.tol, self.line);
    }

    pub fn line(&self) -> Option<Line> {
        self.line
    }

    pub fn predict(&self, k: f64) -> Option<f64> {
        self.line.map(|l| l.at(k))
    }
}

/// A picture's frequency track, kept whole so it can be decoded again once
/// every sync pulse in it is known.
///
/// An eighth of a hertz in a `u16`: plenty for a pixel (the whole black-white
/// swing is 800 Hz) at half the memory of an `f32`. PD290, the longest mode,
/// is under five minutes — 28 MB at 48 kHz.
pub(super) struct FreqTrack {
    base: u64,
    q: Vec<u16>,
    cap: usize,
}

impl FreqTrack {
    pub fn new() -> Self {
        FreqTrack { base: 0, q: Vec::new(), cap: 0 }
    }

    /// Start a new track at absolute sample `base`, holding at most `cap`
    /// samples. The allocation of the last one is reused.
    pub fn start(&mut self, base: u64, cap: usize) {
        self.base = base;
        self.q.clear();
        self.cap = cap;
        self.q.reserve(cap.saturating_sub(self.q.capacity()));
    }

    /// Take `delta` Hz off every frequency kept so far.
    pub fn shift(&mut self, delta: f64) {
        let d = (delta * 8.0).round() as i32;
        for v in &mut self.q {
            *v = (*v as i32 - d).clamp(0, u16::MAX as i32) as u16;
        }
    }

    pub fn clear(&mut self) {
        self.q.clear();
        self.cap = 0;
    }

    pub fn push(&mut self, hz: f64) {
        if self.q.len() < self.cap {
            self.q.push((hz * 8.0).round().clamp(0.0, u16::MAX as f64) as u16);
        }
    }

    /// Frequency at absolute sample `idx`; outside the track, the leader tone,
    /// which is neither sync nor picture.
    pub fn hz(&self, idx: i64) -> f64 {
        let i = idx - self.base as i64;
        if i < 0 {
            return 1900.0;
        }
        self.q.get(i as usize).map_or(1900.0, |&v| v as f64 / 8.0)
    }

    pub fn full(&self) -> bool {
        self.q.len() >= self.cap
    }

    fn start_idx(&self) -> i64 {
        self.base as i64
    }

    fn len(&self) -> usize {
        self.q.len()
    }
}

/// Where the sync pulse sits in every line of a mode.
pub(super) struct Geometry {
    /// Transmitted lines in the picture (PD: half its rows).
    pub lines: usize,
    /// Nominal line period, samples.
    pub period: f64,
    /// From a line's start to the middle of its sync pulse, samples.
    pub sync_centre: f64,
    /// Sync pulse length, samples.
    pub sync_len: f64,
}

/// What [`SyncMap::search`] found.
pub(super) struct Found {
    pub line: Line,
    /// Mean fraction of each line's sync window that is sync tone, along it.
    pub fill: f64,
    /// How far that stands above the same measure along lines at other
    /// phases — the typical fill of a picture's content and noise. A line
    /// through a real sync column stands well clear; one through nothing
    /// does not.
    pub contrast: f64,
}

/// A whole picture's sync tone, as a cumulative count, for scoring candidate
/// lines against all of it at once.
pub(super) struct SyncMap {
    /// `cum[j]`: sync-tone samples before block `j`.
    cum: Vec<u32>,
    block: usize,
    origin: i64,
    g: Geometry,
}

impl SyncMap {
    pub fn new(track: &FreqTrack, g: Geometry) -> Self {
        // Blocks of a sixteenth of a sync pulse: fine enough to place one, and
        // small enough in memory next to the track itself.
        let block = ((g.sync_len / 16.0).floor() as usize).max(1);
        let n = track.len();
        let mut cum = Vec::with_capacity(n / block + 2);
        cum.push(0u32);
        let mut acc = 0u32;
        for j in 0..n.div_ceil(block) {
            for i in j * block..((j + 1) * block).min(n) {
                acc += sync_like(track.hz(track.start_idx() + i as i64)) as u32;
            }
            cum.push(acc);
        }
        SyncMap { cum, block, origin: track.start_idx(), g }
    }

    /// Sync-tone samples before absolute sample `x`, interpolated within a
    /// block.
    fn cum_at(&self, x: f64) -> f64 {
        let rel = (x - self.origin as f64) / self.block as f64;
        let last = (self.cum.len() - 1) as f64;
        if rel <= 0.0 {
            return 0.0;
        }
        if rel >= last {
            return self.cum[self.cum.len() - 1] as f64;
        }
        let j = rel.floor() as usize;
        let f = rel - j as f64;
        self.cum[j] as f64 + f * (self.cum[j + 1] as f64 - self.cum[j] as f64)
    }

    /// Mean sync-tone fill of a `len`-sample window on every line's sync
    /// position, if the lines start where `line` says.
    pub fn score(&self, line: Line, len: f64) -> f64 {
        let mut s = 0.0;
        for k in 0..self.g.lines {
            let c = line.at(k as f64) + self.g.sync_centre;
            s += self.cum_at(c + len / 2.0) - self.cum_at(c - len / 2.0);
        }
        s / (self.g.lines.max(1) as f64 * len)
    }

    /// The line along which the picture's sync tone lies.
    ///
    /// Searched over every start phase and every period within [`MAX_DRIFT`]
    /// of the nominal, first with a window four sync pulses wide (so the
    /// period can be stepped coarsely without the far lines falling off it),
    /// then again around the best of those with a pulse-wide one. Each period
    /// step is small enough that the lines at the picture's ends move by a
    /// quarter of a window at most. `around` only sets where the phase search
    /// is centred; every phase is tried.
    pub fn search(&self, around: Line) -> Option<Found> {
        let g = &self.g;
        let k_n = g.lines.max(1) as f64;
        let k_mid = (k_n - 1.0) / 2.0;
        let at_mid = |c: f64, b: f64| Line { a: c - b * k_mid, b };
        let c0 = around.at(k_mid);

        // Coarse: a wide window, every phase.
        let wide = (4.0 * g.sync_len).min(g.period / 4.0);
        let c_step = wide / 4.0;
        let b_step = wide / (2.0 * k_n);
        let b_half = MAX_DRIFT * g.period;
        let c_n = (g.period / 2.0 / c_step).ceil() as i64;
        let b_n = (b_half / b_step).ceil() as i64;
        let mut best: Option<(f64, f64, f64)> = None; // (score, c, b)
        for bi in -b_n..=b_n {
            let b = g.period + bi as f64 * b_step;
            for ci in -c_n..=c_n {
                let c = c0 + ci as f64 * c_step;
                let s = self.score(at_mid(c, b), wide);
                if best.is_none_or(|(bs, _, _)| s > bs) {
                    best = Some((s, c, b));
                }
            }
        }
        let (_, c1, b1) = best?;

        // How the best phase compares with the others at that period.
        let mut others: Vec<f64> = (-c_n..=c_n)
            .map(|ci| self.score(at_mid(c1 + ci as f64 * c_step, b1), g.sync_len))
            .collect();
        others.sort_by(f64::total_cmp);
        let background = others[others.len() / 2];

        // Fine: a pulse-wide window around the coarse answer.
        let len = g.sync_len;
        let c_step = (len / 4.0).max(1.0);
        let b_step2 = len / (2.0 * k_n);
        let c_n = (wide / 4.0 / c_step).ceil() as i64 + 1;
        let b_n = (b_step / b_step2).ceil() as i64 + 1;
        let mut best: Option<(f64, Line)> = None;
        for bi in -b_n..=b_n {
            let b = b1 + bi as f64 * b_step2;
            for ci in -c_n..=c_n {
                let l = at_mid(c1 + ci as f64 * c_step, b);
                let s = self.score(l, len);
                if best.is_none_or(|(bs, _)| s > bs) {
                    best = Some((s, l));
                }
            }
        }
        let (mut fill, mut line) = best?;

        // Finer: the grid above is fine enough to find the sync column, not to
        // place it — a period step there can be tens of ppm, which in a long
        // mode is several pixels by the bottom. Halve both steps around the
        // best point until they are a small fraction of a sample.
        let (mut c_step, mut b_step) = (c_step, b_step2);
        for _ in 0..5 {
            c_step /= 2.0;
            b_step /= 2.0;
            let (c, b) = (line.at(k_mid), line.b);
            for bi in -2..=2 {
                for ci in -2..=2 {
                    let l = at_mid(c + ci as f64 * c_step, b + bi as f64 * b_step);
                    let s = self.score(l, len);
                    if s > fill {
                        (fill, line) = (s, l);
                    }
                }
            }
        }
        Some(Found { line, fill, contrast: fill - background })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_through_points_ignores_the_ones_off_it() {
        // A sender 0.2 % slow, with a fifth of its pulses misplaced by fades.
        let (a, b) = (1000.0, 20_554.56 * 1.002);
        let mut pts = Vec::new();
        for k in 0..60 {
            let mut m = a + b * k as f64;
            if k % 5 == 3 {
                m += if k % 2 == 0 { 400.0 } else { -350.0 };
            }
            pts.push((k as f64, m));
        }
        let l = robust_line(&pts, 20_554.56, 72.0, None).expect("a line");
        assert!((l.b - b).abs() < 1e-6, "period {} against {b}", l.b);
        assert!((l.a - a).abs() < 1e-3, "start {} against {a}", l.a);
    }

    #[test]
    fn two_points_are_not_yet_a_line() {
        assert!(robust_line(&[(0.0, 0.0), (1.0, 100.0)], 100.0, 5.0, None).is_none());
    }

    #[test]
    fn a_pulse_is_found_at_its_middle_despite_stray_low_samples() {
        // A 400-sample pulse centred on 1200, and noise below the slice
        // scattered across the rest of the window.
        let hz = |i: i64| {
            if (1000..1400).contains(&i) {
                1200.0
            } else if i % 37 == 0 {
                1100.0
            } else {
                1700.0
            }
        };
        let (c, fill) = measure_sync(hz, 1100.0, 500.0, 400.0).expect("a pulse");
        assert!((c - 1200.0).abs() <= 1.0, "centre {c}");
        assert!(fill > 0.99);
    }

    /// A sync column in so much noise that no single pulse is clear enough
    /// to believe — fewer than half its samples read as sync — is still found
    /// to within a few ppm when the whole picture is searched at once, even
    /// starting from a guess a sixth of a line out.
    #[test]
    fn the_sync_column_is_found_when_no_single_pulse_is() {
        let (lines, nominal, len) = (256usize, 20_000.0, 432.0);
        let truth = Line { a: 5_000.0, b: nominal * 1.0023 };
        let mut track = FreqTrack::new();
        let total = (truth.at(lines as f64) + nominal) as usize;
        track.start(0, total);
        let mut x = 0x2545_F491_4F6C_DD1Du64;
        let mut coin = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 11) as f64 / (1u64 << 53) as f64
        };
        for i in 0..total {
            let k = ((i as f64 - truth.a) / truth.b).floor();
            let in_sync = k >= 0.0 && (i as f64 - truth.at(k)) < len;
            let p_sync = if in_sync { 0.45 } else { 0.15 };
            track.push(if coin() < p_sync { 1200.0 } else { 1400.0 + 900.0 * coin() });
        }
        // No pulse on its own gets past the bar a pulse has to clear.
        let clear = (0..lines)
            .filter_map(|k| measure_sync(|i| track.hz(i), truth.at(k as f64) + len / 2.0, len, len))
            .filter(|&(_, fill)| fill >= MIN_FILL)
            .count();
        assert!(clear < lines / 10, "{clear} of {lines} pulses were clear on their own");

        let map = SyncMap::new(
            &track,
            Geometry { lines, period: nominal, sync_centre: len / 2.0, sync_len: len },
        );
        let found = map.search(Line { a: truth.a + nominal / 6.0, b: nominal }).expect("a line");
        let ppm = (found.line.b / truth.b - 1.0) * 1e6;
        assert!(ppm.abs() < 5.0, "period out by {ppm:.1} ppm");
        assert!((found.line.at(0.0) - truth.a).abs() < len / 8.0, "start {}", found.line.a);
        assert!(found.contrast > 0.15, "contrast {}", found.contrast);
    }
}
