//! The station's learned range: where, by bearing and distance from home,
//! confirmed aircraft have been heard — the prior for judging a new fix.
//!
//! A CPR pair decoded across a zone boundary, or a frame with an undetected
//! bit error, puts an aircraft a whole zone away: six degrees of latitude,
//! some 670 km. A jump from a recent fix is caught by speed alone (see
//! `track.rs`). A *first* fix has nothing to be compared with, and neither has
//! one after a minute's silence; for those the question is how likely a real
//! aircraft is to be heard *there*, by this station, at all.
//!
//! That is what this keeps: per ten degrees of bearing, a histogram of the
//! distances of confirmed positions in 10 km bins, out to 1500 km. Old counts
//! fade with a half-life of a week, so a new antenna or amplifier moves the
//! envelope by itself. Only positions of aircraft already confirmed are
//! counted, so an error cannot write itself into the prior it is judged by.
//!
//! The judgement is Bayes in log-odds:
//!
//! ```text
//!   log-odds(real) = ln( (1 − ε) · p_real(d, θ) / (ε · p_error(d)) )
//! ```
//!
//! with ε the share of fixes that are decoding errors, `p_error` uniform over
//! the bins (a zone error lands anywhere from next door to the far side of the
//! next zone), and `p_real` the learned histogram of the sector, smoothed over
//! ±20 km and shrunk towards the whole station's histogram, which is itself
//! shrunk towards a smooth starting radius (a Rayleigh of σ = 130 km with a
//! little of the uniform under it): an unexplored bin is unlikely rather than
//! impossible, a sector with little data borrows the station's envelope, and a
//! fresh station takes a fix out to about 530 km at once.
//!
//! Why: HDZ 2026-11-27: „kannst du nicht ein bayes kriterium was sich selber
//!      teached bauen?“ — instead of a fixed range limit. Measured that day:
//!      42 of 74 339 kept points were zone errors at 600–929 km, on a station
//!      whose real aircraft reach about 360 km.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, Weak};

/// Bearing sectors, ten degrees each.
pub const SECTORS: usize = 36;
/// Width of a distance bin, km.
pub const BIN_KM: f64 = 10.0;
/// Bins per sector: out to 1500 km; anything further counts in the last.
pub const BINS: usize = 150;

/// Share of fixes that are decoding errors, for the prior odds. Measured on a
/// readsb feed: 42 zone errors among 74 339 kept points (6·10⁻⁴); rounded up,
/// because the kept points are thinned and the errors are not.
pub const EPS: f64 = 1e-3;

/// Pseudo-count of a sector's shrinkage towards the station: a sector with
/// little data stays close to the station.
const SMOOTH: f64 = 10.0;

/// Pseudo-count of the station's shrinkage towards [`base`]: what a fresh
/// station starts from, worth a hundred confirmed positions, so a few minutes
/// of traffic outweigh it.
const BASE_WEIGHT: f64 = 100.0;

/// Scale of the starting range, km: distances distributed as a Rayleigh with
/// this σ — rising with the area of the ring, falling as a Gaussian with
/// distance — put 99 % within 3σ ≈ 400 km and the mode at 130 km.
/// Why: HDZ 2026-11-27: „vorbefüllen vielleicht nicht, aber einen gauss
///      radius ohne starke struktur“. Seeding from the thinned track files was
///      tried and counted 26 zone errors beyond 450 km as consistent; a smooth
///      radius with no structure lets the station learn its own.
const BASE_SIGMA_KM: f64 = 130.0;

/// Share of the starting range spread evenly over all bins, so that nothing
/// is impossible, only unlikely: a real aircraft far out is held for one fix,
/// not for ever.
const BASE_FLOOR: f64 = 1e-3;

/// The starting range of a bin, before anything is learned: the Rayleigh of
/// [`BASE_SIGMA_KM`] at the bin's centre, mixed with [`BASE_FLOOR`] of the
/// uniform. Sums to one over the bins. A fresh station takes a fix out to
/// about 530 km at once and holds one beyond that for a second fix.
fn base(k: usize) -> f64 {
    let ray = |k: usize| {
        let d = (k as f64 + 0.5) * BIN_KM;
        let s2 = BASE_SIGMA_KM * BASE_SIGMA_KM;
        d / s2 * (-d * d / (2.0 * s2)).exp()
    };
    let total: f64 = (0..BINS).map(ray).sum();
    (1.0 - BASE_FLOOR) * ray(k) / total + BASE_FLOOR / BINS as f64
}

/// Bins either side a learned count is spread over: ±20 km, so the edge of
/// the envelope is a slope, not a cliff one bin wide.
const KERNEL: usize = 2;

/// Half-life of a learned count, seconds: a week.
const HALF_LIFE_S: f64 = 7.0 * 86_400.0;

/// The least the prior may say for a fix, log-odds: e⁻¹¹·⁵ ≈ 1 : 100 000.
///
/// With every confirmed position the far bins of a well-learned sector grow
/// less likely, and without a floor a real aircraft out there — ducting, a new
/// antenna — would need more confirmations the longer the station has run.
/// Here two following fixes (2 × ln 1000 = 13.8) always lift it past 9 : 1: it
/// is shown at its third fix, about a second after the first.
pub const LOG_ODDS_MIN: f64 = -11.5;

/// How far home may move before what was learned no longer applies, km.
const HOME_MOVED_KM: f64 = 20.0;

/// The learned range of one station.
#[derive(Debug, Clone, PartialEq)]
pub struct RangePrior {
    /// Where the counts were learned from.
    home: Option<(f64, f64)>,
    /// Counts, `SECTORS × BINS`, sector-major.
    w: Vec<f32>,
    /// Unix seconds the counts were last faded to.
    faded_at: i64,
}

impl RangePrior {
    pub fn new(now: i64) -> RangePrior {
        RangePrior { home: None, w: vec![0.0; SECTORS * BINS], faded_at: now }
    }

    /// Point the prior at `home`. If it was learned somewhere else, it starts
    /// again: another place has another horizon.
    pub fn set_home(&mut self, home: Option<(f64, f64)>) {
        let Some(h) = home else { return };
        match self.home {
            Some(old) if sdroxide_types::distance_km(old, h) <= HOME_MOVED_KM => {}
            Some(_) => {
                self.w.iter_mut().for_each(|v| *v = 0.0);
                self.home = Some(h);
            }
            None => self.home = Some(h),
        }
    }

    /// Sector and bin of a position, against home.
    fn cell(&self, lat: f64, lon: f64) -> Option<(usize, usize)> {
        let h = self.home?;
        let km = sdroxide_types::distance_km(h, (lat, lon));
        let brg = sdroxide_types::bearing_deg(h, (lat, lon)).rem_euclid(360.0);
        let s = ((brg / (360.0 / SECTORS as f64)) as usize).min(SECTORS - 1);
        let k = ((km / BIN_KM) as usize).min(BINS - 1);
        Some((s, k))
    }

    /// Count a confirmed position.
    pub fn learn(&mut self, lat: f64, lon: f64, now: i64) {
        self.fade(now);
        if let Some((s, k)) = self.cell(lat, lon) {
            self.w[s * BINS + k] += 1.0;
        }
    }

    /// Let the counts fade to `now`, an hour at a time at most often.
    fn fade(&mut self, now: i64) {
        let dt = now - self.faded_at;
        if dt < 3600 {
            return;
        }
        let f = 0.5f64.powf(dt as f64 / HALF_LIFE_S) as f32;
        self.w.iter_mut().for_each(|v| *v *= f);
        self.faded_at = now;
    }

    /// The smoothed count around bin `k` of one row: the mean over ±[`KERNEL`]
    /// bins, so the edge of the envelope is a slope rather than a cliff.
    fn near(row: &[f64], k: usize) -> f64 {
        let (lo, hi) = (k.saturating_sub(KERNEL), (k + KERNEL).min(BINS - 1));
        row[lo..=hi].iter().sum::<f64>() / (2 * KERNEL + 1) as f64
    }

    /// How likely a real aircraft is in this position's bin, as this station
    /// has learned it. Two levels: the station as a whole, all sectors pooled,
    /// shrunk towards the smooth starting range [`base`]; and the sector,
    /// shrunk towards the station. A sector with little data — over the sea,
    /// behind a hill — so takes the station's envelope rather than a blank,
    /// and a fresh station starts from the smooth radius.
    fn p_real(&self, s: usize, k: usize) -> f64 {
        let row = |s: usize| -> Vec<f64> {
            self.w[s * BINS..(s + 1) * BINS].iter().map(|&v| f64::from(v)).collect()
        };
        let mut all = vec![0.0f64; BINS];
        for t in 0..SECTORS {
            for (a, v) in all.iter_mut().zip(row(t)) {
                *a += v;
            }
        }
        let n_all: f64 = all.iter().sum();
        let station = (Self::near(&all, k) + BASE_WEIGHT * base(k)) / (n_all + BASE_WEIGHT);
        let mine = row(s);
        let n: f64 = mine.iter().sum();
        (Self::near(&mine, k) + SMOOTH * station) / (n + SMOOTH)
    }

    /// The log-odds that a fix at `lat`/`lon` is a real aircraft rather than a
    /// decoding error, before anything else is known about it. Without a home
    /// there is nothing to judge by, and the answer is the uniform one.
    pub fn log_odds(&self, lat: f64, lon: f64) -> f64 {
        let p_err = 1.0 / BINS as f64;
        let p_real = match self.cell(lat, lon) {
            Some((s, k)) => self.p_real(s, k),
            None => p_err,
        };
        ((1.0 - EPS) * p_real / (EPS * p_err)).ln().max(LOG_ODDS_MIN)
    }

    /// The learned counts of a sector, for a display or a test.
    pub fn sector(&self, s: usize) -> &[f32] {
        &self.w[s * BINS..(s + 1) * BINS]
    }

    /// As text: a header line, then one line of counts per sector.
    pub fn to_text(&self) -> String {
        let (la, lo) = self.home.unwrap_or((f64::NAN, f64::NAN));
        let mut s = format!("sdroxide-adsb-prior 1 {la:.5} {lo:.5} {}\n", self.faded_at);
        for row in self.w.chunks(BINS) {
            let line: Vec<String> = row
                .iter()
                .map(|v| if *v == 0.0 { "0".into() } else { format!("{v:.2}") })
                .collect();
            s.push_str(&line.join(" "));
            s.push('\n');
        }
        s
    }

    /// Read what [`Self::to_text`] wrote; `None` for anything else.
    pub fn from_text(text: &str) -> Option<RangePrior> {
        let mut lines = text.lines();
        let head: Vec<&str> = lines.next()?.split_whitespace().collect();
        if head.len() != 5 || head[0] != "sdroxide-adsb-prior" || head[1] != "1" {
            return None;
        }
        let (la, lo): (f64, f64) = (head[2].parse().ok()?, head[3].parse().ok()?);
        let home = (la.is_finite() && lo.is_finite()).then_some((la, lo));
        let faded_at = head[4].parse().ok()?;
        let mut w = Vec::with_capacity(SECTORS * BINS);
        for _ in 0..SECTORS {
            let row: Vec<f32> =
                lines.next()?.split_whitespace().map(|v| v.parse().ok()).collect::<Option<_>>()?;
            if row.len() != BINS {
                return None;
            }
            w.extend(row);
        }
        Some(RangePrior { home, w, faded_at })
    }
}

/// A prior shared by every decoder in the process that keeps the same file:
/// four radios on one Beast feed learn one envelope, not four that overwrite
/// each other's file.
pub type SharedPrior = Arc<Mutex<RangePrior>>;

static OPEN: LazyLock<Mutex<HashMap<PathBuf, Weak<Mutex<RangePrior>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The prior kept in `path`: the one already open in this process, else what
/// the file holds, else a fresh one.
pub fn open(path: &Path, now: i64) -> SharedPrior {
    let mut open = OPEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(p) = open.get(path).and_then(Weak::upgrade) {
        return p;
    }
    let prior = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| RangePrior::from_text(&t))
        .unwrap_or_else(|| RangePrior::new(now));
    let p = Arc::new(Mutex::new(prior));
    open.insert(path.to_path_buf(), Arc::downgrade(&p));
    p
}

/// Write the prior to `path`, through a temporary file, so a crash halfway
/// leaves the old one rather than half a new one.
pub fn save(prior: &RangePrior, path: &Path) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, prior.to_text())?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: (f64, f64) = (51.49792, 7.40417);

    /// A station that has heard aircraft westwards out to about 300 km.
    pub(crate) fn learned_west(now: i64) -> RangePrior {
        let mut p = RangePrior::new(now);
        p.set_home(Some(HOME));
        for i in 0..3000 {
            // Along the parallel westward, 5 to 300 km, evenly.
            let km = 5.0 + 295.0 * (i as f64 / 3000.0);
            let lon = HOME.1 - km / (111.32 * HOME.0.to_radians().cos());
            p.learn(HOME.0, lon, now);
        }
        p
    }

    #[test]
    fn a_fresh_station_starts_from_a_smooth_radius() {
        let mut p = RangePrior::new(0);
        p.set_home(Some(HOME));
        let east = |km: f64| HOME.1 + km / (111.32 * HOME.0.to_radians().cos());
        let accept = (9.0f64).ln();
        for km in [20.0, 150.0, 300.0, 450.0, 500.0] {
            assert!(p.log_odds(HOME.0, east(km)) > accept, "{km} km held on a fresh station");
        }
        for km in [600.0, 929.0, 1400.0] {
            assert!(p.log_odds(HOME.0, east(km)) < accept, "{km} km taken on a fresh station");
        }
        // KLM49L's zone error, from day one.
        assert!(p.log_odds(57.23, -3.06) < accept);
    }

    #[test]
    fn the_starting_range_sums_to_one() {
        let s: f64 = (0..BINS).map(base).sum();
        assert!((s - 1.0).abs() < 1e-9, "{s}");
    }

    #[test]
    fn inside_the_learned_range_a_fix_is_believed_at_once() {
        let p = learned_west(0);
        // 150 km west.
        let lon = HOME.1 - 150.0 / (111.32 * HOME.0.to_radians().cos());
        assert!(p.log_odds(HOME.0, lon) > 7.0, "{}", p.log_odds(HOME.0, lon));
    }

    #[test]
    fn far_beyond_the_learned_range_a_fix_is_doubted() {
        let p = learned_west(0);
        // KLM49L's zone error: 929 km north-west.
        let lo = p.log_odds(57.23, -3.06);
        assert!(lo < (9.0f64).ln(), "{lo}");
        // 600 km due west, in the learned sector but twice as far.
        let lon = HOME.1 - 600.0 / (111.32 * HOME.0.to_radians().cos());
        assert!(p.log_odds(HOME.0, lon) < (9.0f64).ln());
    }

    /// A sector nothing has been heard in borrows the station's envelope: near
    /// is likely, far is not.
    #[test]
    fn an_empty_sector_borrows_the_stations_range() {
        let p = learned_west(0);
        // Due east, a sector with no counts at all.
        let east = |km: f64| HOME.1 + km / (111.32 * HOME.0.to_radians().cos());
        assert!(p.log_odds(HOME.0, east(100.0)) > 5.0, "{}", p.log_odds(HOME.0, east(100.0)));
        assert!(p.log_odds(HOME.0, east(800.0)) < (9.0f64).ln());
    }

    #[test]
    fn counts_fade_with_a_half_life_of_a_week() {
        let mut p = learned_west(0);
        let before: f32 = p.sector(27).iter().sum();
        p.fade(HALF_LIFE_S as i64);
        let after: f32 = p.sector(27).iter().sum();
        assert!((after / before - 0.5).abs() < 1e-3, "{before} → {after}");
    }

    #[test]
    fn a_moved_home_starts_again() {
        let mut p = learned_west(0);
        p.set_home(Some((HOME.0 + 0.05, HOME.1))); // 5.6 km: same place
        assert!(p.sector(27).iter().sum::<f32>() > 0.0);
        p.set_home(Some((48.14, 11.58))); // Munich
        assert_eq!(p.sector(27).iter().sum::<f32>(), 0.0);
    }

    #[test]
    fn the_text_form_reads_back() {
        let p = learned_west(1_794_000_000);
        let q = RangePrior::from_text(&p.to_text()).unwrap();
        assert_eq!(q.home.map(|h| (h.0 * 1e5).round()), p.home.map(|h| (h.0 * 1e5).round()));
        assert_eq!(q.faded_at, p.faded_at);
        for (a, b) in p.w.iter().zip(&q.w) {
            assert!((a - b).abs() < 0.01);
        }
        assert!(RangePrior::from_text("something else").is_none());
    }

    #[test]
    fn one_file_is_one_prior_in_the_process() {
        let dir = std::env::temp_dir().join(format!("sdroxide-prior-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("adsb-prior.txt");
        let a = open(&path, 0);
        let b = open(&path, 0);
        assert!(Arc::ptr_eq(&a, &b));
        a.lock().unwrap().set_home(Some(HOME));
        save(&a.lock().unwrap(), &path).unwrap();
        drop((a, b));
        let c = open(&path, 0);
        assert_eq!(c.lock().unwrap().home.map(|h| h.0.round()), Some(51.0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
