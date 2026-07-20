//! S-meter: green bar to S9, red beyond, with S-unit ticks and a dBm readout.

use eframe::egui::{Align2, Color32, FontId, Rect, Sense, Ui, pos2, vec2};
use sdroxide_types::Meters;

/// Scale endpoints: S0 = -127 dBm … S9+60 = -13 dBm.
const DBM_LO: f32 = -127.0;
const DBM_HI: f32 = -13.0;
const S9_DBM: f32 = -73.0;

pub fn show(ui: &mut Ui, meters: Option<&Meters>) {
    let (rect, _) = ui.allocate_exact_size(vec2(240.0, 24.0), Sense::hover());
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
            Color32::from_rgb(240, 60, 50),
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
        let label = match (tx.fwd_w, tx.swr) {
            (Some(w), Some(s)) => format!("{w:.1} W {s:.1}:1"),
            (Some(w), None) => format!("{w:.1} W"),
            _ => format!("ALC {:.0}%", frac * 100.0),
        };
        p.text(
            pos2(rect.right() - 4.0, rect.center().y),
            Align2::RIGHT_CENTER,
            label,
            FontId::monospace(10.0),
            Color32::from_gray(200),
        );
        return;
    }

    let bar_rect = Rect::from_min_max(
        rect.min + vec2(3.0, 3.0),
        pos2(rect.right() - 64.0, rect.top() + 13.0),
    );
    let frac_of = |dbm: f32| ((dbm - DBM_LO) / (DBM_HI - DBM_LO)).clamp(0.0, 1.0);

    let dbm = meters.map(|m| m.s_dbm).unwrap_or(DBM_LO);
    let frac = frac_of(dbm);
    let s9_frac = frac_of(S9_DBM);

    // Green segment up to S9, red segment beyond.
    let green_w = bar_rect.width() * frac.min(s9_frac);
    if green_w > 0.0 {
        p.rect_filled(
            Rect::from_min_size(bar_rect.min, vec2(green_w, bar_rect.height())),
            0.0,
            Color32::from_rgb(70, 200, 90),
        );
    }
    if frac > s9_frac {
        let x0 = bar_rect.left() + bar_rect.width() * s9_frac;
        let w = bar_rect.width() * (frac - s9_frac);
        p.rect_filled(
            Rect::from_min_size(pos2(x0, bar_rect.top()), vec2(w, bar_rect.height())),
            0.0,
            Color32::from_rgb(230, 70, 60),
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
    let label = if meters.is_none() {
        "—".to_string()
    } else if dbm > S9_DBM {
        format!("S9+{:.0}", dbm - S9_DBM)
    } else {
        let (s, _) = Meters { s_dbm: dbm, adc_peak_dbfs: 0.0, tx: None }.s_units();
        format!("S{s}")
    };
    p.text(
        pos2(rect.right() - 4.0, rect.top() + 3.0),
        Align2::RIGHT_TOP,
        label,
        FontId::monospace(11.0),
        Color32::from_rgb(255, 209, 66),
    );
    if meters.is_some() {
        p.text(
            pos2(rect.right() - 4.0, rect.bottom() - 2.0),
            Align2::RIGHT_BOTTOM,
            format!("{dbm:.0} dBm"),
            FontId::monospace(9.0),
            Color32::from_gray(150),
        );
    }
}
