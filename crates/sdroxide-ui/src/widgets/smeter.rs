//! S-meter with two selectable styles: a horizontal bar and an analog needle
//! instrument. Both are clickable (the caller toggles the style on a click) and
//! switch to a TX meter (power / SWR / ALC) while transmitting.

use eframe::egui::{Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, pos2, vec2};
use sdroxide_types::Meters;

/// Scale endpoints: S0 = -127 dBm … S9+60 = -13 dBm.
const DBM_LO: f32 = -127.0;
const DBM_HI: f32 = -13.0;
const S9_DBM: f32 = -73.0;

const GREEN: Color32 = Color32::from_rgb(70, 200, 90);
const RED: Color32 = Color32::from_rgb(230, 70, 60);
const AMBER: Color32 = Color32::from_rgb(255, 209, 66);

/// Position on the scale for `dbm`, 0.0 (S0) … 1.0 (S9+60).
fn frac_of(dbm: f32) -> f32 {
    ((dbm - DBM_LO) / (DBM_HI - DBM_LO)).clamp(0.0, 1.0)
}

/// Draw the S-meter in the selected style. Returns the (clickable) response so
/// the caller can toggle the style.
pub fn show(ui: &mut Ui, meters: Option<&Meters>, analog: bool) -> Response {
    if analog {
        show_analog(ui, meters)
    } else {
        show_bar(ui, meters)
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
        _ => format!("ALC {:.0}%", tx.alc.clamp(0.0, 1.0) * 100.0),
    }
}

// ---------------------------------------------------------------------------
// Bar style
// ---------------------------------------------------------------------------

fn show_bar(ui: &mut Ui, meters: Option<&Meters>) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(240.0, 24.0), Sense::click());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0, Color32::from_gray(16));

    // Transmitting: show the TX meter instead of the S-meter.
    if let Some(tx) = meters.and_then(|m| m.tx.as_ref()) {
        let bar_rect = Rect::from_min_max(
            rect.min + vec2(30.0, 4.0),
            pos2(rect.right() - 64.0, rect.bottom() - 4.0),
        );
        p.text(
            pos2(rect.left() + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            "TX",
            FontId::monospace(12.0),
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
            pos2(rect.right() - 4.0, rect.center().y),
            Align2::RIGHT_CENTER,
            tx_label(tx),
            FontId::monospace(10.0),
            Color32::from_gray(200),
        );
        return resp;
    }

    let bar_rect = Rect::from_min_max(
        rect.min + vec2(3.0, 3.0),
        pos2(rect.right() - 64.0, rect.top() + 13.0),
    );

    let dbm = meters.map(|m| m.s_dbm).unwrap_or(DBM_LO);
    let frac = frac_of(dbm);
    let s9_frac = frac_of(S9_DBM);

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

    // Ticks: S1..S9 then +20/+40/+60.
    for i in 1..=9 {
        let d = S9_DBM - (9 - i) as f32 * 6.0;
        let x = bar_rect.left() + bar_rect.width() * frac_of(d);
        p.vline(
            x,
            eframe::egui::Rangef::new(bar_rect.bottom() + 1.0, bar_rect.bottom() + 4.0),
            (1.0, Color32::from_gray(120)),
        );
        if i == 9 || i % 2 == 1 {
            p.text(
                pos2(x, rect.bottom() - 1.0),
                Align2::CENTER_BOTTOM,
                format!("{i}"),
                FontId::monospace(7.0),
                Color32::from_gray(130),
            );
        }
    }
    for over in [20.0f32, 40.0, 60.0] {
        let x = bar_rect.left() + bar_rect.width() * frac_of(S9_DBM + over);
        p.vline(
            x,
            eframe::egui::Rangef::new(bar_rect.bottom() + 1.0, bar_rect.bottom() + 4.0),
            (1.0, Color32::from_rgb(200, 90, 80)),
        );
        p.text(
            pos2(x, rect.bottom() - 1.0),
            Align2::CENTER_BOTTOM,
            format!("+{over:.0}"),
            FontId::monospace(7.0),
            Color32::from_rgb(200, 90, 80),
        );
    }

    // Readout: "S9+15" / "-58 dBm".
    let (primary, secondary) = rx_readout(meters, dbm);
    p.text(
        pos2(rect.right() - 4.0, rect.top() + 3.0),
        Align2::RIGHT_TOP,
        primary,
        FontId::monospace(11.0),
        AMBER,
    );
    if !secondary.is_empty() {
        p.text(
            pos2(rect.right() - 4.0, rect.bottom() - 2.0),
            Align2::RIGHT_BOTTOM,
            secondary,
            FontId::monospace(9.0),
            Color32::from_gray(150),
        );
    }
    resp
}

// ---------------------------------------------------------------------------
// Analog needle style
// ---------------------------------------------------------------------------

fn show_analog(ui: &mut Ui, meters: Option<&Meters>) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(248.0, 46.0), Sense::click());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0, Color32::from_gray(16));

    // Wide, shallow arc: the pivot sits well below the short box so the needle
    // sweeps a broad arc across the top (classic wide-format meter).
    let cx = rect.center().x;
    let r = rect.height() * 2.2;
    let pivot = pos2(cx, rect.top() + r + 6.0);
    let half = 52.0_f32.to_radians();
    let ang = |f: f32| (f - 0.5) * 2.0 * half;
    let pt = |rad: f32, a: f32| pivot + vec2(rad * a.sin(), -rad * a.cos());

    let tx = meters.and_then(|m| m.tx.as_ref());
    let redline = if tx.is_some() { 0.85 } else { frac_of(S9_DBM) };

    // Scale arc, green below the red-line, red above.
    let segs = 48;
    for i in 0..segs {
        let f0 = i as f32 / segs as f32;
        let f1 = (i + 1) as f32 / segs as f32;
        let col = if f0 >= redline { RED } else { GREEN };
        p.line_segment([pt(r, ang(f0)), pt(r, ang(f1))], Stroke::new(1.6, col));
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
            let len = if major { 5.0 } else { 3.0 };
            p.line_segment([pt(r - len, a), pt(r, a)], Stroke::new(1.0, Color32::from_gray(160)));
            if major {
                p.text(
                    pt(r - 12.0, a),
                    Align2::CENTER_CENTER,
                    format!("{i}"),
                    FontId::monospace(7.0),
                    Color32::from_gray(175),
                );
            }
        }
        // Over-S9 ticks + red labels.
        for over in [20.0f32, 40.0, 60.0] {
            let a = ang(frac_of(S9_DBM + over));
            p.line_segment([pt(r - 5.0, a), pt(r, a)], Stroke::new(1.0, RED));
            p.text(
                pt(r - 12.0, a),
                Align2::CENTER_CENTER,
                format!("{over:.0}"),
                FontId::monospace(7.0),
                RED,
            );
        }
        p.text(
            pt(r + 3.0, ang(1.0)),
            Align2::LEFT_CENTER,
            "dB",
            FontId::monospace(8.0),
            RED,
        );
    } else {
        p.text(
            pos2(rect.left() + 4.0, rect.top() + 3.0),
            Align2::LEFT_TOP,
            "TX",
            FontId::monospace(10.0),
            RED,
        );
    }

    // Needle (clipped to the box, so it enters from the bottom).
    let a = ang(vfrac.clamp(0.0, 1.0));
    p.line_segment([pivot, pt(r * 0.98, a)], Stroke::new(2.0, Color32::from_rgb(235, 60, 50)));

    // Digital readouts in the corners.
    p.text(
        pos2(rect.left() + 4.0, rect.bottom() - 2.0),
        Align2::LEFT_BOTTOM,
        primary,
        FontId::monospace(10.0),
        AMBER,
    );
    if !secondary.is_empty() {
        p.text(
            pos2(rect.right() - 4.0, rect.bottom() - 2.0),
            Align2::RIGHT_BOTTOM,
            secondary,
            FontId::monospace(9.0),
            Color32::from_gray(150),
        );
    }
    resp
}
