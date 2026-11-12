//! Scanning: working through a list of channels, or a slice of a band, and
//! stopping where something is on the air.
//!
//! The unusual part of doing this on a software-defined radio is that it need
//! not be slow. A handheld scanner has one receiver and has to visit each
//! channel in turn, which is why covering 2 m takes minutes; here the FFT that
//! already draws the panadapter sees a whole span at once, so a range scan moves
//! the hardware one span at a time and reads every channel in it together. Only
//! a front end with no wideband IQ of its own — a CAT rig on a sound card — has
//! to fall back to visiting channels one at a time.

use serde::{Deserialize, Serialize};

use crate::Mode;

/// What a scan runs over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScanKind {
    /// The stored memory channels, each with its own mode and filter.
    #[default]
    Memories,
    /// A frequency range on a fixed channel grid, in one mode.
    Range,
    /// A frequency range swept without stopping, its slices stitched into one
    /// full-range spectrum for the full-band strip (WIDE): a panorama for a
    /// front end that only sees a couple of megahertz at a time.
    // Why: HDZ 2026-11-12: "in einer Stufe den RTL-Leuten ein Panorama
    //      anbieten und dieses dann als Sweep-Scanner weiter ausbauen".
    Panorama,
}

impl ScanKind {
    pub const ALL: [ScanKind; 3] = [ScanKind::Memories, ScanKind::Range, ScanKind::Panorama];

    pub fn label(self) -> &'static str {
        match self {
            ScanKind::Memories => "MEM",
            ScanKind::Range => "RANGE",
            ScanKind::Panorama => "PANO",
        }
    }
}

/// What ends a stop on a busy channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScanResume {
    /// Carry on once the signal drops, after a short grace period — the usual
    /// scanner behaviour, and the one that follows a conversation.
    #[default]
    Carrier,
    /// Carry on after a fixed time whether or not the signal is still there.
    Timed,
    /// Stay until the operator says otherwise.
    Manual,
}

impl ScanResume {
    pub const ALL: [ScanResume; 3] = [ScanResume::Carrier, ScanResume::Timed, ScanResume::Manual];

    pub fn label(self) -> &'static str {
        match self {
            ScanResume::Carrier => "CARRIER",
            ScanResume::Timed => "TIMED",
            ScanResume::Manual => "MANUAL",
        }
    }
}

/// Channel spacings a range scan can step on, in Hz. 12.5 kHz is the European
/// VHF/UHF norm, 25 kHz the older wide spacing, 8.33 kHz airband, 5 kHz
/// broadcast and PMR, 6.25 kHz the narrowest in common use.
pub const SCAN_STEPS_HZ: [f64; 6] = [5_000.0, 6_250.0, 8_333.0, 10_000.0, 12_500.0, 25_000.0];

/// Persisted scanner settings (`scanner.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScannerConfig {
    pub kind: ScanKind,
    pub range_lo_hz: f64,
    pub range_hi_hz: f64,
    /// Channel grid a range scan snaps candidates to.
    pub step_hz: f64,
    /// Mode a range scan uses. A memory scan takes the mode from each memory.
    pub mode: Mode,
    /// Level (dBFS) a channel has to reach to count as busy. The same scale as
    /// [`crate::RxState::squelch_db`], and measured the same way, so the two
    /// agree about what "busy" means.
    pub threshold_db: f32,
    /// Take the threshold from the receiver's own squelch instead, so one
    /// control sets both what stops the scan and what opens the audio.
    pub follow_squelch: bool,
    /// How long to listen on a candidate before deciding it is really busy.
    pub dwell_ms: u32,
    pub resume: ScanResume,
    /// [`ScanResume::Timed`]: how long to stay. [`ScanResume::Carrier`]: how
    /// long to linger after the signal drops, so a gap between overs does not
    /// lose the conversation.
    pub resume_ms: u32,
    /// Memory ids to pass over.
    pub skip: Vec<u32>,
    /// Which memory folders a [`ScanKind::Memories`] scan runs over: `None` is
    /// the top level (the unfiled channels), `Some(id)` is one folder.
    ///
    /// **Empty means all of them** — the historic behaviour, and what a station
    /// with no folders always does. That way round rather than listing every
    /// folder by default because a folder made after the setting was last
    /// touched has to be scanned rather than silently left out, and because a
    /// list that has to be kept in step with the folders is a list that goes
    /// stale the moment one is renamed or removed.
    ///
    /// A folder holds channels for one service — marine, airband, the local
    /// repeaters — and scanning all of them at once is a scan that spends most
    /// of its time somewhere the operator is not listening (issue #236).
    #[serde(default)]
    pub folders: Vec<Option<u32>>,
    /// Read a memory scan's channels off the wideband spectrum instead of
    /// visiting each one (issue #228).
    ///
    /// A memory scan tunes to every channel in turn and listens, which is what a
    /// handheld scanner has to do and costs a settling time each — a hundred
    /// channels at the default dwell is fifteen seconds a lap. A receiver with
    /// I/Q of its own can instead put every channel that falls inside one window
    /// on the same transform the panadapter is already made from, and only visit
    /// the ones something is on. A list on one band then costs a single tune a
    /// lap however long it is.
    ///
    /// Off by default, and an opt-in rather than the new behaviour: the sweep
    /// measures a channel through the FFT rather than through the receiver's own
    /// filter and AGC, so a threshold that was right for one is not always right
    /// for the other, and an operator with a list short enough not to care
    /// should not have to work that out. Ignored on a front end with no span to
    /// search — a CAT rig on a sound card — which visits channels either way.
    #[serde(default)]
    pub mem_fast: bool,
    /// Frequencies (Hz, on the channel grid) a range scan passes over.
    ///
    /// The range-scan twin of [`Self::skip`], and it has to be a frequency
    /// rather than an index because a range scan has no stored channels to
    /// index: what it finds depends on what is on the air at the time. Kept
    /// across runs so a band worked through in several passes does not stop on
    /// the same repeater tail, packet node or birdie every time round.
    pub skip_freq_hz: Vec<f64>,
    /// The `(lo, hi, step)` [`Self::skip_freq_hz`] was collected under.
    ///
    /// "Skip this one" means "not this channel, *here*", and says nothing about
    /// a different band or a different grid — so retuning the scan somewhere
    /// else empties the list rather than carrying stale channels into a range
    /// they were never chosen in. See [`Self::forget_stale_skips`].
    pub skip_freq_for: (f64, f64, f64),
}

impl Default for ScannerConfig {
    fn default() -> Self {
        ScannerConfig {
            kind: ScanKind::Memories,
            // The 2 m band: the most-scanned range there is, and a sane thing to
            // find already filled in the first time the window is opened.
            range_lo_hz: 144_000_000.0,
            range_hi_hz: 146_000_000.0,
            step_hz: 12_500.0,
            mode: Mode::Nfm,
            threshold_db: -80.0,
            follow_squelch: false,
            dwell_ms: 150,
            resume: ScanResume::Carrier,
            resume_ms: 2_000,
            skip: Vec::new(),
            folders: Vec::new(),
            mem_fast: false,
            skip_freq_hz: Vec::new(),
            skip_freq_for: (0.0, 0.0, 0.0),
        }
    }
}

impl ScannerConfig {
    /// The range with its edges the right way round, which is how the engine
    /// wants it however the operator typed it in.
    pub fn range(&self) -> (f64, f64) {
        if self.range_lo_hz <= self.range_hi_hz {
            (self.range_lo_hz, self.range_hi_hz)
        } else {
            (self.range_hi_hz, self.range_lo_hz)
        }
    }

    /// What [`Self::skip_freq_hz`] is tied to: the range, and the grid the
    /// skipped channels were snapped to.
    fn skip_key(&self) -> (f64, f64, f64) {
        let (lo, hi) = self.range();
        (lo, hi, self.step_hz)
    }

    /// Throw the range skips away if they belong to a different range or grid,
    /// and stamp the list with the one they belong to now.
    ///
    /// Called wherever an edited config is adopted, so that moving the scan to
    /// another band starts with a clean sheet — and moving it back does *not*
    /// bring the old skips with it, which would be a scanner quietly refusing
    /// to stop on channels the operator no longer remembers dismissing.
    pub fn forget_stale_skips(&mut self) {
        let key = self.skip_key();
        if self.skip_freq_for != key {
            self.skip_freq_hz.clear();
            self.skip_freq_for = key;
        }
    }

    /// Add a range-scan skip, snapped to the channel grid the scan searches on
    /// — which is the grid every candidate it could offer is already on.
    pub fn skip_freq(&mut self, hz: f64) {
        let step = self.step_hz.max(1.0);
        let snapped = (hz / step).round() * step;
        self.forget_stale_skips();
        if !self.skips_freq(snapped) {
            self.skip_freq_hz.push(snapped);
        }
    }

    /// Whether a range scan should pass over `hz`.
    ///
    /// Compared with half a channel of slack rather than exactly: a candidate
    /// comes from an FFT bin and is snapped to the grid, and the arithmetic
    /// that snapped it need not land on the same last bit as the arithmetic
    /// that snapped the skip.
    pub fn skips_freq(&self, hz: f64) -> bool {
        let tol = self.step_hz.max(1.0) / 2.0;
        self.skip_freq_hz.iter().any(|&f| (f - hz).abs() < tol)
    }

    /// Whether a memory filed under `folder` is one this scan runs over.
    ///
    /// `folder` is the channel's folder *as the list draws it*: a memory whose
    /// folder has gone from under it reads as unfiled, here as everywhere else,
    /// so the caller resolves the id against the folders that exist before
    /// asking.
    pub fn scans_folder(&self, folder: Option<u32>) -> bool {
        self.folders.is_empty() || self.folders.contains(&folder)
    }

    /// Drop a folder that no longer exists from [`Self::folders`].
    ///
    /// Its channels are back at the top level by then, and a selection still
    /// naming it would be a scan quietly looking for them where they are not.
    /// Returns whether anything changed, so the caller only persists a config
    /// that moved.
    pub fn forget_folder(&mut self, id: u32) -> bool {
        let before = self.folders.len();
        self.folders.retain(|f| *f != Some(id));
        self.folders.len() != before
    }

    /// Whether the settings describe a scan that could ever stop anywhere.
    pub fn range_is_usable(&self) -> bool {
        let (lo, hi) = self.range();
        // From 0 Hz up: an HF range is typed as "0 to 30", and a front end that
        // cannot reach the bottom of it says so when the slice is tuned.
        lo.is_finite() && hi.is_finite() && lo >= 0.0 && hi - lo >= self.step_hz.max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ScannerConfig {
        let mut c = ScannerConfig {
            kind: ScanKind::Range,
            range_lo_hz: 144_000_000.0,
            range_hi_hz: 146_000_000.0,
            step_hz: 12_500.0,
            ..ScannerConfig::default()
        };
        c.forget_stale_skips();
        c
    }

    /// A candidate arrives from an FFT bin and is snapped to the grid; the
    /// arithmetic that snapped it need not produce the same last bit as the
    /// arithmetic that snapped the skip, so the match has to have slack in it —
    /// but not so much that it swallows the neighbouring channel.
    #[test]
    fn a_skip_matches_its_own_channel_and_no_other() {
        let mut c = cfg();
        c.skip_freq(145_312_500.0);
        assert!(c.skips_freq(145_312_500.0));
        assert!(c.skips_freq(145_312_500.0 + 1.0), "a hertz of rounding must not miss it");
        assert!(!c.skips_freq(145_325_000.0), "the next channel up is a different channel");
        assert!(!c.skips_freq(145_300_000.0), "and so is the one below");
    }

    /// Anything inside a channel of the grid means that channel — an operator
    /// pressing SKIP is pointing at where the dial is, which is wherever the
    /// sweep put it, not at an exact multiple of the step.
    #[test]
    fn a_skip_is_stored_on_the_grid() {
        let mut c = cfg();
        c.skip_freq(145_310_000.0);
        assert_eq!(c.skip_freq_hz, vec![145_312_500.0]);
        // And asking twice does not list it twice.
        c.skip_freq(145_313_000.0);
        assert_eq!(c.skip_freq_hz.len(), 1, "{:?}", c.skip_freq_hz);
    }

    #[test]
    fn skips_belong_to_the_range_they_were_taken_in() {
        let mut c = cfg();
        c.skip_freq(145_312_500.0);

        // A different band: nothing carries over, and going back does not bring
        // it back either — a skip nobody remembers taking is invisible.
        let mut moved = ScannerConfig { range_lo_hz: 430e6, range_hi_hz: 432e6, ..c.clone() };
        moved.forget_stale_skips();
        assert!(moved.skip_freq_hz.is_empty(), "{:?}", moved.skip_freq_hz);
        let mut back = ScannerConfig { range_lo_hz: 144e6, range_hi_hz: 146e6, ..moved };
        back.forget_stale_skips();
        assert!(back.skip_freq_hz.is_empty(), "the old skips came back");

        // A different grid describes different channels, so it counts as a move.
        let mut regridded = ScannerConfig { step_hz: 25_000.0, ..c.clone() };
        regridded.forget_stale_skips();
        assert!(regridded.skip_freq_hz.is_empty(), "{:?}", regridded.skip_freq_hz);

        // The same range typed the other way round is the same range.
        let mut flipped = ScannerConfig { range_lo_hz: 146e6, range_hi_hz: 144e6, ..c.clone() };
        flipped.forget_stale_skips();
        assert_eq!(flipped.skip_freq_hz, c.skip_freq_hz, "swapping the edges is not a new range");
    }

    /// The stored default has never been stamped, so the first config the
    /// engine adopts must not be read as "skips taken in an unknown range".
    #[test]
    fn a_default_config_has_nothing_to_forget() {
        let mut c = ScannerConfig::default();
        c.forget_stale_skips();
        assert!(c.skip_freq_hz.is_empty());
        assert_eq!(c.skip_freq_for, (144_000_000.0, 146_000_000.0, 12_500.0));
    }
}

/// What the scanner is doing, small enough to ride in every [`crate::RadioState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScanState {
    pub running: bool,
    /// Stopped on a busy channel rather than moving.
    pub holding: bool,
}

#[cfg(test)]
mod folder_tests {
    use super::*;

    /// No selection is every folder — including one made since the setting was
    /// last touched, which is the reason it is stored as a selection rather
    /// than as a list of everything (issue #236).
    #[test]
    fn an_empty_selection_scans_everything() {
        let c = ScannerConfig::default();
        assert!(c.scans_folder(None));
        assert!(c.scans_folder(Some(1)));
        assert!(c.scans_folder(Some(99)));
    }

    /// And a selection is exactly what it names. The top level is a place a
    /// channel can be filed under, so it is selectable like any other.
    #[test]
    fn a_selection_is_the_folders_it_names() {
        let c = ScannerConfig { folders: vec![Some(2)], ..ScannerConfig::default() };
        assert!(c.scans_folder(Some(2)));
        assert!(!c.scans_folder(Some(1)));
        assert!(!c.scans_folder(None), "the unfiled channels are not folder 2");

        let top = ScannerConfig { folders: vec![None], ..ScannerConfig::default() };
        assert!(top.scans_folder(None));
        assert!(!top.scans_folder(Some(1)));
    }

    /// A deleted folder's channels go back to the top level, so a selection
    /// still naming it would look for them where they are not.
    #[test]
    fn a_deleted_folder_leaves_the_selection() {
        let mut c = ScannerConfig { folders: vec![Some(1), Some(2)], ..ScannerConfig::default() };
        assert!(c.forget_folder(2));
        assert_eq!(c.folders, vec![Some(1)]);
        assert!(!c.forget_folder(2), "asking twice is not a change to persist");
    }
}
