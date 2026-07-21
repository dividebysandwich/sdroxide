//! Cyberpunk chrome: cut-corner panel frames, angled chip buttons, and
//! corner accents — the shapes egui's rounded-rect widgets can't draw.

use eframe::egui::{
    self, Color32, FontSelection, Painter, Pos2, Rect, Response, RichText, Sense, Shape, Stroke,
    TextStyle, Ui, WidgetText, pos2, vec2,
};

use crate::theme;

/// Corner cut size for panel frames.
const FRAME_CUT: f32 = 10.0;
/// Corner cut size for chip buttons.
const CHIP_CUT: f32 = 5.0;
/// Fixed module height. Must exceed the tallest content (caption + a combo
/// or slider row + margins) so every module ends up exactly this tall — then
/// they line up regardless of the row's cross-axis alignment.
pub const MODULE_H: f32 = 58.0;

/// A panel with a pink border and cut corners (top-right + bottom-left),
/// sitting on the darker page background.
pub fn angled_frame<R>(ui: &mut Ui, accent: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // A Frame measures its content with UNBOUNDED width to auto-size, and
    // `horizontal_wrapped` inside that pass never wraps (nothing to wrap
    // against). Capture the panel's real width here, before the frame, and
    // pin the content to it so wrapping happens at the visible edge.
    let avail = {
        let a = ui.available_width();
        if a.is_finite() && a > 50.0 { a } else { ui.ctx().content_rect().width() - 24.0 }
    };
    let margin = 10i8;
    let inner = egui::Frame::new()
        .fill(theme::PANEL)
        .inner_margin(egui::Margin::symmetric(margin, 8))
        .show(ui, |ui| {
            // Pin to the panel width (both min and max) so wrapping happens at
            // the visible edge AND the frame — and its cut-corner border — spans
            // the full width even when the last row of content is short.
            let w = (avail - 2.0 * margin as f32).max(120.0);
            ui.set_min_width(w);
            ui.set_max_width(w);
            add(ui)
        });
    paint_cut_border(ui.painter(), inner.response.rect, accent, theme::BG_DEEP);
    inner.inner
}

/// Frame for a floating window: flat panel fill, square corners (the cut
/// corners are painted on top afterwards by [`paint_window_border`]), with a
/// roomy content margin.
pub fn window_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::PANEL)
        .inner_margin(egui::Margin::same(11))
        .corner_radius(egui::CornerRadius::ZERO)
}

/// Paint the pink cut-corner border around a floating window (top-right +
/// bottom-left bevels), matching the main panadapter chrome. Draws on the
/// window's own layer so it sits over the panel fill.
pub fn paint_window_border(ctx: &egui::Context, resp: &Response) {
    let p = ctx.layer_painter(resp.layer_id);
    paint_cut_border(&p, resp.rect, theme::PINK, theme::PANEL);
}

/// Cut-corner border: masks the two corners with `mask` (the surrounding
/// background) and strokes the six-sided outline.
pub fn paint_cut_border(p: &Painter, rect: Rect, color: Color32, mask: Color32) {
    let cut = FRAME_CUT.min(rect.height() * 0.4);
    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());

    // Mask the square corners so the cut reads as a real bevel.
    p.add(Shape::convex_polygon(
        vec![pos2(r - cut, t), pos2(r, t), pos2(r, t + cut)],
        mask,
        Stroke::NONE,
    ));
    p.add(Shape::convex_polygon(
        vec![pos2(l, b - cut), pos2(l + cut, b), pos2(l, b)],
        mask,
        Stroke::NONE,
    ));

    let outline = vec![
        pos2(l, t),
        pos2(r - cut, t),
        pos2(r, t + cut),
        pos2(r, b),
        pos2(l + cut, b),
        pos2(l, b - cut),
    ];
    p.add(Shape::closed_line(outline, Stroke::new(1.2, color)));
}

/// A captioned control module of fixed `width`: a bordered box with a small
/// cyan uppercase label above a row of controls.
///
/// Uses `allocate_ui_with_layout` so the fixed width is reserved *before*
/// the content is drawn — that lets a `horizontal_wrapped` parent wrap the
/// whole module to the next row cleanly (a plain `Frame` instead shrinks
/// into whatever sliver is left, which is the wrong behavior here).
pub fn module<R>(ui: &mut Ui, caption: &str, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // Fixed height too: a bare (w, 0) allocation lets the top-down layout
    // over-reserve vertical space, leaving big gaps between wrapped rows.
    ui.allocate_ui_with_layout(
        egui::vec2(width, MODULE_H),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            egui::Frame::new()
                .fill(theme::FILL)
                .stroke(Stroke::new(1.0, theme::LINE_LIT))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(width - 16.0);
                    // Fill the full module height so every box — captioned or
                    // bare — ends up exactly MODULE_H tall.
                    ui.set_min_height(MODULE_H - 10.0);
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.label(
                        RichText::new(caption.to_uppercase())
                            .color(theme::CYAN_DIM)
                            .size(9.5)
                            .strong(),
                    );
                    // Top-align the control row. egui's ComboBox positions its
                    // button from `available_rect_before_wrap().top()` and so
                    // ignores vertical centering, unlike chips and drag-values;
                    // top-aligning everything keeps them all on one baseline.
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.set_min_height(24.0);
                        add(ui)
                    })
                    .inner
                })
                .inner
        },
    )
    .inner
}

/// Like [`module`] but with no caption — the content fills the full box height
/// (vertically centred). Used for the frequency readout and S-meter, where the
/// label would only waste space.
pub fn module_bare<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        egui::vec2(width, MODULE_H),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            egui::Frame::new()
                .fill(theme::FILL)
                .stroke(Stroke::new(1.0, theme::LINE_LIT))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(width - 16.0);
                    ui.set_min_height(MODULE_H - 10.0);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_min_height(MODULE_H - 10.0);
                        add(ui)
                    })
                    .inner
                })
                .inner
        },
    )
    .inner
}

/// Small L-shaped corner accents (page decoration, reference-style).
pub fn corner_brackets(p: &Painter, rect: Rect, color: Color32) {
    let len = 16.0;
    let s = Stroke::new(2.0, color);
    let r = rect.shrink(3.0);
    // top-left
    p.line_segment([r.left_top(), r.left_top() + vec2(len, 0.0)], s);
    p.line_segment([r.left_top(), r.left_top() + vec2(0.0, len)], s);
    // bottom-right
    p.line_segment([r.right_bottom(), r.right_bottom() - vec2(len, 0.0)], s);
    p.line_segment([r.right_bottom(), r.right_bottom() - vec2(0.0, len)], s);
}

/// A red-bordered content box (cyberpunk section panel). Fills the available
/// width and draws a red left-accent bar.
pub fn red_panel<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let inner = egui::Frame::new()
        .fill(theme::ROW_BG)
        .stroke(Stroke::new(1.0, theme::RED_DEEP))
        .inner_margin(egui::Margin { left: 9, right: 7, top: 6, bottom: 6 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    // Red left-accent bar.
    let r = inner.response.rect;
    ui.painter().rect_filled(
        Rect::from_min_max(r.left_top(), pos2(r.left() + 2.5, r.bottom())),
        0.0,
        theme::PINK,
    );
    inner.inner
}

/// A slider with a visible dark track. egui draws the slider rail with
/// `widgets.inactive.bg_fill`, which equals the module background here, so
/// the empty portion of the track would otherwise be invisible.
pub fn slider(ui: &mut Ui, slider: egui::Slider<'_>) -> Response {
    ui.scope(|ui| {
        ui.visuals_mut().widgets.inactive.bg_fill = theme::INPUT_BG;
        ui.visuals_mut().widgets.hovered.bg_fill = theme::INPUT_BG;
        ui.spacing_mut().slider_rail_height = 6.0;
        ui.add(slider)
    })
    .inner
}

/// Angled chip: a selectable button with a cut bottom-right corner.
/// Selected chips fill cyan with dark ink, like the reference nav pills.
pub fn chip(ui: &mut Ui, selected: bool, text: impl Into<RichText>) -> Response {
    chip_impl(ui, selected, text.into(), None)
}

/// Chip with an explicit accent fill when selected (e.g. PTT red).
pub fn chip_accent(
    ui: &mut Ui,
    selected: bool,
    text: impl Into<RichText>,
    fill: Color32,
    ink: Color32,
) -> Response {
    chip_impl(ui, selected, text.into(), Some((fill, ink)))
}

fn chip_impl(
    ui: &mut Ui,
    selected: bool,
    text: RichText,
    accent: Option<(Color32, Color32)>,
) -> Response {
    let galley = WidgetText::from(text).into_galley(
        ui,
        None,
        f32::INFINITY,
        FontSelection::Style(TextStyle::Button),
    );
    let padding = vec2(9.0, 4.0);
    let size = galley.size() + padding * 2.0;
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let v = ui.style().interact_selectable(&resp, selected);
        let cut = CHIP_CUT.min(size.y * 0.35);
        let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
        let outline = vec![
            pos2(l, t),
            pos2(r, t),
            pos2(r, b - cut),
            pos2(r - cut, b),
            pos2(l, b),
        ];

        let (fill, stroke, ink) = if selected {
            let (fill, ink) = accent.unwrap_or((theme::CYAN, theme::INK_ON_CYAN));
            (fill, Stroke::new(1.0, fill), ink)
        } else {
            (v.bg_fill, v.bg_stroke, v.fg_stroke.color)
        };
        ui.painter()
            .add(Shape::convex_polygon(outline, fill, stroke));

        let text_pos = Pos2 {
            x: rect.center().x - galley.size().x / 2.0,
            y: rect.center().y - galley.size().y / 2.0,
        };
        ui.painter().galley(text_pos, galley, ink);
    }
    resp
}
