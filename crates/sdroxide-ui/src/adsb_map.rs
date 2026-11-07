//! The ADS-B radar picture: every aircraft being tracked, drawn the way a
//! surveillance display draws one.
//!
//! The projection, the pan/zoom and the dotted continents are the FT8 panel
//! map's ([`crate::widgets::worldmap`]) — the views have to agree about where
//! the world is, and one of them owning that is how they do. What is here is
//! everything an air picture needs and a station map does not:
//!
//! - a **square** per target, not an icon. Everything up there is an aeroplane,
//!   so a symbol that said which kind would be saying nothing; what the square
//!   does is stay the same size and shape at every zoom, so a scan across a
//!   busy sector reads as one class of thing.
//! - **history dots** behind it — where it has been, fading with age. A
//!   controller reads speed and turn off the spacing of those dots before
//!   reading any number on the screen.
//! - a **leader line** ahead of it, as long as the distance it covers in
//!   [`sdroxide_types::AdsbSettings::vector_minutes`]. The length *is* the
//!   speed, which is why it is drawn in degrees of latitude and longitude
//!   before being projected: at a glance two aircraft with the same leader are
//!   going the same speed whatever the zoom. A turning aircraft gets a curved
//!   one, bent by the turn rate the tracker derives.
//! - a **data block** beside it: callsign, altitude, speed. Three short lines,
//!   the order every radar display in the world puts them in.
//!
//! # Why a stale target is not drawn at all
//!
//! Past [`sdroxide_types::AdsbSettings::drop_map_s`] a target's position is old
//! news, and it comes off the map — symbol, dots and block together. It stays
//! on the list, greyed. Fading it instead was the obvious alternative and is
//! wrong: a faint square at a stale position is still a claim about where an
//! aeroplane is, drawn in the same ink as the ones that are true, and the map
//! is the one place with no room for a hedge.

use crate::i18n::tr;
use eframe::egui::{Align2, Color32, FontId, Rect, Sense, Ui, pos2, vec2};
use sdroxide_types::{AdsbAircraft, AdsbSettings};

use crate::theme;
use crate::widgets::map_labels::{self, MapLabel};
use crate::widgets::worldmap::{MapView, alpha, draw_base, interact, wrap180};

/// Below this height the map is not worth drawing.
pub const MIN_HEIGHT: f32 = 90.0;

/// How close a click has to land, in points.
const HIT_RADIUS: f32 = 14.0;

/// Half the side of a target square, in points.
const TARGET_R: f32 = 3.5;

/// Never auto-fit tighter than this longitudinal span. Aircraft are hundreds of
/// kilometres apart and one overhead must not blow the map up to street level.
const MIN_LON_SPAN: f64 = 0.5;

/// Extra margin left around the outermost target.
const PAD: f64 = 1.25;

/// Per-frame ease toward the auto-fit.
const EASE: f64 = 0.06;

/// One nautical mile in degrees of latitude. A minute of arc, by definition —
/// which is what makes converting a speed in knots into map degrees exact
/// rather than approximate.
const NM_PER_DEG_LAT: f64 = 60.0;

/// The map's own state, owned by the panel so the view survives across frames.
#[derive(Default)]
pub struct AdsbMapState {
    pub view: MapView,
    /// The aircraft whose card is open, by ICAO address.
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

/// Where a target will be after `minutes`, as a list of (lat, lon) waypoints
/// including where it is now.
///
/// Straight when it is flying straight, and bent by its turn rate when it is
/// not, which is what makes the leader on a turning aircraft point at where it
/// is going rather than where it was going. Stepped rather than solved because
/// the turn rate is itself an estimate: five short chords are as much shape as
/// the data supports.
fn leader(ac: &AdsbAircraft, minutes: f32) -> Vec<(f64, f64)> {
    let (Some(lat), Some(lon)) = (ac.lat, ac.lon) else { return Vec::new() };
    let (Some(kt), Some(track)) = (ac.ground_speed_kt, ac.track_deg) else { return Vec::new() };
    if minutes <= 0.0 || kt <= 1.0 {
        return Vec::new();
    }
    const STEPS: usize = 5;
    let secs = f64::from(minutes) * 60.0 / STEPS as f64;
    // Knots are nautical miles per hour and a nautical mile is a minute of
    // latitude, so this needs no earth radius and no approximation.
    let step_deg = f64::from(kt) * (secs / 3600.0) / NM_PER_DEG_LAT;
    let turn = f64::from(ac.turn_rate_deg_s) * secs;

    let mut out = Vec::with_capacity(STEPS + 1);
    out.push((lat, lon));
    let (mut la, mut lo, mut hdg) = (lat, lon, f64::from(track));
    for _ in 0..STEPS {
        let r = hdg.to_radians();
        la += step_deg * r.cos();
        // A degree of longitude shrinks with latitude; without this a leader at
        // 60 N is drawn half as long east-west as it should be.
        lo += step_deg * r.sin() / la.to_radians().cos().abs().max(0.02);
        if !la.is_finite() || !lo.is_finite() || la.abs() > 89.0 {
            break;
        }
        out.push((la, wrap180(lo)));
        hdg += turn;
    }
    out
}

/// How hard an aircraft's data block fights for room on a busy map: lower wins.
///
/// An emergency first, then anything flying, then anything on the ground — an
/// apron of parked aircraft must not crowd the approach off the picture.
fn block_rank(a: &AdsbAircraft) -> u8 {
    if a.emergency.is_some() {
        0
    } else if !a.on_ground {
        1
    } else {
        2
    }
}

/// Draw the map. Returns the aircraft clicked this frame, if any.
pub fn show(
    ui: &mut Ui,
    state: &mut AdsbMapState,
    aircraft: &[AdsbAircraft],
    home: Option<(f64, f64)>,
    now: i64,
    cfg: AdsbSettings,
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
    // works from this list, so a stale aircraft cannot pull the view toward
    // where it used to be either.
    let live: Vec<&AdsbAircraft> =
        aircraft.iter().filter(|a| !a.pos_stale(now, cfg.drop_map_s)).collect();

    // ── the view ──
    let aspect = (rect.height() / rect.width()) as f64;
    let pts: Vec<(f64, f64)> = live.iter().filter_map(|a| a.lat.zip(a.lon)).collect();
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

    // Which target the pointer is over, decided before anything is drawn so the
    // hovered one can be drawn last and on top.
    let pointer = resp.hover_pos();
    let mut hover: Option<usize> = None;
    let mut best = HIT_RADIUS;
    for (i, a) in live.iter().enumerate() {
        let Some((lat, lon)) = a.lat.zip(a.lon) else { continue };
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
    for a in &live {
        let n = a.track.len();
        for (k, &(plat, plon)) in a.track.iter().enumerate() {
            let c = project(f64::from(plat), f64::from(plon));
            if !rect.contains(c) {
                continue;
            }
            // Oldest faintest, so the trail reads as a direction without a
            // single arrowhead being drawn.
            let age = (k + 1) as f32 / n as f32;
            p.circle_filled(c, 1.3, alpha(map.trail, 40.0 + 150.0 * age));
        }
    }

    let font = FontId::monospace(9.0);
    let selected_icao = state.selected;
    let tint_for = |a: &AdsbAircraft, top: bool| -> Color32 {
        if a.emergency.is_some() || selected_icao == Some(a.icao) {
            map.dx
        } else if top {
            map.hover
        } else {
            map.station
        }
    };

    // ── targets, under the data blocks ──
    // The hovered one last, so it sits over its neighbours.
    let draw_target = |a: &AdsbAircraft, top: bool| {
        let Some((lat, lon)) = a.lat.zip(a.lon) else { return };
        let c = project(lat, lon);
        if !rect.contains(c) && !top {
            return;
        }
        let tint = tint_for(a, top);

        // The leader, under the symbol: where it is now matters more than where
        // it will be.
        let path = leader(a, cfg.vector_minutes);
        if path.len() > 1 {
            let pts: Vec<_> = path.iter().map(|&(la, lo)| project(la, lo)).collect();
            for w in pts.windows(2) {
                p.line_segment([w[0], w[1]], (1.2, alpha(tint, 190.0)));
            }
        }

        // The target: a square, hollow, the same size at every zoom.
        let r = if top || selected_icao == Some(a.icao) { TARGET_R + 1.0 } else { TARGET_R };
        // A halo so it stays readable over the dotted land.
        p.circle_filled(c, r + 2.0, alpha(map.sea, 170.0));
        p.rect_stroke(
            Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)),
            0.0,
            (1.4, tint),
            eframe::egui::StrokeKind::Middle,
        );
        // On the ground: a dot inside, so a taxiing aircraft is distinguishable
        // from one at a thousand feet above the same runway.
        if a.on_ground {
            p.circle_filled(c, 1.4, tint);
        }
    };
    for (i, a) in live.iter().enumerate() {
        if hover != Some(i) {
            draw_target(a, false);
        }
    }
    if let Some(i) = hover {
        draw_target(live[i], true);
    }

    // ── the data blocks ──
    // The selected, the hovered and an emergency always; the rest by how much
    // they matter — what is flying, then what is on the ground — each only
    // where its block misses the symbols and the blocks already placed. A busy
    // sector then keeps the blocks it has room for, rather than losing every
    // one of them at once the moment a couple of dozen aircraft are in view,
    // which is how the AIS chart's names went missing (#408).
    let mut labels = Vec::new();
    for (i, a) in live.iter().enumerate() {
        let Some((lat, lon)) = a.lat.zip(a.lon) else { continue };
        let c = project(lat, lon);
        if !rect.contains(c) {
            continue;
        }
        let top = hover == Some(i);
        let selected = selected_icao == Some(a.icao);
        let tint = tint_for(a, top);
        labels.push(MapLabel {
            at: c,
            r: if top || selected { TARGET_R + 1.0 } else { TARGET_R },
            tick_from: 0.7,
            tick: alpha(tint, 110.0),
            lines: vec![
                (a.label(), alpha(tint, 240.0)),
                (a.fmt_altitude(), alpha(tint, 190.0)),
                (a.fmt_speed(), alpha(tint, 190.0)),
            ],
            must: top || selected || a.emergency.is_some(),
            rank: block_rank(a),
            key: a.icao,
        });
    }
    // Our own mark is drawn over the blocks, so they keep off it.
    let marks: Vec<Rect> = home
        .map(|(lat, lon)| Rect::from_center_size(project(lat, lon), vec2(16.0, 16.0)))
        .into_iter()
        .collect();
    map_labels::draw(&p, rect, &font, labels, &marks);

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
        let a = live[i];
        let mut tip = a.label();
        if !a.callsign.is_empty() {
            tip.push_str(&format!("  {}", a.hex()));
        }
        if let Some(cat) = &a.category {
            tip.push_str(&format!("\n{cat}"));
        }
        tip.push_str(&format!("\n{} ft   {} kt", a.altitude_ft.unwrap_or(0), a.fmt_speed()));
        if let Some(vr) = a.vertical_rate_fpm.filter(|v| v.abs() > 100) {
            tip.push_str(&tr!("adsbmap-ft-min", vr = format!("{:+}", vr)));
        }
        if let (Some(h), Some((lat, lon))) = (home, a.lat.zip(a.lon)) {
            let km = sdroxide_types::distance_km(h, (lat, lon));
            let bear = sdroxide_types::bearing_deg(h, (lat, lon));
            tip.push_str(&format!("\n{km:.0} km   {bear:.0}°"));
        }
        if let Some(e) = &a.emergency {
            tip.push_str(&format!("\n{e}"));
        }
        resp.clone().on_hover_text(tip);
        if resp.clicked() {
            clicked = Some(a.icao);
            state.selected = clicked;
        }
    } else if resp.clicked() {
        state.selected = None;
    }

    if manual && resp.hovered() {
        p.text(
            rect.right_bottom() + vec2(-6.0, -4.0),
            Align2::RIGHT_BOTTOM,
            tr!("adsbmap-double-click-reframe"),
            FontId::proportional(9.0),
            alpha(Color32::WHITE, 110.0),
        );
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flying(kt: f32, track: f32, turn: f32) -> AdsbAircraft {
        let mut a = AdsbAircraft::new(0x3C_6444, 0);
        a.lat = Some(48.0);
        a.lon = Some(16.0);
        a.ground_speed_kt = Some(kt);
        a.track_deg = Some(track);
        a.turn_rate_deg_s = turn;
        a
    }

    /// On a busy map the blocks that give way first are those of aircraft on
    /// the ground, and an emergency gives way to nothing.
    #[test]
    fn a_parked_aircraft_gives_way_to_one_flying() {
        let mut parked = flying(0.0, 0.0, 0.0);
        parked.on_ground = true;
        let airborne = flying(250.0, 0.0, 0.0);
        let mut mayday = flying(250.0, 0.0, 0.0);
        mayday.emergency = Some("7700 general emergency".into());
        assert!(block_rank(&mayday) < block_rank(&airborne));
        assert!(block_rank(&airborne) < block_rank(&parked));
    }

    /// The leader's length is the distance covered in the vector time, in
    /// degrees of latitude — which is what makes two aircraft with equal
    /// leaders equally fast at any zoom.
    #[test]
    fn the_leader_is_as_long_as_a_minute_of_flight() {
        // 480 knots due north for one minute is 8 NM, which is 8 minutes of
        // latitude: 0.1333 degrees.
        let path = leader(&flying(480.0, 0.0, 0.0), 1.0);
        let (lat, lon) = *path.last().expect("a leader");
        assert!((lat - 48.0 - 8.0 / 60.0).abs() < 1e-6, "reached {lat}");
        assert!((lon - 16.0).abs() < 1e-9, "due north should not move east: {lon}");

        // Half the speed, half the line.
        let half = leader(&flying(240.0, 0.0, 0.0), 1.0);
        let (hlat, _) = *half.last().expect("a leader");
        assert!(((hlat - 48.0) - (lat - 48.0) / 2.0).abs() < 1e-9);
    }

    /// A degree of longitude is shorter than a degree of latitude everywhere but
    /// the equator, so an eastbound leader has to be drawn wider in degrees or
    /// it comes out short on the map.
    #[test]
    fn an_eastbound_leader_is_stretched_for_the_latitude_it_is_at() {
        let path = leader(&flying(480.0, 90.0, 0.0), 1.0);
        let (lat, lon) = *path.last().expect("a leader");
        assert!((lat - 48.0).abs() < 1e-6, "due east should not move north: {lat}");
        let want = (8.0 / 60.0) / 48.0f64.to_radians().cos();
        assert!((lon - 16.0 - want).abs() < 1e-4, "reached {lon}, wanted {}", 16.0 + want);
    }

    /// A turning aircraft gets a curved leader — the whole reason the tracker
    /// derives a turn rate at all.
    #[test]
    fn a_turning_aircraft_gets_a_bent_leader() {
        let straight = leader(&flying(300.0, 0.0, 0.0), 1.0);
        let turning = leader(&flying(300.0, 0.0, 3.0), 1.0);
        assert_eq!(straight.len(), turning.len());
        let end_s = straight.last().unwrap();
        let end_t = turning.last().unwrap();
        assert!(
            (end_t.1 - end_s.1).abs() > 0.01,
            "a standard-rate turn should visibly bend the leader"
        );
        // Every leg of a straight leader is on the same meridian; a turning
        // one's are not.
        assert!(straight.iter().all(|p| (p.1 - 16.0).abs() < 1e-9));
    }

    /// A target with nothing to draw a leader from gets none, rather than a
    /// zero-length stub or a line pointing north by default.
    #[test]
    fn a_target_with_no_velocity_gets_no_leader() {
        let mut a = flying(400.0, 90.0, 0.0);
        a.ground_speed_kt = None;
        assert!(leader(&a, 1.0).is_empty());
        let mut a = flying(400.0, 90.0, 0.0);
        a.track_deg = None;
        assert!(leader(&a, 1.0).is_empty());
        // Parked on a stand.
        assert!(leader(&flying(0.0, 90.0, 0.0), 1.0).is_empty());
        // And the operator can turn leaders off altogether.
        assert!(leader(&flying(400.0, 90.0, 0.0), 0.0).is_empty());
    }

    /// One aircraft overhead must not zoom the map to street level, and an
    /// empty sky shows the world rather than the Gulf of Guinea.
    #[test]
    fn the_auto_fit_has_a_floor_and_an_empty_default() {
        let (_, _, span) = target_view(Some((48.2, 16.37)), &[(48.2005, 16.3705)], 0.5);
        assert!(span >= MIN_LON_SPAN);
        let (_, _, span) = target_view(None, &[], 0.5);
        assert_eq!(span, 360.0);
    }
}
