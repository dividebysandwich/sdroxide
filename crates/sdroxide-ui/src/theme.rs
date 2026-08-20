//! The sdroxide look: dark panels, cut corners, Chakra Petch type (SIL OFL —
//! see assets/fonts/OFL.txt) — in one of a handful of colour themes, the
//! classic navy/cyan/hot-pink one by default.
//!
//! The palette used to be a set of `pub const Color32`s; when the theme became
//! switchable at runtime they turned into same-named accessor *functions*
//! reading the current [`Palette`] — which is why the call sites still say
//! `theme::CYAN()`. The current theme and chrome-style selections live in
//! process-wide atomics rather than the egui context because the native
//! solar-3d viewport renders deferred, possibly on another thread, and must
//! see the same look.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use sdroxide_types::{ChromeStyle, FontSize, UiTheme};

/// Every colour role the chrome wears. A theme is one full assignment of
/// these; the field names keep the historic constant names (`cyan` is "the
/// primary accent", whatever hue a theme gives it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// True where the chrome sits on a bright ground. Everything that has to
    /// pick a shade by hand — [`gray`], [`continent_color`], the few widgets
    /// that mix their own tints — asks this rather than sniffing a luminance.
    pub light: bool,
    /// The theme has no dim register: nothing is faint on purpose, and
    /// [`gray`] lifts the hand-written grey scale up to meet the ink instead
    /// of reproducing it.
    pub no_dim: bool,
    pub bg_deep: Color32,
    pub panel: Color32,
    pub input_bg: Color32,
    pub fill: Color32,
    pub fill_hover: Color32,
    pub fill_active: Color32,
    pub line: Color32,
    pub line_lit: Color32,
    pub text: Color32,
    pub text_strong: Color32,
    /// Primary accent: selection fills, headings, links.
    pub cyan: Color32,
    /// Secondary accent: captions, dimmed highlights.
    pub cyan_dim: Color32,
    /// Chrome accent: window borders, decorative strokes.
    pub pink: Color32,
    pub yellow: Color32,
    pub green: Color32,
    /// Dark ink used on top of accent fills.
    pub ink_on_cyan: Color32,
    /// Ink for the chips whose fill is one of the *bright* accents — the
    /// yellow tone-squelch chip, the green satellite/scan chips. Black
    /// everywhere the accents are bright; on a light ground those accents are
    /// dark enough to read as text on white, so their chips take white ink
    /// instead.
    pub ink_on_bright: Color32,
    /// Red-accent chrome (cyberpunk box borders / list rows).
    pub red_deep: Color32,
    pub cq_bg: Color32,
    /// Background for a decode addressed to our own station (warm gold,
    /// stands out).
    pub tome_bg: Color32,
    /// Background for the QSO-complete line in a transcript whose received
    /// messages are already green — the band is what makes it read as an
    /// event, not a message.
    pub done_bg: Color32,
    pub row_bg: Color32,
    pub row_hover: Color32,
    /// Scrollbars: an accent handle riding in a recessed gutter. The gutter is
    /// a touch lighter than every panel/list background it sits on, so the
    /// full scroll range stays readable even when the handle is at the far
    /// end.
    pub scroll_track: Color32,
    pub scroll_handle: Color32,
    pub scroll_handle_hover: Color32,
    pub scroll_handle_drag: Color32,
    /// egui's `faint_bg_color` (striped table rows and the like).
    pub faint_bg: Color32,
    /// TX / SWR / error indications. Red in *every* theme — the phosphor
    /// themes are monochrome everywhere else, but whether RF is leaving the
    /// antenna is never left to a shade of green.
    pub alert: Color32,
    /// The bright stripe of the yellow/black hazard bars ([manual section
    /// rules, the CME arrival banner](crate::chrome::hazard_stripes)). Its own
    /// role rather than [`Self::yellow`] because a hazard bar is a *fill* next
    /// to a dark stripe, not ink on the panel: it stays bright amber even
    /// where the warning text has had to go dark to survive a white ground.
    pub hazard: Color32,
    /// The dark stripe those bars alternate with.
    pub hazard_dark: Color32,
}

/// `c(0x00d0f4)` — a palette entry from one hex triple, so a theme reads as a
/// column of colour values.
const fn c(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// The classic look: dark navy panels, cyan accents, hot-pink strokes. Must
/// stay bit-exact to the historic constants — it is the default every
/// screenshot in the manual shows.
const DEFAULT: Palette = Palette {
    light: false,
    no_dim: false,
    bg_deep: c(0x050810),
    panel: c(0x0b111e),
    input_bg: c(0x04070e),
    fill: c(0x101a2c),
    fill_hover: c(0x17243c),
    fill_active: c(0x1d2f4d),
    line: c(0x1a2740),
    line_lit: c(0x2a4a66),
    text: c(0xb4c6da),
    text_strong: c(0xe8f4ff),
    cyan: c(0x00d0f4),
    cyan_dim: c(0x1d9cbe),
    pink: c(0xff2a55),
    yellow: c(0xffd23f),
    green: c(0x46e07d),
    ink_on_cyan: c(0x021019),
    ink_on_bright: Color32::BLACK,
    red_deep: c(0x6e182c),
    cq_bg: c(0x240c15),
    tome_bg: c(0x2c2406),
    done_bg: c(0x082a17),
    row_bg: c(0x0a101b),
    row_hover: c(0x141e2e),
    scroll_track: c(0x0e1626),
    scroll_handle: c(0x178ead),
    scroll_handle_hover: c(0x00d0f4), // = cyan
    scroll_handle_drag: c(0xff2a55),  // = pink
    faint_bg: c(0x0e1626),
    alert: c(0xff2a55), // = the historic pink, which doubled as the error colour
    hazard: c(0xffd23f),
    hazard_dark: c(0x161204),
};

/// Green CRT phosphor: every role brightness-graded within one green family
/// on a near-black ground. `yellow` keeps its warning hue and `tome_bg` its
/// warm attention gold on purpose — they are the second alert tier.
const GREEN_PHOSPHOR: Palette = Palette {
    light: false,
    no_dim: false,
    bg_deep: c(0x020703),
    panel: c(0x04120a),
    input_bg: c(0x010704),
    fill: c(0x062010),
    fill_hover: c(0x0a2c16),
    fill_active: c(0x0e3a1d),
    line: c(0x0d3018),
    line_lit: c(0x1a5c30),
    text: c(0x55c882),
    text_strong: c(0xccffdd),
    cyan: c(0x00ff66),
    cyan_dim: c(0x00b348),
    pink: c(0x00d957),
    yellow: c(0xffd23f),
    green: c(0x46e07d),
    ink_on_cyan: c(0x021203),
    ink_on_bright: Color32::BLACK,
    red_deep: c(0x124a26),
    cq_bg: c(0x0c2412),
    tome_bg: c(0x2c2406),
    done_bg: c(0x082a17),
    row_bg: c(0x071408),
    row_hover: c(0x0e2415),
    scroll_track: c(0x08180d),
    scroll_handle: c(0x17ad5c),
    scroll_handle_hover: c(0x00ff66),
    scroll_handle_drag: c(0xb3ffcc),
    faint_bg: c(0x08180d),
    alert: c(0xff2a3c),
    hazard: c(0xffd23f),
    hazard_dark: c(0x161204),
};

/// Amber CRT phosphor — the same grading as [`GREEN_PHOSPHOR`] in a warm
/// amber family.
const AMBER_PHOSPHOR: Palette = Palette {
    light: false,
    no_dim: false,
    bg_deep: c(0x070402),
    panel: c(0x120c04),
    input_bg: c(0x070401),
    fill: c(0x201607),
    fill_hover: c(0x2c1f0a),
    fill_active: c(0x3a290e),
    line: c(0x30240d),
    line_lit: c(0x5c451a),
    text: c(0xc89e55),
    text_strong: c(0xffeacc),
    cyan: c(0xffb000),
    cyan_dim: c(0xb37b00),
    pink: c(0xd99500),
    yellow: c(0xffd23f),
    green: c(0xffc966),
    ink_on_cyan: c(0x120902),
    ink_on_bright: Color32::BLACK,
    red_deep: c(0x4a3512),
    cq_bg: c(0x241a0c),
    tome_bg: c(0x2c2406),
    done_bg: c(0x2a2008),
    row_bg: c(0x140e05),
    row_hover: c(0x241a0e),
    scroll_track: c(0x181008),
    scroll_handle: c(0xad7c17),
    scroll_handle_hover: c(0xffb000),
    scroll_handle_drag: c(0xffdb99),
    faint_bg: c(0x181008),
    alert: c(0xff2a3c),
    hazard: c(0xffd23f),
    hazard_dark: c(0x161204),
};

/// The default's structure with teal accents and orange chrome.
const TEAL_ORANGE: Palette = Palette {
    light: false,
    no_dim: false,
    bg_deep: c(0x040e0d),
    panel: c(0x0b1e1c),
    input_bg: c(0x040b0a),
    fill: c(0x102c29),
    fill_hover: c(0x17403b),
    fill_active: c(0x1d524b),
    line: c(0x1a403c),
    line_lit: c(0x2a6660),
    text: c(0xb4dad2),
    text_strong: c(0xe8fffa),
    cyan: c(0x14e0c4),
    cyan_dim: c(0x1d9c8e),
    pink: c(0xff8a3f),
    yellow: c(0xffd23f),
    green: c(0x46e07d),
    ink_on_cyan: c(0x021917),
    ink_on_bright: Color32::BLACK,
    red_deep: c(0x6e3a18),
    cq_bg: c(0x241505),
    tome_bg: c(0x2c2406),
    done_bg: c(0x082a24),
    row_bg: c(0x0a1b18),
    row_hover: c(0x142e2a),
    scroll_track: c(0x0e2622),
    scroll_handle: c(0x17ad9c),
    scroll_handle_hover: c(0x14e0c4),
    scroll_handle_drag: c(0xff8a3f),
    faint_bg: c(0x0e2622),
    alert: c(0xff2a55),
    hazard: c(0xffd23f),
    hazard_dark: c(0x161204),
};

/// The default's dark navy grounds with the hues spread across the roles —
/// magenta borders, blue selections, violet captions, orange warnings, green
/// status. Different roles light different features, which is what makes the
/// whole UI come out multi-coloured.
const RAINBOW: Palette = Palette {
    light: false,
    no_dim: false,
    bg_deep: c(0x050810),
    panel: c(0x0b111e),
    input_bg: c(0x04070e),
    fill: c(0x101a2c),
    fill_hover: c(0x17243c),
    fill_active: c(0x1d2f4d),
    line: c(0x1a2740),
    line_lit: c(0x2a4a66),
    text: c(0xb4c6da),
    text_strong: c(0xe8f4ff),
    cyan: c(0x3fa9ff),
    cyan_dim: c(0x8a5cff),
    pink: c(0xff2ad5),
    yellow: c(0xffa53f),
    green: c(0x46e07d),
    ink_on_cyan: c(0x040d19),
    ink_on_bright: Color32::BLACK,
    red_deep: c(0x6e182c),
    cq_bg: c(0x240c15),
    tome_bg: c(0x2c2406),
    done_bg: c(0x082a17),
    row_bg: c(0x0a101b),
    row_hover: c(0x141e2e),
    scroll_track: c(0x0e1626),
    scroll_handle: c(0x8a5cff),
    scroll_handle_hover: c(0x3fa9ff),
    scroll_handle_drag: c(0xff2ad5),
    faint_bg: c(0x0e1626),
    alert: c(0xff2a55),
    hazard: c(0xffa53f),
    hazard_dark: c(0x161204),
};

/// The one theme on a bright ground: white panels, near-black ink, and accents
/// darkened until each of them clears 4.5:1 against white *as text* — the
/// figure below every colour is its contrast ratio on the panel.
///
/// That is what forces the accents to be so dark. Every accent role doubles as
/// ink somewhere (`green` is a "worked before" tick and a signal-report
/// column, `yellow` is a warning line, `cyan` is every heading and link), so a
/// bright green or a bright yellow — legible on navy, ~1.5:1 on white — would
/// be unreadable in exactly the places an operator glances at fastest. Each
/// one keeps its hue and gives up its brightness instead. The chips those
/// accents fill take white ink ([`Palette::ink_on_bright`]) rather than the
/// black the dark themes use.
///
/// The instruments are not in here: see [`SCOPE_LIGHT`] and [`METER_LIGHT`].
/// The *order* of the grounds is what carries over from the dark themes, not
/// their direction: `bg_deep` is the page a panel sits on, `fill` is a raised
/// control face, and each step away from `panel` is a step further from it.
/// Reading them upside down would put a lit tab behind the strip and a button
/// face into the page, so every one of them is mirrored together — panels are
/// the white here, and the page is the grey they sit on.
const LIGHT: Palette = Palette {
    light: true,
    no_dim: false,
    bg_deep: c(0xeaeef4),  // the page the panels sit on
    panel: c(0xffffff),    // the panels themselves: plain white
    input_bg: c(0xf2f5fa), // fields, recessed a step off the panel
    fill: c(0xe4eaf3),     // button faces, raised → darker on a light ground
    fill_hover: c(0xd3dcea),
    fill_active: c(0xbecddf),
    line: c(0xaab6c6),        // 2.2:1 — a border, never text
    line_lit: c(0x64748b),    // 4.9:1
    text: c(0x1b2431),        // 14.6:1
    text_strong: c(0x000913), // 19.5:1
    cyan: c(0x005a78),        //  7.7:1 — deep teal, still the primary accent
    cyan_dim: c(0x17708d),    //  5.6:1
    pink: c(0xb8123c),        //  6.6:1 — the chrome accent, a deep crimson
    yellow: c(0x8a5a00),      //  5.9:1 — amber taken down to where it reads
    green: c(0x0d7030),       //  6.2:1 — likewise green
    ink_on_cyan: c(0xffffff),
    ink_on_bright: c(0xffffff),
    red_deep: c(0xa8324e),
    cq_bg: c(0xffe1e9), // the row tints: pale washes, so the ink stays near-black
    tome_bg: c(0xffedbe),
    done_bg: c(0xd5f2df),
    row_bg: c(0xf8fafc),
    row_hover: c(0xe7eef7),
    // Inverted with everything else: in the dark themes the gutter is a touch
    // *lighter* than the list behind it, here it has to be a touch darker.
    scroll_track: c(0xe4eaf3),
    scroll_handle: c(0x5d7186),
    scroll_handle_hover: c(0x005a78), // = cyan
    scroll_handle_drag: c(0xb8123c),  // = pink
    faint_bg: c(0xeff3f8),
    alert: c(0xc40024), // 6.2:1, and still unmistakably red
    // A hazard bar is a fill, not ink: it keeps the bright amber every other
    // theme uses, against the same near-black stripe.
    hazard: c(0xffd23f),
    hazard_dark: c(0x161204),
};

/// White on black, and nothing between them that does not have to be there.
///
/// The other themes spend most of their range on shades that mean "this is
/// here but is not what you are looking at" — a border a step off the panel, a
/// caption a step off the body text. This one has no such register. Every ink
/// role is at least 7:1 against the panel (the figure below each is its actual
/// ratio), every border is a shade you can see from across the room, and
/// [`gray`] lifts the whole hand-written grey scale up to meet them rather
/// than mirroring it — see there for what that costs.
///
/// The accents are the ones a high-contrast display convention already uses,
/// so they arrive meaning what an operator expects: cyan selects, yellow
/// warns, green confirms, red stops.
const HIGH_CONTRAST: Palette = Palette {
    light: false,
    no_dim: true,
    // Pure black page. `panel` is a hair off it only because the page has to
    // stay behind the panels in every theme; the *border* is what separates
    // them here, not the fill.
    bg_deep: c(0x000000),
    panel: c(0x0a0a0a),
    input_bg: c(0x000000),
    // Button faces stay near-black on purpose, and their *border* is what
    // makes them buttons — the same bargain every high-contrast convention
    // strikes. A face light enough to read against the panel on its own would
    // have to take the white label with it, and 19.8:1 of label is worth more
    // than a visible fill.
    fill: c(0x1a1a1a),
    fill_hover: c(0x333333),
    fill_active: c(0x4d4d4d),
    line: c(0x8c8c8c),     //  5.9:1 — a border nobody has to hunt for
    line_lit: c(0xffffff), // 19.8:1
    text: c(0xffffff),     // 19.8:1
    text_strong: c(0xffffff),
    cyan: c(0x1aebff),     // 13.6:1 — selections, headings, links
    cyan_dim: c(0x9fe8ff), // 14.6:1 — a *second hue*, not a dimmer one
    pink: c(0xff66ff),     //  8.1:1 — window borders and chrome strokes
    yellow: c(0xffff00),   // 18.4:1
    green: c(0x3ff23f),    // 13.2:1
    ink_on_cyan: c(0x000000),
    ink_on_bright: c(0x000000),
    red_deep: c(0xff5555),
    // Row tints dark enough that white ink still clears 15:1 on them, and
    // saturated enough to read as a band rather than as a smudge.
    cq_bg: c(0x4d001f),
    tome_bg: c(0x4d3a00),
    done_bg: c(0x004d24),
    row_bg: c(0x000000),
    row_hover: c(0x333333),
    scroll_track: c(0x1a1a1a),
    scroll_handle: c(0xffffff),
    scroll_handle_hover: c(0x1aebff),
    scroll_handle_drag: c(0xffff00),
    faint_bg: c(0x1a1a1a),
    // 6.3:1, and as bright as it can be while still being unmistakably red:
    // the green and blue a brighter one would need are exactly what would turn
    // it salmon. The one role where the hue outranks the ratio.
    alert: c(0xff5555),
    hazard: c(0xffff00),
    hazard_dark: c(0x000000),
};

/// Indexed by [`theme_index`].
static PALETTES: [Palette; 7] =
    [DEFAULT, GREEN_PHOSPHOR, AMBER_PHOSPHOR, TEAL_ORANGE, RAINBOW, LIGHT, HIGH_CONTRAST];

/// The S-meter instrument's colours: the face wash, the backlight bloom, the
/// cool-side (below the red-line) inks, the bar's recessed rail, and the cool
/// half of the S/ALC ramps. Kept apart from [`Palette`] because these are one
/// widget's instrument face, not roles the rest of the UI shares.
///
/// Only the cool side lives here on purpose: everything past S9 / past 3:1 —
/// the needle's red-line, the TX backlight, the hot tick/label inks, the
/// amber-to-red ramp tops — stays red in every theme, the same rule that keeps
/// [`ALERT`] red.
/// What the S-meter's face is made of. The instrument's *behaviour* turns on
/// this, not just its shades: which way a lit edge shades, whether the needle
/// blooms, and which of the three reds the over-the-red-line half is printed
/// in. See `widgets::smeter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterFace {
    /// Lit glass over a near-black dial — the historic instrument.
    Glass,
    /// A printed white card, as a moving-coil meter has always been made.
    Paper,
    /// Glass again, but with every shade pushed to the ends of the range:
    /// white graduations, a white pointer, nothing decoratively faint.
    Contrast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterPalette {
    pub face: MeterFace,
    /// The needle: the blade's root, its length, and the last tenth at the
    /// point, plus the halo it throws. Three stops rather than two because
    /// the flare at the tip is what makes a lit blade read as lit.
    pub needle_root: Color32,
    pub needle_mid: Color32,
    pub needle_tip: Color32,
    pub needle_glow: Color32,
    pub face_top: Color32,
    pub face_bot: Color32,
    /// Specular hairline along the top of the glass.
    pub glass: Color32,
    /// Backlight bloom behind the scale on receive (transmit's stays red).
    pub backlight: Color32,
    /// Readout text: the headline value and the sub-reading beside it.
    pub readout: Color32,
    pub subdued: Color32,
    pub tick_minor: Color32,
    pub tick_major: Color32,
    pub label: Color32,
    pub grid_line: Color32,
    pub grid_label: Color32,
    /// The bar face's recessed trough (top/bottom of its wash) and its edge.
    pub rail_top: Color32,
    pub rail_bot: Color32,
    pub rail_edge: Color32,
    /// The cool half of the S and ALC ramps: floor → mid → ice at S9.
    pub ramp_lo: Color32,
    pub ramp_mid: Color32,
    pub ramp_hi: Color32,
}

/// The historic instrument: navy glass, cyan backlight, ice at S9.
const METER_DEFAULT: MeterPalette = MeterPalette {
    face: MeterFace::Glass,
    needle_root: c(0x9e0c22),
    needle_mid: c(0xff2c38),
    needle_tip: c(0xffe2e6),
    needle_glow: c(0xff3c46),
    face_top: c(0x111b2b),
    face_bot: c(0x03060c),
    glass: c(0x2c4460),
    backlight: c(0x1c7496),
    readout: c(0xe6f6ff),
    subdued: c(0x849eb6),
    tick_minor: c(0x54708a),
    tick_major: c(0xcfe6f5),
    label: c(0xbdd6e8),
    grid_line: c(0x1b2a3e),
    grid_label: c(0x637c92),
    rail_top: c(0x04070d),
    rail_bot: c(0x131f2f),
    rail_edge: c(0x1e2e42),
    ramp_lo: c(0x16627e),
    ramp_mid: c(0x1da8cf),
    ramp_hi: c(0x8eecff),
};

const METER_GREEN: MeterPalette = MeterPalette {
    face: MeterFace::Glass,
    needle_root: c(0x9e0c22),
    needle_mid: c(0xff2c38),
    needle_tip: c(0xffe2e6),
    needle_glow: c(0xff3c46),
    face_top: c(0x0c2112),
    face_bot: c(0x020803),
    glass: c(0x2c6040),
    backlight: c(0x1c9654),
    readout: c(0xd9ffe6),
    subdued: c(0x74b68b),
    tick_minor: c(0x4f8a63),
    tick_major: c(0xccf5d8),
    label: c(0xb0e8c4),
    grid_line: c(0x14301d),
    grid_label: c(0x5d9270),
    rail_top: c(0x040d07),
    rail_bot: c(0x132f1d),
    rail_edge: c(0x1e422c),
    ramp_lo: c(0x167e46),
    ramp_mid: c(0x1dcf74),
    ramp_hi: c(0x8effc0),
};

const METER_AMBER: MeterPalette = MeterPalette {
    face: MeterFace::Glass,
    needle_root: c(0x9e0c22),
    needle_mid: c(0xff2c38),
    needle_tip: c(0xffe2e6),
    needle_glow: c(0xff3c46),
    face_top: c(0x211809),
    face_bot: c(0x080502),
    glass: c(0x60482c),
    backlight: c(0x96661c),
    readout: c(0xffefd6),
    subdued: c(0xb69a74),
    tick_minor: c(0x8a734f),
    tick_major: c(0xf5e3c6),
    label: c(0xe8d4b0),
    grid_line: c(0x30240f),
    grid_label: c(0x92795d),
    rail_top: c(0x0d0904),
    rail_bot: c(0x2f2313),
    rail_edge: c(0x42331e),
    ramp_lo: c(0x7e5216),
    ramp_mid: c(0xcf8e1d),
    ramp_hi: c(0xffe28e),
};

const METER_TEAL: MeterPalette = MeterPalette {
    face: MeterFace::Glass,
    needle_root: c(0x9e0c22),
    needle_mid: c(0xff2c38),
    needle_tip: c(0xffe2e6),
    needle_glow: c(0xff3c46),
    face_top: c(0x0c2320),
    face_bot: c(0x030b0a),
    glass: c(0x2c605a),
    backlight: c(0x1c9688),
    readout: c(0xdcfff8),
    subdued: c(0x84b6ac),
    tick_minor: c(0x548a82),
    tick_major: c(0xccf5ec),
    label: c(0xb0e8de),
    grid_line: c(0x143733),
    grid_label: c(0x5d928a),
    rail_top: c(0x040d0c),
    rail_bot: c(0x132f2b),
    rail_edge: c(0x1e423d),
    ramp_lo: c(0x167e72),
    ramp_mid: c(0x1dcfba),
    ramp_hi: c(0x8effef),
};

/// The Light theme's instrument: not lit glass at all, but a *printed dial* —
/// a white card with black graduations and a black needle, the way a
/// moving-coil meter has always been made. It is the one thing on the light
/// theme that is not simply the dark look inverted: an illuminated instrument
/// on a white desk would read as a hole cut in the page, and a paper meter is
/// what the ground it now sits on actually calls for.
///
/// Everything downstream keys off [`MeterPalette::paper`]; the values here are
/// only the shades.
const METER_LIGHT: MeterPalette = MeterPalette {
    face: MeterFace::Paper,
    // Painted metal, darkening rather than flaring towards the point, and
    // throwing no light of its own — the glow is what the blade would emit.
    needle_root: c(0x4a5159),
    needle_mid: c(0x11151a),
    needle_tip: c(0x000000),
    needle_glow: Color32::TRANSPARENT,
    face_top: c(0xffffff),
    face_bot: c(0xeef0f3), // card stock, very slightly shaded toward the bottom
    glass: c(0xc3c9d1),    // the bezel hairline — no specular left to catch
    // Not a backlight any more: a wash this pale is what the RX bloom becomes
    // once it is 30% alpha over white, i.e. very nearly nothing.
    backlight: c(0xdde5ee),
    readout: c(0x10151b),    // 16.6:1 on the card
    subdued: c(0x59636e),    //  5.9:1
    tick_minor: c(0x3d444c), //  9.7:1
    tick_major: c(0x0a0d11),
    label: c(0x0a0d11),
    grid_line: c(0xc0c7cf),
    grid_label: c(0x59636e),
    // The bar's trough, shaded the same way round as every other theme's — a
    // shadow under the top lip — just lighter throughout.
    rail_top: c(0xd6dbe2),
    rail_bot: c(0xf1f3f6),
    rail_edge: c(0xa9b2bd),
    // Ink, not light: the cool half of the ramps has to be readable *as a
    // printed band* on white, so it runs deep teal → cyan rather than climbing
    // to ice.
    ramp_lo: c(0x16576e),
    ramp_mid: c(0x0d7c9c),
    ramp_hi: c(0x00a0c6),
};

/// The High-contrast instrument: the same glass, with every shade driven to
/// the ends of the range. White graduations, a white pointer, and ramps that
/// stay bright the whole way up their cool halves.
const METER_HIGH_CONTRAST: MeterPalette = MeterPalette {
    face: MeterFace::Contrast,
    // A white blade. It needs no flare at the tip to read as lit — there is
    // nothing brighter to flare to — and no bloom, which would only fog the
    // graduations it crosses.
    needle_root: c(0xd9d9d9),
    needle_mid: c(0xffffff),
    needle_tip: c(0xffffff),
    needle_glow: Color32::TRANSPARENT,
    face_top: c(0x0d0d0d),
    face_bot: c(0x000000),
    glass: c(0x999999),
    backlight: c(0x1aebff),
    readout: c(0xffffff),
    subdued: c(0xcccccc), // 16.1:1 — the "secondary" reading, barely secondary
    tick_minor: c(0xb3b3b3),
    tick_major: c(0xffffff),
    label: c(0xffffff),
    grid_line: c(0x808080), //  5.0:1 — a rule you can follow, not a hint
    grid_label: c(0xcccccc),
    rail_top: c(0x000000),
    rail_bot: c(0x262626),
    rail_edge: c(0xb3b3b3),
    ramp_lo: c(0x3399ff), //  7.1:1
    ramp_mid: c(0x00ddff),
    ramp_hi: c(0xccffff),
};

/// Indexed by [`theme_index`], like [`PALETTES`]. Rainbow keeps the historic
/// navy instrument: its grounds are the default's, and the meter already
/// reads in the accents the ramps give it.
static METER_PALETTES: [MeterPalette; 7] = [
    METER_DEFAULT,
    METER_GREEN,
    METER_AMBER,
    METER_TEAL,
    METER_DEFAULT,
    METER_LIGHT,
    METER_HIGH_CONTRAST,
];

/// The current theme's S-meter instrument colours.
#[inline]
pub fn meter_palette() -> &'static MeterPalette {
    &METER_PALETTES[THEME.load(Ordering::Relaxed) as usize]
}

/// What the *instruments* are drawn with: the panadapter and its waterfall,
/// the wide spectrum, the world map, the solar globe's cards.
///
/// These stay on a dark ground in every theme, the Light one included. A
/// waterfall has no bright-ground form — every colour map runs from black up
/// through its hot end, and a signal display inverted per theme would be read
/// wrongly by anyone who had ever used it the other way round. So the
/// instruments keep their glass and this palette carries the inks that go on
/// it, exactly as [`MeterPalette`] already did for the S-meter.
///
/// For the five dark themes each field is that theme's own role, unchanged —
/// see [`scope_from`]. Only [`SCOPE_LIGHT`] differs, and only because the
/// main palette's accents went dark to survive a white panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScopePalette {
    /// Canvas behind an instrument that has no image of its own to fill it —
    /// the world map's ocean.
    pub ground: Color32,
    /// The ground *outside* an instrument's cut-corner border, used to mask
    /// its square fill corners.
    pub shell: Color32,
    /// Labels and readouts painted straight onto the glass.
    pub ink: Color32,
    /// The same, for the one under the pointer.
    pub ink_strong: Color32,
    /// Rules and dividers drawn on it — the band-plan strip's inner edge.
    pub line: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
    /// Worked/heard/good markers.
    pub good: Color32,
    /// Measurement and warning marks.
    pub warn: Color32,
    /// The instrument's own frame and corner brackets.
    pub chrome: Color32,
}

/// A theme whose chrome is already dark lends the instruments its own roles.
const fn scope_from(p: &Palette) -> ScopePalette {
    ScopePalette {
        ground: p.input_bg,
        shell: p.bg_deep,
        ink: p.text,
        ink_strong: p.text_strong,
        line: p.line_lit,
        accent: p.cyan,
        accent_dim: p.cyan_dim,
        good: p.green,
        warn: p.yellow,
        chrome: p.pink,
    }
}

/// The Light theme's instrument inks. Structurally the default's — bright
/// accents on dark glass — with the grounds pulled to a neutral graphite so
/// the panadapter sits on the white page as a component rather than a void.
const SCOPE_LIGHT: ScopePalette = ScopePalette {
    ground: c(0x0d1117),
    // Not the palette's white `bg_deep`: this masks the border's corners, and
    // white wedges cut out of the instrument's frame would read as damage.
    shell: c(0x1b2431),
    ink: c(0xc3cedb),
    ink_strong: c(0xeaf2fa),
    line: c(0x39485c),
    accent: c(0x22d3ee),
    accent_dim: c(0x2a9db8),
    good: c(0x46e07d),
    warn: c(0xffd23f),
    chrome: c(0xff3d63),
};

/// Indexed by [`theme_index`], like [`PALETTES`]. High contrast lends the
/// instruments its own roles like any other dark theme — its accents were
/// already picked to sit on black, which is what the glass is.
static SCOPE_PALETTES: [ScopePalette; 7] = [
    scope_from(&DEFAULT),
    scope_from(&GREEN_PHOSPHOR),
    scope_from(&AMBER_PHOSPHOR),
    scope_from(&TEAL_ORANGE),
    scope_from(&RAINBOW),
    SCOPE_LIGHT,
    scope_from(&HIGH_CONTRAST),
];

/// The current theme's instrument inks — see [`ScopePalette`].
#[inline]
pub fn scope() -> &'static ScopePalette {
    &SCOPE_PALETTES[THEME.load(Ordering::Relaxed) as usize]
}

/// The flat world map's inks: the sea it is drawn on, the dot matrix of land,
/// and every marker that can appear on it.
///
/// Its own palette rather than the [`ScopePalette`] because the map is the one
/// instrument that *does* follow the ground. A waterfall has no bright form —
/// its colour maps start at black — but a map plainly does: pale sea, land
/// picked out on it, dark markers. So on the Light theme the panadapter stays
/// glass and the map turns into an atlas, and the two need separate sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPalette {
    pub sea: Color32,
    /// The dot matrix the continents are stippled in.
    pub land: Color32,
    /// A decoded station with a known grid: a neutral dot, faded by age. Also
    /// the head of the transmit comet — the one mark that has to out-read
    /// every coloured one around it.
    pub station: Color32,
    /// The great-circle path to the current DX, and the comet that runs along
    /// it while transmitting.
    pub trail: Color32,
    pub comet: Color32,
    /// Home, the worked DX, the row under the pointer, and a decode clicked
    /// but not yet answered. Each is drawn as a core in this colour with a
    /// low-alpha halo of it behind.
    pub home: Color32,
    pub dx: Color32,
    pub hover: Color32,
    pub preview: Color32,
    /// The "double-click to reframe" hint — carries its own alpha.
    pub hint: Color32,
    /// The map's frame, and the colour behind it that masks the cut corners.
    pub frame: Color32,
    pub shell: Color32,
}

/// The dark themes' map: a near-black sea with slate-teal continents, white
/// station dots, and the theme's own accents on the markers.
const fn map_from(p: &Palette) -> MapPalette {
    MapPalette {
        sea: p.input_bg,
        land: c(0x1c4458),
        station: Color32::WHITE,
        trail: c(0x00d0f4),
        comet: c(0x78f0ff),
        home: p.green,
        dx: p.pink,
        hover: p.yellow,
        preview: c(0xffd23f),
        hint: Color32::from_rgba_premultiplied(90, 90, 90, 90),
        frame: p.red_deep,
        shell: p.bg_deep,
    }
}

/// The Light theme's map, read as a printed atlas: pale sea, land stippled a
/// few shades into it, and markers dark enough to be found on both.
const SCOPE_MAP_LIGHT: MapPalette = MapPalette {
    sea: c(0xdbe7f1),
    land: c(0x6b93a7),
    station: c(0x111a24), // the white dots inverted — anything paler vanishes
    trail: c(0x006d8f),
    comet: c(0x0b6ea8),
    home: c(0x0b6129),
    dx: c(0xa8102f),
    hover: c(0x9c6100),
    preview: c(0xa87a00),
    hint: Color32::from_rgba_premultiplied(0, 0, 0, 110),
    frame: c(0xa8324e),
    // The map is inside a panel here, and a wedge of the *page* colour cut out
    // of its corners would read as a gap rather than as a bevel.
    shell: c(0xffffff),
};

/// The High-contrast map. Still a night sky — black is the highest contrast
/// ground there is for the bright markers — but the continents are stippled in
/// a grey you can actually make out, rather than the slate-teal that reads as
/// texture more than as land.
const MAP_HIGH_CONTRAST: MapPalette = MapPalette {
    sea: c(0x000000),
    land: c(0x7a7a7a), // 4.9:1 on the sea — the coastline, not a suggestion
    station: c(0xffffff),
    trail: c(0x1aebff),
    comet: c(0x1aebff),
    home: c(0x3ff23f),
    dx: c(0xff5555),
    hover: c(0xffff00),
    preview: c(0xff66ff),
    hint: Color32::from_rgba_premultiplied(200, 200, 200, 200),
    frame: c(0xff5555),
    shell: c(0x000000),
};

/// Indexed by [`theme_index`], like [`PALETTES`].
static MAP_PALETTES: [MapPalette; 7] = [
    map_from(&DEFAULT),
    map_from(&GREEN_PHOSPHOR),
    map_from(&AMBER_PHOSPHOR),
    map_from(&TEAL_ORANGE),
    map_from(&RAINBOW),
    SCOPE_MAP_LIGHT,
    MAP_HIGH_CONTRAST,
];

/// The current theme's world-map inks — see [`MapPalette`].
#[inline]
pub fn map() -> &'static MapPalette {
    &MAP_PALETTES[THEME.load(Ordering::Relaxed) as usize]
}

/// Indexed by [`style_index`].
const STYLE_ORDER: [ChromeStyle; 5] = [
    ChromeStyle::Angled,
    ChromeStyle::Rectangular,
    ChromeStyle::Rounded,
    ChromeStyle::Gradient,
    ChromeStyle::Bevel,
];

const fn theme_index(t: UiTheme) -> u8 {
    match t {
        UiTheme::Default => 0,
        UiTheme::GreenPhosphor => 1,
        UiTheme::AmberPhosphor => 2,
        UiTheme::TealOrange => 3,
        UiTheme::Rainbow => 4,
        UiTheme::Light => 5,
        UiTheme::HighContrast => 6,
    }
}

const fn style_index(s: ChromeStyle) -> u8 {
    match s {
        ChromeStyle::Angled => 0,
        ChromeStyle::Rectangular => 1,
        ChromeStyle::Rounded => 2,
        ChromeStyle::Gradient => 3,
        ChromeStyle::Bevel => 4,
    }
}

// The zero defaults are UiTheme::Default / ChromeStyle::Angled — the look the
// first frame wears if nothing calls `set_look` first.
static THEME: AtomicU8 = AtomicU8::new(0);
static BUTTON_STYLE: AtomicU8 = AtomicU8::new(0);
static WINDOW_STYLE: AtomicU8 = AtomicU8::new(0);

/// Select the process-wide look. Takes effect on whatever is painted next; the
/// egui `Visuals` derived from it need a separate [`apply_visuals`] call.
pub fn set_look(theme: UiTheme, buttons: ChromeStyle, windows: ChromeStyle) {
    THEME.store(theme_index(theme), Ordering::Relaxed);
    BUTTON_STYLE.store(style_index(buttons), Ordering::Relaxed);
    WINDOW_STYLE.store(style_index(windows), Ordering::Relaxed);
}

/// The current theme's palette.
#[inline]
pub fn palette() -> &'static Palette {
    &PALETTES[THEME.load(Ordering::Relaxed) as usize]
}

/// Whether the chrome currently sits on a bright ground.
#[inline]
pub fn is_light() -> bool {
    palette().light
}

/// [`gray`] for a theme with no dim register: the ink half of the scale is
/// lifted until even its bottom clears 7:1, and the surface half is left where
/// it is.
///
/// The split is at [`GRAY_KNEE`], and it is a real seam — 64 comes back as 64
/// and 65 as 155 — which is safe only because nothing uses the levels around
/// it. The scale has always had two populations with a wide gap between them:
/// the recessed *surfaces* (a progress trough at 24, an image mount at 6, an
/// unlit indicator at 48) and the de-emphasised *ink* (a caption at 110, a
/// unit suffix at 140). Lifting the surfaces too would make a bar's empty half
/// as loud as its full one, and an unlit indicator brighter than the lit one
/// beside it; leaving the ink alone would miss the whole point of the theme.
fn lifted(level: u8) -> u8 {
    if level <= GRAY_KNEE {
        return level;
    }
    let t = (level - GRAY_KNEE) as u16;
    let span = (255 - GRAY_KNEE) as u16;
    (GRAY_FLOOR as u16 + t * (255 - GRAY_FLOOR as u16) / span) as u8
}

/// Where [`lifted`] stops treating a level as a surface and starts treating it
/// as ink.
const GRAY_KNEE: u8 = 64;
/// The dimmest ink [`lifted`] will hand back: 7.4:1 on the high-contrast
/// panel, so even the faintest caption in the UI clears AAA.
const GRAY_FLOOR: u8 = 155;

/// [`gray`] for a mark painted on an instrument's dark glass rather than on
/// the panel — the panadapter's spot borders, its resize grip, the mini-scope
/// grids.
///
/// The two differ only on the Light theme, where the chrome inverted and the
/// glass did not: `gray` would hand these a shade picked for white paper and
/// paint it onto black. Everywhere else they agree, high contrast included,
/// where both lift.
pub fn scope_gray(level: u8) -> Color32 {
    if palette().no_dim { Color32::from_gray(lifted(level)) } else { Color32::from_gray(level) }
}

/// One sRGB channel, decoded to light.
fn to_linear(ch: u8) -> f32 {
    let c = ch as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// Light, encoded back to one sRGB channel.
fn from_linear(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let c = if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (c * 255.0).round().clamp(0.0, 255.0) as u8
}

/// WCAG relative luminance.
fn luminance(c: Color32) -> f32 {
    0.2126 * to_linear(c.r()) + 0.7152 * to_linear(c.g()) + 0.0722 * to_linear(c.b())
}

/// A neutral grey from the historic dark-theme scale, re-levelled for the
/// current theme.
///
/// Most of the UI's secondary text — "no memories yet", the units beside a
/// readout, a dupe's greyed-out callsign — was written as a literal
/// `Color32::from_gray(N)`, a scale that only means anything against a
/// near-black panel. Read straight onto white, `from_gray(150)` is 3.0:1:
/// visible, but well under the 4.5:1 a callsign glanced at off a list
/// deserves, and `from_gray(48)` — the "off" state of every little indicator
/// dot — would come out darker than the lit one.
///
/// So the level is *mirrored* rather than reused: the returned grey stands at
/// the same contrast from this theme's panel that `from_gray(level)` stood at
/// from the dark themes'. Dim stays dim, bright stays bright, and the ratio an
/// operator actually reads by is preserved — 150 lands on 96 (6.3:1 both
/// ways), 48 on 221. Dark themes get `from_gray(level)` straight back, bit for
/// bit.
pub fn gray(level: u8) -> Color32 {
    let p = palette();
    if p.no_dim {
        return Color32::from_gray(lifted(level));
    }
    if !p.light {
        return Color32::from_gray(level);
    }
    /// Luminance of [`DEFAULT`]`.panel` — the ground the historic grey scale
    /// was picked against.
    const DARK_PANEL: f32 = 0.005_657;
    let ratio = (to_linear(level) + 0.05) / (DARK_PANEL + 0.05);
    Color32::from_gray(from_linear((luminance(p.panel) + 0.05) / ratio - 0.05))
}

/// The shape buttons currently wear.
#[inline]
pub fn button_style() -> ChromeStyle {
    STYLE_ORDER[BUTTON_STYLE.load(Ordering::Relaxed) as usize]
}

/// The shape windows and popups currently wear.
#[inline]
pub fn window_style() -> ChromeStyle {
    STYLE_ORDER[WINDOW_STYLE.load(Ordering::Relaxed) as usize]
}

const fn font_index(f: FontSize) -> u8 {
    match f {
        FontSize::Small => 0,
        FontSize::Medium => 1,
        FontSize::Large => 2,
    }
}

// The defaults match `UiSettings::default()` — the sizes the first frame wears
// if nothing calls `set_font_sizes` first: skimmer and menus Medium (1),
// panadapter labels Small (0).
static SKIMMER_FONT: AtomicU8 = AtomicU8::new(1);
static PANADAPTER_FONT: AtomicU8 = AtomicU8::new(0);
static MENU_FONT: AtomicU8 = AtomicU8::new(1);

/// Select the process-wide font sizes. Like [`set_look`], the values live in
/// atomics rather than the egui context so the solar-3d viewport's menus see
/// the same sizes from their own thread. Takes effect on whatever is painted
/// next — the affected text is all laid out per frame.
pub fn set_font_sizes(skimmer: FontSize, panadapter: FontSize, menu: FontSize) {
    SKIMMER_FONT.store(font_index(skimmer), Ordering::Relaxed);
    PANADAPTER_FONT.store(font_index(panadapter), Ordering::Relaxed);
    MENU_FONT.store(font_index(menu), Ordering::Relaxed);
}

/// Scale for the skimmer / spot boxes on the waterfall. The historic point
/// sizes are the `Medium` step, so that is 1.
#[inline]
pub fn skimmer_font_scale() -> f32 {
    [0.85, 1.0, 1.2][SKIMMER_FONT.load(Ordering::Relaxed) as usize]
}

/// Scale for the labels painted onto the spectrum and waterfall (frequency
/// scale, band plan, measurements, markers). The historic point sizes are the
/// `Small` step, so that is 1.
#[inline]
pub fn panadapter_font_scale() -> f32 {
    [1.0, 1.25, 1.5][PANADAPTER_FONT.load(Ordering::Relaxed) as usize]
}

/// Scale for the interface itself — menus, dialogs, windows, the tab strip,
/// the top bar and every button and label on it. The historic sizes are the
/// `Medium` step, so that is 1.
///
/// This is applied as egui's own zoom factor (see [`apply_zoom`]) rather than
/// by scaling the text styles: barely half the text in this crate takes its
/// size from a [`TextStyle`], the rest asks for a point size by hand
/// (`RichText::size`, `FontId::proportional`), and a rewrite of the styles
/// alone left all of that — and the padding around it — at the historic size.
/// The zoom factor multiplies *every* point, so nothing is missed and nothing
/// outgrows the box it sits in.
#[inline]
pub fn ui_scale() -> f32 {
    [0.85, 1.0, 1.2][MENU_FONT.load(Ordering::Relaxed) as usize]
}

/// Put the current [`ui_scale`] into `ctx` as its zoom factor.
///
/// Called at startup and whenever the setting changes, never unconditionally
/// per frame: egui also owns this value — its own ctrl+plus / ctrl+minus zoom
/// writes to it — and re-asserting the setting every frame would undo an
/// operator's zoom the moment they made it. The panadapter and skimmer scales
/// ride on top of this one, so those two settings stay relative adjustments to
/// whatever size the interface as a whole is wearing.
pub fn apply_zoom(ctx: &egui::Context) {
    ctx.set_zoom_factor(ui_scale());
}

/// The palette under the historic constant names — these were `pub const`s
/// before the theme became switchable, and the call sites still read them by
/// name.
macro_rules! palette_accessors {
    ($($NAME:ident => $field:ident),* $(,)?) => {
        $(
            #[allow(non_snake_case)]
            #[inline]
            pub fn $NAME() -> Color32 { palette().$field }
        )*
    };
}

palette_accessors! {
    BG_DEEP => bg_deep,
    PANEL => panel,
    INPUT_BG => input_bg,
    FILL => fill,
    FILL_HOVER => fill_hover,
    FILL_ACTIVE => fill_active,
    LINE => line,
    LINE_LIT => line_lit,
    TEXT => text,
    TEXT_STRONG => text_strong,
    CYAN => cyan,
    CYAN_DIM => cyan_dim,
    PINK => pink,
    YELLOW => yellow,
    GREEN => green,
    INK_ON_CYAN => ink_on_cyan,
    INK_ON_BRIGHT => ink_on_bright,
    HAZARD => hazard,
    HAZARD_DARK => hazard_dark,
    RED_DEEP => red_deep,
    CQ_BG => cq_bg,
    TOME_BG => tome_bg,
    DONE_BG => done_bg,
    ROW_BG => row_bg,
    ROW_HOVER => row_hover,
    SCROLL_TRACK => scroll_track,
    SCROLL_HANDLE => scroll_handle,
    SCROLL_HANDLE_HOVER => scroll_handle_hover,
    SCROLL_HANDLE_DRAG => scroll_handle_drag,
    ALERT => alert,
}

/// A colour that came from *data* rather than the palette — a spot kind, a
/// band-plan segment — brought onto the current ground.
///
/// These are picked once, in `sdroxide-types`, for every client that draws
/// them, and they are picked for a dark one: pastels around 0.5 to 0.7
/// luminance, which land between 1.4:1 and 2.5:1 on white. Rather than a
/// second table that would have to be kept in step with the first, this takes
/// the exposure down until the colour clears about 5.5:1, in linear light so
/// the hue and the relative saturation survive the trip.
///
/// Returns the colour untouched on every dark theme.
pub fn data_ink(rgb: (u8, u8, u8)) -> Color32 {
    let (r, g, b) = rgb;
    let c = Color32::from_rgb(r, g, b);
    if !palette().light {
        return c;
    }
    /// The luminance the darkened colour aims for: 5.8:1 against white.
    const TARGET: f32 = 0.13;
    let lum = luminance(c);
    if lum <= TARGET {
        return c;
    }
    let k = TARGET / lum;
    Color32::from_rgb(
        from_linear(to_linear(r) * k),
        from_linear(to_linear(g) * k),
        from_linear(to_linear(b) * k),
    )
}

/// A colour per continent, so a list of decodes reads as a map at a glance —
/// which way the band is open is visible before a single callsign is read.
/// Anything unrecognised comes back grey.
///
/// The Light theme needs its own set: these are ink, and the pastel-bright
/// originals sit between 1.3:1 and 1.9:1 on white — the whole cue would
/// disappear. Same seven hues, taken down to 4.6:1 or better, and kept as far
/// apart from each other as they were.
pub fn continent_color(code: &str) -> Color32 {
    if palette().light {
        return match code {
            "EU" => c(0x1d4ed8),
            "NA" => c(0x0d7030),
            "SA" => c(0x9a4a00),
            "AS" => c(0xb01a80),
            "AF" => c(0x7a5600),
            "OC" => c(0x00706b),
            "AN" => c(0x4a5c73),
            _ => gray(110),
        };
    }
    match code {
        "EU" => Color32::from_rgb(0x7c, 0xa8, 0xff),
        "NA" => Color32::from_rgb(0x46, 0xe0, 0x7d),
        "SA" => Color32::from_rgb(0xff, 0xa5, 0x3f),
        "AS" => Color32::from_rgb(0xff, 0x7a, 0xd9),
        "AF" => Color32::from_rgb(0xff, 0xd2, 0x3f),
        "OC" => Color32::from_rgb(0x3f, 0xe0, 0xd8),
        "AN" => Color32::from_rgb(0xd0, 0xe4, 0xf4),
        _ => Color32::from_gray(110),
    }
}

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    apply_metrics(ctx, crate::layout::Tier::Desktop);
    apply_visuals(ctx);
    apply_zoom(ctx);
}

/// Write the current palette and chrome shapes into the context style. Split
/// out of [`apply`] so a theme change at runtime doesn't rebuild the font
/// atlas.
///
/// Call this only from app construction and the top-of-frame look check —
/// never from inside a frame: [`ScrollPalette`] clones the visuals out of the
/// context style and writes them back, and a rewrite between its push and pop
/// would be popped away again.
pub fn apply_visuals(ctx: &egui::Context) {
    let p = palette();
    // Stock egui widgets follow the chosen shapes as far as a rounded rect
    // can: the Rounded styles round them, every other style keeps them sharp
    // (the cut corners of the Angled style are painted by `chrome`, not egui).
    let window_corner = match window_style() {
        ChromeStyle::Rounded => CornerRadius::same(6),
        _ => CornerRadius::ZERO,
    };
    let button_corner = match button_style() {
        ChromeStyle::Rounded => CornerRadius::same(5),
        _ => CornerRadius::ZERO,
    };

    // egui reads `dark_mode` itself in a few places we never set by hand — the
    // shadow under a window, the tint on a disabled widget, `Color32::
    // placeholder` fallbacks — so it has to agree with the palette or those
    // land inverted.
    ctx.set_theme(if p.light { egui::Theme::Light } else { egui::Theme::Dark });
    ctx.all_styles_mut(|style| {
        let v = &mut style.visuals;
        v.dark_mode = !p.light;
        v.panel_fill = p.panel;
        v.window_fill = p.panel;
        v.extreme_bg_color = p.input_bg;
        v.faint_bg_color = p.faint_bg;
        v.code_bg_color = p.input_bg;

        v.window_stroke = Stroke::new(1.0, p.pink);
        v.window_corner_radius = window_corner;
        v.menu_corner_radius = window_corner;

        v.selection.bg_fill = p.cyan;
        v.selection.stroke = Stroke::new(1.0, p.ink_on_cyan);
        v.hyperlink_color = p.cyan;
        v.warn_fg_color = p.yellow;
        v.error_fg_color = p.alert;
        v.slider_trailing_fill = true;

        v.widgets.noninteractive.bg_fill = p.panel;
        v.widgets.noninteractive.weak_bg_fill = p.panel;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
        v.widgets.noninteractive.corner_radius = button_corner;

        v.widgets.inactive.bg_fill = p.fill;
        v.widgets.inactive.weak_bg_fill = p.fill;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.line_lit);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
        v.widgets.inactive.corner_radius = button_corner;

        v.widgets.hovered.bg_fill = p.fill_hover;
        v.widgets.hovered.weak_bg_fill = p.fill_hover;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.cyan_dim);
        v.widgets.hovered.fg_stroke = Stroke::new(1.2, p.text_strong);
        v.widgets.hovered.corner_radius = button_corner;

        v.widgets.active.bg_fill = p.fill_active;
        v.widgets.active.weak_bg_fill = p.fill_active;
        v.widgets.active.bg_stroke = Stroke::new(1.0, p.cyan);
        v.widgets.active.fg_stroke = Stroke::new(1.2, p.cyan);
        v.widgets.active.corner_radius = button_corner;

        v.widgets.open.bg_fill = p.fill_active;
        v.widgets.open.weak_bg_fill = p.fill_active;
        v.widgets.open.bg_stroke = Stroke::new(1.0, p.cyan_dim);
        v.widgets.open.fg_stroke = Stroke::new(1.0, p.text_strong);
        v.widgets.open.corner_radius = button_corner;
    });
}

/// Sizes, spacing and hit targets for one layout tier. Split out of [`apply`]
/// so it can be re-run when the window crosses a breakpoint — [`apply`] itself
/// installs the fonts, and rebuilding the atlas every time a window is dragged
/// across 600 pt would be ruinous.
///
/// Deliberately touches no `visuals`: [`ScrollPalette`] swaps those in and out
/// of the same context style, and the two must not fight over it.
///
/// A touched screen gets bigger type and roomier targets. Everything a finger
/// has to hit is sized from `button_padding` and `interact_size` — chips
/// through [`crate::chrome::chip`], fields through egui itself — so those two
/// carry most of the tier here.
pub fn apply_metrics(ctx: &egui::Context, tier: crate::layout::Tier) {
    let touch = tier.touch();
    let body = if touch { 14.5 } else { 13.5 };
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(16.0, FontFamily::Name("chakra-bold".into()))),
            (TextStyle::Body, FontId::new(body, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(body, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();

        style.spacing.item_spacing =
            if touch { egui::vec2(6.0, 7.0) } else { egui::vec2(7.0, 5.0) };
        style.spacing.button_padding =
            if touch { egui::vec2(11.0, 9.0) } else { egui::vec2(7.0, 3.0) };
        // Roughly a fingertip where the screen is touched, egui's own default
        // where it is not. Set in *both* directions on purpose: this runs again
        // whenever the window crosses a breakpoint, and a value left behind by
        // the tier before would keep every field and every control box that
        // sizes from it stretched after the layout had gone back to a mouse.
        style.spacing.interact_size.y = if touch { 34.0 } else { 18.0 };
        // Fixed slider width: otherwise sliders expand to fill the row, so a
        // module with a slider balloons and pushes later modules off-screen
        // instead of letting `horizontal_wrapped` wrap them.
        style.spacing.slider_width = if touch { 110.0 } else { 84.0 };
        style.spacing.combo_width = 84.0;

        // egui's default scrollbars float over the content as a 2 px hairline that
        // only fades in on hover — easy to miss and hard to grab. Use solid bars of
        // a constant width that are always fully opaque.
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: if touch { 14.0 } else { 9.0 },
            handle_min_length: 24.0,
            bar_inner_margin: 3.0,
            bar_outer_margin: 0.0,
            // Take the handle colour from `fg_stroke` instead of the (near-black)
            // widget fill, so bars we don't paint by hand — combo popups, menus —
            // still get a handle that stands out from the gutter.
            foreground_color: true,
            ..egui::style::ScrollStyle::solid()
        };
    });
}

/// Paint the scrollbar palette into `v`: cyan handle in a recessed gutter,
/// brightening on hover and going hot pink while dragged.
///
/// egui has no scrollbar colours of its own — it takes the handle colour from
/// the widget visuals of whatever `Ui` owns the bars (with `foreground_color`,
/// the same `fg_stroke` that colours button labels), so this can't simply live
/// in [`apply`]. It goes in around a scroll area and comes back off its
/// contents instead; see [`ThemedScroll`] and [`ScrollPalette`].
fn scroll_palette(v: &mut egui::Visuals) {
    let p = palette();
    v.extreme_bg_color = p.scroll_track; // the gutter
    v.widgets.inactive.fg_stroke.color = p.scroll_handle;
    v.widgets.hovered.fg_stroke.color = p.scroll_handle_hover;
    v.widgets.active.fg_stroke.color = p.scroll_handle_drag;
}

/// `ScrollArea::show` with themed scrollbars.
pub trait ThemedScroll {
    /// Show the area, painting its bars in the [`scroll_palette`].
    fn show_themed<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::containers::scroll_area::ScrollAreaOutput<R>;

    /// `ScrollArea::show_rows` with the same theming.
    ///
    /// For a list long enough that laying every row out per frame is the cost:
    /// egui works out which rows the viewport covers from `row_height` alone and
    /// calls the closure with just those. Every row must therefore actually be
    /// `row_height` tall, or the scroll bar and the content disagree.
    fn show_rows_themed<R>(
        self,
        ui: &mut egui::Ui,
        row_height: f32,
        total_rows: usize,
        add_contents: impl FnOnce(&mut egui::Ui, std::ops::Range<usize>) -> R,
    ) -> egui::containers::scroll_area::ScrollAreaOutput<R>;
}

impl ThemedScroll for egui::ScrollArea {
    fn show_themed<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::containers::scroll_area::ScrollAreaOutput<R> {
        let normal = ui.visuals().clone();
        scroll_palette(ui.visuals_mut());

        let out = self.show(ui, |ui| {
            *ui.visuals_mut() = normal.clone();
            add_contents(ui)
        });

        *ui.visuals_mut() = normal;
        out
    }

    fn show_rows_themed<R>(
        self,
        ui: &mut egui::Ui,
        row_height: f32,
        total_rows: usize,
        add_contents: impl FnOnce(&mut egui::Ui, std::ops::Range<usize>) -> R,
    ) -> egui::containers::scroll_area::ScrollAreaOutput<R> {
        let normal = ui.visuals().clone();
        scroll_palette(ui.visuals_mut());

        let out = self.show_rows(ui, row_height, total_rows, |ui, rows| {
            *ui.visuals_mut() = normal.clone();
            add_contents(ui, rows)
        });

        *ui.visuals_mut() = normal;
        out
    }
}

/// The [`scroll_palette`], lent to the context style for containers that build
/// their own scrollbars — an [`egui::Window`] with `vscroll`, whose bars belong
/// to a `Ui` inside egui that we never get to hold.
///
/// Push it before showing the container, hand the body back the normal palette
/// with [`Self::restore`], then [`Self::pop`] it off the context again.
#[must_use = "the palette stays in the context style until popped"]
pub struct ScrollPalette(egui::Visuals);

impl ScrollPalette {
    pub fn push(ctx: &egui::Context) -> Self {
        let normal = ctx.style_of(ctx.theme()).visuals.clone();
        let mut bars = normal.clone();
        scroll_palette(&mut bars);
        ctx.set_visuals(bars);
        Self(normal)
    }

    /// Give a container body the normal palette back — and the context with it,
    /// so tooltips and dropdowns opened from the body aren't tinted either. The
    /// bars keep the palette: their `Ui` was built (and took its copy of the
    /// style) before this runs.
    pub fn restore(&self, ui: &mut egui::Ui) {
        ui.ctx().set_visuals(self.0.clone());
        *ui.visuals_mut() = self.0.clone();
    }

    /// Take the palette off the context — a no-op once [`Self::restore`] has
    /// run, and the way it comes back off when the container never shows a body
    /// (a closed or collapsed window).
    pub fn pop(self, ctx: &egui::Context) {
        ctx.set_visuals(self.0);
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "chakra".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/ChakraPetch-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "chakra-bold".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/ChakraPetch-SemiBold.ttf"))),
    );
    // Angular techno monospace for the FT8 decode list (Share Tech Mono, OFL).
    fonts.font_data.insert(
        "cyber-mono".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/ShareTechMono-Regular.ttf"
        ))),
    );

    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "chakra".into());
    }
    // Make Share Tech Mono the primary monospace (used by the decode list,
    // frequency readout, and meters).
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(0, "cyber-mono".into());
    }
    fonts.families.insert(FontFamily::Name("cyber-mono".into()), vec!["cyber-mono".to_string()]);
    // Bold family for headings, falling back through the proportional stack.
    let mut bold_stack = vec!["chakra-bold".to_string()];
    if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
        bold_stack.extend(prop.iter().cloned());
    }
    fonts.families.insert(FontFamily::Name("chakra-bold".into()), bold_stack);

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Tier;

    /// Every metric [`apply_metrics`] sets has to be set in *both* directions.
    ///
    /// It runs again whenever the window crosses a breakpoint, so a value
    /// written only for the touched layouts stays behind when the window goes
    /// back to a desktop one. That is how the control boxes came to stand
    /// taller than the frequency box beside them, and how the RIT field came to
    /// sit a few points low in its row: `interact_size` was still a fingertip
    /// tall long after the layout had gone back to a mouse.
    #[test]
    fn desktop_metrics_survive_a_trip_through_the_touch_layouts() {
        let ctx = egui::Context::default();
        apply_metrics(&ctx, Tier::Desktop);
        let (spacing, text) = {
            let s = ctx.style_of(ctx.theme());
            (s.spacing.clone(), s.text_styles.clone())
        };

        for tier in [Tier::Phone, Tier::Tablet, Tier::Phone] {
            apply_metrics(&ctx, tier);
        }
        apply_metrics(&ctx, Tier::Desktop);

        let s = ctx.style_of(ctx.theme());
        assert_eq!(spacing, s.spacing, "a spacing metric was left behind by a touch layout");
        assert_eq!(text, s.text_styles, "a text size was left behind by a touch layout");
    }

    /// The touched layouts really do differ — otherwise the test above would
    /// pass on a function that had stopped doing anything at all.
    #[test]
    fn a_touched_layout_gets_bigger_targets() {
        let ctx = egui::Context::default();
        apply_metrics(&ctx, Tier::Desktop);
        let desktop = ctx.style_of(ctx.theme()).spacing.clone();
        apply_metrics(&ctx, Tier::Phone);
        let phone = ctx.style_of(ctx.theme()).spacing.clone();

        assert!(phone.interact_size.y > desktop.interact_size.y);
        assert!(phone.button_padding.y > desktop.button_padding.y);
        assert!(phone.scroll.bar_width > desktop.scroll.bar_width);
    }

    /// The default palette is the classic look and must never drift: these are
    /// the exact values of the `pub const`s the theme system replaced, which
    /// every screenshot in the manual shows.
    #[test]
    fn default_palette_is_the_historic_look() {
        let p = &PALETTES[theme_index(UiTheme::Default) as usize];
        for (got, want, name) in [
            (p.bg_deep, 0x050810, "bg_deep"),
            (p.panel, 0x0b111e, "panel"),
            (p.input_bg, 0x04070e, "input_bg"),
            (p.fill, 0x101a2c, "fill"),
            (p.fill_hover, 0x17243c, "fill_hover"),
            (p.fill_active, 0x1d2f4d, "fill_active"),
            (p.line, 0x1a2740, "line"),
            (p.line_lit, 0x2a4a66, "line_lit"),
            (p.text, 0xb4c6da, "text"),
            (p.text_strong, 0xe8f4ff, "text_strong"),
            (p.cyan, 0x00d0f4, "cyan"),
            (p.cyan_dim, 0x1d9cbe, "cyan_dim"),
            (p.pink, 0xff2a55, "pink"),
            (p.yellow, 0xffd23f, "yellow"),
            (p.green, 0x46e07d, "green"),
            (p.ink_on_cyan, 0x021019, "ink_on_cyan"),
            (p.red_deep, 0x6e182c, "red_deep"),
            (p.cq_bg, 0x240c15, "cq_bg"),
            (p.tome_bg, 0x2c2406, "tome_bg"),
            (p.done_bg, 0x082a17, "done_bg"),
            (p.row_bg, 0x0a101b, "row_bg"),
            (p.row_hover, 0x141e2e, "row_hover"),
            (p.scroll_track, 0x0e1626, "scroll_track"),
            (p.scroll_handle, 0x178ead, "scroll_handle"),
            (p.scroll_handle_hover, 0x00d0f4, "scroll_handle_hover"),
            (p.scroll_handle_drag, 0xff2a55, "scroll_handle_drag"),
            (p.faint_bg, 0x0e1626, "faint_bg"),
            (p.alert, 0xff2a55, "alert"),
        ] {
            assert_eq!(got, c(want), "default palette field {name} drifted");
        }
    }

    /// The default S-meter instrument must stay the historic one: these are
    /// the exact values of the constants `widgets::smeter` carried before the
    /// instrument followed the theme. Rainbow shares them by design.
    #[test]
    fn default_meter_palette_is_the_historic_instrument() {
        let m = &METER_PALETTES[theme_index(UiTheme::Default) as usize];
        assert_eq!(METER_PALETTES[theme_index(UiTheme::Rainbow) as usize], *m);
        for (got, want, name) in [
            (m.face_top, 0x111b2b, "face_top"),
            (m.face_bot, 0x03060c, "face_bot"),
            (m.glass, 0x2c4460, "glass"),
            (m.backlight, 0x1c7496, "backlight"),
            (m.readout, 0xe6f6ff, "readout"),
            (m.subdued, 0x849eb6, "subdued"),
            (m.tick_minor, 0x54708a, "tick_minor"),
            (m.tick_major, 0xcfe6f5, "tick_major"),
            (m.label, 0xbdd6e8, "label"),
            (m.grid_line, 0x1b2a3e, "grid_line"),
            (m.grid_label, 0x637c92, "grid_label"),
            (m.rail_top, 0x04070d, "rail_top"),
            (m.rail_bot, 0x131f2f, "rail_bot"),
            (m.rail_edge, 0x1e2e42, "rail_edge"),
            (m.ramp_lo, 0x16627e, "ramp_lo"),
            (m.ramp_mid, 0x1da8cf, "ramp_mid"),
            (m.ramp_hi, 0x8eecff, "ramp_hi"),
        ] {
            assert_eq!(got, c(want), "default meter palette field {name} drifted");
        }
    }

    /// Every theme and style maps to a distinct in-bounds slot — the indices
    /// are what ties the enums to `PALETTES`/`STYLE_ORDER`, and a botched
    /// mapping would silently hand one theme another theme's colours.
    ///
    /// Deliberately does not call [`set_look`]: the atomics are process-wide
    /// and tests run in parallel, so flipping the theme here would repaint
    /// another test's world.
    #[test]
    fn look_indices_are_a_bijection() {
        let mut seen = [false; PALETTES.len()];
        for t in UiTheme::ALL {
            let i = theme_index(t) as usize;
            assert!(!seen[i], "two themes share palette slot {i}");
            seen[i] = true;
        }
        assert_eq!(theme_index(UiTheme::Default), 0, "Default must be the boot-up palette");

        let mut seen = [false; STYLE_ORDER.len()];
        for s in ChromeStyle::ALL {
            let i = style_index(s) as usize;
            assert_eq!(STYLE_ORDER[i], s, "STYLE_ORDER and style_index disagree");
            assert!(!seen[i], "two styles share slot {i}");
            seen[i] = true;
        }
        assert_eq!(style_index(ChromeStyle::Angled), 0, "Angled must be the boot-up style");
    }

    /// The alert role stays red in every theme — the phosphor themes are
    /// monochrome, but a TX or SWR indication must never come out green.
    #[test]
    fn every_theme_keeps_alerts_red() {
        for (i, p) in PALETTES.iter().enumerate() {
            assert!(
                p.alert.r() > 180 && p.alert.g() < 90 && p.alert.b() < 120,
                "palette {i} has a non-red alert colour {:?}",
                p.alert
            );
        }
    }

    /// WCAG contrast between two opaque colours.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Every role the Light theme uses as *ink* clears 4.5:1 on its panel.
    ///
    /// This is the whole reason that palette's accents are as dark as they
    /// are, and it is an easy thing to undo by eye: `green` and `yellow` in
    /// particular look wrong beside the other themes' columns and invite being
    /// "fixed" back to a bright hue — at which point a signal report or a
    /// warning line lands at about 1.5:1 on white and cannot be read at all.
    #[test]
    fn the_light_theme_has_no_bright_ink_on_white() {
        let p = &PALETTES[theme_index(UiTheme::Light) as usize];
        assert!(p.light, "the Light palette must declare itself light");
        for (ink, name) in [
            (p.text, "text"),
            (p.text_strong, "text_strong"),
            (p.cyan, "cyan"),
            (p.cyan_dim, "cyan_dim"),
            (p.pink, "pink"),
            (p.yellow, "yellow"),
            (p.green, "green"),
            (p.alert, "alert"),
        ] {
            let r = contrast(ink, p.panel);
            assert!(r >= 4.5, "Light {name} is {r:.2}:1 on the panel — needs 4.5:1");
        }
        // …and on the row tints a decode list paints behind that ink.
        for (bg, name) in [(p.cq_bg, "cq_bg"), (p.tome_bg, "tome_bg"), (p.done_bg, "done_bg")] {
            let r = contrast(p.text, bg);
            assert!(r >= 4.5, "Light text is {r:.2}:1 on {name} — needs 4.5:1");
        }
        // The ink each accent fill carries has to clear it the other way too.
        for (fill, name) in [(p.cyan, "cyan"), (p.green, "green"), (p.yellow, "yellow")] {
            let ink = if name == "cyan" { p.ink_on_cyan } else { p.ink_on_bright };
            let r = contrast(ink, fill);
            assert!(r >= 4.5, "Light ink on the {name} chip is {r:.2}:1 — needs 4.5:1");
        }
    }

    /// The Light theme mirrors the *depth* of the grounds.
    ///
    /// A tab, a module box and a button face each place themselves by stepping
    /// away from `panel`, and on a bright ground every one of those steps has
    /// to go the other way — a button face lighter than a white panel is not a
    /// raised control, it is nothing at all.
    ///
    /// The page is the exception, and is checked the other way on purpose: it
    /// sits *behind* the panels in both, so it stays the darker of the two
    /// however bright the theme is. Mirroring it too would light the
    /// unselected tabs and sink the selected one.
    #[test]
    fn the_light_theme_mirrors_the_ground_order() {
        let (d, l) = (&DEFAULT, &PALETTES[theme_index(UiTheme::Light) as usize]);
        for (dark_a, dark_b, light_a, light_b, what) in [
            (d.panel, d.fill, l.panel, l.fill, "panel vs button face"),
            (d.fill, d.fill_hover, l.fill, l.fill_hover, "button face vs hover"),
            (d.fill_hover, d.fill_active, l.fill_hover, l.fill_active, "hover vs active"),
            (d.panel, d.line, l.panel, l.line, "panel vs border"),
            (d.line, d.line_lit, l.line, l.line_lit, "border vs lit border"),
            (d.row_bg, d.scroll_track, l.row_bg, l.scroll_track, "list vs scroll gutter"),
        ] {
            let dark_step = luminance(dark_b) - luminance(dark_a);
            let light_step = luminance(light_b) - luminance(light_a);
            assert!(
                dark_step * light_step < 0.0,
                "Light did not mirror {what}: {dark_step:+.4} dark, {light_step:+.4} light"
            );
        }
        for p in PALETTES.iter() {
            assert!(
                luminance(p.bg_deep) < luminance(p.panel),
                "a palette put its page in front of its panels"
            );
        }
    }

    /// The instruments do *not* follow the ground — except the map, which
    /// does.
    ///
    /// A waterfall's colour maps all start at black, so a bright panadapter
    /// has no form to take; a map plainly does. That split is the whole reason
    /// [`ScopePalette`] and [`MapPalette`] are separate, and it is invisible
    /// from any one call site, so it is asserted here: the Light theme's scope
    /// inks have to stay *brighter* than its glass, and its map inks *darker*
    /// than its sea.
    #[test]
    fn the_light_theme_keeps_its_glass_dark_and_its_map_bright() {
        let i = theme_index(UiTheme::Light) as usize;
        let (scope, map, meter) = (&SCOPE_PALETTES[i], &MAP_PALETTES[i], &METER_PALETTES[i]);

        assert_eq!(meter.face, MeterFace::Paper, "the Light meter is a printed dial");
        assert_eq!(meter.needle_tip, Color32::BLACK, "a printed pointer is black at the point");
        assert!(
            luminance(meter.face_top) > 0.5 && luminance(meter.readout) < 0.1,
            "the paper dial wants dark ink on a light card"
        );
        for (face, name) in [(scope.ground, "scope ground"), (scope.shell, "scope shell")] {
            assert!(luminance(face) < 0.06, "the Light {name} must stay dark glass");
        }
        for (ink, name) in [
            (scope.ink, "ink"),
            (scope.accent, "accent"),
            (scope.good, "good"),
            (scope.warn, "warn"),
            (scope.chrome, "chrome"),
        ] {
            let r = contrast(ink, scope.ground);
            assert!(r >= 4.5, "Light scope {name} is {r:.2}:1 on the glass — needs 4.5:1");
        }

        assert!(luminance(map.sea) > 0.5, "the Light map is an atlas, not a night sky");
        for (ink, name) in [
            (map.land, "land"),
            (map.station, "station"),
            (map.trail, "trail"),
            (map.home, "home"),
            (map.dx, "dx"),
            (map.hover, "hover"),
            (map.frame, "frame"),
        ] {
            assert!(
                luminance(ink) < luminance(map.sea),
                "Light map {name} is lighter than the sea it is drawn on"
            );
        }
        // The station dot is the neutral one every coloured marker is read
        // against, and the one the user has to be able to find: 7:1.
        let r = contrast(map.station, map.sea);
        assert!(r >= 7.0, "Light map station dots are {r:.2}:1 on the sea — needs 7:1");
    }

    /// [`data_ink`] leaves a dark theme's data colours alone and brings a
    /// bright one's down to where they can be read, without turning them grey.
    #[test]
    fn data_colours_keep_their_hue_when_they_are_darkened() {
        // The spot-kind colours, as `sdroxide-types` publishes them.
        for rgb in [(120u8, 220u8, 255u8), (120, 230, 140), (255, 190, 90), (210, 150, 255)] {
            let (r, g, b) = rgb;
            assert_eq!(
                Color32::from_rgb(r, g, b),
                data_ink(rgb),
                "a dark theme's data colour drifted"
            );
        }

        let light = &PALETTES[theme_index(UiTheme::Light) as usize];
        let darkened = |rgb: (u8, u8, u8)| {
            const TARGET: f32 = 0.13;
            let c = Color32::from_rgb(rgb.0, rgb.1, rgb.2);
            let k = TARGET / luminance(c);
            Color32::from_rgb(
                from_linear(to_linear(rgb.0) * k),
                from_linear(to_linear(rgb.1) * k),
                from_linear(to_linear(rgb.2) * k),
            )
        };
        for rgb in [(120u8, 220u8, 255u8), (120, 230, 140), (255, 190, 90), (210, 150, 255)] {
            let c = darkened(rgb);
            let r = contrast(c, light.panel);
            assert!(r >= 4.5, "a darkened {rgb:?} is {r:.2}:1 on white — needs 4.5:1");
            // Still that colour: the channel with the most of it going in has
            // the most of it coming out.
            let arg_max = |(r, g, b): (u8, u8, u8)| {
                if r >= g && r >= b {
                    0
                } else if g >= b {
                    1
                } else {
                    2
                }
            };
            assert_eq!(
                arg_max(rgb),
                arg_max((c.r(), c.g(), c.b())),
                "darkening {rgb:?} moved its hue"
            );
        }
    }

    /// The High-contrast theme has no dim register anywhere an operator reads.
    ///
    /// Every one of its ink roles clears AAA on the panel, every hand-written
    /// grey that [`gray`] hands back as *ink* clears it too, and the row tints
    /// a decode list paints behind white text do not eat into it. This is the
    /// whole promise of the theme, and it is spread over a palette, a grey
    /// scale and an instrument, so it is asserted in one place.
    #[test]
    fn the_high_contrast_theme_has_no_dim_register() {
        let i = theme_index(UiTheme::HighContrast) as usize;
        let p = &PALETTES[i];
        assert!(p.no_dim, "the High-contrast palette must declare itself");
        assert!(!p.light, "it is white on black, not black on white");

        for (ink, name, floor) in [
            (p.text, "text", 7.0),
            (p.text_strong, "text_strong", 7.0),
            (p.cyan, "cyan", 7.0),
            (p.cyan_dim, "cyan_dim", 7.0),
            (p.pink, "pink", 7.0),
            (p.yellow, "yellow", 7.0),
            (p.green, "green", 7.0),
            // The one role where the hue outranks the ratio: a red bright
            // enough for AAA is no longer a red. AA, and no lower.
            (p.alert, "alert", 4.5),
            // Borders are UI components, not text — WCAG asks 3:1 of them.
            (p.line, "line", 4.5),
            (p.line_lit, "line_lit", 7.0),
        ] {
            let r = contrast(ink, p.panel);
            assert!(r >= floor, "High-contrast {name} is {r:.2}:1 on the panel — needs {floor}:1");
        }
        for (bg, name) in [
            (p.cq_bg, "cq_bg"),
            (p.tome_bg, "tome_bg"),
            (p.done_bg, "done_bg"),
            (p.row_hover, "row_hover"),
        ] {
            let r = contrast(p.text, bg);
            assert!(r >= 7.0, "High-contrast text is {r:.2}:1 on {name} — needs 7:1");
        }

        // The lifted grey scale: every level the UI uses as ink comes back
        // above AAA, and every level it uses as a *surface* is left alone.
        for level in [70u8, 90, 110, 120, 140, 150, 190] {
            let r = contrast(Color32::from_gray(lifted(level)), p.panel);
            assert!(r >= 7.0, "lifted grey {level} is {r:.2}:1 — needs 7:1");
        }
        for level in [6u8, 20, 24, 38, 45, 48, 60] {
            assert_eq!(level, lifted(level), "a surface grey was lifted into the ink range");
        }
        // Nothing in the codebase sits in the seam, which is the only reason
        // the seam is safe. If a call site ever lands here, it has to be
        // classified as ink or as surface first.
        assert!(GRAY_KNEE > 60 && GRAY_FLOOR > GRAY_KNEE);

        // The instrument follows: a white pointer, white graduations, and a
        // "subdued" reading that is barely subdued.
        let m = &METER_PALETTES[i];
        assert_eq!(m.face, MeterFace::Contrast);
        assert_eq!(m.needle_mid, Color32::WHITE, "the high-contrast pointer is white");
        assert_eq!(m.needle_glow, Color32::TRANSPARENT, "a white blade throws no halo");
        for (ink, name) in [
            (m.readout, "readout"),
            (m.subdued, "subdued"),
            (m.tick_minor, "tick_minor"),
            (m.tick_major, "tick_major"),
            (m.label, "label"),
            (m.grid_label, "grid_label"),
        ] {
            let r = contrast(ink, m.face_bot);
            assert!(r >= 7.0, "High-contrast meter {name} is {r:.2}:1 on the dial — needs 7:1");
        }

        // …and so does the map: the continents have to be findable.
        let map = &MAP_PALETTES[i];
        let r = contrast(map.land, map.sea);
        assert!(r >= 4.5, "High-contrast land is {r:.2}:1 on the sea — needs 4.5:1");
    }

    /// [`gray`] hands the dark themes their historic level back untouched, and
    /// gives the Light theme the *same contrast* the other way up.
    ///
    /// Equal contrast, not more: these levels are the UI's de-emphasised ink,
    /// and a mirror that quietly promoted them would flatten the difference
    /// between a caption and the body text beside it. What the mirror does
    /// guarantee is that the ordering survives — an unlit indicator stays
    /// fainter than a caption, which stays fainter than the ink.
    #[test]
    fn the_grey_scale_is_mirrored_not_reused() {
        // Deliberately does not call `set_look` — the theme atomic is
        // process-wide and the tests run in parallel.
        for level in [8u8, 48, 70, 90, 110, 150, 190] {
            assert_eq!(Color32::from_gray(level), gray(level), "a dark theme's grey drifted");
        }

        let light = &PALETTES[theme_index(UiTheme::Light) as usize];
        let mirrored = |level: u8| {
            const DARK_PANEL: f32 = 0.005_657;
            let ratio = (to_linear(level) + 0.05) / (DARK_PANEL + 0.05);
            Color32::from_gray(from_linear((luminance(light.panel) + 0.05) / ratio - 0.05))
        };
        for level in [48u8, 70, 90, 110, 120, 150, 190] {
            let (was, now) = (
                contrast(Color32::from_gray(level), DEFAULT.panel),
                contrast(mirrored(level), light.panel),
            );
            assert!(
                (was - now).abs() / was < 0.05,
                "grey {level} was {was:.2}:1 on navy and came back {now:.2}:1 on white"
            );
            // On the other side of the panel from where it started.
            assert!(
                luminance(mirrored(level)) < luminance(light.panel),
                "grey {level} came back lighter than the panel it sits on"
            );
        }
        // The ordering the whole scale rests on: an unlit indicator is fainter
        // than a caption, and a caption is fainter than the body ink.
        let faint = |c: Color32| contrast(c, light.panel);
        assert!(faint(mirrored(48)) < faint(mirrored(150)));
        assert!(faint(mirrored(150)) < faint(light.text));
    }
}
