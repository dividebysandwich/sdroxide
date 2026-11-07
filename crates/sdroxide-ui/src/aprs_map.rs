//! The APRS map: every station heard, drawn as the icon its symbol asks for.
//!
//! The projection, the pan/zoom and the dotted continents are the FT8 panel
//! map's ([`crate::widgets::worldmap`]) — the two have to agree about where
//! the world is, and one of them owning that is how they do. What is here is
//! everything APRS-specific:
//!
//! - an *icon* per station rather than a dot, from
//!   [`sdroxide_types::AprsSymbolKind`];
//! - a trail behind anything that has moved;
//! - a fade with age, over the same window the station list keeps
//!   ([`sdroxide_types::DigiConfig::aprs_station_ttl_min`]), so a stale
//!   position visibly *is* stale rather than looking like a live one;
//! - the ambiguity square, where a station reported one. A position given to
//!   the nearest ten minutes drawn as a point would be inventing precision the
//!   sender deliberately withheld.
//!
//! Clicking a station selects it, which is what the panel's detail card and
//! the message box's addressee follow.

use crate::i18n::tr;
use eframe::egui::{Align2, Color32, FontId, Rect, Sense, Ui, pos2, vec2};
use sdroxide_types::{AprsEntryKind, AprsPosition, AprsStation};

use crate::aprs_icons::AprsIcons;
use crate::theme;
use crate::widgets::worldmap::{MapView, alpha, draw_base, interact, wrap180};

/// Below this height the map is not worth drawing.
pub const MIN_HEIGHT: f32 = 90.0;

/// How close a click has to land, in points.
const HIT_RADIUS: f32 = 13.0;

/// An icon's drawn size, in points.
const ICON_PX: f32 = 17.0;

/// Never auto-fit tighter than this longitudinal span, so one station in the
/// next street does not blow the map up to house level.
const MIN_LON_SPAN: f64 = 0.6;

/// Extra margin left around the outermost station.
const PAD: f64 = 1.35;

/// Per-frame ease toward the auto-fit.
const EASE: f64 = 0.06;

/// The map's own state, owned by the panel so the view survives across frames.
#[derive(Default)]
pub struct AprsMapState {
    pub view: MapView,
    /// The station whose card is open, by name.
    pub selected: Option<String>,
}

/// How bright a station is drawn, 1 when just heard and fading to a floor over
/// the window it is kept for.
///
/// A floor rather than zero: a station still on the list has to be visible, and
/// the fade is there to say "this is where it *was*", not to hide it.
fn freshness(age_s: i64, ttl_s: i64) -> f32 {
    if ttl_s <= 0 {
        return 1.0;
    }
    let t = (age_s.max(0) as f32 / ttl_s as f32).clamp(0.0, 1.0);
    1.0 - 0.62 * t
}

/// The view to ease toward: everything heard, plus us, framed with a margin.
fn target_view(home: Option<AprsPosition>, pts: &[(f64, f64)], aspect: f64) -> (f64, f64, f64) {
    let all: Vec<(f64, f64)> =
        home.map(|h| (h.lat, h.lon)).into_iter().chain(pts.iter().copied()).collect();
    if all.is_empty() {
        return (20.0, 0.0, 360.0);
    }
    let (clat, clon) = match home {
        Some(h) => (h.lat, h.lon),
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

/// Draw the map. Returns the station clicked this frame, if any.
///
/// `ttl_min` is the window the station list keeps, which is what the fade is
/// measured against — set it short and the map visibly shows only what is live.
pub fn show(
    ui: &mut Ui,
    state: &mut AprsMapState,
    icons: &mut AprsIcons,
    stations: &[AprsStation],
    home: Option<AprsPosition>,
    now: i64,
    ttl_min: u32,
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

    // ── the view ──
    let aspect = (rect.height() / rect.width()) as f64;
    let pts: Vec<(f64, f64)> =
        stations.iter().filter_map(|s| s.pos).map(|q| (q.lat, q.lon)).collect();
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
    let manual = view.manual;

    let dot_r = draw_base(&p, rect, clat, clon, lon_span, lat_span, map);

    let project = |lat: f64, lon: f64| {
        let dlon = wrap180(lon - clon);
        pos2(
            rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
            rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
        )
    };

    let ttl_s = i64::from(ttl_min.max(1)) * 60;
    // Which station the pointer is over, decided before anything is drawn so
    // the hovered one can be drawn last and on top.
    let pointer = resp.hover_pos();
    let mut hover: Option<usize> = None;
    let mut best = HIT_RADIUS;
    for (i, s) in stations.iter().enumerate() {
        let Some(q) = s.pos else { continue };
        let c = project(q.lat, q.lon);
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

    // ── trails, under everything: they are where a station has been ──
    for s in stations {
        // One remembered point plus the current position is already a leg: a
        // station that has moved exactly once should show that it did.
        if s.track.is_empty() || s.pos.is_none() {
            continue;
        }
        let fresh = freshness(now - s.last_heard, ttl_s);
        // The current position closes the trail, so the last leg reaches the
        // icon rather than stopping one report short of it.
        let pts: Vec<_> = s
            .track
            .iter()
            .copied()
            .chain(s.pos.map(|q| (q.lat, q.lon)))
            .map(|(lat, lon)| project(lat, lon))
            .collect();
        for w in pts.windows(2) {
            p.line_segment([w[0], w[1]], (1.4, alpha(map.trail, 130.0 * fresh)));
        }
    }

    // ── stations ──
    //
    // Callsigns only when there are few enough for them to be readable. A
    // label per station on a busy channel is a solid block of text with a map
    // somewhere underneath it.
    let on_screen = stations
        .iter()
        .filter(|s| s.pos.is_some_and(|q| rect.contains(project(q.lat, q.lon))))
        .count();
    let label_all = on_screen <= 22;
    let font = FontId::proportional(9.5);

    let mut draw = |i: usize, s: &AprsStation, top: bool| {
        let Some(q) = s.pos else { return };
        let c = project(q.lat, q.lon);
        if !rect.contains(c) && !top {
            return;
        }
        let fresh = freshness(now - s.last_heard, ttl_s);
        // A killed object is not gone, it is cancelled: greyed rather than
        // dropped, because an object vanishing without trace looks like a
        // receiver problem.
        let tint = if s.killed {
            alpha(map.land, 150.0)
        } else if Some(&s.name) == state.selected.as_ref() {
            map.dx
        } else if top {
            map.hover
        } else if s.entry == AprsEntryKind::Object {
            alpha(map.preview, 255.0 * fresh)
        } else {
            alpha(map.station, 255.0 * fresh)
        };

        // The square a station with an ambiguous position could be anywhere
        // in, where it is big enough on screen to mean anything.
        if q.ambiguity > 0 {
            let span = sdroxide_aprs::ambiguity_span_deg(q.ambiguity);
            let half_lat = span / 2.0;
            let half_lon = half_lat / q.lat.to_radians().cos().abs().max(0.05);
            let a = project(q.lat - half_lat, q.lon - half_lon);
            let b = project(q.lat + half_lat, q.lon + half_lon);
            let r = Rect::from_two_pos(a, b);
            if r.width() > 6.0 {
                p.rect_stroke(r, 0.0, (1.0, alpha(tint, 90.0)), eframe::egui::StrokeKind::Middle);
            }
        }

        let size = if top { ICON_PX + 3.0 } else { ICON_PX };
        let ir = Rect::from_center_size(c, vec2(size, size));
        // A halo, so an icon over the dotted land stays readable.
        p.circle_filled(c, size * 0.62, alpha(map.sea, 170.0));
        icons.paint(ui, ir, s.symbol.kind(), tint);
        // The overlay character a station put over its symbol, which is how it
        // says which network or agency it belongs to.
        if let Some(o) = s.symbol.overlay() {
            p.text(
                c + vec2(size * 0.42, -size * 0.42),
                Align2::CENTER_CENTER,
                o,
                FontId::monospace(8.5),
                alpha(map.hover, 230.0 * fresh),
            );
        }
        if label_all || top || Some(&s.name) == state.selected.as_ref() {
            p.text(
                c + vec2(0.0, size * 0.62 + 1.0),
                Align2::CENTER_TOP,
                &s.name,
                font.clone(),
                alpha(tint, 235.0),
            );
        }
        let _ = i;
    };

    for (i, s) in stations.iter().enumerate() {
        if hover != Some(i) {
            draw(i, s, false);
        }
    }
    if let Some(i) = hover {
        draw(i, &stations[i], true);
    }

    // ── us ──
    if let Some(q) = home {
        let c = project(q.lat, q.lon);
        p.circle_filled(c, dot_r + 5.0, alpha(map.home, 55.0));
        p.circle_filled(c, 3.4, map.home);
        p.circle_stroke(c, 6.5, (1.2, alpha(map.home, 170.0)));
    }

    // ── interaction ──
    let mut clicked = None;
    if let Some(i) = hover {
        let s = &stations[i];
        let age = now - s.last_heard;
        let mut tip = format!("{}\n{}", s.name, s.symbol.kind().label());
        if s.entry == AprsEntryKind::Object {
            tip.push_str(&tr!("aprsmap-object", reported_by = s.reported_by));
        }
        if !s.comment.is_empty() {
            tip.push('\n');
            tip.push_str(&s.comment);
        }
        if let (Some(h), Some(q)) = (home, s.pos) {
            let km = sdroxide_types::distance_km((h.lat, h.lon), (q.lat, q.lon));
            tip.push_str(&format!("\n{km:.0} km"));
        }
        tip.push_str(&tr!("aprsmap-heard-ago", age = crate::app::util::fmt_age(age)));
        resp.clone().on_hover_text(tip);
        if resp.clicked() {
            clicked = Some(s.name.clone());
            state.selected = clicked.clone();
        }
    } else if resp.clicked() {
        state.selected = None;
    }

    if manual && resp.hovered() {
        p.text(
            rect.right_bottom() + vec2(-6.0, -4.0),
            Align2::RIGHT_BOTTOM,
            tr!("aprsmap-double-click-reframe"),
            FontId::proportional(9.0),
            alpha(Color32::WHITE, 110.0),
        );
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A station just heard is at full brightness; one at the end of its
    /// window is dim but still visible. Fading to nothing would hide a station
    /// that is still on the list.
    #[test]
    fn the_fade_never_reaches_invisible() {
        assert!((freshness(0, 3600) - 1.0).abs() < 1e-6);
        assert!(freshness(3600, 3600) > 0.3);
        assert!(freshness(7200, 3600) > 0.3, "past the window it is clamped, not negative");
    }

    /// One station in the next street must not zoom the map to street level.
    #[test]
    fn the_auto_fit_has_a_floor() {
        let home = AprsPosition { lat: 48.2, lon: 16.37, ambiguity: 0 };
        let (_, _, span) = target_view(Some(home), &[(48.2005, 16.3705)], 0.5);
        assert!(span >= MIN_LON_SPAN);
    }

    /// ...and with nothing heard at all it shows the world rather than the
    /// Gulf of Guinea.
    #[test]
    fn an_empty_map_shows_the_world() {
        let (_, _, span) = target_view(None, &[], 0.5);
        assert_eq!(span, 360.0);
    }
}
