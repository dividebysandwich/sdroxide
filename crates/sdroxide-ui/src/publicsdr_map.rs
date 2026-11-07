//! The map view of the "PUBLIC SDRS" window: every receiver the chips and the
//! search let through, as a dot where its directory says it stands.
//!
//! The projection, the pan/zoom and the dotted continents are the FT8 panel
//! map's ([`crate::widgets::worldmap`]), the way the APRS map's are
//! ([`crate::aprs_map`]): one module owns where the world is. What is here is
//! what a directory of receivers needs:
//!
//! - a dot per receiver in its network's colour ([`network_colour`]), grey
//!   where the receiver cannot be taken, and its name beside it wherever the
//!   name has room ([`place_labels`]);
//! - an auto-fit to whatever is visible, so a search ("Japan", a callsign)
//!   flies the map to the answer;
//! - a click that picks *every* receiver under the pointer, not one: several
//!   KiwiSDRs at one site share a coordinate to the last digit, and a click
//!   that chose one of them by accident would hide the others.
//!
//! The window draws the picked receivers below the map as ordinary list rows,
//! so ADD AS RADIO is the same button in both views.
// Why: HDZ 2026-11-04: "eine Karte für die SDR-Stationen dran machen", "Straßen
//      interessieren nicht, nur Städte". The program's own Natural Earth map has
//      coastlines, borders, rivers and cities; OSM tiles (walkers) would have
//      added a dependency, network fetches and a tile policy for streets nobody
//      asked for, and would not run in the browser client.

use eframe::egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Ui, Vec2, pos2, vec2};
use sdroxide_types::{PublicSdrEntry, PublicSdrNetwork};

use crate::i18n::tr;
use crate::theme;
use crate::widgets::worldmap::{MapView, alpha, draw_base_cities, interact, wrap180};

/// Below this size, in points, the map is not worth drawing. Also the floor
/// for its width: a map narrower than it tall is a strip, not a map.
pub const MIN_HEIGHT: f32 = 120.0;

/// How close the pointer has to be to a dot to hover or pick it, in points.
const HIT_RADIUS: f32 = 7.0;

/// Most receivers one click picks. A city with a dozen KiwiSDRs is rare, and
/// the rows below the map take height from the map itself.
pub const MAX_PICKED: usize = 6;

/// A receiver dot's radius, in points.
const DOT_R: f32 = 3.2;

/// Never auto-fit tighter than this longitudinal span, in degrees: one receiver
/// found by its callsign should show its country, not its street. A hand zoom
/// goes further ([`MapView::clamp`] has its own floor).
const MIN_LON_SPAN: f64 = 6.0;

/// Extra margin left around the outermost receiver, as a factor on the span
/// they need: 1.25 leaves an eighth of the view free on each side.
const PAD: f64 = 1.25;

/// Per-frame ease toward the auto-fit: the fraction of the remaining way taken
/// each frame. 0.08 settles in about half a second at 60 frames a second.
const EASE: f64 = 0.08;

/// Past this many dots on screen only the picked receivers are named: the
/// world view has no room for a name anywhere, and trying every dot against
/// every other costs a frame for nothing.
const LABEL_DOTS_MAX: usize = 400;

/// The font the names beside the dots are written in. One function rather than
/// a literal at each use, because [`place_labels`] measures with it and
/// [`show`] draws with it, and the two must agree to the point.
fn label_font() -> FontId {
    FontId::proportional(9.5)
}

/// The map's own state, owned by the app so the view survives across frames
/// and across switching between LIST and MAP.
#[derive(Default)]
pub struct PublicSdrMapState {
    /// Centre and zoom, eased toward the auto-fit until the operator pans or
    /// zooms by hand; a double-click hands it back ([`interact`]).
    pub view: MapView,
    /// The receivers the last click picked, by [`key`], nearest first.
    pub picked: Vec<String>,
    /// The NAMES chip is off: only the picked receivers are named.
    // Why: HDZ 2026-11-04: "gönn der Karte einen Toggle-Knopf 'Labels', ich will
    //      nicht immer Labels sehen". Off rather than on as the stored flag, so
    //      the derived default is names shown.
    pub names_off: bool,
    /// The CITIES chip is off: this map without cities, whatever the shared
    /// setting says. Off as the stored flag for the same reason as `names_off`.
    // Why: HDZ 2026-11-06: "sonst brauchen die anderen Karten ein Städte-Toggle"
    //      (the FT8 map got one of its own).
    pub cities_off: bool,
    /// The connection check of each receiver a click picked, by [`key`].
    /// Kept across clicks, so picking a receiver again within
    /// [`CHECK_FRESH_S`] shows the answer it already has.
    pub checks: std::collections::HashMap<String, Check>,
    /// The map's size when the view was last fitted. A change — the window
    /// resized, the rows under the map grown by a pick — moves the view
    /// straight to the new fit instead of easing there ([`show`]).
    // Why: HDZ 2026-11-04: "wenn man die Map das erste Mal aufmacht und etwas
    //      auswählt, dann rutscht der Kartenzoom … das ist etwas fimmelig".
    //      Easing is for a search narrowing the receivers; a map that changes
    //      size under the pointer should not glide for half a second after it.
    pub fitted_size: Vec2,
}

/// How long after a click its receivers are checked, in seconds: long enough
/// that clicking along a cluster does not fire a check at every dot passed.
// Why: HDZ 2026-11-04: "ein basic Check nach 500 ms ein Connection-Test".
pub const CHECK_DELAY_S: f64 = 0.5;
/// How long an answer is trusted before a new pick checks again.
const CHECK_FRESH_S: f64 = 60.0;
/// How long a check may go unanswered before it counts as silent. Past the
/// three seconds a check takes and the queue of a remote station's one probe
/// worker in front of it.
const CHECK_GIVE_UP_S: f64 = 20.0;

/// Where one receiver's connection check stands. Times are Unix seconds.
///
/// Lifecycle: [`PublicSdrMapState::schedule_checks`] makes it `Due`,
/// [`PublicSdrMapState::due_checks`] turns `Due` into `Asked` (and an `Asked`
/// nobody answered into `Answered` with an error),
/// [`PublicSdrMapState::answered`] turns `Asked` into `Answered`.
#[derive(Debug, Clone, PartialEq)]
pub enum Check {
    /// To be asked at `at`, about `endpoint` (`host:port`).
    Due { at: f64, endpoint: String },
    /// Asked at `at`; the answer will name `endpoint`.
    Asked { at: f64, endpoint: String },
    /// `Ok` with what answered, `Err` with why nothing did.
    Answered { at: f64, result: Result<String, String> },
}

/// The endpoint a check asks about: the address the radio would connect to,
/// by the same rule the radio's own config applies (default port added).
///
/// In: a directory entry. Out: `host:port`, as in `"1.2.3.4:5555"`.
/// Fails: none; an address the config cannot make sense of comes back as it
/// was, and the check reports why it could not connect.
// Why: the check has to ask about the address ADD AS RADIO would use, or a
//      receiver could read ONLINE and then fail to open. Built through
//      `radio_config` rather than by appending a default port here, so the
//      two cannot drift apart.
// Pitfall: an sdr-list receiver goes through the KiwiSDR block, because
//          `PublicSdrEntry::backend` maps it there; its port is the one the
//          directory gave.
pub fn endpoint(e: &PublicSdrEntry) -> String {
    // A blank base: only this receiver's own block matters for the address.
    let cfg = e.radio_config(&sdroxide_types::RadioConfig::default(), false);
    match e.network {
        PublicSdrNetwork::SpyServer => cfg.spyserver.endpoint(),
        PublicSdrNetwork::KiwiSdr | PublicSdrNetwork::SdrList => cfg.kiwi.endpoint(),
    }
}

impl PublicSdrMapState {
    /// After a click: schedule a check of each picked receiver that has no
    /// fresh answer and none on the way, [`CHECK_DELAY_S`] from `now`.
    ///
    /// In: the receivers the click picked, now (Unix seconds). Effect: a `Due`
    /// entry in [`Self::checks`] for each that needs one. Nothing is sent
    /// here; [`Self::due_checks`] sends.
    pub fn schedule_checks(&mut self, picked: &[&PublicSdrEntry], now: f64) {
        for e in picked {
            let k = key(e);
            // "Fresh" means: nothing to do. An answer younger than
            // CHECK_FRESH_S is shown again; a check already on its way is
            // left to arrive rather than asked twice.
            let fresh = match self.checks.get(&k) {
                Some(Check::Answered { at, .. }) => now - at < CHECK_FRESH_S,
                Some(Check::Due { .. } | Check::Asked { .. }) => true,
                None => false,
            };
            if !fresh {
                self.checks
                    .insert(k, Check::Due { at: now + CHECK_DELAY_S, endpoint: endpoint(e) });
            }
        }
    }

    /// The checks to send now, marked as asked; and the ones asked too long
    /// ago, marked silent with `give_up` as the reason.
    ///
    /// In: now, the text for a check nobody answered. Out: (key, endpoint) of
    /// each check that falls due, for the still-picked receivers only — a pick
    /// replaced within the delay is never asked about.
    /// Effect: `Due` → `Asked` for what is returned; `Asked` older than
    /// [`CHECK_GIVE_UP_S`] → `Answered(Err(give_up))`; `Due` entries of
    /// receivers no longer picked are dropped.
    /// Fails: none.
    pub fn due_checks(&mut self, now: f64, give_up: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        // Only the current pick is walked: a check that fell due for a
        // receiver the operator has clicked away from is of no interest.
        for k in &self.picked {
            let Some(c) = self.checks.get_mut(k) else { continue };
            match c {
                // Its moment has come: hand it to the caller to ask, and
                // remember that it was asked, and when.
                Check::Due { at, endpoint } if *at <= now => {
                    out.push((k.clone(), endpoint.clone()));
                    *c = Check::Asked { at: now, endpoint: endpoint.clone() };
                }
                // Asked, and nothing came back in time — a dropped request
                // or a station that went away. Silent is the honest reading.
                Check::Asked { at, .. } if now - *at > CHECK_GIVE_UP_S => {
                    *c = Check::Answered { at: now, result: Err(give_up.to_string()) };
                }
                _ => {}
            }
        }
        // A pick replaced before its check fell due leaves nothing behind.
        let picked = &self.picked;
        self.checks.retain(|k, c| !matches!(c, Check::Due { .. }) || picked.contains(k));
        out
    }

    /// File an answer under every receiver asked about at that endpoint.
    ///
    /// In: the network and endpoint the answer names, the result, now.
    /// Effect: each `Asked` check on that network and endpoint becomes
    /// `Answered`. An answer nobody is waiting for (the check gave up, or the
    /// map state was reset) is dropped.
    // Invariant: matched on network *and* endpoint. Two networks can list the
    //            same host and port (an sdr-list receiver that is also a
    //            KiwiSDR), and the answer to one says nothing about the other.
    pub fn answered(
        &mut self,
        network: PublicSdrNetwork,
        address: &str,
        result: &Result<String, String>,
        now: f64,
    ) {
        // The network part of [`key`], so a key can be matched without the
        // entry it came from.
        let prefix = format!("{}|", network.label());
        for (k, c) in self.checks.iter_mut() {
            if let Check::Asked { endpoint, .. } = c
                && endpoint == address
                && k.starts_with(&prefix)
            {
                *c = Check::Answered { at: now, result: result.clone() };
            }
        }
    }

    /// Whether a picked receiver's check is still to come, so the window keeps
    /// asking for frames until it arrives.
    ///
    /// Out: true while any picked receiver is `Due` or `Asked`.
    // Why: egui draws only when something happens; without frames neither the
    //      500 ms delay nor an answer arriving from a thread would ever show.
    pub fn checks_pending(&self) -> bool {
        self.picked
            .iter()
            .any(|k| matches!(self.checks.get(k), Some(Check::Due { .. } | Check::Asked { .. })))
    }
}

/// What identifies one receiver across directory refreshes: its network and
/// the address it is reached at. The name is not unique ("anonymous").
///
/// In: a directory entry. Out: `"<network label>|<address>"`, as in
/// `"SpyServer|1.2.3.4:5555"`.
// Invariant: the network label comes first and ends at the `|`;
//            [`PublicSdrMapState::answered`] matches on that prefix.
pub fn key(e: &PublicSdrEntry) -> String {
    format!("{}|{}", e.network.label(), e.address)
}

/// The position a receiver is drawn at, if its directory gave one.
///
/// In: a directory entry. Out: (latitude, longitude) in degrees, widened to
/// `f64` for the projection; `None` when either is missing (7 of 1,149 in the
/// directories on 2026-11-04), and such a receiver gets no dot.
fn position(e: &PublicSdrEntry) -> Option<(f64, f64)> {
    match (e.lat, e.lon) {
        (Some(lat), Some(lon)) => Some((f64::from(lat), f64::from(lon))),
        _ => None,
    }
}

/// The view to ease toward.
///
/// In: the visible receivers' positions, home, the map's aspect (height/width).
/// Out: (centre lat, centre lon, lon span). Every receiver framed with a margin,
/// centred on their centroid; when that takes the whole world, centred on home
/// instead, so the operator's own side of the globe is the middle of it.
fn target_view(pts: &[(f64, f64)], home: Option<(f64, f64)>, aspect: f64) -> (f64, f64, f64) {
    if pts.is_empty() {
        return (0.0, home.map_or(0.0, |h| h.1), 360.0);
    }
    // The centroid, with longitudes taken relative to the first point so a
    // cluster across the date line (178°, -178°) averages to 180°, not 0°.
    let n = pts.len() as f64;
    let lon_ref = pts[0].1;
    let clat = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let clon = wrap180(lon_ref + pts.iter().map(|p| wrap180(p.1 - lon_ref)).sum::<f64>() / n);
    // The farthest receiver from the centre in each direction decides the span.
    let mut max_dlat = 0.0f64;
    let mut max_dlon = 0.0f64;
    for &(lat, lon) in pts {
        max_dlat = max_dlat.max((lat - clat).abs());
        max_dlon = max_dlon.max(wrap180(lon - clon).abs());
    }
    // Whichever of the two needs more room sets the zoom; the latitude need is
    // turned into a longitude span through the map's aspect, because the
    // projection is linear in both and the view is given by its lon span.
    let need_lon = 2.0 * max_dlon * PAD;
    let need_lat = 2.0 * max_dlat * PAD;
    let lon_span = need_lon.max(need_lat / aspect.max(1e-3)).clamp(MIN_LON_SPAN, 360.0);
    // The whole world: turn it so home is in the middle.
    let clon = match home {
        Some(h) if lon_span >= 360.0 => h.1,
        _ => clon,
    };
    // Keep the view inside the poles, as `MapView::clamp` does for a hand pan.
    let lat_span = (lon_span * aspect).min(180.0);
    let clat = if lat_span >= 180.0 {
        0.0
    } else {
        clat.clamp(-90.0 + lat_span / 2.0, 90.0 - lat_span / 2.0)
    };
    (clat, clon, lon_span)
}

/// A receiver's colour on the map, one per network, all from the palette.
///
/// In: the network, the map's palette. Out: KiwiSDR the primary accent (cyan),
/// SpyServer the map's own station colour (white on a dark map, ink on a light
/// one), sdr-list green.
// Why: HDZ 2026-11-04: "die rote Farbe für die Spy Server als Symbol ist auch
//      nicht so gut, nimm lieber eine andere Farbe, die die Palette des Programms
//      hergibt". The list's pink is the map's pick ring and reads as an alarm on
//      a dot; the palette has no blue besides cyan (Kiwi) and cyan_dim, which on
//      a 3 pt dot is not told from cyan and sits on the river blue.
fn network_colour(n: PublicSdrNetwork, map: &theme::MapPalette) -> eframe::egui::Color32 {
    match n {
        PublicSdrNetwork::KiwiSdr => theme::CYAN(),
        PublicSdrNetwork::SpyServer => map.station,
        PublicSdrNetwork::SdrList => theme::GREEN(),
    }
}

/// Where the names go: beside every dot whose name fits.
///
/// In: the map rect, the dots on screen, the picked keys, whether the NAMES
/// chip is on. Out: the receivers to name and the rect each name takes. Picked
/// receivers are tried first, then the free ones, then those that cannot be
/// taken; with the chip off, or past [`LABEL_DOTS_MAX`] dots, only the picked
/// ones.
// Why: HDZ 2026-11-04: "wenn ich SpyServer inaktiviere, dann zeigen Kiwi-Server
//      Labels an, sonst nur blau". Names used to be all-or-nothing at 30 dots on
//      screen, so a network chip switched every name in view on or off, and the
//      names that were drawn ran over one another. Placing each where it has
//      room makes the names follow the zoom, not the filter.
fn place_labels<'a>(
    p: &eframe::egui::Painter,
    bounds: Rect,
    screen: &[(&'a PublicSdrEntry, Pos2)],
    picked: &[String],
    names_on: bool,
) -> Vec<(&'a PublicSdrEntry, Rect)> {
    let is_picked = |e: &PublicSdrEntry| picked.iter().any(|k| *k == key(e));
    // Whether anything but the picked receivers is named at all.
    let all = names_on && screen.len() <= LABEL_DOTS_MAX;
    // The order names are tried in, which is the order they win space in:
    // tier 0 picked, tier 1 free, tier 2 cannot be taken.
    let mut order: Vec<usize> = Vec::new();
    for tier in 0..3 {
        for (i, (e, _)) in screen.iter().enumerate() {
            let t = if is_picked(e) {
                0
            } else if e.blocked_reason().is_none() {
                1
            } else {
                2
            };
            if t == tier && (all || t == 0) {
                order.push(i);
            }
        }
    }
    // Measured with the font they are drawn in, so a placed name is exactly
    // the rect it takes. The colour does not change the size.
    let dots: Vec<Pos2> = screen.iter().map(|(_, c)| *c).collect();
    let sizes: Vec<Vec2> = screen
        .iter()
        .map(|(e, _)| p.layout_no_wrap(e.name.clone(), label_font(), Color32::WHITE).size())
        .collect();
    fit_labels(bounds, &dots, &sizes, &order).into_iter().map(|(i, r)| (screen[i].0, r)).collect()
}

/// Greedy label placement, the geometry half of [`place_labels`].
///
/// In: the bounds, every dot, every label's size (same index), the order to
/// try them in. Out: (index, rect) for each label placed. A label goes right of
/// its dot, else left of it; it is placed only inside the bounds, clear of
/// every label placed before it and of every other dot.
/// Fails: none; a label with no room is left out, never squeezed.
fn fit_labels(bounds: Rect, dots: &[Pos2], sizes: &[Vec2], order: &[usize]) -> Vec<(usize, Rect)> {
    /// Space between a dot and its name, in points.
    const GAP: f32 = 3.0;
    /// Space kept free around each placed name, in points.
    const MARGIN: f32 = 1.5;
    let mut placed: Vec<(usize, Rect)> = Vec::new();
    for &i in order {
        // The two places a name may go, vertically centred on its dot.
        let (c, s) = (dots[i], sizes[i]);
        let right = Rect::from_min_size(pos2(c.x + DOT_R + GAP, c.y - s.y / 2.0), s);
        let left = Rect::from_min_size(pos2(c.x - DOT_R - GAP - s.x, c.y - s.y / 2.0), s);
        // Room means: inside the map, clear of every name already placed, and
        // not over any other receiver's dot (its own dot is beside it).
        let fits = |r: Rect| {
            bounds.contains_rect(r)
                && !placed.iter().any(|(_, q)| q.expand(MARGIN).intersects(r))
                && !dots.iter().enumerate().any(|(j, d)| j != i && r.expand(DOT_R).contains(*d))
        };
        // Right first, the way a map is read; left only if right is taken.
        if let Some(r) = [right, left].into_iter().find(|r| fits(*r)) {
            placed.push((i, r));
        }
    }
    placed
}

/// The hover text for one receiver: name, place, band, users, distance, and
/// the reason it cannot be taken where there is one.
///
/// In: the entry, home. Out: two to four lines, the first unindented so a
/// tooltip listing several receivers reads as one block per receiver, as in
/// `"KiwiSDR · DF0HQ\n   Dortmund\n   0–30 MHz · 2/8 · 12 km"`. Each part is
/// left out where the directory gave nothing.
fn tip_line(e: &PublicSdrEntry, home: Option<(f64, f64)>) -> String {
    let mut s = format!("{} · {}", e.network.label(), e.name);
    if !e.location.trim().is_empty() {
        s.push_str(&format!("\n   {}", e.location.trim()));
    }
    s.push_str(&format!("\n   {}", e.range_label()));
    if e.max_users > 0 {
        s.push_str(&format!(" · {}/{}", e.users, e.max_users));
    }
    if let (Some(h), Some(p)) = (home, position(e)) {
        s.push_str(&format!(" · {:.0} km", sdroxide_types::distance_km(h, p)));
    }
    if let Some(why) = e.blocked_reason() {
        s.push_str(&format!("\n   {why}"));
    }
    s
}

/// Draw the map of `entries` into a rect `h` points high.
///
/// In: the receivers the window's filters let through, home (from the station
/// grid), the height to fill. Out: true when a click changed
/// [`PublicSdrMapState::picked`] (the window then shows those receivers' rows).
/// Effect: pans/zooms `state.view`; a click on dots sets `picked` to every
/// receiver within [`HIT_RADIUS`] of it, nearest first, at most [`MAX_PICKED`];
/// a click on empty map clears it.
/// Fails: none; receivers without a position are left off (the window counts
/// them), and a map narrower than [`MIN_HEIGHT`] is not drawn.
pub fn show(
    ui: &mut Ui,
    state: &mut PublicSdrMapState,
    entries: &[&PublicSdrEntry],
    home: Option<(f64, f64)>,
    h: f32,
) -> bool {
    // Too small to be a map: draw nothing and leave the space to the rows.
    let w = ui.available_width();
    if w < MIN_HEIGHT || h < MIN_HEIGHT {
        return false;
    }
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return false;
    }
    // The theme's map colours, and a painter clipped to the map so nothing
    // drawn near the edge spills into the rows below.
    let map = theme::map();
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, map.sea);

    // ── the view ──
    // Height over width: how much latitude a given longitude span shows.
    let aspect = (rect.height() / rect.width()) as f64;
    // Only receivers with a position take part in anything below.
    let placed: Vec<(&PublicSdrEntry, (f64, f64))> =
        entries.iter().filter_map(|e| position(e).map(|q| (*e, q))).collect();
    let pts: Vec<(f64, f64)> = placed.iter().map(|(_, q)| *q).collect();
    let (t_clat, t_clon, t_span) = target_view(&pts, home, aspect);
    let resized = state.fitted_size != rect.size();
    state.fitted_size = rect.size();
    let view = &mut state.view;
    // A NaN from a bad input event would otherwise survive every clamp (#656).
    view.sanitize();
    // First frame, or the map changed size: straight to the fit. Hand-moved:
    // stay, only kept legal. Otherwise: ease toward the fit, so a search
    // glides rather than jumps.
    if !view.initialized || (resized && !view.manual) {
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
        // Frames only while moving; a settled map costs nothing.
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

    // Sea, land, borders, rivers and cities, as every flat map in the program.
    draw_base_cities(&p, rect, clat, clon, lon_span, lat_span, map, !state.cities_off);

    // (lat, lon) in degrees to a point on screen: linear in both, longitude
    // wrapped around the centre so the world repeats sideways.
    let project = |(lat, lon): (f64, f64)| -> Pos2 {
        let dlon = wrap180(lon - clon);
        pos2(
            rect.left() + (0.5 + (dlon / lon_span) as f32) * rect.width(),
            rect.top() + (0.5 - ((lat - clat) / lat_span) as f32) * rect.height(),
        )
    };

    // Screen positions once, and everything under the pointer, nearest first.
    // A dot half off the edge still counts: it is drawn clipped and can be
    // hovered.
    let screen: Vec<(&PublicSdrEntry, Pos2)> = placed
        .iter()
        .map(|(e, q)| (*e, project(*q)))
        .filter(|(_, c)| rect.expand(DOT_R).contains(*c))
        .collect();
    let under = |m: Pos2| -> Vec<(usize, f32)> {
        let mut v: Vec<(usize, f32)> = screen
            .iter()
            .enumerate()
            .map(|(i, (_, c))| (i, c.distance(m)))
            .filter(|&(_, d)| d <= HIT_RADIUS)
            .collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        v
    };
    let hovered: Vec<(usize, f32)> = resp.hover_pos().map(under).unwrap_or_default();

    // ── receivers: the ones that cannot be taken first, under the rest ──
    // Grey for a receiver that cannot be taken, whatever its network: the
    // reason is in its tooltip and in its row.
    let colour = |e: &PublicSdrEntry| {
        if e.blocked_reason().is_some() { theme::gray(110) } else { network_colour(e.network, map) }
    };
    for pass_blocked in [true, false] {
        for (e, c) in &screen {
            let blocked = e.blocked_reason().is_some();
            if blocked != pass_blocked {
                continue;
            }
            // A halo, so a dot over the stippled land stays readable.
            p.circle_filled(*c, DOT_R + 1.6, alpha(map.sea, 170.0));
            p.circle_filled(*c, DOT_R, alpha(colour(e), if blocked { 170.0 } else { 235.0 }));
        }
    }

    // ── names, wherever one fits ──
    for (e, rect_l) in place_labels(&p, rect, &screen, &state.picked, !state.names_off) {
        p.text(
            rect_l.left_center(),
            Align2::LEFT_CENTER,
            &e.name,
            label_font(),
            alpha(colour(e), 210.0),
        );
    }

    // ── picked and hovered, on top ──
    // A ring in the pick colour around each picked receiver, so the dots the
    // rows below belong to can be found again.
    for (e, c) in &screen {
        if state.picked.iter().any(|k| *k == key(e)) {
            p.circle_stroke(*c, DOT_R + 3.5, (1.6, map.dx));
        }
    }
    // The nearest dot under the pointer, lit and redrawn over its neighbours.
    if let Some(&(i, _)) = hovered.first() {
        let (e, c) = screen[i];
        p.circle_filled(c, DOT_R + 1.5, map.hover);
        p.circle_filled(c, DOT_R, network_colour(e.network, map));
    }

    // ── us ──
    // Home from the station grid, drawn like the FT8 and APRS maps draw it.
    if let Some(hq) = home {
        let c = project(hq);
        p.circle_filled(c, 7.0, alpha(map.home, 55.0));
        p.circle_filled(c, 3.4, map.home);
        p.circle_stroke(c, 6.5, (1.2, alpha(map.home, 170.0)));
    }

    // ── interaction ──
    let mut changed = false;
    if !hovered.is_empty() {
        // One block per receiver under the pointer, and a count of the rest.
        let tip = hovered
            .iter()
            .take(MAX_PICKED)
            .map(|&(i, _)| tip_line(screen[i].0, home))
            .collect::<Vec<_>>()
            .join("\n");
        let more = hovered.len().saturating_sub(MAX_PICKED);
        let tip = if more > 0 { format!("{tip}\n+{more}") } else { tip };
        resp.clone().on_hover_text(tip);
        // A click picks all of them (at most MAX_PICKED, nearest first) and
        // schedules their connection checks.
        if resp.clicked() {
            let chosen: Vec<&PublicSdrEntry> =
                hovered.iter().take(MAX_PICKED).map(|&(i, _)| screen[i].0).collect();
            state.picked = chosen.iter().map(|e| key(e)).collect();
            state.schedule_checks(&chosen, crate::time::now_unix_f64());
            changed = true;
        }
    } else if resp.clicked() && !state.picked.is_empty() {
        // A click on open sea or land lets go of the pick.
        state.picked.clear();
        changed = true;
    }

    // Hand-moved: say how to give the view back to the auto-fit.
    if manual && resp.hovered() {
        p.text(
            rect.right_bottom() + vec2(-6.0, -4.0),
            Align2::RIGHT_BOTTOM,
            tr!("aprsmap-double-click-reframe"),
            FontId::proportional(9.0),
            map.hint,
        );
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty map shows the world, turned to home's longitude.
    #[test]
    fn an_empty_map_shows_the_world_around_home() {
        let (lat, lon, span) = target_view(&[], Some((51.5, 7.5)), 0.5);
        assert_eq!(span, 360.0);
        assert_eq!(lat, 0.0);
        assert_eq!(lon, 7.5);
    }

    /// One receiver found by its callsign shows its country, not its street.
    #[test]
    fn one_receiver_is_framed_with_a_floor() {
        let (lat, lon, span) = target_view(&[(35.0, 139.0)], None, 0.5);
        assert!(span >= MIN_LON_SPAN);
        assert!((lat - 35.0).abs() < 1e-9 && (lon - 139.0).abs() < 1e-9);
    }

    /// Receivers on both sides of the date line are framed across it, not
    /// around the whole world the other way.
    #[test]
    fn the_date_line_does_not_split_a_cluster() {
        let (_, lon, span) = target_view(&[(-40.0, 178.0), (-41.0, -178.0)], None, 0.5);
        assert!(span < 30.0, "{span}");
        assert!(lon.abs() > 170.0, "{lon}");
    }

    /// Proves: a lone dot is named, to its right ([`fit_labels`]).
    #[test]
    fn a_lone_dot_gets_its_name_on_the_right() {
        let b = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 200.0));
        let got = fit_labels(b, &[pos2(100.0, 100.0)], &[vec2(60.0, 12.0)], &[0]);
        assert_eq!(got.len(), 1);
        assert!(got[0].1.left() > 100.0);
    }

    /// Proves: two dots side by side never get overlapping names — the second
    /// goes left or is left out — and no name covers another dot.
    #[test]
    fn names_never_overlap_and_never_cover_a_dot() {
        let b = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 200.0));
        let dots = [pos2(100.0, 100.0), pos2(104.0, 104.0), pos2(130.0, 100.0)];
        let sizes = [vec2(60.0, 12.0); 3];
        let got = fit_labels(b, &dots, &sizes, &[0, 1, 2]);
        for (a, (i, ra)) in got.iter().enumerate() {
            for (j, rb) in got.iter().skip(a + 1).map(|(j, r)| (*j, *r)) {
                assert!(!ra.intersects(rb), "labels {i} and {j} overlap");
            }
            for (k, d) in dots.iter().enumerate() {
                assert!(k == *i || !ra.contains(*d), "label {i} covers dot {k}");
            }
        }
    }

    /// Proves: a name that would leave the map is put on the other side, and
    /// one with room on neither is dropped.
    #[test]
    fn a_name_at_the_edge_flips_or_is_dropped() {
        let b = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
        let got = fit_labels(b, &[pos2(190.0, 50.0)], &[vec2(60.0, 12.0)], &[0]);
        assert_eq!(got.len(), 1);
        assert!(got[0].1.right() < 190.0, "flipped to the left");
        let got = fit_labels(b, &[pos2(100.0, 50.0)], &[vec2(150.0, 12.0)], &[0]);
        assert!(got.is_empty(), "no room either side");
    }

    /// A SpyServer at `address`, enough for the check logic.
    fn spy(address: &str) -> PublicSdrEntry {
        PublicSdrEntry {
            network: PublicSdrNetwork::SpyServer,
            name: "anonymous".into(),
            location: String::new(),
            antenna: String::new(),
            device: "AirspyOne".into(),
            address: address.into(),
            lat: Some(1.0),
            lon: Some(2.0),
            grid: String::new(),
            min_hz: 0.0,
            max_hz: 1e9,
            users: 0,
            max_users: 4,
            api_channels: None,
            max_iq_rate: 0.0,
            full_control: true,
            session_limit_min: 0,
            snr_db: None,
        }
    }

    /// Proves: a click's receivers are asked about [`CHECK_DELAY_S`] after it,
    /// not before, once, and the answer finds its row by endpoint.
    #[test]
    fn a_pick_is_checked_after_the_delay_and_answered_by_endpoint() {
        let (a, b) = (spy("1.2.3.4:5555"), spy("5.6.7.8"));
        let mut st = PublicSdrMapState::default();
        st.picked = vec![key(&a), key(&b)];
        st.schedule_checks(&[&a, &b], 100.0);
        assert!(st.due_checks(100.3, "x").is_empty(), "not before the delay");
        let due = st.due_checks(100.6, "x");
        assert_eq!(due.len(), 2);
        assert!(due.iter().any(|(_, ep)| ep == "5.6.7.8:5555"), "default port added: {due:?}");
        assert!(st.due_checks(100.7, "x").is_empty(), "asked once");
        assert!(st.checks_pending());
        st.answered(PublicSdrNetwork::SpyServer, "1.2.3.4:5555", &Ok("up".into()), 101.0);
        assert_eq!(st.checks[&key(&a)], Check::Answered { at: 101.0, result: Ok("up".into()) });
        assert!(matches!(st.checks[&key(&b)], Check::Asked { .. }), "the other still waits");
    }

    /// Proves: a check nobody answers turns silent after [`CHECK_GIVE_UP_S`],
    /// and a fresh answer is reused rather than asked again.
    #[test]
    fn an_unanswered_check_turns_silent_and_a_fresh_one_is_kept() {
        let a = spy("1.2.3.4:5555");
        let mut st = PublicSdrMapState::default();
        st.picked = vec![key(&a)];
        st.schedule_checks(&[&a], 0.0);
        st.due_checks(1.0, "none");
        st.due_checks(1.0 + CHECK_GIVE_UP_S + 1.0, "none");
        assert!(
            matches!(&st.checks[&key(&a)], Check::Answered { result: Err(m), .. } if m == "none")
        );
        let before = st.checks[&key(&a)].clone();
        st.schedule_checks(&[&a], 30.0);
        assert_eq!(st.checks[&key(&a)], before, "fresh: not scheduled again");
        st.schedule_checks(&[&a], 30.0 + CHECK_FRESH_S);
        assert!(matches!(st.checks[&key(&a)], Check::Due { .. }), "stale: checked again");
    }

    /// Proves: a pick replaced before its check fell due is never asked about.
    #[test]
    fn a_replaced_pick_is_not_checked() {
        let (a, b) = (spy("1.2.3.4:5555"), spy("5.6.7.8:5555"));
        let mut st = PublicSdrMapState::default();
        st.picked = vec![key(&a)];
        st.schedule_checks(&[&a], 0.0);
        st.picked = vec![key(&b)];
        st.schedule_checks(&[&b], 0.2);
        let due = st.due_checks(1.0, "x");
        assert_eq!(due, vec![(key(&b), "5.6.7.8:5555".to_string())]);
        assert!(!st.checks.contains_key(&key(&a)));
    }

    /// The key tells two "anonymous" SpyServers apart by their address.
    #[test]
    fn the_key_is_network_and_address() {
        let mut e = PublicSdrEntry {
            network: PublicSdrNetwork::SpyServer,
            name: "anonymous".into(),
            location: String::new(),
            antenna: String::new(),
            device: "AirspyOne".into(),
            address: "1.2.3.4:5555".into(),
            lat: Some(1.0),
            lon: Some(2.0),
            grid: String::new(),
            min_hz: 0.0,
            max_hz: 1e9,
            users: 0,
            max_users: 4,
            api_channels: None,
            max_iq_rate: 0.0,
            full_control: true,
            session_limit_min: 0,
            snr_db: None,
        };
        let a = key(&e);
        e.address = "5.6.7.8:5555".into();
        assert_ne!(a, key(&e));
    }
}
