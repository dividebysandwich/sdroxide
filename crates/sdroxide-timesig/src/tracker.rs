//! When a decoded minute is believed, and what time every second after it is.
//!
//! A frame that passed its parity checks is still only probably right. Single
//! bit errors are caught; two in one parity group are not, and on a weak
//! signal with static they happen. So nothing is believed until consecutive
//! frames agree — each naming a time exactly as far after the last as the
//! samples between them say — and the clock is only ever set from a run of
//! them.
//!
//! Once believed, every second boundary the decoder reports is a measurement:
//! its UTC is the anchor's plus the whole seconds since. That is sixty
//! measurements a minute rather than one, which is what lets a median take
//! the scheduling noise of a desktop computer off the answer.

/// How far apart, in seconds of input, two frames may be and still be checked
/// against each other — six hours, past which a receiver's sample rate could
/// have drifted further than the check can tell apart from an error.
const MAX_GAP_S: f64 = 6.0 * 3600.0;
/// How long after the last agreeing frame the anchor still counts seconds.
/// Holdover across a minute or two of fading, not across an afternoon.
const HOLDOVER_S: f64 = 1800.0;
/// The agreement tolerance: a fixed part for the edge estimate's wander, and a
/// part proportional to the span for the sample clock's error against true
/// time — a hundred parts per million, double what a poor receiver manages.
const TOL_FIXED_S: f64 = 0.05;
const TOL_PPM: f64 = 100e-6;

/// A believed frame: the input sample a known UTC second began on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub pos: f64,
    pub utc: i64,
}

pub struct Tracker {
    rate: f64,
    need: u32,
    run: u32,
    last: Option<Anchor>,
    anchor: Option<Anchor>,
}

impl Tracker {
    /// `need` is how many consecutive agreeing frames make a time believed;
    /// it is held to at least two.
    pub fn new(rate: f64, need: u32) -> Self {
        Tracker { rate, need: need.max(2), run: 0, last: None, anchor: None }
    }

    pub fn set_need(&mut self, need: u32) {
        self.need = need.max(2);
    }

    /// Takes a decoded frame. Returns whether it agreed with the one before.
    pub fn frame(&mut self, pos: f64, utc: i64) -> bool {
        let agrees = self.last.is_some_and(|l| {
            let dt = (pos - l.pos) / self.rate;
            let du = (utc - l.utc) as f64;
            du > 0.0 && dt <= MAX_GAP_S && (dt - du).abs() < TOL_FIXED_S + TOL_PPM * dt
        });
        self.run = if agrees { self.run + 1 } else { 1 };
        let a = Anchor { pos, utc };
        self.last = Some(a);
        if self.run >= self.need {
            self.anchor = Some(a);
        }
        agrees
    }

    /// Consecutive agreeing frames so far.
    pub fn run(&self) -> u32 {
        self.run
    }

    /// Whether a time is believed.
    pub fn anchor(&self) -> Option<Anchor> {
        self.anchor
    }

    /// The UTC second that began at input sample `pos`, if the anchor says.
    pub fn utc_at(&self, pos: f64) -> Option<i64> {
        let a = self.anchor?;
        let el = (pos - a.pos) / self.rate;
        let n = el.round();
        let ok = el.abs() <= HOLDOVER_S && (el - n).abs() < TOL_FIXED_S + TOL_PPM * el.abs();
        ok.then(|| a.utc + n as i64)
    }

    /// Forgets everything — a new station, or a receiver restarted.
    pub fn reset(&mut self) {
        self.run = 0;
        self.last = None;
        self.anchor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 12_000.0;

    #[test]
    fn one_frame_is_not_enough() {
        let mut t = Tracker::new(R, 2);
        assert!(!t.frame(0.0, 600));
        assert_eq!(t.anchor(), None);
        assert!(t.frame(60.0 * R, 660));
        assert_eq!(t.anchor(), Some(Anchor { pos: 60.0 * R, utc: 660 }));
        assert_eq!(t.utc_at(75.0 * R), Some(675));
    }

    #[test]
    fn a_frame_that_disagrees_starts_the_count_again() {
        let mut t = Tracker::new(R, 3);
        t.frame(0.0, 600);
        t.frame(60.0 * R, 660);
        // Parity passed, but the time is an hour out.
        assert!(!t.frame(120.0 * R, 4320));
        assert_eq!(t.run(), 1);
        assert_eq!(t.anchor(), None);
    }

    #[test]
    fn a_missed_minute_between_two_good_ones_still_agrees() {
        let mut t = Tracker::new(R, 2);
        t.frame(0.0, 600);
        assert!(t.frame(120.0 * R, 720));
    }

    #[test]
    fn a_sample_clock_off_by_fifty_ppm_still_agrees_across_an_hour() {
        let mut t = Tracker::new(R, 2);
        t.frame(0.0, 0);
        assert!(t.frame(3600.0 * R * (1.0 + 50e-6), 3600));
    }

    #[test]
    fn a_boundary_between_seconds_has_no_utc() {
        let mut t = Tracker::new(R, 2);
        t.frame(0.0, 600);
        t.frame(60.0 * R, 660);
        assert_eq!(t.utc_at(60.5 * R), None);
        assert_eq!(t.utc_at((60.0 + 2.0 * HOLDOVER_S) * R), None);
    }
}
