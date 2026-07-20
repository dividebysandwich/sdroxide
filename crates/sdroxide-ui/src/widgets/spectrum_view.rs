//! The panadapter: spectrum line + frequency scale + GPU waterfall, with
//! drag-pan, wheel-zoom (around the cursor), click-to-tune (shift-click
//! tunes VFO B), and draggable passband filter edges.

use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2,
};
use sdroxide_types::{Command, RadioState, RxId, SpectrumFrame, Vfo};

use crate::view::ViewState;
use crate::waterfall_gpu::WaterfallCallback;

const SCALE_H: f32 = 18.0;
/// Tuning rounds to this step on click-tune.
const CLICK_TUNE_STEP: f64 = 10.0;
/// Pixel distance for grabbing a filter edge.
const EDGE_GRAB_PX: f32 = 6.0;

/// Decaying peak-hold trace, reset whenever the frame mapping changes.
#[derive(Default)]
pub struct PeakHold {
    bins: Vec<f32>,
    center: f64,
    span: f64,
}

/// dB-scale u8 units per frame (~3.5 dB/s at 30 fps over a 100 dB range).
const PEAK_DECAY: f32 = 0.3;

impl PeakHold {
    fn update(&mut self, f: &SpectrumFrame) {
        let mapping_changed = self.bins.len() != f.bins.len()
            || (self.center - f.center_hz).abs() > f.span_hz * 1e-6
            || (self.span - f.span_hz).abs() > f.span_hz * 1e-6;
        if mapping_changed {
            self.center = f.center_hz;
            self.span = f.span_hz;
            self.bins = f.bins.iter().map(|&b| b as f32).collect();
            return;
        }
        for (p, &b) in self.bins.iter_mut().zip(&f.bins) {
            *p = (b as f32).max(*p - PEAK_DECAY);
        }
    }
}

pub fn show(
    ui: &mut Ui,
    view: &mut ViewState,
    state: &mut RadioState,
    frame: Option<&SpectrumFrame>,
    peaks: &mut PeakHold,
    cmds: &mut Vec<Command>,
) {
    show_ext(ui, view, state, frame, peaks, None, cmds);
}

/// `show` with an optional digital-mode audio marker. When `digi_audio_hz`
/// is `Some`, left-click sets the FT8/FT4 audio TX frequency instead of the
/// VFO, and a marker is drawn at `dial + audio_hz`.
pub fn show_ext(
    ui: &mut Ui,
    view: &mut ViewState,
    state: &mut RadioState,
    frame: Option<&SpectrumFrame>,
    peaks: &mut PeakHold,
    digi_audio_hz: Option<f32>,
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
                let clicked = view.x_to_freq(pos.x, &rect);
                if digi_audio_hz.is_some() {
                    // Digital mode: set the audio TX offset, not the VFO.
                    let audio = (clicked - state.rx_freq_hz()) as f32;
                    cmds.push(Command::SetDigiAudioFreq(audio.clamp(200.0, 3500.0)));
                } else {
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
            draw_trace(
                &painter,
                view,
                f,
                &peaks.bins,
                &spec_rect,
                Color32::from_rgba_unmultiplied(255, 220, 90, 170),
            );
        } else {
            peaks.bins.clear();
        }
        draw_spectrum_line(&painter, view, f, &spec_rect);
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
            frame: Some(f.clone()),
            u_lo,
            u_hi,
            rows_visible: wf_rect.height(),
            lut: view.colormap,
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

    // Chrome: pink cut-corner border + corner accents around the panadapter.
    crate::chrome::paint_cut_border(
        &painter,
        rect.shrink(0.8),
        crate::theme::PINK,
        crate::theme::BG_DEEP,
    );
    crate::chrome::corner_brackets(&painter, rect, crate::theme::PINK);
}

fn draw_spectrum_line(
    painter: &egui::Painter,
    view: &ViewState,
    f: &SpectrumFrame,
    rect: &Rect,
) {
    let values: Vec<f32> = f.bins.iter().map(|&b| b as f32).collect();
    draw_trace(painter, view, f, &values, rect, Color32::from_rgb(120, 220, 255));
}

/// Polyline of `values` (u8-scale) mapped through the current viewport.
fn draw_trace(
    painter: &egui::Painter,
    view: &ViewState,
    f: &SpectrumFrame,
    values: &[f32],
    rect: &Rect,
    color: Color32,
) {
    let n = values.len();
    if n == 0 {
        return;
    }
    let base = f.center_hz - f.span_hz / 2.0;
    let w = rect.width().max(1.0) as usize;
    let mut points = Vec::with_capacity(w);
    for px in 0..w {
        let x = rect.left() + px as f32;
        let hz = view.x_to_freq(x, rect);
        let bin_f = (hz - base) / f.span_hz * n as f64;
        let v = if (0.0..n as f64).contains(&bin_f) {
            values[bin_f as usize] / 255.0
        } else {
            0.0
        };
        points.push(Pos2::new(x, rect.bottom() - v * rect.height()));
    }
    painter.add(Shape::line(points, Stroke::new(1.0, color)));
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
