//! The engine's picture stores: the ten transmit presets an operator composes
//! from, and the directories of received pictures a gallery is a window onto.
//!
//! All of it used to be client state, which meant a browser tab had no presets
//! and no gallery at all, and a `--connect` client kept its own copies of both
//! while driving an engine that wrote to a different disk. The files are on the
//! machine the radio is plugged into, so this is where they belong.
//!
//! Three things live here, deliberately apart:
//!
//! * pure image work ([`normalise`], [`thumbnail`], [`page`]) with no I/O, so
//!   the bounds and the ordering can be tested without a filesystem;
//! * [`ImagePresetStore`], the ten slots and their messages, small enough to
//!   sit on the engine thread;
//! * [`GalleryWorker`], everything that walks a directory or decodes a big
//!   picture — off the engine thread, because that loop is the audio loop and a
//!   two-megapixel chart takes long enough to decode to drop a block.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, Sender};
use image::ImageReader;
use tracing::{info, warn};

use sdroxide_types::{
    IMAGE_PAGE_MAX, IMAGE_SLOT_THUMB_EDGE, IMAGE_SLOTS, IMAGE_SOURCE_MAX_EDGE, IMAGE_THUMB_EDGE,
    ImageEntry, ImageKind, ImageListing, ImagePresets, ImageSlotInfo, RifpMeta,
};

/// Widest and tallest picture the decoder is allowed to believe in.
///
/// The bytes arrive over a socket that nothing authenticates, and a
/// forty-kilobyte PNG is entitled to declare itself sixty thousand pixels
/// square. `image::load_from_memory` would believe it and ask the allocator for
/// fourteen gigabytes, so nothing in this module uses it.
const DECODE_MAX_EDGE: u32 = 20_000;
/// Ceiling on what one decode may allocate, as a second line of defence for a
/// picture that is within the edge limits but still absurd.
const DECODE_MAX_ALLOC: u64 = 256 << 20;

// ── Pure image work ──

/// Read arbitrary picture bytes with the decoder held on a short leash.
fn read_bounded(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(DECODE_MAX_EDGE);
    limits.max_image_height = Some(DECODE_MAX_EDGE);
    limits.max_alloc = Some(DECODE_MAX_ALLOC);
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("unreadable: {e}"))?;
    reader.limits(limits);
    reader.decode().map_err(|e| format!("not a picture we can read: {e}"))
}

/// Encode an RGB image as a PNG.
fn encode_png(img: &image::RgbImage) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("encoding failed: {e}"))?;
    Ok(buf.into_inner())
}

/// Scale so the longest edge is at most `max_edge`, preserving the aspect.
/// A picture already inside the bound is returned untouched — blowing one up
/// costs bytes and adds no detail.
fn fit(img: image::DynamicImage, max_edge: u16) -> image::RgbImage {
    let (w, h) = (img.width(), img.height());
    let max = u32::from(max_edge).max(1);
    if w <= max && h <= max {
        return img.to_rgb8();
    }
    let scale = f64::from(max) / f64::from(w.max(h));
    let nw = ((f64::from(w) * scale).round() as u32).max(1);
    let nh = ((f64::from(h) * scale).round() as u32).max(1);
    image::imageops::resize(&img.to_rgb8(), nw, nh, image::imageops::FilterType::CatmullRom)
}

/// Decode whatever the operator picked and re-encode it as a PNG whose longest
/// edge is at most `max_edge`.
///
/// The store must not hold a phone camera's forty megapixels, and — more
/// importantly — every client has to compose from byte-identical pixels, or two
/// operators looking at the same preset would be sending different pictures.
pub fn normalise(bytes: &[u8], max_edge: u16) -> Result<(Vec<u8>, u16, u16), String> {
    if bytes.is_empty() {
        return Err("empty file".into());
    }
    let img = fit(read_bounded(bytes)?, max_edge);
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err("picture has no pixels".into());
    }
    Ok((encode_png(&img)?, w as u16, h as u16))
}

/// A PNG thumbnail of a stored picture, longest edge at most `max_edge`.
///
/// Same limits as an upload, and for a related reason: a file in the store is
/// only ever as trustworthy as whoever can write to that directory.
pub fn thumbnail(png: &[u8], max_edge: u16) -> Result<(Vec<u8>, u16, u16), String> {
    let img = read_bounded(png)?;
    let (w, h) = (img.width() as u16, img.height() as u16);
    let small = fit(img, max_edge);
    Ok((encode_png(&small)?, w, h))
}

/// A file in a received store, before it is turned into an [`ImageEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw {
    pub name: String,
    pub unix: i64,
    pub bytes: u64,
    pub path: PathBuf,
}

/// Order a store newest first and take one page of it, reporting the whole size.
///
/// Split out from the directory walk so the ordering and the paging can be
/// tested without a disk. The rule is the one the chart gallery already used
/// client-side — time first, name as the tiebreak — so what the operator is
/// used to seeing does not change.
pub fn page(mut all: Vec<Raw>, offset: u32, count: u32) -> (Vec<Raw>, u32) {
    all.sort_by(|a, b| b.unix.cmp(&a.unix).then_with(|| b.name.cmp(&a.name)));
    let total = all.len() as u32;
    let start = offset.min(total) as usize;
    let take = count.min(IMAGE_PAGE_MAX) as usize;
    (all.into_iter().skip(start).take(take).collect(), total)
}

/// Every directory a kind is read from, the one it is written to first.
fn read_dirs(kind: ImageKind) -> Vec<PathBuf> {
    match kind {
        ImageKind::Sstv => sdroxide_config::image_rx_dir("sstv").into_iter().collect(),
        // The legacy directory is read and never written. A collection saved
        // before charts moved to the pictures directory has to stay visible, or
        // the move looks like data loss to the person it happened to.
        ImageKind::Wefax => sdroxide_config::wefax_rx_dir()
            .into_iter()
            .chain(sdroxide_config::wefax_legacy_rx_dir())
            .collect(),
    }
}

/// Turn a client-supplied name into a path inside the store, or refuse it.
///
/// Two checks, because neither is enough alone. The name is sanitised (no
/// separators, no `..`, `.png` only) so it cannot describe somewhere else; and
/// the resolved file is then required to sit directly in the store directory,
/// which is what catches a symlink someone dropped in there.
pub fn resolve(kind: ImageKind, name: &str) -> Option<PathBuf> {
    let name = sdroxide_types::safe_name(name)?;
    read_dirs(kind).into_iter().find_map(|dir| resolve_in(&dir, name))
}

/// Every store directory holding a file by this name, under the same two checks.
///
/// [`resolve`] takes the first, which is right for reading: a listing shows a
/// name once and the current directory wins. Deleting has to take them all, or a
/// chart present in both the pictures directory and the legacy one would come
/// straight back on the next listing, out of the copy nobody can see.
fn resolve_every(kind: ImageKind, name: &str) -> Vec<PathBuf> {
    resolve_every_in(&read_dirs(kind), name)
}

/// The [`resolve_every`] search over a given list of directories, so a delete
/// can be tested against temporary ones rather than the operator's collection.
fn resolve_every_in(dirs: &[PathBuf], name: &str) -> Vec<PathBuf> {
    let Some(name) = sdroxide_types::safe_name(name) else { return Vec::new() };
    dirs.iter().filter_map(|dir| resolve_in(dir, name)).collect()
}

/// Unlink every path, reporting the last failure. Split out from the worker for
/// the same reason as [`resolve_every_in`]: this is the one place in the program
/// where a mistake costs something that cannot be got back.
fn remove_all(paths: &[PathBuf]) -> Option<String> {
    let mut failed = None;
    for path in paths {
        if let Err(e) = std::fs::remove_file(path) {
            warn!(path = %path.display(), "deleting stored picture: {e}");
            failed = Some(e.to_string());
        }
    }
    failed
}

/// The [`resolve`] check against one directory, so it can be tested against a
/// real temporary one rather than the operator's config directory.
fn resolve_in(dir: &Path, name: &str) -> Option<PathBuf> {
    let name = sdroxide_types::safe_name(name)?;
    let real = dir.join(name).canonicalize().ok()?;
    let root = dir.canonicalize().ok()?;
    (real.parent() == Some(root.as_path()) && real.is_file()).then_some(real)
}

/// Walk one directory for pictures this program can hand back.
///
/// A file whose name [`sdroxide_types::safe_name`] refuses simply does not
/// appear — better than listing a name that cannot then be fetched.
fn scan(dir: &Path, kind: ImageKind) -> Vec<Raw> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_str()?.to_string();
            sdroxide_types::safe_name(&name)?;
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            // The name is the only timestamp either store keeps. A file this
            // program did not write falls back to when it was last touched,
            // which at least sorts it somewhere honest.
            let unix = sdroxide_types::received_at(kind, &name).unwrap_or_else(|| {
                meta.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs() as i64)
            });
            Some(Raw { name, unix, bytes: meta.len(), path })
        })
        .collect()
}

/// Everything in a kind's stores, de-duplicated by name so the current
/// directory always wins and a name is never ambiguous.
fn scan_all(kind: ImageKind) -> (Vec<Raw>, String) {
    let dirs = read_dirs(kind);
    let dir_label = dirs.first().map(|d| d.display().to_string()).unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in &dirs {
        for raw in scan(dir, kind) {
            if seen.insert(raw.name.clone()) {
                out.push(raw);
            }
        }
    }
    (out, dir_label)
}

/// Build the listing entry for one stored picture: its thumbnail and its size.
fn entry_for(kind: ImageKind, raw: &Raw, rifp: Option<RifpMeta>) -> Option<ImageEntry> {
    let bytes = std::fs::read(&raw.path).ok()?;
    let (thumb, width, height) = match thumbnail(&bytes, IMAGE_THUMB_EDGE) {
        Ok(t) => t,
        Err(e) => {
            warn!(path = %raw.path.display(), "unreadable picture in store: {e}");
            return None;
        }
    };
    Some(ImageEntry {
        kind,
        name: raw.name.clone(),
        unix: raw.unix,
        width,
        height,
        bytes: raw.bytes,
        thumb,
        rifp,
    })
}

// ── The transmit presets ──

#[derive(Default, Clone)]
struct Slot {
    /// The stored source, already normalised. Empty when the slot is empty.
    png: Vec<u8>,
    w: u16,
    h: u16,
    version: u32,
    thumb: Vec<u8>,
}

/// The ten transmit presets: a source picture and an overlay message each.
///
/// Only the source is kept. The crop, the header strip and the text are
/// composited client-side, so the panel can redraw a preview on every keystroke
/// without a round trip, and the finished picture still reaches the engine as
/// an ordinary `SstvTx` / `RifpTx`.
pub struct ImagePresetStore {
    slots: Vec<Slot>,
    messages: Vec<String>,
    /// Where `slot{i}.png` lives. `None` when the config directory could not be
    /// resolved — the presets then work for the session and store nothing,
    /// exactly as the voice keyer does.
    dir: Option<PathBuf>,
}

impl ImagePresetStore {
    /// Load whatever is on disk. Never fails: an unreadable slot is an empty one.
    pub fn load() -> ImagePresetStore {
        let dir = match sdroxide_config::sstv_tx_dir() {
            Ok(d) => Some(d),
            Err(e) => {
                warn!("no transmit-image directory: {e}; presets will not be stored");
                None
            }
        };
        let mut store = ImagePresetStore {
            slots: vec![Slot::default(); IMAGE_SLOTS],
            messages: vec![String::new(); IMAGE_SLOTS],
            dir,
        };
        let saved = sdroxide_config::load_sstv_messages();
        for (i, m) in saved.into_iter().take(IMAGE_SLOTS).enumerate() {
            store.messages[i] = m;
        }
        for i in 0..IMAGE_SLOTS {
            let Some(path) = store.dir.as_ref().map(|d| slot_path(d, i)) else { continue };
            let Ok(bytes) = std::fs::read(&path) else { continue };
            match normalise(&bytes, IMAGE_SOURCE_MAX_EDGE) {
                Ok((png, w, h)) => {
                    // Slots written before the engine owned them hold the picked
                    // file verbatim — a phone JPEG under a `.png` name. Writing
                    // the normalised form back does that conversion once, rather
                    // than on every start, and shrinks the file while it is at it.
                    if png != bytes {
                        if let Err(e) = std::fs::write(&path, &png) {
                            warn!(path = %path.display(), "rewriting normalised slot: {e}");
                        }
                    }
                    store.slots[i] = build_slot(png, w, h);
                }
                Err(e) => warn!(path = %path.display(), "transmit slot unreadable: {e}"),
            }
        }
        let stored = store.slots.iter().filter(|s| !s.png.is_empty()).count();
        if stored > 0 {
            info!(slots = stored, "transmit-image presets loaded");
        }
        store
    }

    /// A store that touches no disk, for tests.
    #[cfg(test)]
    pub fn detached() -> ImagePresetStore {
        ImagePresetStore {
            slots: vec![Slot::default(); IMAGE_SLOTS],
            messages: vec![String::new(); IMAGE_SLOTS],
            dir: None,
        }
    }

    pub fn status(&self) -> ImagePresets {
        ImagePresets {
            slots: self
                .slots
                .iter()
                .zip(&self.messages)
                .map(|(s, m)| ImageSlotInfo {
                    message: m.clone(),
                    width: s.w,
                    height: s.h,
                    version: s.version,
                    thumb: s.thumb.clone(),
                })
                .collect(),
        }
    }

    /// Adopt an already-normalised picture. The worker did the decoding; all
    /// that is left here is the thumbnail and the write.
    pub fn adopt(&mut self, slot: usize, png: Vec<u8>, w: u16, h: u16) -> bool {
        if slot >= self.slots.len() {
            return false;
        }
        if let Some(path) = self.dir.as_ref().map(|d| slot_path(d, slot)) {
            if let Err(e) = std::fs::write(&path, &png) {
                warn!(path = %path.display(), "storing transmit slot: {e}");
            }
        }
        self.slots[slot] = build_slot(png, w, h);
        true
    }

    /// Empty a slot's picture. The message is the operator's text and stays —
    /// clearing a picture is replacing it, not forgetting what it said.
    pub fn clear(&mut self, slot: usize) -> bool {
        if slot >= self.slots.len() || self.slots[slot].png.is_empty() {
            return false;
        }
        if let Some(path) = self.dir.as_ref().map(|d| slot_path(d, slot)) {
            let _ = std::fs::remove_file(path);
        }
        self.slots[slot] = Slot::default();
        true
    }

    pub fn set_message(&mut self, slot: usize, message: String) -> bool {
        if slot >= self.messages.len() || self.messages[slot] == message {
            return false;
        }
        self.messages[slot] = message;
        if self.dir.is_some() {
            if let Err(e) = sdroxide_config::save_sstv_messages(&self.messages) {
                warn!("saving transmit-image messages: {e}");
            }
        }
        true
    }

    /// A slot's stored source and the fingerprint it had when it was read.
    pub fn source(&self, slot: usize) -> (u32, Vec<u8>) {
        self.slots.get(slot).map_or((0, Vec::new()), |s| (s.version, s.png.clone()))
    }
}

fn slot_path(dir: &Path, i: usize) -> PathBuf {
    dir.join(format!("slot{i}.png"))
}

fn build_slot(png: Vec<u8>, w: u16, h: u16) -> Slot {
    let thumb = thumbnail(&png, IMAGE_SLOT_THUMB_EDGE).map(|(t, _, _)| t).unwrap_or_default();
    let version = fingerprint(&png);
    Slot { png, w, h, version, thumb }
}

/// FNV-1a over the stored bytes.
///
/// A fingerprint rather than a counter because a client caches the pixels it
/// fetched: a counter restarts at zero when the engine does, so a client still
/// holding "version 1" of the old picture would accept the new one's "version
/// 1" and go on composing the wrong photograph.
fn fingerprint(png: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &b in png {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    // Zero means "empty slot" in the wire type, so never hand it out for a
    // picture that is really there.
    if h == 0 { 1 } else { h }
}

// ── The gallery worker ──

/// What the worker hands back to the engine.
pub enum GalleryEvent {
    /// An upload, decoded and scaled, ready for a slot.
    Normalised {
        slot: u8,
        png: Vec<u8>,
        w: u16,
        h: u16,
    },
    /// An upload that was not a picture, or was too big to be one — becomes an
    /// operator notice.
    Rejected(String),
    Listing(ImageListing),
    File {
        kind: ImageKind,
        name: String,
        png: Vec<u8>,
    },
    Saved(ImageEntry),
    /// A stored picture is gone — either just removed, or already absent.
    Deleted {
        kind: ImageKind,
        name: String,
    },
}

/// Directory walks, thumbnailing, full-size reads and upload decoding, all off
/// the engine thread.
///
/// One thread per request into a channel the engine drains, the same shape the
/// network manager uses for lookups and uploads. The engine loop is the audio
/// loop: a forty-megapixel JPEG takes long enough to decode, and a season of
/// weather charts long enough to thumbnail, that doing either in it would drop
/// a block.
pub struct GalleryWorker {
    tx: Sender<GalleryEvent>,
    rx: Receiver<GalleryEvent>,
}

impl Default for GalleryWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl GalleryWorker {
    pub fn new() -> GalleryWorker {
        let (tx, rx) = crossbeam_channel::unbounded();
        GalleryWorker { tx, rx }
    }

    /// Decode, scale and re-encode a picked file for a preset slot.
    pub fn normalise(&self, slot: u8, bytes: Vec<u8>) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let ev = match normalise(&bytes, IMAGE_SOURCE_MAX_EDGE) {
                Ok((png, w, h)) => GalleryEvent::Normalised { slot, png, w, h },
                Err(e) => {
                    warn!(slot, "transmit picture refused: {e}");
                    GalleryEvent::Rejected(format!("Picture refused: {e}"))
                }
            };
            let _ = tx.send(ev);
        });
    }

    pub fn list(&self, kind: ImageKind, offset: u32, count: u32) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let (all, dir) = scan_all(kind);
            let (wanted, total) = page(all, offset, count);
            let entries = wanted.iter().filter_map(|r| entry_for(kind, r, None)).collect();
            let _ =
                tx.send(GalleryEvent::Listing(ImageListing { kind, offset, total, entries, dir }));
        });
    }

    pub fn fetch(&self, kind: ImageKind, name: String) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let png = match resolve(kind, &name) {
                Some(path) => std::fs::read(&path).unwrap_or_default(),
                None => {
                    warn!(%name, "picture fetch refused: not a name in this store");
                    Vec::new()
                }
            };
            // An empty payload rather than silence: a gallery that is waiting
            // has to be able to stop waiting.
            let _ = tx.send(GalleryEvent::File { kind, name, png });
        });
    }

    /// Remove one stored picture.
    ///
    /// Off the engine thread like everything else here — the file is on whatever
    /// the operator's pictures directory is mounted on, and an unlink on a tired
    /// SD card or a network share takes long enough to drop an audio block.
    ///
    /// A name that resolves to nothing is reported as deleted rather than as a
    /// failure. There is nothing left to delete, which is what was asked for,
    /// and a gallery still showing it is exactly the state this is meant to
    /// fix — a refusal would leave the thumbnail sitting there for ever.
    pub fn delete(&self, kind: ImageKind, name: String) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let paths = resolve_every(kind, &name);
            let ev = match remove_all(&paths) {
                // Left where it was: say so, rather than letting the gallery
                // drop a thumbnail for a file that is still on the disk.
                Some(e) => GalleryEvent::Rejected(format!("Could not delete {name}: {e}")),
                None => {
                    if paths.is_empty() {
                        warn!(%name, "picture delete: not a name in this store");
                    } else {
                        info!(%name, copies = paths.len(), "stored picture deleted");
                    }
                    GalleryEvent::Deleted { kind, name }
                }
            };
            let _ = tx.send(ev);
        });
    }

    /// Build the listing entry for a picture the engine has just written, so a
    /// gallery gets it labelled and thumbnailed without asking for the page
    /// again.
    pub fn entry(&self, kind: ImageKind, name: String, rifp: Option<RifpMeta>) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let Some(path) = resolve(kind, &name) else { return };
            let meta = std::fs::metadata(&path).ok();
            let raw = Raw {
                unix: sdroxide_types::received_at(kind, &name).unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs() as i64)
                }),
                bytes: meta.map_or(0, |m| m.len()),
                name,
                path,
            };
            if let Some(entry) = entry_for(kind, &raw, rifp) {
                let _ = tx.send(GalleryEvent::Saved(entry));
            }
        });
    }

    pub fn poll(&self) -> Vec<GalleryEvent> {
        self.rx.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG of a flat colour at the given size.
    fn png_of(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 30, 90]));
        encode_png(&img).expect("encode")
    }

    fn size_of(png: &[u8]) -> (u32, u32) {
        let img = read_bounded(png).expect("decode");
        (img.width(), img.height())
    }

    /// The store must not hold a phone camera's forty megapixels, and every
    /// client has to compose from the same pixels.
    #[test]
    fn an_oversized_picture_is_scaled_to_the_bound() {
        let (png, w, h) = normalise(&png_of(3000, 1500), 1024).expect("normalises");
        assert_eq!((w, h), (1024, 512));
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G'], "the store keeps PNGs");
        assert_eq!(size_of(&png), (1024, 512));
        // A tall picture is bounded on its long edge too.
        let (_, w, h) = normalise(&png_of(500, 2000), 1024).expect("normalises");
        assert_eq!((w, h), (256, 1024));
    }

    /// Blowing a small picture up costs bytes and adds no detail — the same
    /// rule the chart view applies to magnification.
    #[test]
    fn a_small_picture_is_not_blown_up() {
        let (_, w, h) = normalise(&png_of(400, 300), 1024).expect("normalises");
        assert_eq!((w, h), (400, 300));
    }

    /// The regression test for the reason nothing here uses
    /// `image::load_from_memory`: a tiny file may declare an enormous picture,
    /// and believing it would ask the allocator for gigabytes.
    #[test]
    fn a_picture_that_declares_more_than_it_delivers_is_refused() {
        // A valid 1×1 PNG with the IHDR width/height overwritten to 60000², and
        // the header CRC recomputed so the decoder gets as far as the size.
        let mut png = png_of(1, 1);
        let ihdr = 8 + 4 + 4; // signature, length, "IHDR"
        png[ihdr..ihdr + 4].copy_from_slice(&60_000u32.to_be_bytes());
        png[ihdr + 4..ihdr + 8].copy_from_slice(&60_000u32.to_be_bytes());
        let crc = crc32(&png[ihdr - 4..ihdr + 13]);
        png[ihdr + 13..ihdr + 17].copy_from_slice(&crc.to_be_bytes());
        let err = normalise(&png, 1024).expect_err("must refuse");
        assert!(!err.is_empty());
    }

    #[test]
    fn rubbish_is_refused_rather_than_stored() {
        assert!(normalise(&[], 1024).is_err());
        assert!(normalise(b"this is a text file, not a picture", 1024).is_err());
        assert!(normalise(&[0x89, b'P', b'N', b'G', 0, 0, 0, 0], 1024).is_err());
    }

    #[test]
    fn a_thumbnail_fits_the_bound_and_keeps_its_shape() {
        // A weather chart's proportions: three times wider than tall.
        let (thumb, w, h) = thumbnail(&png_of(1809, 600), 192).expect("thumbnails");
        assert_eq!((w, h), (1809, 600), "the reported size is the picture's, not the thumb's");
        let (tw, th) = size_of(&thumb);
        assert_eq!(tw, 192);
        assert!((th as i32 - 64).abs() <= 1, "aspect should survive: {tw}×{th}");
    }

    fn raw(name: &str, unix: i64) -> Raw {
        Raw { name: name.into(), unix, bytes: 100, path: PathBuf::from(name) }
    }

    /// A gallery is browsed newest first, and a client asking for four billion
    /// entries gets a page.
    #[test]
    fn the_newest_pictures_come_first_and_a_page_never_exceeds_the_cap() {
        let all = vec![raw("b.png", 100), raw("d.png", 300), raw("a.png", 100), raw("c.png", 200)];
        let (p, total) = page(all.clone(), 0, 10);
        assert_eq!(total, 4);
        // Newest first; equal times fall back to the name, descending.
        assert_eq!(
            p.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["d.png", "c.png", "b.png", "a.png"],
        );
        // Paging walks the same order.
        let (p, total) = page(all.clone(), 2, 10);
        assert_eq!((p.len(), total), (2, 4));
        assert_eq!(p[0].name, "b.png");
        // An offset past the end is an empty page, not a panic, and still says
        // how much there is.
        let (p, total) = page(all.clone(), 99, 10);
        assert!(p.is_empty());
        assert_eq!(total, 4);
        // And the cap holds however much is asked for.
        let many: Vec<Raw> = (0..200).map(|i| raw(&format!("x{i:03}.png"), i)).collect();
        let (p, total) = page(many, 0, u32::MAX);
        assert_eq!(p.len(), IMAGE_PAGE_MAX as usize);
        assert_eq!(total, 200);
    }

    /// A name is a fetch key from an unauthenticated socket joined to a
    /// directory we then read. It must not be able to name anything outside it.
    #[test]
    fn a_name_cannot_reach_outside_the_store() {
        let root = std::env::temp_dir().join(format!("sdroxide-store-{}", std::process::id()));
        let store = root.join("rx");
        std::fs::create_dir_all(&store).expect("temp store");
        std::fs::write(store.join("sstv-1753795200000.png"), png_of(4, 4)).expect("inside");
        std::fs::write(root.join("outside.png"), png_of(4, 4)).expect("outside");

        assert!(resolve_in(&store, "sstv-1753795200000.png").is_some());
        for bad in ["../outside.png", "..%2Foutside.png", "/etc/passwd", "outside.png"] {
            assert!(resolve_in(&store, bad).is_none(), "{bad} should not resolve");
        }
        // A symlink inside the store still points outside it, which the name
        // check alone cannot see.
        #[cfg(unix)]
        {
            let link = store.join("sneaky.png");
            let _ = std::fs::remove_file(&link);
            std::os::unix::fs::symlink(root.join("outside.png"), &link).expect("symlink");
            assert!(resolve_in(&store, "sneaky.png").is_none(), "a symlink out must be refused");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A chart saved before the store moved is listed once, from the current
    /// directory, rather than twice under the same name.
    #[test]
    fn a_picture_present_in_both_directories_is_listed_once() {
        let root = std::env::temp_dir().join(format!("sdroxide-dup-{}", std::process::id()));
        let (new, old) = (root.join("new"), root.join("old"));
        std::fs::create_dir_all(&new).expect("new");
        std::fs::create_dir_all(&old).expect("old");
        let name = "wefax-20260729-141530Z.png";
        std::fs::write(new.join(name), png_of(8, 4)).expect("write");
        std::fs::write(old.join(name), png_of(8, 4)).expect("write");
        std::fs::write(old.join("wefax-20260728-141530Z.png"), png_of(8, 4)).expect("write");

        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for dir in [&new, &old] {
            for r in scan(dir, ImageKind::Wefax) {
                if seen.insert(r.name.clone()) {
                    out.push(r);
                }
            }
        }
        assert_eq!(out.len(), 2, "the shared name should appear once");
        let shared = out.iter().find(|r| r.name == name).expect("present");
        assert!(shared.path.starts_with(&new), "the current store wins");
        // And the date came out of the name, not off the clock.
        assert_eq!(shared.unix, sdroxide_types::ymd_hms_to_unix(2026, 7, 29, 14, 15, 30));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The delete's own path check, which is the one that matters most here: a
    /// name arrives over a socket nothing authenticates and ends at
    /// `remove_file`, and there is no undo behind it.
    #[test]
    fn a_delete_cannot_reach_outside_the_store() {
        let root = std::env::temp_dir().join(format!("sdroxide-del-{}", std::process::id()));
        let store = root.join("rx");
        std::fs::create_dir_all(&store).expect("temp store");
        let outside = root.join("precious.png");
        std::fs::write(&outside, png_of(4, 4)).expect("outside");
        std::fs::write(store.join("sstv-1753795200000.png"), png_of(4, 4)).expect("inside");
        let dirs = [store.clone()];

        for bad in ["../precious.png", "/etc/passwd", "precious.png", "..", "a/b.png"] {
            assert!(resolve_every_in(&dirs, bad).is_empty(), "{bad} must name nothing");
        }
        // A symlink planted in the store still points outside it, which the name
        // check alone cannot see — and following it would delete the target.
        #[cfg(unix)]
        {
            let link = store.join("sneaky.png");
            std::os::unix::fs::symlink(&outside, &link).expect("symlink");
            assert!(resolve_every_in(&dirs, "sneaky.png").is_empty());
            let _ = std::fs::remove_file(&link);
        }
        assert!(outside.is_file(), "nothing outside the store may be removed");

        // And a name that is really in it resolves, and goes.
        let paths = resolve_every_in(&dirs, "sstv-1753795200000.png");
        assert_eq!(paths.len(), 1);
        assert_eq!(remove_all(&paths), None);
        assert!(!store.join("sstv-1753795200000.png").exists());
        // Deleting what is already gone resolves to nothing rather than failing:
        // there is nothing left to delete, which is what was asked for.
        assert!(resolve_every_in(&dirs, "sstv-1753795200000.png").is_empty());
        assert_eq!(remove_all(&[]), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A chart present in both the current directory and the legacy one is
    /// listed once, so deleting it has to take both copies — otherwise the
    /// operator deletes a chart and it comes straight back on the next listing,
    /// out of the copy they cannot see.
    #[test]
    fn a_delete_takes_every_copy_a_listing_would_have_hidden() {
        let root = std::env::temp_dir().join(format!("sdroxide-del2-{}", std::process::id()));
        let (new, old) = (root.join("new"), root.join("old"));
        std::fs::create_dir_all(&new).expect("new");
        std::fs::create_dir_all(&old).expect("old");
        let name = "wefax-20260729-141530Z.png";
        std::fs::write(new.join(name), png_of(8, 4)).expect("write");
        std::fs::write(old.join(name), png_of(8, 4)).expect("write");
        let dirs = [new.clone(), old.clone()];

        let paths = resolve_every_in(&dirs, name);
        assert_eq!(paths.len(), 2, "both copies have to be found: {paths:?}");
        assert_eq!(remove_all(&paths), None);
        assert!(!new.join(name).exists() && !old.join(name).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A slot's fingerprint has to move when the picture does — a client caches
    /// pixels against it — and stay put when only the caption changes, or every
    /// keystroke would refetch a megabyte.
    #[test]
    fn presets_round_trip_without_a_directory() {
        let mut store = ImagePresetStore::detached();
        assert_eq!(store.status().slots.len(), IMAGE_SLOTS);
        assert!(!store.status().slot(1).has_picture());

        let (png, w, h) = normalise(&png_of(64, 32), 1024).unwrap();
        assert!(store.adopt(1, png.clone(), w, h));
        let after_picture = store.status().slot(1);
        assert!(after_picture.has_picture());
        assert_eq!((after_picture.width, after_picture.height), (64, 32));
        assert_ne!(after_picture.version, 0, "a stored picture is never version 0");
        assert!(!after_picture.thumb.is_empty(), "the slot strip paints from this");
        assert_eq!(store.source(1), (after_picture.version, png));

        assert!(store.set_message(1, "CQ CQ".into()));
        let after_message = store.status().slot(1);
        assert_eq!(after_message.message, "CQ CQ");
        assert_eq!(after_message.version, after_picture.version, "text is not pixels");
        // Setting the same text again is not a change, so it emits nothing.
        assert!(!store.set_message(1, "CQ CQ".into()));

        let (png2, w2, h2) = normalise(&png_of(80, 40), 1024).unwrap();
        assert!(store.adopt(1, png2, w2, h2));
        assert_ne!(store.status().slot(1).version, after_picture.version);
        // The caption survives its picture being replaced.
        assert_eq!(store.status().slot(1).message, "CQ CQ");

        assert!(store.clear(1));
        assert!(!store.status().slot(1).has_picture());
        assert_eq!(store.status().slot(1).message, "CQ CQ", "clearing a picture keeps the text");
        assert!(!store.clear(1), "an empty slot is not a change");
        // Out-of-range indices are a client bug, not a panic.
        assert!(!store.adopt(99, vec![1], 1, 1));
        assert!(!store.set_message(99, "x".into()));
    }

    /// CRC-32 as PNG defines it, for the malformed-header test above.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }
}
