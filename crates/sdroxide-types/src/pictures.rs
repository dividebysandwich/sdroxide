//! The engine-owned picture stores: the transmit presets an operator composes
//! from, and the galleries of what has come in.
//!
//! Both used to be client state, which meant a browser tab and the console
//! attached to the same radio showed different slots and different pictures —
//! and the browser, having no filesystem to keep them in, showed none at all.
//! The pictures themselves stay engine-side, because they are files on the
//! machine the radio is plugged into; everything here is the small metadata a
//! panel needs to draw a gallery without holding one.

use serde::{Deserialize, Serialize};

/// Which store a picture belongs to.
///
/// Two, not three: SSTV and RIFP share one because a received picture is a
/// received picture whichever protocol carried it, and the engine has always
/// written both into `sstv_rx`. Charts get their own — a quarter-hour chart
/// arriving every half hour would bury a session's SSTV, and the two are
/// browsed for entirely different reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImageKind {
    /// SSTV and RIFP.
    #[default]
    Sstv,
    /// Weather fax.
    Wefax,
}

impl ImageKind {
    pub const ALL: [ImageKind; 2] = [ImageKind::Sstv, ImageKind::Wefax];

    /// The token this store's directory and file names are built from.
    pub fn key(self) -> &'static str {
        match self {
            ImageKind::Sstv => "sstv",
            ImageKind::Wefax => "wefax",
        }
    }

    /// What the panel calls a collection of these.
    pub fn label(self) -> &'static str {
        match self {
            ImageKind::Sstv => "pictures",
            ImageKind::Wefax => "charts",
        }
    }
}

/// How many transmit presets there are. Fixed so a slot number means the same
/// thing in the panel, in `slot{i}.png` on disk, and on the wire.
///
/// Ten since issue #753; a slot past an older peer's five reads as empty there.
pub const IMAGE_SLOTS: usize = 10;

/// Longest edge a stored source picture is kept at.
///
/// The same bound the compositor already applied when it loaded a picked file,
/// moved to the one place that now decides it. Two reasons it has to be here:
/// the store must not hold a phone camera's forty megapixels, and every client
/// has to compose from byte-identical pixels or two operators looking at the
/// same preset would be sending different pictures.
pub const IMAGE_SOURCE_MAX_EDGE: u16 = 1024;

/// Longest edge of a preset's thumbnail — enough for the 70×54 slot button at
/// twice the pixel density.
pub const IMAGE_SLOT_THUMB_EDGE: u16 = 160;

/// Longest edge of a gallery thumbnail. A weather chart is three times wider
/// than it is tall, so this is a ~192×64 strip: legible in the list, and a
/// hundredth of the two megapixels behind it.
pub const IMAGE_THUMB_EDGE: u16 = 192;

/// Largest upload accepted for a preset slot. Generous enough for any camera
/// JPEG, small enough that it cannot be used to exhaust the engine's memory
/// from a socket that nothing authenticates.
pub const IMAGE_UPLOAD_MAX: usize = 16 * 1024 * 1024;

/// Most entries one listing request may return.
pub const IMAGE_PAGE_MAX: u32 = 48;

/// Longest file name accepted from a client.
pub const IMAGE_NAME_MAX: usize = 128;

/// One transmit preset as a panel sees it. The source pixels are fetched
/// separately, and only for the slot being composed.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageSlotInfo {
    /// Text composited over this slot's picture. Per slot, not per session:
    /// switching slots swaps the text, which is what makes the five of them
    /// presets rather than one picture and one caption.
    pub message: String,
    /// Size of the stored source; `(0, 0)` when the slot holds no picture.
    pub width: u16,
    pub height: u16,
    /// Fingerprint of the stored bytes, 0 when the slot is empty.
    ///
    /// A fingerprint rather than a counter on purpose. A client caches the
    /// source pixels it fetched and has to know when they have gone stale; a
    /// counter restarts at zero when the engine does, so a client still holding
    /// "version 1" of the old picture would accept the new one's "version 1"
    /// and go on composing the wrong photograph.
    pub version: u32,
    /// PNG thumbnail, longest edge [`IMAGE_SLOT_THUMB_EDGE`].
    ///
    /// Carried in the snapshot rather than fetched, because the panel draws all
    /// five slot buttons at once and only ever composes from one of them.
    pub thumb: Vec<u8>,
}

impl ImageSlotInfo {
    pub fn has_picture(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Every transmit preset, emitted at startup and on every change — the same
/// contract as [`crate::VoiceStatus`], and for the same reason: what the
/// station sends belongs to the radio, not to whichever screen is looking at it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagePresets {
    /// Always [`IMAGE_SLOTS`] entries; index = slot number.
    pub slots: Vec<ImageSlotInfo>,
}

impl Default for ImagePresets {
    fn default() -> Self {
        ImagePresets { slots: vec![ImageSlotInfo::default(); IMAGE_SLOTS] }
    }
}

impl ImagePresets {
    pub fn slot(&self, i: usize) -> ImageSlotInfo {
        self.slots.get(i).cloned().unwrap_or_default()
    }
}

/// One picture in a received store, as a gallery lists it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageEntry {
    pub kind: ImageKind,
    /// Bare file name inside the store — never a path. It is also the whole of
    /// a chart's metadata (see [`crate::WefaxChartMeta`]) and the key every
    /// fetch names it by, which is why it is sanitised on the way back in.
    pub name: String,
    /// Unix seconds it was received, read out of the name; the file's
    /// modification time for anything this program did not write.
    pub unix: i64,
    pub width: u16,
    pub height: u16,
    /// Size of the stored file, so a gallery can say what a full-size fetch
    /// will cost before pulling it down a phone link.
    pub bytes: u64,
    /// PNG thumbnail, longest edge [`IMAGE_THUMB_EDGE`].
    pub thumb: Vec<u8>,
    /// What the RIFP manifest said, for a picture that has just arrived over
    /// RIFP. Always `None` for anything read back off disk: the manifest is not
    /// in the PNG and there is nowhere it survives, so the enlarged view says
    /// nothing rather than inventing a sender.
    pub rifp: Option<crate::RifpMeta>,
}

/// One page of a received store, newest first.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageListing {
    pub kind: ImageKind,
    /// Where `entries` starts within the whole store.
    pub offset: u32,
    /// How many pictures the store holds altogether, so a gallery can say
    /// "312 older" exactly instead of guessing from what it was given.
    pub total: u32,
    pub entries: Vec<ImageEntry>,
    /// The directory the pictures are in, for the panel to show. On a remote
    /// client this names a directory on the *radio's* machine, which is why the
    /// panel has to say so rather than just printing it.
    pub dir: String,
}

/// When a stored picture was received, read out of its file name.
///
/// The name is the only timestamp there is — neither store writes a sidecar —
/// so every form this program has ever written is accepted. `None` for a file
/// it did not write, and the caller falls back to the modification time.
pub fn received_at(kind: ImageKind, name: &str) -> Option<i64> {
    match kind {
        // `sstv-<unix millis>.png`, as the engine writes it.
        ImageKind::Sstv => {
            let rest = name.strip_prefix("sstv-")?;
            let digits = rest.strip_suffix(".png").or_else(|| rest.strip_suffix(".PNG"))?;
            digits.parse::<i64>().ok().map(|ms| ms / 1000)
        }
        ImageKind::Wefax => crate::WefaxChartMeta::from_file_name(name).map(|m| m.unix),
    }
}

/// Accept a file name from a client, or refuse it.
///
/// Names arrive over a socket that nothing authenticates and end up joined to a
/// directory this program will read. So: nothing but the characters our own
/// names use, no leading dot, no `..`, and a `.png` extension — which every
/// picture in either store has. Refusing is cheap and there is no legitimate
/// name this rejects; a file the operator dropped in by hand with a space in it
/// simply never appears in a listing, which is better than handing back a name
/// that cannot then be fetched.
pub fn safe_name(name: &str) -> Option<&str> {
    if name.is_empty() || name.len() > IMAGE_NAME_MAX {
        return None;
    }
    if name.starts_with('.') || name.contains("..") {
        return None;
    }
    // One pass kills `/`, `\`, `:`, NUL, newline and every Unicode look-alike
    // separator, rather than a blocklist that has to keep up with them.
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_')) {
        return None;
    }
    let dot = name.rfind('.')?;
    name[dot..].eq_ignore_ascii_case(".png").then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name is a fetch key that arrives over an unauthenticated socket and
    /// is joined to a directory we then read. Everything that could reach out
    /// of that directory has to be refused, and every name either store
    /// actually writes has to be accepted.
    #[test]
    fn a_name_from_outside_the_store_is_refused() {
        for bad in [
            "",
            "../../etc/passwd",
            "../secret.png",
            "..",
            "a/b.png",
            "a\\b.png",
            "/etc/shadow.png",
            ".hidden.png",
            "chart.jpg",
            "chart.png.jpg",
            "chart",
            "chart.png\0",
            "chart\n.png",
            "chart .png",
            "chárt.png",
            // A separator that is not ASCII is still a separator.
            "a\u{2044}b.png",
        ] {
            assert_eq!(safe_name(bad), None, "{bad:?} should be refused");
        }
        // Too long, one byte over.
        let long = format!("{}.png", "x".repeat(IMAGE_NAME_MAX));
        assert_eq!(safe_name(&long), None);

        // And the names the two stores really write, dots and all.
        for good in [
            "sstv-1753795200000.png",
            "wefax-20260729-141530Z-7878.1kHz-DWD.png",
            "wefax-20260729-141530Z.png",
            "chart.PNG",
            "holiday_2026.png",
        ] {
            assert_eq!(safe_name(good), Some(good), "{good:?} should be accepted");
        }
    }

    /// A gallery is browsed newest first, and the only timestamp either store
    /// keeps is in the file name.
    #[test]
    fn a_stored_name_carries_the_time_it_was_received() {
        assert_eq!(received_at(ImageKind::Sstv, "sstv-1753795200000.png"), Some(1_753_795_200));
        assert_eq!(received_at(ImageKind::Sstv, "sstv-1753795200000.PNG"), Some(1_753_795_200));
        assert_eq!(
            received_at(ImageKind::Wefax, "wefax-20260729-141530Z-7878.1kHz-DWD.png"),
            Some(crate::ymd_hms_to_unix(2026, 7, 29, 14, 15, 30)),
        );
        // The millisecond names charts were saved under before the date went in.
        assert_eq!(received_at(ImageKind::Wefax, "wefax-1753795200000.png"), Some(1_753_795_200));
        // Anything else has no date to give, and the caller falls back to mtime
        // rather than inventing one.
        for foreign in ["holiday.png", "sstv-.png", "sstv-abc.png", "wefax.png"] {
            assert_eq!(received_at(ImageKind::Sstv, foreign), None, "{foreign}");
            assert_eq!(received_at(ImageKind::Wefax, foreign), None, "{foreign}");
        }
    }

    #[test]
    fn the_presets_default_to_five_empty_slots() {
        let p = ImagePresets::default();
        assert_eq!(p.slots.len(), IMAGE_SLOTS);
        assert!(p.slots.iter().all(|s| !s.has_picture() && s.version == 0 && s.message.is_empty()));
        // Asking for a slot that is not there gives an empty one rather than
        // panicking — the panel indexes this from a fixed-size strip.
        assert!(!p.slot(99).has_picture());
    }
}
