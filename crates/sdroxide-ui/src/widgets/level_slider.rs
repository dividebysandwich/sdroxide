//! A vertical waterfall level control, drawn in its own column beside the
//! waterfall.
//!
//! The waterfall maps power to colour over a `[db_floor, db_ceil]` window (see
//! `crate::app::spectrum`), and that pair is the "sensitivity" every other SDR
//! puts on a slider next to the waterfall. This is that slider:
//!
//! - a plain drag slides the whole window up or down — brighter or darker, with
//!   the contrast (the window's width) left alone;
//! - a shift-drag widens or narrows the window — more or less contrast.
//!
//! Dragging it is a manual override, so the caller disarms auto-fit (the FIT
//! chip); otherwise the next automatic glide would quietly undo it. The control
//! uses the same `db_floor`/`db_ceil` state and the same limits as the DISP
//! popup's sliders and the keyboard bindings, so the three always agree.

use eframe::egui::{self, Color32, Rect, Sense, Stroke, Ui, Vec2};

use crate::view::ViewState;

/// The full axis the control spans, and the narrowest window it allows — the
/// same limits `input.rs` clamps the bindings to.
const DB_MIN: f32 = -160.0;
const DB_MAX: f32 = 40.0;
const MIN_RANGE: f32 = 5.0;

/// Width reserved beside the waterfall for the control.
pub const WIDTH: f32 = 16.0;
/// How much one wheel notch moves the window.
const WHEEL_DB: f32 = 2.0;
/// Below this, the control is not worth the strip it costs.
const MIN_H: f32 = 90.0;
const MIN_W: f32 = 260.0;

/// Split `area` into the spectrum/waterfall area and the control's column.
/// `None` where there is no room to spare, so a phone or a tiny window keeps
/// every column for the picture.
pub fn split(area: Rect) -> Option<(Rect, Rect)> {
    if area.height() < MIN_H || area.width() < MIN_W {
        return None;
    }
    let spec = Rect::from_min_max(area.min, egui::pos2(area.right() - WIDTH, area.bottom()));
    let ctl = Rect::from_min_max(egui::pos2(area.right() - WIDTH, area.top()), area.max);
    Some((spec, ctl))
}

/// Where on the track a level sits. Up is louder.
fn y_for(track: Rect, db: f32) -> f32 {
    let t = ((db - DB_MIN) / (DB_MAX - DB_MIN)).clamp(0.0, 1.0);
    track.bottom() - t * track.height()
}

/// Draw the control and handle its drag. Returns true when the operator moved
/// it this frame.
pub fn show(ui: &mut Ui, area: Rect, view: &mut ViewState) -> bool {
    let id = crate::layout::salted_id(ui.ctx(), "wf-level-slider");
    let resp = ui.interact(area, id, Sense::click_and_drag());
    let painter = ui.painter_at(area);

    let track = area.shrink2(Vec2::new(area.width() * 0.32, 6.0));
    painter.rect_filled(track, 2.0, Color32::from_gray(22));

    let mut changed = false;
    let shift = ui.input(|i| i.modifiers.shift);
    if resp.dragged() {
        // Up is louder: an upward drag (negative dy) is a positive dB step.
        let step = -(resp.drag_delta().y / area.height().max(1.0)) * (DB_MAX - DB_MIN);
        let (floor, ceil) = if shift {
            // Contrast: the floor drops and the ceiling rises by the drag.
            clamp_window(view.db_floor - step, view.db_ceil + step)
        } else {
            // Brightness: both ends move together, so contrast is unchanged.
            clamp_window(view.db_floor + step, view.db_ceil + step)
        };
        if floor != view.db_floor || ceil != view.db_ceil {
            view.db_floor = floor;
            view.db_ceil = ceil;
            changed = true;
        }
    } else if resp.hovered() {
        // Wheel over the control: whole notches, like the panadapter's tuning,
        // rather than the smoothed pixel stream. Shift gives contrast, as the
        // drag does. The panadapter never sees these — the pointer is off its
        // rect, so its own wheel handler leaves them alone.
        let bank_id = id.with("wheel-bank");
        let mut bank: f32 = ui.data(|d| d.get_temp(bank_id)).unwrap_or(0.0);
        let detents = crate::widgets::wheel_detents(ui, shift, &mut bank);
        ui.data_mut(|d| d.insert_temp(bank_id, bank));
        if detents != 0.0 {
            let step = detents * WHEEL_DB;
            let (floor, ceil) = if shift {
                clamp_window(view.db_floor - step, view.db_ceil + step)
            } else {
                clamp_window(view.db_floor + step, view.db_ceil + step)
            };
            if floor != view.db_floor || ceil != view.db_ceil {
                view.db_floor = floor;
                view.db_ceil = ceil;
                changed = true;
            }
        }
    }

    // The window, and a bright rule at each end of it.
    let y_ceil = y_for(track, view.db_ceil);
    let y_floor = y_for(track, view.db_floor);
    let band = Rect::from_min_max(
        egui::pos2(track.left(), y_ceil),
        egui::pos2(track.right(), y_floor),
    );
    painter.rect_filled(band, 2.0, crate::theme::CYAN().gamma_multiply(0.30));
    let rule = Stroke::new(1.0, crate::theme::CYAN());
    painter.line_segment([egui::pos2(track.left(), y_ceil), egui::pos2(track.right(), y_ceil)], rule);
    painter.line_segment(
        [egui::pos2(track.left(), y_floor), egui::pos2(track.right(), y_floor)],
        rule,
    );

    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    resp.on_hover_text(format!(
        "Waterfall level\nfloor {:.0} dB · ceiling {:.0} dB\n\n\
         Drag or scroll to brighten or darken; shift for contrast.\n\
         Moving it turns auto-fit (FIT) off.",
        view.db_floor, view.db_ceil,
    ));
    changed
}

/// Clamp a `(floor, ceiling)` request to the axis and the minimum range,
/// keeping the window whole inside the axis by sliding it rather than
/// squashing it.
fn clamp_window(floor: f32, ceil: f32) -> (f32, f32) {
    let range = (ceil - floor).clamp(MIN_RANGE, DB_MAX - DB_MIN);
    let center = (floor + ceil) / 2.0;
    let mut f = center - range / 2.0;
    let mut c = center + range / 2.0;
    if f < DB_MIN {
        f = DB_MIN;
        c = f + range;
    }
    if c > DB_MAX {
        c = DB_MAX;
        f = c - range;
    }
    (f.clamp(DB_MIN, DB_MAX - MIN_RANGE), c.clamp(f + MIN_RANGE, DB_MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_slides_within_the_axis_without_changing_width() {
        // A 100 dB window asked for below the axis slides up to sit on it,
        // keeping its width rather than being squashed against the floor.
        let (f, c) = clamp_window(-170.0, -70.0);
        assert_eq!(f, DB_MIN);
        assert!(((c - f) - 100.0).abs() < 0.01);
        // And one asked for above the axis slides down the same way.
        let (f, c) = clamp_window(0.0, 100.0);
        assert_eq!(c, DB_MAX);
        assert!(((c - f) - 100.0).abs() < 0.01);
    }

    #[test]
    fn a_window_never_narrows_past_the_minimum_range() {
        let (f, c) = clamp_window(-100.0, -99.0);
        assert!((c - f) >= MIN_RANGE - 0.01);
    }

    /// One headless frame mirroring the app's layout: reserve the whole area,
    /// draw a pane over the spectrum part in a child UI, then the control in
    /// the column that is left. A press on the control must register, and a
    /// drag must move the window.
    fn frame(ctx: &egui::Context, view: &mut ViewState, events: Vec<egui::Event>) {
        let screen = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0));
        let input = egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
        ctx.run_ui(input, |ui| {
            let area = ui.available_rect_before_wrap();
            let Some((spec, level)) = split(area) else { return };
            ui.allocate_space(area.size());
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(spec)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            // What the panadapter does with its own rect: claim it, clicks and
            // drags included, so the control beside it has to win on its own.
            let _ = child.allocate_rect(spec, Sense::click_and_drag());
            show(ui, level, view);
        })
        .drop_without_applying_deltas();
    }

    #[test]
    fn a_drag_beside_a_pane_moves_the_window() {
        let ctx = egui::Context::default();
        let mut view = ViewState { db_floor: -120.0, db_ceil: -20.0, ..Default::default() };
        let area = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0));
        let level_center = split(area).unwrap().1.center();
        let press = egui::Event::PointerButton {
            pos: level_center,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        };
        // egui resolves interaction against the previous frame's widget rects,
        // so the control has to have existed (and been hovered) for a frame
        // before the press — as it has in the running app.
        frame(&ctx, &mut view, vec![egui::Event::PointerMoved(level_center)]);
        frame(&ctx, &mut view, vec![egui::Event::PointerMoved(level_center), press]);
        // Drag down: darker, both ends move together.
        let down = level_center + egui::vec2(0.0, 40.0);
        frame(&ctx, &mut view, vec![egui::Event::PointerMoved(down)]);
        assert!(view.db_floor < -120.0, "the floor should have moved down: {}", view.db_floor);
        assert!(
            (view.db_ceil - view.db_floor) >= MIN_RANGE,
            "the window must stay at least the minimum range"
        );
    }

    #[test]
    fn a_wheel_notch_over_the_control_moves_the_window() {
        let ctx = egui::Context::default();
        let mut view = ViewState { db_floor: -120.0, db_ceil: -20.0, ..Default::default() };
        let area = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0));
        let level_center = split(area).unwrap().1.center();
        frame(&ctx, &mut view, vec![egui::Event::PointerMoved(level_center)]);
        let notch = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            modifiers: Default::default(),
        };
        frame(&ctx, &mut view, vec![egui::Event::PointerMoved(level_center), notch]);
        assert_ne!(view.db_floor, -120.0, "a wheel notch should move the window");
        assert!(
            (view.db_ceil - view.db_floor) >= MIN_RANGE,
            "the window must stay at least the minimum range"
        );
    }
}
