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
use eframe::egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Ui, pos2, vec2};
use sdroxide_types::{AdsbAircraft, AdsbSettings};

use crate::theme;
use crate::widgets::map_labels::{self, MapLabel};
use crate::widgets::worldmap::{MapView, alpha, draw_base_layers, interact, wrap180};

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

/// The coverage diagram: each cell an annular slice, yellow like the
/// propagation heat, brighter for a stronger signal and more opaque for more
/// positions; the boundary as a line through the outer edge of each sector's
/// last well-filled cell; and the slice from home out to the selected aircraft,
/// so its bearing and distance read against the diagram (as SDRangel draws it).
fn draw_coverage(
    p: &eframe::egui::Painter,
    cov: &crate::adsb_coverage::CoverageView,
    selected: Option<(f64, f64)>,
    project: &dyn Fn(f64, f64) -> Pos2,
) {
    use crate::adsb_coverage::{RING_KM, RINGS, SECTORS, destination};
    let step = 360.0 / SECTORS as f64;
    let yellow = theme::YELLOW();
    // An annular slice from `r0` to `r1` km between two bearings, as a convex
    // polygon: at 5° it is close enough to one.
    let slice = |b0: f64, b1: f64, r0: f64, r1: f64| -> Vec<Pos2> {
        let mut pts = Vec::with_capacity(8);
        for k in 0..=2 {
            let (la, lo) = destination(cov.home, b0 + (b1 - b0) * k as f64 / 2.0, r1);
            pts.push(project(la, lo));
        }
        for k in (0..=2).rev() {
            let (la, lo) = destination(cov.home, b0 + (b1 - b0) * k as f64 / 2.0, r0);
            pts.push(project(la, lo));
        }
        pts
    };
    for s in 0..SECTORS {
        let (b0, b1) = (s as f64 * step, (s + 1) as f64 * step);
        for r in 0..RINGS {
            let c = cov.cells[s * RINGS + r];
            if c.n == 0 {
                continue;
            }
            // −30 dBFS weak … −3 dBFS strong.
            let q = ((c.mean_rssi() + 30.0) / 27.0).clamp(0.0, 1.0);
            let fill = ((c.n as f32).ln_1p() / 20f32.ln_1p()).min(1.0);
            let a = 18.0 + 80.0 * fill * (0.35 + 0.65 * q);
            let pts = slice(b0, b1, r as f64 * RING_KM, (r + 1) as f64 * RING_KM);
            p.add(eframe::egui::Shape::convex_polygon(
                pts,
                alpha(yellow, a),
                eframe::egui::Stroke::NONE,
            ));
        }
    }
    // The boundary: outer arc of each sector, joined across neighbours.
    let mut line: Vec<Pos2> = Vec::new();
    let ink = alpha(yellow, 200.0);
    for s in 0..=SECTORS {
        match cov.boundary[s % SECTORS] {
            Some(r) => {
                let km = (r + 1) as f64 * RING_KM;
                for b in [s as f64 * step, (s as f64 + 1.0) * step] {
                    let (la, lo) = destination(cov.home, b, km);
                    line.push(project(la, lo));
                }
            }
            None => {
                if line.len() >= 2 {
                    p.add(eframe::egui::Shape::line(std::mem::take(&mut line), (1.2, ink)));
                }
                line.clear();
            }
        }
        if s == SECTORS {
            break;
        }
    }
    if line.len() >= 2 {
        p.add(eframe::egui::Shape::line(line, (1.2, ink)));
    }
    // The selected aircraft's slice, from home to it.
    if let Some(pos) = selected {
        let km = sdroxide_types::distance_km(cov.home, pos);
        let b = sdroxide_types::bearing_deg(cov.home, pos);
        let b0 = (b / step).floor() * step;
        let mut pts = vec![project(cov.home.0, cov.home.1)];
        for k in 0..=4 {
            let (la, lo) = destination(cov.home, b0 + step * k as f64 / 4.0, km);
            pts.push(project(la, lo));
        }
        p.add(eframe::egui::Shape::convex_polygon(
            pts,
            alpha(theme::CYAN(), 40.0),
            eframe::egui::Stroke::new(1.0, alpha(theme::CYAN(), 150.0)),
        ));
    }
}

/// How a target is drawn, from the emitter category it sends.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Symbol {
    /// A top-view aeroplane, nose along the track, this many pixels from the
    /// centre to the nose: bigger for a heavier class.
    Plane(f32),
    /// A helicopter: a hub and a crossed rotor.
    Rotor,
    /// Anything else (a vehicle, an obstacle, a balloon): the plain square.
    Square,
}

impl Symbol {
    /// How far the symbol reaches from its centre, for keeping labels off it.
    fn radius(self) -> f32 {
        match self {
            Symbol::Plane(l) => l,
            Symbol::Rotor => 4.5,
            Symbol::Square => TARGET_R,
        }
    }
}

/// The symbol for an aircraft. Sizes by the ADS-B weight classes (DO-260
/// set A): light under 7 t, small to 34 t, large to 136 t, heavy above; a
/// target that has sent no category yet is drawn as a large one, the most
/// common class over Europe (HDZ 2026-11-17: „größere für heavy, kleine für
/// kleine“).
fn symbol_of(a: &AdsbAircraft) -> Symbol {
    match a.category.as_deref() {
        Some("heavy" | "high-vortex large") => Symbol::Plane(10.0),
        Some("large") | None => Symbol::Plane(7.5),
        Some("small" | "high performance") => Symbol::Plane(6.0),
        Some("light" | "glider" | "ultralight" | "unmanned") => Symbol::Plane(5.0),
        Some("rotorcraft") => Symbol::Rotor,
        Some(_) => Symbol::Square,
    }
}

/// The aeroplane outline, nose at (0, −1), wingspan 2: fuselage, wings and
/// tailplane, each a convex piece so it can be filled as one.
const PLANE_PARTS: [&[(f32, f32)]; 5] = [
    &[(0.0, -1.0), (0.11, -0.8), (0.11, 0.8), (0.0, 0.95), (-0.11, 0.8), (-0.11, -0.8)],
    &[(0.1, -0.3), (1.0, 0.12), (1.0, 0.28), (0.1, 0.15)],
    &[(-0.1, -0.3), (-0.1, 0.15), (-1.0, 0.28), (-1.0, 0.12)],
    &[(0.08, 0.62), (0.42, 0.82), (0.42, 0.93), (0.08, 0.85)],
    &[(-0.08, 0.62), (-0.08, 0.85), (-0.42, 0.93), (-0.42, 0.82)],
];

/// The same outline as one closed line, for the halo behind it.
const PLANE_OUTLINE: [(f32, f32); 17] = [
    (0.0, -1.0),
    (0.11, -0.8),
    (0.11, -0.3),
    (1.0, 0.12),
    (1.0, 0.28),
    (0.11, 0.15),
    (0.11, 0.62),
    (0.42, 0.82),
    (0.42, 0.93),
    (0.0, 0.95),
    (-0.42, 0.93),
    (-0.42, 0.82),
    (-0.11, 0.62),
    (-0.11, 0.15),
    (-1.0, 0.28),
    (-1.0, 0.12),
    (-0.11, -0.3),
];

/// An aeroplane at `c`, `len` pixels to the nose, the nose along `dir` (a
/// unit vector on screen). Filled in the air, outlined only on the ground.
fn draw_plane(
    p: &eframe::egui::Painter,
    c: Pos2,
    len: f32,
    dir: eframe::egui::Vec2,
    on_ground: bool,
    tint: Color32,
    sea: Color32,
) {
    // The nose (0, −1) turned onto `dir`.
    let (sin, cos) = (dir.x, -dir.y);
    let at = |&(x, y): &(f32, f32)| c + vec2(x * cos - y * sin, x * sin + y * cos) * len;
    let outline: Vec<Pos2> = PLANE_OUTLINE.iter().map(at).collect();
    p.add(eframe::egui::Shape::closed_line(outline.clone(), (2.6, alpha(sea, 200.0))));
    if on_ground {
        p.add(eframe::egui::Shape::closed_line(outline, (1.1, tint)));
        return;
    }
    for part in PLANE_PARTS {
        p.add(eframe::egui::Shape::convex_polygon(
            part.iter().map(at).collect(),
            tint,
            eframe::egui::Stroke::NONE,
        ));
    }
}

/// How far in (degrees of longitude across the map) each airport rank shows:
/// the large ones from a continent's width, medium ones with scheduled service
/// at a few countries, the rest at a region.
const AIRPORT_RANK_SPAN: [f64; 3] = [40.0, 10.0, 4.0];

/// The AIRPORTS layer: a small ringed circle, the chart sign for an airport,
/// with its ICAO code beside it where the code has room. Returns the airport
/// under the pointer, for the readout.
fn draw_airports<'a>(
    p: &eframe::egui::Painter,
    rect: Rect,
    aps: &'a [crate::adsb_route::MapAirport],
    lon_span: f64,
    map: &theme::MapPalette,
    project: &dyn Fn(f64, f64) -> Pos2,
    pointer: Option<Pos2>,
) -> Option<&'a crate::adsb_route::MapAirport> {
    // The river's blue, lifted halfway to white: the plain river blue was
    // barely readable on the dark sea (HDZ 2026-11-17: „heller machen“).
    let lift = |c: Color32| {
        let l = |v: u8| v + ((255 - v) as f32 * 0.5) as u8;
        Color32::from_rgb(l(c.r()), l(c.g()), l(c.b()))
    };
    let blue = lift(map.river);
    let ink = alpha(blue, 230.0);
    let font = FontId::monospace(8.5);
    let mut taken: Vec<Rect> = Vec::new();
    let mut near: Option<(&crate::adsb_route::MapAirport, f32)> = None;
    // Large ones first, so in a crowd it is they that keep their code.
    for (rank, &span) in AIRPORT_RANK_SPAN.iter().enumerate() {
        if lon_span > span {
            continue;
        }
        for ap in aps.iter().filter(|a| a.rank as usize == rank) {
            let c = project(ap.lat, ap.lon);
            if !rect.shrink(2.0).contains(c) {
                continue;
            }
            let r = if rank == 0 { 3.2 } else { 2.6 };
            p.circle_filled(c, r + 1.5, alpha(map.sea, 150.0));
            p.circle_stroke(c, r, (1.2, ink));
            for (dx, dy) in [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)] {
                let d = vec2(dx, dy);
                p.line_segment([c + d * r, c + d * (r + 2.0)], (1.0, ink));
            }
            if let Some(m) = pointer {
                let d = c.distance(m);
                if d <= 7.0 && near.is_none_or(|(_, best)| d < best) {
                    near = Some((ap, d));
                }
            }
            // At a continent's width the large ones keep their sign, not their code.
            if rank == 0 && lon_span > 20.0 {
                continue;
            }
            // The IATA code on the map, the one on tickets and boards (HAJ);
            // the ICAO code only in the readout (HDZ 2026-11-17: „HAJ als
            // erstes, EDVK in den Tooltip verbannen“). Fields without one
            // (military, many medium ones) keep their ICAO code.
            let code = if ap.iata.is_empty() { &ap.icao } else { &ap.iata };
            let galley = p.layout_no_wrap(code.clone(), font.clone(), alpha(blue, 225.0));
            let at = c + vec2(r + 4.0, -galley.size().y * 0.5);
            let bbox = Rect::from_min_size(at, galley.size()).expand(1.0);
            if rect.contains_rect(bbox) && !taken.iter().any(|t| t.intersects(bbox)) {
                taken.push(bbox);
                p.galley(at, galley, Color32::WHITE);
            }
        }
    }
    near.map(|(a, _)| a)
}

/// The place under a screen point: the inverse of the map's projection.
fn pointer_lat_lon(
    m: Pos2,
    rect: Rect,
    clat: f64,
    clon: f64,
    lon_span: f64,
    lat_span: f64,
) -> (f64, f64) {
    let fx = f64::from((m.x - rect.left()) / rect.width());
    let fy = f64::from((m.y - rect.top()) / rect.height());
    let lat = (clat + (0.5 - fy) * lat_span).clamp(-89.9, 89.9);
    (lat, wrap180(clon + (fx - 0.5) * lon_span))
}

/// What the coverage diagram holds at a place: the cell's positions and mean
/// level, and how far reception reaches along that bearing.
fn coverage_readout(cov: &crate::adsb_coverage::CoverageView, at: (f64, f64)) -> String {
    use crate::adsb_coverage::{RING_KM, RINGS, SECTORS, cell_of};
    let bear = sdroxide_types::bearing_deg(cov.home, at);
    let sector = ((bear.rem_euclid(360.0) / (360.0 / SECTORS as f64)) as usize).min(SECTORS - 1);
    let reach = match cov.boundary[sector] {
        Some(r) => tr!("adsbmap-reach", km = format!("{:.0}", (r + 1) as f64 * RING_KM)),
        None => tr!("adsbmap-reach-none"),
    };
    let cell = match cell_of(cov.home, at).map(|i| cov.cells[i]) {
        Some(c) if c.n > 0 => {
            tr!("adsbmap-cell", n = c.n.to_string(), rssi = format!("{:.0}", c.mean_rssi()))
        }
        Some(_) => tr!("adsbmap-cell-empty"),
        None => tr!("adsbmap-cell-outside", km = format!("{:.0}", RINGS as f64 * RING_KM)),
    };
    format!("{cell}\n{reach}")
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

/// What the map lays over the ground, each switched by a chip above it.
pub struct Layers<'a> {
    /// The grey line's texture, when NIGHT is on.
    pub night: Option<eframe::egui::TextureId>,
    pub cities: bool,
    /// The coverage diagram, when RANGE is on and there is a home.
    pub coverage: Option<&'a crate::adsb_coverage::CoverageView>,
    /// The pointer readout (chip CURSOR): bearing, distance, coverage cell
    /// and the airport under it. Off means no readout at all.
    pub pointer: bool,
    /// The AIRPORTS layer, when on and its file is in.
    pub airports: Option<&'a [crate::adsb_route::MapAirport]>,
}

/// Draw the map. Returns the aircraft clicked this frame, if any.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut Ui,
    state: &mut AdsbMapState,
    aircraft: &[AdsbAircraft],
    route: &[crate::adsb_route::Airport],
    home: Option<(f64, f64)>,
    now: i64,
    cfg: AdsbSettings,
    max_h: f32,
    layers: Layers<'_>,
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

    let project = |lat: f64, lon: f64| {
        let dlon = wrap180(lon - clon);
        pos2(
            rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
            rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
        )
    };

    let selected_pos = state
        .selected
        .and_then(|i| live.iter().find(|a| a.icao == i))
        .and_then(|a| a.lat.zip(a.lon));
    let dot_r = draw_base_layers(
        &p,
        rect,
        clat,
        clon,
        lon_span,
        lat_span,
        map,
        layers.night,
        layers.cities,
        || {
            if let Some(cov) = layers.coverage {
                draw_coverage(&p, cov, selected_pos, &project);
            }
        },
    );

    // ── airports, over the cities, under the traffic ──
    let near_airport = layers
        .airports
        .and_then(|aps| draw_airports(&p, rect, aps, lon_span, map, &project, resp.hover_pos()));

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

    // ── the selected aircraft's route, under the targets ──
    // Dashed great circles: departure → aircraft → destination when the route
    // has no stops and the aircraft has a place, else airport to airport. The
    // whole route, not just the leg ahead (HDZ 2026-11-16: „Strecke gesamt
    // für gewählt ja“).
    // Pitfall: the route is the callsign's usual one, not today's plan; an
    //          aircraft far off its line is a reused callsign or a diversion.
    if route.len() >= 2 {
        let here = selected_icao
            .and_then(|i| live.iter().find(|a| a.icao == i))
            .and_then(|a| a.lat.zip(a.lon));
        let mut stops: Vec<(f64, f64)> = route.iter().map(|ap| (ap.lat, ap.lon)).collect();
        if let (2, Some(h)) = (stops.len(), here) {
            stops.insert(1, h);
        }
        let ink = alpha(map.dx, 170.0);
        for leg in stops.windows(2) {
            let pts: Vec<_> = crate::adsb_route::great_circle(leg[0], leg[1], 64)
                .into_iter()
                .map(|(la, lo)| project(la, lo))
                .collect();
            // A leg that crosses the map's back edge would be drawn as a line
            // across the whole picture; it is cut where it jumps.
            let mut seg = vec![pts[0]];
            for w in pts.windows(2) {
                if (w[1].x - w[0].x).abs() > rect.width() * 0.5 {
                    p.extend(eframe::egui::Shape::dashed_line(&seg, (1.3, ink), 6.0, 4.0));
                    seg.clear();
                }
                seg.push(w[1]);
            }
            p.extend(eframe::egui::Shape::dashed_line(&seg, (1.3, ink), 6.0, 4.0));
        }
        for ap in route {
            let c = project(ap.lat, ap.lon);
            if !rect.contains(c) {
                continue;
            }
            p.circle_filled(c, 3.0, alpha(map.sea, 200.0));
            p.circle_stroke(c, 3.0, (1.2, ink));
            p.text(c + vec2(5.0, -1.0), Align2::LEFT_BOTTOM, &ap.icao, font.clone(), ink);
        }
    }
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

        let lift = if top || selected_icao == Some(a.icao) { 1.5 } else { 0.0 };
        match symbol_of(a) {
            Symbol::Plane(len) => {
                // The nose along the track as the map shows it: projected,
                // since the map stretches east-west away from the equator and
                // the leader line, drawn the same way, must leave the nose.
                let dir = a
                    .track_deg
                    .or(a.avionics.heading_deg)
                    .map(|t| {
                        let (la, lo) =
                            crate::adsb_coverage::destination((lat, lon), f64::from(t), 10.0);
                        (project(la, lo) - c).normalized()
                    })
                    .filter(|v| v.x.is_finite() && v.y.is_finite())
                    .unwrap_or(vec2(0.0, -1.0));
                draw_plane(&p, c, len + lift, dir, a.on_ground, tint, map.sea)
            }
            Symbol::Rotor => {
                let r = 4.5 + lift;
                p.circle_filled(c, r + 2.0, alpha(map.sea, 170.0));
                p.circle_filled(c, 2.2, tint);
                // The rotor, turned with the track so it reads as a craft.
                let b = a.track_deg.or(a.avionics.heading_deg).unwrap_or(0.0).to_radians() + 0.785;
                for k in 0..2 {
                    let d = vec2((b + k as f32 * 1.571).cos(), (b + k as f32 * 1.571).sin()) * r;
                    p.line_segment([c - d, c + d], (1.3, tint));
                }
            }
            Symbol::Square => {
                // The target: a square, hollow, the same size at every zoom.
                let r = TARGET_R + lift * 0.7;
                // A halo so it stays readable over the dotted land.
                p.circle_filled(c, r + 2.0, alpha(map.sea, 170.0));
                p.rect_stroke(
                    Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)),
                    0.0,
                    (1.4, tint),
                    eframe::egui::StrokeKind::Middle,
                );
                // On the ground: a dot inside, so a taxiing aircraft is
                // distinguishable from one at a thousand feet above the same
                // runway.
                if a.on_ground {
                    p.circle_filled(c, 1.4, tint);
                }
            }
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
            r: symbol_of(a).radius() + if top || selected { 1.5 } else { 0.0 },
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

    // ── the pointer's bearing and distance from us ──
    // Over open map, a faint line from home to the pointer, the range ring
    // through it, and a readout beside the pointer: bearing, distance, and with
    // RANGE on what the diagram holds there, so the yellow area can be read off
    // (HDZ 2026-11-17: „Kurs und Entfernung … damit ich die gelbe Fläche
    // abschätzen kann“). Painted, not a tooltip: a tooltip waits for the
    // pointer to stand still, and this is read while moving.
    // CURSOR off turns the whole readout off, airport names included
    // (HDZ 2026-11-17: „Chip, Cursor-Details an/aus“).
    let bearing = layers.pointer.then_some(home).flatten();
    let near_airport = near_airport.filter(|_| layers.pointer);
    if let (Some(m), None) = (pointer, hover)
        && rect.contains(m)
        && !resp.dragged()
        && layers.pointer
    {
        let at = pointer_lat_lon(m, rect, clat, clon, lon_span, lat_span);
        let mut lines: Vec<String> = Vec::new();
        if let Some(ap) = near_airport {
            let code = if ap.iata.is_empty() {
                ap.icao.clone()
            } else {
                format!("{} · {}", ap.iata, ap.icao)
            };
            lines.push(format!("{code}  {}", ap.name));
        }
        if let Some(h) = bearing {
            let km = sdroxide_types::distance_km(h, at);
            let bear = sdroxide_types::bearing_deg(h, at);
            let ink = alpha(map.home, 120.0);
            p.extend(eframe::egui::Shape::dashed_line(
                &[project(h.0, h.1), m],
                (1.0, ink),
                4.0,
                3.0,
            ));
            let ring: Vec<Pos2> = (0..=96)
                .map(|k| {
                    let (la, lo) = crate::adsb_coverage::destination(h, k as f64 * 3.75, km);
                    project(la, lo)
                })
                .collect();
            p.add(eframe::egui::Shape::line(ring, (0.8, alpha(map.home, 70.0))));
            lines.push(format!("{bear:03.0}°   {km:.0} km"));
            if let Some(cov) = layers.coverage {
                lines.push(coverage_readout(cov, at));
            }
        }
        let text = lines.join("\n");
        let galley = p.layout_no_wrap(text, FontId::monospace(10.0), alpha(Color32::WHITE, 230.0));
        let size = galley.size() + vec2(8.0, 6.0);
        let empty = lines.is_empty();
        // Below right of the pointer, flipped where it would leave the map.
        let mut tl = m + vec2(14.0, 14.0);
        if tl.x + size.x > rect.right() {
            tl.x = m.x - 10.0 - size.x;
        }
        if tl.y + size.y > rect.bottom() {
            tl.y = m.y - 10.0 - size.y;
        }
        if !empty {
            let bg = Rect::from_min_size(tl, size);
            p.rect_filled(bg, 3.0, alpha(map.sea, 225.0));
            p.galley(tl + vec2(4.0, 3.0), galley, Color32::WHITE);
        }
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

    #[test]
    fn the_pointer_reads_back_the_place_the_map_put_there() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 100.0));
        let (clat, clon, lon_span, lat_span) = (51.5, 179.0, 4.0, 2.0);
        let mid = pointer_lat_lon(rect.center(), rect, clat, clon, lon_span, lat_span);
        assert!((mid.0 - 51.5).abs() < 1e-6 && (mid.1 - 179.0).abs() < 1e-6);
        // The right edge is past the date line, and wraps.
        let (lat, lon) = pointer_lat_lon(rect.right_top(), rect, clat, clon, lon_span, lat_span);
        assert!((lat - 52.5).abs() < 1e-6);
        assert!((lon + 179.0).abs() < 1e-6, "{lon}");
    }

    #[test]
    fn a_heavy_is_drawn_bigger_than_a_light_one_and_a_helicopter_as_a_rotor() {
        let mut a = flying(450.0, 90.0, 0.0);
        let len = |a: &AdsbAircraft| match symbol_of(a) {
            Symbol::Plane(l) => l,
            s => panic!("{s:?}"),
        };
        a.category = Some("heavy".into());
        let heavy = len(&a);
        a.category = Some("light".into());
        let light = len(&a);
        a.category = None;
        let unknown = len(&a);
        assert!(heavy > unknown && unknown > light);
        a.category = Some("rotorcraft".into());
        assert_eq!(symbol_of(&a), Symbol::Rotor);
        a.category = Some("service vehicle".into());
        assert_eq!(symbol_of(&a), Symbol::Square);
    }

    #[test]
    fn the_plane_parts_lie_inside_the_outline_span() {
        let (mut lo, mut hi) = ((0f32, 0f32), (0f32, 0f32));
        for &(x, y) in PLANE_OUTLINE.iter() {
            lo = (lo.0.min(x), lo.1.min(y));
            hi = (hi.0.max(x), hi.1.max(y));
        }
        for part in PLANE_PARTS {
            for &(x, y) in part {
                assert!(x >= lo.0 && x <= hi.0 && y >= lo.1 && y <= hi.1);
            }
        }
        assert_eq!((lo.1, hi.0), (-1.0, 1.0), "nose up, wingspan two");
    }
}
