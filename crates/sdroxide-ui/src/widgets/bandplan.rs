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
    s(14.070 * M, 14.099 * M, "Digi", Kind::Digi),
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

// ── Explicit digi-mode detail rows ──────────────────────────────────────────
//
// When zoomed into a band, the coarse "Digi" allocation is broken out into the
// individual modes that actually live there (FT8, FT4, JS8, WSPR, QRSS, PSK,
// RTTY, SSTV). Many of these overlap in frequency, so they're partitioned into
// non-overlapping rows and stacked above the allocation strip.

const C_FT8: Color32 = Color32::from_rgb(0x4D, 0x8C, 0xFF);
const C_FT4: Color32 = Color32::from_rgb(0x1F, 0xC7, 0xB0);
const C_JS8: Color32 = Color32::from_rgb(0x8B, 0xD1, 0x3A);
const C_WSPR: Color32 = Color32::from_rgb(0xB4, 0x8E, 0xF9);
const C_QRSS: Color32 = Color32::from_rgb(0x76, 0x6A, 0xD6);
const C_PSK: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x3D);
const C_RTTY: Color32 = Color32::from_rgb(0xF2, 0xC2, 0x4B);
const C_SSTV: Color32 = Color32::from_rgb(0xF0, 0x5A, 0x9C);

// Below this view span, draw the explicit digi-mode detail rows; above it they'd
// be sub-pixel and only clutter the strip.
const DIGI_MAX_SPAN: f64 = 100_000.0;
const MAX_DIGI_ROWS: usize = 3;

#[derive(Clone, Copy)]
struct DigiSeg {
    lo: f64,
    hi: f64,
    label: &'static str,
    color: Color32,
}

const fn dg(lo: f64, hi: f64, label: &'static str, color: Color32) -> DigiSeg {
    DigiSeg { lo, hi, label, color }
}

/// The explicit digi-mode activity spans, derived from the shared calling-
/// frequency tables so they stay consistent with the skimmer gating.
fn digi_segments() -> Vec<DigiSeg> {
    use sdroxide_types::{
        FT4_DIALS, FT8_DIALS, JS8_DIALS, PSK_RANGES, RTTY_RANGES, SSTV_CALLING, WSPR_DIALS,
    };
    let mut v = Vec::with_capacity(64);
    for &f in FT8_DIALS {
        v.push(dg(f, f + 2700.0, "FT8", C_FT8));
    }
    for &f in FT4_DIALS {
        v.push(dg(f, f + 2500.0, "FT4", C_FT4));
    }
    for &f in JS8_DIALS {
        v.push(dg(f, f + 2500.0, "JS8", C_JS8));
    }
    for &f in WSPR_DIALS {
        // QRSS/MEPT beacons sit just below the 200 Hz WSPR window.
        v.push(dg(f + 1000.0, f + 1400.0, "QRSS", C_QRSS));
        v.push(dg(f + 1400.0, f + 1600.0, "WSPR", C_WSPR));
    }
    for &(lo, hi) in PSK_RANGES {
        v.push(dg(lo, hi, "PSK", C_PSK));
    }
    for &(lo, hi) in RTTY_RANGES {
        v.push(dg(lo, hi, "RTTY", C_RTTY));
    }
    for &f in SSTV_CALLING {
        v.push(dg(f, f + 2700.0, "SSTV", C_SSTV));
    }
    v
}

/// Partition the in-view digi spans into non-overlapping rows (greedy interval
/// colouring): each span goes in the first row whose last span ends before it
/// starts, else a new row. Overlapping modes therefore land in separate rows.
fn stack_digi(lo: f64, hi: f64) -> Vec<Vec<DigiSeg>> {
    let mut vis: Vec<DigiSeg> =
        digi_segments().into_iter().filter(|d| d.hi > lo && d.lo < hi).collect();
    vis.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    let mut rows: Vec<Vec<DigiSeg>> = Vec::new();
    let mut row_hi: Vec<f64> = Vec::new();
    'next: for d in vis {
        for (r, last) in row_hi.iter_mut().enumerate() {
            if d.lo >= *last {
                *last = d.hi;
                rows[r].push(d);
                continue 'next;
            }
        }
        row_hi.push(d.hi);
        rows.push(vec![d]);
    }
    rows
}

/// Draw one coloured segment in `[top, top+h]`, clipped to the view, with a
/// centred label when it fits.
#[allow(clippy::too_many_arguments)]
fn draw_seg(
    p: &Painter,
    view: &ViewState,
    wf: &Rect,
    lo: f64,
    hi: f64,
    color: Color32,
    label: &str,
    top: f32,
    h: f32,
    font: f32,
) {
    if hi <= view.view_lo_hz || lo >= view.view_hi_hz {
        return;
    }
    let x0 = view.freq_to_x(lo, wf).max(wf.left());
    let x1 = view.freq_to_x(hi, wf).min(wf.right());
    if x1 - x0 < 1.5 {
        return;
    }
    let rect = Rect::from_min_max(pos2(x0, top), pos2(x1, top + h));
    p.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 150));
    // Left divider between adjacent segments.
    p.vline(x0, rect.y_range(), Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120)));

    let white = p.layout_no_wrap(label.to_string(), FontId::proportional(font), Color32::WHITE);
    if white.size().x + 6.0 <= x1 - x0 && white.size().y <= h {
        let tp = pos2((x0 + x1) * 0.5 - white.size().x * 0.5, top + (h - white.size().y) * 0.5);
        // Black outline in eight directions for legibility over the busy
        // waterfall and the semi-transparent segment fills. `layout_no_wrap`
        // bakes the colour into the galley, so the outline needs its own black
        // galley (the colour argument to `galley()` is only a fallback).
        let black = p.layout_no_wrap(label.to_string(), FontId::proportional(font), Color32::BLACK);
        for (ox, oy) in [
            (-1.0, 0.0),
            (1.0, 0.0),
            (0.0, -1.0),
            (0.0, 1.0),
            (-1.0, -1.0),
            (1.0, -1.0),
            (-1.0, 1.0),
            (1.0, 1.0),
        ] {
            p.galley(tp + eframe::egui::vec2(ox, oy), black.clone(), Color32::BLACK);
        }
        p.galley(tp, white, Color32::WHITE);
    }
}

/// Draw the bandplan strip over the bottom of the waterfall rect.
pub fn overlay(p: &Painter, view: &ViewState, wf: &Rect) {
    let span = view.span();
    if span <= 0.0 || wf.height() < 24.0 {
        return;
    }
    let (lo, hi) = (view.view_lo_hz, view.view_hi_hz);

    let base_h = 18.0f32;
    let digi_h = 14.0f32;

    // Explicit digi-mode rows, stacked so overlapping modes are separated.
    let digi_rows = if span <= DIGI_MAX_SPAN { stack_digi(lo, hi) } else { Vec::new() };
    // Cap rows both by preference and by how much waterfall we're willing to use.
    let fit = (((wf.height() * 0.5) - base_h) / digi_h).floor().max(0.0) as usize;
    let n_digi = digi_rows.len().min(MAX_DIGI_ROWS).min(fit);

    let total_h = base_h + n_digi as f32 * digi_h;
    let base_top = wf.bottom() - base_h;
    let strip_top = wf.bottom() - total_h;

    // Subtle base so the strip reads as one band even between segments.
    p.rect_filled(
        Rect::from_min_max(pos2(wf.left(), strip_top), wf.max),
        0.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, 80),
    );

    // Base allocation row (coarse bands, or fine CW/Digi/SSB sub-segments).
    let segs: &[Seg] = if span <= FINE_MAX_SPAN { FINE } else { COARSE };
    for seg in segs {
        draw_seg(p, view, wf, seg.lo, seg.hi, seg.kind.color(), seg.label, base_top, base_h, 10.5);
    }

    // Digi-mode rows stacked above the allocation row.
    for (i, row) in digi_rows.iter().take(n_digi).enumerate() {
        let row_top = base_top - (i as f32 + 1.0) * digi_h;
        for d in row {
            draw_seg(p, view, wf, d.lo, d.hi, d.color, d.label, row_top, digi_h, 9.5);
        }
    }

    // Top border of the strip.
    p.hline(wf.x_range(), strip_top, Stroke::new(1.0, theme::LINE_LIT));
}
