//! The automatic LNA: which front-end state an RSP should be in, decided from
//! what the receiver itself reports.
//!
//! An RSP has two gain stages and they are both counted backwards. The IF
//! gain reduction the service's own AGC already looks after. The LNA state it
//! does not, and it is the one that matters: too little attenuation and the
//! converter overloads, which nothing after it can undo; too much and the
//! noise figure is thrown away for nothing. The right state changes with the
//! band, the aerial and the time of day, which is a lot to ask an operator to
//! find by hand on a ladder whose length changes from band to band.
//!
//! The rule is the one a careful operator follows: as much front-end gain as
//! the receiver will take, and no more.
//!
//! - **Up** (more attenuation) when the service reports the converter
//!   overloaded, or when the IF AGC has run out of reduction — it is against
//!   its stop and the level is still above its set point.
//! - **Down** (more gain) when neither is true and the IF AGC has room to take
//!   up the step, but never below the state the operator set: that one is the
//!   most front-end gain this loop may use.
//!
//! Fast up, slow down, and a state that overloaded is left alone for a while
//! before it is tried again — longer each time it fails. The asymmetry is the
//! same one the Hermes-Lite loop uses, for the same reason: whatever caused
//! the overload has not necessarily gone away.
//!
//! No hardware in here: [`AutoLna::step`] is told what the receiver reports
//! and answers with the state to ask for, so every rule above has a test.
// Why: KC2RGW 2026-11-23: "This receiver has unusual gain stages, confusing
//      to figure out, is there any opportunity to set an automatic mode".

use std::time::{Duration, Instant};

/// How often the state may go up while the converter is overloading. Long
/// enough for the last step to reach the hardware and the overload report to
/// clear, short enough that a ten-step climb is three seconds.
const ATTACK_OVERLOAD: Duration = Duration::from_millis(300);
/// How often the state may go up because the IF AGC is against its stop.
/// Slower than the overload case: the AGC needs time to come off the stop
/// after each step — at its 5 Hz setting, most of a second — and nothing is
/// being damaged while it does.
const ATTACK_AGC: Duration = Duration::from_secs(1);
/// How often the state may come back down.
const DECAY: Duration = Duration::from_secs(2);
/// How long a state that had to be left is kept away from, the first time.
const HOLD_BASE: Duration = Duration::from_secs(20);
/// The longest a state is kept away from, however often it has failed.
const HOLD_MAX: Duration = Duration::from_secs(600);
/// A step up this soon after a step down was caused by the step down: the
/// state just tried does not work, and the next try waits twice as long.
const BOUNCE: Duration = Duration::from_secs(10);
/// How close to its maximum reduction the IF AGC has to be to count as out of
/// range, in dB.
const AGC_STOP_MARGIN_DB: i32 = 1;
/// How much reduction the IF AGC must have in hand before the LNA gives it a
/// step's worth of extra signal, in dB. Wider than the largest single step on
/// any RSP's LNA ladder, so a step down can never put the AGC on its stop and
/// send the state straight back up.
const AGC_HEADROOM_DB: i32 = 20;

/// What the receiver reports, once per look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inputs {
    /// The service has reported the converter overloaded since the last look.
    pub overload: bool,
    /// The LNA state the hardware is in.
    pub lna: u8,
    /// The lowest state the loop may use: the operator's own setting.
    pub floor: u8,
    /// The highest state this band has.
    pub max_lna: u8,
    /// The IF gain reduction the service's AGC has arrived at and the most it
    /// can apply, in dB. `None` while the AGC is off, when the IF gain is the
    /// operator's and says nothing about the signal.
    pub if_gr: Option<(i32, i32)>,
}

/// Why the loop asked for a new state — for the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The converter was overloading.
    Overload,
    /// The IF AGC was at maximum reduction.
    AgcAtStop,
    /// Neither, and there was gain to give back.
    Recovered,
    /// The state was below the operator's floor.
    Floor,
}

impl Reason {
    pub fn describe(self) -> &'static str {
        match self {
            Reason::Overload => "the converter was overloading",
            Reason::AgcAtStop => "the IF AGC was at maximum reduction",
            Reason::Recovered => "the overload has cleared",
            Reason::Floor => "back to the state you set",
        }
    }
}

/// The loop's memory between looks.
#[derive(Debug, Clone)]
pub struct AutoLna {
    last_move: Option<Instant>,
    last_down: Option<Instant>,
    /// No step down before this.
    hold_until: Option<Instant>,
    hold: Duration,
}

impl Default for AutoLna {
    fn default() -> Self {
        AutoLna { last_move: None, last_down: None, hold_until: None, hold: HOLD_BASE }
    }
}

impl AutoLna {
    /// Forget what was learned: the band or the aerial has changed, or the
    /// operator has moved the control, and a state that failed before may be
    /// exactly right now.
    pub fn reset(&mut self) {
        *self = AutoLna::default();
    }

    /// The state to ask for, if it should change. One step at a time.
    pub fn step(&mut self, now: Instant, i: Inputs) -> Option<(u8, Reason)> {
        let since = |t: Option<Instant>| t.map(|t| now.saturating_duration_since(t));
        let moved = since(self.last_move);
        let floor = i.floor.min(i.max_lna);

        if i.lna < floor {
            self.last_move = Some(now);
            return Some((floor, Reason::Floor));
        }

        let agc_at_stop = i.if_gr.is_some_and(|(gr, max)| gr >= max - AGC_STOP_MARGIN_DB);
        if i.overload || agc_at_stop {
            let (reason, attack) = if i.overload {
                (Reason::Overload, ATTACK_OVERLOAD)
            } else {
                (Reason::AgcAtStop, ATTACK_AGC)
            };
            if i.lna >= i.max_lna || moved.is_some_and(|d| d < attack) {
                return None;
            }
            // Driven back up by the step down just taken: that state does not
            // work at the moment, so the next try at it waits twice as long.
            if since(self.last_down).is_some_and(|d| d < BOUNCE) {
                self.hold = (self.hold * 2).min(HOLD_MAX);
            }
            self.hold_until = Some(now + self.hold);
            self.last_move = Some(now);
            return Some((i.lna + 1, reason));
        }

        let agc_has_room = i.if_gr.is_none_or(|(gr, max)| gr <= max - AGC_HEADROOM_DB);
        if i.lna > floor
            && agc_has_room
            && moved.is_none_or(|d| d >= DECAY)
            && self.hold_until.is_none_or(|t| now >= t)
        {
            // A step down that was not followed by a step up for a long while
            // means the last trouble is over; the next one starts from the
            // short hold again.
            if moved.is_some_and(|d| d >= HOLD_MAX) {
                self.hold = HOLD_BASE;
            }
            self.last_down = Some(now);
            self.last_move = Some(now);
            return Some((i.lna - 1, Reason::Recovered));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGC_MAX: i32 = 59;

    fn quiet(lna: u8) -> Inputs {
        Inputs { overload: false, lna, floor: 0, max_lna: 18, if_gr: Some((30, AGC_MAX)) }
    }

    fn overloaded(lna: u8) -> Inputs {
        Inputs { overload: true, ..quiet(lna) }
    }

    /// Runs the loop at ten looks a second for `secs`, applying every answer,
    /// and returns the state it ended in.
    fn run(
        a: &mut AutoLna,
        t: &mut Instant,
        secs: u64,
        mut i: Inputs,
        f: impl Fn(u8) -> Inputs,
    ) -> u8 {
        for _ in 0..secs * 10 {
            i = f(i.lna);
            if let Some((s, _)) = a.step(*t, i) {
                i.lna = s;
            }
            *t += Duration::from_millis(100);
        }
        i.lna
    }

    #[test]
    fn a_receiver_that_is_content_is_left_alone() {
        let mut a = AutoLna::default();
        let mut t = Instant::now();
        assert_eq!(run(&mut a, &mut t, 60, quiet(0), quiet), 0);
    }

    #[test]
    fn an_overload_steps_the_attenuation_up_and_stops_when_it_clears() {
        let mut a = AutoLna::default();
        let mut t = Instant::now();
        // Overloads in states 0 to 3, clean from 4 up.
        let band = |lna: u8| if lna < 4 { overloaded(lna) } else { quiet(lna) };
        assert_eq!(run(&mut a, &mut t, 5, quiet(0), band), 4);
    }

    #[test]
    fn the_climb_is_one_step_at_a_time_and_no_faster_than_the_attack() {
        let mut a = AutoLna::default();
        let t = Instant::now();
        assert_eq!(a.step(t, overloaded(2)), Some((3, Reason::Overload)));
        assert_eq!(a.step(t + Duration::from_millis(100), overloaded(3)), None);
        assert_eq!(
            a.step(t + Duration::from_millis(300), overloaded(3)),
            Some((4, Reason::Overload))
        );
    }

    #[test]
    fn it_stops_at_the_top_of_the_band_s_ladder() {
        let mut a = AutoLna::default();
        let mut t = Instant::now();
        assert_eq!(run(&mut a, &mut t, 30, quiet(0), overloaded), 18);
    }

    #[test]
    fn the_gain_comes_back_once_the_overload_has_been_gone_a_while() {
        let mut a = AutoLna::default();
        let mut t = Instant::now();
        let band = |lna: u8| if lna < 4 { overloaded(lna) } else { quiet(lna) };
        assert_eq!(run(&mut a, &mut t, 5, quiet(0), band), 4);
        // The station goes off the air. Nothing for the hold, then a step
        // every two seconds, all the way back.
        assert_eq!(run(&mut a, &mut t, 10, quiet(4), quiet), 4, "still inside the hold");
        assert_eq!(run(&mut a, &mut t, 30, quiet(4), quiet), 0);
    }

    #[test]
    fn it_never_goes_below_the_state_the_operator_set() {
        let mut a = AutoLna::default();
        let mut t = Instant::now();
        let floored = |lna: u8| Inputs { floor: 3, ..quiet(lna) };
        assert_eq!(run(&mut a, &mut t, 120, floored(6), floored), 3);
    }

    #[test]
    fn a_state_below_the_floor_is_put_back_on_it_at_once() {
        let mut a = AutoLna::default();
        let i = Inputs { floor: 5, ..quiet(2) };
        assert_eq!(a.step(Instant::now(), i), Some((5, Reason::Floor)));
    }

    #[test]
    fn a_floor_this_band_does_not_have_is_the_top_of_its_ladder() {
        let mut a = AutoLna::default();
        let i = Inputs { floor: 20, max_lna: 9, ..quiet(9) };
        assert_eq!(a.step(Instant::now(), i), None);
    }

    /// The case that would otherwise oscillate for ever: state 3 overloads,
    /// state 4 does not. Each failed try at 3 doubles the wait before the next.
    #[test]
    fn a_state_that_keeps_failing_is_tried_less_and_less_often() {
        let mut a = AutoLna::default();
        let mut t = Instant::now();
        let band = |lna: u8| if lna < 4 { overloaded(lna) } else { quiet(lna) };
        let mut i = quiet(0);
        let mut tries_at_3 = Vec::new();
        for _ in 0..20 * 60 * 10 {
            i = band(i.lna);
            if let Some((s, r)) = a.step(t, i) {
                if r == Reason::Recovered {
                    tries_at_3.push(t);
                }
                i.lna = s;
            }
            t += Duration::from_millis(100);
        }
        assert!(i.lna == 4 || i.lna == 3);
        let gaps: Vec<u64> =
            tries_at_3.windows(2).map(|w| w[1].duration_since(w[0]).as_secs()).collect();
        assert!(gaps.len() >= 3, "it does keep trying: {gaps:?}");
        assert!(gaps.windows(2).all(|w| w[1] >= w[0]), "and never more often: {gaps:?}");
        assert!(gaps.last().is_some_and(|g| *g >= 300), "ending minutes apart: {gaps:?}");
        assert!(tries_at_3.len() < 12, "a handful of tries in twenty minutes: {gaps:?}");
    }

    #[test]
    fn an_agc_at_its_stop_steps_up_more_slowly_than_an_overload() {
        let mut a = AutoLna::default();
        let t = Instant::now();
        let stuck = |lna: u8| Inputs { if_gr: Some((59, AGC_MAX)), ..quiet(lna) };
        assert_eq!(a.step(t, stuck(0)), Some((1, Reason::AgcAtStop)));
        assert_eq!(a.step(t + Duration::from_millis(500), stuck(1)), None);
        assert_eq!(a.step(t + Duration::from_secs(1), stuck(1)), Some((2, Reason::AgcAtStop)));
    }

    #[test]
    fn the_gain_only_comes_back_when_the_agc_can_take_up_the_step() {
        let mut t = Instant::now();
        // 45 dB of reduction in use out of 59: a step down could put the AGC
        // on its stop, so the state stays.
        let tight = |lna: u8| Inputs { if_gr: Some((45, AGC_MAX)), ..quiet(lna) };
        assert_eq!(run(&mut AutoLna::default(), &mut t, 120, tight(6), tight), 6);
        // 39 dB in use leaves the twenty in hand.
        let roomy = |lna: u8| Inputs { if_gr: Some((39, AGC_MAX)), ..quiet(lna) };
        assert_eq!(run(&mut AutoLna::default(), &mut t, 120, roomy(6), roomy), 0);
    }

    #[test]
    fn with_the_agc_off_only_the_overload_report_moves_it() {
        let mut a = AutoLna::default();
        let mut t = Instant::now();
        let manual = |lna: u8| Inputs { if_gr: None, ..quiet(lna) };
        assert_eq!(run(&mut a, &mut t, 60, manual(5), manual), 0, "nothing to wait for");
        let hot = |lna: u8| Inputs { if_gr: None, overload: lna < 2, ..quiet(lna) };
        let end = run(&mut a, &mut t, 5, manual(0), hot);
        assert_eq!(end, 2);
    }

    #[test]
    fn a_reset_forgets_the_hold() {
        let mut a = AutoLna::default();
        let t = Instant::now();
        assert_eq!(a.step(t, overloaded(2)), Some((3, Reason::Overload)));
        let later = t + Duration::from_secs(3);
        assert_eq!(a.step(later, quiet(3)), None, "held");
        a.reset();
        assert_eq!(a.step(later, quiet(3)), Some((2, Reason::Recovered)));
    }
}
