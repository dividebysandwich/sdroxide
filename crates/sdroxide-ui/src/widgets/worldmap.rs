//! A small pixel/dot-matrix world map for the FT8 QSO panel: renders the
//! continents as glowing dots and marks the home + DX locations with a
//! great-circle path between them, cyberpunk style.
//!
//! The map is centred on the operator's home grid (the world wraps around it
//! rather than being shifted) and smoothly auto-zooms to frame home plus every
//! decoded station, re-fitting whenever new stations appear.

use eframe::egui::{Color32, Pos2, Rect, Sense, Ui, pos2, vec2};
use sdroxide_types::{great_circle_points, land_cell, land_mask_dims};

use crate::theme;

/// Below this height the map is not worth drawing — the caller should omit it
/// entirely so the QSO controls keep the space.
pub const MIN_HEIGHT: f32 = 72.0;

/// Never zoom tighter than this longitudinal span (degrees), so a single nearby
/// contact doesn't blow the map up to street level.
const MIN_LON_SPAN: f64 = 30.0;
/// Fraction of extra margin left around the outermost contact.
const PAD: f64 = 1.4;
/// Per-frame ease toward the target view (0..1); smaller = slower/smoother.
const EASE: f64 = 0.0375;

/// Persistent, animated view state (centre + longitudinal span, in degrees).
/// Owned by the caller so the zoom eases across frames.
pub struct MapView {
    clat: f64,
    clon: f64,
    lon_span: f64,
    initialized: bool,
}

impl Default for MapView {
    fn default() -> Self {
        MapView { clat: 20.0, clon: 0.0, lon_span: 360.0, initialized: false }
    }
}

/// Wrap a longitude delta into [-180, 180).
fn wrap180(mut d: f64) -> f64 {
    d = (d + 180.0).rem_euclid(360.0) - 180.0;
    d
}

/// Centre of a set of points, unwrapping longitude around the first point.
fn centroid(pts: &[(f64, f64)]) -> Option<(f64, f64)> {
    if pts.is_empty() {
        return None;
    }
    let n = pts.len() as f64;
    let lat = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let lon_ref = pts[0].1;
    let dlon = pts.iter().map(|p| wrap180(p.1 - lon_ref)).sum::<f64>() / n;
    Some((lat, wrap180(lon_ref + dlon)))
}

/// The view to ease toward: centred on home (else the contacts' centroid),
/// zoomed symmetrically to frame home plus every contact.
fn target_view(
    home: Option<(f64, f64)>,
    contacts: &[(f64, f64)],
    aspect: f64,
) -> (f64, f64, f64) {
    let (clat, clon) = home.or_else(|| centroid(contacts)).unwrap_or((20.0, 0.0));
    if contacts.is_empty() {
        // Nothing to frame yet: whole world, centred on home.
        return (0.0, clon, 360.0);
    }
    let mut max_dlat = 0.0f64;
    let mut max_dlon = 0.0f64;
    for &(lat, lon) in contacts {
        max_dlat = max_dlat.max((lat - clat).abs());
        max_dlon = max_dlon.max(wrap180(lon - clon).abs());
    }
    let need_lon = 2.0 * max_dlon * PAD;
    let need_lat = 2.0 * max_dlat * PAD;
    // Fit both dimensions under the map's aspect (lat_span = lon_span * aspect).
    let lon_span = need_lon.max(need_lat / aspect.max(1e-3)).clamp(MIN_LON_SPAN, 360.0);
    let lat_span = (lon_span * aspect).min(180.0);
    // Keep the latitude window inside the poles (avoids empty polar space).
    let clat = if lat_span >= 180.0 {
        0.0
    } else {
        clat.clamp(-90.0 + lat_span / 2.0, 90.0 - lat_span / 2.0)
    };
    (clat, clon, lon_span)
}

/// Draw the map filling the available width (2:1 aspect). `view` carries the
/// animated centre/zoom across frames. `home`/`dx`/`preview` are (lat, lon) in
/// degrees. `stations` are (lat, lon, alpha) for every decoded station still on
/// the map — drawn as white dots (alpha fades them out over time) under the
/// coloured markers, and used to drive the auto-zoom. `preview` is a faint
/// marker for a decode the user clicked but hasn't answered yet (distinct colour
/// from the active DX). When `tx_active`, an animated pulse travels the home→dx
/// path. `max_h` caps the height: on short windows the map shrinks (keeping its
/// aspect, centered) rather than pushing the QSO controls off-screen.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut Ui,
    view: &mut MapView,
    home: Option<(f64, f64)>,
    dx: Option<(f64, f64)>,
    preview: Option<(f64, f64)>,
    stations: &[(f64, f64, f32)],
    tx_active: bool,
    max_h: f32,
) {
    let avail_w = ui.available_width();
    // Natural size is 2:1 by width; fit it within the caller's vertical budget,
    // preserving aspect so it just gets smaller and centered (never squished).
    let h = (avail_w * 0.5).clamp(90.0, 300.0).min(max_h);
    if h < MIN_HEIGHT {
        return;
    }
    let w = (h * 2.0).min(avail_w);
    let (row, _) = ui.allocate_exact_size(vec2(avail_w, h), Sense::hover());
    let rect = Rect::from_center_size(row.center(), vec2(w, h));
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, theme::INPUT_BG);

    // ── Ease the view toward the target (fit home + all contacts) ──
    let aspect = (rect.height() / rect.width()) as f64;
    let mut contacts: Vec<(f64, f64)> = stations.iter().map(|&(lat, lon, _)| (lat, lon)).collect();
    if let Some(c) = dx {
        contacts.push(c);
    }
    if let Some(c) = preview {
        contacts.push(c);
    }
    let (t_clat, t_clon, t_span) = target_view(home, &contacts, aspect);
    if !view.initialized {
        view.clat = t_clat;
        view.clon = t_clon;
        view.lon_span = t_span;
        view.initialized = true;
    } else {
        view.clat += (t_clat - view.clat) * EASE;
        view.clon = wrap180(view.clon + wrap180(t_clon - view.clon) * EASE);
        view.lon_span += (t_span - view.lon_span) * EASE;
    }
    let settled = (view.clat - t_clat).abs() < 0.05
        && wrap180(t_clon - view.clon).abs() < 0.05
        && (view.lon_span - t_span).abs() < 0.05;
    if !settled {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }
    let (clat, clon, lon_span) = (view.clat, view.clon, view.lon_span);
    let lat_span = lon_span * aspect;

    // Render a dot grid sized to the available pixels (about one dot every
    // ~4 px), sampling the high-res land bitmap for crisp coastlines. Each cell
    // maps to a (lat, lon) in the current view; longitude wraps around home.
    let (mw, mh) = land_mask_dims();
    let cols = ((rect.width() / 4.0) as usize).clamp(80, mw);
    let rows = ((rect.height() / 4.0) as usize).clamp(40, mh);
    let cell_w = rect.width() / cols as f32;
    let cell_h = rect.height() / rows as f32;
    let dot_r = (cell_w.min(cell_h) * 0.44).max(0.7);
    let land = Color32::from_rgb(0x1c, 0x44, 0x58);

    for row in 0..rows {
        let fy = (row as f64 + 0.5) / rows as f64; // 0 top .. 1 bottom
        let lat = clat + (0.5 - fy) * lat_span;
        if !(-90.0..=90.0).contains(&lat) {
            continue; // beyond a pole → open space, no land
        }
        let mrow = (((90.0 - lat) / 180.0 * mh as f64) as usize).min(mh - 1);
        for col in 0..cols {
            let fx = (col as f64 + 0.5) / cols as f64; // 0 left .. 1 right
            let lonw = wrap180(clon + (fx - 0.5) * lon_span);
            let mcol = ((lonw + 180.0) / 360.0 * mw as f64) as usize % mw;
            if land_cell(mcol, mrow) {
                let x = rect.left() + (col as f32 + 0.5) * cell_w;
                let y = rect.top() + (row as f32 + 0.5) * cell_h;
                p.circle_filled(pos2(x, y), dot_r, land);
            }
        }
    }

    // Project (lat, lon) to screen using the current view; longitude wraps.
    let project = |lat: f64, lon: f64| -> Pos2 {
        let dlon = wrap180(lon - clon);
        let x = rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width();
        let y = rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height();
        pos2(x, y)
    };

    // Every decoded station with a known grid, as small white dots that fade
    // with age (`alpha`). The active DX (pink), the clicked preview (amber) and
    // home (green) are painted over these below, so a selected/answered station
    // keeps its own colour.
    for &(lat, lon, alpha) in stations {
        if alpha <= 0.0 {
            continue;
        }
        let c = project(lat, lon);
        let halo = (55.0 * alpha) as u8;
        let core = (255.0 * alpha) as u8;
        p.circle_filled(c, 2.6, Color32::from_rgba_unmultiplied(255, 255, 255, halo));
        p.circle_filled(c, 1.7, Color32::from_rgba_unmultiplied(255, 255, 255, core));
    }
    // Keep the slow fade progressing even after the zoom has settled.
    if !stations.is_empty() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
    }

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
