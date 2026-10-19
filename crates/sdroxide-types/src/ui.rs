//! UI / display preferences (persisted in `config.toml` under `[ui]`), plus the
//! coarse speed enum shared by the waterfall-scroll and spectrum-averaging
//! settings. Kept wasm-safe (no I/O) so the egui client can use it directly.

use core::sync::atomic::{AtomicBool, Ordering};

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

/// How the colour under the spectrum line is shaded, measured on the height of
/// the spectrum area rather than on each column — so the noise floor's ragged
/// edge does not turn into stripes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpectrumFillStyle {
    /// One colour, the same strength everywhere under the line.
    Fixed,
    /// Full at the bottom of the spectrum area, fading out towards the top.
    #[default]
    Gradient,
    /// Full in the lower half, fading out across the upper half.
    Mixed,
}

impl SpectrumFillStyle {
    pub const ALL: [SpectrumFillStyle; 3] =
        [SpectrumFillStyle::Fixed, SpectrumFillStyle::Gradient, SpectrumFillStyle::Mixed];

    pub fn label(self) -> &'static str {
        match self {
            SpectrumFillStyle::Fixed => "Fixed",
            SpectrumFillStyle::Gradient => "Gradient",
            SpectrumFillStyle::Mixed => "Mixed",
        }
    }
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
    /// Picks one from the viewport size — the default, and where serde sends a
    /// value this build has never heard of.
    ///
    /// Declared last for the same reason as [`FontSize::Medium`]: the catch-all
    /// must be final. Without `#[serde(other)]` a `config.toml` written by a
    /// build with a later layout — `Small` was the last one added — fails the
    /// **whole** file, and `Settings::load` quarantines it: the radio, audio,
    /// speech and alerts settings go with the one unknown value. Degrading to
    /// `Auto` costs a layout, not a configuration.
    #[default]
    #[serde(other)]
    Auto,
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
    /// The Nordic palette: polar-night navy grounds, snow-storm text, frost
    /// cyan accents and a purple chrome. Already a dark theme — NordDark is
    /// its near-black echo.
    Nord,
    /// The same accents as [`UiTheme::Nord`] on near-black grounds, so the
    /// panels sit almost in the page behind them.
    NordDark,
    /// The warm Gruvbox palette: parchment text on near-black browns, blue and
    /// aqua accents, and the iconic bright-orange chrome.
    Gruvbox,
    /// The Everforest palette: green-tinged deep blues, soft cream text, and
    /// mossy accent hues.
    Everforest,
    /// Ethan Schoonover's Solarized dark: teal, blue and magenta accents on
    /// the deep blue-grey base03/base02 grounds.
    SolarizedDark,
    /// Solarized on paper: the same accents carried down until they read as
    /// text on the pale base3 ground.
    Solarized,
    /// The Dracula palette: graphite mantles, mint-bright cyan, soft magenta
    /// chrome.
    Dracula,
    /// Catppuccin Mocha, the dark flavour: a deep indigo base, sky-blue and
    /// mauve accents.
    CatppuccinMocha,
    /// Catppuccin Latte, the light flavour: a cream base with teal and blue
    /// accents taken down to where they read as text on it.
    CatppuccinLatte,
    /// A near-monochrome light theme: white panels, graphite lines and a
    /// single restrained blue accent — the modern development-tool look.
    ModernMinimalist,
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
    /// The SDRuno look: deep blue grounds, near-white ink, a single calm blue
    /// accent. Named for how it reads rather than for the program it echoes.
    ///
    /// Declared before [`UiTheme::Default`] because the catch-all must stay
    /// last, which means **`Default`'s discriminant moves** when a theme is
    /// added — a `UiTheme` rides [`ClientScreen`], so this is the reason the
    /// theme arrived with a `PROTO_VERSION` bump rather than a free change.
    Calm,
    /// Summit Radio's palette — seven colours and nothing else.
    ///
    /// Taken from summitradiodesigns.com, whose whole stylesheet is one `:root`
    /// block: a near-black navy ground, one panel, one border, two text inks
    /// and exactly two accents. That restraint is the point of it — the site is
    /// liked for being usable by somebody who has never seen an antenna
    /// model — so this theme adds no register the palette does not have.
    ///
    /// Declared before the catch-all for the same reason as [`UiTheme::Calm`],
    /// which means **`Default`'s discriminant moves** and this rides
    /// `ClientScreen`.
    Calmer,
    /// Compline, from joshuablais/compline (GPL-3.0): a cool grey-blue ground
    /// with every hue drawn from one narrow chroma, so nothing in it shouts.
    ///
    /// Its partner [`UiTheme::Lauds`] is the same palette at dawn.
    Compline,
    /// Lauds, the light half of the Compline palette: warm cream grounds and a
    /// cool near-black for ink.
    Lauds,
    /// The classic navy/cyan/pink look. Declared last because serde demands
    /// the catch-all be the final variant: it also swallows an unrecognised
    /// value in a hand-edited config, so a typo degrades to the default theme
    /// instead of throwing the whole config away.
    #[default]
    #[serde(other)]
    Default,
}

impl UiTheme {
    pub const ALL: [UiTheme; 21] = [
        UiTheme::Default,
        UiTheme::Light,
        UiTheme::HighContrast,
        UiTheme::GreenPhosphor,
        UiTheme::AmberPhosphor,
        UiTheme::TealOrange,
        UiTheme::Rainbow,
        UiTheme::Nord,
        UiTheme::NordDark,
        UiTheme::Gruvbox,
        UiTheme::Everforest,
        UiTheme::SolarizedDark,
        UiTheme::Solarized,
        UiTheme::Dracula,
        UiTheme::CatppuccinMocha,
        UiTheme::CatppuccinLatte,
        UiTheme::ModernMinimalist,
        UiTheme::Calm,
        UiTheme::Calmer,
        UiTheme::Compline,
        UiTheme::Lauds,
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
            UiTheme::Nord => "Nord",
            UiTheme::NordDark => "Nord dark",
            UiTheme::Gruvbox => "Gruvbox",
            UiTheme::Everforest => "Everforest",
            UiTheme::SolarizedDark => "Solarized dark",
            UiTheme::Solarized => "Solarized",
            UiTheme::Dracula => "Dracula",
            UiTheme::CatppuccinMocha => "Catppuccin mocha",
            UiTheme::CatppuccinLatte => "Catppuccin latte",
            UiTheme::ModernMinimalist => "Modern minimalist",
            UiTheme::Calm => "Calm",
            UiTheme::Calmer => "Calmer",
            UiTheme::Compline => "Compline",
            UiTheme::Lauds => "Lauds",
        }
    }

    /// True where the chrome sits on a bright ground, so anything that has to
    /// pick an ink or a shade by hand knows which way round the world is.
    pub fn is_light(self) -> bool {
        matches!(
            self,
            UiTheme::Light
                | UiTheme::Solarized
                | UiTheme::CatppuccinLatte
                | UiTheme::ModernMinimalist
        )
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

/// Where the solar-system 3D window was: its inner size and, where the platform
/// reports one, its outer position — both in egui points.
///
/// Kept so opening it — the first time after a restart, or again after it was
/// closed — puts it back where the operator left it rather than at the built-in
/// size. `pos` is `None` on Wayland, which gives a client no absolute window
/// position; the size still comes back and the compositor places the window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Solar3dWindow {
    /// Inner (drawing-area) size in points.
    pub size: [f32; 2],
    /// Outer (top-left, including decorations) position in points.
    #[serde(default)]
    pub pos: Option<[f32; 2]>,
}

/// A module pulled out into its **own OS window**: where it last was, so
/// reopening it — or the next start — puts it back rather than at a built-in
/// size. The first user is the detached panadapter
/// ([`UiSettings::panadapter_window`](crate::ui::UiSettings::panadapter_window));
/// the type is kept general so the next detachable module reuses it.
///
/// The same shape and the same Wayland caveat as [`Solar3dWindow`]: a client is
/// given no absolute window position, so `pos` is `None` there and the
/// compositor places the window. The **app-id** the window carries
/// (`sdroxide-panadapter`) is what a compositor rule matches on to float it and
/// pin it to a monitor — see `UNDOCKED-HANDOVER.md`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DetachedWindow {
    /// Inner (drawing-area) size in points.
    pub size: [f32; 2],
    /// Outer (top-left) position in points; `None` on Wayland.
    #[serde(default)]
    pub pos: Option<[f32; 2]>,
}

/// A module that can be pulled out of the main window into its own OS window —
/// **undocked**. The panadapter and the mode's operating panel are the first
/// two; the end goal is a user-arrangeable workspace in the SDRuno mould, so
/// this is an enum keyed in [`UiSettings::detached`] rather than a field pair
/// per module: a new detachable module is a variant here plus its UI spec,
/// nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DetachableModule {
    /// The spectrum and waterfall.
    Panadapter,
    /// The mode's operating panel — the decoder list, the transcript and the
    /// controls under the panadapter.
    Panel,
    /// The top control strip — the frequency readout, the S-meter and the
    /// receiver and transmitter controls (SDRuno's "RX control").
    Controls,
    /// A **second** spectrum and waterfall — SDRuno's AUX SP, the window that
    /// lets the operator watch a second span of the band without giving up the
    /// first one.
    ///
    /// Appended, and drawn from the same spectrum the panadapter is, so this is
    /// a **second view of the same receiver**, not a second receiver: both
    /// windows show one station's waterfall. Pointing it at another radio is a
    /// later thing and would need the shell to own one window per (module,
    /// radio) rather than per module.
    AuxPanadapter,
    /// The **band/mode selector** in a window of its own — SDRuno's RX-control
    /// band panel.
    ///
    /// It exists because the docked column cannot be wide: the operating panels
    /// below it are built for ~680 pt, so a docked selector that showed its
    /// modes in five columns would squeeze them and overrun. In a window of its
    /// own it can be as wide as the operator likes, with the operating panel
    /// keeping its full width. Appended last.
    BandMenu,
    /// The **antenna rotator** — a compass the operator points by hand, with the
    /// hardware's own reported position and a country lookup for DX bearings.
    ///
    /// A window because it is a tool the operator opens to swing a beam and
    /// then leaves, not a panel that belongs under the waterfall: the satellite
    /// lock already steers the antenna from the SAT window, and this is the
    /// manual half — point at a callsign, a map, or a country. Appended last.
    Rotor,
}

impl DetachableModule {
    /// How many detachable modules there are — the length of
    /// [`UiSettings::detached`], which is an array so `UiSettings` stays `Copy`.
    pub const COUNT: usize = 6;

    /// Every module, so a settings list or a test can walk them all.
    pub const ALL: [DetachableModule; DetachableModule::COUNT] = [
        DetachableModule::Panadapter,
        DetachableModule::Panel,
        DetachableModule::Controls,
        DetachableModule::AuxPanadapter,
        DetachableModule::BandMenu,
        DetachableModule::Rotor,
    ];

    /// This module's slot in [`UiSettings::detached`]. A plain array rather than
    /// a map keeps `UiSettings` `Copy`, which the whole UI leans on.
    pub fn index(self) -> usize {
        match self {
            DetachableModule::Panadapter => 0,
            DetachableModule::Panel => 1,
            DetachableModule::Controls => 2,
            DetachableModule::AuxPanadapter => 3,
            DetachableModule::BandMenu => 4,
            DetachableModule::Rotor => 5,
        }
    }

    /// A short, stable label for a settings row or a window title.
    pub fn label(self) -> &'static str {
        match self {
            DetachableModule::Panadapter => "Panadapter",
            DetachableModule::Panel => "Operating panel",
            DetachableModule::Controls => "Controls",
            DetachableModule::AuxPanadapter => "AUX SP (second spectrum)",
            DetachableModule::BandMenu => "Band & mode selector",
            DetachableModule::Rotor => "Antenna rotator",
        }
    }

    /// The window's **application id** — the handle a Wayland compositor rule
    /// matches on to float this window and pin it to a monitor. A public
    /// contract with the operator: change it and their rules stop matching.
    pub fn app_id(self) -> &'static str {
        match self {
            DetachableModule::Panadapter => "sdroxide-panadapter",
            DetachableModule::Panel => "sdroxide-panel",
            DetachableModule::Controls => "sdroxide-controls",
            DetachableModule::AuxPanadapter => "sdroxide-panadapter-aux",
            DetachableModule::BandMenu => "sdroxide-bandmenu",
            DetachableModule::Rotor => "sdroxide-rotor",
        }
    }
}

/// A named window arrangement — SDRuno's **workspace**. SDRuno keeps up to ten
/// and recalls them by name; this is the same idea for our module windows: which
/// of them are in their own OS windows, and where those windows are.
///
/// A **list of its own**, not a field in [`UiSettings`], and the reason is that
/// `UiSettings` is `Copy`: the whole UI passes it around by value, and a `Vec`
/// would take that away from every one of those sites to add one screen's worth
/// of arrangements. The list lives in `workspaces.json`, reached through the
/// same config store as the memories and the logbook.
///
/// A workspace is the arrangement only — the modules and their windows. What is
/// *on* is what a station profile or a radio carries; a workspace is where the
/// furniture is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Workspace {
    /// What the operator called it. Not an index: SDRuno recalls by name, and a
    /// name survives a reorder.
    pub name: String,
    /// One slot per [`DetachableModule`] in `index()` order **at save time**. A
    /// `Vec` rather than the fixed array because a workspace written by a build
    /// with a module this one has not heard of must still load and apply the
    /// modules it does know — the same tolerance [`UiSettings::detached`]'s
    /// reader has, for the same reason.
    pub slots: Vec<DetachedState>,
}

/// How many workspaces may be saved — SDRuno's ten. A cap rather than a
/// refusal-at-save would be a memory growing without bound; ten named
/// arrangements is already more than a station uses.
pub const WORKSPACE_MAX: usize = 10;

impl Workspace {
    /// Snapshot the arrangement in `settings` under `name`.
    pub fn capture(name: impl Into<String>, settings: &UiSettings) -> Self {
        Self { name: name.into(), slots: settings.detached.to_vec() }
    }

    /// Put this arrangement back into `settings`, leaving any module the
    /// workspace does not mention — a slot a newer or older build wrote —
    /// docked.
    pub fn apply(&self, settings: &mut UiSettings) {
        let mut out = [DetachedState::default(); DetachableModule::COUNT];
        for (slot, saved) in out.iter_mut().zip(self.slots.iter()) {
            *slot = *saved;
        }
        settings.detached = out;
    }
}

/// Whether one [`DetachableModule`] is undocked, and where its window last was.
/// See [`UiSettings::detached`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct DetachedState {
    /// True when the module is drawn in its own window instead of the main one.
    pub detached: bool,
    /// Where its window last was; `None` until it has been open once.
    #[serde(default)]
    pub window: Option<DetachedWindow>,
}

/// Which broadcast stations a listener's screen is allowed to show.
///
/// **The window is the panadapter's span, and that is right for the waterfall
/// label** — a station off the edge of a 10 kHz window is not something to
/// label on it. But the LISTEN window's map is a whole-band picture, and a
/// listener looking at it wants the band, not a sliver. The two disagree, so
/// the choice is the operator's rather than ours (`#80`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BroadcastScope {
    /// Only what the panadapter's own span covers, half a span either side.
    InView,
    /// Everything the schedule has, whether it is on screen or not.
    WholeBand,
}

impl Default for BroadcastScope {
    fn default() -> Self {
        // Unchanged from before this existed: a fresh screen behaves as every
        // screen has.
        BroadcastScope::InView
    }
}

impl BroadcastScope {
    pub fn label(self) -> &'static str {
        match self {
            BroadcastScope::InView => "On this span",
            BroadcastScope::WholeBand => "The whole band",
        }
    }
}

/// User display preferences. All have defaults so a missing `[ui]` table loads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// A row of finger-sized tuning buttons under the strip on a phone or
    /// tablet: step down, the step itself, step up.
    ///
    /// On by default, and never drawn on a desktop, which has the dial, the
    /// wheel and the panadapter. A touched client has none of those working
    /// well: the readout's per-digit scroll needs a wheel, and dragging the
    /// panadapter to a station 3 kHz away is a gesture nobody lands (issue
    /// #380). Costs one chip-height of waterfall, which is why it can be
    /// turned off.
    pub tune_step_buttons: bool,
    /// What one press of those buttons moves the dial, in hertz. Remembered
    /// between sessions: an operator working one band works one channel
    /// spacing.
    pub tune_step_hz: f64,
    /// Make a press of the step row land on the step's own grid — the next
    /// multiple of the step in the direction pressed — instead of moving the
    /// dial by exactly the step. A dial already on the grid moves by the step,
    /// so only the first press from an untidy frequency differs.
    ///
    /// **On** by default. It shipped off, on the argument that the step buttons
    /// move by exactly the step and a press that lands somewhere else is a
    /// second, invisible edit — see `tune_step_row`. That is the tidy-dial
    /// argument, and the untidy dial is the one that hurts: a frequency typed
    /// or panned to lands on 27 265 436 and every press after that carries the
    /// odd 436 Hz around the band. On, the first press tidies it and the rest
    /// move by the step as usual, which is the touch-screen habit it was added
    /// for (issue #422, and asked for again as a default on a phone — fork
    /// discussion #9). Only meaningful with [`Self::tune_step_buttons`].
    pub tune_step_round_first: bool,
    /// Whether the waterfall's history is drawn through a smoothing filter.
    ///
    /// On — the default, and what it has always done — each screen pixel is
    /// interpolated between the bins and rows around it, which is what makes a
    /// signal look continuous when the display is wider than the transform. Off
    /// draws every bin and every row as the block it is, which is what a
    /// listener reading a signal's *signature* off the waterfall wants: an
    /// interpolated picture cannot be told apart from a genuinely wider signal
    /// (issue #375).
    pub waterfall_smooth: bool,
    /// Freeze the waterfall while this station is transmitting.
    ///
    /// Off by default. On, the waterfall stops scrolling for the length of an
    /// over and resumes where it left off, so the picture is continuous
    /// received history rather than a block of rows showing the transmitter's
    /// own signal (or, on a radio that mutes its receiver during transmit, rows
    /// of noise that never happened). The spectrum line is untouched — it still
    /// follows the receiver.
    ///
    /// For a listener who leaves a recording running this keeps the two halves
    /// of a QSO together: the other station's transmission, then your reply,
    /// then theirs again with no gap where your own over sat.
    pub waterfall_freeze_on_tx: bool,
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
    /// Fill the area under the spectrum trace with a colour that is full at
    /// the bottom and fades out as it rises to the line. Off by default, so
    /// the trace stays the bare line it has always been. The SPEC popup's
    /// FILL chip switches the same flag.
    #[serde(default)]
    pub spectrum_fill: bool,
    /// The colour under the trace (sRGB, 0–255). The line keeps its own cyan.
    #[serde(default = "default_spectrum_fill_color")]
    pub spectrum_fill_color: [u8; 3],
    /// How the fill is shaded — fixed, fading, or half of each.
    #[serde(default)]
    pub spectrum_fill_style: SpectrumFillStyle,
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
    /// The operator's own zoom on top of [`Self::menu_font_size`] — what
    /// ctrl+plus / ctrl+minus (and ctrl+0) set. Kept so a window zoomed out to
    /// fit every control opens that way next time instead of at 100 %
    /// (issue #425). `1.0` is no zoom.
    pub ui_zoom: f32,
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
    /// **Retired** global SWL-mode switch. SWL mode is per radio now
    /// (`RadioConfig::hide_tx`, Settings → Radio), because what a radio is for
    /// is a property of the radio, not of the screen. This is still read once
    /// at startup to seed [`Self::start_swl`]-style behaviour for a listener
    /// upgrading from a version that only had the global switch; nothing in
    /// the UI writes it any more, and it is cleared the first time the
    /// per-radio switch is used.
    #[serde(default)]
    pub swl: bool,
    /// Start every session in SWL mode, whether or not [`Self::swl`] was left
    /// on when the program closed. Off by default — and the box beside it in
    /// Settings is the "unless the user asked for it" half of the rule: a
    /// listener ticks it once and gets the listener's interface at every start;
    /// nobody else is changed.
    #[serde(default)]
    pub start_swl: bool,
    /// Whether the operator has told the out-of-band transmit warning not to
    /// come up again on this screen.
    ///
    /// The warning's button is a one-shot acknowledgement for the session;
    /// this is the remembered half, ticked from the checkbox beside it by an
    /// operator who runs with `--oob-tx` every launch and knows what it means.
    /// In `[ui]` because it belongs to the screen, not the station — a remote
    /// client is warned from the engine's state too, and should make its own
    /// choice rather than inherit the shack machine's.
    #[serde(default)]
    pub oob_tx_dismissed: bool,
    /// Whether the operator has confirmed the one-time warning shown when they
    /// first switch on transmit for the 11 m citizens' band: that CB is not an
    /// amateur band, and that using it is subject to the rules of the country
    /// they are in.
    ///
    /// The confirmation is taken once per screen rather than every time the
    /// switch is flipped. In `[ui]` like `oob_tx_dismissed`, for the same
    /// reason as there: the permission itself is the station's
    /// (`cb_tx_allowed` in `config.toml`), the acknowledgement is the screen's.
    #[serde(default)]
    pub cb_tx_warning_ack: bool,
    /// Simple interface. When on, the chips for the advanced extras — the 3D
    /// view, the skimmers, the layer switches, award tracking, satellites, ISM
    /// decoding, radio email — are hidden from the top strip, leaving the
    /// controls a CB operator or a short-wave listener actually reaches for.
    ///
    /// A display preference like the theme, so each screen chooses; the
    /// features themselves are untouched, only their entry points. Off by
    /// default: an operator arriving from the full interface should not find
    /// parts of it missing until they ask.
    #[serde(default)]
    pub simple_ui: bool,
    /// Keep the screen on: ask the browser for a Screen Wake Lock so a phone
    /// or tablet left on the waterfall does not dim and lock itself. A
    /// property of this screen, like the theme, and carried with the profile's
    /// screen ([`ClientScreen`]) so it is remembered on the server; off by
    /// default, because it costs battery. The installed program has no
    /// equivalent and says so.
    #[serde(default)]
    pub keep_screen_on: bool,
    /// Show the welcome message for the sound when the web client opens: a
    /// browser plays nothing until the page is touched, and without the
    /// message a fresh session reads as broken. While it is up, the first
    /// click, tap or key starts the sound and does nothing else. On by
    /// default, for the beginner it is there for; an operator who knows can
    /// turn it off. Carried in the profile's screen ([`ClientScreen`]).
    #[serde(default = "default_true")]
    pub sound_welcome: bool,
    /// Where the solar-system 3D window last was — see [`Solar3dWindow`].
    ///
    /// Here rather than in the operator's view state because window geometry is
    /// a property of this screen, not of the radio, and a remote client keeps
    /// its own.
    #[serde(default)]
    pub solar3d_window: Option<Solar3dWindow>,
    /// Also carry this remote client's **control bindings** in the server
    /// profile, as well as its screen (which always travels — see
    /// `ClientScreen`).
    ///
    /// Off by default, and **not recommended**. On a station several people
    /// share, the keyboard is shared too: a profile that carries bindings would
    /// rebind another operator's PTT, Space or tuning keys merely by signing
    /// in. The setting is for a one-person server reached from several devices,
    /// where the bindings hurt nobody — and turning it on asks for an
    /// acknowledgement of the risk first. Fork-only.
    #[serde(default)]
    pub client_share_bindings: bool,
    /// This client has already been **asked** about a profile's bindings and
    /// said no. Without it the button reading "Keep mine (and stop asking)"
    /// was the only one of the two answers that survived nothing: the opt-in
    /// is persisted on *yes*, so a decline left the offer to come back next
    /// session and on every radio of the station. Fork-only, and deliberately
    /// client-local like the flag above, so a decline here never declines it on
    /// another machine.
    #[serde(default)]
    pub client_bindings_declined: bool,
    /// Advisories this operator has dismissed for good, as a bit over
    /// [`UiSettings::ADVISORY_KEYS`].
    ///
    /// **Advice only, never state.** A warning that reports something true *now*
    /// — the front end is overloading, the connection is gone, transmit is
    /// locked — must not be dismissible, because it stops being true the moment
    /// the condition clears and would leave a disabled control behind. What is
    /// here is the other kind: "this will do badly unless you widen something",
    /// which is true indefinitely and is answered by having read it once.
    ///
    /// Keyed by *identity* rather than by text, so a later warning about a
    /// different number is still shown. Client-local like the flags above, and
    /// deliberately not on [`crate::ClientScreen`]: whether you want to be told
    /// again is a decision made on a machine.
    ///
    /// A bitmask rather than a set because `UiSettings` is `Copy` and a
    /// collection is not — and because a bounded, **named** list is the better
    /// design anyway: it makes "the things you may dismiss" a closed set that
    /// can be read in one place, rather than whatever has opted in.
    ///
    /// State is never in it. If a line reports something true *now*, dismissing
    /// it leaves a dead control behind the moment the condition clears.
    #[serde(default)]
    pub dismissed_advisories: u64,
    /// Ask the fork's GitHub Releases whether a **newer release** exists, once
    /// at start-up, and show a banner if so. On by default; off means no
    /// outbound call at all. Only full releases count — the nightly is a
    /// pre-release, so it never triggers the banner.
    #[serde(default = "default_true")]
    pub check_for_updates: bool,
    /// Which modules are undocked into their own windows, and where those
    /// windows last were — one [`DetachedState`] per [`DetachableModule`], in
    /// `module.index()` order. An array rather than a map so `UiSettings` stays
    /// `Copy`. `#[serde(default)]`: a config written before this loads and
    /// simply starts docked.
    ///
    /// **Native only.** The browser has no second window and keeps every module
    /// in-window, so the flags are ignored there. Machine-local, and
    /// deliberately not on the wire (`ClientScreen`): where a window sits is a
    /// property of this screen, as [`Self::solar3d_window`]'s note says.
    ///
    /// Read through [`detached_slots`], not as a fixed array, **because a module
    /// added to the enum makes this array longer**: a config written by a build
    /// with three slots carries a three-element list, and a derived array
    /// deserializer would reject the whole file over it — and
    /// `Settings::load` *quarantines* a file it cannot parse and hands back
    /// defaults, so adding AUX SP that way would have reset every operator's
    /// theme, fonts and layout along with their window geometry. A short list
    /// leaves the modules it does not reach docked, which is what a fresh slot
    /// wants anyway.
    #[serde(default = "default_detached", deserialize_with = "detached_slots")]
    pub detached: [DetachedState; DetachableModule::COUNT],
    /// Legacy — the panadapter's undocked flag from before [`Self::detached`]
    /// existed, read once at load by [`Self::migrate_detached`] and never
    /// written again. Public only because a `..*settings` struct update needs
    /// every field; do not read it, use [`Self::is_detached`].
    #[serde(default, skip_serializing, rename = "panadapter_detached")]
    pub panadapter_detached_legacy: bool,
    /// Legacy — the panadapter window geometry from before the array. See the
    /// note above.
    #[serde(default, skip_serializing, rename = "panadapter_window")]
    pub panadapter_window_legacy: Option<DetachedWindow>,
    /// Which broadcast stations a listener's screen may show — see
    /// [`BroadcastScope`]. Defaults to the span, which is what every screen did
    /// before the choice existed.
    #[serde(default)]
    pub broadcast_scope: BroadcastScope,
    /// How the **survey maps** are painted — the aircraft picture today, and
    /// the shipping and datalink ones if they take it up.
    ///
    /// Three answers, because the two are for different questions. *Radar* is
    /// the dark high-contrast ground an air picture wants and the one this
    /// program drew first. *Terrain* fills the land so a country reads as a
    /// country. *Chart* is the pale marked-up look of an aeronautical chart,
    /// graticule and all.
    ///
    /// Asked for by two readers on r/ShortwavePlus (2026-10-17), the moderator
    /// putting it precisely: *"a choice between nav maps and regular
    /// terrain/road maps"*. Roads are not ours to give — see the note on the
    /// data in `sdroxide-ui`'s `basemap` — but the choice is.
    #[serde(default)]
    pub survey_map_style: MapStyle,
    /// **Classic UI** — every control in one column down the left, the way
    /// SDR++ and SDR++Brown lay theirs out, with each group a section that
    /// folds away.
    ///
    /// It exists because that is the layout most short-wave listeners already
    /// know: the two readers who called this program "too complicated, controls
    /// all over the place" on r/ShortwavePlus were both SDR++ users, and what
    /// they were describing is the difference between a strip of boxes with
    /// twelve menus behind it and one visible column. Same program, same
    /// controls, arranged the way a hand already knows.
    ///
    /// Off by default: the strip is what this program's own screens are built
    /// around, and a layout is a bigger thing than a preference to change under
    /// someone. Local `config.toml`, not [`ClientScreen`] — it arranges *this*
    /// window, which is the machine's business and not a station's.
    #[serde(default)]
    pub classic_ui: bool,

    /// **The order of the Classic UI column's sections**, top to bottom.
    ///
    /// The operator's own arrangement, dragged into place: he wanted the
    /// recorder at the top of his column, and one shipped order is a guess about
    /// which controls a listener reaches for first.
    ///
    /// A comma-separated list of section keys, and **deliberately a string**:
    /// any text at all parses, where a list of the enum's names would make the
    /// whole config unparseable the day a section is added or removed — and
    /// `Settings::load` answers an unparseable `config.toml` by **quarantining
    /// it and returning defaults**. An operator would lose their theme, their
    /// fonts and their layout to a list of sections. Read it through
    /// [`ClassicSection::order`]: unknown names are dropped and anything missing
    /// is appended, so a value this build does not understand costs the
    /// arrangement and nothing else.
    #[serde(default)]
    pub classic_sections: String,
}

/// One section of the Classic UI column — the units the operator drags into the
/// order he wants.
///
/// A section is a *category*, not a module: each is a fold of the column with a
/// heading of its own, and moving one moves everything under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassicSection {
    Source,
    Bands,
    Receiver,
    Tuning,
    Transmit,
    Recorder,
    Display,
    Tools,
}

impl ClassicSection {
    /// Every section, in the order the column ships with — SDR++'s order top to
    /// bottom, and the order anything missing from a stored list comes back in.
    pub const ALL: [Self; 8] = [
        // **The operator's order, and it is not the obvious one.** SDR++ puts the
        // front end first because that is where its device picker lives; here the
        // tools and the receiver are what a listener reaches for, so they lead and
        // the device readout follows. He asked for it in as many words: *"default
        // the Windows and Tools to the top and the Radio 2nd or 3rd"*. A stored
        // arrangement is his own and is never re-ordered by this — `ALL` is the
        // order a *fresh* column opens in, and the order anything missing from a
        // stored list is appended in.
        Self::Tools,
        Self::Receiver,
        Self::Bands,
        Self::Source,
        Self::Tuning,
        Self::Transmit,
        Self::Recorder,
        Self::Display,
    ];

    /// What the heading says.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Bands => "Band & mode",
            Self::Receiver => "Receiver",
            Self::Tuning => "Tuning",
            Self::Transmit => "Transmit",
            Self::Recorder => "Recorder",
            Self::Display => "Display",
            Self::Tools => "Windows & tools",
        }
    }

    /// The name this section is stored under, and the egui id its fold is
    /// remembered by. Stable on purpose: renaming a heading must not close a
    /// section an operator left open.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Bands => "bands",
            Self::Receiver => "receiver",
            Self::Tuning => "tuning",
            Self::Transmit => "transmit",
            Self::Recorder => "recorder",
            Self::Display => "display",
            Self::Tools => "tools",
        }
    }

    /// Open when the column is new? The three a newcomer reaches for first; the
    /// rest start folded away.
    pub fn default_open(&self) -> bool {
        // What a listener reaches for first, and the two the operator put at the
        // top of the column: everything else opens folded.
        matches!(self, Self::Tools | Self::Receiver | Self::Bands)
    }

    /// **Read a stored order.** Unknown names are dropped, repeats are dropped,
    /// and any section the stored list does not mention is appended in
    /// [`Self::ALL`] order — so a list written before a section existed still
    /// shows it, and a list naming one this build no longer has does not leave a
    /// heading with nothing under it. Both halves are the point: the operator's
    /// order is kept, and nothing is lost by an upgrade.
    pub fn order(stored: &str) -> Vec<Self> {
        let mut list: Vec<Self> = stored
            .split(',')
            .filter_map(|name| {
                let name = name.trim();
                Self::ALL.iter().copied().find(|s| s.key() == name)
            })
            .collect();
        list.dedup();
        for s in Self::ALL {
            if !list.contains(&s) {
                list.push(s);
            }
        }
        list
    }

    /// Write an order back out — the inverse of [`Self::order`].
    pub fn order_text(list: &[Self]) -> String {
        list.iter().map(|s| s.key()).collect::<Vec<_>>().join(",")
    }
}

/// How a survey map is painted. See [`UiSettings::survey_map_style`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapStyle {
    /// The dark ground with the land filled: what an air picture is read
    /// against, and the default because it is what this program drew first.
    Radar,
    /// Land lifted and warmed so it reads as ground; politics dimmed, because
    /// terrain does not care where a border is.
    Terrain,
    /// Pale land, mid sea, borders picked out, and a **graticule** — the
    /// latitude and longitude lines of a chart, which is what makes it read as
    /// one rather than as a bright map.
    Chart,
}

impl MapStyle {
    pub const ALL: [MapStyle; 3] = [MapStyle::Radar, MapStyle::Terrain, MapStyle::Chart];

    pub fn label(self) -> &'static str {
        match self {
            MapStyle::Radar => "Radar",
            MapStyle::Terrain => "Terrain",
            MapStyle::Chart => "Chart",
        }
    }

    /// The next one round, for a chip that cycles. Three states is a button,
    /// not a menu — the noise-reduction picker at sixteen is the other case.
    pub fn next(self) -> MapStyle {
        match self {
            MapStyle::Radar => MapStyle::Terrain,
            MapStyle::Terrain => MapStyle::Chart,
            MapStyle::Chart => MapStyle::Radar,
        }
    }
}

impl Default for MapStyle {
    fn default() -> Self {
        // What the aircraft picture has always been drawn as.
        MapStyle::Radar
    }
}

/// Default for [`UiSettings::spot_colors`] — every kind on its stock tint.
/// `true` for a `#[serde(default)]` field that must default on, not off.
fn default_true() -> bool {
    true
}

/// The fill under the spectrum trace starts as the line's own cyan.
fn default_spectrum_fill_color() -> [u8; 3] {
    [120, 220, 255]
}

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

/// Default for [`UiSettings::detached`] — every module docked, no windows
/// placed yet. The same state a build starts in, so a slot a config does not
/// reach needs nothing else.
fn default_detached() -> [DetachedState; DetachableModule::COUNT] {
    [DetachedState::default(); DetachableModule::COUNT]
}

/// Read [`UiSettings::detached`] as a list of any length, so a config written
/// before a module was added — or by a build that has one more — still loads.
///
/// This is the one field that has to survive the registry growing, and the cost
/// of not doing it is not a lost setting but the whole file:
/// `sdroxide_config::Settings::load` quarantines a `config.toml` it cannot parse
/// and answers `Settings::default()`, so a single over-long (or short) `detached`
/// list would take the operator's theme, fonts, layout and the rest of their
/// configuration with it. A short list leaves the modules it does not reach
/// docked; a long one has its tail ignored, the way a newer build's extra module
/// is.
///
/// Safe to leave on the wire-adjacent types because `UiSettings` is local
/// `config.toml` and is never postcard-encoded — see the field's note.
fn detached_slots<'de, D>(d: D) -> Result<[DetachedState; DetachableModule::COUNT], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let list = Vec::<DetachedState>::deserialize(d)?;
    let mut out = default_detached();
    for (slot, state) in out.iter_mut().zip(list) {
        *slot = state;
    }
    Ok(out)
}

impl UiSettings {
    /// The advisories that may be dismissed for good, in bit order.
    ///
    /// Deliberately short. Every entry is a thing that is **advice** — true
    /// until the operator acts, answered by having read it once — and nothing
    /// that reports a live condition. Adding a name here is a decision that a
    /// warning is safe to lose, which is exactly the decision that should be
    /// made in one visible list rather than spread across call sites.
    pub const ADVISORY_KEYS: &'static [&'static str] =
        &["dab-rate", "ais-coverage", "adsb-coverage", "vdl2-coverage"];

    /// The bit an advisory's dismissal lives in, or `None` for a name that is
    /// not in [`UiSettings::ADVISORY_KEYS`].
    ///
    /// `None` means **always shown**, and that is the safe answer: an unknown
    /// key cannot be dismissed, so adding a call site without deciding whether
    /// it should be dismissible leaves it nagging rather than muting it.
    pub fn advisory_bit(key: &str) -> Option<u32> {
        Self::ADVISORY_KEYS.iter().position(|k| *k == key).map(|i| i as u32)
    }

    /// Has this client dismissed `key`?
    pub fn advisory_dismissed(&self, key: &str) -> bool {
        match Self::advisory_bit(key) {
            Some(bit) => self.dismissed_advisories & (1u64 << bit) != 0,
            None => false,
        }
    }

    /// Remember that this client has dismissed `key`. An unknown key is ignored,
    /// which is the same safe direction as [`UiSettings::advisory_dismissed`].
    pub fn dismiss_advisory(&mut self, key: &str) {
        if let Some(bit) = Self::advisory_bit(key) {
            self.dismissed_advisories |= 1u64 << bit;
        }
    }
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
            waterfall_smooth: true,
            waterfall_freeze_on_tx: false,
            tune_step_buttons: true,
            // 1 kHz: the round step the operators who asked for this tune in,
            // on a band where the stations sit 3 kHz apart.
            tune_step_hz: 1_000.0,
            tune_step_round_first: true,
            spectrum_detail: SpectrumDetail::Auto,
            spectrum_gradient: true,
            gradient_top: [64, 0, 0],   // dark red
            gradient_bottom: [0, 0, 0], // black
            spectrum_fill: false,
            spectrum_fill_color: default_spectrum_fill_color(),
            spectrum_fill_style: SpectrumFillStyle::Gradient,
            spot_colors: default_spot_colors(),
            bandplan_colors: default_bandplan_colors(),
            layout: LayoutMode::Auto,
            theme: UiTheme::Default,
            button_style: ChromeStyle::Angled,
            window_style: ChromeStyle::Angled,
            skimmer_font_size: FontSize::Medium,
            waterfall_font_size: FontSize::Small,
            menu_font_size: FontSize::Medium,
            ui_zoom: 1.0,
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
            swl: false,
            oob_tx_dismissed: false,
            cb_tx_warning_ack: false,
            simple_ui: false,
            keep_screen_on: false,
            sound_welcome: true,
            start_swl: false,
            solar3d_window: None,
            client_share_bindings: false,
            client_bindings_declined: false,
            dismissed_advisories: 0,
            check_for_updates: true,
            detached: [DetachedState::default(); DetachableModule::COUNT],
            panadapter_detached_legacy: false,
            panadapter_window_legacy: None,
            broadcast_scope: BroadcastScope::default(),
            survey_map_style: MapStyle::default(),
            classic_ui: false,
            classic_sections: String::new(),
        }
    }
}

impl UiSettings {
    /// The steps the tuning buttons cycle through, in hertz — from the 10 Hz
    /// that trims a carrier onto zero-beat up to the 25 kHz of an FM channel,
    /// by way of the AM broadcast spacings (9 kHz in Regions 1 and 3, 10 kHz in
    /// Region 2), the 5 kHz most shortwave broadcasters sit on, and the VHF/UHF
    /// channel spacings — 6.25 kHz (dPMR, NXDN, PMR446 digital) and 12.5 kHz
    /// (narrow FM, PMR446, DMR) — beside the 25 kHz wide channel (issue #28).
    pub const TUNE_STEPS_HZ: [f64; 11] = [
        10.0, 100.0, 500.0, 1_000.0, 2_500.0, 5_000.0, 6_250.0, 9_000.0, 10_000.0, 12_500.0,
        25_000.0,
    ];

    /// The step after the current one, wrapping. A tap on the step button.
    pub fn next_tune_step(&self) -> f64 {
        let steps = Self::TUNE_STEPS_HZ;
        let at = steps.iter().position(|s| (s - self.tune_step_hz).abs() < 0.5);
        steps[at.map_or(0, |i| (i + 1) % steps.len())]
    }

    /// The current step written the way a radio's own display would: "100 Hz",
    /// "1 kHz", "12.5 kHz", "6.25 kHz" — as many decimals as the step has, so
    /// 6.25 kHz is not rounded to a 6.2 that is on no channel plan.
    pub fn tune_step_label(&self) -> String {
        let hz = self.tune_step_hz;
        if hz < 1_000.0 {
            return format!("{hz:.0} Hz");
        }
        let khz = format!("{:.3}", hz / 1_000.0);
        format!("{} kHz", khz.trim_end_matches('0').trim_end_matches('.'))
    }

    /// One module's undocked state, or the docked default if it has never been
    /// touched.
    pub fn detached_state(&self, module: DetachableModule) -> DetachedState {
        self.detached[module.index()]
    }

    /// Whether a module is undocked.
    pub fn is_detached(&self, module: DetachableModule) -> bool {
        self.detached[module.index()].detached
    }

    /// Set (or clear) a module's undocked flag, leaving its remembered window
    /// geometry alone.
    pub fn set_detached(&mut self, module: DetachableModule, on: bool) {
        self.detached[module.index()].detached = on;
    }

    /// Fold the pre-array `panadapter_detached` / `panadapter_window` keys from
    /// an older config into [`Self::detached`]. Idempotent, so the two load
    /// paths (native `config.toml`, browser storage) can both call it and
    /// neither has to know whether there is anything to do.
    pub fn migrate_detached(&mut self) {
        if !self.panadapter_detached_legacy && self.panadapter_window_legacy.is_none() {
            return;
        }
        let entry = &mut self.detached[DetachableModule::Panadapter.index()];
        entry.detached |= self.panadapter_detached_legacy;
        if entry.window.is_none() {
            entry.window = self.panadapter_window_legacy;
        }
        self.panadapter_detached_legacy = false;
        self.panadapter_window_legacy = None;
    }

    /// Selectable frame rates for the UI combo.
    ///
    /// The rates below 30 are for machines that cannot keep up — a Raspberry Pi
    /// driving a 4K panel, a remote client on a thin laptop. They cost detail in
    /// the waterfall (fewer distinct rows; the scroll speed is absolute, so a
    /// row is simply repeated) and nothing else: the engine still processes
    /// every sample, and only the spectrum frame it publishes slows down.
    pub const FPS_OPTIONS: [u32; 6] = [5, 10, 15, 30, 60, 90];

    /// Frame rate clamped to a sane range (guards a hand-edited config).
    pub fn fps(&self) -> u32 {
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
    pub fn waterfall_rows_per_sec(&self) -> f32 {
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
    pub fn spectrum_3d_rows_per_sec(&self) -> f32 {
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
    pub fn spectrum_avg_tc(&self) -> f32 {
        match self.spectrum_speed {
            // The waterfall-only rates mean the same thing here as `Fast`:
            // no averaging. Reachable only from a hand-edited config.
            Speed::Fast | Speed::Faster | Speed::Fastest => 0.0,
            Speed::Medium => 0.1,
            Speed::Slow => 0.2,
        }
    }
}

/// A remote client's **screen settings**, in the form that may cross the wire
/// and be kept on the server (`UiSettings::client_save_scope`).
///
/// It is a type of its own rather than a [`UiSettings`] because that struct
/// **cannot** be postcard-encoded: two of its fields (`spot_colors`,
/// `bandplan_colors`) deserialize through custom functions that read a `Vec`
/// where the serializer writes a fixed array — a mismatch self-describing TOML
/// absorbs and postcard cannot. Sending the whole struct would have been a
/// wire bug.
///
/// It is also deliberately narrow. Everything here is a look or a layout —
/// nothing that points the program anywhere, and nothing that is a fact about
/// the machine rather than the person:
///
/// - window geometry, display zoom, the decode-list views and the one-shot
///   acknowledgements are all left behind, so adopting a login's screen never
///   moves a window off a laptop or swallows a warning the operator has not
///   seen.
///
/// So it is what an operator actually redoes each session — theme, layout,
/// waterfall and spectrum look, fonts, Simple UI, the map layers —
/// and no more. (There was never an injection risk: `UiSettings` carries no
/// URLs, paths or feeds, only scalars. This is a scope cut, not a guard.)
/// The "do not ask me again" answers a remote client has given, kept **on the
/// server** against the profile it signed in as.
///
/// Browser storage cannot hold them reliably — not every browser grants
/// persistent storage, some refuse cookies, and a reload can land before the
/// next periodic save — so a question answered "stop asking" came back on every
/// session. The server is the one place every device of a login reaches, and the
/// one an administrator can reset.
///
/// Every field is a one-way answer the program has no "undo" for except the
/// bindings choice, which is why [`ClientAcks::merge`] is a union: two radio
/// tabs of one login each send their own copy, and neither may erase the
/// other's answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientAcks {
    /// The answer to "this profile carries keyboard bindings": `Some(true)` use
    /// them, `Some(false)` keep this device's own (and the server stops sending
    /// them), `None` not answered yet.
    pub bindings: Option<bool>,
    /// [`UiSettings::dismissed_advisories`].
    pub dismissed_advisories: u64,
    /// [`UiSettings::cb_tx_warning_ack`].
    pub cb_tx_warning_ack: bool,
    /// Radios (by station id) whose "this radio is receive-only — use the
    /// listening screen?" banner was dismissed.
    pub rx_only_dismissed: Vec<u32>,
}

impl ClientAcks {
    /// Fold `other` in. Dismissals only accumulate; a bindings answer replaces
    /// the stored one, and an unanswered one (`None`) leaves it alone.
    pub fn merge(&mut self, other: &ClientAcks) {
        if other.bindings.is_some() {
            self.bindings = other.bindings;
        }
        self.dismissed_advisories |= other.dismissed_advisories;
        self.cb_tx_warning_ack |= other.cb_tx_warning_ack;
        for id in &other.rx_only_dismissed {
            if !self.rx_only_dismissed.contains(id) {
                self.rx_only_dismissed.push(*id);
            }
        }
        self.rx_only_dismissed.sort_unstable();
    }

    /// The bindings answer as the two client flags express it.
    pub fn bindings_from(share: bool, declined: bool) -> Option<bool> {
        if share {
            Some(true)
        } else if declined {
            Some(false)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientScreen {
    pub frame_rate_fps: u32,
    pub waterfall_speed: Speed,
    pub spectrum_speed: Speed,
    pub spectrum_3d_speed: Speed,
    pub waterfall_palette: usize,
    pub waterfall_smooth: bool,
    pub waterfall_freeze_on_tx: bool,
    pub spectrum_detail: SpectrumDetail,
    pub spectrum_gradient: bool,
    pub gradient_top: [u8; 3],
    pub gradient_bottom: [u8; 3],
    pub spot_colors: [[u8; 3]; SpotKind::COUNT],
    pub bandplan_colors: [[u8; 3]; BandplanKind::COUNT],
    pub layout: LayoutMode,
    pub theme: UiTheme,
    pub button_style: ChromeStyle,
    pub window_style: ChromeStyle,
    pub skimmer_font_size: FontSize,
    pub waterfall_font_size: FontSize,
    pub menu_font_size: FontSize,
    pub smeter_style: SmeterStyle,
    pub map_cities: bool,
    pub simple_ui: bool,
    /// Keep the tuned frequency in the middle of the panadapter.
    ///
    /// **Carried here as a plain value, not as part of `UiSettings`**, because
    /// the panadapter's own settings live in the client's `ViewState` — which
    /// this crate cannot name. `from_settings` cannot see it, so it fills the
    /// default and the client overwrites it from the view it has; the same
    /// number comes back and the client puts it into the view again. The point
    /// is that a profile can hold it at all: reported as settings that "are
    /// never remembered" (fork discussion #9), because the browser's own copy
    /// is cleared when a phone sleeps and nothing carried them to the server.
    pub center_on_vfo: bool,
    /// The panadapter FFT size — `ViewState::fft_size`, carried for the same
    /// reason as [`Self::center_on_vfo`].
    pub fft_size: u32,
    /// Whether the first press of the step row tidies the dial onto the step.
    pub tune_step_round_first: bool,
    /// The web client's welcome message for the sound —
    /// [`UiSettings::sound_welcome`]. Appended last: this struct rides the
    /// wire positionally.
    pub sound_welcome: bool,
    /// Keep the screen on — [`UiSettings::keep_screen_on`]. Appended last
    /// with the field above.
    pub keep_screen_on: bool,
    /// The colour under the spectrum trace — [`UiSettings::spectrum_fill`]
    /// and [`UiSettings::spectrum_fill_color`]. Appended last: this struct
    /// rides the wire positionally.
    pub spectrum_fill: bool,
    pub spectrum_fill_color: [u8; 3],
    /// How the fill is shaded — [`UiSettings::spectrum_fill_style`].
    /// Appended last: this struct rides the wire positionally.
    pub spectrum_fill_style: SpectrumFillStyle,
    /// Which broadcast stations a screen may show — [`BroadcastScope`], and it
    /// belongs on the wire for the same reason the theme does: it is what the
    /// listener sees, and they see it from whichever device they signed in on.
    /// Appended last: this struct rides the wire positionally.
    pub broadcast_scope: BroadcastScope,
}

impl Default for ClientScreen {
    fn default() -> Self {
        Self::from_settings(&UiSettings::default())
    }
}

impl ClientScreen {
    /// Lift the screen fields out of a full [`UiSettings`].
    pub fn from_settings(s: &UiSettings) -> Self {
        ClientScreen {
            frame_rate_fps: s.frame_rate_fps,
            waterfall_speed: s.waterfall_speed,
            spectrum_speed: s.spectrum_speed,
            spectrum_3d_speed: s.spectrum_3d_speed,
            waterfall_palette: s.waterfall_palette,
            waterfall_smooth: s.waterfall_smooth,
            waterfall_freeze_on_tx: s.waterfall_freeze_on_tx,
            spectrum_detail: s.spectrum_detail,
            spectrum_gradient: s.spectrum_gradient,
            gradient_top: s.gradient_top,
            gradient_bottom: s.gradient_bottom,
            spot_colors: s.spot_colors,
            bandplan_colors: s.bandplan_colors,
            layout: s.layout,
            theme: s.theme,
            button_style: s.button_style,
            window_style: s.window_style,
            skimmer_font_size: s.skimmer_font_size,
            waterfall_font_size: s.waterfall_font_size,
            menu_font_size: s.menu_font_size,
            smeter_style: s.smeter_style,
            map_cities: s.map_cities,
            simple_ui: s.simple_ui,
            // The panadapter's own, filled by the client from its `ViewState`
            // — see the field. Defaults here are the view's own.
            center_on_vfo: true,
            fft_size: 4096,
            tune_step_round_first: s.tune_step_round_first,
            sound_welcome: s.sound_welcome,
            keep_screen_on: s.keep_screen_on,
            spectrum_fill: s.spectrum_fill,
            spectrum_fill_color: s.spectrum_fill_color,
            spectrum_fill_style: s.spectrum_fill_style,
            broadcast_scope: s.broadcast_scope,
        }
    }

    /// Lay these screen settings back over a full [`UiSettings`], leaving every
    /// field this does not carry — window geometry, zoom, list views, the
    /// acknowledgements, the save scope — exactly as the client had it.
    pub fn apply_to(&self, s: &mut UiSettings) {
        s.frame_rate_fps = self.frame_rate_fps;
        s.waterfall_speed = self.waterfall_speed;
        s.spectrum_speed = self.spectrum_speed;
        s.spectrum_3d_speed = self.spectrum_3d_speed;
        s.waterfall_palette = self.waterfall_palette;
        s.waterfall_smooth = self.waterfall_smooth;
        s.waterfall_freeze_on_tx = self.waterfall_freeze_on_tx;
        s.spectrum_detail = self.spectrum_detail;
        s.spectrum_gradient = self.spectrum_gradient;
        s.gradient_top = self.gradient_top;
        s.gradient_bottom = self.gradient_bottom;
        s.spot_colors = self.spot_colors;
        s.bandplan_colors = self.bandplan_colors;
        s.layout = self.layout;
        s.theme = self.theme;
        s.button_style = self.button_style;
        s.window_style = self.window_style;
        s.skimmer_font_size = self.skimmer_font_size;
        s.waterfall_font_size = self.waterfall_font_size;
        s.menu_font_size = self.menu_font_size;
        s.smeter_style = self.smeter_style;
        s.map_cities = self.map_cities;
        s.simple_ui = self.simple_ui;
        s.tune_step_round_first = self.tune_step_round_first;
        s.sound_welcome = self.sound_welcome;
        s.keep_screen_on = self.keep_screen_on;
        s.spectrum_fill = self.spectrum_fill;
        s.spectrum_fill_color = self.spectrum_fill_color;
        s.spectrum_fill_style = self.spectrum_fill_style;
        s.broadcast_scope = self.broadcast_scope;
    }
}

#[cfg(test)]
mod client_screen_tests {
    use super::*;

    /// The screen round-trips through postcard — the thing `UiSettings` itself
    /// cannot do. This is the test that would fail first if a non-postcard-safe
    /// field were ever added here.
    /// Two tabs of one login each send their own copy; neither may erase the
    /// other's answer, and a later explicit bindings answer replaces an earlier
    /// one while "not answered" never does.
    #[test]
    fn answers_accumulate_across_copies() {
        let mut stored = ClientAcks {
            rx_only_dismissed: vec![2],
            dismissed_advisories: 0b01,
            ..Default::default()
        };
        stored.merge(&ClientAcks {
            bindings: Some(false),
            rx_only_dismissed: vec![0],
            dismissed_advisories: 0b10,
            ..Default::default()
        });
        assert_eq!(stored.bindings, Some(false));
        assert_eq!(stored.rx_only_dismissed, vec![0, 2]);
        assert_eq!(stored.dismissed_advisories, 0b11);
        // A stale tab that never saw the answer sends `None`: it stays.
        stored.merge(&ClientAcks::default());
        assert_eq!(stored.bindings, Some(false));
        assert_eq!(stored.rx_only_dismissed, vec![0, 2]);
        // The operator changes their mind: the new answer replaces it.
        stored.merge(&ClientAcks { bindings: Some(true), ..Default::default() });
        assert_eq!(stored.bindings, Some(true));
    }

    #[test]
    fn the_answers_survive_a_postcard_round_trip() {
        let a = ClientAcks {
            bindings: Some(false),
            dismissed_advisories: 5,
            cb_tx_warning_ack: true,
            rx_only_dismissed: vec![0, 3],
        };
        let back: ClientAcks = postcard::from_bytes(&postcard::to_allocvec(&a).unwrap()).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn the_screen_survives_a_postcard_round_trip() {
        let screen = ClientScreen::from_settings(&UiSettings::default());
        let bytes = postcard::to_allocvec(&screen).expect("encodes");
        let back: ClientScreen = postcard::from_bytes(&bytes).expect("decodes");
        assert_eq!(back, screen);
    }

    /// The sound welcome is on unless asked off, and the wake lock off unless
    /// asked on — including for a profile stored before either was carried,
    /// which has neither key and must not lose its welcome message.
    #[test]
    fn the_sound_welcome_and_wake_lock_travel_with_the_profile() {
        let d = ClientScreen::default();
        assert!(d.sound_welcome, "the welcome message is on by default");
        assert!(!d.keep_screen_on, "the wake lock is off by default");

        let old: ClientScreen = serde_json::from_str(r#"{"simple_ui":true}"#).expect("old profile");
        assert!(old.sound_welcome, "a profile from before the field keeps the message");
        assert!(!old.keep_screen_on);

        let mut mine = UiSettings::default();
        mine.sound_welcome = false;
        mine.keep_screen_on = true;
        let carried = ClientScreen::from_settings(&mine);
        let back: ClientScreen =
            postcard::from_bytes(&postcard::to_allocvec(&carried).unwrap()).unwrap();
        let mut other = UiSettings::default();
        back.apply_to(&mut other);
        assert!(!other.sound_welcome, "the message stays off on the next device");
        assert!(other.keep_screen_on, "and the screen stays on");
    }

    /// The fill under the trace is off and cyan until asked otherwise —
    /// including for a profile stored before it was carried — and the
    /// operator's colour reaches the next device.
    #[test]
    fn the_spectrum_fill_travels_with_the_profile() {
        let d = ClientScreen::default();
        assert!(!d.spectrum_fill, "the fill is off by default");
        assert_eq!(d.spectrum_fill_color, [120, 220, 255], "the fill starts as the line's cyan");

        let old: ClientScreen = serde_json::from_str(r#"{"simple_ui":true}"#).expect("old profile");
        assert!(!old.spectrum_fill, "a profile from before the field has no fill");
        assert_eq!(old.spectrum_fill_color, [120, 220, 255]);
        assert_eq!(
            old.spectrum_fill_style,
            SpectrumFillStyle::Gradient,
            "the style starts as the gradient"
        );

        let mut mine = UiSettings::default();
        mine.spectrum_fill = true;
        mine.spectrum_fill_color = [40, 220, 60];
        mine.spectrum_fill_style = SpectrumFillStyle::Mixed;
        let carried = ClientScreen::from_settings(&mine);
        let back: ClientScreen =
            postcard::from_bytes(&postcard::to_allocvec(&carried).unwrap()).unwrap();
        let mut other = UiSettings::default();
        back.apply_to(&mut other);
        assert!(other.spectrum_fill, "the fill stayed behind");
        assert_eq!(other.spectrum_fill_color, [40, 220, 60], "the colour stayed behind");
        assert_eq!(other.spectrum_fill_style, SpectrumFillStyle::Mixed, "the style stayed behind");
    }

    /// Applying a served screen leaves every machine-specific field alone.
    #[test]
    fn applying_a_screen_keeps_the_machines_own_facts() {
        let mut live = UiSettings::default();
        live.ui_zoom = 1.7;
        live.oob_tx_dismissed = true;
        live.solar3d_window = Some(Solar3dWindow { size: [900.0, 600.0], pos: Some([10.0, 10.0]) });

        let mut served = UiSettings::default();
        served.theme = UiTheme::Dracula;
        served.simple_ui = true;
        ClientScreen::from_settings(&served).apply_to(&mut live);

        assert_eq!(live.theme, UiTheme::Dracula, "the look was adopted");
        assert!(live.simple_ui);
        assert_eq!(live.ui_zoom, 1.7, "the client's zoom is its own");
        assert!(live.oob_tx_dismissed, "and its acknowledgements");
        assert!(live.solar3d_window.is_some(), "and its window place");
    }
}

#[cfg(test)]
mod tune_step_tests {
    use super::UiSettings;

    /// The step button walks the ladder and comes back round, from wherever a
    /// stored setting left it — including a value that is not on the ladder at
    /// all, which is what a hand-edited `config.toml` can hold (issue #380).
    #[test]
    fn the_step_button_walks_the_ladder_and_wraps() {
        let mut ui = UiSettings::default();
        assert_eq!(ui.tune_step_hz, 1_000.0, "the shipped default is a round kilohertz");

        let mut seen = vec![ui.tune_step_hz];
        for _ in 1..UiSettings::TUNE_STEPS_HZ.len() {
            ui.tune_step_hz = ui.next_tune_step();
            seen.push(ui.tune_step_hz);
        }
        let mut sorted = seen.clone();
        sorted.sort_by(f64::total_cmp);
        assert_eq!(sorted, UiSettings::TUNE_STEPS_HZ, "every step is reachable, exactly once");
        ui.tune_step_hz = ui.next_tune_step();
        assert_eq!(ui.tune_step_hz, seen[0], "and the ladder wraps back to where it started");

        // A figure that is on no rung starts again at the bottom rather than
        // sticking, which is the only behaviour that cannot strand an operator.
        ui.tune_step_hz = 3_333.0;
        assert_eq!(ui.next_tune_step(), UiSettings::TUNE_STEPS_HZ[0]);
    }

    /// Written the way a radio's own display writes it.
    #[test]
    fn the_step_reads_as_a_radio_would_write_it() {
        let mut ui = UiSettings::default();
        for (hz, want) in [
            (10.0, "10 Hz"),
            (500.0, "500 Hz"),
            (1_000.0, "1 kHz"),
            (2_500.0, "2.5 kHz"),
            (6_250.0, "6.25 kHz"),
            (9_000.0, "9 kHz"),
            (12_500.0, "12.5 kHz"),
            (25_000.0, "25 kHz"),
        ] {
            ui.tune_step_hz = hz;
            assert_eq!(ui.tune_step_label(), want);
        }
    }
}

/// `--swl` for this process: force SWL mode on for the run regardless of the
/// stored preference. Set by the binary at startup; read when a tab is built.
/// A process-wide flag rather than a field on `UiSettings` because it is not a
/// preference — it lasts as long as the run and is never written to disk.
static FORCE_SWL: AtomicBool = AtomicBool::new(false);

/// Whether `--swl` asked for SWL mode this run.
pub fn force_swl() -> bool {
    FORCE_SWL.load(Ordering::Relaxed)
}

/// Set by the binary when `--swl` was passed.
pub fn set_force_swl(on: bool) {
    FORCE_SWL.store(on, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A stored section order is the operator's, and nothing in it is lost by
    /// an upgrade.** Both halves matter: what he dragged is kept in the order he
    /// dragged it, and a list that predates a section — or names one this build
    /// no longer has — still shows every section exactly once.
    #[test]
    fn a_section_order_survives_what_it_does_not_know() {
        // Nothing stored: the shipped order.
        assert_eq!(ClassicSection::order(""), ClassicSection::ALL.to_vec());
        // His own arrangement, kept as it is.
        let mine = "recorder,source,tuning";
        let mut expect =
            vec![ClassicSection::Recorder, ClassicSection::Source, ClassicSection::Tuning];
        // The rest follow in the shipped order — read from `ALL` rather than
        // written out, so changing the default cannot leave this assertion
        // agreeing with a stale copy of it.
        let rest: Vec<_> =
            ClassicSection::ALL.iter().copied().filter(|s| !expect.contains(s)).collect();
        expect.extend(rest);
        assert_eq!(
            ClassicSection::order(mine),
            expect,
            "what he placed keeps its place, and the rest follow in the shipped order"
        );
        // A name this build does not have is dropped, not a heading with nothing
        // under it — and not a config the loader quarantines.
        assert_eq!(ClassicSection::order("wat,source").len(), ClassicSection::ALL.len());
        // Repeats are one section, not two.
        let twice: Vec<_> = ClassicSection::order("source,source");
        assert_eq!(twice.iter().filter(|s| **s == ClassicSection::Source).count(), 1);
        // And it round-trips, so the next frame reads back what this one wrote.
        let list = ClassicSection::order("recorder,source");
        assert_eq!(ClassicSection::order(&ClassicSection::order_text(&list)), list);
    }

    /// A workspace snapshots the arrangement and puts it back, and a slot it
    /// does not carry leaves that module docked rather than half-restored.
    #[test]
    fn a_workspace_round_trips_the_arrangement() {
        let mut s = UiSettings::default();
        s.set_detached(DetachableModule::Panadapter, true);
        s.set_detached(DetachableModule::Controls, true);
        let saved = Workspace::capture("second monitor", &s);

        let mut back = UiSettings::default();
        saved.apply(&mut back);
        assert!(back.is_detached(DetachableModule::Panadapter));
        assert!(back.is_detached(DetachableModule::Controls));
        assert!(!back.is_detached(DetachableModule::Panel), "a module it did not set stays docked");
        assert!(!back.is_detached(DetachableModule::AuxPanadapter));
        assert_eq!(back.detached, s.detached, "the whole arrangement, not a subset");

        // A workspace written before a module existed carries a shorter list:
        // the modules it does reach are restored, and the rest stay docked.
        let short = Workspace {
            name: "old".into(),
            slots: vec![DetachedState { detached: true, window: None }],
        };
        let mut from_old = UiSettings::default();
        short.apply(&mut from_old);
        assert!(from_old.is_detached(DetachableModule::Panadapter));
        assert!(!from_old.is_detached(DetachableModule::Controls));
    }

    /// A `detached` list of **any** length loads, and a module the list does
    /// not reach simply starts docked.
    ///
    /// This is the field the registry grows through: adding AUX SP makes the
    /// array longer, and every config written before it carries a shorter list.
    /// Without the tolerant reader the whole `[ui]` table — and with it the
    /// theme, the fonts and the layout, because `Settings::load` quarantines a
    /// file it cannot parse — would go over one list length.
    #[test]
    fn the_detached_list_loads_at_any_length() {
        let one = |n: usize| {
            let list: Vec<String> = (0..n).map(|_| r#"{"detached":true}"#.to_string()).collect();
            let json = format!(r#"{{"detached":[{}]}}"#, list.join(","));
            serde_json::from_str::<UiSettings>(&json).expect("a short list")
        };

        let short = one(1);
        assert!(short.detached[0].detached, "the slot it does carry is kept");
        for m in &DetachableModule::ALL[1..] {
            assert!(
                !short.detached[m.index()].detached,
                "{:?} a list that does not reach it starts docked",
                m.label()
            );
        }

        // A longer list — a build with a module this one has not heard of — has
        // its tail ignored rather than failing the file.
        let long = one(DetachableModule::COUNT + 2);
        for m in DetachableModule::ALL {
            assert!(long.detached[m.index()].detached, "{:?} kept", m.label());
        }

        // And the reader keeps the operator's own windows, geometry included,
        // rather than defaulting the slot.
        let kept: UiSettings = serde_json::from_str(
            r#"{"detached":[{"detached":true,"window":{"size":[800.0,600.0],"pos":[10.0,20.0]}}]}"#,
        )
        .expect("a placed window");
        let w = kept.detached[0].window.expect("the window it carried");
        assert_eq!((w.size, w.pos), ([800.0, 600.0], Some([10.0, 20.0])));
    }

    /// The registry's own bookkeeping, which is what `UiSettings::detached` is
    /// indexed by: every module has a **distinct** index inside the array, and
    /// `ALL` and `COUNT` agree. Two modules sharing a slot would silently share
    /// one window's geometry, and an `ALL` longer than `COUNT` would index past
    /// the array.
    ///
    /// This is the test that has to keep passing **every** time a module is
    /// added, because adding one grows `detached` — and a saved config written
    /// before it carries a shorter list. The reader is tolerant of that (see
    /// [`detached_slots`]); this pins the bookkeeping the tolerance depends on.
    #[test]
    fn every_module_has_its_own_slot() {
        assert_eq!(DetachableModule::ALL.len(), DetachableModule::COUNT);
        let mut seen = [false; DetachableModule::COUNT];
        for m in DetachableModule::ALL {
            assert!(m.index() < DetachableModule::COUNT, "{:?} indexes past the array", m.label());
            assert!(
                !seen[m.index()],
                "{:?} shares slot {} with another module",
                m.label(),
                m.index()
            );
            seen[m.index()] = true;
        }
        assert!(seen.iter().all(|s| *s), "every slot is a module: {seen:?}");
    }

    /// The registry's own bookkeeping: every module has a **distinct** index and
    /// `ALL` and `COUNT` agree, because `UiSettings::detached` is indexed by it
    /// and two modules sharing a slot would silently share one module's window
    /// geometry.
    /// A layout value this build has never heard of costs that field, not the
    /// whole `config.toml`. `Settings::load` quarantines the entire file on a
    /// parse error, so before `#[serde(other)]` a config written by a build
    /// that knew a later `LayoutMode` took the radio, audio and speech settings
    /// down with the one unknown value (issue #469).
    #[test]
    fn an_unknown_layout_degrades_to_auto() {
        let m: LayoutMode = serde_json::from_str("\"Holographic\"").unwrap();
        assert_eq!(m, LayoutMode::Auto);
        // The known ones still name themselves.
        let d: LayoutMode = serde_json::from_str("\"Desktop\"").unwrap();
        assert_eq!(d, LayoutMode::Desktop);
        let s: LayoutMode = serde_json::from_str("\"Small\"").unwrap();
        assert_eq!(s, LayoutMode::Small);
        // And `ALL` — the picker's list — is unaffected by the declaration
        // order: it is written out, not derived.
        assert!(LayoutMode::ALL.contains(&LayoutMode::Auto));
        assert_eq!(LayoutMode::ALL.len(), 5);
    }

    /// The undocked state survives a serialise round trip, and the pre-array
    /// keys of an older config fold into the array rather than being dropped.
    #[test]
    fn undocked_state_round_trips_and_migrates() {
        // A fresh config starts docked everywhere.
        let mut s = UiSettings::default();
        for m in DetachableModule::ALL {
            assert!(!s.is_detached(m), "{m:?} must start docked");
        }
        // Undock the panel and give it a window; the panadapter stays docked.
        s.set_detached(DetachableModule::Panel, true);
        s.detached[DetachableModule::Panel.index()].window =
            Some(DetachedWindow { size: [520.0, 700.0], pos: None });
        let json = serde_json::to_string(&s).unwrap();
        let back: UiSettings = serde_json::from_str(&json).unwrap();
        assert!(back.is_detached(DetachableModule::Panel));
        assert!(!back.is_detached(DetachableModule::Panadapter));
        assert_eq!(
            back.detached_state(DetachableModule::Panel).window,
            Some(DetachedWindow { size: [520.0, 700.0], pos: None })
        );

        // An old config's two panadapter keys fold in on `migrate_detached`.
        let mut migrated = UiSettings::default();
        migrated.panadapter_detached_legacy = true;
        migrated.panadapter_window_legacy =
            Some(DetachedWindow { size: [960.0, 540.0], pos: None });
        assert!(!migrated.is_detached(DetachableModule::Panadapter), "not until migration runs");
        migrated.migrate_detached();
        assert!(migrated.is_detached(DetachableModule::Panadapter));
        assert_eq!(
            migrated.detached_state(DetachableModule::Panadapter).window.map(|w| w.size),
            Some([960.0, 540.0])
        );
        // Idempotent: a second call changes nothing.
        migrated.migrate_detached();
        assert!(migrated.is_detached(DetachableModule::Panadapter));
    }
}

#[cfg(test)]
mod advisory_tests {
    use super::UiSettings;

    /// The point of the whole exercise: dismissing one warning must not silence
    /// another, and must survive the settings being written and read back.
    #[test]
    fn dismissing_one_advisory_leaves_the_others_alone() {
        let mut s = UiSettings::default();
        assert!(!s.advisory_dismissed("dab-rate"), "shown until dismissed");
        s.dismiss_advisory("dab-rate");
        assert!(s.advisory_dismissed("dab-rate"), "dismissed for good");
        assert_eq!(s.dismissed_advisories.count_ones(), 1, "exactly one bit, not the lot");
    }

    /// An unclassified name is **always shown**. Silence by omission is the
    /// dangerous direction: a new call site must keep nagging until somebody
    /// decides it is advice, rather than muting itself by accident.
    #[test]
    fn an_unknown_advisory_can_never_be_dismissed() {
        let mut s = UiSettings::default();
        assert_eq!(UiSettings::advisory_bit("something-new"), None);
        s.dismiss_advisory("something-new");
        assert!(!s.advisory_dismissed("something-new"), "dismissal ignored");
        assert_eq!(s.dismissed_advisories, 0, "nothing recorded");
    }

    /// The list is the design decision, so it is pinned: it is short, it has no
    /// duplicates, and every name fits its bit.
    #[test]
    fn the_dismissible_list_is_short_and_well_formed() {
        let keys = UiSettings::ADVISORY_KEYS;
        assert!(keys.len() <= 64, "a u64 bitmask holds 64 advisories");
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(UiSettings::advisory_bit(key), Some(i as u32), "{key}");
        }
        for a in keys {
            assert_eq!(keys.iter().filter(|k| *k == a).count(), 1, "{a} listed twice");
        }
    }
}
