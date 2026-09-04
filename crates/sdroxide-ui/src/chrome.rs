//! Cyberpunk chrome: cut-corner panel frames, angled chip buttons, and
//! corner accents — the shapes egui's rounded-rect widgets can't draw.

use eframe::egui::{
    self, Color32, CornerRadius, FontSelection, Mesh, Painter, Pos2, Rect, Response, RichText,
    Sense, Shape, Stroke, StrokeKind, TextStyle, Ui, WidgetText, layers::ShapeIdx, pos2, vec2,
};
use sdroxide_types::ChromeStyle;

use crate::theme::{self, ThemedScroll};

/// Corner cut size for panel frames.
const FRAME_CUT: f32 = 10.0;
/// Corner cut size for chip buttons.
const CHIP_CUT: f32 = 5.0;
/// The one module height: every box on the strip is this tall, whether its
/// content is one centred row (frequency readout, S-meter) or two stacked
/// control rows (VFO/RIT, RX, TX, Display, System) — so a wrapped strip's rows
/// always line up regardless of which boxes landed on them.
pub const MODULE_TALL_H: f32 = 72.0;
/// The hairline border a module box wears. egui draws a [`egui::Frame`]'s
/// stroke outside the content it was given, so a module built to `height`
/// stands `height + 2 * MODULE_BORDER` tall overall — which is the figure a box
/// that paints its own border has to match. See [`module_bare_flush_h`].
pub const MODULE_BORDER: f32 = 1.0;

/// A panel with a pink border and cut corners (top-right + bottom-left),
/// sitting on the darker page background.
pub fn angled_frame<R>(ui: &mut Ui, accent: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // A Frame measures its content with UNBOUNDED width to auto-size, and
    // `horizontal_wrapped` inside that pass never wraps (nothing to wrap
    // against). Capture the panel's real width here, before the frame, and
    // pin the content to it so wrapping happens at the visible edge.
    let avail = {
        // Never wider than the window itself, whatever the parent reports. A
        // `Frame`'s outer rect is its content plus its margins, so a child that
        // took all the width there was leaves the parent expanded past the
        // screen edge — and every row measured against *that* wraps too late
        // and has whatever crossed the edge clipped away.
        let a = ui.available_width().min(ui.ctx().content_rect().width());
        if a.is_finite() && a > 50.0 { a } else { ui.ctx().content_rect().width() - 24.0 }
    };
    let margin = 10i8;
    // The Gradient style needs a fill sized to the frame's final rect, which
    // is only known after the content ran — so reserve a slot in the paint
    // list now, leave the frame's own fill transparent, and set the gradient
    // into the slot afterwards, where it renders *under* the content.
    let grad_slot =
        (theme::window_style() == ChromeStyle::Gradient).then(|| ui.painter().add(Shape::Noop));
    let inner = egui::Frame::new()
        .fill(if grad_slot.is_some() { Color32::TRANSPARENT } else { theme::PANEL() })
        .corner_radius(window_corner_radius())
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
    if let Some(slot) = grad_slot {
        ui.painter().set(slot, panel_gradient(inner.response.rect));
    }
    paint_cut_border(ui.painter(), inner.response.rect, accent, theme::BG_DEEP());
    inner.inner
}

/// The Gradient window style's fill for `rect`: the panel colour, lit a touch
/// at the top and falling off dark at the bottom.
fn panel_gradient(rect: Rect) -> Shape {
    let panel = theme::PANEL();
    // The light falls from above either way. On a dark panel the fall-off has
    // most of the range to work in; on a white one the same 45% would take the
    // bottom of every panel down to mid-grey, so it is a tenth of that.
    let shade = if theme::is_light() { 0.10 } else { 0.45 };
    grad_rect(rect, lerp(panel, Color32::WHITE, 0.06), lerp(panel, Color32::BLACK, shade))
}

/// Restyle a floating window's body for the current window style — call it
/// first thing inside the body closure of every `egui::Window` framed with
/// [`window_frame`].
///
/// A no-op for most styles. For Gradient it paints the body gradient (an
/// `egui::Frame` cannot fill with one, and a window's frame belongs to egui,
/// so the body's own first shape is the only slot under the widgets we can
/// reach); Bevel gets a subtle sheen along the top edge. Sized from the
/// body's `max_rect` grown back over [`window_frame`]'s 11 pt margin — on the
/// frame a window is resized its rect lags one frame behind, which egui's own
/// window-size memory makes invisible in practice.
pub fn window_body_bg(ui: &mut Ui) {
    let r = ui.max_rect().expand(11.0);
    match theme::window_style() {
        ChromeStyle::Gradient => {
            ui.painter().add(panel_gradient(r));
        }
        ChromeStyle::Bevel => {
            let mut sheen = r;
            sheen.set_height((r.height() * 0.2).min(26.0));
            ui.painter().add(grad_rect(sheen, Color32::from_white_alpha(14), Color32::TRANSPARENT));
        }
        _ => {}
    }
}

/// Frame for a floating window: flat panel fill with a roomy content margin,
/// shaped by the current window style — square for most styles (the Angled
/// cuts are painted on top afterwards by [`paint_window_border`]), rounded
/// under the Rounded style.
pub fn window_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::PANEL())
        .inner_margin(egui::Margin::same(11))
        .corner_radius(window_corner_radius())
}

/// Paint the pink cut-corner border around a floating window (top-right +
/// bottom-left bevels), matching the main panadapter chrome. Draws on the
/// window's own layer so it sits over the panel fill.
pub fn paint_window_border(ctx: &egui::Context, resp: &Response) {
    let p = ctx.layer_painter(resp.layer_id);
    paint_cut_border(&p, resp.rect, theme::PINK(), theme::PANEL());
}

/// Multiply a colour's alpha by `a` (for fading chrome in/out).
fn fade(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * a.clamp(0.0, 1.0)) as u8)
}

/// Linear blend from `a` to `b`, all four channels. Colours are premultiplied,
/// where a straight per-channel lerp is the correct blend.
fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_premultiplied(
        ch(a.r(), b.r()),
        ch(a.g(), b.g()),
        ch(a.b(), b.b()),
        ch(a.a(), b.a()),
    )
}

/// A rect filled with a vertical `top` → `bottom` gradient — the fill the
/// Gradient chrome style uses. egui has no gradient shape of its own, so this
/// is a two-triangle mesh with the colours on the vertices.
fn grad_rect(rect: Rect, top: Color32, bottom: Color32) -> Shape {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}

/// The corner radius a Rounded-style window wears. Matches what
/// [`crate::theme::apply_visuals`] gives egui's own windows and menus.
const ROUND_WINDOW_R: u8 = 6;
/// The corner radius a Rounded-style chip wears.
const ROUND_CHIP_R: f32 = 5.0;

/// The corner radius for the current window style — rounded only under
/// `Rounded`; every other style keeps square fills (Angled paints its cuts on
/// top).
fn window_corner_radius() -> CornerRadius {
    match theme::window_style() {
        ChromeStyle::Rounded => CornerRadius::same(ROUND_WINDOW_R),
        _ => CornerRadius::ZERO,
    }
}

/// A floating-window frame (flat panel, square corners) with its fill faded to
/// `alpha` — pair with [`paint_popup_cut_border`] for a fading popup.
pub fn window_frame_alpha(alpha: f32) -> egui::Frame {
    let mut f = window_frame();
    f.fill = fade(f.fill, alpha);
    f
}

/// Paint the pink top-right/bottom-left cut border of a popup, faded to `alpha`.
pub fn paint_popup_cut_border(ctx: &egui::Context, resp: &Response, alpha: f32) {
    let p = ctx.layer_painter(resp.layer_id);
    paint_cut_border(&p, resp.rect, fade(theme::PINK(), alpha), fade(theme::PANEL(), alpha));
}

/// Fade timing for an auto-dismissing popup: full opacity for `HOLD` seconds
/// after it opens, then a linear fade over `FADE` seconds, then it closes.
/// `since` (caller-owned, one per popup) remembers when it opened. Returns the
/// current opacity; apply it to the frame ([`window_frame_alpha`]), the content
/// (`ui.set_opacity`), and the border ([`paint_popup_cut_border`]).
pub fn popup_fade_alpha(
    ctx: &egui::Context,
    popup_id: egui::Id,
    now: f64,
    since: &mut Option<f64>,
) -> f32 {
    const HOLD: f64 = 5.0;
    const FADE: f64 = 3.0;
    // A touched screen has no hover to hold a popup open with — the browser
    // reports the pointer gone the instant a finger lifts — so the fade would
    // take a popup away while it was being read. There it closes on a tap
    // outside, and on nothing else.
    if crate::layout::tier(ctx).touch() {
        *since = None;
        return 1.0;
    }
    if !egui::Popup::is_id_open(ctx, popup_id) {
        *since = None;
        return 1.0;
    }
    let t0 = *since.get_or_insert(now);
    let elapsed = now - t0;
    if elapsed >= HOLD + FADE {
        egui::Popup::close_id(ctx, popup_id);
        *since = None;
        0.0
    } else if elapsed > HOLD {
        crate::repaint::animate(ctx); // animate the fade
        (1.0 - (elapsed - HOLD) / FADE) as f32
    } else {
        // Wake up exactly when the fade should begin.
        crate::repaint::after(ctx, std::time::Duration::from_secs_f64((HOLD - elapsed).max(0.001)));
        1.0
    }
}

/// The border every panel and floating window wears, in the current window
/// style. Kept under its historic name — it *was* only the cut-corner border,
/// and its ~30 callers all still mean "give this rect the app's frame chrome".
///
/// `mask` is the surrounding background, used only by the Angled style to
/// cover the square fill corners so the cut reads as a real bevel; the other
/// styles get their shape from the frame fill itself.
pub fn paint_cut_border(p: &Painter, rect: Rect, color: Color32, mask: Color32) {
    match theme::window_style() {
        ChromeStyle::Angled => paint_angled_border(p, rect, color, mask),
        ChromeStyle::Rectangular | ChromeStyle::Gradient => {
            p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.2, color), StrokeKind::Inside);
        }
        ChromeStyle::Terminal => {
            // The whole of what an edge is on a character display: a run of
            // `-` along the top and bottom, a column of `|` down each side,
            // and a `+` where they meet. Laid on the edge itself rather than
            // inside it, so the border sits where a stroke would have and the
            // panel's own margin still holds its content clear of it.
            for shape in ascii_box(p, rect, color, terminal_size(p), Sides::All) {
                p.add(shape);
            }
        }
        ChromeStyle::Rounded => {
            p.rect_stroke(
                rect,
                CornerRadius::same(ROUND_WINDOW_R),
                Stroke::new(1.2, color),
                StrokeKind::Inside,
            );
        }
        ChromeStyle::Bevel => {
            // Raised 3D edge: lit where the light falls (top + left), shaded
            // opposite. The hairline accent outline stays so the frame keeps
            // its colour identity from across the room.
            let rr = rect.shrink(1.0);
            let light = Stroke::new(2.0, lerp(color, Color32::WHITE, 0.4));
            let dark = Stroke::new(2.0, lerp(color, Color32::BLACK, 0.55));
            p.line_segment([rr.left_bottom(), rr.left_top()], light);
            p.line_segment([rr.left_top(), rr.right_top()], light);
            p.line_segment([rr.right_top(), rr.right_bottom()], dark);
            p.line_segment([rr.right_bottom(), rr.left_bottom()], dark);
        }
    }
}

/// The classic look: masks the two corners with `mask` and strokes the
/// six-sided outline.
fn paint_angled_border(p: &Painter, rect: Rect, color: Color32, mask: Color32) {
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

/// 45° yellow/black hazard stripes filling `rect` (clipped to it).
///
/// The app's "pay attention to this" mark: it runs down the side of every
/// section header in the manual, and along the CME arrival banner in the solar
/// view. Shared so the two cannot drift apart.
pub fn hazard_stripes(p: &Painter, rect: Rect, stripe_w: f32) {
    let clip = p.with_clip_rect(rect);
    let h = rect.height();
    let mut x = rect.left() - h;
    let mut k = 0i32;
    while x < rect.right() + stripe_w {
        let color = if k % 2 == 0 { theme::HAZARD() } else { theme::HAZARD_DARK() };
        clip.add(Shape::convex_polygon(
            vec![
                pos2(x, rect.bottom()),
                pos2(x + h, rect.top()),
                pos2(x + h + stripe_w, rect.top()),
                pos2(x + stripe_w, rect.bottom()),
            ],
            color,
            Stroke::NONE,
        ));
        x += stripe_w;
        k += 1;
    }
}

/// A module's inner margin on each side. What a caller measuring its own
/// contents has to add on top of them to arrive at the `width` to reserve.
pub const MODULE_MARGIN_X: f32 = 8.0;

/// A caption-less control module of fixed `width` and `height` (usually
/// [`MODULE_TALL_H`]): a bordered box whose content fills the full box height,
/// vertically centred.
///
/// Uses `allocate_ui_with_layout` so the fixed size is reserved *before* the
/// content is drawn — a module never shrinks into whatever sliver of a row is
/// left; it takes the width it asked for or overflows (a plain `Frame` instead
/// shrinks, which is the wrong behavior here). The fixed height matters too: a
/// bare (w, 0) allocation lets the layout over-reserve vertical space, leaving
/// big gaps between the strip's rows.
pub fn module_bare_h<R>(ui: &mut Ui, width: f32, height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            egui::Frame::new()
                .fill(theme::FILL())
                .stroke(Stroke::new(1.0, theme::LINE_LIT()))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 5 })
                .show(ui, |ui| {
                    ui.set_width(width - 16.0);
                    ui.set_min_height(module_content_h(height));
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_min_height(module_content_h(height));
                        add(ui)
                    })
                    .inner
                })
                .inner
        },
    )
    .inner
}

/// How much of a [`module_bare_h`] box of `height` its contents get: the box
/// less the inner margin above and below them.
///
/// Exposed because a caller sometimes has to know whether two rows will fit
/// *before* it starts drawing — the frequency box decides between stacking the
/// power switch above the A/B pair and leaving it to the VFO menu — and the
/// alternative is that margin written out a second time somewhere it would not
/// follow this one.
pub fn module_content_h(height: f32) -> f32 {
    height - 9.0
}

/// Like [`module_bare_h`] but with zero inner margin and no border, so the
/// content fills the box edge-to-edge. Used by the S-meter, which paints its
/// own instrument face over the whole rect (an opaque fill would otherwise hide
/// a frame border) and draws the box border itself on top.
///
/// It takes [`MODULE_BORDER`] into its own content on both axes: a bordered
/// module's stroke is drawn *outside* the content it was handed, so one built
/// to `height` stands `height + 2` tall. This box's border is painted inside
/// its content instead, so without the two points back it would come out two
/// short — and the S-meter would sit a hair below the boxes beside it.
pub fn module_bare_flush_h<R>(
    ui: &mut Ui,
    width: f32,
    height: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let width = width + 2.0 * MODULE_BORDER;
    let height = height + 2.0 * MODULE_BORDER;
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            egui::Frame::new()
                .fill(theme::FILL())
                .inner_margin(egui::Margin::ZERO)
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_min_height(height);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.set_min_height(height);
                        add(ui)
                    })
                    .inner
                })
                .inner
        },
    )
    .inner
}

/// A row of controls inside a module box or inside a menu popup.
///
/// The bodies of the top-bar modules are written as rows sized for a box of a
/// few hundred points. A popup is one narrow column instead, so the same row
/// has to wrap and its sliders have to shrink to what is left. That difference
/// is the whole of `narrow`, which is why it describes the *container* rather
/// than the layout tier: one body, two shapes, and no second copy to drift.
pub fn control_row<R>(ui: &mut Ui, narrow: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    if narrow {
        ui.horizontal_wrapped(|ui| {
            // Leave room for the label and readout that ride beside a slider;
            // the floor keeps a rail draggable even in the narrowest popup.
            ui.spacing_mut().slider_width = (ui.available_width() - 96.0).clamp(80.0, 220.0);
            add(ui)
        })
        .inner
    } else {
        ui.horizontal(add).inner
    }
}

/// Take a bare Return off the event queue, so a transmit box can use it as
/// "send this line" instead of "new line".
///
/// It has to be called *before* the box is built. A multiline [`egui::TextEdit`]
/// reads the events as it draws, so by the time its response comes back the
/// newline is already in the buffer and there is nothing left to intercept.
///
/// Shift+Return is deliberately left alone. egui's own shortcut matching
/// ignores a stray Shift, which would swallow both; matching the modifiers
/// exactly leaves the edit its combination for breaking a line without sending
/// it. `edit_id` is the id given to the box, so a Return typed anywhere else on
/// the panel is not read as a send.
pub fn take_return(ui: &Ui, edit_id: egui::Id) -> bool {
    if !ui.memory(|m| m.has_focus(edit_id)) {
        return false;
    }
    ui.input_mut(|i| {
        let mut hit = false;
        i.events.retain(|e| {
            let bare = matches!(
                e,
                egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. }
                    if modifiers.is_none()
            );
            hit |= bare;
            !bare
        });
        hit
    })
}

/// The trailing items of a header row: pinned to the right where there is room
/// to spare, and simply next in line where there is not.
///
/// A right-to-left child claims what is left of the row and right-aligns inside
/// it. That reads well on a row with slack, and badly on one that has already
/// wrapped — "what is left of the row" is then the sliver beside the items just
/// placed, and the pinned ones get drawn over them. A compact layout wraps
/// nearly every header, so there it stays in flow.
pub fn row_tail<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    if crate::layout::tier(ui.ctx()).compact() {
        add(ui)
    } else {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add).inner
    }
}

/// A tap-to-open menu popup wearing the app's cut-corner chrome.
///
/// Full opacity and no auto-fade — a menu is read, not glanced at, and on a
/// touch screen there is no hover to hold [`popup_fade_alpha`] open with. It
/// closes on a tap outside or on the chip again.
pub fn menu_popup<R>(ui: &mut Ui, btn: &Response, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    popup_body(ui, btn, None, add)
}

/// [`menu_popup`], but it dismisses itself after a while on a pointer layout —
/// for a popup that also opens from a control box on the desktop strip, where
/// nothing else would take it away. `since` is the caller-owned open time
/// [`popup_fade_alpha`] keeps; the fade is off on a touched layout, so there the
/// two helpers behave identically.
pub fn fading_menu_popup<R>(
    ui: &mut Ui,
    btn: &Response,
    since: &mut Option<f64>,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    popup_body(ui, btn, Some(since), add)
}

/// The body both menu popups share: cut-corner chrome, a width the viewport can
/// hold, and a scrolled content column.
///
/// Sized against the screen rather than the wish, in both directions, because a
/// popup is the one thing here that is not laid out inside a panel:
///
/// - Width comes out of the viewport with the frame's own margins already
///   subtracted. egui constrains a popup's *position* to the screen but cannot
///   shrink one that is too wide, so a content column sized to the full viewport
///   is a popup hanging off the edge of the phone by its margins.
/// - Height is scrolled rather than truncated, for the same reason: a menu
///   taller than a phone in landscape would otherwise have its bottom quietly
///   cut off, with no way to reach it.
///
/// Both sizes are handed to a child `Ui` rather than taken from the popup's
/// own: egui gives an `Area`'s `Ui` the size that area came out as on the
/// *previous* frame, and a `ScrollArea` fills whatever it is offered and
/// scrolls the rest. A menu whose content grew — a layer switched back on, a
/// band appearing — would then be capped at the room it had while it was
/// smaller, and every frame after that re-measures the same cap, so it never
/// grows back. Sizing the body against the screen breaks that ratchet.
fn popup_body<R>(
    ui: &mut Ui,
    btn: &Response,
    mut fade: Option<&mut Option<f64>>,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let screen = ui.ctx().content_rect();
    let now = ui.input(|i| i.time);
    let alpha = match fade.as_deref_mut() {
        Some(since) => {
            popup_fade_alpha(ui.ctx(), egui::Popup::default_response_id(btn), now, since)
        }
        None => 1.0,
    };
    // 24 = the frame's 11 pt inner margin either side, plus a couple of points
    // so the cut-corner border is not flush against the screen edge.
    let max_w = (screen.width() - 24.0).clamp(160.0, 430.0);
    let max_h = screen.height() * 0.6;
    let resp = egui::Popup::from_toggle_button_response(btn)
        .frame(window_frame_alpha(alpha))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_opacity(alpha);
            window_body_bg(ui);
            // (The menu font-size setting is egui's zoom factor — see
            // `theme::apply_zoom` — so it is already in every point here.)
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            let body = Rect::from_min_size(ui.max_rect().min, vec2(max_w, max_h));
            ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                egui::ScrollArea::vertical().max_height(max_h).show_themed(ui, add).inner
            })
            .inner
        });
    if let Some(r) = &resp {
        paint_popup_cut_border(ui.ctx(), &r.response, alpha);
        // Hovering the popup keeps it up: the fade is for a menu left open and
        // forgotten, not one being read.
        if r.response.contains_pointer()
            && let Some(since) = fade
        {
            *since = Some(now);
        }
    }
    resp.map(|r| r.inner)
}

/// A section caption inside a menu popup — the same small cyan label the
/// module boxes wear, so a menu reads as the box it replaced.
pub fn menu_caption(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).color(theme::CYAN_DIM()).size(9.5).strong());
}

/// A captioned box around a run of related controls — the caption
/// [`menu_caption`] would have drawn on its own, plus a hairline border around
/// exactly what it names. A popup that covers two subjects then reads as two
/// subjects rather than as one long list.
///
/// `width` is the content width, and every group in the same popup is given
/// the same one so the boxes come out matching whatever each holds. It is
/// pinned as both the minimum and the maximum for the reason [`angled_frame`]
/// gives: a `Frame` auto-sizes by measuring its content with unbounded width,
/// and a `horizontal_wrapped` row measured that way never wraps.
pub fn menu_group<R>(ui: &mut Ui, title: &str, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(theme::ROW_BG())
        .stroke(Stroke::new(1.0, theme::LINE()))
        .corner_radius(window_corner_radius())
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_min_width(width);
            ui.set_max_width(width);
            menu_caption(ui, title);
            add(ui)
        })
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
        .fill(theme::ROW_BG())
        .stroke(Stroke::new(1.0, theme::RED_DEEP()))
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
        theme::PINK(),
    );
    inner.inner
}

/// A slider with a visible dark track. egui draws the slider rail with
/// `widgets.inactive.bg_fill`, which equals the module background here, so
/// the empty portion of the track would otherwise be invisible.
pub fn slider(ui: &mut Ui, slider: egui::Slider<'_>) -> Response {
    // A fatter rail where the handle is dragged with a finger. Set here rather
    // than in the theme so the handful of raw `Slider`s elsewhere keep theirs.
    let rail = if crate::layout::tier(ui.ctx()).touch() { 12.0 } else { 6.0 };
    ui.scope(|ui| {
        ui.visuals_mut().widgets.inactive.bg_fill = theme::INPUT_BG();
        ui.visuals_mut().widgets.hovered.bg_fill = theme::INPUT_BG();
        ui.spacing_mut().slider_rail_height = rail;
        // The rail, the trailing fill and the knob are all chrome, so the whole
        // widget goes through the skin — see [`reskin`].
        reskin(ui, Skin::Whole, Depth::Recessed, |ui| ui.add(slider))
    })
    .inner
}

/// [`slider`] that may be greyed out — `ui.add_enabled`, with a skin.
pub fn slider_enabled(ui: &mut Ui, enabled: bool, slider: egui::Slider<'_>) -> Response {
    ui.add_enabled_ui(enabled, |ui| self::slider(ui, slider)).inner
}

// ---------------------------------------------------------------------------
// Reskinning egui's own widgets
// ---------------------------------------------------------------------------

/// Which way a surface faces the light. A button, a dropdown's face and a
/// slider's knob stand proud of the panel; a text box, a tick box and a
/// slider's rail are sunk into it. Only the Gradient and 3D bevel styles read
/// it, and it is the whole of what makes those two say *depth* rather than
/// merely *decoration*.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Depth {
    Raised,
    Recessed,
    /// A slider's knob: raised like a button, and small enough that the
    /// Terminal style gives it a character of its own rather than a frame.
    Knob,
}

/// How much of what a widget painted counts as its chrome.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Skin {
    /// The widget's own background — the first filled rect it painted, and
    /// nothing after it. A text box paints its frame first and its selection
    /// highlight later, and a highlight is content, not chrome.
    Frame,
    /// Every rect and every disc it painted: a slider's rail, its trailing
    /// fill and its knob are all chrome, and they arrive in that order.
    Whole,
}

/// Run `add` — a stock egui widget — and reshape the chrome it painted into
/// the current button style.
///
/// egui draws its widgets as plain rects, which is all a `CornerRadius` can
/// express: the Rectangular and Rounded styles are already exactly that, and
/// the other three have nothing to hang on. Rather than reimplement
/// `TextEdit`, `Slider` and `ComboBox` — three widgets whose *behaviour* is
/// the hard part and is not what we want to change — this runs them untouched
/// and rewrites the shapes they left behind in the paint list. The widget
/// keeps every keystroke, every drag and every id it had; only its skin moves.
///
/// Two passes over the list, because the replacements may need to lay text out
/// (the Terminal style draws its frames from characters) and the font atlas
/// cannot be read while the graphics lock is held.
fn reskin<R>(ui: &mut Ui, skin: Skin, depth: Depth, add: impl FnOnce(&mut Ui) -> R) -> R {
    let style = theme::button_style();
    if matches!(style, ChromeStyle::Rectangular | ChromeStyle::Rounded) {
        return add(ui);
    }
    let layer = ui.layer_id();
    let from = ui.ctx().graphics_mut(|g| g.entry(layer).next_idx());
    let out = add(ui);

    let mut taken: Vec<(ShapeIdx, Shape)> = Vec::new();
    ui.ctx().graphics_mut(|g| {
        let list = g.entry(layer);
        for i in from.0..list.next_idx().0 {
            list.mutate_shape(ShapeIdx(i), |cs| {
                if is_chrome(&cs.shape, skin) {
                    taken.push((ShapeIdx(i), cs.shape.clone()));
                }
            });
            if skin == Skin::Frame && !taken.is_empty() {
                break;
            }
        }
    });
    let redone: Vec<(ShapeIdx, Shape)> = taken
        .iter()
        .filter_map(|(i, s)| {
            let depth = if skin == Skin::Whole && is_knob(s) { Depth::Knob } else { depth };
            Some((*i, reskin_shape(ui, s, style, depth)?))
        })
        .collect();
    if !redone.is_empty() {
        ui.ctx().graphics_mut(|g| {
            let list = g.entry(layer);
            for (i, shape) in redone {
                list.mutate_shape(i, move |cs| cs.shape = shape);
            }
        });
    }
    out
}

/// Is this one of the shapes a widget paints as its own frame, rather than as
/// its contents? Anything textured, blurred, invisible or smaller than a
/// character is left alone — those are shadows, hairlines and text carets.
fn is_chrome(shape: &Shape, skin: Skin) -> bool {
    match shape {
        Shape::Rect(r) => {
            (r.fill.a() > 0 || r.stroke.width > 0.0)
                && r.rect.width() >= 3.0
                && r.rect.height() >= 3.0
                && r.brush.is_none()
                && r.blur_width == 0.0
        }
        // A disc in a widget's paint list is a slider knob; nothing else here
        // draws one.
        Shape::Circle(c) => skin == Skin::Whole && c.radius >= 2.0,
        _ => false,
    }
}

/// Is this one of the three rects a slider paints its knob?
///
/// The rail and the fill that tracks the value are long and thin along
/// whichever way the slider runs; the knob is the compact one, and it is the
/// only part of a slider that stands proud of the panel. The size cap keeps
/// the readout beside a slider — a `DragValue`, which is a rect too — out of
/// it: a knob is a couple of dozen points at the very most, even under the
/// touch metrics.
fn is_knob(shape: &Shape) -> bool {
    let Shape::Rect(r) = shape else { return false };
    let (w, h) = (r.rect.width(), r.rect.height());
    w < 3.0 * h && h < 3.0 * w && w.max(h) <= 30.0
}

fn reskin_shape(ui: &Ui, shape: &Shape, style: ChromeStyle, depth: Depth) -> Option<Shape> {
    match shape {
        Shape::Rect(r) => Some(styled_surface(ui, r, style, depth)),
        Shape::Circle(c) => Some(styled_knob(ui, c, style)),
        _ => None,
    }
}

/// The character-display face at `size` points. Everything the Terminal style
/// draws is set in it, frames included: a frame made of `+` and `-` only lines
/// up if the glyphs come off one grid.
///
/// The generic monospace family rather than `cyber-mono` by name: the app
/// makes Share Tech Mono the head of that family
/// ([`crate::theme::install_fonts`]), so this is the same face wherever the
/// fonts were installed — and a context that never installed them (a test, a
/// bare viewport) falls back instead of panicking, which naming a family that
/// is not bound would do.
fn cell_font(size: f32) -> egui::FontId {
    egui::FontId::monospace(size)
}

/// `text` laid out in the character-display face. Newlines start a new row, so
/// a whole column of `|` is one galley.
fn cells(p: &Painter, text: &str, size: f32, color: Color32) -> std::sync::Arc<egui::Galley> {
    p.layout_no_wrap(text.to_owned(), cell_font(size), color)
}

/// One character cell of the display at `size`: the advance of a glyph, and
/// the height of a row.
fn cell_size(p: &Painter, size: f32) -> egui::Vec2 {
    cells(p, "-", size, Color32::WHITE).size()
}

/// The point size the Terminal style draws its frames at — the body text
/// size, so a frame built from characters is the same scale as the words
/// inside it and follows the interface font setting with them.
///
/// Read off the context rather than a `Ui`, because a window's border is
/// painted straight onto a layer, with no `Ui` in hand.
fn terminal_size(p: &Painter) -> f32 {
    let ctx = p.ctx();
    TextStyle::Body.resolve(&ctx.style_of(ctx.theme())).size
}

/// A box drawn the way a character display draws one: `+` at the corners, a
/// run of `-` along the top and bottom, a column of `|` down each side.
///
/// Used for anything with the room to carry three rows of characters — a
/// multi-line text box, a floating window. Shorter controls get [`brackets`]
/// instead, which is what a one-line field looks like on the same display.
fn ascii_box(p: &Painter, rect: Rect, ink: Color32, size: f32, sides: Sides) -> Vec<Shape> {
    let cell = cell_size(p, size);
    let (cw, ch) = (cell.x.max(1.0), cell.y.max(1.0));
    // The border cells straddle the edge, so the *ink* lands on it: a `-` is
    // inked across the middle of its own cell and a `|` down the middle of
    // its, and what hangs over the edge is the blank part of the cell. That
    // is what lets a frame sit on a control's border without the top row of
    // characters landing on the first line of text inside it.
    let (x0, x1) = (rect.left() - cw / 2.0, rect.right() - cw / 2.0);
    let (y0, y1) = (rect.top() - ch * 0.42, rect.bottom() - ch * 0.58);
    let mut out = Vec::new();
    let mut put = |pos: Pos2, text: String| {
        out.push(Shape::galley(pos, cells(p, &text, size, ink), ink));
    };
    let closed = sides == Sides::All;
    for corner in [pos2(x0, y0), pos2(x1, y0)] {
        put(corner, "+".into());
    }
    if closed {
        for corner in [pos2(x0, y1), pos2(x1, y1)] {
            put(corner, "+".into());
        }
    }
    // The runs between the corners, centred on the gap they fill: a whole
    // number of cells rarely divides it exactly, and half the remainder at
    // each end reads as kerning where all of it at one end reads as a hole.
    let across = ((x1 - x0) / cw).floor() as i32 - 1;
    if across > 0 {
        let run = across as usize;
        let x = x0 + cw + ((x1 - x0 - cw) - run as f32 * cw) / 2.0;
        put(pos2(x, y0), "-".repeat(run));
        if closed {
            put(pos2(x, y1), "-".repeat(run));
        }
    }
    // The sides run to the bottom corners where there are any, and all the way
    // to the bottom edge where the frame is open — a tab has no line across
    // the join with the page it opens.
    let bottom = if closed { y1 } else { rect.bottom() };
    let down = ((bottom - y0) / ch).floor() as i32 - 1;
    if down > 0 {
        let run = down as usize;
        let y = y0 + ch + ((bottom - y0 - ch) - run as f32 * ch) / 2.0;
        let column = std::iter::repeat_n("|", run).collect::<Vec<_>>().join("\n");
        put(pos2(x0, y), column.clone());
        put(pos2(x1, y), column);
    }
    out
}

/// Whether a character frame closes along its bottom edge.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sides {
    /// All four — a window, a panel, a multi-line text box.
    All,
    /// Open at the bottom, which is the join with the page below: a tab.
    OpenBottom,
}

/// The `[` and `]` a one-line control wears on a character display, tucked
/// into the padding at either end of `rect`.
fn brackets(p: &Painter, rect: Rect, ink: Color32) -> Vec<Shape> {
    // The display's own size rather than the control's height: a bracket
    // scaled up to fill a roomy field would have its ink out past the 4 pt a
    // text box keeps for a margin, and would sit on the first letter typed.
    let size = (terminal_size(p) * 0.85).min(rect.height() * 0.9);
    let mut out = Vec::new();
    for (text, x) in [("[", rect.left()), ("]", rect.right() - cell_size(p, size).x)] {
        let g = cells(p, text, size, ink);
        let y = rect.center().y - g.size().y / 2.0;
        out.push(Shape::galley(pos2(x, y), g, ink));
    }
    out
}

/// A run of one character filling `rect` across — the Terminal style's answer
/// to a bar too short to frame: a slider rail is `----------`, and the fill
/// behind the knob is `==========` over the top of it.
fn cell_run(p: &Painter, rect: Rect, glyph: &str, ink: Color32, size: f32) -> Vec<Shape> {
    let cw = cell_size(p, size).x.max(1.0);
    let n = ((rect.width() / cw).round() as i32).max(1) as usize;
    let g = cells(p, &glyph.repeat(n), size, ink);
    vec![Shape::galley(pos2(rect.left(), rect.center().y - g.size().y / 2.0), g, ink)]
}

/// One rect of widget chrome, redrawn in `style`.
///
/// Shared by the reskinned egui widgets and by the tick box [`checkbox`]
/// paints by hand, so a text field and the checkbox beside it cannot come out
/// wearing different corners.
fn styled_surface(ui: &Ui, r: &egui::epaint::RectShape, style: ChromeStyle, depth: Depth) -> Shape {
    let (rect, fill, stroke) = (r.rect, r.fill, r.stroke);
    match style {
        ChromeStyle::Rectangular | ChromeStyle::Rounded => Shape::Rect(r.clone()),
        ChromeStyle::Angled => {
            let cut = CHIP_CUT.min(rect.height() * 0.35).min(rect.width() * 0.35);
            let pts = chip_outline(rect, cut);
            let mut out = vec![Shape::convex_polygon(pts.clone(), fill, stroke)];
            // The two chamfers, lit, each running on a little way along the
            // edges it joins: the mark that says a corner was *cut* rather
            // than merely missing, and the one accent with room to carry on a
            // control this small. The spurs are what turn a chamfer into a
            // bracket, so they wait until there is height to spend on them.
            if stroke.width > 0.0 && cut > 1.5 {
                let lit = Stroke::new(stroke.width + 1.0, lerp(stroke.color, theme::CYAN(), 0.7));
                out.push(Shape::line_segment([pts[5], pts[0]], lit));
                out.push(Shape::line_segment([pts[2], pts[3]], lit));
                if rect.height() >= 14.0 {
                    let spur = cut * 1.4;
                    out.push(Shape::line_segment([pts[0], pts[0] + vec2(spur, 0.0)], lit));
                    out.push(Shape::line_segment([pts[5], pts[5] + vec2(0.0, spur)], lit));
                    out.push(Shape::line_segment([pts[2], pts[2] - vec2(0.0, spur)], lit));
                    out.push(Shape::line_segment([pts[3], pts[3] - vec2(spur, 0.0)], lit));
                }
            }
            Shape::Vec(out)
        }
        ChromeStyle::Gradient => {
            let mut out = Vec::new();
            if fill.a() > 0 {
                // Light from above: a raised face catches it on top and falls
                // away below, a sunken one is shadowed at the top and picks up
                // the bounce along its bottom lip.
                let (top, bottom) = match depth {
                    Depth::Recessed => {
                        (lerp(fill, Color32::BLACK, 0.55), lerp(fill, Color32::WHITE, 0.17))
                    }
                    _ => (lerp(fill, Color32::WHITE, 0.26), lerp(fill, Color32::BLACK, 0.32)),
                };
                out.push(grad_rect(rect, top, bottom));
            }
            if stroke.width > 0.0 {
                out.push(Shape::Rect(egui::epaint::RectShape {
                    fill: Color32::TRANSPARENT,
                    ..r.clone()
                }));
            }
            Shape::Vec(out)
        }
        ChromeStyle::Bevel => {
            let mut out = vec![Shape::Rect(egui::epaint::RectShape {
                corner_radius: CornerRadius::ZERO,
                ..r.clone()
            })];
            // Overlaid white and black rather than shades of the fill: an
            // input well is nearly black on most of these themes, and a bevel
            // mixed out of *it* would have nothing left to darken.
            let (near, far) = match depth {
                Depth::Recessed => (Color32::from_black_alpha(175), Color32::from_white_alpha(80)),
                _ => (Color32::from_white_alpha(105), Color32::from_black_alpha(150)),
            };
            let rr = rect.shrink(stroke.width.max(0.5));
            let w = (rect.height() * 0.14).clamp(1.2, 2.5);
            out.push(Shape::line_segment([rr.left_bottom(), rr.left_top()], Stroke::new(w, near)));
            out.push(Shape::line_segment([rr.left_top(), rr.right_top()], Stroke::new(w, near)));
            out.push(Shape::line_segment([rr.right_top(), rr.right_bottom()], Stroke::new(w, far)));
            out.push(Shape::line_segment(
                [rr.right_bottom(), rr.left_bottom()],
                Stroke::new(w, far),
            ));
            Shape::Vec(out)
        }
        ChromeStyle::Terminal => {
            let ink = if stroke.width > 0.0 { stroke.color } else { theme::LINE_LIT() };
            let p = ui.painter();
            let size = terminal_size(p);
            let cell = cell_size(p, size);
            if depth == Depth::Knob {
                // The outline colour, not the fill: egui gives a knob the
                // panel's own dark fill and puts all its contrast in the
                // stroke, and a `#` painted in that fill would vanish into
                // the rail it is meant to be sliding along.
                let ink = if stroke.width > 0.0 { stroke.color } else { fill };
                let g = cells(p, "#", rect.height().min(rect.width() * 2.0), ink);
                return Shape::galley(rect.center() - g.size() / 2.0, g, ink);
            }
            // Thinner than a character and running across: a bar, not a box.
            // A slider draws its rail and the fill that tracks the value as
            // two of these, one over the other, which is how a gauge is drawn
            // on a display with only characters to draw with. A rail running
            // *down* keeps its solid block — a column of glyphs would have
            // the row spacing showing through it as gaps.
            if rect.height() < cell.y * 1.4 && rect.width() > rect.height() {
                let bar = if fill.a() > 0 { fill } else { ink };
                let size = (rect.height() * 1.9).clamp(8.0, 15.0);
                return Shape::Vec(cell_run(p, rect, "=", bar, size));
            }
            let mut out = Vec::new();
            // A sunken surface keeps its fill: an input well on a character
            // display is a block of inverse video, and it is the only thing
            // that says where the typing will land. A raised one — a button
            // face, a dropdown — is drawn from its characters alone.
            if depth == Depth::Recessed && fill.a() > 0 {
                out.push(Shape::rect_filled(rect, CornerRadius::ZERO, fill));
            }
            // A one-line control stands about as tall as the layout's own hit
            // target — that, not the text, is what sets a field's height — so
            // anything a third taller again is carrying more than one line and
            // gets the full `+--+ | | +--+` frame. A multi-line text box is
            // the case this is really for; `[` and `]` around four lines of
            // text would say nothing about where the box ends.
            let one_line = ui.spacing().interact_size.y.max(cell.y) + 8.0;
            if rect.height() >= one_line * 1.35 {
                out.extend(ascii_box(p, rect, ink, size, Sides::All));
            } else {
                out.extend(brackets(p, rect, ink));
            }
            Shape::Vec(out)
        }
    }
}

/// A slider's knob, redrawn in `style`. Always raised — a knob is the one part
/// of a slider a finger is meant to reach for.
fn styled_knob(ui: &Ui, c: &egui::epaint::CircleShape, style: ChromeStyle) -> Shape {
    let (centre, rad) = (c.center, c.radius);
    match style {
        ChromeStyle::Angled => {
            // A hex grip rather than a disc, flat side up so it reads as
            // machined rather than turned.
            let pts = (0..6)
                .map(|k| {
                    let a = std::f32::consts::PI * (k as f32 / 3.0 + 1.0 / 6.0);
                    centre + vec2(a.cos() * rad, a.sin() * rad)
                })
                .collect();
            Shape::convex_polygon(pts, c.fill, c.stroke)
        }
        // A fader cap, square so it suits a vertical rail as well as a
        // horizontal one — the disc carries no orientation for us to keep.
        ChromeStyle::Gradient | ChromeStyle::Bevel => {
            let cap = Rect::from_center_size(centre, egui::Vec2::splat(rad * 1.7));
            styled_surface(
                ui,
                &egui::epaint::RectShape::new(
                    cap,
                    CornerRadius::ZERO,
                    c.fill,
                    c.stroke,
                    StrokeKind::Inside,
                ),
                style,
                Depth::Raised,
            )
        }
        // A knob on a character display is the one cell that is not rail.
        ChromeStyle::Terminal => {
            // The outline colour, not the fill: egui gives a knob the panel's
            // own dark fill and puts all its contrast in the stroke, and a `#`
            // painted in that fill would be invisible on the rail.
            let ink = if c.stroke.width > 0.0 { c.stroke.color } else { c.fill };
            let g = cells(ui.painter(), "#", (rad * 2.6).clamp(9.0, 18.0), ink);
            Shape::galley(centre - g.size() / 2.0, g, ink)
        }
        _ => Shape::Circle(*c),
    }
}

/// A stock egui widget drawn as a sunken input surface — a text box, a
/// numeric field — with its frame reshaped to the current button style.
///
/// Takes the built widget rather than its arguments so every `TextEdit`
/// setting a caller reaches for keeps working: this is `ui.add`, with a skin.
pub fn field(ui: &mut Ui, widget: impl egui::Widget) -> Response {
    reskin(ui, Skin::Frame, Depth::Recessed, |ui| ui.add(widget))
}

/// [`field`] at an exact size — `ui.add_sized`, with a skin.
pub fn field_sized(
    ui: &mut Ui,
    size: impl Into<egui::Vec2>,
    widget: impl egui::Widget,
) -> Response {
    reskin(ui, Skin::Frame, Depth::Recessed, |ui| ui.add_sized(size, widget))
}

/// [`field`] that may be greyed out — `ui.add_enabled`, with a skin.
pub fn field_enabled(ui: &mut Ui, enabled: bool, widget: impl egui::Widget) -> Response {
    reskin(ui, Skin::Frame, Depth::Recessed, |ui| ui.add_enabled(enabled, widget))
}

/// A dropdown wearing the app's chrome: [`egui::ComboBox`], with its face
/// reshaped to the current button style.
///
/// An extension trait rather than a free function because a combo is built by
/// a chain (`ComboBox::from_id_salt(..).selected_text(..)`) and reads far
/// better ended with `.show_styled(ui, ..)` than wrapped in a call whose
/// opening paren is thirty characters back up the line.
pub trait StyledCombo {
    /// [`egui::ComboBox::show_ui`], with the button face restyled.
    fn show_styled<R>(
        self,
        ui: &mut Ui,
        menu_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>>;
}

impl StyledCombo for egui::ComboBox {
    fn show_styled<R>(
        self,
        ui: &mut Ui,
        menu_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        // egui's filled triangle is the one part of a dropdown that cannot be
        // reskinned after the fact — it is a polygon, not a rect — so the
        // Terminal style hands the widget a character to draw instead.
        let combo = match theme::button_style() {
            ChromeStyle::Terminal => self.icon(terminal_arrow),
            _ => self,
        };
        reskin(ui, Skin::Frame, Depth::Raised, move |ui| combo.show_ui(ui, menu_contents))
    }
}

/// The `v` a dropdown wears on a character display.
fn terminal_arrow(ui: &Ui, rect: Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    let ink = visuals.fg_stroke.color;
    let g = cells(ui.painter(), "v", terminal_size(ui.painter()), ink);
    ui.painter().galley(rect.center() - g.size() / 2.0, g, ink);
}

/// A checkbox in the app's chrome.
///
/// Drawn by hand rather than reskinned, because a checkbox *is* its tick box:
/// the widget paints the box and its label a single rect apart, and there is
/// no way to tell one from the other after the fact. The metrics follow
/// egui's own so a column of these still lines up with everything beside it.
pub fn checkbox(ui: &mut Ui, on: &mut bool, text: impl Into<WidgetText>) -> Response {
    let (icon_w, gap) = (tick_box_width(ui), ui.spacing().icon_spacing);
    let wrap = {
        let a = ui.available_width() - icon_w - gap;
        if a.is_finite() && a > 40.0 { a } else { f32::INFINITY }
    };
    let galley = WidgetText::from(text.into()).into_galley(
        ui,
        None,
        wrap,
        FontSelection::Style(TextStyle::Button),
    );
    let label = galley.size();
    let mut size =
        vec2(if label.x > 0.0 { icon_w + gap + label.x } else { icon_w }, label.y.max(icon_w));
    size = size.max(egui::Vec2::splat(ui.spacing().interact_size.y));
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, galley.text())
    });

    if ui.is_rect_visible(rect) {
        let v = *ui.style().interact(&resp);
        let box_rect = Rect::from_center_size(
            pos2(rect.left() + icon_w / 2.0, rect.center().y),
            egui::Vec2::splat(icon_w),
        );
        paint_tick_box(ui, box_rect, *on, &v);
        if label.x > 0.0 {
            let pos = pos2(rect.left() + icon_w + gap, rect.center().y - label.y / 2.0);
            ui.painter().galley(pos, galley, v.text_color());
        }
    }
    resp
}

/// How wide a [`checkbox`]'s tick box stands: three character cells under the
/// Terminal style, where the box *is* the three characters `[X]` and has to
/// be readable, and egui's own icon width everywhere else.
fn tick_box_width(ui: &Ui) -> f32 {
    if theme::button_style() == ChromeStyle::Terminal {
        (3.0 * cell_size(ui.painter(), terminal_size(ui.painter())).x).max(ui.spacing().icon_width)
    } else {
        ui.spacing().icon_width
    }
}

/// The tick box of a [`checkbox`], in the current button style.
fn paint_tick_box(ui: &Ui, rect: Rect, on: bool, v: &egui::style::WidgetVisuals) {
    let p = ui.painter();
    // On a character display a tick box is literally `[X]` — no box to shape
    // and no tick to draw, so this one style takes the whole control.
    if theme::button_style() == ChromeStyle::Terminal {
        let ink = if on { theme::CYAN() } else { v.bg_stroke.color };
        let g = cells(p, if on { "[X]" } else { "[ ]" }, terminal_size(p), ink);
        p.galley(rect.center() - g.size() / 2.0, g, ink);
        return;
    }
    p.add(styled_surface(
        ui,
        &egui::epaint::RectShape::new(
            rect,
            v.corner_radius,
            v.bg_fill,
            v.bg_stroke,
            StrokeKind::Inside,
        ),
        theme::button_style(),
        Depth::Recessed,
    ));
    if on {
        let t = rect.shrink(rect.width() * 0.26);
        p.add(Shape::line(
            vec![
                pos2(t.left(), t.center().y),
                pos2(t.center().x, t.bottom()),
                pos2(t.right(), t.top()),
            ],
            Stroke::new((rect.width() * 0.14).clamp(1.4, 2.4), v.fg_stroke.color),
        ));
    }
}

/// The draggable divider between two regions of a panel: allocates the strip,
/// shows the resize cursor over it, and paints three grip marks so it reads as
/// something that can be moved. Returns the response for the caller's own drag
/// arithmetic.
///
/// One helper for every splitter in the program because the grip marks are the
/// only thing that says a divider exists at all — a handle that paints nothing
/// until the pointer is already on it is a control nobody finds.
///
/// Orientation comes from the shape: a tall, narrow strip divides two columns
/// (grips stacked down it, horizontal resize cursor); a wide, short one divides
/// two rows. `bg` fills the strip first, for handles that sit on the page
/// background rather than inside a panel.
pub fn split_handle(ui: &mut Ui, size: egui::Vec2, bg: Option<Color32>) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    // A strip taller than it is wide separates left from right.
    let columns = size.y >= size.x;
    let hot = resp.hovered() || resp.dragged();
    if hot {
        ui.ctx().set_cursor_icon(if columns {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::ResizeVertical
        });
    }
    let p = ui.painter_at(rect);
    if let Some(bg) = bg {
        p.rect_filled(rect, 0.0, bg);
    }
    let col = if hot { theme::CYAN() } else { theme::gray(70) };
    let (cx, cy) = (rect.center().x, rect.center().y);
    for d in [-16.0f32, 0.0, 16.0] {
        let seg = if columns {
            [pos2(cx, cy + d - 6.0), pos2(cx, cy + d + 6.0)]
        } else {
            [pos2(cx + d - 6.0, cy), pos2(cx + d + 6.0, cy)]
        };
        p.line_segment(seg, Stroke::new(2.0, col));
    }
    resp
}

/// How wide `text` lays out in `font`.
pub fn text_width(ui: &Ui, text: &str, font: egui::FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font, Color32::WHITE).size().x
}

/// The font a chip uses for `size` points of text, or for its default.
///
/// Monospace under the Terminal style: a button drawn as `[ LABEL ]` on a
/// character display is set in the display's own face, and every width in the
/// program comes back through [`chip_width`], so the rows still measure.
fn chip_font(ui: &Ui, size: Option<f32>) -> egui::FontId {
    let base = match size {
        Some(pt) => egui::FontId::proportional(pt),
        None => TextStyle::Button.resolve(ui.style()),
    };
    if theme::button_style() == ChromeStyle::Terminal { cell_font(base.size) } else { base }
}

/// What a chip carrying `label` will measure, padding included — the same
/// arithmetic [`chip`] does, for a caller that has to budget a row before
/// drawing it. `size` is the text size where the chip sets one.
pub fn chip_width(ui: &Ui, label: &str, size: Option<f32>) -> f32 {
    text_width(ui, label, chip_font(ui, size)) + 2.0 * chip_padding(ui).x
}

/// How tall a chip stands, padding included. A literal would be wrong the
/// moment the layout changes: chip padding comes from the style, and a touched
/// one is roomier.
pub fn chip_height(ui: &Ui, size: Option<f32>) -> f32 {
    ui.painter().layout_no_wrap("0".to_owned(), chip_font(ui, size), Color32::WHITE).size().y
        + 2.0 * chip_padding(ui).y
}

/// A chip's padding. A shade roomier than a plain button, which is what gives a
/// chip its pill-like proportions. Taken from the style rather than fixed so a
/// touched layout's larger `button_padding` grows every chip in the program at
/// once — a chip is the only button this app has.
fn chip_padding(ui: &Ui) -> egui::Vec2 {
    // The Terminal style spends the extra width on its brackets, so the label
    // still has the same air around it as it does under every other style.
    let brackets = if theme::button_style() == ChromeStyle::Terminal {
        vec2(7.0, 0.0)
    } else {
        vec2(0.0, 0.0)
    };
    ui.spacing().button_padding + vec2(2.0, 1.0) + brackets
}

/// The colour a chip wears instead of the cyan a selected one takes by
/// default, and the ink that goes on it. `lit` says the chip wears it always
/// rather than only while selected — see [`chip_lit_sized`].
#[derive(Clone, Copy)]
struct Accent {
    fill: Color32,
    ink: Color32,
    lit: bool,
}

impl Accent {
    /// An accent that stands in for the cyan of a *selected* chip: PTT red,
    /// the scanner's green. The chip is plain until it is selected.
    fn on_select(fill: Color32, ink: Color32) -> Self {
        Self { fill, ink, lit: false }
    }

    /// An accent the chip wears the whole time, selected or not.
    fn lit(fill: Color32, ink: Color32) -> Self {
        Self { fill, ink, lit: true }
    }
}

/// Angled chip: a selectable button with cut top-left and bottom-right corners.
/// Selected chips fill cyan with dark ink, like the reference nav pills.
pub fn chip(ui: &mut Ui, selected: bool, text: impl Into<RichText>) -> Response {
    chip_impl(ui, selected, text.into(), None, Sense::click(), None)
}

/// [`chip`], but the pointer may also drag it: the settings roster's radio
/// chips, which are rearranged by dragging (issue #224). A drag that never
/// travels far enough to be one is still delivered as a click, so nothing is
/// lost by making a chip draggable.
pub fn chip_draggable(ui: &mut Ui, selected: bool, text: impl Into<RichText>) -> Response {
    chip_impl(ui, selected, text.into(), None, Sense::click_and_drag(), None)
}

/// What sdroxide's own link to a radio does, in the one place all three of its
/// homes read it from: the frequency box, the tab strip and the settings
/// roster.
///
/// It says what is let go of, because the alternative reading — that this is
/// the rig's own power button — is a fair guess and a wrong one, and an
/// operator who believed it would think a radio that is still connected had
/// been switched off (issue #169).
///
/// The chips it labels say **LINK** rather than ON and OFF for the same
/// reason, and it took a second report to get there: a switch reading "on/off"
/// beside a radio is read as the radio's, whatever the hover text says. On/off
/// now belongs to one control in the whole program — **PWR**, which really does
/// throw the set's own switch over the control link.
pub const LINK_CLOSE_TIP: &str = "Close sdroxide's link to this radio. This is sdroxide's \
     switch, not the rig's: the radio itself stays powered — its own switch is PWR. What is \
     let go of is this end — the device released, a CAT port closed, a network session hung \
     up — so the radio can neither receive nor transmit here. Its settings are kept.";

/// The other half of [`LINK_CLOSE_TIP`].
pub const LINK_OPEN_TIP: &str = "Open sdroxide's link to this radio again, where it was left. \
     This does not power the radio: that is PWR.";

/// A chip carrying two interlocking rings — a link — instead of a label, at an
/// exact size. The mark is painted, not typed: a bundled font is not to be
/// counted on for a symbol, and as text a missing glyph comes out as a tofu
/// box. It takes the ink the label would have taken — the same choice
/// [`chip_impl`] makes — so it follows the chip through selection and hover
/// under every chrome style.
///
/// It used to be the IEC power symbol (⏻), and that was the trouble: this is
/// sdroxide's link to the radio, and the mains symbol is the single strongest
/// argument that it is the radio's own switch. See [`LINK_CLOSE_TIP`].
pub fn chip_link(ui: &mut Ui, selected: bool, size: egui::Vec2) -> Response {
    let resp = chip_impl(ui, selected, RichText::new(""), None, Sense::click(), Some(size));
    if ui.is_rect_visible(resp.rect) {
        let v = ui.style().interact_selectable(&resp, selected);
        let ink = if selected { theme::INK_ON_CYAN() } else { v.fg_stroke.color };
        let stroke = Stroke::new((size.y * 0.07).clamp(1.4, 2.2), ink);
        let r = resp.rect.height() * 0.24;
        let c = resp.rect.center();
        // Two rings overlapping by a little under half a radius: far enough
        // apart to read as two, close enough to read as joined.
        let p = ui.painter();
        for dx in [-0.55, 0.55] {
            p.circle_stroke(Pos2::new(c.x + dx * r, c.y), r, stroke);
        }
    }
    resp
}

/// A chip stretched to an exact `size` rather than hugging its label — for the
/// compact strip's button grid, whose rows divide the width they were given
/// between them instead of clustering at one end of it. The label stays
/// centred; everything else matches [`chip`].
pub fn chip_sized(
    ui: &mut Ui,
    selected: bool,
    text: impl Into<RichText>,
    size: egui::Vec2,
) -> Response {
    chip_impl(ui, selected, text.into(), None, Sense::click(), Some(size))
}

/// [`chip_accent`] at an exact size — for a stretched row that carries accent
/// chips among plain ones (the condensed System box's SAT and SCAN), so every
/// chip in it can grow by the same amount.
pub fn chip_accent_sized(
    ui: &mut Ui,
    selected: bool,
    text: impl Into<RichText>,
    fill: Color32,
    ink: Color32,
    size: egui::Vec2,
) -> Response {
    chip_impl(
        ui,
        selected,
        text.into(),
        Some(Accent::on_select(fill, ink)),
        Sense::click(),
        Some(size),
    )
}

/// A chip that wears its accent colour the whole time rather than only while
/// selected, at an exact size — for the one button on a bar that has to be
/// found without being looked for (the band/mode selector). It still answers
/// the pointer: the fill brightens under the cursor and sinks while it is
/// held, so it reads as a button and not as a status light.
pub fn chip_lit_sized(
    ui: &mut Ui,
    text: impl Into<RichText>,
    fill: Color32,
    ink: Color32,
    size: egui::Vec2,
) -> Response {
    chip_impl(ui, false, text.into(), Some(Accent::lit(fill, ink)), Sense::click(), Some(size))
}

/// [`chip_hold`] at an exact size — the compact strip's PTT, which is drawn
/// bigger than its label needs because it is the one control worth a whole
/// thumb.
pub fn chip_hold_sized(
    ui: &mut Ui,
    selected: bool,
    text: impl Into<RichText>,
    fill: Color32,
    ink: Color32,
    size: egui::Vec2,
) -> Response {
    chip_impl(
        ui,
        selected,
        text.into(),
        Some(Accent::on_select(fill, ink)),
        Sense::click_and_drag(),
        Some(size),
    )
}

/// A chip that may be greyed out, in a row that is allowed to wrap.
///
/// `Ui::add_enabled_ui` builds a child `Ui`, and a child `Ui` inside a
/// `horizontal_wrapped` row does not wrap: it is measured first and placed
/// afterwards with `allocate_rect`, which never consults the wrapping placer.
/// The row then simply grows past the edge it should have broken at — which is
/// how the band row of the band/mode menu came to be one 870 pt line, wider
/// than any phone it opened on. Reserving the chip's own size up front puts the
/// wrap decision back where the layout can make it, and the child then fits
/// exactly inside what was reserved.
pub fn chip_enabled(ui: &mut Ui, enabled: bool, selected: bool, label: &str) -> Response {
    let size = vec2(chip_width(ui, label, None), chip_height(ui, None));
    ui.allocate_ui(size, |ui| ui.add_enabled_ui(enabled, |ui| chip(ui, selected, label)).inner)
        .inner
}

/// [`chip_enabled`] with an explicit accent fill and text size — a transmit
/// control that greys out on a receive-only radio, in a row allowed to wrap.
///
/// The reservation is the point, for the reason [`chip_enabled`] gives: the
/// disabled chips in a wrapping row (WSPR's duty cycle among them) would
/// otherwise run off the edge instead of breaking at it.
pub fn chip_accent_enabled(
    ui: &mut Ui,
    enabled: bool,
    selected: bool,
    label: &str,
    size: Option<f32>,
    fill: Color32,
    ink: Color32,
) -> Response {
    let exact = vec2(chip_width(ui, label, size), chip_height(ui, size));
    let text = match size {
        Some(pt) => RichText::new(label).size(pt),
        None => RichText::new(label),
    };
    ui.allocate_ui(exact, |ui| {
        ui.add_enabled_ui(enabled, |ui| {
            chip_impl(ui, selected, text, Some(Accent::on_select(fill, ink)), Sense::click(), None)
        })
        .inner
    })
    .inner
}

/// [`chip_enabled`], with the label carrying a colour while it is unselected,
/// and an optional accent line along the chip's bottom edge.
///
/// For a row where each chip has a status of its own — the band menu, where
/// every band has published conditions — and the reader is choosing between
/// them. Only while unselected: a selected chip is already filled with the
/// accent, and a second colour inside it would be read as a different state
/// rather than the same one. `None` leaves the chip exactly as it was, which
/// is what a band with nothing published must look like.
///
/// The underline marks a chip with something to offer beyond its neighbours —
/// the band menu draws it under the bands where the current mode has a
/// standard frequency defined, against the ones a click merely takes to the
/// band. Drawn inside the chip rather than under it, so a wrapped row's
/// spacing does not have to make room for it, and stopping short of the cut
/// bottom-right corner so it follows the chip's own outline. On a selected
/// chip it vanishes into the accent fill, which reads correctly: selection
/// already says the chip has been taken up on the offer.
pub fn chip_enabled_tinted(
    ui: &mut Ui,
    enabled: bool,
    selected: bool,
    label: &str,
    tint: Option<Color32>,
    underline: bool,
) -> Response {
    let size = vec2(chip_width(ui, label, None), chip_height(ui, None));
    let text = match tint.filter(|_| !selected) {
        Some(c) => RichText::new(label).color(c),
        None => RichText::new(label),
    };
    let resp = ui
        .allocate_ui(size, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                chip_impl(ui, selected, text, None, Sense::click(), None)
            })
            .inner
        })
        .inner;
    if underline {
        let r = resp.rect;
        // Stop short of the cut corner so the line follows the chip's own
        // outline — only the Angled style has one to dodge.
        let cut = match theme::button_style() {
            ChromeStyle::Angled => CHIP_CUT.min(r.height() * 0.35),
            _ => 0.0,
        };
        let y = r.bottom() - 2.0;
        ui.painter().line_segment(
            [pos2(r.left() + 2.0, y), pos2(r.right() - cut, y)],
            Stroke::new(2.0, theme::CYAN()),
        );
    }
    resp
}

/// Chip with an explicit accent fill when selected (e.g. PTT red).
pub fn chip_accent(
    ui: &mut Ui,
    selected: bool,
    text: impl Into<RichText>,
    fill: Color32,
    ink: Color32,
) -> Response {
    chip_impl(ui, selected, text.into(), Some(Accent::on_select(fill, ink)), Sense::click(), None)
}

/// An accent chip that reports being *held* rather than clicked — for a control
/// that is on only while a finger or a mouse button is on it. Read the result
/// with [`Response::is_pointer_button_down_on`]: it goes false the moment the
/// press ends, including when the pointer is taken away entirely.
pub fn chip_hold(
    ui: &mut Ui,
    selected: bool,
    text: impl Into<RichText>,
    fill: Color32,
    ink: Color32,
) -> Response {
    chip_impl(
        ui,
        selected,
        text.into(),
        Some(Accent::on_select(fill, ink)),
        Sense::click_and_drag(),
        None,
    )
}

/// The classic chip shape: corners cut on the top-left and bottom-right (the
/// matching diagonal to the frames' top-right/bottom-left).
fn chip_outline(rect: Rect, cut: f32) -> Vec<Pos2> {
    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
    vec![
        pos2(l + cut, t),
        pos2(r, t),
        pos2(r, b - cut),
        pos2(r - cut, b),
        pos2(l, b),
        pos2(l, t + cut),
    ]
}

fn chip_impl(
    ui: &mut Ui,
    selected: bool,
    text: RichText,
    accent: Option<Accent>,
    sense: Sense,
    exact: Option<egui::Vec2>,
) -> Response {
    // The font [`chip_width`] measured with, not the button text style: the
    // two must be the same face or every row that budgets its chips up front —
    // the whole top strip — reserves the wrong width and overflows the box it
    // was packed into.
    let galley = WidgetText::from(text).into_galley(
        ui,
        None,
        f32::INFINITY,
        FontSelection::FontId(chip_font(ui, None)),
    );
    let padding = chip_padding(ui);
    let size = exact.unwrap_or(galley.size() + padding * 2.0);
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    if ui.is_rect_visible(rect) {
        let v = ui.style().interact_selectable(&resp, selected);

        // A lit chip carries its accent whether or not it is selected.
        let lit = accent.is_some_and(|a| a.lit);
        let (fill, stroke, ink) = if selected || lit {
            let (fill, ink) =
                accent.map_or((theme::CYAN(), theme::INK_ON_CYAN()), |a| (a.fill, a.ink));
            // A lit chip's colour is not reporting a state, so the colour is
            // not free to answer the pointer with: it has to move under the
            // cursor the way the plain chips beside it do, or it reads as
            // painted-on rather than pressable.
            let fill = if lit && !selected {
                if resp.is_pointer_button_down_on() {
                    lerp(fill, Color32::BLACK, 0.25)
                } else if resp.hovered() {
                    lerp(fill, Color32::WHITE, 0.20)
                } else {
                    fill
                }
            } else {
                fill
            };
            (fill, Stroke::new(1.0, fill), ink)
        } else {
            (v.bg_fill, v.bg_stroke, v.fg_stroke.color)
        };

        let p = ui.painter();
        match theme::button_style() {
            ChromeStyle::Angled => {
                let cut = CHIP_CUT.min(size.y * 0.35);
                p.add(Shape::convex_polygon(chip_outline(rect, cut), fill, stroke));
            }
            ChromeStyle::Rectangular => {
                p.rect(rect, CornerRadius::ZERO, fill, stroke, StrokeKind::Inside);
            }
            ChromeStyle::Rounded => {
                let radius = ROUND_CHIP_R.min(size.y * 0.35) as u8;
                p.rect(rect, CornerRadius::same(radius), fill, stroke, StrokeKind::Inside);
            }
            ChromeStyle::Gradient => {
                p.add(grad_rect(
                    rect,
                    lerp(fill, Color32::WHITE, 0.22),
                    lerp(fill, Color32::BLACK, 0.30),
                ));
                p.rect_stroke(rect, CornerRadius::ZERO, stroke, StrokeKind::Inside);
            }
            ChromeStyle::Bevel => {
                p.rect(rect, CornerRadius::ZERO, fill, stroke, StrokeKind::Inside);
                let rr = rect.shrink(0.75);
                let light = Stroke::new(1.5, lerp(fill, Color32::WHITE, 0.35));
                let dark = Stroke::new(1.5, lerp(fill, Color32::BLACK, 0.5));
                p.line_segment([rr.left_bottom(), rr.left_top()], light);
                p.line_segment([rr.left_top(), rr.right_top()], light);
                p.line_segment([rr.right_top(), rr.right_bottom()], dark);
                p.line_segment([rr.right_bottom(), rr.left_bottom()], dark);
            }
            ChromeStyle::Terminal => {
                // `[ LABEL ]`, and inverse video when it is on: those are the
                // two states a button has on a display with no shapes to draw
                // with. The brackets take the stroke colour, so a chip still
                // lights under the pointer the way its neighbours do.
                if selected || lit {
                    p.rect_filled(rect, CornerRadius::ZERO, fill);
                }
                let bracket = if selected || lit { ink } else { stroke.color };
                for shape in brackets(p, rect, bracket) {
                    p.add(shape);
                }
            }
        }

        let text_pos = Pos2 {
            x: rect.center().x - galley.size().x / 2.0,
            y: rect.center().y - galley.size().y / 2.0,
        };
        ui.painter().galley(text_pos, galley, ink);
    }
    resp
}

/// Corner cut — or radius, under the rounded styles — of a tab's two *top*
/// corners. The bottom two stay square: that edge is where the tab meets the
/// page it opens, and a folder tab is not shaped there.
const TAB_CUT: f32 = 6.0;
/// How far below the top of the strip an unselected tab sits. The step is what
/// says the selected tab is in front of the others rather than beside them.
const TAB_LIFT: f32 = 3.0;
/// The gap between neighbouring tabs. Small on purpose: a tab strip has to read
/// as one run of folder edges, not as a row of separate buttons.
const TAB_GAP: f32 = 2.0;

/// A tab's padding. Roomier than a chip's ([`chip_padding`]) in both
/// directions, because a tab is a place the operator goes rather than a button
/// they press — and because the extra height is what lets an unselected tab
/// drop by [`TAB_LIFT`] without its label ending up cramped against the top.
fn tab_padding(ui: &Ui) -> egui::Vec2 {
    ui.spacing().button_padding + vec2(6.0, 4.0)
}

/// The strip [`tab_bar`] is building.
///
/// It hands out the tabs and remembers where each one landed, so the baseline
/// can be drawn along the row afterwards with a gap under the selected one.
/// That gap is the whole point: it is the only mark that says *this* tab and
/// the page below it are the same surface. A row of chips cannot say that, and
/// a chip strip standing in for a tab strip — as the Settings dialog's did —
/// reads as a row of buttons that happen to stay pressed.
pub struct TabBar {
    /// Every tab drawn, in layout order, and whether it was the selected one.
    tabs: Vec<(Rect, bool)>,
    /// The strip's own left and right edge — how far its baseline runs. The
    /// right is `f32::MIN` where the width was not known (an unbounded
    /// measuring pass), and the widest row stands in for it.
    span: (f32, f32),
    /// The row's ordinary item spacing, kept for whatever follows the tabs —
    /// see [`TabBar::end_tabs`].
    spacing: egui::Vec2,
}

/// A row of tabs: the strip's tabs in a wrapping row, with the baseline drawn
/// under them afterwards, broken under the selected one.
///
/// The closure gets the strip as well as the `Ui`, so a bar may carry ordinary
/// widgets beside its tabs (the Winlink header's session buttons) — only what
/// is drawn through [`TabBar::tab`] is treated as a tab, and the baseline runs
/// on under everything else.
pub fn tab_bar<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui, &mut TabBar) -> R) -> R {
    let left = ui.cursor().left();
    let avail = ui.available_width();
    let mut bar = TabBar {
        tabs: Vec::new(),
        span: (left, if avail.is_finite() { left + avail } else { f32::MIN }),
        spacing: ui.spacing().item_spacing,
    };
    let inner = ui
        .horizontal_wrapped(|ui| {
            // Shoulder to shoulder, and stacked without a gap where the strip
            // wraps: a second row of tabs sits on the first one's baseline the
            // way a second row of folders sits in a drawer.
            ui.spacing_mut().item_spacing = vec2(TAB_GAP, 0.0);
            add(ui, &mut bar)
        })
        .inner;
    bar.paint_baseline(ui);
    inner
}

impl TabBar {
    /// One tab carrying `text`. Click it like a chip; it reports the same
    /// [`Response`].
    pub fn tab(&mut self, ui: &mut Ui, selected: bool, text: impl Into<RichText>) -> Response {
        let galley = WidgetText::from(text.into()).into_galley(
            ui,
            None,
            f32::INFINITY,
            FontSelection::FontId(chip_font(ui, None)),
        );
        let size = galley.size() + tab_padding(ui) * 2.0;
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        if ui.is_rect_visible(rect) {
            let face = tab_face(rect, selected);
            let p = ui.painter();
            let (shape, ink) = tab_shape(p, face, selected, resp.hovered(), theme::CYAN());
            p.add(shape);
            let pos = Pos2 {
                x: face.center().x - galley.size().x / 2.0,
                y: face.center().y - galley.size().y / 2.0,
            };
            p.galley(pos, galley, ink);
        }
        self.tabs.push((rect, selected));
        resp
    }

    /// A tab wide enough to hold `add` — for a strip whose tabs carry more than
    /// a label (the radio strip's mute and split-view buttons, which belong to
    /// their tab the way a close box belongs to a browser's).
    ///
    /// The returned response is the tab's own background, sensed *below* its
    /// contents: a click anywhere on the tab that did not land on a widget
    /// inside it comes back here, and the widgets keep their own clicks.
    pub fn tab_body<R>(
        &mut self,
        ui: &mut Ui,
        selected: bool,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        self.tab_body_accent(ui, selected, theme::CYAN(), add)
    }

    /// [`TabBar::tab_body`] in a colour of its own — for a strip where the
    /// selected tab has more than one state to report (the radio strip marks
    /// the pane holding the keyboard in pink, the rest in cyan).
    pub fn tab_body_accent<R>(
        &mut self,
        ui: &mut Ui,
        selected: bool,
        accent: Color32,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        // The face has to be painted under the content, and its size is only
        // known once the content has been laid out — so reserve the slot now
        // and fill it in afterwards.
        let slot = ui.painter().add(Shape::Noop);
        let pad = tab_padding(ui);
        let (x, y) = (pad.x.round() as i8, pad.y.round() as i8);
        let spacing = self.spacing;
        let out = ui.scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: x,
                    right: x,
                    // The lift comes out of the top margin, so a tab's content
                    // sits centred in the face rather than in the slot.
                    top: y + TAB_LIFT.round() as i8,
                    bottom: y,
                })
                .show(ui, |ui| {
                    // Inside the tab, ordinary spacing again: the strip's own
                    // shoulder-to-shoulder gap belongs between tabs, not
                    // between a tab's label and its buttons.
                    ui.spacing_mut().item_spacing = spacing;
                    // A tab's text is the name on a button, not a passage to
                    // select. egui's selectable labels take `click_and_drag`
                    // to run the selection, which swallows the click before
                    // the tab under them ever sees it — so the tab only
                    // switched when the pointer landed on the padding *around*
                    // its name.
                    ui.style_mut().interaction.selectable_labels = false;
                    add(ui)
                })
                .inner
        });
        let rect = out.response.rect;
        if ui.is_rect_visible(rect) {
            // `contains_pointer`, not `hovered`: the pointer resting on a tab's
            // own mute button is still on the tab, and a tab that unlit itself
            // there would read as two things side by side.
            let hovered = out.response.contains_pointer();
            let (shape, _) =
                tab_shape(ui.painter(), tab_face(rect, selected), selected, hovered, accent);
            ui.painter().set(slot, shape);
        }
        self.tabs.push((rect, selected));
        out
    }

    /// End the run of tabs: puts the row's ordinary spacing back and leaves a
    /// gap, for a strip that carries buttons beside its tabs. Without it they
    /// would inherit the tabs' shoulder-to-shoulder spacing and read as part of
    /// the strip.
    pub fn end_tabs(&self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = self.spacing;
        ui.add_space(10.0);
    }

    /// The line along the bottom of each row of tabs — the top edge of the page
    /// the strip opens onto, drawn everywhere except under the selected tab.
    fn paint_baseline(&self, ui: &Ui) {
        let right = self.tabs.iter().map(|(r, _)| r.right()).fold(self.span.1, f32::max);
        let stroke = Stroke::new(1.2, theme::CYAN_DIM());
        let p = ui.painter();
        let terminal = theme::button_style() == ChromeStyle::Terminal;
        for (y, segments) in baseline_segments(&self.tabs, (self.span.0, right)) {
            for (a, b) in segments {
                if terminal {
                    // The page's top edge is a run of `-`, so it joins the
                    // tabs' own frames instead of cutting across them.
                    let size = terminal_size(p);
                    let ch = cell_size(p, size).y;
                    let rule = Rect::from_min_max(pos2(a, y - ch / 2.0), pos2(b, y + ch / 2.0));
                    for shape in cell_run(p, rule, "-", stroke.color, size) {
                        p.add(shape);
                    }
                } else {
                    p.hline(a..=b, y, stroke);
                }
            }
        }
    }
}

/// Where the baseline runs under a strip of tabs: one entry per row of tabs,
/// keyed by that row's bottom edge, each holding the x segments that are drawn
/// — everything between `span`'s two edges except the selected tab.
///
/// Pure arithmetic so the join can be tested without a painter: the gap under
/// the selected tab is the one thing about a tab strip that must not break.
fn baseline_segments(tabs: &[(Rect, bool)], span: (f32, f32)) -> Vec<(f32, Vec<(f32, f32)>)> {
    // Rows in the order they were drawn, each with the selected tab's span (a
    // wrapped strip has one row per line, and only one of them has the gap).
    let mut rows: Vec<(f32, Vec<(f32, f32)>)> = Vec::new();
    for (rect, selected) in tabs {
        let y = rect.bottom();
        let row = match rows.iter().position(|(ry, _)| (*ry - y).abs() < 0.5) {
            Some(i) => i,
            None => {
                rows.push((y, Vec::new()));
                rows.len() - 1
            }
        };
        if *selected {
            rows[row].1.push((rect.left(), rect.right()));
        }
    }
    rows.into_iter()
        .map(|(y, gaps)| {
            let mut segments = Vec::new();
            let mut x = span.0;
            for (l, r) in gaps {
                if l > x {
                    segments.push((x, l));
                }
                x = x.max(r);
            }
            if span.1 > x {
                segments.push((x, span.1));
            }
            (y, segments)
        })
        .collect()
}

/// The rect a tab is actually painted in, given the slot it was allocated: the
/// selected one fills its slot and reaches a point past the baseline so the
/// join is seamless; the others start [`TAB_LIFT`] lower, behind it.
fn tab_face(rect: Rect, selected: bool) -> Rect {
    if selected {
        Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() + 1.0))
    } else {
        Rect::from_min_max(pos2(rect.left(), rect.top() + TAB_LIFT), rect.max)
    }
}

/// The outline of a tab in the current button style: up the left edge, across
/// the top, down the right — and *open* along the bottom, which is what a
/// closed rect cannot express. Filled as a polygon (the fill closes itself
/// along the bottom) and stroked as a line, so no border is ever drawn across
/// the join with the page.
fn tab_outline(rect: Rect, style: ChromeStyle) -> Vec<Pos2> {
    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
    let cut = TAB_CUT.min(rect.height() * 0.4).min(rect.width() * 0.35);
    match style {
        ChromeStyle::Angled => vec![
            pos2(l, b),
            pos2(l, t + cut),
            pos2(l + cut, t),
            pos2(r - cut, t),
            pos2(r, t + cut),
            pos2(r, b),
        ],
        ChromeStyle::Rounded => {
            let mut pts = vec![pos2(l, b)];
            quarter_turn(&mut pts, pos2(l + cut, t + cut), cut, std::f32::consts::PI);
            quarter_turn(&mut pts, pos2(r - cut, t + cut), cut, 1.5 * std::f32::consts::PI);
            pts.push(pos2(r, b));
            pts
        }
        _ => vec![pos2(l, b), pos2(l, t), pos2(r, t), pos2(r, b)],
    }
}

/// Points along a quarter circle of `radius` about `center`, from `from`
/// radians clockwise on screen (y grows downwards).
fn quarter_turn(out: &mut Vec<Pos2>, center: Pos2, radius: f32, from: f32) {
    const STEPS: usize = 5;
    for k in 0..=STEPS {
        let a = from + std::f32::consts::FRAC_PI_2 * k as f32 / STEPS as f32;
        out.push(center + vec2(a.cos() * radius, a.sin() * radius));
    }
}

/// The shape a tab wears, and the colour its label is drawn in.
///
/// The selected tab is filled with the *page's* own colour and outlined in the
/// accent, so it and the page below read as one surface; the rest are darker
/// than the page and outlined in a hairline, so they read as sitting behind it.
/// That difference — not the fill alone — is what tells a tab from a chip.
fn tab_shape(
    p: &Painter,
    face: Rect,
    selected: bool,
    hovered: bool,
    accent: Color32,
) -> (Shape, Color32) {
    let (fill, line, ink) = if selected {
        (theme::PANEL(), accent, theme::TEXT_STRONG())
    } else if hovered {
        (theme::FILL(), theme::CYAN_DIM(), theme::TEXT_STRONG())
    } else {
        (theme::BG_DEEP(), theme::LINE_LIT(), theme::TEXT())
    };
    let style = theme::button_style();
    let pts = tab_outline(face, style);
    let stroke = Stroke::new(1.2, line);
    let mut shapes = Vec::new();
    match style {
        ChromeStyle::Terminal => {
            // A folder edge drawn from characters: `+---+` over the top and a
            // `|` down each side, open along the bottom where the tab meets
            // the page. The accent goes into the frame itself rather than
            // into a bar over it — a bar would land on the top rule.
            shapes.push(Shape::convex_polygon(pts, fill, Stroke::NONE));
            shapes.extend(ascii_box(p, face, line, terminal_size(p), Sides::OpenBottom));
            return (Shape::Vec(shapes), ink);
        }
        ChromeStyle::Gradient => {
            shapes.push(grad_rect(face, lerp(fill, Color32::WHITE, 0.18), fill));
            shapes.push(Shape::line(pts, stroke));
        }
        ChromeStyle::Bevel => {
            shapes.push(Shape::convex_polygon(pts, fill, Stroke::NONE));
            let rr = face.shrink(0.75);
            let light = Stroke::new(1.5, lerp(fill, Color32::WHITE, 0.35));
            let dark = Stroke::new(1.5, lerp(fill, Color32::BLACK, 0.5));
            shapes.push(Shape::line_segment([rr.left_bottom(), rr.left_top()], light));
            shapes.push(Shape::line_segment([rr.left_top(), rr.right_top()], light));
            shapes.push(Shape::line_segment([rr.right_top(), rr.right_bottom()], dark));
        }
        _ => {
            shapes.push(Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
            shapes.push(Shape::line(pts, stroke));
        }
    }
    // The accent bar across the top of the selected tab: the strip's one bright
    // mark, so which page is open survives a glance from across the room.
    if selected {
        let cut = TAB_CUT.min(face.height() * 0.4).min(face.width() * 0.35);
        let y = face.top() + 1.6;
        shapes.push(Shape::line_segment(
            [pos2(face.left() + cut, y), pos2(face.right() - cut, y)],
            Stroke::new(2.2, accent),
        ));
    }
    (Shape::Vec(shapes), ink)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Press Return with `modifiers` into a fresh context and report whether
    /// [`take_return`] claimed it, and how many events were left behind for the
    /// text box that draws afterwards.
    fn press_return(modifiers: egui::Modifiers, focused: bool) -> (bool, usize) {
        let ctx = egui::Context::default();
        let id = egui::Id::new("a-transmit-box");
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        };
        let (mut took, mut left) = (false, 0);
        ctx.run_ui(input, |ui| {
            if focused {
                ui.memory_mut(|m| m.request_focus(id));
            }
            took = take_return(ui, id);
            left = ui.input(|i| i.events.len());
        })
        .drop_without_applying_deltas();
        (took, left)
    }

    /// Return sends, Shift+Return breaks the line. egui's own shortcut matching
    /// ignores a stray Shift, so taking the key with it would leave no way to
    /// type a newline at all — and the box would sit there eating the
    /// combination while doing nothing with it.
    #[test]
    fn only_a_bare_return_is_taken_from_the_box() {
        assert_eq!(press_return(egui::Modifiers::NONE, true), (true, 0));
        assert_eq!(
            press_return(egui::Modifiers::SHIFT, true),
            (false, 1),
            "Shift+Return was swallowed, so the box could not break a line"
        );
        // A Return typed with the focus somewhere else belongs to whatever has
        // it, not to a transmit box that is merely on the same panel.
        assert_eq!(press_return(egui::Modifiers::NONE, false), (false, 1));
    }

    /// Lay a single module out in a fresh context and return how tall it made
    /// the row, along with whatever height it handed its own content.
    fn module_height(height: f32, flush: bool) -> (f32, f32) {
        let ctx = egui::Context::default();
        let (mut row, mut content) = (0.0, 0.0);
        ctx.run_ui(Default::default(), |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                let grab = |ui: &mut Ui| content = ui.available_size().y;
                if flush {
                    module_bare_flush_h(ui, 100.0, height, grab);
                } else {
                    module_bare_h(ui, 100.0, height, grab);
                }
                row = ui.min_rect().height();
            });
        })
        .drop_without_applying_deltas();
        (row, content)
    }

    /// The S-meter's box has no frame stroke — it paints its own border inside
    /// its content — so it has to allocate the two points a bordered module
    /// gets for free from the stroke egui draws outside the content it was
    /// handed. Without them the meter stands two points shorter than every box
    /// beside it, which reads as a row out of alignment rather than as rounding.
    #[test]
    fn a_flush_module_is_exactly_as_tall_as_a_bordered_one() {
        for height in [50.0, MODULE_TALL_H, 40.0] {
            let (bordered, _) = module_height(height, false);
            let (flush, content) = module_height(height, true);
            assert_eq!(bordered, flush, "at {height} pt: bordered {bordered}, flush {flush}");
            // And the meter really does get the whole of it to paint in.
            assert_eq!(content, flush, "the flush box handed its content {content} of {flush}");
        }
    }

    /// The figure the box heights are quoted in: a module built to `height`
    /// stands that tall plus its border. If egui ever stops drawing a frame's
    /// stroke outside the content, this is the assumption that breaks.
    #[test]
    fn a_bordered_module_stands_its_height_plus_its_border() {
        let (row, content) = module_height(MODULE_TALL_H, false);
        assert_eq!(row, MODULE_TALL_H + 2.0 * MODULE_BORDER, "outer height moved");
        // Inner margin of 4 above and 5 below, inside the border.
        assert_eq!(content, MODULE_TALL_H - 9.0, "content height moved");
    }

    /// Stand-in for a menu with more in it than any phone can show at once.
    fn a_long_menu(ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            for i in 0..60 {
                chip(ui, false, format!("CHIP{i}"));
            }
        });
    }

    /// Open a menu popup from a chip on a `screen`-sized viewport and return the
    /// rect it took.
    fn menu_popup_rect(screen: egui::Vec2) -> Rect {
        let ctx = egui::Context::default();
        let tier = crate::layout::tier_for(screen, sdroxide_types::LayoutMode::Auto);
        crate::layout::set_tier(&ctx, tier);
        theme::apply_metrics(&ctx, tier);
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen)),
            ..Default::default()
        };
        // First pass to give the chip an id, then open its popup and lay it out.
        let mut id = None;
        ctx.run_ui(input(), |ui| {
            let btn = chip(ui, false, "MENU");
            id = Some(egui::Popup::default_response_id(&btn));
            menu_popup(ui, &btn, a_long_menu);
        })
        .drop_without_applying_deltas();
        let id = id.expect("the chip was drawn");
        egui::Popup::open_id(&ctx, id);
        ctx.run_ui(input(), |ui| {
            let btn = chip(ui, false, "MENU");
            menu_popup(ui, &btn, a_long_menu);
        })
        .drop_without_applying_deltas();
        ctx.memory(|m| m.area_rect(id)).expect("the popup was shown")
    }

    /// A popup is the one thing here not laid out inside a panel: egui will move
    /// one that lands off the edge, but it cannot shrink one that is too big for
    /// the screen — it simply hangs off it, and the part that hangs off cannot
    /// be reached at all. Every menu therefore has to come out no larger than
    /// the phone it opened on, however much content it was handed.
    #[test]
    fn a_menu_popup_fits_the_screen_it_opens_on() {
        // Phones, portrait and landscape, plus a tablet for company.
        for screen in
            [vec2(360.0, 800.0), vec2(393.0, 852.0), vec2(852.0, 393.0), vec2(768.0, 1024.0)]
        {
            let r = menu_popup_rect(screen);
            assert!(r.width() <= screen.x, "{screen:?}: popup {} pt wide", r.width());
            assert!(r.height() <= screen.y, "{screen:?}: popup {} pt tall", r.height());
            assert!(
                r.left() >= 0.0 && r.right() <= screen.x,
                "{screen:?}: popup spans {}..{}",
                r.left(),
                r.right()
            );
        }
    }

    /// A row of tabs `w` wide and `h` tall starting at x = 10, with tab
    /// `selected` picked, wrapping into a new row every `per_row` of them.
    fn a_strip(count: usize, selected: usize, per_row: usize) -> Vec<(Rect, bool)> {
        let (w, h) = (60.0, 24.0);
        (0..count)
            .map(|i| {
                let (col, row) = (i % per_row, i / per_row);
                let min = pos2(10.0 + col as f32 * w, 5.0 + row as f32 * h);
                (Rect::from_min_size(min, vec2(w, h)), i == selected)
            })
            .collect()
    }

    /// The gap under the selected tab is the whole of what makes a tab strip a
    /// tab strip: it is the only mark that says the tab and the page below it
    /// are one surface. Everything either side of it is drawn.
    #[test]
    fn the_baseline_breaks_under_the_selected_tab() {
        let tabs = a_strip(4, 1, 4);
        let rows = baseline_segments(&tabs, (10.0, 400.0));
        assert_eq!(rows.len(), 1, "one row of tabs, one baseline");
        let (y, segments) = &rows[0];
        assert_eq!(*y, 29.0, "the baseline runs along the bottom of the tabs");
        assert_eq!(segments, &vec![(10.0, 70.0), (130.0, 400.0)], "tab 1 spans 70..130");
    }

    /// A selected tab at either end leaves one segment, not an empty one of
    /// zero width — and the line still runs to the far edge of the strip, past
    /// the last tab, because that is where the page below ends.
    #[test]
    fn a_baseline_has_no_empty_segments_at_the_ends() {
        for (pick, want) in
            [(0usize, vec![(70.0, 400.0)]), (3, vec![(10.0, 190.0), (250.0, 400.0)])]
        {
            let rows = baseline_segments(&a_strip(4, pick, 4), (10.0, 400.0));
            assert_eq!(rows[0].1, want, "with tab {pick} selected");
        }
    }

    /// The Settings strip wraps to two lines on a narrow window. Each line is a
    /// row of tabs in its own right and gets its own baseline — with the gap on
    /// whichever line the selected tab landed on.
    #[test]
    fn a_wrapped_strip_gets_a_baseline_per_row() {
        let rows = baseline_segments(&a_strip(6, 4, 3), (10.0, 400.0));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, vec![(10.0, 400.0)], "the first row is unbroken");
        assert_eq!(rows[1].1, vec![(10.0, 70.0), (130.0, 400.0)], "tab 4 is the second of its row");
    }

    /// A tab is open at the bottom in every chrome style: that edge is the join
    /// with the page, and a border across it would draw the tab shut. The
    /// outline therefore has to start and end on the bottom edge, and touch it
    /// nowhere in between.
    #[test]
    fn every_tab_outline_is_open_along_its_bottom_edge() {
        let rect = Rect::from_min_max(pos2(10.0, 20.0), pos2(90.0, 50.0));
        for style in ChromeStyle::ALL {
            let pts = tab_outline(rect, style);
            assert_eq!(pts.first(), Some(&pos2(10.0, 50.0)), "{style:?} starts bottom-left");
            assert_eq!(pts.last(), Some(&pos2(90.0, 50.0)), "{style:?} ends bottom-right");
            assert!(
                pts[1..pts.len() - 1].iter().all(|p| p.y < 50.0),
                "{style:?} runs back along its own bottom edge"
            );
            assert!(
                pts.iter().all(|p| rect.expand(0.01).contains(*p)),
                "{style:?} paints outside the tab"
            );
        }
    }

    /// The selected tab reaches past the baseline so the two surfaces meet with
    /// no seam; an unselected one starts lower, which is the step that says it
    /// is behind. Both keep the bottom edge they were allocated, so the whole
    /// row shares one baseline.
    #[test]
    fn a_selected_tab_stands_taller_than_its_neighbours() {
        let slot = Rect::from_min_max(pos2(10.0, 20.0), pos2(90.0, 50.0));
        let (on, off) = (tab_face(slot, true), tab_face(slot, false));
        assert!(on.top() < off.top(), "the selected tab is the taller one");
        assert_eq!(off.top() - on.top(), TAB_LIFT);
        assert!(on.bottom() > slot.bottom(), "the selected tab crosses the baseline");
        assert_eq!(off.bottom(), slot.bottom());
    }

    /// One frame of a one-tab strip carrying a chip, with the pointer clicking
    /// at `pos`. Returns where the tab and the chip landed, and which of them
    /// took the click.
    fn strip_click(ctx: &egui::Context, pos: Option<Pos2>) -> (Rect, Rect, bool, bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
            ..Default::default()
        };
        if let Some(pos) = pos {
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            input.events = vec![egui::Event::PointerMoved(pos), button(true), button(false)];
        }
        let (mut tab, mut inner) = (Rect::NOTHING, Rect::NOTHING);
        let (mut tab_hit, mut chip_hit) = (false, false);
        ctx.run_ui(input, |ui| {
            tab_bar(ui, |ui, bar| {
                let out = bar.tab_body(ui, true, |ui| {
                    ui.label("Radio 1");
                    let c = chip(ui, false, "🔊");
                    inner = c.rect;
                    chip_hit = c.clicked();
                });
                tab = out.response.rect;
                tab_hit = out.response.clicked();
            });
        })
        .drop_without_applying_deltas();
        (tab, inner, tab_hit, chip_hit)
    }

    /// The radio strip's tabs carry buttons of their own (mute, split view). A
    /// click on one of those belongs to the button; a click anywhere else on
    /// the tab switches to that radio. Getting this backwards would mute a
    /// radio every time the operator reached for its tab — the tab's own sense
    /// has to sit *below* its contents.
    #[test]
    fn a_tab_body_yields_its_clicks_to_the_buttons_inside_it() {
        let ctx = egui::Context::default();
        // A first frame to place everything: egui hit-tests against the layout
        // of the frame before.
        let (tab, chip, ..) = strip_click(&ctx, None);
        assert!(tab.contains(chip.center()), "the chip is inside its tab");

        let (.., tab_hit, chip_hit) = strip_click(&ctx, Some(chip.center()));
        assert!(chip_hit, "the chip did not get its own click");
        assert!(!tab_hit, "the tab stole its chip's click");

        let elsewhere = pos2(tab.left() + 3.0, tab.center().y);
        let (.., tab_hit, chip_hit) = strip_click(&ctx, Some(elsewhere));
        assert!(tab_hit, "clicking the tab itself did nothing");
        assert!(!chip_hit);
    }

    /// Every style has to keep what it paints inside the box the widget was
    /// allocated. A reskin that spilled would draw over the control beside it —
    /// and the operator would have no idea which widget the stray mark
    /// belonged to.
    #[test]
    fn a_reskinned_surface_stays_inside_the_widget() {
        // Tall enough for a one-line control, and again for one with room for
        // a character frame — the Terminal style draws those two differently.
        for h in [22.0f32, 90.0] {
            let rect = Rect::from_min_max(pos2(20.0, 30.0), pos2(140.0, 30.0 + h));
            let base = egui::epaint::RectShape::new(
                rect,
                CornerRadius::same(3),
                Color32::from_gray(20),
                Stroke::new(1.0, Color32::from_gray(90)),
                StrokeKind::Inside,
            );
            let ctx = egui::Context::default();
            ctx.run_ui(Default::default(), |ui| {
                for style in ChromeStyle::ALL {
                    // The Terminal style frames a control with characters
                    // straddling its edge, so its *cells* reach a character
                    // out either way — the ink inside them does not. Every
                    // other style has only a stroke to spare.
                    let slack = match style {
                        ChromeStyle::Terminal => {
                            cell_size(ui.painter(), terminal_size(ui.painter()))
                        }
                        _ => egui::Vec2::splat(3.0),
                    };
                    for depth in [Depth::Raised, Depth::Recessed, Depth::Knob] {
                        let got = styled_surface(ui, &base, style, depth).visual_bounding_rect();
                        assert!(
                            rect.expand2(slack).contains_rect(got),
                            "{style:?} {depth:?} at {h} pt painted {got:?} outside {rect:?}"
                        );
                        assert!(got.is_positive(), "{style:?} painted nothing at all");
                    }
                }
            })
            .drop_without_applying_deltas();
        }
    }

    /// A window's border under the Terminal style has to land *on* the edge:
    /// the frame is the only thing that says where the window ends, and one
    /// drawn a whole character inside it reads as a box floating on the panel
    /// rather than as the panel's own outline. It may hang over the edge by
    /// the border cell itself and no further — the ink of a `-`, a `|` and a
    /// `+` all sit in the middle of their own cell, so what hangs over is
    /// blank.
    #[test]
    fn the_terminal_border_sits_on_the_edge_it_frames() {
        let rect = Rect::from_min_max(pos2(40.0, 60.0), pos2(340.0, 220.0));
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            let p = ui.painter();
            let size = terminal_size(p);
            let cell = cell_size(p, size);
            let ink = ascii_box(p, rect, Color32::WHITE, size, Sides::All)
                .iter()
                .fold(Rect::NOTHING, |acc, s| acc.union(s.visual_bounding_rect()));
            assert!(ink.min.x < rect.left() && ink.min.y < rect.top(), "{ink:?} vs {rect:?}");
            assert!(ink.max.x > rect.right() && ink.max.y > rect.bottom(), "{ink:?} vs {rect:?}");
            assert!(rect.expand2(cell).contains_rect(ink), "{ink:?} spills past {rect:?}");
        })
        .drop_without_applying_deltas();
    }

    /// Same for a slider's knob: it is drawn at the value's position, and a
    /// knob that grew would sit over the rail's own ends.
    #[test]
    fn a_reskinned_knob_stays_the_size_of_its_disc() {
        let disc = egui::epaint::CircleShape {
            center: pos2(80.0, 40.0),
            radius: 7.0,
            fill: Color32::from_gray(30),
            stroke: Stroke::new(1.0, Color32::WHITE),
        };
        let bounds = Rect::from_center_size(disc.center, egui::Vec2::splat(2.0 * disc.radius));
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            for style in ChromeStyle::ALL {
                let got = styled_knob(ui, &disc, style).visual_bounding_rect();
                assert!(bounds.expand(3.0).contains_rect(got), "{style:?} painted {got:?}");
            }
        })
        .drop_without_applying_deltas();
    }

    /// Lay one checkbox out in a fresh context and report the size it took and
    /// whether the click at its centre toggled it.
    fn one_checkbox(label: &str, stock: bool) -> (egui::Vec2, bool) {
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0));
        let mut on = false;
        let (mut size, mut hit) = (egui::Vec2::ZERO, Pos2::ZERO);
        let mut input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
        for pass in 0..2 {
            ctx.run_ui(input.clone(), |ui| {
                let r =
                    if stock { ui.checkbox(&mut on, label) } else { checkbox(ui, &mut on, label) };
                size = r.rect.size();
                hit = r.rect.center();
            })
            .drop_without_applying_deltas();
            if pass == 0 {
                // egui hit-tests against the layout of the frame before, so the
                // click can only be aimed once the box has been placed.
                let button = |pressed| egui::Event::PointerButton {
                    pos: hit,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                };
                input.events = vec![egui::Event::PointerMoved(hit), button(true), button(false)];
            }
        }
        (size, on)
    }

    /// The hand-drawn checkbox stands in for egui's everywhere in the program,
    /// so it has to take the same room: a column of settings rows lines its
    /// labels up against the boxes, and a control a few points wider or
    /// shorter than the one it replaced knocks every row out of true.
    #[test]
    fn the_checkbox_matches_the_stock_one_it_replaced() {
        for label in ["", "Report stations I decode"] {
            let (ours, toggled) = one_checkbox(label, false);
            let (stock, _) = one_checkbox(label, true);
            assert!(toggled, "clicking the box did not tick it");
            assert!(
                (ours.y - stock.y).abs() < 0.5 && (ours.x - stock.x).abs() < 4.0,
                "{label:?}: ours {ours:?} against egui's {stock:?}"
            );
        }
    }

    /// A chip has to come out exactly as wide as [`chip_width`] said it
    /// would. The top strip budgets every row from that figure before a chip
    /// is drawn, and a chip wider than its budget is not clipped to its box —
    /// it pushes the boxes after it along the row until the last one goes off
    /// the edge of the window. That is what the Terminal style did while it
    /// measured in the character face and then drew in the proportional one;
    /// both now come from [`chip_font`], and this holds them there for
    /// whichever style the process is wearing.
    #[test]
    fn a_chip_is_as_wide_as_it_was_measured() {
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            for label in ["LOG", "AWARDS", "\u{2699} SETTINGS", "20M \u{b7} USB"] {
                for size in [None, Some(15.0)] {
                    let want = (chip_width(ui, label, size), chip_height(ui, size));
                    let text = match size {
                        Some(pt) => RichText::new(label).size(pt),
                        None => RichText::new(label),
                    };
                    let got = chip_impl(ui, false, text, None, Sense::click(), None).rect.size();
                    assert!(
                        (want.0 - got.x).abs() < 0.5 && (want.1 - got.y).abs() < 0.5,
                        "{label:?} at {size:?}: measured {want:?}, drew {got:?}"
                    );
                }
            }
        })
        .drop_without_applying_deltas();
    }

    /// The Angled chip outline is the app's signature shape and must never
    /// drift: these are the six points [`chip_impl`] has always painted.
    #[test]
    fn angled_chip_outline_is_the_historic_shape() {
        let rect = Rect::from_min_max(pos2(10.0, 20.0), pos2(80.0, 44.0));
        let cut = CHIP_CUT; // 24 pt tall, so the 0.35 height cap does not bite
        assert_eq!(
            chip_outline(rect, cut),
            vec![
                pos2(15.0, 20.0),
                pos2(80.0, 20.0),
                pos2(80.0, 39.0),
                pos2(75.0, 44.0),
                pos2(10.0, 44.0),
                pos2(10.0, 25.0),
            ]
        );
    }
}
