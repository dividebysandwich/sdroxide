//! A small pixel/dot-matrix world map for the FT8 QSO panel: renders the
//! continents as glowing dots and marks the home + DX locations with a
//! great-circle path between them, cyberpunk style.

use eframe::egui::{Color32, Pos2, Sense, Ui, pos2, vec2};
use sdroxide_types::{great_circle_points, land_cell, land_mask_dims};

use crate::theme;

/// Draw the map filling the available width (2:1 aspect). `home`/`dx`/`preview`
/// are (lat, lon) in degrees. `preview` is a faint marker for a decode the
/// user clicked but hasn't answered yet (distinct colour from the active DX).
/// When `tx_active`, an animated pulse travels the home→dx path to show we are
/// transmitting toward the contact.
pub fn show(
    ui: &mut Ui,
    home: Option<(f64, f64)>,
    dx: Option<(f64, f64)>,
    preview: Option<(f64, f64)>,
    tx_active: bool,
) {
    let w = ui.available_width();
    let h = (w * 0.5).clamp(90.0, 300.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, theme::INPUT_BG);

    // Render a dot grid sized to the available pixels (about one dot every
    // ~4 px), sampling the high-res land bitmap for crisp coastlines.
    let (mw, mh) = land_mask_dims();
    let cols = ((rect.width() / 4.0) as usize).clamp(80, mw);
    let rows = ((rect.height() / 4.0) as usize).clamp(40, mh);
    let cell_w = rect.width() / cols as f32;
    let cell_h = rect.height() / rows as f32;
    let dot_r = (cell_w.min(cell_h) * 0.44).max(0.7);
    let land = Color32::from_rgb(0x1c, 0x44, 0x58);

    for row in 0..rows {
        let mrow = row * mh / rows;
        for col in 0..cols {
            let mcol = col * mw / cols;
            if land_cell(mcol, mrow) {
                let x = rect.left() + (col as f32 + 0.5) * cell_w;
                let y = rect.top() + (row as f32 + 0.5) * cell_h;
                p.circle_filled(pos2(x, y), dot_r, land);
            }
        }
    }

    let project = |lat: f64, lon: f64| -> Pos2 {
        let x = rect.left() + ((lon + 180.0) / 360.0) as f32 * rect.width();
        let y = rect.top() + ((90.0 - lat) / 180.0) as f32 * rect.height();
        pos2(x, y)
    };

    // Great-circle path as a dotted cyan trail (dots avoid antimeridian wrap).
    if let (Some(hll), Some(dll)) = (home, dx) {
        for (lat, lon) in great_circle_points(hll, dll, 90) {
            p.circle_filled(
                project(lat, lon),
                dot_r.max(1.0),
                Color32::from_rgba_unmultiplied(0, 208, 244, 150),
            );
        }
    }

    // Faint amber preview marker for a clicked-but-unanswered decode.
    if let Some((lat, lon)) = preview {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 3.0, Color32::from_rgba_unmultiplied(255, 210, 63, 45));
        p.circle_filled(c, 2.4, Color32::from_rgba_unmultiplied(255, 210, 63, 190));
    }

    // Endpoints with a glow.
    if let Some((lat, lon)) = home {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 3.0, Color32::from_rgba_unmultiplied(70, 224, 125, 60));
        p.circle_filled(c, 2.6, theme::GREEN);
    }
    if let Some((lat, lon)) = dx {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 3.5, Color32::from_rgba_unmultiplied(255, 42, 85, 70));
        p.circle_filled(c, 3.0, theme::PINK);
    }

    // Animated pulse travelling home → dx while we transmit toward the contact.
    if tx_active {
        if let (Some(hll), Some(dll)) = (home, dx) {
            let pts = great_circle_points(hll, dll, 128);
            let n = pts.len();
            if n >= 2 {
                let phase = (ui.input(|i| i.time) * 0.45).rem_euclid(1.0); // ~2.2s sweep
                let head = ((phase * (n - 1) as f64) as usize).min(n - 1);
                // Comet tail behind the head (toward home).
                for k in 1..=6usize {
                    if head >= k {
                        let (la, lo) = pts[head - k];
                        let a = 150u8.saturating_sub(k as u8 * 22);
                        p.circle_filled(
                            project(la, lo),
                            dot_r.max(1.2),
                            Color32::from_rgba_unmultiplied(120, 240, 255, a),
                        );
                    }
                }
                // Bright leading head with a glow.
                let (la, lo) = pts[head];
                let c = project(la, lo);
                p.circle_filled(c, dot_r + 4.0, Color32::from_rgba_unmultiplied(120, 240, 255, 70));
                p.circle_filled(c, 3.2, Color32::WHITE);
                // ~30 fps is plenty for the comet; an unconditional repaint
                // would drive the whole app at vsync rate during TX.
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
            }
        }
    }

    // Frame (red-accent, matching the QSO section panels).
    crate::chrome::paint_cut_border(&p, rect.shrink(0.5), theme::RED_DEEP, theme::BG_DEEP);
}
