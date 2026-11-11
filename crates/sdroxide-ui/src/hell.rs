//! The Hellschreiber receive raster: a scrolling strip of dot columns.
//!
//! Hell has no sync, so the vertical phase of the incoming text floats freely
//! against the receiver's column boundary — a character can straddle the wrap.
//! The fix, and the reason fldigi's display looks the way it does, is to draw
//! every column **twice, stacked**: whatever the phase, one complete legible
//! copy is always on screen.
//!
//! The strip is a ring texture written incrementally with `set_partial` and
//! scrolled by animating a UV rect over a `Repeat` sampler. The alternative —
//! re-uploading a whole `ColorImage` per batch, as the SSTV panel does per
//! scanline — would be half a megabyte twenty times a second at this size.
//!
//! Shading happens on the way into the texture, from a CPU-side copy of the raw
//! grays. That copy is what lets the contrast and reverse controls repaint the
//! entire scrollback rather than only the columns that arrive after the change.

use eframe::egui::{self, Color32};

/// Dot rows in one received column. Fixed by the mode; batches carrying
/// anything else are rejected rather than allowed to corrupt the ring.
pub const ROWS: usize = 14;

/// Rows actually drawn — every column twice, stacked. See the module docs.
pub const DISPLAY_ROWS: usize = ROWS * 2;

/// Columns of scrollback — about two minutes at Feld Hell's 17.5 columns a
/// second, for 229 kB of RGBA.
///
/// Capped at 2048 because that is the texture side WebGL2 guarantees, and this
/// crate runs in the browser. The GPU waterfall is sized the same way.
pub const RING_COLS: usize = 2048;

/// The fldigi "paper" look: dark ink on a warm light ground.
const PAPER: Color32 = Color32::from_rgb(0xe8, 0xe2, 0xd0);
const INK: Color32 = Color32::from_rgb(0x14, 0x11, 0x0c);

pub struct HellUi {
    /// Raw received grays, `RING_COLS * ROWS`, column-major. Kept so a change
    /// of contrast can re-derive the whole texture.
    gray: Vec<u8>,
    tex: Option<egui::TextureHandle>,
    /// Absolute columns received; also the ring write head.
    head: u64,
    /// The `seq` the next batch should carry.
    expect: u64,
    /// The appearance the texture was last built with.
    shade_for: (f32, f32, bool),
}

impl Default for HellUi {
    fn default() -> Self {
        HellUi {
            gray: vec![0; RING_COLS * ROWS],
            tex: None,
            head: 0,
            expect: 0,
            shade_for: (f32::NAN, f32::NAN, false),
        }
    }
}

/// Map one raw gray through the appearance controls.
fn shade(v: u8, view: &crate::view::HellView) -> Color32 {
    let x = ((v as f32 / 255.0) * view.bright).clamp(0.0, 1.0).powf(view.contrast.max(0.05));
    let x = if view.reverse { 1.0 - x } else { x };
    // 0 = blank paper, 1 = full ink.
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * x) as u8;
    Color32::from_rgb(lerp(PAPER.r(), INK.r()), lerp(PAPER.g(), INK.g()), lerp(PAPER.b(), INK.b()))
}

impl HellUi {
    /// Wipe the strip — on a mode change, a variant change, or on request.
    pub fn clear(&mut self) {
        self.gray.iter_mut().for_each(|v| *v = 0);
        self.head = 0;
        self.expect = 0;
        self.tex = None;
    }

    /// Absorb a batch of received columns.
    pub fn on_columns(
        &mut self,
        seq: u64,
        rows: u8,
        cols: &[u8],
        view: &crate::view::HellView,
        ctx: &egui::Context,
    ) {
        if rows as usize != ROWS || cols.is_empty() || cols.len() % ROWS != 0 {
            return;
        }
        // `seq` going backwards means the engine restarted the receiver (a
        // variant change, or a new mode): nothing already drawn lines up with
        // what follows.
        if seq < self.expect {
            self.clear();
        } else if seq > self.expect {
            // A batch was dropped on a backed-up wire. Leave a gap of the right
            // width rather than splicing, which would shear the text sideways.
            let gap = (seq - self.expect).min(RING_COLS as u64);
            self.blank(gap);
        }

        let n = cols.len() / ROWS;
        let start = self.head;
        for c in 0..n {
            let x = ((start + c as u64) % RING_COLS as u64) as usize;
            self.gray[x * ROWS..(x + 1) * ROWS].copy_from_slice(&cols[c * ROWS..(c + 1) * ROWS]);
        }
        self.head = start + n as u64;
        self.expect = seq + n as u64;
        self.upload(start, n, view, ctx);
    }

    /// Advance the head over `n` columns of blank paper.
    fn blank(&mut self, n: u64) {
        for c in 0..n {
            let x = ((self.head + c) % RING_COLS as u64) as usize;
            self.gray[x * ROWS..(x + 1) * ROWS].fill(0);
        }
        self.head += n;
        self.expect += n;
    }

    /// Build an `[n, DISPLAY_ROWS]` image from ring columns `x0..x0+n`, with
    /// each column's dots written into both stacked halves.
    ///
    /// Takes `gray` rather than `&self` so `upload` can hold `&mut self.tex`
    /// across the call.
    fn patch(gray: &[u8], x0: usize, n: usize, view: &crate::view::HellView) -> egui::ColorImage {
        let mut px = vec![Color32::BLACK; n * DISPLAY_ROWS];
        for c in 0..n {
            let x = (x0 + c) % RING_COLS;
            for r in 0..ROWS {
                let col = shade(gray[x * ROWS + r], view);
                // Row 0 is the **first dot transmitted**, which is the bottom of
                // the cell: Hell scans a column upwards. Drawing row 0 at the
                // top put the picture upside down, in the receive strip and in
                // the transmit echo alike (#444).
                let y = ROWS - 1 - r;
                px[y * n + c] = col;
                px[(y + ROWS) * n + c] = col;
            }
        }
        egui::ColorImage::new([n, DISPLAY_ROWS], px)
    }

    /// Push `n` columns starting at absolute index `start` into the texture,
    /// splitting into two writes when the run wraps the ring.
    fn upload(&mut self, start: u64, n: usize, view: &crate::view::HellView, ctx: &egui::Context) {
        let knobs = (view.contrast, view.bright, view.reverse);
        if self.tex.is_none() || self.shade_for != knobs {
            self.rebuild(view, ctx);
            return;
        }
        let Some(tex) = self.tex.as_mut() else { return };
        let x0 = (start % RING_COLS as u64) as usize;
        let first = n.min(RING_COLS - x0);
        tex.set_partial([x0, 0], Self::patch(&self.gray, x0, first, view), texture_options());
        if n > first {
            tex.set_partial([0, 0], Self::patch(&self.gray, 0, n - first, view), texture_options());
        }
    }

    /// Re-derive the entire texture from the stored grays. Runs on first use and
    /// whenever the appearance controls move — which is what makes the contrast
    /// slider act on the whole scrollback, not just what arrives next.
    fn rebuild(&mut self, view: &crate::view::HellView, ctx: &egui::Context) {
        let img = Self::patch(&self.gray, 0, RING_COLS, view);
        self.tex = Some(ctx.load_texture("hell_rx", img, texture_options()));
        self.shade_for = (view.contrast, view.bright, view.reverse);
    }

    /// Draw the strip into `rect`, newest column at the right edge.
    pub fn draw(&mut self, ui: &mut egui::Ui, rect: egui::Rect, view: &crate::view::HellView) {
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 2.0, if view.reverse { Color32::from_gray(8) } else { PAPER });

        let knobs = (view.contrast, view.bright, view.reverse);
        if self.tex.is_none() || self.shade_for != knobs {
            self.rebuild(view, ui.ctx());
        }
        if let Some(tex) = self.tex.as_ref() {
            let visible = (rect.width() / view.col_px.max(0.5)).max(1.0);
            let u1 = self.head as f32 / RING_COLS as f32;
            let u0 = u1 - visible / RING_COLS as f32;
            // Negative and >1 UVs are fine: the sampler wraps, which is exactly
            // how the ring becomes a continuous scroll with no per-frame copy.
            let vh = if view.doubled { 1.0 } else { 0.5 };
            // Snap the offset to a whole dot row so the raster stays crisp.
            let v0 = if view.doubled {
                0.0
            } else {
                (view.valign.clamp(0.0, 1.0) * DISPLAY_ROWS as f32).round() / DISPLAY_ROWS as f32
            };
            let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v0 + vh));
            egui::Image::new(tex).uv(uv).paint_at(ui, rect);
        }
        p.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, crate::theme::LINE_LIT()),
            egui::StrokeKind::Inside,
        );
    }
}

fn texture_options() -> egui::TextureOptions {
    let mut opt = egui::TextureOptions::NEAREST;
    // Repeat is what lets the UV window run off either edge of the ring.
    opt.wrap_mode = egui::TextureWrapMode::Repeat;
    opt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::HellView;

    fn ramp(n: usize) -> Vec<u8> {
        (0..n * ROWS).map(|i| (i % 256) as u8).collect()
    }

    #[test]
    fn a_batch_advances_the_head_by_its_column_count() {
        let mut ui = HellUi::default();
        let view = HellView::default();
        let ctx = egui::Context::default();
        ui.on_columns(0, ROWS as u8, &ramp(5), &view, &ctx);
        assert_eq!(ui.head, 5);
        assert_eq!(ui.expect, 5);
        ui.on_columns(5, ROWS as u8, &ramp(3), &view, &ctx);
        assert_eq!(ui.head, 8);
    }

    #[test]
    fn a_dropped_batch_leaves_a_gap_of_the_right_width() {
        let mut ui = HellUi::default();
        let view = HellView::default();
        let ctx = egui::Context::default();
        ui.on_columns(0, ROWS as u8, &vec![255; 2 * ROWS], &view, &ctx);
        // Pretend columns 2..7 never arrived.
        ui.on_columns(7, ROWS as u8, &vec![255; 2 * ROWS], &view, &ctx);
        assert_eq!(ui.head, 9, "the gap must occupy its real width");
        assert_eq!(ui.expect, 9);
        // The skipped span is blank, not a splice of the surrounding text.
        for x in 2..7 {
            assert!(ui.gray[x * ROWS..(x + 1) * ROWS].iter().all(|&v| v == 0), "column {x}");
        }
    }

    #[test]
    fn a_rewound_seq_clears_the_strip() {
        let mut ui = HellUi::default();
        let view = HellView::default();
        let ctx = egui::Context::default();
        ui.on_columns(0, ROWS as u8, &vec![255; 4 * ROWS], &view, &ctx);
        assert_eq!(ui.head, 4);
        // The engine restarted the receiver (variant change).
        ui.on_columns(0, ROWS as u8, &vec![128; ROWS], &view, &ctx);
        assert_eq!(ui.head, 1, "a rewound seq must start a fresh strip");
    }

    #[test]
    fn a_mismatched_row_count_is_rejected() {
        let mut ui = HellUi::default();
        let view = HellView::default();
        let ctx = egui::Context::default();
        ui.on_columns(0, 7, &vec![255; 7], &view, &ctx);
        assert_eq!(ui.head, 0, "a batch of the wrong geometry must not touch the ring");
    }

    #[test]
    fn writes_wrap_the_ring_without_overflowing() {
        let mut ui = HellUi::default();
        let view = HellView::default();
        let ctx = egui::Context::default();
        // Land the head near the end, then write across the seam.
        ui.head = RING_COLS as u64 - 3;
        ui.expect = ui.head;
        ui.on_columns(ui.expect, ROWS as u8, &vec![200; 6 * ROWS], &view, &ctx);
        assert_eq!(ui.head, RING_COLS as u64 + 3);
        assert!(ui.gray[(RING_COLS - 1) * ROWS] == 200, "pre-seam column");
        assert!(ui.gray[0] == 200, "post-seam column");
    }

    /// Row 0 of a column is the first dot transmitted, and Hell scans a column
    /// upwards, so it belongs at the bottom of the picture. Drawn at the top it
    /// put the received strip and the transmit echo upside down (#444).
    #[test]
    fn the_first_transmitted_dot_is_drawn_at_the_bottom() {
        let mut gray = vec![0u8; RING_COLS * ROWS];
        // Only the first dot of column 0 is lit.
        gray[0] = 255;
        let view = HellView::default();
        let img = HellUi::patch(&gray, 0, 1, &view);
        // n = 1, so pixel index == row index. Row 0 is the top of the image.
        let bottom = img.pixels[ROWS - 1];
        let top = img.pixels[0];
        assert!(
            bottom.r() < top.r(),
            "the first dot must be drawn on the bottom row (bottom {:?}, top {:?})",
            bottom,
            top
        );
    }

    #[test]
    fn reverse_video_inverts_the_ink() {
        let mut view = HellView::default();
        view.contrast = 1.0;
        let dark = shade(255, &view);
        view.reverse = true;
        let light = shade(255, &view);
        assert!(dark.r() < light.r(), "reverse video should flip ink and paper");
    }
}
