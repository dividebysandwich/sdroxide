//! A colour-coded bandplan overlay drawn along the bottom of the waterfall.
//!
//! Two zoom tiers: when zoomed out, coarse allocations (ham bands, broadcast
//! bands, CB, AM); when zoomed into a ham band, the fine sub-segments (CW,
//! digital, SSB, beacons). Labels are drawn only where the segment is wide
//! enough on screen. HF ham sub-bands follow the IARU Region 1 plan.

use eframe::egui::{Color32, FontId, Painter, Rect, Stroke, pos2};

use crate::theme;
use crate::view::ViewState;

/// Usage class → colour.
#[derive(Clone, Copy)]
enum Kind {
    Cw,
    Digi,
    Phone,
    Beacon,
    Broadcast,
    Ham,
    Cb,
    Am,
}

impl Kind {
    fn color(self) -> Color32 {
        match self {
            Kind::Cw => Color32::from_rgb(0xE6, 0xB0, 0x3C),
            Kind::Digi => Color32::from_rgb(0x2E, 0xC4, 0xE6),
            Kind::Phone => Color32::from_rgb(0x4C, 0xC9, 0x6A),
            Kind::Beacon => Color32::from_rgb(0xE0, 0x5A, 0xA0),
            Kind::Broadcast => Color32::from_rgb(0xE8, 0x82, 0x2E),
            Kind::Ham => Color32::from_rgb(0x2C, 0x9E, 0x8C),
            Kind::Cb => Color32::from_rgb(0x9A, 0x6C, 0xE0),
            Kind::Am => Color32::from_rgb(0xC9, 0x6A, 0x3C),
        }
    }
}

struct Seg {
    lo: f64,
    hi: f64,
    label: &'static str,
    kind: Kind,
}

const fn s(lo: f64, hi: f64, label: &'static str, kind: Kind) -> Seg {
    Seg { lo, hi, label, kind }
}

// Above this view span, show coarse bands; below it, fine ham sub-segments.
const FINE_MAX_SPAN: f64 = 800_000.0;

const M: f64 = 1_000_000.0;

/// Coarse allocations shown when zoomed out.
static COARSE: &[Seg] = &[
    s(0.153 * M, 0.279 * M, "LW AM", Kind::Am),
    s(0.531 * M, 1.602 * M, "MW AM", Kind::Am),
    s(1.810 * M, 2.000 * M, "160m HAM", Kind::Ham),
    s(3.500 * M, 3.800 * M, "80m HAM", Kind::Ham),
    s(3.900 * M, 4.000 * M, "75m BC", Kind::Broadcast),
    s(4.750 * M, 5.060 * M, "60m BC", Kind::Broadcast),
    s(5.351 * M, 5.366 * M, "60m HAM", Kind::Ham),
    s(5.900 * M, 6.200 * M, "49m BC", Kind::Broadcast),
    s(7.000 * M, 7.200 * M, "40m HAM", Kind::Ham),
    s(7.200 * M, 7.450 * M, "41m BC", Kind::Broadcast),
    s(9.400 * M, 9.900 * M, "31m BC", Kind::Broadcast),
    s(10.100 * M, 10.150 * M, "30m HAM", Kind::Ham),
    s(11.600 * M, 12.100 * M, "25m BC", Kind::Broadcast),
    s(13.570 * M, 13.870 * M, "22m BC", Kind::Broadcast),
    s(14.000 * M, 14.350 * M, "20m HAM", Kind::Ham),
    s(15.100 * M, 15.830 * M, "19m BC", Kind::Broadcast),
    s(17.480 * M, 17.900 * M, "16m BC", Kind::Broadcast),
    s(18.068 * M, 18.168 * M, "17m HAM", Kind::Ham),
    s(21.000 * M, 21.450 * M, "15m HAM", Kind::Ham),
    s(21.450 * M, 21.850 * M, "13m BC", Kind::Broadcast),
    s(24.890 * M, 24.990 * M, "12m HAM", Kind::Ham),
    s(25.670 * M, 26.100 * M, "11m BC", Kind::Broadcast),
    s(26.965 * M, 27.405 * M, "CB", Kind::Cb),
    s(28.000 * M, 29.700 * M, "10m HAM", Kind::Ham),
];

/// Fine segments shown when zoomed into a band. Broadcast/CB/AM keep their
/// coarse blocks (no sub-structure); ham bands split into usage segments.
static FINE: &[Seg] = &[
    // Broadcast + others (unchanged from coarse).
    s(0.153 * M, 0.279 * M, "LW AM", Kind::Am),
    s(0.531 * M, 1.602 * M, "MW AM", Kind::Am),
    s(3.900 * M, 4.000 * M, "75m BC", Kind::Broadcast),
    s(4.750 * M, 5.060 * M, "60m BC", Kind::Broadcast),
    s(5.900 * M, 6.200 * M, "49m BC", Kind::Broadcast),
    s(7.200 * M, 7.450 * M, "41m BC", Kind::Broadcast),
    s(9.400 * M, 9.900 * M, "31m BC", Kind::Broadcast),
    s(11.600 * M, 12.100 * M, "25m BC", Kind::Broadcast),
    s(13.570 * M, 13.870 * M, "22m BC", Kind::Broadcast),
    s(15.100 * M, 15.830 * M, "19m BC", Kind::Broadcast),
    s(17.480 * M, 17.900 * M, "16m BC", Kind::Broadcast),
    s(21.450 * M, 21.850 * M, "13m BC", Kind::Broadcast),
    s(25.670 * M, 26.100 * M, "11m BC", Kind::Broadcast),
    s(26.965 * M, 27.405 * M, "CB", Kind::Cb),
    // 160m
    s(1.810 * M, 1.838 * M, "CW", Kind::Cw),
    s(1.838 * M, 1.843 * M, "Digi", Kind::Digi),
    s(1.843 * M, 2.000 * M, "SSB", Kind::Phone),
    // 80m
    s(3.500 * M, 3.570 * M, "CW", Kind::Cw),
    s(3.570 * M, 3.600 * M, "Digi", Kind::Digi),
    s(3.600 * M, 3.800 * M, "SSB", Kind::Phone),
    // 60m
    s(5.351 * M, 5.366 * M, "60m", Kind::Ham),
    // 40m
    s(7.000 * M, 7.040 * M, "CW", Kind::Cw),
    s(7.040 * M, 7.100 * M, "Digi", Kind::Digi),
    s(7.100 * M, 7.200 * M, "SSB", Kind::Phone),
    // 30m
    s(10.100 * M, 10.130 * M, "CW", Kind::Cw),
    s(10.130 * M, 10.150 * M, "Digi", Kind::Digi),
    // 20m
    s(14.000 * M, 14.070 * M, "CW", Kind::Cw),
    s(14.070 * M, 14.099 * M, "Digi (FT8)", Kind::Digi),
    s(14.099 * M, 14.101 * M, "Bcn", Kind::Beacon),
    s(14.101 * M, 14.350 * M, "SSB", Kind::Phone),
    // 17m
    s(18.068 * M, 18.095 * M, "CW", Kind::Cw),
    s(18.095 * M, 18.109 * M, "Digi", Kind::Digi),
    s(18.109 * M, 18.111 * M, "Bcn", Kind::Beacon),
    s(18.111 * M, 18.168 * M, "SSB", Kind::Phone),
    // 15m
    s(21.000 * M, 21.070 * M, "CW", Kind::Cw),
    s(21.070 * M, 21.150 * M, "Digi", Kind::Digi),
    s(21.150 * M, 21.450 * M, "SSB", Kind::Phone),
    // 12m
    s(24.890 * M, 24.915 * M, "CW", Kind::Cw),
    s(24.915 * M, 24.930 * M, "Digi", Kind::Digi),
    s(24.930 * M, 24.990 * M, "SSB", Kind::Phone),
    // 10m
    s(28.000 * M, 28.070 * M, "CW", Kind::Cw),
    s(28.070 * M, 28.190 * M, "Digi", Kind::Digi),
    s(28.190 * M, 28.300 * M, "Bcn", Kind::Beacon),
    s(28.300 * M, 29.700 * M, "SSB", Kind::Phone),
];

/// Draw the bandplan strip over the bottom of the waterfall rect.
pub fn overlay(p: &Painter, view: &ViewState, wf: &Rect) {
    let span = view.span();
    if span <= 0.0 || wf.height() < 24.0 {
        return;
    }
    let segs: &[Seg] = if span <= FINE_MAX_SPAN { FINE } else { COARSE };

    let strip_h = 20.0f32;
    let top = wf.bottom() - strip_h;
    let (lo, hi) = (view.view_lo_hz, view.view_hi_hz);

    // Subtle base so the strip reads as one band even between segments.
    p.rect_filled(
        Rect::from_min_max(pos2(wf.left(), top), wf.max),
        0.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, 70),
    );

    for seg in segs {
        if seg.hi <= lo || seg.lo >= hi {
            continue;
        }
        let x0 = view.freq_to_x(seg.lo, wf).max(wf.left());
        let x1 = view.freq_to_x(seg.hi, wf).min(wf.right());
        if x1 - x0 < 1.5 {
            continue;
        }
        let rect = Rect::from_min_max(pos2(x0, top), pos2(x1, wf.bottom()));
        let c = seg.kind.color();
        p.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 128));
        // Left divider between adjacent segments.
        p.vline(x0, rect.y_range(), Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120)));

        // Label, only if it fits inside the segment's on-screen width.
        let galley =
            p.layout_no_wrap(seg.label.to_string(), FontId::proportional(10.5), Color32::WHITE);
        if galley.size().x + 8.0 <= x1 - x0 {
            let tp = pos2(
                (x0 + x1) * 0.5 - galley.size().x * 0.5,
                top + (strip_h - galley.size().y) * 0.5,
            );
            // Shadow for legibility over the busy waterfall.
            p.galley(tp + eframe::egui::vec2(1.0, 1.0), galley.clone(), Color32::from_black_alpha(180));
            p.galley(tp, galley, Color32::WHITE);
        }
    }

    // Top border of the strip.
    p.hline(wf.x_range(), top, Stroke::new(1.0, theme::LINE_LIT));
}
