//! The grid tracker: the Maidenhead squares in the log, drawn on a map.
//!
//! The awards dashboard already tallies grids, and the 3D view already places
//! DXCC entities on a globe — but neither says *where* a worked square is. This
//! draws every 4-character square in the log as a filled cell on the flat map,
//! green where a QSL has come back and amber where it has not, so a gap in a
//! continent reads at a glance.
//!
//! The live decode list is offered as a second layer — the squares *heard*
//! rather than worked — behind a HEARD toggle. The two are told apart by colour,
//! and the heard layer is drawn under the worked one so a square that is both
//! reads as worked.

use eframe::egui::{self, Rect, Sense, Ui, pos2, vec2};
use std::collections::HashSet;

use crate::theme;
use crate::widgets::worldmap::{MapView, alpha, draw_base, interact, wrap180};

use crate::app::SdroxideApp;

/// Below this size the map is not worth drawing and the grid unreadable.
pub const MIN_HEIGHT: f32 = 200.0;

/// A heard square is drawn this alpha; the worked layers are solid enough to
/// out-read it, so a heard-and-worked square never looks merely heard.
const HEARD_ALPHA: f32 = 88.0;
/// Worked and confirmed cells are opaque. A frame of "not worked" would be the
/// whole world, and drawing that is drawing the sea.
const WORKED_ALPHA: f32 = 205.0;

/// The tracker's own state, owned by the app so the pan/zoom survives a close.
#[derive(Default)]
pub struct GridTracker {
    pub view: MapView,
    /// Shade the squares heard but not worked, from the live decode list.
    pub show_heard: bool,
    /// `(log length, (square, confirmed))`: every band's squares, tallied once
    /// per log change rather than every frame.
    worked: Option<(usize, Vec<(String, bool)>)>,
}

/// Draw the map into `rect`. Returns the grid square under the pointer, for a
/// hover label.
///
/// A free function so the projection and the cell fill are testable without an
/// app: `worked` is `(grid, confirmed)` and `heard` is the live squares.
pub fn draw(
    ui: &mut Ui,
    state: &mut GridTracker,
    worked: &[(String, bool)],
    heard: &HashSet<String>,
    home: Option<(f64, f64)>,
    max_h: f32,
) -> Option<String> {
    let avail_w = ui.available_width();
    if avail_w < MIN_HEIGHT {
        return None;
    }
    let h = max_h.min(avail_w).max(MIN_HEIGHT);
    let (rect, resp) = ui.allocate_exact_size(vec2(avail_w, h), Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return None;
    }
    let p = ui.painter_at(rect);
    let map = theme::map();
    p.rect_filled(rect, 0.0, map.sea);

    let aspect = (rect.height() / rect.width()) as f64;
    // A grid tracker has no natural autofit — the squares span continents — so
    // it opens on the whole world and stays where the operator puts it.
    if !state.view.initialized {
        state.view.clat = 0.0;
        state.view.clon = 0.0;
        state.view.lon_span = 360.0;
        state.view.initialized = true;
    }
    state.view.clamp(aspect);
    if interact(ui, &mut state.view, &resp, aspect) {
        crate::repaint::animate(ui.ctx());
    }
    let (clat, clon, lon_span) = (state.view.clat, state.view.clon, state.view.lon_span);
    let lat_span = lon_span * aspect;

    let dot_r = draw_base(&p, rect, clat, clon, lon_span, lat_span, map);

    let project = |lat: f64, lon: f64| {
        let dlon = wrap180(lon - clon);
        pos2(
            rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
            rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
        )
    };

    // One 4-character square is 2° of longitude by 1° of latitude, so its
    // on-screen size is the same for every cell at a given zoom.
    let cell =
        |lat: f64, lon: f64| -> Rect { cell_rect(rect, clat, clon, lon_span, lat_span, lat, lon) };

    let heard_col = alpha(theme::CYAN(), HEARD_ALPHA);
    let worked_col = alpha(theme::YELLOW(), WORKED_ALPHA);
    let confirmed_col = alpha(theme::GREEN(), WORKED_ALPHA);

    // Heard first, under the worked squares: a square in both lists is worked,
    // and must not read as merely heard.
    for g in heard {
        if let Some((lat, lon)) = sdroxide_types::grid_to_latlon(g) {
            let r = cell(lat, lon);
            if rect.intersects(r) {
                p.rect_filled(r, 0.0, heard_col);
            }
        }
    }
    for (g, confirmed) in worked {
        if let Some((lat, lon)) = sdroxide_types::grid_to_latlon(g) {
            let r = cell(lat, lon);
            if rect.intersects(r) {
                p.rect_filled(r, 0.0, if *confirmed { confirmed_col } else { worked_col });
            }
        }
    }

    // Home last, over the grid, so the operator can find themselves.
    if let Some((lat, lon)) = home {
        let c = project(lat, lon);
        if rect.contains(c) {
            p.circle_filled(c, dot_r.max(3.0) + 1.0, alpha(map.home, 90.0));
            p.circle_filled(c, dot_r.max(2.5), map.home);
        }
    }

    // Which square the pointer is over, decided after drawing so the label can
    // be painted on top. Worked squares win over heard ones.
    let mut hovered: Option<String> = None;
    if let Some(m) = resp.hover_pos() {
        let mut best = f32::MAX;
        for g in worked.iter().map(|(g, _)| g).chain(heard.iter()) {
            let Some((lat, lon)) = sdroxide_types::grid_to_latlon(g) else { continue };
            let r = cell(lat, lon);
            if r.contains(m) {
                let d = r.center().distance(m);
                if d <= best {
                    best = d;
                    hovered = Some(g.clone());
                }
            }
        }
        if let Some(g) = &hovered
            && let Some((lat, lon)) = sdroxide_types::grid_to_latlon(g)
        {
            let r = cell(lat, lon);
            p.text(
                r.center() + vec2(0.0, -(r.height() * 0.5).max(6.0)),
                egui::Align2::CENTER_BOTTOM,
                g,
                egui::FontId::monospace(11.0),
                alpha(map.hover, 235.0),
            );
        }
    }
    hovered
}

/// The screen rectangle a 4-character square covers, centred on its
/// `grid_to_latlon` point.
///
/// A square is 2° of longitude by 1° of latitude, so every cell is the same
/// size at a given zoom and the projection stays linear in both axes. The
/// minimum of one point keeps a cell visible on a fully zoomed-out view, where
/// 2° would otherwise round to nothing.
fn cell_rect(
    rect: Rect,
    clat: f64,
    clon: f64,
    lon_span: f64,
    lat_span: f64,
    lat: f64,
    lon: f64,
) -> Rect {
    let dlon = wrap180(lon - clon);
    let c = pos2(
        rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
        rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
    );
    let w = (2.0 / lon_span * f64::from(rect.width())) as f32;
    let h = (1.0 / lat_span * f64::from(rect.height())) as f32;
    Rect::from_center_size(c, vec2(w.max(1.0), h.max(1.0)))
}

impl SdroxideApp {
    pub(in crate::app) fn grid_tracker_window(&mut self, ctx: &egui::Context) {
        crate::layout::arm_center_on_open(
            self.show_grid,
            &mut self.system_was_open.grid,
            &mut self.system_center_pending.grid,
        );
        if !self.show_grid && !self.system_docked_as(crate::app::SystemDock::Grid) {
            return;
        }
        // Every band, whatever the AWARDS window's filter is set to: the map
        // has no band picker of its own, so following that one would hide
        // squares with nothing on screen to say why.
        let len = self.qso_log.len();
        if self.grid_tracker.worked.as_ref().is_none_or(|(l, _)| *l != len) {
            let awards = sdroxide_types::compute_awards(&self.qso_log, None, None);
            let squares = awards.grids.iter().map(|(g, s)| (g.clone(), s.confirmed)).collect();
            self.grid_tracker.worked = Some((len, squares));
        }
        let worked = self.grid_tracker.worked.as_ref().map(|(_, w)| w.clone()).unwrap_or_default();
        let confirmed = worked.iter().filter(|(_, c)| *c).count();
        // Counted whether or not the layer is shown, so the toggle says how
        // many it would add before it is switched on.
        let heard: HashSet<String> = self
            .digi_decodes
            .iter()
            .filter_map(|d| d.grid.as_deref().and_then(sdroxide_types::grid4))
            .collect();
        let home = {
            let g = self.my_grid();
            sdroxide_types::grid_to_latlon(&g)
        };

        let size = egui::vec2(
            crate::layout::window_w(ctx, 760.0),
            crate::layout::window_h(ctx, 560.0),
        );
        let id = crate::layout::salted_id(ctx, "GRID TRACKER");
        if self.system_docked_as(crate::app::SystemDock::Grid) {
            if let Some(win) = self.system_dock_host("GRID TRACKER", id, crate::app::SystemDock::Grid) {
                win.show(ctx, |ui| {
                    crate::chrome::window_body_bg(ui);
                    self.clip_system_dock_body(ui, crate::app::SystemDock::Grid);
                    grid_tracker_body(
                        ui,
                        &mut self.grid_tracker,
                        &worked,
                        confirmed,
                        &heard,
                        home,
                    );
                });
            }
            return;
        }
        let mut open = self.show_grid;
        let pending = self.system_center_pending.grid;
        let resp = crate::layout::center_on_open(
            egui::Window::new("GRID TRACKER")
                .id(id)
                .open(&mut open)
                .frame(crate::chrome::window_frame())
                .resizable(true)
                .default_width(size.x)
                .default_height(size.y),
            ctx,
            pending,
            size,
        )
        .show(ctx, |ui| {
            crate::chrome::window_body_bg(ui);
            grid_tracker_body(ui, &mut self.grid_tracker, &worked, confirmed, &heard, home);
        });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
            self.system_center_pending.grid = false;
        }
        if resp
            .as_ref()
            .is_some_and(|r| self.system_title_pin(ctx, &r.response))
        {
            self.dock_system(crate::app::SystemDock::Grid);
        }
        self.show_grid = open;
    }
}

fn grid_tracker_body(
    ui: &mut egui::Ui,
    tracker: &mut GridTracker,
    worked: &[(String, bool)],
    confirmed: usize,
    heard: &HashSet<String>,
    home: Option<(f64, f64)>,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} worked", worked.len()))
                .color(theme::YELLOW())
                .monospace(),
        );
        ui.label(
            egui::RichText::new(format!("{confirmed} confirmed"))
                .color(theme::GREEN())
                .monospace(),
        );
        if ui
            .selectable_label(
                tracker.show_heard,
                egui::RichText::new(format!("{} heard", heard.len()))
                    .color(theme::CYAN())
                    .monospace(),
            )
            .on_hover_text(
                "Shade the squares heard on the live decode list, not only the \
                 ones in the log — what is on the air now, in cyan under the \
                 worked squares.",
            )
            .clicked()
        {
            tracker.show_heard = !tracker.show_heard;
        }
    });
    ui.separator();
    let h = ui.available_height();
    let none = HashSet::new();
    let shade = if tracker.show_heard { heard } else { &none };
    draw(ui, tracker, worked, shade, home, h);
    ui.separator();
    ui.label(
        egui::RichText::new(
            "Drag to pan, wheel to zoom. Amber = worked, green = confirmed, \
             cyan = heard.",
        )
        .size(10.0)
        .color(theme::gray(150)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    #[test]
    fn a_grid_cell_covers_its_own_two_by_one_square() {
        // World view sized 1 pt per degree, so the arithmetic is readable.
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(360.0, 180.0));
        let a = cell_rect(rect, 0.0, 0.0, 360.0, 180.0, 0.0, 0.0);
        assert!((a.center().x - 180.0).abs() < 0.01, "{}", a.center().x);
        assert!((a.center().y - 90.0).abs() < 0.01, "{}", a.center().y);
        assert!((a.width() - 2.0).abs() < 0.01, "{}", a.width());
        assert!((a.height() - 1.0).abs() < 0.01, "{}", a.height());
        // A square one degree east is one point east and no taller or wider.
        let b = cell_rect(rect, 0.0, 0.0, 360.0, 180.0, 0.0, 1.0);
        assert!((b.center().x - a.center().x - 1.0).abs() < 0.01);
        assert!((b.width() - a.width()).abs() < 0.01);
    }

    #[test]
    fn a_cell_across_the_antimeridian_wraps_the_short_way() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(360.0, 180.0));
        // View centred on +179: a square at -179 is 2° east of it, not 358°
        // west, which is what stops a pan across the seam scattering cells.
        let a = cell_rect(rect, 0.0, 179.0, 360.0, 180.0, 0.0, 179.0);
        let b = cell_rect(rect, 0.0, 179.0, 360.0, 180.0, 0.0, -179.0);
        assert!((b.center().x - a.center().x - 2.0).abs() < 0.01);
    }
}
