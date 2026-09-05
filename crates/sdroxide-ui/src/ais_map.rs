//! The AIS chart: every vessel being tracked, drawn the way a marine traffic
//! display draws one.
//!
//! The projection, the pan/zoom and the dotted continents are the FT8 panel
//! map's ([`crate::widgets::worldmap`]) — the views have to agree about where
//! the world is, and one of them owning that is how they do. What is here is
//! everything a sea picture needs and an air picture does not:
//!
//! - a **ship-shaped symbol**, drawn to the vessel's heading. This is the one
//!   place this map differs from [`crate::adsb_map`] on purpose rather than by
//!   accident: everything overhead is an aeroplane, so ADS-B draws a square and
//!   says nothing about the kind — but what is on the water is a tanker, a
//!   ferry, a fishing boat, a buoy, a lighthouse or a lifeboat, and which it is
//!   changes what a watchkeeper does next. So a ship is a hull outline pointed
//!   the way its bow is, a base station is a square with a mast, an aid to
//!   navigation is a diamond, a SAR aircraft is an aircraft, and a distress
//!   beacon is a cross that nothing else can be mistaken for.
//! - **history dots** behind it — where it has been, fading with age. The
//!   spacing of the dots is the speed, read before any number on the screen is.
//! - a **vector** ahead of it, as long as the distance covered in
//!   [`sdroxide_types::AisSettings::vector_minutes`]. Six minutes is the
//!   default a chart plotter uses, not the one minute a radar uses, because a
//!   ship's world moves at a tenth of an aircraft's.
//! - a **label** beside it: the name, and the speed. Two short lines, not the
//!   three a radar data block carries — a ship's altitude is zero and its name
//!   is what everybody calls it by.
//!
//! # Why the symbol points at the heading and the vector at the course
//!
//! They are different facts and a display that conflated them would be lying
//! about the one thing this picture is for. A vessel crabbing across a tideway
//! *points* twenty degrees off the track it is *making*, and both matter: the
//! heading is where the bow is and the course is where it will be. So the hull
//! is drawn to [`sdroxide_types::AisVessel::icon_deg`] and the vector to the
//! course over ground, and on a windy day they visibly disagree.
//!
//! # Why a stale target is not drawn at all
//!
//! Past [`sdroxide_types::AisSettings::drop_map_s`] a target's position is old
//! news and it comes off the chart — symbol, dots and label together. It stays
//! on the list, greyed. Fading it instead was the obvious alternative and is
//! wrong, for the reason the ADS-B map's note gives: a faint symbol at a stale
//! position is still a claim about where a ship is, drawn in the same ink as
//! the ones that are true.
//!
//! That window is five minutes here rather than ADS-B's ten *seconds*, and the
//! difference is not taste — a vessel at anchor reports once every three
//! minutes, so a shorter one would blank most of a harbour between two
//! perfectly good reports.

use eframe::egui::{Align2, Color32, FontId, Pos2, Sense, Stroke, Ui, pos2, vec2};
use sdroxide_types::{AisKind, AisSettings, AisVessel};

use crate::theme;
use crate::widgets::worldmap::{MapView, alpha, draw_base, interact, wrap180};

/// Below this height the map is not worth drawing.
pub const MIN_HEIGHT: f32 = 90.0;

/// How close a click has to land, in points.
const HIT_RADIUS: f32 = 14.0;

/// Half the length of a vessel symbol, in points. The same at every zoom: a
/// scan across a busy estuary has to read as one class of thing, and a ship
/// drawn to scale would be invisible at any zoom that showed more than one.
const HULL_R: f32 = 5.5;

/// Never auto-fit tighter than this longitudinal span.
///
/// A tenth of ADS-B's, because ships are: a harbour is a couple of kilometres
/// across and the vessels in it are a few hundred metres apart, where aircraft
/// are hundreds of kilometres apart. About five kilometres of longitude at
/// these latitudes.
const MIN_LON_SPAN: f64 = 0.05;

/// Extra margin left around the outermost target.
const PAD: f64 = 1.25;

/// Per-frame ease toward the auto-fit.
const EASE: f64 = 0.06;

/// Above this many targets on screen, only the selected and hovered ones keep
/// their label. A busy approach channel is a wall of names with a chart
/// somewhere underneath it.
const LABEL_LIMIT: usize = 30;

/// One nautical mile in degrees of latitude. A minute of arc, by definition —
/// which is what makes converting knots into map degrees exact rather than
/// approximate.
const NM_PER_DEG_LAT: f64 = 60.0;

/// The map's own state, owned by the panel so the view survives across frames.
#[derive(Default)]
pub struct AisMapState {
    pub view: MapView,
    /// The vessel whose card is open, by MMSI.
    pub selected: Option<u32>,
}

/// The view to ease toward: every target that can be drawn, plus us, framed.
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

/// Where a vessel will be after `minutes`, as (lat, lon).
///
/// A straight line, unlike the ADS-B leader's curve. A ship's rate of turn is
/// broadcast rather than derived, but it is also tiny — a large vessel turns at
/// a degree or two a minute — so over a six-minute vector the arc and the chord
/// differ by less than the width of the symbol, and a curve drawn from it would
/// be claiming a precision the number does not have.
fn vector(v: &AisVessel, minutes: f32) -> Option<(f64, f64)> {
    let (lat, lon) = v.lat.zip(v.lon)?;
    let kt = v.sog_kt?;
    let course = v.cog_deg.or(v.heading_deg)?;
    if minutes <= 0.0 || kt <= 0.2 || !v.kind.is_underway() {
        return None;
    }
    // Knots are nautical miles an hour and a nautical mile is a minute of
    // latitude, so this needs no earth radius and no approximation.
    let step_deg = f64::from(kt) * (f64::from(minutes) / 60.0) / NM_PER_DEG_LAT;
    let r = f64::from(course).to_radians();
    let la = lat + step_deg * r.cos();
    // A degree of longitude shrinks with latitude; without this a vector at
    // 60 N is drawn half as long east-west as it should be.
    let lo = lon + step_deg * r.sin() / lat.to_radians().cos().abs().max(0.02);
    (la.is_finite() && lo.is_finite() && la.abs() < 89.5).then(|| (la, wrap180(lo)))
}

/// Rotate a point given in symbol space (x to starboard, y forward) into screen
/// space around `c`, for a symbol pointing `deg` degrees true.
fn rot(c: Pos2, deg: f32, x: f32, y: f32) -> Pos2 {
    // Screen y grows downwards and a compass bearing grows clockwise from
    // north, so "forward" is -y and the rotation is the ordinary one.
    let (s, k) = deg.to_radians().sin_cos();
    pos2(c.x + x * k + y * s, c.y + x * s - y * k)
}

/// Draw one station's symbol. `deg` is which way it is pointing, where that
/// means anything.
fn symbol(
    p: &eframe::egui::Painter,
    c: Pos2,
    kind: AisKind,
    deg: Option<f32>,
    r: f32,
    tint: Color32,
    sea: Color32,
) {
    // A halo, so the symbol stays readable over the dotted land.
    p.circle_filled(c, r + 2.0, alpha(sea, 175.0));
    let stroke = Stroke::new(1.4, tint);
    match kind {
        AisKind::BaseStation => {
            // A square with a mast: a shore station, which does not move.
            let h = r * 0.7;
            let pts = [
                pos2(c.x - h, c.y - h),
                pos2(c.x + h, c.y - h),
                pos2(c.x + h, c.y + h),
                pos2(c.x - h, c.y + h),
                pos2(c.x - h, c.y - h),
            ];
            for w in pts.windows(2) {
                p.line_segment([w[0], w[1]], stroke);
            }
            p.line_segment([pos2(c.x, c.y - h), pos2(c.x, c.y - r - 3.0)], stroke);
        }
        AisKind::AidToNavigation => {
            // A diamond, which is what a chart uses for a mark.
            let pts = [
                pos2(c.x, c.y - r),
                pos2(c.x + r * 0.72, c.y),
                pos2(c.x, c.y + r),
                pos2(c.x - r * 0.72, c.y),
                pos2(c.x, c.y - r),
            ];
            for w in pts.windows(2) {
                p.line_segment([w[0], w[1]], stroke);
            }
        }
        AisKind::Sart => {
            // A cross in a circle: nothing else on this chart looks like it,
            // which is the entire requirement.
            p.circle_stroke(c, r, stroke);
            p.line_segment([pos2(c.x - r, c.y), pos2(c.x + r, c.y)], stroke);
            p.line_segment([pos2(c.x, c.y - r), pos2(c.x, c.y + r)], stroke);
        }
        AisKind::SarAircraft => {
            // A fuselage and a wing, pointed the way it is flying.
            let d = deg.unwrap_or(0.0);
            p.line_segment([rot(c, d, 0.0, -r), rot(c, d, 0.0, r)], stroke);
            p.line_segment([rot(c, d, -r * 0.8, 0.0), rot(c, d, r * 0.8, 0.0)], stroke);
            p.line_segment([rot(c, d, -r * 0.4, -r * 0.8), rot(c, d, r * 0.4, -r * 0.8)], stroke);
        }
        _ => {
            // A hull: pointed bow, parallel sides, square transom. Drawn to the
            // heading where the vessel reports one; with no heading at all
            // there is no bow to point, so it becomes a circle rather than a
            // ship arbitrarily facing north.
            let Some(d) = deg else {
                p.circle_stroke(c, r * 0.6, stroke);
                return;
            };
            let w = r * 0.5;
            let pts = [
                rot(c, d, 0.0, r),       // bow
                rot(c, d, w, r * 0.25),  // starboard shoulder
                rot(c, d, w, -r),        // starboard quarter
                rot(c, d, -w, -r),       // port quarter
                rot(c, d, -w, r * 0.25), // port shoulder
                rot(c, d, 0.0, r),
            ];
            for pair in pts.windows(2) {
                p.line_segment([pair[0], pair[1]], stroke);
            }
        }
    }
}

/// Draw the map. Returns the vessel clicked this frame, if any.
pub fn show(
    ui: &mut Ui,
    state: &mut AisMapState,
    vessels: &[AisVessel],
    home: Option<(f64, f64)>,
    now: i64,
    cfg: AisSettings,
    max_h: f32,
) -> Option<u32> {
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

    // Only the targets with a position fresh enough to draw. Everything below
    // works from this list, so a stale vessel cannot pull the view toward where
    // it used to be either.
    let live: Vec<&AisVessel> =
        vessels.iter().filter(|v| !v.pos_stale(now, cfg.drop_map_s)).collect();

    // ── the view ──
    let aspect = (rect.height() / rect.width()) as f64;
    let pts: Vec<(f64, f64)> = live.iter().filter_map(|v| v.lat.zip(v.lon)).collect();
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
        let settled = (view.clat - t_clat).abs() < 0.002
            && wrap180(t_clon - view.clon).abs() < 0.002
            && (view.lon_span - t_span).abs() < 0.002;
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

    // Which target the pointer is over, decided before anything is drawn so the
    // hovered one can be drawn last and on top.
    let pointer = resp.hover_pos();
    let mut hover: Option<usize> = None;
    let mut best = HIT_RADIUS;
    for (i, v) in live.iter().enumerate() {
        let Some((lat, lon)) = v.lat.zip(v.lon) else { continue };
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

    // ── history, under everything ──
    for v in &live {
        let n = v.track.len();
        for (k, &(plat, plon)) in v.track.iter().enumerate() {
            let c = project(f64::from(plat), f64::from(plon));
            if !rect.contains(c) {
                continue;
            }
            // Oldest faintest, so the trail reads as a direction without a
            // single arrowhead being drawn.
            let age = (k + 1) as f32 / n as f32;
            p.circle_filled(c, 1.3, alpha(map.trail, 35.0 + 150.0 * age));
        }
    }

    let on_screen = live
        .iter()
        .filter(|v| v.lat.zip(v.lon).is_some_and(|(la, lo)| rect.contains(project(la, lo))))
        .count();
    let label_all = on_screen <= LABEL_LIMIT;
    let font = FontId::monospace(9.0);

    let draw = |v: &AisVessel, top: bool| {
        let Some((lat, lon)) = v.lat.zip(v.lon) else { return };
        let c = project(lat, lon);
        if !rect.contains(c) && !top {
            return;
        }
        let selected = state.selected == Some(v.mmsi);
        // A distress beacon and the row the operator picked get the same
        // attention colour, because both mean "look here" and a chart with two
        // of those is a chart with none.
        let tint = if v.is_alarm() || selected {
            map.dx
        } else if top {
            map.hover
        } else if v.kind == AisKind::ClassA {
            map.station
        } else {
            // Everything that is not a SOLAS ship — Class B, buoys, base
            // stations — is dimmer, so a busy marina does not out-shout the
            // traffic in the channel next to it.
            alpha(map.station, 175.0)
        };

        // The vector, under the symbol: where it is now matters more than where
        // it will be.
        if let Some((la, lo)) = vector(v, cfg.vector_minutes) {
            p.line_segment([c, project(la, lo)], (1.2, alpha(tint, 185.0)));
        }

        let r = if top || selected { HULL_R + 1.0 } else { HULL_R };
        symbol(&p, c, v.kind, v.icon_deg(), r, tint, map.sea);

        // The label, up and to the right, with a tick joining it to the symbol
        // so a crowded picture still says which label belongs to which.
        if label_all || top || selected {
            let anchor = c + vec2(r + 3.0, -(r + 2.0));
            p.line_segment([c + vec2(r * 0.6, -r * 0.6), anchor], (1.0, alpha(tint, 110.0)));
            // The speed only where there is one to have. A buoy labelled
            // "--- kt" is two characters of information and a line of noise on
            // a chart that may carry a hundred of them.
            let mut lines = vec![v.label()];
            if let Some(kt) = v.sog_kt.filter(|_| v.kind.is_underway()) {
                lines.push(format!("{kt:.1} kt"));
            }
            for (k, line) in lines.iter().enumerate() {
                p.text(
                    anchor + vec2(2.0, -((lines.len() - 1 - k) as f32) * 10.0 - 5.0),
                    Align2::LEFT_CENTER,
                    line,
                    font.clone(),
                    alpha(tint, if k == 0 { 240.0 } else { 185.0 }),
                );
            }
        }
    };

    for (i, v) in live.iter().enumerate() {
        if hover != Some(i) {
            draw(v, false);
        }
    }
    if let Some(i) = hover {
        draw(live[i], true);
    }

    // ── us ──
    if let Some((lat, lon)) = home {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 5.0, alpha(map.home, 55.0));
        p.circle_filled(c, 3.4, map.home);
        p.circle_stroke(c, 6.5, (1.2, alpha(map.home, 170.0)));
    }

    // ── interaction ──
    let mut clicked = None;
    if let Some(i) = hover {
        let v = live[i];
        let mut tip = v.label();
        if !v.name.is_empty() {
            tip.push_str(&format!("  {}", v.mmsi));
        }
        tip.push_str(&format!("\n{}", v.kind.label()));
        if let Some(t) = v.type_label() {
            tip.push_str(&format!(" — {t}"));
        }
        if let Some(s) = v.nav_status.map(sdroxide_types::nav_status_label) {
            tip.push_str(&format!("\n{s}"));
        }
        tip.push_str(&format!("\n{} kt   {}°", v.fmt_speed(), v.fmt_course()));
        let size = v.fmt_size();
        if !size.is_empty() {
            tip.push_str(&format!("\n{size}"));
        }
        if let (Some(h), Some((lat, lon))) = (home, v.lat.zip(v.lon)) {
            let km = sdroxide_types::distance_km(h, (lat, lon));
            let bear = sdroxide_types::bearing_deg(h, (lat, lon));
            tip.push_str(&format!("\n{km:.1} km   {bear:.0}°"));
        }
        resp.clone().on_hover_text(tip);
        if resp.clicked() {
            clicked = Some(v.mmsi);
            state.selected = clicked;
        }
    } else if resp.clicked() {
        state.selected = None;
    }

    // ── why the chart is empty ──
    //
    // A list with rows in it and a chart with nothing on it reads as a broken
    // map, and that is how issue #330 was reported. It usually is not: a
    // station has to send a position report before it can be drawn, and until
    // it does there is genuinely nowhere to put it. Saying so is the difference
    // between a display that looks faulty and one that is telling the operator
    // to keep listening.
    if live.is_empty() && !vessels.is_empty() {
        let heard = vessels.len();
        let note = if vessels.iter().any(|v| v.lat.is_some() && v.lon.is_some()) {
            format!("{heard} heard — no position newer than {}s", cfg.drop_map_s)
        } else if heard == 1 {
            "1 station heard — waiting for it to report a position".to_string()
        } else {
            format!("{heard} stations heard — waiting for a position report")
        };
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            note,
            FontId::proportional(11.0),
            alpha(Color32::WHITE, 130.0),
        );
    }

    if manual && resp.hovered() {
        p.text(
            rect.right_bottom() + vec2(-6.0, -4.0),
            Align2::RIGHT_BOTTOM,
            "double-click to reframe",
            FontId::proportional(9.0),
            alpha(Color32::WHITE, 110.0),
        );
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn under_way(kt: f32, course: f32) -> AisVessel {
        let mut v = AisVessel::new(244_660_000, 0);
        v.lat = Some(52.0);
        v.lon = Some(4.0);
        v.sog_kt = Some(kt);
        v.cog_deg = Some(course);
        v
    }

    /// The vector's length is the distance covered in the vector time, in
    /// degrees of latitude — which is what makes two vessels with equal vectors
    /// equally fast at any zoom.
    #[test]
    fn the_vector_is_as_long_as_the_run_it_stands_for() {
        // 12 knots due north for six minutes is 1.2 NM, which is 1.2 minutes of
        // latitude: 0.02 degrees.
        let (lat, lon) = vector(&under_way(12.0, 0.0), 6.0).expect("a vector");
        assert!((lat - 52.0 - 0.02).abs() < 1e-9, "reached {lat}");
        assert!((lon - 4.0).abs() < 1e-9, "due north should not move east: {lon}");
        // Half the speed, half the line.
        let (half, _) = vector(&under_way(6.0, 0.0), 6.0).expect("a vector");
        assert!(((half - 52.0) - (lat - 52.0) / 2.0).abs() < 1e-9);
    }

    /// A degree of longitude is shorter than a degree of latitude everywhere
    /// but the equator, so an eastbound vector has to be drawn wider in degrees
    /// or it comes out short on the chart.
    #[test]
    fn an_eastbound_vector_is_stretched_for_the_latitude_it_is_at() {
        let (lat, lon) = vector(&under_way(12.0, 90.0), 6.0).expect("a vector");
        assert!((lat - 52.0).abs() < 1e-9, "due east should not move north: {lat}");
        let want = 0.02 / 52.0f64.to_radians().cos();
        assert!((lon - 4.0 - want).abs() < 1e-6, "reached {lon}, wanted {}", 4.0 + want);
    }

    /// Things that do not move get no vector, whatever else they report. A buoy
    /// with a spurious course would otherwise be drawn sailing off station.
    #[test]
    fn a_mark_gets_no_vector() {
        let mut buoy = under_way(3.0, 90.0);
        buoy.kind = AisKind::AidToNavigation;
        assert!(vector(&buoy, 6.0).is_none());
        let mut base = under_way(3.0, 90.0);
        base.kind = AisKind::BaseStation;
        assert!(vector(&base, 6.0).is_none());
        // ...and neither does a ship alongside, or one that has said no speed.
        assert!(vector(&under_way(0.0, 90.0), 6.0).is_none());
        let mut v = under_way(10.0, 90.0);
        v.sog_kt = None;
        assert!(vector(&v, 6.0).is_none());
        // And the operator can switch vectors off altogether.
        assert!(vector(&under_way(10.0, 90.0), 0.0).is_none());
    }

    /// The symbol rotation is a compass bearing on a screen whose y grows
    /// downwards, which is the one place a sign gets flipped: a vessel heading
    /// north has its bow *above* its centre, and one heading east to the right
    /// of it.
    #[test]
    fn the_bow_points_where_the_compass_says() {
        let c = pos2(100.0, 100.0);
        let bow = rot(c, 0.0, 0.0, 10.0);
        assert!((bow.x - 100.0).abs() < 1e-4 && bow.y < 95.0, "north: {bow:?}");
        let bow = rot(c, 90.0, 0.0, 10.0);
        assert!(bow.x > 105.0 && (bow.y - 100.0).abs() < 1e-4, "east: {bow:?}");
        let bow = rot(c, 180.0, 0.0, 10.0);
        assert!((bow.x - 100.0).abs() < 1e-4 && bow.y > 105.0, "south: {bow:?}");
        // Starboard is to the right of the bow, whichever way it is pointing.
        let stbd = rot(c, 0.0, 10.0, 0.0);
        assert!(stbd.x > 105.0 && (stbd.y - 100.0).abs() < 1e-4, "starboard: {stbd:?}");
    }

    /// One vessel alongside must not zoom the chart past the point where the
    /// harbour around it is visible, and an empty sea shows the world rather
    /// than the Gulf of Guinea.
    #[test]
    fn the_auto_fit_has_a_floor_and_an_empty_default() {
        let (_, _, span) = target_view(Some((52.37, 4.89)), &[(52.3701, 4.8901)], 0.5);
        assert!(span >= MIN_LON_SPAN);
        let (_, _, span) = target_view(None, &[], 0.5);
        assert_eq!(span, 360.0);
        // ...and the floor is tight enough to frame a *harbour* rather than a
        // sector, which is where it differs from the air picture's half a
        // degree: two vessels two hundred metres apart have to fill a usable
        // part of the chart rather than sharing one pixel in the middle of it.
        let (_, _, span) = target_view(None, &[(51.9500, 4.0500), (51.9518, 4.0500)], 0.5);
        assert!(span <= 0.06, "two ships 200 m apart framed to {span}°");
    }
}
