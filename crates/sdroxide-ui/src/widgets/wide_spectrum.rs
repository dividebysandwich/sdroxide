//! The full-band strip: everything a direct-sampling receiver can see at once,
//! as a scrolling waterfall.
//!
//! Deliberately *not* a second [`crate::widgets::spectrum_view`]. That widget is
//! two thousand lines of panadapter — filter-edge grips, sub-receiver dragging,
//! fling, zoom, viewport negotiation with the engine — all of it pinned to the
//! device passband through `ViewState::clamp_to`. None of that applies here: the
//! span is fixed at whatever the front end can see, there is nothing to zoom
//! into, and the two interactions that matter are "take me there" and "how wide
//! is that" — a click tunes, and shift+drag measures a span with the
//! panadapter's own ruler.
//!
//! Overloading `state.sample_rate` to describe this wider window instead would
//! have avoided a new widget and broken two things that read it as the real IQ
//! span: `keep_vfo_in_span`, which would then never retune, and the sub-receiver
//! clamp, which would let the sub be placed somewhere the DDC cannot reach.
//!
//! # Why this waterfall is on the CPU
//!
//! [`crate::waterfall_gpu`] keeps its state in egui's *type-keyed*
//! `CallbackResources`, so a second instance needs a newtype and another pair of
//! 2048×2048 textures — 8 MB — to show a strip a few dozen rows tall. This one
//! rebuilds a 1024×120 image twenty times a second, which is not worth a second
//! GPU pipeline.

use std::collections::VecDeque;

use eframe::egui::{
    self, Color32, ColorImage, CursorIcon, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    Vec2,
};
use sdroxide_types::{Command, RadioState, SpectrumFrame};

use crate::{colormap, widgets::spectrum_view};

/// Height of the strip, in points.
pub const STRIP_HEIGHT: f32 = 96.0;

/// The colour of the receiving slice and the tuned frequency: magenta, which
/// nine of the eleven waterfall palettes never draw (Classic among them). Neon
/// and Synthwave do, and there the triangles on the edges still carry the mark.
///
/// Used by: the slice outline and the tuned-frequency line in [`show`], and
/// the fill of the edge triangles ([`edge_triangle`]).
// Why: HDZ 2026-11-04 (KiwiSDR, whole 0-30 MHz strip): the white slice outline
//      and the amber tuned line were lost in the waterfall; "für den Marker
//      zumindest eine andere Farbe nehmen, die nicht im Wasserfall drin ist …
//      ein schreiendes Lila". Checked against `colormap::lut`: only Neon
//      (210, 0, 190) and Synthwave (150, 12, 130 to 240, 40, 110) come near it.
// Invariant: one colour for slice and tuned line, so the two read as one mark.
const MARK: Color32 = Color32::from_rgb(255, 0, 255);

/// Height of the triangles that point at the tuned frequency from the strip's
/// top and bottom edge, in points, before the panadapter font scale; as wide
/// as they are tall (see [`edge_triangle`]).
// Why: HDZ 2026-11-04: "mit kleinen Dreiecken an den Rändern verdeutlichen".
//      7 pt is found at a glance on a 96 pt strip and covers less than a tenth
//      of its height at each edge.
const MARK_TRIANGLE: f32 = 7.0;

/// Rows of history kept. At ~20 frames a second this is about six seconds —
/// long enough to see a band opening without the strip turning into a second
/// waterfall competing with the main one.
pub(crate) const ROWS: usize = 120;

/// A click tunes to a multiple of this. 100 Hz is finer than a pixel of a
/// 32 MHz strip, so it never fights the hover readout showing where it lands.
const CLICK_STEP: f64 = 100.0;

/// Width the rows are stored at. Independent of the widget's pixel width, so a
/// window resize does not throw the history away.
const COLS: usize = 1024;

/// Scrolling waterfall state for the full-band strip.
///
/// Lives in the app rather than the widget because it has to survive between
/// frames — and because it must be droppable when the source changes, or the
/// history goes on showing a band nothing is receiving any more.
#[derive(Default)]
pub struct WideWaterfall {
    /// Newest row last, each `COLS` bytes of 0..=255 magnitude.
    rows: VecDeque<Vec<u8>>,
    tex: Option<TextureHandle>,
    last_seq: u32,
    palette: usize,
    /// The dB window the newest row was built with, for the scale readout.
    levels: (f32, f32),
    /// The centre and span the history was drawn against — see
    /// [`WideWaterfall::push`].
    window: Option<(f64, f64)>,
    /// The span the main panadapter is showing, low and high edge in Hz, as
    /// last drawn: what the strip's outline frames. Set by the app after each
    /// strip, so the outline runs at most one frame behind a zoom or a pan.
    pub view_span: Option<(f64, f64)>,
}

impl WideWaterfall {
    /// Fold a new frame in, if it is one we have not seen.
    pub fn push(&mut self, frame: &SpectrumFrame, palette: usize) {
        if frame.seq == self.last_seq && !self.rows.is_empty() {
            return;
        }
        // Every row is drawn against the *newest* frame's axis, so history from
        // before a centre change is history at the wrong frequency — but it is
        // not history of the wrong *band*. A centre that moves is the window
        // sliding along, and the rows are still good a few columns over, so
        // they are scrolled rather than discarded.
        //
        // This lane's first backend, the RX-888, has a centre that cannot move
        // — its full band is the whole ADC — so nothing needed this until a
        // front end whose window slides arrived. An Icom's scope follows the
        // dial, and clearing on every move meant each small tuning nudge blanked
        // six seconds of waterfall: at a 200 kHz span one column is under
        // 200 Hz, so tuning across a single signal wiped the strip repeatedly.
        //
        // Whole columns only, with the remainder carried in `window`: three
        // nudges of a third of a column each have to add up to one shift rather
        // than to none at all. What is left is under half a column of skew
        // against the newest frame's axis, which is the same sub-pixel error the
        // old threshold tolerated.
        match self.window {
            Some((c, s)) if (frame.span_hz - s).abs() <= 1.0 && frame.span_hz > 0.0 => {
                let bin_hz = frame.span_hz / COLS as f64;
                let shift = ((frame.center_hz - c) / bin_hz).round();
                if shift.abs() >= COLS as f64 {
                    // Moved clean off its own width: nothing would survive the
                    // scroll, so take the cheap path.
                    self.rows.clear();
                    self.window = None;
                } else if shift != 0.0 {
                    self.scroll(shift as isize);
                    self.window = Some((c + shift * bin_hz, s));
                }
            }
            // A different span is a different scale, and no amount of sliding
            // maps one onto the other.
            Some(_) => {
                self.rows.clear();
                self.window = None;
            }
            None => {}
        }
        if self.window.is_none() {
            self.window = Some((frame.center_hz, frame.span_hz));
        }
        self.last_seq = frame.seq;
        self.levels = (frame.db_floor, frame.db_ceil);
        self.palette = palette;

        // Resample to the history width, keeping the strongest bin in each
        // column: a carrier occupying one bin of two thousand has to survive
        // being squeezed into one pixel.
        let n = frame.bins.len().max(1);
        let mut row = vec![0u8; COLS];
        for (c, slot) in row.iter_mut().enumerate() {
            let b0 = c * n / COLS;
            let b1 = (((c + 1) * n / COLS).max(b0 + 1)).min(n);
            *slot = frame.bins.get(b0..b1).and_then(|s| s.iter().copied().max()).unwrap_or(0);
        }
        self.rows.push_back(row);
        while self.rows.len() > ROWS {
            self.rows.pop_front();
        }
        // Rebuilt on the next draw.
        self.tex = None;
    }

    /// Slide every stored row sideways: left for a positive `by` (the window
    /// moved up the band, so what was at a column now belongs to a lower one),
    /// right for a negative one.
    ///
    /// The vacated edge is filled with the bottom of the scale rather than
    /// wrapped: that part of the band genuinely has no history yet, and a floor
    /// reads as "nothing recorded here" where wrapped rows would read as
    /// signals that were never there.
    fn scroll(&mut self, by: isize) {
        let k = by.unsigned_abs();
        if k == 0 || k >= COLS {
            return;
        }
        for row in &mut self.rows {
            if by > 0 {
                row.copy_within(k.., 0);
                row[COLS - k..].fill(0);
            } else {
                row.copy_within(..COLS - k, k);
                row[..k].fill(0);
            }
        }
    }

    /// Drop the history — used when the radio source changes.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.tex = None;
        self.last_seq = 0;
        self.window = None;
    }

    fn texture(&mut self, ctx: &egui::Context) -> Option<&TextureHandle> {
        if self.tex.is_none() && !self.rows.is_empty() {
            let lut = colormap::lut(self.palette);
            // Newest row first, so the strip scrolls downward like the main
            // waterfall rather than against it. Built as one contiguous vec —
            // `ColorImage` wants pixels row-major top to bottom anyway, and
            // pushing into it beats indexing a pre-filled image.
            let mut pixels = Vec::with_capacity(COLS * self.rows.len());
            for row in self.rows.iter().rev() {
                pixels.extend(row.iter().map(|v| {
                    let i = *v as usize * 4;
                    Color32::from_rgb(lut[i], lut[i + 1], lut[i + 2])
                }));
            }
            let img = ColorImage::new([COLS, self.rows.len()], pixels);
            self.tex = Some(ctx.load_texture("wide-waterfall", img, egui::TextureOptions::LINEAR));
        }
        self.tex.as_ref()
    }
}

/// Draw the full-band strip and handle its input: a click tunes, hovering reads
/// out the frequency under the cursor, and shift+drag measures a bandwidth —
/// the last two exactly as on the main waterfall.
pub fn show(
    ui: &mut egui::Ui,
    wf: &mut WideWaterfall,
    frame: &SpectrumFrame,
    state: &RadioState,
    palette: usize,
    lines: Option<GuideLines>,
    cmds: &mut Vec<Command>,
) {
    wf.push(frame, palette);

    // Drags are sensed for one gesture only — the shift+drag bandwidth ruler.
    // Nothing else here drags: the span is fixed, so there is no pan or zoom to
    // arbitrate against.
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), STRIP_HEIGHT),
        Sense::click_and_drag(),
    );
    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter_at(rect);
    let lo = frame.center_hz - frame.span_hz / 2.0;
    let hi = frame.center_hz + frame.span_hz / 2.0;
    let x_of = |hz: f64| -> f32 {
        rect.left() + ((hz - lo) / (hi - lo)).clamp(0.0, 1.0) as f32 * rect.width()
    };

    // Shift+drag measures a span here too. The strip is where a signal wider
    // than the panadapter's view is seen whole, so it is where its width gets
    // asked about — and the ruler drawn is the panadapter's own, so the two read
    // identically. The bookkeeping is local and short: unlike over there, no
    // filter grip, pan or fling is competing for the same drag.
    let shift = ui.input(|i| i.modifiers.shift);
    let measure_id = ui.id().with("wide-bw-measure");
    // Frequency the drag started at; `None` when not measuring.
    let mut measuring: Option<f64> = ui.data(|d| d.get_temp(measure_id)).unwrap_or(None);
    let fade_id = ui.id().with("wide-bw-fade");
    // After releasing, the frozen span fades out: (start_hz, end_hz, release_time).
    let mut fade: Option<(f64, f64, f64)> = ui.data(|d| d.get_temp(fade_id)).unwrap_or(None);

    if resp.drag_started_by(egui::PointerButton::Primary) {
        // From the PRESS position: by the time the drag threshold trips the
        // pointer has already left where the measurement should start.
        if shift {
            measuring = ui.input(|i| i.pointer.press_origin()).map(|p| hz_at(&rect, lo, hi, p.x));
            fade = None; // a new measurement cancels the previous one's fade
        } else {
            measuring = None;
        }
        ui.data_mut(|d| {
            d.insert_temp(measure_id, measuring);
            d.insert_temp(fade_id, fade);
        });
    }
    if resp.drag_stopped() {
        if let Some(start_hz) = measuring {
            let end_hz =
                resp.interact_pointer_pos().map(|p| hz_at(&rect, lo, hi, p.x)).unwrap_or(start_hz);
            fade = Some((start_hz, end_hz, ui.input(|i| i.time)));
        }
        measuring = None;
        ui.data_mut(|d| {
            d.insert_temp(measure_id, measuring);
            d.insert_temp(fade_id, fade);
        });
    }
    if measuring.is_some() || (resp.hovered() && shift) {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }

    painter.rect_filled(rect, 2.0, Color32::BLACK);
    let ctx = ui.ctx().clone();
    if let Some(tex) = wf.texture(&ctx) {
        painter.image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        // The band plan's allocation row along the bottom, under everything
        // drawn over the waterfall after it.
        crate::widgets::bandplan::wide_row(&painter, &rect, &x_of);
    }

    // S/N and SK over the whole band, as on the panadapter: the kurtosis
    // shows whoever comes and goes — a voice, a key, a burst — where the
    // waterfall already shows the carriers.
    // Kept under its own name: `lines` below is the scale's.
    let guide_lines = lines;
    if let Some(l) = &guide_lines {
        draw_guide_lines(&painter, &rect, l, &x_of);
    }

    // What the panadapter below is showing, outlined rather than filled so it
    // does not tint the waterfall underneath it. Not the I/Q window behind
    // it: that jumps with every retune and says nothing about the picture.
    // Why: HDZ 2026-11-10: "mich interessiert am Kasten nicht, wie viel im
    //      Puffer ist, sondern der Kasten sollte mir zeigen, was ich sehe".
    if let Some((sx0, sx1)) = view_box_x(wf.view_span, &x_of) {
        painter.rect_stroke(
            Rect::from_min_max(Pos2::new(sx0, rect.top()), Pos2::new(sx1, rect.bottom())),
            0.0,
            // In the mark's colour, like the tuned line: see [`MARK`].
            Stroke::new(1.0, MARK),
            StrokeKind::Inside,
        );
    }

    // Frequency scale, ticked and labelled along the top so it stays clear of the
    // newest rows. The step is the panadapter's own, so a strip and the
    // panadapter under it never divide the same band differently — and unlike
    // the fixed megahertz grid this replaces, it still labels a front end whose
    // whole window is narrower than 1 MHz, such as a rig's own band scope.
    let (lo_text, hi_text, lines) = scale_labels(lo, hi);
    let font = egui::FontId::monospace(9.0 * crate::theme::panadapter_font_scale());
    let tick = Stroke::new(1.0, Color32::from_white_alpha(90));

    // The band's own limits. On a strip whose span the hardware fixes these are
    // what says which band this is, and a window that starts on no round number
    // gets no gridline near its edges at all.
    let lo_box = backed_label(
        &painter,
        Pos2::new(rect.left() + 3.0, rect.top() + 1.0),
        egui::Align2::LEFT_TOP,
        &lo_text,
        &font,
        Color32::from_white_alpha(235),
    );
    let hi_box = backed_label(
        &painter,
        Pos2::new(rect.right() - 3.0, rect.top() + 1.0),
        egui::Align2::RIGHT_TOP,
        &hi_text,
        &font,
        Color32::from_white_alpha(235),
    );

    // Minor ticks between the labelled lines, at a round fraction of the step.
    let minor = spectrum_view::minor_grid_step(spectrum_view::freq_grid_step(lo, hi));
    let mut hz = (lo / minor).ceil() * minor;
    while hz <= hi {
        let x = x_of(hz);
        painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.top() + 3.0)], tick);
        hz += minor;
    }

    for (hz, text) in lines {
        let x = x_of(hz);
        painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.top() + 7.0)], tick);
        // A label that would run into either edge label is dropped rather than
        // drawn over it: the limits outrank a gridline that is one tick away
        // from being spelled out already.
        let pos = Pos2::new(x + 2.0, rect.top() + 1.0);
        let w = painter.layout_no_wrap(text.clone(), font.clone(), Color32::WHITE).size().x;
        if pos.x - 4.0 > lo_box.right() && pos.x + w + 4.0 < hi_box.left() {
            backed_label(
                &painter,
                pos,
                egui::Align2::LEFT_TOP,
                &text,
                &font,
                Color32::from_white_alpha(170),
            );
        }
    }

    // The tuned frequency: a line, and a triangle on each edge pointing at it.
    //
    // On a strip of the whole band the slice above is narrower than a pixel and
    // a one-pixel line in a colour the palettes also use (white, amber) is lost
    // in the waterfall under it, so the place is marked in magenta (see
    // [`MARK`]) and by a black-rimmed triangle on the top and the bottom edge,
    // which the eye finds where a line one pixel wide is gone.
    let vfo_x = x_of(state.active_freq_hz());
    painter.line_segment(
        [Pos2::new(vfo_x, rect.top()), Pos2::new(vfo_x, rect.bottom())],
        Stroke::new(1.0, MARK),
    );
    // Grown with the panadapter font, like every other label on the strip.
    let size = MARK_TRIANGLE * crate::theme::panadapter_font_scale();
    // Pitfall: the tips point *into* the strip. A triangle hung outside its
    //          edge would sit on the panel above or below and be clipped by it.
    // Why: the black rim keeps the triangle readable on Neon and Synthwave,
    //      whose palettes reach magenta.
    for (edge, inward) in [(rect.top(), 1.0), (rect.bottom(), -1.0)] {
        painter.add(egui::Shape::convex_polygon(
            edge_triangle(vfo_x, edge, inward, size).to_vec(),
            MARK,
            Stroke::new(1.0, Color32::BLACK),
        ));
    }

    // The auto-ranged window, so the scale is not a mystery.
    painter.text(
        Pos2::new(rect.right() - 3.0, rect.bottom() - 1.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{:.0} … {:.0} dBFS", wf.levels.0, wf.levels.1),
        egui::FontId::proportional(9.0 * crate::theme::panadapter_font_scale()),
        Color32::from_white_alpha(120),
    );

    // Straight to `SetVfo`: the engine's own `keep_vfo_in_span` notices the new
    // frequency is outside the received slice and retunes the front end. On this
    // hardware that costs nothing — retuning is a change of FFT bin, not an LO
    // move — so a click anywhere in 32 MHz lands immediately.
    //
    // A shift-click is a bandwidth measurement whose drag never left the press
    // point, not a request to tune somewhere.
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.clicked() && !shift) {
        let hz = hz_at(&rect, lo, hi, pos.x);
        cmds.push(Command::SetVfo { vfo: state.active_vfo, hz: tuned(hz) });
    }

    // --- cursor readout (hover) -------------------------------------------
    // The same faint crosshair and frequency box the main waterfall draws, so
    // the cursor reads a band the same way whichever strip it is over. Dropped
    // while the ruler is out: that carries its own labels, and a third box at
    // the pointer would sit on top of them.
    if let Some(p) = resp.hover_pos().filter(|_| measuring.is_none() && !resp.dragged()) {
        let line = Color32::from_rgba_unmultiplied(185, 205, 225, 70);
        painter.vline(p.x, rect.y_range(), Stroke::new(1.0, line));
        let hz = hz_at(&rect, lo, hi, p.x);
        let mut boxes = vec![(format!("{:.5} MHz", tuned(hz) / 1e6), Color32::WHITE)];
        // With the S/N or SK line on, their values under the pointer, from the
        // strip's own means, in the lines' colours — as the panadapter does.
        // Why: HDZ 2026-11-10: "im Panorama dann auch wie unten im Spektrum
        //      S/N und SK am Cursor mit anzeigen".
        if let Some(l) = &guide_lines {
            use crate::widgets::spectrum_view::{SK_GUIDE_COLOUR, SNR_GUIDE_COLOUR};
            if l.snr
                && let Some(snr) = l.guide.snr_at(hz)
            {
                boxes.push((format!("S/N {snr:.1} dB"), SNR_GUIDE_COLOUR));
            }
            if l.sk
                && let Some(k) = l.guide.sk_at(hz)
            {
                boxes.push((format!("SK {:.1} dB", 10.0 * k.max(1e-3).log10()), SK_GUIDE_COLOUR));
            }
        }
        // As one block: the strip is low, and boxes pushed to its edge one by
        // one would land on top of each other.
        let step = 11.0 * crate::theme::panadapter_font_scale() + 6.0;
        let top = (p.y - 9.0).min(rect.bottom() - step * boxes.len() as f32).max(rect.top());
        for (i, (text, colour)) in boxes.iter().enumerate() {
            spectrum_view::label_box(
                &painter,
                Pos2::new(p.x + 8.0, top + step * i as f32),
                text,
                *colour,
                rect,
            );
        }
    }

    // --- bandwidth measurement (shift+drag) + fade-out --------------------
    if let (Some(start_hz), Some(p)) = (measuring, resp.interact_pointer_pos()) {
        let end_hz = hz_at(&rect, lo, hi, p.x);
        spectrum_view::draw_bw_measure(&painter, x_of, &rect, start_hz, end_hz, 1.0);
    } else if let Some((start_hz, end_hz, t0)) = fade {
        let elapsed = ui.input(|i| i.time) - t0;
        if elapsed < spectrum_view::BW_FADE_SECS {
            let alpha = (1.0 - elapsed / spectrum_view::BW_FADE_SECS) as f32;
            spectrum_view::draw_bw_measure(&painter, x_of, &rect, start_hz, end_hz, alpha);
            crate::repaint::animate(ui.ctx()); // keep animating the fade to completion
        } else {
            ui.data_mut(|d| d.insert_temp(fade_id, None::<(f64, f64, f64)>));
        }
    }
}

/// The scale's text: the two edge labels — the only ones carrying the unit, so
/// the rest can stay bare numbers — and a `(frequency, label)` pair per
/// gridline, each with just enough decimals to tell its neighbours apart.
fn scale_labels(lo: f64, hi: f64) -> (String, String, Vec<(f64, String)>) {
    let step = spectrum_view::freq_grid_step(lo, hi);
    let dec = mhz_decimals(step);
    // The edges are not on the grid, so they need the resolution of the band
    // rather than of the step: kilohertz at least, more when the step is finer.
    let edge_dec = dec.max(3);
    let lines = spectrum_view::freq_gridlines(lo, hi)
        .into_iter()
        .map(|hz| (hz, format!("{:.*}", dec, hz / 1e6)))
        .collect();
    (format!("{:.*} MHz", edge_dec, lo / 1e6), format!("{:.*} MHz", edge_dec, hi / 1e6), lines)
}

/// Decimals needed to write a step of `step_hz` in MHz without losing it.
fn mhz_decimals(step_hz: f64) -> usize {
    let mhz = step_hz / 1e6;
    if mhz <= 0.0 || !mhz.is_finite() {
        return 3;
    }
    (-mhz.log10().floor()).clamp(0.0, 6.0) as usize
}

/// Draw a scale label with a dark backing and return the box it took. Without
/// the backing a label over a strong carrier — the brightest thing on the strip
/// — is the one place the scale cannot be read.
fn backed_label(
    p: &egui::Painter,
    pos: Pos2,
    align: egui::Align2,
    text: &str,
    font: &egui::FontId,
    color: Color32,
) -> Rect {
    let galley = p.layout_no_wrap(text.to_string(), font.clone(), color);
    let bg = align.anchor_size(pos, galley.size()).expand2(Vec2::new(2.0, 0.5));
    p.rect_filled(bg, 2.0, Color32::from_black_alpha(120));
    p.galley(align.anchor_size(pos, galley.size()).min, galley, color);
    bg
}

/// The triangle that points at `x` from an edge of the strip.
///
/// In: `x` and `y` in screen points, `y` on the edge; `inward` is +1 from the
/// top edge and -1 from the bottom one; `size` in points.
/// Out: the three corners — the base on the edge, centred on `x` and `size`
/// wide, then the tip `size` points inside the strip at `x`.
/// Fails: none; a zero `size` gives a degenerate triangle that draws nothing.
// Why: a pure function, so where the mark lands is tested without a painter
//      (`the_triangles_point_into_the_strip_from_both_edges`).
fn edge_triangle(x: f32, y: f32, inward: f32, size: f32) -> [Pos2; 3] {
    let half = size / 2.0;
    [Pos2::new(x - half, y), Pos2::new(x + half, y), Pos2::new(x, y + inward * size)]
}

/// The frequency at screen `x`, clamped to the band the strip is showing —
/// a measuring drag is free to leave the strip, the front end is not.
fn hz_at(rect: &Rect, lo: f64, hi: f64, x: f32) -> f64 {
    let frac = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
    lo + frac * (hi - lo)
}

/// Where a click lands: the dial rounds to [`CLICK_STEP`], and the hover readout
/// goes through the same rounding so it promises exactly what the click will do.
fn tuned(hz: f64) -> f64 {
    (hz / CLICK_STEP).round() * CLICK_STEP
}

/// Narrowest outline worth drawing, pt: below it the box is a line that hides
/// the tuned mark beside it (a KiwiSDR's 12 kHz on a 30 MHz strip is under one).
const VIEW_BOX_MIN_W: f32 = 3.0;

/// Where the panadapter's view sits on the strip, as left and right x; `None`
/// before there is a view, or where it is narrower than [`VIEW_BOX_MIN_W`].
fn view_box_x(view: Option<(f64, f64)>, x_of: &dyn Fn(f64) -> f32) -> Option<(f32, f32)> {
    let (lo, hi) = view?;
    let (x0, x1) = (x_of(lo), x_of(hi));
    (x1 - x0 >= VIEW_BOX_MIN_W).then_some((x0, x1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proves: the outline frames the span the panadapter shows, moving with
    /// it as a pan or a zoom does — not the I/Q window, which a retune moves
    /// in jumps — and stands aside where it would be narrower than a line.
    #[test]
    fn the_outline_frames_what_the_panadapter_shows() {
        // 1000 pt over 0–30 MHz: 30 kHz a point.
        let x_of = |hz: f64| -> f32 { (hz / 30_000.0).clamp(0.0, 1000.0) as f32 };
        let at = |lo: f64, hi: f64| view_box_x(Some((lo, hi)), &x_of);
        assert_eq!(at(14.0e6, 14.3e6), Some((14.0e6 as f32 / 30_000.0, 14.3e6 as f32 / 30_000.0)));
        // A pan of 150 kHz moves it 5 pt, smoothly, both edges alike.
        let (a, b) = at(14.0e6, 14.3e6).unwrap();
        let (c, d) = at(14.15e6, 14.45e6).unwrap();
        assert!(((c - a) - 5.0).abs() < 1e-3 && ((d - b) - 5.0).abs() < 1e-3);
        // A KiwiSDR zoomed in on its 12 kHz: 0.4 pt, not drawn.
        assert_eq!(at(14.0e6, 14.012e6), None);
        // No view yet: nothing.
        assert_eq!(view_box_x(None, &x_of), None);
    }

    fn frame(seq: u32, bins: Vec<u8>) -> SpectrumFrame {
        SpectrumFrame {
            seq,
            center_hz: 16.2e6,
            span_hz: 32.4e6,
            db_floor: -120.0,
            db_ceil: -20.0,
            bins,
            rows: Vec::new(),
            rows_clocked: false,
        }
    }

    /// The axis mapping is the part that has to be right: a frame claiming
    /// 0–32.4 MHz must put 0 at the left edge and Nyquist at the right.
    #[test]
    fn the_axis_spans_dc_to_nyquist() {
        let f = frame(1, vec![0; 2048]);
        let lo = f.center_hz - f.span_hz / 2.0;
        let hi = f.center_hz + f.span_hz / 2.0;
        assert!(lo.abs() < 1.0, "left edge should be DC, got {lo}");
        assert!((hi - 32.4e6).abs() < 1.0, "right edge should be Nyquist, got {hi}");
    }

    /// A strip 100 pt wide starting 10 pt in, so a mapping that forgets
    /// `rect.left()` fails instead of passing by symmetry.
    fn strip() -> Rect {
        Rect::from_min_size(Pos2::new(10.0, 0.0), Vec2::new(100.0, STRIP_HEIGHT))
    }

    #[test]
    fn a_click_maps_back_to_the_frequency_under_it() {
        // A click a quarter of the way across a 32.4 MHz strip is 8.1 MHz...
        let hz = hz_at(&strip(), 0.0, 32.4e6, 35.0);
        assert!((hz - 8.1e6).abs() < 1.0, "got {hz}");
        // ...and the dial it sets is within half a step of it, which is what
        // lets the hover readout show the click's own frequency.
        assert!((tuned(hz) - hz).abs() <= CLICK_STEP / 2.0);
        assert_eq!(tuned(14_074_063.0), 14_074_100.0);
    }

    /// The strip's whole point is showing a band at once, so its two limits are
    /// labelled outright — they are what names the band, and a window that
    /// starts on no round number has no gridline near its edges to imply them.
    #[test]
    fn the_scale_labels_the_bands_own_limits() {
        let (lo_text, hi_text, lines) = scale_labels(0.0, 32.4e6);
        assert_eq!(lo_text, "0.000 MHz");
        assert_eq!(hi_text, "32.400 MHz");
        // ...and every 5 MHz, as bare numbers under that unit. The line on the
        // low edge keeps its tick, but the drawing drops its label in favour of
        // the edge's own — nobody needs "0" and "0.000 MHz" side by side.
        assert_eq!(
            lines.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(),
            ["0", "5", "10", "15", "20", "25", "30"]
        );
    }

    /// The grid this replaced was a fixed 1 MHz, which left a front end whose
    /// whole window is narrower than that — a rig's own band scope, an Icom's
    /// 100 kHz sweep — with a scale carrying no label at all.
    #[test]
    fn a_window_narrower_than_a_megahertz_is_still_labelled() {
        let (lo_text, hi_text, lines) = scale_labels(14.05e6, 14.15e6);
        assert_eq!((lo_text.as_str(), hi_text.as_str()), ("14.050 MHz", "14.150 MHz"));
        assert!(lines.len() >= 4, "a 100 kHz window got {} labels", lines.len());
        // Two decimals: enough to tell 20 kHz apart, and no more.
        assert_eq!(lines.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>()[0], "14.06");
    }

    /// Minor ticks have to land on round frequencies for every step the grid
    /// picks, or they read as noise between the labels.
    #[test]
    fn minor_ticks_land_on_round_frequencies() {
        for (step, want) in [(1e6, 200e3), (2e6, 1e6), (5e6, 1e6), (20e3, 10e3), (50e3, 10e3)] {
            let got = spectrum_view::minor_grid_step(step);
            assert!((got - want).abs() < 1.0, "step {step} gave a {got} Hz tick, wanted {want}");
        }
    }

    /// A measuring drag is free to leave the strip; the band it measures is not.
    /// Without the clamp the ruler would report a span the front end cannot see
    /// — and, on the click path, tune outside it.
    #[test]
    fn a_position_outside_the_strip_clamps_to_the_band() {
        let r = strip();
        assert_eq!(hz_at(&r, 0.0, 32.4e6, r.left() - 40.0), 0.0);
        assert_eq!(hz_at(&r, 0.0, 32.4e6, r.right() + 40.0), 32.4e6);
    }

    #[test]
    fn history_scrolls_and_is_bounded() {
        let mut wf = WideWaterfall::default();
        for seq in 1..(ROWS as u32 + 40) {
            wf.push(&frame(seq, vec![0; 2048]), 0);
        }
        assert_eq!(wf.rows.len(), ROWS, "history grew past its bound");
    }

    #[test]
    fn the_same_frame_is_not_added_twice() {
        let mut wf = WideWaterfall::default();
        wf.push(&frame(7, vec![0; 2048]), 0);
        wf.push(&frame(7, vec![0; 2048]), 0);
        assert_eq!(wf.rows.len(), 1, "a repeated seq added a second row");
    }

    /// The whole point of max-pooling: a carrier one bin wide out of two
    /// thousand must still be visible after resampling to the history width.
    #[test]
    fn a_narrow_carrier_survives_resampling() {
        let mut bins = vec![0u8; 2048];
        bins[1000] = 255;
        let mut wf = WideWaterfall::default();
        wf.push(&frame(1, bins), 0);
        let row = wf.rows.back().unwrap();
        assert_eq!(row.iter().copied().max(), Some(255), "the carrier vanished");
        assert_eq!(row[1000 * COLS / 2048], 255, "the carrier moved column");
    }

    #[test]
    fn clearing_drops_the_history() {
        let mut wf = WideWaterfall::default();
        wf.push(&frame(1, vec![9; 2048]), 0);
        wf.clear();
        assert!(wf.rows.is_empty());
        // ...and a frame with the same seq is then accepted again, rather than
        // being mistaken for one already shown.
        wf.push(&frame(1, vec![9; 2048]), 0);
        assert_eq!(wf.rows.len(), 1);
    }

    /// A window that slides is still looking at the same band a few columns
    /// over, so the history is scrolled onto the new axis rather than thrown
    /// away. A front end whose band view moves — a SpyServer's FFT, an Icom's
    /// scope following the dial — used to blank six seconds of strip on every
    /// tuning nudge: at a 200 kHz span one column is under 200 Hz, so tuning
    /// across a single signal wiped it again and again.
    #[test]
    fn moving_the_window_scrolls_the_history() {
        let mut wf = WideWaterfall::default();
        // One hot column, so where it ends up says how far the rows moved.
        let mut bins = vec![0u8; COLS];
        bins[COLS / 2] = 200;
        wf.push(&frame(1, bins), 0);

        // Re-centre upwards by a hundred columns' worth: the same frequency now
        // sits a hundred columns further left, and the marker has to be there.
        const STEP: usize = 100;
        let mut moved = frame(2, vec![0; COLS]);
        moved.center_hz += moved.span_hz / COLS as f64 * STEP as f64;
        wf.push(&moved, 0);
        assert_eq!(wf.rows.len(), 2, "a slide is not a new band");
        assert_eq!(wf.rows[0][COLS / 2 - STEP], 200, "the history did not follow the axis");
        assert_eq!(wf.rows[0][COLS / 2], 0, "the marker was left where it was");
        assert_eq!(wf.rows[0][COLS - 1], 0, "the vacated edge is not history");
    }

    /// Two things no amount of sliding can fix: a jump further than the strip is
    /// wide (nothing of the old band is still on screen) and a span change (the
    /// axis is stretched, not shifted).
    #[test]
    fn a_jump_off_its_own_width_or_a_new_span_throws_the_history_away() {
        let mut wf = WideWaterfall::default();
        for seq in 1..=5 {
            wf.push(&frame(seq, vec![9; 64]), 0);
        }
        assert_eq!(wf.rows.len(), 5);

        let mut jumped = frame(6, vec![9; 64]);
        jumped.center_hz += jumped.span_hz * 2.0;
        wf.push(&jumped, 0);
        assert_eq!(wf.rows.len(), 1, "nothing of the old band is still in view");

        let mut zoomed = frame(7, vec![9; 64]);
        zoomed.center_hz = jumped.center_hz;
        zoomed.span_hz /= 2.0;
        wf.push(&zoomed, 0);
        assert_eq!(wf.rows.len(), 1);
    }

    /// Sub-column drift accumulates instead of being discarded: ten nudges of a
    /// tenth of a column each have to add up to one shift, or a slow tuning
    /// sweep would leave the history a whole column behind the axis it is drawn
    /// against and nothing would ever correct it.
    #[test]
    fn sub_column_drift_adds_up_to_a_shift() {
        let mut wf = WideWaterfall::default();
        let mut bins = vec![0u8; COLS];
        bins[COLS / 2] = 200;
        wf.push(&frame(1, bins), 0);

        let bin_hz = 32.4e6 / COLS as f64;
        let mut center = 16.2e6;
        for seq in 2..=11 {
            center += bin_hz / 10.0;
            let mut f = frame(seq, vec![0; COLS]);
            f.center_hz = center;
            wf.push(&f, 0);
        }
        assert_eq!(wf.rows[0][COLS / 2 - 1], 200, "ten tenths of a column shifted nothing");
    }

    /// ...but not for a wobble smaller than a pixel. Throwing the band away
    /// over a sub-column drift would be its own bug, and a re-centre that
    /// lands within rounding of where it was is exactly what a rate-limited
    /// servo produces.
    #[test]
    fn a_sub_pixel_drift_keeps_the_history() {
        let mut wf = WideWaterfall::default();
        for seq in 1..=4 {
            wf.push(&frame(seq, vec![9; 64]), 0);
        }
        let mut nudged = frame(5, vec![9; 64]);
        // One tenth of a column of a 32.4 MHz span across 1024 columns.
        nudged.center_hz += nudged.span_hz / COLS as f64 / 10.0;
        wf.push(&nudged, 0);
        assert_eq!(wf.rows.len(), 5, "a sub-column nudge is not a new band");
    }

    #[test]
    fn the_reported_window_follows_the_frame() {
        let mut wf = WideWaterfall::default();
        let mut f = frame(1, vec![0; 64]);
        f.db_floor = -101.0;
        f.db_ceil = -17.0;
        wf.push(&f, 0);
        assert_eq!(wf.levels, (-101.0, -17.0));
    }

    /// Proves: both edge triangles stand on their edge, centred on the tuned
    /// frequency, with the tip inside the strip under it ([`edge_triangle`]).
    #[test]
    fn the_triangles_point_into_the_strip_from_both_edges() {
        let (top, bottom) = (10.0, 106.0);
        let down = edge_triangle(50.0, top, 1.0, 7.0);
        let up = edge_triangle(50.0, bottom, -1.0, 7.0);
        // The base sits on the edge, centred on the tuned frequency...
        assert_eq!((down[0].y, down[1].y), (top, top));
        assert_eq!((down[0].x + down[1].x) / 2.0, 50.0);
        // ...and the tip is inside the strip, under the same frequency.
        assert_eq!(down[2], Pos2::new(50.0, top + 7.0));
        assert_eq!(up[2], Pos2::new(50.0, bottom - 7.0));
    }
}

/// The panadapter's S/N and SK lines, laid over the full-band strip.
pub struct GuideLines {
    pub guide: std::sync::Arc<crate::mean_spectrum::Guide>,
    pub snr: bool,
    pub sk: bool,
    pub snr_top_db: f32,
    pub sk_bottom_db: f32,
    pub sk_top_db: f32,
}

/// Each line in the lower half of the strip, on the panadapter's scales and
/// colours. No caption: the strip is busy enough, and the panadapter's 1D
/// spectrum names the scales (HDZ 2026-11-09: "die Zahlen brauchst du in das
/// Panorama nicht reinzuschreiben").
fn draw_guide_lines(p: &egui::Painter, rect: &Rect, l: &GuideLines, x_of: &dyn Fn(f64) -> f32) {
    use crate::widgets::spectrum_view::{SK_GUIDE_COLOUR, SNR_GUIDE_COLOUR};
    let g = &l.guide;
    let base = rect.bottom() - 1.0;
    let full = rect.height() * 0.5;
    let line = |vals: &mut dyn Iterator<Item = (usize, f32)>, colour: Color32| {
        let mut run: Vec<Pos2> = Vec::new();
        for (i, frac) in vals {
            if !frac.is_finite() {
                if run.len() >= 2 {
                    p.add(egui::Shape::line(std::mem::take(&mut run), Stroke::new(1.0, colour)));
                }
                run.clear();
                continue;
            }
            let hz = g.lo_hz + (i as f64 + 0.5) * g.cell_hz;
            run.push(Pos2::new(x_of(hz), base - frac.clamp(0.0, 1.0) * full));
        }
        if run.len() >= 2 {
            p.add(egui::Shape::line(run, Stroke::new(1.0, colour)));
        }
    };
    if l.snr {
        let top = l.snr_top_db.max(1.0);
        line(&mut g.snr_db.iter().map(|s| s / top).enumerate(), SNR_GUIDE_COLOUR);
    }
    if l.sk {
        let (lo, hi) = (l.sk_bottom_db, l.sk_top_db.max(l.sk_bottom_db + 1.0));
        let frac = |k: &f32| {
            if k.is_finite() { (10.0 * k.max(1e-3).log10() - lo) / (hi - lo) } else { f32::NAN }
        };
        line(&mut g.sk.iter().map(frac).enumerate(), SK_GUIDE_COLOUR);
    }
}
