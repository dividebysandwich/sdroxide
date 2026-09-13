//! The spectrum-painting panel: draw text or an image onto the waterfall.
//!
//! [`RfPaintUi`] caches the two sources the operator can paint from — the
//! rasterised text and a picked image — along with the scrolling previews that
//! show what the receiving station's waterfall will look like.

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::Command;

use crate::app::panels::widgets::{pick_image, sstv_section};
use crate::app::{SdroxideApp, rx_only_hint};

// ───────────────────────── RF Paint panel ──────────────────────────────

/// RF Paint (Spectrum Painting) panel state: the text to paint, a loaded image
/// (as a bounded grayscale bitmap), and the colour-mapped preview textures shown
/// in the two scrolling "preview waterfall" boxes.
pub(in crate::app) struct RfPaintUi {
    /// The line of text to paint.
    text: String,
    /// The text the current preview texture was built from (rebuild on change).
    text_built_for: String,
    /// Grayscale, vertically-tiling preview of the text banner.
    text_prev: Option<egui::TextureHandle>,
    /// Loaded source image as a grayscale bitmap (bounded to the paint size).
    img_gray: Option<(Vec<u8>, u16, u16)>,
    /// Grayscale display of the loaded image (for the image box).
    img_disp: Option<egui::TextureHandle>,
    /// Grayscale, vertically-tiling preview of the image.
    img_prev: Option<egui::TextureHandle>,
    img_dirty: bool,
    /// File-picker result inbox (raw image bytes), filled by the picker task.
    inbox: Arc<Mutex<Option<Vec<u8>>>>,
}

impl Default for RfPaintUi {
    fn default() -> Self {
        RfPaintUi {
            text: String::new(),
            text_built_for: "\0".to_string(), // sentinel != "" so an empty box builds once
            text_prev: None,
            img_gray: None,
            img_disp: None,
            img_prev: None,
            img_dirty: true,
            inbox: Arc::new(Mutex::new(None)),
        }
    }
}

impl RfPaintUi {
    /// Rebuild the preview textures whose source (text or image) changed since
    /// the last frame.
    fn ensure(&mut self, ctx: &egui::Context) {
        // Text banner preview.
        if self.text_built_for != self.text {
            self.text_built_for = self.text.clone();
            match crate::rf_paint::text_bitmap(&self.text) {
                Some((gray, w, h)) => {
                    let ci = crate::rf_paint::preview_gray_image(&gray, w, h);
                    self.text_prev = Some(load_scroll_tex(ctx, "rfpaint_text", ci));
                }
                None => {
                    self.text_prev = None;
                }
            }
        }
        // Image preview + grayscale display.
        if self.img_dirty {
            self.img_dirty = false;
            match &self.img_gray {
                Some((gray, w, h)) => {
                    let ci = crate::rf_paint::preview_gray_image(gray, *w, *h);
                    self.img_prev = Some(load_scroll_tex(ctx, "rfpaint_img_prev", ci));
                    // Natural grayscale for the image box.
                    let (iw, ih) = (*w as usize, *h as usize);
                    let mut disp = egui::ColorImage::new([iw, ih], vec![Color32::BLACK; iw * ih]);
                    for (px, &v) in disp.pixels.iter_mut().zip(gray.iter()) {
                        *px = Color32::from_gray(v);
                    }
                    self.img_disp = Some(ctx.load_texture(
                        "rfpaint_img_disp",
                        disp,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                None => {
                    self.img_prev = None;
                    self.img_disp = None;
                }
            }
        }
    }
}

/// Load a preview texture that tiles vertically (so the scrolling waterfall loops
/// seamlessly) with crisp pixel edges.
fn load_scroll_tex(ctx: &egui::Context, name: &str, ci: egui::ColorImage) -> egui::TextureHandle {
    let mut opt = egui::TextureOptions::NEAREST;
    opt.wrap_mode = egui::TextureWrapMode::Repeat;
    ctx.load_texture(name, ci, opt)
}

/// Draw a scrolling "preview waterfall" of `tex` in a fixed box: the picture
/// (frequency across the width) slides downward like a live waterfall.
fn draw_scroll_preview(
    ui: &mut egui::Ui,
    tex: Option<&egui::TextureHandle>,
    size: egui::Vec2,
    time: f64,
    empty_hint: &str,
) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, Color32::from_gray(8));
    match tex {
        Some(tex) => {
            // Frequency (texture width) fills the box width; time (texture height)
            // scrolls through a window sized so the pixels stay roughly square, so
            // a long banner scrolls by at a readable scale instead of squashing to
            // fit. Newest data enters at the top and flows down (the top of the
            // picture renders first), like a standard waterfall; the scroll speed
            // is a constant rows/sec.
            let [tw, th] = tex.size();
            let (tw, th) = (tw.max(1) as f32, th.max(1) as f32);
            let px_per_bin = rect.width() / tw;
            let rows_visible = (rect.height() / px_per_bin).max(1.0);
            let vspan = (rows_visible / th).min(1.0);
            // Decreasing offset scrolls the sampled window up in texture space, so
            // content moves down on screen (newest at top).
            let off = (-(time as f32) * 45.0 / th).rem_euclid(1.0);
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, off), egui::pos2(1.0, off + vspan));
            egui::Image::new(tex).uv(uv).paint_at(ui, rect);
            crate::repaint::animate(ui.ctx()); // keep the scroll animating
        }
        None => {
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                empty_hint,
                egui::FontId::proportional(11.0),
                Color32::from_gray(90),
            );
        }
    }
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::LINE_LIT()),
        egui::StrokeKind::Inside,
    );
}

/// Draw a static, aspect-preserved image box (the loaded picture, or a hint).
fn draw_image_box(
    ui: &mut egui::Ui,
    tex: Option<&egui::TextureHandle>,
    dims: Option<(u16, u16)>,
    size: egui::Vec2,
    empty_hint: &str,
) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, Color32::from_gray(10));
    if let (Some(tex), Some((w, h))) = (tex, dims) {
        let inner = rect.shrink(3.0);
        let ar = w as f32 / h.max(1) as f32;
        let (mut fw, mut fh) = (inner.width(), inner.width() / ar);
        if fh > inner.height() {
            fh = inner.height();
            fw = inner.height() * ar;
        }
        let fr = egui::Rect::from_center_size(inner.center(), egui::vec2(fw, fh));
        egui::Image::new(tex).paint_at(ui, fr);
    } else {
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            empty_hint,
            egui::FontId::proportional(11.0),
            Color32::from_gray(90),
        );
    }
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::LINE_LIT()),
        egui::StrokeKind::Inside,
    );
}

impl SdroxideApp {
    pub(in crate::app) fn rf_paint_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        _panel_h: f32,
    ) {
        let ctx = ui.ctx().clone();
        let time = ctx.input(|i| i.time);

        // Absorb a freshly-picked image file.
        if let Some(bytes) = self.rf_paint.inbox.lock().ok().and_then(|mut g| g.take()) {
            self.rf_paint.img_gray = crate::rf_paint::image_bitmap(&bytes);
            self.rf_paint.img_dirty = true;
        }
        self.rf_paint.ensure(&ctx);

        if self.ui_settings.swl {
            egui::Frame::new()
                .fill(crate::theme::ROW_BG())
                .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
                .inner_margin(egui::Margin { left: 10, right: 8, top: 6, bottom: 7 })
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("RF Paint is a transmit-only mode — nothing to show in SWL.")
                            .size(11.0)
                            .weak(),
                    );
                });
            return;
        }

        let status = self.digi_status.clone();
        let transmitting = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        let progress =
            status.as_ref().map(|s| (s.tx_sent as f32 / 1000.0).clamp(0.0, 1.0)).unwrap_or(0.0);
        // Both halves of this panel exist only to paint something onto the
        // band, but composing and previewing still work on a receiver — it is
        // the two TRANSMIT buttons that go grey.
        let tx_ok = self.tx_capable();

        // Header: title, transmit-speed slider, and the transmit indicator.
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("RF PAINT").size(11.0).strong().color(crate::theme::CYAN()));
            ui.add_space(12.0);
            ui.label(RichText::new("Scan speed").size(10.5).color(crate::theme::CYAN_DIM()));
            let mut speed = self.digi_cfg_edit.rf_paint_speed;
            ui.spacing_mut().slider_width = 150.0;
            let resp = crate::chrome::slider(
                ui,
                egui::Slider::new(&mut speed, 0.0625..=1.0)
                    .logarithmic(true)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .custom_parser(|s| {
                        s.trim().trim_end_matches('%').parse::<f64>().ok().map(|p| p / 100.0)
                    }),
            )
            .on_hover_text(
                "How fast the text/image is scanned onto the waterfall. Lower is slower and \
                     more legible; 100% = base rate, 25% (centre) is the default.",
            );
            if resp.changed() {
                self.digi_cfg_edit.rf_paint_speed = speed;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
            crate::chrome::row_tail(ui, |ui| {
                if transmitting {
                    if crate::chrome::chip(ui, false, "Abort").clicked() {
                        cmds.push(Command::DigiAbortTx);
                    }
                    ui.label(
                        RichText::new(format!("● TX {:.0}%", progress * 100.0))
                            .size(11.0)
                            .strong()
                            .color(crate::theme::ALERT()),
                    );
                }
            });
        });
        // Transmit-progress bar spanning the panel while keyed.
        {
            let (bar, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 3.0), egui::Sense::hover());
            let p = ui.painter_at(bar);
            p.rect_filled(bar, 0.0, crate::theme::gray(24));
            if transmitting {
                let mut fill = bar;
                fill.set_width(bar.width() * progress);
                p.rect_filled(fill, 0.0, crate::theme::ALERT());
            }
        }
        ui.add_space(4.0);

        let avail_w = ui.available_width();
        let gap = 10.0;
        // A phone paints one at a time. The two sections have a 150 pt floor
        // each, which is most of the screen before either has a preview in it,
        // and a preview of what is about to go on the air is the point of both.
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let half = if pane.is_some() { avail_w } else { ((avail_w - gap) / 2.0).max(150.0) };
        let content_h = (ui.available_height() - 2.0).max(150.0);

        ui.horizontal_top(|ui| {
            // ── Text paint ──
            if pane.is_none_or(|p| p == 0) {
                sstv_section(ui, "TEXT PAINT", egui::vec2(half, content_h), |ui| {
                    let inner_w = ui.available_width();
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.rf_paint.text)
                            .hint_text("Type text to paint…")
                            .desired_width(inner_w),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("PREVIEW WATERFALL")
                            .size(8.5)
                            .color(crate::theme::CYAN_DIM()),
                    );
                    ui.add_space(2.0);
                    let prev_h = (ui.available_height() - 40.0).max(44.0);
                    draw_scroll_preview(
                        ui,
                        self.rf_paint.text_prev.as_ref(),
                        egui::vec2(inner_w, prev_h),
                        time,
                        "type text to preview",
                    );
                    ui.add_space(6.0);
                    let ready = !self.rf_paint.text.trim().is_empty();
                    ui.add_enabled_ui(ready && !transmitting && tx_ok, |ui| {
                        if rx_only_hint(
                            crate::chrome::chip_accent(
                                ui,
                                true,
                                "  TRANSMIT  ",
                                crate::theme::ALERT(),
                                Color32::WHITE,
                            ),
                            tx_ok,
                        )
                        .clicked()
                            && let Some((gray, w, h)) =
                                crate::rf_paint::text_bitmap(&self.rf_paint.text)
                            && let Some(png) = crate::rf_paint::gray_to_png(&gray, w, h)
                        {
                            cmds.push(Command::DigiImageTx { png });
                        }
                    });
                });
            }
            if pane.is_none() {
                ui.add_space(gap);
            }
            // ── Image paint ──
            if pane.is_none_or(|p| p != 0) {
                sstv_section(ui, "IMAGE PAINT", egui::vec2(half, content_h), |ui| {
                    let inner_w = ui.available_width();
                    let img_h = (content_h * 0.4).clamp(56.0, 150.0);
                    draw_image_box(
                        ui,
                        self.rf_paint.img_disp.as_ref(),
                        self.rf_paint.img_gray.as_ref().map(|(_, w, h)| (*w, *h)),
                        egui::vec2(inner_w, img_h),
                        "no image loaded",
                    );
                    ui.add_space(4.0);
                    if crate::chrome::chip(ui, false, "Load image…").clicked() {
                        pick_image(self.rf_paint.inbox.clone());
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("PREVIEW WATERFALL")
                            .size(8.5)
                            .color(crate::theme::CYAN_DIM()),
                    );
                    ui.add_space(2.0);
                    let prev_h = (ui.available_height() - 40.0).max(40.0);
                    draw_scroll_preview(
                        ui,
                        self.rf_paint.img_prev.as_ref(),
                        egui::vec2(inner_w, prev_h),
                        time,
                        "load an image to preview",
                    );
                    ui.add_space(6.0);
                    let ready = self.rf_paint.img_gray.is_some();
                    ui.add_enabled_ui(ready && !transmitting && tx_ok, |ui| {
                        if rx_only_hint(
                            crate::chrome::chip_accent(
                                ui,
                                true,
                                "  TRANSMIT  ",
                                crate::theme::ALERT(),
                                Color32::WHITE,
                            ),
                            tx_ok,
                        )
                        .clicked()
                            && let Some((gray, w, h)) = &self.rf_paint.img_gray
                            && let Some(png) = crate::rf_paint::gray_to_png(gray, *w, *h)
                        {
                            cmds.push(Command::DigiImageTx { png });
                        }
                    });
                });
            }
        });
    }
}
