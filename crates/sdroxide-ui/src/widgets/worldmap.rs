//! A small pixel/dot-matrix world map for the FT8 QSO panel: renders the
//! continents as glowing dots — with their borders, their rivers and the
//! cities that fit — and marks the home + DX locations with a great-circle
//! path between them, cyberpunk style.
//!
//! The ground under it all comes from [`crate::basemap`], which is the same
//! Natural Earth data the 3D globe is textured with. That is deliberate: the
//! two views have to agree about where a shoreline is, and one set of assets
//! for both is how they do. The land arrives as a coverage raster and the lines
//! as the polylines they were digitised as — the two halves of the map that
//! want opposite things from their data.
//!
//! The map is centred on the operator's home grid (the world wraps around it
//! rather than being shifted) and smoothly auto-zooms to frame home plus every
//! decoded station, re-fitting whenever new stations appear.
//!
//! The auto-fit is a starting point, not a cage: drag (or one finger) pans,
//! wheel and pinch zoom about the pointer, and a double-click hands the view
//! back to the auto-fit.

use eframe::egui::{
    Align2, Color32, CursorIcon, FontId, PointerButton, Pos2, Response, Sense, Ui, Vec2, pos2, vec2,
};
use sdroxide_types::great_circle_points;

use crate::theme;

/// Below this height the map is not worth drawing — the caller should omit it
/// entirely so the QSO controls keep the space.
pub const MIN_HEIGHT: f32 = 72.0;

/// Never zoom tighter than this longitudinal span (degrees), so a single nearby
/// contact doesn't blow the map up to street level.
const MIN_LON_SPAN: f64 = 30.0;
/// The floor for a zoom the user asked for by hand — lower than the auto-fit's,
/// because "show me that corner of Europe" is a real request. A degree across
/// is about a hundred kilometres, which is past what the land raster resolves
/// (1/22.75°, ~4.9 km) but not past what the map *draws*: the coastline is
/// stroked along that field's contour and the borders and rivers come from
/// geometry, so both stay clean lines at any zoom. What this buys, past the
/// point where new detail arrives, is room between the markers.
const MIN_USER_LON_SPAN: f64 = 1.0;
/// Fraction of extra margin left around the outermost contact.
const PAD: f64 = 1.4;
/// Per-frame ease toward the target view (0..1); smaller = slower/smoother.
const EASE: f64 = 0.0375;

/// Persistent, animated view state (centre + longitudinal span, in degrees).
/// Owned by the caller so the zoom eases across frames.
pub struct MapView {
    pub(crate) clat: f64,
    pub(crate) clon: f64,
    pub(crate) lon_span: f64,
    pub(crate) initialized: bool,
    /// The user has panned or zoomed by hand: the auto-fit stops moving the
    /// view under them until they double-click to hand it back.
    pub(crate) manual: bool,
}

impl Default for MapView {
    fn default() -> Self {
        MapView { clat: 20.0, clon: 0.0, lon_span: 360.0, initialized: false, manual: false }
    }
}

impl MapView {
    /// The latitude window this zoom covers on a map of the given aspect
    /// (height/width) — the projection is linear in both axes.
    pub(crate) fn lat_span(&self, aspect: f64) -> f64 {
        self.lon_span * aspect
    }

    /// Keep the view legal: the zoom inside its limits, and the latitude window
    /// inside the poles so a pan cannot drift off into empty space above the
    /// map. Longitude wraps instead of clamping — the world repeats sideways.
    pub(crate) fn clamp(&mut self, aspect: f64) {
        self.lon_span = self.lon_span.clamp(MIN_USER_LON_SPAN, 360.0);
        let lat_span = self.lat_span(aspect);
        self.clat = if lat_span >= 180.0 {
            0.0
        } else {
            self.clat.clamp(-90.0 + lat_span / 2.0, 90.0 - lat_span / 2.0)
        };
        self.clon = wrap180(self.clon);
    }

    /// Drag the map by a fraction of its own size, grab-the-content sense: the
    /// land under the pointer stays under it.
    fn pan(&mut self, dx_frac: f64, dy_frac: f64, aspect: f64) {
        self.clon = wrap180(self.clon - dx_frac * self.lon_span);
        self.clat += dy_frac * self.lat_span(aspect);
        self.clamp(aspect);
    }

    /// Put a place in the middle and hold it there.
    ///
    /// Holding it is the point: the auto-fit frames everything on the map,
    /// which is the opposite of what somebody who asked to centre on one
    /// station wants. A double-click hands the view back.
    pub(crate) fn centre_on(&mut self, lat: f64, lon: f64) {
        self.clat = lat;
        self.clon = wrap180(lon);
        self.initialized = true;
        self.manual = true;
    }

    /// Zoom by `factor` (below 1 zooms *in*) about a point given as a fraction
    /// of the map rect — (0,0) top-left, (1,1) bottom-right — keeping whatever
    /// is under that point in place.
    fn zoom_about(&mut self, factor: f64, fx: f64, fy: f64, aspect: f64) {
        // Where the anchor sits relative to the centre, in view fractions, and
        // the place it is currently over.
        let (ax, ay) = (fx - 0.5, 0.5 - fy);
        let lon_a = self.clon + ax * self.lon_span;
        let lat_a = self.clat + ay * self.lat_span(aspect);
        self.lon_span = (self.lon_span * factor).clamp(MIN_USER_LON_SPAN, 360.0);
        // Re-centre so that place lands back under the same fraction.
        self.clon = wrap180(lon_a - ax * self.lon_span);
        self.clat = lat_a - ay * self.lat_span(aspect);
        self.clamp(aspect);
    }
}

/// `c` at `a`/255 opacity — the halo behind a marker, or a station dot faded
/// by age. Takes the alpha as a float because every caller is scaling one.
pub(crate) fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a.clamp(0.0, 255.0) as u8)
}

/// Wrap a longitude delta into [-180, 180).
pub(crate) fn wrap180(mut d: f64) -> f64 {
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

/// Mouse and touch control of the view: drag (or one finger) pans, wheel and
/// pinch zoom about the pointer, double-click hands the view back to the
/// auto-fit. Returns true while the user is actually moving it, so the caller
/// can keep the frames coming.
pub(crate) fn interact(ui: &Ui, view: &mut MapView, resp: &Response, aspect: f64) -> bool {
    let rect = resp.rect;
    let mut touched = false;
    let frac = |p: Pos2| {
        (((p.x - rect.left()) / rect.width()) as f64, ((p.y - rect.top()) / rect.height()) as f64)
    };

    // Two fingers zoom and slide the map: on a screen with no wheel there is no
    // other way in.
    //
    // First in the chain, and it swallows the drag, for the reason the
    // panadapter's pinch does: the browser keeps reporting a one-finger drag
    // from the *first* finger down for the whole gesture, so without this a
    // pinch would pan the map at the same time as it zoomed.
    let multi = ui.input(|i| i.multi_touch());
    let pinch = multi.filter(|mt| rect.contains(mt.center_pos));
    if let Some(mt) = pinch {
        if mt.zoom_delta > 0.0 && mt.zoom_delta != 1.0 {
            // Fingers apart is zoom *in*, i.e. a smaller span.
            let (fx, fy) = frac(mt.center_pos);
            view.zoom_about(1.0 / mt.zoom_delta as f64, fx, fy, aspect);
            touched = true;
        }
        let t = mt.translation_delta;
        if t != Vec2::ZERO {
            view.pan((t.x / rect.width()) as f64, (t.y / rect.height()) as f64, aspect);
            touched = true;
        }
    } else if resp.dragged_by(PointerButton::Primary) {
        let d = resp.drag_delta();
        if d != Vec2::ZERO {
            view.pan((d.x / rect.width()) as f64, (d.y / rect.height()) as f64, aspect);
            touched = true;
        }
    }

    // Wheel zoom about the pointer. `zoom_delta` carries a trackpad pinch and
    // ctrl+wheel, which egui reports as a zoom factor rather than as scroll —
    // but it also mirrors a two-finger pinch anywhere on screen, so it is only
    // read when no touch gesture is running.
    if let Some(pos) = resp.hover_pos().filter(|_| multi.is_none()) {
        let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        // Multiplicative, so a wheel click covers the same visual fraction at
        // any zoom — the same rate the panadapter uses.
        let factor = 0.998f64.powf(scroll as f64 * 2.0) / (zoom.max(0.01) as f64);
        if (factor - 1.0).abs() > 1e-4 {
            let (fx, fy) = frac(pos);
            view.zoom_about(factor, fx, fy, aspect);
            touched = true;
        }
    }

    if touched {
        view.manual = true;
    }
    // Double-click hands the view back: the auto-fit resumes and eases home
    // from wherever the user left it.
    if resp.double_clicked() {
        view.manual = false;
    }
    if resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    touched
}

/// The view to ease toward: centred on home (else the contacts' centroid),
/// zoomed symmetrically to frame home plus every contact.
fn target_view(home: Option<(f64, f64)>, contacts: &[(f64, f64)], aspect: f64) -> (f64, f64, f64) {
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

/// Pitch of the dot matrix, in points. The grid the whole map is drawn on, and
/// so the map's resolution: coarse enough that the dots still read as a stipple
/// rather than as a fill, and no coarser. Every dot is a `circle_filled`, so
/// this is quadratic in what it costs — three points is about twice the work of
/// four and is where the two stop trading against each other.
const DOT_PITCH: f32 = 3.0;

/// How solid a land dot is, from the coverage under it.
///
/// Zoomed out, coverage very nearly *is* the alpha: a cell half full of Denmark
/// comes out a half-lit dot, and a coast reads as an edge rather than as a
/// staircase. Zoomed in past the grid the same field is contrast-stretched
/// about its ½ contour instead — which is where the coastline actually is — so
/// the shore stays a drawn line instead of dissolving into a gradient. That is
/// the flat map's version of the `fwidth` stroke `solar_body.wgsl` puts on the
/// globe, and it is why both show the same shoreline at the same sharpness.
fn land_ink(cov: f32, mag: f32) -> f32 {
    ((cov - 0.5) * 1.4 * mag.max(1.0) + 0.5).clamp(0.0, 1.0)
}

/// Walk one line layer onto the dot grid, marking every cell a line passes
/// through with that line's rank plus one.
///
/// This is what a flat map does instead of sampling `borders.png`. A border is
/// one texel wide there, so once the map is zoomed in far enough for a texel to
/// cover several dots, no threshold on the interpolated coverage gives a line
/// back: a low one draws the interpolation's skirt as well and the border comes
/// out a band four dots across, a high one breaks the same border into
/// fragments wherever it happens to fall between two texels. Walking the
/// geometry has neither failure: the line is one cell wide because it is drawn
/// one cell at a time, and it is where it was digitised.
///
/// Marking rather than drawing, because a line crosses its own cells over and
/// over — every vertex of a meander inside one cell would otherwise stack a
/// dozen half-transparent dots on top of each other and burn that cell white.
fn stamp_lines(
    layer: &crate::basemap::LineLayer,
    (clat, clon, lon_span, lat_span): (f64, f64, f64, f64),
    (cols, rows): (usize, usize),
    marks: &mut [u8],
) {
    let level = layer.level(lon_span / cols as f64);
    let (half_lon, half_lat) = (lon_span * 0.5, lat_span * 0.5);
    // Grid coordinates, in cells: (0,0) is the middle of the top-left one.
    let project = |lat: f64, lon: f64| {
        (
            (0.5 + wrap180(lon - clon) / lon_span) * cols as f64 - 0.5,
            (0.5 - (lat - clat) / lat_span) * rows as f64 - 0.5,
        )
    };
    for part in &level.parts {
        // Whole parts first, on the box each was measured into: at a continent
        // a time this is most of the world's rivers rejected on two subtractions.
        if wrap180(part.mid.1 - clon).abs() > half_lon + part.half.1
            || (part.mid.0 - clat).abs() > half_lat + part.half.0
        {
            continue;
        }
        let mut prev = project(f64::from(part.pts[0].0), f64::from(part.pts[0].1));
        for point in &part.pts[1..] {
            let cur = project(f64::from(point.0), f64::from(point.1));
            // A segment that leaves the map and comes back the other side is
            // the date line under the projection's wrap; drawn straight it
            // would be a scar across the whole map.
            let (dx, dy) = (cur.0 - prev.0, cur.1 - prev.1);
            if dx.abs() < cols as f64 {
                // Step along it half a cell at a time — half, so a diagonal
                // leaves no gaps at the corners.
                let steps = (dx.abs().max(dy.abs()) * 2.0).ceil().max(1.0);
                for k in 0..=(steps as usize) {
                    let f = k as f64 / steps;
                    let (x, y) = (prev.0 + dx * f, prev.1 + dy * f);
                    let (col, row) = (x.round(), y.round());
                    if col >= 0.0 && row >= 0.0 && col < cols as f64 && row < rows as f64 {
                        let i = row as usize * cols + col as usize;
                        marks[i] = marks[i].max(part.rank + 1);
                    }
                }
            }
            prev = cur;
        }
    }
}

/// The world itself: land, rivers, borders and the cities that fit, as a dot
/// matrix sized to the available pixels (about one dot every [`DOT_PITCH`]
/// points). Each cell maps to a (lat, lon) in the current view; longitude
/// wraps.
///
/// The land comes from the coverage raster the 3D globe is textured with
/// ([`crate::basemap`]), so a coastline is in the same place in both views; the
/// lines come from the polylines behind that raster, walked onto this same
/// grid — see [`stamp_lines`].
///
/// Returns the dot radius, which is the scale the callers draw their own
/// markers against.
pub(crate) fn draw_base(
    p: &eframe::egui::Painter,
    rect: eframe::egui::Rect,
    clat: f64,
    clon: f64,
    lon_span: f64,
    lat_span: f64,
    map: &theme::MapPalette,
) -> f32 {
    let cols = ((rect.width() / DOT_PITCH) as usize).max(24);
    let rows = ((rect.height() / DOT_PITCH) as usize).max(12);
    let cell_w = rect.width() / cols as f32;
    let cell_h = rect.height() / rows as f32;
    let dot_r = (cell_w.min(cell_h) * 0.44).max(0.7);
    let at = |col: usize, row: usize| {
        pos2(rect.left() + (col as f32 + 0.5) * cell_w, rect.top() + (row as f32 + 0.5) * cell_h)
    };

    // One sampler for the whole grid: the zoom is what picks the mip level, and
    // it is the same for every cell.
    let cell_deg = lon_span / cols as f64;
    let land = crate::basemap::land().sampler(cell_deg);
    let land_mag = land.magnification();
    for row in 0..rows {
        let fy = (row as f64 + 0.5) / rows as f64; // 0 top .. 1 bottom
        let lat = clat + (0.5 - fy) * lat_span;
        if !(-90.0..=90.0).contains(&lat) {
            continue; // beyond a pole → open space, no land
        }
        for col in 0..cols {
            let fx = (col as f64 + 0.5) / cols as f64; // 0 left .. 1 right
            let lon = wrap180(clon + (fx - 0.5) * lon_span);
            let ground = land_ink(land.at(lon, lat), land_mag);
            if ground > 0.02 {
                p.circle_filled(at(col, row), dot_r, alpha(map.land, 255.0 * ground));
            }
        }
    }

    // Rivers under borders: where the two run together — and they often do,
    // because a border is frequently a river somebody agreed on — the political
    // line is the one a callsign is looked up in.
    let view = (clat, clon, lon_span, lat_span);
    let mut marks = vec![0u8; cols * rows];
    let lines = crate::basemap::lines();
    // A river is drawn by the size Natural Earth ranks it at, so at a glance
    // the Danube reads as a different kind of thing from the brook feeding it.
    // A frontier has no such scale — a border is a border — so borders take the
    // whole layer at one weight.
    for (layer, ink, weight, base, spread) in [
        (&lines.rivers, map.river, 215.0, 0.42, 0.58),
        (&lines.borders, map.border, 195.0, 0.62, 0.0),
    ] {
        marks.fill(0);
        stamp_lines(layer, view, (cols, rows), &mut marks);
        for (i, &rank) in marks.iter().enumerate() {
            if rank == 0 {
                continue;
            }
            let a = base + spread * (f32::from(rank - 1) / 12.0).min(1.0);
            p.circle_filled(at(i % cols, i / cols), dot_r, alpha(ink, weight * a));
        }
    }
    if theme::map_cities() {
        draw_cities(p, rect, clat, clon, lon_span, lat_span, dot_r, map);
    }
    dot_r
}

/// The cities that fit, biggest first. Drawn only when the operator has left
/// them on ([`sdroxide_types::UiSettings::map_cities`]) — on a small panel map
/// a field of named grey dots is most of what is on it, and the contacts are
/// what somebody is trying to read (issue #312).
///
/// [`crate::basemap::cities`] is sorted by population, so "which cities show"
/// needs no threshold: walk the list, draw what falls inside the view, and stop
/// once the map holds as many as it has room for. Zooming in shrinks the view
/// faster than it exhausts the list, so smaller places arrive on their own.
///
/// Labels are the part that has to be earned — they cost far more room than the
/// dot does. One goes on only if the map is big enough to carry text at all and
/// the name misses every label already placed; the dot stays either way, so a
/// city crowded out of its name is still a mark on the map.
#[allow(clippy::too_many_arguments)]
fn draw_cities(
    p: &eframe::egui::Painter,
    rect: eframe::egui::Rect,
    clat: f64,
    clon: f64,
    lon_span: f64,
    lat_span: f64,
    dot_r: f32,
    map: &theme::MapPalette,
) {
    // How many cities the map has room for, by area: a 600×300 panel map takes
    // twenty, a full-window one a hundred and forty.
    let budget = ((rect.width() * rect.height()) / 9000.0) as usize;
    let budget = budget.clamp(6, 150);
    // Under this there is no room for a name next to the dot, and a map of
    // unlabelled specks says less than one without them.
    let label = rect.width() >= 260.0 && rect.height() >= 140.0;
    let font = FontId::proportional(9.5);

    let mut placed: Vec<Pos2> = Vec::with_capacity(budget);
    let mut labels: Vec<eframe::egui::Rect> = Vec::new();
    for city in crate::basemap::cities() {
        if placed.len() >= budget {
            break;
        }
        let dlon = wrap180(city.lon - clon);
        if dlon.abs() > lon_span / 2.0 || (city.lat - clat).abs() > lat_span / 2.0 {
            continue;
        }
        let c = pos2(
            rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
            rect.top() + (0.5 - ((city.lat - clat) / lat_span) as f32) * rect.height(),
        );
        // Two cities a few points apart are one smudge; the bigger one wins.
        if placed.iter().any(|q| q.distance(c) < 7.0) {
            continue;
        }
        placed.push(c);
        // The dot grows with the place, by decade of population: a map that
        // draws Tokyo and a county town the same size has thrown away the one
        // thing it knows about both. Capitals get a ring instead of a size —
        // a capital is not necessarily big, and this is the mark that says so.
        let r = dot_r.max(1.0) + 0.35 * ((city.pop.max(1) as f32).log10() - 4.0).clamp(0.0, 3.5);
        p.circle_filled(c, r + 1.2, alpha(map.city, 45.0));
        p.circle_filled(c, r, map.city);
        if city.capital {
            p.circle_stroke(c, r + 2.0, (0.7, alpha(map.city, 150.0)));
        }
        if !label {
            continue;
        }
        let galley = p.layout_no_wrap(city.name.to_owned(), font.clone(), map.city_label);
        let at = pos2(c.x + r + 3.0, c.y - galley.size().y / 2.0);
        let area = eframe::egui::Rect::from_min_size(at, galley.size()).expand(1.0);
        if !rect.contains_rect(area) || labels.iter().any(|l| l.intersects(area)) {
            continue;
        }
        labels.push(area);
        p.galley(at, galley, map.city_label);
    }
}

/// Draw the map filling the available width (2:1 aspect). `view` carries the
/// animated centre/zoom across frames. `home`/`dx`/`preview` are (lat, lon) in
/// degrees. `stations` is every decoded station still on the map — drawn as
/// white dots (their fade takes them out over time) under the coloured markers,
/// and used to drive the auto-zoom. (Their callsigns go unused here: at this
/// size a name per dot is a solid block of text. The globe, which has room,
/// draws them.) `preview` is a faint
/// marker for a decode the user clicked but hasn't answered yet (distinct colour
/// from the active DX). When `tx_active`, an animated pulse travels the home→dx
/// path. `max_h` caps the height: on short windows the map shrinks (keeping its
/// aspect, centered) rather than pushing the QSO controls off-screen.
///
/// Drag, wheel and pinch move the view by hand (see [`interact`]); doing so
/// suspends the auto-fit until a double-click reframes it.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut Ui,
    view: &mut MapView,
    home: Option<(f64, f64)>,
    dx: Option<(f64, f64)>,
    preview: Option<(f64, f64)>,
    hover: Option<(f64, f64)>,
    stations: &[crate::digi_map::DigiStation],
    // Network spots with a known location: (lat, lon, rgb tint by kind).
    spots: &[(f64, f64, (u8, u8, u8))],
    // Propagation heat, as an equirectangular RGBA image of the whole world
    // (see `crate::prop_map::PropHeat`). Painted under the continents, so the
    // coastline stays readable on top of it.
    heat: Option<eframe::egui::TextureId>,
    tx_active: bool,
    max_h: f32,
) {
    let avail_w = ui.available_width();
    if avail_w < MIN_HEIGHT {
        return;
    }
    // Fill the caller's width and its (user-draggable) height budget. The map is
    // no longer aspect-locked to 2:1, so it can be dragged taller than half its
    // width; the projection adapts (`lat_span = lon_span * aspect`). Capped at a
    // 1:1 aspect so it never becomes taller than wide.
    let h = max_h.min(avail_w).max(MIN_HEIGHT);
    let w = avail_w;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter_at(rect);
    let map = theme::map();
    p.rect_filled(rect, 0.0, map.sea);

    // ── Ease the view toward the target (fit home + all contacts) ──
    let aspect = (rect.height() / rect.width()) as f64;
    let mut contacts: Vec<(f64, f64)> = stations.iter().map(|d| (d.lat, d.lon)).collect();
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
    } else if view.manual {
        // The user is holding the view. Nothing eases it out from under them —
        // but a resized panel changes the aspect, so it still has to stay legal.
        view.clamp(aspect);
    } else {
        view.clat += (t_clat - view.clat) * EASE;
        view.clon = wrap180(view.clon + wrap180(t_clon - view.clon) * EASE);
        view.lon_span += (t_span - view.lon_span) * EASE;
        let settled = (view.clat - t_clat).abs() < 0.05
            && wrap180(t_clon - view.clon).abs() < 0.05
            && (view.lon_span - t_span).abs() < 0.05;
        if !settled {
            crate::repaint::after_ms(ui.ctx(), 16);
        }
    }
    // Mouse/touch pan and zoom, applied on top of (and suspending) the auto-fit.
    if interact(ui, view, &resp, aspect) {
        crate::repaint::animate(ui.ctx());
    }
    let (clat, clon, lon_span) = (view.clat, view.clon, view.lon_span);
    let lat_span = lon_span * aspect;

    // Propagation heat, first — under everything, because it is the ground the
    // map is drawn on rather than a thing on the map.
    //
    // One textured quad rather than a per-cell fill: the projection is linear
    // in latitude and longitude, so the whole world is an axis-aligned
    // rectangle here, and the texture's own bilinear filtering is what turns
    // 2.5° cells into soft shapes with no visible edges. The quad is repeated
    // sideways to cover a view that straddles the antimeridian; the painter's
    // clip rectangle trims what falls outside.
    if let Some(tex) = heat {
        let lon_to_x =
            |lon: f64| rect.left() + (0.5 + ((lon - clon) / lon_span) as f32) * rect.width();
        let lat_to_y =
            |lat: f64| rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height();
        let world = eframe::egui::Rect::from_min_max(
            pos2(lon_to_x(-180.0), lat_to_y(90.0)),
            pos2(lon_to_x(180.0), lat_to_y(-90.0)),
        );
        let world_w = world.width();
        if world_w > 1.0 {
            // Which copies of the world overlap what is on screen.
            let first = ((rect.left() - world.right()) / world_w).floor() as i32;
            let last = ((rect.right() - world.left()) / world_w).ceil() as i32;
            let uv = eframe::egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            for k in first..=last {
                let r = world.translate(vec2(k as f32 * world_w, 0.0));
                if r.intersects(rect) {
                    p.image(tex, r, uv, Color32::WHITE);
                }
            }
        }
    }

    let dot_r = draw_base(&p, rect, clat, clon, lon_span, lat_span, map);

    // Project (lat, lon) to screen using the current view; longitude wraps.
    let project = |lat: f64, lon: f64| -> Pos2 {
        let dlon = wrap180(lon - clon);
        let x = rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width();
        let y = rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height();
        pos2(x, y)
    };

    // Every decoded station with a known grid, as small neutral dots that fade
    // with age (`alpha`). The active DX, the clicked preview and home are
    // painted over these below, so a selected/answered station keeps its own
    // colour.
    for d in stations {
        if d.fade <= 0.0 {
            continue;
        }
        let c = project(d.lat, d.lon);
        p.circle_filled(c, 2.6, alpha(map.station, 55.0 * d.fade));
        p.circle_filled(c, 1.7, alpha(map.station, 255.0 * d.fade));
    }
    // Keep the slow fade progressing even after the zoom has settled.
    if !stations.is_empty() {
        crate::repaint::after_ms(ui.ctx(), 300);
    }

    // Network spots (DX cluster / POTA / SOTA / PSK) as small kind-coloured
    // diamonds — drawn under the home/DX markers so an active QSO stays clear.
    for &(lat, lon, rgb) in spots {
        let c = project(lat, lon);
        let kind = theme::data_ink(rgb);
        p.circle_filled(c, dot_r + 2.0, alpha(kind, 55.0));
        p.circle_filled(c, 2.2, kind);
    }

    // Great-circle path as a dotted cyan trail (dots avoid antimeridian wrap).
    if let (Some(hll), Some(dll)) = (home, dx) {
        for (lat, lon) in great_circle_points(hll, dll, 90) {
            p.circle_filled(project(lat, lon), dot_r.max(1.0), alpha(map.trail, 150.0));
        }
    }

    // Faint amber preview marker for a clicked-but-unanswered decode.
    if let Some((lat, lon)) = preview {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 3.0, alpha(map.preview, 45.0));
        p.circle_filled(c, 2.4, alpha(map.preview, 190.0));
    }

    // Endpoints with a glow.
    if let Some((lat, lon)) = home {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 3.0, alpha(map.home, 60.0));
        p.circle_filled(c, 2.6, map.home);
    }
    if let Some((lat, lon)) = dx {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 3.5, alpha(map.dx, 70.0));
        p.circle_filled(c, 3.0, map.dx);
    }
    // The decode row hovered in the table (drawn on top).
    if let Some((lat, lon)) = hover {
        let c = project(lat, lon);
        p.circle_filled(c, dot_r + 4.0, alpha(map.hover, 80.0));
        p.circle_filled(c, 3.2, map.hover);
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
                        let a = 150.0 - (k as f32) * 22.0;
                        p.circle_filled(project(la, lo), dot_r.max(1.2), alpha(map.comet, a));
                    }
                }
                // Bright leading head with a glow.
                let (la, lo) = pts[head];
                let c = project(la, lo);
                p.circle_filled(c, dot_r + 4.0, alpha(map.comet, 70.0));
                p.circle_filled(c, 3.2, map.station);
                // ~30 fps is plenty for the comet; an unconditional repaint
                // would drive the whole app at vsync rate during TX.
                crate::repaint::after_ms(ui.ctx(), 33);
            }
        }
    }

    // Once the user has taken the view, say how to give it back — but only
    // under the pointer, so an idle map stays a map and not a label.
    if view.manual && resp.hovered() {
        p.text(
            pos2(rect.right() - 5.0, rect.bottom() - 4.0),
            Align2::RIGHT_BOTTOM,
            "DOUBLE-CLICK TO REFRAME",
            FontId::proportional(9.0),
            map.hint,
        );
    }

    // Frame (red-accent, matching the QSO section panels).
    crate::chrome::paint_cut_border(&p, rect.shrink(0.5), map.frame, map.shell);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #312: the cities can be turned off, and turning them off takes the
    /// *names* with them — those are what get written across the dot somebody
    /// is trying to read on a panel map.
    ///
    /// Counted off the painter rather than eyeballed: `draw_base` puts land,
    /// rivers and borders down as circles and the city names as text, so the
    /// text count is the cities' labels and nothing else, and the circle count
    /// falling proves the dots went too.
    ///
    /// The setting is a process-wide atomic, so this is the only test here that
    /// may paint a basemap; another one doing so in parallel would read
    /// whichever value this had reached. It is put back on the way out.
    #[test]
    fn cities_can_be_turned_off_and_take_their_names_with_them() {
        use eframe::egui;

        fn count_shape(s: &egui::Shape, circles: &mut usize, texts: &mut usize) {
            match s {
                egui::Shape::Circle(_) => *circles += 1,
                egui::Shape::Text(_) => *texts += 1,
                egui::Shape::Vec(v) => {
                    for x in v {
                        count_shape(x, circles, texts);
                    }
                }
                _ => {}
            }
        }

        // Western Europe at a width that holds a great many cities, on a panel
        // big enough for the labels to be drawn at all.
        fn draw_counted() -> (usize, usize) {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let rect =
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
                let p = ui.painter_at(rect);
                draw_base(&p, rect, 50.0, 5.0, 40.0, 20.0, theme::map());
            });
            let (mut circles, mut texts) = (0usize, 0usize);
            for prim in &out.shapes {
                count_shape(&prim.shape, &mut circles, &mut texts);
            }
            // A frame's font atlas has to be taken or dropping the output
            // panics; nothing here draws it.
            out.textures_delta.clear();
            (circles, texts)
        }

        theme::set_map_cities(true);
        let (with_circles, with_texts) = draw_counted();
        assert!(with_texts > 0, "no city names were drawn to begin with");

        theme::set_map_cities(false);
        let (without_circles, without_texts) = draw_counted();
        assert_eq!(without_texts, 0, "{without_texts} names survived the setting");
        assert!(
            without_circles < with_circles,
            "the dots stayed too: {without_circles} against {with_circles}"
        );

        // Left as it is found: on by default, which is what every build before
        // this one drew.
        theme::set_map_cities(true);
        assert!(theme::map_cities());
    }

    /// Aspect of a typical map: half as tall as it is wide.
    const ASPECT: f64 = 0.5;

    /// Coverage is the alpha while the map is zoomed out, so a coast is an edge
    /// rather than a staircase — and becomes a hard line at the ½ contour once
    /// the dots have outrun the texels, so it does not dissolve into a gradient
    /// instead.
    #[test]
    fn the_coast_sharpens_as_the_map_zooms_in() {
        assert!((land_ink(0.5, 0.2) - 0.5).abs() < 1e-6);
        assert!(land_ink(0.75, 0.2) > 0.7 && land_ink(0.25, 0.2) < 0.3);
        assert!(land_ink(0.55, 20.0) > 0.99, "the coast blurred out at full zoom");
        assert!(land_ink(0.45, 20.0) < 0.01);
        // Open sea is never lit and solid ground always is, at any zoom.
        for mag in [0.05, 1.0, 30.0] {
            assert_eq!(land_ink(0.0, mag), 0.0);
            assert_eq!(land_ink(1.0, mag), 1.0);
        }
    }

    /// A border comes out one dot wide however far in the map is zoomed —
    /// which the raster this replaced could not manage: thresholded low it drew
    /// a band four dots across, thresholded high the same border broke into
    /// fragments wherever it fell between two texels.
    #[test]
    fn a_border_is_a_hairline_at_every_zoom() {
        let (cols, rows) = (200usize, 80usize);
        for span in [0.5, 2.0, 8.0, 40.0, 360.0] {
            let mut marks = vec![0u8; cols * rows];
            // Lake Constance, where Germany, Austria and Switzerland meet:
            // a frontier crosses the view at every one of these spans, and a
            // dozen of them do at the widest.
            let view = (47.55, 9.6, span, span * rows as f64 / cols as f64);
            stamp_lines(&crate::basemap::lines().borders, view, (cols, rows), &mut marks);
            let on = |c: isize, r: isize| {
                (0..cols as isize).contains(&c)
                    && (0..rows as isize).contains(&r)
                    && marks[r as usize * cols + c as usize] != 0
            };
            let (mut drawn, mut buried, mut lonely) = (0usize, 0usize, 0usize);
            for row in 0..rows as isize {
                for col in 0..cols as isize {
                    if !on(col, row) {
                        continue;
                    }
                    drawn += 1;
                    let around = (-1..=1)
                        .flat_map(|dr| (-1..=1).map(move |dc| (dc, dr)))
                        .filter(|d| *d != (0, 0))
                        .filter(|(dc, dr)| on(col + dc, row + dr))
                        .count();
                    // Every cell inside a band is surrounded by more band...
                    buried += usize::from(around == 8);
                    // ...and a fragmented line is dots with nothing beside them.
                    lonely += usize::from(around == 0);
                }
            }
            assert!(drawn > 40, "{span}°: only {drawn} cells drawn");
            assert!(buried * 50 < drawn, "{span}°: {buried} of {drawn} cells are inside a band");
            assert!(lonely * 20 < drawn, "{span}°: {lonely} of {drawn} cells stand alone");
        }
    }

    /// The date line is a seam in the projection, not in the world: a line
    /// crossing it is drawn on both sides rather than straight across the map.
    #[test]
    fn nothing_is_drawn_across_the_date_line() {
        let (cols, rows) = (200usize, 80usize);
        let mut marks = vec![0u8; cols * rows];
        // The Pacific, where Chukotka and Alaska sit either side of ±180°.
        let view = (64.0, 180.0, 60.0, 24.0);
        stamp_lines(&crate::basemap::lines().rivers, view, (cols, rows), &mut marks);
        for row in 0..rows {
            let filled = marks[row * cols..(row + 1) * cols].iter().filter(|m| **m != 0).count();
            assert!(filled * 3 < cols, "row {row} is {filled}/{cols} wide — a wrap scar");
        }
    }

    /// What the point at rect fraction (fx, fy) is over, in (lat, lon).
    fn under(v: &MapView, fx: f64, fy: f64, aspect: f64) -> (f64, f64) {
        (v.clat + (0.5 - fy) * v.lat_span(aspect), wrap180(v.clon + (fx - 0.5) * v.lon_span))
    }

    fn view(clat: f64, clon: f64, lon_span: f64) -> MapView {
        MapView { clat, clon, lon_span, initialized: true, manual: false }
    }

    /// A drag moves the land with the pointer, not against it: grabbing the map
    /// and pulling right brings what was to the west into view.
    #[test]
    fn a_drag_carries_the_land_with_it() {
        let mut v = view(0.0, 0.0, 180.0);
        // Half a map-width to the right.
        v.pan(0.5, 0.0, ASPECT);
        assert!((v.clon - -90.0).abs() < 1e-9, "clon = {}", v.clon);
        // And a quarter of its height down: the centre moves north.
        v.pan(0.0, 0.25, ASPECT);
        assert!((v.clat - 22.5).abs() < 1e-9, "clat = {}", v.clat);
    }

    /// The point under the cursor is the fixed point of a wheel zoom — that is
    /// the whole feel of "zoom in on *that*".
    #[test]
    fn zoom_keeps_what_is_under_the_cursor_under_it() {
        for (fx, fy) in [(0.5, 0.5), (0.2, 0.8), (0.93, 0.07)] {
            let mut v = view(15.0, 40.0, 200.0);
            let before = under(&v, fx, fy, ASPECT);
            v.zoom_about(0.4, fx, fy, ASPECT);
            let after = under(&v, fx, fy, ASPECT);
            assert!(
                (after.0 - before.0).abs() < 1e-9 && wrap180(after.1 - before.1).abs() < 1e-9,
                "({fx}, {fy}): {before:?} -> {after:?}"
            );
            assert!((v.lon_span - 80.0).abs() < 1e-9, "span = {}", v.lon_span);
        }
    }

    /// Zoom stops at both ends: the whole world out, the bitmap's own pixels in.
    #[test]
    fn zoom_stops_at_its_limits() {
        let mut v = view(0.0, 0.0, 360.0);
        v.zoom_about(4.0, 0.5, 0.5, ASPECT);
        assert!((v.lon_span - 360.0).abs() < 1e-9, "zoomed out past the world: {}", v.lon_span);
        for _ in 0..40 {
            v.zoom_about(0.5, 0.5, 0.5, ASPECT);
        }
        assert!(
            (v.lon_span - MIN_USER_LON_SPAN).abs() < 1e-9,
            "zoomed in past the floor: {}",
            v.lon_span
        );
    }

    /// Panning north cannot walk the window off the top of the world: the map
    /// stops with the pole at its edge rather than showing empty space.
    #[test]
    fn a_pan_stops_at_the_poles() {
        let mut v = view(0.0, 0.0, 90.0); // lat_span = 45
        for _ in 0..20 {
            v.pan(0.0, 0.5, ASPECT);
        }
        assert!((v.clat - (90.0 - 22.5)).abs() < 1e-9, "clat = {}", v.clat);
        // Longitude has no such edge — it wraps, and stays in [-180, 180).
        for _ in 0..20 {
            v.pan(-0.5, 0.0, ASPECT);
        }
        assert!((-180.0..180.0).contains(&v.clon), "clon = {}", v.clon);
    }

    /// A view zoomed out far enough to hold both poles centres itself on the
    /// equator instead of tipping to one side.
    #[test]
    fn a_whole_world_view_sits_on_the_equator() {
        let mut v = view(40.0, 0.0, 360.0);
        v.clamp(0.6); // lat_span = 216 > 180
        assert_eq!(v.clat, 0.0);
    }
}
