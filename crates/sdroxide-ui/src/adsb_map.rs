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

use eframe::egui::{
    Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2,
};
use sdroxide_types::{AdsbAircraft, AdsbSettings, Vfo};

use crate::theme;
use crate::widgets::map_labels::{self, MapLabel};
use crate::widgets::worldmap::{
    MapView, alpha, draw_painted_base, interact_with, paint_night, wrap180,
};

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
    /// The map's optional layers — the flight paths, and the airports and
    /// navigation aids underneath them. Chips on the map pane's own row.
    pub layers: MapLayers,
    /// The aircraft whose card is open, by ICAO address.
    pub selected: Option<u32>,
    /// The map under the picture: land, rivers and borders as an image rather
    /// than the dot stipple the propagation maps use. Rebuilt only when the
    /// view moves by more than a texel.
    pub ground: crate::widgets::worldmap::GroundCache,
}

/// Which of the map's optional layers are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLayers {
    /// **The flight's path**: where it has been and where it is going, as one
    /// line through it rather than a scatter of history dots.
    pub paths: bool,
    /// **Airports and radio navigation aids** — the ground truth under the
    /// picture, from the public-domain tables in [`crate::airdata`].
    pub nav: bool,
}

impl Default for MapLayers {
    fn default() -> Self {
        // Both on. They are what the picture is for, and each has a chip to
        // turn it off — a layer nobody can find is a layer that does not exist.
        Self { paths: true, nav: true }
    }
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

/// Airports and radio navigation aids, filtered by zoom.
///
/// **The filter is the feature.** The tables hold 11 008 navaids and 5 280
/// airports, and drawing them all at every zoom is a wall of idents with a map
/// somewhere behind it. So the view decides: large airports always, medium ones
/// once they are places rather than specks, navaids only once a station is
/// something you could point an aerial at — which is also the zoom at which the
/// **MF ones become useful to a listener**, because those are receivable.
fn draw_nav_layer(
    p: Option<&Painter>,
    map: &theme::MapPalette,
    layers: MapLayers,
    clat: f64,
    clon: f64,
    lon_span: f64,
    lat_span: f64,
    project: &impl Fn(f64, f64) -> Pos2,
    pointer: Option<Pos2>,
) -> Option<NavHover> {
    if !layers.nav || !crate::airdata::loaded() {
        return None;
    }
    let zoom = nav_zoom(lon_span);
    let (half_lat, half_lon) = (lat_span * 0.5, lon_span * 0.5);
    let in_view = |lat: f64, lon: f64| {
        (lat - clat).abs() <= half_lat && wrap180(lon - clon).abs() <= half_lon
    };
    let font = FontId::monospace(9.0);
    // Only what is *drawn* can be hovered — see the note on the filter above.
    // A name for something invisible would be a map that answers questions
    // about things that are not on it.
    let mut hit: Option<(f32, Pos2, String, Option<u32>)> = None;

    for a in crate::airdata::airports() {
        if (!a.large && !zoom.medium_airports) || !in_view(a.lat, a.lon) {
            continue;
        }
        let c = project(a.lat, a.lon);
        if let Some(p) = p {
            p.circle_stroke(c, 2.6, Stroke::new(1.0, alpha(map.border, 150.0)));
            if a.large {
                p.circle_filled(c, 1.2, alpha(map.city_label, 200.0));
            }
            if zoom.airport_labels && a.large {
                let name = if a.iata.is_empty() { a.ident.as_str() } else { a.iata.as_str() };
                p.text(
                    c + vec2(5.0, 3.0),
                    Align2::LEFT_TOP,
                    name,
                    font.clone(),
                    alpha(map.city_label, 190.0),
                );
            }
        }
        if let Some(m) = pointer {
            let d = c.distance(m);
            if d <= NAV_HIT_RADIUS && hit.as_ref().is_none_or(|(best, _, _, _)| d < *best) {
                // An airport carries no frequency in the source table: what is
                // tuned is the *aid* — a VOR or an NDB — which is the row below.
                hit = Some((d, c, airport_hover_text(a), None));
            }
        }
    }

    if !zoom.navaids {
        return hit.map(|(_, at, text, freq_khz)| NavHover { at, text, freq_khz });
    }
    for n in crate::airdata::navaids() {
        if !in_view(n.lat, n.lon) {
            continue;
        }
        let c = project(n.lat, n.lon);
        // A listener's aid is a different colour from a pilot's: an NDB is on
        // medium wave and can be *heard*, and that is the half of this table
        // this fork cares most about.
        let ink = if n.kind.is_audible_on_mf() { map.home } else { map.river };
        if let Some(p) = p {
            if n.kind.is_audible_on_mf() {
                p.circle_filled(c, 1.5, alpha(ink, 210.0));
            } else {
                // The triangle a chart draws for a VOR family aid.
                p.add(Shape::closed_line(
                    vec![c + vec2(0.0, -2.8), c + vec2(2.4, 1.7), c + vec2(-2.4, 1.7)],
                    Stroke::new(1.0, alpha(ink, 210.0)),
                ));
            }
        }
        if let Some(m) = pointer {
            let d = c.distance(m);
            if d <= NAV_HIT_RADIUS && hit.as_ref().is_none_or(|(best, _, _, _)| d < *best) {
                hit = Some((d, c, navaid_hover_text(n), (n.freq_khz != 0).then_some(n.freq_khz)));
            }
        }
        if let Some(p) = p
            && zoom.navaid_labels
            && !n.ident.is_empty()
        {
            p.text(
                c + vec2(4.5, 3.0),
                Align2::LEFT_TOP,
                &n.ident,
                font.clone(),
                alpha(map.city_label, 200.0),
            );
        }
    }
    hit.map(|(_, at, text, freq_khz)| NavHover { at, text, freq_khz })
}

/// Where a coordinate lands on the map, for a view and a rect. One function:
/// the hover pick, the drawing and the pre-pick that decides whether a
/// double-click is a tune or a reframe all have to agree about it.
fn project_at(
    rect: Rect,
    clat: f64,
    clon: f64,
    lon_span: f64,
    lat_span: f64,
    lat: f64,
    lon: f64,
) -> Pos2 {
    let dlon = wrap180(lon - clon);
    pos2(
        rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
        rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
    )
}

/// The name of whatever the pointer is over, drawn at it.
///
/// Drawn rather than handed to egui as a tooltip: the map is one widget, so a
/// tooltip could only say "somewhere on the map", and what the operator wants
/// is the name beside the symbol they are pointing at.
fn draw_hover_label(p: &Painter, rect: Rect, map: &theme::MapPalette, hover: &NavHover) {
    // The gesture is *said*, where the station's name is read: a station that can
    // be tuned has to look like one, or the feature is invisible. Only the ones
    // that are on a frequency say it.
    let hint =
        hover.freq_khz.is_some().then_some("double-click to tune · shift for the other dial");
    let galley =
        p.layout_no_wrap(hover.text.clone(), FontId::monospace(10.0), alpha(map.city_label, 255.0));
    let hint_galley = hint.map(|h| {
        p.layout_no_wrap(h.to_owned(), FontId::proportional(9.0), alpha(map.city_label, 190.0))
    });
    let pad = vec2(6.0, 4.0);
    let hint_h = hint_galley.as_ref().map_or(0.0, |g| g.size().y + 2.0);
    let size = galley.size() + pad * 2.0 + vec2(0.0, hint_h);
    let mut at = hover.at + vec2(11.0, -size.y - 7.0);
    // Kept on the map: a name for something at the edge is the one that most
    // needs to move, because the auto-fit leaves the outermost targets close to
    // the edge.
    at.x = at.x.clamp(rect.left() + 2.0, (rect.right() - size.x - 2.0).max(rect.left() + 2.0));
    at.y = at.y.clamp(rect.top() + 2.0, (rect.bottom() - size.y - 2.0).max(rect.top() + 2.0));
    let box_ = Rect::from_min_size(at, size);
    p.rect_filled(box_, 3.0, alpha(map.sea, 240.0));
    p.rect_stroke(
        box_,
        3.0,
        Stroke::new(1.0, alpha(map.border, 210.0)),
        eframe::egui::StrokeKind::Inside,
    );
    let text_h = galley.size().y;
    p.galley(box_.min + pad, galley, map.city_label);
    if let Some(g) = hint_galley {
        p.galley(box_.min + pad + vec2(0.0, text_h + 2.0), g, map.city_label);
    }
    // A ring on the symbol itself, so the name is attached to a mark.
    p.circle_stroke(hover.at, 4.5, Stroke::new(1.3, alpha(map.city_label, 220.0)));
}

/// What a hovered airport says: the abbreviation first, because that is what
/// the operator asked for and what a chart writes, then the name.
fn airport_hover_text(a: &crate::airdata::Airport) -> String {
    if a.iata.is_empty() {
        format!("{} · {}", a.ident, a.name)
    } else {
        format!("{} · {}", a.iata, a.name)
    }
}

/// What a hovered navaid says: its ident, what kind of aid it is, and the
/// frequency on the unit it is published in — **kHz for the medium-wave ones,
/// because those are audible**, which is the half of this table a listener
/// cares about.
fn navaid_hover_text(n: &crate::airdata::Navaid) -> String {
    let freq = if n.kind.is_audible_on_mf() {
        format!("{} kHz (medium wave)", n.freq_khz)
    } else {
        format!("{:.3} MHz", n.freq_khz as f64 / 1000.0)
    };
    if n.name.is_empty() {
        format!("{} · {} · {}", n.ident, n.kind.label(), freq)
    } else {
        format!("{} · {} · {} — {}", n.ident, n.kind.label(), freq, n.name)
    }
}

/// A feature the pointer is over, what to say about it, and — for the stations
/// that are on a radio frequency — where to tune to hear it.
struct NavHover {
    at: Pos2,
    text: String,
    freq_khz: Option<u32>,
}

/// **Where a double-clicked station goes: the *other* dial, always.**
///
/// Never the one in use, and that is a property of the mode rather than a
/// preference. This map is the ADS-B panel, and ADS-B is a channel: entering it
/// tunes the active VFO to 1090 MHz and switching back to that VFO puts it there
/// again — *"ADS-B is on 1090 MHz; tuning there"*, in the engine's own words. A
/// frequency written to that dial is overwritten the moment the VFO is recalled,
/// which is exactly the fault the operator reported before he traced it himself:
/// *"the mode is ADS-B so it wants to switch back to the ADS-B frequency… in that
/// mode you cannot set frequency A because the mode dictates"*. His conclusion,
/// and his rule for this gesture: *"if you want to click a frequency it always
/// has to go to B"*.
#[must_use]
pub fn other_dial(active: Vfo) -> Vfo {
    match active {
        Vfo::A => Vfo::B,
        Vfo::B => Vfo::A,
    }
}

/// The dial's frequency for a station's published one.
///
/// The tables are in **kHz** (`airdata::Navaid::freq_khz` — "the unit both kinds
/// are published in") and a dial is in Hz, which is the conversion this fork has
/// got wrong before in the other direction.
#[must_use]
pub fn tune_hz(freq_khz: u32) -> f64 {
    freq_khz as f64 * 1000.0
}

/// What a double-click on the map asked for: a station to listen to, on the dial
/// the current mode does not own (see [`other_dial`]).
pub struct MapTune {
    /// Where to tune, in Hz.
    pub hz: f64,
    /// The dial to put it on.
    pub vfo: Vfo,
}

/// How close the pointer has to be to a symbol to name it. Wider than the
/// aircraft's hit radius: these are small marks on a busy map, and the operator
/// is asking about a place rather than selecting a target.
const NAV_HIT_RADIUS: f32 = 9.0;

/// What the view's span allows the navigation layer to draw.
///
/// **The filter is the feature.** The tables hold 11 008 navaids and 5 280
/// airports; drawn all at once at every zoom they are a wall of idents with a
/// map somewhere behind. Each threshold opens as the view tightens and none
/// ever closes again going in — `nav_zoom` is a step function of the span, and
/// that monotonicity is the property the test pins, because a layer that
/// *loses* detail as you zoom in is the bug this shape exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NavZoom {
    medium_airports: bool,
    airport_labels: bool,
    navaids: bool,
    navaid_labels: bool,
}

fn nav_zoom(lon_span: f64) -> NavZoom {
    NavZoom {
        medium_airports: lon_span <= 40.0,
        airport_labels: lon_span <= 12.0,
        // Below this a station is a place rather than a speck — and it is also
        // the zoom at which the medium-wave beacons become a listening list
        // rather than decoration.
        navaids: lon_span <= 20.0,
        navaid_labels: lon_span <= 6.0,
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
    night: Option<eframe::egui::TextureId>,
    style: sdroxide_types::MapStyle,
    max_h: f32,
    active_vfo: Vfo,
) -> Option<MapTune> {
    let avail_w = ui.available_width();
    if avail_w < MIN_HEIGHT {
        return None;
    }
    let h = max_h.min(avail_w).max(MIN_HEIGHT);
    let (rect, resp) = ui.allocate_exact_size(vec2(avail_w, h), Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return None;
    }
    let map = &theme::map_styled(style);
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
    // **Is the double-click for a station, or for the view?** Asked from the
    // view as it stands *before* the interaction, because a double-click never
    // pans — the coordinates are the ones the pick will use a moment later.
    // Painting nothing: the same function, asked a question.
    let station_under_pointer = {
        let pre = |lat: f64, lon: f64| {
            project_at(rect, view.clat, view.clon, view.lon_span, view.lon_span * aspect, lat, lon)
        };
        draw_nav_layer(
            None,
            map,
            state.layers,
            view.clat,
            view.clon,
            view.lon_span,
            view.lon_span * aspect,
            &pre,
            resp.hover_pos(),
        )
        .and_then(|h| h.freq_khz)
        .is_some()
    };
    if interact_with(ui, view, &resp, aspect, !(resp.double_clicked() && station_under_pointer)) {
        crate::repaint::animate(ui.ctx());
    }
    let (clat, clon, lon_span) = (view.clat, view.clon, view.lon_span);
    let lat_span = lon_span * aspect;
    let manual = view.manual;

    // A painted map, not the dot stipple: an air picture is looked at, and a
    // stippled ground is the propagation panel's idiom (see `draw_painted_base`).
    let dot_r = draw_painted_base(
        ui.ctx(),
        &p,
        rect,
        clat,
        clon,
        lon_span,
        lat_span,
        map,
        style,
        &mut state.ground,
    );
    // Over the continents, so land darkens with the sea on the night side; the
    // marks drawn below stay lit.
    paint_night(&p, rect, clat, clon, lon_span, lat_span, night);

    let project = |lat: f64, lon: f64| project_at(rect, clat, clon, lon_span, lat_span, lat, lon);

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

    // ── airports and navaids, at the very bottom ──
    let nav_hover = draw_nav_layer(
        Some(&p),
        map,
        state.layers,
        clat,
        clon,
        lon_span,
        lat_span,
        &project,
        pointer,
    );

    // **A double-click on a station tunes it.** The map is where the stations
    // are *seen*; without this the operator reads a frequency off the hover text
    // and types it into a box. Shift puts it on the other dial, which is the A/B
    // pair an operator listening to two stations at once is already using.
    let mut tune: Option<MapTune> = None;
    if resp.double_clicked()
        && let Some(khz) = nav_hover.as_ref().and_then(|h| h.freq_khz)
    {
        tune = Some(MapTune { hz: tune_hz(khz), vfo: other_dial(active_vfo) });
    }

    // ── history, under everything else ──
    for a in &live {
        let n = a.track.len();
        if state.layers.paths && n >= 2 {
            // **The flight's path**: the history joined into one line, and run
            // through the aircraft's position so it carries on into the leader
            // drawn below — where it has been and where it is going read as one
            // thing rather than as a scatter of dots and a stub.
            let mut pts: Vec<Pos2> =
                a.track.iter().map(|&(la, lo)| project(f64::from(la), f64::from(lo))).collect();
            if let Some((lat, lon)) = a.lat.zip(a.lon) {
                pts.push(project(lat, lon));
            }
            p.add(Shape::line(pts, Stroke::new(1.2, alpha(map.trail, 130.0))));
            continue;
        }
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

    // ── the name of whatever the pointer is over, on top of all of it ──
    // An aircraft wins: it carries its own data block, and a place name drawn
    // over it would be answering a different question than the one being asked.
    if hover.is_none() {
        if let Some(h) = &nav_hover {
            draw_hover_label(&p, rect, map, h);
        }
    }

    // ── interaction ──
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
            tip.push_str(&format!("\n{vr:+} ft/min"));
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
            state.selected = Some(a.icao);
        }
    } else if resp.clicked() {
        state.selected = None;
    }

    // Only where it is true: over a station the double-click tunes instead, and a
    // hint that promises the wrong gesture is worse than none.
    if manual && resp.hovered() && !station_under_pointer {
        p.text(
            rect.right_bottom() + vec2(-6.0, -4.0),
            Align2::RIGHT_BOTTOM,
            "double-click to reframe",
            FontId::proportional(9.0),
            alpha(Color32::WHITE, 110.0),
        );
    }
    // A **tune wins over the aircraft pick**: a double-click on a station is a
    // request for a frequency, and the click that selected an aircraft under it
    // is the lesser answer.
    tune
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A hover names the thing under the pointer**, in the words a chart
    /// would use: the abbreviation first for an airport, and for a navaid its
    /// ident, what it is, and the frequency on the unit it is published in —
    /// kHz and "medium wave" for the ones a listener can actually hear.
    #[test]
    fn what_a_hovered_nav_feature_says() {
        let ams = crate::airdata::airports()
            .iter()
            .find(|a| a.iata == "AMS")
            .expect("Schiphol is in the table");
        // The name is the source's, and the field is wide enough to hold it
        // whole — the first cut gave names 20 bytes and the hover read
        // "Amsterdam Airport Sc". Asserted from the table rather than from
        // memory: this line has been wrong twice.
        assert_eq!(airport_hover_text(ams), "AMS · Amsterdam Airport Schiphol");

        // An airport with no IATA code falls back to its ident rather than
        // showing a gap where the abbreviation should be.
        let mut unnamed = ams.clone();
        unnamed.iata.clear();
        unnamed.ident = "EHAM".into();
        assert!(airport_hover_text(&unnamed).starts_with("EHAM · "));

        // A medium-wave beacon says kHz, because that is what a receiver dial
        // is marked in, and says that it is audible.
        let ndb = crate::airdata::navaids()
            .iter()
            .find(|n| n.kind == crate::airdata::NavaidKind::Ndb)
            .expect("there are NDBs");
        let text = navaid_hover_text(ndb);
        assert!(text.contains("NDB"), "{text}");
        assert!(text.contains("kHz (medium wave)"), "{text}");

        // And a VHF aid says MHz, to three places — the spacing the band uses.
        let vor = crate::airdata::navaids()
            .iter()
            .find(|n| n.kind == crate::airdata::NavaidKind::VorDme)
            .expect("there are VOR/DMEs");
        let text = navaid_hover_text(vor);
        assert!(text.contains("VOR/DME"), "{text}");
        assert!(text.contains("MHz"), "{text}");
        assert!(!text.contains("kHz"), "{text}");
    }

    /// **The hover finds the airport under the pointer**, on the real tables
    /// and the real filter — not a re-implementation of either.
    ///
    /// `draw_nav_layer` takes a painter and numbers, so it can be driven without
    /// a window: a painter over a rect, a span, and a pointer placed where
    /// Schiphol projects to. That is the whole path the mouse takes.
    #[test]
    fn the_hover_finds_the_airport_under_the_pointer() {
        let ctx = eframe::egui::Context::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
        // Schiphol, and a view tight enough that the layer is switched on.
        let (clat, clon, lon_span) = (52.3, 4.75, 6.0);
        let lat_span = lon_span * 0.66;
        let project = |lat: f64, lon: f64| {
            pos2(
                rect.left() + (0.5 + (wrap180(lon - clon) / lon_span) as f32) * rect.width(),
                rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
            )
        };
        let map = theme::map_styled(sdroxide_types::MapStyle::Radar);
        let ams = crate::airdata::airports()
            .iter()
            .find(|a| a.iata == "AMS")
            .expect("Schiphol is in the table");
        let at = project(ams.lat, ams.lon);
        let sea = pos2(rect.left() + 3.0, rect.top() + 3.0);

        // Inside a frame: egui has no fonts until the first `run`, and the
        // layer lays text out.
        let mut asked = Vec::new();
        let input = eframe::egui::RawInput { screen_rect: Some(rect), ..Default::default() };
        let full = ctx.run_ui(input, |ui| {
            let p = ui.painter().clone();
            for (layers, pointer) in [
                (MapLayers { paths: true, nav: true }, Some(at)),
                (MapLayers { paths: true, nav: true }, Some(sea)),
                (MapLayers { paths: true, nav: false }, Some(at)),
            ] {
                asked.push(draw_nav_layer(
                    Some(&p),
                    &map,
                    layers,
                    clat,
                    clon,
                    lon_span,
                    lat_span,
                    &project,
                    pointer,
                ));
            }
        });
        full.drop_without_applying_deltas();

        let named = asked[0].as_ref().expect("the pointer is on the airport and nothing was named");
        assert!(named.text.starts_with("AMS · "), "named {:?}", named.text);
        assert!(
            asked[1].is_none(),
            "the sea named something: {:?}",
            asked[1].as_ref().map(|h| &h.text)
        );
        assert!(
            asked[2].is_none(),
            "the layer is off and the hover still answered — a name for a mark that is not drawn"
        );
    }

    /// **A station with a frequency says so, and one without does not.** The
    /// hover is what offers the gesture, and the frequency it carries is what a
    /// double-click tunes to — so this is the whole feature's data path, read
    /// from an aid the table actually has.
    #[test]
    fn the_hover_carries_the_frequency_a_station_can_be_tuned_to() {
        let ctx = eframe::egui::Context::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
        let map = theme::map_styled(sdroxide_types::MapStyle::Radar);
        // An aid that is on a frequency, and the view tight enough for the
        // navaids to be drawn at all.
        let ndb = crate::airdata::navaids()
            .iter()
            .find(|n| n.freq_khz > 0)
            .expect("a navaid with a frequency");
        let (clat, clon) = (ndb.lat, ndb.lon);
        let lon_span = 0.5;
        let lat_span = lon_span * 0.66;
        let project =
            |lat: f64, lon: f64| project_at(rect, clat, clon, lon_span, lat_span, lat, lon);
        let at = project(ndb.lat, ndb.lon);
        let input = eframe::egui::RawInput { screen_rect: Some(rect), ..Default::default() };
        let mut hover = None;
        let full = ctx.run_ui(input, |ui| {
            let p = ui.painter().clone();
            hover = draw_nav_layer(
                Some(&p),
                &map,
                MapLayers { paths: false, nav: true },
                clat,
                clon,
                lon_span,
                lat_span,
                &project,
                Some(at),
            );
        });
        full.drop_without_applying_deltas();
        let hover = hover.expect("the pointer is on the aid and nothing was named");
        assert_eq!(
            hover.freq_khz,
            Some(ndb.freq_khz),
            "the hover did not carry the frequency its own text names"
        );
        assert!(hover.text.contains(&ndb.ident), "named the wrong thing: {:?}", hover.text);
        // And the conversion the dial is given: kHz in the table, Hz on the dial.
        assert!((tune_hz(108_400) - 108_400_000.0).abs() < 1.0, "108 400 kHz is 108.4 MHz");
    }

    /// **A station goes to the dial the mode is not using**, and that A/B pair is
    /// the operator's own reason for the gesture — the plane on one dial, the
    /// ground station on the other, switched between with the VFO key.
    #[test]
    fn a_tuned_station_goes_to_the_dial_the_mode_does_not_own() {
        assert_eq!(other_dial(Vfo::A), Vfo::B, "ADS-B holds A, so the station takes B");
        assert_eq!(other_dial(Vfo::B), Vfo::A);
    }

    /// **The filter is the feature, and it only opens.** Zooming in must never
    /// take something away: a layer that loses the medium airports when you get
    /// close, or the navaids when you get closer, reads as the map breaking
    /// rather than as a rule.
    #[test]
    fn the_navigation_layer_only_ever_opens_as_you_zoom_in() {
        // World scale down to the tightest this map goes.
        let spans = [360.0, 180.0, 60.0, 41.0, 40.0, 20.0, 12.0, 6.0, 3.0, 0.5];
        let steps: Vec<NavZoom> = spans.iter().map(|s| nav_zoom(*s)).collect();
        for pair in steps.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                (!a.medium_airports || b.medium_airports)
                    && (!a.airport_labels || b.airport_labels)
                    && (!a.navaids || b.navaids)
                    && (!a.navaid_labels || b.navaid_labels),
                "zooming in from {:?} to {:?} took something away",
                a,
                b
            );
        }
        // And the ends are what they should be: a world view is large airports
        // only, and the closest view labels everything.
        assert!(!steps[0].navaids && !steps[0].medium_airports, "the world view is crowded");
        assert!(
            steps[steps.len() - 1].navaids && steps[steps.len() - 1].navaid_labels,
            "the closest view does not name the stations"
        );
    }

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
