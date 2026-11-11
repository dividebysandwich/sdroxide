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
    /// The classic navy/cyan/pink look. Declared last because serde demands
    /// the catch-all be the final variant: it also swallows an unrecognised
    /// value in a hand-edited config, so a typo degrades to the default theme
    /// instead of throwing the whole config away.
    #[default]
    #[serde(other)]
    Default,
}

impl UiTheme {
    pub const ALL: [UiTheme; 17] = [
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
        }
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
    /// The same position in screen pixels. Points don't survive a second
    /// monitor at a different display scale: a new window starts with the
    /// main screen's scale, so the same points land somewhere else. Pixels
    /// are the same on every screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos_px: Option<[f32; 2]>,
}

/// A module pulled out into its **own OS window**: where it last was, so
/// reopening it — or the next start — puts it back rather than at a built-in
/// size.
///
/// The same shape and the same Wayland caveat as [`Solar3dWindow`]: a client is
/// given no absolute window position, so `pos` is `None` there and the
/// compositor places the window. The **app-id** the window carries
/// ([`DetachableModule::app_id`]) is what a compositor rule matches on to float
/// it and pin it to a monitor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DetachedWindow {
    /// Inner (drawing-area) size in points.
    pub size: [f32; 2],
    /// Outer (top-left) position in points; `None` on Wayland.
    #[serde(default)]
    pub pos: Option<[f32; 2]>,
}

/// A module that can be pulled out of the main window into its own OS window —
/// **undocked** — in the SDRuno mould: the spectrum on one monitor, the decoder
/// on another. An enum keyed in [`UiSettings::detached`] rather than a field
/// pair per module, so a new detachable module is a variant here plus its UI,
/// nothing else. **Append only**: the variant's [`Self::index`] is its slot in
/// the saved settings.
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
    /// A **second** spectrum and waterfall — SDRuno's AUX SP. A second view of
    /// the same receiver, with its own zoom and layers, not a second receiver.
    AuxPanadapter,
    /// The **band/mode selector** in a window of its own, where it can be as
    /// wide as the operator likes without squeezing the operating panel.
    BandMenu,
}

impl DetachableModule {
    /// How many detachable modules there are — the length of
    /// [`UiSettings::detached`], which is an array so `UiSettings` stays `Copy`.
    pub const COUNT: usize = 5;

    /// Every module, so a settings list or a test can walk them all.
    pub const ALL: [DetachableModule; DetachableModule::COUNT] = [
        DetachableModule::Panadapter,
        DetachableModule::Panel,
        DetachableModule::Controls,
        DetachableModule::AuxPanadapter,
        DetachableModule::BandMenu,
    ];

    /// This module's slot in [`UiSettings::detached`].
    pub fn index(self) -> usize {
        match self {
            DetachableModule::Panadapter => 0,
            DetachableModule::Panel => 1,
            DetachableModule::Controls => 2,
            DetachableModule::AuxPanadapter => 3,
            DetachableModule::BandMenu => 4,
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
        }
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

/// A named window arrangement — SDRuno's **workspace**: which modules are in
/// their own OS windows, and where those windows are. SDRuno keeps up to ten and
/// recalls them by name; so does this.
///
/// A **list of its own**, not a field in [`UiSettings`], because `UiSettings` is
/// `Copy` and the whole UI passes it around by value. The list lives in
/// `workspaces.json`, beside the memories and the logbook. A workspace is the
/// arrangement only — where the furniture is, not what is switched on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Workspace {
    /// What the operator called it. Not an index: a name survives a reorder.
    pub name: String,
    /// One slot per [`DetachableModule`] in `index()` order **at save time**. A
    /// `Vec` rather than the fixed array, so a workspace written by a build with
    /// a module this one has not heard of still loads and applies the modules it
    /// does know — the same tolerance [`UiSettings::detached`]'s reader has.
    pub slots: Vec<DetachedState>,
}

/// How many workspaces may be saved — SDRuno's ten, and the ten number keys
/// that recall them.
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
        let mut out = default_detached();
        for (slot, saved) in out.iter_mut().zip(self.slots.iter()) {
            *slot = *saved;
        }
        settings.detached = out;
    }
}

fn default_detached() -> [DetachedState; DetachableModule::COUNT] {
    [DetachedState::default(); DetachableModule::COUNT]
}

/// Read [`UiSettings::detached`] at any length. A config written by a build with
/// fewer (or more) modules carries a shorter (or longer) list, and a derived
/// array deserializer would reject the whole file over it — which would reset
/// every other setting along with the window geometry. A short list leaves the
/// modules it does not reach docked; a long one drops what this build does not
/// know.
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

/// The language the interface is drawn in: a BCP 47 tag such as `es` or
/// `pt-BR`, or none to follow the operating system's.
///
/// A tag rather than an enum of the languages there are, because the list is
/// whatever translations the client carries, and this crate knows none of
/// them. A tag the client has no translation for is not an error: it falls
/// back to the nearest it has, and to English last, the same as a system
/// language it has never heard of. Held inline so [`UiSettings`] stays `Copy`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct UiLanguage {
    tag: [u8; UiLanguage::MAX_LEN],
    len: u8,
}

impl UiLanguage {
    /// The longest tag held. Language, script and region come to eleven
    /// (`zh-Hant-TW`); the rest is room for a variant.
    pub const MAX_LEN: usize = 23;

    /// Follow the operating system's language — the default.
    pub const SYSTEM: UiLanguage = UiLanguage { tag: [0; Self::MAX_LEN], len: 0 };

    /// A language picked by tag. `None` for an empty tag, one too long to
    /// hold, or one with a character no language tag has.
    pub fn from_tag(tag: &str) -> Option<UiLanguage> {
        let tag = tag.trim();
        if tag.is_empty()
            || tag.len() > Self::MAX_LEN
            || !tag.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return None;
        }
        let mut out = Self::SYSTEM;
        out.tag[..tag.len()].copy_from_slice(tag.as_bytes());
        out.len = tag.len() as u8;
        Some(out)
    }

    /// The picked tag, or `None` for the system's language.
    pub fn tag(&self) -> Option<&str> {
        // Only ever filled from a `&str` of ASCII, so this cannot fail.
        (self.len > 0).then(|| std::str::from_utf8(&self.tag[..self.len as usize]).unwrap_or(""))
    }

    pub fn is_system(&self) -> bool {
        self.len == 0
    }
}

impl std::fmt::Debug for UiLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.tag() {
            Some(tag) => write!(f, "UiLanguage({tag})"),
            None => f.write_str("UiLanguage(system)"),
        }
    }
}

/// Written as the bare tag, and as an empty string for the system's language,
/// so `config.toml` reads `language = "es"`.
impl Serialize for UiLanguage {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.tag().unwrap_or(""))
    }
}

/// Anything that is not a tag it can hold — hand-edited nonsense, or a longer
/// tag from some later build — reads as the system's language rather than
/// failing the whole `[ui]` table, which would lose the theme and the layout
/// along with it.
impl<'de> Deserialize<'de> for UiLanguage {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let tag = String::deserialize(d)?;
        Ok(UiLanguage::from_tag(&tag).unwrap_or(UiLanguage::SYSTEM))
    }
}

/// User display preferences. All have defaults so a missing `[ui]` table loads.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// GUI repaint + spectrum frame rate, in frames per second.
    pub frame_rate_fps: u32,
    /// How long the pointer rests on a control before its tooltip shows, in
    /// seconds. egui's own half second unless the operator sets another.
    pub tooltip_delay_s: f32,
    /// Once a tooltip has shown, how long the next control's shows without
    /// waiting, in seconds (egui's grace time). 0: every tooltip waits its
    /// delay.
    pub tooltip_grace_s: f32,
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
    /// Off by default, and deliberately so: the step buttons move by exactly
    /// the step, and a press that lands somewhere else is a second, invisible
    /// edit — see `tune_step_row`. On, it is the touch-screen habit of tidying a
    /// dial left anywhere before working down a band (issue #422). Only
    /// meaningful with [`Self::tune_step_buttons`].
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
    /// The language the interface is drawn in — see [`UiLanguage`]. This
    /// screen's preference, like the theme: a remote client picks its own.
    pub language: UiLanguage,
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
    /// Download HAMQSL.com's HF band-conditions forecast (at most hourly) to
    /// colour the band menu. A switch because it is one of the requests the
    /// app makes on its own, and every one of those can be turned off. The 3D
    /// view's own feed is not covered: opening that window asks for its data.
    pub band_conditions_download: bool,
    /// Download the EiBi broadcast schedule from eibispace.de when a new
    /// broadcasting season starts. Off, the compiled-in schedule (or the last
    /// one cached) stays in use, and the REFRESH button still fetches on demand.
    pub broadcast_schedule_download: bool,
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
    /// Whose licence-free channel plans the waterfall marks — see
    /// [`Country`](crate::region::Country). This screen's preference: it
    /// follows the computer's country unless the operator picks one.
    pub country: crate::region::Country,
    /// The UTC + local wall clock at the panadapter's top-right (#655).
    pub panadapter_clock: bool,
    /// Where the solar-system 3D window last was — see [`Solar3dWindow`].
    ///
    /// Here rather than in the operator's view state because window geometry is
    /// a property of this screen, not of the radio, and a remote client keeps
    /// its own.
    #[serde(default)]
    pub solar3d_window: Option<Solar3dWindow>,
    /// The AMP window in a window of its own rather than inside the main one,
    /// as the solar-system window is. Desktop only; the browser keeps it in.
    #[serde(default)]
    pub amp_popped: bool,
    /// Whether the AMP window was open, so it comes back as it was closed.
    #[serde(default)]
    pub amp_open: bool,
    /// Where the popped-out AMP window was, as for the solar-system window.
    #[serde(default)]
    pub amp_window: Option<Solar3dWindow>,
    /// Where the docked AMP window was inside the main one.
    #[serde(default)]
    pub amp_docked: Option<AmpDocked>,
    /// The AMP window's meter picks: top and bottom of the left meter, then of
    /// the right one, each as its place in the window's FWD, RFL, VDD, IDD, DRV
    /// list, 255 for no bottom needle. `None` for the built-in picks.
    #[serde(default)]
    pub amp_meters: Option<[u8; 4]>,
    /// Which modules are undocked into their own windows, and where those
    /// windows last were — one [`DetachedState`] per [`DetachableModule`], in
    /// `module.index()` order. An array rather than a map so `UiSettings` stays
    /// `Copy`; read at any length, see [`detached_slots`].
    ///
    /// **Native only.** The browser has no second window and keeps every module
    /// in-window, so the flags are ignored there. Machine-local: where a window
    /// sits is a property of this screen.
    #[serde(default = "default_detached", deserialize_with = "detached_slots")]
    pub detached: [DetachedState; DetachableModule::COUNT],
}

/// The docked AMP window's place: its top-left corner and width, in points.
/// Its height follows the width.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AmpDocked {
    pub pos: [f32; 2],
    pub width: f32,
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

/// egui's own tooltip delay, the default for [`UiSettings::tooltip_delay_s`].
pub const TOOLTIP_DELAY_DEFAULT_S: f32 = 0.5;
/// The range the tooltip delay is set in.
pub const TOOLTIP_DELAY_RANGE_S: std::ops::RangeInclusive<f32> = 0.0..=1.5;
/// egui's own grace time, the default for [`UiSettings::tooltip_grace_s`].
pub const TOOLTIP_GRACE_DEFAULT_S: f32 = 0.2;
/// The range the tooltip grace time is set in.
pub const TOOLTIP_GRACE_RANGE_S: std::ops::RangeInclusive<f32> = 0.0..=1.0;

impl Default for UiSettings {
    fn default() -> Self {
        UiSettings {
            // Off: the dial is what every other radio shows, and an operator
            // who has not asked for the change should not find their readout
            // reading differently from the rig beside it.
            cw_qrg: false,
            frame_rate_fps: 60,
            tooltip_delay_s: TOOLTIP_DELAY_DEFAULT_S,
            tooltip_grace_s: TOOLTIP_GRACE_DEFAULT_S,
            waterfall_speed: Speed::Medium,
            spectrum_speed: Speed::Medium,
            spectrum_3d_speed: Speed::Medium,
            waterfall_palette: 0,
            waterfall_smooth: true,
            tune_step_buttons: true,
            // 1 kHz: the round step the operators who asked for this tune in,
            // on a band where the stations sit 3 kHz apart.
            tune_step_hz: 1_000.0,
            tune_step_round_first: false,
            spectrum_detail: SpectrumDetail::Auto,
            spectrum_gradient: true,
            gradient_top: [64, 0, 0],   // dark red
            gradient_bottom: [0, 0, 0], // black
            spot_colors: default_spot_colors(),
            bandplan_colors: default_bandplan_colors(),
            layout: LayoutMode::Auto,
            language: UiLanguage::SYSTEM,
            theme: UiTheme::Default,
            button_style: ChromeStyle::Angled,
            window_style: ChromeStyle::Angled,
            skimmer_font_size: FontSize::Medium,
            waterfall_font_size: FontSize::Small,
            menu_font_size: FontSize::Medium,
            ui_zoom: 1.0,
            smeter_style: SmeterStyle::Needle,
            update_check: true,
            band_conditions_download: true,
            broadcast_schedule_download: true,
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
            country: Default::default(),
            panadapter_clock: true,
            solar3d_window: None,
            amp_popped: false,
            amp_open: false,
            amp_window: None,
            amp_docked: None,
            amp_meters: None,
            detached: default_detached(),
        }
    }
}

impl UiSettings {
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

    /// The steps the tuning buttons cycle through, in hertz — from the 10 Hz
    /// that trims a carrier onto zero-beat up to the 25 kHz of an FM channel,
    /// by way of the AM broadcast spacings (9 kHz in Regions 1 and 3, 10 kHz in
    /// Region 2) and the 5 kHz most shortwave broadcasters sit on.
    pub const TUNE_STEPS_HZ: [f64; 9] =
        [10.0, 100.0, 500.0, 1_000.0, 2_500.0, 5_000.0, 9_000.0, 10_000.0, 25_000.0];

    /// The step after the current one, wrapping. A tap on the step button.
    pub fn next_tune_step(&self) -> f64 {
        let steps = Self::TUNE_STEPS_HZ;
        let at = steps.iter().position(|s| (s - self.tune_step_hz).abs() < 0.5);
        steps[at.map_or(0, |i| (i + 1) % steps.len())]
    }

    /// The current step written the way a radio's own display would: "100 Hz",
    /// "1 kHz", "12.5 kHz".
    pub fn tune_step_label(&self) -> String {
        let hz = self.tune_step_hz;
        if hz < 1_000.0 {
            return format!("{hz:.0} Hz");
        }
        let khz = hz / 1_000.0;
        if (khz - khz.round()).abs() < 1e-6 {
            format!("{khz:.0} kHz")
        } else {
            format!("{khz:.1} kHz")
        }
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
            (9_000.0, "9 kHz"),
            (25_000.0, "25 kHz"),
        ] {
            ui.tune_step_hz = hz;
            assert_eq!(ui.tune_step_label(), want);
        }
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;

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

    /// The language is written as the bare tag, the system's as an empty
    /// string, and anything that is not a tag reads back as the system's
    /// rather than costing the rest of `[ui]`.
    #[test]
    fn the_language_round_trips_as_a_bare_tag() {
        let es = UiLanguage::from_tag("es").unwrap();
        assert_eq!(serde_json::to_string(&es).unwrap(), "\"es\"");
        assert_eq!(serde_json::to_string(&UiLanguage::SYSTEM).unwrap(), "\"\"");
        let back: UiLanguage = serde_json::from_str("\"pt-BR\"").unwrap();
        assert_eq!(back.tag(), Some("pt-BR"));
        let none: UiLanguage = serde_json::from_str("\"\"").unwrap();
        assert!(none.is_system());
        let junk: UiLanguage = serde_json::from_str("\"<script>\"").unwrap();
        assert!(junk.is_system());
        let long: UiLanguage = serde_json::from_str(&format!("\"{}\"", "a".repeat(40))).unwrap();
        assert!(long.is_system());
        // A config written before the field existed loads with the system's.
        let ui: UiSettings = serde_json::from_str("{}").unwrap();
        assert!(ui.language.is_system());
    }
}

#[cfg(test)]
mod workspace_tests {
    use super::*;

    /// A workspace puts back exactly the arrangement it captured, and one saved
    /// by a build with fewer modules leaves the rest docked.
    #[test]
    fn a_workspace_round_trips_and_tolerates_another_builds_length() {
        let mut ui = UiSettings::default();
        ui.set_detached(DetachableModule::Panadapter, true);
        ui.detached[DetachableModule::Panel.index()].window =
            Some(DetachedWindow { size: [500.0, 400.0], pos: Some([10.0, 20.0]) });
        let ws = Workspace::capture("Contest", &ui);
        let mut other = UiSettings::default();
        other.set_detached(DetachableModule::Controls, true);
        ws.apply(&mut other);
        assert_eq!(other.detached, ui.detached);

        let short = Workspace {
            name: "Old".into(),
            slots: vec![DetachedState { detached: true, window: None }],
        };
        let mut target = UiSettings::default();
        target.set_detached(DetachableModule::BandMenu, true);
        short.apply(&mut target);
        assert!(target.is_detached(DetachableModule::Panadapter));
        assert!(
            !target.is_detached(DetachableModule::BandMenu),
            "a slot the workspace never had stays docked"
        );
        let json = serde_json::to_string(&ws).unwrap();
        assert_eq!(serde_json::from_str::<Workspace>(&json).unwrap(), ws);
    }
}
