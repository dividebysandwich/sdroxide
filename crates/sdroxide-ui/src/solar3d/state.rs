//! Everything the solar-system window's deferred viewport callback touches.
//!
//! `show_viewport_deferred` requires an `Fn + Send + Sync + 'static` closure, so
//! the window cannot borrow `SdroxideApp`. All of its mutable state lives here
//! behind an `Arc<Mutex<_>>` that both the root pass and the child pass hold.

use crate::i18n::tr;
use std::sync::{Arc, Mutex};

use sdroxide_solar::SolarData;

use crate::view::Solar3dView;

/// Which body the orbit camera pivots around.
///
/// Persisted as an integer in [`Solar3dView::focus`], so the encoding in
/// [`Focus::to_id`] is a stable format: the four original values keep indices
/// 0–3 and everything new is appended after them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sun,
    Earth,
    Moon,
    /// Midpoint of the Earth–Moon line, for framing the pair.
    EarthMoon,
    Planet(sdroxide_solar::Planet),
    /// A moon of another planet, by index into [`sdroxide_solar::planets::MOONS`].
    Satellite(usize),
    /// A dwarf planet, asteroid or comet, by index into
    /// [`sdroxide_solar::smallbody::BODIES`].
    Small(usize),
}

impl Focus {
    /// The four targets that are not a table lookup.
    pub const NEAR: [Focus; 4] = [Focus::Sun, Focus::Earth, Focus::Moon, Focus::EarthMoon];

    /// Where the small bodies start in the persisted encoding.
    ///
    /// A round number well past the moons rather than immediately after them,
    /// so that adding a moon — which the moon table's own docs invite, by
    /// appending — cannot shift every asteroid's stored id underneath somebody's
    /// settings file. The gap costs nothing: the encoding is a `u16` and this
    /// is the only thing in it that has to be arranged rather than counted.
    const SMALL_BASE: usize = 256;

    /// Every target, grouped the way the picker lays them out: the Sun and the
    /// Earth–Moon system first, then a row per planet with its own moons, then
    /// the dwarf planets.
    ///
    /// The asteroids and comets are deliberately absent. There are thirty-five
    /// of them, they would swamp a popup meant for choosing between eleven
    /// planets, and the search box finds them by name — which is how you look
    /// for a body you already have in mind, and the only way that scales.
    pub fn groups() -> Vec<(String, Vec<Focus>)> {
        let mut v = vec![(tr!("solarstate-home"), Focus::NEAR.to_vec())];
        for p in sdroxide_solar::Planet::ALL {
            let mut row = vec![Focus::Planet(p)];
            row.extend(
                sdroxide_solar::planets::MOONS
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.parent == p)
                    .map(|(i, _)| Focus::Satellite(i)),
            );
            // Captions are drawn as given, so the name is put in capitals here.
            v.push((p.name().to_uppercase(), row));
        }
        v.push((
            tr!("solarstate-dwarf-planets"),
            sdroxide_solar::smallbody::BODIES
                .iter()
                .enumerate()
                .filter(|(_, b)| b.class == sdroxide_solar::SmallClass::Dwarf)
                .map(|(i, _)| Focus::Small(i))
                .collect(),
        ));
        v
    }

    /// Every target, flattened — the picker's, plus the small bodies it leaves
    /// to the search box. Used by the tests that guard the persisted encoding.
    #[cfg(test)]
    pub fn all() -> Vec<Focus> {
        let mut v: Vec<Focus> = Focus::groups().into_iter().flat_map(|(_, row)| row).collect();
        for i in 0..sdroxide_solar::smallbody::BODIES.len() {
            if !v.contains(&Focus::Small(i)) {
                v.push(Focus::Small(i));
            }
        }
        v
    }

    pub fn from_id(v: u16) -> Focus {
        let v = v as usize;
        if let Some(f) = Focus::NEAR.get(v) {
            return *f;
        }
        if v >= Focus::SMALL_BASE {
            // An id from a newer build with more bodies: fall back to the Sun
            // rather than to a body that is not the one meant.
            let i = v - Focus::SMALL_BASE;
            return if i < sdroxide_solar::smallbody::BODIES.len() {
                Focus::Small(i)
            } else {
                Focus::Sun
            };
        }
        let k = v - Focus::NEAR.len();
        match sdroxide_solar::Planet::ALL.get(k) {
            Some(p) => Focus::Planet(*p),
            None => {
                let m = k - sdroxide_solar::Planet::ALL.len();
                if m < sdroxide_solar::planets::MOONS.len() {
                    Focus::Satellite(m)
                } else {
                    Focus::Sun
                }
            }
        }
    }

    pub fn to_id(self) -> u16 {
        let base = Focus::NEAR.len();
        let planets = sdroxide_solar::Planet::ALL.len();
        (match self {
            Focus::Planet(p) => base + p.index(),
            Focus::Satellite(i) => base + planets + i,
            Focus::Small(i) => Focus::SMALL_BASE + i,
            f => Focus::NEAR.iter().position(|x| *x == f).unwrap_or(0),
        }) as u16
    }

    /// Full name, as the picker shows it.
    pub fn label(self) -> &'static str {
        match self {
            Focus::Sun => "Sun",
            Focus::Earth => "Earth",
            Focus::Moon => "Moon",
            Focus::EarthMoon => "Earth + Moon",
            Focus::Planet(p) => p.name(),
            Focus::Satellite(i) => sdroxide_solar::planets::MOONS.get(i).map_or("Sun", |m| m.name),
            Focus::Small(i) => sdroxide_solar::smallbody::BODIES.get(i).map_or("Sun", |b| b.name),
        }
    }

    /// Short upper-case form for the button face.
    pub fn short(self) -> String {
        match self {
            Focus::EarthMoon => "E+M".to_string(),
            f => f.label().to_uppercase(),
        }
    }

    /// True for a body that orbits another body this view also draws — the
    /// picker indents those under their planet.
    pub fn is_satellite(self) -> bool {
        matches!(self, Focus::Moon | Focus::Satellite(_))
    }
}

/// Shared window state. Never hold the lock across I/O or across a call into
/// egui that could re-enter the viewport callback.
pub struct SolarUi {
    /// Persisted camera / layer / scale settings, mirrored back into
    /// `ViewState` by the root pass each frame.
    pub view: Solar3dView,
    /// Set by the child pass when the OS window's close button is hit; drained
    /// by the root pass, which then stops emitting the viewport.
    ///
    /// Native only: in the browser this view *is* the tab, and closing a tab is
    /// the browser's business rather than something to route through here.
    #[cfg(not(target_arch = "wasm32"))]
    pub close_requested: bool,
    /// Set by the overlay's ↻ button; drained by the root pass, which owns the
    /// feed handle the child pass cannot reach.
    pub refresh_requested: bool,
    /// The engine's satellite lock, pushed by the host each frame: the
    /// catalogue number the scene highlights, draws the QTH line to, and the
    /// AUTO camera frames. `None` in the browser tab, which nothing pushes it
    /// into yet.
    pub sat_lock: Option<u64>,
    /// Set by the pass window's LOCK button; drained by the root pass, which
    /// owns the command path to the engine. Native only, like
    /// `close_requested`: the browser tab's `/solar-ws` relay carries no
    /// commands, so the button is not even drawn there.
    #[cfg(not(target_arch = "wasm32"))]
    pub lock_requested: Option<u64>,
    /// The matching UNLOCK, same contract.
    #[cfg(not(target_arch = "wasm32"))]
    pub unlock_requested: bool,
    /// Operator QTH as configured (Maidenhead) and its decoded (lat, lon).
    pub qth_grid: String,
    pub qth: Option<(f64, f64)>,
    /// Simulated-time offset from now, in seconds — driven by the time chips so
    /// the whole scene can be scrubbed forward and back.
    pub sim_offset_s: f64,
    /// The background feed's snapshot, once the feed has been started. A second
    /// handle on the feed's own mutex, because this closure outlives any borrow
    /// of the feed itself.
    ///
    /// Lock ordering is always `SolarUi` then `SolarData`; the worker thread
    /// only ever takes `SolarData`, so there is no cycle.
    pub data: Option<Arc<Mutex<SolarData>>>,
    /// FT8/FT4 traffic to plot on the globe.
    pub digi: DigiTraffic,
    /// Which contact the QSO card is typing out, and when it began typing.
    ///
    /// The key is the callsign and the contact's start time together, so
    /// working the same station twice retypes the card rather than leaving the
    /// second QSO wearing the first one's finished text. Cleared when the arc
    /// goes away, which is what makes the next contact start from a blank card.
    pub qso_card: Option<(String, f64)>,
    /// Award coverage to paint: every DXCC entity, placed, and how far along it
    /// is in the logbook. Republished by the host whenever the log changes.
    ///
    /// Empty in the browser tab: the logbook lives in the main window, and the
    /// `/solar-ws` relay carries live data rather than the operator's records.
    /// The layer chip goes away with it rather than pretending to paint.
    pub awards: std::sync::Arc<Vec<sdroxide_types::EntitySlot>>,
    /// The propagation field: which bands are getting through where, from every
    /// mode this station runs. Republished by the host each frame.
    ///
    /// Behind an `Arc` because a couple of hundred kilobytes a plane crossing
    /// into this window sixty times a second is the one part of this that would
    /// cost anything.
    pub prop: std::sync::Arc<sdroxide_types::PropField>,
    /// The propagation field resolved to RGBA, and what it was resolved from.
    ///
    /// Cached here rather than rebuilt per frame: it costs ten thousand pixels
    /// of colour arithmetic, and the field only moves when a decode lands. The
    /// key is (field generation, mode, band, band mask) — everything the
    /// picture depends on.
    pub prop_rgba: std::sync::Arc<Vec<u8>>,
    pub prop_rgba_key: Option<(u64, u8, u8, u32)>,
    /// Bumped on every rebuild, so the GPU upload can skip an unchanged one.
    pub prop_gen: u64,
    /// The 3D window's remembered geometry, seeded from the screen's
    /// `UiSettings`. The builder consults it only when the window is
    /// (re)built, so a live resize is never fought. Native only: the browser
    /// tab is the page's to size.
    #[cfg(not(target_arch = "wasm32"))]
    pub window_seed: Option<sdroxide_types::Solar3dWindow>,
    /// The geometry the window is actually at, written by the window's own pass
    /// and read back into `UiSettings` by the host. `pos` is `None` on Wayland,
    /// which reports no absolute window position.
    #[cfg(not(target_arch = "wasm32"))]
    pub window_now: Option<sdroxide_types::Solar3dWindow>,
    /// Where the activity time-lapse's replay head sits, in seconds before now.
    /// Zero is live, which is where it starts every run: a globe that came back
    /// up showing forty minutes ago would read as a stalled feed.
    pub lapse_back_s: f64,
    /// Whether the replay head is sweeping forward on its own.
    pub lapse_playing: bool,
    /// The operator's satellite additions: their own element sets (already
    /// pushed into the feed) and the frequency entries that override the
    /// built-in table in the pass window. Republished by the host whenever the
    /// station's config changes.
    ///
    /// Only `freqs` is read here, which is why the browser tab — where this
    /// arrives over `/solar-ws` as a bare frequency table — can fill nothing
    /// else and lose nothing by it. Anything that comes to want another field
    /// has to widen [`SolarServerMsg::SatFreqs`] to carry it.
    ///
    /// [`SolarServerMsg::SatFreqs`]: sdroxide_proto::solar::SolarServerMsg::SatFreqs
    pub sat_cfg: std::sync::Arc<sdroxide_types::SatConfig>,
    /// What has been typed into the find box.
    ///
    /// Matches are drawn with their orbit and label whether or not they
    /// otherwise would be — looking for something that is *not* on screen is
    /// the main reason to search at all. It covers two populations that have
    /// the same problem: ninety satellites around one planet, and forty small
    /// bodies scattered over fifty AU, both of them dots too small and too
    /// numerous to find by reading labels.
    ///
    /// The small bodies have no layer chip of their own on purpose. A chip
    /// answers "show me all of these", which for thirty-five asteroids is not a
    /// question anyone has; the question people actually have is "where is
    /// Apophis", and that is a search box.
    pub search: String,
    /// Satellite whose pass table is open, by catalogue number.
    pub selected_sat: Option<u64>,
    /// Cached pass prediction: which satellite, from what QTH, computed when,
    /// and the result. Stepping a whole orbit at second resolution is far too
    /// expensive to redo every frame.
    pub sat_passes: Option<SatPasses>,
    /// Pivot supplied by the AUTO tour while it is flying between stations:
    /// position and the radius the distance clamp uses. Frame-scoped — cleared
    /// whenever the tour is not driving.
    pub focus_override: Option<(super::math::V3, f32)>,
    /// Animated camera tour state, and the frame time it last advanced at.
    pub tour: super::camera::Tour,
    pub last_frame_time: f64,
    /// Set when the target changes, so the next frame — which has the bodies
    /// placed already — can pull the camera in to frame whatever was picked.
    pub retarget: bool,
    /// When each menu chip's popup was opened, for the fade that takes a
    /// forgotten one away again. One slot per chip, in bar order; owned here
    /// because [`crate::chrome::popup_fade_alpha`] keeps no state of its own.
    pub menu_since: [Option<f64>; MENUS],
}

/// How many menu chips the overlay's bar has — the width of
/// [`SolarUi::menu_since`].
pub const MENUS: usize = 8;

/// ASCII-case-insensitive substring test.
///
/// Allocation-free because this runs once per satellite per frame, and the
/// alternative — uppercasing both sides — is two `String`s ninety times at
/// sixty frames a second. Satellite designators are ASCII, so folding only
/// ASCII case loses nothing.
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    let (h, n) = (haystack.as_bytes(), needle.as_bytes());
    !n.is_empty() && h.len() >= n.len() && h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// A cached pass prediction for one satellite.
pub struct SatPasses {
    pub norad_id: u64,
    pub name: String,
    /// The QTH it was computed for, so moving the QTH invalidates it.
    pub qth: (f64, f64),
    /// Wall clock when it was computed, so it can be refreshed as it ages.
    pub computed_unix: f64,
    pub result: sdroxide_solar::PassSearch,
}

/// FT8/FT4 activity, republished into the window each frame by the root pass.
///
/// The decode list lives in `SdroxideApp`, which this window cannot borrow, so
/// the positions are copied across rather than shared.
#[derive(Clone, Default)]
pub struct DigiTraffic {
    /// Decoded stations: where they are, how brightly, and who they are.
    pub stations: Vec<crate::digi_map::DigiStation>,
    /// The station currently being worked.
    pub dx: Option<(f64, f64)>,
    /// A name for the `dx` end, when it is a named transmitter rather than a
    /// callsign the operator can already read off the panel — a weather-fax
    /// station or a broadcast station. `None` leaves the arc unlabelled, which
    /// is right for a QSO: the call is on screen twice already.
    pub dx_label: Option<String>,
    /// What the card over the arc says about the contact in progress. `None`
    /// when the arc is not a QSO — a weather-fax or broadcast transmitter has
    /// no reports to exchange and no clock running on it.
    pub qso: Option<QsoInfo>,
    /// A decode the operator has clicked but not yet answered.
    pub preview: Option<(f64, f64)>,
    /// True while transmitting, which animates the arc.
    pub transmitting: bool,
    /// The last hour of located decodes, oldest first — what the activity
    /// time-lapse replays. Shared rather than copied: republishing a few
    /// thousand of these every frame is the one part of this that would cost
    /// anything.
    pub history: std::sync::Arc<Vec<crate::digi_map::DigiHit>>,
}

/// The contact in progress, as the card over the arc's apex reports it.
///
/// Assembled where the traffic layer is built, because that is the one place
/// holding both halves at once: the sequencer's live state and the operator's
/// own grid and dial. Every field is optional in effect — a QSO that has not
/// got as far as exchanging reports still deserves a card, and it simply says
/// less.
#[derive(Clone, Default, PartialEq)]
pub struct QsoInfo {
    /// The station being worked. The card's heading.
    pub call: String,
    /// Mode label — "FT8", "FT4", "JS8".
    pub mode: String,
    /// Their grid square, when they have sent one.
    pub grid: Option<String>,
    /// DXCC entity resolved from the callsign.
    pub entity: Option<&'static str>,
    /// Great-circle distance between the two grids, in km.
    pub distance_km: Option<f64>,
    /// Initial bearing from us to them, in degrees true.
    pub bearing_deg: Option<f64>,
    /// Unix seconds the contact began, for the running clock.
    pub started_utc: Option<i64>,
    /// The report we sent them.
    pub rpt_sent: Option<i16>,
    /// The report they sent us.
    pub rpt_rcvd: Option<i16>,
    /// Their signal at us in the most recent slot they were decoded in — the
    /// live number, which keeps moving after the reports are settled.
    pub snr_db: Option<i16>,
    /// ADIF band name from the dial.
    pub band: String,
}

impl SolarUi {
    pub fn new(mut view: Solar3dView) -> Self {
        // A layer mask persisted before a layer existed would leave that layer
        // off for anyone upgrading, which reads as the feature being broken. Any
        // mask that was "everything" at the time becomes "everything" now.
        if crate::view::solar_layer::PREVIOUS_ALL.contains(&view.layers) {
            view.layers = crate::view::solar_layer::ALL;
        }
        SolarUi {
            view,
            #[cfg(not(target_arch = "wasm32"))]
            close_requested: false,
            refresh_requested: false,
            sat_lock: None,
            #[cfg(not(target_arch = "wasm32"))]
            lock_requested: None,
            #[cfg(not(target_arch = "wasm32"))]
            unlock_requested: false,
            qth_grid: String::new(),
            qth: None,
            sim_offset_s: 0.0,
            data: None,
            digi: DigiTraffic::default(),
            qso_card: None,
            awards: Default::default(),
            prop: Default::default(),
            prop_rgba: Default::default(),
            prop_rgba_key: None,
            prop_gen: 0,
            #[cfg(not(target_arch = "wasm32"))]
            window_seed: None,
            #[cfg(not(target_arch = "wasm32"))]
            window_now: None,
            lapse_back_s: 0.0,
            lapse_playing: false,
            search: String::new(),
            sat_cfg: Default::default(),
            selected_sat: None,
            sat_passes: None,
            focus_override: None,
            tour: super::camera::Tour::default(),
            last_frame_time: 0.0,
            retarget: false,
            menu_since: [None; MENUS],
        }
    }

    /// Adopt the operator's grid square, re-decoding only when it changes.
    pub fn set_qth(&mut self, grid: &str) {
        if self.qth_grid == grid {
            return;
        }
        self.qth_grid = grid.to_string();
        self.qth = sdroxide_types::grid_to_latlon(grid);
    }

    pub fn focus(&self) -> Focus {
        Focus::from_id(self.view.focus)
    }

    /// Point the camera at a body, from the picker or from a click in the view.
    ///
    /// Cancels the tour — the tour drives the target itself, so a user choosing
    /// one has to win — and asks the next frame to close the distance.
    pub fn set_focus(&mut self, f: Focus) {
        if self.focus() != f {
            self.retarget = true;
        }
        self.view.focus = f.to_id();
        self.view.auto = false;
    }

    /// True when the activity replay head is at the present moment, which is
    /// where it stays unless the operator winds it back.
    pub fn lapse_live(&self) -> bool {
        self.lapse_back_s <= 0.0
    }

    /// Wall clock the replay head is showing, given the scene's timestamp.
    ///
    /// It is offset from the scene's own clock rather than from the real one,
    /// so scrubbing the whole scene with the Time chips carries the replay with
    /// it instead of leaving the traffic behind at a Sun that has moved.
    pub fn lapse_head(&self, sim_now: f64) -> f64 {
        sim_now - self.lapse_back_s.max(0.0)
    }

    /// How long a decode's arc stays on the globe behind the head, in seconds.
    pub fn lapse_trail_s(&self) -> f64 {
        (self.view.lapse_trail_min as f64 * 60.0).clamp(30.0, crate::digi_map::HISTORY_S as f64)
    }

    /// Park the replay head `back_s` seconds before now, clamped to the hour of
    /// history that exists.
    pub fn set_lapse_back(&mut self, back_s: f64) {
        self.lapse_back_s = back_s.clamp(0.0, crate::digi_map::HISTORY_S as f64);
    }

    /// Whether a satellite matches what is in the find box.
    ///
    /// Case-insensitive substring on the name, and on the catalogue number as
    /// text so `25544` finds the ISS. An empty box matches *nothing* rather
    /// than everything: this drives a highlight, and highlighting all ninety
    /// would be the same as highlighting none.
    pub fn sat_hit(&self, name: &str, norad_id: u64) -> bool {
        let q = self.search.trim();
        if q.is_empty() {
            return false;
        }
        contains_ignore_ascii_case(name, q) || contains_ignore_ascii_case(&norad_id.to_string(), q)
    }

    /// Whether a small body matches what is in the find box. Same rule as
    /// [`SolarUi::sat_hit`], over the body's name and full designation — see
    /// [`sdroxide_solar::SmallBody::matches`].
    pub fn small_hit(&self, b: &sdroxide_solar::SmallBody) -> bool {
        b.matches(&self.search)
    }

    /// Every small body the find box currently picks out.
    pub fn small_hits(&self) -> impl Iterator<Item = (usize, &'static sdroxide_solar::SmallBody)> {
        sdroxide_solar::smallbody::search(&self.search)
    }

    pub fn layer(&self, bit: u32) -> bool {
        self.view.layers & bit != 0
    }

    /// Turn every bit in `mask` on or off together.
    ///
    /// One chip may stand for more than one layer — `SUN OBS` is the sunspots
    /// and the flares — and XOR is the wrong operator for that. A mask holding
    /// only one of a pair, which any settings file written before the two chips
    /// merged may well hold, would flip to holding only the *other* one. A chip
    /// like that is lit when any of its bits are set, so clicking it has to mean
    /// "all of you, off".
    pub fn set_layers(&mut self, mask: u32, on: bool) {
        if on {
            self.view.layers |= mask;
        } else {
            self.view.layers &= !mask;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The camera target is persisted as an integer, so its encoding is a file
    /// format: every target has to survive the round trip, and the ones that
    /// existed before each later addition have to keep their original values or
    /// an upgrade would silently move everyone's camera.
    #[test]
    fn every_target_round_trips_through_its_persisted_id() {
        for f in Focus::all() {
            assert_eq!(Focus::from_id(f.to_id()), f, "{f:?} did not survive");
        }
        for (i, f) in Focus::NEAR.iter().enumerate() {
            assert_eq!(f.to_id() as usize, i, "{f:?} moved off its historical index");
        }
        // The planets and moons keep the ids they were written with when the
        // field was still a byte — everything new had to go above 255.
        assert_eq!(Focus::Planet(sdroxide_solar::Planet::Mercury).to_id(), 4);
        assert_eq!(Focus::Satellite(0).to_id(), 4 + 7);
        assert!(Focus::Satellite(sdroxide_solar::planets::MOONS.len() - 1).to_id() < 256);
        assert!(Focus::Small(0).to_id() >= 256);
        // Distinct ids, or two bodies would share a slot.
        let mut seen: Vec<u16> = Focus::all().iter().map(|f| f.to_id()).collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), count, "two targets encode to the same id");
    }

    /// A stored id from a *newer* build, or from a corrupt file, must land on
    /// something rather than on a body that is not the one meant.
    #[test]
    fn an_unknown_target_id_falls_back_to_the_sun() {
        assert_eq!(Focus::from_id(u16::MAX), Focus::Sun);
        assert_eq!(Focus::from_id(200), Focus::Sun);
        assert_eq!(
            Focus::from_id(256 + sdroxide_solar::smallbody::BODIES.len() as u16),
            Focus::Sun
        );
    }

    #[test]
    fn the_picker_lists_every_body_with_its_planet() {
        let all = Focus::all();
        // The four near targets, seven planets, every moon in the table, and
        // every small body.
        assert_eq!(
            all.len(),
            4 + 7 + sdroxide_solar::planets::MOONS.len() + sdroxide_solar::smallbody::BODIES.len()
        );
        // Each planet is immediately followed by its own moons.
        let jupiter = all.iter().position(|f| *f == Focus::Planet(sdroxide_solar::Planet::Jupiter));
        let after = &all[jupiter.expect("Jupiter is in the list") + 1..][..4];
        for f in after {
            let Focus::Satellite(i) = f else { panic!("{f:?} is not a moon") };
            assert_eq!(sdroxide_solar::planets::MOONS[*i].parent, sdroxide_solar::Planet::Jupiter);
        }
        assert_eq!(after[0].label(), "Io");
    }

    /// The search has to find a satellite by any part of its designator or by
    /// its catalogue number, and an empty box has to match nothing.
    #[test]
    fn the_search_matches_names_and_catalogue_numbers() {
        let mut st = SolarUi::new(Solar3dView::default());
        // Nothing typed: nothing highlighted, or every satellite would be.
        assert!(!st.sat_hit("ISS", 25544));
        st.search = "   ".into();
        assert!(!st.sat_hit("ISS", 25544));

        st.search = "iss".into();
        assert!(st.sat_hit("ISS", 25544));
        assert!(!st.sat_hit("AO-73", 39444));
        // Substrings anywhere, in either case.
        st.search = "o-7".into();
        assert!(st.sat_hit("AO-73", 39444));
        assert!(st.sat_hit("ao-7", 7530));
        assert!(!st.sat_hit("RS-44", 44909));
        // ...and by catalogue number, which is how you find one whose
        // designator you cannot remember.
        st.search = "25544".into();
        assert!(st.sat_hit("ISS", 25544));
        assert!(!st.sat_hit("ISS", 25545));
        // Surrounding whitespace is not part of the query.
        st.search = "  QO-100 ".into();
        assert!(st.sat_hit("QO-100", 43700));
        // A query longer than the name cannot match it.
        st.search = "QO-100-AND-MORE".into();
        assert!(!st.sat_hit("QO-100", 43700));
    }

    #[test]
    fn choosing_a_target_stops_the_tour_and_asks_for_a_reframe() {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        st.set_focus(Focus::Planet(sdroxide_solar::Planet::Saturn));
        assert!(!st.view.auto, "the tour kept driving after the user picked a target");
        assert!(st.retarget);
        assert_eq!(st.focus(), Focus::Planet(sdroxide_solar::Planet::Saturn));

        // Picking the body that is already the target is not a reframe: it
        // would yank the camera back out of a close-up the user zoomed into.
        st.retarget = false;
        st.set_focus(Focus::Planet(sdroxide_solar::Planet::Saturn));
        assert!(!st.retarget);
    }
}
