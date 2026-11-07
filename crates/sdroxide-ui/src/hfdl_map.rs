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

use eframe::egui::{Align2, FontId, Rect, Sense, Ui, pos2, vec2};
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
            match self.plots.get_mut(&key) {
                Some(p) if d.unix > p.last_at => {
                    p.label = fix.label();
                    p.lat = fix.lat;
                    p.lon = fix.lon;
                    p.last_at = d.unix;
                    p.snr_db = d.snr_db;
                    p.freq_khz = d.freq_khz;
                    p.fixes = p.fixes.saturating_add(1);
                }
                Some(_) => {}
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
                        },
                    );
                }
            }
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

    pub fn is_empty(&self) -> bool {
        self.plots.is_empty()
    }
}

/// The view to ease toward: every plot, plus us, framed.
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

/// Draw the map. Returns the key of the aircraft clicked this frame, if any.
pub fn show(
    ui: &mut Ui,
    state: &mut HfdlMapState,
    home: Option<(f64, f64)>,
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
    } else if resp.clicked() {
        // A click on empty map clears the selection, the way the ADS-B picture
        // does.
        state.selected = None;
    }

    clicked
}

/// A short age, for a tooltip: `12 s`, `4 min`, `2 h`.
fn fmt_age(secs: i64) -> String {
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
