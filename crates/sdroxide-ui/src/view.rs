//! Client-local display state (not part of the shared radio state).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewState {
    /// Visible frequency window; 0/0 means "fit the full device span".
    pub view_lo_hz: f64,
    pub view_hi_hz: f64,
    /// Which front end the window above was fitted against
    /// ([`sdroxide_types::DeviceCaps::driver`]).
    ///
    /// A zoom only means anything against the span it was taken in. Opening a
    /// different interface on the same radio leaves the old one's window
    /// behind, and the arithmetic that keeps a view honest only ever *narrows*
    /// it — so a 12 kHz KiwiSDR window carried straight into an 800 kHz
    /// SpyServer session and stayed there, a 70× zoom nobody asked for and no
    /// gesture undid short of Fit. Remembered here rather than in memory so
    /// that a restart on the *same* interface still restores the operator's
    /// zoom, which is the whole point of persisting the window at all.
    #[serde(default)]
    pub driver: String,
    /// Voice-mode window saved on entering FT8/FT4 (which locks the view to the
    /// narrow sub-band), restored on leaving so the panadapter isn't left stuck
    /// zoomed in.
    ///
    /// Persisted with the rest of the view, not held only in memory: quitting
    /// from a digital mode would otherwise store the sub-band window as *the*
    /// zoom, and the next voice mode would come up zoomed into 3.7 kHz of a
    /// band the operator had been looking at whole.
    #[serde(default)]
    pub pre_digi_view: Option<(f64, f64)>,
    pub db_floor: f32,
    pub db_ceil: f32,
    /// Keep [`ViewState::db_floor`]/[`ViewState::db_ceil`] fitted to what the
    /// receiver is actually hearing, rather than only when FIT is clicked: on a
    /// band change, once a pan or zoom has settled, and whenever the levels
    /// have drifted far enough that the waterfall has gone flat or blown out.
    /// See `app::spectrum::auto_fit_tick` for the triggers and the interval.
    ///
    /// On by default — an operator who wants the levels held where they put
    /// them switches it off, which is also what the FIT chip now shows.
    #[serde(default = "auto_fit_default")]
    pub auto_fit: bool,
    pub fft_size: u32,
    /// Fraction of the panadapter height used by the spectrum line (rest = waterfall).
    pub spectrum_fraction: f32,
    /// Draw a decaying peak-hold trace over the spectrum.
    pub peak_hold: bool,
    /// Draw the spectrum strip as a receding 3D surface instead of a flat
    /// line: the newest spectrum across the front, the ones before it flowing
    /// away from the viewer. See [`crate::widgets::spectrum3d`].
    ///
    /// Off by default — the flat line is the display everything else in the
    /// panadapter is measured against, and the surface costs the strip's whole
    /// height to say the same thing with time in it.
    #[serde(default)]
    pub spectrum_3d: bool,
    /// How that surface is drawn: `true` fills it and colours it by the
    /// waterfall palette, `false` draws each remembered spectrum as a trace
    /// over an opaque fill.
    ///
    /// Solid by default: it is the rendering that puts the level in the colour
    /// as well as in the shape, and it reuses the palette the operator has
    /// already picked for the waterfall, so the two halves of the panadapter
    /// agree about what a strong signal looks like.
    #[serde(default = "spectrum_3d_solid_default")]
    pub spectrum_3d_solid: bool,
    /// Keep the tuned frequency in the middle of the panadapter: every time
    /// the dial moves, the window slides under it so the marker stays put
    /// (issue #174).
    ///
    /// Off is the display as it always was — the window holds still and the
    /// marker travels across it until the dial leaves the span, at which point
    /// the picture jumps a whole window. That jump is what this exists to
    /// remove: with it on the band scrolls past a fixed marker instead.
    ///
    /// On for a new station (issues #411, #422): operators arriving from other
    /// SDR programs expect the dial in the middle and read the jump as a fault.
    /// A saved view keeps whatever it was left on.
    ///
    /// Zoomed in the slide is free — the window is a viewport onto a wider
    /// captured span. Zoomed all the way out there is nowhere left to slide,
    /// so the front end's own centre is moved instead, exactly as a drag that
    /// reaches the edge already does (issue #133). A receiver whose centre
    /// *is* its dial has nothing to do either way: it is already centred.
    #[serde(default = "center_on_vfo_default")]
    pub center_on_vfo: bool,
    /// Hide the spectrum line, showing only the waterfall (and, in FT8/FT4,
    /// giving the freed height to the operating panel).
    pub spectrum_collapsed: bool,
    /// Hide the waterfall, giving its height to the spectrum line — the other
    /// half of the pair the SPEC popup switches.
    ///
    /// Independent of [`ViewState::spectrum_collapsed`], both directions: the
    /// four combinations are spectrum, waterfall, both and neither, and the
    /// last one is a display the popup can be left in on purpose. With neither
    /// layer the panadapter is not drawn at all (see `app::frame`) — the
    /// operating panel takes the height in a mode that has one — and the SPEC
    /// chip in the Display module is the way back.
    #[serde(default)]
    pub waterfall_collapsed: bool,
    /// Scroll the waterfall upwards: the newest row sits at the *bottom* and
    /// history flows up and off the top, the way several other SDR programs
    /// draw it. Affects the time gridlines and the spot lanes too.
    pub waterfall_flip: bool,
    /// Label decoded stations on the waterfall: one box per callsign at the
    /// frequency it was heard on, in the slotted modes (FT8, FT4, FT2, JS8, the
    /// contest ones).
    ///
    /// On by default — putting a decode where it came from is most of the
    /// reason the waterfall is worth looking at in those modes. It is also the
    /// busiest overlay there is: a good opening puts thirty boxes across the
    /// span, twice a minute, over exactly the traces the operator is reading. A
    /// list of every one of them is already down the side of the same screen,
    /// so switching the labels off loses nothing but the crowding (issue #248).
    #[serde(default = "decode_labels_default")]
    pub decode_labels: bool,
    /// Show the full-band strip above the panadapter, where the front end
    /// supplies one. Purely a display switch: the source keeps producing the
    /// frames while it is off, which is what keeps the chip that controls it on
    /// screen (its presence is the evidence that this receiver has a full-band
    /// lane at all).
    #[serde(default = "wide_waterfall_default")]
    pub wide_waterfall: bool,
    /// Draw the propagation heat under the operating panel's flat map.
    ///
    /// Here and not in [`Solar3dView`], even though the globe has the same
    /// idea, because that struct belongs to the globe window: the root pass
    /// republishes it wholesale every frame (`view.solar3d = solar.persisted()`
    /// in `frame`), so anything the panel wrote into it would be overwritten a
    /// frame later — which showed up as the heat appearing for an instant and
    /// vanishing. The two views keep their own display choice; the *field* they
    /// draw is the one shared thing, and that lives in `PropStore`.
    ///
    /// Off by default: a study aid rather than something to leave switched on
    /// over a decode list.
    #[serde(default)]
    pub prop_on_map: bool,
    /// The panel map's display: 0 = one band through the ramp, 1 = every live
    /// band at once, one hue each.
    #[serde(default = "prop_map_mode_default")]
    pub prop_map_mode: u8,
    /// The band the panel map's single-band display shows, as an index into
    /// `Band::ALL`.
    #[serde(default = "prop_map_band_default")]
    pub prop_map_band: u8,
    /// Which spot kinds are shown in the SPOTS list, on the panadapter and on
    /// the world map — indexed by `SpotKind::index`, so the chip order in the
    /// SPOTS window and this array have to stay in lockstep.
    #[serde(default = "spot_kinds_default")]
    pub spot_kinds_shown: [bool; SPOT_KINDS],
    /// Fraction of the FT8/FT4 layout height used by the operating panel (the
    /// decode list + QSO area); the rest is the waterfall. User-draggable.
    pub digi_panel_fraction: f32,
    /// Fraction of the FT8/FT4 panel width used by the decode table; the rest is
    /// the map/QSO area. User-draggable.
    pub digi_split_fraction: f32,
    /// Fraction of the JS8 panel width given to the activity list; the rest is
    /// the conversation. Separate from [`ViewState::digi_split_fraction`]
    /// because the two panels want different balances — JS8's right column is a
    /// chat log, FT8's is a map and a station card.
    #[serde(default = "js8_split_default")]
    pub js8_split_fraction: f32,
    /// Fraction of the ADS-B panel's width given to the aircraft list; the rest
    /// is the map. User-draggable.
    #[serde(default = "adsb_split_default")]
    pub adsb_split_fraction: f32,
    /// Fraction of the aircraft column's height given to the detail card of the
    /// selected target; the rest is the list. User-draggable, and only divides
    /// anything while a target is selected.
    #[serde(default = "adsb_card_default")]
    pub adsb_card_fraction: f32,
    /// Fraction of the AIS panel's width given to the vessel list; the rest is
    /// the chart.
    #[serde(default = "ais_split_default")]
    pub ais_split_fraction: f32,
    /// ...and of its height given to the detail card, while one is open.
    #[serde(default = "ais_card_default")]
    pub ais_card_fraction: f32,
    /// Fraction of the QSO area's height given to the world map; the rest is the
    /// station card + transcript + buttons. User-draggable.
    ///
    /// Desktop only: the map is the first thing a compact layout gives up, so
    /// there is nothing for this to divide there.
    pub digi_map_fraction: f32,
    /// Which of the digital panel's views a phone is showing, as an index into
    /// the mode's own list (see `panels::panel_tabs`) — the panes differ by
    /// mode, and the waterfall is always the last of them. They do not fit
    /// together at a phone's width, so they take turns.
    ///
    /// Clamped on use rather than on load: the list is shorter in some modes
    /// than others, and a stored index is only meaningful against the mode it
    /// was stored in. Replaces an older `digi_tab` enum, which a stored blob
    /// may still carry; serde ignores it and everyone starts on the first pane.
    #[serde(default)]
    pub digi_pane: usize,
    /// Fraction of the SSTV panel width given to the TRANSMIT (send) column; the
    /// rest is the receive side (LIVE + RECEIVED). User-draggable.
    pub sstv_tx_fraction: f32,
    /// Fraction of the SSTV receive side given to the RECEIVED gallery; the rest
    /// is the LIVE image. User-draggable.
    pub sstv_gallery_fraction: f32,
    /// Fraction of the WEFAX panel width given to the SAVED gallery; the rest is
    /// the chart itself. User-draggable.
    #[serde(default = "wefax_gallery_default")]
    pub wefax_gallery_fraction: f32,
    /// Hellschreiber raster appearance. Client-side rather than in `DigiConfig`
    /// because the panel keeps the raw grays: changing contrast repaints the
    /// whole scrollback, which engine-side shading could never do.
    pub hell: HellView,
    /// Solar-system 3D window: open state, camera and layer selection. The
    /// window itself is native-only, but this rides in `ViewState` on both
    /// targets so the persisted blob stays identical across builds.
    pub solar3d: Solar3dView,
    /// Which programme-type table the RDS window reads a station against.
    ///
    /// A display preference rather than anything the radio does — the raw codes
    /// travel either way — so it lives here, per radio, and takes effect on what
    /// has already been received.
    #[serde(default)]
    pub rds_standard: sdroxide_types::RdsStandard,
}

/// Layer visibility bits for [`Solar3dView::layers`].
pub mod solar_layer {
    pub const ORBITS: u32 = 1 << 0;
    pub const CME: u32 = 1 << 1;
    pub const SPOTS: u32 = 1 << 2;
    pub const FLARES: u32 = 1 << 3;
    /// Retired: the heliographic graticule and the star field are always drawn
    /// now, and nothing reads these two bits any more. They are kept so the
    /// other layers stay on the bit positions everyone's settings file already
    /// uses, and so the historical [`PREVIOUS_ALL`] masks still match.
    pub const GRID: u32 = 1 << 4;
    pub const LABELS: u32 = 1 << 5;
    /// Retired — see [`GRID`].
    pub const STARS: u32 = 1 << 6;
    /// Decoded FT8/FT4 stations and the arc to the station being worked.
    pub const QSO: u32 = 1 << 7;
    /// Amateur-radio satellites and their orbits.
    pub const SATS: u32 = 1 << 8;
    /// The auroral oval and its equatorward edge.
    pub const AURORA: u32 = 1 << 9;
    /// The other planets, their moons and their ring systems.
    pub const PLANETS: u32 = 1 << 10;
    /// Cloud cover and the lightning inside it.
    pub const CLOUDS: u32 = 1 << 12;
    /// Names on the asteroids and comets.
    ///
    /// Separate from [`LABELS`] because it is a different question. `LABELS`
    /// is "do I want names at all"; this is "do I want thirty-five asteroid
    /// designations *as well*", and the answer differs by what you are looking
    /// at. Whatever the state of it, the body the camera is pointed at and
    /// anything the find box has matched keep their names — those were asked
    /// for by name, and withholding them would read as the search being broken.
    pub const SMALL_LABELS: u32 = 1 << 13;
    /// Award coverage: which DXCC entities are still missing from the log.
    ///
    /// Deliberately absent from [`ALL`], and so off until it is asked for: it
    /// puts a marker on all three hundred-odd DXCC entities, which is a study
    /// aid rather than something to leave switched on over the orrery.
    pub const AWARDS: u32 = 1 << 11;
    /// Where signals are actually getting through, per band — the propagation
    /// heat map.
    ///
    /// Absent from [`ALL`] for the reason [`AWARDS`] is: it paints the whole
    /// Earth and is a study aid rather than scenery. Because `ALL` therefore
    /// does not change, [`PREVIOUS_ALL`] needs no new entry — adding one would
    /// switch this on for everybody who upgrades.
    pub const PROPAGATION: u32 = 1 << 14;
    /// Sunspot regions and flare sources together, as the `SUN OBS` chip shows
    /// them. Not a layer: [`SPOTS`] and [`FLARES`] are still drawn and tested
    /// separately, and both bits keep their positions so every settings file
    /// already written still means what it meant. This is only the pair of them
    /// that one button stands for.
    pub const SUN_OBS: u32 = SPOTS | FLARES;
    /// Every layer that is on by default.
    pub const ALL: u32 = ORBITS
        | CME
        | SPOTS
        | FLARES
        | GRID
        | LABELS
        | STARS
        | QSO
        | SATS
        | AURORA
        | PLANETS
        | CLOUDS
        | SMALL_LABELS;
    /// Values `ALL` has had in earlier versions. A stored mask equal to one of
    /// these was "everything" when it was written, so it is upgraded rather
    /// than leaving newly added layers silently switched off.
    #[allow(dead_code)]
    pub const PREVIOUS_ALL: [u32; 6] = [
        ORBITS | CME | SPOTS | FLARES | GRID | LABELS | STARS,
        ORBITS | CME | SPOTS | FLARES | GRID | LABELS | STARS | QSO,
        ORBITS | CME | SPOTS | FLARES | GRID | LABELS | STARS | QSO | SATS,
        ORBITS | CME | SPOTS | FLARES | GRID | LABELS | STARS | QSO | SATS | AURORA,
        ORBITS | CME | SPOTS | FLARES | GRID | LABELS | STARS | QSO | SATS | AURORA | PLANETS,
        ORBITS
            | CME
            | SPOTS
            | FLARES
            | GRID
            | LABELS
            | STARS
            | QSO
            | SATS
            | AURORA
            | PLANETS
            | CLOUDS,
    ];
}

/// Persisted appearance of the Hellschreiber receive raster.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HellView {
    /// Gamma applied to the received grays. Higher is harder/more contrasty.
    pub contrast: f32,
    /// Linear gain applied before the gamma.
    pub bright: f32,
    /// Reverse video: light dots on dark paper instead of the fldigi look.
    pub reverse: bool,
    /// Screen pixels per received column. Square dots would fit only ~18
    /// characters across a wide panel, so the horizontal scale is independent.
    pub col_px: f32,
    /// Draw every column twice, stacked. Hell's vertical phase free-runs, so
    /// this is what guarantees one complete legible copy is always on screen.
    pub doubled: bool,
    /// Vertical alignment, 0..1, when `doubled` is off and the operator lines
    /// the text up by hand.
    pub valign: f32,
}

impl Default for HellView {
    fn default() -> Self {
        HellView {
            contrast: 1.6,
            bright: 1.0,
            reverse: false,
            col_px: 2.0,
            doubled: true,
            valign: 0.0,
        }
    }
}

/// Persisted state of the solar-system 3D window.
///
/// Deliberately free of any `eframe`/`wgpu` type so it compiles on wasm32 —
/// `ViewState` must serialize identically on both targets.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Solar3dView {
    /// Window open state, restored on the next launch.
    pub open: bool,
    /// Camera focus body (see `solar3d::state::Focus`).
    ///
    /// Widened from `u8` when the small bodies arrived: there are more targets
    /// than a byte holds, and the encoding keeps every value it ever wrote, so
    /// a settings file from before the change still points at the same body.
    /// TOML and JSON both store an integer without a width, so nothing has to
    /// be migrated.
    pub focus: u16,
    /// Orbit camera: yaw/pitch in radians, distance in gigametres.
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    /// Whether the animated camera tour is running.
    pub auto: bool,
    /// SDO channel index (see `sdroxide_solar::SdoChannel`).
    pub channel: u8,
    /// SDO image edge length in pixels (1024 or 2048).
    pub resolution: u16,
    /// Radius exaggeration for Earth and Moon (positions are never scaled).
    pub body_scale: f32,
    /// Exaggeration of the Earth→Moon distance.
    pub moon_orbit_scale: f32,
    /// Radius exaggeration for the Sun.
    pub sun_scale: f32,
    /// Layer visibility bits — see [`solar_layer`].
    pub layers: u32,
    /// Draw the clouds by marching the volume instead of stacking shells.
    /// Truer light — a flash glows *through* a tower rather than merely
    /// brightening it — at several times the cost per pixel, so the cheap path
    /// is the default and this is the switch for people whose GPU can spare it.
    pub cloud_march: bool,
    /// How far back CMEs are kept on screen, in hours.
    pub cme_window_h: f32,
    /// Show every satellite in the element set rather than the curated few.
    pub all_satellites: bool,
    /// Activity time-lapse: how long an FT8/FT4 decode's arc stays on the
    /// globe behind the replay head, in minutes.
    pub lapse_trail_min: f32,
    /// How many times real time the activity replay runs at.
    pub lapse_speed: f32,
    /// Which propagation picture the heat layer draws: 0 = one band through the
    /// blue→red ramp, 1 = every selected band at once, one hue each.
    #[serde(default = "prop_mode_default")]
    pub prop_mode: u8,
    /// The band the per-band display shows, as an index into `Band::ALL`.
    #[serde(default = "prop_band_default")]
    pub prop_band: u8,
    /// Which bands the combined display sums, one bit per `Band::ALL` index.
    ///
    /// Thirty-two bits rather than sixteen because `Band::ALL` outgrew sixteen
    /// entries when the microwave bands arrived: at u16, 23 cm and everything
    /// above it had no bit to be selected by.
    #[serde(default = "prop_bands_default")]
    pub prop_bands: u32,
    /// Half-life of an observation's contribution, in minutes. The ionosphere's
    /// memory is short; an hour-old spot should not out-argue a two-minute one.
    #[serde(default = "prop_halflife_default")]
    pub prop_halflife_min: f32,
    /// Which sources feed the propagation field, one bit per index into
    /// `PropSource::ALL`.
    ///
    /// Lives here rather than beside the store because the globe's menu edits
    /// it and the flat map reads it: one setting, both views, and no way for
    /// them to be looking at different evidence.
    #[serde(default = "prop_sources_default")]
    pub prop_sources: u16,
}

/// Default for [`Solar3dView::prop_mode`] — the combined view, which reads
/// without being configured first.
fn prop_mode_default() -> u8 {
    1
}

/// Default for [`Solar3dView::prop_band`] — 20 m, the band most stations sit on.
fn prop_band_default() -> u8 {
    sdroxide_types::Band::ALL.iter().position(|b| *b == sdroxide_types::Band::M20).unwrap_or(0)
        as u8
}

/// Default for [`Solar3dView::prop_bands`] — every band with a hue, i.e. all of
/// them except the general-coverage placeholder.
fn prop_bands_default() -> u32 {
    let mut m = 0u32;
    for (i, b) in sdroxide_types::Band::ALL.iter().enumerate() {
        if *b != sdroxide_types::Band::Gen {
            m |= 1 << i;
        }
    }
    m
}

/// Default for [`Solar3dView::prop_sources`] — everything. Each source is a
/// real observation of a real path, and a map that quietly ignored the logbook
/// would be a map of the last hour rather than of the station.
fn prop_sources_default() -> u16 {
    (1u16 << sdroxide_types::PropSource::ALL.len()) - 1
}

impl Solar3dView {
    /// Bring a stored view forward to the sources this build knows about.
    ///
    /// `prop_sources` is a bitmask over `PropSource::ALL`, so a settings file
    /// written before a source was added has that source's bit clear — which
    /// is indistinguishable, on the wire, from the operator having switched it
    /// off. It is distinguishable by what it *was*: with `n` sources, "all of
    /// them" is exactly `2ⁿ - 1`, and anything less is a deliberate choice. So
    /// a mask that was all-on for a shorter list is carried forward as all-on
    /// rather than as a filter nobody set.
    ///
    /// Without this, enabling the Reverse Beacon Network in settings would
    /// show nothing at all until the operator also found a chip they never
    /// touched — the feed would look broken when it was only hidden.
    fn migrate_prop_sources(&mut self) {
        let now = prop_sources_default();
        for shorter in 1..sdroxide_types::PropSource::ALL.len() {
            if self.prop_sources == (1u16 << shorter) - 1 {
                self.prop_sources = now;
                return;
            }
        }
    }
}

impl ViewState {
    /// Called once, on the state restored from disk, before anything reads it.
    pub fn migrate(&mut self) {
        self.solar3d.migrate_prop_sources();
    }
}

/// Default for [`Solar3dView::prop_halflife_min`].
fn prop_halflife_default() -> f32 {
    (sdroxide_types::PROP_DEFAULT_HALFLIFE_S / 60.0) as f32
}

impl Default for Solar3dView {
    fn default() -> Self {
        Solar3dView {
            open: false,
            focus: 0,
            // A three-quarter view from above that frames the whole Earth orbit.
            yaw: 0.6,
            pitch: 0.55,
            dist: 360.0,
            auto: false,
            // HMI continuum: white light, so real sunspots are visible.
            channel: 0,
            resolution: 1024,
            body_scale: 20.0,
            moon_orbit_scale: 1.0,
            sun_scale: 1.0,
            layers: solar_layer::ALL,
            cloud_march: false,
            cme_window_h: 72.0,
            all_satellites: false,
            // Ten minutes of trail is about forty FT8 slots: enough for the
            // band's shape to show, short enough that one opening does not
            // smear into the next.
            lapse_trail_min: 10.0,
            lapse_speed: 60.0,
            prop_mode: prop_mode_default(),
            prop_band: prop_band_default(),
            prop_bands: prop_bands_default(),
            prop_sources: prop_sources_default(),
            prop_halflife_min: prop_halflife_default(),
        }
    }
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            view_lo_hz: 0.0,
            view_hi_hz: 0.0,
            driver: String::new(),
            pre_digi_view: None,
            rds_standard: sdroxide_types::RdsStandard::default(),
            db_floor: -120.0,
            db_ceil: -20.0,
            auto_fit: auto_fit_default(),
            fft_size: 4096,
            spectrum_fraction: 0.35,
            peak_hold: false,
            spectrum_3d: false,
            spectrum_3d_solid: spectrum_3d_solid_default(),
            center_on_vfo: center_on_vfo_default(),
            spectrum_collapsed: false,
            waterfall_collapsed: false,
            waterfall_flip: false,
            decode_labels: decode_labels_default(),
            wide_waterfall: wide_waterfall_default(),
            prop_on_map: false,
            prop_map_mode: prop_map_mode_default(),
            prop_map_band: prop_map_band_default(),
            spot_kinds_shown: spot_kinds_default(),
            digi_panel_fraction: 0.46,
            digi_split_fraction: 0.52,
            js8_split_fraction: js8_split_default(),
            adsb_split_fraction: adsb_split_default(),
            adsb_card_fraction: adsb_card_default(),
            ais_split_fraction: ais_split_default(),
            ais_card_fraction: ais_card_default(),
            digi_map_fraction: 0.6,
            digi_pane: 0,
            sstv_tx_fraction: 0.38,
            sstv_gallery_fraction: 0.4,
            wefax_gallery_fraction: wefax_gallery_default(),
            hell: HellView::default(),
            solar3d: Solar3dView::default(),
        }
    }
}

impl ViewState {
    /// Whether the spectrum line is drawn.
    pub fn spectrum_visible(&self) -> bool {
        !self.spectrum_collapsed
    }

    /// Whether the waterfall is drawn.
    pub fn waterfall_visible(&self) -> bool {
        !self.waterfall_collapsed
    }

    /// Whether the panadapter is drawn at all. False with both layers switched
    /// off, which is what tells the layout to skip the widget and hand its
    /// height to whatever else the pane holds.
    pub fn panadapter_visible(&self) -> bool {
        self.spectrum_visible() || self.waterfall_visible()
    }

    /// Show or hide the spectrum line.
    pub fn set_spectrum_visible(&mut self, on: bool) {
        self.spectrum_collapsed = !on;
    }

    /// Show or hide the waterfall.
    pub fn set_waterfall_visible(&mut self, on: bool) {
        self.waterfall_collapsed = !on;
    }

    /// Effective spectrum-height fraction: 0 with the spectrum hidden, the
    /// whole strip with the waterfall hidden, the operator's split otherwise.
    ///
    /// Pure geometry for the strip the panadapter is given — whether it is
    /// given one at all is [`Self::panadapter_visible`], which the layout
    /// checks first.
    pub fn effective_spectrum_fraction(&self) -> f32 {
        match (self.spectrum_visible(), self.waterfall_visible()) {
            (false, _) => 0.0,
            (_, false) => 1.0,
            _ => self.spectrum_fraction,
        }
    }

    pub fn span(&self) -> f64 {
        self.view_hi_hz - self.view_lo_hz
    }

    pub fn is_unset(&self) -> bool {
        self.span() <= 0.0
    }

    /// Note which front end is now streaming, and drop the window if it is a
    /// different one — see [`ViewState::driver`].
    ///
    /// Dropping means *unsetting*, not fitting: the panadapter fits an unset
    /// view to the device on its first draw, so this hands the decision to the
    /// one place that already makes it rather than inventing a second.
    ///
    /// Returns whether the window was dropped.
    pub fn adopt_driver(&mut self, driver: &str) -> bool {
        if self.driver == driver {
            return false;
        }
        // A first run, or a view stored before this field existed: adopt the
        // driver without touching a window the operator may have left set.
        let known = !self.driver.is_empty();
        self.driver = driver.to_string();
        if known && !self.is_unset() {
            self.view_lo_hz = 0.0;
            self.view_hi_hz = 0.0;
            return true;
        }
        false
    }

    /// Reset to show the whole device passband.
    pub fn fit(&mut self, center_hz: f64, span_hz: f64) {
        self.view_lo_hz = center_hz - span_hz / 2.0;
        self.view_hi_hz = center_hz + span_hz / 2.0;
    }

    /// Clamp the viewport inside the device passband, preserving width.
    pub fn clamp_to(&mut self, center_hz: f64, span_hz: f64) {
        let (lo, hi) = (center_hz - span_hz / 2.0, center_hz + span_hz / 2.0);
        let w = self.span().min(span_hz).max(span_hz / 1000.0);
        if self.view_lo_hz < lo {
            self.view_lo_hz = lo;
            self.view_hi_hz = lo + w;
        }
        if self.view_hi_hz > hi {
            self.view_hi_hz = hi;
            self.view_lo_hz = hi - w;
        }
    }

    pub fn freq_to_x(&self, hz: f64, rect: &eframe::egui::Rect) -> f32 {
        let frac = (hz - self.view_lo_hz) / self.span();
        rect.left() + rect.width() * frac as f32
    }

    pub fn x_to_freq(&self, x: f32, rect: &eframe::egui::Rect) -> f64 {
        let frac = ((x - rect.left()) / rect.width()) as f64;
        self.view_lo_hz + frac * self.span()
    }
}

/// Default for [`ViewState::decode_labels`] — on: the callsign boxes are what
/// makes a slotted mode's waterfall readable, and an operator who finds them
/// busy switches them off.
fn decode_labels_default() -> bool {
    true
}

/// Default for [`ViewState::wide_waterfall`] — on, so a receiver that has a
/// full-band lane shows it without anyone having to find the chip first.
fn wide_waterfall_default() -> bool {
    true
}

/// Default for [`ViewState::spectrum_3d_solid`] — the filled, palette-coloured
/// surface, which is what the 3D chip should show the first time it is clicked.
fn spectrum_3d_solid_default() -> bool {
    true
}

/// Default for [`ViewState::auto_fit`] — on, so the waterfall reads on the
/// first band it is pointed at without anyone having to click FIT.
fn auto_fit_default() -> bool {
    true
}

/// Number of spot-kind filter chips, i.e. the width of
/// [`ViewState::spot_kinds_shown`]. Named here rather than beside the chips
/// because it fixes the shape of the persisted blob, and taken from the kind
/// list itself so the two cannot drift apart.
pub const SPOT_KINDS: usize = sdroxide_types::SpotKind::COUNT;

/// Default for [`ViewState::center_on_vfo`]: on.
fn center_on_vfo_default() -> bool {
    true
}

/// Default for [`ViewState::spot_kinds_shown`] — every kind shown, so enabling
/// a feed is enough to see its spots.
fn spot_kinds_default() -> [bool; SPOT_KINDS] {
    [true; SPOT_KINDS]
}

/// Default for [`ViewState::prop_map_mode`] — every band at once, which reads
/// without being configured first.
fn prop_map_mode_default() -> u8 {
    1
}

/// Default for [`ViewState::prop_map_band`] — 20 m.
fn prop_map_band_default() -> u8 {
    sdroxide_types::Band::ALL.iter().position(|b| *b == sdroxide_types::Band::M20).unwrap_or(0)
        as u8
}

/// Default for [`ViewState::adsb_split_fraction`]. Enough for the aircraft
/// table's full set of columns, the signal one included, and the rest to the
/// map — which is the pane that gains most from whatever is left.
fn adsb_split_default() -> f32 {
    0.42
}

/// Default for [`ViewState::adsb_card_fraction`].
fn adsb_card_default() -> f32 {
    0.40
}

/// Default for [`ViewState::ais_split_fraction`]. A little wider than the
/// aircraft table's share: a vessel's name is twenty characters where a
/// callsign is seven.
fn ais_split_default() -> f32 {
    0.46
}

/// Default for [`ViewState::ais_card_fraction`]. Deeper than the aircraft
/// card's, because a ship's card has a dozen more rows in it — a name, a call
/// sign, an IMO number, dimensions, a draught, a destination and an ETA.
fn ais_card_default() -> f32 {
    0.45
}

/// Default for [`ViewState::js8_split_fraction`].
fn js8_split_default() -> f32 {
    0.46
}

/// Default for [`ViewState::wefax_gallery_fraction`] — the fixed width the
/// gallery had before the divider was draggable.
fn wefax_gallery_default() -> f32 {
    0.18
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The operating panel's propagation settings must not live in
    /// [`Solar3dView`].
    ///
    /// That struct belongs to the globe window, and the root pass republishes
    /// it wholesale every frame — `view.solar3d = solar.persisted()`. A setting
    /// the panel wrote there survived exactly one frame before the globe's copy
    /// overwrote it, which is what turned the panel's PROP chip into a flicker.
    /// This asserts the shape that made that impossible.
    #[test]
    fn the_panels_propagation_settings_survive_the_globe_republishing_its_own() {
        let mut v = ViewState::default();
        v.prop_on_map = true;
        v.prop_map_mode = 0;
        v.prop_map_band = 3;

        // Exactly what `frame` does with what the globe window hands back.
        v.solar3d = Solar3dView::default();

        assert!(v.prop_on_map, "the globe's republish switched the panel's heat off");
        assert_eq!(v.prop_map_mode, 0, "the globe's republish reset the panel's display");
        assert_eq!(v.prop_map_band, 3, "the globe's republish reset the panel's band");
    }

    /// eframe's storage is one blob per key, so a field added here has to be
    /// optional in the stored form or *every* existing panadapter view — zoom,
    /// levels, every panel divider — would fail to parse and reset to defaults
    /// on the version that added it.
    #[test]
    fn a_view_stored_by_an_older_version_still_loads() {
        // A real `"view"` blob, from before the zoom was remembered across a
        // digital-mode session.
        let stored = "(view_lo_hz:14062061.420876093,view_hi_hz:14289352.162287109,\
                      db_floor:-129.32353,db_ceil:-100.0,fft_size:32768,\
                      spectrum_fraction:0.17981522,peak_hold:false)";
        let v: ViewState = ron::from_str(stored).expect("an older view must still parse");
        assert_eq!(v.view_lo_hz, 14_062_061.420876093, "the zoom survives the upgrade");
        assert_eq!(v.fft_size, 32_768);
        assert_eq!(v.pre_digi_view, None, "the new field falls back to its default");
        // Both of these default *on*, which is the whole point: an added filter
        // array that fell back to `[false; N]` would silently hide every spot
        // for everyone who upgrades, and it would look like the feeds broke.
        assert!(v.wide_waterfall, "the full-band strip stays on offer after an upgrade");
        assert_eq!(v.spot_kinds_shown, [true; SPOT_KINDS], "no spot kind is hidden by upgrading");
        assert!(v.auto_fit, "auto-fit is on for everyone, not only for fresh installs");
        // The layer the SPEC popup added: a blob from before it must come back
        // with the waterfall shown, not hidden.
        assert!(v.waterfall_visible(), "upgrading hid the waterfall");
        // The same for the 3D surface behind that popup's 3D chip: an upgrade
        // must land on the flat trace, with the solid rendering waiting behind
        // the chip rather than the wireframe a bare `default` would give.
        assert!(!v.spectrum_3d, "upgrading switched the 3D surface on by itself");
        assert!(v.spectrum_3d_solid, "the 3D surface would have come up as a wireframe");
    }

    /// The other direction of the same trap: a blob written by a build that
    /// *did* keep the S-meter face in the view carries a field this struct no
    /// longer has. Serde must skip it — a hard error here would reset the
    /// operator's zoom, levels and every panel divider on the version that
    /// moved the face into `[ui]` (issue #185).
    #[test]
    fn a_view_that_still_carries_the_old_smeter_face_loads() {
        let stored = "(view_lo_hz:14062061.420876093,view_hi_hz:14289352.162287109,\
                      db_floor:-129.32353,db_ceil:-100.0,fft_size:32768,\
                      spectrum_fraction:0.17981522,peak_hold:false,smeter_style:Trace)";
        let v: ViewState = ron::from_str(stored).expect("a view with the old face must parse");
        assert_eq!(v.fft_size, 32_768, "the rest of the view survived the field moving out");
    }

    /// The same trap as the spot filters above, one layer down: a propagation
    /// source added after a settings file was written must not read as one the
    /// operator switched off, or the feed looks broken when it is only hidden.
    #[test]
    fn an_all_on_source_mask_from_a_shorter_list_stays_all_on() {
        let all_now = prop_sources_default();
        // What a build with six sources wrote when everything was enabled.
        let mut v = ViewState::default();
        v.solar3d.prop_sources = (1 << 6) - 1;
        v.migrate();
        assert_eq!(v.solar3d.prop_sources, all_now, "an upgrade hid a source");

        // A mask the operator actually narrowed is left exactly as it is —
        // migrating that would override a deliberate choice.
        let mut v = ViewState::default();
        v.solar3d.prop_sources = 0b010_1101;
        v.migrate();
        assert_eq!(v.solar3d.prop_sources, 0b010_1101, "a chosen filter was overwritten");

        // And a current all-on mask is already correct.
        let mut v = ViewState::default();
        v.migrate();
        assert_eq!(v.solar3d.prop_sources, all_now);
    }

    /// The chips write into the persisted view, so a stored blob has to bring
    /// back exactly the ones that were switched off.
    #[test]
    fn the_spot_chips_survive_a_restart() {
        let mut v = ViewState { wide_waterfall: false, ..ViewState::default() };
        v.spot_kinds_shown[3] = false; // PSK off
        let back: ViewState = ron::from_str(&ron::to_string(&v).unwrap()).unwrap();
        assert_eq!(back.spot_kinds_shown, [true, true, true, false, true, true]);
        assert!(!back.wide_waterfall);
    }

    /// The SPEC popup's two chips are independent switches, so all four
    /// displays are reachable: spectrum, waterfall, both, and neither. Each one
    /// has to give the layout the height split it expects.
    #[test]
    fn the_layer_chips_reach_all_four_displays() {
        let mut v = ViewState::default();
        assert!(v.panadapter_visible(), "both layers by default");
        assert_eq!(v.effective_spectrum_fraction(), v.spectrum_fraction);

        // Spectrum off: the waterfall has the strip.
        v.set_spectrum_visible(false);
        assert!(v.panadapter_visible());
        assert_eq!(v.effective_spectrum_fraction(), 0.0);

        // Waterfall off instead: the spectrum has the strip.
        v.set_spectrum_visible(true);
        v.set_waterfall_visible(false);
        assert!(v.panadapter_visible());
        assert_eq!(v.effective_spectrum_fraction(), 1.0);

        // Neither: no panadapter at all — the layout skips the widget rather
        // than allocating it a strip.
        v.set_spectrum_visible(false);
        assert!(!v.panadapter_visible(), "both chips off still drew a panadapter");

        v.set_waterfall_visible(true);
        assert!(v.panadapter_visible());
    }

    #[test]
    fn the_view_roundtrips_through_storage() {
        let v = ViewState {
            view_lo_hz: 7_074_000.0,
            view_hi_hz: 7_077_000.0,
            pre_digi_view: Some((7_000_000.0, 7_200_000.0)),
            ..ViewState::default()
        };
        let back: ViewState = ron::from_str(&ron::to_string(&v).unwrap()).unwrap();
        assert_eq!(back, v);
    }
}

#[cfg(test)]
mod driver_tests {
    use super::ViewState;

    /// The 70× zoom nobody asked for: a KiwiSDR's 12 kHz window carried into a
    /// SpyServer session, where nothing would ever widen it again.
    #[test]
    fn a_window_from_another_front_end_is_dropped() {
        let mut v = ViewState { driver: "kiwisdr".into(), ..Default::default() };
        v.fit(14_324_375.0, 11_998.9);
        assert!(v.adopt_driver("spyserver"), "a new interface drops the window");
        assert!(v.is_unset(), "unset, so the panadapter fits it on the next draw");
        assert_eq!(v.driver, "spyserver");
    }

    /// The whole reason the window is persisted: coming back up on the same
    /// interface restores the operator's zoom.
    #[test]
    fn the_same_front_end_keeps_its_zoom() {
        let mut v = ViewState { driver: "rtlsdr".into(), ..Default::default() };
        v.fit(14_100_000.0, 48_000.0);
        let (lo, hi) = (v.view_lo_hz, v.view_hi_hz);
        assert!(!v.adopt_driver("rtlsdr"));
        assert_eq!((v.view_lo_hz, v.view_hi_hz), (lo, hi));
    }

    /// A view stored before this field existed, or a first run: adopt the
    /// driver without throwing away a window the operator may have left set.
    #[test]
    fn a_view_with_no_remembered_driver_is_left_alone() {
        let mut v = ViewState::default();
        v.fit(14_100_000.0, 48_000.0);
        let (lo, hi) = (v.view_lo_hz, v.view_hi_hz);
        assert!(!v.adopt_driver("rtlsdr"));
        assert_eq!((v.view_lo_hz, v.view_hi_hz), (lo, hi), "an upgrade keeps the zoom");
        assert_eq!(v.driver, "rtlsdr");
    }

    /// An already-unset window has nothing to drop, and must not be reported as
    /// a change the caller should log.
    #[test]
    fn an_unset_window_is_not_dropped_twice() {
        let mut v = ViewState { driver: "kiwisdr".into(), ..Default::default() };
        assert!(!v.adopt_driver("spyserver"));
        assert!(v.is_unset());
    }
}
