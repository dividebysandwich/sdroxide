//! UI / display preferences (persisted in `config.toml` under `[ui]`), plus the
//! coarse speed enum shared by the waterfall-scroll and spectrum-averaging
//! settings. Kept wasm-safe (no I/O) so the egui client can use it directly.

use serde::{Deserialize, Serialize};

use crate::SpotKind;

/// Coarse speed setting for the waterfall scroll and the spectrum line.
///
/// The last two are the waterfall's alone. They exist because the engine now
/// clocks waterfall rows itself rather than one per published frame, so a rate
/// past the screen's refresh is real time resolution instead of the same line
/// drawn twice — see [`crate::SpectrumConfig::rows_per_sec`]. The spectrum
/// *line* has nothing to gain from them (it is redrawn once a frame whatever
/// happens), so its combo offers [`Speed::ALL`] and the waterfall's offers
/// [`Speed::WATERFALL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speed {
    Slow,
    Medium,
    Fast,
    Faster,
    Fastest,
}

impl Speed {
    /// The three that mean something to the spectrum line.
    pub const ALL: [Speed; 3] = [Speed::Slow, Speed::Medium, Speed::Fast];

    /// Every scroll rate the waterfall offers.
    pub const WATERFALL: [Speed; 5] =
        [Speed::Slow, Speed::Medium, Speed::Fast, Speed::Faster, Speed::Fastest];

    /// The four the 3D spectrum's flow offers. No `Fastest`: the surface is
    /// advanced by the client from the frames it is sent, so its ceiling is the
    /// frame rate — 60 by default — and a fifth step above `Faster` would only
    /// repeat rows.
    pub const SURFACE: [Speed; 4] = [Speed::Slow, Speed::Medium, Speed::Fast, Speed::Faster];

    pub fn label(self) -> &'static str {
        match self {
            Speed::Slow => "Slow",
            Speed::Medium => "Medium",
            Speed::Fast => "Fast",
            Speed::Faster => "Faster",
            Speed::Fastest => "Fastest",
        }
    }
}

/// How much detail the panadapter is drawn with: how many columns its waterfall
/// history holds, and so how many bins the engine is asked to put in every
/// frame (see [`crate::SpectrumConfig::display_bins`]).
///
/// `Auto` is the default and is what nearly everyone should leave it on. It
/// reads the GPU's own texture limit, what the adapter calls itself, which
/// backend is in use, whether the engine is across a network, and how wide the
/// panadapter actually is in *pixels* — then picks the most that machine can
/// carry. The named steps are for overruling it in either direction: a remote
/// client on a link Auto is being cautious about, or a machine that would
/// rather have the frame rate than the columns.
///
/// Steps above what the renderer can hold are shown greyed rather than hidden,
/// so the ladder is visible even where it cannot be climbed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpectrumDetail {
    /// Pick from this machine and this screen, and follow them if they change.
    #[default]
    Auto,
    /// 2048 columns — what every sdroxide before this drew, and about what a
    /// 1080p panadapter can show.
    Standard,
    /// 4096 columns — one per pixel of a 4K panadapter.
    High,
    /// 8192 columns — two per pixel of a 4K panadapter, which is what keeps a
    /// carrier sharp while the view is panned off the pixel grid.
    Ultra,
}

impl SpectrumDetail {
    pub const ALL: [SpectrumDetail; 4] = [
        SpectrumDetail::Auto,
        SpectrumDetail::Standard,
        SpectrumDetail::High,
        SpectrumDetail::Ultra,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SpectrumDetail::Auto => "Auto",
            SpectrumDetail::Standard => "Standard (2048)",
            SpectrumDetail::High => "High (4096)",
            SpectrumDetail::Ultra => "Ultra (8192)",
        }
    }

    /// The width this asks for, or `None` for `Auto` — which only the client
    /// that knows its own renderer can answer
    /// (`sdroxide_ui::waterfall_gpu::auto_display_bins`).
    pub fn columns(self) -> Option<u32> {
        match self {
            SpectrumDetail::Auto => None,
            SpectrumDetail::Standard => Some(2048),
            SpectrumDetail::High => Some(4096),
            SpectrumDetail::Ultra => Some(8192),
        }
    }
}

/// A usage class on the band-plan strip painted along the bottom of the
/// waterfall — what an allocation is *for*, which is the only thing the strip
/// colours by.
///
/// Here rather than in the widget that draws it because the operator can
/// retint every class, and the picked colours are kept in
/// [`UiSettings::bandplan_colors`], indexed by [`BandplanKind::index`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BandplanKind {
    /// Amateur band as a whole, drawn when zoomed out past its sub-segments.
    Ham,
    /// CW sub-segment of an amateur band.
    Cw,
    /// Digital-mode sub-segment.
    Digi,
    /// Voice sub-segment.
    Phone,
    /// Beacon sub-segment.
    Beacon,
    /// Shortwave/tropical broadcasting allocation.
    Broadcast,
    /// Longwave and mediumwave AM broadcasting.
    Am,
    /// Citizens' band.
    Cb,
}

impl BandplanKind {
    /// Every class, in the order the settings colour pickers and
    /// [`BandplanKind::index`] use. Anything indexed by class —
    /// [`UiSettings::bandplan_colors`] — is this wide.
    pub const ALL: [BandplanKind; 8] = [
        BandplanKind::Ham,
        BandplanKind::Cw,
        BandplanKind::Digi,
        BandplanKind::Phone,
        BandplanKind::Beacon,
        BandplanKind::Broadcast,
        BandplanKind::Am,
        BandplanKind::Cb,
    ];

    /// How many classes there are, i.e. the width of every per-class array.
    pub const COUNT: usize = BandplanKind::ALL.len();

    /// This class's position in [`BandplanKind::ALL`].
    pub fn index(self) -> usize {
        match self {
            BandplanKind::Ham => 0,
            BandplanKind::Cw => 1,
            BandplanKind::Digi => 2,
            BandplanKind::Phone => 3,
            BandplanKind::Beacon => 4,
            BandplanKind::Broadcast => 5,
            BandplanKind::Am => 6,
            BandplanKind::Cb => 7,
        }
    }

    /// Short label for the settings picker beside the swatch.
    pub fn label(self) -> &'static str {
        match self {
            BandplanKind::Ham => "Ham",
            BandplanKind::Cw => "CW",
            BandplanKind::Digi => "Digital",
            BandplanKind::Phone => "Voice",
            BandplanKind::Beacon => "Beacon",
            BandplanKind::Broadcast => "Broadcast",
            BandplanKind::Am => "AM / LW / MW",
            BandplanKind::Cb => "CB",
        }
    }

    /// The stock RGB the strip shades this class with (r, g, b).
    ///
    /// Where a class starts, not where it has to stay: the operator can retint
    /// any of them from the UI settings tab, and what they chose is kept in
    /// [`UiSettings::bandplan_colors`]. Clients read that, not this.
    ///
    /// The blocks are painted at about 60% opacity over a near-black
    /// waterfall, so a saturated hue lands a good deal darker than it looks
    /// here — which is why the two broadcast classes read as brown and why
    /// they are worth being able to change (issue #145).
    pub fn default_color(self) -> (u8, u8, u8) {
        match self {
            BandplanKind::Ham => (0x2C, 0x9E, 0x8C),
            BandplanKind::Cw => (0xE6, 0xB0, 0x3C),
            BandplanKind::Digi => (0x2E, 0xC4, 0xE6),
            BandplanKind::Phone => (0x4C, 0xC9, 0x6A),
            BandplanKind::Beacon => (0xE0, 0x5A, 0xA0),
            BandplanKind::Broadcast => (0xE8, 0x82, 0x2E),
            BandplanKind::Am => (0xC9, 0x6A, 0x3C),
            BandplanKind::Cb => (0x9A, 0x6C, 0xE0),
        }
    }
}

/// Coarse font-size step for one family of hand-painted labels — the skimmer
/// boxes, the panadapter's own labels, the popup menus. Each family maps the
/// three steps onto its own point sizes, so `Small` here is "the small end of
/// that family's range", not one absolute size.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontSize {
    Small,
    Large,
    /// Declared last for the same reason as [`UiTheme::Default`]: serde
    /// demands the catch-all be the final variant, so a typo in a hand-edited
    /// config degrades to the middle size instead of throwing the whole
    /// config away.
    #[default]
    #[serde(other)]
    Medium,
}

impl FontSize {
    pub const ALL: [FontSize; 3] = [FontSize::Small, FontSize::Medium, FontSize::Large];

    pub fn label(self) -> &'static str {
        match self {
            FontSize::Small => "Small",
            FontSize::Medium => "Medium",
            FontSize::Large => "Large",
        }
    }
}

/// Which layout the window wears. `Auto` picks one from the viewport size; the
/// rest force it — for testing the compact strips without a phone to hand, and
/// for anyone who would rather have the menus in a small desktop window than a
/// control strip wrapped over three rows.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutMode {
    #[default]
    Auto,
    Desktop,
    Tablet,
    Phone,
    /// A small desktop screen: the tablet tier's menus, always on the
    /// single-row strip, with the operating panels' chips and spacing pulled
    /// in. For 1366×768 and the like, where the tablet layout fits but spends
    /// more of a short screen on chrome than it has to spare (issue #211).
    ///
    /// Appended rather than slotted in beside `Tablet`, because this is
    /// serialised into `config.toml` and inserting a variant would rename
    /// everyone else's.
    Small,
}

impl LayoutMode {
    pub const ALL: [LayoutMode; 5] = [
        LayoutMode::Auto,
        LayoutMode::Desktop,
        LayoutMode::Tablet,
        LayoutMode::Small,
        LayoutMode::Phone,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LayoutMode::Auto => "Auto",
            LayoutMode::Desktop => "Desktop",
            LayoutMode::Tablet => "Tablet",
            LayoutMode::Small => "Small screen",
            LayoutMode::Phone => "Phone",
        }
    }
}

/// Colour theme for the UI chrome. Every theme recolours the same set of
/// roles (backgrounds, borders, accents, text); content colours — waterfall
/// palettes, band plan, map — are untouched. The phosphor themes are
/// monochrome on purpose, except that transmit/SWR/error indications stay red
/// so an operator never has to wonder whether RF is leaving the antenna.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiTheme {
    GreenPhosphor,
    AmberPhosphor,
    TealOrange,
    Rainbow,
    /// White panels, near-black ink, dark saturated accents — the one theme
    /// that inverts the ground. The instruments (panadapter, S-meter, map,
    /// solar globe) keep their dark glass: a waterfall has no bright-ground
    /// form, and a signal display is read the same way in every theme.
    Light,
    /// White on black at the highest contrast the screen can give, with every
    /// dim shade in the UI pulled up to meet it — nothing is decoratively
    /// faint. For low vision, for glare, and for a display that has lost its
    /// contrast.
    HighContrast,
    /// The classic navy/cyan/pink look. Declared last because serde demands
    /// the catch-all be the final variant: it also swallows an unrecognised
    /// value in a hand-edited config, so a typo degrades to the default theme
    /// instead of throwing the whole config away.
    #[default]
    #[serde(other)]
    Default,
}

impl UiTheme {
    pub const ALL: [UiTheme; 7] = [
        UiTheme::Default,
        UiTheme::Light,
        UiTheme::HighContrast,
        UiTheme::GreenPhosphor,
        UiTheme::AmberPhosphor,
        UiTheme::TealOrange,
        UiTheme::Rainbow,
    ];

    pub fn label(self) -> &'static str {
        match self {
            UiTheme::Default => "Default",
            UiTheme::Light => "Light",
            UiTheme::HighContrast => "High contrast",
            UiTheme::GreenPhosphor => "Green phosphor",
            UiTheme::AmberPhosphor => "Amber phosphor",
            UiTheme::TealOrange => "Teal / orange",
            UiTheme::Rainbow => "Rainbow",
        }
    }

    /// True where the chrome sits on a bright ground, so anything that has to
    /// pick an ink or a shade by hand knows which way round the world is.
    pub fn is_light(self) -> bool {
        matches!(self, UiTheme::Light)
    }
}

/// The shape a piece of chrome wears — one list serves both the buttons and
/// the windows, each chosen separately.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromeStyle {
    Rectangular,
    Rounded,
    Gradient,
    Bevel,
    /// Drawn as if the screen were a character display: frames built out of
    /// `+`, `-` and `|`, buttons wearing `[` and `]`, tick boxes reading
    /// `[X]`, and everything set in the monospace face.
    Terminal,
    /// The classic cut-corner look. Last for the same reason as
    /// [`UiTheme::Default`]: the serde catch-all must be the final variant.
    #[default]
    #[serde(other)]
    Angled,
}

impl ChromeStyle {
    pub const ALL: [ChromeStyle; 6] = [
        ChromeStyle::Angled,
        ChromeStyle::Rectangular,
        ChromeStyle::Rounded,
        ChromeStyle::Gradient,
        ChromeStyle::Bevel,
        ChromeStyle::Terminal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ChromeStyle::Angled => "Angled",
            ChromeStyle::Rectangular => "Rectangular",
            ChromeStyle::Rounded => "Rounded",
            ChromeStyle::Gradient => "Gradient",
            ChromeStyle::Bevel => "3D bevel",
            ChromeStyle::Terminal => "Terminal",
        }
    }
}

/// Which face the S-meter wears. Cycled by clicking the meter itself.
///
/// A preference of the operator rather than of the radio — which instrument
/// somebody reads a signal on has nothing to do with what is being received —
/// so it sits in `[ui]` beside the theme and the fonts, is written the moment
/// it is clicked, and every radio tab comes up wearing it (issue #185). It
/// used to ride in the client's per-radio panadapter view, where a second
/// radio came up on the stock face and a session that ended without a clean
/// quit lost the choice with the rest of eframe's not-yet-autosaved blob.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmeterStyle {
    /// Horizontal gradient bar with a graduated scale beneath it.
    Bar,
    /// Scrolling trace of the last quarter-minute — reads fading and QSB (and,
    /// on transmit, how SWR behaved across the over) the way neither of the
    /// instantaneous faces can.
    Trace,
    /// Analog moving-coil instrument with a swinging needle.
    ///
    /// Declared last because serde demands the catch-all be the final variant:
    /// it also swallows an unrecognised value in a hand-edited config, so a
    /// typo degrades to the stock face instead of throwing the whole `[ui]`
    /// table away.
    #[default]
    #[serde(other)]
    Needle,
}

impl SmeterStyle {
    /// The next face in the click cycle.
    pub fn next(self) -> Self {
        match self {
            SmeterStyle::Needle => SmeterStyle::Bar,
            SmeterStyle::Bar => SmeterStyle::Trace,
            SmeterStyle::Trace => SmeterStyle::Needle,
        }
    }

    /// The face for a box wider than it is tall — the shape the compact strip
    /// hands the meter on a phone.
    ///
    /// The needle drops out there. Its arc is a chord across the box, so its
    /// radius follows the *width*, and the headline chip ends up covering the
    /// half of the scale the arc has not yet descended past — the reading and
    /// the instrument printed over each other. The bar says the same thing in
    /// a strip, which is exactly the shape available.
    pub fn compact(self) -> Self {
        match self {
            SmeterStyle::Needle => SmeterStyle::Bar,
            other => other,
        }
    }

    /// The next face in the click cycle, skipping any this box cannot show.
    pub fn next_compact(self) -> Self {
        let next = self.next();
        if next.compact() != next { next.next() } else { next }
    }
}

/// User display preferences. All have defaults so a missing `[ui]` table loads.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// GUI repaint + spectrum frame rate, in frames per second.
    pub frame_rate_fps: u32,
    /// How fast the waterfall scrolls.
    pub waterfall_speed: Speed,
    /// How fast the spectrum line reacts (averaging; slower = smoother).
    pub spectrum_speed: Speed,
    /// How fast the 3D spectrum flows away from the viewer.
    ///
    /// Its own setting rather than the waterfall's, even though both are time
    /// axes, because the two are read at different lengths: the waterfall is
    /// minutes of band and this is the last few seconds of it, and the rate
    /// that makes a CW trace legible on one buries the other's whole depth in a
    /// fifth of a second. See [`UiSettings::spectrum_3d_rows_per_sec`].
    pub spectrum_3d_speed: Speed,
    /// Waterfall colour palette, as an index into the client's palette list.
    pub waterfall_palette: usize,
    /// How many columns the panadapter and its waterfall are drawn with.
    ///
    /// This screen's preference, like the frame rate above it and for the same
    /// reason: what a machine can carry is a fact about the machine looking,
    /// not about the radio being looked at. A remote client picks its own.
    pub spectrum_detail: SpectrumDetail,
    /// Fill the spectrum area with a vertical top→bottom colour gradient.
    pub spectrum_gradient: bool,
    /// Gradient colour at the top of the spectrum area (sRGB, 0–255).
    pub gradient_top: [u8; 3],
    /// Gradient colour at the bottom of the spectrum area (sRGB, 0–255).
    pub gradient_bottom: [u8; 3],
    /// The tint each spot kind wears — on the panadapter labels, in the SPOTS
    /// list and on the world map — indexed by [`SpotKind::index`]. Starts at
    /// [`SpotKind::default_color`]; the UI tab retints any of them.
    ///
    /// This screen's preference, like the theme: the spots themselves are the
    /// station's, but what colour they are painted is the operator's, and a
    /// remote client picks its own.
    #[serde(default = "default_spot_colors", deserialize_with = "spot_colors")]
    pub spot_colors: [[u8; 3]; SpotKind::COUNT],
    /// The shade each band-plan class wears on the strip along the bottom of
    /// the waterfall, indexed by [`BandplanKind::index`]. Starts at
    /// [`BandplanKind::default_color`]; the UI tab retints any of them.
    ///
    /// This screen's preference, like the spot tints above: the plan itself is
    /// the station's, but what colour an allocation is shaded is whatever the
    /// operator looking at it can read (issue #145).
    #[serde(default = "default_bandplan_colors", deserialize_with = "bandplan_colors")]
    pub bandplan_colors: [[u8; 3]; BandplanKind::COUNT],
    /// Which layout the window wears, or `Auto` to pick from the viewport.
    pub layout: LayoutMode,
    /// Colour theme for the UI chrome.
    pub theme: UiTheme,
    /// The shape buttons wear.
    pub button_style: ChromeStyle,
    /// The shape floating windows and popups wear.
    pub window_style: ChromeStyle,
    /// Font size for the skimmer / spot boxes overlaid on the waterfall.
    /// `Medium` is the historic size.
    pub skimmer_font_size: FontSize,
    /// Font size for the labels painted onto the spectrum and waterfall —
    /// the frequency scale, the band plan, the measurement and marker
    /// labels. `Small` is the historic size.
    pub waterfall_font_size: FontSize,
    /// Font size for the interface itself — menus, dialogs, windows, the tab
    /// strip, the top bar and every button on it. Applied as the client's zoom
    /// factor, so the spacing around the text follows it and the waterfall and
    /// skimmer sizes below are relative to it. `Medium` is the historic size.
    pub menu_font_size: FontSize,
    /// Which face the S-meter wears — needle (the stock one), bar or trace.
    /// Cycled by clicking the meter; see [`SmeterStyle`] for why it is a
    /// screen preference rather than part of a radio's view.
    pub smeter_style: SmeterStyle,
    /// Ask sdroxide.com once per start whether a newer release has been
    /// published, and say so in the notice banner above the panadapter. In
    /// `[ui]` because it is this screen's preference, like the theme — the
    /// native client checks for its own build, wherever its radio is.
    pub update_check: bool,
    /// How the memory channel window orders its list. This screen's
    /// preference, not the station's: the store keeps its own order and every
    /// client reads it whichever way its operator asked for.
    pub memory_sort: crate::MemorySort,
    /// Read that order backwards — Z to A, highest frequency first, and
    /// newest-stored first for [`crate::MemorySort::Stored`].
    pub memory_sort_desc: bool,
    /// How the FT8/FT4 decode list orders the stations. This screen's
    /// preference like [`UiSettings::memory_sort`] above, and for the same
    /// reason: the decodes are the station's, the order they are read in is
    /// the operator's.
    pub decode_sort: crate::DecodeSort,
    /// Read that order backwards — weakest and nearest first, countries Z to A.
    pub decode_sort_desc: bool,
    /// Show every decode in one list, newest turn first, instead of grouping
    /// them into odd/even turn blocks.
    pub decode_single_list: bool,
    /// Decode-list filter: only stations calling a CQ we may answer.
    pub decode_cq_only: bool,
    /// Decode-list filter: only stations that would put something new in the
    /// log (new entity, new band-slot, new grid, or a callsign never worked).
    pub decode_new_only: bool,
    /// In CW, read the frequency of the *signal* rather than of the dial —
    /// the QRG, in the Q-code an operator would use to ask for it.
    ///
    /// A CW dial sits a sidetone pitch below what is being copied, so the
    /// number in the readout is not the number either operator would quote —
    /// it is that number minus the pitch, and the arithmetic is left to the
    /// person. With this on, the main readout shows what
    /// [`crate::Mode::on_air_hz`] answers, and the tuning line on the
    /// panadapter moves to sit on the signal, where the passband is already
    /// centred (the engine keeps the filter on the pitch).
    ///
    /// A display preference, not a change of tuning: the dial is still the
    /// dial, and everything that tunes, stores or checks a band edge goes on
    /// using it. Off by default, which is what every other radio does.
    ///
    /// CW only for now. RTTY and WEFAX sit off their dials too
    /// ([`crate::Mode::tunes_off_dial`]) and could join it, but each wants
    /// checking against real signals first.
    pub cw_qrg: bool,
    /// Draw the world's cities on the flat maps — the dot per place and the
    /// name beside it.
    ///
    /// On by default: on a map of the whole world the cities are most of what
    /// says *where* a dot is, and a coastline alone is a poor substitute. But
    /// they are also the busiest thing on that map, and the panel maps are
    /// small — an operator watching where their FT8 contacts are coming from is
    /// reading a handful of coloured dots against a field of grey ones with
    /// names attached to them, which is the complaint this answers (issue
    /// #312).
    ///
    /// This screen's preference, like the theme and the spot tints: the
    /// decodes are the station's, what the map they are drawn on carries is the
    /// operator's. The 3D globe is untouched — its cities are night-side lights
    /// rather than markers, and nothing is written across a contact there.
    pub map_cities: bool,
}

/// Default for [`UiSettings::spot_colors`] — every kind on its stock tint.
fn default_spot_colors() -> [[u8; 3]; SpotKind::COUNT] {
    let mut out = [[0u8; 3]; SpotKind::COUNT];
    for kind in SpotKind::ALL {
        let (r, g, b) = kind.default_color();
        out[kind.index()] = [r, g, b];
    }
    out
}

/// Read [`UiSettings::spot_colors`] as a list of any length, so a config
/// written before a spot kind was added — or by a newer build that has one
/// more — still loads. A short list leaves the kinds it doesn't reach on their
/// stock tint; a long one has its tail ignored. Without this the whole `[ui]`
/// table would fail to parse over one extra entry, and the operator would lose
/// their theme, their fonts and their layout along with the colours.
fn spot_colors<'de, D>(d: D) -> Result<[[u8; 3]; SpotKind::COUNT], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let list = Vec::<[u8; 3]>::deserialize(d)?;
    let mut out = default_spot_colors();
    for (slot, c) in out.iter_mut().zip(list) {
        *slot = c;
    }
    Ok(out)
}

/// Default for [`UiSettings::bandplan_colors`] — every class on its stock shade.
fn default_bandplan_colors() -> [[u8; 3]; BandplanKind::COUNT] {
    let mut out = [[0u8; 3]; BandplanKind::COUNT];
    for kind in BandplanKind::ALL {
        let (r, g, b) = kind.default_color();
        out[kind.index()] = [r, g, b];
    }
    out
}

/// Read [`UiSettings::bandplan_colors`] as a list of any length, for the same
/// reason [`spot_colors`] does: one extra or one missing entry must cost the
/// operator that entry, not the whole `[ui]` table.
fn bandplan_colors<'de, D>(d: D) -> Result<[[u8; 3]; BandplanKind::COUNT], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let list = Vec::<[u8; 3]>::deserialize(d)?;
    let mut out = default_bandplan_colors();
    for (slot, c) in out.iter_mut().zip(list) {
        *slot = c;
    }
    Ok(out)
}

impl Default for UiSettings {
    fn default() -> Self {
        UiSettings {
            // Off: the dial is what every other radio shows, and an operator
            // who has not asked for the change should not find their readout
            // reading differently from the rig beside it.
            cw_qrg: false,
            frame_rate_fps: 60,
            waterfall_speed: Speed::Medium,
            spectrum_speed: Speed::Medium,
            spectrum_3d_speed: Speed::Medium,
            waterfall_palette: 0,
            spectrum_detail: SpectrumDetail::Auto,
            spectrum_gradient: true,
            gradient_top: [64, 0, 0],   // dark red
            gradient_bottom: [0, 0, 0], // black
            spot_colors: default_spot_colors(),
            bandplan_colors: default_bandplan_colors(),
            layout: LayoutMode::Auto,
            theme: UiTheme::Default,
            button_style: ChromeStyle::Angled,
            window_style: ChromeStyle::Angled,
            skimmer_font_size: FontSize::Medium,
            waterfall_font_size: FontSize::Small,
            menu_font_size: FontSize::Medium,
            smeter_style: SmeterStyle::Needle,
            update_check: true,
            memory_sort: crate::MemorySort::Stored,
            memory_sort_desc: false,
            decode_sort: crate::DecodeSort::None,
            // Strongest and farthest first, which is the useful end of both
            // numbers; the Country order flips this when it is picked.
            decode_sort_desc: true,
            decode_single_list: false,
            decode_cq_only: false,
            decode_new_only: false,
            map_cities: true,
        }
    }
}

impl UiSettings {
    /// Selectable frame rates for the UI combo.
    ///
    /// The rates below 30 are for machines that cannot keep up — a Raspberry Pi
    /// driving a 4K panel, a remote client on a thin laptop. They cost detail in
    /// the waterfall (fewer distinct rows; the scroll speed is absolute, so a
    /// row is simply repeated) and nothing else: the engine still processes
    /// every sample, and only the spectrum frame it publishes slows down.
    pub const FPS_OPTIONS: [u32; 6] = [5, 10, 15, 30, 60, 90];

    /// Frame rate clamped to a sane range (guards a hand-edited config).
    pub fn fps(self) -> u32 {
        self.frame_rate_fps.clamp(5, 240)
    }

    /// Waterfall scroll rate in rows per second. Absolute (independent of the
    /// frame rate) so the time axis — and the 60-second gridlines — stay stable
    /// when the frame rate changes.
    ///
    /// `Fast` is twice the old fast rate, which now sits on `Medium`: at 28
    /// rows/s a CW or FT8 trace still smears vertically, and chasing a fading
    /// signal wants the extra time resolution.
    ///
    /// `Faster` and `Fastest` are past what a screen redraws at, which is the
    /// point: the engine clocks rows on its own clock now, so 224 a second is
    /// 224 *different* lines rather than 56 of them drawn four times. What they
    /// cost is history — the client's ring is a fixed number of rows, so
    /// `Fastest` holds nine seconds of it where `Medium` holds seventy-three —
    /// and, to a remote client, bytes: a row is one per column.
    ///
    /// Nothing is gained past the rate the analyser produces transforms at
    /// (`rate / (fft_size / 2)`), and rows simply repeat above it. That is a
    /// property of the front end and the FFT size, not something to clamp here:
    /// an RX-888 at 8 Msps through a 32768-point window makes 494 a second and
    /// can feed any of these; a 48 kHz audio lane makes 23 and cannot feed even
    /// `Medium`.
    pub fn waterfall_rows_per_sec(self) -> f32 {
        match self.waterfall_speed {
            Speed::Slow => 5.0,
            Speed::Medium => 28.0,
            Speed::Fast => 56.0,
            Speed::Faster => 112.0,
            Speed::Fastest => 224.0,
        }
    }

    /// Rows a second the 3D spectrum flows away from the viewer.
    ///
    /// The surface remembers a fixed number of spectra, so this is also how
    /// much time its depth covers — at 48 rows deep, eight seconds at `Slow`
    /// down to one at `Faster`. Slower is a longer memory and a surface that
    /// crawls; faster is a shorter one that moves, which is what makes a signal
    /// that is only there for a moment show up as a shape rather than a blip.
    ///
    /// Nothing above the frame rate buys anything: the client advances the
    /// surface from the spectra it is sent, so a rate past them repeats rows —
    /// which is why the chips stop at `Faster` and why `Fastest` is answered
    /// here anyway, for a hand-edited config.
    pub fn spectrum_3d_rows_per_sec(self) -> f32 {
        match self.spectrum_3d_speed {
            Speed::Slow => 6.0,
            Speed::Medium => 12.0,
            Speed::Fast => 24.0,
            Speed::Faster => 48.0,
            Speed::Fastest => 96.0,
        }
    }

    /// Exponential averaging time constant (seconds) for the spectrum line.
    /// Fast disables averaging (snappy); slower values smooth it out.
    pub fn spectrum_avg_tc(self) -> f32 {
        match self.spectrum_speed {
            // The waterfall-only rates mean the same thing here as `Fast`:
            // no averaging. Reachable only from a hand-edited config.
            Speed::Fast | Speed::Faster | Speed::Fastest => 0.0,
            Speed::Medium => 0.1,
            Speed::Slow => 0.2,
        }
    }
}
