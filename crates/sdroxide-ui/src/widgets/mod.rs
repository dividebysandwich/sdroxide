pub mod bandplan;
pub mod channels;
pub mod freq_display;
pub mod map_labels;
pub mod memories;
pub mod smeter;
pub mod spectrum3d;
pub mod spectrum_view;
pub mod wide_spectrum;
pub mod worldmap;

/// Scroll points worth one detent — one notch of the wheel — until the
/// operator says otherwise ([`set_wheel_points_per_step`]).
pub(crate) const SCROLL_PER_DETENT: f32 = 30.0;

/// What this frame's scrolling was worth, in whole steps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WheelSteps {
    /// The wheel itself: vertical scrolling, or either axis with Shift held.
    pub main: f32,
    /// Sideways scrolling without Shift — a thumb wheel, a tilt wheel, two
    /// fingers swept across a trackpad.
    pub side: f32,
}

/// Scrolling not yet worth a step, one bank per axis so that a thumb wheel's
/// crumbs never top up the main wheel's. Kept by the caller across frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WheelBank {
    pub main: f32,
    pub side: f32,
}

/// Tell every wheel-stepped control how much smooth scrolling is one step.
/// Called once a frame with [`sdroxide_types::WheelSettings::points_per_step`].
pub(crate) fn set_wheel_points_per_step(ctx: &eframe::egui::Context, points: f32) {
    let points = if points.is_finite() { points.clamp(1.0, 1_000.0) } else { SCROLL_PER_DETENT };
    ctx.data_mut(|d| d.insert_temp(eframe::egui::Id::new("wheel-points-per-step"), points));
}

fn wheel_points_per_step(ctx: &eframe::egui::Context) -> f32 {
    ctx.data(|d| d.get_temp(eframe::egui::Id::new("wheel-points-per-step")))
        .unwrap_or(SCROLL_PER_DETENT)
}

/// Whether a scroll event with no gesture around it is a notch by itself.
///
/// True on macOS, where the two kinds of scrolling can be told apart. A
/// trackpad, and a free-running thumb wheel, send a gesture: a `Start`, a
/// stream of small moves, an `End`. A wheel with detents sends one bare event
/// per notch — and in *points*, 12 or 13 of them at an unhurried pace and a
/// hundred when spun, because macOS accelerates it. Banked against a fixed
/// notch size that took two to four clicks for one tuning step, and a
/// different number every time.
///
/// Not elsewhere: Windows and a browser send a trackpad's crumbs with no
/// gesture around them either, and every crumb would become a step.
// Why: KC2RGW 2026-11-23, MX Master on macOS: "it takes two or three clicks of
//      the scroll wheel to move one increment in step".
const BARE_EVENT_IS_A_NOTCH: bool = cfg!(all(target_os = "macos", not(test)));

/// Whole wheel detents from this frame's scroll events, banking the remainder
/// in `acc` (kept by the caller across frames).
///
/// A notch has to be worth exactly one tuning step, and no platform agrees on
/// what a notch is worth: winit spends a line on it, a browser 100 pixels or
/// three lines, and egui then smears whichever it was over several frames of
/// `smooth_scroll_delta`. Counting points there tuned three steps for one
/// notch in a browser and one and a third natively, so the raw events are
/// counted instead — one event is one detent, whichever way it points.
///
/// Fine-grained scrolling is the exception, and the reason for the bank. A
/// trackpad, or a high-resolution wheel reporting a notch in pieces, arrives
/// as a stream of deltas far smaller than a notch; those add up in `acc` until
/// they are worth a whole step, so a slow scroll still tunes instead of being
/// thrown away.
///
/// With `shift` — egui's horizontal-scroll modifier, and what macOS reports a
/// shifted wheel on to begin with — either axis counts, so the gesture is not
/// dead on the platforms that move it sideways.
pub(crate) fn wheel_detents(ui: &eframe::egui::Ui, shift: bool, acc: &mut f32) -> f32 {
    wheel_detents_ctx(ui.ctx(), shift, acc)
}

pub(crate) fn wheel_detents_ctx(ctx: &eframe::egui::Context, shift: bool, acc: &mut f32) -> f32 {
    let mut bank = WheelBank { main: *acc, side: 0.0 };
    let steps = wheel_steps_ctx(ctx, shift, &mut bank);
    *acc = bank.main;
    steps.main
}

/// [`wheel_detents`] for a control that tells the side wheel from the main
/// one: the same counting, with sideways scrolling kept apart instead of
/// ignored.
pub(crate) fn wheel_steps(ui: &eframe::egui::Ui, shift: bool, bank: &mut WheelBank) -> WheelSteps {
    wheel_steps_ctx(ui.ctx(), shift, bank)
}

pub(crate) fn wheel_steps_ctx(
    ctx: &eframe::egui::Context,
    shift: bool,
    bank: &mut WheelBank,
) -> WheelSteps {
    let per_step = wheel_points_per_step(ctx);
    // Whether a gesture was under way when this frame's events began. Several
    // controls read the same events in one frame, so the answer is remembered
    // per pass rather than advanced by each of them in turn.
    let id = eframe::egui::Id::new("wheel-gesture");
    let pass = ctx.cumulative_pass_nr();
    let (seen, before, after): (u64, bool, bool) =
        ctx.data(|d| d.get_temp(id)).unwrap_or((u64::MAX, false, false));
    let mut gesture = if seen == pass { before } else { after };
    let started_in = gesture;
    let steps = ctx.input(|i| {
        count_wheel_steps(&i.events, shift, per_step, BARE_EVENT_IS_A_NOTCH, &mut gesture, bank)
    });
    ctx.data_mut(|d| d.insert_temp(id, (pass, started_in, gesture)));
    steps
}

/// The counting behind [`wheel_steps`], on plain values.
///
/// `per_step` is how many points of smooth scrolling make one step.
/// `bare_notch` is [`BARE_EVENT_IS_A_NOTCH`]; `gesture` is whether a
/// `Start` has been seen without its `End`, carried from frame to frame.
fn count_wheel_steps(
    events: &[eframe::egui::Event],
    shift: bool,
    per_step: f32,
    bare_notch: bool,
    gesture: &mut bool,
    bank: &mut WheelBank,
) -> WheelSteps {
    use eframe::egui::{Event, MouseWheelUnit, TouchPhase};

    let mut steps = WheelSteps::default();
    for ev in events {
        let Event::MouseWheel { unit, delta, phase, .. } = ev else { continue };
        tracing::trace!(target: "wheel", "{ev:?}");
        if *phase == TouchPhase::Start {
            *gesture = true;
        }
        let in_gesture = *gesture;
        if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
            *gesture = false;
        }
        // Shift folds both axes onto the main wheel. Otherwise an event
        // belongs to its larger axis, so a swipe that is not quite level does
        // not turn both wheels at once.
        let (d, side) = if shift {
            (delta.x + delta.y, false)
        } else if delta.x.abs() > delta.y.abs() {
            (delta.x, true)
        } else {
            (delta.y, false)
        };
        let points = match unit {
            MouseWheelUnit::Point => d,
            // A line, or a page, is a notch by itself, so it counts as a
            // whole one below however few points a line is worth here.
            MouseWheelUnit::Line | MouseWheelUnit::Page => d * per_step,
        };
        if points == 0.0 {
            continue;
        }
        let (count, acc) = if side {
            (&mut steps.side, &mut bank.side)
        } else {
            (&mut steps.main, &mut bank.main)
        };
        let bare = bare_notch && !in_gesture && *unit == MouseWheelUnit::Point;
        if bare || points.abs() >= per_step {
            // A whole notch, whatever the platform spent on it. Anything
            // banked belonged to the finer gesture before it.
            *count += points.signum();
            *acc = 0.0;
        } else {
            *acc += points;
            if acc.abs() >= per_step {
                *count += acc.signum();
                *acc -= acc.signum() * per_step;
            }
        }
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Event, MouseWheelUnit, TouchPhase, Vec2};

    /// One headless frame of wheel events, and what it was worth in detents.
    fn detents(events: Vec<Event>, acc: &mut f32) -> f32 {
        let ctx = egui::Context::default();
        let input = egui::RawInput { events, ..Default::default() };
        let mut n = 0.0;
        ctx.run_ui(input, |ui| n = wheel_detents(ui, false, acc)).drop_without_applying_deltas();
        n
    }

    fn wheel(unit: MouseWheelUnit, y: f32) -> Event {
        Event::MouseWheel {
            unit,
            delta: Vec2::new(0.0, y),
            phase: TouchPhase::Move,
            modifiers: Default::default(),
        }
    }

    /// Issue #136: a notch is one step, whatever the platform spends on it —
    /// a browser's 100 pixels, winit's one line, Firefox's three.
    #[test]
    fn a_notch_is_one_detent_whatever_it_costs_in_points() {
        let mut acc = 0.0;
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 100.0)], &mut acc), 1.0);
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Line, 1.0)], &mut acc), 1.0);
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Line, -3.0)], &mut acc), -1.0);
        // A fast spin is not throttled, only kept whole: three notches landing
        // in one frame are three steps.
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 100.0); 3], &mut acc), 3.0);
        // A high-resolution wheel reports one notch in pieces; the pieces make
        // one step between them, not four.
        let quarters = vec![wheel(MouseWheelUnit::Line, 0.25); 4];
        assert_eq!(detents(quarters, &mut acc), 1.0);
        assert_eq!(acc, 0.0);
    }

    /// A trackpad's crumbs bank up rather than being thrown away.
    #[test]
    fn fine_scrolling_banks_until_it_is_worth_a_step() {
        let mut acc = 0.0;
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 10.0)], &mut acc), 0.0);
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 10.0)], &mut acc), 0.0);
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 10.0)], &mut acc), 1.0);
        assert_eq!(acc, 0.0, "a spent bank starts over");
        // Backwards is not a slower forwards: the bank nets out.
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 20.0)], &mut acc), 0.0);
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, -20.0)], &mut acc), 0.0);
        assert_eq!(acc, 0.0);
        // And a notch arriving mid-gesture is worth exactly one step, not one
        // plus whatever the crumbs before it had added up to.
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 20.0)], &mut acc), 0.0);
        assert_eq!(detents(vec![wheel(MouseWheelUnit::Point, 100.0)], &mut acc), 1.0);
        assert_eq!(acc, 0.0);
    }

    /// One frame of events through the counting itself, with a notch size, the
    /// macOS rule switched on or off, and the gesture state carried along.
    fn steps(
        events: Vec<Event>,
        shift: bool,
        bare_notch: bool,
        gesture: &mut bool,
        bank: &mut WheelBank,
    ) -> WheelSteps {
        count_wheel_steps(&events, shift, 30.0, bare_notch, gesture, bank)
    }

    fn scroll(x: f32, y: f32, phase: TouchPhase) -> Event {
        Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: Vec2::new(x, y),
            phase,
            modifiers: Default::default(),
        }
    }

    /// What an MX Master sends on macOS: one bare event a click, 13 points at
    /// rest and a hundred when spun. Each is one step, not a share of one.
    #[test]
    fn a_bare_click_on_macos_is_one_step_however_few_points_it_is() {
        let (mut g, mut bank) = (false, WheelBank::default());
        for _ in 0..10 {
            let n =
                steps(vec![scroll(0.0, 13.0, TouchPhase::Move)], false, true, &mut g, &mut bank);
            assert_eq!(n, WheelSteps { main: 1.0, side: 0.0 });
        }
        let spun = vec![
            scroll(0.0, -12.0, TouchPhase::Move),
            scroll(0.0, -103.0, TouchPhase::Move),
            scroll(0.0, -103.0, TouchPhase::Move),
        ];
        assert_eq!(steps(spun, false, true, &mut g, &mut bank).main, -3.0);
        assert_eq!(bank, WheelBank::default(), "a click leaves nothing banked");
        // Where bare events cannot be told from a trackpad's crumbs, the same
        // 13 points are banked as before.
        let n = steps(vec![scroll(0.0, 13.0, TouchPhase::Move)], false, false, &mut g, &mut bank);
        assert_eq!(n.main, 0.0);
        assert_eq!(bank.main, 13.0);
    }

    /// A gesture — a trackpad, a thumb wheel — is banked even on macOS, across
    /// frames, until its `End`; the click after it is a click again.
    #[test]
    fn a_gesture_is_banked_and_ends_with_its_end() {
        let (mut g, mut bank) = (false, WheelBank::default());
        let n = steps(
            vec![scroll(0.0, 4.0, TouchPhase::Start), scroll(0.0, 5.0, TouchPhase::Move)],
            false,
            true,
            &mut g,
            &mut bank,
        );
        assert_eq!(n.main, 0.0);
        assert!(g, "the gesture carries into the next frame");
        let n = steps(vec![scroll(0.0, 5.0, TouchPhase::Move); 5], false, true, &mut g, &mut bank);
        assert_eq!(n.main, 1.0, "34 points banked is one step of 30");
        assert_eq!(bank.main, 4.0);
        // The `End` event belongs to the gesture; the one after it does not.
        let n = steps(
            vec![scroll(0.0, 5.0, TouchPhase::End), scroll(0.0, 13.0, TouchPhase::Move)],
            false,
            true,
            &mut g,
            &mut bank,
        );
        assert!(!g);
        assert_eq!(n.main, 1.0);
    }

    /// Sideways scrolling is the side wheel, with a bank of its own; Shift
    /// makes it the main wheel again, which is how macOS reports Shift + wheel.
    #[test]
    fn sideways_scrolling_is_the_side_wheel_unless_shift_is_held() {
        let (mut g, mut bank) = (false, WheelBank::default());
        let turn = |n: usize| {
            let mut v = vec![scroll(-5.0, 0.0, TouchPhase::Start)];
            v.extend(vec![scroll(-5.0, 0.0, TouchPhase::Move); n - 2]);
            v.push(scroll(-5.0, 0.0, TouchPhase::End));
            v
        };
        // Sixty points to the left: two fine steps down, nothing on the main.
        assert_eq!(
            steps(turn(12), false, true, &mut g, &mut bank),
            WheelSteps { main: 0.0, side: -2.0 }
        );
        // Crumbs on one axis do not top up the other.
        steps(vec![scroll(20.0, 0.0, TouchPhase::Start)], false, true, &mut g, &mut bank);
        let n = steps(vec![scroll(0.0, 20.0, TouchPhase::Move)], false, true, &mut g, &mut bank);
        assert_eq!(n, WheelSteps::default());
        assert_eq!(bank, WheelBank { main: 20.0, side: 20.0 });
        // A swipe that is not quite level turns the wheel it mostly is.
        let (mut g, mut bank) = (true, WheelBank::default());
        let n = steps(vec![scroll(40.0, 9.0, TouchPhase::Move)], false, true, &mut g, &mut bank);
        assert_eq!(n, WheelSteps { main: 0.0, side: 1.0 });
        // With Shift there is no side wheel: both axes are the main one.
        let (mut g, mut bank) = (false, WheelBank::default());
        let n = steps(vec![scroll(13.0, 0.0, TouchPhase::Move)], true, true, &mut g, &mut bank);
        assert_eq!(n, WheelSteps { main: 1.0, side: 0.0 });
    }

    /// The notch size is the operator's: at 13 points a step, 13 points of
    /// banked scrolling is a step, and a line is still exactly one.
    #[test]
    fn the_notch_size_is_what_settings_say() {
        let (mut g, mut bank) = (false, WheelBank::default());
        let ev = vec![scroll(0.0, 13.0, TouchPhase::Move)];
        assert_eq!(count_wheel_steps(&ev, false, 13.0, false, &mut g, &mut bank).main, 1.0);
        let line = vec![wheel(MouseWheelUnit::Line, 1.0)];
        assert_eq!(count_wheel_steps(&line, false, 13.0, false, &mut g, &mut bank).main, 1.0);
        assert_eq!(count_wheel_steps(&line, false, 200.0, false, &mut g, &mut bank).main, 1.0);
    }
}
