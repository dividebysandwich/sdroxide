pub mod bandplan;
pub mod freq_display;
pub mod level_slider;
pub mod memories;
pub mod smeter;
pub mod spectrum3d;
pub mod spectrum_view;
pub mod wide_spectrum;
pub mod worldmap;

/// Scroll points worth one detent — one notch of the wheel.
pub(crate) const SCROLL_PER_DETENT: f32 = 30.0;

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
    use eframe::egui::{Event, MouseWheelUnit};

    let mut detents = 0.0f32;
    ui.input(|i| {
        for ev in &i.events {
            let Event::MouseWheel { unit, delta, .. } = ev else { continue };
            let d = if shift { delta.x + delta.y } else { delta.y };
            let points = match unit {
                MouseWheelUnit::Point => d,
                // A line, or a page, is a notch by itself, so it counts as a
                // whole one below however few points a line is worth here.
                MouseWheelUnit::Line | MouseWheelUnit::Page => d * SCROLL_PER_DETENT,
            };
            if points == 0.0 {
                continue;
            }
            if points.abs() >= SCROLL_PER_DETENT {
                // A whole notch, whatever the platform spent on it. Anything
                // banked belonged to the finer gesture before it.
                detents += points.signum();
                *acc = 0.0;
            } else {
                *acc += points;
                if acc.abs() >= SCROLL_PER_DETENT {
                    detents += acc.signum();
                    *acc -= acc.signum() * SCROLL_PER_DETENT;
                }
            }
        }
    });
    detents
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
}
