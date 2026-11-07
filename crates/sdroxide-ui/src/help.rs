//! F1 help: the embedded user manual, rendered as cyberpunk markdown with a
//! separately-scrollable navigation outline on the left.
//!
//! The manual text and every screenshot are baked into the binary with
//! `include_str!` / `include_bytes!`, so the help window works with no files
//! on disk (and in the wasm build). A small hand-rolled markdown parser turns
//! the manual into a block list once, then the window renders it with angled,
//! hazard-striped section headers to match the rest of the UI chrome.
//!
//! Across the top is a find bar that searches as you type: every occurrence is
//! highlighted where it is drawn, ◀/▶ (or Enter / F3) step through them, and
//! the CONTENTS rows holding a hit are tinted and badged with their count, so
//! a term that appears in one chapter out of eleven is visible before any
//! scrolling happens.

use crate::i18n::tr;
use std::collections::HashMap;

use eframe::egui::{
    self, Align, Color32, CursorIcon, FontFamily, FontId, Layout, Rect, Response, RichText, Sense,
    Shape, Stroke, Ui, pos2, vec2,
};

use crate::theme::{self, ThemedScroll};

/// The manual source, embedded from the repository `docs/` directory.
const MANUAL_MD: &str = include_str!("../../../docs/USER_MANUAL.md");

/// Points one arrow-key press scrolls the manual — a few lines, close to a
/// wheel notch so the two gestures feel the same.
const ARROW_STEP: f32 = 56.0;
/// Fraction of the visible height Page Up/Down travels. The remainder is
/// deliberate overlap: a full-height jump loses the reader's place.
const PAGE_FRACTION: f32 = 0.88;

/// Embedded screenshot bytes, keyed by the `images/<name>` path the manual
/// uses. Everything is baked in so no image files are needed at runtime. The
/// bandwidth screenshot is referenced with a hyphen in the manual but the file
/// on disk uses an underscore, so both spellings map to the one real file.
fn embedded_image(path: &str) -> Option<&'static [u8]> {
    let name = path.rsplit('/').next().unwrap_or(path);
    Some(match name {
        "amp-window.png" => &include_bytes!("../../../docs/images/amp-window.png")[..],
        "01-main-window.jpg" => &include_bytes!("../../../docs/images/01-main-window.jpg")[..],
        "02-top-bar.jpg" => &include_bytes!("../../../docs/images/02-top-bar.jpg")[..],
        "03-panadapter-tuning.png" => {
            &include_bytes!("../../../docs/images/03-panadapter-tuning.png")[..]
        }
        "04-band-mode-popup.jpg" => {
            &include_bytes!("../../../docs/images/04-band-mode-popup.jpg")[..]
        }
        "05-colormaps.png" => &include_bytes!("../../../docs/images/05-colormaps.png")[..],
        "06-memories.png" => &include_bytes!("../../../docs/images/06-memories.png")[..],
        "07-ft8-panel.png" => &include_bytes!("../../../docs/images/07-ft8-panel.png")[..],
        "08-ft8-setup.png" => &include_bytes!("../../../docs/images/08-ft8-setup.png")[..],
        "09-logbook.png" => &include_bytes!("../../../docs/images/09-logbook.png")[..],
        "10-skimmer.png" => &include_bytes!("../../../docs/images/10-skimmer.png")[..],
        "13-web-client.png" => &include_bytes!("../../../docs/images/13-web-client.png")[..],
        "14-spots-panel.jpg" => &include_bytes!("../../../docs/images/14-spots-panel.jpg")[..],
        "15-settings-spots.jpg" => {
            &include_bytes!("../../../docs/images/15-settings-spots.jpg")[..]
        }
        "16-settings-uploads.jpg" => {
            &include_bytes!("../../../docs/images/16-settings-uploads.jpg")[..]
        }
        "17-sats-passes.jpg" => &include_bytes!("../../../docs/images/17-sats-passes.jpg")[..],
        "18-awards.jpg" => &include_bytes!("../../../docs/images/18-awards.jpg")[..],
        "bw_measurement.jpg" | "bw-measurement.jpg" => {
            &include_bytes!("../../../docs/images/bw_measurement.jpg")[..]
        }
        "adsb-panel.jpg" => &include_bytes!("../../../docs/images/adsb-panel.jpg")[..],
        "vdl2-panel.jpg" => &include_bytes!("../../../docs/images/vdl2-panel.jpg")[..],
        "aprs-panel.jpg" => &include_bytes!("../../../docs/images/aprs-panel.jpg")[..],
        "cw.jpg" => &include_bytes!("../../../docs/images/cw.jpg")[..],
        "rit_xit.jpg" => &include_bytes!("../../../docs/images/rit_xit.jpg")[..],
        "rtty.jpg" => &include_bytes!("../../../docs/images/rtty.jpg")[..],
        "hellschreiber.jpg" => &include_bytes!("../../../docs/images/hellschreiber.jpg")[..],
        "js8call.jpg" => &include_bytes!("../../../docs/images/js8call.jpg")[..],
        "rfpaint.jpg" => &include_bytes!("../../../docs/images/rfpaint.jpg")[..],
        "sstv.jpg" => &include_bytes!("../../../docs/images/sstv.jpg")[..],
        "wefax.jpg" => &include_bytes!("../../../docs/images/wefax.jpg")[..],
        "settings-general.jpg" => &include_bytes!("../../../docs/images/settings-general.jpg")[..],
        "settings-controls-keyboard.jpg" => {
            &include_bytes!("../../../docs/images/settings-controls-keyboard.jpg")[..]
        }
        "settings-controls-mouse.jpg" => {
            &include_bytes!("../../../docs/images/settings-controls-mouse.jpg")[..]
        }
        "settings-controls-midi.jpg" => {
            &include_bytes!("../../../docs/images/settings-controls-midi.jpg")[..]
        }
        "settings-servers-hamlib.jpg" => {
            &include_bytes!("../../../docs/images/settings-servers-hamlib.jpg")[..]
        }
        "settings-servers-tci.jpg" => {
            &include_bytes!("../../../docs/images/settings-servers-tci.jpg")[..]
        }
        "settings-freedv.jpg" => &include_bytes!("../../../docs/images/settings-freedv.jpg")[..],

        "settings-radio-soapysdr.jpg" => {
            &include_bytes!("../../../docs/images/settings-radio-soapysdr.jpg")[..]
        }
        "settings-radio-cat.jpg" => {
            &include_bytes!("../../../docs/images/settings-radio-cat.jpg")[..]
        }
        "settings-radio-hpsdr.jpg" => {
            &include_bytes!("../../../docs/images/settings-radio-hpsdr.jpg")[..]
        }
        "settings-radio-tci.jpg" => {
            &include_bytes!("../../../docs/images/settings-radio-tci.jpg")[..]
        }
        "settings-radio-rtlsdr.jpg" => {
            &include_bytes!("../../../docs/images/settings-radio-rtlsdr.jpg")[..]
        }
        "settings-radio-smartsdr.jpg" => {
            &include_bytes!("../../../docs/images/settings-radio-smartsdr.jpg")[..]
        }
        "settings-radio-plutosdr.jpg" => {
            &include_bytes!("../../../docs/images/settings-radio-plutosdr.jpg")[..]
        }
        "settings-tle1.jpg" => &include_bytes!("../../../docs/images/settings-tle1.jpg")[..],
        "settings-tle2.jpg" => &include_bytes!("../../../docs/images/settings-tle2.jpg")[..],
        "settings-tle3.jpg" => &include_bytes!("../../../docs/images/settings-tle3.jpg")[..],
        "settings-winlink.jpg" => &include_bytes!("../../../docs/images/settings-winlink.jpg")[..],
        "propagation.jpg" => &include_bytes!("../../../docs/images/propagation.jpg")[..],
        "bandconditions.jpg" => &include_bytes!("../../../docs/images/bandconditions.jpg")[..],
        "satellite.jpg" => &include_bytes!("../../../docs/images/satellite.jpg")[..],
        "rotator.jpg" => &include_bytes!("../../../docs/images/rotator.jpg")[..],
        "3d-sun.jpg" => &include_bytes!("../../../docs/images/3d-sun.jpg")[..],
        "3d-earth.jpg" => &include_bytes!("../../../docs/images/3d-earth.jpg")[..],
        "3d-cme.jpg" => &include_bytes!("../../../docs/images/3d-cme.jpg")[..],
        "settings-ui.jpg" => &include_bytes!("../../../docs/images/settings-ui.jpg")[..],
        "spec-popup.png" => &include_bytes!("../../../docs/images/spec-popup.png")[..],
        "spectrum-3d.png" => &include_bytes!("../../../docs/images/spectrum-3d.png")[..],
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

/// The parsed manual: block list, the derived navigation outline, and the
/// flattened text the find bar searches. Built once and never mutated, so it
/// can be borrowed alongside the mutable UI state without conflict.
struct Doc {
    blocks: Vec<Block>,
    nav: Vec<NavEntry>,
    /// Each block's visible text, in block order — the haystack the outline
    /// badges are counted from, so a keystroke costs one pass over a few
    /// hundred short strings instead of a walk of the whole block tree.
    text: Vec<String>,
    /// For each block, the outline rows it sits under: its chapter (a level-2
    /// heading) and its subsection (level 3), where each exists. This is what
    /// lets a hit deep in the body light up the chapter that holds it.
    owner: Vec<[Option<usize>; 2]>,
}

impl Doc {
    fn parse(md: &str) -> Self {
        let blocks = parse_blocks(md);
        let mut nav: Vec<NavEntry> = Vec::new();
        let mut owner = Vec::with_capacity(blocks.len());
        let (mut chapter, mut section) = (None, None);
        for b in &blocks {
            // A heading owns itself as well as everything under it, so
            // searching for a chapter's title lights up that chapter's row.
            if let Block::Heading { level, text, slug } = b {
                match level {
                    // The document title ends the previous chapter's run.
                    1 => (chapter, section) = (None, None),
                    2 | 3 => {
                        nav.push(NavEntry {
                            slug: slug.clone(),
                            level: *level,
                            label: plain_text(text),
                        });
                        if *level == 2 {
                            (chapter, section) = (Some(nav.len() - 1), None);
                        } else {
                            section = Some(nav.len() - 1);
                        }
                    }
                    _ => {}
                }
            }
            owner.push([chapter, section]);
        }
        let text = blocks.iter().map(block_text).collect();
        Doc { blocks, nav, text, owner }
    }
}

/// Every piece of a block's text that reaches the screen, newline-separated so
/// a search term can never match across a join the reader does not see.
fn block_text(b: &Block) -> String {
    fn push(s: &mut String, t: &str) {
        if !s.is_empty() {
            s.push('\n');
        }
        s.push_str(t);
    }
    let mut s = String::new();
    match b {
        Block::Heading { text, .. } | Block::Paragraph(text) | Block::Quote(text) => {
            push(&mut s, &plain_text(text));
        }
        Block::Bullets(items) => items.iter().for_each(|i| push(&mut s, &plain_text(i))),
        Block::Numbered(items) => items.iter().for_each(|(_, i)| push(&mut s, &plain_text(i))),
        Block::Code(code) => push(&mut s, code),
        Block::Image { alt, .. } => push(&mut s, alt),
        Block::Rule => {}
        Block::Table { header, rows } => {
            header.iter().for_each(|c| push(&mut s, &plain_text(c)));
            rows.iter().flatten().for_each(|c| push(&mut s, &plain_text(c)));
        }
    }
    s
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
    /// Scroll the keyboard has asked for but the content pane has not applied
    /// yet (points, positive = towards the end of the manual). Filled by
    /// [`Help::grab_keys`] at the top of the frame, spent when the pane draws.
    key_scroll: f32,
    /// Content-pane geometry from the last frame — visible height, total height
    /// and current offset. Page Up/Down and Home/End are expressed in points,
    /// so they need to know how tall a page is and where the ends are.
    viewport_h: f32,
    content_h: f32,
    offset_y: f32,
    /// Find-in-page: the term, which occurrence is selected, and how many the
    /// last drawn frame found. The tally comes from the drawing pass rather
    /// than a second scan of the text, so "3 / 17" can never disagree with
    /// what the ◀/▶ buttons step through.
    search: String,
    hit: usize,
    hits: usize,
    /// Frames left to keep pulling the viewport onto the selected hit. Held
    /// for a few, like [`Help::scroll_to`], because a screenshot decoding
    /// above the hit moves it after the first pull.
    seek_frames: u8,
    /// Hits per outline row, indexed like `doc.nav` — what tints the CONTENTS
    /// rows that hold a match. Recomputed only when the term changes.
    nav_hits: Vec<usize>,
    /// The term `nav_hits` was counted for.
    nav_needle: String,
    /// Set by Ctrl+F, spent by the find field the next time it draws.
    focus_find: bool,
    /// False until the window has been drawn once since it was opened — that
    /// first frame is the one that sizes it to fill the viewport.
    shown: bool,
}

impl Default for Help {
    fn default() -> Self {
        let doc = Doc::parse(MANUAL_MD);
        let active = doc.nav.first().map(|n| n.slug.clone()).unwrap_or_default();
        Help {
            open: false,
            doc,
            textures: HashMap::new(),
            scroll_to: None,
            scroll_frames: 0,
            active,
            key_scroll: 0.0,
            viewport_h: 0.0,
            content_h: 0.0,
            offset_y: 0.0,
            search: String::new(),
            hit: 0,
            hits: 0,
            seek_frames: 0,
            nav_hits: Vec::new(),
            nav_needle: String::new(),
            focus_find: false,
            shown: false,
        }
    }
}

/// The side channel the block renderers get: what a rendered link wants to
/// trigger (collected during the frame and applied afterwards, to avoid borrow
/// conflicts) and the find pass the text renderers register their highlights
/// with.
struct Pass<'a> {
    link: Option<String>,
    find: Find<'a>,
}

/// One frame of find-in-page.
///
/// The occurrence index is a *drawing* index: every renderer that puts text on
/// screen claims its matches in document order as it goes, whether or not they
/// are on screen. Deriving it from the drawing rather than from a separate
/// scan of the text is what guarantees the selected hit is a hit that exists —
/// the ◀/▶ buttons can never step onto something the pane will not draw.
struct Find<'a> {
    /// What to look for; empty when the find bar is idle.
    needle: &'a str,
    /// Which occurrence, in document order, is selected.
    sel: usize,
    /// Occurrences claimed so far this frame.
    seen: usize,
    /// Where the selected occurrence landed — what the pane scrolls to.
    rect: Option<Rect>,
}

impl Find<'_> {
    /// Take the next occurrence in document order; true if it is the selected
    /// one, in which case the caller follows up with [`Find::place`] once it
    /// knows where the occurrence ended up.
    fn claim(&mut self) -> bool {
        let selected = self.seen == self.sel;
        self.seen += 1;
        selected
    }

    fn place(&mut self, rect: Rect) {
        self.rect = Some(rect);
    }
}

impl Help {
    /// Jump the outline (and content) to a heading slug over the next few frames.
    fn go_to(&mut self, slug: String) {
        self.active = slug.clone();
        self.scroll_to = Some(slug);
        self.scroll_frames = 3;
    }

    /// While the manual is open, claim the scrolling keys for it.
    ///
    /// The events are pulled out of egui's queue rather than merely acted on,
    /// and this runs *before*
    /// [`crate::input::InputRuntime::poll_pointer_and_keys`], so an arrow key
    /// bound to tuning scrolls the manual instead of doing both at once. ←/→
    /// step whole sections, the only reading-shaped meaning a horizontal key has
    /// in a document that wraps.
    pub fn grab_keys(&mut self, ctx: &egui::Context) {
        use egui::Key;

        if !self.open {
            return;
        }

        // Ctrl+F and F3 are claimed before the focus test below, because they
        // are the two keys the operator reaches for *while* typing a search
        // term — and the find field having the keyboard is exactly the case
        // the test would bail out on. Like the scrolling keys, they are only
        // taken from the bindings while the manual is open.
        let (mut focus, mut step) = (false, 0i32);
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                let egui::Event::Key { key, pressed, modifiers, .. } = e else { return true };
                match key {
                    Key::F if modifiers.command || modifiers.ctrl => focus |= *pressed,
                    Key::F3 => {
                        if *pressed {
                            step = if modifiers.shift { -1 } else { 1 };
                        }
                    }
                    _ => return true,
                }
                false
            });
        });
        self.focus_find |= focus;
        if step != 0 {
            self.step_hit(step);
        }

        // A focused text field elsewhere (the settings dialog can sit on top of
        // the manual) owns the keyboard; leave its arrows alone. So does the
        // find field, whose arrows move the text cursor.
        if ctx.egui_wants_keyboard_input() || ctx.memory(|m| m.focused()).is_some() {
            return;
        }

        let page = (self.viewport_h * PAGE_FRACTION).max(ARROW_STEP);
        let mut dy = 0.0f32;
        // Home/End are absolute, so they are resolved after the scan and win
        // over any line/page steps in the same batch.
        let mut jump: Option<bool> = None;
        let mut section = 0i32;
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                let egui::Event::Key { key, pressed, .. } = e else { return true };
                // Key repeats arrive as further presses, which is what gives a
                // held arrow its run-on scroll. Releases are dropped too, so a
                // binding can never see half a keystroke.
                let step = match key {
                    Key::ArrowUp => -ARROW_STEP,
                    Key::ArrowDown => ARROW_STEP,
                    Key::PageUp => -page,
                    Key::PageDown => page,
                    Key::Home | Key::End => {
                        if *pressed {
                            jump = Some(*key == Key::End);
                        }
                        return false;
                    }
                    Key::ArrowLeft | Key::ArrowRight => {
                        if *pressed {
                            section += if *key == Key::ArrowRight { 1 } else { -1 };
                        }
                        return false;
                    }
                    _ => return true,
                };
                if *pressed {
                    dy += step;
                }
                false
            });
        });

        if let Some(to_end) = jump {
            let max = (self.content_h - self.viewport_h).max(0.0);
            dy = if to_end { max - self.offset_y } else { -self.offset_y };
        }
        if section != 0 {
            self.step_section(section);
        } else {
            self.key_scroll += dy;
        }
    }

    /// Move `dir` entries through the navigation outline, so ←/→ page the manual
    /// by section. Clamped rather than wrapped: running off the end of the
    /// manual by holding a key is not what the operator meant.
    fn step_section(&mut self, dir: i32) {
        let n = self.doc.nav.len() as i32;
        if n == 0 {
            return;
        }
        let cur =
            self.doc.nav.iter().position(|e| e.slug == self.active).map(|i| i as i32).unwrap_or(0);
        let i = (cur + dir).clamp(0, n - 1) as usize;
        self.go_to(self.doc.nav[i].slug.clone());
    }

    /// The term to look for — empty (the find bar is idle) when nothing but
    /// whitespace has been typed, so a stray space does not light up the whole
    /// manual.
    fn needle(&self) -> &str {
        if self.search.trim().is_empty() { "" } else { self.search.as_str() }
    }

    /// Select the next / previous match, wrapping at the ends. Wrapping is
    /// right here where stepping *sections* clamps: a search that has run off
    /// the last hit means "start again", while paging off the last chapter
    /// means nothing.
    ///
    /// The count is the one the last frame drew, which is also the count the
    /// tally shows, so the buttons and the readout always agree.
    fn step_hit(&mut self, dir: i32) {
        if self.hits == 0 {
            return;
        }
        let n = self.hits as i32;
        self.hit = (self.hit as i32 + dir).rem_euclid(n) as usize;
        self.seek_frames = 3;
    }

    /// Recount the outline badges. One pass over the flattened blocks, run only
    /// when the term changes rather than on every frame.
    fn recount_nav(&mut self) {
        self.nav_needle = self.search.clone();
        self.nav_hits.clear();
        self.nav_hits.resize(self.doc.nav.len(), 0);
        let needle = if self.search.trim().is_empty() { "" } else { self.search.as_str() };
        if needle.is_empty() {
            return;
        }
        for (i, text) in self.doc.text.iter().enumerate() {
            let n = count_ci(text, needle);
            if n == 0 {
                continue;
            }
            // Both owners: a hit in 5.2.3 belongs to that subsection's row and
            // to chapter 5's, because the reader scanning the outline for
            // somewhere to look is reading the chapters.
            for row in self.doc.owner[i].iter().flatten() {
                self.nav_hits[*row] += n;
            }
        }
    }

    /// The find bar across the top of the window: the term, the two step
    /// buttons and the tally. Everything it changes takes effect in the same
    /// frame, in the body drawn below it.
    fn find_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            // Spelled out rather than drawn: the bundled fonts have no
            // magnifier glyph (nor ⌕), and a tofu box is worse than a word.
            ui.label(RichText::new(tr!("help-find")).color(theme::CYAN_DIM()).size(10.0).strong());

            // The buttons and the tally are budgeted first so the field takes
            // what is left: on a phone that is a stub, but it is still a field.
            let reserved = 230.0;
            let w = (ui.available_width() - reserved).clamp(70.0, 460.0);
            let before = self.search.clone();
            let field = crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.search)
                    .desired_width(w)
                    .hint_text(tr!("help-find-manual"))
                    .text_color(theme::TEXT_STRONG()),
            );
            if std::mem::take(&mut self.focus_find) {
                field.request_focus();
            }
            if self.search != before {
                // A new term: back to the first hit, and one more repaint so
                // the tally — drawn above the body that counts it — catches up
                // with the number the body is about to arrive at.
                self.hit = 0;
                self.seek_frames = 3;
                ui.ctx().request_repaint();
            }
            // Enter steps like the buttons, and keeps the field, so a run of
            // presses walks the matches. (egui surrenders focus on Enter in a
            // single-line field, hence the re-request.)
            if field.has_focus() {
                let (enter, shift) =
                    ui.input(|i| (i.key_pressed(egui::Key::Enter), i.modifiers.shift));
                if enter {
                    self.step_hit(if shift { -1 } else { 1 });
                    field.request_focus();
                }
            }

            let any = self.hits > 0;
            if crate::chrome::chip_enabled(ui, any, false, "◀")
                .on_hover_text(tr!("help-previous-match-shift-enter-shift"))
                .clicked()
            {
                self.step_hit(-1);
            }
            if crate::chrome::chip_enabled(ui, any, false, "▶")
                .on_hover_text(tr!("help-next-match-enter-f3"))
                .clicked()
            {
                self.step_hit(1);
            }
            let (tally, color) = if self.needle().is_empty() {
                (String::new(), theme::CYAN_DIM())
            } else if self.hits == 0 {
                (tr!("help-no-matches").to_string(), theme::ALERT())
            } else {
                (format!("{} / {}", self.hit + 1, self.hits), theme::CYAN())
            };
            ui.label(RichText::new(tally).color(color).size(12.0));

            // Next to the tally, not out at the right edge: a second ✕ sitting
            // under the window's own close button is a trap. (`×`, because the
            // heavier ✕ is one of the glyphs the bundled fonts lack.)
            if !self.search.is_empty()
                && crate::chrome::chip(ui, false, "×")
                    .on_hover_text(tr!("help-clear-search-esc"))
                    .clicked()
            {
                self.clear_search();
            }
        });
    }

    /// Drop the search term and everything derived from it.
    fn clear_search(&mut self) {
        self.search.clear();
        self.hit = 0;
        self.hits = 0;
        self.seek_frames = 0;
    }

    /// Draw the help window if open. Self-contained: needs only the egui
    /// context, so it never touches the radio controller.
    pub fn ui(&mut self, ctx: &egui::Context) {
        if !self.open {
            // Re-opening starts a fresh window, sized to the viewport again.
            self.shown = false;
            return;
        }
        // Esc closes, matching the other overlays — but a search in progress
        // is dismissed first, the way every find bar behaves.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.search.is_empty() {
                self.open = false;
                return;
            }
            self.clear_search();
        }

        // A pending scroll target set by a link click last frame, or a nav
        // click this frame (below). Held for a few frames so it survives
        // late-loading image layout shifts.
        let mut target = self.scroll_to.clone();
        // Keyboard scroll banked by `grab_keys`, taken out here so it is spent
        // exactly once even if the window turns out to be closed or collapsed.
        // A heading jump in flight wins: it is the more specific request.
        let key_scroll = std::mem::take(&mut self.key_scroll);
        let key_scroll = if target.is_some() { 0.0 } else { key_scroll };

        // Hits counted by this frame's drawing, once it has happened. `None`
        // if the window did not draw at all, which must leave the tally alone
        // rather than reset it to zero.
        let mut counted: Option<usize> = None;

        let mut open = self.open;
        let win = egui::Window::new(tr!("help-sdroxide-manual"))
            .id(crate::layout::salted_id(ctx, "SDROXIDE MANUAL"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .collapsible(false);
        // The manual opens filling the viewport: it is a reference document,
        // and at the old 940 pt a screenshot alone took the width.
        //
        // `default_*` cannot do this, because egui persists a window's size
        // and only consults the default for a window it has never seen — the
        // second F1 of the session would come back at whatever size the first
        // was left at. `fixed_size` for the one frame the window opens does:
        // `Resize` clamps its remembered size into [min, max], and fixed_size
        // sets both to the size we pass. Every frame after this one is
        // resizable again, starting from a full-screen window.
        let full = ctx.content_rect().shrink(8.0);
        let win = if std::mem::replace(&mut self.shown, true) {
            win.resizable(true)
                .default_width(crate::layout::window_w(ctx, 940.0))
                .default_height(crate::layout::window_h(ctx, 680.0))
                .min_width(crate::layout::window_w(ctx, 560.0))
                .min_height(crate::layout::window_h(ctx, 360.0))
        } else {
            win.current_pos(full.min).fixed_size(full.size())
        };
        let resp = win.show(ctx, |ui| {
            crate::chrome::window_body_bg(ui);

            // The find bar first: what it changes applies to the body
            // below it in this same frame, so typing highlights without a
            // frame of lag, and it takes its height out of the columns'
            // `available_height` for free.
            self.find_bar(ui);
            ui.add_space(4.0);
            if self.nav_needle != self.search {
                self.recount_nav();
            }
            // Borrowed by the renderers for the whole pass, so it is a
            // local copy rather than a field of `self`, which they also
            // need mutably (the texture cache, the active outline row).
            let needle = self.needle().to_string();
            let seek = self.seek_frames > 0;
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
            let mut pass = Pass {
                link: None,
                find: Find { needle: &needle, sel: self.hit, seen: 0, rect: None },
            };
            let mut nav_click: Option<String> = None;

            // A compact layout drops the contents column: 236 pt of it is
            // two thirds of a phone's width, spent on an index for a
            // document the reader would then have no room to read. The
            // manual's own headings still get there by scrolling.
            let nav = !crate::layout::tier(ctx).compact();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                // --- Left: navigation outline (own scroll area) ---
                const NAV_W: f32 = 236.0;
                if nav {
                    ui.allocate_ui_with_layout(
                        vec2(NAV_W, full_h),
                        Layout::top_down(Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(theme::BG_DEEP())
                                .inner_margin(egui::Margin { left: 8, right: 8, top: 8, bottom: 8 })
                                .show(ui, |ui| {
                                    ui.set_min_height(full_h - 16.0);
                                    ui.set_width(NAV_W - 16.0);
                                    ui.label(
                                        RichText::new(tr!("help-contents"))
                                            .color(theme::CYAN_DIM())
                                            .size(10.0)
                                            .strong(),
                                    );
                                    ui.add_space(6.0);
                                    egui::ScrollArea::vertical()
                                        .id_salt("help_nav")
                                        .auto_shrink([false, false])
                                        .show_themed(ui, |ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            for (i, entry) in self.doc.nav.iter().enumerate() {
                                                let hits =
                                                    self.nav_hits.get(i).copied().unwrap_or(0);
                                                if nav_item(
                                                    ui,
                                                    entry,
                                                    entry.slug == self.active,
                                                    hits,
                                                ) {
                                                    nav_click = Some(entry.slug.clone());
                                                }
                                            }
                                        });
                                });
                        },
                    );

                    // Divider between the panes.
                    let (drect, _) = ui.allocate_exact_size(vec2(9.0, full_h), Sense::hover());
                    ui.painter().vline(
                        drect.center().x,
                        drect.y_range(),
                        Stroke::new(1.0, theme::LINE_LIT()),
                    );
                }

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
                            .show_themed(ui, |ui| {
                                ui.set_width(ui.available_width() - 6.0);
                                // `scroll_with_delta` moves the *content*, so
                                // scrolling down means shifting it up.
                                if key_scroll != 0.0 {
                                    ui.scroll_with_delta(vec2(0.0, -key_scroll));
                                }
                                let mut heading_tops: Vec<(String, f32)> = Vec::new();
                                for (idx, block) in self.doc.blocks.iter().enumerate() {
                                    draw_block(
                                        ui,
                                        idx,
                                        block,
                                        &mut self.textures,
                                        target.as_deref(),
                                        &mut heading_tops,
                                        &mut pass,
                                    );
                                }
                                // Bring the selected match into view — but
                                // only while a step is in flight, so the
                                // reader can scroll away from a hit and
                                // stay where they went. A heading jump
                                // asked for first wins: both end up
                                // setting the pane's scroll target, and it
                                // is the more specific request.
                                if seek
                                    && target.is_none()
                                    && let Some(r) = pass.find.rect
                                {
                                    ui.scroll_to_rect(r, Some(Align::Center));
                                }
                                heading_tops
                            });

                        // Remembered for the next frame's Page/Home/End keys,
                        // which need to know how tall a page is and how far
                        // there is left to travel.
                        self.viewport_h = out.inner_rect.height();
                        self.content_h = out.content_size.y;
                        self.offset_y = out.state.offset.y;

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

            counted = Some(pass.find.seen);

            // Apply a link click: an in-page anchor jumps the outline, an
            // external URL opens in the browser.
            if let Some(href) = pass.link.take() {
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

        // Adopt this frame's hit count. Fewer matches than before (a longer
        // term) can leave the selection past the end, which starts again at
        // the first — the same place typing a term lands.
        if let Some(hits) = counted {
            self.hits = hits;
            if self.hit >= hits {
                self.hit = 0;
            }
        }
        if self.seek_frames > 0 {
            ctx.request_repaint();
            self.seek_frames -= 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Navigation outline
// ---------------------------------------------------------------------------

/// One clickable outline row. Level-3 entries are indented; the active row gets
/// a yellow accent bar and a faint fill. Returns true when clicked.
///
/// `hits` is how many times the search term appears in this row's section
/// (subsections counting towards their chapter as well). A row that holds any
/// gets a warm wash and its tally on the right, which is what turns the
/// outline into a map of where the term lives.
fn nav_item(ui: &mut Ui, entry: &NavEntry, active: bool, hits: usize) -> bool {
    let indent = if entry.level >= 3 { 15.0 } else { 2.0 };
    let size = if entry.level >= 3 { 12.0 } else { 13.0 };
    let color = if active {
        theme::CYAN()
    } else if hits > 0 {
        theme::YELLOW()
    } else if entry.level >= 3 {
        theme::TEXT()
    } else {
        theme::TEXT_STRONG()
    };
    let avail = ui.available_width();
    let badge = (hits > 0).then(|| {
        let font = FontId::new(10.0, FontFamily::Proportional);
        ui.painter().layout_no_wrap(hits.to_string(), font, theme::YELLOW())
    });
    let badge_w = badge.as_ref().map_or(0.0, |g| g.size().x + 8.0);
    let text_w = (avail - indent - 10.0 - badge_w).max(40.0);
    let font = FontId::new(size, FontFamily::Proportional);
    let galley = ui.painter().layout(entry.label.clone(), font, color, text_w);
    let h = galley.size().y + 6.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(avail, h), Sense::click());

    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if active {
            p.rect_filled(rect, 0.0, theme::FILL());
        } else if resp.hovered() {
            // Hover wins over the search wash, or a badged row would be the
            // one row in the outline that does not answer the pointer.
            p.rect_filled(rect, 0.0, theme::ROW_HOVER());
        } else if hits > 0 {
            p.rect_filled(rect, 0.0, theme::YELLOW().gamma_multiply(0.16));
        }
        if active {
            p.rect_filled(
                Rect::from_min_max(rect.left_top(), pos2(rect.left() + 3.0, rect.bottom())),
                0.0,
                theme::YELLOW(),
            );
        }
        let ty = rect.center().y - galley.size().y / 2.0;
        p.galley(pos2(rect.left() + indent + 6.0, ty), galley, color);
        if let Some(badge) = badge {
            let at =
                pos2(rect.right() - badge.size().x - 4.0, rect.center().y - badge.size().y / 2.0);
            p.galley(at, badge, theme::YELLOW());
        }
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
    pass: &mut Pass,
) {
    match block {
        Block::Heading { level, text, slug } => {
            // Breathing room above the bar: a clear break before a chapter, a
            // smaller one before a subsection. The first block needs neither.
            ui.add_space(if idx == 0 {
                0.0
            } else if *level <= 2 {
                68.0
            } else {
                28.0
            });
            let resp = draw_header(ui, *level, &plain_text(text), &mut pass.find);
            if *level == 2 || *level == 3 {
                heading_tops.push((slug.clone(), resp.rect.top()));
            }
            if scroll_target == Some(slug.as_str()) {
                resp.scroll_to_me(Some(Align::TOP));
            }
            ui.add_space(if idx == 0 {
                6.0
            } else if *level <= 2 {
                24.0
            } else {
                12.0
            });
        }
        Block::Paragraph(inl) => {
            draw_inline(ui, inl, theme::TEXT(), false, pass);
            ui.add_space(7.0);
        }
        Block::Bullets(items) => {
            for item in items {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(8.0);
                    ui.label(RichText::new("▸ ").color(theme::YELLOW()).strong());
                    draw_inline(ui, item, theme::TEXT(), false, pass);
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
                    ui.label(RichText::new(format!("{num}. ")).color(theme::CYAN()).strong());
                    draw_inline(ui, item, theme::TEXT(), false, pass);
                });
                ui.add_space(2.0);
            }
            ui.add_space(5.0);
        }
        Block::Quote(inl) => {
            ui.add_space(4.0);
            let inner = egui::Frame::new()
                .fill(theme::CQ_BG())
                .inner_margin(egui::Margin { left: 13, right: 10, top: 7, bottom: 7 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width() - 23.0);
                    draw_inline(ui, inl, theme::TEXT_STRONG(), false, pass);
                });
            let r = inner.response.rect;
            ui.painter().rect_filled(
                Rect::from_min_max(r.left_top(), pos2(r.left() + 3.0, r.bottom())),
                0.0,
                theme::YELLOW(),
            );
            ui.add_space(7.0);
        }
        Block::Code(code) => {
            ui.add_space(3.0);
            egui::Frame::new()
                .fill(theme::INPUT_BG())
                .stroke(Stroke::new(1.0, theme::LINE()))
                .inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width() - 20.0);
                    for line in code.lines() {
                        draw_found_row(ui, line, &mut pass.find, |s| {
                            RichText::new(s).monospace().color(theme::GREEN())
                        });
                    }
                });
            ui.add_space(6.0);
        }
        Block::Rule => {
            ui.add_space(9.0);
            let w = ui.available_width();
            let (rect, _) = ui.allocate_exact_size(vec2(w, 6.0), Sense::hover());
            crate::chrome::hazard_stripes(&ui.painter().clone(), rect, 11.0);
            ui.add_space(9.0);
        }
        Block::Image { alt, path } => {
            ui.add_space(5.0);
            match ensure_texture(textures, ui.ctx(), path) {
                Some(tex) => {
                    let nat = tex.size_vec2();
                    let w = nat.x.min(ui.available_width() - 8.0).min(900.0);
                    egui::Frame::new()
                        .stroke(Stroke::new(1.0, theme::LINE_LIT()))
                        .inner_margin(3)
                        .show(ui, |ui| {
                            ui.add(egui::Image::new(tex).max_width(w).corner_radius(0));
                        });
                }
                None => {
                    ui.colored_label(theme::ALERT(), tr!("help-missing-image", path = path));
                }
            }
            if !alt.is_empty() {
                ui.add_space(3.0);
                draw_found_row(ui, alt, &mut pass.find, |s| {
                    RichText::new(s).italics().color(theme::CYAN_DIM()).size(11.5)
                });
            }
            ui.add_space(9.0);
        }
        Block::Table { header, rows } => {
            ui.add_space(5.0);
            draw_table(ui, idx, header, rows, pass);
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
    pass: &mut Pass,
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

    egui::Frame::new().stroke(Stroke::new(1.0, theme::LINE_LIT())).inner_margin(0).show(ui, |ui| {
        ui.set_width(total);
        // Header row.
        table_row(ui, header, &col_w, spacing, theme::FILL(), true, pass, idx);
        for (ri, row) in rows.iter().enumerate() {
            let fill = if ri % 2 == 0 { theme::ROW_BG() } else { theme::PANEL() };
            table_row(ui, row, &col_w, spacing, fill, false, pass, idx);
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
    pass: &mut Pass,
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
                                let (color, strong) = if header {
                                    (theme::CYAN(), true)
                                } else {
                                    (theme::TEXT(), false)
                                };
                                draw_inline(ui, cell, color, strong, pass);
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

/// An angled section header: a cut-corner bar with a yellow/black hazard tab on
/// the left and the section title in the techno heading face. Level 1 is the
/// document title, 2 is a section, 3 a subsection.
fn draw_header(ui: &mut Ui, level: u8, text: &str, find: &mut Find) -> Response {
    let (size, bar_h, accent, fill, txt_col) = match level {
        1 => (23.0, 50.0, theme::CYAN(), theme::FILL(), theme::TEXT_STRONG()),
        2 => (17.0, 37.0, theme::PINK(), theme::FILL(), theme::CYAN()),
        _ => (14.5, 29.0, theme::YELLOW(), theme::ROW_BG(), theme::TEXT_STRONG()),
    };
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, bar_h), Sense::hover());

    // Claimed before the visibility test, and before the painting below can
    // return early: an occurrence's document-order index must not depend on
    // whether its heading happens to be on screen.
    let marks: Vec<(usize, usize, bool)> = matches(text, find.needle)
        .into_iter()
        .map(|(s, e)| {
            let selected = find.claim();
            if selected {
                find.place(rect);
            }
            (s, e, selected)
        })
        .collect();

    if ui.is_rect_visible(rect) {
        let p = ui.painter().clone();
        let cut = 9.0f32.min(bar_h * 0.42);
        // Bar fill (already beveled) sitting on the panel background.
        p.add(Shape::convex_polygon(cut_outline(rect, cut), fill, Stroke::NONE));
        // Hazard-stripe accent tab down the left edge.
        let tab = Rect::from_min_max(rect.left_top(), pos2(rect.left() + 13.0, rect.bottom()));
        crate::chrome::hazard_stripes(&p, tab, 8.0);
        // Mask the two square corners back to the panel colour so the bevel
        // reads as a real cut (this also clips the hazard tab's overflow), then
        // stroke the angled outline.
        let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
        p.add(Shape::convex_polygon(
            vec![pos2(r - cut, t), pos2(r, t), pos2(r, t + cut)],
            theme::PANEL(),
            Stroke::NONE,
        ));
        p.add(Shape::convex_polygon(
            vec![pos2(l, b - cut), pos2(l + cut, b), pos2(l, b)],
            theme::PANEL(),
            Stroke::NONE,
        ));
        p.add(Shape::closed_line(cut_outline(rect, cut), Stroke::new(1.4, accent)));
        // Title text, clear of the hazard tab.
        let font = FontId::new(size, FontFamily::Name("chakra-bold".into()));
        let galley = p.layout_no_wrap(text.to_string(), font.clone(), txt_col);
        let tx = rect.left() + 24.0;
        let ty = rect.center().y - galley.size().y / 2.0;
        p.galley(pos2(tx, ty), galley, txt_col);
        // Found terms are painted over the title: the run before the match
        // measures where it starts, and the match itself is re-laid in the
        // highlight's ink so it stays readable on a bright wash.
        for (s, e, selected) in marks {
            let (bg, ink) = hit_colors(selected);
            let lead = p.layout_no_wrap(text[..s].to_string(), font.clone(), ink);
            let hit = p.layout_no_wrap(text[s..e].to_string(), font.clone(), ink);
            let at = pos2(tx + lead.size().x, ty);
            p.rect_filled(Rect::from_min_size(at, hit.size()).expand2(vec2(1.0, 0.0)), 0.0, bg);
            p.galley(at, hit, ink);
        }
    }
    resp
}

// ---------------------------------------------------------------------------
// Inline rendering
// ---------------------------------------------------------------------------

/// Render a run of inline spans, wrapping at the available width. Zero
/// horizontal item spacing keeps adjacent styled runs from gaining stray gaps.
fn draw_inline(ui: &mut Ui, inl: &[Inline], base: Color32, strong: bool, pass: &mut Pass) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().item_spacing.y = 3.0;
        draw_runs(ui, inl, strong, false, base, pass);
    });
}

fn draw_runs(
    ui: &mut Ui,
    inl: &[Inline],
    strong: bool,
    italic: bool,
    base: Color32,
    pass: &mut Pass,
) {
    for span in inl {
        match span {
            Inline::Text(t) => {
                let col = if strong && base == theme::TEXT() { theme::TEXT_STRONG() } else { base };
                draw_found(ui, t, &mut pass.find, |s| {
                    let mut rt = RichText::new(s).color(col);
                    if strong {
                        rt = rt.strong();
                    }
                    if italic {
                        rt = rt.italics();
                    }
                    rt
                });
            }
            Inline::Code(c) => {
                // A code span stays one chip rather than being split at the
                // match: the padded box is the point of it. It takes the
                // highlight whole, and counts every occurrence inside it so
                // the tally still adds up.
                let hits = matches(c, pass.find.needle);
                let selected = hits.iter().fold(false, |acc, _| pass.find.claim() || acc);
                let (bg, fg) = if hits.is_empty() {
                    (theme::INPUT_BG(), theme::CYAN())
                } else {
                    hit_colors(selected)
                };
                // Padded with no-break spaces so the highlight doesn't crowd
                // the glyphs.
                let resp = ui.label(
                    RichText::new(format!("\u{00a0}{c}\u{00a0}"))
                        .monospace()
                        .color(fg)
                        .background_color(bg),
                );
                if selected {
                    pass.find.place(resp.rect);
                }
            }
            Inline::Bold(v) => draw_runs(ui, v, true, italic, base, pass),
            Inline::Italic(v) => draw_runs(ui, v, strong, true, base, pass),
            Inline::Link { text, href } => {
                let label = plain_text(text);
                let style = |s: &str| RichText::new(s).color(theme::CYAN()).underline();
                let resp = if matches(&label, pass.find.needle).is_empty() {
                    ui.add(egui::Label::new(style(&label)).sense(Sense::click()))
                } else {
                    // Highlighting splits the label into several, so the click
                    // is taken from the box those pieces occupy between them
                    // rather than from any one of them.
                    let rect = draw_found(ui, &label, &mut pass.find, style);
                    let id = ui.next_auto_id();
                    ui.skip_ahead_auto_ids(1);
                    ui.interact(rect, id, Sense::click())
                };
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    pass.link = Some(href.clone());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Find in page
// ---------------------------------------------------------------------------

/// Every occurrence of `needle` in `hay`, as byte ranges in order. Empty when
/// nothing is being looked for, which is the idle case and allocates nothing.
///
/// The comparison is byte-wise and ASCII-case-insensitive, which is safe on
/// UTF-8 text: a valid needle's bytes can only ever occur at character
/// boundaries, and no non-ASCII byte is folded. So `SSTV` finds `sstv` and
/// `Grüße` still finds itself.
fn matches(hay: &str, needle: &str) -> Vec<(usize, usize)> {
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    if n.is_empty() || h.len() < n.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + n.len() <= h.len() {
        if h[i..i + n.len()].eq_ignore_ascii_case(n) {
            out.push((i, i + n.len()));
            i += n.len();
        } else {
            i += 1;
        }
    }
    out
}

/// How many times `needle` appears in `hay` — [`matches`] without the vector,
/// for the outline badges, which count a few hundred blocks per keystroke.
fn count_ci(hay: &str, needle: &str) -> usize {
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    if n.is_empty() || h.len() < n.len() {
        return 0;
    }
    let (mut i, mut count) = (0, 0);
    while i + n.len() <= h.len() {
        if h[i..i + n.len()].eq_ignore_ascii_case(n) {
            count += 1;
            i += n.len();
        } else {
            i += 1;
        }
    }
    count
}

/// Background and ink for a found term: the selected hit takes the yellow this
/// UI uses for "you are here" everywhere else, the rest a cyan wash that reads
/// as found without competing with it.
fn hit_colors(selected: bool) -> (Color32, Color32) {
    if selected {
        (theme::YELLOW(), theme::INK_ON_BRIGHT())
    } else {
        (theme::CYAN().gamma_multiply(0.32), theme::TEXT_STRONG())
    }
}

/// Draw one run of text, split around any occurrence of the find term so each
/// can be highlighted and the selected one scrolled to. `style` gives a piece
/// its normal look; a highlighted piece takes that and overrides the colours.
/// Returns the box the pieces occupy.
///
/// Splitting a run into several labels is what lets the highlight sit *inside*
/// a sentence, and costs a wrap opportunity at each seam — which is why it
/// only happens for runs that actually match.
fn draw_found(
    ui: &mut Ui,
    text: &str,
    find: &mut Find,
    style: impl Fn(&str) -> RichText,
) -> egui::Rect {
    let hits = matches(text, find.needle);
    if hits.is_empty() {
        return ui.label(style(text)).rect;
    }
    let mut union = Rect::NOTHING;
    let mut at = 0;
    for (s, e) in hits {
        if s > at {
            union = union.union(ui.label(style(&text[at..s])).rect);
        }
        let selected = find.claim();
        let (bg, ink) = hit_colors(selected);
        let rect = ui.label(style(&text[s..e]).color(ink).background_color(bg)).rect;
        if selected {
            find.place(rect);
        }
        union = union.union(rect);
        at = e;
    }
    if at < text.len() {
        union = union.union(ui.label(style(&text[at..])).rect);
    }
    union
}

/// [`draw_found`] for a label that sits in a vertical layout — a line of a code
/// block, an image caption. The split pieces have to be laid out side by side,
/// but an unsplit one stays a plain label so an empty line keeps its height.
fn draw_found_row(ui: &mut Ui, text: &str, find: &mut Find, style: impl Fn(&str) -> RichText) {
    if matches(text, find.needle).is_empty() {
        ui.label(style(text));
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            draw_found(ui, text, find, style);
        });
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
        } else if c == '_' {
            // GitHub keeps underscores verbatim (`rtl_tcp` stays `rtl_tcp`).
            out.push('_');
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
            collect_list(
                &lines,
                &mut i,
                |content| items.push(parse_inline(content)),
                is_bullet,
                strip_bullet,
            );
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
        //
        // The *first* line is always taken, even when it looks like the start
        // of a block. Reaching here means every branch above declined it — a
        // table row whose header got separated from it, a `#heading` with no
        // space after the hash, an image line that would not parse — and
        // re-testing it here would break out of this loop with `i` exactly
        // where it started, leaving the outer loop to reconsider the same line
        // forever.
        //
        // That is not a cosmetic bug. This runs inside the eframe app-creation
        // closure, before the first frame, so a parser that never returns is a
        // program that opens no window at all: audio and the engine come up,
        // the process sits there, and nothing on screen says why. One stray
        // newline inside a table row in `USER_MANUAL.md` did exactly that.
        // Malformed markdown may render badly; it may not hang the GUI.
        let start = i;
        let mut buf = String::new();
        while i < lines.len() {
            let l = lines[i];
            let lt = l.trim();
            if lt.is_empty() || (i > start && is_block_start(l)) {
                break;
            }
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(lt);
            i += 1;
        }
        // Belt and braces: whatever the branches above do in future, this loop
        // is the last one and it must always move.
        debug_assert!(i > start, "parse_blocks made no progress at line {start}");
        i = i.max(start + 1);
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
        if c == '`'
            && let Some(end) = find_char(&chars, i + 1, '`')
        {
            flush(&mut buf, &mut out);
            out.push(Inline::Code(chars[i + 1..end].iter().collect()));
            i = end + 1;
            continue;
        }

        // Link `[text](href)`.
        if c == '['
            && let Some((text, href, next)) = parse_link(&chars, i)
        {
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
    (from + 1..chars.len()).find(|&k| {
        chars[k] == '_' && chars.get(k + 1).map(|c| !c.is_alphanumeric()).unwrap_or(true)
    })
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

    /// Markdown that no block branch claims must still be consumed.
    ///
    /// Each of these used to spin `parse_blocks` on one line forever. Because
    /// the parse happens inside the eframe app-creation closure, before the
    /// first frame, the symptom was not a broken help page — it was a program
    /// that started, brought up the radio and the audio, and then opened no
    /// window at all, with nothing on screen to say why. A stray newline in the
    /// middle of a table row in `USER_MANUAL.md` was enough to cause it: the
    /// rows after the break were orphaned from their header, and an orphaned
    /// `|` row is a line that looks like a block and belongs to none.
    ///
    /// In a debug build the `debug_assert!` in `parse_blocks` turns a
    /// reintroduced hang into a panic here rather than a test that never ends.
    #[test]
    fn markdown_that_matches_no_block_is_still_consumed() {
        for (name, md) in [
            ("orphaned table row", "| a | b |\n| c | d |\n"),
            ("table whose header lost its row", "| File | Format |\nnot a row |\n| x | y |\n"),
            ("hash with no space", "#nospace\n"),
            ("image that will not parse", "![unterminated\n"),
            ("a lone pipe", "|\n"),
            ("pipe after a paragraph", "text\n| orphan |\nmore text\n"),
        ] {
            let blocks = parse_blocks(md);
            assert!(!blocks.is_empty(), "{name}: parsed to nothing");
        }
        // And the ordinary constructs still parse as themselves, so the
        // always-take-the-first-line rule has not swallowed real blocks.
        let doc = "# Title\n\ntext\n\n- one\n- two\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        let blocks = parse_blocks(doc);
        assert!(matches!(blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(blocks[1], Block::Paragraph(_)));
        assert!(matches!(blocks[2], Block::Bullets(ref v) if v.len() == 2));
        assert!(matches!(blocks[3], Block::Table { ref rows, .. } if rows.len() == 1));
    }

    /// The shipped manual must parse, because it is parsed on the way to the
    /// first frame. This is the test that would have caught the broken table
    /// row before it ever reached a running program.
    #[test]
    fn the_bundled_manual_parses() {
        let blocks = parse_blocks(MANUAL_MD);
        assert!(blocks.len() > 500, "only {} blocks — the manual did not parse", blocks.len());
        // A table row split across two lines orphans every row below it, which
        // shows up as the table count collapsing rather than as an error.
        let tables = blocks.iter().filter(|b| matches!(b, Block::Table { .. })).count();
        assert!(tables > 10, "only {tables} tables — a table row is probably split over two lines");
    }

    /// One key-down event, no modifiers.
    fn press(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Run one egui pass with `events` and let the help window claim what it
    /// wants, returning it plus what survived in the queue.
    fn grab(help: &mut Help, events: Vec<egui::Event>) -> Vec<egui::Event> {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput { events, ..Default::default() });
        help.grab_keys(&ctx);
        let left = ctx.input(|i| i.events.clone());
        ctx.end_pass().drop_without_applying_deltas();
        left
    }

    /// Arrow and page keys must scroll the manual *and* be taken off the queue —
    /// otherwise the same press also drives whatever the operator has them bound
    /// to, and reading the manual tunes the radio.
    #[test]
    fn scroll_keys_are_claimed_while_open() {
        let mut help = Help { open: true, viewport_h: 500.0, ..Help::default() };
        let left = grab(&mut help, vec![press(egui::Key::ArrowDown), press(egui::Key::PageDown)]);
        assert!(left.is_empty(), "scroll keys should be consumed, got {left:?}");
        assert!(help.key_scroll > ARROW_STEP, "both keys should have banked scroll");

        // Up cancels down, so a wobble on the keyboard doesn't drift.
        help.key_scroll = 0.0;
        grab(&mut help, vec![press(egui::Key::ArrowDown), press(egui::Key::ArrowUp)]);
        assert_eq!(help.key_scroll, 0.0);
    }

    /// A closed manual must not touch the keyboard at all.
    #[test]
    fn scroll_keys_pass_through_while_closed() {
        let mut help = Help::default();
        let left = grab(&mut help, vec![press(egui::Key::ArrowDown)]);
        assert_eq!(left.len(), 1, "a closed manual must leave the bindings alone");
        assert_eq!(help.key_scroll, 0.0);
    }

    /// Home/End travel to the ends of the manual from wherever the pane is, and
    /// override any line steps arriving in the same batch.
    #[test]
    fn home_and_end_reach_the_ends() {
        let mut help = Help { open: true, viewport_h: 400.0, content_h: 3000.0, ..Help::default() };
        help.offset_y = 1000.0;
        grab(&mut help, vec![press(egui::Key::ArrowUp), press(egui::Key::End)]);
        assert_eq!(help.key_scroll, 3000.0 - 400.0 - 1000.0);

        help.key_scroll = 0.0;
        grab(&mut help, vec![press(egui::Key::Home)]);
        assert_eq!(help.key_scroll, -1000.0);
    }

    /// ←/→ step the outline instead of scrolling, and stop at the ends.
    #[test]
    fn left_right_step_sections() {
        let mut help = Help { open: true, ..Help::default() };
        let first = help.doc.nav[0].slug.clone();
        let second = help.doc.nav[1].slug.clone();

        grab(&mut help, vec![press(egui::Key::ArrowRight)]);
        assert_eq!(help.active, second);
        assert_eq!(help.scroll_to.as_deref(), Some(second.as_str()));
        assert_eq!(help.key_scroll, 0.0, "a section jump is not a scroll");

        grab(&mut help, vec![press(egui::Key::ArrowLeft)]);
        assert_eq!(help.active, first);
        // Already at the top of the outline: stay put rather than wrapping.
        grab(&mut help, vec![press(egui::Key::ArrowLeft)]);
        assert_eq!(help.active, first);
    }

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
        // Underscores survive; only the parentheses and dots are dropped.
        assert_eq!(
            slugify("5.2.11 RTL-SDR over rtl_tcp (network dongles)"),
            "5211-rtl-sdr-over-rtl_tcp-network-dongles"
        );
    }

    /// The find is case-insensitive, non-overlapping, and never splits a
    /// character: a byte-wise scan over UTF-8 text has to land on boundaries
    /// or the slicing in `draw_found` would panic on a manual full of ←, ▸ and
    /// °.
    #[test]
    fn matching_is_case_insensitive_and_utf8_safe() {
        assert_eq!(matches("SSTV, sstv and Sstv", "sstv").len(), 3);
        assert_eq!(matches("anything", ""), vec![], "an idle find bar matches nothing");
        assert_eq!(matches("aaaa", "aa"), vec![(0, 2), (2, 4)], "matches do not overlap");
        assert_eq!(count_ci("Wefax, WEFAX", "wefax"), 2);

        // Multi-byte text: the ranges must be slice-able, and a non-ASCII
        // needle must not be found inside a different character.
        let hay = "← tune ° Grüße ▸ grüße";
        for (s, e) in matches(hay, "GRÜSSE") {
            assert!(hay.is_char_boundary(s) && hay.is_char_boundary(e));
        }
        assert_eq!(matches(hay, "grüße").len(), 2, "only the ASCII half folds case");
        for (s, e) in matches(hay, "grüße") {
            assert_eq!(&hay[s..e].to_lowercase(), "grüße");
        }
        assert_eq!(matches("°", "\u{00b0}").len(), 1);
    }

    /// A hit anywhere in a section lights up that section's outline row *and*
    /// the chapter above it — that is the whole point of the badges: to say
    /// which chapter to go and read.
    #[test]
    fn outline_badges_count_the_chapter_a_hit_sits_in() {
        let md = "\
# Manual

## 1. Antennas

Text about dipoles.

### 1.1 Verticals

A vertical is a dipole stood on end.

## 2. Filters

Nothing to see here.
";
        let mut help = Help { doc: Doc::parse(md), ..Help::default() };
        help.search = "dipole".to_string();
        help.recount_nav();

        fn row(help: &Help, slug: &str) -> usize {
            let i = help.doc.nav.iter().position(|n| n.slug == slug).expect("row exists");
            help.nav_hits[i]
        }
        assert_eq!(row(&help, "11-verticals"), 1, "the subsection holds one");
        assert_eq!(row(&help, "1-antennas"), 2, "the chapter holds its own and its subsection's");
        assert_eq!(row(&help, "2-filters"), 0, "and the chapter without the term stays unlit");

        // Searching a chapter's own title lights that chapter.
        help.search = "filters".to_string();
        help.recount_nav();
        assert_eq!(row(&help, "2-filters"), 1);
        assert_eq!(row(&help, "1-antennas"), 0);

        // An idle bar leaves the whole outline alone.
        help.search = "   ".to_string();
        help.recount_nav();
        assert!(help.nav_hits.iter().all(|&n| n == 0));
    }

    /// The blocks a section owns are the ones between its heading and the
    /// next, and every block belongs to something — an off-by-one here would
    /// badge the wrong chapter, which is worse than no badge at all.
    #[test]
    fn every_block_of_the_manual_lands_under_its_chapter() {
        let doc = Doc::parse(MANUAL_MD);
        assert_eq!(doc.owner.len(), doc.blocks.len());
        assert_eq!(doc.text.len(), doc.blocks.len());
        for (i, b) in doc.blocks.iter().enumerate() {
            let [chapter, section] = doc.owner[i];
            if let Some(c) = chapter {
                assert_eq!(doc.nav[c].level, 2, "a chapter row is a level-2 heading");
            }
            if let Some(s) = section {
                assert_eq!(doc.nav[s].level, 3);
                assert!(chapter.is_some(), "a subsection always sits in a chapter");
            }
            // The title and the table of contents come before section 1; past
            // that, every block has a chapter.
            if let Block::Heading { level: 2, slug, .. } = b {
                assert_eq!(doc.nav[chapter.expect("its own row")].slug, *slug);
            }
        }
        let orphans = doc.owner.iter().filter(|o| o[0].is_none()).count();
        assert!(orphans < doc.blocks.len() / 10, "{orphans} blocks sit outside every chapter");
    }

    /// Stepping wraps at both ends and does nothing at all with no matches —
    /// the buttons are enabled off the same count, so a mismatch would leave a
    /// live button that moves nothing.
    #[test]
    fn stepping_matches_wraps_at_both_ends() {
        let mut help = Help { open: true, ..Help::default() };
        help.step_hit(1);
        assert_eq!(help.hit, 0, "no matches, nowhere to step");
        assert_eq!(help.seek_frames, 0, "and nothing to scroll to");

        help.hits = 3;
        help.step_hit(1);
        assert_eq!(help.hit, 1);
        assert!(help.seek_frames > 0, "a step asks the pane to follow");
        help.step_hit(1);
        help.step_hit(1);
        assert_eq!(help.hit, 0, "past the last match, back to the first");
        help.step_hit(-1);
        assert_eq!(help.hit, 2, "and back off the first to the last");
    }

    /// F1 opens the manual filling the viewport, however small the window was
    /// left last time — and hands it back resizable.
    ///
    /// This is the one behaviour here built on an egui internal (`Resize`
    /// clamping its remembered size into the min/max a `fixed_size` window
    /// sets), so it is worth a test that would notice the day that changes and
    /// the manual starts coming back at whatever size it was closed at.
    #[test]
    fn the_manual_opens_filling_the_viewport() {
        // A short manual: this is about the window, and decoding forty
        // screenshots to find that out would be a slow way to ask.
        let doc = Doc::parse("# Manual\n\n## 1. One\n\nText.\n\n## 2. Two\n\nMore text.\n");
        let mut help = Help { doc, ..Help::default() };

        let ctx = egui::Context::default();
        // The section headers are drawn in the techno face, which a bare
        // context has never heard of.
        crate::theme::apply(&ctx);
        let id = crate::layout::salted_id(&ctx, "SDROXIDE MANUAL");
        let pass = |help: &mut Help, w: f32, h: f32| {
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h))),
                ..Default::default()
            });
            help.ui(&ctx);
            ctx.end_pass().drop_without_applying_deltas();
            ctx.memory(|m| m.area_rect(id)).expect("the manual is on screen")
        };

        // A session in a small window, so the manual has a remembered size.
        help.open = true;
        let small = pass(&mut help, 900.0, 700.0);
        pass(&mut help, 900.0, 700.0);
        assert!(small.width() < 950.0);

        // Closed, then opened again on a bigger screen: it must fill *that*,
        // not come back at the size it was left.
        help.open = false;
        pass(&mut help, 1600.0, 1000.0);
        help.open = true;
        let rect = pass(&mut help, 1600.0, 1000.0);
        // Within a couple of points of the viewport less its margin: the
        // window's own frame rounds to the pixel grid.
        assert!(
            (rect.width() - 1584.0).abs() < 6.0 && (rect.height() - 984.0).abs() < 6.0,
            "opened at {:?}, wanted the 1600x1000 viewport less its margin",
            rect.size()
        );
        assert!(rect.min.x < 12.0 && rect.min.y < 12.0, "and in the corner, not at {:?}", rect.min);

        // The frames after it leave the size alone rather than re-forcing it,
        // so the operator can drag the manual smaller again.
        let again = pass(&mut help, 1600.0, 1000.0);
        assert_eq!(again.size(), rect.size(), "a settled manual keeps its size");
    }

    /// Ctrl+F and F3 are claimed even while the find field owns the keyboard —
    /// that is when they are reached for — and they never reach the radio's
    /// bindings.
    #[test]
    fn find_keys_are_claimed_while_open() {
        let mut help = Help { open: true, hits: 4, ..Help::default() };
        let ctrl_f = egui::Event::Key {
            key: egui::Key::F,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        };
        let left = grab(&mut help, vec![ctrl_f, press(egui::Key::F3)]);
        assert!(left.is_empty(), "find keys should be consumed, got {left:?}");
        assert!(help.focus_find, "Ctrl+F puts the caret in the find field");
        assert_eq!(help.hit, 1, "F3 steps to the next match");

        // Plain F is the "fit the view" binding and must survive.
        let mut help = Help { open: true, ..Help::default() };
        let left = grab(&mut help, vec![press(egui::Key::F)]);
        assert_eq!(left.len(), 1, "plain F belongs to the bindings");
        assert!(!help.focus_find);
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
