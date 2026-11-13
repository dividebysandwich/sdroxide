//! The radiosonde picture: every sonde the decoder has heard, drawn with the
//! track it has flown over the same dotted continents the ADS-B, AIS and HFDL
//! maps use ([`crate::widgets::worldmap`]).
//!
//! A sonde reports its own GPS position once a second, so unlike HFDL the map
//! has a real track to draw: a line from where it was first heard to where it
//! is now, a mark at the head, and beside it the serial, the altitude and the
//! climb rate. The table is the decoder's own ([`sdroxide_types::SondeStatus`]);
//! this map keeps only the view and the selection.

use crate::i18n::tr;
use crate::sonde_online::OnlineSonde;
use eframe::egui::{
    Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, Tooltip, Ui, Vec2, pos2, vec2,
};
use sdroxide_types::SondeTrack;

use crate::hfdl_map::target_view;
use crate::theme;
use crate::widgets::map_labels::{self, MapLabel};
use crate::widgets::worldmap::{MapView, alpha, draw_base, interact, wrap180};

/// Below this height the map is not worth drawing.
pub const MIN_HEIGHT: f32 = 90.0;

/// How close a click has to land, in points. A mark is 8 points across; a
/// fingertip or a shaky hand is not, so the reach is generous and the nearest
/// mark wins.
const HIT_RADIUS: f32 = 18.0;

/// Width of one character of the 9-point monospace label, points.
const LABEL_CHAR_W: f32 = 5.5;

/// What the pointer can pick on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    /// A sonde this radio decodes (index into the placed ones).
    Own(usize),
    /// A sonde only SondeHub has (index into the drawn ones).
    Hub(usize),
}

/// The nearest candidate within `radius`; on a tie the first, so a sonde this
/// radio decodes beats a SondeHub one at the same spot.
fn nearest<T: Copy>(candidates: &[(T, f32)], radius: f32) -> Option<T> {
    let mut best: Option<(T, f32)> = None;
    for &(what, d) in candidates {
        if d <= radius && best.is_none_or(|(_, b)| d < b) {
            best = Some((what, d));
        }
    }
    best.map(|(what, _)| what)
}

/// Per-frame ease toward the auto-fit.
const EASE: f64 = 0.06;

/// A sonde not heard for this long is shown as lost rather than flying: after
/// landing it is silent, and its last position is where it came down.
pub const LOST_AFTER_S: i64 = 60;

/// The map's state: the view and the selected sonde, kept by the app so they
/// survive the window being closed.
#[derive(Default)]
pub struct SondeMapState {
    pub view: MapView,
    /// The serial of the sonde whose details are shown.
    pub selected: Option<String>,
    /// The line shown after a flight was saved to a file.
    pub note: Option<String>,
    /// What was last seen of the selected sonde, so its card outlives the list
    /// it came from (SondeHub's is asked again every minute, within a radius).
    pub kept: Kept,
    /// Where the selected sonde is on screen and the part of the map's
    /// rectangle that is on show, this frame: the card pins itself beside the sonde, inside the map.
    pub anchor: Option<Pos2>,
    pub bounds: Option<Rect>,
    /// The size the card had last frame, to place it before it is drawn.
    pub card_size: Option<Vec2>,
    /// The card is its own window, off the map.
    pub detached: bool,
    /// The flight so far of the selected sonde, from SondeHub.
    pub flight: crate::sonde_online::Flight,
    /// Only this type is drawn and listed (`None`: every type).
    pub type_filter: Option<&'static str>,
    /// Walks the window across the band while a scan finds nothing.
    pub stepper: Stepper,
}

/// How long the window rests on one stretch of the band before the scan moves on.
pub const SCAN_DWELL_S: i64 = 10;
/// A sonde heard this recently keeps the scan where it is.
pub const SCAN_HOLD_S: i64 = 45;

/// Lowest frequency a scan visits (the band's floor, Hz).
const SCAN_LOW_HZ: f64 = 400_150_000.0;

/// The window centre a scan moves to after `current_hz`, for a window `rate_hz`
/// wide: the band is covered in steps of four fifths of the window (so the
/// stretches overlap), the last stretch ends on the band's top edge, and after
/// it the walk starts over from the bottom. A window as wide as the band
/// stays on the middle. A centre that is not one of the stretches (the dial
/// was somewhere else) starts at the bottom.
pub fn next_scan_centre(current_hz: f64, rate_hz: f64) -> f64 {
    let top = sdroxide_types::SONDE_BAND_HZ.1;
    let rate = rate_hz.abs();
    if rate >= top - SCAN_LOW_HZ {
        return (SCAN_LOW_HZ + top) / 2.0;
    }
    let half = rate / 2.0;
    let (first, last) = (SCAN_LOW_HZ + half, top - half);
    if current_hz < first - 1.0 || current_hz > last + 1.0 {
        return first;
    }
    let next = current_hz + rate * 0.8;
    if next <= last + 1.0 {
        next
    } else if current_hz < last - 1.0 {
        last
    } else {
        first
    }
}

/// When the scan moves the window: after [`SCAN_DWELL_S`] of nothing heard.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Stepper {
    moved_at: i64,
}

impl Stepper {
    /// Forget the dwell, so the next frame of an active scan moves at once.
    pub fn restart(&mut self) {
        self.moved_at = 1;
    }

    /// Whether to move the window now. `active`: a scan is running; `busy`: a
    /// sonde is being heard, so the window stays.
    pub fn due(&mut self, now: i64, active: bool, busy: bool) -> bool {
        if !active {
            self.moved_at = 0;
            return false;
        }
        if busy || self.moved_at == 0 {
            self.moved_at = now;
            return false;
        }
        if now - self.moved_at >= SCAN_DWELL_S {
            self.moved_at = now;
            return true;
        }
        false
    }
}

/// Whether a type (as the picker names it, e.g. "C34/C50" or "iMet") covers a
/// sonde's type as a decoder or SondeHub writes it ("iMet-4", "RS41").
pub fn type_matches(filter: &str, model: &str) -> bool {
    let norm = |t: &str| -> String {
        t.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
    };
    let m = norm(model);
    !m.is_empty()
        && filter
            .split('/')
            .map(norm)
            .any(|f| !f.is_empty() && (m.starts_with(&f) || f.starts_with(&m)))
}

impl SondeMapState {
    /// Select a sonde: the copy kept of the previous one goes.
    pub fn select(&mut self, serial: String) {
        if self.selected.as_deref() != Some(serial.as_str()) {
            self.kept = Kept::default();
            self.card_size = None;
        }
        self.selected = Some(serial);
    }

    /// Close the card.
    pub fn deselect(&mut self) {
        self.selected = None;
        self.kept = Kept::default();
        self.card_size = None;
        self.detached = false;
    }
}

/// The last live copy of the selected sonde, from each source. The radio's
/// track is dropped (a card does not use it), so keeping it costs a few
/// hundred bytes.
#[derive(Default, Clone)]
pub struct Kept {
    pub radio: Option<SondeTrack>,
    pub hub: Option<OnlineSonde>,
}

impl Kept {
    /// Take what the sources say now. Returns whether either still has the
    /// sonde: when neither does, the card is drawn from the copy and says so.
    pub fn update(&mut self, radio: Option<&SondeTrack>, hub: Option<&OnlineSonde>) -> bool {
        if let Some(s) = radio {
            let same = self
                .radio
                .as_ref()
                .is_some_and(|k| k.serial == s.serial && k.last_unix == s.last_unix);
            if !same {
                let mut c = s.clone();
                c.track = Vec::new();
                self.radio = Some(c);
            }
        }
        if let Some(o) = hub
            && self.hub.as_ref() != Some(o)
        {
            self.hub = Some(o.clone());
        }
        radio.is_some() || hub.is_some()
    }
}

/// What a sonde is doing, from its latest frame and how long ago it was heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Ascending,
    Descending,
    /// No frame for [`LOST_AFTER_S`]: on the ground, or out of range.
    Silent,
}

pub fn phase(s: &SondeTrack, now: i64) -> Phase {
    if now - s.last_unix > LOST_AFTER_S {
        Phase::Silent
    } else if s.climb_ms.is_some_and(|c| c < -1.0) {
        Phase::Descending
    } else {
        Phase::Ascending
    }
}

fn phase_color(p: Phase, map: &theme::MapPalette) -> Color32 {
    match p {
        Phase::Ascending => map.station,
        Phase::Descending => map.dx,
        Phase::Silent => alpha(map.station, 110.0),
    }
}

/// One line of data under a sonde's name: altitude and climb.
pub fn summary(s: &SondeTrack) -> String {
    let alt = s.alt_m.map(|a| format!("{a:.0} m")).unwrap_or_else(|| "—".into());
    match s.climb_ms {
        Some(c) => format!("{alt}  {c:+.1} m/s"),
        None => alt,
    }
}

/// The gap between a sonde and the card pinned beside it, points.
const PIN_GAP: f32 = 14.0;

/// Top-left of a card of `size` pinned to a sonde at `anchor`: below and to
/// the right of it, flipped to the other side where that would leave the map,
/// and kept inside `bounds` whatever is left.
pub fn pin_placement(anchor: Pos2, size: Vec2, bounds: Rect) -> Pos2 {
    let mut x = anchor.x + PIN_GAP;
    if x + size.x > bounds.right() {
        x = anchor.x - PIN_GAP - size.x;
    }
    let mut y = anchor.y + PIN_GAP;
    if y + size.y > bounds.bottom() {
        y = anchor.y - PIN_GAP - size.y;
    }
    // Last, and the minimum last of all: a card bigger than the map is pinned
    // to its top-left corner rather than asked to fit where it cannot.
    pos2(
        x.min(bounds.right() - size.x).max(bounds.left()),
        y.min(bounds.bottom() - size.y).max(bounds.top()),
    )
}

/// Draw the map. Returns the serial clicked this frame, if any.
pub fn show(
    ui: &mut Ui,
    state: &mut SondeMapState,
    sondes: &[SondeTrack],
    online: &[OnlineSonde],
    home: Option<(f64, f64)>,
    now: i64,
    max_h: f32,
) -> Option<String> {
    state.anchor = None;
    state.bounds = None;
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

    // Only sondes with a place can be drawn.
    let placed: Vec<(&SondeTrack, f64, f64)> =
        sondes.iter().filter_map(|s| Some((s, s.lat?, s.lon?))).collect();

    // ── the view ──
    let aspect = (rect.height() / rect.width()) as f64;
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for (s, lat, lon) in &placed {
        pts.push((*lat, *lon));
        // The whole track frames, not just the head, so a long flight fits.
        pts.extend(s.track.iter().step_by(30).map(|q| (q.lat, q.lon)));
    }
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

    if placed.is_empty() {
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            tr!("sondemap-no-sonde-yet"),
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

    // The part of the map on show: in a short window the page scrolls, and a
    // card pinned to the whole map floated past the window's edge.
    state.bounds = Some(rect.intersect(ui.clip_rect()));

    // The sondes only SondeHub has, where they are drawn and what is written
    // beside them.
    let hub_marks: Vec<(&OnlineSonde, Pos2, String)> = online
        .iter()
        .filter(|o| !sondes.iter().any(|s| s.serial == o.serial))
        .map(|o| {
            let alt = o.alt_m.map(|a| format!(" {a:.0} m")).unwrap_or_default();
            (o, project(o.lat, o.lon), format!("{}{alt}", o.serial))
        })
        .filter(|(_, c, _)| rect.contains(*c))
        .collect();

    // The mark under the pointer, decided before drawing: the nearest of every
    // mark (this radio's and SondeHub's) within reach. A SondeHub label is part
    // of its mark, so a click on the text counts.
    let pointer = resp.hover_pos();
    let mut picked: Option<Pick> = None;
    if let Some(m) = pointer {
        let mut cands: Vec<(Pick, f32)> = Vec::new();
        for (i, (_, lat, lon)) in placed.iter().enumerate() {
            cands.push((Pick::Own(i), project(*lat, *lon).distance(m)));
        }
        for (j, (_, c, text)) in hub_marks.iter().enumerate() {
            let label = Rect::from_min_size(
                pos2(c.x + 4.0, c.y - 7.0),
                vec2(text.chars().count() as f32 * LABEL_CHAR_W + 6.0, 14.0),
            );
            cands.push((Pick::Hub(j), c.distance(m).min(label.distance_to_pos(m))));
        }
        picked = nearest(&cands, HIT_RADIUS);
    }
    let hover = match picked {
        Some(Pick::Own(i)) => Some(i),
        _ => None,
    };
    let hub_hover = match picked {
        Some(Pick::Hub(j)) => Some(j),
        _ => None,
    };

    let font = FontId::monospace(9.0);
    let selected = state.selected.clone();

    // ── tracks, under the marks ──
    for (i, (s, _, _)) in placed.iter().enumerate() {
        let is_sel = selected.as_deref() == Some(s.serial.as_str());
        let tint = phase_color(phase(s, now), map);
        let w = if hover == Some(i) || is_sel { 2.2 } else { 1.4 };
        let line: Vec<_> = s.track.iter().map(|q| project(q.lat, q.lon)).collect();
        // Split where the line wraps the antimeridian, rather than drawing a
        // stroke across the whole map.
        for seg in line.windows(2) {
            if (seg[0].x - seg[1].x).abs() < rect.width() * 0.5 {
                p.line_segment([seg[0], seg[1]], Stroke::new(w, alpha(tint, 190.0)));
            }
        }
    }

    // ── the flight SondeHub holds for the selected sonde: where it has been ──
    if let Some(sel) = selected.as_deref() {
        let line: Vec<_> = state.flight.of(sel).iter().map(|(a, b)| project(*a, *b)).collect();
        let ink = alpha(Color32::from_rgb(110, 165, 255), 220.0);
        for seg in line.windows(2) {
            if (seg[0].x - seg[1].x).abs() < rect.width() * 0.5 {
                p.line_segment([seg[0], seg[1]], Stroke::new(2.2, ink));
            }
        }
    }

    // ── the sondes other stations hear (SondeHub), hollow diamonds ──
    let online_ink = alpha(Color32::from_rgb(110, 165, 255), 230.0);
    for (j, (o, c, text)) in hub_marks.iter().enumerate() {
        let c = *c;
        let lit = hub_hover == Some(j) || selected.as_deref() == Some(o.serial.as_str());
        let r = if lit { 5.5 } else { 4.0 };
        let d =
            vec![pos2(c.x, c.y - r), pos2(c.x + r, c.y), pos2(c.x, c.y + r), pos2(c.x - r, c.y)];
        p.add(eframe::egui::Shape::closed_line(
            d,
            Stroke::new(if lit { 2.0 } else { 1.4 }, online_ink),
        ));
        p.text(
            c + vec2(7.0, 0.0),
            Align2::LEFT_CENTER,
            text,
            FontId::monospace(9.0),
            alpha(online_ink, if lit { 255.0 } else { 190.0 }),
        );
    }

    // ── marks and labels ──
    let mut labels = Vec::new();
    for (i, (s, lat, lon)) in placed.iter().enumerate() {
        let c = project(*lat, *lon);
        let top = hover == Some(i);
        let is_sel = selected.as_deref() == Some(s.serial.as_str());
        let ph = phase(s, now);
        let tint = if is_sel {
            map.dx
        } else if top {
            map.hover
        } else {
            phase_color(ph, map)
        };
        let r = if top || is_sel { 5.0 } else { 4.0 };
        p.circle_filled(c, r + 2.0, alpha(map.sea, 170.0));
        match ph {
            // A balloon going up: a disc. Coming down under the parachute: a
            // triangle. Silent: a cross, where it was last heard.
            Phase::Ascending => {
                p.circle_stroke(c, r, (1.6, tint));
            }
            Phase::Descending => {
                let pts = vec![
                    pos2(c.x, c.y - r),
                    pos2(c.x - r, c.y + r * 0.8),
                    pos2(c.x + r, c.y + r * 0.8),
                ];
                p.add(eframe::egui::Shape::closed_line(pts, Stroke::new(1.6, tint)));
            }
            Phase::Silent => {
                p.line_segment([c + vec2(-r, -r), c + vec2(r, r)], (1.6, tint));
                p.line_segment([c + vec2(-r, r), c + vec2(r, -r)], (1.6, tint));
            }
        }
        if !rect.contains(c) && !top {
            continue;
        }
        labels.push(MapLabel {
            at: c,
            r,
            tick_from: 0.7,
            tick: alpha(tint, 110.0),
            lines: vec![(s.serial.clone(), alpha(tint, 240.0)), (summary(s), alpha(tint, 190.0))],
            must: top || is_sel,
            rank: if top || is_sel { 0 } else { 1 },
            key: i as u32,
        });
    }
    let marks: Vec<eframe::egui::Rect> = home
        .map(|(lat, lon)| eframe::egui::Rect::from_center_size(project(lat, lon), vec2(16.0, 16.0)))
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

    // ── where the card pins itself ──
    if let Some(sel) = selected.as_deref() {
        let at = placed
            .iter()
            .find(|(s, ..)| s.serial == sel)
            .map(|(_, lat, lon)| project(*lat, *lon))
            .or_else(|| hub_marks.iter().find(|(o, ..)| o.serial == sel).map(|(_, c, _)| *c));
        state.anchor = at.map(|a| rect.clamp(a));
    }

    // ── interaction ──
    let mut clicked = None;
    if let Some(pick) = picked {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        // One tooltip, at the pointer, at once: on the map's own response it
        // would sit at the corner of the whole map, after a delay.
        let tip = match pick {
            Pick::Own(i) => {
                let (s, lat, lon) = &placed[i];
                own_tip(s, *lat, *lon, home)
            }
            Pick::Hub(j) => hub_tip(hub_marks[j].0),
        };
        Tooltip::for_widget(&resp).at_pointer().gap(12.0).show(|ui| {
            ui.label(tip);
        });
        if resp.clicked() {
            let serial = match pick {
                Pick::Own(i) => placed[i].0.serial.clone(),
                Pick::Hub(j) => hub_marks[j].0.serial.clone(),
            };
            state.select(serial.clone());
            clicked = Some(serial);
        }
    }
    // A click on empty map changes nothing: the card closes with its own
    // button or Esc, so a near miss does not take it away.
    clicked
}

/// The tooltip of a sonde this radio decodes.
fn own_tip(s: &SondeTrack, lat: f64, lon: f64, home: Option<(f64, f64)>) -> String {
    let mut tip = format!("{}  {}\n{lat:.4}, {lon:.4}", s.model, s.serial);
    tip.push_str(&format!("\n{}", summary(s)));
    if let Some(v) = s.speed_ms {
        tip.push_str(&format!("\n{:.0} km/h", v * 3.6));
    }
    tip.push_str(&format!("\n{:.3} MHz", s.freq_hz / 1e6));
    if let Some(v) = s.battery_v {
        tip.push_str(&format!("\n{v:.1} V"));
    }
    if let Some((hlat, hlon)) = home {
        let km = sdroxide_types::distance_km((hlat, hlon), (lat, lon));
        let bear = sdroxide_types::bearing_deg((hlat, hlon), (lat, lon));
        tip.push_str(&format!("\n{km:.0} km   {bear:.0}°"));
    }
    tip
}

/// The tooltip of a sonde only SondeHub has.
fn hub_tip(o: &OnlineSonde) -> String {
    let mut tip = format!("SondeHub  {} {}\n{:.4}, {:.4}", o.model, o.serial, o.lat, o.lon);
    if let Some(a) = o.alt_m {
        tip.push_str(&format!("\n{a:.0} m"));
        if let Some(c) = o.climb_ms {
            tip.push_str(&format!("  {c:+.1} m/s"));
        }
    }
    if let Some(f) = o.freq_mhz {
        tip.push_str(&format!("\n{f:.3} MHz"));
    }
    tip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scan_walks_a_narrow_window_over_the_whole_band_and_starts_over() {
        let rate = 2_400_000.0;
        let mut c = 1_090_000_000.0; // the dial was somewhere else
        let mut seen = Vec::new();
        for _ in 0..8 {
            c = next_scan_centre(c, rate);
            seen.push(c);
        }
        assert_eq!(seen[0], SCAN_LOW_HZ + rate / 2.0, "starts at the bottom of the band");
        let top = sdroxide_types::SONDE_BAND_HZ.1;
        assert!(seen.iter().any(|c| *c == top - rate / 2.0), "reaches the top edge");
        assert!(seen.windows(2).all(|w| w[0] != w[1]));
        let last = seen.iter().position(|c| *c == top - rate / 2.0).unwrap();
        assert_eq!(seen[last + 1], seen[0], "then starts over");
        // Every stretch overlaps the next: nothing falls between two of them.
        assert!(seen[..=last].windows(2).all(|w| w[1] - w[0] < rate));
        assert_eq!(next_scan_centre(403e6, 8_000_000.0), (SCAN_LOW_HZ + top) / 2.0);
    }

    #[test]
    fn the_stepper_waits_a_dwell_holds_while_a_sonde_is_heard_and_restarts() {
        let mut s = Stepper::default();
        assert!(!s.due(100, false, false), "no scan, no move");
        assert!(!s.due(100, true, false), "the first look rests a dwell");
        assert!(!s.due(100 + SCAN_DWELL_S - 1, true, false));
        assert!(s.due(100 + SCAN_DWELL_S, true, false));
        assert!(!s.due(100 + SCAN_DWELL_S + 1, true, false), "and the next dwell starts");
        let t = 200;
        assert!(!s.due(t, true, true), "a sonde heard: stay");
        assert!(!s.due(t + 3 * SCAN_DWELL_S, true, true), "as long as it is heard");
        s.restart();
        assert!(s.due(t + 3 * SCAN_DWELL_S + 1, true, false), "a new choice moves at once");
        assert!(!s.due(1_000, false, false));
        assert!(!s.due(1_001, true, false), "off then on starts with a dwell");
    }

    #[test]
    fn a_type_covers_the_names_decoders_and_sondehub_give_it() {
        for (filter, model) in [
            ("RS41", "RS41"),
            ("RS41", "RS41-SG"),
            ("DFM", "DFM09"),
            ("iMet", "iMet-4"),
            ("iMet", "iMet-54"),
            ("C34/C50", "C50"),
            ("CF06/HT03", "HT03"),
            ("M10", "m10"),
        ] {
            assert!(type_matches(filter, model), "{filter} / {model}");
        }
        for (filter, model) in [("RS41", "M10"), ("M10", "M20"), ("DFM", ""), ("iMet-54", "iMet-4")]
        {
            assert!(!type_matches(filter, model), "{filter} / {model}");
        }
    }

    fn sonde(climb: Option<f32>, last: i64) -> SondeTrack {
        SondeTrack {
            model: "RS41".into(),
            serial: "N1".into(),
            freq_hz: 403e6,
            frame: 1,
            lat: Some(48.0),
            lon: Some(2.0),
            alt_m: Some(1500.0),
            speed_ms: Some(5.0),
            course_deg: None,
            climb_ms: climb,
            num_sv: Some(9),
            battery_v: Some(2.7),
            temp_c: None,
            humidity_pct: None,
            pressure_hpa: None,
            last_unix: last,
            frames: 1,
            track: vec![],
        }
    }

    #[test]
    fn the_phase_follows_the_climb_and_the_silence() {
        assert_eq!(phase(&sonde(Some(5.0), 100), 110), Phase::Ascending);
        assert_eq!(phase(&sonde(Some(-8.0), 100), 110), Phase::Descending);
        assert_eq!(phase(&sonde(Some(-0.2), 100), 110), Phase::Ascending, "float, not a fall");
        assert_eq!(phase(&sonde(Some(5.0), 100), 100 + LOST_AFTER_S + 1), Phase::Silent);
    }

    #[test]
    fn the_nearest_mark_within_reach_is_picked() {
        let c = [(Pick::Own(0), 15.0), (Pick::Hub(1), 6.0), (Pick::Hub(2), 30.0)];
        assert_eq!(nearest(&c, HIT_RADIUS), Some(Pick::Hub(1)), "nearest, not first");
        assert_eq!(nearest(&[(Pick::Own(0), HIT_RADIUS + 1.0)], HIT_RADIUS), None);
        let tie = [(Pick::Own(0), 5.0), (Pick::Hub(0), 5.0)];
        assert_eq!(nearest(&tie, HIT_RADIUS), Some(Pick::Own(0)), "this radio's sonde wins a tie");
        assert_eq!(nearest::<Pick>(&[], HIT_RADIUS), None);
    }

    #[test]
    fn selecting_another_sonde_drops_the_copy_kept_of_the_first() {
        let mut st = SondeMapState::default();
        st.select("A".into());
        st.kept.update(Some(&sonde(Some(1.0), 10)), None);
        assert!(st.kept.radio.is_some());
        st.select("A".into());
        assert!(st.kept.radio.is_some(), "the same sonde keeps its copy");
        st.select("B".into());
        assert!(st.kept.radio.is_none() && st.selected.as_deref() == Some("B"));
        st.detached = true;
        st.deselect();
        assert!(st.selected.is_none() && !st.detached, "closing re-attaches the next card");
    }

    #[test]
    fn a_copy_outlives_the_list_it_came_from() {
        let hub =
            OnlineSonde { serial: "H".into(), lat: 50.0, lon: 5.0, unix: 9, ..Default::default() };
        let mut k = Kept::default();
        assert!(k.update(None, Some(&hub)), "live while a list has it");
        assert!(!k.update(None, None), "gone from every list");
        assert_eq!(k.hub.as_ref().map(|o| o.serial.as_str()), Some("H"), "but not forgotten");
        let point = sdroxide_types::SondePoint {
            lat: 1.0,
            lon: 2.0,
            alt_m: 3.0,
            unix: 4,
            temp_c: None,
            humidity_pct: None,
            pressure_hpa: None,
        };
        let long = SondeTrack { track: vec![point; 500], ..sonde(None, 1) };
        k.update(Some(&long), None);
        assert!(k.radio.as_ref().unwrap().track.is_empty(), "no track is kept for a card");
    }

    #[test]
    fn a_pinned_card_stays_inside_the_map() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0));
        let size = vec2(100.0, 80.0);
        assert_eq!(pin_placement(pos2(50.0, 50.0), size, bounds), pos2(64.0, 64.0));
        let flipped = pin_placement(pos2(390.0, 290.0), size, bounds);
        assert_eq!(flipped, pos2(390.0 - PIN_GAP - 100.0, 290.0 - PIN_GAP - 80.0));
        let huge = pin_placement(pos2(10.0, 10.0), vec2(900.0, 900.0), bounds);
        assert_eq!(huge, pos2(0.0, 0.0));
    }

    #[test]
    fn the_summary_names_altitude_and_climb() {
        assert_eq!(summary(&sonde(Some(5.04), 0)), "1500 m  +5.0 m/s");
        assert_eq!(summary(&sonde(None, 0)), "1500 m");
    }
}
