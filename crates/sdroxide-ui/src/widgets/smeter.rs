//! S-meter with three selectable faces: an analog needle instrument (the
//! default), a horizontal bar, and a scrolling trace of the last fifteen
//! seconds. All three are clickable — the caller cycles the face on a click —
//! and all three switch to a TX meter while transmitting, where the headline
//! reading is SWR on its own logarithmic scale, drawn as a real meter rather
//! than a number.
//!
//! The three share their scales, colour ramps and header, so a reading looks
//! the same wherever it is shown; only the visualisation differs.
//!
//! The meter fills its whole box edge-to-edge: it paints its own instrument
//! face over the full allocated rect and draws the box border on top, so there
//! is no padding between the instrument and the frame.
//!
//! Everything is laid out against `k`, the box height over its 72 px design
//! height, so the whole instrument scales with the box it is handed.

use crate::i18n::tr;
use eframe::egui::{
    Align2, Color32, FontId, Id, Painter, Pos2, Rangef, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2,
    epaint::{Mesh, Vertex, WHITE_UV},
    pos2, vec2,
};
use sdroxide_types::Meters;

/// Scale endpoints: S0 = -127 dBm … S9+60 = -13 dBm.
const DBM_LO: f32 = -127.0;
const DBM_HI: f32 = -13.0;
const S9_DBM: f32 = -73.0;

/// Widest chord the needle's scale spans. The arc's radius follows the chord,
/// and its ends dip `rad·(1 − cos 31°)` below the top — past ~439 pt of face
/// that reaches below a 72 pt box. In a face wider than this the scale rides
/// centred instead, so the desktop strip can hand the meter any width the row
/// has over; see `the_stretched_desktop_smeter_keeps_its_scale_inside_its_box`.
pub(crate) const NEEDLE_FACE_MAX_W: f32 = 430.0;

/// SWR at the top of the SWR scale. With 3:1 pinned to the halfway point and a
/// logarithmic scale, the end of travel falls out as 3² = 9:1.
const SWR_MAX: f32 = 9.0;

/// The current theme's instrument colours: face, needle, backlight, cool-side
/// inks and ramps.
fn meter() -> &'static crate::theme::MeterPalette {
    crate::theme::meter_palette()
}

/// Whether the face is a printed dial rather than lit glass.
///
/// Only the *paper* face changes the instrument's physics — no bloom, a cast
/// shadow instead of a halo, marks that shade darker than the fill rather than
/// lighter — so this is the one distinction the drawing code asks about
/// directly. Which of the three reds to print in goes through [`hot`] instead.
fn paper() -> bool {
    meter().face == crate::theme::MeterFace::Paper
}

/// Whether the face blooms on receive: the backlight rising behind the scale
/// and the halo under the lit arc.
///
/// A bloom is light spread thin, which is the exact opposite of what the
/// high-contrast face is for — both of them fog the graduations they sit
/// behind, and against black they buy nothing the graduations were not already
/// saying. The printed dial has no lamp either, but it keeps its pale wash:
/// 30% of a near-white over white card is not light spread thin, it is
/// nothing, and it is what makes the card look like card.
fn blooms() -> bool {
    meter().face != crate::theme::MeterFace::Contrast
}

/// One of the reds that mean "past the red-line", in the shade the current
/// face needs.
///
/// The hot half of every scale stays red on every face — that rule has not
/// moved. What changes is only *which* red. The lit-glass shades are picked to
/// glow off a near-black dial: on white card they sit at 1.6:1 to 3:1 and read
/// as pastel decoration rather than as a warning, and on the high-contrast
/// face the muted ones among them — the minor graduations above S9 especially
/// — would be the only dim thing left on the instrument.
fn hot(glass: Color32, print: Color32, bright: Color32) -> Color32 {
    match meter().face {
        crate::theme::MeterFace::Glass => glass,
        crate::theme::MeterFace::Paper => print,
        crate::theme::MeterFace::Contrast => bright,
    }
}

/// The red-line red: the transmit chip's accent.
#[allow(non_snake_case)]
fn RED() -> Color32 {
    hot(c(0xff3c46), c(0xc40024), c(0xff5555))
}

/// The transmit backlight — red on every face, like [`RED`] and the hot inks
/// below. The themed half of the instrument lives in
/// [`crate::theme::MeterPalette`]; everything that means "past the red-line"
/// or "transmitting" deliberately does not.
///
/// On the paper dial this is not a backlight but a wash: 34% of it over white
/// card comes out a clear rose, which is the same message by other means.
#[allow(non_snake_case)]
fn BACKLIGHT_TX() -> Color32 {
    hot(c(0x8e1e2c), c(0x8e1e2c), c(0xff3344))
}

/// Readout text: the headline value and the smaller sub-reading beside it.
/// Uppercase like the constants these were before the instrument followed the
/// theme, matching the `theme::` accessors.
#[allow(non_snake_case)]
fn READOUT() -> Color32 {
    meter().readout
}
#[allow(non_snake_case)]
fn SUBDUED() -> Color32 {
    meter().subdued
}

/// Tick and label inks, cool (below the red-line, themed) and hot (above it,
/// always red — see [`hot`] for which red).
#[allow(non_snake_case)]
fn TICK_MINOR() -> [Color32; 2] {
    [meter().tick_minor, hot(c(0x934e52), c(0x9c2b2b), c(0xff8080))]
}
#[allow(non_snake_case)]
fn TICK_MAJOR() -> [Color32; 2] {
    [meter().tick_major, hot(c(0xff928a), c(0xa80e20), c(0xff9999))]
}
#[allow(non_snake_case)]
fn LABEL() -> [Color32; 2] {
    [meter().label, hot(c(0xff7d74), c(0xa80e20), c(0xff8080))]
}
/// Trace-view gridlines and the axis labels beside them, cool and hot.
#[allow(non_snake_case)]
fn GRID_LINE() -> [Color32; 2] {
    [meter().grid_line, hot(c(0x47232b), c(0xe7c2c6), c(0xb36e6e))]
}
#[allow(non_snake_case)]
fn GRID_LABEL() -> [Color32; 2] {
    [meter().grid_label, hot(c(0xbb605d), c(0x9c3a38), c(0xffa3a3))]
}

/// `c(0xff2a55)` — one hex triple, so a three-way [`hot`] still fits on a line.
const fn c(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Position on the S-scale for `dbm`, 0.0 (S0) … 1.0 (S9+60).
fn frac_of(dbm: f32) -> f32 {
    ((dbm - DBM_LO) / (DBM_HI - DBM_LO)).clamp(0.0, 1.0)
}

/// Position on the SWR scale, 0.0 (1:1) … 1.0 (9:1). Logarithmic, so 3:1 — the
/// point where most operators stop transmitting — lands exactly halfway.
fn swr_frac(swr: f32) -> f32 {
    (swr.max(1.0).ln() / SWR_MAX.ln()).clamp(0.0, 1.0)
}

/// Which face the meter wears — the operator's choice, kept in `[ui]` with
/// the rest of this screen's preferences (see [`sdroxide_types::SmeterStyle`]),
/// and re-exported here so a caller drawing a meter has the face beside the
/// widget that draws it.
pub use sdroxide_types::SmeterStyle;

/// What the meter is showing right now, in words, for the hover.
///
/// The face is dense by design — a chip, a right-hand figure, one or two bars
/// and a scale — and none of it says what it *is*. An operator seeing a needle
/// at 50 during an over has no way to tell whether that is half the radio's
/// power, half its modulation, or something else entirely, and an operator
/// looking for SWR has no way to tell a radio that has no bridge from a reading
/// that is missing (issue #373). So every line here names the quantity and its
/// units, and the transmit case says outright when the radio measures nothing.
pub fn hover_text(meters: Option<&Meters>) -> String {
    let mut out = String::new();
    if let Some(tx) = meters.and_then(|m| m.tx.as_ref()) {
        out.push_str(&tr!("smeter-alc-how-hard-transmitter-being"));
        match tx.fwd_w {
            Some(w) => {
                out.push_str(&tr!("smeter-forward-power-w-measured-radio", w = format!("{:.1}", w)))
            }
            None => out.push_str(&tr!("smeter-radio-reports-no-forward-power")),
        }
        if let Some(po) = tx.po {
            out.push_str(&tr!(
                "smeter-po-radio-s-own-power",
                clamp = format!("{:.0}", po.clamp(0.0, 1.0) * 100.0)
            ));
        }
        match tx.swr {
            Some(swr) => out.push_str(&tr!(
                "smeter-swr-1-match-antenna-presenting",
                swr = format!("{:.1}", swr)
            )),
            None => out.push_str(&tr!("smeter-no-swr-radio-has-no")),
        }
        return out;
    }

    let Some(m) = meters else {
        return tr!("smeter-no-signal-report-yet-receiver");
    };
    let (s, over) = m.s_units();
    if over > 0.0 {
        out.push_str(&tr!(
            "smeter-received-signal-strength-s9-db",
            over = format!("{:.0}", over),
            s_dbm = format!("{:.0}", m.s_dbm)
        ));
    } else {
        out.push_str(&tr!(
            "smeter-received-signal-strength-s-dbm",
            value = s,
            s_dbm = format!("{:.0}", m.s_dbm)
        ));
    }
    out.push_str(&tr!("smeter-s9-73-dbm-each-s"));
    if m.adc_overloaded() {
        out.push_str(&tr!("smeter-ovld-converter-into-rails-so"));
    }
    if let Some(t) = m.pa_temp_c {
        out.push_str(&tr!("smeter-radio-reports-c", t = format!("{:.0}", t)));
    }
    if let Some(ps) = m.puresignal {
        if ps.locked {
            out.push_str(&tr!(
                "smeter-ps-puresignal-correcting-db-compression",
                correction_db = format!("{:.1}", ps.correction_db),
                held = if ps.frozen { tr!("smeter-table-held") } else { String::from("") },
                score = format!("{:.2}", ps.score)
            ));
        } else {
            out.push_str(&tr!(
                "smeter-ps-puresignal-switched-but-has",
                score = format!("{:.2}", ps.score)
            ));
        }
    }
    out
}

/// Draw the S-meter in the selected style, filling the box's full interior.
/// Returns the (clickable) response so the caller can cycle the style.
/// `trace` is how the trace face scrolls; the other faces ignore it. `snr` is
/// the S/N face's reading, `None` while the S/N guide is off.
pub fn show(
    ui: &mut Ui,
    meters: Option<&Meters>,
    style: SmeterStyle,
    trace: TraceOpts,
    snr: Option<&SnrReading>,
) -> Response {
    // Fill whatever the (zero-padding) box hands us.
    let size = ui.available_size();
    match style {
        SmeterStyle::Needle => show_needle(ui, meters, size),
        SmeterStyle::Bar => show_bar(ui, meters, size),
        SmeterStyle::Trace => show_trace(ui, meters, size, trace),
        // On transmit every face turns into the transmit meter; S/N has
        // nothing to say there, so the bar face's transmit rows stand in.
        SmeterStyle::Snr if meters.is_some_and(|m| m.tx.is_some()) => show_bar(ui, meters, size),
        SmeterStyle::Snr => show_snr(ui, size, snr),
    }
}

/// Stops on the S scale the trace's range rail snaps to: S0…S9, then S9+10 …
/// S9+60 in 10 dB steps.
pub const STOPS: u8 = 16;

/// A stored S range made safe: `lo < hi ≤ STOPS - 1`, whatever a hand-edited
/// config holds.
pub fn s_range_clamped(lo: u8, hi: u8) -> (u8, u8) {
    let hi = hi.clamp(1, STOPS - 1);
    (lo.min(hi - 1), hi)
}

/// The level of stop `i` in dBm: S-units of 6 dB up to S9, then 10 dB a stop.
pub fn stop_dbm(i: u8) -> f32 {
    let i = i.min(STOPS - 1);
    if i <= 9 { DBM_LO + 6.0 * f32::from(i) } else { S9_DBM + 10.0 * f32::from(i - 9) }
}

/// The stop's name on the rail: `S3`, `S9`, `+20`.
pub fn stop_label(i: u8) -> String {
    let i = i.min(STOPS - 1);
    if i <= 9 { format!("S{i}") } else { format!("+{}", 10 * (i - 9)) }
}

/// What the S/N face shows: the passband's S/N (mean over the cursor's
/// width) and the strongest cell at its middle, both in dB over the guide's
/// noise floor, the passband's width and how many frames the guide has
/// averaged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnrReading {
    pub db: f32,
    pub mid_db: f32,
    pub width_hz: f64,
    pub frames: u32,
}

/// S/N over the passband `f_lo..f_hi` from the guide's cells (`lo_hz` and
/// `cell_hz` lay them out, `snr_db` is each cell's mean over the floor):
/// the passband's power over the noise the same width would carry — the mean
/// of the cells in linear power, back in dB — and the strongest cell at the
/// passband's middle, within a tenth of its width either side
/// ([`MID_FRACTION`]; at least the cell the middle falls in). `None` where
/// no cell in the passband has been measured.
///
/// The mean is (S+N)/N like the S/N line itself: an empty passband reads
/// about 0 dB, a carrier in a 2.4 kHz filter reads its power against
/// 2.4 kHz of noise. The middle value is a tuning aid: a signal tuned into
/// the centre of the filter shows there, one at the filter's edge does not.
// Why: HDZ 2026-11-09: "das S/N-Meter soll sich im Max auf Mitte und im Mean
//      auf die Breite des Cursors beziehen".
pub fn passband_snr(
    lo_hz: f64,
    cell_hz: f64,
    snr_db: &[f32],
    f_lo: f64,
    f_hi: f64,
) -> Option<(f32, f32)> {
    if !(cell_hz > 0.0) || f_hi <= f_lo {
        return None;
    }
    let first = ((f_lo - lo_hz) / cell_hz).floor().max(0.0) as usize;
    let last = (((f_hi - lo_hz) / cell_hz).ceil().max(0.0) as usize).min(snr_db.len());
    let (mut sum, mut n) = (0.0f64, 0usize);
    for v in snr_db.get(first..last).unwrap_or(&[]).iter().filter(|v| v.is_finite()) {
        sum += 10f64.powf(f64::from(*v) / 10.0);
        n += 1;
    }
    let half = MID_FRACTION * (f_hi - f_lo);
    let mid = 0.5 * (f_lo + f_hi);
    let m_first = ((mid - half - lo_hz) / cell_hz).floor().max(0.0) as usize;
    let m_last = ((((mid + half - lo_hz) / cell_hz).ceil().max(0.0)) as usize)
        .max(m_first + 1)
        .min(snr_db.len());
    let mid_max = snr_db
        .get(m_first..m_last)
        .unwrap_or(&[])
        .iter()
        .filter(|v| v.is_finite())
        .fold(f32::NEG_INFINITY, |a, v| a.max(*v));
    // Nothing measured at the middle: the marker is left off rather than
    // standing at the passband's edge.
    let mid_max = if mid_max.is_finite() { mid_max } else { f32::NAN };
    (n > 0).then(|| ((10.0 * (sum / n as f64).log10()) as f32, mid_max))
}

/// How far either side of the passband's middle the S/N face's middle
/// value looks, as a share of the passband's width.
pub const MID_FRACTION: f64 = 0.1;

/// How the trace face scrolls: samples a second, and which edge the newest
/// one sits at. Read from the screen's `[ui]` settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceOpts {
    pub hz: f64,
    pub newest_left: bool,
    /// The plot's bottom and top as stops on the S scale
    /// ([`stop_dbm`]); receive only.
    pub lo: u8,
    pub hi: u8,
}

impl TraceOpts {
    pub fn of(ui: &sdroxide_types::UiSettings) -> Self {
        let (lo, hi) = s_range_clamped(ui.smeter_trace_lo, ui.smeter_trace_hi);
        TraceOpts { hz: ui.smeter_trace_hz(), newest_left: ui.smeter_trace_newest_left, lo, hi }
    }

    /// The range as fractions of the full S scale, bottom below top.
    fn s_range(self) -> (f32, f32) {
        let (lo, hi) = (self.lo.min(self.hi), self.hi.max(self.lo));
        let hi = if hi == lo { lo + 1 } else { hi };
        (frac_of(stop_dbm(lo)), frac_of(stop_dbm(hi)))
    }

    /// Whether the range is narrower than the whole scale.
    fn spread(self) -> bool {
        self.s_range() != (0.0, 1.0)
    }
}

impl Default for TraceOpts {
    fn default() -> Self {
        TraceOpts { hz: TRACE_HZ, newest_left: false, lo: 0, hi: STOPS - 1 }
    }
}

/// The S-scale gridlines for a spread trace: one per stop inside the range,
/// thinned to every second, third … so at most five are drawn, labelled as
/// the rail names them less the "S" (`3`, `9`, `+20`). Positions are
/// fractions of the spread range.
fn spread_grid(opts: TraceOpts) -> Vec<Grid> {
    let (lo, hi) = (opts.lo.min(opts.hi), opts.hi.max(opts.lo));
    let (f0, f1) = opts.s_range();
    let inner: Vec<u8> = (lo + 1..hi).collect();
    let step = inner.len().div_ceil(5).max(1);
    inner
        .iter()
        .rev()
        .step_by(step)
        .map(|&i| {
            let f = frac_of(stop_dbm(i));
            let label = stop_label(i).trim_start_matches('S').to_string();
            grid_at((f - f0) / (f1 - f0), &label, i > 9)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Gradient primitives
// ---------------------------------------------------------------------------

/// A colour ramp stop, positioned in scale-fraction space.
type Stop = (f32, Color32);

/// S-meter ramp: the theme's cool side climbing to ice at S9, then the
/// over-S9 half in amber through to red — the warning half is the same in
/// every theme.
fn s_stops() -> Vec<Stop> {
    let m = meter();
    vec![
        (0.00, m.ramp_lo),
        (0.26, m.ramp_mid),
        (0.4737, m.ramp_hi),
        (0.4738, hot(c(0xffd23f), c(0xc88a00), c(0xffff00))),
        (0.74, hot(c(0xff8a3f), c(0xc45c00), c(0xffaa33))),
        (1.00, hot(c(0xff2a55), c(0xc40024), c(0xff5555))),
    ]
}

/// Power-output ramp: one cool-to-warm sweep with no red in it.
///
/// Deliberately unlike the ALC and SWR ramps, which turn red because their top
/// ends are faults — overdriving, and a bad match. A rig sitting at full power
/// output is doing exactly what it was asked to, so colouring the top of this
/// scale as a warning would teach the eye to read a healthy transmitter as a
/// problem. What the operator is watching for here is the needle FALLING while
/// drive stays put, and that shows as movement rather than as colour.
fn po_stops() -> Vec<Stop> {
    let m = meter();
    vec![(0.00, m.ramp_lo), (0.35, m.ramp_mid), (1.00, m.ramp_hi)]
}

/// SWR ramp: green while the match is good, amber as it approaches 3:1, and
/// the whole upper half — everything past 3:1 — in red. Deliberately the same
/// in every theme: green-means-safe and red-means-stop are not up for
/// restyling.
fn swr_stops() -> Vec<Stop> {
    vec![
        (0.00, hot(c(0x3fe086), c(0x0d7030), c(0x3ff23f))),
        (0.32, hot(c(0x9de05c), c(0x5c7d0a), c(0xcaff3f))),
        (0.4999, hot(c(0xffc03a), c(0xc88a00), c(0xffff00))),
        (0.50, hot(c(0xff3c46), c(0xc40024), c(0xff5555))),
        (1.00, hot(c(0xd60c2e), c(0x8c001a), c(0xff3333))),
    ]
}

/// Drive/ALC ramp, used when the rig reports no SWR bridge. Its cool side is
/// themed like the S ramp's.
fn alc_stops() -> Vec<Stop> {
    let m = meter();
    vec![
        (0.00, m.ramp_mid),
        (0.70, m.ramp_hi),
        (0.86, hot(c(0xffd23f), c(0xc88a00), c(0xffff00))),
        (1.00, hot(c(0xff2a55), c(0xc40024), c(0xff5555))),
    ]
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let c = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        c(a.r(), b.r()),
        c(a.g(), b.g()),
        c(a.b(), b.b()),
        c(a.a(), b.a()),
    )
}

/// Sample a stop list at `f`.
fn ramp(stops: &[Stop], f: f32) -> Color32 {
    let f = f.clamp(0.0, 1.0);
    let mut i = 0;
    while i + 2 < stops.len() && f > stops[i + 1].0 {
        i += 1;
    }
    let (a, ca) = stops[i];
    let (b, cb) = stops[i + 1];
    mix(ca, cb, if b > a { (f - a) / (b - a) } else { 0.0 })
}

fn push_quad(mesh: &mut Mesh, a: Pos2, b: Pos2, ca: Color32, cb: Color32) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex { pos: a, uv: WHITE_UV, color: ca });
    mesh.vertices.push(Vertex { pos: b, uv: WHITE_UV, color: cb });
    if base >= 2 {
        mesh.indices.extend_from_slice(&[base - 2, base - 1, base, base - 1, base + 1, base]);
    }
}

/// Vertical two-stop gradient over `rect`.
fn grad_v(p: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    push_quad(&mut mesh, rect.left_top(), rect.left_bottom(), top, bottom);
    push_quad(&mut mesh, rect.right_top(), rect.right_bottom(), top, bottom);
    p.add(Shape::mesh(mesh));
}

/// Horizontal gradient over `rect`, taking its colour from `col` sampled across
/// the scale fractions `f0..f1`. The fill is shaded top-to-bottom as well, so a
/// bar reads as a lit tube rather than a flat block.
fn grad_h(p: &Painter, rect: Rect, f0: f32, f1: f32, col: impl Fn(f32) -> Color32) {
    if rect.width() <= 0.0 {
        return;
    }
    let segs = (rect.width() / 3.0).ceil().clamp(1.0, 256.0) as usize;
    let mut top = Mesh::default();
    let mut bot = Mesh::default();
    let mid = rect.top() + rect.height() * 0.42;
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let x = rect.left() + rect.width() * t;
        let c = col(f0 + (f1 - f0) * t);
        push_quad(&mut top, pos2(x, rect.top()), pos2(x, mid), shade(c, 0.42), c);
        push_quad(&mut bot, pos2(x, mid), pos2(x, rect.bottom()), c, shade(c, -0.42));
    }
    p.add(Shape::mesh(top));
    p.add(Shape::mesh(bot));
}

/// Make `c` stand out from the fill around it by `amount` — a lit edge, a peak
/// marker, a cap on the trace.
///
/// Away from the face, whichever way that is: on lit glass the mark is
/// brighter than the bar it rides on, and on white card it is darker. Doing
/// this by hand at each call site is how a printed instrument ends up with a
/// leading edge that fades into the page instead of marking it.
fn lift(c: Color32, amount: f32) -> Color32 {
    shade(c, if paper() { -amount } else { amount })
}

/// The unlit remainder of a rail: the ramp's own hue at `amount` of its
/// strength, so the red zone is visible before the bar ever reaches it.
///
/// Taken down less far on the high-contrast face. The standard fraction leaves
/// the hot end of the travel at about 2:1 against black — enough to hint at,
/// not enough to read — and how much scale is left is exactly the thing a
/// meter is being looked at for.
fn unlit(c: Color32, amount: f32) -> Color32 {
    c.linear_multiply(if blooms() { amount } else { amount * 1.9 })
}

/// Lighten (`amount` > 0) or darken a colour, keeping its alpha.
fn shade(c: Color32, amount: f32) -> Color32 {
    if amount >= 0.0 {
        mix(c, Color32::from_rgba_premultiplied(c.a(), c.a(), c.a(), c.a()), amount)
    } else {
        mix(c, Color32::from_rgba_premultiplied(0, 0, 0, c.a()), -amount)
    }
}

/// Soft radial bloom, fading to nothing at `radius`.
fn glow(p: &Painter, center: Pos2, radius: f32, color: Color32) {
    const N: u32 = 40;
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex { pos: center, uv: WHITE_UV, color });
    for i in 0..=N {
        let a = i as f32 / N as f32 * std::f32::consts::TAU;
        mesh.vertices.push(Vertex {
            pos: center + vec2(radius * a.cos(), radius * a.sin()),
            uv: WHITE_UV,
            color: Color32::TRANSPARENT,
        });
    }
    for i in 1..=N {
        mesh.indices.extend_from_slice(&[0, i, i + 1]);
    }
    p.add(Shape::mesh(mesh));
}

/// The analog scale's geometry: a pivot far below the box, a rail radius, and
/// the half-angle the needle sweeps either side of vertical.
struct Arc {
    pivot: Pos2,
    rad: f32,
    half: f32,
}

impl Arc {
    /// Needle angle for a scale fraction, 0 at the left stop … 1 at the right.
    fn ang(&self, f: f32) -> f32 {
        (f.clamp(0.0, 1.0) - 0.5) * 2.0 * self.half
    }

    /// The point `r` out from the pivot along `a`.
    fn at(&self, r: f32, a: f32) -> Pos2 {
        self.pivot + vec2(r * a.sin(), -r * a.cos())
    }

    /// How many segments a sweep of `f0..f1` needs to look smooth.
    fn segs(&self, f0: f32, f1: f32, per_turn: f32) -> usize {
        ((f1 - f0) * per_turn).ceil().clamp(1.0, 256.0) as usize
    }
}

/// A bloom hugging the rail: brightest on the arc itself and fading to nothing
/// `width` either side of it, so the lit part of the scale looks like it is
/// glowing rather than sitting inside a rectangle.
fn arc_glow(p: &Painter, arc: &Arc, width: f32, f0: f32, f1: f32, col: impl Fn(f32) -> Color32) {
    if f1 <= f0 {
        return;
    }
    let segs = arc.segs(f0, f1, 96.0);
    let (mut inner, mut outer) = (Mesh::default(), Mesh::default());
    for i in 0..=segs {
        let f = f0 + (f1 - f0) * i as f32 / segs as f32;
        let a = arc.ang(f);
        let color = col(f);
        let (lo, mid, hi) =
            (arc.at(arc.rad - width, a), arc.at(arc.rad, a), arc.at(arc.rad + width, a));
        push_quad(&mut inner, lo, mid, Color32::TRANSPARENT, color);
        push_quad(&mut outer, mid, hi, color, Color32::TRANSPARENT);
    }
    p.add(Shape::mesh(inner));
    p.add(Shape::mesh(outer));
}

/// A band swept between two radii over the scale fractions `f0..f1`, coloured
/// by `col`. This is the analog scale's rail — one mesh, so the ramp runs
/// smoothly along the arc instead of stepping between segments.
fn arc_band(
    p: &Painter,
    arc: &Arc,
    radii: (f32, f32),
    f0: f32,
    f1: f32,
    col: impl Fn(f32) -> Color32,
) {
    if f1 <= f0 {
        return;
    }
    let segs = arc.segs(f0, f1, 128.0);
    let mut mesh = Mesh::default();
    for i in 0..=segs {
        let f = f0 + (f1 - f0) * i as f32 / segs as f32;
        let a = arc.ang(f);
        let color = col(f);
        push_quad(&mut mesh, arc.at(radii.0, a), arc.at(radii.1, a), color, color);
    }
    p.add(Shape::mesh(mesh));
}

// ---------------------------------------------------------------------------
// Scales
// ---------------------------------------------------------------------------

/// One graduation: where it sits, how prominent it is, what it is called, and
/// whether it falls in the scale's hot (over-limit) zone.
struct Tick {
    f: f32,
    major: bool,
    label: Option<String>,
    hot: bool,
}

/// A reference level for the trace view's gridlines. Deliberately sparser than
/// the graduations: a 72 px plot only has room for a handful.
struct Grid {
    f: f32,
    label: String,
    hot: bool,
}

/// Which quantity a scale measures. The trace view keys its history on this, so
/// switching between receive and transmit starts a fresh plot rather than
/// splicing S-units onto SWR.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScaleKind {
    S,
    Swr,
    Alc,
    Po,
    Snr,
}

/// A meter scale: its graduations, its gridlines, and the ramp along its rail.
struct Scale {
    kind: ScaleKind,
    ticks: Vec<Tick>,
    grid: Vec<Grid>,
    stops: Vec<Stop>,
}

fn grid_at(f: f32, label: &str, hot: bool) -> Grid {
    Grid { f, label: label.to_string(), hot }
}

/// S0…S9 every S-unit (numbered on the odd ones), then 10 dB steps to S9+60.
fn s_scale() -> Scale {
    let mut ticks = Vec::new();
    for i in 0..=9 {
        let major = i % 2 == 1;
        ticks.push(Tick {
            f: frac_of(S9_DBM - (9 - i) as f32 * 6.0),
            major,
            label: major.then(|| i.to_string()),
            hot: false,
        });
    }
    for over in [10.0f32, 20.0, 30.0, 40.0, 50.0, 60.0] {
        let major = (over as i32) % 20 == 0;
        ticks.push(Tick {
            f: frac_of(S9_DBM + over),
            major,
            label: major.then(|| format!("+{over:.0}")),
            hot: true,
        });
    }
    let grid = vec![
        grid_at(frac_of(S9_DBM - 36.0), "3", false),
        grid_at(frac_of(S9_DBM - 18.0), "6", false),
        grid_at(frac_of(S9_DBM), "9", false),
        grid_at(frac_of(S9_DBM + 30.0), "+30", true),
    ];
    Scale { kind: ScaleKind::S, ticks, grid, stops: s_stops() }
}

/// 1:1 … 9:1, logarithmic, with 3:1 at mid-travel and everything past it hot.
fn swr_scale() -> Scale {
    const GRADS: &[(f32, bool)] = &[
        (1.0, true),
        (1.2, false),
        (1.5, true),
        (2.0, true),
        (2.5, false),
        (3.0, true),
        (4.0, false),
        (5.0, true),
        (7.0, false),
        (9.0, true),
    ];
    let mut ticks = Vec::new();
    for &(swr, major) in GRADS {
        // Whole ratios read better without a trailing zero; 1.5 needs its .5.
        let label = if swr.fract() == 0.0 { format!("{swr:.0}") } else { format!("{swr:.1}") };
        ticks.push(Tick {
            f: swr_frac(swr),
            major,
            label: major.then_some(label),
            hot: swr >= 3.0,
        });
    }
    let grid = vec![
        grid_at(swr_frac(1.5), "1.5", false),
        grid_at(swr_frac(2.0), "2", false),
        grid_at(swr_frac(3.0), "3", true),
        grid_at(swr_frac(5.0), "5", true),
    ];
    Scale { kind: ScaleKind::Swr, ticks, grid, stops: swr_stops() }
}

/// 0…100 % drive, the fallback when the rig has no SWR bridge.
fn alc_scale() -> Scale {
    let ticks = (0..=10)
        .map(|i| {
            let f = i as f32 / 10.0;
            let major = i % 5 == 0;
            Tick { f, major, label: major.then(|| format!("{}", i * 10)), hot: f >= 0.9 }
        })
        .collect();
    let grid = vec![
        grid_at(0.25, "25", false),
        grid_at(0.5, "50", false),
        grid_at(0.75, "75", false),
        grid_at(0.95, "95", true),
    ];
    Scale { kind: ScaleKind::Alc, ticks, grid, stops: alc_stops() }
}

/// 0…100 % of the rig's own power-output meter.
///
/// Graduated like the ALC scale beside it so the two rows read as one
/// instrument: the same ten divisions, numbered every 50. The gridlines differ
/// because what matters on them differs — none of these is a limit, so none is
/// marked `hot`.
fn po_scale() -> Scale {
    let ticks = (0..=10)
        .map(|i| {
            let f = i as f32 / 10.0;
            let major = i % 5 == 0;
            Tick { f, major, label: major.then(|| format!("{}", i * 10)), hot: false }
        })
        .collect();
    let grid =
        vec![grid_at(0.25, "25", false), grid_at(0.5, "50", false), grid_at(0.75, "75", false)];
    Scale { kind: ScaleKind::Po, ticks, grid, stops: po_stops() }
}

/// The S/N face's scale: 0…40 dB over the noise floor, graduated every 5 dB
/// and numbered every 10, in the green of the S/N line.
const SNR_FULL_DB: f32 = 40.0;
fn snr_scale() -> Scale {
    let ticks = (0..=8)
        .map(|i| {
            let db = i as f32 * 5.0;
            let major = i % 2 == 0;
            Tick {
                f: db / SNR_FULL_DB,
                major,
                label: major.then(|| format!("{db:.0}")),
                hot: false,
            }
        })
        .collect();
    let grid =
        vec![grid_at(0.25, "10", false), grid_at(0.5, "20", false), grid_at(0.75, "30", false)];
    let g = Color32::from_rgb(120, 255, 140);
    let stops = vec![(0.0, g.linear_multiply(0.25)), (0.3, g.linear_multiply(0.6)), (1.0, g)];
    Scale { kind: ScaleKind::Snr, ticks, grid, stops }
}

/// The S/N face's chip: the bar's mean, then the marker's value (the strongest
/// cell at the middle) where there is one — the chip is where the eye goes,
/// and the fine line's figure was otherwise only in the header.
// Why: HDZ 2026-11-10: "ergänze den Wert links noch um den Cursor-Wert … das
//      Maximum, den Wert, der zur feinen Linie passt".
fn snr_chip(s: &SnrReading) -> String {
    if s.mid_db.is_finite() {
        format!("S/N {:.1} · {:.1} dB", s.db, s.mid_db)
    } else {
        format!("S/N {:.1} dB", s.db)
    }
}

/// The S/N face: the passband's S/N as a bar, the strongest cell at its
/// middle as the marker on it, and in the header the figure, the filter's width and the
/// frames behind it. Says so when the S/N guide is off.
fn show_snr(ui: &mut Ui, size: Vec2, snr: Option<&SnrReading>) -> Response {
    let (rect, resp) = face(ui, size, false);
    let p = ui.painter_at(rect);
    let k = (rect.height() / 72.0).clamp(0.55, 2.0);
    let frac = |db: f32| (db / SNR_FULL_DB).clamp(0.0, 1.0);
    let r = match snr {
        Some(s) => Reading {
            scale: snr_scale(),
            frac: frac(s.db),
            chip: snr_chip(s),
            accent: Color32::from_rgb(120, 255, 140),
            right: tr!(
                "smeter-snr-right",
                mid = if s.mid_db.is_finite() {
                    format!("{:.0}", s.mid_db)
                } else {
                    "—".to_string()
                },
                khz = format!("{:.1}", s.width_hz / 1000.0),
                frames = s.frames
            )
            .to_string(),
            right_strong: false,
            right_alert: false,
        },
        None => Reading {
            scale: snr_scale(),
            frac: 0.0,
            chip: "S/N —".to_string(),
            accent: SUBDUED(),
            right: tr!("smeter-snr-off").to_string(),
            right_strong: false,
            right_alert: false,
        },
    };
    header(&p, rect, &r, k);
    let left = rect.left() + 6.0 * k;
    let right = rect.right() - 6.0 * k;
    let bar_rect =
        Rect::from_min_max(pos2(left, rect.top() + 21.0 * k), pos2(right, rect.top() + 45.0 * k));
    bar_row(
        &p,
        bar_rect,
        &r.scale,
        r.frac,
        snr.filter(|s| s.mid_db.is_finite()).map(|s| frac(s.mid_db)),
    );
    bar_ticks(&p, bar_rect, rect, &r.scale, k, bar_rect.bottom() + 8.0 * k);
    border(ui, rect);
    resp
}

/// What the instrument is currently displaying: the scale to draw, where the
/// needle sits on it, and the two readouts in the header.
struct Reading {
    scale: Scale,
    frac: f32,
    /// Headline value in the top-left chip, and the chip's accent.
    chip: String,
    accent: Color32,
    /// Right-hand reading.
    right: String,
    /// Whether `right` is a co-equal reading (TX) or a quiet sub-label (RX).
    right_strong: bool,
    /// The right-hand reading is a warning, and prints in the red-line red.
    right_alert: bool,
}

fn reading(meters: Option<&Meters>) -> Reading {
    if let Some(tx) = meters.and_then(|m| m.tx.as_ref()) {
        let power = match tx.fwd_w {
            Some(w) => format!("{w:.1} W"),
            None => format!("ALC {:.0}%", tx.alc.clamp(0.0, 1.0) * 100.0),
        };
        return match tx.swr {
            Some(swr) => Reading {
                scale: swr_scale(),
                frac: swr_frac(swr),
                chip: format!("SWR {swr:.1}"),
                accent: RED(),
                right: power,
                right_strong: true,
                right_alert: false,
            },
            // No SWR bridge: the needle falls back to the engine's own drive
            // level, which is the only TX quantity every backend reports.
            None => Reading {
                scale: alc_scale(),
                frac: tx.alc.clamp(0.0, 1.0),
                chip: "TX".to_string(),
                accent: RED(),
                right: power,
                right_strong: true,
                right_alert: false,
            },
        };
    }

    let dbm = meters.map(|m| m.s_dbm).unwrap_or(DBM_LO);
    let (primary, secondary) = match meters {
        None => ("—".to_string(), String::new()),
        Some(_) if dbm > S9_DBM => {
            (format!("S9+{:.0}", dbm - S9_DBM), format!("{dbm:.0} dBm", dbm = dbm))
        }
        Some(m) => (format!("S{}", m.s_units().0), format!("{dbm:.0} dBm", dbm = dbm)),
    };
    // The converter is into its rails. This takes the dBm's place rather than
    // sitting beside it, because the dBm is no longer a measurement: a clipped
    // carrier reads *lower* than it is, so leaving the number up would have the
    // instrument quietly disagreeing with the warning printed next to it.
    let overload = meters.is_some_and(Meters::adc_overloaded);
    Reading {
        scale: s_scale(),
        frac: frac_of(dbm),
        chip: primary,
        accent: if overload { RED() } else { crate::theme::CYAN() },
        right: if overload { "OVL".to_string() } else { secondary },
        right_strong: overload,
        right_alert: overload,
    }
}

// ---------------------------------------------------------------------------
// Shared chrome
// ---------------------------------------------------------------------------

/// Paint the instrument face over the full box and return the rect + response.
/// The backlight goes red on transmit — a cue that costs no space in a box this
/// small, and that reads before any of the text does.
fn face(ui: &mut Ui, size: Vec2, tx: bool) -> (Rect, Response) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter_at(rect);
    let m = meter();
    grad_v(&p, rect, m.face_top, m.face_bot);
    // Backlight bloom rising from behind the scale, and a hairline of specular
    // along the top edge so the glass reads as glass.
    //
    // Transmit keeps its red wash on every face — whether RF is leaving the
    // antenna is not decoration. Receive gets one only where the face blooms.
    if tx || blooms() {
        glow(
            &p,
            pos2(rect.center().x, rect.top() + rect.height() * 0.72),
            rect.width() * 0.52,
            if tx {
                BACKLIGHT_TX().linear_multiply(0.34)
            } else {
                m.backlight.linear_multiply(0.30)
            },
        );
    }
    p.hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, m.glass.linear_multiply(0.7)));
    (rect, resp)
}

/// The board temperature above which the tag stops being subdued, and the one
/// above which it turns red, in degrees Celsius.
///
/// A Hermes-Lite 2 idles in the thirties and settles somewhere in the forties
/// under a duty cycle it is happy with; the fifties are the point at which an
/// operator would want to know, and past seventy the board is being asked for
/// more than its heatsinking can carry away. Neither figure is a limit the
/// radio enforces — this is an instrument, not a protection circuit, and the
/// colours are there to catch the eye rather than to make a claim.
const TEMP_WARM_C: f32 = 55.0;
const TEMP_HOT_C: f32 = 70.0;

/// The radio's own temperature, in the face's bottom-left corner, where the
/// board reports one (issue #333).
///
/// Down there rather than on the header strip because it is not the reading
/// this instrument is for: the signal or the power is, and neither may give up
/// its place. It is a background fact about the hardware that happens to be
/// worth a corner of the one panel an operator already watches — which is why
/// it says nothing at all when the radio has no sensor, and why it is
/// subdued until it is worth noticing.
fn temperature(p: &Painter, rect: Rect, meters: Option<&Meters>, k: f32) {
    let Some(deg) = meters.and_then(|m| m.pa_temp_c) else { return };
    let ink = if deg >= TEMP_HOT_C {
        RED()
    } else if deg >= TEMP_WARM_C {
        hot(c(0xffc03a), c(0xc88a00), c(0xffff00))
    } else {
        SUBDUED()
    };
    p.text(
        pos2(rect.left() + 6.0 * k, rect.bottom() - 3.0 * k),
        Align2::LEFT_BOTTOM,
        format!("{deg:.0}°C"),
        FontId::monospace(9.0 * k.max(0.85)),
        ink,
    );
}

/// The predistortion loop's state, in the face's bottom-**right** corner,
/// opposite the temperature (issue #441).
///
/// PureSignal fails silently by nature: a loop that never finds its feedback
/// leaves the transmitter exactly as it would have been with the feature off,
/// and nothing about the transmission says which of the two happened. Until
/// this reached the screen, the only place the answer was ever written was the
/// log. So the indicator's job is not to report a number — it is to separate
/// "correcting" from "on but blind", which is why the unlocked state is drawn
/// in the warning colour rather than left subdued like the temperature beside
/// it.
///
/// Says nothing at all on a radio with no loop, which is nearly all of them.
fn puresignal(p: &Painter, rect: Rect, meters: Option<&Meters>, k: f32) {
    let Some(ps) = meters.and_then(|m| m.puresignal) else { return };
    let (text, ink) = if !ps.locked {
        // Blind: the feature is on and doing nothing.
        (tr!("smeter-ps").to_string(), hot(c(0xffc03a), c(0xc88a00), c(0xffff00)))
    } else {
        let held = if ps.frozen { "*" } else { "" };
        (format!("PS {:.0}dB{held}", ps.correction_db), SUBDUED())
    };
    p.text(
        pos2(rect.right() - 6.0 * k, rect.bottom() - 3.0 * k),
        Align2::RIGHT_BOTTOM,
        text,
        FontId::monospace(9.0 * k.max(0.85)),
        ink,
    );
}

/// Draw the box border on top of the finished meter (the meter face already
/// covers the whole rect, so the frame's own border would be hidden).
fn border(ui: &Ui, rect: Rect) {
    ui.painter_at(rect).rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, crate::theme::LINE_LIT()),
        StrokeKind::Inside,
    );
}

/// A small tag: gradient-filled, accent-outlined, with the text inside. Returns
/// its rect so the caller can lay the next element out beside it.
fn chip(p: &Painter, top_left: Pos2, text: &str, accent: Color32, pt: f32) -> Rect {
    // On glass the label is the accent lifted towards white; on card there is
    // nothing above it to lift towards, so it goes the other way. Baked into
    // the galley rather than passed to `Painter::galley`: that argument is
    // only a *fallback* for sections with no colour of their own, so a galley
    // laid out in white would stay white on both faces.
    let ink =
        if paper() { mix(accent, Color32::BLACK, 0.2) } else { mix(accent, Color32::WHITE, 0.65) };
    let g = p.layout_no_wrap(text.to_owned(), FontId::monospace(pt), ink);
    let pad = vec2(5.0, 1.5);
    let rect = Rect::from_min_size(top_left, g.size() + pad * 2.0);
    grad_v(p, rect, accent.linear_multiply(0.40), accent.linear_multiply(0.14));
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, accent.linear_multiply(0.80)), StrokeKind::Inside);
    p.galley(rect.min + pad, g, ink);
    rect
}

/// Type sizes on the header, as `(chip, right-hand readout)`.
///
/// Floored below the instrument's own scale: everything else on the face is
/// geometry, which reads fine at any size, but these two are *numbers*, and at
/// the `k` a phone-sized box works out to they would be grey mush. Positions
/// still follow `k`, so the header keeps its place on the face.
fn header_pt(k: f32) -> (f32, f32) {
    let tk = k.max(0.85);
    (11.0 * tk, 14.0 * tk)
}

/// The bottom of the strip [`header`] occupies — for the faces that have to
/// know what not to draw behind. Measured rather than assumed so it cannot
/// drift from what `header` actually paints.
fn header_h(p: &Painter, rect: Rect, k: f32) -> f32 {
    let (chip_pt, _) = header_pt(k);
    let text_h = p.layout_no_wrap("0".to_owned(), FontId::monospace(chip_pt), READOUT()).size().y;
    // 3·k down from the top, the chip's own 1.5 pt of padding either side of the
    // text, then 2·k of clearance under it.
    rect.top() + 3.0 * k + text_h + 3.0 + 2.0 * k
}

/// The top row every face shares: the headline value in a chip on the left, the
/// secondary reading on the right. Returns the strip it occupies.
fn header(p: &Painter, rect: Rect, r: &Reading, k: f32) -> Rect {
    let head_y = rect.top() + 3.0 * k;
    let (chip_pt, strong_pt) = header_pt(k);
    let chip_rect = chip(p, pos2(rect.left() + 6.0 * k, head_y), &r.chip, r.accent, chip_pt);
    if !r.right.is_empty() {
        let (pt, ink) = match (r.right_strong, r.right_alert) {
            (_, true) => (strong_pt, RED()),
            (true, false) => (strong_pt, READOUT()),
            (false, false) => (chip_pt, SUBDUED()),
        };
        // The secondary reading goes only where there is room for it beside the
        // headline one. In a box narrow enough for the two to meet, the chip is
        // the one worth keeping — it carries the signal report, and the pair
        // printed over each other would cost both.
        let w = p.layout_no_wrap(r.right.clone(), FontId::monospace(pt), ink).size().x;
        if chip_rect.right() + 8.0 + w <= rect.right() - 6.0 * k {
            p.text(
                pos2(rect.right() - 6.0 * k, head_y),
                Align2::RIGHT_TOP,
                &r.right,
                FontId::monospace(pt),
                ink,
            );
        }
    }
    Rect::from_min_max(rect.left_top(), pos2(rect.right(), header_h(p, rect, k)))
}

/// Peak-hold state: the highest fraction seen, and when it was set.
#[derive(Clone, Copy)]
struct Peak {
    frac: f32,
    at: f64,
}

/// Needle motion: a lightly-damped second-order follower. `vel` is in fractions
/// of full scale per second.
#[derive(Clone, Copy)]
struct Motion {
    frac: f32,
    vel: f32,
}

/// Step the needle towards `target` and return where it is now, so it swings
/// into a new reading and settles instead of teleporting. Asks for a repaint
/// while it is still moving, so the swing stays smooth even when the spectrum
/// — and with it the window's frame rate — is running slowly.
fn needle_motion(ui: &Ui, id: Id, target: f32) -> f32 {
    /// How hard the needle is pulled towards the reading.
    const OMEGA: f32 = 26.0;
    /// Damping ratio. Just under 1: the hint of overshoot is what makes a
    /// moving-coil movement look like one.
    const ZETA: f32 = 0.75;

    let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
    let mut m = ui.data(|d| d.get_temp::<Motion>(id)).unwrap_or(Motion { frac: target, vel: 0.0 });
    // Semi-implicit Euler — stable at the step sizes a UI frame hands us.
    m.vel += (OMEGA * OMEGA * (target - m.frac) - 2.0 * ZETA * OMEGA * m.vel) * dt;
    let next = m.frac + m.vel * dt;
    if !(0.0..=1.0).contains(&next) {
        m.vel = 0.0; // the movement has hit its end stop
    }
    m.frac = next.clamp(0.0, 1.0);
    ui.data_mut(|d| d.insert_temp(id, m));
    if (m.frac - target).abs() > 0.0005 || m.vel.abs() > 0.002 {
        animate(ui);
    }
    m.frac
}

/// Ask for the next frame at the meter's own rate. Deliberately *not* a bare
/// `request_repaint`: on a busy band the needle is always creeping somewhere,
/// and an unpaced request would pin the whole window — panadapter included — at
/// the display's refresh rate for the sake of one small widget.
///
/// 33 ms is a ceiling on how fast the needle needs to move, not a promise that
/// it will: [`crate::repaint::after_ms`] floors every request at one frame, so
/// below 30 fps the meter follows the rate the operator set rather than
/// dragging the window up to its own.
fn animate(ui: &Ui) {
    crate::repaint::after_ms(ui.ctx(), 33);
}

/// The decaying peak marker for `frac` — held briefly at the maximum, then
/// falling back until the live reading catches up. `None` once it has.
fn peak_hold(ui: &Ui, id: Id, frac: f32) -> Option<f32> {
    /// Seconds the marker sits still before it starts falling.
    const HOLD: f64 = 0.7;
    /// How fast it falls afterwards, in fractions of full scale per second.
    const FALL: f32 = 0.55;

    let now = ui.input(|i| i.time);
    let shown = |p: Peak| p.frac - FALL * (now - p.at - HOLD).max(0.0) as f32;
    let prev = ui.data(|d| d.get_temp::<Peak>(id));
    let peak = match prev {
        Some(p) if shown(p) > frac => p,
        _ => Peak { frac, at: now },
    };
    ui.data_mut(|d| d.insert_temp(id, peak));
    let showing = shown(peak) > frac + 0.01;
    if showing {
        animate(ui); // keep the fall-back smooth even if no new reading arrives
    }
    showing.then(|| shown(peak).clamp(0.0, 1.0))
}

// ---------------------------------------------------------------------------
// Bar style
// ---------------------------------------------------------------------------

/// One labelled bar row: a recessed trough, a gradient fill with a lit leading
/// edge, and the peak marker riding on top.
fn bar_row(p: &Painter, rect: Rect, scale: &Scale, frac: f32, peak: Option<f32>) {
    let m = meter();
    grad_v(p, rect, m.rail_top, m.rail_bot);
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, m.rail_edge), StrokeKind::Inside);
    let inner = rect.shrink(1.0);
    // The unlit remainder of the rail keeps the ramp's hue, so the red zone is
    // visible before the bar ever reaches it.
    grad_h(p, inner, 0.0, 1.0, |f| unlit(ramp(&scale.stops, f), 0.16));

    let x = inner.left() + inner.width() * frac.clamp(0.0, 1.0);
    if x > inner.left() {
        let fill = Rect::from_min_max(inner.min, pos2(x, inner.bottom()));
        grad_h(p, fill, 0.0, frac, |f| ramp(&scale.stops, f));
        // Lit leading edge: a bar this small reads much faster with one.
        p.vline(
            x - 0.5,
            Rangef::new(inner.top(), inner.bottom()),
            Stroke::new(1.5, lift(ramp(&scale.stops, frac), 0.65)),
        );
    }
    if let Some(pk) = peak {
        let px = inner.left() + inner.width() * pk;
        p.vline(
            px,
            Rangef::new(inner.top(), inner.bottom()),
            Stroke::new(1.5, lift(ramp(&scale.stops, pk), 0.55)),
        );
    }
}

/// Graduations under a bar row: ticks against the bar's bottom edge and the
/// numbers below them, nudged inwards at the ends so the outermost number is
/// never half-eaten by the frame.
fn bar_ticks(p: &Painter, bar: Rect, bounds: Rect, scale: &Scale, k: f32, num_top: f32) {
    for t in &scale.ticks {
        let x = (bar.left() + bar.width() * t.f).round();
        let len = if t.major { 5.0 * k } else { 3.0 * k };
        let hot = usize::from(t.hot);
        p.vline(
            x,
            Rangef::new(bar.bottom() + 2.0 * k, bar.bottom() + 2.0 * k + len),
            Stroke::new(1.0, if t.major { TICK_MAJOR()[hot] } else { TICK_MINOR()[hot] }),
        );
        if let Some(label) = &t.label {
            let g = p.layout_no_wrap(label.clone(), FontId::monospace(9.0 * k), LABEL()[hot]);
            let x = (x - g.size().x * 0.5)
                .clamp(bounds.left() + 1.0, bounds.right() - g.size().x - 1.0);
            p.galley(pos2(x, num_top), g, LABEL()[hot]);
        }
    }
}

fn show_bar(ui: &mut Ui, meters: Option<&Meters>, size: Vec2) -> Response {
    let tx = meters.and_then(|m| m.tx);
    let (rect, resp) = face(ui, size, tx.is_some());
    let p = ui.painter_at(rect);
    let k = (rect.height() / 72.0).clamp(0.55, 2.0);
    let r = reading(meters);
    let peak = peak_hold(ui, resp.id, r.frac);

    let left = rect.left() + 6.0 * k;
    let right = rect.right() - 6.0 * k;
    header(&p, rect, &r, k);
    temperature(&p, rect, meters, k);
    puresignal(&p, rect, meters, k);

    if let Some(tx) = tx {
        // Two stacked rows: the engine's drive on top, and below it whichever
        // transmit meter the rig actually gives us. With nothing to put there
        // only the drive row is drawn, grown to fill the space.
        //
        // The lower row prefers POWER OUTPUT over SWR where the rig reports it,
        // which on the face of it throws away a reading. It does not. The SWR
        // figure keeps its place in the header chip, where it is a number and
        // is legible at a glance; what it was doing down here was drawing a bar
        // that sits at the extreme left of a logarithmic scale for the whole of
        // any normal over, so the row carried almost no information while
        // occupying half the instrument. Power output uses the same space to
        // show something that moves: whether the rig is delivering what it was
        // asked for, and whether it is folding back.
        //
        // A rig that reports no PO meter keeps the SWR row exactly as before.
        let alc = alc_scale();
        let row = |top: f32, bottom: f32| {
            Rect::from_min_max(
                pos2(left + 26.0 * k, rect.top() + top * k),
                pos2(right, rect.top() + bottom * k),
            )
        };
        // What goes in the lower row, if anything: its label, its scale, where
        // the bar sits on it, and whether it carries a peak-hold marker. Peak
        // hold belongs to the SWR reading (`r.frac` is the needle's own value,
        // and `peak` was computed from it); the PO bar is read live and a
        // held peak on it would just be the loudest syllable of the over.
        let lower = match (tx.po, tx.swr) {
            (Some(po), _) => Some(("PO", po_scale(), po.clamp(0.0, 1.0), None)),
            (None, Some(_)) => Some(("SWR", swr_scale(), r.frac, peak)),
            (None, None) => None,
        };
        let (drive_rect, lower_rect) = match lower {
            Some(_) => (row(20.0, 33.0), Some(row(36.0, 49.0))),
            None => (row(24.0, 42.0), None),
        };
        let row_label = |rect: Rect, text: &str| {
            p.text(
                pos2(rect.left() - 4.0 * k, rect.center().y),
                Align2::RIGHT_CENTER,
                text,
                FontId::monospace(9.5 * k),
                SUBDUED(),
            );
        };
        row_label(drive_rect, "ALC");
        bar_row(&p, drive_rect, &alc, tx.alc.clamp(0.0, 1.0), None);
        match (lower, lower_rect) {
            (Some((label, scale, frac, hold)), Some(lower_rect)) => {
                row_label(lower_rect, label);
                bar_row(&p, lower_rect, &scale, frac, hold);
                bar_ticks(&p, lower_rect, rect, &scale, k, lower_rect.bottom() + 8.0 * k);
            }
            _ => bar_ticks(&p, drive_rect, rect, &alc, k, drive_rect.bottom() + 8.0 * k),
        }
        border(ui, rect);
        return resp;
    }

    let bar_rect =
        Rect::from_min_max(pos2(left, rect.top() + 21.0 * k), pos2(right, rect.top() + 45.0 * k));
    bar_row(&p, bar_rect, &r.scale, r.frac, peak);
    bar_ticks(&p, bar_rect, rect, &r.scale, k, bar_rect.bottom() + 8.0 * k);
    border(ui, rect);
    resp
}

// ---------------------------------------------------------------------------
// Analog needle style
// ---------------------------------------------------------------------------

/// The needle: a tapered blade with a drop shadow and a soft bloom, running
/// from the (off-box) pivot out to the scale rail.
fn draw_needle(p: &Painter, pivot: Pos2, a: f32, r_tip: f32, k: f32) {
    let dir = vec2(a.sin(), -a.cos());
    let nrm = vec2(-dir.y, dir.x);
    // Width and colour along the blade. Only the outer third is ever on-box, so
    // the taper is tuned to look right there.
    const PROFILE: &[(f32, f32)] = &[(0.0, 1.0), (0.6, 0.82), (0.9, 0.48), (1.0, 0.05)];
    // On lit glass the blade is a filament: dark red at the root, hot red
    // along it, flaring almost white at the tip. A moving-coil meter's pointer
    // is painted metal, and a high-contrast one is plain white — so the three
    // stops come from the palette rather than from here.
    let m = meter();
    let ink = |t: f32| {
        if t < 0.9 {
            mix(m.needle_root, m.needle_mid, t / 0.9)
        } else {
            mix(m.needle_mid, m.needle_tip, (t - 0.9) / 0.1)
        }
    };

    // `soft` fades the blade out sideways, which is what makes the bloom read
    // as light rather than as a second, wider needle.
    let blade = |width: f32, off: Vec2, soft: bool, col: &dyn Fn(f32) -> Color32| {
        let (mut left, mut right) = (Mesh::default(), Mesh::default());
        for &(t, w) in PROFILE {
            let c = pivot + dir * (r_tip * t) + off;
            let hw = nrm * (width * w);
            let (core, edge) = (col(t), if soft { Color32::TRANSPARENT } else { col(t) });
            push_quad(&mut left, c - hw, c, edge, core);
            push_quad(&mut right, c, c + hw, core, edge);
        }
        p.add(Shape::mesh(left));
        p.add(Shape::mesh(right));
    };

    // The bloom is the blade's own light, so a printed pointer has none — it
    // gets the cast shadow alone, softer and closer in than the one a lit
    // needle throws. A white blade has none either: there is nothing brighter
    // for it to flare to, and a halo would only fog the graduations it
    // crosses.
    if paper() {
        blade(3.4 * k, vec2(1.0, 1.5), false, &|_| Color32::from_black_alpha(38));
    } else {
        if m.needle_glow != Color32::TRANSPARENT {
            blade(7.0 * k, Vec2::ZERO, true, &|_| m.needle_glow.linear_multiply(0.22));
        }
        blade(3.6 * k, vec2(1.5, 2.0), false, &|_| Color32::from_black_alpha(150));
    }
    blade(3.2 * k, Vec2::ZERO, false, &ink);
}

fn show_needle(ui: &mut Ui, meters: Option<&Meters>, size: Vec2) -> Response {
    let tx = meters.is_some_and(|m| m.tx.is_some());
    let (rect, resp) = face(ui, size, tx);
    let p = ui.painter_at(rect);
    let k = (rect.height() / 72.0).clamp(0.55, 2.0);
    let r = reading(meters);
    let peak = peak_hold(ui, resp.id, r.frac);
    // The rail's lit length follows the needle rather than the raw reading, so
    // the two never disagree mid-swing.
    let frac = needle_motion(ui, resp.id, r.frac);

    // A wide, shallow arc: the pivot sits far below the short box, so the
    // needle sweeps a broad band across it and the scale's ends dip towards the
    // lower corners — leaving the top corners free for the readouts. The sweep
    // is what sets the radius: the chord has to span the box — up to
    // [`NEEDLE_FACE_MAX_W`], past which the scale stays at its widest and rides
    // centred in the face (the arc gets *taller* with width, and a wider one
    // would dip below the box; the readouts keep the corners either way).
    let half = 31.0_f32.to_radians();
    let chord = rect.width().min(NEEDLE_FACE_MAX_W);
    let rad = ((chord - 14.0) / (2.0 * half.sin())).max(24.0);
    let arc = Arc { pivot: pos2(rect.center().x, rect.top() + 13.0 * k + rad), rad, half };
    let band = 2.6 * k;
    let rail = (rad - band, rad + band);
    let col = |f: f32| ramp(&r.scale.stops, f);

    // Rail: the whole span dimmed, the travelled part lit, with a bloom under
    // the lit part so the instrument looks illuminated rather than printed.
    arc_band(&p, &arc, rail, 0.0, 1.0, |f| unlit(col(f), 0.18));
    if blooms() && !paper() {
        arc_glow(&p, &arc, band * 3.2, 0.0, frac, |f| col(f).linear_multiply(0.22));
    }
    arc_band(&p, &arc, rail, 0.0, frac, col);

    // Graduations, inside the rail.
    let tick_out = rad - band - 1.5 * k;
    for t in &r.scale.ticks {
        let a = arc.ang(t.f);
        let hot = usize::from(t.hot);
        let (len, w, ink) = if t.major {
            (6.0 * k, 1.4, TICK_MAJOR()[hot])
        } else {
            (3.5 * k, 1.0, TICK_MINOR()[hot])
        };
        p.line_segment([arc.at(tick_out - len, a), arc.at(tick_out, a)], Stroke::new(w, ink));
        if let Some(label) = &t.label {
            p.text(
                arc.at(rad - 17.5 * k, a),
                Align2::CENTER_CENTER,
                label,
                FontId::monospace(9.0 * k),
                LABEL()[hot],
            );
        }
    }

    if let Some(pk) = peak {
        let a = arc.ang(pk);
        p.line_segment(
            [arc.at(rail.0, a), arc.at(rad + band * 2.0, a)],
            Stroke::new(1.6, lift(col(pk), 0.6)),
        );
    }

    draw_needle(&p, arc.pivot, arc.ang(frac), rad - band * 1.6, k);

    // Readouts along the top edge, where the arc has dipped out of the way.
    header(&p, rect, &r, k);
    temperature(&p, rect, meters, k);
    puresignal(&p, rect, meters, k);
    border(ui, rect);
    resp
}

// ---------------------------------------------------------------------------
// Trace style
// ---------------------------------------------------------------------------

/// Samples the trace keeps and the rate it takes them at by default: 240 at
/// 16 Hz is a fifteen-second window, and roughly one sample per pixel at the
/// meter's size. The rate follows the operator's speed step
/// ([`TraceOpts::hz`]); the count stays, so a slower step is a longer window.
const TRACE_LEN: usize = 240;
const TRACE_HZ: f64 = 16.0;

/// The rolling plot behind the trace face.
#[derive(Clone)]
struct Trace {
    vals: Vec<f32>,
    /// What the samples measure; a change starts a fresh plot.
    kind: ScaleKind,
    /// Wall-clock time the next sample is due.
    next: f64,
    /// The rate the samples were taken at; a change starts a fresh plot, as
    /// samples of two rates on one time axis would lie about when.
    hz: f64,
}

/// Append to the rolling history and hand back the whole window.
fn trace_history(ui: &Ui, id: Id, frac: f32, kind: ScaleKind, hz: f64) -> Vec<f32> {
    let now = ui.input(|i| i.time);
    animate(ui); // the plot scrolls on the clock, with or without fresh readings
    let hz = hz.max(0.1);
    ui.data_mut(|d| {
        let fresh = || Trace { vals: vec![frac; TRACE_LEN], kind, next: now, hz };
        let t = d.get_temp_mut_or_insert_with(id, fresh);
        // Receive ↔ transmit changes what is being plotted; start over rather
        // than splice two different quantities into one line.
        if t.kind != kind || t.hz != hz {
            *t = fresh();
        }
        // Advance in whole steps so the scroll follows the clock rather than the
        // frame rate. A long stall slides the window along instead of squashing
        // it — and the step cap stops a huge jump spinning here.
        let mut steps = 0;
        while now >= t.next && steps < TRACE_LEN {
            t.vals.rotate_left(1);
            t.vals[TRACE_LEN - 1] = frac;
            t.next += 1.0 / hz;
            steps += 1;
        }
        if steps == TRACE_LEN {
            t.next = now + 1.0 / hz;
        }
        t.vals.clone()
    })
}

fn show_trace(ui: &mut Ui, meters: Option<&Meters>, size: Vec2, opts: TraceOpts) -> Response {
    let tx = meters.is_some_and(|m| m.tx.is_some());
    let (rect, resp) = face(ui, size, tx);
    let p = ui.painter_at(rect);
    let k = (rect.height() / 72.0).clamp(0.55, 2.0);
    let mut r = reading(meters);
    let raw = trace_history(ui, resp.id, r.frac, r.scale.kind, opts.hz);
    // Spread to the operator's S range on receive: the plot's height is the
    // range, levels outside it ride the bottom or top edge. Colours stay
    // those of the full scale, so S9 is S9's colour wherever it is drawn.
    let spread = r.scale.kind == ScaleKind::S && opts.spread();
    let (f0, f1) = if spread { opts.s_range() } else { (0.0, 1.0) };
    let z = |f: f32| ((f - f0) / (f1 - f0)).clamp(0.0, 1.0);
    let hist: Vec<(f32, f32)> = raw.iter().map(|&f| (z(f), f)).collect();
    let live = r.frac;
    r.frac = z(live);
    if spread {
        r.scale.grid = spread_grid(opts);
    }
    let col = |f: f32| ramp(&r.scale.stops, f);

    // A gutter on the right carries the level axis, so the newest sample sits
    // right up against the scale it is read on.
    let plot = Rect::from_min_max(
        pos2(rect.left() + 1.0, rect.top() + 1.0),
        pos2(rect.right() - 24.0 * k, rect.bottom() - 1.0),
    );
    let y_of = |f: f32| plot.bottom() - plot.height() * f.clamp(0.0, 1.0);
    // Where a point `age` (0 = now … 1 = the oldest sample) lies across the
    // plot: now at the right edge, or at the left when the trace runs the
    // other way.
    let x_of = |age: f32| {
        if opts.newest_left {
            plot.left() + plot.width() * age
        } else {
            plot.right() - plot.width() * age
        }
    };
    let now_x = x_of(0.0);
    // Where the header ends; nothing that would be drawn behind it is drawn.
    let head_bottom = header_h(&p, rect, k);

    // Time gridlines anchored to "now", two to six of them whatever the speed:
    // every five seconds over the stock quarter-minute, wider on slower steps.
    let span_s = (TRACE_LEN as f64 / opts.hz.max(0.1)) as f32;
    let every = [1.0f32, 2.0, 5.0, 10.0, 30.0, 60.0]
        .into_iter()
        .find(|e| span_s / e <= 6.0)
        .unwrap_or(120.0);
    let mut secs = every;
    while secs < span_s {
        let x = x_of(secs / span_s);
        p.vline(x, Rangef::new(plot.top(), plot.bottom()), Stroke::new(1.0, GRID_LINE()[0]));
        secs += every;
    }

    // Level gridlines, labelled out in the gutter.
    for g in &r.scale.grid {
        let y = y_of(g.f);
        if y < head_bottom {
            continue;
        }
        let hot = usize::from(g.hot);
        p.hline(Rangef::new(plot.left(), plot.right()), y, Stroke::new(1.0, GRID_LINE()[hot]));
        p.text(
            pos2(plot.right() + 3.0 * k, y),
            Align2::LEFT_CENTER,
            &g.label,
            FontId::monospace(8.5 * k),
            GRID_LABEL()[hot],
        );
    }

    // The trace itself: one column per sample, each carrying the colour of its
    // own level, so the plot reads as a band of signal rather than a silhouette.
    let mut fill = Mesh::default();
    // Meshes are not antialiased, so the fill's top edge would show a one-pixel
    // staircase. `feather` fades the last pixel of it out and smooths it away.
    let mut feather = Mesh::default();
    let mut ridge = Vec::with_capacity(TRACE_LEN);
    for (i, &(v, full)) in hist.iter().enumerate() {
        // `hist` runs oldest to newest; `v` is the height in the (spread)
        // plot, `full` the level on the whole scale, which picks the colour.
        let x = x_of(1.0 - i as f32 / (TRACE_LEN - 1) as f32);
        let y = y_of(v);
        let c = col(full);
        push_quad(
            &mut fill,
            pos2(x, y),
            pos2(x, plot.bottom()),
            c.linear_multiply(0.60),
            c.linear_multiply(0.05),
        );
        push_quad(
            &mut feather,
            pos2(x, y - 1.2),
            pos2(x, y),
            Color32::TRANSPARENT,
            c.linear_multiply(0.60),
        );
        ridge.push(pos2(x, y));
    }
    p.add(Shape::mesh(fill));
    p.add(Shape::mesh(feather));
    p.add(Shape::line(ridge, Stroke::new(1.5, READOUT().linear_multiply(0.75))));

    // "Now": a hairline across the plot at the live level and a lit cap at the
    // axis, so the current reading is findable without tracing the line.
    let y = y_of(r.frac);
    p.hline(
        Rangef::new(plot.left(), plot.right()),
        y,
        Stroke::new(1.0, col(live).linear_multiply(0.30)),
    );
    // The cap sits on the edge the newest sample is drawn at.
    let inward = if opts.newest_left { 1.0 } else { -1.0 };
    glow(&p, pos2(now_x + inward, y), 9.0 * k, col(live).linear_multiply(0.35));
    p.rect_filled(
        Rect::from_two_pos(pos2(now_x + inward * 4.0 * k, y - 1.5 * k), pos2(now_x, y + 1.5 * k)),
        0.0,
        lift(col(live), 0.4),
    );

    // The trace runs full-bleed, so the header needs a scrim under it.
    grad_v(
        &p,
        Rect::from_min_max(rect.left_top(), pos2(rect.right(), head_bottom)),
        if paper() { Color32::from_white_alpha(215) } else { Color32::from_black_alpha(180) },
        Color32::TRANSPARENT,
    );
    header(&p, rect, &r, k);
    temperature(&p, rect, meters, k);
    puresignal(&p, rect, meters, k);
    border(ui, rect);
    resp
}

#[cfg(test)]
mod tests {
    use super::SmeterStyle;

    /// Proves: the S/N chip names the mean and, after it, the marker's value;
    /// without a marker value only the mean.
    #[test]
    fn the_snr_chip_names_mean_and_marker() {
        use super::{SnrReading, snr_chip};
        let r = SnrReading { db: 26.5, mid_db: 31.04, width_hz: 48_000.0, frames: 4122 };
        assert_eq!(snr_chip(&r), "S/N 26.5 · 31.0 dB");
        let r = SnrReading { mid_db: f32::NAN, ..r };
        assert_eq!(snr_chip(&r), "S/N 26.5 dB");
    }

    #[test]
    fn the_compact_cycle_visits_every_face_it_can_show_and_no_others() {
        let mut seen = Vec::new();
        let mut s = SmeterStyle::default();
        for _ in 0..6 {
            s = s.next_compact();
            let shown = s.compact();
            assert_ne!(shown, SmeterStyle::Needle, "a compact box cannot draw the needle");
            if !seen.contains(&shown) {
                seen.push(shown);
            }
        }
        assert_eq!(seen.len(), 3, "the cycle should reach every compact face: {seen:?}");
    }

    /// The trace's speed steps keep the waterfall's proportions, and `Fast`
    /// is the quarter-minute the trace always showed.
    #[test]
    fn the_trace_runs_on_the_waterfall_steps() {
        use sdroxide_types::{Speed, UiSettings};
        let hz = |s: Speed| {
            UiSettings { smeter_trace_speed: s, ..UiSettings::default() }.smeter_trace_hz()
        };
        assert_eq!(UiSettings::default().smeter_trace_speed, Speed::Fast);
        assert!((hz(Speed::Fast) - super::TRACE_HZ).abs() < 1e-9);
        assert!((hz(Speed::Medium) - 8.0).abs() < 1e-9);
        assert!((hz(Speed::Fastest) - 64.0).abs() < 1e-9);
        // 240 samples: Slow holds 168 s.
        assert!((super::TRACE_LEN as f64 / hz(Speed::Slow) - 168.0).abs() < 1e-6);
        let mut prev = 0.0;
        for s in Speed::WATERFALL {
            assert!(hz(s) > prev, "{s:?} is not faster than the step before");
            prev = hz(s);
        }
    }

    /// The range rail's stops: S3–S9 spreads −109…−73 dBm over the plot,
    /// its gridlines sit inside it with S-unit labels, and the full rail is
    /// no spread at all.
    #[test]
    fn the_trace_spreads_to_its_s_range() {
        use super::TraceOpts;
        assert_eq!(super::stop_dbm(3), -109.0);
        assert_eq!(super::stop_dbm(9), -73.0);
        assert_eq!(super::stop_dbm(11), -53.0);
        assert_eq!(super::stop_label(11), "+20");
        assert!(!TraceOpts::default().spread());
        let o = TraceOpts { lo: 3, hi: 9, ..TraceOpts::default() };
        assert!(o.spread());
        let (f0, f1) = o.s_range();
        assert!(
            (f0 - super::frac_of(-109.0)).abs() < 1e-6 && (f1 - super::frac_of(-73.0)).abs() < 1e-6
        );
        let g = super::spread_grid(o);
        assert!(!g.is_empty() && g.len() <= 5, "{} lines", g.len());
        assert!(g.iter().all(|l| l.f > 0.0 && l.f < 1.0));
        assert!(g.iter().any(|l| l.label == "6"), "S6 is labelled");
        // S6–S9: S7 and S8 inside.
        let g = super::spread_grid(TraceOpts { lo: 6, hi: 9, ..TraceOpts::default() });
        assert_eq!(g.iter().map(|l| l.label.as_str()).collect::<Vec<_>>(), ["8", "7"]);
    }

    /// Without S/N the cycle passes the S/N face by, on desktop and phone.
    #[test]
    fn the_snr_face_is_skipped_without_snr() {
        let mut s = SmeterStyle::default();
        for _ in 0..8 {
            s = s.next_with(false);
            assert_ne!(s, SmeterStyle::Snr);
        }
        for _ in 0..8 {
            s = s.next_compact_with(false);
            assert_ne!(s, SmeterStyle::Snr);
        }
        assert_eq!(SmeterStyle::Trace.next_with(true), SmeterStyle::Snr);
        assert_eq!(SmeterStyle::Trace.next_with(false), SmeterStyle::Needle);
    }

    /// The passband reading: noise reads 0 dB; one 20 dB cell among nine
    /// measured lifts the mean to 10·log10((100 + 8)/9) and, at the middle,
    /// is the middle value; cells outside the filter and unmeasured ones do
    /// not count. A stronger cell at the filter's edge raises the mean but
    /// not the middle value.
    #[test]
    fn the_passband_snr_is_the_mean_power_over_the_noise() {
        // 100 Hz cells from 7.000 MHz; the filter covers cells 10..20.
        let mut cells = vec![0.0f32; 40];
        cells[15] = 20.0;
        cells[30] = 35.0; // outside the passband
        cells[11] = f32::NAN; // not yet measured
        let (db, peak) =
            super::passband_snr(7_000_000.0, 100.0, &cells, 7_001_000.0, 7_002_000.0).unwrap();
        let want = (10.0 * ((100.0f64 + 8.0) / 9.0).log10()) as f32;
        assert!((db - want).abs() < 1e-3, "{db} vs {want}");
        assert_eq!(peak, 20.0);
        // The same filter with the 20 dB cell moved to its edge (cell 10)
        // and 6 dB in the middle (cell 15): the middle reads 6.
        let mut edge = vec![0.0f32; 40];
        edge[10] = 20.0;
        edge[15] = 6.0;
        let (_, mid) =
            super::passband_snr(7_000_000.0, 100.0, &edge, 7_001_000.0, 7_002_000.0).unwrap();
        assert_eq!(mid, 6.0);
        let flat = vec![0.0f32; 40];
        let (db0, _) =
            super::passband_snr(7_000_000.0, 100.0, &flat, 7_001_000.0, 7_002_000.0).unwrap();
        assert!(db0.abs() < 1e-6);
        assert!(
            super::passband_snr(7_000_000.0, 100.0, &cells, 8_000_000.0, 8_001_000.0).is_none()
        );
    }

    #[test]
    fn the_full_cycle_still_visits_all_four() {
        let mut seen = Vec::new();
        let mut s = SmeterStyle::default();
        for _ in 0..4 {
            if !seen.contains(&s) {
                seen.push(s);
            }
            s = s.next();
        }
        assert_eq!(seen.len(), 4, "the desktop cycle lost a face: {seen:?}");
        assert_eq!(s, SmeterStyle::default(), "the cycle should come back round");
    }
}
