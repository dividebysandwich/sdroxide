//! A colour-coded bandplan overlay drawn along the bottom of the waterfall.
//!
//! Two zoom tiers: when zoomed out, coarse allocations (ham bands, broadcast
//! bands, CB, AM); when zoomed into a ham band, the fine sub-segments (CW,
//! digital, SSB, beacons). Labels are drawn only where the segment is wide
//! enough on screen.
//!
//! Both tiers are built from the station's band plan — the ham blocks from
//! [`sdroxide_types::Band::edges_in`], their sub-segments from the shared
//! segment table the skimmers gate on — plus the region-specific broadcast list
//! below. Nothing here restates a band edge: the plan is the operator's
//! `bandplan.json`, and a strip that disagreed with the transmit lockout would
//! be worse than no strip.

use eframe::egui::{Color32, FontId, Painter, Rect, Stroke, pos2};
use sdroxide_types::{Band, BandplanKind as Kind, Region, SegmentKind};

use crate::theme;
use crate::view::ViewState;

/// The shade this usage class is currently painted in — the operator's pick,
/// or the stock one until they make one. Looked up per block per frame rather
/// than cached, the same way [`theme::spot_color`] is: it is an atomic load,
/// and a retint has to show on the very next frame to be worth picking.
fn color(kind: Kind) -> Color32 {
    let (r, g, b) = theme::bandplan_color(kind);
    Color32::from_rgb(r, g, b)
}

/// One block of the allocation strip.
struct Seg {
    lo: f64,
    hi: f64,
    label: &'static str,
    kind: Kind,
}

const fn s(lo: f64, hi: f64, label: &'static str, kind: Kind) -> Seg {
    Seg { lo, hi, label, kind }
}

/// The whole-band label for a ham block. A match rather than lower-casing
/// `Band::label` at each use: this runs every frame, and a `&'static str` costs
/// nothing where a `String` would cost an allocation per band per frame.
///
/// Takes the region for the one band whose name depends on it — 5650 MHz is
/// 6 cm under the Region 1 plan and 5 cm in the other two. The strip is already
/// drawn per region, so the caller has it in hand.
fn ham_label(band: Band, region: Region) -> &'static str {
    match band {
        Band::M160 => "160m HAM",
        Band::M80 => "80m HAM",
        Band::M60 => "60m HAM",
        Band::M40 => "40m HAM",
        Band::M30 => "30m HAM",
        Band::M20 => "20m HAM",
        Band::M17 => "17m HAM",
        Band::M15 => "15m HAM",
        Band::M12 => "12m HAM",
        Band::M10 => "10m HAM",
        Band::M6 => "6m HAM",
        Band::M4 => "4m HAM",
        Band::M2 => "2m HAM",
        Band::M125 => "1.25m HAM",
        Band::M70 => "70cm HAM",
        Band::Cm33 => "33cm HAM",
        Band::Cm23 => "23cm HAM",
        Band::Cm13 => "13cm HAM",
        Band::Cm9 => "9cm HAM",
        Band::Cm6 => match region {
            Region::R1 => "6cm HAM",
            Region::R2 | Region::R3 => "5cm HAM",
        },
        Band::Cm3 => "3cm HAM",
        Band::Gen => "GEN",
    }
}

// Above this view span, show coarse bands; below it, fine ham sub-segments.
const FINE_MAX_SPAN: f64 = 800_000.0;

const M: f64 = 1_000_000.0;

/// Broadcast, CB and AM allocations — everything on the strip that is not an
/// amateur band. Shown at both zoom tiers, since none of it has sub-structure
/// worth drawing.
///
/// Three of these are region-dependent, and two of those are the reason it
/// matters here: 3.900–4.000 and 7.200–7.300 are shortwave broadcasting in
/// Regions 1 and 3 and amateur allocations in Region 2, so drawing them
/// unconditionally would paint an orange broadcast block across an American
/// operator's own band.
fn non_ham(region: Region) -> Vec<Seg> {
    let mut v = vec![
        s(0.153 * M, 0.279 * M, "LW AM", Kind::Am),
        s(0.531 * M, 1.602 * M, "MW AM", Kind::Am),
        s(4.750 * M, 5.060 * M, "60m BC", Kind::Broadcast),
        s(5.900 * M, 6.200 * M, "49m BC", Kind::Broadcast),
        s(9.400 * M, 9.900 * M, "31m BC", Kind::Broadcast),
        s(11.600 * M, 12.100 * M, "25m BC", Kind::Broadcast),
        s(13.570 * M, 13.870 * M, "22m BC", Kind::Broadcast),
        s(15.100 * M, 15.830 * M, "19m BC", Kind::Broadcast),
        s(17.480 * M, 17.900 * M, "16m BC", Kind::Broadcast),
        s(21.450 * M, 21.850 * M, "13m BC", Kind::Broadcast),
        s(25.670 * M, 26.100 * M, "11m BC", Kind::Broadcast),
        s(26.965 * M, 27.405 * M, "CB", Kind::Cb),
    ];
    // 75 m broadcasting: 3.950–4.000 in Region 1, all of 3.900–4.000 in
    // Region 3, and none of it in Region 2, where the band is 80 m.
    match region {
        Region::R1 => v.push(s(3.950 * M, 4.000 * M, "75m BC", Kind::Broadcast)),
        Region::R3 => v.push(s(3.900 * M, 4.000 * M, "75m BC", Kind::Broadcast)),
        Region::R2 => {}
    }
    // 41 m broadcasting starts where 40 m ends: 7.200 outside Region 2, 7.300
    // inside it.
    let bc41_lo = if region == Region::R2 { 7.300 * M } else { 7.200 * M };
    v.push(s(bc41_lo, 7.450 * M, "41m BC", Kind::Broadcast));
    v.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    v
}

/// The coarse tier: whole amateur bands plus the non-ham allocations.
///
/// Every HF and VHF/UHF band `Band::ALL` names appears, so 6 m, 2 m and 70 cm
/// finally get a block of their own — and get it at the right width for the
/// region.
///
/// Rebuilt each frame rather than cached. It is two forty-element vectors and a
/// sort, which is nothing beside the FFT and the waterfall upload either side of
/// it — and a cache would need invalidating whenever the operator reloads
/// `bandplan.json` or the station announces a different one, which is exactly
/// the kind of bookkeeping that quietly goes stale.
fn coarse(region: Region) -> Vec<Seg> {
    let mut v = non_ham(region);
    for band in Band::ALL {
        let Some((lo, hi)) = band.edges_in(region) else { continue };
        v.push(s(lo, hi, ham_label(band, region), Kind::Ham));
    }
    v.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    v
}

/// The fine tier: the region's sub-segments where there are any, and the whole
/// band where there are not (everything above 30 MHz).
fn fine(region: Region) -> Vec<Seg> {
    let segs = sdroxide_types::segments_in(region);
    let mut v = non_ham(region);
    for seg in segs {
        let (label, kind) = match seg.kind {
            SegmentKind::Cw => ("CW", Kind::Cw),
            SegmentKind::Digi => ("Digi", Kind::Digi),
            SegmentKind::Phone => ("SSB", Kind::Phone),
            SegmentKind::Beacon => ("Bcn", Kind::Beacon),
            SegmentKind::All => ("All", Kind::Ham),
        };
        v.push(s(seg.lo, seg.hi, label, kind));
    }
    // The bands with no sub-segment table keep their whole-band block, so
    // zooming into 2 m or 70 cm does not empty the strip.
    for band in Band::ALL {
        let Some((lo, hi)) = band.edges_in(region) else { continue };
        if !segs.iter().any(|s| s.lo < hi && s.hi > lo) {
            v.push(s(lo, hi, ham_label(band, region), Kind::Ham));
        }
    }
    v.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    v
}

// ── Explicit digi-mode detail rows ──────────────────────────────────────────
//
// When zoomed into a band, the coarse "Digi" allocation is broken out into the
// individual modes that actually live there (FT8, FT4, JS8, WSPR, QRSS, PSK,
// RTTY, SSTV, RIFP). Many of these overlap in frequency, so they're partitioned into
// non-overlapping rows and stacked above the allocation strip.

const C_FT8: Color32 = Color32::from_rgb(0x4D, 0x8C, 0xFF);
const C_FT4: Color32 = Color32::from_rgb(0x1F, 0xC7, 0xB0);
const C_JS8: Color32 = Color32::from_rgb(0x8B, 0xD1, 0x3A);
/// FT2 shares FT4's family, so it takes a neighbouring hue rather than a
/// contrasting one — the eye should read "the fast pair" at a glance.
const C_FT2: Color32 = Color32::from_rgb(0x2B, 0xA8, 0xE0);
const C_WSPR: Color32 = Color32::from_rgb(0xB4, 0x8E, 0xF9);
const C_QRSS: Color32 = Color32::from_rgb(0x76, 0x6A, 0xD6);
const C_PSK: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x3D);
const C_RTTY: Color32 = Color32::from_rgb(0xF2, 0xC2, 0x4B);
const C_SSTV: Color32 = Color32::from_rgb(0xF0, 0x5A, 0x9C);
const C_RIFP: Color32 = Color32::from_rgb(0x5A, 0xE0, 0xD0);

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
///
/// The WSJT modes' dials are one global list and stay that way; PSK, RTTY and
/// SSTV move with the region, and the skimmer windows they are drawn from are
/// the ones the skimmers actually use.
fn digi_segments(region: Region) -> Vec<DigiSeg> {
    use sdroxide_types::{
        FT2_DIALS, FT4_DIALS, FT8_DIALS, JS8_DIALS, RIFP_CALLING, WSPR_DIALS, psk_ranges_in,
        rtty_ranges_in, sstv_dials_in,
    };
    let mut v = Vec::with_capacity(64);
    for &f in FT8_DIALS {
        v.push(dg(f, f + 2700.0, "FT8", C_FT8));
    }
    for &f in FT4_DIALS {
        v.push(dg(f, f + 2500.0, "FT4", C_FT4));
    }
    for &f in FT2_DIALS {
        v.push(dg(f, f + 2500.0, "FT2", C_FT2));
    }
    for &f in JS8_DIALS {
        v.push(dg(f, f + 2500.0, "JS8", C_JS8));
    }
    for &f in WSPR_DIALS {
        // QRSS/MEPT beacons sit just below the 200 Hz WSPR window.
        v.push(dg(f + 1000.0, f + 1400.0, "QRSS", C_QRSS));
        v.push(dg(f + 1400.0, f + 1600.0, "WSPR", C_WSPR));
    }
    for &(lo, hi) in psk_ranges_in(region) {
        v.push(dg(lo, hi, "PSK", C_PSK));
    }
    for &(lo, hi) in rtty_ranges_in(region) {
        v.push(dg(lo, hi, "RTTY", C_RTTY));
    }
    for f in sstv_dials_in(region) {
        v.push(dg(f, f + 2700.0, "SSTV", C_SSTV));
    }
    // RIFP is the one entry centred on its frequency rather than starting at
    // it: its CPFSK channel straddles the dial.
    for &f in RIFP_CALLING {
        let half = 12_500.0;
        v.push(dg(f - half, f + half, "RIFP", C_RIFP));
    }
    v
}

/// Partition the in-view digi spans into non-overlapping rows (greedy interval
/// colouring): each span goes in the first row whose last span ends before it
/// starts, else a new row. Overlapping modes therefore land in separate rows.
fn stack_digi(lo: f64, hi: f64, region: Region) -> Vec<Vec<DigiSeg>> {
    let mut vis: Vec<DigiSeg> =
        digi_segments(region).into_iter().filter(|d| d.hi > lo && d.lo < hi).collect();
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

/// Fraction of the waterfall the strip may take while an operating panel is
/// sharing the height with it. Past this it is dropped entirely: with a panel
/// below, what is left of the waterfall *is* the signal being worked, and the
/// allocation labels are not worth a fifth of it — especially in the digital
/// modes, where the view is locked to one sub-band the strip has nothing new to
/// say about.
const PANEL_MAX_FRAC: f32 = 0.10;

/// How deep the strip will be, worked out without drawing it. Zero where it
/// would stand aside.
///
/// Wanted on its own because the memory marks stack inwards from the strip's
/// inner edge and are laid out before the pointer is dealt with — hundreds of
/// lines before anything on the waterfall is painted. See
/// [`super::memories::layout`].
pub fn height(view: &ViewState, wf: &Rect, panel_below: bool) -> f32 {
    plan(view, wf, panel_below).map_or(0.0, |p| p.total_h)
}

/// What the strip works out before it draws anything.
struct Plan {
    fs: f32,
    base_h: f32,
    digi_h: f32,
    region: sdroxide_types::Region,
    digi_rows: Vec<Vec<DigiSeg>>,
    n_digi: usize,
    total_h: f32,
}

/// Decide the strip's rows and depth, or `None` where it stands aside.
fn plan(view: &ViewState, wf: &Rect, panel_below: bool) -> Option<Plan> {
    let span = view.span();
    if span <= 0.0 || wf.height() < 24.0 {
        return None;
    }
    let (lo, hi) = (view.view_lo_hz, view.view_hi_hz);

    // Rows scale with the panadapter font-size setting along with their
    // labels — `draw_seg` drops any label taller than its row, so a bigger
    // font in an unscaled row would show fewer labels, not bigger ones.
    let fs = crate::theme::panadapter_font_scale();
    let base_h = 18.0f32 * fs;
    let digi_h = 14.0f32 * fs;

    // Read once per frame, so every row of the strip is drawn under the same
    // band plan even if the operator changes the setting mid-frame.
    let region = sdroxide_types::region();

    // Explicit digi-mode rows, stacked so overlapping modes are separated.
    let digi_rows = if span <= DIGI_MAX_SPAN { stack_digi(lo, hi, region) } else { Vec::new() };
    // Cap rows both by preference and by how much waterfall we're willing to use.
    let fit = (((wf.height() * 0.5) - base_h) / digi_h).floor().max(0.0) as usize;
    let n_digi = digi_rows.len().min(MAX_DIGI_ROWS).min(fit);

    let total_h = base_h + n_digi as f32 * digi_h;
    if panel_below && total_h > wf.height() * PANEL_MAX_FRAC {
        return None;
    }
    Some(Plan { fs, base_h, digi_h, region, digi_rows, n_digi, total_h })
}

/// Draw the bandplan strip along the oldest edge of the waterfall rect — its
/// bottom normally, its top when the waterfall is flipped — so the strip never
/// covers the newest rows.
///
/// `panel_below` — a mode panel (FT8's decodes, JS8's chat, CW's keyboard …) is
/// taking part of the height, so the strip has to fit in [`PANEL_MAX_FRAC`] of
/// what is left or step aside.
///
/// Returns how deep the strip ended up, so that whatever else annotates the
/// same edge — the memory marks — can stack inwards from it instead of over
/// it. Zero when it stood aside.
pub fn overlay(p: &Painter, view: &ViewState, wf: &Rect, panel_below: bool) -> f32 {
    let span = view.span();
    let Some(plan) = plan(view, wf, panel_below) else { return 0.0 };
    let Plan { fs, base_h, digi_h, region, digi_rows, n_digi, total_h } = plan;
    // The allocation row sits outermost, on the waterfall's oldest edge, with
    // the digi rows stacked inwards from it.
    let flip = view.waterfall_flip;
    let base_top = if flip { wf.top() } else { wf.bottom() - base_h };
    let (strip_top, strip_bottom) =
        if flip { (wf.top(), wf.top() + total_h) } else { (wf.bottom() - total_h, wf.bottom()) };

    // Subtle base so the strip reads as one band even between segments.
    p.rect_filled(
        Rect::from_min_max(pos2(wf.left(), strip_top), pos2(wf.right(), strip_bottom)),
        0.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, 80),
    );

    // Base allocation row (coarse bands, or fine CW/Digi/SSB sub-segments).
    for seg in if span <= FINE_MAX_SPAN { fine(region) } else { coarse(region) } {
        let color = color(seg.kind);
        draw_seg(p, view, wf, seg.lo, seg.hi, color, seg.label, base_top, base_h, 10.5 * fs);
    }

    // Digi-mode rows stacked inwards from the allocation row.
    for (i, row) in digi_rows.iter().take(n_digi).enumerate() {
        let row_top = if flip {
            base_top + base_h + i as f32 * digi_h
        } else {
            base_top - (i as f32 + 1.0) * digi_h
        };
        for d in row {
            draw_seg(p, view, wf, d.lo, d.hi, d.color, d.label, row_top, digi_h, 9.5 * fs);
        }
    }

    // Border along the strip's inner edge.
    p.hline(
        wf.x_range(),
        if flip { strip_bottom } else { strip_top },
        Stroke::new(1.0, theme::scope().line),
    );
    total_h
}
