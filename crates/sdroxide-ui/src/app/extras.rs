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
/// The window's opening height.
const EXTRAS_H: f32 = 110.0;

/// The window's width: the five slots, their gaps and the frame around them.
pub(in crate::app) const EXTRAS_W: f32 =
    EXTRAS_SLOTS as f32 * SLOT_W + (EXTRAS_SLOTS as f32 - 1.0) * SLOT_GAP + 24.0;

/// Where the window opens, and how big: the remembered geometry when there is
/// one, else its own size at the top right of `avail` (the spectrum frame, just
/// below the top bar and so below the EXTRAS chip). Always kept on `screen`, so
/// a window saved on a larger display is still reachable.
pub(in crate::app) fn extras_place(
    saved: Option<sdroxide_types::DetachedWindow>,
    avail: egui::Rect,
    screen: egui::Rect,
) -> (egui::Pos2, egui::Vec2) {
    let size = saved.map_or(egui::vec2(EXTRAS_W, EXTRAS_H), |g| egui::vec2(g.size[0], g.size[1]));
    let size = size.max(egui::vec2(EXTRAS_W, EXTRAS_H)).min(screen.size() * 0.95);
    let pos = match saved.and_then(|g| g.pos) {
        Some([x, y]) => egui::pos2(x, y),
        None => egui::pos2(avail.right() - size.x - 6.0, avail.top() + 6.0),
    };
    let x = pos.x.clamp(screen.left(), (screen.right() - size.x).max(screen.left()));
    let y = pos.y.clamp(screen.top(), (screen.bottom() - 40.0).max(screen.top()));
    (egui::pos2(x, y), size)
}

impl super::SdroxideApp {
    pub(in crate::app) fn extras_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.show_extras {
            self.extras_was_open = false;
            return;
        }
        // Each opening gets its own window id, so egui holds no position or
        // size for it and the remembered geometry below is what applies.
        if !self.extras_was_open {
            self.extras_epoch += 1;
            self.extras_was_open = true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if !self.tool_windows.contains_key("extras") {
            self.tool_windows.insert(
                "extras",
                super::ToolWindowState {
                    undocked: false,
                    window: self.ui_settings.extras_os_window,
                },
            );
        }
        let screen = ctx.content_rect();
        let frame =
            egui::Rect::from_min_max(egui::pos2(screen.left(), self.top_bar_bottom), screen.max);
        let (pos, size) = extras_place(self.ui_settings.extras_window, frame, screen);
        self.tool_place = Some(super::ToolPlace { pos, size, epoch: self.extras_epoch });
        self.tool_rect = None;
        let mut out = Vec::new();
        let show = self.tool_window_sized(
            ctx,
            "extras",
            "EXTRAS",
            [EXTRAS_W, EXTRAS_H],
            [EXTRAS_W, 0.0],
            false,
            true,
            |me, ui| me.extras_body(ui, &mut out),
        );
        self.tool_place = None;
        cmds.extend(out);
        self.show_extras = show;
        self.remember_extras_geometry(ctx, pos, size);
    }

    /// Keep where the window was left — once the pointer is up, so a drag is
    /// one save — for the docked window and for the detached one.
    fn remember_extras_geometry(&mut self, ctx: &egui::Context, pos: egui::Pos2, size: egui::Vec2) {
        if ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        let mut changed = false;
        #[cfg(not(target_arch = "wasm32"))]
        let undocked = self.tool_windows.get("extras").is_some_and(|s| s.undocked);
        #[cfg(target_arch = "wasm32")]
        let undocked = false;
        if undocked {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(g) = self.tool_windows.get("extras").and_then(|s| s.window) {
                if self.ui_settings.extras_os_window != Some(g) {
                    self.ui_settings.extras_os_window = Some(g);
                    changed = true;
                }
            }
        } else if let Some(r) = self.tool_rect {
            let g = sdroxide_types::DetachedWindow {
                size: [r.width(), r.height()],
                pos: Some([r.min.x, r.min.y]),
            };
            let moved = (r.min - pos).abs().max_elem() > 1.0 || (r.size().x - size.x).abs() > 1.0;
            let differs = self.ui_settings.extras_window.is_none_or(|s| {
                (s.size[0] - g.size[0]).abs() > 1.0
                    || (s.size[1] - g.size[1]).abs() > 1.0
                    || s.pos
                        .zip(g.pos)
                        .is_none_or(|(a, b)| (a[0] - b[0]).abs() > 1.0 || (a[1] - b[1]).abs() > 1.0)
            });
            // Opening at the default is not a choice: only a window the
            // operator has moved or resized is remembered.
            if differs && (moved || self.ui_settings.extras_window.is_some()) {
                self.ui_settings.extras_window = Some(g);
                changed = true;
            }
        }
        if changed {
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
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

    fn screen() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0))
    }

    #[test]
    fn it_opens_at_the_top_right_of_the_spectrum_frame() {
        let avail = egui::Rect::from_min_max(egui::pos2(0.0, 180.0), egui::pos2(1920.0, 1080.0));
        let (pos, size) = extras_place(None, avail, screen());
        assert_eq!(pos.y, 186.0, "just under the top bar");
        assert_eq!(pos.x + size.x, 1914.0, "against the right edge of the frame");
    }

    #[test]
    fn a_remembered_place_and_size_win() {
        let g = sdroxide_types::DetachedWindow { size: [700.0, 200.0], pos: Some([300.0, 400.0]) };
        let (pos, size) = extras_place(Some(g), screen(), screen());
        assert_eq!((pos, size), (egui::pos2(300.0, 400.0), egui::vec2(700.0, 200.0)));
    }

    #[test]
    fn a_place_saved_on_a_bigger_screen_is_pulled_back() {
        let g =
            sdroxide_types::DetachedWindow { size: [700.0, 200.0], pos: Some([5000.0, 4000.0]) };
        let (pos, size) = extras_place(Some(g), screen(), screen());
        assert!(screen().contains_rect(egui::Rect::from_min_size(pos, egui::vec2(size.x, 40.0))));
    }
}
