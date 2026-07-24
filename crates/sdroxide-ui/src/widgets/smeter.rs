//! S-meter with two selectable styles: a horizontal bar and an analog needle
//! instrument. Both are clickable (the caller toggles the style on a click) and
//! switch to a TX meter (power / SWR / ALC) while transmitting.
//!
//! The meter fills its whole box edge-to-edge: it paints its own dark face over
//! the full allocated rect and draws the box border on top, so there is no
//! padding between the instrument and the frame.

use eframe::egui::{
    Align2, Color32, FontId, Rangef, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2,
    vec2,
};
use sdroxide_types::Meters;

/// Scale endpoints: S0 = -127 dBm … S9+60 = -13 dBm.
const DBM_LO: f32 = -127.0;
const DBM_HI: f32 = -13.0;
const S9_DBM: f32 = -73.0;

const GREEN: Color32 = Color32::from_rgb(70, 200, 90);
const RED: Color32 = Color32::from_rgb(230, 70, 60);
const AMBER: Color32 = Color32::from_rgb(255, 209, 66);
/// Dark instrument face, painted over the whole box.
const FACE: Color32 = Color32::from_gray(16);

/// Position on the scale for `dbm`, 0.0 (S0) … 1.0 (S9+60).
fn frac_of(dbm: f32) -> f32 {
    ((dbm - DBM_LO) / (DBM_HI - DBM_LO)).clamp(0.0, 1.0)
}

/// Draw the S-meter in the selected style, filling the box's full interior.
/// Returns the (clickable) response so the caller can toggle the style.
pub fn show(ui: &mut Ui, meters: Option<&Meters>, analog: bool) -> Response {
    // Fill whatever the (zero-padding) box hands us.
    let size = ui.available_size();
    if analog {
        show_analog(ui, meters, size)
    } else {
        show_bar(ui, meters, size)
    }
}

/// The RX S-unit reading (amber) and the dBm sub-reading (grey).
fn rx_readout(meters: Option<&Meters>, dbm: f32) -> (String, String) {
    if meters.is_none() {
        return ("—".to_string(), String::new());
    }
    let primary = if dbm > S9_DBM {
        format!("S9+{:.0}", dbm - S9_DBM)
    } else {
        let (s, _) = Meters { s_dbm: dbm, adc_peak_dbfs: 0.0, tx: None }.s_units();
        format!("S{s}")
    };
    (primary, format!("{dbm:.0} dBm"))
}

/// Power / SWR / ALC label for the TX meter.
fn tx_label(tx: &sdroxide_types::TxMeters) -> String {
    match (tx.fwd_w, tx.swr) {
        (Some(w), Some(s)) => format!("{w:.1} W {s:.1}:1"),
        (Some(w), None) => format!("{w:.1} W"),
        // SWR without a power sensor (CAT/TCI rigs that only report SWR).
        (None, Some(s)) => format!("SWR {s:.1}:1"),
        (None, None) => format!("ALC {:.0}%", tx.alc.clamp(0.0, 1.0) * 100.0),
    }
}

/// Paint the instrument face over the full box and return the rect + response.
fn face(ui: &mut Ui, size: Vec2) -> (Rect, Response) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    ui.painter_at(rect).rect_filled(rect, 0.0, FACE);
    (rect, resp)
}

/// Draw the box border on top of the finished meter (the meter face already
/// covers the whole rect, so the frame's own border would be hidden).
fn border(ui: &Ui, rect: Rect) {
    ui.painter_at(rect).rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, crate::theme::LINE_LIT),
        StrokeKind::Inside,
    );
}

// ---------------------------------------------------------------------------
// Bar style
// ---------------------------------------------------------------------------

fn show_bar(ui: &mut Ui, meters: Option<&Meters>, size: Vec2) -> Response {
    let (rect, resp) = face(ui, size);
    let p = ui.painter_at(rect);

    const INSET: f32 = 5.0; // top/left breathing room for the bar
    const READOUT_W: f32 = 78.0; // right column reserved for the digital readout
    const LABEL_BAND: f32 = 16.0; // bottom strip for the S-scale ticks + numbers

    // Transmitting: show the TX meter instead of the S-meter.
    if let Some(tx) = meters.and_then(|m| m.tx.as_ref()) {
        let bar_rect = Rect::from_min_max(
            pos2(rect.left() + 34.0, rect.top() + INSET),
            pos2(rect.right() - READOUT_W, rect.bottom() - INSET),
        );
        p.text(
            pos2(rect.left() + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            "TX",
            FontId::monospace(18.0),
            RED,
        );
        let frac = tx.alc.clamp(0.0, 1.0);
        p.rect_filled(bar_rect, 2.0, Color32::from_gray(30));
        p.rect_filled(
            Rect::from_min_size(bar_rect.min, vec2(bar_rect.width() * frac, bar_rect.height())),
            2.0,
            if frac > 0.95 {
                Color32::from_rgb(230, 70, 60)
            } else {
                Color32::from_rgb(230, 150, 50)
            },
        );
        p.text(
            pos2(rect.right() - 5.0, rect.center().y),
            Align2::RIGHT_CENTER,
            tx_label(tx),
            FontId::monospace(13.0),
            Color32::from_gray(200),
        );
        border(ui, rect);
        return resp;
    }

    let bar_rect = Rect::from_min_max(
        pos2(rect.left() + INSET, rect.top() + INSET),
        pos2(rect.right() - READOUT_W, rect.bottom() - LABEL_BAND),
    );

    let dbm = meters.map(|m| m.s_dbm).unwrap_or(DBM_LO);
    let frac = frac_of(dbm);
    let s9_frac = frac_of(S9_DBM);

    // Trough behind the bar for definition.
    p.rect_filled(bar_rect, 0.0, Color32::from_gray(28));

    // Green segment up to S9, red segment beyond.
    let green_w = bar_rect.width() * frac.min(s9_frac);
    if green_w > 0.0 {
        p.rect_filled(
            Rect::from_min_size(bar_rect.min, vec2(green_w, bar_rect.height())),
            0.0,
            GREEN,
        );
    }
    if frac > s9_frac {
        let x0 = bar_rect.left() + bar_rect.width() * s9_frac;
        let w = bar_rect.width() * (frac - s9_frac);
        p.rect_filled(
            Rect::from_min_size(pos2(x0, bar_rect.top()), vec2(w, bar_rect.height())),
            0.0,
            RED,
        );
    }

    // Ticks: S1..S9 then +20/+40/+60, just below the bar.
    let tick_top = bar_rect.bottom() + 1.0;
    let tick_bot = bar_rect.bottom() + 5.0;
    let num_y = rect.bottom() - 1.0;
    for i in 1..=9 {
        let d = S9_DBM - (9 - i) as f32 * 6.0;
        let x = bar_rect.left() + bar_rect.width() * frac_of(d);
        p.vline(x, Rangef::new(tick_top, tick_bot), (1.0, Color32::from_gray(120)));
        if i == 9 || i % 2 == 1 {
            p.text(
                pos2(x, num_y),
                Align2::CENTER_BOTTOM,
                format!("{i}"),
                FontId::monospace(9.0),
                Color32::from_gray(140),
            );
        }
    }
    for over in [20.0f32, 40.0, 60.0] {
        let x = bar_rect.left() + bar_rect.width() * frac_of(S9_DBM + over);
        p.vline(x, Rangef::new(tick_top, tick_bot), (1.0, Color32::from_rgb(200, 90, 80)));
        p.text(
            pos2(x, num_y),
            Align2::CENTER_BOTTOM,
            format!("+{over:.0}"),
            FontId::monospace(9.0),
            Color32::from_rgb(200, 90, 80),
        );
    }

    // Readout: "S9+15" / "-58 dBm".
    let (primary, secondary) = rx_readout(meters, dbm);
    p.text(
        pos2(rect.right() - 5.0, rect.top() + 4.0),
        Align2::RIGHT_TOP,
        primary,
        FontId::monospace(20.0),
        AMBER,
    );
    if !secondary.is_empty() {
        p.text(
            pos2(rect.right() - 5.0, rect.bottom() - 3.0),
            Align2::RIGHT_BOTTOM,
            secondary,
            FontId::monospace(11.0),
            Color32::from_gray(150),
        );
    }
    border(ui, rect);
    resp
}

// ---------------------------------------------------------------------------
// Analog needle style
// ---------------------------------------------------------------------------

fn show_analog(ui: &mut Ui, meters: Option<&Meters>, size: Vec2) -> Response {
    let (rect, resp) = face(ui, size);
    let p = ui.painter_at(rect);

    // Wide, shallow arc: the pivot sits well below the short box so the needle
    // sweeps a broad arc across the top (classic wide-format meter). The radius
    // tracks the box height so the meter scales to fill it.
    let cx = rect.center().x;
    let r = rect.height() * 2.2;
    let pivot = pos2(cx, rect.top() + r + 8.0);
    let half = 52.0_f32.to_radians();
    let ang = |f: f32| (f - 0.5) * 2.0 * half;
    let pt = |rad: f32, a: f32| pivot + vec2(rad * a.sin(), -rad * a.cos());

    let tx = meters.and_then(|m| m.tx.as_ref());
    let redline = if tx.is_some() { 0.85 } else { frac_of(S9_DBM) };

    // Scale arc, green below the red-line, red above.
    let segs = 64;
    for i in 0..segs {
        let f0 = i as f32 / segs as f32;
        let f1 = (i + 1) as f32 / segs as f32;
        let col = if f0 >= redline { RED } else { GREEN };
        p.line_segment([pt(r, ang(f0)), pt(r, ang(f1))], Stroke::new(2.2, col));
    }

    // Value fraction + readouts.
    let (vfrac, primary, secondary) = if let Some(tx) = tx {
        (tx.alc.clamp(0.0, 1.0), tx_label(tx), String::new())
    } else {
        let dbm = meters.map(|m| m.s_dbm).unwrap_or(DBM_LO);
        let (a, b) = rx_readout(meters, dbm);
        (frac_of(dbm), a, b)
    };

    if tx.is_none() {
        // S1..S9 ticks + odd-value numbers.
        for i in 1..=9 {
            let d = S9_DBM - (9 - i) as f32 * 6.0;
            let a = ang(frac_of(d));
            let major = i % 2 == 1;
            let len = if major { 7.0 } else { 4.0 };
            p.line_segment([pt(r - len, a), pt(r, a)], Stroke::new(1.2, Color32::from_gray(170)));
            if major {
                p.text(
                    pt(r - 15.0, a),
                    Align2::CENTER_CENTER,
                    format!("{i}"),
                    FontId::monospace(9.0),
                    Color32::from_gray(185),
                );
            }
        }
        // Over-S9 ticks + red labels.
        for over in [20.0f32, 40.0, 60.0] {
            let a = ang(frac_of(S9_DBM + over));
            p.line_segment([pt(r - 7.0, a), pt(r, a)], Stroke::new(1.2, RED));
            p.text(
                pt(r - 15.0, a),
                Align2::CENTER_CENTER,
                format!("{over:.0}"),
                FontId::monospace(9.0),
                RED,
            );
        }
        p.text(
            pt(r + 4.0, ang(1.0)),
            Align2::LEFT_CENTER,
            "dB",
            FontId::monospace(9.0),
            RED,
        );
    } else {
        // "TX" indicator sits in the (now clear) bottom-left corner so it stays
        // out of the readout row moved up top.
        p.text(
            pos2(rect.left() + 5.0, rect.bottom() - 3.0),
            Align2::LEFT_BOTTOM,
            "TX",
            FontId::monospace(12.0),
            RED,
        );
    }

    // Needle (clipped to the box, so it enters from the bottom).
    let a = ang(vfrac.clamp(0.0, 1.0));
    p.line_segment([pivot, pt(r * 0.98, a)], Stroke::new(2.6, Color32::from_rgb(235, 60, 50)));

    // Digital readouts along the top edge: the arc dips low toward the sides,
    // so the top corners stay clear of the needle and scale — keeping the S and
    // dBm labels readable.
    p.text(
        pos2(rect.left() + 5.0, rect.top() + 4.0),
        Align2::LEFT_TOP,
        primary,
        FontId::monospace(16.0),
        AMBER,
    );
    if !secondary.is_empty() {
        p.text(
            pos2(rect.right() - 5.0, rect.top() + 4.0),
            Align2::RIGHT_TOP,
            secondary,
            FontId::monospace(11.0),
            Color32::from_gray(150),
        );
    }
    border(ui, rect);
    resp
}
