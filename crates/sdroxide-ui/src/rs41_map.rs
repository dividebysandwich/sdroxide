//! The RS41 radiosonde map: every sonde with a fresh position, drawn on the
//! shared worldmap the ADS-B / AIS panels use.

use eframe::egui::{Color32, FontId, Rect, Sense, Ui, pos2, vec2};
use sdroxide_types::{Rs41Settings, Rs41Sonde};

use crate::theme;
use crate::widgets::map_labels::{self, MapLabel};
use crate::widgets::worldmap::{MapView, alpha, draw_base, interact, wrap180};

pub const MIN_HEIGHT: f32 = 90.0;
const HIT_RADIUS: f32 = 14.0;
const TARGET_R: f32 = 3.5;
const MIN_LON_SPAN: f64 = 0.5;
const PAD: f64 = 1.25;
const EASE: f64 = 0.06;

#[derive(Default)]
pub struct Rs41MapState {
    pub view: MapView,
    /// Selected sonde serial.
    pub selected: Option<String>,
}

fn target_view(home: Option<(f64, f64)>, pts: &[(f64, f64)], aspect: f64) -> (f64, f64, f64) {
    let all: Vec<(f64, f64)> = home.into_iter().chain(pts.iter().copied()).collect();
    if all.is_empty() {
        return (20.0, 0.0, 360.0);
    }
    let (clat, clon) = match home {
        Some(h) => h,
        None => {
            let n = all.len() as f64;
            let lon_ref = all[0].1;
            (
                all.iter().map(|p| p.0).sum::<f64>() / n,
                wrap180(lon_ref + all.iter().map(|p| wrap180(p.1 - lon_ref)).sum::<f64>() / n),
            )
        }
    };
    let mut max_dlat = 0.0f64;
    let mut max_dlon = 0.0f64;
    for &(lat, lon) in &all {
        max_dlat = max_dlat.max((lat - clat).abs());
        max_dlon = max_dlon.max(wrap180(lon - clon).abs());
    }
    let need_lon = 2.0 * max_dlon * PAD;
    let need_lat = 2.0 * max_dlat * PAD;
    let lon_span = need_lon.max(need_lat / aspect.max(1e-3)).clamp(MIN_LON_SPAN, 360.0);
    let lat_span = (lon_span * aspect).min(180.0);
    let clat = if lat_span >= 180.0 {
        0.0
    } else {
        clat.clamp(-90.0 + lat_span / 2.0, 90.0 - lat_span / 2.0)
    };
    (clat, clon, lon_span)
}

/// Draw the map. Returns the serial clicked this frame, if any.
pub fn show(
    ui: &mut Ui,
    state: &mut Rs41MapState,
    sondes: &[Rs41Sonde],
    home: Option<(f64, f64)>,
    now: i64,
    cfg: Rs41Settings,
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
    let map = theme::map();
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, map.sea);

    let live: Vec<&Rs41Sonde> =
        sondes.iter().filter(|s| !s.pos_stale(now, cfg.drop_map_s)).collect();

    let aspect = (rect.height() / rect.width()) as f64;
    let pts: Vec<(f64, f64)> = live.iter().filter_map(|s| s.lat.zip(s.lon)).collect();
    let (t_clat, t_clon, t_span) = target_view(home, &pts, aspect);
    let view = &mut state.view;
    if !view.initialized {
        view.clat = t_clat;
        view.clon = t_clon;
        view.lon_span = t_span;
        view.initialized = true;
    } else if view.manual {
        view.clamp(aspect);
    } else {
        view.clat += (t_clat - view.clat) * EASE;
        view.clon = wrap180(view.clon + wrap180(t_clon - view.clon) * EASE);
        view.lon_span += (t_span - view.lon_span) * EASE;
        let settled = (view.clat - t_clat).abs() < 0.02
            && wrap180(t_clon - view.clon).abs() < 0.02
            && (view.lon_span - t_span).abs() < 0.02;
        if !settled {
            crate::repaint::after_ms(ui.ctx(), 16);
        }
    }
    if interact(ui, view, &resp, aspect) {
        crate::repaint::animate(ui.ctx());
    }
    let (clat, clon, lon_span) = (view.clat, view.clon, view.lon_span);
    let lat_span = lon_span * aspect;

    let _ = draw_base(&p, rect, clat, clon, lon_span, lat_span, map);

    let project = |lat: f64, lon: f64| {
        let dlon = wrap180(lon - clon);
        pos2(
            rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
            rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
        )
    };

    let pointer = resp.hover_pos();
    let mut hover: Option<usize> = None;
    let mut best = HIT_RADIUS;
    for (i, s) in live.iter().enumerate() {
        let Some((lat, lon)) = s.lat.zip(s.lon) else { continue };
        let c = project(lat, lon);
        if !rect.contains(c) {
            continue;
        }
        if let Some(m) = pointer {
            let d = c.distance(m);
            if d <= best {
                best = d;
                hover = Some(i);
            }
        }
    }

    for s in &live {
        let n = s.track.len().max(1);
        for (k, &(plat, plon)) in s.track.iter().enumerate() {
            let c = project(f64::from(plat), f64::from(plon));
            if !rect.contains(c) {
                continue;
            }
            let age = (k + 1) as f32 / n as f32;
            p.circle_filled(c, 1.3, alpha(map.trail, 40.0 + 150.0 * age));
        }
    }

    let font = FontId::monospace(9.0);
    let selected = state.selected.clone();
    let mut labels: Vec<MapLabel> = Vec::new();

    for (i, s) in live.iter().enumerate() {
        let Some((lat, lon)) = s.lat.zip(s.lon) else { continue };
        let c = project(lat, lon);
        if !rect.contains(c) {
            continue;
        }
        let top = hover == Some(i);
        let sel = selected.as_deref() == Some(s.serial.as_str());
        let tint = if sel || top { map.dx } else { map.station };
        let r = if top || sel { TARGET_R + 1.0 } else { TARGET_R };
        p.circle_filled(c, r + 1.5, alpha(map.sea, 200.0));
        p.circle_stroke(c, r, (1.5, tint));
        let alt = s.altitude_m.map(|a| format!("{a:.0} m")).unwrap_or_else(|| "——".into());
        let key = {
            let mut h = 0u32;
            for b in s.serial.bytes() {
                h = h.wrapping_mul(31).wrapping_add(u32::from(b));
            }
            h
        };
        labels.push(MapLabel {
            at: c,
            r,
            tick_from: 0.6,
            tick: alpha(tint, 110.0),
            lines: vec![(s.serial.clone(), tint), (alt, Color32::from_gray(180))],
            must: top || sel,
            rank: 1,
            key,
        });
    }

    let marks: Vec<Rect> = home
        .map(|(lat, lon)| Rect::from_center_size(project(lat, lon), vec2(16.0, 16.0)))
        .into_iter()
        .collect();
    map_labels::draw(&p, rect, &font, labels, &marks);

    if let Some((lat, lon)) = home {
        let c = project(lat, lon);
        if rect.contains(c) {
            p.circle_filled(c, 3.0, map.home);
        }
    }

    if resp.clicked() {
        if let Some(i) = hover {
            return Some(live[i].serial.clone());
        }
    }
    None
}
