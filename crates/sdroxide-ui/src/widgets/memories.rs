//! Stored frequency memories, marked along the bottom of the waterfall.
//!
//! One label per memory whose dial lands in the visible span, reading
//! `Mem: folder / name` — or `Mem: name` for an unfiled one — hung on a thin
//! vertical line at the frequency itself.
//!
//! Stacked inwards from the band-plan strip rather than over it, and packed
//! into lanes so that channels a few kilohertz apart stagger instead of
//! overprinting. What does not fit is dropped: the marks annotate the picture,
//! and a memory list deep enough to bury the waterfall would cost more than it
//! tells.
//!
//! # Clicking one recalls it
//!
//! The label is a control as well as an annotation (issue #320). A mark is a
//! channel somebody stored on purpose and then went looking for on the
//! waterfall; making them cross to the memory window to press it was asking
//! them to name a thing they were already pointing at. A click recalls the
//! memory whole — dial, mode and filter — exactly as the list does, because a
//! memory is a channel and not a frequency.
//!
//! Laying out and drawing are separate passes for that reason: the boxes have
//! to exist before the pointer is dealt with, several hundred lines before
//! anything is painted. Same split, and for the same reason, as the network
//! spot boxes next door.

use eframe::egui::{
    Color32, FontId, Painter, Rect, Stroke, StrokeKind, pos2,
    text::{LayoutJob, TextFormat, TextWrapping},
    vec2,
};

use crate::view::ViewState;

/// One memory to mark on the waterfall.
///
/// The label arrives formatted, the way [`super::spectrum_view::IsmLabel`]
/// does: which folder a memory is filed under is a lookup into the folder list,
/// and the painter has no business holding one.
pub struct MemMark {
    /// Which memory this is, for [`sdroxide_types::Command::RecallMemory`] when
    /// the mark is clicked.
    pub id: u32,
    /// The memory's dial frequency.
    pub freq_hz: f64,
    /// Already formatted — `Mem: folder / name`.
    pub text: String,
}

/// One mark, placed. Produced by [`layout`] and consumed by [`draw`], with the
/// pointer handled in between.
pub struct MemBox {
    /// The label's box — what a click has to land in.
    pub rect: Rect,
    /// Which mark in the slice this was laid out from.
    pub idx: usize,
    /// The frequency line, from the strip edge out past the label.
    tick: [eframe::egui::Pos2; 2],
    galley: std::sync::Arc<eframe::egui::Galley>,
}

const BOX_H: f32 = 16.0;
const PT: f32 = 10.5;
const PAD_X: f32 = 5.0;
/// Vertical gap between staggered lanes.
const LANE_GAP: f32 = 2.0;
/// Minimum horizontal gap between two labels sharing a lane.
const H_GAP: f32 = 6.0;
/// How far past the label the frequency line runs, so the mark is visible over
/// the signal it names and not only where it is written down.
const TICK: f32 = 8.0;
/// Cap on stacked lanes; a memory that would need a deeper one is dropped.
const MAX_LANES: usize = 4;
/// Widest a label may grow before its text is cut short with an ellipsis. A
/// memory name is free text and nothing stops it being a sentence.
const BOX_MAX_W: f32 = 240.0;
/// Fraction of the waterfall the lanes may take, band-plan strip included.
const MAX_FRAC: f32 = 0.5;
/// The same, with a mode panel sharing the height — see [`super::bandplan`],
/// which steps aside on the same reasoning: what is left of the waterfall *is*
/// the signal being worked.
const PANEL_MAX_FRAC: f32 = 0.12;

/// Place the memory marks along the waterfall's oldest edge — its bottom
/// normally, its top when the waterfall is flipped — stacked inwards past the
/// band-plan strip.
///
/// `strip_h` is how much of that edge the band-plan strip has already taken
/// (zero when it stood aside), and `panel_below` says a mode panel is sharing
/// the height.
///
/// Nothing is painted here; [`draw`] does that once the pointer has had its
/// look at the boxes.
pub fn layout(
    p: &Painter,
    view: &ViewState,
    wf: &Rect,
    marks: &[MemMark],
    strip_h: f32,
    panel_below: bool,
) -> Vec<MemBox> {
    let mut out = Vec::new();
    if marks.is_empty() || view.span() <= 0.0 || wf.height() < 24.0 {
        return out;
    }
    let fs = crate::theme::panadapter_font_scale();
    let box_h = BOX_H * fs;
    let frac = if panel_below { PANEL_MAX_FRAC } else { MAX_FRAC };
    let fit = ((wf.height() * frac - strip_h) / (box_h + LANE_GAP)).floor().max(0.0) as usize;
    let lanes = fit.min(MAX_LANES);
    if lanes == 0 {
        return out;
    }

    let ink = crate::theme::scope().good;
    let font = FontId::proportional(PT * fs);

    // In screen order, so packing a label into a lane only ever has to look
    // leftwards — as the spot lanes do.
    let mut vis: Vec<(f32, usize, &MemMark)> = marks
        .iter()
        .enumerate()
        .filter(|(_, m)| (view.view_lo_hz..=view.view_hi_hz).contains(&m.freq_hz))
        .map(|(i, m)| (view.freq_to_x(m.freq_hz, wf), i, m))
        .collect();
    vis.sort_by(|a, b| a.0.total_cmp(&b.0));

    let flip = view.waterfall_flip;
    let edge = if flip { wf.top() + strip_h } else { wf.bottom() - strip_h };
    let mut lane_right: Vec<f32> = Vec::new();
    for (x, idx, m) in vis {
        // Truncated rather than clipped, so a name too long for the box says so
        // with an ellipsis instead of stopping mid-word.
        let mut job = LayoutJob::single_section(
            m.text.clone(),
            TextFormat { font_id: font.clone(), color: ink, ..Default::default() },
        );
        job.wrap = TextWrapping::truncate_at_width(BOX_MAX_W * fs - 2.0 * PAD_X);
        let galley = p.layout_job(job);
        let w = galley.size().x + 2.0 * PAD_X;
        // Centred on its own frequency, but never hanging off the picture: a
        // label pushed back inside is still tied to its dial by the line.
        let left = (x - w * 0.5).clamp(wf.left(), (wf.right() - w).max(wf.left()));
        let mut lane = lane_right.len();
        for (k, &r) in lane_right.iter().enumerate() {
            if left >= r {
                lane = k;
                break;
            }
        }
        if lane >= lanes {
            continue;
        }
        if lane == lane_right.len() {
            lane_right.push(0.0);
        }
        lane_right[lane] = left + w + H_GAP;

        let off = lane as f32 * (box_h + LANE_GAP);
        let (top, bottom) =
            if flip { (edge + off, edge + off + box_h) } else { (edge - off - box_h, edge - off) };
        // From the band-plan strip out past the label, so one stacked two lanes
        // deep still reads as belonging to the frequency under it.
        let tip = if flip { bottom + TICK } else { top - TICK };
        out.push(MemBox {
            rect: Rect::from_min_size(pos2(left, top), vec2(w, box_h)),
            idx,
            tick: [pos2(x, edge), pos2(x, tip)],
            galley,
        });
    }
    out
}

/// Paint what [`layout`] placed. `hovered` is the box the pointer is over, if
/// any — it brightens, because a mark that can be pressed has to say so.
pub fn draw(p: &Painter, boxes: &[MemBox], hovered: Option<usize>) {
    let ink = crate::theme::scope().good;
    let line = Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 140);
    for (k, b) in boxes.iter().enumerate() {
        let on = hovered == Some(k);
        let edge = if on { ink } else { line };
        p.line_segment(b.tick, Stroke::new(1.0, edge));
        p.circle_filled(b.tick[0], 1.8, ink);
        let fill = if on {
            Color32::from_rgba_unmultiplied(14, 34, 44, 240)
        } else {
            Color32::from_rgba_unmultiplied(6, 12, 20, 222)
        };
        p.rect_filled(b.rect, 2.0, fill);
        p.rect_stroke(b.rect, 2.0, Stroke::new(if on { 1.4 } else { 1.0 }, edge), StrokeKind::Inside);
        let g = b.galley.clone();
        let y = b.rect.top() + (b.rect.height() - g.size().y) * 0.5;
        p.galley(pos2(b.rect.left() + PAD_X, y), g, ink);
    }
}
