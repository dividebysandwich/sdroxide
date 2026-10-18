//! Client-side state for the weather-fax panel: the chart being painted, and
//! the gallery of ones already saved.
//!
//! The picture is built here as well as in the engine. That is not duplication
//! for its own sake — a remote client sees only the line events, never the
//! engine's buffer, and the panel has to be able to paint a chart that is still
//! twelve minutes from finishing.

use eframe::egui;

/// Rows the live chart grows by. A chart is 1809 pixels wide, so this is about
/// half a megabyte at a time rather than a reallocation per scan line.
const GROW_ROWS: usize = 256;

/// Widest a chart may get before the panel refuses it, as a guard on a length
/// that arrives from the wire.
const MAX_W: usize = 4096;
/// Tallest, matching the demodulator's own limit with headroom.
const MAX_H: usize = 4096;

/// Charts the gallery will hold thumbnails for.
///
/// Not a limit on the collection — the engine counts the whole store and the
/// panel says how much more there is — but on what this client keeps as
/// textures. A thumbnail is a few tens of kilobytes where the chart behind it
/// is two megapixels, so this is several screens' worth of scrolling and still
/// a fraction of what the old whole-chart gallery cost.
pub const GALLERY_MAX: usize = 240;

/// Magnification limits for the live view. The lower end is enough to see a
/// whole chart's layout at a glance; the upper end is where a fax pixel is
/// four screen pixels, past which there is no more detail to find.
pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 4.0;

/// How far the picture may be stretched or squashed vertically. Wide enough to
/// straighten a chart received at the wrong line rate — 60 against 120 is a
/// factor of two, and taking the wrong one of a pair of adjacent rates is the
/// usual way to end up with a chart twice as tall as it should be.
pub const MIN_ASPECT: f32 = 0.25;
pub const MAX_ASPECT: f32 = 4.0;

/// How the live chart is scaled into the panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Zoom {
    /// Scale to the panel width, never past one screen pixel per fax pixel.
    FitWidth,
    /// Scale so the whole chart is in view at once, however tall it has grown.
    Whole,
    /// A fixed magnification, in screen pixels per fax pixel.
    Fixed(f32),
}

/// A saved chart in the gallery.
///
/// What is held is a thumbnail; the chart itself is two megapixels and lives in
/// the engine's store, fetched only when one is opened.
pub struct Chart {
    /// Thumbnail, as the engine rendered it out of the stored file.
    pub texture: egui::TextureHandle,
    /// The chart at full size, once fetched — or promoted straight from one
    /// that has just been received, which the panel already held.
    pub full: Option<egui::TextureHandle>,
    /// File name it was saved under, which carries its date and dial.
    pub name: String,
    /// When it was received and where from, read back out of that name. `None`
    /// for a file in the directory that is not one of ours.
    pub meta: Option<sdroxide_types::WefaxChartMeta>,
    /// Size of the chart itself, not of the thumbnail.
    pub size: (u16, u16),
    /// When it was received, as the engine ordered the store by.
    pub unix: i64,
}

impl Chart {
    /// Date, time and station, or the bare file name when the name is not one
    /// this program wrote.
    pub fn title(&self) -> String {
        self.meta.map_or_else(|| self.name.clone(), |m| m.label())
    }
}

pub struct WefaxUi {
    /// Engine status, as of the last update.
    pub status: sdroxide_types::WefaxStatus,
    /// The chart being received: one byte per pixel, top row first.
    live: Vec<u8>,
    live_w: u16,
    live_h: u16,
    /// Which picture `live` belongs to, so a restarted transmission clears it
    /// rather than appending to the previous chart.
    image_id: u32,
    /// The live chart as a texture, rebuilt when rows have arrived.
    live_tex: Option<egui::TextureHandle>,
    dirty: bool,
    /// How the live chart is scaled into the panel.
    pub zoom: Zoom,
    /// Extra vertical scaling of the live chart, on top of `zoom`.
    ///
    /// Its own control because the two things that make a chart the wrong shape
    /// are different problems: the *size* on screen is a display preference,
    /// while the *proportions* are a property of the signal — a chart received
    /// at the wrong line rate comes out stretched or squashed, and being able to
    /// pull it back into shape is what makes it readable before the operator has
    /// worked out which rate the station is actually using.
    pub aspect: f32,
    /// Whether the live view keeps the newest rows in sight. The operator
    /// scrolling up turns it off — reading the top of a chart while the bottom
    /// is still arriving is the whole point of being able to scroll — and
    /// scrolling back to the bottom turns it on again.
    pub follow: bool,
    /// Saved charts, newest first.
    pub gallery: Vec<Chart>,
    /// Whether the engine's store has been listed this session.
    pub listed: bool,
    /// How many charts the store holds altogether — exact, where the old
    /// client-side gallery could only guess from what it had read.
    pub total: u32,
    /// True while a listing page is outstanding, so a click cannot spray
    /// requests down the one reliable lane.
    pub page_pending: bool,
    /// Name of the chart whose full size has been asked for.
    ///
    /// Records the *ask*, and is cleared when the viewer moves on rather than
    /// when the answer arrives: a chart that fails to come back must be asked
    /// for once, not once a frame for as long as the window is open.
    pub full_asked: Option<String>,
    /// The answer to that ask was empty — the store no longer has it.
    pub full_gone: bool,
    /// The chart that has just finished arriving, held until the engine names
    /// the file it went into.
    fresh: Option<(egui::TextureHandle, Vec<u8>)>,
    /// Bytes of the chart last fetched or received, and its name — what "Save
    /// chart…" writes out. One at a time: a chart is megabytes, and holding
    /// every one for a button nobody may press would be a gallery in memory.
    pub full_png: Option<(String, Vec<u8>)>,
    /// Where charts are being saved, for the panel to show and to copy.
    pub dir: String,
    /// Which gallery entry is open full-size, if any.
    pub viewing: Option<usize>,
    /// Name of the chart whose DELETE has been pressed once and is waiting to
    /// be pressed again.
    ///
    /// Two presses rather than a modal, because the file goes for good and
    /// there is no undo to offer: a stray click on a chip beside SAVE would
    /// otherwise be the last anyone saw of a chart. Armed per name, so it
    /// cannot survive into the next chart and delete that one instead.
    pub confirm_delete: Option<String>,
}

impl Default for WefaxUi {
    fn default() -> Self {
        WefaxUi {
            status: Default::default(),
            live: Vec::new(),
            live_w: 0,
            live_h: 0,
            image_id: 0,
            live_tex: None,
            dirty: false,
            // Fitted, unstretched and following: what an operator wants before
            // they have decided to go looking at anything in particular.
            zoom: Zoom::FitWidth,
            aspect: 1.0,
            follow: true,
            gallery: Vec::new(),
            listed: false,
            total: 0,
            page_pending: false,
            full_asked: None,
            full_gone: false,
            fresh: None,
            full_png: None,
            dir: String::new(),
            viewing: None,
            confirm_delete: None,
        }
    }
}

impl WefaxUi {
    /// Rotate the chart already received, so the PHASE nudge corrects the whole
    /// picture rather than only the lines still to come (issue #439).
    pub fn shift_phase(&mut self, pixels: i32) {
        if self.live_w == 0 || self.live_h == 0 {
            return;
        }
        sdroxide_types::shift_fax_rows(
            &mut self.live,
            self.live_w as usize,
            self.live_h as usize,
            pixels,
        );
        self.dirty = true;
    }

    /// Adopt a freshly decoded scan line.
    pub fn push_line(&mut self, image_id: u32, y: u16, gray: &[u8]) {
        if gray.is_empty() || gray.len() > MAX_W {
            return;
        }
        // A new id, or a row before the write head, means a new transmission.
        if image_id != self.image_id || y == 0 {
            self.image_id = image_id;
            self.live.clear();
            self.live_w = gray.len() as u16;
            self.live_h = 0;
        }
        if gray.len() != self.live_w as usize || self.live_h as usize >= MAX_H {
            return;
        }
        // Rows arrive in order. A gap means lines were dropped somewhere
        // upstream; fill it with mid-grey rather than sliding the rest of the
        // chart up, which would put a seam through the picture instead of a
        // band and be far harder to see.
        while (self.live_h as usize) < y as usize {
            self.live.extend(std::iter::repeat_n(128u8, self.live_w as usize));
            self.live_h += 1;
        }
        if y as usize != self.live_h as usize {
            return;
        }
        if self.live.capacity() < self.live.len() + self.live_w as usize {
            self.live.reserve(self.live_w as usize * GROW_ROWS);
        }
        self.live.extend_from_slice(gray);
        self.live_h += 1;
        self.dirty = true;
    }

    /// Rows received for the chart in progress.
    pub fn live_size(&self) -> (u16, u16) {
        (self.live_w, self.live_h)
    }

    pub fn has_live(&self) -> bool {
        self.live_h > 0
    }

    /// Throw the live chart away — the operator restarting, or a completed one
    /// having moved to the gallery.
    pub fn clear_live(&mut self) {
        self.live.clear();
        self.live_h = 0;
        self.live_tex = None;
        self.dirty = false;
    }

    /// The live chart as a texture, rebuilt only when rows have arrived.
    ///
    /// A full chart is two megapixels and re-uploading it every frame at 120
    /// lines a minute would spend the whole GPU budget on a picture that
    /// changes twice a second.
    pub fn live_texture(&mut self, ctx: &egui::Context) -> Option<&egui::TextureHandle> {
        if self.live_h == 0 {
            return None;
        }
        if self.dirty || self.live_tex.is_none() {
            let img = gray_image(&self.live, self.live_w, self.live_h);
            match &mut self.live_tex {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => {
                    self.live_tex =
                        Some(ctx.load_texture("wefax-live", img, egui::TextureOptions::LINEAR))
                }
            }
            self.dirty = false;
        }
        self.live_tex.as_ref()
    }

    /// Hold a chart that has just finished arriving, until the engine names the
    /// file it went into.
    ///
    /// The name is the metadata — when the chart was received and what it was
    /// tuned to, both read straight back out of it — and only the engine knows
    /// it, because only the engine wrote the file. Reconstructing it here off a
    /// second clock got a chart within a second of the truth, which is close
    /// enough to look right and not close enough to fetch by.
    pub fn hold_fresh(&mut self, ctx: &egui::Context, png: &[u8]) {
        self.fresh = decode_gray(png).map(|(gray, w, h)| {
            let tex = ctx.load_texture(
                "wefax-fresh",
                gray_image(&gray, w, h),
                egui::TextureOptions::LINEAR,
            );
            (tex, png.to_vec())
        });
    }

    /// One page of the engine's chart store.
    pub fn on_listing(&mut self, listing: sdroxide_types::ImageListing, ctx: &egui::Context) {
        self.page_pending = false;
        self.total = listing.total;
        self.dir = listing.dir;
        for entry in listing.entries {
            self.insert_entry(entry, None, ctx);
        }
    }

    /// Whether the viewer must ask the engine for the full-size chart `name`:
    /// when its texture is missing, or when the PNG held for SAVE belongs to
    /// another chart (the store keeps one, and a texture already in memory
    /// would otherwise leave SAVE missing), and not yet asked for this one.
    pub fn needs_full(&self, name: &str, has_texture: bool) -> bool {
        let png_here = self.full_png.as_ref().is_some_and(|(n, _)| n == name);
        (!has_texture || !png_here) && self.full_asked.as_deref() != Some(name)
    }

    /// Whether more of the store can still be paged in, or this client is
    /// holding as many thumbnails as it will.
    pub fn can_page(&self) -> bool {
        self.gallery.len() < GALLERY_MAX
    }

    /// A chart the engine has just stored, taking the full-size texture the
    /// panel already holds from the one that has this moment finished arriving.
    pub fn on_saved(&mut self, entry: sdroxide_types::ImageEntry, ctx: &egui::Context) {
        let fresh = self.fresh.take();
        if let Some((_, png)) = &fresh {
            self.full_png = Some((entry.name.clone(), png.clone()));
        }
        self.total += 1;
        self.insert_entry(entry, fresh.map(|(tex, _)| tex), ctx);
    }

    /// A chart is no longer in the store: this screen's delete, or another's.
    ///
    /// The viewer is an index into the gallery, so it moves with it. Deleting
    /// the chart on screen leaves the viewer on the next-older one — the same
    /// place OLDER ▶ would have gone — which is what makes throwing away a run
    /// of blank pages a sequence of clicks rather than a reopen after each.
    pub fn on_deleted(&mut self, name: &str) {
        let Some(at) = self.gallery.iter().position(|c| c.name == name) else { return };
        self.gallery.remove(at);
        // `total` is the whole store's count, of which this gallery is a
        // window. It comes down for a chart that was in the window, because
        // that is the case where this client knows the store really shrank; for
        // one past the end it is left alone and the next listing settles it.
        self.total = self.total.saturating_sub(1);
        if self.full_png.as_ref().is_some_and(|(n, _)| n == name) {
            self.full_png = None;
        }
        if self.confirm_delete.as_deref() == Some(name) {
            self.confirm_delete = None;
        }
        if let Some(v) = self.viewing {
            let next = if at < v { v - 1 } else { v };
            self.viewing = (next < self.gallery.len()).then_some(next);
            // Only a delete of the chart being viewed puts a different one in
            // the window; anything above it just shifted the same chart up.
            if at == v {
                self.full_asked = None;
                self.full_gone = false;
            }
        }
    }

    /// A fetched full-size chart. An empty answer means the store no longer has
    /// it — the file moved or was deleted between listing and opening.
    pub fn on_file(&mut self, name: &str, png: &[u8], ctx: &egui::Context) {
        let Some((gray, w, h)) = decode_gray(png) else {
            if self.full_asked.as_deref() == Some(name) {
                self.full_gone = true;
            }
            return;
        };
        let tex = ctx.load_texture(
            format!("wefax-{name}"),
            gray_image(&gray, w, h),
            egui::TextureOptions::LINEAR,
        );
        if let Some(c) = self.gallery.iter_mut().find(|c| c.name == name) {
            c.full = Some(tex);
        }
        self.full_png = Some((name.to_string(), png.to_vec()));
    }

    /// Add a gallery entry in received order, skipping one already held.
    ///
    /// A listing and a just-arrived notification can name the same chart when
    /// one lands in the gap between the request and its answer; the name is
    /// what tells them apart.
    fn insert_entry(
        &mut self,
        entry: sdroxide_types::ImageEntry,
        full: Option<egui::TextureHandle>,
        ctx: &egui::Context,
    ) {
        if self.gallery.iter().any(|c| c.name == entry.name) {
            return;
        }
        let Some((gray, tw, th)) = decode_gray(&entry.thumb) else { return };
        let texture = ctx.load_texture(
            format!("wefax-thumb-{}", entry.name),
            gray_image(&gray, tw, th),
            egui::TextureOptions::LINEAR,
        );
        let at = self.gallery.partition_point(|c| c.unix > entry.unix);
        self.gallery.insert(
            at,
            Chart {
                texture,
                full,
                meta: sdroxide_types::WefaxChartMeta::from_file_name(&entry.name),
                size: (entry.width, entry.height),
                unix: entry.unix,
                name: entry.name,
            },
        );
        // The entry the viewer is on has just moved down if this went above it.
        if let Some(v) = self.viewing.as_mut() {
            if at <= *v {
                *v += 1;
            }
        }
        // A season of charts is more than this client will hold thumbnails for.
        // The oldest go, not the newest — they are still in the store, and
        // `total` still counts them.
        self.gallery.truncate(GALLERY_MAX);
    }
}

/// The size on screen of a chart `size` pixels across, shown in a `view`-sized
/// area at magnification `zoom` and vertical stretch `aspect`.
pub fn live_size(zoom: Zoom, aspect: f32, view: (f32, f32), size: (u16, u16)) -> (f32, f32) {
    let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
    let aspect = if aspect.is_finite() { aspect.clamp(MIN_ASPECT, MAX_ASPECT) } else { 1.0 };
    let scale = match zoom {
        // Fit — but never blow a chart up past its own pixels. A 904-pixel
        // IOC 288 chart stretched across a wide panel is a blurrier picture,
        // not a bigger one, and magnification can be asked for.
        Zoom::FitWidth => (view.0 / w).clamp(MIN_ZOOM, 1.0),
        // No lower limit worth the name: a finished chart is two thousand lines
        // tall in a panel a few hundred high, and a floor that stopped short of
        // fitting it would make this the same button as FIT.
        Zoom::Whole => (view.0 / w).min(view.1 / (h * aspect)).clamp(0.01, 1.0),
        Zoom::Fixed(z) => {
            if z.is_finite() {
                z.clamp(MIN_ZOOM, MAX_ZOOM)
            } else {
                1.0
            }
        }
    };
    (w * scale, h * scale * aspect)
}

/// Decode a PNG to a single-channel raster plus its size.
pub fn decode_gray(png: &[u8]) -> Option<(Vec<u8>, u16, u16)> {
    let img = image::load_from_memory(png).ok()?.to_luma8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || w as usize > MAX_W * 4 || h as usize > MAX_H * 4 {
        return None;
    }
    Some((img.into_raw(), w as u16, h as u16))
}

/// A single-channel raster as an egui image.
pub fn gray_image(gray: &[u8], w: u16, h: u16) -> egui::ColorImage {
    let (w, h) = (w as usize, h as usize);
    let mut px = Vec::with_capacity(w * h);
    for i in 0..w * h {
        let v = gray.get(i).copied().unwrap_or(0);
        px.push(egui::Color32::from_gray(v));
    }
    egui::ColorImage { size: [w, h], pixels: px, source_size: egui::vec2(w as f32, h as f32) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chart whose texture is in memory but whose PNG is another chart's
    /// must be fetched again, or SAVE never appears.
    #[test]
    fn a_chart_with_a_texture_but_no_png_is_fetched_for_save() {
        let mut ui = WefaxUi::default();
        ui.full_png = Some(("b".into(), vec![1]));
        assert!(ui.needs_full("a", true), "SAVE would be missing for `a`");
        ui.full_asked = Some("a".into());
        assert!(!ui.needs_full("a", true), "asked once is enough");
        assert!(!ui.needs_full("b", true), "b already has its PNG");
        assert!(ui.needs_full("c", false));
    }

    #[test]
    fn rows_build_the_chart_downwards() {
        let mut ui = WefaxUi::default();
        for y in 0..4u16 {
            ui.push_line(1, y, &[y as u8; 8]);
        }
        assert_eq!(ui.live_size(), (8, 4));
        assert_eq!(ui.live[0], 0);
        assert_eq!(ui.live[8], 1);
        assert_eq!(ui.live[24], 3);
        assert!(ui.has_live());
    }

    /// A gap in the rows has to leave a band, not slide the rest of the chart
    /// up — a seam through the middle of a weather map is much harder to spot
    /// than a grey stripe, and it puts every isobar below it in the wrong place.
    #[test]
    fn a_dropped_row_leaves_a_band_rather_than_a_seam() {
        let mut ui = WefaxUi::default();
        ui.push_line(1, 0, &[10; 4]);
        ui.push_line(1, 3, &[20; 4]);
        assert_eq!(ui.live_size(), (4, 4));
        assert_eq!(ui.live[0], 10);
        assert_eq!(&ui.live[4..12], &[128; 8], "the gap should be mid-grey");
        assert_eq!(ui.live[12], 20);
    }

    /// A new transmission starts a new picture rather than appending to the
    /// last one.
    #[test]
    fn a_new_transmission_starts_a_new_chart() {
        let mut ui = WefaxUi::default();
        ui.push_line(1, 0, &[1; 4]);
        ui.push_line(1, 1, &[2; 4]);
        assert_eq!(ui.live_size(), (4, 2));
        ui.push_line(2, 0, &[3; 6]);
        assert_eq!(ui.live_size(), (6, 1), "the width should follow the new picture");
        assert_eq!(ui.live[0], 3);
        ui.clear_live();
        assert!(!ui.has_live());
        assert_eq!(ui.live_size(), (6, 0));
    }

    /// A one-pixel PNG, for the gallery tests: `insert_entry` decodes the
    /// thumbnail it is given, so the entry has to carry a real picture.
    fn dot_png() -> Vec<u8> {
        let img = image::GrayImage::from_pixel(2, 2, image::Luma([128]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .expect("encode");
        buf.into_inner()
    }

    /// A gallery of `n` charts, newest first, named the way the store names them.
    fn gallery_of(n: i64, ctx: &egui::Context) -> WefaxUi {
        let mut ui = WefaxUi::default();
        for i in 0..n {
            ui.on_saved(
                sdroxide_types::ImageEntry {
                    kind: sdroxide_types::ImageKind::Wefax,
                    name: format!("wefax-2026072{i}-141530Z.png"),
                    unix: 1_000 + i,
                    width: 2,
                    height: 2,
                    bytes: 64,
                    thumb: dot_png(),
                    rifp: None,
                },
                ctx,
            );
        }
        ui
    }

    /// The viewer is an index into the gallery, so a deletion has to move it.
    ///
    /// Getting this wrong is not cosmetic: an index left where it was after the
    /// list shortened means the operator deletes one chart and the window jumps
    /// to a different one — or, at the end of the list, that the next DELETE is
    /// aimed at whatever slid into the gap.
    #[test]
    fn deleting_a_chart_carries_the_viewer_to_the_next_one() {
        let ctx = egui::Context::default();
        let mut ui = gallery_of(4, &ctx);
        // Newest first: index 0 is the highest unix.
        assert_eq!(ui.gallery.len(), 4);
        assert_eq!(ui.total, 4);
        let names: Vec<String> = ui.gallery.iter().map(|c| c.name.clone()).collect();

        // Deleting above the viewer shifts the same chart up under it.
        ui.viewing = Some(2);
        ui.on_deleted(&names[0]);
        assert_eq!(ui.viewing, Some(1));
        assert_eq!(ui.gallery[1].name, names[2], "the viewer must stay on its chart");
        assert_eq!(ui.total, 3, "the store shrank by one");

        // Deleting the chart being viewed lands on the next-older one, which is
        // what makes culling a run of blank pages a sequence of clicks.
        ui.on_deleted(&names[2]);
        assert_eq!(ui.viewing, Some(1));
        assert_eq!(ui.gallery[1].name, names[3]);

        // Deleting the last one has nowhere to go, so the window closes.
        ui.on_deleted(&names[3]);
        assert_eq!(ui.viewing, None);
        assert_eq!(ui.gallery.len(), 1);

        // A name this client is not holding — an older chart deleted from
        // another screen — changes nothing, `total` included: this gallery is
        // only a window on the store and cannot tell what happened outside it.
        ui.total = 40;
        ui.on_deleted("wefax-20990101-000000Z.png");
        assert_eq!((ui.gallery.len(), ui.total), (1, 40));
    }

    /// An armed DELETE belongs to one chart. Left set, it would be a loaded
    /// second click waiting for whatever the operator opened next.
    #[test]
    fn an_armed_delete_does_not_survive_its_chart() {
        let ctx = egui::Context::default();
        let mut ui = gallery_of(2, &ctx);
        let name = ui.gallery[0].name.clone();
        ui.confirm_delete = Some(name.clone());
        ui.on_deleted(&name);
        assert_eq!(ui.confirm_delete, None);
    }

    /// Rubbish off the wire must be refused rather than sized into an
    /// allocation, and a row that does not match the picture's width dropped.
    #[test]
    fn malformed_rows_are_refused() {
        let mut ui = WefaxUi::default();
        ui.push_line(1, 0, &[]);
        assert!(!ui.has_live());
        ui.push_line(1, 0, &vec![0u8; MAX_W + 1]);
        assert!(!ui.has_live());
        // A width change mid-picture is not a picture.
        ui.push_line(1, 0, &[1; 8]);
        ui.push_line(1, 1, &[1; 9]);
        assert_eq!(ui.live_size(), (8, 1));
        // A row behind the write head is dropped rather than overwriting.
        ui.push_line(1, 5, &[2; 8]);
        assert_eq!(ui.live_size(), (8, 6), "the gap is filled forwards");
    }

    /// Fitting must never magnify — a chart blown up past its own pixels is
    /// blurrier, not more legible — and an explicit magnification must be
    /// honoured whatever the panel is doing, since that is what makes the
    /// isobars readable while the chart is still arriving.
    #[test]
    fn fitting_shrinks_to_the_panel_and_magnifying_ignores_it() {
        // A 1809-pixel chart in a 400-pixel panel fits at a little over a fifth.
        let (w, h) = live_size(Zoom::FitWidth, 1.0, (400.0, 300.0), (1809, 1200));
        assert!((w - 400.0).abs() < 1e-3, "{w}");
        assert!((h - 1200.0 * 400.0 / 1809.0).abs() < 1e-3, "{h}");
        // A narrow chart in a wide panel stays at its own size.
        assert_eq!(live_size(Zoom::FitWidth, 1.0, (2000.0, 900.0), (904, 500)), (904.0, 500.0));
        // Magnification is taken as given, and clamped to something sane.
        assert_eq!(live_size(Zoom::Fixed(1.0), 1.0, (400.0, 300.0), (1809, 100)).0, 1809.0);
        assert_eq!(live_size(Zoom::Fixed(2.0), 1.0, (400.0, 300.0), (1809, 100)).0, 3618.0);
        assert_eq!(
            live_size(Zoom::Fixed(99.0), 1.0, (400.0, 300.0), (100, 10)).0,
            100.0 * MAX_ZOOM
        );
        assert_eq!(live_size(Zoom::Fixed(0.0), 1.0, (400.0, 300.0), (100, 10)).0, 100.0 * MIN_ZOOM);
        // A zero-sized picture must not divide by zero.
        let (w, h) = live_size(Zoom::FitWidth, 1.0, (400.0, 300.0), (0, 0));
        assert!(w.is_finite() && h.is_finite());
    }

    /// "Whole" has to bring a chart taller than the panel entirely into view —
    /// that is the difference between it and fitting the width, and the reason
    /// a finished chart can be taken in at a glance.
    #[test]
    fn the_whole_chart_fits_in_the_panel_both_ways() {
        // 1809 × 1200 in a 400 × 200 panel: width-fitting is 400 × 265, too
        // tall; whole shrinks until the height fits.
        let (_, h) = live_size(Zoom::FitWidth, 1.0, (400.0, 200.0), (1809, 1200));
        assert!(h > 200.0, "width-fitting leaves it too tall: {h}");
        let (w, h) = live_size(Zoom::Whole, 1.0, (400.0, 200.0), (1809, 1200));
        assert!(w <= 400.001 && h <= 200.001, "{w} × {h} should fit");
        // The aspect stretch is part of what has to fit, or a stretched chart
        // would still hang out of the bottom of the panel.
        let (w, h) = live_size(Zoom::Whole, 2.0, (400.0, 200.0), (1809, 1200));
        assert!(w <= 400.001 && h <= 200.001, "stretched: {w} × {h}");
    }

    /// The vertical stretch is what pulls a chart received at the wrong line
    /// rate back into shape, so it has to scale the height and nothing else.
    #[test]
    fn the_aspect_control_stretches_only_the_height() {
        let (w1, h1) = live_size(Zoom::Fixed(1.0), 1.0, (400.0, 300.0), (900, 200));
        let (w2, h2) = live_size(Zoom::Fixed(1.0), 2.0, (400.0, 300.0), (900, 200));
        assert_eq!(w1, w2, "the width must not move");
        assert_eq!((h1, h2), (200.0, 400.0));
        // Squashing works the same way, and both ends are clamped.
        assert_eq!(live_size(Zoom::Fixed(1.0), 0.5, (400.0, 300.0), (900, 200)).1, 100.0);
        assert_eq!(
            live_size(Zoom::Fixed(1.0), 99.0, (400.0, 300.0), (900, 200)).1,
            200.0 * MAX_ASPECT
        );
        assert_eq!(
            live_size(Zoom::Fixed(1.0), 0.0, (400.0, 300.0), (900, 200)).1,
            200.0 * MIN_ASPECT
        );
        // Rubbish must not produce a NaN-sized widget.
        let (w, h) = live_size(Zoom::Fixed(f32::NAN), f32::NAN, (400.0, 300.0), (900, 200));
        assert!(w.is_finite() && h.is_finite());
    }

    /// The gallery is browsed newest first, and the ordering has to come from
    /// when a chart was received rather than the order files happened to be
    /// read off disk.
    #[test]
    fn charts_are_ordered_newest_first_by_their_own_timestamps() {
        use sdroxide_types::WefaxChartMeta;
        let names = [
            "wefax-20260729-141530Z-7878.1kHz-DWD.png",
            "wefax-20260729-061500Z-3853.1kHz-DWD.png",
            "wefax-20260728-235900Z.png",
        ];
        let mut metas: Vec<_> =
            names.iter().map(|n| WefaxChartMeta::from_file_name(n).expect(n)).collect();
        metas.sort_by(|a, b| b.unix.cmp(&a.unix));
        assert_eq!(metas[0].when_label(), "2026-07-29 14:15Z");
        assert_eq!(metas[2].when_label(), "2026-07-28 23:59Z");
        // The newest of them names its station rather than a bare frequency.
        assert!(metas[0].where_label().unwrap().contains("DWD"));
    }

    #[test]
    fn a_raster_becomes_a_grey_image_of_the_right_size() {
        let img = gray_image(&[0, 128, 255, 64], 2, 2);
        assert_eq!(img.size, [2, 2]);
        assert_eq!(img.pixels[0], egui::Color32::from_gray(0));
        assert_eq!(img.pixels[2], egui::Color32::from_gray(255));
        // A short buffer is padded rather than panicking.
        assert_eq!(gray_image(&[1], 2, 2).pixels.len(), 4);
    }
}
