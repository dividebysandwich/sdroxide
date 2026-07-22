//! The panadapter: spectrum line + frequency scale + GPU waterfall, with
//! drag-pan, wheel-zoom (around the cursor), click-to-tune (shift-click
//! tunes VFO B), and draggable passband filter edges.

use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui,
    pos2, vec2,
};
use sdroxide_types::{Command, Mode, RadioState, RxId, SkimmerSpot, SpectrumFrame, Vfo};

use crate::view::ViewState;
use crate::waterfall_gpu::WaterfallCallback;

const SCALE_H: f32 = 18.0;
/// Tuning rounds to this step on click-tune.
const CLICK_TUNE_STEP: f64 = 10.0;
/// Pixel distance for grabbing a filter edge.
const EDGE_GRAB_PX: f32 = 6.0;

/// Per-frame display tuning from the app. Both the waterfall advance and the
/// time gridlines are driven by the same wall-clock so they scroll in lockstep
/// (the app converts elapsed time × `rows_per_sec` into `rows_to_write`).
#[derive(Clone, Copy)]
pub struct WfTuning {
    /// Waterfall rows to append this frame (from elapsed wall-clock time).
    pub rows_to_write: u32,
    /// Scroll rate in rows/second — used both to advance the waterfall and to
    /// space the 60-second gridlines, so the line tracks the waterfall exactly.
    pub rows_per_sec: f32,
    /// Wall-clock UTC seconds, for the minute-boundary time labels.
    pub now_unix: f64,
}

// --- skimmer spot boxes ---------------------------------------------------
const SPOT_BOX_W: f32 = 236.0;
const SPOT_BOX_H: f32 = 19.0;
/// Font size for the box's callsign / message text.
const SPOT_CALL_PT: f32 = 13.0;
const SPOT_MSG_PT: f32 = 12.5;
/// Horizontal padding inside a box.
const SPOT_PAD: f32 = 5.0;
/// Vertical gap between staggered lanes.
const SPOT_LANE_GAP: f32 = 3.0;
/// Gap from the top of the waterfall to the first lane.
const SPOT_TOP_MARGIN: f32 = 4.0;
/// Minimum horizontal gap between two boxes sharing a lane.
const SPOT_H_GAP: f32 = 6.0;
/// Horizontal leader length from the signal to the box's left edge.
const SPOT_LEADER: f32 = 16.0;
/// Cap on stacked lanes; spots that would need a deeper lane are dropped.
const SPOT_MAX_LANES: usize = 6;

/// A laid-out skimmer box: its screen rect, the x of the signal it belongs to
/// (the box sits to its right, joined by a leader), and the spot's index.
struct SpotBox {
    rect: Rect,
    sig_x: f32,
    idx: usize,
}

/// Border/leader colour for a spot: cyan when hovered, dim-cyan once a
/// callsign is known, grey otherwise.
fn spot_color(spot: &SkimmerSpot, hovered: bool) -> Color32 {
    if hovered {
        crate::theme::CYAN
    } else if spot.callsign.is_some() {
        crate::theme::CYAN_DIM
    } else {
        Color32::from_gray(78)
    }
}

/// The on-screen width a box needs to hold just its callsign (used by the
/// fit-to-text FT8 boxes, which show the callsign only).
fn spot_content_width(p: &egui::Painter, spot: &SkimmerSpot) -> f32 {
    let mut w = 2.0 * SPOT_PAD;
    if let Some(call) = &spot.callsign {
        w += p.layout_no_wrap(call.clone(), FontId::monospace(SPOT_CALL_PT), Color32::WHITE).size().x;
    }
    w.clamp(30.0, 240.0)
}

/// Lay skimmer spots out into staggered lanes over the waterfall. Each box sits
/// to the right of its signal (offset by a leader). Visible spots are sorted by
/// x and greedily packed into the lowest lane whose footprint — from the signal
/// x through the box's right edge — clears the previous box, so nearby signals
/// stack vertically instead of overlapping. Off-view / past-cap spots are omitted.
///
/// When `fit` is set, each box is sized to its text (used for FT8 station boxes,
/// whose message is a complete fixed string); otherwise a fixed width is used
/// (the CW skimmer, whose message is a live-growing tail).
fn layout_spots(
    p: &egui::Painter,
    view: &ViewState,
    rect: &Rect,
    wf_rect: &Rect,
    spots: &[SkimmerSpot],
    fit: bool,
) -> Vec<SpotBox> {
    let mut vis: Vec<(f32, usize)> = spots
        .iter()
        .enumerate()
        .filter(|(_, s)| (view.view_lo_hz..=view.view_hi_hz).contains(&s.freq_hz))
        .map(|(i, s)| (view.freq_to_x(s.freq_hz, rect), i))
        .collect();
    vis.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut lane_right: Vec<f32> = Vec::new();
    let mut out = Vec::with_capacity(vis.len());
    for (xc, idx) in vis {
        let box_w = if fit { spot_content_width(p, &spots[idx]) } else { SPOT_BOX_W };
        let box_left = xc + SPOT_LEADER;
        // Footprint spans the signal tick through the box's right edge, so a
        // later box's tick can't land on top of an earlier box in the lane.
        let foot_right = box_left + box_w + SPOT_H_GAP;
        let mut lane = lane_right.len();
        for (k, &r) in lane_right.iter().enumerate() {
            if xc >= r {
                lane = k;
                break;
            }
        }
        if lane >= SPOT_MAX_LANES {
            continue; // too crowded here — drop the box
        }
        if lane == lane_right.len() {
            lane_right.push(0.0);
        }
        lane_right[lane] = foot_right;
        let top = wf_rect.top() + SPOT_TOP_MARGIN + lane as f32 * (SPOT_BOX_H + SPOT_LANE_GAP);
        out.push(SpotBox {
            rect: Rect::from_min_size(pos2(box_left, top), vec2(box_w, SPOT_BOX_H)),
            sig_x: xc,
            idx,
        });
    }
    out
}

/// Scale a colour's alpha by `a` (for fading spots out).
fn fade(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * a).round() as u8)
}

/// Draw one skimmer box at opacity `alpha` (1 = solid, 0 = gone). In
/// `callsign_only` mode (FT8/FT4) just the callsign is shown — green when the
/// station is calling CQ, white otherwise; otherwise the CW layout is used
/// (green callsign + rolling message).
fn draw_spot_box(
    p: &egui::Painter,
    b: &SpotBox,
    spot: &SkimmerSpot,
    hovered: bool,
    alpha: f32,
    callsign_only: bool,
) {
    let rect = b.rect;
    p.rect_filled(rect, 2.0, fade(Color32::from_rgba_unmultiplied(5, 11, 18, 225), alpha));
    let border = spot_color(spot, hovered);
    p.rect_stroke(
        rect,
        2.0,
        Stroke::new(if hovered { 1.5 } else { 1.0 }, fade(border, alpha)),
        StrokeKind::Inside,
    );

    let pad = SPOT_PAD;
    let cy = rect.center().y;

    if callsign_only {
        if let Some(call) = &spot.callsign {
            // FT8 message text starts with "CQ" for callers; colour those green.
            let base = if spot.text.starts_with("CQ") {
                crate::theme::GREEN
            } else {
                Color32::WHITE
            };
            let cc = fade(base, alpha);
            let g = p.layout_no_wrap(call.clone(), FontId::monospace(SPOT_CALL_PT), cc);
            p.galley(pos2(rect.left() + pad, cy - g.size().y * 0.5), g, cc);
        }
        return;
    }

    let mut x = rect.left() + pad;
    if let Some(call) = &spot.callsign {
        let cc = fade(crate::theme::GREEN, alpha);
        let g = p.layout_no_wrap(call.clone(), FontId::monospace(SPOT_CALL_PT), cc);
        p.galley(pos2(x, cy - g.size().y * 0.5), g.clone(), cc);
        x += g.size().x + 6.0;
    }

    // Message area: whatever width is left of the box. The message is anchored
    // to the right edge and clipped on the left, so the newest decoded text is
    // always shown (older text slides off the left as it arrives) — steadier to
    // read than a marquee.
    let msg_rect = Rect::from_min_max(pos2(x, rect.top()), pos2(rect.right() - pad, rect.bottom()));
    if msg_rect.width() < 6.0 {
        return;
    }
    let text = if spot.text.is_empty() { "…" } else { spot.text.as_str() };
    let col = fade(crate::theme::TEXT, alpha);
    let g = p.layout_no_wrap(text.to_string(), FontId::monospace(SPOT_MSG_PT), col);
    let ty = cy - g.size().y * 0.5;
    // Left-align while it fits; once it overflows, pin the tail to the right.
    let gx = if g.size().x <= msg_rect.width() {
        msg_rect.left()
    } else {
        msg_rect.right() - g.size().x
    };
    let mp = p.with_clip_rect(msg_rect);
    mp.galley(pos2(gx, ty), g, col);
}

/// Decaying peak-hold trace, reset whenever the frame mapping changes.
#[derive(Default)]
pub struct PeakHold {
    bins: Vec<f32>,
    center: f64,
    span: f64,
    /// Sequence of the frame last folded in, so decay runs once per *new*
    /// frame rather than once per repaint (repaint rate must not change the
    /// decay speed).
    last_seq: Option<u32>,
    /// Bumped whenever `bins` change outside the seq progression (mapping
    /// change, clear) — part of the trace-cache key.
    generation: u32,
}

/// dB-scale u8 units per new spectrum frame (~3.5 dB/s at 30 fps over a
/// 100 dB range).
const PEAK_DECAY: f32 = 0.3;

impl PeakHold {
    fn update(&mut self, f: &SpectrumFrame) {
        if self.last_seq == Some(f.seq) {
            return; // same frame redrawn — nothing new to fold in
        }
        self.last_seq = Some(f.seq);
        let mapping_changed = self.bins.len() != f.bins.len()
            || (self.center - f.center_hz).abs() > f.span_hz * 1e-6
            || (self.span - f.span_hz).abs() > f.span_hz * 1e-6;
        if mapping_changed {
            self.center = f.center_hz;
            self.span = f.span_hz;
            self.bins = f.bins.iter().map(|&b| b as f32).collect();
            self.generation = self.generation.wrapping_add(1);
            return;
        }
        for (p, &b) in self.bins.iter_mut().zip(&f.bins) {
            *p = (b as f32).max(*p - PEAK_DECAY);
        }
    }

    fn clear(&mut self) {
        self.bins.clear();
        self.last_seq = None;
        self.generation = self.generation.wrapping_add(1);
    }
}

/// Screen-space spectrum polylines, recomputed only when the underlying frame,
/// viewport, or rect actually change — pure repaints reuse the cached points.
#[derive(Default)]
pub struct TraceCache {
    live: TraceEntry,
    hold: TraceEntry,
}

#[derive(Default)]
struct TraceEntry {
    key: Option<TraceKey>,
    points: Vec<Pos2>,
}

/// Everything the per-pixel trace math depends on. Float fields are compared
/// as exact bits — any change must invalidate. (`db_floor`/`db_ceil` need no
/// entry: they reach the trace via the engine's u8 mapping, i.e. a new seq.)
#[derive(Clone, Copy, PartialEq)]
struct TraceKey {
    seq: u32,
    generation: u32,
    view_lo: u64,
    view_hi: u64,
    rect: [u32; 4],
}

fn trace_key(f: &SpectrumFrame, generation: u32, view: &ViewState, rect: &Rect) -> TraceKey {
    TraceKey {
        seq: f.seq,
        generation,
        view_lo: view.view_lo_hz.to_bits(),
        view_hi: view.view_hi_hz.to_bits(),
        rect: [
            rect.left().to_bits(),
            rect.top().to_bits(),
            rect.right().to_bits(),
            rect.bottom().to_bits(),
        ],
    }
}

impl TraceEntry {
    /// Recompute the polyline only when `key` changed; hand back a copy for
    /// `Shape::line` (a memcpy — far cheaper than the per-pixel f64 math).
    fn points_for(&mut self, key: TraceKey, compute: impl FnOnce() -> Vec<Pos2>) -> Vec<Pos2> {
        if self.key != Some(key) {
            self.points = compute();
            self.key = Some(key);
        }
        self.points.clone()
    }
}

pub fn show(
    ui: &mut Ui,
    view: &mut ViewState,
    state: &mut RadioState,
    frame: Option<&Arc<SpectrumFrame>>,
    peaks: &mut PeakHold,
    trace: &mut TraceCache,
    skimmer: &[SkimmerSpot],
    alpha: &[f32],
    wf: WfTuning,
    cmds: &mut Vec<Command>,
) {
    show_ext(ui, view, state, frame, peaks, trace, None, skimmer, alpha, wf, cmds);
}

/// `show` with an optional digital-mode audio marker. When `digi_audio_hz`
/// is `Some`, left-click sets the FT8/FT4 audio TX frequency instead of the
/// VFO, and a marker is drawn at `dial + audio_hz`.
///
/// `skimmer` are the overlay boxes (CW skimmer spots, or FT8 station callsigns
/// in digital mode) and `alpha` is a parallel per-box opacity for fade-out.
pub fn show_ext(
    ui: &mut Ui,
    view: &mut ViewState,
    state: &mut RadioState,
    frame: Option<&Arc<SpectrumFrame>>,
    peaks: &mut PeakHold,
    trace: &mut TraceCache,
    digi_audio_hz: Option<f32>,
    skimmer: &[SkimmerSpot],
    alpha: &[f32],
    wf: WfTuning,
    cmds: &mut Vec<Command>,
) {
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_gray(8));

    let Some(f) = frame.filter(|f| f.span_hz > 0.0 && !f.bins.is_empty()) else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "waiting for spectrum…",
            FontId::proportional(16.0),
            Color32::GRAY,
        );
        return;
    };

    // The pan/zoom bounds are the full device passband (frames may cover
    // only the zoomed viewport).
    let dev_center = state.center_hz;
    let dev_span = state.sample_rate;
    if view.is_unset() {
        view.fit(dev_center, dev_span);
    }

    // Layout first — interactions depend on which strip the pointer is in.
    // A collapsed spectrum shows only the waterfall.
    let frac = view.effective_spectrum_fraction();
    let spec_h = if frac <= 0.0 { 0.0 } else { ((rect.height() - SCALE_H) * frac).max(40.0) };
    let spec_rect = Rect::from_min_size(rect.min, vec2(rect.width(), spec_h));
    let scale_rect =
        Rect::from_min_size(pos2(rect.left(), spec_rect.bottom()), vec2(rect.width(), SCALE_H));
    let wf_rect = Rect::from_min_max(pos2(rect.left(), scale_rect.bottom()), rect.max);

    // Skimmer boxes are laid out up front so the click hit-test (below) and the
    // draw pass (bottom) agree on their rects. FT8 (digital) boxes fit their
    // text; CW skimmer boxes use a fixed width for their live-growing tail.
    let spot_boxes = layout_spots(&painter, view, &rect, &wf_rect, skimmer, digi_audio_hz.is_some());

    // --- interactions -----------------------------------------------------
    // Model: grabbing a filter edge (left button, spectrum strip) always
    // wins and only moves the edge. Otherwise: left-drag pans the view AND
    // drags the tuning with it; right-drag pans the view only.
    let vfo_hz = state.rx_freq_hz();
    let px0 = view.freq_to_x(vfo_hz + state.rx[0].filter_lo as f64, &rect);
    let px1 = view.freq_to_x(vfo_hz + state.rx[0].filter_hi as f64, &rect);

    let edge_at = |p: egui::Pos2| -> Option<u8> {
        // Grabbable in either the spectrum or the waterfall strip, so the edges
        // can still be dragged when the spectrum line is collapsed.
        if !(spec_rect.contains(p) || wf_rect.contains(p)) {
            return None;
        }
        if (p.x - px0).abs() < EDGE_GRAB_PX {
            Some(0u8)
        } else if (p.x - px1).abs() < EDGE_GRAB_PX {
            Some(1u8)
        } else {
            None
        }
    };

    let edge_id = ui.id().with("pb-edge");
    let mut edge: Option<u8> = ui.data(|d| d.get_temp(edge_id)).unwrap_or(None);
    let hover_edge = resp.hover_pos().and_then(edge_at);

    // The frequency-scale strip doubles as the spectrum/waterfall resize grip:
    // a vertical drag there changes the spectrum height.
    let resize_id = ui.id().with("spec-resize");
    let mut resizing: bool = ui.data(|d| d.get_temp(resize_id)).unwrap_or(false);
    let hover_resize = resp.hover_pos().map(|p| scale_rect.contains(p)).unwrap_or(false);

    if resp.drag_started_by(egui::PointerButton::Primary) {
        // Decide from the PRESS position, not the current pointer position —
        // by the time the drag threshold trips, the pointer may already have
        // left the grab zone.
        let origin = ui.input(|i| i.pointer.press_origin());
        edge = origin.and_then(edge_at);
        resizing = edge.is_none() && origin.map(|p| scale_rect.contains(p)).unwrap_or(false);
        ui.data_mut(|d| {
            d.insert_temp(edge_id, edge);
            d.insert_temp(resize_id, resizing);
        });
    }
    if resp.drag_stopped() {
        edge = None;
        resizing = false;
        ui.data_mut(|d| {
            d.insert_temp(edge_id, edge);
            d.insert_temp(resize_id, resizing);
        });
    }
    if hover_edge.is_some() || edge.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    } else if hover_resize || resizing {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
    }

    // Secondary-button panning is tracked manually — egui only registers
    // widget drags started with the primary button.
    let sec_id = ui.id().with("sec-pan");
    let mut sec_pan: bool = ui.data(|d| d.get_temp(sec_id)).unwrap_or(false);
    let (sec_down, press_origin, pointer_delta) = ui.input(|i| {
        (i.pointer.secondary_down(), i.pointer.press_origin(), i.pointer.delta())
    });
    let sec_pan_before = sec_pan;
    if sec_down && !sec_pan && press_origin.is_some_and(|p| rect.contains(p)) {
        sec_pan = true;
    }
    if !sec_down {
        sec_pan = false;
    }
    if sec_pan != sec_pan_before {
        ui.data_mut(|d| d.insert_temp(sec_id, sec_pan));
    }

    if resizing && resp.dragged_by(egui::PointerButton::Primary) {
        // Spectrum/waterfall resize — set the spectrum height from the pointer.
        if let Some(p) = resp.interact_pointer_pos() {
            let usable = (rect.height() - SCALE_H).max(1.0);
            view.spectrum_fraction = ((p.y - rect.top()) / usable).clamp(0.10, 0.85);
            view.spectrum_collapsed = false; // dragging it open implies visible
        }
    } else if let (Some(e), true) = (edge, resp.dragged_by(egui::PointerButton::Primary)) {
        // Filter edge drag — exclusive, never pans.
        if let Some(p) = resp.interact_pointer_pos() {
            let rel = view.x_to_freq(p.x, &rect) - vfo_hz;
            let max_hz = state.rx[0].mode.max_filter_hz() as f64;
            let rx0 = &mut state.rx[0];
            let (mut lo, mut hi) = (rx0.filter_lo as f64, rx0.filter_hi as f64);
            if e == 0 {
                lo = rel.clamp(-max_hz, hi - 50.0);
            } else {
                hi = rel.clamp(lo + 50.0, max_hz);
            }
            // Optimistic echo so the grip tracks the pointer exactly.
            (rx0.filter_lo, rx0.filter_hi) = (lo as f32, hi as f32);
            cmds.push(Command::SetFilter { rx: RxId::Main, lo: lo as f32, hi: hi as f32 });
        }
    } else if sec_pan {
        // Right-drag: pan only, grab-the-content semantics.
        let dhz = -pointer_delta.x as f64 * view.span() / rect.width() as f64;
        view.view_lo_hz += dhz;
        view.view_hi_hz += dhz;
    } else if resp.dragged_by(egui::PointerButton::Primary) {
        // Left-drag: grab the spectrum and slide it — content follows the
        // mouse, and the tuning follows the content (dragging right tunes
        // down). The view pans along so the VFO marker keeps its place.
        let dhz = -resp.drag_delta().x as f64 * view.span() / rect.width() as f64;
        view.view_lo_hz += dhz;
        view.view_hi_hz += dhz;
        let hz = (state.active_freq_hz() + dhz).max(0.0);
        match state.active_vfo {
            Vfo::A => state.vfo_a_hz = hz, // optimistic echo
            Vfo::B => state.vfo_b_hz = hz,
        }
        cmds.push(Command::SetVfo { vfo: state.active_vfo, hz });
    } else {
        // --- zoom / click-tune --------------------------------------------
        if let Some(pos) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                // Zoom around the cursor; scroll up = zoom in.
                let factor = 0.998f64.powf(scroll as f64 * 2.0);
                let fpos = view.x_to_freq(pos.x, &rect);
                let lo = fpos - (fpos - view.view_lo_hz) * factor;
                let hi = fpos + (view.view_hi_hz - fpos) * factor;
                let min_span = (dev_span / 1024.0).max(1_000.0);
                if hi - lo >= min_span {
                    view.view_lo_hz = lo;
                    view.view_hi_hz = hi;
                }
            }
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some(sb) = spot_boxes.iter().find(|b| b.rect.contains(pos)) {
                    let spot_hz = skimmer[sb.idx].freq_hz;
                    if digi_audio_hz.is_some() {
                        // FT8 station box: set the audio TX offset to it.
                        let audio = (spot_hz - state.rx_freq_hz()) as f32;
                        cmds.push(Command::SetDigiAudioFreq(audio.clamp(200.0, 3500.0)));
                    } else {
                        // CW skimmer box: put the dial a sidetone-pitch below the
                        // signal so it lands inside the CW filter, and switch to
                        // CW. (CW is USB-side; the passband is centred on ~700 Hz.)
                        let (lo, hi) = Mode::Cw.default_filter();
                        let pitch = ((lo + hi) * 0.5) as f64;
                        cmds.push(Command::SetVfo { vfo: state.active_vfo, hz: spot_hz - pitch });
                        cmds.push(Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
                    }
                } else if digi_audio_hz.is_some() {
                    // Digital mode: set the audio TX offset, not the VFO.
                    let audio = (view.x_to_freq(pos.x, &rect) - state.rx_freq_hz()) as f32;
                    cmds.push(Command::SetDigiAudioFreq(audio.clamp(200.0, 3500.0)));
                } else {
                    let clicked = view.x_to_freq(pos.x, &rect);
                    let hz = (clicked / CLICK_TUNE_STEP).round() * CLICK_TUNE_STEP;
                    let shift = ui.input(|i| i.modifiers.shift);
                    let vfo = if shift { Vfo::B } else { state.active_vfo };
                    cmds.push(Command::SetVfo { vfo, hz });
                }
            }
        }
    }
    view.clamp_to(dev_center, dev_span);

    // --- drawing ----------------------------------------------------------
    // Recompute after this frame's pan/tune/edge updates.
    let vfo_hz = state.rx_freq_hz();
    let px0 = view.freq_to_x(vfo_hz + state.rx[0].filter_lo as f64, &rect);
    let px1 = view.freq_to_x(vfo_hz + state.rx[0].filter_hi as f64, &rect);

    if spec_h > 1.0 {
        draw_grid(&painter, view, &spec_rect);
        if view.peak_hold {
            peaks.update(f);
            let key = trace_key(f, peaks.generation, view, &spec_rect);
            let pts = trace
                .hold
                .points_for(key, || compute_trace(view, f, Some(&peaks.bins), &spec_rect));
            painter.add(Shape::line(
                pts,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 220, 90, 170)),
            ));
        } else {
            peaks.clear();
        }
        let key = trace_key(f, 0, view, &spec_rect);
        let pts = trace.live.points_for(key, || compute_trace(view, f, None, &spec_rect));
        painter.add(Shape::line(pts, Stroke::new(1.0, Color32::from_rgb(120, 220, 255))));
    }
    draw_scale(&painter, view, &scale_rect);

    // Resize grip: a short centred handle in the scale strip, brightening when
    // hovered/dragged, so the divider is discoverable.
    {
        let cy = scale_rect.top() + 1.5;
        let cx = scale_rect.center().x;
        let col = if hover_resize || resizing {
            crate::theme::CYAN
        } else {
            Color32::from_gray(70)
        };
        for dx in [-16.0f32, 0.0, 16.0] {
            painter.line_segment(
                [pos2(cx + dx - 6.0, cy), pos2(cx + dx + 6.0, cy)],
                Stroke::new(2.0, col),
            );
        }
    }

    // Waterfall (GPU): viewport expressed in frame coordinates.
    let base = f.center_hz - f.span_hz / 2.0;
    let u_lo = ((view.view_lo_hz - base) / f.span_hz) as f32;
    let u_hi = ((view.view_hi_hz - base) / f.span_hz) as f32;
    ui.painter().add(crate::egui_wgpu::Callback::new_paint_callback(
        wf_rect,
        WaterfallCallback {
            // Arc::clone, not a bins deep-clone — keep it explicit.
            frame: Some(Arc::clone(f)),
            u_lo,
            u_hi,
            rows_visible: wf_rect.height(),
            lut: view.colormap,
            rows_to_write: wf.rows_to_write,
        },
    ));

    // Bandplan strip along the bottom of the waterfall (over the GPU layer).
    crate::widgets::bandplan::overlay(&painter, view, &wf_rect);

    // --- VFO markers + passband shading -----------------------------------
    let in_view = |hz: f64| (view.view_lo_hz..=view.view_hi_hz).contains(&hz);

    let (x0c, x1c) = (
        px0.clamp(rect.left(), rect.right()),
        px1.clamp(rect.left(), rect.right()),
    );
    if x1c > x0c {
        // Passband shading on the spectrum strip and (fainter) on the
        // waterfall, so the filter width stays visible when collapsed.
        if spec_h > 1.0 {
            painter.rect_filled(
                Rect::from_min_max(pos2(x0c, spec_rect.top()), pos2(x1c, spec_rect.bottom())),
                0.0,
                Color32::from_rgba_unmultiplied(255, 90, 90, 26),
            );
        }
        painter.rect_filled(
            Rect::from_min_max(pos2(x0c, wf_rect.top()), pos2(x1c, wf_rect.bottom())),
            0.0,
            Color32::from_rgba_unmultiplied(255, 90, 90, 16),
        );
    }
    // Edge grips (brighter when grabbable). Drawn on both strips so the
    // filter edges remain visible with the spectrum line collapsed.
    for (x, e) in [(px0, 0u8), (px1, 1u8)] {
        if rect.x_range().contains(x) {
            let hot = hover_edge == Some(e) || edge == Some(e);
            let w = if hot { 2.0 } else { 1.0 };
            if spec_h > 1.0 {
                let color =
                    if hot { Color32::from_rgb(255, 170, 90) } else { Color32::from_gray(90) };
                painter.vline(x, spec_rect.y_range(), Stroke::new(w, color));
            }
            let wf_color = if hot {
                Color32::from_rgba_unmultiplied(255, 170, 90, 200)
            } else {
                Color32::from_rgba_unmultiplied(150, 160, 170, 120)
            };
            painter.vline(x, wf_rect.y_range(), Stroke::new(w, wf_color));
        }
    }

    if in_view(vfo_hz) {
        let x = view.freq_to_x(vfo_hz, &rect);
        painter.vline(x, spec_rect.y_range(), Stroke::new(1.0, Color32::from_rgb(255, 60, 60)));
        painter.vline(
            x,
            wf_rect.y_range(),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 60, 60, 140)),
        );
    }

    // Digital-mode audio TX marker (cyan) at dial + audio_hz.
    if let Some(a) = digi_audio_hz {
        let hz = state.rx_freq_hz() + a as f64;
        if in_view(hz) {
            let x = view.freq_to_x(hz, &rect);
            painter.vline(x, spec_rect.y_range(), Stroke::new(1.5, crate::theme::CYAN));
            painter.vline(
                x,
                wf_rect.y_range(),
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(0, 208, 244, 160)),
            );
        }
    }
    // Inactive VFO marker (sub-RX listens here when enabled).
    let inactive_hz = match state.active_vfo {
        Vfo::A => state.vfo_b_hz,
        Vfo::B => state.vfo_a_hz,
    };
    if in_view(inactive_hz) {
        let x = view.freq_to_x(inactive_hz, &rect);
        let color = if state.sub_rx_enabled {
            Color32::from_rgb(255, 170, 40)
        } else {
            Color32::from_rgba_unmultiplied(255, 170, 40, 90)
        };
        painter.vline(x, spec_rect.y_range(), Stroke::new(1.0, color));
        painter.vline(
            x,
            wf_rect.y_range(),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 170, 40, 110)),
        );
    }

    // Skimmer spot boxes over the waterfall (staggered lanes, on top of the
    // markers so they stay readable and clickable). Each box sits to the right
    // of its signal: a faint vertical line marks the signal's centre and a
    // horizontal leader joins it to the box.
    let hover_pos = resp.hover_pos();
    for b in &spot_boxes {
        let spot = &skimmer[b.idx];
        let a = alpha.get(b.idx).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        let hovered = hover_pos.is_some_and(|p| b.rect.contains(p));
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let border = spot_color(spot, hovered);
        let cy = b.rect.center().y;
        // Vertical indicator over the signal centre (faint, full waterfall).
        let vcol = fade(
            Color32::from_rgba_unmultiplied(border.r(), border.g(), border.b(), if hovered { 170 } else { 90 }),
            a,
        );
        painter.vline(b.sig_x, wf_rect.y_range(), Stroke::new(1.0, vcol));
        // Horizontal leader from the signal to the box, with a junction node.
        painter.line_segment(
            [pos2(b.sig_x, cy), pos2(b.rect.left(), cy)],
            Stroke::new(1.0, fade(border, a)),
        );
        painter.circle_filled(pos2(b.sig_x, cy), 1.8, fade(border, a));
        draw_spot_box(&painter, b, spot, hovered, a, digi_audio_hz.is_some());
    }

    // --- 60-second time gridlines on the waterfall ------------------------
    // The newest row (top of the waterfall) is "now"; rows below are older at
    // `rows_per_sec` rows/second (≈ 1 row per pixel). Draw a faint gray line at
    // each whole UTC minute that falls in the visible window, labelled HH:MM.
    let rows_per_sec = wf.rows_per_sec as f64;
    if rows_per_sec > 0.01 && wf_rect.height() > 4.0 {
        let secs_per_px = 1.0 / rows_per_sec;
        let visible_secs = wf_rect.height() as f64 * secs_per_px;
        let now = wf.now_unix;
        let oldest = now - visible_secs;
        let grid = Color32::from_rgba_unmultiplied(200, 205, 215, 60);
        let mut t = (oldest / 60.0).ceil() * 60.0; // first minute boundary ≥ oldest
        while t <= now {
            let y = wf_rect.top() + ((now - t) * rows_per_sec) as f32;
            if (wf_rect.top()..=wf_rect.bottom()).contains(&y) {
                painter.hline(wf_rect.x_range(), y, Stroke::new(1.0, grid));
                let tod = (t as i64).rem_euclid(86_400);
                let text = format!("{:02}:{:02}", tod / 3600, (tod % 3600) / 60);
                label_box(&painter, pos2(wf_rect.left() + 2.0, y + 1.0), &text, Color32::from_gray(215), wf_rect);
            }
            t += 60.0;
        }
    }

    // --- cursor readouts (hover) ------------------------------------------
    // A hovered filter edge shows its offset from the VFO; otherwise hovering
    // the spectrum/waterfall shows a faint crosshair + the frequency a click
    // would tune to. Suppressed while dragging (pan/edge/resize).
    if let Some(p) = hover_pos {
        if let Some(e) = hover_edge {
            // Item 7: filter edge offset from the VFO.
            let off = if e == 0 { state.rx[0].filter_lo } else { state.rx[0].filter_hi };
            let edge_x = if e == 0 { px0 } else { px1 };
            let ytop = if spec_h > 1.0 { spec_rect.top() } else { wf_rect.top() };
            label_box(
                &painter,
                pos2(edge_x + 7.0, ytop + 3.0),
                &format!("{:+} Hz", off.round() as i64),
                Color32::from_rgb(255, 190, 120),
                rect,
            );
        } else if (spec_rect.contains(p) || wf_rect.contains(p))
            && edge.is_none()
            && !resizing
            && !resp.dragged()
            && !spot_boxes.iter().any(|b| b.rect.contains(p))
        {
            // Item 6: crosshair + click-tune frequency readout.
            let line = Color32::from_rgba_unmultiplied(185, 205, 225, 70);
            if spec_h > 1.0 {
                painter.vline(p.x, spec_rect.y_range(), Stroke::new(1.0, line));
            }
            painter.vline(p.x, wf_rect.y_range(), Stroke::new(1.0, line));
            let text = click_tune_label(view, state, &rect, p.x, digi_audio_hz);
            label_box(&painter, pos2(p.x + 8.0, p.y - 9.0), &text, Color32::WHITE, rect);
        }
    }

    // Chrome: pink cut-corner border + corner accents around the panadapter.
    crate::chrome::paint_cut_border(
        &painter,
        rect.shrink(0.8),
        crate::theme::PINK,
        crate::theme::BG_DEEP,
    );
    crate::chrome::corner_brackets(&painter, rect, crate::theme::PINK);
}

/// Draw `text` in a small semi-transparent black box, clamped inside `bounds`.
fn label_box(p: &egui::Painter, top_left: Pos2, text: &str, fg: Color32, bounds: Rect) {
    let galley = p.layout_no_wrap(text.to_string(), FontId::monospace(11.0), fg);
    let pad = vec2(4.0, 2.0);
    let size = galley.size() + pad * 2.0;
    let x = (top_left.x).min(bounds.right() - size.x).max(bounds.left());
    let y = (top_left.y).clamp(bounds.top(), bounds.bottom() - size.y);
    let bg = Rect::from_min_size(pos2(x, y), size);
    p.rect_filled(bg, 2.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180));
    p.galley(pos2(x + pad.x, y + pad.y), galley, fg);
}

/// The value a left-click at screen `x` would set: the audio TX offset in a
/// digital mode, else the (10 Hz-rounded) dial frequency.
fn click_tune_label(
    view: &ViewState,
    state: &RadioState,
    rect: &Rect,
    x: f32,
    digi_audio_hz: Option<f32>,
) -> String {
    let hz = view.x_to_freq(x, rect);
    if digi_audio_hz.is_some() {
        let audio = (hz - state.rx_freq_hz()).clamp(200.0, 3500.0);
        format!("{audio:.0} Hz")
    } else {
        let tuned = (hz / CLICK_TUNE_STEP).round() * CLICK_TUNE_STEP;
        format!("{:.5} MHz", tuned / 1e6)
    }
}

/// Per-pixel polyline of the frame's bins (or of `values` when given, e.g.
/// the peak-hold bins) mapped through the current viewport. This is the
/// expensive path the [`TraceCache`] avoids on unchanged repaints.
fn compute_trace(
    view: &ViewState,
    f: &SpectrumFrame,
    values: Option<&[f32]>,
    rect: &Rect,
) -> Vec<Pos2> {
    let n = values.map(|v| v.len()).unwrap_or(f.bins.len());
    if n == 0 {
        return Vec::new();
    }
    let base = f.center_hz - f.span_hz / 2.0;
    let w = rect.width().max(1.0) as usize;
    let mut points = Vec::with_capacity(w);
    for px in 0..w {
        let x = rect.left() + px as f32;
        let hz = view.x_to_freq(x, rect);
        let bin_f = (hz - base) / f.span_hz * n as f64;
        let v = if (0.0..n as f64).contains(&bin_f) {
            let i = bin_f as usize;
            let raw = match values {
                Some(v) => v[i],
                None => f.bins[i] as f32,
            };
            raw / 255.0
        } else {
            0.0
        };
        points.push(Pos2::new(x, rect.bottom() - v * rect.height()));
    }
    points
}

fn draw_grid(painter: &egui::Painter, view: &ViewState, rect: &Rect) {
    let grid = Stroke::new(0.5, Color32::from_gray(42));
    // dB lines every 20 dB of the display range.
    let range = view.db_ceil - view.db_floor;
    if range > 1.0 {
        let mut db = (view.db_floor / 20.0).ceil() * 20.0;
        while db < view.db_ceil {
            let frac = (db - view.db_floor) / range;
            let y = rect.bottom() - frac * rect.height();
            painter.hline(rect.x_range(), y, grid);
            painter.text(
                pos2(rect.left() + 2.0, y - 1.0),
                Align2::LEFT_BOTTOM,
                format!("{db:.0}"),
                FontId::monospace(9.0),
                Color32::from_gray(110),
            );
            db += 20.0;
        }
    }
    for hz in freq_gridlines(view) {
        let x = view.freq_to_x(hz, rect);
        painter.vline(x, rect.y_range(), grid);
    }
}

fn draw_scale(painter: &egui::Painter, view: &ViewState, rect: &Rect) {
    painter.rect_filled(*rect, 0.0, Color32::from_gray(20));
    for hz in freq_gridlines(view) {
        let x = view.freq_to_x(hz, rect);
        painter.vline(
            x,
            egui::Rangef::new(rect.top(), rect.top() + 5.0),
            Stroke::new(1.0, Color32::from_gray(120)),
        );
        painter.text(
            pos2(x, rect.center().y + 2.0),
            Align2::CENTER_CENTER,
            format!("{:.4}", hz / 1e6),
            FontId::monospace(10.0),
            Color32::from_gray(190),
        );
    }
}

/// Gridline frequencies at a 1/2/5·10^k step giving ~5–10 lines.
fn freq_gridlines(view: &ViewState) -> Vec<f64> {
    let span = view.span();
    let raw = span / 8.0;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * mag)
        .find(|&s| s >= raw)
        .unwrap_or(mag * 10.0);
    let mut out = Vec::new();
    let mut hz = (view.view_lo_hz / step).ceil() * step;
    while hz <= view.view_hi_hz {
        out.push(hz);
        hz += step;
    }
    out
}
