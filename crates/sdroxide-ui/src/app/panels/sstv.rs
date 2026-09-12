//! The SSTV and RIFP image panel.
//!
//! [`SstvUi`] draws two things the engine owns and one it does not. The five
//! transmit presets and the gallery of received pictures live on the machine
//! the radio is plugged into — the panel holds metadata, thumbnails and
//! whatever pixels it has been handed, and asks for the rest. The picture
//! currently arriving is the exception: scanlines are painted into a texture as
//! they come, so a picture builds up on screen exactly as it does on the air.
//!
//! Compositing stays here. The crop, the header strip and the overlay text are
//! applied to the fetched source client-side, so the preview redraws on every
//! keystroke without a round trip, and only the finished picture goes back to
//! the engine as an ordinary `SstvTx` / `RifpTx`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{
    Command, IMAGE_SLOTS, IMAGE_UPLOAD_MAX, ImageEntry, ImageKind, ImagePresets, Mode,
    RifpEncoding, RifpProfile, RifpSize, RifpStatus, SstvMode, SstvStatus,
};

use crate::theme::ThemedScroll;

use crate::app::panels::widgets::{pick_image, sstv_section};
use crate::app::util::shorten;
use crate::app::{SdroxideApp, rx_only_hint};

// ───────────────────────────── SSTV panel ──────────────────────────────

/// A fetched transmit source: the picture the compositor works from, and which
/// stored version it is, so a slot changed from another screen is refetched
/// rather than composed from the old photograph.
pub(in crate::app) struct SstvSlot {
    src_rgb: Vec<u8>,
    sw: u16,
    sh: u16,
    version: u32,
}

/// Pictures the gallery will hold thumbnails for.
///
/// Not a limit on the collection — the engine counts the whole store and the
/// panel says how many more there are — but on what this client keeps as
/// textures.
const GALLERY_MAX: usize = 240;

/// A received-picture gallery entry.
pub(in crate::app) struct SstvRecv {
    entry: ImageEntry,
    thumb: egui::TextureHandle,
    /// The picture at full size, once it has been fetched — or promoted
    /// straight from the one that had just been received, which the panel
    /// already held. `None` until the operator enlarges it.
    full: Option<egui::TextureHandle>,
}

/// Image-panel state, shared by SSTV and RIFP: received gallery, in-progress
/// incoming picture, transmit slots, the overlay message, the current mode, and
/// cached textures.
///
/// One workspace for both modes on purpose. The pictures an operator wants to
/// send, the captions on them, and the pictures that came back are the same
/// things whichever protocol carried them; only the control strip and the
/// transmit sizing differ.
pub(in crate::app) struct SstvUi {
    pub(in crate::app) tx_mode: SstvMode,
    /// Latest RIFP engine status (transfer progress, sessions, counters).
    pub(in crate::app) rifp: RifpStatus,
    /// Size of the picture currently arriving, so the live canvas can be built
    /// before the whole object is in. `(0, 0)` when nothing is arriving.
    pub(in crate::app) rx_dims: (u16, u16),
    /// Size the cached preview was composed at, so a change of transmit size
    /// rebuilds it.
    pub(in crate::app) preview_dims: (u16, u16),
    /// The banner drawn across the top of the transmit image, resolved from the
    /// station's digital-mode config. `None` when the operator switched it off.
    ///
    /// Resolved here rather than at each compose because it is what tells the
    /// preview it is stale: the strip changes when the callsign changes, when a
    /// template is edited, when a colour is picked, and comparing the composed
    /// banner catches all of that with one test.
    pub(in crate::app) banner: Option<crate::sstv::Banner>,
    /// Whether the banner editor window is open.
    pub(in crate::app) banner_open: bool,
    /// Auto mode: RX auto-detects the mode; TX defaults to Martin 1 until a mode
    /// is heard or the operator picks one.
    pub(in crate::app) auto: bool,
    /// The engine's transmit presets: what is in each slot and the message over
    /// it. The truth — `slots` below only caches its pixels.
    pub(in crate::app) presets: ImagePresets,
    /// Thumbnails of the presets, rebuilt when the engine announces a change.
    pub(in crate::app) slot_thumbs: Vec<Option<egui::TextureHandle>>,
    /// The message being typed and which slot it belongs to. Only that one slot
    /// is client-owned, so an echo of our own write — or an edit made on another
    /// screen — can update the other four without fighting the keyboard.
    pub(in crate::app) msg_edit: Option<(usize, String)>,
    /// Fetched source pictures, keyed by slot; `None` until one arrives.
    pub(in crate::app) slots: Vec<Option<SstvSlot>>,
    /// The version last asked for per slot.
    ///
    /// Deliberately not cleared when the answer lands: it records the *ask*, so
    /// a picture that fails to decode is asked for once rather than once a
    /// frame for ever. A slot changed on the radio arrives with a new version,
    /// which is what makes the next ask happen.
    pub(in crate::app) src_asked: HashMap<usize, u32>,
    pub(in crate::app) selected_slot: usize,
    pub(in crate::app) received: Vec<SstvRecv>,
    /// How many pictures the store holds altogether, so the gallery can say
    /// exactly how many older ones there are rather than guessing.
    pub(in crate::app) total: u32,
    /// Where the engine is saving received pictures, for the gallery to show.
    pub(in crate::app) dir: String,
    /// True while a listing page is outstanding — the reliable lane is finite
    /// and a scroll must not spray requests down it.
    pub(in crate::app) page_pending: bool,
    /// Name of the picture whose full size has been asked for. Like
    /// `src_asked`, it records the ask and is cleared when the viewer moves on,
    /// not when the answer arrives.
    pub(in crate::app) full_asked: Option<String>,
    /// The answer to that ask was empty — the store no longer has it.
    pub(in crate::app) full_gone: bool,
    /// In-progress incoming image (painted line-by-line).
    pub(in crate::app) rx_color: Option<egui::ColorImage>,
    pub(in crate::app) rx_tex: Option<egui::TextureHandle>,
    pub(in crate::app) rx_id: u32,
    /// The picture that has just finished arriving, held until the engine names
    /// the file it went into. Enlarging it then costs nothing, where fetching
    /// back what the panel was handed a moment ago would cost a megabyte.
    pub(in crate::app) fresh: Option<(egui::TextureHandle, Vec<u8>)>,
    /// The bytes of the full-size picture last fetched or received, and its
    /// name. One at a time: this is what "Save picture…" writes out, and
    /// keeping every picture's bytes for a button nobody may press would hold
    /// a gallery's worth of megabytes for nothing.
    pub(in crate::app) full_png: Option<(String, Vec<u8>)>,
    pub(in crate::app) status: SstvStatus,
    /// Received-gallery index currently shown enlarged in an overlay window.
    pub(in crate::app) enlarged: Option<usize>,
    /// Name of the picture whose DELETE has been pressed once and is waiting to
    /// be pressed again.
    ///
    /// Two presses rather than a modal, because the file goes for good and there
    /// is no undo to offer: a stray click on a chip beside SAVE would otherwise
    /// be the last anyone saw of a picture. Armed per name, so it cannot survive
    /// into the next picture and delete that one instead.
    pub(in crate::app) confirm_delete: Option<String>,
    /// Last VIS/free-run-detected mode we auto-applied to `tx_mode`, so a steady
    /// detection doesn't keep overriding the operator's manual mode choice.
    pub(in crate::app) last_detected: Option<SstvMode>,
    pub(in crate::app) preview_tex: Option<egui::TextureHandle>,
    pub(in crate::app) preview_dirty: bool,
    /// Whether the store has been listed this session.
    pub(in crate::app) listed: bool,
    /// File-picker result inbox (raw image bytes), filled by the picker task.
    pub(in crate::app) inbox: Arc<Mutex<Option<Vec<u8>>>>,
    pub(in crate::app) pick_target: Option<usize>,
    /// A local complaint about a picked file — too big to be worth sending, or
    /// one the engine refused. Cleared when the next picture is picked.
    pub(in crate::app) pick_error: Option<String>,
}

impl Default for SstvUi {
    fn default() -> Self {
        SstvUi {
            tx_mode: SstvMode::Martin1,
            rifp: RifpStatus::default(),
            rx_dims: (0, 0),
            preview_dims: (0, 0),
            banner: None,
            banner_open: false,
            auto: true,
            presets: ImagePresets::default(),
            slot_thumbs: (0..IMAGE_SLOTS).map(|_| None).collect(),
            msg_edit: None,
            slots: (0..IMAGE_SLOTS).map(|_| None).collect(),
            src_asked: HashMap::new(),
            selected_slot: 0,
            received: Vec::new(),
            total: 0,
            dir: String::new(),
            page_pending: false,
            full_asked: None,
            full_gone: false,
            rx_color: None,
            rx_tex: None,
            rx_id: 0,
            fresh: None,
            full_png: None,
            status: SstvStatus::default(),
            enlarged: None,
            confirm_delete: None,
            last_detected: None,
            preview_tex: None,
            preview_dirty: true,
            listed: false,
            inbox: Arc::new(Mutex::new(None)),
            pick_target: None,
            pick_error: None,
        }
    }
}

impl SstvUi {
    /// A decoded scanline arrived: paint it into the in-progress image.
    pub(in crate::app) fn on_line(&mut self, id: u32, y: u16, rgb: &[u8], ctx: &egui::Context) {
        let Some(mode) = self.status.detected else { return };
        let (w, h) = mode.dimensions();
        if self.rx_id != id || self.rx_color.is_none() {
            self.rx_id = id;
            self.rx_color =
                Some(crate::sstv::color_image(&vec![0u8; w as usize * h as usize * 3], w, h));
        }
        let Some(ci) = self.rx_color.as_mut() else { return };
        let (w, h) = (w as usize, h as usize);
        if (y as usize) < h && rgb.len() >= w * 3 {
            let row = y as usize * w;
            for x in 0..w {
                ci.pixels[row + x] = Color32::from_rgb(rgb[x * 3], rgb[x * 3 + 1], rgb[x * 3 + 2]);
            }
        }
        self.rx_tex = Some(ctx.load_texture("sstv_rx", ci.clone(), egui::TextureOptions::NEAREST));
    }

    /// A completed picture arrived. The gallery entry comes from the engine a
    /// moment later, naming the file it was saved as; what is held here is the
    /// full-size texture, so enlarging it does not fetch back what the panel
    /// has just been given.
    pub(in crate::app) fn on_image(&mut self, png: &[u8], ctx: &egui::Context) {
        self.fresh = crate::sstv::decode_image(png).map(|(rgb, w, h)| {
            let ci = crate::sstv::color_image(&rgb, w, h);
            (ctx.load_texture("sstv_recv", ci, egui::TextureOptions::NEAREST), png.to_vec())
        });
        self.rx_color = None;
        self.rx_tex = None;
    }

    /// RIFP: reassembled raster rows arrived — paint them into the live
    /// picture. Only the unencoded raster gets here; everything else appears
    /// whole in [`SstvUi::on_rifp_image`].
    pub(in crate::app) fn on_rifp_rows(
        &mut self,
        id: u32,
        y: u16,
        w: u16,
        h: u16,
        gray: &[u8],
        ctx: &egui::Context,
    ) {
        if self.rx_id != id || self.rx_color.is_none() || self.rx_dims != (w, h) {
            self.rx_id = id;
            self.rx_dims = (w, h);
            self.rx_color =
                Some(crate::sstv::color_image(&vec![0u8; w as usize * h as usize * 3], w, h));
        }
        let Some(ci) = self.rx_color.as_mut() else { return };
        let (wu, hu) = (w as usize, h as usize);
        for (row, pixels) in gray.chunks_exact(wu).enumerate() {
            let y = y as usize + row;
            if y >= hu {
                break;
            }
            for (x, &g) in pixels.iter().enumerate() {
                ci.pixels[y * wu + x] = Color32::from_gray(g);
            }
        }
        self.rx_tex = Some(ctx.load_texture("rifp_rx", ci.clone(), egui::TextureOptions::NEAREST));
    }

    /// RIFP: a complete, digest-verified picture arrived.
    pub(in crate::app) fn on_rifp_image(&mut self, png: &[u8], ctx: &egui::Context) {
        self.on_image(png, ctx);
        self.rx_dims = (0, 0);
    }

    /// The engine announced the transmit presets: adopt them, rebuild the slot
    /// thumbnails, and drop any cached source whose picture has been replaced.
    pub(in crate::app) fn on_presets(&mut self, presets: ImagePresets, ctx: &egui::Context) {
        for (i, slot) in presets.slots.iter().enumerate() {
            let held = self.slots.get(i).and_then(|s| s.as_ref()).map(|s| s.version);
            if held.is_some_and(|v| v != slot.version) || (held.is_some() && !slot.has_picture()) {
                if let Some(cell) = self.slots.get_mut(i) {
                    *cell = None;
                }
                if i == self.selected_slot {
                    self.preview_dirty = true;
                }
            }
            let thumb = (!slot.thumb.is_empty())
                .then(|| crate::sstv::decode_image(&slot.thumb))
                .flatten()
                .map(|(rgb, w, h)| {
                    let ci = crate::sstv::color_image(&rgb, w, h);
                    ctx.load_texture("sstv_slot", ci, egui::TextureOptions::LINEAR)
                });
            if let Some(cell) = self.slot_thumbs.get_mut(i) {
                *cell = thumb;
            }
        }
        // The message box takes the engine's text for every slot except the one
        // under the cursor, so an edit made elsewhere shows up.
        if self.presets.slot(self.selected_slot).message != presets.slot(self.selected_slot).message
            && !matches!(&self.msg_edit, Some((i, _)) if *i == self.selected_slot)
        {
            self.preview_dirty = true;
        }
        self.presets = presets;
    }

    /// A fetched source picture for a slot.
    pub(in crate::app) fn on_slot_source(&mut self, slot: u8, version: u32, png: &[u8]) {
        let slot = slot as usize;
        let Some(cell) = self.slots.get_mut(slot) else { return };
        *cell = crate::sstv::load_source_bounded(png, 1024).map(|(rgb, sw, sh)| SstvSlot {
            src_rgb: rgb,
            sw,
            sh,
            version,
        });
        if slot == self.selected_slot {
            self.preview_dirty = true;
        }
    }

    /// One page of the received store.
    pub(in crate::app) fn on_listing(
        &mut self,
        listing: sdroxide_types::ImageListing,
        ctx: &egui::Context,
    ) {
        self.page_pending = false;
        self.total = listing.total;
        self.dir = listing.dir;
        for entry in listing.entries {
            self.insert_entry(entry, None, ctx);
        }
    }

    /// A picture the engine has just stored. It goes to the front of the
    /// gallery, taking the full-size texture the panel already holds from the
    /// picture that has this moment finished arriving.
    pub(in crate::app) fn on_saved(&mut self, entry: ImageEntry, ctx: &egui::Context) {
        let fresh = self.fresh.take();
        if let Some((_, png)) = &fresh {
            self.full_png = Some((entry.name.clone(), png.clone()));
        }
        self.total += 1;
        self.insert_entry(entry, fresh.map(|(tex, _)| tex), ctx);
    }

    /// A picture is no longer in the store: this screen's delete, or another's.
    ///
    /// The enlarged view is an index into the list, so it is moved with the
    /// list. Deleting the picture being looked at leaves the index on the
    /// next-older one, which is what makes culling a gallery a sequence of
    /// clicks in one place rather than a reopen after every one.
    pub(in crate::app) fn on_deleted(&mut self, name: &str) {
        let Some(at) = self.received.iter().position(|r| r.entry.name == name) else { return };
        self.received.remove(at);
        // `total` counts the whole store, of which this list is only a window.
        // It comes down for a picture that was in the window, because that is
        // the case where this client knows the store really did shrink; for one
        // past the end it is left alone and the next listing settles it.
        self.total = self.total.saturating_sub(1);
        if self.full_png.as_ref().is_some_and(|(n, _)| n == name) {
            self.full_png = None;
        }
        if self.confirm_delete.as_deref() == Some(name) {
            self.confirm_delete = None;
        }
        if let Some(v) = self.enlarged {
            let next = if at < v { v - 1 } else { v };
            self.enlarged = (next < self.received.len()).then_some(next);
            // Only a delete of the picture being looked at puts a different one
            // in the window; anything above it just shifted the same picture up.
            if at == v {
                self.full_asked = None;
                self.full_gone = false;
            }
        }
    }

    /// A fetched full-size picture. An empty answer means the store no longer
    /// has it — the file moved or was deleted between listing and opening.
    pub(in crate::app) fn on_file(&mut self, name: &str, png: &[u8], ctx: &egui::Context) {
        let Some((rgb, w, h)) = crate::sstv::decode_image(png) else {
            if self.full_asked.as_deref() == Some(name) {
                self.full_gone = true;
            }
            return;
        };
        let tex = ctx.load_texture(
            "sstv_full",
            crate::sstv::color_image(&rgb, w, h),
            egui::TextureOptions::NEAREST,
        );
        if let Some(r) = self.received.iter_mut().find(|r| r.entry.name == name) {
            r.full = Some(tex);
        }
        self.full_png = Some((name.to_string(), png.to_vec()));
    }

    /// Add a gallery entry in received order, skipping one already held.
    ///
    /// The listing and the just-arrived notification can name the same picture
    /// when one lands in the gap between the request and its answer; the name
    /// is what tells them apart.
    fn insert_entry(
        &mut self,
        entry: ImageEntry,
        full: Option<egui::TextureHandle>,
        ctx: &egui::Context,
    ) {
        if self.received.iter().any(|r| r.entry.name == entry.name) {
            return;
        }
        let Some((rgb, w, h)) = crate::sstv::decode_image(&entry.thumb) else { return };
        let thumb = ctx.load_texture(
            "sstv_thumb",
            crate::sstv::color_image(&rgb, w, h),
            egui::TextureOptions::LINEAR,
        );
        let at = self.received.partition_point(|r| r.entry.unix > entry.unix);
        self.received.insert(at, SstvRecv { entry, thumb, full });
        // The enlarged view is an index into this list, so anything inserted
        // above it moves what the operator is looking at.
        if let Some(v) = self.enlarged.as_mut() {
            if at <= *v {
                *v += 1;
            }
        }
        // A long session receives more pictures than this client will hold
        // textures for. The oldest go, not the newest — they are still in the
        // store, and `total` still counts them.
        self.received.truncate(GALLERY_MAX);
    }

    /// The overlay message for the slot currently being edited.
    pub(in crate::app) fn current_message(&self) -> &str {
        crate::sstv::message_shown(&self.msg_edit, &self.presets, self.selected_slot)
    }

    /// Rebuild the transmit preview when the size, slot, or message changed.
    /// `dims` is the transmitted picture size — the SSTV line format's, or the
    /// operator's chosen RIFP size.
    pub(in crate::app) fn ensure_preview(&mut self, dims: (u16, u16), ctx: &egui::Context) {
        if !self.preview_dirty {
            return;
        }
        self.preview_dirty = false;
        let message = self.current_message().to_string();
        match self.slots.get(self.selected_slot).and_then(|s| s.as_ref()) {
            Some(slot) => {
                let (rgb, w, h) = crate::sstv::compose(
                    dims.0,
                    dims.1,
                    &slot.src_rgb,
                    slot.sw,
                    slot.sh,
                    &message,
                    self.banner.as_ref(),
                );
                let ci = crate::sstv::color_image(&rgb, w, h);
                self.preview_tex =
                    Some(ctx.load_texture("sstv_preview", ci, egui::TextureOptions::NEAREST));
            }
            None => self.preview_tex = None,
        }
    }

    /// The composed PNG for the current selection, for transmit.
    pub(in crate::app) fn compose_png(&self, dims: (u16, u16)) -> Option<Vec<u8>> {
        let slot = self.slots.get(self.selected_slot).and_then(|s| s.as_ref())?;
        let (rgb, w, h) = crate::sstv::compose(
            dims.0,
            dims.1,
            &slot.src_rgb,
            slot.sw,
            slot.sh,
            self.current_message(),
            self.banner.as_ref(),
        );
        crate::sstv::encode_png(&rgb, w, h)
    }

    /// Send a picked file to the engine for a slot.
    ///
    /// Nothing is kept here: the engine scales the picture, stores it and
    /// announces the presets, and the slot fills in when that comes back. That
    /// round trip is the point — it is what makes the browser tab and the
    /// console show the same five pictures.
    pub(in crate::app) fn set_slot(
        &mut self,
        slot: usize,
        bytes: Vec<u8>,
        cmds: &mut Vec<Command>,
    ) {
        self.selected_slot = slot;
        // Checked here as well as engine-side: pushing forty megabytes down the
        // socket only to have it refused at the far end is a poor experience,
        // and it keeps a huge upload clear of the WebSocket's frame limit.
        if bytes.len() > IMAGE_UPLOAD_MAX {
            self.pick_error = Some(format!(
                "That picture is {} MB — the limit is {} MB.",
                bytes.len() / 1_048_576,
                IMAGE_UPLOAD_MAX / 1_048_576,
            ));
            return;
        }
        self.pick_error = None;
        cmds.push(Command::ImageSetSlot { slot: slot as u8, bytes });
    }
}

/// One incoming RIFP transfer's chunk map: a lit cell per chunk received, dark
/// per chunk still missing. Beyond what fits, it degrades to a plain bar — the
/// point is to see *where* the holes are, and a thousand one-pixel cells show
/// nothing.
fn rifp_chunk_map(ui: &mut egui::Ui, session: &sdroxide_types::RifpSession) {
    let cells = session.total.max(session.have) as usize;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(120.0, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, crate::theme::gray(20));
    let have = |i: usize| session.map.get(i / 8).is_some_and(|b| b >> (i % 8) & 1 != 0);
    if cells > 0 && cells <= rect.width() as usize {
        let cw = rect.width() / cells as f32;
        for i in 0..cells {
            if !have(i) {
                continue;
            }
            let x = rect.left() + i as f32 * cw;
            p.rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(cw.max(1.0), 10.0)),
                0.0,
                crate::theme::GREEN(),
            );
        }
    } else if session.total > 0 {
        let mut fill = rect;
        fill.set_width(rect.width() * (session.have as f32 / session.total as f32).clamp(0.0, 1.0));
        p.rect_filled(fill, 2.0, crate::theme::GREEN());
    }
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::gray(60)),
        egui::StrokeKind::Inside,
    );
    resp.on_hover_text("Chunks received (lit) and still missing (dark)");
}

fn sstv_level_bar(ui: &mut egui::Ui, level: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(90.0, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, crate::theme::gray(20));
    // Log scale (~ -60..0 dBFS mean-abs) so weak-but-decodable signals still show.
    let db = 20.0 * level.max(1e-6).log10();
    let frac = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    let mut fill = rect;
    fill.set_width(rect.width() * frac);
    let col = if frac > 0.06 { crate::theme::GREEN() } else { crate::theme::gray(45) };
    p.rect_filled(fill, 2.0, col);
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::gray(60)),
        egui::StrokeKind::Inside,
    );
}

impl SdroxideApp {
    /// The image panel, shared by SSTV and RIFP: a live picture and a gallery
    /// on the left, a transmit compositor on the right, and a control strip
    /// that is the only part either mode owns alone.
    pub(in crate::app) fn image_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        mode: Mode,
    ) {
        let ctx = ui.ctx().clone();
        let rifp = mode.is_rifp();
        // The store is the engine's; list it once when the panel first opens.
        // Pictures that arrive afterwards are announced one at a time.
        if !self.sstv.listed {
            self.sstv.listed = true;
            self.sstv.page_pending = true;
            cmds.push(Command::ImageList {
                kind: ImageKind::Sstv,
                offset: 0,
                count: sdroxide_types::IMAGE_PAGE_MAX,
            });
        }
        // Drain a completed file-pick (only consume the target once bytes arrive).
        let picked = self.sstv.inbox.lock().ok().and_then(|mut g| g.take());
        if let Some(bytes) = picked {
            if let Some(target) = self.sstv.pick_target.take() {
                self.sstv.set_slot(target, bytes, cmds);
            }
        }
        // Fetch the selected slot's source when what is cached is not what the
        // engine says is stored — a slot loaded here, or changed from another
        // screen. Asked once, not once per frame.
        let sel = self.sstv.selected_slot;
        let want = self.sstv.presets.slot(sel);
        let have = self.sstv.slots.get(sel).and_then(|s| s.as_ref()).map(|s| s.version);
        if want.has_picture()
            && have != Some(want.version)
            && self.sstv.src_asked.get(&sel) != Some(&want.version)
        {
            self.sstv.src_asked.insert(sel, want.version);
            cmds.push(Command::ImageGetSlot(sel as u8));
        }
        // Keep the banner in sync with the operator config — the callsign it
        // prints, the two templates, the colours and the height.
        let banner = crate::sstv::Banner::from_config(&self.digi_cfg_edit);
        if self.sstv.banner != banner {
            self.sstv.banner = banner;
            self.sstv.preview_dirty = true;
        }
        // The transmitted size: SSTV's line format fixes it, RIFP leaves it to
        // the operator. Changing it invalidates the composed preview.
        let dims = if rifp {
            self.digi_cfg_edit.rifp_size.dimensions()
        } else {
            self.sstv.tx_mode.dimensions()
        };
        if self.sstv.preview_dims != dims {
            self.sstv.preview_dims = dims;
            self.sstv.preview_dirty = true;
        }
        self.sstv.ensure_preview(dims, &ctx);
        crate::repaint::after_ms(&ctx, 120);

        let st = self.sstv.status.clone();
        let (signal, tx_active, progress) = if rifp {
            (self.sstv.rifp.signal, self.sstv.rifp.tx_active, self.sstv.rifp.tx_progress)
        } else {
            (st.signal, st.tx_active, st.progress)
        };

        // Whole-panel size. The mode/signal/slant controls sit in a boxed strip
        // on the left above LIVE + RECEIVED; the transmit compositor spans the
        // full height on the right, reclaiming the space the old full-width
        // control rows used to leave empty at the top.
        let avail = ui.available_size();
        let full_h = avail.y;
        let handle_w = 7.0;
        // TRANSMIT (send) column takes a user-draggable fraction of the width; the
        // receive side (LIVE + RECEIVED) gets the rest. Each keeps a usable minimum.
        // A phone takes the receive side and the compositor in turns: each has a
        // 300 pt floor of its own, so together they want twice a phone's width
        // before either has drawn a picture.
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let (left_w, tx_w) = match pane {
            // Whichever one is up takes the row.
            Some(0) => (avail.x, 0.0),
            Some(_) => (0.0, avail.x),
            None if self.ui_settings.swl => (avail.x, 0.0),
            None => {
                let tx = (avail.x * self.view.sstv_tx_fraction)
                    .clamp(300.0, (avail.x - handle_w - 300.0).max(300.0));
                ((avail.x - tx - handle_w).max(300.0), tx)
            }
        };
        // LIVE takes the rest of the receive side; the RECEIVED gallery width is a
        // user-draggable fraction of it (min one thumbnail column).
        let gallery_w = (left_w * self.view.sstv_gallery_fraction)
            .clamp(150.0, (left_w - handle_w - 160.0).max(150.0));
        let live_w = (left_w - gallery_w - handle_w).max(160.0);

        ui.horizontal_top(|ui| {
            // A received thumbnail was clicked → enlarge it (applied after the row).
            let mut enlarge: Option<usize> = None;
            // The gallery asked for the next page.
            let mut more = false;
            // A picture was deleted from the gallery's context menu. By name,
            // not by index: the answer comes back asynchronously and the list
            // may have grown a picture at the front by then.
            let mut delete: Option<String> = None;

            // ── LEFT: boxed controls, then LIVE + RECEIVED ──
            if pane.is_none_or(|p| p == 0) {
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, full_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::Frame::new()
                        .fill(crate::theme::ROW_BG())
                        .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
                        .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 7 })
                        .show(ui, |ui| {
                            ui.set_min_width(left_w - 16.0);
                            ui.set_max_width(left_w - 16.0);
                            if rifp {
                                self.rifp_controls(ui, cmds);
                                return;
                            }

                            // Mode selection: Auto + the per-mode chips.
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new("SSTV")
                                        .size(12.0)
                                        .strong()
                                        .color(crate::theme::CYAN()),
                                );
                                self.digi_freq_chip(ui, cmds);
                                let auto_label = if self.sstv.auto {
                                    format!("Auto ({})", self.sstv.tx_mode.label())
                                } else {
                                    "Auto".to_string()
                                };
                                if crate::chrome::chip(ui, self.sstv.auto, &auto_label).clicked() {
                                    self.sstv.auto = true;
                                    self.sstv.tx_mode = SstvMode::Martin1;
                                    self.sstv.preview_dirty = true;
                                    cmds.push(Command::SstvSetMode(None));
                                }
                                for m in SstvMode::ALL {
                                    let active = !self.sstv.auto && self.sstv.tx_mode == m;
                                    if crate::chrome::chip(ui, active, m.label()).clicked() {
                                        self.sstv.auto = false;
                                        self.sstv.tx_mode = m;
                                        self.sstv.preview_dirty = true;
                                        cmds.push(Command::SstvSetMode(Some(m)));
                                    }
                                }
                            });
                            ui.add_space(5.0);

                            // Signal meter + activity, and the TX-slant trim.
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Signal").size(10.0).weak());
                                sstv_level_bar(ui, signal);
                                if tx_active {
                                    ui.label(
                                        RichText::new(format!("● TX {:.0}%", progress * 100.0))
                                            .size(11.0)
                                            .strong()
                                            .color(crate::theme::ALERT()),
                                    );
                                } else if st.rx_active {
                                    ui.label(
                                        RichText::new(format!("● RX {:.0}%", st.progress * 100.0))
                                            .size(11.0)
                                            .strong()
                                            .color(crate::theme::GREEN()),
                                    );
                                } else if let Some(m) = st.detected {
                                    ui.label(
                                        RichText::new(format!("last: {}", m.label()))
                                            .size(10.0)
                                            .weak(),
                                    );
                                } else {
                                    ui.label(RichText::new("listening…").size(10.0).weak());
                                }

                                // Issue #397. A receiver that has locked on is
                                // committed for the whole length of the mode it
                                // locked on to, and Scottie DX is four and a
                                // half minutes — so a VIS misread as a slow
                                // mode costs every picture sent while it runs
                                // out. On QO-100, where one station follows
                                // another over the same transponder, that is
                                // the next few overs.
                                //
                                // Offered whether or not a picture is under
                                // way: re-arming an idle hunt costs nothing,
                                // and a chip that appears only once the mistake
                                // has been made is one the operator has to find
                                // in a hurry. The half-picture goes with it,
                                // here as well as in the decoder — leaving the
                                // abandoned frame on screen would say the
                                // button had not worked.
                                if crate::chrome::chip(ui, false, "Restart RX")
                                    .on_hover_text(
                                        "Abandon the picture being received and listen for the                                          next header. For a transmission that started decoding                                          in the wrong mode — the receiver is otherwise committed                                          until that mode runs out.",
                                    )
                                    .clicked()
                                {
                                    cmds.push(Command::SstvRestartRx);
                                    self.sstv.rx_color = None;
                                    self.sstv.rx_tex = None;
                                }

                                // Who sent it. The FSK ID arrives in tones a
                                // fraction of a second after the picture, which
                                // is exactly when the operator is looking at the
                                // frame and wondering whose it is — and unlike a
                                // banner drawn into the image, this is the
                                // station's own machine-readable identification.
                                if let Some(id) = st.rx_id.as_deref() {
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new(format!("ID {id}"))
                                            .size(11.0)
                                            .strong()
                                            .color(crate::theme::CYAN_DIM()),
                                    )
                                    .on_hover_text(
                                        "The callsign the last station sent as an FSK ID after \
                                         its picture.",
                                    );
                                }

                                if !self.ui_settings.swl {
                                    ui.add_space(12.0);
                                    ui.separator();
                                    ui.label(RichText::new("TX slant").size(10.0).weak()).on_hover_text(
                                        "Transmit clock trim (ppm) to remove slant on the far-end decoder",
                                    );
                                    ui.add_enabled_ui(self.digi_cfg_seeded, |ui| {
                                        ui.spacing_mut().slider_width = 130.0;
                                        let resp = crate::chrome::slider(ui, egui::Slider::new(
                                                &mut self.digi_cfg_edit.sstv_tx_ppm,
                                                -5000.0..=5000.0,
                                            )
                                            .suffix(" ppm")
                                            .fixed_decimals(0),
                                        );
                                        if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                        if ui
                                            .small_button("0")
                                            .on_hover_text("Reset to 0 ppm")
                                            .clicked()
                                        {
                                            self.digi_cfg_edit.sstv_tx_ppm = 0.0;
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                        ui.separator();
                                        // The callsign in tones after the picture.
                                        // Beside the slant trim rather than in the
                                        // banner window: the banner identifies the
                                        // station to a person looking at the
                                        // picture, this identifies it to the
                                        // repeater decoding it, and the two are set
                                        // for different reasons.
                                        if crate::chrome::checkbox(
                                            ui,
                                            &mut self.digi_cfg_edit.sstv_fsk_id,
                                            "FSK ID",
                                        )
                                        .on_hover_text(
                                            "Send your callsign in tones after each picture — the \
                                             identification SSTV repeaters and other programs read. \
                                             Adds about 2.5 seconds, and sends nothing at all until \
                                             you have set a callsign.",
                                        )
                                        .changed()
                                        {
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                        ui.separator();
                                        // Dead air before the calibration header, so
                                        // the rig is really on the air by the time
                                        // the VIS code goes out. Here rather than in
                                        // the setup window because it is the same
                                        // kind of per-station trim as the slant
                                        // beside it, and the operator who needs it
                                        // finds out by transmitting.
                                        ui.label(RichText::new("TX lead").size(10.0).weak());
                                        if ui
                                            .add(
                                                egui::DragValue::new(
                                                    &mut self.digi_cfg_edit.sstv_txdelay_ms,
                                                )
                                                .range(0..=3000)
                                                .speed(10.0)
                                                .suffix(" ms"),
                                            )
                                            .on_hover_text(
                                                "Silence sent after keying and before the picture's \
                                                 leader and VIS code. A decoder that misses any of \
                                                 that header shows no picture at all, so this covers \
                                                 the gap between asking a rig for PTT and it really \
                                                 being on the air. 0 for an SDR that keys instantly.",
                                            )
                                            .changed()
                                        {
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                    });
                                }
                            });
                        });
                    ui.add_space(6.0);

                    // LIVE + RECEIVED fill the remaining height of the left column.
                    //
                    // Stacked on a phone rather than side by side: halving 377
                    // points leaves two columns too narrow to show a picture in,
                    // and a received picture is the whole point of the mode.
                    let stack = pane.is_some();
                    let row_h = ui.available_height().max(160.0);
                    let (row_h, live_w, gallery_w) = if stack {
                        let w = ui.available_width();
                        ((row_h / 2.0).max(120.0), w, w)
                    } else {
                        (row_h, live_w, gallery_w)
                    };
                    let lay = if stack {
                        egui::Layout::top_down(egui::Align::Min)
                    } else {
                        egui::Layout::left_to_right(egui::Align::Min)
                    };
                    ui.with_layout(lay, |ui| {
                        // LIVE: the picture currently decoding, shown large.
                        sstv_section(ui, "LIVE", egui::vec2(live_w, row_h), |ui| {
                            ui.centered_and_justified(|ui| {
                                if let Some(tex) = &self.sstv.rx_tex {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .max_height(row_h - 34.0)
                                            .max_width(live_w - 16.0),
                                    );
                                } else {
                                    let msg = if rifp {
                                        // RIFP only paints live from the raw
                                        // raster; anything else appears whole.
                                        "waiting for a picture…"
                                    } else if signal > 0.0008 {
                                        "waiting for a signal…"
                                    } else {
                                        "no / low audio"
                                    };
                                    ui.label(RichText::new(msg).size(11.0).weak());
                                }
                            });
                        });
                        // Draggable vertical divider between LIVE and RECEIVED.
                        if !stack {
                            let hresp =
                                crate::chrome::split_handle(ui, egui::vec2(handle_w, row_h), None);
                            if hresp.dragged() {
                                // Dragging right shrinks the gallery (grows LIVE).
                                let d = hresp.drag_delta().x / left_w.max(1.0);
                                self.view.sstv_gallery_fraction =
                                    (self.view.sstv_gallery_fraction - d).clamp(0.2, 0.6);
                            }
                        }

                        // RECEIVED: narrow multi-column gallery of decoded
                        // pictures, thumbnailed by the engine out of its own
                        // store — so it is the same gallery here, on a browser
                        // tab, and on a client dialled in from anywhere else.
                        sstv_section(ui, "RECEIVED", egui::vec2(gallery_w, row_h), |ui| {
                            if self.sstv.received.is_empty() {
                                let msg = if self.sstv.page_pending {
                                    "Reading the radio's pictures…".to_string()
                                } else if self.sstv.dir.is_empty() {
                                    "Decoded pictures collect here.".to_string()
                                } else {
                                    format!(
                                        "Decoded pictures are saved {} and collect here.",
                                        self.store_where(&self.sstv.dir),
                                    )
                                };
                                ui.label(RichText::new(msg).size(11.0).weak());
                                return;
                            }
                            let thumb = egui::vec2(112.0, 90.0);
                            let shown = self.sstv.received.len() as u32;
                            let older = self.sstv.total.saturating_sub(shown);
                            egui::ScrollArea::vertical()
                                .id_salt("sstv-gallery")
                                .max_height(row_h - 24.0)
                                .auto_shrink([false, false])
                                .show_themed(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                                        for (i, r) in self.sstv.received.iter().enumerate() {
                                            let resp = ui
                                                .add(
                                                    egui::Image::new(&r.thumb)
                                                        .fit_to_exact_size(thumb)
                                                        .corner_radius(2.0)
                                                        .sense(egui::Sense::click()),
                                                )
                                                .on_hover_text(
                                                    "Click to enlarge · right-click to delete",
                                                );
                                            if resp.clicked() {
                                                enlarge = Some(i);
                                            }
                                            // A menu rather than an ✕ on the
                                            // thumbnail: a picture is 112 points
                                            // across in a wrapped grid, and a
                                            // delete target that close to the
                                            // one for opening it would be pressed
                                            // by accident.
                                            resp.context_menu(|ui| {
                                                ui.label(
                                                    RichText::new(shorten(&r.entry.name, 32))
                                                        .size(10.0)
                                                        .weak(),
                                                );
                                                if ui.button("Delete this picture").clicked() {
                                                    delete = Some(r.entry.name.clone());
                                                    ui.close();
                                                }
                                            });
                                        }
                                    });
                                    // The rest of the store is a request away.
                                    // Explicit rather than automatic on scroll:
                                    // a page is a few hundred kilobytes and the
                                    // operator may be on a phone link.
                                    if older > 0 && self.sstv.received.len() >= GALLERY_MAX {
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(format!("{older} older in the store"))
                                                .size(9.5)
                                                .weak(),
                                        );
                                    } else if older > 0 {
                                        ui.add_space(4.0);
                                        let label = if self.sstv.page_pending {
                                            "loading…".to_string()
                                        } else {
                                            format!("{older} older — load more")
                                        };
                                        if crate::chrome::chip(ui, false, label).clicked()
                                            && !self.sstv.page_pending
                                        {
                                            more = true;
                                        }
                                    }
                                });
                        });
                    });
                },
            );

            }

            // Draggable vertical divider between the receive side and the
            // TRANSMIT (send) column — mirrors the FT8 decode/QSO splitter.
            if !self.ui_settings.swl && pane.is_none() {
                let hresp = crate::chrome::split_handle(ui, egui::vec2(handle_w, full_h), None);
                if hresp.dragged() {
                    // Dragging right shrinks the TX column (grows the receive side).
                    let d = hresp.drag_delta().x / avail.x.max(1.0);
                    self.view.sstv_tx_fraction = (self.view.sstv_tx_fraction - d).clamp(0.22, 0.6);
                }
            }

            // ── RIGHT: transmit compositor, full height ──
            if !self.ui_settings.swl && pane.is_none_or(|p| p != 0) {
            ui.allocate_ui(egui::vec2(tx_w, full_h), |ui| {
                sstv_section(ui, "TRANSMIT", egui::vec2(tx_w, full_h), |ui| {
                    // The compositor is a fixed stack — the five slots, the
                    // load buttons, the preview, the message box and the
                    // transmit controls — and none of it can be dropped
                    // without dropping a control. On a screen with less height
                    // than the stack needs it used to run off the bottom edge
                    // and simply not be there (issue #231). Scrolled, the whole
                    // stack stays reachable, and where there is room the bar
                    // never appears.
                    egui::ScrollArea::vertical()
                        .id_salt("sstv-transmit")
                        .auto_shrink([false, false])
                        .show_themed(ui, |ui| {
                        let inner_w = tx_w - 16.0;

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            for i in 0..IMAGE_SLOTS {
                                let sel = self.sstv.selected_slot == i;
                                let size = egui::vec2(70.0, 54.0);
                                let resp = if let Some(tex) =
                                    self.sstv.slot_thumbs.get(i).and_then(|t| t.as_ref())
                                {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .fit_to_exact_size(size)
                                            .corner_radius(2.0)
                                            .sense(egui::Sense::click()),
                                    )
                                } else {
                                    let (rect, resp) =
                                        ui.allocate_exact_size(size, egui::Sense::click());
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        egui::Stroke::new(1.0, crate::theme::gray(70)),
                                        egui::StrokeKind::Inside,
                                    );
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "+",
                                        egui::FontId::proportional(22.0),
                                        crate::theme::gray(110),
                                    );
                                    resp
                                };
                                // Active-tab highlight: a cyan wash + heavier border so
                                // it is obvious which slot the message box targets.
                                if sel {
                                    ui.painter().rect_filled(
                                        resp.rect,
                                        2.0,
                                        Color32::from_rgba_unmultiplied(0x00, 0xd0, 0xf4, 34),
                                    );
                                    ui.painter().rect_stroke(
                                        resp.rect,
                                        2.0,
                                        egui::Stroke::new(2.5, crate::theme::CYAN()),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                // Slot number badge (1..5), like a tab label.
                                let badge = egui::Rect::from_min_size(
                                    resp.rect.left_top() + egui::vec2(2.0, 2.0),
                                    egui::vec2(15.0, 13.0),
                                );
                                ui.painter().rect_filled(badge, 2.0, Color32::from_black_alpha(150));
                                ui.painter().text(
                                    badge.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{}", i + 1),
                                    egui::FontId::proportional(10.0),
                                    if sel { crate::theme::CYAN() } else { crate::theme::gray(170) },
                                );
                                let resp = resp.on_hover_text(
                                    "Click to edit this slot's message · double-click to load an image",
                                );
                                if resp.double_clicked() {
                                    self.sstv.pick_target = Some(i);
                                    pick_image(self.sstv.inbox.clone());
                                } else if resp.clicked() && !sel {
                                    // Hand the slot we are leaving back to the
                                    // engine before moving on, so an edit is never
                                    // lost to a click.
                                    cmds.extend(crate::sstv::claim_message(
                                        &mut self.sstv.msg_edit,
                                        &self.sstv.presets,
                                        i,
                                    ));
                                    self.sstv.selected_slot = i;
                                    self.sstv.preview_dirty = true;
                                }
                            }
                        });
                        ui.add_space(5.0);

                        // Load/replace/clear the active slot's picture.
                        ui.horizontal(|ui| {
                            let sel = self.sstv.selected_slot;
                            let has_img = self.sstv.presets.slot(sel).has_picture();
                            let label = if has_img { "Change image…" } else { "Load image…" };
                            if crate::chrome::chip(ui, false, label).clicked() {
                                self.sstv.pick_target = Some(sel);
                                pick_image(self.sstv.inbox.clone());
                            }
                            if has_img
                                && crate::chrome::chip(ui, false, "Clear")
                                    .on_hover_text("Empty this slot's picture (the message stays)")
                                    .clicked()
                            {
                                cmds.push(Command::ImageClearSlot(sel as u8));
                            }
                            // The banner belongs to the station, not to the slot:
                            // one editor, reached from whichever slot is in front.
                            if crate::chrome::chip(ui, self.sstv.banner_open, "Banner…")
                                .on_hover_text(
                                    "What is printed across the top of every picture this station \
                                     sends",
                                )
                                .clicked()
                            {
                                self.sstv.banner_open = !self.sstv.banner_open;
                            }
                        });
                        if let Some(err) = &self.sstv.pick_error {
                            ui.label(RichText::new(err).size(10.0).color(crate::theme::YELLOW()));
                        }
                        ui.add_space(6.0);

                        // Preview gets a capped share of the height; the message box
                        // grows to fill whatever's left above the buttons.
                        let btn_h = 42.0;
                        let gap = 6.0;
                        let preview_h = (ui.available_height() * 0.45).clamp(80.0, 260.0);
                        egui::Frame::new()
                            .fill(crate::theme::gray(6))
                            .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
                            .inner_margin(2.0)
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(inner_w, preview_h));
                                ui.set_max_size(egui::vec2(inner_w, preview_h));
                                ui.centered_and_justified(|ui| {
                                    if let Some(tex) = &self.sstv.preview_tex {
                                        ui.add(
                                            egui::Image::new(tex)
                                                .max_height(preview_h - 4.0)
                                                .max_width(inner_w - 4.0),
                                        );
                                    } else {
                                        let waiting =
                                            self.sstv.presets.slot(self.sstv.selected_slot).has_picture();
                                        ui.label(
                                            RichText::new(if waiting {
                                                "Fetching this slot's picture…"
                                            } else {
                                                "Load an image into this slot →"
                                            })
                                            .size(11.0)
                                            .weak(),
                                        );
                                    }
                                });
                            });
                        ui.add_space(gap);

                        // Overlay message for the active slot — fills the height
                        // above the buttons. The engine owns the text; this box
                        // claims the slot while the cursor is in it and hands it
                        // back when focus leaves, so an edit made on another screen
                        // still reaches every slot but this one.
                        let sel = self.sstv.selected_slot;
                        let msg_h = (ui.available_height() - btn_h - gap).max(48.0);
                        let mut buf = self.sstv.current_message().to_string();
                        let resp = ui
                            .push_id(sel, |ui| {
                                crate::chrome::field_sized(ui, egui::vec2(inner_w, msg_h),
                                    egui::TextEdit::multiline(&mut buf)
                                        .hint_text("Drawn on this slot's image"),
                                )
                            })
                            .inner;
                        if resp.changed() {
                            self.sstv.msg_edit = Some((sel, buf));
                            self.sstv.preview_dirty = true;
                        } else if resp.gained_focus() {
                            cmds.extend(crate::sstv::claim_message(
                                &mut self.sstv.msg_edit,
                                &self.sstv.presets,
                                sel,
                            ));
                        }
                        if resp.lost_focus() {
                            cmds.extend(crate::sstv::commit_message(
                                &mut self.sstv.msg_edit,
                                &self.sstv.presets,
                            ));
                        }
                        ui.add_space(gap);

                        // Large cut-corner TX / ABORT buttons.
                        ui.horizontal(|ui| {
                            // The source has to have arrived as well as be stored:
                            // pressing TX while it is still on its way would compose
                            // nothing and look like the button was broken. The radio
                            // has to have a transmitter too — composing a picture is
                            // worth doing on a receiver, sending it is not.
                            let tx_ok = self.tx_capable();
                            let can_tx = tx_ok
                                && self
                                    .sstv
                                    .slots
                                    .get(self.sstv.selected_slot)
                                    .is_some_and(|s| s.is_some())
                                && !tx_active;
                            let tx = ui
                                .add_enabled_ui(can_tx, |ui| {
                                    rx_only_hint(
                                        crate::chrome::chip_accent(
                                            ui,
                                            can_tx,
                                            RichText::new("   TX   ").size(16.0).strong(),
                                            crate::theme::ALERT(),
                                            Color32::WHITE,
                                        ),
                                        tx_ok,
                                    )
                                })
                                .inner;
                            if tx.clicked() {
                                // Compose before committing: what goes on the air is
                                // what is on screen, including an edit the operator
                                // never clicked out of. The commit follows so the
                                // engine stores it too.
                                let png = self.sstv.compose_png(dims);
                                cmds.extend(crate::sstv::commit_message(
                                    &mut self.sstv.msg_edit,
                                    &self.sstv.presets,
                                ));
                                if let Some(png) = png {
                                    cmds.push(if rifp {
                                        Command::RifpTx { png }
                                    } else {
                                        Command::SstvTx { mode: self.sstv.tx_mode, png }
                                    });
                                }
                            }
                            ui.add_space(8.0);
                            let abort = ui
                                .add_enabled_ui(tx_active, |ui| {
                                    crate::chrome::chip(
                                        ui,
                                        false,
                                        RichText::new(" ABORT TX ").size(15.0).strong(),
                                    )
                                })
                                .inner;
                            if abort.clicked() {
                                cmds.push(Command::DigiAbortTx);
                            }
                        });
                        });
                });
            });
            }

            if let Some(i) = enlarge {
                self.sstv.enlarged = Some(i);
            }
            if more {
                self.sstv.page_pending = true;
                cmds.push(Command::ImageList {
                    kind: ImageKind::Sstv,
                    offset: self.sstv.received.len() as u32,
                    count: sdroxide_types::IMAGE_PAGE_MAX,
                });
            }
            // The gallery entry goes when the engine says the file has, not
            // here: the store is on the radio's machine, and a thumbnail that
            // vanished from a delete that then failed would be a lie.
            if let Some(name) = delete {
                cmds.push(Command::ImageDelete { kind: ImageKind::Sstv, name });
            }
        });

        self.banner_window(&ctx, cmds);

        // Enlarged view of a clicked received image (overlay window).
        if let Some(idx) = self.sstv.enlarged {
            let mut open = true;
            let mut save = false;
            let mut pressed_delete = false;
            if let Some(r) = self.sstv.received.get(idx) {
                // The full-size picture lives on the radio. Ask for it the
                // moment one is opened, and show the thumbnail scaled up in the
                // meantime rather than an empty window. Asked once per picture,
                // whatever the answer — a fetch that fails must not become a
                // request every frame.
                if r.full.is_none() && self.sstv.full_asked.as_deref() != Some(&r.entry.name) {
                    self.sstv.full_asked = Some(r.entry.name.clone());
                    self.sstv.full_gone = false;
                    cmds.push(Command::ImageGet {
                        kind: ImageKind::Sstv,
                        name: r.entry.name.clone(),
                    });
                }
                // A picture can be stored before the first listing has come
                // back, so the directory is not always known to name.
                let del_hint = if self.sstv.dir.is_empty() {
                    "Delete this picture from the store".to_string()
                } else {
                    format!("Delete this picture {}", self.store_where(&self.sstv.dir))
                };
                let r = &self.sstv.received[idx];
                let savable = self.sstv.full_png.as_ref().is_some_and(|(n, _)| *n == r.entry.name);
                let armed = self.sstv.confirm_delete.as_deref() == Some(&r.entry.name);
                egui::Window::new("Received image")
                    .id(crate::layout::salted_id(&ctx, "Received image"))
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(true)
                    .default_size([
                        crate::layout::window_w(&ctx, 660.0),
                        crate::layout::window_h(&ctx, 528.0),
                    ])
                    .frame(crate::chrome::window_frame())
                    .show(&ctx, |ui| {
                        crate::chrome::window_body_bg(ui);
                        // Scale up to fill the window width (preserving aspect).
                        // A thumbnail stands in until the real one arrives, at
                        // the size the real one will be, so nothing jumps.
                        let tex = r.full.as_ref().unwrap_or(&r.thumb);
                        let native =
                            egui::vec2(f32::from(r.entry.width), f32::from(r.entry.height));
                        let native = if native.x > 0.0 { native } else { tex.size_vec2() };
                        let avail_w = ui.available_width().min(1000.0);
                        let scale = (avail_w / native.x.max(1.0)).clamp(1.0, 4.0);
                        ui.add(egui::Image::new(tex).fit_to_exact_size(native * scale));
                        ui.horizontal(|ui| {
                            if self.sstv.full_gone {
                                ui.label(
                                    RichText::new("no longer in the store")
                                        .size(10.0)
                                        .color(crate::theme::YELLOW()),
                                );
                            } else if r.full.is_none() {
                                ui.label(RichText::new("loading full size…").size(10.0).weak());
                            } else if savable
                                && crate::chrome::chip(ui, false, "Save picture…")
                                    .on_hover_text("Save a copy on this computer")
                                    .clicked()
                            {
                                save = true;
                            }
                            // Two presses, the second one red: the file goes
                            // from the radio's disk and there is nothing to
                            // undo it with.
                            let del = if armed {
                                crate::chrome::chip_accent(
                                    ui,
                                    true,
                                    "Delete — sure?",
                                    crate::theme::PINK(),
                                    crate::theme::INK_ON_CYAN(),
                                )
                                .on_hover_text("Click again to delete it for good")
                            } else {
                                crate::chrome::chip(ui, false, "Delete…").on_hover_text(&del_hint)
                            };
                            if del.clicked() {
                                pressed_delete = true;
                            }
                            ui.label(
                                RichText::new(format!(
                                    "{} · {}×{}",
                                    r.entry.name, r.entry.width, r.entry.height
                                ))
                                .size(10.0)
                                .weak(),
                            );
                        });
                        // RIFP knows where a picture came from and how it was
                        // carried; SSTV knows none of that, and says nothing.
                        if let Some(m) = &r.entry.rifp {
                            ui.add_space(4.0);
                            let from = m.sender.as_deref().unwrap_or("unidentified");
                            ui.label(
                                RichText::new(format!(
                                    "{from} · {} · {}×{} {}-bit · {} / {} · {} octets in {} chunks \
                                     ({} first pass) · session {}",
                                    m.filename,
                                    m.width,
                                    m.height,
                                    m.bits_per_pixel,
                                    m.media_type,
                                    m.content_encoding,
                                    m.encoded_size,
                                    m.chunk_count,
                                    m.chunks_first_pass,
                                    m.session,
                                ))
                                .size(10.5)
                                .weak(),
                            );
                            if let Some(hint) = &m.hint {
                                ui.label(RichText::new(hint).size(11.0).italics());
                            }
                        }
                    });
            } else {
                open = false;
            }
            if save {
                if let Some((name, png)) = &self.sstv.full_png {
                    crate::download::save_as(name, png, crate::download::Mime::Png);
                }
            }
            // First press arms the chip, second sends it. The gallery entry
            // stays until the engine confirms the file is gone.
            if pressed_delete {
                if let Some(name) = self.sstv.received.get(idx).map(|r| r.entry.name.clone()) {
                    if self.sstv.confirm_delete.as_deref() == Some(name.as_str()) {
                        self.sstv.confirm_delete = None;
                        cmds.push(Command::ImageDelete { kind: ImageKind::Sstv, name });
                    } else {
                        self.sstv.confirm_delete = Some(name);
                    }
                }
            }
            if !open {
                self.sstv.enlarged = None;
                self.sstv.full_asked = None;
                self.sstv.full_gone = false;
                self.sstv.confirm_delete = None;
            }
        }
    }

    /// The editor for the banner drawn across the top of every transmitted
    /// picture: the two texts, the two colours, and how tall the strip is.
    ///
    /// A window rather than another row in the transmit column, because this is
    /// set once for the station and then left alone, and the column it would
    /// have gone in is already the narrow half of a split panel.
    fn banner_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.sstv.banner_open {
            return;
        }
        let seeded = self.digi_cfg_seeded;
        let mut open = true;
        let mut changed = false;
        egui::Window::new("Image banner")
            .id(crate::layout::salted_id(ctx, "Image banner"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 420.0))
            .frame(crate::chrome::window_frame())
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.label(
                    RichText::new(
                        "Drawn into every picture this station transmits, over the slot's own \
                         message.",
                    )
                    .size(10.5)
                    .weak(),
                );
                ui.add_space(6.0);
                let cfg = &mut self.digi_cfg_edit;
                ui.add_enabled_ui(seeded, |ui| {
                    if crate::chrome::checkbox(ui, &mut cfg.sstv_banner, "Draw the banner")
                        .changed()
                    {
                        changed = true;
                    }
                    ui.add_space(4.0);
                    ui.add_enabled_ui(cfg.sstv_banner, |ui| {
                        // The placeholder list is the whole documentation for
                        // these two fields, so it hangs off both of them.
                        let hint: String = std::iter::once(
                            "Substituted when the picture is composed:".to_string(),
                        )
                        .chain(
                            crate::sstv::BANNER_PLACEHOLDERS
                                .iter()
                                .map(|(k, what)| format!("  {k} — {what}")),
                        )
                        .collect::<Vec<_>>()
                        .join("\n");
                        egui::Grid::new("sstv-banner-grid")
                            .num_columns(2)
                            .spacing([10.0, 7.0])
                            .show(ui, |ui| {
                                ui.label("Top left");
                                let resp = crate::chrome::field(
                                    ui,
                                    egui::TextEdit::singleline(&mut cfg.sstv_banner_left)
                                        .desired_width(240.0)
                                        .hint_text("{call}"),
                                )
                                .on_hover_text(&hint);
                                // On focus loss, like the RIFP caption: this is
                                // persisted engine-side and a config write per
                                // keystroke would be absurd. The preview still
                                // follows every keystroke, because it is
                                // recomposed from `digi_cfg_edit` and not from
                                // what the engine has heard about.
                                if resp.lost_focus() && resp.changed() {
                                    changed = true;
                                }
                                ui.end_row();

                                ui.label("Top right");
                                let resp = crate::chrome::field(
                                    ui,
                                    egui::TextEdit::singleline(&mut cfg.sstv_banner_right)
                                        .desired_width(240.0)
                                        .hint_text("SDRoxide v{version}"),
                                )
                                .on_hover_text(&hint);
                                if resp.lost_focus() && resp.changed() {
                                    changed = true;
                                }
                                ui.end_row();

                                ui.label("Colours");
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgb(&mut cfg.sstv_banner_fill)
                                        .on_hover_text(
                                            "The strip, at its top edge — it fades to black at \
                                             the bottom",
                                        )
                                        .changed();
                                    ui.label(RichText::new("strip").size(10.5).weak());
                                    ui.add_space(8.0);
                                    changed |= ui
                                        .color_edit_button_srgb(&mut cfg.sstv_banner_ink)
                                        .on_hover_text("Both texts")
                                        .changed();
                                    ui.label(RichText::new("text").size(10.5).weak());
                                });
                                ui.end_row();

                                ui.label("Height").on_hover_text(
                                    "How tall the strip is, in pixels of the transmitted \
                                     picture. The text is sized from it — an SSTV frame is only \
                                     320 pixels wide, so a taller banner is what makes it \
                                     readable on the far end.",
                                );
                                ui.spacing_mut().slider_width = 180.0;
                                let resp = crate::chrome::slider(
                                    ui,
                                    egui::Slider::new(&mut cfg.sstv_banner_height, 8..=64)
                                        .suffix(" px"),
                                );
                                changed |=
                                    resp.drag_stopped() || (resp.changed() && !resp.dragged());
                                ui.end_row();
                            });
                    });
                    ui.add_space(6.0);
                    if ui
                        .button("Reset")
                        .on_hover_text(
                            "Back to the callsign on the left and the version on the right",
                        )
                        .clicked()
                    {
                        let d = sdroxide_types::DigiConfig::default();
                        cfg.sstv_banner = d.sstv_banner;
                        cfg.sstv_banner_left = d.sstv_banner_left;
                        cfg.sstv_banner_right = d.sstv_banner_right;
                        cfg.sstv_banner_fill = d.sstv_banner_fill;
                        cfg.sstv_banner_ink = d.sstv_banner_ink;
                        cfg.sstv_banner_height = d.sstv_banner_height;
                        changed = true;
                    }
                    if !seeded {
                        ui.label(
                            RichText::new(
                                "Waiting for the station's digital-mode settings to load…",
                            )
                            .size(10.0)
                            .weak(),
                        );
                    }
                });
            });
        if changed {
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
        self.sstv.banner_open = open;
    }

    /// The RIFP half of the image panel's control strip: profile, picture size
    /// and encoding, robustness, the transfer readout, and the sessions being
    /// reassembled.
    fn rifp_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let st = self.sstv.rifp.clone();
        let seeded = self.digi_cfg_seeded;
        let mut changed = false;

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("RIFP").size(12.0).strong().color(crate::theme::CYAN()));
            // Outside the enabled scope: which frequency to sit on has nothing
            // to do with whether the operator's digi config has loaded yet.
            self.digi_freq_chip(ui, cmds);
            ui.add_enabled_ui(seeded, |ui| {
                for p in RifpProfile::ALL {
                    let active = self.digi_cfg_edit.rifp_profile == p;
                    if crate::chrome::chip(ui, active, p.label())
                        .on_hover_text(format!(
                            "{} — {:.0} baud CPFSK, ±{:.0} Hz, {:.0} kHz occupied bandwidth",
                            p.name(),
                            p.symbol_rate(),
                            p.deviation_hz(),
                            p.bandwidth_hz() / 1000.0,
                        ))
                        .clicked()
                        && !active
                    {
                        self.digi_cfg_edit.rifp_profile = p;
                        changed = true;
                    }
                }
                ui.separator();
                ui.label(RichText::new("Size").size(10.0).weak());
                for s in RifpSize::ALL {
                    let active = self.digi_cfg_edit.rifp_size == s;
                    if crate::chrome::chip(ui, active, s.label()).clicked() && !active {
                        self.digi_cfg_edit.rifp_size = s;
                        self.sstv.preview_dirty = true;
                        changed = true;
                    }
                }
            });
        });
        ui.add_space(4.0);

        // The bandwidth warning, and a jump to the calling frequency. RIFP
        // itself is band-agnostic; what is legal is not.
        let dial = self.state.rx_freq_hz();
        ui.horizontal_wrapped(|ui| {
            let profile = self.digi_cfg_edit.rifp_profile;
            if profile.fits_at(dial) {
                ui.label(
                    RichText::new(format!(
                        "{} · ~{:.0} kHz occupied · dial is the channel centre",
                        profile.name(),
                        profile.bandwidth_hz() / 1000.0,
                    ))
                    .size(10.5)
                    .weak(),
                );
            } else {
                ui.label(
                    RichText::new(format!(
                        "⚠ {} occupies ~{:.0} kHz — too wide for a narrow-band segment",
                        profile.name(),
                        profile.bandwidth_hz() / 1000.0,
                    ))
                    .size(10.5)
                    .strong()
                    .color(crate::theme::ALERT()),
                )
                .on_hover_text(format!(
                    "RIFP assigns no frequency, and sdroxide will transmit it wherever you tune. \
                     A {:.0} kHz channel only fits where wideband or FM operation is allowed — \
                     {} — and not in a narrow-band segment, least of all on HF. Even inside those \
                     your own licence conditions may be narrower. You are the operator; check \
                     your own rules.",
                    profile.bandwidth_hz() / 1000.0,
                    profile.wide_segments_text(),
                ));
            }
            if (dial - sdroxide_types::RIFP_CALLING_HZ).abs() > 1.0
                && crate::chrome::chip(ui, false, "433.920")
                    .on_hover_text("The calling frequency the draft names")
                    .clicked()
            {
                cmds.push(Command::SetVfo {
                    vfo: self.state.active_vfo,
                    hz: sdroxide_types::RIFP_CALLING_HZ,
                });
            }
        });
        ui.add_space(5.0);

        // Encoding and depth: what the picture is turned into before framing.
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(seeded, |ui| {
                ui.label(RichText::new("Encode").size(10.0).weak()).on_hover_text(
                    "How the picture is encoded into the object RIFP carries. Auto tries each and \
                 sends the smallest.",
                );
                for e in RifpEncoding::TX_MENU {
                    let active = self.digi_cfg_edit.rifp_encoding == e;
                    let hover = match e.manifest_pair() {
                        Some((mt, ce)) => format!("{mt} / {ce}"),
                        None => "Try every encoding, send the smallest (never lossy)".into(),
                    };
                    if crate::chrome::chip(ui, active, e.label()).on_hover_text(hover).clicked()
                        && !active
                    {
                        self.digi_cfg_edit.rifp_encoding = e;
                        changed = true;
                    }
                }
                ui.separator();
                ui.label(RichText::new("Gray").size(10.0).weak()).on_hover_text(
                "Grayscale depth. RIFP's raster is grayscale by definition — colour has no place \
                 in its manifest.",
            );
                for bits in [1u8, 2, 4, 8] {
                    let active = self.digi_cfg_edit.rifp_bits_per_pixel == bits;
                    if crate::chrome::chip(ui, active, &format!("{bits}b")).clicked() && !active {
                        self.digi_cfg_edit.rifp_bits_per_pixel = bits;
                        changed = true;
                    }
                }
                let mut dither = self.digi_cfg_edit.rifp_dither;
                if crate::chrome::chip(ui, dither, "Dither")
                    .on_hover_text("Diffuse quantisation error — worth it below 8 bits")
                    .clicked()
                {
                    dither = !dither;
                    self.digi_cfg_edit.rifp_dither = dither;
                    changed = true;
                }
            });
        });
        ui.add_space(5.0);

        // The content hint: a caption carried in the manifest itself, so it
        // reaches a receiver as text rather than as pixels they have to read.
        // Distinct from the slot message, which is drawn into the picture.
        ui.horizontal(|ui| {
            ui.add_enabled_ui(seeded, |ui| {
                ui.label(RichText::new("Caption").size(10.0).weak()).on_hover_text(
                    "Sent in the manifest as the content hint, and shown under the picture by \
                     receivers. Travels as text — it is not drawn into the image.",
                );
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.digi_cfg_edit.rifp_content_hint)
                        .desired_width(f32::INFINITY)
                        .hint_text("What this picture is"),
                );
                // On focus loss rather than per keystroke: this is persisted
                // engine-side, and a config write per character is absurd.
                if resp.lost_focus() && resp.changed() {
                    changed = true;
                }
            });
        });
        ui.add_space(5.0);

        // Robustness: RIFP has no repair requests, so repetition is the only
        // recovery there is.
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(seeded, |ui| {
                ui.label(RichText::new("Repeat data").size(10.0).weak()).on_hover_text(
                    "Send every data frame this many times. RIFP is one-way with no repair \
                     requests, so this is the only recovery a receiver gets.",
                );
                ui.spacing_mut().slider_width = 90.0;
                changed |= crate::chrome::slider(
                    ui,
                    egui::Slider::new(&mut self.digi_cfg_edit.rifp_data_repeats, 1..=4),
                )
                .drag_stopped();
                ui.label(RichText::new("Chunk").size(10.0).weak())
                    .on_hover_text("Payload octets per data frame (the profile recommends 192)");
                changed |= crate::chrome::slider(
                    ui,
                    egui::Slider::new(&mut self.digi_cfg_edit.rifp_chunk_size, 32..=1024)
                        .step_by(16.0),
                )
                .drag_stopped();
            });
            ui.separator();
            if st.tx_active {
                ui.label(
                    RichText::new(format!(
                        "● TX frame {}/{} · {} s left",
                        st.tx_frame, st.tx_frames, st.tx_remaining_s
                    ))
                    .size(11.0)
                    .strong()
                    .color(crate::theme::ALERT()),
                );
            }
            if let Some(enc) = st.tx_encoding {
                ui.label(
                    RichText::new(format!("sent as {} · {} octets", enc.label(), st.tx_bytes))
                        .size(10.0)
                        .weak(),
                );
            }
        });
        ui.add_space(5.0);

        // Counters and the sessions being reassembled.
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "frames {} · bad {} · pictures {}",
                    st.rx_frames, st.rx_bad_frames, st.rx_objects
                ))
                .size(10.0)
                .weak(),
            )
            .on_hover_text("Valid frames, frames that failed CRC, and complete verified pictures");
            if st.sessions.is_empty() {
                ui.label(RichText::new("no transfer in progress").size(10.0).weak());
            }
            for s in &st.sessions {
                ui.separator();
                let from = s.sender.as_deref().unwrap_or_else(|| shorten(&s.session, 8));
                let label = if s.total > 0 {
                    format!("{from} {}/{}", s.have, s.total)
                } else {
                    format!("{from} {}", s.have)
                };
                let colour =
                    if s.have_manifest { crate::theme::GREEN() } else { crate::theme::YELLOW() };
                ui.label(RichText::new(label).size(10.5).strong().color(colour)).on_hover_text(
                    if s.have_manifest {
                        format!("session {} · idle {} s", s.session, s.idle_s)
                    } else {
                        format!(
                            "session {} · chunks held, still waiting for the manifest · idle {} s",
                            s.session, s.idle_s
                        )
                    },
                );
                rifp_chunk_map(ui, s);
                if crate::chrome::chip(ui, false, "✕")
                    .on_hover_text("Forget this incomplete transfer")
                    .clicked()
                {
                    cmds.push(Command::RifpDropSession(s.session.clone()));
                }
            }
        });
        if let Some(err) = &st.last_error {
            ui.label(RichText::new(err).size(10.0).color(crate::theme::YELLOW()));
        }
        if changed && seeded {
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
    }

    /// How to name a directory the engine reported.
    ///
    /// A path is only a path if the operator can walk to it. Driving a radio on
    /// another machine, it names a directory over there, and saying so is the
    /// difference between a useful hint and a wrong one.
    pub(in crate::app) fn store_where(&self, dir: &str) -> String {
        if self.ctrl.engine_is_remote() {
            format!("on the radio, in {dir}")
        } else {
            format!("in {dir}")
        }
    }
}
