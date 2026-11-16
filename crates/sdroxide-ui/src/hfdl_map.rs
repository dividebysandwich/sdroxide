//! The HFDL aircraft picture: every aeroplane the ground network has located,
//! drawn over the same dotted continents the FT8, ADS-B and AIS maps use
//! ([`crate::widgets::worldmap`]) so all of them agree about where the world is.
//!
//! HFDL is not ADS-B. A position arrives only when an aircraft sends a
//! performance-data or frequency-data record — a downlink every few minutes at
//! best, not twice a second — and it carries no altitude, speed or track for
//! the map to draw a leader from. What it does carry is identity: the flight
//! number, and often the ICAO address resolved from the logon-confirm cache. So
//! the symbol is a plain square with the flight beside it, the plot refreshes
//! in place as later fixes for the same aircraft arrive, and nothing is drawn
//! that the data cannot support.
//!
//! The table here is the map's own, not the decode log's: an aircraft stays on
//! the picture while its fixes keep coming, even after its earliest decodes
//! have scrolled out of the log's rolling window. It is retired only when
//! nothing has refreshed it for [`PLOT_STALE_S`], which is far longer than an
//! aircraft stays in range of one ground station.

use crate::i18n::tr;
use std::collections::BTreeMap;

use eframe::egui::{Align2, FontId, Pos2, Rect, Sense, Ui, pos2, vec2};
use sdroxide_types::HfdlDecode;

use crate::theme;
use crate::widgets::map_labels::{self, MapLabel};
use crate::widgets::worldmap::{MapView, alpha, draw_base, interact, wrap180};

/// Below this height the map is not worth drawing.
pub const MIN_HEIGHT: f32 = 90.0;

/// How close a click has to land, in points.
const HIT_RADIUS: f32 = 14.0;

/// Half the side of a target square, in points.
const TARGET_R: f32 = 3.0;

/// Never auto-fit tighter than this longitudinal span (degrees). HFDL fixes are
/// intercontinental; one aircraft overhead must not blow the map up to street
/// level.
const MIN_LON_SPAN: f64 = 5.0;

/// Extra margin left around the outermost target.
const PAD: f64 = 1.25;

/// Per-frame ease toward the auto-fit.
const EASE: f64 = 0.06;

/// How long an un-refreshed plot stays on the map. An aircraft is in range of
/// one ground station for minutes; half an hour of silence means it has gone,
/// and a marker that old would be a stale claim about where it is.
pub const PLOT_STALE_S: i64 = 30 * 60;

/// How many past positions a plot keeps and draws behind it. HFDL fixes are
/// minutes apart, so five is most of an hour of flight — enough to show which
/// way the aircraft is going and how fast, not so many that a busy ocean turns
/// to dots.
pub const TRAIL_MAX: usize = 5;

/// A past position nearer than this to the one after it is dropped: the same
/// fix reported twice (a performance-data and a frequency-data record from the
/// same minute) would otherwise stack two dots on one spot.
const TRAIL_MIN_STEP_KM: f64 = 2.0;

/// A past position older than this is dropped however few there are, so a
/// trail never reaches back across a gap in which the aircraft could have gone
/// anywhere.
pub const TRAIL_STALE_S: i64 = 60 * 60;

/// How long a plot blinks after the list asks for it, in seconds. Long enough
/// to find it by eye across a world map, short enough not to be left running.
pub const BLINK_S: f64 = 3.0;

/// Blinks per second: a plot lit for a quarter of a second, dark for a quarter.
const BLINK_HZ: f64 = 2.0;

/// Where a blink is at `now` seconds when it began at `start`: `None` once it
/// is over, else `(lit, ring)` — whether the square is in its lit half of the
/// cycle, and how far (0..1) the ring that spreads from it has got. Pure, so
/// the timing is testable without a frame.
pub(crate) fn blink_phase(start: f64, now: f64) -> Option<(bool, f32)> {
    let t = now - start;
    if !(0.0..BLINK_S).contains(&t) {
        return None;
    }
    let cycle = t * BLINK_HZ;
    Some((cycle.fract() < 0.5, cycle.fract() as f32))
}

/// An HFDL ground station, as the map draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct MapStation {
    pub id: u8,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    /// How long ago it was heard, while that is recent enough to count; `None`
    /// for a station known only from the table.
    pub heard_age: Option<i64>,
}

/// The town of a station's name, for a label: `Shannon, Ireland` → `Shannon`.
fn town(name: &str) -> &str {
    name.split(',').next().unwrap_or(name).trim()
}

/// Projected points of a path, split wherever it crosses the map's edge — a
/// step longer than half the map's width is the path leaving one side and
/// coming back on the other, and joining it would draw a line across the whole
/// map.
pub(crate) fn split_on_wrap(pts: &[Pos2], width: f32) -> Vec<Vec<Pos2>> {
    let mut out: Vec<Vec<Pos2>> = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    for &q in pts {
        if let Some(last) = cur.last()
            && (q.x - last.x).abs() > width / 2.0
        {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(q);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.retain(|seg| seg.len() >= 2);
    out
}

/// A position an aircraft reported before its latest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrailPoint {
    pub lat: f64,
    pub lon: f64,
    /// Unix second the fix was decoded.
    pub at: i64,
}

/// One aircraft's latest known position, as the map holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct HfdlPlot {
    /// The flight, ICAO or alias to label it by.
    pub label: String,
    pub lat: f64,
    pub lon: f64,
    /// Unix second of the newest fix folded in.
    pub last_at: i64,
    /// EVM-derived SNR of that fix's burst, where the demod measured one.
    pub snr_db: Option<f32>,
    /// The channel the fix arrived on, in kHz.
    pub freq_khz: u32,
    /// How many fixes this aircraft has contributed — a lone fix is a plot, a
    /// dozen is a track.
    pub fixes: u32,
    /// Where it was before [`Self::lat`]/[`Self::lon`]: at most [`TRAIL_MAX`]
    /// positions, oldest first, none within [`TRAIL_MIN_STEP_KM`] of the
    /// next, none older than [`TRAIL_STALE_S`].
    pub trail: Vec<TrailPoint>,
}

impl HfdlPlot {
    /// File a past position, unless one is already held for that second.
    fn remember(&mut self, pt: TrailPoint) {
        if pt.at != self.last_at && !self.trail.iter().any(|t| t.at == pt.at) {
            self.trail.push(pt);
        }
    }

    /// Put the trail in order and within bounds: oldest first, expired points
    /// gone, a point that sits on the one after it gone, and only the newest
    /// [`TRAIL_MAX`] kept. Idempotent, so it can run after every snapshot.
    fn tidy_trail(&mut self, now: i64) {
        self.trail.sort_by_key(|t| t.at);
        self.trail.retain(|t| now - t.at <= TRAIL_STALE_S);
        let mut kept: Vec<TrailPoint> = Vec::with_capacity(self.trail.len());
        let mut next = (self.lat, self.lon);
        for t in self.trail.iter().rev() {
            if sdroxide_types::distance_km(next, (t.lat, t.lon)) >= TRAIL_MIN_STEP_KM {
                kept.push(*t);
                next = (t.lat, t.lon);
            }
        }
        kept.reverse();
        let excess = kept.len().saturating_sub(TRAIL_MAX);
        kept.drain(..excess);
        self.trail = kept;
    }
}

/// The map's state, owned by the app so the view survives across frames and the
/// table survives the window being closed, and re-opened.
#[derive(Default)]
pub struct HfdlMapState {
    pub view: MapView,
    /// The aircraft whose details are shown, by [`HfdlPlot`] key.
    pub selected: Option<String>,
    /// Latest fix per identity, keyed by [`sdroxide_types::HfdlFix::key`].
    plots: BTreeMap<String, HfdlPlot>,
    /// The plot asked to blink, the time (egui seconds) it started, and the
    /// ground station the aircraft is talking to — the line to it is drawn for
    /// as long as the blink lasts, and only then.
    blink: Option<(String, f64, Option<u8>)>,
}

impl HfdlMapState {
    /// Fold a status snapshot's log — newest first — into the table.
    ///
    /// Idempotent: a fix already held at the same timestamp is left alone, so a
    /// snapshot re-sent (they carry the whole log every time) cannot inflate the
    /// fix count. Only a strictly newer fix moves a plot.
    pub fn observe(&mut self, log: &[HfdlDecode], now: i64) {
        for d in log {
            let Some(fix) = &d.position else { continue };
            let key = fix.key();
            let here = TrailPoint { lat: fix.lat, lon: fix.lon, at: d.unix };
            match self.plots.get_mut(&key) {
                Some(p) if d.unix > p.last_at => {
                    // The position it was at becomes the newest of its trail.
                    let was = TrailPoint { lat: p.lat, lon: p.lon, at: p.last_at };
                    p.label = fix.label();
                    p.lat = fix.lat;
                    p.lon = fix.lon;
                    p.last_at = d.unix;
                    p.snr_db = d.snr_db;
                    p.freq_khz = d.freq_khz;
                    p.fixes = p.fixes.saturating_add(1);
                    p.remember(was);
                }
                // An older fix than the one held — the log carries several per
                // aircraft, newest first — is a place it has been.
                Some(p) => p.remember(here),
                None => {
                    self.plots.insert(
                        key,
                        HfdlPlot {
                            label: fix.label(),
                            lat: fix.lat,
                            lon: fix.lon,
                            last_at: d.unix,
                            snr_db: d.snr_db,
                            freq_khz: d.freq_khz,
                            fixes: 1,
                            trail: Vec::new(),
                        },
                    );
                }
            }
        }
        for p in self.plots.values_mut() {
            p.tidy_trail(now);
        }
        // Retire what nothing has refreshed for a long time.
        self.plots.retain(|_, p| now - p.last_at <= PLOT_STALE_S);
    }

    /// The aircraft currently plotted, oldest-heard first (a stable order, so
    /// the map's tie-breaks do not shuffle between frames).
    pub fn plots(&self) -> impl Iterator<Item = (&String, &HfdlPlot)> {
        self.plots.iter()
    }

    pub fn len(&self) -> usize {
        self.plots.len()
    }

    /// Make the plot under `key` blink for [`BLINK_S`] seconds, starting at
    /// `now` (egui seconds), and select it. Returns whether there is such a
    /// plot — an aircraft that has sent no position has nothing to blink.
    ///
    /// The view is handed back to the auto-fit, which frames every plot, so a
    /// plot panned or zoomed out of sight is in view for the blink.
    ///
    /// `gs` is the ground station the aircraft is talking to: while it blinks a
    /// line is drawn from it to that station, where the station's position is
    /// known.
    pub fn blink(&mut self, key: &str, now: f64, gs: Option<u8>) -> bool {
        if !self.plots.contains_key(key) {
            return false;
        }
        self.blink = Some((key.to_owned(), now, gs));
        self.selected = Some(key.to_owned());
        self.view.manual = false;
        true
    }

    /// Whether an aircraft is plotted under `key` — so a list row can select
    /// its plot only when there is one to select.
    pub fn has(&self, key: &str) -> bool {
        self.plots.contains_key(key)
    }

    pub fn is_empty(&self) -> bool {
        self.plots.is_empty()
    }
}

/// The view to ease toward: every plot, plus us, framed.
pub(crate) fn target_view(
    home: Option<(f64, f64)>,
    pts: &[(f64, f64)],
    aspect: f64,
) -> (f64, f64, f64) {
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

/// Draw the map. Returns the key of the aircraft clicked this frame, if any.
pub fn show(
    ui: &mut Ui,
    state: &mut HfdlMapState,
    home: Option<(f64, f64)>,
    stations: &[MapStation],
    now: i64,
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

    // A snapshot of the table: it is a handful of aircraft, and copying it
    // leaves `state` free for the selection the interaction sets. The key order
    // is the table's own, which is stable between frames.
    let live: Vec<(String, HfdlPlot)> =
        state.plots().map(|(k, p)| (k.clone(), p.clone())).collect();

    // ── the view ──
    let aspect = (rect.height() / rect.width()) as f64;
    let pts: Vec<(f64, f64)> = live.iter().map(|(_, a)| (a.lat, a.lon)).collect();
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

    let dot_r = draw_base(&p, rect, clat, clon, lon_span, lat_span, map);

    // Nothing to plot yet is the normal state on a quiet channel — say so, so a
    // blank map is not read as a broken one. An HFDL position only arrives when
    // an aircraft downlinks a performance- or frequency-data record; squitters
    // carry the ground station, not a fix.
    if state.is_empty() {
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            tr!("hfdlmap-no-aircraft-located-yet"),
            FontId::proportional(11.0),
            alpha(map.station, 160.0),
        );
    }

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
    for (i, (_, a)) in live.iter().enumerate() {
        let c = project(a.lat, a.lon);
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

    let font = FontId::monospace(9.0);
    let selected = state.selected.clone();

    // ── the ground stations ──
    // Small triangles: lit and named while heard, dim and unnamed while known
    // only from the table — a world of sixteen labels would bury the aircraft.
    let heard_ink = crate::theme::GREEN();
    for st in stations {
        let c = project(st.lat, st.lon);
        if !rect.contains(c) {
            continue;
        }
        let (fill, edge) = match st.heard_age {
            Some(_) => (alpha(heard_ink, 220.0), alpha(map.sea, 200.0)),
            None => (alpha(map.city, 150.0), alpha(map.sea, 160.0)),
        };
        let tri = vec![c + vec2(0.0, -4.5), c + vec2(4.2, 3.0), c + vec2(-4.2, 3.0)];
        p.add(eframe::egui::Shape::convex_polygon(tri, fill, (1.0, edge)));
        if st.heard_age.is_some() {
            p.text(
                c + vec2(6.0, 0.0),
                Align2::LEFT_CENTER,
                town(&st.name),
                FontId::proportional(9.0),
                alpha(heard_ink, 200.0),
            );
        }
    }

    // ── where each aircraft has been ──
    // Dots, the oldest smallest and faintest, under the squares so the present
    // position is always the clearest thing on the map. Drawn for every
    // aircraft; the selected one's in the selection's colour.
    for (key, a) in &live {
        let tint = if selected.as_deref() == Some(key.as_str()) { map.dx } else { map.station };
        let n = a.trail.len();
        for (i, t) in a.trail.iter().enumerate() {
            let c = project(t.lat, t.lon);
            if !rect.contains(c) {
                continue;
            }
            // 1/n for the oldest up to 1 for the newest.
            let f = (i + 1) as f32 / n as f32;
            p.circle_filled(c, 1.8 + 0.8 * f, alpha(map.sea, 150.0));
            p.circle_filled(c, 1.2 + 0.8 * f, alpha(tint, 70.0 + 140.0 * f));
        }
    }
    let tint_for = |key: &str, top: bool| -> eframe::egui::Color32 {
        if selected.as_deref() == Some(key) {
            map.dx
        } else if top {
            map.hover
        } else {
            map.station
        }
    };

    // ── targets, under the data blocks ──
    let draw_target = |key: &str, a: &HfdlPlot, top: bool| {
        let c = project(a.lat, a.lon);
        if !rect.contains(c) && !top {
            return;
        }
        let tint = tint_for(key, top);
        let r = if top || selected.as_deref() == Some(key) { TARGET_R + 1.0 } else { TARGET_R };
        // A halo so it stays readable over the dotted land.
        p.circle_filled(c, r + 2.0, alpha(map.sea, 170.0));
        p.rect_stroke(
            Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)),
            0.0,
            (1.4, tint),
            eframe::egui::StrokeKind::Middle,
        );
    };
    for (i, (key, a)) in live.iter().enumerate() {
        if hover != Some(i) {
            draw_target(key, a, false);
        }
    }
    if let Some(i) = hover {
        let (key, a) = &live[i];
        draw_target(key, a, true);
    }

    // ── the blink asked for from the list ──
    let t_now = ui.input(|i| i.time);
    if let Some((key, start, gs)) = state.blink.clone() {
        match blink_phase(start, t_now) {
            None => state.blink = None,
            Some((lit, ring)) => {
                if let Some((_, a)) = live.iter().find(|(k, _)| *k == key) {
                    let c = project(a.lat, a.lon);
                    let tint = map.dx;
                    // The great circle to the station it is talking to, for as
                    // long as it blinks — the only time it is drawn — and the
                    // station marked at its end.
                    if let Some(st) = gs.and_then(|id| stations.iter().find(|s| s.id == id)) {
                        let path: Vec<Pos2> = sdroxide_types::great_circle_points(
                            (a.lat, a.lon),
                            (st.lat, st.lon),
                            64,
                        )
                        .into_iter()
                        .map(|(la, lo)| project(la, lo))
                        .collect();
                        for seg in split_on_wrap(&path, rect.width()) {
                            p.add(eframe::egui::Shape::line(seg, (1.6, alpha(map.trail, 220.0))));
                        }
                        let sc = project(st.lat, st.lon);
                        p.circle_stroke(sc, 7.0, (1.6, alpha(map.trail, 230.0)));
                        p.text(
                            sc + vec2(9.0, 0.0),
                            Align2::LEFT_CENTER,
                            town(&st.name),
                            FontId::proportional(10.0),
                            alpha(map.trail, 240.0),
                        );
                    }
                    // The ring spreads and fades once per half-second cycle; the
                    // disc under the square flashes with it.
                    let spread = TARGET_R + 3.0 + 16.0 * ring;
                    p.circle_stroke(c, spread, (2.0, alpha(tint, 255.0 * (1.0 - ring))));
                    if lit {
                        p.circle_filled(c, TARGET_R + 6.0, alpha(tint, 150.0));
                    }
                }
                crate::repaint::animate(ui.ctx());
            }
        }
    }

    // ── the data blocks ──
    // The selected and the hovered always; the rest in the table's stable order,
    // each only where its block misses the symbols already placed. The map
    // carries few enough targets that the greedy placement rarely drops one, but
    // it is the same placement the ADS-B and AIS charts use, so a crowded
    // picture culls the same way rather than piling up.
    let mut labels = Vec::new();
    for (i, (key, a)) in live.iter().enumerate() {
        let c = project(a.lat, a.lon);
        if !rect.contains(c) {
            continue;
        }
        let top = hover == Some(i);
        let is_sel = selected.as_deref() == Some(key.as_str());
        let tint = tint_for(key, top);
        let line2 = match a.snr_db {
            Some(snr) => format!("{snr:.0} dB  {:.3}M", a.freq_khz as f32 / 1e3),
            None => format!("{:.3}M", a.freq_khz as f32 / 1e3),
        };
        labels.push(MapLabel {
            at: c,
            r: if top || is_sel { TARGET_R + 1.0 } else { TARGET_R },
            tick_from: 0.7,
            tick: alpha(tint, 110.0),
            lines: vec![(a.label.clone(), alpha(tint, 240.0)), (line2, alpha(tint, 190.0))],
            must: top || is_sel,
            rank: if top || is_sel { 0 } else { 1 },
            key: i as u32,
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
        let (key, a) = &live[i];
        let mut tip = a.label.clone();
        tip.push_str(&format!("\n{:.4}, {:.4}", a.lat, a.lon));
        tip.push_str(&tr!("hfdlmap-mhz", e3 = format!("{:.3}", a.freq_khz as f32 / 1e3)));
        if let Some(snr) = a.snr_db {
            tip.push_str(&format!("\n{snr:.0} dB"));
        }
        tip.push_str(&tr!(
            "hfdlmap-fix",
            fixes = a.fixes,
            es = if a.fixes == 1 { "" } else { "es" }
        ));
        tip.push_str(&tr!("hfdlmap-heard-ago", last_at = fmt_age(now - a.last_at)));
        if let Some((hlat, hlon)) = home {
            let km = sdroxide_types::distance_km((hlat, hlon), (a.lat, a.lon));
            let bear = sdroxide_types::bearing_deg((hlat, hlon), (a.lat, a.lon));
            tip.push_str(&format!("\n{km:.0} km   {bear:.0}°"));
        }
        resp.clone().on_hover_text(tip);
        if resp.clicked() {
            state.selected = Some(key.clone());
            clicked = Some(key.clone());
        }
    } else {
        // Over a ground station: what it is, and whether it has been heard.
        if let Some(m) = pointer
            && let Some(st) = stations
                .iter()
                .map(|st| (st, project(st.lat, st.lon).distance(m)))
                .filter(|(_, d)| *d <= HIT_RADIUS)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(st, _)| st)
        {
            let heard = match st.heard_age {
                Some(age) => tr!("hfdlmap-gs-heard", age = fmt_age(age)).to_string(),
                None => tr!("hfdlmap-gs-not-heard").to_string(),
            };
            resp.clone().on_hover_text(format!("{} (GS {})\n{heard}", st.name, st.id));
        }
        if resp.clicked() {
            // A click on empty map clears the selection, the way the ADS-B
            // picture does.
            state.selected = None;
        }
    }

    clicked
}

/// A short age, for a tooltip: `12 s`, `4 min`, `2 h`.
pub(crate) fn fmt_age(secs: i64) -> String {
    let s = secs.max(0);
    if s < 90 {
        format!("{s} s")
    } else if s < 5400 {
        format!("{value} min", value = s / 60)
    } else {
        format!("{} h", s / 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::HfdlFix;

    fn decode(unix: i64, fix: Option<HfdlFix>) -> HfdlDecode {
        HfdlDecode {
            unix,
            kind: "performance-data".into(),
            gs: None,
            freq_khz: 21_931,
            snr_db: Some(12.0),
            freq_skew_hz: None,
            fec_corrected: None,
            details: "{}".into(),
            position: fix,
            acars: None,
        }
    }

    fn fix(icao: Option<&str>, id: Option<u32>, lat: f64, lon: f64) -> HfdlFix {
        HfdlFix {
            lat,
            lon,
            aircraft_id: id,
            icao: icao.map(str::to_owned),
            flight: Some("BAW123".into()),
        }
    }

    fn plot_of<'a>(m: &'a HfdlMapState, key: &str) -> Option<&'a HfdlPlot> {
        m.plots().find(|(k, _)| k.as_str() == key).map(|(_, p)| p)
    }

    #[test]
    fn observe_keeps_the_newest_fix_per_identity() {
        let mut m = HfdlMapState::default();
        let now = 1_000;
        // Newest first, as the status log is.
        m.observe(
            &[
                decode(now, Some(fix(Some("040087"), Some(0x42), 50.0, -3.0))),
                decode(now - 300, Some(fix(Some("040087"), Some(0x42), 49.0, -2.0))),
            ],
            now,
        );
        assert_eq!(m.len(), 1, "one aircraft, one plot");
        let (key, plot) = m.plots().next().unwrap();
        assert_eq!(key, "icao:040087");
        assert!((plot.lat - 50.0).abs() < 1e-9, "the newer fix wins");
        assert_eq!(plot.fixes, 1);
    }

    #[test]
    fn observe_is_idempotent_on_a_repeated_snapshot() {
        let mut m = HfdlMapState::default();
        let log = [decode(1_000, Some(fix(Some("040087"), None, 50.0, -3.0)))];
        m.observe(&log, 1_000);
        m.observe(&log, 1_000);
        m.observe(&log, 1_000);
        let plot = plot_of(&m, "icao:040087").unwrap();
        assert_eq!(plot.fixes, 1, "the same fix must not count three times");
    }

    #[test]
    fn a_newer_fix_moves_the_plot_and_counts() {
        let mut m = HfdlMapState::default();
        m.observe(&[decode(1_000, Some(fix(Some("040087"), None, 50.0, -3.0)))], 1_000);
        m.observe(&[decode(1_060, Some(fix(Some("040087"), None, 51.0, -4.0)))], 1_060);
        let plot = plot_of(&m, "icao:040087").unwrap();
        assert!((plot.lon + 4.0).abs() < 1e-9);
        assert_eq!(plot.fixes, 2);
    }

    #[test]
    fn old_plots_are_retired() {
        let mut m = HfdlMapState::default();
        m.observe(&[decode(1_000, Some(fix(Some("040087"), None, 50.0, -3.0)))], 1_000);
        m.observe(&[], 1_000 + PLOT_STALE_S + 1);
        assert!(m.is_empty(), "a plot nothing refreshes must come off the map");
    }

    /// Fixes of one aircraft a degree of latitude (~111 km) and ten minutes
    /// apart, oldest first: `n` of them from `t0`.
    fn flight(n: usize, t0: i64) -> Vec<HfdlDecode> {
        (0..n)
            .map(|i| {
                decode(t0 + 600 * i as i64, Some(fix(Some("484123"), None, 40.0 + i as f64, -30.0)))
            })
            .collect()
    }

    fn newest_first(mut v: Vec<HfdlDecode>) -> Vec<HfdlDecode> {
        v.reverse();
        v
    }

    fn trail_ats(m: &HfdlMapState) -> Vec<i64> {
        plot_of(m, "icao:484123").unwrap().trail.iter().map(|t| t.at).collect()
    }

    /// The log carries several fixes per aircraft, so the first snapshot already
    /// has a history: the older ones are where it has been.
    #[test]
    fn the_first_snapshot_already_gives_a_trail() {
        let mut m = HfdlMapState::default();
        m.observe(&newest_first(flight(3, 1_000)), 3_000);
        let p = plot_of(&m, "icao:484123").unwrap();
        assert_eq!((p.last_at, p.lat), (2_200, 42.0), "the latest is the plot");
        assert_eq!(trail_ats(&m), vec![1_000, 1_600]);
        assert_eq!(p.trail[0].lat, 40.0);
        assert_eq!(p.fixes, 1, "history is not counted as new fixes");
    }

    /// Snapshots repeat the whole log; a trail must not grow with each.
    #[test]
    fn a_repeated_snapshot_leaves_the_trail_alone() {
        let mut m = HfdlMapState::default();
        let log = newest_first(flight(4, 1_000));
        m.observe(&log, 3_000);
        let once = trail_ats(&m);
        m.observe(&log, 3_001);
        m.observe(&log, 3_002);
        assert_eq!(trail_ats(&m), once);
        assert_eq!(once, vec![1_000, 1_600, 2_200]);
    }

    #[test]
    fn a_newer_fix_pushes_the_old_position_onto_the_trail() {
        let mut m = HfdlMapState::default();
        m.observe(&flight(1, 1_000), 1_100);
        assert!(trail_ats(&m).is_empty(), "a lone fix has no past");
        m.observe(&newest_first(flight(2, 1_000)), 1_700);
        assert_eq!(trail_ats(&m), vec![1_000]);
        m.observe(&newest_first(flight(3, 1_000)), 2_300);
        assert_eq!(trail_ats(&m), vec![1_000, 1_600]);
        assert_eq!(plot_of(&m, "icao:484123").unwrap().last_at, 2_200);
    }

    #[test]
    fn a_trail_keeps_only_the_newest_few() {
        let mut m = HfdlMapState::default();
        m.observe(&newest_first(flight(9, 1_000)), 6_000);
        let ats = trail_ats(&m);
        assert_eq!(ats.len(), TRAIL_MAX);
        // Of the eight past fixes the five latest, oldest first.
        assert_eq!(ats, vec![2_800, 3_400, 4_000, 4_600, 5_200]);
    }

    #[test]
    fn a_fix_reported_twice_does_not_stack_two_dots() {
        let mut m = HfdlMapState::default();
        let mut log = flight(3, 1_000);
        // A second record from the same spot as the first, a minute later.
        log.insert(1, decode(1_060, Some(fix(Some("484123"), None, 40.0001, -30.0))));
        m.observe(&newest_first(log), 3_000);
        assert_eq!(trail_ats(&m).len(), 2, "{:?}", trail_ats(&m));
    }

    /// An aircraft that reports only every twenty minutes would otherwise carry
    /// a trail reaching back across hours: points over an hour old go.
    #[test]
    fn a_position_from_long_ago_is_not_part_of_the_trail() {
        let mut m = HfdlMapState::default();
        let sparse: Vec<HfdlDecode> = (0..6)
            .map(|i| {
                decode(1_000 + 1_200 * i, Some(fix(Some("484123"), None, 40.0 + i as f64, -30.0)))
            })
            .collect();
        // Fixes at 1000, 2200, 3400, 4600, 5800, 7000; it is 7010.
        m.observe(&newest_first(sparse), 7_010);
        assert_eq!(trail_ats(&m), vec![4_600, 5_800], "1000..3400 are over an hour old");
    }

    #[test]
    fn two_aircraft_keep_separate_trails() {
        let mut m = HfdlMapState::default();
        let mut log = flight(3, 1_000);
        log.extend((0..3).map(|i| {
            decode(1_000 + 600 * i, Some(fix(Some("A1B2C3"), None, 10.0 + i as f64, 100.0)))
        }));
        m.observe(&newest_first(log), 3_000);
        assert_eq!(trail_ats(&m).len(), 2);
        assert_eq!(plot_of(&m, "icao:A1B2C3").unwrap().trail.len(), 2);
    }

    #[test]
    fn a_blink_lasts_three_seconds_in_half_second_beats() {
        // Not started yet, and over.
        assert_eq!(blink_phase(10.0, 9.0), None);
        assert_eq!(blink_phase(10.0, 10.0 + BLINK_S), None);
        // Lit for the first quarter second, dark for the next.
        assert_eq!(blink_phase(10.0, 10.1).map(|p| p.0), Some(true));
        assert_eq!(blink_phase(10.0, 10.3).map(|p| p.0), Some(false));
        assert_eq!(blink_phase(10.0, 10.6).map(|p| p.0), Some(true));
        // The ring starts at the plot and grows to the edge of each beat.
        let (_, early) = blink_phase(10.0, 10.05).unwrap();
        let (_, late) = blink_phase(10.0, 10.45).unwrap();
        assert!(early < 0.2 && late > 0.8, "{early} {late}");
        assert!((0.0..=1.0).contains(&late));
    }

    #[test]
    fn blinking_needs_a_plot_and_hands_the_view_back_to_the_auto_fit() {
        let mut m = HfdlMapState::default();
        m.observe(&[decode(1_000, Some(fix(Some("484123"), None, 52.0, -30.0)))], 1_010);
        m.view.manual = true;
        assert!(!m.blink("icao:999999", 5.0, None), "nothing plotted under that key");
        assert!(m.view.manual, "a miss leaves the view alone");
        assert_eq!(m.selected, None);
        assert!(m.blink("icao:484123", 5.0, Some(4)));
        assert!(!m.view.manual, "the plot is framed so the blink is seen");
        assert_eq!(m.selected.as_deref(), Some("icao:484123"));
    }

    #[test]
    fn a_path_is_split_where_it_crosses_the_edge_of_the_map() {
        let p = |x: f32| pos2(x, 10.0);
        // Straight across: one piece.
        assert_eq!(split_on_wrap(&[p(10.0), p(20.0), p(30.0)], 100.0).len(), 1);
        // Off the right edge and back on the left: two pieces, no line across.
        let segs = split_on_wrap(&[p(80.0), p(95.0), p(3.0), p(15.0)], 100.0);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].len(), 2);
        assert_eq!(segs[1].len(), 2);
        // A lone point either side of a wrap draws nothing on its own.
        assert_eq!(split_on_wrap(&[p(95.0), p(3.0)], 100.0).len(), 0);
    }

    #[test]
    fn a_station_is_labelled_by_its_town() {
        assert_eq!(town("Shannon, Ireland"), "Shannon");
        assert_eq!(town("GS 12"), "GS 12");
    }

    #[test]
    fn the_blink_remembers_the_station_for_its_line() {
        let mut m = HfdlMapState::default();
        m.observe(&[decode(1_000, Some(fix(Some("484123"), None, 52.0, -30.0)))], 1_010);
        assert!(m.blink("icao:484123", 5.0, Some(7)));
        assert_eq!(m.blink.as_ref().map(|b| b.2), Some(Some(7)));
    }

    #[test]
    fn the_map_says_whether_a_key_is_plotted() {
        let mut m = HfdlMapState::default();
        m.observe(&[decode(1_000, Some(fix(Some("484123"), None, 52.0, -30.0)))], 1_010);
        assert!(m.has("icao:484123"));
        assert!(!m.has("icao:999999"));
    }

    #[test]
    fn a_decode_without_a_position_plots_nothing() {
        let mut m = HfdlMapState::default();
        m.observe(&[decode(1_000, None)], 1_000);
        assert!(m.is_empty());
    }

    #[test]
    fn the_target_view_frames_every_plot() {
        let pts = [(-10.0, -20.0), (30.0, 40.0)];
        let (clat, clon, span) = target_view(None, &pts, 0.5);
        assert!((clat - 10.0).abs() < 1e-6, "mid latitude: {clat}");
        assert!(wrap180(clon - 10.0).abs() < 1e-6, "mid longitude: {clon}");
        // Wide enough that both points fall inside the frame.
        assert!(span >= 2.0 * 30.0 * PAD - 1e-6, "span {span}");
    }
}
