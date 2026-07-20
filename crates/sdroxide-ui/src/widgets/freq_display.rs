//! PowerSDR-style frequency readout: each digit tunes with the scroll wheel,
//! or click upper/lower half to increment/decrement.

use eframe::egui::{self, Color32, Label, RichText, Sense, Ui};

const DIGIT_SIZE: f32 = 30.0;
/// Smooth-scroll points per tuning step.
const SCROLL_STEP: f32 = 30.0;

/// Shows `hz` as a 10-digit tunable readout. Returns `Some(new_hz)` on change.
pub fn show(ui: &mut Ui, id: egui::Id, hz: f64) -> Option<f64> {
    let mut freq = hz.round().max(0.0) as i64;
    let orig = freq;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        for p in (0..10u32).rev() {
            if p < 9 && (p + 1) % 3 == 0 {
                ui.add(Label::new(
                    RichText::new(".").monospace().size(DIGIT_SIZE).color(Color32::from_gray(110)),
                ));
            }

            let step = 10i64.pow(p);
            let digit = (freq / step) % 10;
            let leading_zero = p > 0 && freq < step;
            let color = if leading_zero {
                Color32::from_gray(70)
            } else {
                Color32::from_rgb(255, 209, 66)
            };

            let resp = ui
                .add(
                    Label::new(
                        RichText::new(format!("{digit}"))
                            .monospace()
                            .size(DIGIT_SIZE)
                            .color(color),
                    )
                    .sense(Sense::click()),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);

            if resp.hovered() {
                ui.painter().hline(
                    resp.rect.x_range(),
                    resp.rect.bottom() - 1.0,
                    (2.0, Color32::from_rgb(255, 209, 66)),
                );
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                let acc_id = id.with("acc").with(p);
                let mut acc = ui.data_mut(|d| d.get_temp::<f32>(acc_id).unwrap_or(0.0));
                acc += scroll;
                while acc >= SCROLL_STEP {
                    freq += step;
                    acc -= SCROLL_STEP;
                }
                while acc <= -SCROLL_STEP {
                    freq = (freq - step).max(0);
                    acc += SCROLL_STEP;
                }
                ui.data_mut(|d| d.insert_temp(acc_id, acc));
            }

            if resp.clicked() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    if pos.y < resp.rect.center().y {
                        freq += step;
                    } else {
                        freq = (freq - step).max(0);
                    }
                }
            }
        }
        ui.add(Label::new(
            RichText::new(" Hz").size(12.0).color(Color32::from_gray(140)),
        ));
    });

    (freq != orig).then_some(freq as f64)
}
