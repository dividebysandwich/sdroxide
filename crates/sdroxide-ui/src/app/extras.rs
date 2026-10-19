//! **EXTRAS**: a small launcher for the receivers that are not a band's
//! ordinary modes. The System box's EXTRAS chip opens it, and like PLAYBACK it
//! is a tool window — it can be pulled out into a window of its own with ⇱.
//!
//! Five buttons side by side is the room it is built for; only DAB is in it for
//! now, on the left.

use eframe::egui;
use sdroxide_types::{Command, Mode, RxId};

/// How many launcher buttons the panel is wide enough for.
pub(in crate::app) const EXTRAS_SLOTS: usize = 5;

/// One button's width and height, and the gap between buttons.
const SLOT_W: f32 = 112.0;
const SLOT_H: f32 = 30.0;
const SLOT_GAP: f32 = 6.0;

/// The window's width: the five slots, their gaps and the frame around them.
pub(in crate::app) const EXTRAS_W: f32 =
    EXTRAS_SLOTS as f32 * SLOT_W + (EXTRAS_SLOTS as f32 - 1.0) * SLOT_GAP + 24.0;

impl super::SdroxideApp {
    pub(in crate::app) fn extras_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.show_extras {
            return;
        }
        let mut out = Vec::new();
        let show = self.tool_window(ctx, "extras", "EXTRAS", [EXTRAS_W, 110.0], true, |me, ui| {
            me.extras_body(ui, &mut out)
        });
        cmds.extend(out);
        self.show_extras = show;
    }

    fn extras_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::window_body_bg(ui);
        crate::chrome::menu_caption(ui, "Receivers");
        let mode = self.state.rx[0].mode;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = SLOT_GAP;
            let dab =
                crate::chrome::chip_sized(ui, mode == Mode::Dab, "DAB", egui::vec2(SLOT_W, SLOT_H))
                    .on_hover_text(
                        "DAB / DAB+ digital radio — the player opens below the waterfall",
                    );
            if dab.clicked() {
                cmds.push(Command::SetMode { rx: RxId::Main, mode: Mode::Dab });
            }
            // The other four slots are kept: the panel is sized for them, so a
            // receiver added here does not widen the window.
            for _ in 1..EXTRAS_SLOTS {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(SLOT_W, SLOT_H), egui::Sense::hover());
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    egui::Stroke::new(1.0, crate::theme::gray(60)),
                    egui::StrokeKind::Inside,
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_holds_five_buttons_side_by_side() {
        let row = EXTRAS_SLOTS as f32 * SLOT_W + (EXTRAS_SLOTS as f32 - 1.0) * SLOT_GAP;
        assert!(EXTRAS_W >= row, "{EXTRAS_W} pt is narrower than five buttons ({row} pt)");
    }
}
