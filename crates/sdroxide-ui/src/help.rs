//! F1 help: the embedded user manual, rendered as cyberpunk markdown with a
//! separately-scrollable navigation outline on the left.
//!
//! The manual text and every screenshot are baked into the binary with
//! `include_str!` / `include_bytes!`, so the help window works with no files
//! on disk (and in the wasm build). A small hand-rolled markdown parser turns
//! the manual into a block list once, then the window renders it with angled,
//! hazard-striped section headers to match the rest of the UI chrome.

use std::collections::HashMap;

use eframe::egui::{
    self, Align, Color32, CursorIcon, FontFamily, FontId, Layout, Painter, Rect, Response,
    RichText, Sense, Shape, Stroke, Ui, pos2, vec2,
};

use crate::theme;

/// The manual source, embedded from the repository `docs/` directory.
const MANUAL_MD: &str = include_str!("../../../docs/USER_MANUAL.md");

/// Embedded screenshot bytes, keyed by the `images/<name>` path the manual
/// uses. Everything is baked in so no image files are needed at runtime. The
/// bandwidth screenshot is referenced with a hyphen in the manual but the file
/// on disk uses an underscore, so both spellings map to the one real file.
fn embedded_image(path: &str) -> Option<&'static [u8]> {
    let name = path.rsplit('/').next().unwrap_or(path);
    Some(match name {
        "01-main-window.png" => &include_bytes!("../../../docs/images/01-main-window.png")[..],
        "02-top-bar.png" => &include_bytes!("../../../docs/images/02-top-bar.png")[..],
        "03-panadapter-tuning.png" => {
            &include_bytes!("../../../docs/images/03-panadapter-tuning.png")[..]
        }
        "04-band-mode-popup.png" => {
            &include_bytes!("../../../docs/images/04-band-mode-popup.png")[..]
        }
        "05-colormaps.png" => &include_bytes!("../../../docs/images/05-colormaps.png")[..],
        "06-memories.png" => &include_bytes!("../../../docs/images/06-memories.png")[..],
        "07-ft8-panel.png" => &include_bytes!("../../../docs/images/07-ft8-panel.png")[..],
        "08-ft8-setup.png" => &include_bytes!("../../../docs/images/08-ft8-setup.png")[..],
        "09-logbook.png" => &include_bytes!("../../../docs/images/09-logbook.png")[..],
        "10-skimmer.png" => &include_bytes!("../../../docs/images/10-skimmer.png")[..],
        "11-settings.png" => &include_bytes!("../../../docs/images/11-settings.png")[..],
        "12-audio-cat.png" => &include_bytes!("../../../docs/images/12-audio-cat.png")[..],
        "13-web-client.png" => &include_bytes!("../../../docs/images/13-web-client.png")[..],
        "bw_measurement.jpg" | "bw-measurement.jpg" => {
            &include_bytes!("../../../docs/images/bw_measurement.jpg")[..]
        }
        "rit_xit.jpg" => &include_bytes!("../../../docs/images/rit_xit.jpg")[..],
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Document model
// ---------------------------------------------------------------------------

/// A span of inline text with (possibly nested) emphasis.
#[derive(Clone)]
enum Inline {
    Text(String),
    Code(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Link { text: Vec<Inline>, href: String },
}

/// A top-level block of the manual.
enum Block {
    Heading { level: u8, text: Vec<Inline>, slug: String },
    Paragraph(Vec<Inline>),
    Bullets(Vec<Vec<Inline>>),
    Numbered(Vec<(String, Vec<Inline>)>),
    Quote(Vec<Inline>),
    Code(String),
    Rule,
    Image { alt: String, path: String },
    Table { header: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>> },
}

/// One entry in the left navigation outline (headings, levels 2 and 3).
struct NavEntry {
    slug: String,
    level: u8,
    label: String,
}

/// The parsed manual: block list plus the derived navigation outline. Built
/// once and never mutated, so it can be borrowed alongside the mutable UI
/// state without conflict.
struct Doc {
    blocks: Vec<Block>,
    nav: Vec<NavEntry>,
}

impl Doc {
    fn parse(md: &str) -> Self {
        let blocks = parse_blocks(md);
        let nav = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Heading { level, text, slug } if *level == 2 || *level == 3 => {
                    Some(NavEntry { slug: slug.clone(), level: *level, label: plain_text(text) })
                }
                _ => None,
            })
            .collect();
        Doc { blocks, nav }
    }
}

// ---------------------------------------------------------------------------
// Help window state
// ---------------------------------------------------------------------------

pub struct Help {
    pub open: bool,
    doc: Doc,
    /// Decoded screenshot textures, keyed by manual image path (None = decode
    /// failed / missing), lazily filled the first time each image is shown.
    textures: HashMap<String, Option<egui::TextureHandle>>,
    /// Heading slug the content pane should scroll to, and how many more frames
    /// to keep nudging it there (images loading can shift layout for a frame or
    /// two after a jump, so we hold the target briefly).
    scroll_to: Option<String>,
    scroll_frames: u8,
    /// Slug highlighted in the outline — follows the scroll position, or the
    /// last item the user clicked.
    active: String,
}

impl Default for Help {
    fn default() -> Self {
        let doc = Doc::parse(MANUAL_MD);
        let active = doc.nav.first().map(|n| n.slug.clone()).unwrap_or_default();
        Help { open: false, doc, textures: HashMap::new(), scroll_to: None, scroll_frames: 0, active }
    }
}

/// Actions a rendered inline element (a link) wants to trigger, collected
/// during the frame and applied afterwards to avoid borrow conflicts.
#[derive(Default)]
struct Actions {
    link: Option<String>,
}

impl Help {
    /// Jump the outline (and content) to a heading slug over the next few frames.
    fn go_to(&mut self, slug: String) {
        self.active = slug.clone();
        self.scroll_to = Some(slug);
        self.scroll_frames = 3;
    }

    /// Draw the help window if open. Self-contained: needs only the egui
    /// context, so it never touches the radio controller.
    pub fn ui(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        // Esc closes, matching the other overlays.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = false;
            return;
        }

        // A pending scroll target set by a link click last frame, or a nav
        // click this frame (below). Held for a few frames so it survives
        // late-loading image layout shifts.
        let mut target = self.scroll_to.clone();

        let mut open = self.open;
        let resp = egui::Window::new("SDROXIDE MANUAL")
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .collapsible(false)
            .default_width(940.0)
            .default_height(680.0)
            .min_width(560.0)
            .min_height(360.0)
            .show(ctx, |ui| {
                // egui's resizable window never shrinks and grows every frame to
                // `max(desired_size, last_content_size)` (see egui resize.rs). We
                // allocate each column to exactly the available width/height, and
                // egui rounds every widget rect to the pixel grid independently, so
                // the summed columns can land a hair larger than the window — which
                // the ratchet then locks in and repeats until the window fills the
                // screen. A few pixels of slack keeps the content strictly inside
                // the window so its size stays put.
                const SLACK: f32 = 2.0;
                let full_h = (ui.available_height() - SLACK).max(0.0);
                let mut actions = Actions::default();
                let mut nav_click: Option<String> = None;

                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // --- Left: navigation outline (own scroll area) ---
                    const NAV_W: f32 = 236.0;
                    ui.allocate_ui_with_layout(
                        vec2(NAV_W, full_h),
                        Layout::top_down(Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(theme::BG_DEEP)
                                .inner_margin(egui::Margin { left: 8, right: 8, top: 8, bottom: 8 })
                                .show(ui, |ui| {
                                    ui.set_min_height(full_h - 16.0);
                                    ui.set_width(NAV_W - 16.0);
                                    ui.label(
                                        RichText::new("CONTENTS")
                                            .color(theme::CYAN_DIM)
                                            .size(10.0)
                                            .strong(),
                                    );
                                    ui.add_space(6.0);
                                    egui::ScrollArea::vertical()
                                        .id_salt("help_nav")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            for entry in &self.doc.nav {
                                                if nav_item(ui, entry, entry.slug == self.active) {
                                                    nav_click = Some(entry.slug.clone());
                                                }
                                            }
                                        });
                                });
                        },
                    );

                    // Divider between the panes.
                    let (drect, _) =
                        ui.allocate_exact_size(vec2(9.0, full_h), Sense::hover());
                    ui.painter().vline(
                        drect.center().x,
                        drect.y_range(),
                        Stroke::new(1.0, theme::LINE_LIT),
                    );

                    // A nav click wins over a stale link target and takes
                    // effect this same frame.
                    if let Some(slug) = &nav_click {
                        target = Some(slug.clone());
                    }

                    // --- Right: rendered manual (own scroll area) ---
                    // Slack (see above) so the two panes never quite reach the
                    // window edge, which would ratchet the window wider each frame.
                    let content_w = (ui.available_width() - SLACK).max(0.0);
                    ui.allocate_ui_with_layout(
                        vec2(content_w, full_h),
                        Layout::top_down(Align::Min),
                        |ui| {
                            let out = egui::ScrollArea::vertical()
                                .id_salt("help_body")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width() - 6.0);
                                    let mut heading_tops: Vec<(String, f32)> = Vec::new();
                                    for (idx, block) in self.doc.blocks.iter().enumerate() {
                                        draw_block(
                                            ui,
                                            idx,
                                            block,
                                            &mut self.textures,
                                            target.as_deref(),
                                            &mut heading_tops,
                                            &mut actions,
                                        );
                                    }
                                    heading_tops
                                });

                            // Scroll-spy: highlight the last heading whose top
                            // has passed the viewport top — but don't fight an
                            // in-progress jump.
                            if self.scroll_to.is_none() {
                                let top = out.inner_rect.top() + 6.0;
                                for (slug, y) in &out.inner {
                                    if *y <= top {
                                        self.active = slug.clone();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        },
                    );
                });

                // Apply a link click: an in-page anchor jumps the outline, an
                // external URL opens in the browser.
                if let Some(href) = actions.link.take() {
                    if let Some(anchor) = href.strip_prefix('#') {
                        self.go_to(anchor.to_string());
                    } else {
                        ctx.open_url(egui::OpenUrl::new_tab(href));
                    }
                }
            });

        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.open = open;

        // Advance / retire the pending scroll target.
        if self.scroll_to.is_some() {
            ctx.request_repaint();
            self.scroll_frames = self.scroll_frames.saturating_sub(1);
            if self.scroll_frames == 0 {
                self.scroll_to = None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Navigation outline
// ---------------------------------------------------------------------------

/// One clickable outline row. Level-3 entries are indented; the active row gets
/// a yellow accent bar and a faint fill. Returns true when clicked.
fn nav_item(ui: &mut Ui, entry: &NavEntry, active: bool) -> bool {
    let indent = if entry.level >= 3 { 15.0 } else { 2.0 };
    let size = if entry.level >= 3 { 12.0 } else { 13.0 };
    let color = if active {
        theme::CYAN
    } else if entry.level >= 3 {
        theme::TEXT
    } else {
        theme::TEXT_STRONG
    };
    let avail = ui.available_width();
    let text_w = (avail - indent - 10.0).max(40.0);
    let font = FontId::new(size, FontFamily::Proportional);
    let galley = ui.painter().layout(entry.label.clone(), font, color, text_w);
    let h = galley.size().y + 6.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(avail, h), Sense::click());

    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if active {
            p.rect_filled(rect, 0.0, theme::FILL);
            p.rect_filled(
                Rect::from_min_max(rect.left_top(), pos2(rect.left() + 3.0, rect.bottom())),
                0.0,
                theme::YELLOW,
            );
        } else if resp.hovered() {
            p.rect_filled(rect, 0.0, theme::ROW_HOVER);
        }
        let ty = rect.center().y - galley.size().y / 2.0;
        p.galley(pos2(rect.left() + indent + 6.0, ty), galley, color);
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

// ---------------------------------------------------------------------------
// Block rendering
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn draw_block(
    ui: &mut Ui,
    idx: usize,
    block: &Block,
    textures: &mut HashMap<String, Option<egui::TextureHandle>>,
    scroll_target: Option<&str>,
    heading_tops: &mut Vec<(String, f32)>,
    actions: &mut Actions,
) {
    match block {
        Block::Heading { level, text, slug } => {
            ui.add_space(if *level <= 2 { 15.0 } else { 11.0 });
            let resp = draw_header(ui, *level, &plain_text(text));
            if *level == 2 || *level == 3 {
                heading_tops.push((slug.clone(), resp.rect.top()));
            }
            if scroll_target == Some(slug.as_str()) {
                resp.scroll_to_me(Some(Align::TOP));
            }
            ui.add_space(6.0);
        }
        Block::Paragraph(inl) => {
            draw_inline(ui, inl, theme::TEXT, false, actions);
            ui.add_space(7.0);
        }
        Block::Bullets(items) => {
            for item in items {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(8.0);
                    ui.label(RichText::new("▸ ").color(theme::YELLOW).strong());
                    draw_inline(ui, item, theme::TEXT, false, actions);
                });
                ui.add_space(2.0);
            }
            ui.add_space(5.0);
        }
        Block::Numbered(items) => {
            for (num, item) in items {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("{num}. ")).color(theme::CYAN).strong());
                    draw_inline(ui, item, theme::TEXT, false, actions);
                });
                ui.add_space(2.0);
            }
            ui.add_space(5.0);
        }
        Block::Quote(inl) => {
            ui.add_space(4.0);
            let inner = egui::Frame::new()
                .fill(theme::CQ_BG)
                .inner_margin(egui::Margin { left: 13, right: 10, top: 7, bottom: 7 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width() - 23.0);
                    draw_inline(ui, inl, theme::TEXT_STRONG, false, actions);
                });
            let r = inner.response.rect;
            ui.painter().rect_filled(
                Rect::from_min_max(r.left_top(), pos2(r.left() + 3.0, r.bottom())),
                0.0,
                theme::YELLOW,
            );
            ui.add_space(7.0);
        }
        Block::Code(code) => {
            ui.add_space(3.0);
            egui::Frame::new()
                .fill(theme::INPUT_BG)
                .stroke(Stroke::new(1.0, theme::LINE))
                .inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width() - 20.0);
                    for line in code.lines() {
                        ui.label(RichText::new(line).monospace().color(theme::GREEN));
                    }
                });
            ui.add_space(6.0);
        }
        Block::Rule => {
            ui.add_space(9.0);
            let w = ui.available_width();
            let (rect, _) = ui.allocate_exact_size(vec2(w, 6.0), Sense::hover());
            hazard_stripes(&ui.painter().clone(), rect, 11.0);
            ui.add_space(9.0);
        }
        Block::Image { alt, path } => {
            ui.add_space(5.0);
            match ensure_texture(textures, ui.ctx(), path) {
                Some(tex) => {
                    let nat = tex.size_vec2();
                    let w = nat.x.min(ui.available_width() - 8.0).min(900.0);
                    egui::Frame::new()
                        .stroke(Stroke::new(1.0, theme::LINE_LIT))
                        .inner_margin(3)
                        .show(ui, |ui| {
                            ui.add(egui::Image::new(tex).max_width(w).corner_radius(0));
                        });
                }
                None => {
                    ui.colored_label(theme::PINK, format!("[missing image: {path}]"));
                }
            }
            if !alt.is_empty() {
                ui.add_space(3.0);
                ui.label(RichText::new(alt.as_str()).italics().color(theme::CYAN_DIM).size(11.5));
            }
            ui.add_space(9.0);
        }
        Block::Table { header, rows } => {
            ui.add_space(5.0);
            draw_table(ui, idx, header, rows, actions);
            ui.add_space(8.0);
        }
    }
}

/// A markdown table. Everything here is two-column (a short key / option / mode
/// against a long description), so the first column is pinned narrow and the
/// second takes the rest; other column counts split evenly. Rows are striped.
fn draw_table(
    ui: &mut Ui,
    idx: usize,
    header: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    actions: &mut Actions,
) {
    let cols = header.len().max(1);
    let total = ui.available_width();
    let spacing = 10.0;
    let inner = total - 16.0 - spacing * (cols as f32 - 1.0);
    let col_w: Vec<f32> = if cols == 2 {
        let l = (inner * 0.28).clamp(80.0, 240.0);
        vec![l, (inner - l).max(120.0)]
    } else {
        vec![(inner / cols as f32).max(80.0); cols]
    };

    egui::Frame::new()
        .stroke(Stroke::new(1.0, theme::LINE_LIT))
        .inner_margin(0)
        .show(ui, |ui| {
            ui.set_width(total);
            // Header row.
            table_row(ui, header, &col_w, spacing, theme::FILL, true, actions, idx);
            for (ri, row) in rows.iter().enumerate() {
                let fill = if ri % 2 == 0 { theme::ROW_BG } else { theme::PANEL };
                table_row(ui, row, &col_w, spacing, fill, false, actions, idx);
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn table_row(
    ui: &mut Ui,
    cells: &[Vec<Inline>],
    col_w: &[f32],
    spacing: f32,
    fill: Color32,
    header: bool,
    actions: &mut Actions,
    idx: usize,
) {
    egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - 16.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                for (ci, cell) in cells.iter().enumerate() {
                    let w = col_w.get(ci).copied().unwrap_or(120.0);
                    ui.push_id((idx, ci, header), |ui| {
                        ui.allocate_ui_with_layout(
                            vec2(w, 0.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.set_width(w);
                                let (color, strong) =
                                    if header { (theme::CYAN, true) } else { (theme::TEXT, false) };
                                draw_inline(ui, cell, color, strong, actions);
                            },
                        );
                    });
                }
            });
        });
}

/// Lazily decode and cache an embedded screenshot as a texture.
fn ensure_texture<'a>(
    textures: &'a mut HashMap<String, Option<egui::TextureHandle>>,
    ctx: &egui::Context,
    path: &str,
) -> Option<&'a egui::TextureHandle> {
    if !textures.contains_key(path) {
        let tex = embedded_image(path).and_then(|bytes| {
            crate::sstv::decode_image(bytes).map(|(rgb, w, h)| {
                let ci = crate::sstv::color_image(&rgb, w, h);
                ctx.load_texture(format!("help_{path}"), ci, egui::TextureOptions::LINEAR)
            })
        });
        textures.insert(path.to_string(), tex);
    }
    textures.get(path).and_then(|o| o.as_ref())
}

// ---------------------------------------------------------------------------
// Cyberpunk section headers
// ---------------------------------------------------------------------------

/// The six-point cut-corner outline (top-right + bottom-left bevels), matching
/// [`crate::chrome`]'s panel chrome.
fn cut_outline(rect: Rect, cut: f32) -> Vec<egui::Pos2> {
    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
    vec![
        pos2(l, t),
        pos2(r - cut, t),
        pos2(r, t + cut),
        pos2(r, b),
        pos2(l + cut, b),
        pos2(l, b - cut),
    ]
}

/// 45° yellow/black hazard stripes filling `rect` (clipped to it).
fn hazard_stripes(p: &Painter, rect: Rect, stripe_w: f32) {
    let clip = p.with_clip_rect(rect);
    let dark = Color32::from_rgb(0x16, 0x12, 0x04);
    let h = rect.height();
    let mut x = rect.left() - h;
    let mut k = 0i32;
    while x < rect.right() + stripe_w {
        let color = if k % 2 == 0 { theme::YELLOW } else { dark };
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

/// An angled section header: a cut-corner bar with a yellow/black hazard tab on
/// the left and the section title in the techno heading face. Level 1 is the
/// document title, 2 is a section, 3 a subsection.
fn draw_header(ui: &mut Ui, level: u8, text: &str) -> Response {
    let (size, bar_h, accent, fill, txt_col) = match level {
        1 => (23.0, 50.0, theme::CYAN, theme::FILL, theme::TEXT_STRONG),
        2 => (17.0, 37.0, theme::PINK, theme::FILL, theme::CYAN),
        _ => (14.5, 29.0, theme::YELLOW, theme::ROW_BG, theme::TEXT_STRONG),
    };
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, bar_h), Sense::hover());

    if ui.is_rect_visible(rect) {
        let p = ui.painter().clone();
        let cut = 9.0f32.min(bar_h * 0.42);
        // Bar fill (already beveled) sitting on the panel background.
        p.add(Shape::convex_polygon(cut_outline(rect, cut), fill, Stroke::NONE));
        // Hazard-stripe accent tab down the left edge.
        let tab = Rect::from_min_max(rect.left_top(), pos2(rect.left() + 13.0, rect.bottom()));
        hazard_stripes(&p, tab, 8.0);
        // Mask the two square corners back to the panel colour so the bevel
        // reads as a real cut (this also clips the hazard tab's overflow), then
        // stroke the angled outline.
        let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
        p.add(Shape::convex_polygon(
            vec![pos2(r - cut, t), pos2(r, t), pos2(r, t + cut)],
            theme::PANEL,
            Stroke::NONE,
        ));
        p.add(Shape::convex_polygon(
            vec![pos2(l, b - cut), pos2(l + cut, b), pos2(l, b)],
            theme::PANEL,
            Stroke::NONE,
        ));
        p.add(Shape::closed_line(cut_outline(rect, cut), Stroke::new(1.4, accent)));
        // Title text, clear of the hazard tab.
        let font = FontId::new(size, FontFamily::Name("chakra-bold".into()));
        let galley = p.layout_no_wrap(text.to_string(), font, txt_col);
        let ty = rect.center().y - galley.size().y / 2.0;
        p.galley(pos2(rect.left() + 24.0, ty), galley, txt_col);
    }
    resp
}

// ---------------------------------------------------------------------------
// Inline rendering
// ---------------------------------------------------------------------------

/// Render a run of inline spans, wrapping at the available width. Zero
/// horizontal item spacing keeps adjacent styled runs from gaining stray gaps.
fn draw_inline(ui: &mut Ui, inl: &[Inline], base: Color32, strong: bool, actions: &mut Actions) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().item_spacing.y = 3.0;
        draw_runs(ui, inl, strong, false, base, actions);
    });
}

fn draw_runs(
    ui: &mut Ui,
    inl: &[Inline],
    strong: bool,
    italic: bool,
    base: Color32,
    actions: &mut Actions,
) {
    for span in inl {
        match span {
            Inline::Text(t) => {
                let col = if strong && base == theme::TEXT { theme::TEXT_STRONG } else { base };
                let mut rt = RichText::new(t).color(col);
                if strong {
                    rt = rt.strong();
                }
                if italic {
                    rt = rt.italics();
                }
                ui.label(rt);
            }
            Inline::Code(c) => {
                // Padded with no-break spaces so the highlight doesn't crowd
                // the glyphs.
                ui.label(
                    RichText::new(format!("\u{00a0}{c}\u{00a0}"))
                        .monospace()
                        .color(theme::CYAN)
                        .background_color(theme::INPUT_BG),
                );
            }
            Inline::Bold(v) => draw_runs(ui, v, true, italic, base, actions),
            Inline::Italic(v) => draw_runs(ui, v, strong, true, base, actions),
            Inline::Link { text, href } => {
                let label = plain_text(text);
                let resp = ui.add(
                    egui::Label::new(RichText::new(label).color(theme::CYAN).underline())
                        .sense(Sense::click()),
                );
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    actions.link = Some(href.clone());
                }
            }
        }
    }
}

/// Flatten inline spans to their plain text (for headings, links, and slugs).
fn plain_text(inl: &[Inline]) -> String {
    let mut s = String::new();
    for span in inl {
        match span {
            Inline::Text(t) | Inline::Code(t) => s.push_str(t),
            Inline::Bold(v) | Inline::Italic(v) => s.push_str(&plain_text(v)),
            Inline::Link { text, .. } => s.push_str(&plain_text(text)),
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Markdown parsing
// ---------------------------------------------------------------------------

/// GitHub-style heading anchor: lowercase, drop punctuation, spaces → hyphens.
/// Matches the in-manual table-of-contents links (e.g. `5.9 UI preferences`
/// → `59-ui-preferences`).
fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' || c == '-' {
            out.push('-');
        }
    }
    out
}

fn parse_blocks(md: &str) -> Vec<Block> {
    let lines: Vec<&str> = md.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();
        if t.is_empty() {
            i += 1;
            continue;
        }

        // Fenced code block.
        if t.starts_with("```") {
            let mut code = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                code.push_str(lines[i]);
                code.push('\n');
                i += 1;
            }
            i += 1; // closing fence
            while code.ends_with('\n') {
                code.pop();
            }
            blocks.push(Block::Code(code));
            continue;
        }

        // Heading.
        if let Some(h) = t.strip_prefix('#') {
            let mut level = 1u8;
            let mut rest = h;
            while let Some(r) = rest.strip_prefix('#') {
                level += 1;
                rest = r;
            }
            if let Some(txt) = rest.strip_prefix(' ') {
                let txt = txt.trim();
                blocks.push(Block::Heading {
                    level: level.min(6),
                    text: parse_inline(txt),
                    slug: slugify(txt),
                });
                i += 1;
                continue;
            }
        }

        // Horizontal rule.
        if t == "---" || t == "***" || t == "___" {
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }

        // Whole-line image.
        if let Some(img) = parse_image_line(t) {
            blocks.push(img);
            i += 1;
            continue;
        }

        // Pipe table (row followed by a `| --- | --- |` separator).
        if t.starts_with('|') && i + 1 < lines.len() && is_table_separator(lines[i + 1]) {
            let header = parse_table_row(t);
            let mut rows = Vec::new();
            i += 2;
            while i < lines.len() && lines[i].trim().starts_with('|') {
                rows.push(parse_table_row(lines[i].trim()));
                i += 1;
            }
            blocks.push(Block::Table { header, rows });
            continue;
        }

        // Blockquote (a run of `>` lines, joined).
        if t.starts_with('>') {
            let mut buf = String::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                let l = lines[i].trim_start().trim_start_matches('>').trim();
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(l);
                i += 1;
            }
            blocks.push(Block::Quote(parse_inline(&buf)));
            continue;
        }

        // Bullet list.
        if is_bullet(t) {
            let mut items = Vec::new();
            collect_list(&lines, &mut i, |content| items.push(parse_inline(content)), is_bullet, strip_bullet);
            blocks.push(Block::Bullets(items));
            continue;
        }

        // Numbered list.
        if is_numbered(t) {
            let mut items = Vec::new();
            collect_list(
                &lines,
                &mut i,
                |content| {
                    let (num, rest) = split_number(content);
                    items.push((num, parse_inline(rest)));
                },
                is_numbered,
                |s| s, // number kept; split later
            );
            blocks.push(Block::Numbered(items));
            continue;
        }

        // Paragraph: consecutive plain lines.
        let mut buf = String::new();
        while i < lines.len() {
            let l = lines[i];
            let lt = l.trim();
            if lt.is_empty() || is_block_start(l) {
                break;
            }
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(lt);
            i += 1;
        }
        blocks.push(Block::Paragraph(parse_inline(&buf)));
    }
    blocks
}

/// Collect a list starting at `*i`: each item is a marker line plus any
/// indented continuation lines (wrapped prose), folded to one string.
fn collect_list(
    lines: &[&str],
    i: &mut usize,
    mut push: impl FnMut(&str),
    is_marker: impl Fn(&str) -> bool,
    strip: impl Fn(&str) -> &str,
) {
    while *i < lines.len() {
        let t = lines[*i].trim();
        if t.is_empty() || !is_marker(t) {
            break;
        }
        let mut item = strip(t).to_string();
        *i += 1;
        // Indented, non-marker, non-blank lines continue the current item.
        while *i < lines.len() {
            let raw = lines[*i];
            let ct = raw.trim();
            if ct.is_empty() || is_marker(ct) || is_block_start(raw) {
                break;
            }
            if raw.starts_with(' ') || raw.starts_with('\t') {
                item.push(' ');
                item.push_str(ct);
                *i += 1;
            } else {
                break;
            }
        }
        push(&item);
    }
}

/// True if a line begins a new block (used to end paragraph / list runs).
fn is_block_start(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() {
        return false;
    }
    t.starts_with('#')
        || t.starts_with("```")
        || t == "---"
        || t == "***"
        || t == "___"
        || t.starts_with("![")
        || t.starts_with('|')
        || t.starts_with('>')
        || is_bullet(t)
        || is_numbered(t)
}

fn is_bullet(t: &str) -> bool {
    t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ")
}

fn strip_bullet(t: &str) -> &str {
    t.get(2..).unwrap_or("").trim_start()
}

fn is_numbered(t: &str) -> bool {
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    !digits.is_empty() && t[digits.len()..].starts_with(". ")
}

/// Split `12. rest` into ("12", "rest").
fn split_number(t: &str) -> (String, &str) {
    let num: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest = t[num.len()..].trim_start_matches(". ").trim_start_matches('.').trim_start();
    (num, rest)
}

fn is_table_separator(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with('|') {
        return false;
    }
    t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Split a `| a | b |` row into its trimmed cells (parsed inline).
fn parse_table_row(line: &str) -> Vec<Vec<Inline>> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| parse_inline(c.trim())).collect()
}

/// Parse a line that is a single `![alt](path)` image.
fn parse_image_line(t: &str) -> Option<Block> {
    let rest = t.strip_prefix("![")?;
    let close = rest.find(']')?;
    let alt = &rest[..close];
    let after = rest[close + 1..].strip_prefix('(')?;
    let end = after.find(')')?;
    // Must be the whole line (nothing trailing but whitespace).
    if after[end + 1..].trim().is_empty() {
        Some(Block::Image { alt: alt.to_string(), path: after[..end].trim().to_string() })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Inline parsing
// ---------------------------------------------------------------------------

fn parse_inline(s: &str) -> Vec<Inline> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    fn flush(buf: &mut String, out: &mut Vec<Inline>) {
        if !buf.is_empty() {
            out.push(Inline::Text(std::mem::take(buf)));
        }
    }

    while i < chars.len() {
        let c = chars[i];

        // Inline code — parsed first so its contents are never re-interpreted.
        if c == '`' && let Some(end) = find_char(&chars, i + 1, '`') {
            flush(&mut buf, &mut out);
            out.push(Inline::Code(chars[i + 1..end].iter().collect()));
            i = end + 1;
            continue;
        }

        // Link `[text](href)`.
        if c == '[' && let Some((text, href, next)) = parse_link(&chars, i) {
            flush(&mut buf, &mut out);
            out.push(Inline::Link { text: parse_inline(&text), href });
            i = next;
            continue;
        }

        // Bold `**...**`.
        if c == '*'
            && chars.get(i + 1) == Some(&'*')
            && let Some(end) = find_str(&chars, i + 2, &['*', '*'])
        {
            flush(&mut buf, &mut out);
            let inner: String = chars[i + 2..end].iter().collect();
            out.push(Inline::Bold(parse_inline(&inner)));
            i = end + 2;
            continue;
        }

        // Italic `*...*`.
        if c == '*'
            && let Some(end) = find_char(&chars, i + 1, '*')
            && end > i + 1
        {
            flush(&mut buf, &mut out);
            let inner: String = chars[i + 1..end].iter().collect();
            out.push(Inline::Italic(parse_inline(&inner)));
            i = end + 1;
            continue;
        }

        // Italic `_..._`, only at word boundaries so identifiers like
        // `sample_rate` are left alone.
        if c == '_'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some(end) = find_underscore_close(&chars, i + 1)
        {
            flush(&mut buf, &mut out);
            let inner: String = chars[i + 1..end].iter().collect();
            out.push(Inline::Italic(parse_inline(&inner)));
            i = end + 1;
            continue;
        }

        buf.push(c);
        i += 1;
    }
    flush(&mut buf, &mut out);
    out
}

fn find_char(chars: &[char], from: usize, target: char) -> Option<usize> {
    (from..chars.len()).find(|&k| chars[k] == target)
}

fn find_str(chars: &[char], from: usize, target: &[char]) -> Option<usize> {
    (from..chars.len().saturating_sub(target.len() - 1))
        .find(|&k| chars[k..k + target.len()] == *target)
}

/// The closing `_` of an emphasis span: an underscore past non-empty content
/// (`k > from`) that is followed by a word boundary, so identifiers survive.
fn find_underscore_close(chars: &[char], from: usize) -> Option<usize> {
    (from + 1..chars.len())
        .find(|&k| chars[k] == '_' && chars.get(k + 1).map(|c| !c.is_alphanumeric()).unwrap_or(true))
}

/// Parse `[text](href)` at `chars[start] == '['`, returning (text, href, index
/// after the closing paren).
fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let close = find_char(chars, start + 1, ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = find_char(chars, close + 2, ')')?;
    let text: String = chars[start + 1..close].iter().collect();
    let href: String = chars[close + 2..end].iter().collect();
    Some((text, href, end + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect every link href in a run of inline spans (recursing into
    /// emphasis and link text).
    fn collect_hrefs(inl: &[Inline], out: &mut Vec<String>) {
        for span in inl {
            match span {
                Inline::Link { text, href } => {
                    out.push(href.clone());
                    collect_hrefs(text, out);
                }
                Inline::Bold(v) | Inline::Italic(v) => collect_hrefs(v, out),
                _ => {}
            }
        }
    }

    #[test]
    fn manual_parses_with_sections_and_nav() {
        let doc = Doc::parse(MANUAL_MD);
        assert!(doc.blocks.len() > 100, "manual should parse into many blocks");
        // Every top-level numbered section (1..=11) is a level-2 heading.
        let sections = doc.nav.iter().filter(|n| n.level == 2).count();
        assert!(sections >= 12, "expected the ToC + 11 sections, got {sections}");
    }

    #[test]
    fn slugs_match_github_anchors() {
        assert_eq!(slugify("1. Feature overview"), "1-feature-overview");
        assert_eq!(slugify("5.9 UI preferences"), "59-ui-preferences");
        assert_eq!(slugify("3. Digital modes"), "3-digital-modes");
    }

    /// Every whole-line image the manual references must be baked into the
    /// binary — a missing embed would render as a red placeholder at runtime.
    #[test]
    fn every_image_is_embedded() {
        let doc = Doc::parse(MANUAL_MD);
        let mut count = 0;
        for b in &doc.blocks {
            if let Block::Image { path, .. } = b {
                count += 1;
                assert!(
                    embedded_image(path).is_some(),
                    "image `{path}` referenced by the manual is not embedded"
                );
            }
        }
        assert!(count >= 15, "expected the manual's screenshots, found {count}");
    }

    /// Every in-page anchor link (`#slug`) must resolve to a real heading, so
    /// the table of contents and cross-references all navigate.
    #[test]
    fn anchor_links_resolve_to_headings() {
        let doc = Doc::parse(MANUAL_MD);
        let slugs: std::collections::HashSet<&str> = doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Heading { slug, .. } => Some(slug.as_str()),
                _ => None,
            })
            .collect();

        let mut hrefs = Vec::new();
        for b in &doc.blocks {
            match b {
                Block::Paragraph(inl) | Block::Quote(inl) => collect_hrefs(inl, &mut hrefs),
                Block::Bullets(items) => items.iter().for_each(|i| collect_hrefs(i, &mut hrefs)),
                Block::Numbered(items) => {
                    items.iter().for_each(|(_, i)| collect_hrefs(i, &mut hrefs))
                }
                Block::Table { header, rows } => {
                    header.iter().for_each(|c| collect_hrefs(c, &mut hrefs));
                    rows.iter().flatten().for_each(|c| collect_hrefs(c, &mut hrefs));
                }
                _ => {}
            }
        }

        let anchors: Vec<&String> = hrefs.iter().filter(|h| h.starts_with('#')).collect();
        assert!(!anchors.is_empty(), "manual should contain in-page links (the ToC)");
        for a in anchors {
            let slug = a.trim_start_matches('#');
            assert!(slugs.contains(slug), "anchor `{a}` has no matching heading");
        }
    }
}
