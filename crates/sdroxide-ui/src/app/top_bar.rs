//! The control strip along the top of the window.
//!
//! One method per module in the strip, all called from [`SdroxideApp::top_bar`]
//! in the order they appear: frequency, S-meter, VFO/RIT, band and mode,
//! RX filter, sub-RX, TX, the skimmer and display popups, and the window
//! buttons. Each pushes [`Command`]s rather than touching the controller, so
//! the whole strip is a pure function of the state it draws from.
//!
//! Only the desktop draws that full strip, and it lays it out itself rather
//! than letting a wrapping row break wherever a box happens not to fit: every
//! box is one uniform height with its controls in up to two rows, every width
//! is measured before anything is drawn, [`plan_top_strip`] picks the row
//! breaks that come out balanced, and each row's leftover goes to the boxes
//! that can use it — the S-meter widens, sliders lengthen, chips spread — so
//! the strip meets the right edge instead of leaving a ragged gap there.
//!
//! A tablet keeps the readout and the S-meter at full size and folds
//! everything else into menus; a phone stacks a compact readout, the meter and
//! one row of menu chips. And a tablet-tier window too *short* for even two
//! stacked rows — a 1280x720 panel — gets the single-row strip: the VFO box
//! (a type-in readout over an S-meter bar) beside a thumb-sized PTT and two
//! rows of menu buttons stretched to the edge of the screen.

use eframe::egui::{self, Color32, ComboBox, DragValue, RichText, Slider};
use sdroxide_types::{
    AgcMode, BURST_MS_RANGE, Band, Command, CwSkimmerDecoder, DCS_CODES, DIV_FREEZE_ELEMENT,
    DIV_MODE_ELEMENT, DIV_RATE_ELEMENT, DIV_RESET_ELEMENT, DeviceCaps, Direction, DiversityMode,
    GainElement, GainUnit, MAX_OFFSET_HZ, Mode, NrEngine, NrLevel, NrStrength, RadioState, RxId,
    Shift, SkimmerKind, SpectrumDetail, Speed, SubTone, ToneMode, Vfo,
};

use crate::widgets::{freq_display, smeter};

use crate::app::SdroxideApp;
use crate::app::panels::digi_freq_for_band;
use crate::chrome::StyledCombo;

/// Width of the VFO A/B column in the frequency box.
const AB_W: f32 = 68.0;
/// Text size whose chip height the link chip above the A/B selector takes —
/// the size the radio strip's LINK switch uses, because it is the same
/// switch. (The chip itself carries a painted symbol, not text: see
/// [`crate::chrome::chip_link`].)
const LINK_TEXT: f32 = 11.0;
/// Vertical gap between the link chip and the A/B selector under it.
const LINK_GAP: f32 = 4.0;
/// What the RIG box's receiving-antenna chip says, in the one place the width
/// measurement and the chip itself both read it from.
const RX_ANT_LABEL: &str = "RX ANT";
/// Width of the frequency box's right column (inactive VFO + band/mode chip).
const RIGHT_W: f32 = 96.0;
/// Width of the S-meter box at its design size. It has no ceiling: the bar and
/// trace faces scale to any width, and the needle face parks its scale at
/// [`crate::widgets::smeter::NEEDLE_FACE_MAX_W`] and centres it — so the meter
/// is the strip's bottomless width absorber, and a row that carries it always
/// reaches the right edge.
const SMETER_W: f32 = 250.0;
/// What the diversity mode chip reads, by whether it is combining. Measured
/// and drawn from the same pair, so the box cannot be planned around a label
/// it does not carry.
const DIV_MODE_LABELS: [&str; 2] = ["CANCEL", "COMBINE"];

/// Width of the sub-receiver box at its design size.
const SUB_W: f32 = 404.0;
/// How much a box that grows by lengthening its slider rails or widening its
/// offset fields (RX, TX, SUB) may add to them.
const RAIL_STRETCH_MAX: f32 = 240.0;
/// How much the VFO/RIT box may add to its controls. Far short of
/// [`RAIL_STRETCH_MAX`], because unlike a rail neither of its rows has
/// anything to *do* with the width: past its label a chip is padding, and past
/// a signed 4-digit offset an Hz field is empty. Left uncapped it took the
/// whole of a row's slack — a 500 pt box of four huge chips over two 230 pt
/// fields reading "0 Hz", with the leftover it still couldn't absorb justified
/// into a gap beside it — while the row below overflowed onto a third.
const VFO_STRETCH_MAX: f32 = 72.0;
/// Length the strip's stretchable rails — the RX box's Vol and SQL, the TX
/// box's Drive and Tune — are *priced* at when a box is measured, and drawn at
/// when its row has nothing to spare.
///
/// Deliberately shorter than the style's `slider_width`: a box reserves its
/// widest row, and every point of that reservation is a point the packer has
/// to find before it can keep two boxes on one row. The rails are the one
/// thing on those rows that reads the same at any length, so they are what
/// gives — and they get it all straight back as the box's stretch, so nothing
/// changes on a row with room to spare. Reserving the full 84 pt each cost the
/// strip a whole row on the narrowest desktop window: RX, TX, Display and
/// System together came to 1425 pt of a 1377 pt pane and Display and System
/// spilled onto a third row, which costs the waterfall far more height than
/// two short rails cost the eye. See `the_desktop_strip_packs_a_cat_rig_into_two_rows`.
const STRIP_RAIL_W: f32 = 48.0;
/// How much a box that grows by widening its chips (Display, System) may be
/// stretched to, as a factor of its natural width.
const CHIP_STRETCH_FACTOR: f32 = 2.0;
/// The VFO/RIT box's utility chips — the labels its top row is measured and
/// drawn from.
const VFO_CHIPS: [&str; 4] = ["A↔B", "A→B", "SPLIT", "SUB"];
/// The repeater chips that follow them on the same row: the transmit shift and
/// the tone that goes out under it.
///
/// In this box rather than one of their own because they belong with the other
/// three things that decide where and how this radio transmits relative to
/// where it listens — split, RIT and XIT — and because the strip has no room
/// for another box. Both labels are fixed rather than showing the setting: a
/// chip whose label grew from "TONE" to "88.5" would change width and move
/// every chip beside it each time the operator picked a tone. What is set is in
/// the hover text and, at a glance, in whether the chip is lit.
const DUPLEX_CHIP: &str = "DUPLEX";
const TONE_CHIP: &str = "TONE";
/// Width of a RIT/XIT offset field on the desktop: a signed 4-digit offset
/// plus " Hz".
const HZ_FIELD_W: f32 = 74.0;
/// Horizontal gap between the controls inside a two-row box — the
/// `item_spacing` every condensed box sets, and therefore the gap its
/// measurement has to count.
const MODULE_ROW_SPACING: f32 = 5.0;
/// Width of the value readout riding beside the TX Drive and Tune sliders
/// ("100%" in a drag-value frame). Calibrated at the desktop tier, like the
/// RX box's row figures; `the_condensed_tx_box_fits_its_rows` keeps it honest.
const TX_SLIDER_VALUE_W: f32 = 48.0;
/// Width of the condensed TX box's mic column: the "Mic" label over a vertical
/// rail. Calibrated the same way, guarded by the same test.
const TX_MIC_COL_W: f32 = 30.0;
/// Width of the condensed TX box's transmit-audio column, which stands in the
/// mic column's place in the modes where the microphone is not the payload
/// (issue #186). Wider than [`TX_MIC_COL_W`] because its caption is its
/// readout — the level in dB, permanently on screen rather than behind a hover
/// — and "-40 dB" is what it has to fit: 30 pt of it at the desktop tier,
/// against the 18 the word "Mic" takes. Same calibration, same test.
const TX_LEVEL_COL_W: f32 = 36.0;
/// Width of the condensed TX box's envelope-processor column, which stands
/// beside the mic column in voice single sideband (issue #294). Wide enough
/// for the word "CESSB" over its rail — 30 pt of it at the desktop tier, the
/// caption being the name here rather than a readout — and calibrated the same
/// way as the two columns above, guarded by the same test.
const TX_CESSB_COL_W: f32 = 36.0;
/// Padding between the TX rows' readouts and the mic column, so the vertical
/// rail stands apart from the sliders beside it.
const TX_MIC_GAP: f32 = 16.0;
/// Width of the RX box's pinned dB rails — the front-end Gain slider and the
/// manual gain behind an AGC that is switched off. Narrower than the Vol and
/// SQL rails on purpose: these two carry a dB readout, and the box has to stay
/// inside one row. See [`db_rail_w`].
const RX_DB_RAIL_W: f32 = 76.0;
/// Text size of the band/mode chip's label.
const BAND_MODE_TEXT: f32 = 14.0;
/// How much taller the band/mode chip stands than a plain one. Band and mode
/// are what an operator changes most, and the chip opens the longest menu in
/// the program; standing taller — and lit in the palette's green rather than
/// the fill every other chip wears — is what makes it findable at a glance
/// among a strip of chips instead of one more of them. Green and not the
/// palette's pink: pink doubles as the error colour in most of the themes, and
/// a permanently red button on the bar reads as something being wrong.
const BAND_MODE_EXTRA_H: f32 = 10.0;
/// Below this the frequency digits stop reading as a dial, so the box sheds
/// something else rather than shrinking them further.
const MIN_DIGIT: f32 = 22.0;
/// Largest digit size the phone box uses, and its height. Digits big enough to
/// tune with a thumb, in a box that leaves the waterfall the screen.
const PHONE_DIGIT_MAX: f32 = 30.0;
const PHONE_FREQ_H: f32 = 42.0;
/// Height of the phone's S-meter box.
const PHONE_SMETER_H: f32 = 40.0;
/// How narrow the phone's S-meter may be squeezed to make room for the menu
/// chips beside it. Below this the scale has no room left to be read on.
const PHONE_SMETER_MIN_W: f32 = 80.0;
/// How wide it may grow. The needle's radius comes from the box width — its arc
/// is a chord across it — so a wider box means a *taller* arc, and past this it
/// would draw the ends of the scale below a 40 pt box. See
/// `the_phone_smeter_keeps_its_scale_inside_its_box`.
const PHONE_SMETER_MAX_W: f32 = 220.0;
/// Least ink-to-edge a stretched menu chip is allowed: below this the label
/// stops reading as being *in* the chip. A chip drawn to an exact size centres
/// its text and lets it hang over the edges rather than clipping it, so this is
/// what decides when the labels have to shrink instead. See [`plan_phone_tail`].
const MENU_TEXT_PAD: f32 = 5.0;
/// How small a stretched menu chip's label may be set. Four characters at this
/// size on a phone held at arm's length is the floor; a narrower screen than
/// that keeps the size and overhangs the cell, which reads better than a label
/// nobody can make out.
const MENU_TEXT_MIN: f32 = 11.0;
/// The compact strip's PTT: its label and text size. The spaces are padding —
/// it is the one chip on the row worth more of a target than its label needs.
const PTT_LABEL: &str = " PTT ";
const PTT_TEXT: f32 = 15.0;
/// What the compact PTT chip says on hover. Hover is a pointing device's
/// affordance — a finger never sees this — so it leads with the half that
/// only a mouse has. See [`SdroxideApp::held_ptt`].
const PTT_HOLD_HINT: &str = "Hold to transmit — a click latches it on, and the next press lets go";
/// Digit sizes of the single-row strip's readout. It is a type-in field — tap
/// and type, no per-digit targets — so the floor is about reading the
/// frequency, not hitting one digit of it.
const STRIP_DIGIT_MIN: f32 = 18.0;
const STRIP_DIGIT_MAX: f32 = 30.0;
/// Vertical gap between the strip's two button rows — and so, with two chip
/// heights, what sets the height of everything on the strip.
const STRIP_ROW_GAP: f32 = 6.0;
/// Text size of the strip's PTT label. The chip around it is padded wider
/// than any grid button and stands the strip's full height, because it is the
/// one control worth a whole thumb.
const STRIP_PTT_TEXT: f32 = 17.0;

/// What a digit size costs the frequency readout, and what size fits a width.
///
/// The readout is `digits` fixed-width digits ([`freq_display::DIGITS`]
/// normally, [`freq_display::DIGITS_EXT`] on a radio whose converter reaches
/// past 10 GHz — see [`SdroxideApp::readout_digits`]), three group separators
/// and a " Hz" tail, spaced 1 pt apart — so its width is linear in the digit
/// size, and one measurement of the live fonts gives the slope. Inverting
/// that is what lets one formula serve a 360 pt phone and a 2560 pt desktop.
struct ReadoutFit {
    /// Width per point of digit size.
    per_pt: f32,
    /// Height per point of digit size.
    h_per_pt: f32,
    /// The 1 pt gaps between the readout's pieces — `digits` digits, 3 dots
    /// and the " Hz" tail — added once rather than per point of size.
    gaps: f32,
}

impl ReadoutFit {
    /// Everything up to the group separators scales with the digit size, and
    /// `freq_display` draws " Hz" at 0.3x it, so one reference measurement of
    /// each glyph is enough.
    fn measure(ui: &egui::Ui, digits: u32) -> Self {
        const REF: f32 = 40.0;
        let w = |s: &str, f: egui::FontId| {
            ui.painter().layout_no_wrap(s.to_owned(), f, Color32::WHITE).size()
        };
        let digit = w("0", egui::FontId::monospace(REF));
        let dot = w(".", egui::FontId::monospace(REF)).x;
        let hz = w(" Hz", egui::FontId::proportional(REF)).x;
        Self {
            per_pt: (digits as f32 * digit.x + 3.0 * dot + 0.3 * hz) / REF,
            h_per_pt: digit.y / REF,
            gaps: (digits + 3) as f32,
        }
    }

    /// Width of the readout at `size`, including `freq_display`'s 1 pt spacing
    /// between its pieces.
    fn width(&self, size: f32) -> f32 {
        size * self.per_pt + self.gaps
    }

    fn height(&self, size: f32) -> f32 {
        size * self.h_per_pt
    }

    /// The largest digit size whose readout fits `budget`. Uncapped and
    /// unfloored — callers clamp to their own limits.
    fn fit(&self, budget: f32) -> f32 {
        (budget - self.gaps) / self.per_pt
    }
}

/// The frequency box's measured geometry — see
/// [`SdroxideApp::freq_box_plan`]: the digit size, the two side columns, the
/// readout between them, and the outer width the box reserves around it all.
struct FreqBoxPlan {
    size: f32,
    ab_w: f32,
    right_w: f32,
    readout_w: f32,
    readout_h: f32,
    box_w: f32,
}

/// The geometry of the short-screen single-row strip, planned before anything
/// is drawn.
///
/// Everything on the strip shares one height — two grid rows of chips — and
/// the width splits three ways: the VFO box hugs its readout, the PTT hugs its
/// label, and the button grid stretches over whatever is left, which is what
/// makes the buttons scale with the screen.
struct ShortStrip {
    /// Digit size the readout gets.
    digit: f32,
    /// The VFO box, outer width.
    box_w: f32,
    /// The one shared height: two chip rows and the gap between them.
    box_h: f32,
    /// The button grid's width, and the uniform cell width of each of its rows.
    grid_w: f32,
    cell1_w: f32,
    cell2_w: f32,
}

/// The measured widths and heights [`plan_short_strip`] works from — taken
/// off the live style when the strip is drawn, and given as plain numbers by
/// the layout tests.
struct StripChips {
    /// A grid chip's height.
    chip_h: f32,
    /// The active-VFO tag's width.
    tag_w: f32,
    /// The band/mode chip's width at its current label.
    bm_w: f32,
    /// The PTT's width; 0 for a rig that cannot transmit.
    ptt_w: f32,
    /// Each grid row: its cell count, and the width its widest label's chip
    /// measures on its own.
    row1: (usize, f32),
    row2: (usize, f32),
}

/// Plan the strip for `avail` points of row. A free function of measured
/// numbers so the arithmetic can be tested without an app around it. `gap`
/// separates the strip's three blocks, `cell_gap` the grid's cells.
fn plan_short_strip(
    avail: f32,
    fit: &ReadoutFit,
    c: &StripChips,
    gap: f32,
    cell_gap: f32,
) -> ShortStrip {
    let box_h = 2.0 * c.chip_h + STRIP_ROW_GAP;
    // Equal cells sized by the row's widest label, so stretching the rows to
    // the same width never squeezes one chip below its own text.
    let row_min = |(n, w): (usize, f32)| n as f32 * w + (n - 1) as f32 * cell_gap;
    let grid_min = row_min(c.row1).max(row_min(c.row2));
    let gaps = if c.ptt_w > 0.0 { 2.0 } else { 1.0 } * gap;
    // The digits get whatever the PTT and the grid at its minimum leave over —
    // box margins, the VFO tag and its gap already spoken for — with a few
    // points of slack so rounding never wraps the row.
    let overhead = 16.0 + c.tag_w + 6.0;
    let digit = fit
        .fit(avail - c.ptt_w - grid_min - gaps - overhead - 4.0)
        .clamp(STRIP_DIGIT_MIN, STRIP_DIGIT_MAX);
    // The box hugs the wider of its rows: the readout above, or the band/mode
    // chip plus the meter at its narrowest below.
    let inner = (c.tag_w + 6.0 + fit.width(digit)).max(c.bm_w + 6.0 + PHONE_SMETER_MIN_W);
    let box_w = inner + 16.0;
    // The buttons take every point the box and the PTT left on the row.
    let grid_w = (avail - box_w - c.ptt_w - gaps - 4.0).max(grid_min);
    let cell = |(n, _): (usize, f32)| (grid_w - (n - 1) as f32 * cell_gap) / n as f32;
    ShortStrip { digit, box_w, box_h, grid_w, cell1_w: cell(c.row1), cell2_w: cell(c.row2) }
}

/// Which menu a chip on a compact strip opens. One list drives both the
/// measuring and the drawing of the row (see [`SdroxideApp::menu_chips`]), so
/// the two cannot come to disagree about what is on it — a chip counted but
/// not drawn, or the reverse, is exactly what breaks a planned row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuChip {
    Rx,
    Vfo,
    Div,
    Sub,
    Rig,
    Tx,
    Disp,
    Sys,
}

impl MenuChip {
    fn label(self) -> &'static str {
        match self {
            Self::Rx => "RX",
            Self::Vfo => "VFO",
            Self::Div => "DIV",
            Self::Sub => "SUB",
            Self::Rig => "RIG",
            Self::Tx => "TX",
            Self::Disp => "DISP",
            Self::Sys => "SYS",
        }
    }
}

/// How the menu chips are drawn: hugging their labels, or stretched to a
/// uniform cell — the phone's own row of buttons, where the row is divided
/// between them instead of being left part empty.
#[derive(Clone, Copy)]
enum ChipFit {
    Hug,
    /// The cell every chip is drawn at, and the label size that fits it
    /// (`None` leaves the style's own size alone).
    Cell(egui::Vec2, Option<f32>),
}

/// The phone strip's tail — the S-meter and the menu chips — planned before
/// either is drawn.
///
/// The chips hug their labels beside the meter wherever the row holds all of
/// them. Where it does not they take a row of their own, stretched to divide
/// it: five buttons across the screen rather than four and a lonely fifth
/// under them. A phone gives the top bar three rows before the waterfall
/// starts paying for them, and a row carrying one chip costs the same height
/// as a row carrying six.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PhoneTail {
    /// The S-meter's width on the row it lands on.
    meter_w: f32,
    /// How much wider than its label each chip that stays beside the meter is
    /// drawn: what the meter could not take of its row — it has a ceiling —
    /// split between them, so the row still reaches the right edge.
    lead_extra: f32,
    /// The menu chips' own row, when they take one.
    grid: Option<MenuGrid>,
}

/// The menu chips' own row: one cell width for all of them, and the label size
/// that fits it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MenuGrid {
    /// The row divided by the chip count, gaps off.
    cell_w: f32,
    /// The size the labels shrink to where the widest of them will not fit a
    /// cell at the style's own; `None` leaves the style's size alone.
    text: Option<f32>,
}

/// The measured widths [`plan_phone_tail`] works from — taken off the live
/// style when the strip is drawn, and given as plain numbers by the layout
/// tests.
struct PhoneChips<'a> {
    /// The chips that stay with the meter — the band/mode chip where the
    /// frequency box could not keep it, and the PTT: how many, and what they
    /// take of the row together with the gap before each.
    lead: (usize, f32),
    /// What each menu chip measures on its own, gaps excluded.
    menu: &'a [f32],
    /// The widest menu label's *text* width at `text_size` — what a label that
    /// has to shrink into its cell is scaled from.
    text_w: f32,
    text_size: f32,
}

/// Plan the phone strip's tail for a row of `row` points with `left` of it
/// still free where the meter is about to be drawn. A free function of
/// measured numbers, like [`plan_short_strip`], so the arithmetic can be
/// tested without an app around it.
///
/// The meter is the elastic one — the bar and trace faces take any width — so
/// it is what gives, down to [`PHONE_SMETER_MIN_W`]. Past that squeezing it
/// buys nothing readable, and the chips are better off with a row of their
/// own than spilling one at a time onto one.
fn plan_phone_tail(row: f32, left: f32, c: &PhoneChips, gap: f32) -> PhoneTail {
    // A few points held back wherever a width is measured against the room for
    // it: one that comes out exactly equal to the space left rounds the wrong
    // way often enough, and the cost of being wrong is the stray row this
    // whole plan exists to save.
    const SLACK: f32 = 6.0;
    let menu_w: f32 = c.menu.iter().map(|w| w + gap).sum();
    let (n_lead, lead_w) = c.lead;
    let holds = |space: f32, beside: f32| space - beside - SLACK >= PHONE_SMETER_MIN_W;
    // Which row the meter lands on: this one, if what is left of it can hold
    // the meter at its narrowest and the chips that must stay beside it; else
    // the fresh row the wrapping layout is about to break onto.
    let space = if holds(left, lead_w) { left } else { row };
    // Do the menu chips fit there too, at the price of squeezing the meter?
    let one_row = holds(space, lead_w + menu_w);
    let beside = if one_row { lead_w + menu_w } else { lead_w };
    let meter_w = (space - beside - SLACK).clamp(PHONE_SMETER_MIN_W, PHONE_SMETER_MAX_W);
    let spare = (space - beside - SLACK - meter_w).max(0.0);
    let grid = (!one_row).then(|| {
        let n = c.menu.len().max(1);
        // The row, divided — but never past twice the widest chip's own width:
        // beyond that the stretch stops reading as a button and starts reading
        // as a mistake, and a phone in landscape has width to burn. The
        // desktop's chip boxes are held to the same factor.
        let widest = c.menu.iter().fold(0.0f32, |a, &b| a.max(b));
        let cell_w = ((row - (n - 1) as f32 * gap) / n as f32).min(CHIP_STRETCH_FACTOR * widest);
        // Cells this size are usually *wider* than the labels asked for — the
        // point is to spend the row rather than waste it. Only where the row
        // cannot afford even that does the text give way.
        let room = cell_w - 2.0 * MENU_TEXT_PAD;
        let text = (c.text_w > room)
            .then(|| (c.text_size * room / c.text_w).max(MENU_TEXT_MIN).min(c.text_size));
        MenuGrid { cell_w, text }
    });
    PhoneTail { meter_w, lead_extra: if n_lead > 0 { spare / n_lead as f32 } else { 0.0 }, grid }
}

/// A box the desktop packer places: its natural (minimum) outer width, how
/// much of a row's slack it absorbs relative to its neighbours, and the widest
/// it may be stretched to before the growth stops looking like anything.
#[derive(Clone, Copy)]
struct StripBox {
    w: f32,
    flex: f32,
    max_w: f32,
}

/// One planned row of the desktop strip: which boxes landed on it (a
/// contiguous range of the input — the packer never reorders), the width each
/// is drawn at, and any gap justified between neighbours once every box on the
/// row has been stretched to its cap.
struct PlannedRow {
    boxes: std::ops::Range<usize>,
    widths: Vec<f32>,
    extra_gap: f32,
}

struct StripPlan {
    rows: Vec<PlannedRow>,
}

/// The rows greedy first-fit needs for `boxes` in order — the fewest possible
/// when the order is fixed. A lone box "fits" its row by definition, however
/// wide, so the count is always achievable.
fn rows_needed(avail: f32, gap: f32, boxes: &[StripBox]) -> usize {
    let mut rows = 1;
    let mut run = boxes[0].w;
    for b in &boxes[1..] {
        if run + gap + b.w <= avail {
            run += gap + b.w;
        } else {
            rows += 1;
            run = b.w;
        }
    }
    rows
}

/// Pack `boxes` — in the order given — into rows of `avail` points with `gap`
/// between neighbours. A free function of measured numbers, like
/// [`plan_short_strip`], so the arithmetic can be tested without an app.
///
/// Fewest rows first, because a row costs the waterfall its height. Among the
/// break points that manage that count, the ones whose leftover can actually
/// be *absorbed* — a row's spare width is only worth having on a row whose
/// boxes can stretch over it, so unabsorbable spare is minimised first and
/// natural balance breaks the ties. Then each row's leftover is fed to its
/// flexible boxes, so the row meets the right edge instead of stopping short
/// of it.
///
/// A box wider than `avail` gets a row of its own and overflows it, exactly as
/// the old wrapping layout would have — reachable only under a forced Desktop
/// layout on a window the desktop tier would never choose itself.
fn plan_top_strip(avail: f32, gap: f32, boxes: &[StripBox]) -> StripPlan {
    let n = boxes.len();
    if n == 0 {
        return StripPlan { rows: Vec::new() };
    }
    let run_w = |r: &std::ops::Range<usize>| {
        boxes[r.clone()].iter().map(|b| b.w).sum::<f32>() + gap * (r.len() - 1) as f32
    };
    let fits = |r: &std::ops::Range<usize>| r.len() == 1 || run_w(r) <= avail;
    let rows = rows_needed(avail, gap, boxes);

    // Every way of placing `rows - 1` breaks among the `n - 1` seams. With at
    // most eight boxes this is at most 128 masks — not worth a cleverer
    // algorithm. Scored per row by how much leftover would survive every box
    // stretching to its cap (squared), then by the raw leftover (squared) as
    // the tiebreak — an infinite cap (the S-meter) swallows any leftover, and
    // `max(0.0, -inf)` keeps the arithmetic finite.
    let mut best: Option<((f32, f32), Vec<std::ops::Range<usize>>)> = None;
    for mask in 0u32..(1 << (n - 1)) {
        if mask.count_ones() as usize != rows - 1 {
            continue;
        }
        let mut runs = Vec::with_capacity(rows);
        let mut start = 0;
        for seam in 0..n - 1 {
            if mask & (1 << seam) != 0 {
                runs.push(start..seam + 1);
                start = seam + 1;
            }
        }
        runs.push(start..n);
        if !runs.iter().all(&fits) {
            continue;
        }
        let (mut dead, mut ragged) = (0.0f32, 0.0f32);
        for r in &runs {
            let leftover = (avail - run_w(r)).max(0.0);
            let capacity: f32 = boxes[r.clone()].iter().map(|b| b.max_w - b.w).sum();
            dead += (leftover - capacity).max(0.0).powi(2);
            ragged += leftover.powi(2);
        }
        if best.as_ref().is_none_or(|(s, _)| (dead, ragged) < *s) {
            best = Some(((dead, ragged), runs));
        }
    }
    let (_, runs) = best.expect("greedy's own partition is always a candidate");

    // Stretch each row edge-to-edge: its leftover goes to the flexible boxes
    // in proportion to their flex, water-filling against each box's cap — a
    // box that hits its cap hands the rest back for the others. Whatever no
    // box can take is justified between them instead. A couple of points are
    // held back so rounding never pushes the last box past the edge.
    let target = avail - 2.0;
    let rows = runs
        .into_iter()
        .map(|r| {
            let idx: Vec<usize> = r.clone().collect();
            let mut widths: Vec<f32> = idx.iter().map(|&i| boxes[i].w).collect();
            let mut leftover = (target - run_w(&r)).max(0.0);
            loop {
                let growable: Vec<usize> = (0..idx.len())
                    .filter(|&j| boxes[idx[j]].flex > 0.0 && widths[j] < boxes[idx[j]].max_w - 0.5)
                    .collect();
                let flex: f32 = growable.iter().map(|&j| boxes[idx[j]].flex).sum();
                if leftover <= 0.5 || flex <= 0.0 {
                    break;
                }
                let pool = leftover;
                for &j in &growable {
                    let share = pool * boxes[idx[j]].flex / flex;
                    let grown = (widths[j] + share).min(boxes[idx[j]].max_w);
                    leftover -= grown - widths[j];
                    widths[j] = grown;
                }
            }
            let extra_gap =
                if r.len() > 1 { leftover.max(0.0) / (r.len() - 1) as f32 } else { 0.0 };
            PlannedRow { boxes: r, widths, extra_gap }
        })
        .collect();
    StripPlan { rows }
}

/// The widest frequency box that still packs the strip into as few rows as
/// the narrowest one would — how the desktop trades digit size for a whole
/// row of strip, and for nothing less. `rest` is every box after the
/// frequency box, in order; the result is `design_w` untouched whenever
/// shrinking wouldn't save a row.
///
/// Greedy row count is monotone in a box's width, so the boundary is found by
/// bisection; the half point stepped back off it keeps the result clear of
/// the exact-fit edge that `plan_top_strip` measures with `<=`.
fn freq_w_for_fewest_rows(
    avail: f32,
    gap: f32,
    design_w: f32,
    min_w: f32,
    rest: &[StripBox],
) -> f32 {
    let rows_with = |w: f32| {
        let mut boxes = Vec::with_capacity(rest.len() + 1);
        boxes.push(StripBox { w, flex: 0.0, max_w: w });
        boxes.extend_from_slice(rest);
        rows_needed(avail, gap, &boxes)
    };
    let fewest = rows_with(min_w);
    if min_w >= design_w || rows_with(design_w) <= fewest {
        return design_w;
    }
    let (mut lo, mut hi) = (min_w, design_w);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if rows_with(mid) <= fewest {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo - 0.5).max(min_w)
}

impl SdroxideApp {
    pub(in crate::app) fn top_bar(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        let tier = crate::layout::tier(ui.ctx());
        // The desktop plans its own rows; only the compact strips still lean
        // on a wrapping layout.
        if !tier.compact() {
            self.desktop_strip(ui, cmds);
            return;
        }
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true), |ui| {
            // A tablet-tier window too *short* for even the two stacked
            // compact rows — a 1280x720 panel — gets the single-row strip:
            // everything beside everything, and the height goes to the
            // waterfall. Taller tablet windows keep the stacked layout below,
            // with its full-size readout.
            if crate::layout::short_tablet(ui.ctx()) {
                self.short_strip(ui, cmds);
                return;
            }
            let band_mode_shown = self.freq_module(ui, cmds, tier);
            // The boxes come as menus here: a module reserves its width before
            // it draws, so on a window this narrow it would wrap whole or
            // overflow, and never shrink.
            //
            // The meter is the one thing here that can be any width, so it
            // is what gives: the phone measures the chips that follow it and
            // hands it the rest of the row, and where they cannot all fit
            // beside it they take a stretched row of their own rather than
            // spill one at a time onto one. See [`plan_phone_tail`].
            if tier == crate::layout::Tier::Phone {
                let tail = self.phone_tail(ui, band_mode_shown);
                self.smeter_box(ui, tail.meter_w, PHONE_SMETER_H, true);
                self.menu_bar(ui, cmds, tier, band_mode_shown, Some(tail));
            } else {
                self.smeter_box(ui, SMETER_W, crate::chrome::MODULE_TALL_H, false);
                self.menu_bar(ui, cmds, tier, band_mode_shown, None);
            }
        });
    }

    /// The desktop strip: measure every box, plan balanced rows, then draw
    /// each row stretched to the right edge. See [`plan_top_strip`].
    ///
    /// Widths are measured against the live style (fonts and paddings change
    /// with the tier) and against state — the RX box grows an NFM tone chip,
    /// the TX box a voice-keyer button — so the plan is remade each frame, and
    /// toggling one of those can re-break the rows. The old wrapping layout
    /// reflowed on the same toggles.
    fn desktop_strip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let tier = crate::layout::tier(ui.ctx());
        // Pane-local, not viewport: in split view each radio packs its own
        // strip against its own pane.
        let avail = ui.available_width();
        let gap = ui.spacing().item_spacing.x;
        let fit = ReadoutFit::measure(ui, self.readout_digits());
        // The readout keeps its design size wherever the row can hold it, and
        // shrinks against the full row where it cannot (a forced Desktop
        // layout on a narrow pane) — a box the packer cannot break up any
        // further. The few points of `beside` keep rounding from pushing a
        // full-row box past the edge.
        let mut freq = self.freq_box_plan(ui, &fit, avail, 4.0, tier);

        #[derive(Clone, Copy)]
        enum Kind {
            Freq,
            Smeter,
            VfoRit,
            RxFilter,
            Div,
            Sub,
            Rig,
            Tx,
            Display,
            System,
        }
        let mut boxes: Vec<(Kind, StripBox)> = vec![
            (Kind::Freq, StripBox { w: freq.box_w, flex: 0.0, max_w: freq.box_w }),
            (Kind::Smeter, StripBox { w: SMETER_W, flex: 3.0, max_w: f32::INFINITY }),
            (Kind::VfoRit, {
                let w = self.vfo_rows_w(ui);
                StripBox { w, flex: 1.0, max_w: w + VFO_STRETCH_MAX }
            }),
            (Kind::RxFilter, {
                let w = self.rx_filter_w(ui);
                StripBox { w, flex: 2.0, max_w: w + RAIL_STRETCH_MAX }
            }),
        ];
        // Only while two aerials are actually being combined — the same rule
        // as the sub receiver below, and for the same two reasons: the box
        // appearing is the confirmation that the filter is running, and a
        // strip is too narrow to carry controls for hardware nobody has.
        if self.has_diversity() {
            let w = div_rows_w(ui);
            boxes.push((Kind::Div, StripBox { w, flex: 1.0, max_w: w + RAIL_STRETCH_MAX }));
        }
        // Only while the sub is running: the module appearing is itself the
        // confirmation that SUB took effect, and it costs strip width that
        // operators who never use it should not have to pay.
        if self.state.sub_rx_enabled {
            boxes.push((
                Kind::Sub,
                StripBox { w: SUB_W, flex: 1.0, max_w: SUB_W + RAIL_STRETCH_MAX },
            ));
        }
        // The transceiver's own switches — the aerial socket and the mains
        // switch — for a radio that has either. On the strip because an
        // operator changes bands and reaches for the other aerial in the same
        // breath, and both were reachable only through Settings (issue #258).
        if self.rig_box_shown() {
            let w = self.rig_rows_w(ui);
            boxes.push((Kind::Rig, StripBox { w, flex: 1.0, max_w: w * CHIP_STRETCH_FACTOR }));
        }
        if self.tx_capable() {
            let w = self.tx_rows_w(ui);
            boxes.push((Kind::Tx, StripBox { w, flex: 2.0, max_w: w + RAIL_STRETCH_MAX }));
        }
        let w = self.display_rows_w(ui);
        boxes.push((Kind::Display, StripBox { w, flex: 1.0, max_w: w * CHIP_STRETCH_FACTOR }));
        let w = system_rows_w(ui);
        boxes.push((Kind::System, StripBox { w, flex: 1.0, max_w: w * CHIP_STRETCH_FACTOR }));

        // A whole row is worth more than digit size: when the strip packs into
        // fewer rows with the readout at its floor, shrink it just enough to
        // get there. The saved row goes to the waterfall, and the rows that
        // remain come out full rather than one of them nearly empty.
        let overhead = freq.box_w - freq.readout_w;
        let min_w = overhead + fit.width(MIN_DIGIT);
        let rest: Vec<StripBox> = boxes[1..].iter().map(|(_, b)| *b).collect();
        let freq_w = freq_w_for_fewest_rows(avail, gap, freq.box_w, min_w, &rest);
        if freq_w < freq.box_w {
            freq = self.freq_box_plan(ui, &fit, freq_w, 0.0, tier);
            boxes[0].1 = StripBox { w: freq.box_w, flex: 0.0, max_w: freq.box_w };
        }

        let plan = plan_top_strip(avail, gap, &boxes.iter().map(|(_, b)| *b).collect::<Vec<_>>());
        for row in &plan.rows {
            ui.horizontal(|ui| {
                for (j, i) in row.boxes.clone().enumerate() {
                    if j > 0 && row.extra_gap > 0.0 {
                        ui.add_space(row.extra_gap);
                    }
                    let w = row.widths[j];
                    match boxes[i].0 {
                        Kind::Freq => self.freq_module_at(ui, cmds, &freq),
                        Kind::Smeter => self.smeter_box(ui, w, crate::chrome::MODULE_TALL_H, false),
                        Kind::VfoRit => self.vfo_rit_module(ui, cmds, w),
                        Kind::RxFilter => self.rx_filter_module(ui, cmds, w),
                        Kind::Div => self.div_module(ui, cmds, w),
                        Kind::Sub => self.sub_rx_module(ui, cmds, w),
                        Kind::Rig => self.rig_module(ui, cmds, w),
                        Kind::Tx => self.tx_condensed(ui, cmds, w),
                        Kind::Display => self.display_condensed(ui, cmds, w),
                        Kind::System => self.windows_condensed(ui, w),
                    }
                }
            });
        }
    }

    /// The menu chips the compact strips carry, in the order they are drawn.
    /// The one list the row is measured from and drawn with, so a chip counted
    /// but not drawn — or the reverse — cannot break the plan around it.
    fn menu_chips(&self, tx_capable: bool) -> Vec<MenuChip> {
        let mut chips = vec![MenuChip::Rx, MenuChip::Vfo];
        // Both of these appear only while what they drive is running: the chip
        // appearing is itself the confirmation.
        if self.has_diversity() {
            chips.push(MenuChip::Div);
        }
        if self.state.sub_rx_enabled {
            chips.push(MenuChip::Sub);
        }
        // Same rule again: only a radio that has an aerial selector or a
        // control-link power switch carries the chip for them.
        if self.rig_box_shown() {
            chips.push(MenuChip::Rig);
        }
        if tx_capable {
            chips.push(MenuChip::Tx);
        }
        chips.extend([MenuChip::Disp, MenuChip::Sys]);
        chips
    }

    /// Whether a menu chip is drawn lit: the state its menu holds, showing
    /// through the closed chip.
    fn menu_chip_lit(&self, chip: MenuChip) -> bool {
        match chip {
            MenuChip::Vfo => self.state.split,
            // Lit while the pair is being combined rather than cancelled: the
            // difference the chip is there to show at a glance.
            MenuChip::Div => self.radio_cfg.as_ref().is_some_and(|c| match c.backend {
                sdroxide_types::Backend::Lime => c.lime.aux.mode == DiversityMode::Combine,
                sdroxide_types::Backend::SdrPlay => c.sdrplay.duo.mode == DiversityMode::Combine,
                _ => false,
            }),
            MenuChip::Sub => true,
            MenuChip::Tx => self.state.tx.tune,
            // Nothing here reads back: the socket is a name rather than an
            // on/off, and a radio that is switched off answers nothing at all.
            MenuChip::Rig | MenuChip::Rx | MenuChip::Disp | MenuChip::Sys => false,
        }
    }

    /// Measure the phone strip's chips against the live style and plan the
    /// meter and the menu row around them — see [`plan_phone_tail`].
    ///
    /// Call this where the meter is about to be drawn and nowhere else: the
    /// plan turns on how much of the current row is still free, and only the
    /// cursor knows that.
    fn phone_tail(&self, ui: &egui::Ui, band_mode_shown: bool) -> PhoneTail {
        let tx_capable = self.tx_capable();
        let gap = ui.spacing().item_spacing.x;
        let mut lead = (0usize, 0.0f32);
        let mut add_lead = |w: f32| {
            lead.0 += 1;
            lead.1 += w + gap;
        };
        if !band_mode_shown {
            add_lead(crate::chrome::chip_width(ui, &self.band_mode_label(), Some(BAND_MODE_TEXT)));
        }
        if tx_capable {
            add_lead(crate::chrome::chip_width(ui, PTT_LABEL, Some(PTT_TEXT)));
        }
        let font = egui::TextStyle::Button.resolve(ui.style());
        let (mut menu, mut text_w) = (Vec::new(), 0.0f32);
        for chip in self.menu_chips(tx_capable) {
            menu.push(crate::chrome::chip_width(ui, chip.label(), None));
            text_w = text_w.max(crate::chrome::text_width(ui, chip.label(), font.clone()));
        }
        // Not `available_width()`: in a wrapping layout that reports the width
        // of the row this item would wrap *onto*, which is the full row
        // however much of the current one is already spoken for. The cursor is
        // what knows — in landscape the frequency box has taken part of it.
        let row = ui.max_rect().width();
        let left = row - (ui.cursor().min.x - ui.max_rect().min.x).max(0.0);
        let chips = PhoneChips { lead, menu: &menu, text_w, text_size: font.size };
        plan_phone_tail(row, left, &chips, gap)
    }

    /// The strip a short tablet-tier window wears: everything on one row.
    ///
    /// The VFO box — a type-in readout over an S-meter bar — then PTT at
    /// thumb size, then the menu buttons in two rows (RX and VFO above; TX,
    /// DISP and SYS below) stretched to the edge of the screen. On a screen
    /// under [`crate::layout::SHORT_H`] it stands in for the stacked tablet
    /// rows — a full-width frequency box above the meter and the menu chips —
    /// which would cost a 720 pt screen a quarter of its height before the
    /// waterfall got any.
    fn short_strip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let tx_capable = self.tx_capable();
        let sub = self.state.sub_rx_enabled;
        let div = self.has_diversity();
        let rig = self.rig_box_shown();
        let gap = ui.spacing().item_spacing.x;
        let chip_h = crate::chrome::chip_height(ui, None);
        let fit = ReadoutFit::measure(ui, self.readout_digits());

        let active = self.state.active_vfo;
        let tag = match active {
            Vfo::A => "A",
            Vfo::B => "B",
        };
        let tag_w = crate::chrome::text_width(ui, tag, egui::FontId::proportional(13.0));
        let bm_w = crate::chrome::chip_width(ui, &self.band_mode_label(), Some(BAND_MODE_TEXT));
        let ptt_w = if tx_capable {
            crate::chrome::chip_width(ui, "PTT", Some(STRIP_PTT_TEXT)) + 14.0
        } else {
            0.0
        };
        let widest = |labels: &[&str]| {
            labels.iter().map(|l| crate::chrome::chip_width(ui, l, None)).fold(0.0, f32::max)
        };
        let chips = StripChips {
            chip_h,
            tag_w,
            bm_w,
            ptt_w,
            row1: {
                // The same list the row below is drawn from, in the same
                // order: a chip counted but not drawn — or the reverse —
                // breaks the plan around it.
                let mut r1 = vec!["RX", "VFO"];
                if div {
                    r1.push("DIV");
                }
                if sub {
                    r1.push("SUB");
                }
                if rig {
                    r1.push("RIG");
                }
                (r1.len(), widest(&r1))
            },
            row2: if tx_capable {
                (3, widest(&["TX", "DISP", "SYS"]))
            } else {
                (2, widest(&["DISP", "SYS"]))
            },
        };
        let plan = plan_short_strip(ui.available_width(), &fit, &chips, gap, gap);

        // The VFO box. The A/B selector and the other VFO's frequency are in
        // the VFO menu (see [`Self::vfo_menu`]); the box shows which VFO the
        // dial is, the dial itself, and — under it — the band/mode chip beside
        // the meter.
        crate::chrome::module_bare_h(ui, plan.box_w, plan.box_h, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 3.0);
                ui.horizontal(|ui| {
                    let (shown, ink, offset) = self.readout();
                    ui.label(
                        RichText::new(tag)
                            .size(13.0)
                            .strong()
                            .color(ink.unwrap_or_else(crate::theme::CYAN)),
                    );
                    if let Some(hz) = freq_display::show_typed(
                        ui,
                        crate::layout::salted_id(ui.ctx(), "main-freq"),
                        shown,
                        plan.digit,
                        ink,
                        self.readout_digits(),
                    ) {
                        cmds.push(Command::SetVfo { vfo: active, hz: hz - offset });
                    }
                });
                let meter_h = ui.available_height();
                ui.horizontal(|ui| {
                    self.band_mode_button(ui, cmds);
                    // The meter takes whatever width the chip left and
                    // whatever height the readout did. Bar or trace only — a
                    // strip this shape cannot hold the needle's arc, see
                    // [`smeter::SmeterStyle::compact`].
                    let style = self.ui_settings.smeter_style;
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), meter_h),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            let resp = smeter::show(ui, self.meters.as_ref(), style.compact())
                                .on_hover_text("Click to cycle meter face: bar / trace");
                            if resp.clicked() {
                                self.set_smeter_style(style.next_compact());
                            }
                        },
                    );
                });
            });
        });

        if tx_capable {
            let resp = crate::chrome::chip_hold_sized(
                ui,
                self.state.tx.ptt,
                RichText::new("PTT").size(STRIP_PTT_TEXT).strong(),
                crate::theme::ALERT(),
                Color32::WHITE,
                egui::vec2(ptt_w, plan.box_h),
            )
            .on_hover_text(PTT_HOLD_HINT);
            self.apply_held_ptt(&resp, cmds);
        }

        // The menu buttons, stretched over the rest of the row.
        ui.allocate_ui_with_layout(
            egui::vec2(plan.grid_w, plan.box_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(gap, STRIP_ROW_GAP);
                let cell1 = egui::vec2(plan.cell1_w, chip_h);
                ui.horizontal(|ui| {
                    let btn = crate::chrome::chip_sized(ui, false, "RX", cell1);
                    self.rx_menu(ui, btn, cmds);
                    let btn = crate::chrome::chip_sized(ui, self.state.split, "VFO", cell1);
                    self.vfo_menu(ui, btn, cmds, true);
                    if div {
                        let lit = self.menu_chip_lit(MenuChip::Div);
                        let btn = crate::chrome::chip_sized(ui, lit, "DIV", cell1);
                        self.div_menu(ui, btn, cmds);
                    }
                    if sub {
                        let btn = crate::chrome::chip_sized(ui, true, "SUB", cell1);
                        self.sub_menu(ui, btn, cmds);
                    }
                    if rig {
                        let btn = crate::chrome::chip_sized(ui, false, "RIG", cell1);
                        self.rig_menu(ui, btn, cmds);
                    }
                });
                let cell2 = egui::vec2(plan.cell2_w, chip_h);
                ui.horizontal(|ui| {
                    if tx_capable {
                        let btn = crate::chrome::chip_sized(ui, self.state.tx.tune, "TX", cell2);
                        self.tx_menu(ui, btn, cmds);
                    }
                    let btn = crate::chrome::chip_sized(ui, false, "DISP", cell2);
                    self.disp_menu(ui, btn, cmds);
                    let btn = crate::chrome::chip_sized(ui, false, "SYS", cell2);
                    self.sys_menu(ui, btn, cmds);
                });
            },
        );
    }

    /// The compact control strip: PTT under a thumb, and one menu chip per
    /// control box the layout gave up.
    ///
    /// `band_mode_shown` says whether the frequency box could afford the
    /// band/mode chip; when it could not, this row carries it instead. `tail`
    /// is the phone's plan for the row — how much wider than their labels the
    /// chips beside the meter are drawn, and whether the menu chips take a
    /// stretched row of their own; the tablet passes `None` and every chip
    /// hugs its label.
    fn menu_bar(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        tier: crate::layout::Tier,
        band_mode_shown: bool,
        tail: Option<PhoneTail>,
    ) {
        let tx_capable = self.tx_capable();
        let extra = tail.map_or(0.0, |t| t.lead_extra);
        if !band_mode_shown {
            self.band_mode_chip(ui, cmds, extra);
        }
        if tx_capable {
            self.held_ptt(ui, cmds, extra);
        }
        let chips = self.menu_chips(tx_capable);
        let Some(grid) = tail.and_then(|t| t.grid) else {
            self.menu_chip_row(ui, cmds, tier, &chips, ChipFit::Hug);
            return;
        };
        // A row of their own, taken whole: a wrapping layout gives an item
        // wider than the space left a fresh row, so allocating the width the
        // cells add up to is what breaks the row — and, once they are on it,
        // what stops the layout breaking it again under them.
        let h = crate::chrome::chip_height(ui, None);
        let n = chips.len() as f32;
        let w = n * grid.cell_w + (n - 1.0) * ui.spacing().item_spacing.x;
        ui.allocate_ui_with_layout(
            egui::vec2(w, h),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                let fit = ChipFit::Cell(egui::vec2(grid.cell_w, h), grid.text);
                self.menu_chip_row(ui, cmds, tier, &chips, fit);
            },
        );
    }

    /// Draw the menu chips, each dressed with the menu it opens. `fit` decides
    /// whether they hug their labels or divide a row between them.
    fn menu_chip_row(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        tier: crate::layout::Tier,
        chips: &[MenuChip],
        fit: ChipFit,
    ) {
        for &chip in chips {
            let lit = self.menu_chip_lit(chip);
            let btn = match fit {
                ChipFit::Hug => crate::chrome::chip(ui, lit, chip.label()),
                ChipFit::Cell(size, text) => {
                    let label = RichText::new(chip.label());
                    let label = match text {
                        Some(pt) => label.size(pt),
                        None => label,
                    };
                    crate::chrome::chip_sized(ui, lit, label, size)
                }
            };
            match chip {
                MenuChip::Rx => self.rx_menu(ui, btn, cmds),
                // The tablet's full frequency box already carries the A/B
                // selector and the other VFO's frequency; the phone box shows
                // only a tag, so its VFO menu carries them instead.
                MenuChip::Vfo => self.vfo_menu(ui, btn, cmds, tier == crate::layout::Tier::Phone),
                MenuChip::Div => self.div_menu(ui, btn, cmds),
                MenuChip::Sub => self.sub_menu(ui, btn, cmds),
                MenuChip::Rig => self.rig_menu(ui, btn, cmds),
                MenuChip::Tx => self.tx_menu(ui, btn, cmds),
                MenuChip::Disp => self.disp_menu(ui, btn, cmds),
                MenuChip::Sys => self.sys_menu(ui, btn, cmds),
            }
        }
    }

    /// The RX menu: the receiver and filter/noise controls. Takes the chip it
    /// hangs off — the phone's hugging chip or the tablet's stretched one —
    /// and dresses it with its hover text, so the two strips cannot drift.
    fn rx_menu(&mut self, ui: &mut egui::Ui, btn: egui::Response, cmds: &mut Vec<Command>) {
        let btn = btn.on_hover_text(
            "Volume, gain, AGC, squelch, the noise controls, and the RDS readout on FM \
             broadcast",
        );
        crate::chrome::menu_popup(ui, &btn, |ui| {
            crate::chrome::menu_caption(ui, "Receiver");
            self.rx_controls(ui, cmds, true);
        });
    }

    /// The VFO menu: the utility chips and the RIT/XIT offsets.
    ///
    /// With `selector`, the A/B chips and the other VFO's frequency lead it —
    /// for the layouts whose frequency box shows only which VFO is being
    /// tuned (the phone box, the short strip). The tablet's full box already
    /// carries both, and would show them twice.
    fn vfo_menu(
        &mut self,
        ui: &mut egui::Ui,
        btn: egui::Response,
        cmds: &mut Vec<Command>,
        selector: bool,
    ) {
        let btn = btn.on_hover_text("VFO A/B, split, and the RIT/XIT offsets");
        crate::chrome::menu_popup(ui, &btn, |ui| {
            self.link_menu_row(ui);
            if selector {
                crate::chrome::menu_caption(ui, "VFO");
                let active = self.state.active_vfo;
                ui.horizontal(|ui| {
                    vfo_ab_chips(ui, active, cmds);
                    ui.label(
                        RichText::new(self.inactive_vfo_label())
                            .monospace()
                            .size(12.0)
                            .color(crate::theme::gray(120)),
                    );
                });
            }
            crate::chrome::menu_caption(ui, "Tuning");
            self.vfo_controls(ui, cmds, true);
        });
    }

    /// The DIV menu, shown only while two aerials are being combined.
    fn div_menu(&mut self, ui: &mut egui::Ui, btn: egui::Response, cmds: &mut Vec<Command>) {
        let btn = btn.on_hover_text(
            "The diversity filter: which way it combines the two aerials, how fast it \
             adapts, and holding it where it is",
        );
        crate::chrome::menu_popup(ui, &btn, |ui| {
            crate::chrome::menu_caption(ui, "Diversity");
            self.div_controls(ui, cmds, true);
        });
    }

    /// The SUB menu, shown only while the second receiver runs.
    fn sub_menu(&mut self, ui: &mut egui::Ui, btn: egui::Response, cmds: &mut Vec<Command>) {
        let btn = btn.on_hover_text("The second receiver's frequency, mode, filter and level");
        crate::chrome::menu_popup(ui, &btn, |ui| {
            crate::chrome::menu_caption(ui, "Sub receiver");
            self.sub_controls(ui, cmds, true, 0.0);
        });
    }

    /// The RIG menu, shown only on a radio with an aerial selector or a power
    /// switch this end can reach.
    fn rig_menu(&mut self, ui: &mut egui::Ui, btn: egui::Response, cmds: &mut Vec<Command>) {
        let btn = btn.on_hover_text("The transceiver's own aerial socket, and its power switch");
        crate::chrome::menu_popup(ui, &btn, |ui| {
            crate::chrome::menu_caption(ui, "Radio");
            self.rig_controls(ui, cmds, true);
        });
    }

    /// The TX menu: tune, the voice keyer, and the drive and mic levels.
    fn tx_menu(&mut self, ui: &mut egui::Ui, btn: egui::Response, cmds: &mut Vec<Command>) {
        let btn = btn.on_hover_text("Tune, the voice keyer, and the drive and mic levels");
        crate::chrome::menu_popup(ui, &btn, |ui| {
            crate::chrome::menu_caption(ui, "Transmit");
            // PTT is on the strip already; TUNE rides with the levels it is
            // set up with.
            self.tx_controls(ui, cmds, true);
            if crate::chrome::chip_accent(
                ui,
                self.state.tx.tune,
                RichText::new(" TUNE ").size(15.0),
                crate::theme::YELLOW(),
                crate::theme::INK_ON_CYAN(),
            )
            .clicked()
            {
                cmds.push(Command::SetTune(!self.state.tx.tune));
            }
        });
    }

    /// The DISP menu: waterfall, spectrum and skimmer controls.
    fn disp_menu(&mut self, ui: &mut egui::Ui, btn: egui::Response, cmds: &mut Vec<Command>) {
        let btn = btn.on_hover_text(
            "The panadapter — its two layers, their speeds and detail, peak hold — plus \
             waterfall contrast, FFT size and the skimmers",
        );
        crate::chrome::menu_popup(ui, &btn, |ui| {
            crate::chrome::menu_caption(ui, "Display");
            self.display_controls(ui, cmds, true);
        });
    }

    /// The SYS menu: the window buttons.
    fn sys_menu(&mut self, ui: &mut egui::Ui, btn: egui::Response, _cmds: &mut Vec<Command>) {
        let btn = btn.on_hover_text(
            "Logbook, spots, awards, memories, the scanner, settings and the manual",
        );
        crate::chrome::menu_popup(ui, &btn, |ui| {
            crate::chrome::menu_caption(ui, "System");
            self.windows_controls(ui, true);
        });
    }

    /// PTT on a compact layout: pressed to talk, clicked to latch.
    ///
    /// A press keys the transmitter either way, and what happens when it is let
    /// go depends on what did the letting go. A **finger** always unkeys:
    /// a latching chip an inch from a pannable waterfall is one mis-tap away
    /// from a transmitter left on with nobody watching, and letting go always
    /// dropping it is also what covers the browser taking the touch away
    /// because the tab went to the background — which arrives here as the
    /// pointer simply no longer being down on the chip. A **mouse click**
    /// latches it on, and the next press lets go, exactly as the desktop
    /// strip's chip does.
    ///
    /// The mouse half is not a softening of the rule above; it is the rule
    /// applied to the thing it was aimed at. The compact tiers are not only
    /// touchscreens — `Auto` picks Tablet for any window under 1400 pt wide,
    /// and an operator may force one outright — so hold-to-talk keyed off the
    /// tier alone took the latching PTT away from a mouse on a 1280-wide
    /// desktop that the same mouse has at 1440, and left no way at all to hold
    /// a long over without holding the button down for it.
    ///
    /// `extra` widens it past its label — its share of what the S-meter beside
    /// it could not take of the row.
    fn held_ptt(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, extra: f32) {
        let label = RichText::new(PTT_LABEL).size(PTT_TEXT).strong();
        let (fill, ink) = (crate::theme::ALERT(), Color32::WHITE);
        let resp = if extra > 0.5 {
            let w = crate::chrome::chip_width(ui, PTT_LABEL, Some(PTT_TEXT)) + extra;
            let h = crate::chrome::chip_height(ui, Some(PTT_TEXT));
            let size = egui::vec2(w, h);
            crate::chrome::chip_hold_sized(ui, self.state.tx.ptt, label, fill, ink, size)
        } else {
            crate::chrome::chip_hold(ui, self.state.tx.ptt, label, fill, ink)
        }
        .on_hover_text(PTT_HOLD_HINT);
        self.apply_held_ptt(&resp, cmds);
    }

    /// Key or unkey from the compact PTT chip's response — see [`PttPress`],
    /// which is where the decision itself lives.
    fn apply_held_ptt(&mut self, resp: &egui::Response, cmds: &mut Vec<Command>) {
        let down = self.ptt.still_down(
            resp.is_pointer_button_down_on(),
            resp.ctx.input(|i| i.pointer.primary_down()),
        );
        // Against our own last edge, not the engine's echo: the echo lags a
        // round trip, and comparing to it would re-send the same command every
        // frame until it caught up.
        if down == self.ptt.pressed() {
            return;
        }
        // Asked at the press and remembered by `PttPress::Keying`: by the time
        // the finger lifts there are no touches left to ask about.
        let touch = down && resp.ctx.input(|i| i.any_touches());
        let (next, cmd) = self.ptt.on_pointer(down, touch, resp.clicked());
        self.ptt = next;
        if let Some(on) = cmd {
            cmds.push(Command::SetPtt(on));
        }
    }

    /// How many digit columns the frequency readout needs: [`freq_display::DIGITS`]
    /// on every radio this program reaches on its own, one more
    /// ([`freq_display::DIGITS_EXT`]) once its published receive range —
    /// already shifted by whatever converter/LNB offset is configured, see
    /// `shift_caps` — reaches 10 GHz or past it. A QO-100 station is the case
    /// this exists for: without it, the dial reading 10489.750 MHz has no
    /// column for the leading "1" and silently shows "0489.750.000" instead.
    ///
    /// Read from the live capabilities rather than the offset alone, so a
    /// receiver that reaches 10 GHz on its own hardware (a wideband direct
    /// sampler, a paired panadapter) gets the same extra column without
    /// needing a converter to ask for it.
    ///
    /// The published range is not the only tell, though: a SoapySDR driver
    /// that never implemented `getFrequencyRange` publishes an empty list, so
    /// a 3-cm LNB station driven through one would still truncate its
    /// 10489.750 MHz dial. So the configured converter offset and the dial
    /// itself each earn the column on their own — the offset so the column is
    /// there *before* the operator tunes up rather than appearing mid-digit.
    fn readout_digits(&self) -> u32 {
        let range_reaches_10ghz =
            self.caps.as_ref().is_some_and(|c| c.freq_ranges_rx.iter().any(|&(_, hi)| hi >= 1e10));
        let converter_offset_hz =
            self.radio_cfg.as_ref().map(|c| c.converter_offset_hz).unwrap_or(0.0);
        readout_digit_count(range_reaches_10ghz, converter_offset_hz, self.state.active_freq_hz())
    }

    /// The VFO frequency controls (A/B select + big readout + the inactive
    /// VFO's frequency) in a label-less box, always the first module.
    ///
    /// On a compact layout the box gives up its side columns instead of being
    /// clipped: the A/B selector and the inactive VFO's frequency move to the
    /// VFO menu, and the digits shrink to whatever the row can actually spare.
    /// Returns whether the band/mode chip found room here — when it did not,
    /// the menu row shows it instead, so it is never simply lost.
    fn freq_module(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        tier: crate::layout::Tier,
    ) -> bool {
        let fit = ReadoutFit::measure(ui, self.readout_digits());
        if tier == crate::layout::Tier::Phone {
            return self.freq_module_compact(ui, cmds, &fit);
        }
        // Wide enough for the full box: size the digits to their design size,
        // dropping only as far as the row makes necessary. The S-meter is the
        // one thing that has to stay on this row with them — a tablet in
        // portrait has just enough width for both if the digits give a little.
        // The few extra points of slack keep a readout that comes out exactly
        // the width of the space left from rounding its way onto the next row.
        let plan = self.freq_box_plan(ui, &fit, ui.available_width(), SMETER_W + 8.0 + 4.0, tier);
        self.freq_module_at(ui, cmds, &plan);
        true
    }

    /// The frequency box's measured geometry: the digit size the width budget
    /// buys, the measured side-column widths, and the outer width the box will
    /// reserve. `beside` is width that must stay free of the same `avail` —
    /// the tablet keeps the S-meter beside the readout; the desktop packer,
    /// which breaks rows itself, passes only a few points of rounding slack.
    fn freq_box_plan(
        &self,
        ui: &egui::Ui,
        fit: &ReadoutFit,
        avail: f32,
        beside: f32,
        tier: crate::layout::Tier,
    ) -> FreqBoxPlan {
        // Both side columns are measured rather than assumed. Their design
        // widths hold for a desktop, but a touched layout pads every chip out
        // past them — and a column reserved too narrow does not clip, it
        // overflows the box, which is what would push the meter onto a row of
        // its own on a tablet in portrait.
        let ab_w = AB_W.max(2.0 * crate::chrome::chip_width(ui, "A", Some(15.0)) + 6.0);
        let right_w = RIGHT_W
            .max(crate::chrome::chip_width(ui, &self.band_mode_label(), Some(BAND_MODE_TEXT)))
            .max(crate::chrome::text_width(
                ui,
                &self.inactive_vfo_label(),
                egui::FontId::monospace(12.0),
            ));
        let overhead = ab_w + right_w + 38.0; // side columns, gaps, box margins
        let size = fit.fit(avail - overhead - beside).clamp(MIN_DIGIT, tier.digit_cap());
        let (readout_w, readout_h) = (fit.width(size), fit.height(size));
        // The box still hugs its contents: that keeps the right column against
        // the box edge (no empty space) and lets the readout be centred
        // vertically by exact geometry rather than a fragile layout hint.
        let box_w = 8.0 + ab_w + 10.0 + readout_w + 12.0 + right_w + 8.0;
        FreqBoxPlan { size, ab_w, right_w, readout_w, readout_h, box_w }
    }

    /// Draw the frequency box to a pre-measured [`FreqBoxPlan`].
    fn freq_module_at(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, plan: &FreqBoxPlan) {
        let &FreqBoxPlan { size, ab_w, right_w, readout_w, readout_h, box_w } = plan;
        crate::chrome::module_bare_h(ui, box_w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // control every gap explicitly
            let active = self.state.active_vfo;
            let full_h = ui.available_height();

            // VFO A/B selector, vertically centred in the full box height —
            // with sdroxide's link switch stacked above it where this station
            // holds the switch and the box has room for both rows. Where it has
            // not — the touched tiers, whose chips are half again as tall — the
            // VFO menu carries the same switch instead, and the two ask
            // [`Self::stacked_link`] so they cannot both answer yes.
            let ab_h = crate::chrome::chip_height(ui, Some(15.0));
            let power_h = crate::chrome::chip_height(ui, Some(LINK_TEXT));
            if let Some(on) = self.stacked_link(ui) {
                ui.allocate_ui_with_layout(
                    egui::vec2(ab_w, full_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        ui.add_space(((full_h - power_h - LINK_GAP - ab_h) / 2.0).max(0.0));
                        // As wide as the A/B pair under it: the two rows read
                        // as one block, and the symbol earns a target worth
                        // clicking instead of a chip hugging a glyph.
                        let pair_w = 2.0 * crate::chrome::chip_width(ui, "A", Some(15.0)) + 6.0;
                        ui.horizontal(|ui| {
                            self.link_chip(ui, on, egui::vec2(pair_w, power_h));
                        });
                        ui.add_space(LINK_GAP);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            vfo_ab_chips(ui, active, cmds);
                        });
                    },
                );
            } else {
                ui.allocate_ui_with_layout(
                    egui::vec2(ab_w, full_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        vfo_ab_chips(ui, active, cmds);
                    },
                );
            }
            ui.add_space(10.0);

            // Big frequency readout, centred vertically by measured height.
            // On the air through a repeater it is the transmit frequency — see
            // [`Self::readout`].
            let (shown, ink, offset) = self.readout();
            let mut new_hz = None;
            let readout = ui.allocate_ui_with_layout(
                egui::vec2(readout_w, full_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_space(((full_h - readout_h) / 2.0).max(0.0));
                    new_hz = freq_display::show(
                        ui,
                        crate::layout::salted_id(ui.ctx(), "main-freq"),
                        shown,
                        self.input.cfg.wheel,
                        size,
                        ink,
                        self.readout_digits(),
                    );
                },
            );
            // When the VFO sits exactly on a stored memory, say which one.
            // Painted rather than laid out, so the readout never shifts as it
            // appears — anchored to the bottom of the box, not the digit row
            // (`readout.response.rect` is the *used* rect, which ends at the
            // digits), so it clears their ink instead of hugging the baseline.
            if let Some(name) = self.memory_name_at_vfo() {
                let r = readout.response.rect;
                ui.painter().text(
                    egui::pos2(r.left(), r.top() + full_h),
                    egui::Align2::LEFT_BOTTOM,
                    name,
                    egui::FontId::proportional(10.0),
                    crate::theme::CYAN_DIM(),
                );
            }
            if let Some(hz) = new_hz {
                cmds.push(Command::SetVfo { vfo: active, hz: hz - offset });
            }
            ui.add_space(12.0);

            // Right column: inactive VFO frequency anchored top-right, band/mode
            // selector anchored bottom-right, hard against the box edge.
            let inactive = self.inactive_vfo_label();
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, full_h),
                egui::Layout::top_down(egui::Align::Max),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(
                        RichText::new(inactive)
                            .monospace()
                            .size(12.0)
                            .color(crate::theme::gray(120)),
                    );
                    // Push the band/mode chip to the bottom of the column by
                    // its measured height. A literal here would leave the
                    // column taller than the box on a touched layout, where a
                    // chip is half again as tall — and a box that outgrows
                    // `MODULE_TALL_H` no longer lines up with the S-meter.
                    let pad = (ui.available_height() - band_mode_h(ui)).max(0.0);
                    ui.add_space(pad);
                    self.band_mode_button(ui, cmds);
                },
            );
        });
    }

    /// Whether the frequency box stacks the link switch above the A/B pair —
    /// and where the switch stands if it does.
    ///
    /// Measured against the live style rather than assumed by tier, so a
    /// roomier chip cannot silently overflow the box; and asked in both places
    /// that could draw the switch — the box, and the VFO menu that carries it
    /// for the layouts whose box has no room — so it can never come out in
    /// both at once, or in neither.
    fn stacked_link(&self, ui: &egui::Ui) -> Option<bool> {
        let rows = crate::chrome::chip_height(ui, Some(15.0))
            + LINK_GAP
            + crate::chrome::chip_height(ui, Some(LINK_TEXT));
        self.own_link_state()
            .filter(|_| rows <= crate::chrome::module_content_h(crate::chrome::MODULE_TALL_H))
    }

    /// The link switch as the VFO menu carries it, for the compact strips
    /// whose frequency box had no room to stack it — which is every touched
    /// tier. The menu is the A/B selector's other home, so it is where the
    /// switch that sits above that selector belongs.
    ///
    /// Spelled out in words beside the symbol, unlike the box's: a menu has
    /// the width for it, and a finger gets no hover text to read the symbol
    /// by. Without this a single-radio touch layout — a phone browser on a
    /// headless station, most of all — has nowhere at all to switch its radio:
    /// there is no tab strip with one radio, and the settings roster leaves
    /// the switch off for the same reason.
    fn link_menu_row(&mut self, ui: &mut egui::Ui) {
        let Some(on) = self.own_link_state().filter(|_| self.stacked_link(ui).is_none()) else {
            return;
        };
        crate::chrome::menu_caption(ui, "Radio");
        ui.horizontal(|ui| {
            let h = crate::chrome::chip_height(ui, Some(LINK_TEXT));
            self.link_chip(ui, on, egui::vec2(2.0 * h, h));
            let label = if on { "Linked" } else { "Not linked" };
            ui.label(RichText::new(label).size(12.5).color(crate::theme::gray(160)));
        });
    }

    /// sdroxide's link switch, above the A/B selector: the same switch this
    /// radio's tab on the strip carries, wired to the same shell request, so
    /// the two can never disagree. Having it on the main window is what lets a
    /// *single*-radio session — which has no strip, and whose settings roster
    /// offers no switch — put its radio down and pick it back up.
    fn link_chip(&mut self, ui: &mut egui::Ui, on: bool, size: egui::Vec2) {
        let chip = crate::chrome::chip_link(ui, on, size);
        let tip = if on { crate::chrome::LINK_CLOSE_TIP } else { crate::chrome::LINK_OPEN_TIP };
        if chip.on_hover_text(tip).clicked() {
            self.radio_tab_requests
                .push(crate::app::RadioTabRequest::Power { id: self.radio_id, on: !on });
        }
    }

    /// The frequency box for a phone: which VFO is being tuned, the digits, and
    /// the band/mode chip if it fits.
    ///
    /// The A/B chips and the inactive VFO's frequency are in the VFO menu
    /// instead. At this width they cost more than the digits can spare, and a
    /// readout too small to read is worse than a selector one tap away. The
    /// readout is the type-in kind: at this size a fingertip covers three
    /// digits, so per-digit tuning would be a lottery.
    fn freq_module_compact(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        fit: &ReadoutFit,
    ) -> bool {
        let active = self.state.active_vfo;
        let tag = match active {
            Vfo::A => "A",
            Vfo::B => "B",
        };
        let tag_w = ui
            .painter()
            .layout_no_wrap(tag.to_owned(), egui::FontId::proportional(13.0), Color32::WHITE)
            .size()
            .x;

        // What the band/mode chip would cost, measured rather than guessed: its
        // label runs from "20m · USB" to "160m · DIGU" and the chip's padding
        // grows with the touch style.
        let bm_label = self.band_mode_label();
        let bm_w = ui
            .painter()
            .layout_no_wrap(bm_label, egui::FontId::proportional(BAND_MODE_TEXT), Color32::WHITE)
            .size()
            .x
            + 2.0 * (ui.spacing().button_padding.x + 2.0);

        let fixed = 16.0 + tag_w + 6.0; // box margins + the VFO tag
        let avail = ui.available_width();
        // Try to keep the chip; give it up only when keeping it would push the
        // digits below the size at which they stop reading as a dial.
        let with_chip = fit.fit(avail - fixed - 8.0 - bm_w).min(PHONE_DIGIT_MAX);
        let band_mode = with_chip >= MIN_DIGIT;
        let size = if band_mode {
            with_chip
        } else {
            fit.fit(avail - fixed).clamp(MIN_DIGIT, PHONE_DIGIT_MAX)
        };

        let box_w = fixed + fit.width(size) + if band_mode { 8.0 + bm_w } else { 0.0 };
        crate::chrome::module_bare_h(ui, box_w, PHONE_FREQ_H, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // control every gap explicitly
            let (shown, ink, offset) = self.readout();
            ui.label(
                RichText::new(tag)
                    .size(13.0)
                    .strong()
                    .color(ink.unwrap_or_else(crate::theme::CYAN)),
            );
            ui.add_space(6.0);
            let new_hz = freq_display::show_typed(
                ui,
                crate::layout::salted_id(ui.ctx(), "main-freq"),
                shown,
                size,
                ink,
                self.readout_digits(),
            );
            if let Some(hz) = new_hz {
                cmds.push(Command::SetVfo { vfo: active, hz: hz - offset });
            }
            if band_mode {
                ui.add_space(8.0);
                self.band_mode_button(ui, cmds);
            }
        });
        band_mode
    }

    /// What the big frequency readout shows: the number, the ink to draw it in,
    /// and how far it sits from the dial.
    ///
    /// Normally that is the dial itself, in the resting amber, at no offset.
    /// While the transmitter is keyed and a repeater shift has put it somewhere
    /// else, the readout follows it there in the alert red — because on a
    /// repeater the number on the front of the radio is no longer the frequency
    /// being listened to, and a display that quietly went on showing the output
    /// while the transmitter was on the input would be saying something untrue
    /// at the one moment it matters. It is also what every radio with a duplex
    /// button does.
    ///
    /// The offset comes back with it so the readout stays *tunable*: a wheel
    /// turn or a typed frequency on those digits still has to move the VFO by
    /// what the operator changed, not jump it by the shift.
    ///
    /// The whole gap to the transmit frequency is used, not just the repeater's
    /// share of it, so a shift stacked on split or XIT reads as the frequency
    /// that is actually going out. What it is gated on is the repeater shift
    /// alone: split has always left this readout on the dial, and this is not
    /// the place to change that.
    fn readout(&self) -> (f64, Option<Color32>, f64) {
        readout_for(&self.state, self.tab_tx_on(), self.ui_settings.cw_qrg, self.cw_pitch_hz())
    }

    /// The band/mode chip's label, e.g. `20m · USB`.
    fn band_mode_label(&self) -> String {
        format!("{} · {}", self.state.band.label(), self.state.rx[0].mode.label())
    }

    /// The name of the stored memory channel the active VFO is parked on, if
    /// any: same frequency to the Hz the readout shows, and same mode.
    fn memory_name_at_vfo(&self) -> Option<&str> {
        let hz = self.state.active_freq_hz().round() as i64;
        let mode = self.state.rx[0].mode;
        self.memories
            .iter()
            .find(|m| m.mode == mode && m.freq_hz.round() as i64 == hz)
            .map(|m| m.name.as_str())
    }

    /// The VFO that is *not* being tuned, as a MHz label.
    fn inactive_vfo_label(&self) -> String {
        let hz = match self.state.active_vfo {
            Vfo::A => self.state.vfo_b_hz,
            Vfo::B => self.state.vfo_a_hz,
        };
        format!("{hz:.6} MHz", hz = hz / 1e6)
    }

    /// Take the face a click asked for and write it out.
    ///
    /// Straight to disk rather than left for eframe's periodic save: the face
    /// is a screen preference in `[ui]` (issue #185), it is written the moment
    /// it changes like every other one, and a session that ends in a `pkill`
    /// or a crash then still comes back on the instrument the operator chose.
    fn set_smeter_style(&mut self, style: sdroxide_types::SmeterStyle) {
        if self.ui_settings.smeter_style != style {
            self.ui_settings.smeter_style = style;
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
    }

    /// The S-meter in a label-less box, always pinned top-right. Clicking it
    /// cycles the needle / bar / trace faces.
    ///
    /// The meter lays itself out against the rect it is given, so a shorter box
    /// is simply a smaller instrument and any width at all is a meter — which
    /// is what makes it the strip's width absorber. `compact` is the phone's
    /// 40 pt box: no room for an arc, so the needle is skipped, both in what is
    /// drawn and in what a click cycles to. The operator's persisted choice is
    /// left alone — the desktop it was made on still honours it. What width
    /// the phone's box gets is [`plan_phone_tail`]'s decision; the desktop
    /// packer makes its own.
    ///
    /// The bar and trace faces scale with the box, and the needle face keeps
    /// its scale readable by capping and centring it (see
    /// [`crate::widgets::smeter::NEEDLE_FACE_MAX_W`]).
    fn smeter_box(&mut self, ui: &mut egui::Ui, w: f32, h: f32, compact: bool) {
        let style = self.ui_settings.smeter_style;
        let shown = if compact { style.compact() } else { style };
        let mut picked = None;
        crate::chrome::module_bare_flush_h(ui, w, h, |ui| {
            let resp = smeter::show(ui, self.meters.as_ref(), shown).on_hover_text(if compact {
                "Click to cycle meter face: bar / trace"
            } else {
                "Click to cycle meter face: needle / bar / trace"
            });
            if resp.clicked() {
                picked = Some(if compact { style.next_compact() } else { style.next() });
            }
        });
        if let Some(style) = picked {
            self.set_smeter_style(style);
        }
    }

    /// Combined VFO + RIT/XIT box: the VFO A/B utility chips on top, with the
    /// RIT/XIT tuning-offset controls stacked underneath. Bare and tall — this
    /// replaces the separate VFO and RIT/XIT boxes. Width past
    /// [`Self::vfo_rows_w`] is spent by the controls themselves: the utility
    /// chips widen, and the offset fields grow.
    fn vfo_rit_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        let tx_capable = self.tx_capable();
        let inner = w - 2.0 * crate::chrome::MODULE_MARGIN_X - 4.0;
        let chips = vfo_chip_labels(tx_capable);
        let extra1 = ((inner - chip_row_w(ui, &chips)) / chips.len() as f32).max(0.0);
        let fields = if tx_capable { 2.0 } else { 1.0 };
        let extra2 = ((inner - vfo_offsets_w(ui, tx_capable)) / fields).max(0.0);
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                ui.horizontal(|ui| self.vfo_util_chips(ui, cmds, extra1));
                ui.horizontal(|ui| self.vfo_offset_row(ui, cmds, false, extra2));
            });
        });
    }

    /// The VFO utility chips — the top row of the VFO/RIT box, and the head of
    /// the VFO menu. `extra` stretches each chip; the popup passes 0.
    fn vfo_util_chips(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, extra: f32) {
        let [swap, copy, split, sub] = VFO_CHIPS;
        if chip_stretched(ui, false, swap, extra).on_hover_text("Swap VFOs").clicked() {
            cmds.push(Command::SwapVfos);
        }
        if chip_stretched(ui, false, copy, extra).on_hover_text("Copy A to B").clicked() {
            cmds.push(Command::CopyAtoB);
        }
        if chip_stretched(ui, self.state.split, split, extra).clicked() {
            cmds.push(Command::SetSplit(!self.state.split));
        }
        if chip_stretched(ui, self.state.sub_rx_enabled, sub, extra)
            .on_hover_text(
                "Second receiver, in the right ear. It tunes independently of \
                 A/B — its controls appear in the SUB module, and its passband \
                 on the waterfall.",
            )
            .clicked()
        {
            cmds.push(Command::SetSubRx(!self.state.sub_rx_enabled));
        }
        if self.tx_capable() {
            self.duplex_chip(ui, cmds, extra);
        }
        self.repeater_tone_chip(ui, cmds, extra);
    }

    /// The RIT/XIT tuning offsets — the bottom row of the VFO/RIT box, and the
    /// tail of the VFO menu. `extra` widens each Hz field; the popup passes 0.
    fn vfo_offset_row(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        narrow: bool,
        extra: f32,
    ) {
        let tx_capable = self.tx_capable();
        // Wide enough for a signed 4-digit offset plus " Hz", and tall enough
        // to hit with a finger where the layout expects one.
        let hz_field = if narrow {
            egui::vec2(96.0, ui.spacing().interact_size.y.max(22.0))
        } else {
            egui::vec2(HZ_FIELD_W + extra, 22.0)
        };
        let rit = self.state.rit;
        if crate::chrome::chip(ui, rit.enabled, "RIT").clicked() {
            cmds.push(Command::SetRit { enabled: !rit.enabled, hz: rit.hz });
        }
        let mut rit_hz = rit.hz;
        if ui
            .add_sized(
                hz_field,
                DragValue::new(&mut rit_hz).speed(5).range(-9999..=9999).suffix(" Hz"),
            )
            .changed()
        {
            cmds.push(Command::SetRit { enabled: rit.enabled, hz: rit_hz });
        }
        if tx_capable {
            let xit = self.state.xit;
            if crate::chrome::chip(ui, xit.enabled, "XIT").clicked() {
                cmds.push(Command::SetXit { enabled: !xit.enabled, hz: xit.hz });
            }
            let mut xit_hz = xit.hz;
            if ui
                .add_sized(
                    hz_field,
                    DragValue::new(&mut xit_hz).speed(5).range(-9999..=9999).suffix(" Hz"),
                )
                .changed()
            {
                cmds.push(Command::SetXit { enabled: xit.enabled, hz: xit_hz });
            }
        }
    }

    /// The DUPLEX chip and the repeater-shift popup behind it.
    ///
    /// Lit — and in the warning colour, like the tone squelch — whenever the
    /// transmitter is not where the receiver is. That is the whole of the
    /// "split indicator" a repeater needs at a glance: the panadapter already
    /// draws the transmit marker at its real frequency, and the hover text
    /// spells the figure out.
    fn duplex_chip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, extra: f32) {
        let r = self.state.repeater;
        let shifted = r.shift != Shift::Simplex;
        let hover = if shifted {
            format!(
                "Repeater shift {} — receiving on {:.6}, transmitting on {:.6} MHz{}",
                r.shift_label(),
                self.state.rx_freq_hz() / 1e6,
                self.state.tx_freq_hz() / 1e6,
                if r.auto { ", from the band plan" } else { "" },
            )
        } else if r.auto {
            "Repeater shift: following the band plan, which has no shift for this \
             frequency — simplex"
                .to_string()
        } else {
            "Repeater shift: transmit above or below the dial, to work a repeater. \
             Simplex now"
                .to_string()
        };
        let btn = if shifted {
            accent_chip_stretched(
                ui,
                true,
                DUPLEX_CHIP,
                crate::theme::YELLOW(),
                crate::theme::INK_ON_BRIGHT(),
                extra,
            )
        } else {
            chip_stretched(ui, false, DUPLEX_CHIP, extra)
        }
        .on_hover_text(hover);

        let popup_id = egui::Popup::default_response_id(&btn);
        let now = ui.input(|i| i.time);
        let alpha =
            crate::chrome::popup_fade_alpha(ui.ctx(), popup_id, now, &mut self.duplex_popup_since);
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                self.duplex_controls(ui, cmds);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            if r.response.contains_pointer() {
                self.duplex_popup_since = Some(now);
            }
        }
    }

    /// The repeater shift: which way, how far, and what that works out to.
    fn duplex_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut r = self.state.repeater;
        let mut changed = false;
        crate::chrome::menu_caption(ui, "Repeater shift");
        ui.horizontal_wrapped(|ui| {
            for s in Shift::ALL {
                if crate::chrome::chip(ui, r.shift == s, s.label()).clicked() {
                    r.shift = s;
                    // Chosen by hand, so stop following the plan — otherwise
                    // the next turn of the dial would put it straight back and
                    // the chip would look broken.
                    r.auto = false;
                    changed = true;
                }
            }
            if crate::chrome::chip(ui, r.auto, "AUTO")
                .on_hover_text(
                    "Take the shift from the band plan as the dial moves. It only \
                     speaks inside a repeater output sub-band — everywhere else it \
                     leaves the radio simplex, so the calling channels stay simplex.",
                )
                .clicked()
            {
                r.auto = !r.auto;
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Offset").weak());
            let mut khz = f64::from(r.offset_hz) / 1e3;
            if crate::chrome::field(
                ui,
                DragValue::new(&mut khz)
                    .speed(5.0)
                    .range(0.0..=f64::from(MAX_OFFSET_HZ) / 1e3)
                    // Four decimals is a shift to the Hz. Nothing published
                    // needs them, but a hand-built machine might.
                    .max_decimals(4)
                    .suffix(" kHz"),
            )
            .changed()
            {
                r.offset_hz = (khz * 1e3).round().clamp(0.0, f64::from(MAX_OFFSET_HZ)) as u32;
                r.auto = false;
                changed = true;
            }
        });
        // What it works out to, on the settings as they stand in this popup —
        // the state's own figure is still the old one until the engine answers.
        let tx = self.state.tx_freq_hz() - self.state.repeater.shift_hz() + r.shift_hz();
        ui.label(
            RichText::new(format!(
                "RX {:.6}   TX {:.6} MHz",
                self.state.rx_freq_hz() / 1e6,
                tx / 1e6,
            ))
            .monospace()
            .size(11.0)
            .color(if r.shift == Shift::Simplex {
                crate::theme::gray(140)
            } else {
                crate::theme::YELLOW()
            }),
        );
        if changed {
            self.state.repeater = r; // optimistic echo
            cmds.push(Command::SetRepeater(r));
        }
    }

    /// The TONE chip in the VFO box, and the repeater signalling behind it: the
    /// CTCSS tone or DCS code that goes out under the voice, the 1750 Hz burst,
    /// and the receive tone squelch.
    ///
    /// Not the same control as the RX box's tone chip, which is a *readout* of
    /// what is arriving and only appears in NFM. This one is what the station
    /// sends, which is a thing to set up before there is anything to hear —
    /// and it carries the squelch as well, because a repeater directory gives
    /// the tone once and radios ask for it on both sides.
    fn repeater_tone_chip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, extra: f32) {
        let r = self.state.repeater;
        let sending = r.tx_tone();
        let hover = match (sending, r.burst_auto) {
            (Some(t), true) => format!(
                "Transmitting {} under the voice, and a {} ms 1750 Hz burst at the \
                 start of every over",
                t.label(),
                r.burst_ms,
            ),
            (Some(t), false) => format!("Transmitting {} under the voice", t.label()),
            (None, true) => {
                format!("A {} ms 1750 Hz burst at the start of every over", r.burst_ms)
            }
            (None, false) => "Repeater tone: the CTCSS/DCS that goes out under the voice, \
                              the 1750 Hz burst, and the receive tone squelch"
                .to_string(),
        };
        let lit = sending.is_some() || r.burst_auto;
        let btn = if lit {
            accent_chip_stretched(
                ui,
                true,
                TONE_CHIP,
                crate::theme::YELLOW(),
                crate::theme::INK_ON_BRIGHT(),
                extra,
            )
        } else {
            chip_stretched(ui, false, TONE_CHIP, extra)
        }
        .on_hover_text(hover);

        let popup_id = egui::Popup::default_response_id(&btn);
        let now = ui.input(|i| i.time);
        let alpha = crate::chrome::popup_fade_alpha(
            ui.ctx(),
            popup_id,
            now,
            &mut self.rpt_tone_popup_since,
        );
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                self.repeater_tone_controls(ui, cmds);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            if r.response.contains_pointer() {
                self.rpt_tone_popup_since = Some(now);
            }
        }
    }

    /// The body of the TONE popup.
    fn repeater_tone_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut r = self.state.repeater;
        let mut changed = false;
        let fm = self.state.rx[0].mode == Mode::Nfm;

        crate::chrome::menu_caption(ui, "Transmit tone");
        if !fm {
            // Said rather than greyed out: the settings are worth arranging
            // ahead of the mode, and a memory stores them whatever mode it is
            // in. What is not worth doing is leaving the operator to wonder
            // why nothing goes out.
            ui.label(
                RichText::new("set here, sent on NFM — sub-audible signalling is an FM channel's")
                    .weak()
                    .size(10.0),
            );
        }
        ui.horizontal(|ui| {
            for m in ToneMode::ALL {
                if crate::chrome::chip(ui, r.tone == m, m.label()).clicked() {
                    r.tone = m;
                    changed = true;
                }
            }
        });
        match r.tone {
            ToneMode::Off => {}
            ToneMode::Ctcss => {
                egui::ScrollArea::vertical().max_height(200.0).id_salt("rpt-ctcss").show(
                    ui,
                    |ui| {
                        egui::Grid::new("rpt-ctcss-grid").spacing([3.0, 3.0]).show(ui, |ui| {
                            for (i, &tenths) in sdroxide_types::CTCSS_TONES.iter().enumerate() {
                                let label = format!("{}.{}", tenths / 10, tenths % 10);
                                if crate::chrome::chip(ui, r.ctcss_tenths == tenths, label)
                                    .clicked()
                                {
                                    r.ctcss_tenths = tenths;
                                    changed = true;
                                }
                                if i % 10 == 9 {
                                    ui.end_row();
                                }
                            }
                        });
                    },
                );
            }
            ToneMode::Dcs => {
                ui.horizontal(|ui| {
                    for (invert, label) in [(false, "NORMAL"), (true, "INVERT")] {
                        if crate::chrome::chip(ui, r.dcs_invert == invert, label).clicked() {
                            r.dcs_invert = invert;
                            changed = true;
                        }
                    }
                });
                ui.label(
                    RichText::new(
                        "the bit order DCS is encoded with here is transcribed from the \
                         standard and has never been checked against a repeater — if it \
                         will not open, try CTCSS",
                    )
                    .weak()
                    .size(10.0),
                );
                egui::ScrollArea::vertical().max_height(200.0).id_salt("rpt-dcs").show(ui, |ui| {
                    egui::Grid::new("rpt-dcs-grid").spacing([3.0, 3.0]).show(ui, |ui| {
                        for (i, &code) in DCS_CODES.iter().enumerate() {
                            if crate::chrome::chip(ui, r.dcs_code == code, format!("{code:03}"))
                                .clicked()
                            {
                                r.dcs_code = code;
                                changed = true;
                            }
                            if i % 8 == 7 {
                                ui.end_row();
                            }
                        }
                    });
                });
            }
        }

        crate::chrome::menu_caption(ui, "1750 Hz burst");
        ui.horizontal(|ui| {
            if crate::chrome::chip(ui, r.burst_auto, "EVERY OVER")
                .on_hover_text("Open with the burst every time the transmitter keys")
                .clicked()
            {
                r.burst_auto = !r.burst_auto;
                changed = true;
            }
            let mut ms = r.burst_ms;
            if crate::chrome::field(
                ui,
                DragValue::new(&mut ms).speed(10).range(BURST_MS_RANGE).suffix(" ms"),
            )
            .changed()
            {
                r.burst_ms = ms;
                changed = true;
            }
            // The single button the whole feature is named after: on receive it
            // keys, sends the burst and unkeys again; mid-over it plays over
            // the microphone.
            if crate::chrome::chip_accent_enabled(
                ui,
                fm,
                false,
                "SEND",
                None,
                crate::theme::GREEN(),
                crate::theme::INK_ON_BRIGHT(),
            )
            .on_hover_text(if fm {
                "Send the burst now — keying the transmitter for its length if it is \
                 not already keyed"
            } else {
                "NFM only: the burst is an FM repeater's door-opener"
            })
            .clicked()
            {
                cmds.push(Command::ToneBurst);
            }
        });

        crate::chrome::menu_caption(ui, "Receive tone squelch");
        let heard = self.meters.as_ref().and_then(|m| m.tone);
        let armed = self.state.rx[0].tone_sql;
        // The shortcut that saves reading the tone out of one grid and finding
        // it again in another: a repeater directory gives one tone and the
        // radio wants it on both sides.
        if let Some(t) = r.tx_tone() {
            let want = match t {
                sdroxide_types::TxSubTone::Ctcss(tenths) => SubTone::Ctcss(tenths),
                sdroxide_types::TxSubTone::Dcs { .. } => SubTone::Dcs,
            };
            if armed != Some(want)
                && crate::chrome::chip(ui, false, format!("MATCH TX ({})", t.label()))
                    .on_hover_text("Require on receive what this station transmits")
                    .clicked()
            {
                self.state.rx[0].tone_sql = Some(want); // optimistic echo
                cmds.push(Command::SetToneSquelch { rx: RxId::Main, tone: Some(want) });
            }
        }
        self.tone_controls(ui, cmds, heard, armed);

        if changed {
            self.state.repeater = r; // optimistic echo
            cmds.push(Command::SetRepeater(r));
        }
    }

    /// The VFO utility chips and the RIT/XIT offsets — the body of the VFO
    /// menu. See [`crate::chrome::control_row`] for `narrow`.
    fn vfo_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, narrow: bool) {
        crate::chrome::control_row(ui, narrow, |ui| self.vfo_util_chips(ui, cmds, 0.0));
        crate::chrome::control_row(ui, narrow, |ui| self.vfo_offset_row(ui, cmds, narrow, 0.0));
    }

    /// The VFO/RIT box's natural width: the wider of the utility-chip row and
    /// the RIT/XIT row, plus the box margins and a little rounding slack. A
    /// receive-only rig has no XIT, and stops paying for it.
    fn vfo_rows_w(&self, ui: &egui::Ui) -> f32 {
        let tx_capable = self.tx_capable();
        chip_row_w(ui, &vfo_chip_labels(tx_capable)).max(vfo_offsets_w(ui, tx_capable))
            + 2.0 * crate::chrome::MODULE_MARGIN_X
            + 4.0
    }

    /// The band/mode selector button plus the floating popup with the band +
    /// mode + digital button rows.
    fn band_mode_button(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        self.band_mode_chip(ui, cmds, 0.0);
    }

    /// [`Self::band_mode_button`], `extra` points wider than its label — its
    /// share of what the S-meter beside it could not take of the phone's row.
    fn band_mode_chip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, extra: f32) {
        let mode = self.state.rx[0].mode;
        let label = RichText::new(self.band_mode_label()).size(BAND_MODE_TEXT);
        let w = crate::chrome::chip_width(ui, &self.band_mode_label(), Some(BAND_MODE_TEXT))
            + extra.max(0.0);
        let size = egui::vec2(w, band_mode_h(ui));
        let btn = crate::chrome::chip_lit_sized(
            ui,
            label,
            crate::theme::GREEN(),
            crate::theme::INK_ON_BRIGHT(),
            size,
        );

        // The same scrolled, viewport-sized popup the menu chips use. This is
        // the longest menu in the program — three sections and forty chips —
        // and it opens on every layout, so it is the one that has to be held
        // inside the screen in both directions rather than hang off it.
        let (state, caps) = (&self.state, &self.caps);
        let stated = self.radio_cfg.as_ref().is_some_and(|c| !c.freq_ranges_rx.is_empty());
        let (conditions, daylight) = (self.band_conditions.as_ref(), self.daylight);
        crate::chrome::fading_menu_popup(ui, &btn, &mut self.mode_popup_since, |ui| {
            band_mode_menu(ui, mode, state, caps.as_ref(), stated, conditions, daylight, cmds);
        });
    }

    /// [`rx_rows`] against this radio: what the front end offers, what the
    /// AGC is set to, and what mode the receiver is in.
    fn rx_rows(&self, ui: &egui::Ui) -> RxRows {
        rx_rows(
            ui,
            self.rx_gain().is_some(),
            self.decim_range().is_some(),
            self.state.rx[0].agc == AgcMode::Off,
            self.state.rx[0].mode,
        )
    }

    /// The Receiver + Filter/Noise box's natural width: the wider of its two
    /// rows once [`rx_rows`] has balanced them, plus the box margins and a
    /// little rounding slack. Which row leads changes with the rig and the
    /// state: the noise row usually, the receive row once it carries both a
    /// front-end gain rail and the manual-gain rail that appears with the AGC
    /// off.
    fn rx_filter_w(&self, ui: &egui::Ui) -> f32 {
        self.rx_rows(ui).w() + 2.0 * crate::chrome::MODULE_MARGIN_X + 4.0
    }

    /// Combined Receiver + Filter/Noise box: volume, gain and AGC on top, with
    /// the squelch + noise + mute/record chips stacked underneath. Bare and
    /// tall, like the VFO/RIT box. Width past [`Self::rx_filter_w`] goes to
    /// the Vol and SQL rails — one per row, so both rows grow by the same
    /// amount (the Gain and Man rails pin their own width and stay put).
    fn rx_filter_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        let extra = (w - self.rx_filter_w(ui)).clamp(0.0, RAIL_STRETCH_MAX);
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                ui.spacing_mut().slider_width = STRIP_RAIL_W + extra;
                self.rx_controls(ui, cmds, false);
            });
        });
    }

    /// The device's front-end RX gain, if it has one the software can set — the
    /// Hermes-Lite 2's LNA, a SoapySDR device's first RX stage. A rig with none
    /// (a CAT radio on a sound card) gets no slider and no extra module width,
    /// so nothing moves for the people who can't use it.
    fn rx_gains(&self) -> Vec<GainElement> {
        self.caps
            .as_ref()
            .map(|c| c.gains.iter().filter(|g| g.direction == Direction::Rx).cloned().collect())
            .unwrap_or_default()
    }

    fn rx_gain(&self) -> Option<GainElement> {
        self.rx_gains().first().cloned()
    }

    /// Whether the SQL rail drives the *radio's* squelch rather than the
    /// engine's own gate — true on a transceiver that hands us audio it has
    /// already squelched, which is the only front end where the software gate
    /// cannot reach what is muting the operator.
    fn rig_squelch(&self) -> bool {
        self.caps.as_ref().is_some_and(|c| c.commands_squelch)
    }

    /// The device rate the front end is streaming, and the deepest decimation
    /// it has the bandwidth for. `None` on a radio with no IQ to decimate — a
    /// CAT rig on a sound card — and on one already streaming so narrow a span
    /// that halving it would leave a single channel.
    fn decim_range(&self) -> Option<(f64, u32)> {
        if self.caps.as_ref().is_some_and(|c| c.audio_mode) {
            return None;
        }
        // The published rate is what the receiver runs at, which is already
        // divided by whatever decimation is in force; the device's own rate —
        // the one the ceiling is worked out from — is the two multiplied back
        // together.
        let device_hz = self.state.sample_rate * self.state.decimation.max(1) as f64;
        let max = sdroxide_types::max_decimation(device_hz);
        (max > 1).then_some((device_hz, max))
    }

    /// Front-end decimation: how much of the span to throw away before the
    /// receiver sees any of it.
    ///
    /// A cycling chip rather than a combo, for the reason set out beside the
    /// AGC chip in [`Self::rx_controls`]. It sits with the front-end gain
    /// rather than with the noise chips because it is the same kind of control
    /// — what the receiver is given to work with, decided once for the whole
    /// radio, rather than something done to one receiver's audio afterwards.
    fn decim_chip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some((device_hz, max)) = self.decim_range() else { return };
        let now = self.state.decimation.max(1);
        let label = if now > 1 { format!("DEC /{now}") } else { "DEC off".to_string() };
        let span_hz = device_hz / now as f64;
        let span = if span_hz >= 1e6 {
            format!("{:.3} MHz", span_hz / 1e6)
        } else {
            format!("{:.0} kHz", span_hz / 1e3)
        };
        let what = if now > 1 {
            format!(
                "Front-end decimation: the receiver is being given the middle 1/{now} of the \
                 {:.3} Msps this radio streams — {span} — and the rest is thrown away before \
                 any of it reaches the receiver.",
                device_hz / 1e6,
            )
        } else {
            format!(
                "Front-end decimation, off: the receiver sees the whole {span} this radio \
                 streams.",
            )
        };
        let hint = format!(
            "{what}\n\n\
             A narrower span means finer waterfall resolution, a quieter noise floor \
             (3 dB per halving) and less CPU — at the cost of the band either side of it. \
             The dial still tunes anywhere; the radio moves its LO to follow.\n\n\
             Click to cycle: off / 2 / 4 … {max}."
        );
        if crate::chrome::chip(ui, now > 1, label).on_hover_text(hint).clicked() {
            let next = if now * 2 > max { 1 } else { now * 2 };
            cmds.push(Command::SetDecimation(next));
        }
    }

    /// The receiver and filter/noise controls — the body of the RX box, and of
    /// the RX menu. See [`crate::chrome::control_row`] for `narrow`.
    ///
    /// Two rows, with the chip run breaking between them wherever [`rx_rows`]
    /// says rather than at a fixed place: what the receive row carries is the
    /// rig's business, and a run left whole under the squelch rail makes the
    /// box wider than the strip can pay for.
    fn rx_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, narrow: bool) {
        let rx_gains = self.rx_gains();
        let rx_gain = rx_gains.first().cloned();
        let chips = rx_chips(self.state.rx[0].mode);
        // A menu column wraps its rows and is as wide as the menu around it,
        // so there is nothing to balance there: the run stays whole, under the
        // squelch rail, in the order it has always had.
        let lifted = if narrow { 0 } else { self.rx_rows(ui).lifted };
        // Receiver: volume, RF gain, AGC and the manual gain it falls back to,
        // then as much of the chip run as this row has been given.
        crate::chrome::control_row(ui, narrow, |ui| {
            let mut vol = self.state.rx[0].volume;
            ui.label("Vol");
            if crate::chrome::slider(ui, Slider::new(&mut vol, 0.0..=1.0).show_value(false))
                .changed()
            {
                self.state.rx[0].volume = vol; // optimistic echo
                cmds.push(Command::SetVolume { rx: RxId::Main, v: vol });
            }
            if let Some(g) = &rx_gain {
                let mut hint = format!(
                    "Front-end RX gain ({}). Too much clips the receiver's ADC and \
                             smears spurious signals across the band; too little and it goes deaf.",
                    g.name
                );
                // A stage counted in steps rather than decibels — an RSP's
                // LNA state, an Airspy's place on its gain curve, a
                // SpyServer's index into the far end's table. Say so, because
                // the bare number on the rail otherwise reads as decibels for
                // want of anything saying it is not. Deliberately without a
                // direction of travel: every one of these is carried so that
                // right is more gain, but what 0 means differs between them.
                if g.unit == GainUnit::Step {
                    hint.push_str(
                        "\n\nCounted in steps, not decibels: what one step is worth is \
                         the receiver's own business, and on an SDRplay it depends on the \
                         band as well. Right is still more gain.",
                    );
                }
                if rx_gains.len() > 1 {
                    hint.push_str(&format!(
                        "\n\nThis rig has {} RX gain stages — the rest are in \
                                 Settings → Device.",
                        rx_gains.len()
                    ));
                }
                ui.label("Gain").on_hover_text(&hint);
                let mut db = self
                    .state
                    .gains
                    .iter()
                    .find(|(n, _)| *n == g.name)
                    .map(|(_, d)| *d)
                    .unwrap_or(g.min_db);
                let step = if g.step_db > 0.0 { g.step_db } else { 1.0 };
                // Narrower rail than Vol: this one carries a dB readout,
                // and the module has to stay inside one wrapped row. In
                // a menu the column is the constraint instead, and
                // `control_row` has already sized the rail to it.
                let resp = ui
                    .scope(|ui| {
                        if !narrow {
                            ui.spacing_mut().slider_width = RX_DB_RAIL_W;
                        }
                        crate::chrome::slider(
                            ui,
                            Slider::new(&mut db, g.min_db..=g.max_db)
                                .step_by(step)
                                // Whatever this element is actually counted in.
                                // Labelling a step index "dB" reported a number
                                // three times too small, in a unit it was not.
                                .suffix(g.suffix()),
                        )
                    })
                    .inner
                    .on_hover_text(&hint);
                if resp.changed() {
                    // Optimistic echo so the knob tracks the drag instead
                    // of snapping back until the engine answers.
                    match self.state.gains.iter_mut().find(|(n, _)| *n == g.name) {
                        Some((_, d)) => *d = db,
                        None => self.state.gains.push((g.name.clone(), db)),
                    }
                    cmds.push(Command::SetGain { dir: Direction::Rx, element: g.name.clone(), db });
                }
            }
            self.decim_chip(ui, cmds);
            // A cycling chip rather than a combo: a combo inside a menu
            // opens a second popup layer, and clicking it counts as
            // "outside" and closes the menu it was opened from. Four
            // settings is few enough to walk through anyway.
            //
            // Not drawn at all in FM, where the chain bypasses the AGC
            // (Mode::audio_agc) and the chip would do nothing.
            if self.state.rx[0].mode.audio_agc() {
                let agc = self.state.rx[0].agc;
                if crate::chrome::chip(ui, agc != AgcMode::Off, format!("AGC {}", agc.label()))
                    .on_hover_text("AGC hang time — click to cycle: Off / Slow / Med / Fast")
                    .clicked()
                {
                    cmds.push(Command::SetAgc { rx: RxId::Main, agc: agc.next() });
                }
                // With the AGC off the audio rides on this fixed gain instead.
                // It has to be here: unlevelled, the demodulator's output is
                // whatever the band handed it, and a weak SSB signal is tens of
                // dB below anything the volume control can reach. Switching the
                // AGC off seeds it from where the AGC had settled, so this
                // starts in the right place and only needs trimming.
                if agc == AgcMode::Off {
                    let mut db = self.state.rx[0].manual_gain_db;
                    ui.label("Man");
                    let resp = ui
                        .scope(|ui| {
                            if !narrow {
                                ui.spacing_mut().slider_width = RX_DB_RAIL_W;
                            }
                            crate::chrome::slider(
                                ui,
                                Slider::new(&mut db, 0.0..=sdroxide_types::MAX_MANUAL_GAIN_DB)
                                    .step_by(1.0)
                                    .suffix(" dB"),
                            )
                        })
                        .inner
                        .on_hover_text(
                            "Manual audio gain, used while the AGC is off. Seeded from \
                             the level the AGC was holding when it was switched off.",
                        );
                    if resp.changed() {
                        self.state.rx[0].manual_gain_db = db; // optimistic echo
                        cmds.push(Command::SetManualGain { rx: RxId::Main, db });
                    }
                }
            }
            for &c in &chips[..lifted] {
                self.rx_chip(ui, cmds, c, narrow);
            }
        });
        // Filter / Noise: squelch, then whatever is left of the run — the
        // noise chips, then mute and record, the two that act on the finished
        // audio rather than on the level.
        crate::chrome::control_row(ui, narrow, |ui| {
            ui.label("SQL");
            if self.rig_squelch() {
                // The radio's own gate, on the radio's own scale. The dBFS rail
                // below would be a control that does nothing here: what the
                // sound card receives has already been through the rig's
                // squelch, so a threshold on this side can close further on
                // what got through and can never open what was shut out
                // (issue #192).
                let mut sql = self.state.rig_squelch;
                if crate::chrome::slider(
                    ui,
                    Slider::new(&mut sql, 0.0..=1.0).show_value(true).custom_formatter(|v, _| {
                        if v <= 0.001 { "open".into() } else { format!("{:.0}%", v * 100.0) }
                    }),
                )
                .on_hover_text(
                    "The radio's own squelch, sent over the control link. This is the \
                     gate the audio actually passes through — the software one would only \
                     close further on what the rig already let by.",
                )
                .changed()
                {
                    self.state.rig_squelch = sql; // optimistic echo
                    cmds.push(Command::SetRigSquelch { frac: sql });
                }
            } else {
                let mut sql = self.state.rx[0].squelch_db;
                if crate::chrome::slider(
                    ui,
                    Slider::new(&mut sql, sdroxide_types::SQUELCH_OPEN_DB..=-30.0)
                        .show_value(true)
                        .custom_formatter(|v, _| {
                            if v <= (sdroxide_types::SQUELCH_OPEN_DB + 1.0) as f64 {
                                "off".into()
                            } else {
                                format!("{v:.0}")
                            }
                        }),
                )
                .changed()
                {
                    self.state.rx[0].squelch_db = sql; // optimistic echo
                    cmds.push(Command::SetSquelch { rx: RxId::Main, db: sql });
                }
            }
            for &c in &chips[lifted..] {
                self.rx_chip(ui, cmds, c, narrow);
            }
        });
        if narrow {
            // The engine picker the chip above cannot open from inside a menu.
            self.nr_controls(ui, cmds);
        }
    }

    /// Draw one chip of the RX box's run. Which row it lands on is [`rx_rows`]'s
    /// business; what the chip does is here. `narrow` is the menu column, where
    /// the NR chip stands in for a picker that cannot be opened from inside a
    /// menu.
    fn rx_chip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, chip: RxChip, narrow: bool) {
        match chip {
            RxChip::Nb => {
                let nb = self.state.noise_blanker;
                if crate::chrome::chip(ui, nb, "NB")
                    .on_hover_text("Impulse noise blanker")
                    .clicked()
                {
                    cmds.push(Command::SetNoiseBlanker(!nb));
                }
            }
            RxChip::Anc => {
                // Auto-notch — cancels constant tones (heterodynes / carriers).
                let anc = self.state.rx[0].auto_notch;
                if crate::chrome::chip(ui, anc, "ANC")
                    .on_hover_text("Auto-notch: cancel constant tone elements (heterodynes)")
                    .clicked()
                {
                    self.state.rx[0].auto_notch = !anc; // optimistic echo
                    cmds.push(Command::SetAutoNotch { rx: RxId::Main, on: !anc });
                }
            }
            RxChip::Nr => {
                // Noise reduction. The chip says only whether it is in circuit; the
                // picker behind it chooses which of the four engines and how hard.
                // A cycling chip was fine at seven states and two engines; at
                // thirteen and four it is a dozen clicks to cross, and which engine
                // to use is a considered choice rather than something to walk past
                // on the way to the one you wanted.
                if narrow {
                    // This row is itself inside the RX menu on a compact layout, and
                    // a popup opened from a popup counts as a click outside the
                    // first and closes it (see `sub_mode_picker`). So here the chip
                    // rides the strength and the picker is inlined below.
                    let nr = self.state.rx[0].noise_reduction;
                    let hover = match nr.engine() {
                        Some(e) => format!(
                            "Noise reduction: {} — {}\n\nClick to cycle the strength: \
                             Low / Med / High / Off. The engine is in the rows below.",
                            nr.label(),
                            e.name()
                        ),
                        None => "Noise reduction, off — click to switch it on, or pick an engine \
                                 in the rows below"
                            .to_string(),
                    };
                    if crate::chrome::chip(ui, nr.is_on(), "NR").on_hover_text(hover).clicked() {
                        let next = nr.next();
                        self.state.rx[0].noise_reduction = next; // optimistic echo
                        cmds.push(Command::SetNoiseReduction { rx: RxId::Main, level: next });
                    }
                } else {
                    self.nr_button(ui, cmds);
                }
            }
            RxChip::Bin => {
                // Binaural audio: the passband spread across the two ears, so
                // that pitch becomes direction. The hover text says what it is
                // for rather than what it does — an operator who has not met a
                // binaural receiver has no reason to guess that a stereo
                // effect is a copying aid — and it says something different in
                // CW and SSB, because what it buys is a different thing in
                // each.
                let on = self.state.rx[0].binaural;
                let sub = self.state.sub_rx_enabled;
                let cw = self.state.rx[0].mode == Mode::Cw;
                let chip = ui
                    .add_enabled_ui(!sub, |ui| crate::chrome::chip(ui, on && !sub, "BIN"))
                    .inner
                    .on_hover_text(if sub {
                        "Binaural audio — not while the sub receiver has the right ear"
                    } else if on {
                        "Binaural audio: the passband is spread across the two ears, so signals \
                         at different pitches come from different directions and the one you \
                         tune floats across. Click to go back to mono"
                    } else if cw {
                        "Binaural CW: spread the passband across the two ears, so that signals \
                         at different pitches come from different directions — a pile-up becomes \
                         several places instead of one crowded note, and tuning a station floats \
                         it across. Best on headphones"
                    } else {
                        "Binaural audio: spread the passband across the two ears. On voice the \
                         noise spreads over the whole image while the station stays in the \
                         middle of it, which is easier to listen to for an hour — at the cost \
                         of the voice itself being spread out. Best on headphones"
                    });
                if chip.clicked() {
                    self.state.rx[0].binaural = !on; // optimistic echo
                    cmds.push(Command::SetBinaural { rx: RxId::Main, on: !on });
                }
            }
            RxChip::Mute => {
                let muted = self.state.rx[0].muted;
                if crate::chrome::chip_accent(
                    ui,
                    muted,
                    "MUTE",
                    crate::theme::ALERT(),
                    Color32::WHITE,
                )
                .clicked()
                {
                    cmds.push(Command::SetMute { rx: RxId::Main, muted: !muted });
                }
            }
            RxChip::Rec => {
                // Two things can be recorded and they are not the same thing:
                // the audio of a QSO, and the band the receiver is hearing. The
                // chip lights while either is running and opens a picker for
                // both, rather than being a toggle for whichever one somebody
                // decided was the default (issue #217).
                let audio = self.state.recording;
                let iq = self.state.iq_recording;
                let rec = crate::chrome::chip_accent(
                    ui,
                    audio || iq,
                    "REC",
                    crate::theme::ALERT(),
                    Color32::WHITE,
                )
                .on_hover_text(
                    match (&self.state.recording_file, &self.state.iq_recording_file) {
                        (Some(a), Some(q)) => format!("Recording {a} and {q}"),
                        (Some(a), None) => format!("Recording audio to {a}"),
                        (None, Some(q)) => format!("Recording I/Q to {q}"),
                        (None, None) => "Record the audio, the raw I/Q, or both".to_string(),
                    },
                );
                self.rec_popup(ui, cmds, &rec);
            }
            RxChip::Stereo => {
                // WFM broadcast stereo: lit while a 19 kHz pilot is locked,
                // click to force mono. Only WFM has a pilot to find.
                let want = self.state.rx[0].wfm_stereo;
                let locked = self.meters.as_ref().is_some_and(|m| m.stereo);
                let hover = if !want {
                    "WFM stereo forced off — click for automatic stereo"
                } else if locked {
                    "WFM stereo: pilot locked. Click to force mono"
                } else {
                    "WFM stereo: automatic, no pilot on this station"
                };
                if crate::chrome::chip(ui, want && locked, "ST").on_hover_text(hover).clicked() {
                    self.state.rx[0].wfm_stereo = !want; // optimistic echo
                    cmds.push(Command::SetWfmStereo { rx: RxId::Main, on: !want });
                }
            }
            RxChip::Rds => {
                // RDS: what the station says about itself on its 57 kHz data
                // subcarrier. Lit while data is actually arriving, so the chip
                // answers "does this station carry it?" without opening
                // anything. Decoding runs whether or not the window is open.
                let rds = self.rds.as_ref().is_some_and(|r| r.sync);
                let hover = match self.rds.as_ref().and_then(|r| r.ps.clone()) {
                    Some(ps) if rds => format!("RDS: {ps}. Click for the station's data"),
                    _ if rds => "RDS data is arriving — click to see it".to_string(),
                    _ => "RDS — no data on this station. Click anyway for the decoder's \
                          diagnostics"
                        .to_string(),
                };
                if crate::chrome::chip(ui, rds, "RDS").on_hover_text(hover).clicked() {
                    self.show_rds = !self.show_rds;
                }
            }
            RxChip::Drm => {
                // DRM: lit once the decoder is actually producing audio, not merely
                // holding sync on a carrier, so the chip answers "is this station
                // being decoded?" at a glance. The window behind it says where the
                // chain stopped when the answer is no.
                let d = self.drm.as_ref();
                let decoding = d.is_some_and(|d| d.decoding());
                let hover = match d {
                    Some(d) if decoding && !d.service.label.is_empty() => {
                        format!("DRM: {}. Click for the broadcast's details", d.service.label)
                    }
                    Some(_) if decoding => "DRM is decoding — click for the details".to_string(),
                    // Locked and reading the multiplex, and silent: the one
                    // dark-chip state that is not a signal problem.
                    Some(d) if d.locked && !d.service.codec_supported => match d.service.codec {
                        Some(c) => format!(
                            "DRM: {} — locked, but its {} audio cannot be decoded here. \
                             Click for what is missing",
                            d.summary(),
                            c.label()
                        ),
                        None => format!("DRM: {} — click for the decoder's state", d.summary()),
                    },
                    Some(d) => format!("DRM: {} — click for the decoder's state", d.summary()),
                    None => "DRM — click for the decoder's state".to_string(),
                };
                if crate::chrome::chip(ui, decoding, "DRM").on_hover_text(hover).clicked() {
                    self.show_drm = !self.show_drm;
                }
            }
            RxChip::Tone => {
                // CTCSS/DCS: what is coming in, and optionally what has to be
                // present before the audio opens. Only NFM carries either.
                self.tone_button(ui, cmds);
            }
        }
    }

    /// The NR chip and the picker behind it: which denoiser, and how hard.
    /// Fades out on its own, like the tone popup.
    ///
    /// The chip's label is the bare "NR" whatever is running: lit or not, it
    /// answers "is noise reduction in circuit?" and nothing else. A label that
    /// grew to "NR DFNR High" and shrank to "NR" changed width — and so moved
    /// every chip beside it — each time the engine or the strength changed.
    /// Which of the two is running is one click away, in the picker itself.
    fn nr_button(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let nr = self.state.rx[0].noise_reduction;
        let hover = match nr.engine() {
            Some(e) => format!(
                "Noise reduction: {} — {}\n\nClick to change engine or strength",
                nr.label(),
                e.name()
            ),
            None => "Noise reduction (voice), off — click to pick an engine".to_string(),
        };
        let btn = crate::chrome::chip(ui, nr.is_on(), "NR").on_hover_text(hover);

        let popup_id = egui::Popup::default_response_id(&btn);
        let now = ui.input(|i| i.time);
        let alpha =
            crate::chrome::popup_fade_alpha(ui.ctx(), popup_id, now, &mut self.nr_popup_since);
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                self.nr_controls(ui, cmds);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            // Hovering the popup keeps it up: the fade is for a menu left open
            // and forgotten, not one being read.
            if r.response.contains_pointer() {
                self.nr_popup_since = Some(now);
            }
        }
    }

    /// What the REC chip opens: one row per thing that can be recorded.
    ///
    /// A popup rather than a toggle because there are two answers and neither
    /// is the obvious one — an operator archiving a QSO wants the audio, one
    /// capturing a band to work on offline wants the I/Q, and the second is not
    /// reachable at all from a button that does the first (issue #217).
    fn rec_popup(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, btn: &egui::Response) {
        let popup_id = egui::Popup::default_response_id(btn);
        let now = ui.input(|i| i.time);
        let alpha =
            crate::chrome::popup_fade_alpha(ui.ctx(), popup_id, now, &mut self.rec_popup_since);
        let resp = egui::Popup::from_toggle_button_response(btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                ui.set_max_width(300.0);
                self.rec_controls(ui, cmds);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            if r.response.contains_pointer() {
                self.rec_popup_since = Some(now);
            }
        }
    }

    /// The rows inside the REC popup. Split out for the same reason
    /// [`Self::nr_controls`] is: a menu inlines them.
    fn rec_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let audio = self.state.recording;
        let iq = self.state.iq_recording;

        crate::chrome::menu_caption(ui, "Audio");
        ui.horizontal_wrapped(|ui| {
            if crate::chrome::chip_accent(
                ui,
                audio,
                if audio { "● MP3" } else { "MP3" },
                crate::theme::ALERT(),
                Color32::WHITE,
            )
            .on_hover_text(
                "What you hear, as an MP3: the receiver in one channel and your own transmit in \
                 the other. This is the recording of a QSO.",
            )
            .clicked()
            {
                cmds.push(Command::SetRecording(!audio));
            }
            // Channel layout is a property of the *next* recording — the
            // encoder is initialised with it — so it greys out while one runs.
            let mono = self.state.recording_mono;
            let mono_chip = ui
                .add_enabled_ui(!audio, |ui| crate::chrome::chip(ui, mono, "MONO"))
                .inner
                .on_hover_text(if mono {
                    "Mixed to one channel — click for two"
                } else {
                    "Two channels: RX left, TX right — click for one mixed channel"
                });
            if mono_chip.clicked() {
                cmds.push(Command::SetRecordingMono(!mono));
            }
        });
        if let Some(f) = &self.state.recording_file {
            ui.label(RichText::new(f).size(9.5).color(crate::theme::CYAN_DIM()));
        }

        crate::chrome::menu_caption(ui, "Spectrum");
        // A demod-audio radio hands over audio and no I/Q, so there is nothing
        // for this to write. Said on the chip rather than hidden: an operator
        // looking for the feature has to find out that this radio has not got
        // it, not that sdroxide has not.
        let have_iq = self.state.sample_rate > 96_000.0;
        ui.horizontal_wrapped(|ui| {
            let chip = ui
                .add_enabled_ui(have_iq || iq, |ui| {
                    crate::chrome::chip_accent(
                        ui,
                        iq,
                        if iq { "● I/Q WAV" } else { "I/Q WAV" },
                        crate::theme::ALERT(),
                        Color32::WHITE,
                    )
                })
                .inner
                .on_hover_text(if have_iq || iq {
                    "The raw spectrum the receiver is delivering, as a stereo 32-bit float WAV \
                     (RF64 past 4 GB) that SDR#, SDRuno, HDSDR and SDRangel open — and that \
                     sdroxide itself plays back with --file. Large: 8 bytes a sample."
                } else {
                    "This radio hands over demodulated audio, so there is no I/Q to record."
                });
            if chip.clicked() {
                cmds.push(Command::SetIqRecording(!iq));
            }
            if iq {
                let mb = self.state.iq_recording_mb;
                let rate = self.state.sample_rate.max(1.0);
                let secs = f64::from(mb) * f64::from(1u32 << 20) / (rate * 8.0);
                ui.label(
                    RichText::new(format!("{mb} MB · {:.0}:{:02}", secs / 60.0, secs as u64 % 60))
                        .size(9.5)
                        .color(crate::theme::ALERT()),
                );
            } else if have_iq {
                // The bill, before it is run up rather than after: at 2.4 Msps
                // this is a gigabyte a minute.
                let mbs = self.state.sample_rate * 8.0 / f64::from(1u32 << 20);
                ui.label(
                    RichText::new(format!("{mbs:.0} MB/s"))
                        .size(9.5)
                        .color(crate::theme::CYAN_DIM()),
                )
                .on_hover_text("What a capture costs at this sample rate");
            }
        });
        if let Some(f) = &self.state.iq_recording_file {
            ui.label(RichText::new(f).size(9.5).color(crate::theme::CYAN_DIM()));
        }
    }

    /// The engine row and the strength row. Its own function because a menu has
    /// to inline this rather than open it as a popup — see [`Self::rx_controls`].
    fn nr_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let nr = self.state.rx[0].noise_reduction;
        let pick = |app: &mut Self, cmds: &mut Vec<Command>, level: NrLevel| {
            app.state.rx[0].noise_reduction = level; // optimistic echo
            cmds.push(Command::SetNoiseReduction { rx: RxId::Main, level });
        };

        crate::chrome::menu_caption(ui, "Engine");
        ui.horizontal_wrapped(|ui| {
            if crate::chrome::chip(ui, !nr.is_on(), "OFF")
                .on_hover_text("No noise reduction — the decoders never saw it anyway")
                .clicked()
            {
                pick(self, cmds, NrLevel::Off);
            }
            for e in NrEngine::ALL {
                if crate::chrome::chip(ui, nr.engine() == Some(e), e.tag())
                    .on_hover_text(e.name())
                    .clicked()
                {
                    pick(self, cmds, nr.with_engine(e));
                }
            }
        });

        crate::chrome::menu_caption(ui, "Strength");
        ui.horizontal_wrapped(|ui| {
            for st in NrStrength::ALL {
                // These work with NR off too: they switch it on at that strength
                // on RNNoise, which is what reaching for "Med" on a dead chip
                // means.
                if crate::chrome::chip(ui, nr.strength() == Some(st), st.label()).clicked() {
                    pick(self, cmds, nr.with_strength(st));
                }
            }
        });
    }

    /// The sub-audible readout: the CTCSS tone or DCS code being received, and a
    /// popup to require one before the audio gate opens.
    ///
    /// The chip shows what is *heard* in preference to what is *armed*, because
    /// on a monitoring receiver the tone is mostly a label — it says which
    /// repeater or system you are listening to — and the armed code is
    /// something you set once and then stop thinking about.
    fn tone_button(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let heard = self.meters.as_ref().and_then(|m| m.tone);
        let armed = self.state.rx[0].tone_sql;
        let label = match (heard, armed) {
            (Some(t), _) => t.label(),
            // Armed but silent: the dot marks it as a requirement, not a decode.
            (None, Some(t)) => format!("·{}", t.label()),
            (None, None) => "TONE".to_string(),
        };
        let hover = match (heard, armed) {
            (Some(h), Some(a)) if h == a => {
                format!("Receiving {}, which is the tone squelch — audio open", h.label())
            }
            (Some(h), Some(a)) => format!(
                "Receiving {}, but the tone squelch wants {} — audio stays closed",
                h.label(),
                a.label()
            ),
            (Some(h), None) => format!("Receiving CTCSS/DCS {}", h.label()),
            (None, Some(a)) => {
                format!("Tone squelch {}: nothing matching it is being received", a.label())
            }
            (None, None) => "CTCSS / DCS — no sub-audible tone on this signal".to_string(),
        };
        let btn = match armed {
            // Yellow while a gate is armed, so a silent receiver reads as
            // "waiting for its tone" rather than as a dead one.
            Some(_) => crate::chrome::chip_accent(
                ui,
                heard == armed,
                label,
                crate::theme::YELLOW(),
                crate::theme::INK_ON_BRIGHT(),
            ),
            None => crate::chrome::chip(ui, heard.is_some(), label),
        }
        .on_hover_text(hover);

        let popup_id = egui::Popup::default_response_id(&btn);
        let now = ui.input(|i| i.time);
        let alpha =
            crate::chrome::popup_fade_alpha(ui.ctx(), popup_id, now, &mut self.tone_popup_since);
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                crate::chrome::menu_caption(ui, "Tone squelch");
                self.tone_controls(ui, cmds, heard, armed);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            if r.response.contains_pointer() {
                self.tone_popup_since = Some(now);
            }
        }
    }

    /// The tone picker: off, the tone being received, then the 50 CTCSS tones
    /// and the 104 DCS codes in each polarity.
    fn tone_controls(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        heard: Option<SubTone>,
        armed: Option<SubTone>,
    ) {
        let mut pick: Option<Option<SubTone>> = None;
        ui.horizontal(|ui| {
            if crate::chrome::chip(ui, armed.is_none(), "OFF")
                .on_hover_text("Carrier squelch: open on any signal")
                .clicked()
            {
                pick = Some(None);
            }
            // The shortcut that matters in practice — you are listening to a
            // repeater, it is sending its tone, and you want only that.
            if let Some(h) = heard {
                if armed != Some(h)
                    && crate::chrome::chip(ui, false, format!("USE {}", h.label()))
                        .on_hover_text("Require the tone currently being received")
                        .clicked()
                {
                    pick = Some(Some(h));
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            ui.label(RichText::new("CTCSS").size(10.0).color(crate::theme::CYAN_DIM()));
            egui::Grid::new("ctcss-grid").spacing([3.0, 3.0]).show(ui, |ui| {
                for (i, &tenths) in sdroxide_types::CTCSS_TONES.iter().enumerate() {
                    let t = SubTone::Ctcss(tenths);
                    if crate::chrome::chip(ui, armed == Some(t), t.label()).clicked() {
                        pick = Some(Some(t));
                    }
                    if i % 10 == 9 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new("DCS").size(10.0).color(crate::theme::CYAN_DIM()));
            if crate::chrome::chip(ui, armed == Some(SubTone::Dcs), "ANY DCS")
                .on_hover_text(
                    "Open on any DCS-coded signal. Which of the 104 codes it carries cannot be \
                     read reliably here, so there is nothing finer to choose",
                )
                .clicked()
            {
                pick = Some(Some(SubTone::Dcs));
            }
        });
        if let Some(tone) = pick {
            self.state.rx[0].tone_sql = tone; // optimistic echo
            cmds.push(Command::SetToneSquelch { rx: RxId::Main, tone });
        }
    }

    /// The sub receiver's own controls, shown only while it is running. The sub
    /// has a frequency, a mode and a filter of its own — none of which the main
    /// receiver's controls can reach — so without this module it is a second
    /// receiver that can only be switched on and off.
    fn sub_rx_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        // Width past the design size goes to the frequency field on the top
        // row and the volume rail on the bottom one — one growing control per
        // row, so both rows widen by the same amount.
        let extra = (w - SUB_W).clamp(0.0, RAIL_STRETCH_MAX);
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                self.sub_controls(ui, cmds, false, extra);
            });
        });
    }

    /// The sockets the radio will put its receiver on, or empty where it has
    /// no selector to offer. Only worth a control where there is a choice.
    fn rig_antennas(&self) -> &[String] {
        match &self.caps {
            Some(c) if c.antennas_rx.len() > 1 => &c.antennas_rx,
            _ => &[],
        }
    }

    /// Whether the radio's own power switch can be reached from here.
    fn rig_power(&self) -> bool {
        self.caps.as_ref().is_some_and(|c| c.commands_rig_power)
    }

    /// Whether the radio has a separate receiving antenna to switch in and out
    /// of circuit, and which way it is set. A different thing from the socket
    /// chip beside it: this one leaves the main aerial on transmit throughout.
    fn rig_rx_antenna(&self) -> Option<bool> {
        self.caps.as_ref().filter(|c| c.has_rx_antenna).map(|_| self.state.rx_antenna)
    }

    /// Whether the RIG box has anything to carry. Like DIV and SUB, it appears
    /// only for hardware that has what it drives — a strip is too narrow to
    /// hold controls for a radio that would ignore them.
    fn rig_box_shown(&self) -> bool {
        !self.rig_antennas().is_empty() || self.rig_rx_antenna().is_some() || self.rig_power()
    }

    /// The RIG box's natural width: the wider of its two rows.
    fn rig_rows_w(&self, ui: &egui::Ui) -> f32 {
        let gap = MODULE_ROW_SPACING;
        let body = egui::TextStyle::Body.resolve(ui.style());
        let ants = self.rig_antennas();
        // The aerial row: the label, the socket chip where there is a choice of
        // sockets, and the receiving antenna's own chip where there is one.
        // Both are optional and either may be alone — an IC-7300MK2 has one
        // socket and a receiving antenna, an IC-7700 the other way about.
        let mut top = 0.0f32;
        if !ants.is_empty() {
            // The chip wears whichever socket the radio is on, so the box has
            // to be as wide as the longest of them or it would resize as the
            // operator switched.
            let widest =
                ants.iter().fold(0.0f32, |a, n| a.max(crate::chrome::chip_width(ui, n, None)));
            top += gap + widest;
        }
        if self.rig_rx_antenna().is_some() {
            top += gap + crate::chrome::chip_width(ui, RX_ANT_LABEL, None);
        }
        if top > 0.0 {
            top += crate::chrome::text_width(ui, "ANT", body.clone());
        }
        let bottom = if self.rig_power() {
            crate::chrome::text_width(ui, "PWR", body)
                + gap
                + crate::chrome::chip_width(ui, "ON", None)
                + gap
                + crate::chrome::chip_width(ui, "OFF", None)
        } else {
            0.0
        };
        top.max(bottom) + 2.0 * crate::chrome::MODULE_MARGIN_X
    }

    /// The RIG box: the radio's aerial socket and its power switch.
    fn rig_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                self.rig_controls(ui, cmds, false);
            });
        });
    }

    /// The radio's own switches — the body of the RIG box, and of the RIG
    /// menu. See [`crate::chrome::control_row`] for `narrow`.
    ///
    /// The socket is a cycling chip rather than a combo, for the same reason
    /// the AGC and DIV chips are: a combo inside a menu opens a second popup
    /// layer, and clicking it counts as "outside" and closes the menu it was
    /// opened from. Two or three sockets is hardly a walk.
    ///
    /// The power switch is two buttons rather than a toggle, because nothing
    /// here *reads* it: a radio that is off answers nothing, so the only thing
    /// a toggle could show is the last thing it was told. Same reasoning as the
    /// pair in Settings → Radio, which these do not replace.
    fn rig_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, narrow: bool) {
        let ants: Vec<String> = self.rig_antennas().to_vec();
        let rx_ant = self.rig_rx_antenna();
        // One row for the aerials, whichever of the two the radio has. It stays
        // one row on purpose: the box is two rows tall and the power switch
        // owns the other.
        if !ants.is_empty() || rx_ant.is_some() {
            crate::chrome::control_row(ui, narrow, |ui| {
                ui.label("ANT").on_hover_text(
                    "Which socket on the back the radio is receiving on — its own ANT \
                     command, the same setting as the ANT button on the front panel. \
                     Click to step to the next one.\n\n\
                     The choice is remembered per band, and put back the next time the \
                     dial crosses into that band.",
                );
                if !ants.is_empty() {
                    let here = ants.iter().position(|a| *a == self.state.antenna_rx);
                    // The socket's own name, whole: a front end that spells its
                    // ports out is entitled to be quoted — an RSPduo's "50 Ohm
                    // port" and "Hi-Z port" abbreviate to the same word, and a
                    // chip that cannot tell two sockets apart is worse than a
                    // wide one. The box was measured against the longest.
                    let label = match here {
                        Some(i) => ants[i].clone(),
                        // Before the radio has said, and after a switch to a
                        // socket this list does not name.
                        None => "—".to_string(),
                    };
                    if crate::chrome::chip(ui, true, label)
                        .on_hover_text(format!("Sockets: {}", ants.join(", ")))
                        .clicked()
                    {
                        let next = ants[here.map_or(0, |i| (i + 1) % ants.len())].clone();
                        self.state.antenna_rx = next.clone(); // optimistic echo
                        cmds.push(Command::SetAntenna { dir: Direction::Rx, name: next });
                    }
                }
                // The separate receiving antenna, where the radio has one: a
                // toggle rather than a choice of sockets, because that is what
                // it is — an extra input switched into the receive path, with
                // the main aerial left on transmit throughout. Lit while it is
                // in circuit, like every other chip that names a state.
                if let Some(on) = rx_ant
                    && crate::chrome::chip(ui, on, RX_ANT_LABEL)
                        .on_hover_text(
                            "The radio's separate receiving antenna, switched into the \
                             receive path or out of it — its own RX ANT setting. The \
                             aerial on the main socket stays on transmit either way.\n\n\
                             The radio remembers this per band itself, so sdroxide reads \
                             it back after every band change rather than putting back \
                             what it last saw: clicking here is the only thing that \
                             moves it.",
                        )
                        .clicked()
                {
                    self.state.rx_antenna = !on; // optimistic echo
                    cmds.push(Command::SetRxAntenna(!on));
                }
            });
        }
        if self.rig_power() {
            crate::chrome::control_row(ui, narrow, |ui| {
                ui.label("PWR").on_hover_text(
                    "The radio's own power switch, over the control link, and the one true \
                     on/off in the program — not sdroxide's LINK switch, which closes \
                     sdroxide's end and leaves the radio running.\n\n\
                     For ON to reach anything the radio's control end has to stay awake \
                     while it is off: Network Control over the LAN, or a CI-V port still \
                     fed from the mains on a set switched off at the front.",
                );
                if crate::chrome::chip(ui, false, "ON")
                    .on_hover_text("Switch the radio on")
                    .clicked()
                {
                    cmds.push(Command::SetRigPower(true));
                }
                if crate::chrome::chip(ui, false, "OFF")
                    .on_hover_text("Switch the radio off. The audio and the meters stop with it.")
                    .clicked()
                {
                    cmds.push(Command::SetRigPower(false));
                }
            });
        }
    }

    /// Whether two coherent aerials are being combined into the span on
    /// screen — a LimeSDR's two chains, an RSPduo's two tuners. The source
    /// says so; nothing here has to know which board it is.
    fn has_diversity(&self) -> bool {
        self.caps.as_ref().is_some_and(|c| c.diversity)
    }

    /// The filter's settings, from the radio's own configuration.
    ///
    /// Fetched here rather than waiting for the settings dialog to be opened:
    /// these controls are on the strip precisely so that nobody has to open
    /// it. One read of one small file, once, on a radio that has a filter.
    fn div_cfg(&mut self) -> Option<(DiversityMode, f32, bool)> {
        if self.radio_cfg.is_none() {
            self.radio_cfg = self.ctrl.radio_config();
        }
        let cfg = self.radio_cfg.as_ref()?;
        match cfg.backend {
            sdroxide_types::Backend::Lime => {
                Some((cfg.lime.aux.mode, cfg.lime.aux.rate, cfg.lime.aux.frozen))
            }
            sdroxide_types::Backend::SdrPlay => {
                Some((cfg.sdrplay.duo.mode, cfg.sdrplay.duo.rate, cfg.sdrplay.duo.frozen))
            }
            // Fobos's own filter only exists on FobosPort::HfDual — every
            // other port has nothing to combine, and `caps.diversity` (set
            // from that same port choice) already keeps the box off screen
            // for them, so no port check is needed here too.
            sdroxide_types::Backend::Fobos => {
                Some((cfg.fobos.div_mode, cfg.fobos.div_rate, cfg.fobos.div_frozen))
            }
            // Every other interface with a second receiver keeps them apart.
            _ => None,
        }
    }

    /// Change the filter and remember the change.
    ///
    /// Two messages, and both are wanted: the pseudo-gain reaches the running
    /// filter now, and the configuration is what it comes back as after a
    /// reconnect. Saving without reopening is exactly what the settings
    /// dialog's own live controls do.
    fn div_edit(&mut self, cmds: &mut Vec<Command>, element: &str, db: f64) {
        cmds.push(Command::SetGain { dir: Direction::Rx, element: element.to_string(), db });
        // A restart is momentary — there is nothing about it to remember.
        if element == DIV_RESET_ELEMENT {
            return;
        }
        let Some(cfg) = self.radio_cfg.as_mut() else { return };
        let (mode, rate, frozen) = match cfg.backend {
            sdroxide_types::Backend::Lime => {
                let a = &mut cfg.lime.aux;
                (&mut a.mode, &mut a.rate, &mut a.frozen)
            }
            sdroxide_types::Backend::SdrPlay => {
                let d = &mut cfg.sdrplay.duo;
                (&mut d.mode, &mut d.rate, &mut d.frozen)
            }
            sdroxide_types::Backend::Fobos => {
                (&mut cfg.fobos.div_mode, &mut cfg.fobos.div_rate, &mut cfg.fobos.div_frozen)
            }
            _ => return,
        };
        match element {
            DIV_MODE_ELEMENT => {
                *mode = if db >= 0.5 { DiversityMode::Combine } else { DiversityMode::Cancel }
            }
            DIV_RATE_ELEMENT => *rate = db as f32,
            DIV_FREEZE_ELEMENT => *frozen = db >= 0.5,
            _ => return,
        }
        cmds.push(Command::SetRadioConfig { cfg: Box::new(cfg.clone()), reopen: false });
    }

    /// The DIV box: the diversity filter, on the strip because it is worked
    /// while listening (issue #165).
    fn div_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        let extra = (w - div_rows_w(ui)).clamp(0.0, RAIL_STRETCH_MAX);
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                ui.spacing_mut().slider_width = STRIP_RAIL_W + extra;
                self.div_controls(ui, cmds, false);
            });
        });
    }

    /// The diversity filter's controls — the body of the DIV box, and of the
    /// DIV menu. See [`crate::chrome::control_row`] for `narrow`.
    ///
    /// Which way it combines, how fast it chases, and holding it: the three
    /// an operator reaches for with the waterfall in front of them. Everything
    /// else about the filter — how many taps, what the second aerial's gain
    /// is — is set once and left, and stays in Settings → Radio.
    fn div_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, narrow: bool) {
        let Some((mode, rate, frozen)) = self.div_cfg() else { return };
        crate::chrome::control_row(ui, narrow, |ui| {
            ui.label(RichText::new("DIV").size(11.0).strong());
            // A cycling chip rather than a combo, for the same reason as the
            // AGC chip: a combo inside a menu opens a second popup layer, and
            // clicking it counts as "outside" and closes the menu it was
            // opened from. Two settings is hardly a walk.
            let combine = mode == DiversityMode::Combine;
            if crate::chrome::chip(ui, combine, DIV_MODE_LABELS[usize::from(combine)])
                .on_hover_text(
                    "What the second aerial is for — click to swap.\n\n\
                     CANCEL subtracts it from the first, in the gain, phase and delay that \
                     make a local noise source line up on both: the DSP form of a \
                     noise-cancelling phaser. COMBINE adds the two in the phase that \
                     reinforces, weighted by how well each hears — diversity reception, \
                     which fills in fades.",
                )
                .clicked()
            {
                self.div_edit(cmds, DIV_MODE_ELEMENT, f64::from(u8::from(!combine)));
            }
            if crate::chrome::chip(ui, frozen, "HOLD")
                .on_hover_text(
                    "Stop the filter moving. Reach for this the moment a null appears: a \
                     filter left adapting will re-aim itself at whatever becomes loudest, \
                     which on a quiet band is the station you are listening to.",
                )
                .clicked()
            {
                self.div_edit(cmds, DIV_FREEZE_ELEMENT, f64::from(u8::from(!frozen)));
            }
            if crate::chrome::chip(ui, false, "RESTART")
                .on_hover_text("Zero the filter and find the null again.")
                .clicked()
            {
                self.div_edit(cmds, DIV_RESET_ELEMENT, 1.0);
            }
        });
        crate::chrome::control_row(ui, narrow, |ui| {
            ui.label("Adapt").on_hover_text(
                "How fast the filter chases: slow and steady at the left, converging inside \
                 a fraction of a second and visibly hunting at the right. Start fast to find \
                 the null, then HOLD it.",
            );
            let mut v = rate;
            if crate::chrome::slider(ui, Slider::new(&mut v, 0.0..=1.0).show_value(false)).changed()
            {
                self.div_edit(cmds, DIV_RATE_ELEMENT, f64::from(v));
            }
        });
    }

    /// The sub receiver's controls — the body of the SUB box, and of the SUB
    /// menu. See [`crate::chrome::control_row`] for `narrow`; `extra` widens
    /// the frequency field and the volume rail, and the popup passes 0.
    fn sub_controls(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        narrow: bool,
        extra: f32,
    ) {
        // The sub tunes anywhere inside the device passband and nowhere outside
        // it: both receivers are DDCs on the same IQ stream.
        let half = self.state.sample_rate / 2.0;
        let (dev_lo, dev_hi) = (self.state.center_hz - half, self.state.center_hz + half);
        // Field height, and the height every row is told to be. egui sizes a
        // horizontal row from `interact_size.y` and then grows it as taller
        // widgets land in it — which drops everything added after the first
        // chip a few pixels below everything added before it. Starting the row
        // at the height its tallest widget will be leaves nothing to grow.
        // A touched layout has already raised `interact_size` to a fingertip;
        // don't shrink it back down.
        let field_h = ui.spacing().interact_size.y.max(22.0);
        ui.spacing_mut().interact_size.y = field_h;
        // Frequency, mode, and the two moves worth a single click:
        // send the sub to the dial, or bring the dial to the sub.
        crate::chrome::control_row(ui, narrow, |ui| {
            ui.label(
                RichText::new("SUB")
                    .color(crate::widgets::spectrum_view::SUB_COLOR)
                    .size(11.0)
                    .strong(),
            );
            let mut hz = self.state.sub_rx_hz;
            let resp = ui
                .add_sized(
                    [116.0 + extra, field_h],
                    DragValue::new(&mut hz)
                        .speed(10.0)
                        .range(dev_lo..=dev_hi)
                        // Typed and shown in MHz — the unit the operator
                        // reads a frequency in — while the drag step
                        // stays in Hz so it tunes like a dial.
                        .custom_formatter(|v, _| format!("{:.6}", v / 1e6))
                        .custom_parser(|s| s.trim().parse::<f64>().ok().map(|m| m * 1e6))
                        .suffix(" MHz"),
                )
                .on_hover_text(
                    "Where the sub receiver listens. Shift-click the waterfall, or \
                             drag inside the sub's passband, to move it.",
                );
            if resp.changed() {
                self.state.sub_rx_hz = hz; // optimistic echo
                cmds.push(Command::SetSubRxFreq(hz));
            }
            if let Some(m) = sub_mode_picker(ui, self.state.rx[1].mode, narrow) {
                cmds.push(Command::SetMode { rx: RxId::Sub, mode: m });
            }
            if crate::chrome::chip(ui, false, "←DIAL")
                .on_hover_text("Move the sub receiver to the main dial")
                .clicked()
            {
                cmds.push(Command::SetSubRxFreq(self.state.rx_freq_hz()));
            }
            if crate::chrome::chip(ui, false, "DIAL←")
                .on_hover_text("Move the main dial to the sub receiver")
                .clicked()
            {
                cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz: self.state.sub_rx_hz });
            }
        });
        // Filter, level, mute.
        crate::chrome::control_row(ui, narrow, |ui| {
            let rx1 = self.state.rx[1];
            let max = rx1.mode.max_filter_hz();
            ui.label("Filter").on_hover_text("Sub receiver passband edges, in Hz");
            let mut lo = rx1.filter_lo;
            let mut hi = rx1.filter_hi;
            let lo_changed = ui
                .add_sized([70.0, field_h], DragValue::new(&mut lo).speed(10).range(-max..=max))
                .changed();
            let hi_changed = ui
                .add_sized([70.0, field_h], DragValue::new(&mut hi).speed(10).range(-max..=max))
                .changed();
            if lo_changed || hi_changed {
                let (lo, hi) = if rx1.mode.filter_symmetric() {
                    // A channel about the carrier: whichever edge was typed
                    // sets the half width and the other follows (issue #256),
                    // the same rule the panadapter grips follow.
                    let half = if hi_changed { hi.abs() } else { lo.abs() }.clamp(25.0, max);
                    (-half, half)
                } else {
                    // Same 50 Hz floor the waterfall grips enforce, so the
                    // passband can't be dragged shut from either route.
                    (lo.min(hi - 50.0), hi.max(lo + 50.0))
                };
                (self.state.rx[1].filter_lo, self.state.rx[1].filter_hi) = (lo, hi);
                cmds.push(Command::SetFilter { rx: RxId::Sub, lo, hi });
            }
            let mut vol = rx1.volume;
            ui.label("Vol").on_hover_text("Sub receiver level (it plays in the right ear)");
            if ui
                .scope(|ui| {
                    ui.spacing_mut().slider_width = 64.0 + extra;
                    crate::chrome::slider(ui, Slider::new(&mut vol, 0.0..=1.0).show_value(false))
                })
                .inner
                .changed()
            {
                self.state.rx[1].volume = vol; // optimistic echo
                cmds.push(Command::SetVolume { rx: RxId::Sub, v: vol });
            }
            if crate::chrome::chip_accent(
                ui,
                rx1.muted,
                "MUTE",
                crate::theme::ALERT(),
                Color32::WHITE,
            )
            .clicked()
            {
                cmds.push(Command::SetMute { rx: RxId::Sub, muted: !rx1.muted });
            }
        });
    }

    /// The PTT latching chip, red even at rest — it is the transmit control,
    /// and the one chip in the strip whose accident matters. Only the desktop
    /// draws it here: a compact layout keys the transmitter from the menu row
    /// instead, where it is one tap away rather than two — burying
    /// push-to-talk in a menu is not a thing to do to an operator.
    fn tx_ptt_chip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let tx = self.state.tx;
        let label = RichText::new(" PTT ").size(15.0).strong();
        // Tinted only while idle: keyed, the chip fills red and the label
        // needs its white ink (`chip_enabled_tinted` is the precedent).
        let label = if tx.ptt { label } else { label.color(crate::theme::ALERT()) };
        if crate::chrome::chip_accent_sized(
            ui,
            tx.ptt,
            label,
            crate::theme::ALERT(),
            Color32::WHITE,
            tx_key_chip_size(ui),
        )
        .clicked()
        {
            cmds.push(Command::SetPtt(!tx.ptt));
        }
    }

    /// The TUNE latching chip: keys a carrier at the tune level drawn beside
    /// it.
    fn tx_tune_chip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let tx = self.state.tx;
        if crate::chrome::chip_accent_sized(
            ui,
            tx.tune,
            RichText::new(" TUNE ").size(15.0),
            crate::theme::YELLOW(),
            crate::theme::INK_ON_CYAN(),
            tx_key_chip_size(ui),
        )
        .clicked()
        {
            cmds.push(Command::SetTune(!tx.tune));
        }
    }

    /// The voice keyer's chip, where the keyer can transmit — every voice mode
    /// plus RADE, which takes a message as its microphone.
    fn tx_keyer_chip(&mut self, ui: &mut egui::Ui) {
        if !self.state.rx[0].mode.allows_voice_keyer() {
            return;
        }
        // Lit while a message is on the air, so the button doubles as
        // the "something is transmitting from the keyer" indicator.
        let playing = self.voice.playing.is_some();
        let hover = match self.voice.playing {
            Some(i) => format!(
                "Transmitting {} — click to open the voice keyer",
                sdroxide_types::slot_label(i as usize, &self.voice.slot(i as usize).name)
            ),
            None => "Voice keyer: record and transmit stored messages".to_string(),
        };
        if crate::chrome::chip_accent(
            ui,
            playing || self.show_voice,
            RichText::new(" ▶ ").size(15.0),
            if playing { crate::theme::ALERT() } else { crate::theme::CYAN() },
            if playing { Color32::WHITE } else { crate::theme::INK_ON_CYAN() },
        )
        .on_hover_text(hover)
        .clicked()
        {
            self.show_voice = !self.show_voice;
        }
    }

    /// The Drive label + rail + readout.
    fn tx_drive(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut drive = self.state.tx.drive;
        ui.label("Drive");
        if crate::chrome::slider(
            ui,
            Slider::new(&mut drive, 0.0..=1.0)
                .show_value(true)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
        {
            cmds.push(Command::SetTxDrive(drive));
        }
    }

    /// The tune-carrier level: label + rail + readout.
    fn tx_tune_level(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut tune_drive = self.state.tx.tune_drive;
        ui.label("Tune");
        if crate::chrome::slider(
            ui,
            Slider::new(&mut tune_drive, 0.0..=1.0)
                .show_value(true)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
        {
            cmds.push(Command::SetTuneDrive(tune_drive));
        }
    }

    /// The Mic gain: label + rail, no readout.
    fn tx_mic(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut mic = self.state.tx.mic_gain;
        ui.label("Mic");
        if crate::chrome::slider(ui, Slider::new(&mut mic, 0.0..=1.0).show_value(false)).changed() {
            cmds.push(Command::SetMicGain(mic));
        }
    }

    /// Whether the envelope processor is the live control for what is on the
    /// air — and so whether either transmit surface should offer it.
    ///
    /// Two things have to hold, and each rules out a rail that would do
    /// nothing:
    ///
    /// - the mode is voice single sideband. Every digital mode carries its
    ///   information in the very envelope this processor flattens, so there is
    ///   nothing for it to do in one;
    /// - and we make that sideband ourselves. A radio that modulates the audio
    ///   we send it (a CAT rig on its sound card, a FLEX, an Icom on its
    ///   network port) builds the envelope in its own DSP, downstream of
    ///   anything this end can do to it — which is the same split
    ///   [`digi_tx_level_applies_to`] turns on, read the other way round, and
    ///   the same one the engine builds its envelope processor behind: no
    ///   modulator this end, no `Cessb` in the chain.
    ///
    /// A control that is present but inert is a control an operator will spend
    /// an evening turning up.
    fn cessb_applies(&self) -> bool {
        cessb_applies_to(self.state.rx[0].mode, self.caps.as_ref())
    }

    /// The hover that explains the envelope processor wherever it is drawn.
    fn cessb_hover(&self) -> String {
        format!(
            "Controlled-envelope SSB: more average power for the same peak, without \
             splatter — three or four decibels of apparent loudness at the far end. \
             How many decibels the voice is driven into the processor; 0 is off, 6 is \
             a sensible first try. It does not touch Drive.\n\nNow: {}",
            cessb_value_text(self.state.tx.cessb_db)
        )
    }

    /// Controlled-envelope SSB: label + rail, in decibels of compression.
    ///
    /// A single number, because that is the control: with nothing driven into
    /// the envelope processor it cannot do anything, so "how much" already
    /// answers "whether", and there is no switch to leave in the wrong position.
    fn tx_cessb(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut db = self.state.tx.cessb_db;
        let hover = self.cessb_hover();
        ui.label("CESSB").on_hover_text(hover.clone());
        if crate::chrome::slider(
            ui,
            Slider::new(&mut db, 0.0..=sdroxide_types::CESSB_MAX_DB)
                .show_value(true)
                .custom_formatter(|v, _| cessb_value_text(v as f32)),
        )
        .on_hover_text(hover)
        .changed()
        {
            cmds.push(Command::SetCessb(db));
        }
    }

    /// The envelope processor as a vertical rail with its name above, in the
    /// column beside the mic rail — the shape the condensed box has room for.
    ///
    /// Named rather than captioned with its own readout, unlike the
    /// transmit-audio rail it stands beside: "CESSB" is not a word anyone
    /// guesses from a number, the rail's own position already says off from
    /// on, and the figure is one hover away. The whole control costs the box
    /// [`TX_CESSB_COL_W`] of width, and only in the modes that can use it.
    fn tx_cessb_vertical(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut db = self.state.tx.cessb_db;
        let hover = self.cessb_hover();
        let mut set = None;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new("CESSB").size(10.5)).on_hover_text(hover.clone());
            // The rail takes whatever height the label left it.
            ui.spacing_mut().slider_width = (ui.available_height() - 2.0).max(24.0);
            if crate::chrome::slider(
                ui,
                Slider::new(&mut db, 0.0..=sdroxide_types::CESSB_MAX_DB)
                    .vertical()
                    .show_value(false),
            )
            .on_hover_text(hover)
            .changed()
            {
                set = Some(db);
            }
        });
        if let Some(db) = set {
            cmds.push(Command::SetCessb(db));
        }
    }

    /// The Mic gain as a vertical rail with its label above — the whole
    /// control costs the condensed TX box [`TX_MIC_COL_W`] of width instead of
    /// a third slider row's worth.
    fn tx_mic_vertical(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new("Mic").size(10.5));
            let mut mic = self.state.tx.mic_gain;
            // The rail takes whatever height the label left it.
            ui.spacing_mut().slider_width = (ui.available_height() - 2.0).max(24.0);
            if crate::chrome::slider(
                ui,
                Slider::new(&mut mic, 0.0..=1.0).vertical().show_value(false),
            )
            .on_hover_text("Microphone gain")
            .changed()
            {
                cmds.push(Command::SetMicGain(mic));
            }
        });
    }

    /// Whether the digital transmit-audio level is the live control for what is
    /// on the air — and so whether the strip should offer it (issue #186).
    ///
    /// Three things have to hold, and each rules out a rail that would do
    /// nothing:
    ///
    /// - the radio modulates what we send it (a CAT rig on its sound card, a
    ///   FLEX, an Icom on its network port). Where we modulate it ourselves the
    ///   modulator and Drive own the level and this is never consulted;
    /// - the mode transmits through the digi engine at all;
    /// - and for CW, that the rig is keyed as audio. With its own keyer sending,
    ///   CW leaves as text over the control port and never touches the card.
    ///
    /// Not gated on the mode being *digital*: RADE is, and its microphone is
    /// the payload, so it keeps the mic rail and reaches this one from the TX
    /// menu instead — see [`Self::tx_controls`].
    fn digi_tx_level_applies(&self) -> bool {
        digi_tx_level_applies_to(self.state.rx[0].mode, self.caps.as_ref())
    }

    /// The mode's transmit-audio level as the strip should draw it, in dB.
    fn digi_tx_level_db(&self) -> f32 {
        sdroxide_types::tx_level_db(self.digi_cfg_edit.tx_level_for(self.state.rx[0].mode))
    }

    /// Send a level the operator has just set, and keep the local copy in step
    /// so the rail does not snap back before the engine's echo arrives.
    ///
    /// Gated on `digi_cfg_seeded` like every other write to this configuration:
    /// before the first status the local copy is `DigiConfig::default()`, and a
    /// command built from it would be a level nobody asked for.
    fn set_digi_tx_level(&mut self, db: f32, cmds: &mut Vec<Command>) {
        if !self.digi_cfg_seeded {
            return;
        }
        let mode = self.state.rx[0].mode;
        let level = sdroxide_types::tx_level_from_db(db);
        self.digi_cfg_edit.set_tx_level(mode, level);
        cmds.push(Command::SetDigiTxLevel { mode, level });
    }

    /// The hover that explains the transmit-audio level wherever it is drawn.
    fn digi_tx_level_hover(&self) -> &'static str {
        if self.state.rx[0].mode.is_fm_carrier() {
            "Deviation: how far this mode's burst swings a radio that modulates \
             it itself. An FM transmitter turns audio level into frequency swing \
             and has no ALC to catch it, so full scale into a data input set for \
             voice over-deviates — which sounds completely normal to a listener \
             and decodes for nobody.\n\nKept per mode, so a deviation set for \
             1200 baud packet never lands on FT8."
        } else {
            "Transmit audio: how hard this mode drives the modulator of a radio \
             that modulates what we send it — a CAT rig on its sound card, a \
             FLEX, an Icom on its network port. Bring it down until the rig's \
             ALC is barely moving and set the power at the radio; ALC riding on \
             a constant-envelope digital mode is what splatters.\n\nDrive is \
             not this control: on these radios Drive reaches the rig's power \
             register and never touches its audio.\n\nKept per mode — FT8, \
             RTTY, PSK and MCW each keep their own."
        }
    }

    /// The transmit-audio level: label + rail + dB readout, laid out like Drive.
    fn tx_digi_level(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut db = self.digi_tx_level_db();
        ui.label("TX audio");
        if crate::chrome::slider(
            ui,
            Slider::new(&mut db, sdroxide_types::TX_AUDIO_LEVEL_MIN_DB..=0.0)
                .show_value(true)
                .custom_formatter(|v, _| format!("{v:.0} dB")),
        )
        .on_hover_text(self.digi_tx_level_hover())
        .changed()
        {
            self.set_digi_tx_level(db, cmds);
        }
    }

    /// The transmit-audio level as a vertical rail, in the mic rail's place.
    ///
    /// Its caption is its readout rather than a name: the level in dB, which is
    /// the number the operator is trying to see. The control it replaces was a
    /// drag-value in a dialog most digital modes cannot even open, and being
    /// unable to see the figure is half of what issue #186 reported.
    fn tx_digi_level_vertical(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mut db = self.digi_tx_level_db();
        let hover = self.digi_tx_level_hover();
        let mut set = None;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(format!("{db:.0} dB")).size(10.5));
            // The rail takes whatever height the caption left it.
            ui.spacing_mut().slider_width = (ui.available_height() - 2.0).max(24.0);
            if crate::chrome::slider(
                ui,
                Slider::new(&mut db, sdroxide_types::TX_AUDIO_LEVEL_MIN_DB..=0.0)
                    .vertical()
                    .show_value(false),
            )
            .on_hover_text(hover)
            .changed()
            {
                set = Some(db);
            }
        });
        if let Some(db) = set {
            self.set_digi_tx_level(db, cmds);
        }
    }

    /// The transmit controls as the TX menu shows them: the keyer and the
    /// levels, in the order the box draws them. PTT is on the strip already;
    /// TUNE rides with the caller (see [`Self::tx_menu`]). See
    /// [`crate::chrome::control_row`] for `narrow`.
    fn tx_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, narrow: bool) {
        // Both levels where both apply, unlike the condensed box: this is a
        // list with room to grow, so there is no reason to make RADE — the one
        // mode that has a use for each — choose between them.
        //
        // Mic stays wherever the box would still be showing it, which is what
        // keeps the two surfaces saying the same thing. It is inert in a
        // digital mode whatever the radio is, but only where the level rail
        // stands in its place has anything replaced it; dropping it in the menu
        // and not in the box would just be the same control missing from one of
        // the two places it lives.
        let level = self.digi_tx_level_applies();
        let mic = self.state.rx[0].mode.allows_voice_keyer() || !level;
        crate::chrome::control_row(ui, narrow, |ui| {
            self.tx_keyer_chip(ui);
            self.tx_drive(ui, cmds);
            self.tx_tune_level(ui, cmds);
            if mic {
                self.tx_mic(ui, cmds);
            }
            if level {
                self.tx_digi_level(ui, cmds);
            }
            // The envelope processor, where it reaches the air at all — see
            // [`Self::cessb_applies`]. The condensed box draws the same
            // control as a rail beside the mic one, so neither surface is
            // missing a control the other has (issue #294).
            if self.cessb_applies() {
                self.tx_cessb(ui, cmds);
            }
        });
    }

    /// The condensed TX box's natural width — [`tx_rows_w_for`] with the
    /// voice keyer's presence read off the current mode.
    fn tx_rows_w(&self, ui: &egui::Ui) -> f32 {
        tx_rows_w_for(ui, self.state.rx[0].mode.allows_voice_keyer(), self.tx_side_col_w())
    }

    /// What the condensed TX box's right-hand columns cost: the transmit-audio
    /// rail where it applies, else the mic rail — exactly one of the two is
    /// drawn, so the box pays for one of them — plus the envelope rail beside
    /// it in the voice modes that can use it, gap included.
    ///
    /// In practice the second column only ever joins the mic one: the
    /// transmit-audio rail wants a radio that modulates what we send it, and
    /// CESSB wants one that does not.
    fn tx_side_col_w(&self) -> f32 {
        let side = if self.digi_tx_level_applies() { TX_LEVEL_COL_W } else { TX_MIC_COL_W };
        side + if self.cessb_applies() { MODULE_ROW_SPACING + TX_CESSB_COL_W } else { 0.0 }
    }

    /// The condensed TX box, keyed by what each row transmits: PTT beside the
    /// drive it keys at (and the voice keyer, which transmits the same way),
    /// TUNE beside the carrier level it keys at, and the levels that are not
    /// keyed at all standing at the right as vertical rails — the mic gain (or
    /// the transmit-audio level standing in for it), and the envelope
    /// processor where it applies. Each row's rail is sized so the two
    /// readouts end flush with each other at the box edge, whatever width the
    /// packer granted.
    fn tx_condensed(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        let keyer = self.state.rx[0].mode.allows_voice_keyer();
        let level = self.digi_tx_level_applies();
        let cessb = self.cessb_applies();
        let side_w = self.tx_side_col_w();
        let (fixed1, fixed2) = tx_rows_fixed_w(ui, keyer);
        let inner = w - 2.0 * crate::chrome::MODULE_MARGIN_X - 4.0;
        let rows_w = inner - TX_MIC_GAP - side_w;
        let (rail1, rail2) =
            ((rows_w - fixed1).max(STRIP_RAIL_W), (rows_w - fixed2).max(STRIP_RAIL_W));
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = rail1;
                    self.tx_ptt_chip(ui, cmds);
                    self.tx_keyer_chip(ui);
                    self.tx_drive(ui, cmds);
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = rail2;
                    self.tx_tune_chip(ui, cmds);
                    self.tx_tune_level(ui, cmds);
                });
            });
            // The side rail stands apart from the rows' readouts, so it reads
            // as its own control rather than a fourth element of the rows.
            //
            // One of these two, never both. In a digital mode the mic gain
            // reaches nothing — the microphone is drained and discarded, and
            // only the voice paths ever scale it — so the level rail takes its
            // place rather than crowding a box of fixed height with a dead
            // control beside a live one. Flipping USB to FT8 and watching the rail change is
            // also how an operator finds this at all, which is the other half
            // of issue #186.
            ui.add_space(TX_MIC_GAP - MODULE_ROW_SPACING);
            if level {
                self.tx_digi_level_vertical(ui, cmds);
            } else {
                self.tx_mic_vertical(ui, cmds);
            }
            // And the envelope processor beside it in voice sideband, in the
            // signal's own order: the mic gain drives what CESSB then flattens.
            // Without this the control existed only in the TX menu, which the
            // desktop strip does not have — the whole of issue #294.
            if cessb {
                self.tx_cessb_vertical(ui, cmds);
            }
        });
    }

    /// The SKIM chip: lit while any skimmer runs, and a popup with one row per
    /// kind (CW / PSK / RTTY) — an on/off chip plus that skimmer's squelch, the
    /// SNR a track must reach before it earns a box on the waterfall. Fades out
    /// on its own like the band/mode popup.
    fn skimmer_button(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, extra: f32) {
        let [_, _, skim, _] = DISPLAY_TOOL_CHIPS;
        let btn = chip_stretched(ui, self.state.skimmer.any_enabled(), skim, extra).on_hover_text(
            "CW / PSK / RTTY skimmers — decode signals across the band and mark them on the waterfall",
        );
        let popup_id = egui::Popup::default_response_id(&btn);
        let now = ui.input(|i| i.time);
        let alpha =
            crate::chrome::popup_fade_alpha(ui.ctx(), popup_id, now, &mut self.skimmer_popup_since);
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                crate::chrome::menu_caption(ui, "Skimmers");
                self.skimmer_controls(ui, cmds);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            if r.response.contains_pointer() {
                self.skimmer_popup_since = Some(now); // keep it up while the pointer is on it
            }
        }
    }

    /// One row per skimmer kind: an on/off chip plus that skimmer's squelch.
    ///
    /// Its own function because a menu has to inline this rather than open it
    /// as a popup — a popup opened from a popup counts as a click outside the
    /// first, which closes the menu out from under the control being reached
    /// for.
    fn skimmer_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        // A CAT rig feeding demodulated audio has no IQ span to skim; the engine
        // forces the skimmers off there, so the rows are shown disabled.
        let wideband = self.caps.as_ref().is_none_or(|c| !c.audio_mode);
        {
            {
                // Edit a copy and send the whole struct on any change; the
                // engine echoes it back in the next RadioState.
                let mut cfg = self.state.skimmer;
                // A grid so the squelch fields line up under each other despite
                // the kind chips having different widths.
                egui::Grid::new("skimmer-kinds").num_columns(3).spacing([6.0, 5.0]).show(
                    ui,
                    |ui| {
                        if !wideband {
                            ui.disable();
                        }
                        for kind in SkimmerKind::ALL {
                            if crate::chrome::chip(ui, cfg.enabled(kind), kind.label())
                                .on_hover_text("Run this skimmer")
                                .clicked()
                            {
                                cfg.set_enabled(kind, !cfg.enabled(kind));
                            }
                            ui.label(
                                RichText::new("sql").size(10.0).color(crate::theme::CYAN_DIM()),
                            );
                            let mut sql = cfg.squelch_db(kind);
                            if ui
                                .add(
                                    DragValue::new(&mut sql)
                                        .speed(0.25)
                                        .range(0..=40)
                                        .suffix(" dB"),
                                )
                                .on_hover_text("Minimum SNR a decoded signal needs to be spotted")
                                .changed()
                            {
                                cfg.set_squelch_db(kind, sql);
                            }
                            ui.end_row();
                        }
                    },
                );
                if !wideband {
                    ui.label(
                        RichText::new("needs a wideband IQ source")
                            .size(9.5)
                            .color(crate::theme::gray(150)),
                    );
                }

                // Which decoder reads the CW, and — for the neural one, the only
                // one whose cost depends on it — how many stations at once.
                if wideband && cfg.enabled(SkimmerKind::Cw) {
                    ui.add_space(2.0);
                    crate::chrome::menu_caption(ui, "CW decoder");
                    ui.horizontal_wrapped(|ui| {
                        for d in CwSkimmerDecoder::ALL {
                            if crate::chrome::chip(ui, cfg.cw_decoder == d, d.label())
                                .on_hover_text(d.hint())
                                .clicked()
                            {
                                cfg.cw_decoder = d;
                            }
                        }
                    });
                    if cfg.cw_decoder == CwSkimmerDecoder::Neural {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("stations")
                                    .size(10.0)
                                    .color(crate::theme::CYAN_DIM()),
                            );
                            for n in sdroxide_types::CW_SLOT_CHOICES {
                                if crate::chrome::chip(ui, cfg.cw_slots == n, &n.to_string())
                                    .on_hover_text(
                                        "How many signals the model reads at once. \
                                         The rest keep their marker but carry no text.",
                                    )
                                    .clicked()
                                {
                                    cfg.cw_slots = n;
                                }
                            }
                        });
                    }
                }

                if cfg != self.state.skimmer {
                    cmds.push(Command::SetSkimmerConfig(cfg));
                }
            }
        }
    }

    /// The solar view and the chips that pick what the panadapter draws — the
    /// condensed Display box's top row, and the head of the DISP menu's row.
    /// `extra` stretches each chip past its label; the popup passes 0, which
    /// draws exactly the chip it always drew.
    ///
    /// `narrow` marks the menu, which cannot open SPEC's popup — a popup
    /// opened from a popup counts as a click outside the first and closes it,
    /// exactly as for SKIM and FFT below — so the menu leaves the chip out here
    /// and [`Self::display_controls`] inlines its contents instead.
    fn display_view_chips(&mut self, ui: &mut egui::Ui, narrow: bool, extra: f32) {
        let [_, spec, wide] = DISPLAY_VIEW_CHIPS;
        // Only a front end with a full-band lane has ever sent one of these, so
        // its presence is what says the strip is on offer at all — there is no
        // capability flag for it, and inventing one would mean a wire-format
        // change for something the frames themselves already answer.
        let has_wide = self.wide_frame.is_some();
        // A phone draws the waterfall alone, so the two chips that choose what
        // else is drawn have nothing to control there.
        let picks_layers = !crate::layout::tier(ui.ctx()).waterfall_only();
        self.solar_button(ui, extra);
        if picks_layers && !narrow {
            self.layers_button(ui, spec, extra);
        }
        if picks_layers
            && has_wide
            && chip_stretched(ui, self.view.wide_waterfall, wide, extra)
                .on_hover_text(
                    "Show/hide the full-band waterfall strip above the panadapter — \
                     everything this receiver can see at once",
                )
                .clicked()
        {
            self.view.wide_waterfall = !self.view.wide_waterfall;
            // History kept while the strip is hidden would come back as a
            // block of minutes-old band, drawn as if it were the last few
            // seconds. Start it again from now instead.
            self.wide_wf.clear();
        }
    }

    /// The SPEC chip: the spectrum/waterfall layer switches, behind a popup.
    /// Lit while both layers are drawn, so a display with one of them switched
    /// off says so from the strip without the popup being opened.
    fn layers_button(&mut self, ui: &mut egui::Ui, label: &str, extra: f32) {
        let both = self.view.spectrum_visible() && self.view.waterfall_visible();
        let btn = chip_stretched(ui, both, label, extra).on_hover_text(
            "Spectrum and waterfall — either layer, both, or neither. Lit while both are shown.",
        );
        let popup_id = egui::Popup::default_response_id(&btn);
        let now = ui.input(|i| i.time);
        let alpha =
            crate::chrome::popup_fade_alpha(ui.ctx(), popup_id, now, &mut self.layers_popup_since);
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                self.panadapter_controls(ui);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            if r.response.contains_pointer() {
                self.layers_popup_since = Some(now); // keep it up while the pointer is on it
            }
        }
    }

    /// The SPEC popup's body, and what the DISP menu inlines in its place:
    /// everything the panadapter is drawn by, in two boxes — one for the
    /// spectrum line across the top, one for the waterfall under it — so each
    /// setting sits with the half of the picture it changes.
    ///
    /// The two switches are labelled SHOW … rather than named after their
    /// layer: lit-means-drawn only reads as a switch once you already know
    /// what the chip does, and one of the four displays they reach — both off,
    /// no panadapter at all — is not a place to arrive at by guessing.
    /// Reaching it deliberately stays allowed, and is not a state to be talked
    /// out of: a mode with an operating panel gives it the whole height, and
    /// this popup is the way back.
    ///
    /// The detail and the two speeds are this screen's own preferences rather
    /// than the radio's ([`sdroxide_types::UiSettings`]), which is why they are
    /// written and persisted here instead of going out as a [`Command`].
    /// Everything that reads them picks them up again next frame, the engine's
    /// spectrum config among it.
    ///
    /// Its own function because the DISP menu has to inline it rather than
    /// open it as a popup, for the reason given on [`Self::skimmer_controls`].
    fn panadapter_controls(&mut self, ui: &mut egui::Ui) {
        // A phone draws the waterfall alone: there is no spectrum line to
        // switch on, to peak-hold, or to slow down, so its box is left out —
        // and the detail row, which sizes both layers, moves into the box that
        // is still there rather than going with it.
        let picks_layers = !crate::layout::tier(ui.ctx()).waterfall_only();
        let w = panadapter_group_w(ui);
        let mut cfg = self.ui_settings;
        if picks_layers {
            crate::chrome::menu_group(ui, "Spectrum", w, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let on = self.view.spectrum_visible();
                    if crate::chrome::chip(ui, on, "SHOW SPECTRUM")
                        .on_hover_text(
                            "Draw the spectrum line across the top of the panadapter. \
                             Switched off, the waterfall takes the whole height.",
                        )
                        .clicked()
                    {
                        self.view.set_spectrum_visible(!on);
                    }
                    if crate::chrome::chip(ui, self.view.peak_hold, "PEAK HOLD")
                        .on_hover_text(
                            "Trace the highest level each column has reached over the live \
                             line, decaying back down — what the band did while you were \
                             looking elsewhere",
                        )
                        .clicked()
                    {
                        self.view.peak_hold = !self.view.peak_hold;
                    }
                    if crate::chrome::chip(ui, self.view.spectrum_3d, "3D")
                        .on_hover_text(
                            "Draw the spectrum as a receding surface instead of a flat line: \
                             the newest spectrum across the front, the ones before it flowing \
                             away from you. The last couple of seconds of the band as a \
                             landscape — a carrier that comes and goes is a ridge rather than \
                             a line that twitches. The grid and the peak hold belong to the \
                             flat line and are not drawn on it.",
                        )
                        .clicked()
                    {
                        self.view.spectrum_3d = !self.view.spectrum_3d;
                    }
                });
                self.spectrum_3d_rows(ui, &mut cfg);
                speed_row(
                    ui,
                    "reaction",
                    &mut cfg.spectrum_speed,
                    &Speed::ALL,
                    "How quickly the spectrum line follows the band. Slower averages more \
                     frames into each other: a steadier line, and a weak carrier that stands \
                     still long enough to read. The waterfall is not touched by it — those \
                     rows get every frame either way.",
                );
                self.detail_row(ui, &mut cfg);
            });
        }
        crate::chrome::menu_group(ui, "Waterfall", w, |ui| {
            if picks_layers {
                let on = self.view.waterfall_visible();
                if crate::chrome::chip(ui, on, "SHOW WATERFALL")
                    .on_hover_text(
                        "Draw the scrolling waterfall below the spectrum. Switched off, the \
                         spectrum line takes the whole height.",
                    )
                    .clicked()
                {
                    self.view.set_waterfall_visible(!on);
                }
            }
            speed_row(
                ui,
                "scroll",
                &mut cfg.waterfall_speed,
                &Speed::WATERFALL,
                "How fast the waterfall scrolls, in lines a second: Slow 5, Medium 28, Fast \
                 56, Faster 112, Fastest 224. The engine clocks these itself, so the two \
                 fastest are real detail rather than the same line drawn twice — as far as \
                 the receiver can feed them: a line can never show more than one transform, \
                 and a narrow front end makes only a few dozen a second. They cost history, \
                 since the waterfall keeps a fixed number of lines — 73 seconds at Medium, 9 \
                 at Fastest.",
            );
            if crate::chrome::chip(ui, self.view.decode_labels, "DECODE LABELS")
                .on_hover_text(
                    "Mark every decoded station on the waterfall with its callsign, at the \
                     frequency it was heard on — FT8, FT4 and the other slotted modes. A \
                     good opening puts thirty of them across the span twice a minute, over \
                     the traces you are reading; switch them off and the waterfall is just \
                     the band. The decode list beside it still has every callsign.",
                )
                .clicked()
            {
                self.view.decode_labels = !self.view.decode_labels;
            }
            if !picks_layers {
                self.detail_row(ui, &mut cfg);
            }
        });
        if cfg != self.ui_settings {
            self.ui_settings = cfg;
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
    }

    /// The two rows of [`Self::panadapter_controls`] that belong to the 3D
    /// display: how the surface is drawn, and how fast it flows away from the
    /// viewer. Both are live only once the 3D chip above them is lit.
    ///
    /// Greyed rather than hidden while the flat line is showing, the way the
    /// detail row greys a width this machine cannot hold: rows that come and go
    /// resize the popup under the pointer, and the choices they hold are still
    /// real ones — they are what the 3D chip will show when it is clicked.
    fn spectrum_3d_rows(&mut self, ui: &mut egui::Ui, cfg: &mut sdroxide_types::UiSettings) {
        let on = self.view.spectrum_3d;
        let grey = "Switch 3D on to set this";
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("surface").size(10.0).color(crate::theme::CYAN_DIM()))
                .on_hover_text(
                    "How the 3D spectrum is drawn. Only the shape differs — both renderings \
                     show the same spectra, and both hide what is behind them, so a strong \
                     signal in front stands over the band it is covering.",
                );
            for (solid, label, hint) in [
                (
                    false,
                    "LINES",
                    "One trace per remembered spectrum, in the flat line's own colour, each \
                     hiding the ones behind it. The shape without the levels, and the reading \
                     to pick when the shape is what you are after — it costs a little more \
                     than the solid one rather than less, because it draws the same surface \
                     and then strokes every crest on top of it.",
                ),
                (
                    true,
                    "SOLID",
                    "A filled surface coloured by the waterfall's palette, so the level is in \
                     the colour as well as in the height and the two halves of the panadapter \
                     agree about what a strong signal looks like. Change the palette in \
                     Settings › Display.",
                ),
            ] {
                let r = crate::chrome::chip_enabled(
                    ui,
                    on,
                    self.view.spectrum_3d_solid == solid,
                    label,
                );
                if on {
                    if r.on_hover_text(hint).clicked() {
                        self.view.spectrum_3d_solid = solid;
                    }
                } else {
                    r.on_disabled_hover_text(grey);
                }
            }
        });
        // The flow rate, and what each step of it means on the surface: the
        // depth is a fixed number of rows, so the rate *is* the seconds of band
        // the picture holds, and that is the number worth putting on the hover
        // rather than the rows a second nobody counts.
        let depth = crate::widgets::spectrum3d::DEPTH as f32;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("flow").size(10.0).color(crate::theme::CYAN_DIM()))
                .on_hover_text(
                    "How fast the 3D spectrum flows away from you, in rows a second: Slow 6, \
                     Medium 12, Fast 24, Faster 48. The surface is a fixed number of rows \
                     deep, so this is also how much time it holds. Slower is a longer memory \
                     and a surface that crawls; faster is a shorter one that moves. The \
                     waterfall keeps its own scroll rate, below.",
                );
            for step in Speed::SURFACE {
                let label = step.label().to_uppercase();
                let r = crate::chrome::chip_enabled(ui, on, cfg.spectrum_3d_speed == step, &label);
                if !on {
                    r.on_disabled_hover_text(grey);
                    continue;
                }
                let rate = sdroxide_types::UiSettings { spectrum_3d_speed: step, ..*cfg }
                    .spectrum_3d_rows_per_sec();
                let hint =
                    format!("{:.0} rows a second — {:.0} seconds of band", rate, depth / rate);
                if r.on_hover_text(hint).clicked() {
                    cfg.spectrum_3d_speed = step;
                }
            }
        });
    }

    /// The detail row of [`Self::panadapter_controls`]: how many columns the
    /// panadapter is asked for, and the width that choice actually comes out at
    /// on this machine.
    ///
    /// A step this renderer cannot hold is drawn greyed with its reason on the
    /// hover rather than left out. A ladder with rungs missing reads as a bug;
    /// a ladder with rungs out of reach reads as the truth.
    fn detail_row(&self, ui: &mut egui::Ui, cfg: &mut sdroxide_types::UiSettings) {
        let report = self.detail_report(cfg.spectrum_detail);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("detail").size(10.0).color(crate::theme::CYAN_DIM()))
                .on_hover_text(
                    "How many columns the panadapter and its waterfall are drawn with. Auto \
                     reads this machine's renderer and the size of the panadapter and picks \
                     the most it can carry — a 4K screen wants 4096. Every column is a byte \
                     in every frame, so a client connected to a remote station pays for the \
                     detail on its link: 4096 columns at 60 fps is about a quarter of a \
                     megabyte a second, twice the standard width.",
                );
            for d in SpectrumDetail::ALL {
                let over = d.columns().is_some_and(|c| c > report.ceiling);
                let r = crate::chrome::chip_enabled(
                    ui,
                    !over,
                    cfg.spectrum_detail == d,
                    &detail_chip_label(d),
                );
                if over {
                    r.on_disabled_hover_text(&report.reason);
                } else {
                    let hint = match d.columns() {
                        None => "Read this machine and this screen, take the most they can \
                                 carry, and follow them if they change"
                            .to_string(),
                        Some(c) => format!("{c} columns, whatever Auto would have picked here"),
                    };
                    if r.on_hover_text(hint).clicked() {
                        cfg.spectrum_detail = d;
                    }
                }
            }
            ui.label(
                RichText::new(format!("{} columns", report.chosen))
                    .size(10.0)
                    .color(Color32::from_gray(150)),
            )
            .on_hover_text("The width in force right now, whatever the row above asks for");
        });
    }

    /// The level fit, the skimmers and the FFT popup — the condensed Display
    /// box's bottom row, and the tail of the DISP menu's row.
    fn display_tool_chips(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        narrow: bool,
        extra: f32,
    ) {
        let [fit, ctr, _, fft] = DISPLAY_TOOL_CHIPS;
        // Lit while the floor/ceiling are kept fitted by themselves. Switching
        // it on fits immediately, which is also how a one-off fit is asked for:
        // click it off and on again.
        if chip_stretched(ui, self.view.auto_fit, fit, extra)
            .on_hover_text(
                "Keep the floor/ceiling set for best waterfall contrast — eased back into place \
                 on a band change, after a pan or zoom, and when the levels drift. Switch it on \
                 to fit at once; switch it off to keep the levels where you set them.",
            )
            .clicked()
        {
            self.view.auto_fit = !self.view.auto_fit;
            if self.view.auto_fit {
                self.fit_levels_now(ui.input(|i| i.time));
            }
        }
        // FIT's twin for the frequency axis (issue #174): lit while the window
        // follows the dial, so the tuned frequency stays in the middle of the
        // picture and the band scrolls past it instead of the picture jumping a
        // whole span every time the dial leaves it. Switching it on centres at
        // once, which is also how a one-off "put me back in the middle" is
        // asked for: click it on, and off again if you would rather pan freely.
        if chip_stretched(ui, self.view.center_on_vfo, ctr, extra)
            .on_hover_text(
                "Keep the tuned frequency in the middle of the panadapter: the window slides \
                 under the dial instead of the picture jumping a whole span when you tune off \
                 the edge. Switch it on to centre at once; switch it off to pan and zoom \
                 wherever you like.",
            )
            .clicked()
        {
            self.view.center_on_vfo = !self.view.center_on_vfo;
        }
        // In a box these two hang off chips of their own. A menu inlines
        // them below instead: a popup opened from a popup counts as a click
        // outside the first and closes it.
        if narrow {
            return;
        }
        self.skimmer_button(ui, cmds, extra);
        // Floor/ceiling + FFT size live in a popup off this button.
        let fft_btn = chip_stretched(ui, false, fft, extra)
            .on_hover_text("Spectrum floor / ceiling and FFT size");
        let fft_id = egui::Popup::default_response_id(&fft_btn);
        let now = ui.input(|i| i.time);
        let alpha =
            crate::chrome::popup_fade_alpha(ui.ctx(), fft_id, now, &mut self.fft_popup_since);
        let fft_resp = egui::Popup::from_toggle_button_response(&fft_btn)
            .frame(crate::chrome::window_frame_alpha(alpha))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_opacity(alpha);
                crate::chrome::window_body_bg(ui);
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                self.spectrum_controls(ui);
            });
        if let Some(r) = &fft_resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, alpha);
            if r.response.contains_pointer() {
                self.fft_popup_since = Some(now);
            }
        }
    }

    /// The display controls — the body of the DISP menu. See
    /// [`crate::chrome::control_row`] for `narrow`.
    fn display_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, narrow: bool) {
        crate::chrome::control_row(ui, narrow, |ui| {
            self.display_view_chips(ui, narrow, 0.0);
            self.display_tool_chips(ui, cmds, narrow, 0.0);
        });
        if narrow {
            // Unconditional, unlike the SPEC chip above, which a phone leaves
            // out: the panadapter box holds more than the layer switches it is
            // the only home for, and it hides the switches itself where there
            // are no layers to pick.
            self.panadapter_controls(ui);
            crate::chrome::menu_caption(ui, "Skimmers");
            self.skimmer_controls(ui, cmds);
            self.spectrum_controls(ui);
        }
    }

    /// The condensed Display box's natural width: the wider of its two chip
    /// rows plus the box margins.
    fn display_rows_w(&self, ui: &egui::Ui) -> f32 {
        let row1: &[&str] =
            if self.wide_frame.is_some() { &DISPLAY_VIEW_CHIPS } else { &DISPLAY_VIEW_CHIPS[..2] };
        chip_row_w(ui, row1).max(chip_row_w(ui, &DISPLAY_TOOL_CHIPS))
            + 2.0 * crate::chrome::MODULE_MARGIN_X
    }

    /// The condensed Display box: the view chips on top, the tool chips below,
    /// each row's chips splitting its share of the packer's stretch evenly.
    fn display_condensed(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        let inner = w - 2.0 * crate::chrome::MODULE_MARGIN_X;
        let row1: &[&str] =
            if self.wide_frame.is_some() { &DISPLAY_VIEW_CHIPS } else { &DISPLAY_VIEW_CHIPS[..2] };
        let extra1 = ((inner - chip_row_w(ui, row1)) / row1.len() as f32).max(0.0);
        let extra2 = ((inner - chip_row_w(ui, &DISPLAY_TOOL_CHIPS))
            / DISPLAY_TOOL_CHIPS.len() as f32)
            .max(0.0);
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                ui.horizontal(|ui| self.display_view_chips(ui, false, extra1));
                ui.horizontal(|ui| self.display_tool_chips(ui, cmds, false, extra2));
            });
        });
    }

    /// Spectrum floor/ceiling, FFT size and the waterfall's scroll direction.
    /// Inlined by the DISP menu, behind the FFT chip in the Display box — see
    /// [`Self::skimmer_controls`] for why a menu cannot use the popup.
    fn spectrum_controls(&mut self, ui: &mut egui::Ui) {
        crate::chrome::menu_caption(ui, "Spectrum");
        ui.horizontal(|ui| {
            ui.label("floor");
            ui.add(
                DragValue::new(&mut self.view.db_floor)
                    .speed(1.0)
                    .range(-160.0..=-40.0)
                    .suffix(" dB"),
            );
            ui.label("ceil");
            ui.add(
                DragValue::new(&mut self.view.db_ceil)
                    .speed(1.0)
                    .range(-100.0..=20.0)
                    .suffix(" dB"),
            );
        });
        // Chips rather than a ComboBox: the combo opens a second popup
        // layer, and clicking it counts as "outside" and closes this one.
        crate::chrome::menu_caption(ui, "FFT size");
        ui.horizontal_wrapped(|ui| {
            let cols = self.panadapter_bins();
            for n in [2048u32, 4096, 8192, 16384, 32768, 65536, 131_072] {
                // Past the panadapter's own width a bigger transform stops
                // adding columns and starts sharpening the ones there are —
                // each is the maximum of more bins, so a weak carrier stands
                // further out of the noise. Worth saying, because "no more
                // detail" is what it looks like otherwise.
                let hint = if n <= cols {
                    format!("{n} bins across the whole stream — one per column, or finer")
                } else {
                    format!(
                        "{n} bins pooled into the panadapter's {cols} columns: \
                         sharper signals, not more of them"
                    )
                };
                if crate::chrome::chip(ui, self.view.fft_size == n, format!("{n}"))
                    .on_hover_text(hint)
                    .clicked()
                {
                    self.view.fft_size = n;
                }
            }
        });
        crate::chrome::menu_caption(ui, "Waterfall");
        if crate::chrome::chip(ui, self.view.waterfall_flip, "FLIP")
            .on_hover_text("Scroll the waterfall upwards — newest row at the bottom (V)")
            .clicked()
        {
            self.view.waterfall_flip = !self.view.waterfall_flip;
        }
    }

    /// The first five window chips — the condensed System box's top row.
    /// `extra` stretches each chip past its label; the popup passes 0.
    fn system_chips_top(&mut self, ui: &mut egui::Ui, extra: f32) {
        let [log, spots, awards, bands, sat_label, ism, public_sdrs] = SYSTEM_CHIPS_TOP;
        if chip_stretched(ui, self.show_logbook, log, extra)
            .on_hover_text("Logbook — all QSOs (digital + manual)")
            .clicked()
        {
            self.show_logbook = !self.show_logbook;
        }
        if chip_stretched(ui, self.show_spots, spots, extra)
            .on_hover_text("Live spots — DX cluster, POTA, SOTA, PSK Reporter")
            .clicked()
        {
            self.show_spots = !self.show_spots;
        }
        if chip_stretched(ui, self.show_awards, awards, extra)
            .on_hover_text("Award tracking — DXCC / WAS / WAZ / grids")
            .clicked()
        {
            self.show_awards = !self.show_awards;
        }
        if chip_stretched(ui, self.show_bands, bands, extra)
            .on_hover_text(
                "Band conditions — the published forecast beside what has \
                 actually been heard on each band",
            )
            .clicked()
        {
            self.show_bands = !self.show_bands;
        }
        // Accented while a satellite lock *or* the QO-100 beacon hunt is
        // running, like the scanner: both spend the receiver whether or not the
        // window is open, and that has to be visible. QO-100 shares this chip
        // because it shares the window — it is a satellite, and the only reason
        // it ever had a chip of its own was that its calibration arrived first.
        let qo100_running = self.state.qo100.enabled;
        let sat_chip = if self.sat_track.is_some() || qo100_running {
            accent_chip_stretched(
                ui,
                true,
                sat_label,
                crate::theme::GREEN(),
                crate::theme::INK_ON_BRIGHT(),
                extra,
            )
        } else {
            chip_stretched(ui, self.show_sat, sat_label, extra)
        };
        if sat_chip
            .on_hover_text(match (&self.sat_track, qo100_running) {
                (Some(t), _) => format!("Satellite — locked on {}", t.name),
                (None, true) => "Satellite — the QO-100 beacon calibration is running".into(),
                (None, false) => {
                    "Satellite — Doppler tracking, and the QO-100 beacon calibration".into()
                }
            })
            .clicked()
        {
            self.show_sat = !self.show_sat;
        }
        // Accented while the decoder is actually running, like the scanner and
        // the satellite lock: it is spending CPU on four downconverters whether
        // or not the window is open.
        let ism_running = self.state.ism.any_enabled();
        let ism_chip = if ism_running {
            accent_chip_stretched(
                ui,
                true,
                ism,
                crate::theme::GREEN(),
                crate::theme::INK_ON_BRIGHT(),
                extra,
            )
        } else {
            chip_stretched(ui, self.show_ism, ism, extra)
        };
        if ism_chip
            .on_hover_text(if ism_running {
                // Which is not the same as "this window is open", and the chip
                // cannot say so on its own: an operator who closes the window
                // and finds the chip still lit has no way to guess that the
                // green is the decoder rather than the window, or where its
                // switch went. Same wording problem the SAT chip solves above.
                "ISM-band devices — decoding now, whether or not this window is \
                 open. Switch it off with DECODING inside the window."
            } else {
                "ISM-band devices — weather sensors, meters and home \
                 automation heard around you"
            })
            .clicked()
        {
            self.show_ism = !self.show_ism;
        }
        // Named for what it lists rather than for the WebSDR network, which is
        // the one thing it does *not* list: PA3FWM's receivers speak a
        // proprietary codec their author asks third-party clients to stay away
        // from, and OpenWebRX has neither a stable protocol across its forks
        // nor a machine-readable directory. "WEB SDR" on the chip had operators
        // looking for websdr.org's list behind it (issue #254).
        if chip_stretched(ui, self.show_public_sdrs, public_sdrs, extra)
            .on_hover_text(
                "Public SDRs on the internet — browse the KiwiSDR and SpyServer directories \
                 and open one as a radio",
            )
            .clicked()
        {
            self.show_public_sdrs = !self.show_public_sdrs;
        }
    }

    /// The remaining window chips — the condensed System box's bottom row.
    fn system_chips_bottom(&mut self, ui: &mut egui::Ui, extra: f32) {
        let [mail, mem, scan_label, settings, help] = SYSTEM_CHIPS_BOTTOM;
        if chip_stretched(ui, self.mail.open, mail, extra)
            .on_hover_text("Winlink radio email")
            .clicked()
        {
            self.mail.open = !self.mail.open;
        }
        if chip_stretched(ui, self.show_memories, mem, extra)
            .on_hover_text("Memory channels")
            .clicked()
        {
            self.show_memories = !self.show_memories;
        }
        // Accented while a scan is actually running, so its state is visible
        // with the window closed — which is how it will usually be.
        let scan = self.state.scan;
        let scan_chip = if scan.running {
            accent_chip_stretched(
                ui,
                true,
                scan_label,
                if scan.holding { crate::theme::GREEN() } else { crate::theme::CYAN() },
                crate::theme::INK_ON_BRIGHT(),
                extra,
            )
        } else {
            chip_stretched(ui, self.show_scanner, scan_label, extra)
        };
        if scan_chip
            .on_hover_text(if scan.holding {
                "Scanner — stopped on a signal"
            } else if scan.running {
                "Scanner — running"
            } else {
                "Scan memory channels or a frequency range"
            })
            .clicked()
        {
            self.show_scanner = !self.show_scanner;
        }
        if chip_stretched(ui, self.show_settings, settings, extra)
            .on_hover_text("Settings — device gains, antennas, audio devices")
            .clicked()
        {
            self.show_settings = !self.show_settings;
        }
        if chip_stretched(ui, self.help.open, help, extra)
            .on_hover_text("User manual (F1)")
            .clicked()
        {
            self.help.open = !self.help.open;
        }
    }

    /// The window buttons — the body of the SYS menu. See
    /// [`crate::chrome::control_row`] for `narrow`.
    fn windows_controls(&mut self, ui: &mut egui::Ui, narrow: bool) {
        crate::chrome::control_row(ui, narrow, |ui| {
            self.system_chips_top(ui, 0.0);
            self.system_chips_bottom(ui, 0.0);
        });
    }

    /// The condensed System box: the window chips over two rows, each row's
    /// chips splitting its share of the packer's stretch evenly.
    fn windows_condensed(&mut self, ui: &mut egui::Ui, w: f32) {
        let inner = w - 2.0 * crate::chrome::MODULE_MARGIN_X;
        let (top, bottom): (&[&str], &[&str]) = (&SYSTEM_CHIPS_TOP, &SYSTEM_CHIPS_BOTTOM);
        let extra1 = ((inner - chip_row_w(ui, top)) / top.len() as f32).max(0.0);
        let extra2 = ((inner - chip_row_w(ui, bottom)) / bottom.len() as f32).max(0.0);
        crate::chrome::module_bare_h(ui, w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                ui.horizontal(|ui| self.system_chips_top(ui, extra1));
                ui.horizontal(|ui| self.system_chips_bottom(ui, extra2));
            });
        });
    }
}

/// What the compact layout's PTT chip is doing between frames.
///
/// Push-to-talk and a latch on one control, told apart by what the operator
/// did with it rather than by the tier the window is wearing — see
/// [`SdroxideApp::held_ptt`] for why the tier is the wrong thing to ask.
/// The state is here rather than in a pair of bools so that the one rule that
/// matters — a finger's release always unkeys — is a single `match` arm
/// somebody can check, and so [`Self::on_pointer`] can be tested without an
/// egui context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum PttPress {
    /// Nothing on the chip and nothing on the air.
    #[default]
    Idle,
    /// Pressed, and keying for as long as it stays pressed. `touch` is whether
    /// a finger started it, captured at the press because a release has no
    /// touches left to report.
    Keying { touch: bool },
    /// Left keyed by a mouse click; the next press lets go.
    Latched,
    /// The press that is letting go of a latch. Its release owes no command —
    /// without this state it would read as the end of an over that had already
    /// ended, and unkey a transmitter somebody had keyed again in between.
    Unlatching,
}

impl PttPress {
    /// Whether a pointer is on the chip right now. This is what an incoming
    /// pointer state is compared against to find an edge, so a latch — held
    /// with nothing pressing it — has to read as *not* pressed.
    pub(in crate::app) fn pressed(self) -> bool {
        matches!(self, Self::Keying { .. } | Self::Unlatching)
    }

    /// Whether the chip is holding the transmitter keyed, pressed or latched.
    /// Read where an over has to be ended by something other than the chip
    /// itself — closing the window, for one.
    pub(in crate::app) fn keying(self) -> bool {
        matches!(self, Self::Keying { .. } | Self::Latched)
    }

    /// Whether the chip counts as pressed this frame: egui's own flag for it,
    /// `down_on`, with one gap patched from the pointer button itself.
    ///
    /// A still finger held past egui's click window is read as the
    /// press-and-hold that opens a context menu, and egui implements that by
    /// taking the press off the widget — from here indistinguishable from the
    /// finger having lifted. So hold-to-talk on a touch screen cut every over
    /// at 0.8 s, and it cut them silently: a finger that jitters more than the
    /// drag threshold is a drag instead and holds fine, so the same gesture
    /// works or does not depending on how steady a hand is on it.
    ///
    /// The button is not part of that gesture, so while this chip already owns
    /// a press it is the honest answer. `pressed()` is what keeps it honest in
    /// the other direction: this can only ever *extend* a press the chip
    /// started, never begin one from a pointer that went down somewhere else.
    pub(in crate::app) fn still_down(self, down_on: bool, primary_down: bool) -> bool {
        down_on || (self.pressed() && primary_down)
    }

    /// The next state and the PTT command it owes, from one pointer edge.
    ///
    /// `down` is the new pointer state, `touch` whether a finger caused a
    /// press (meaningless on a release, and ignored there), and `click`
    /// whether egui called this release a click — short enough and still on
    /// the chip. A drag off the chip is deliberately not a click, so pressing
    /// it by accident and sliding away ends the over rather than latching it.
    pub(in crate::app) fn on_pointer(
        self,
        down: bool,
        touch: bool,
        click: bool,
    ) -> (Self, Option<bool>) {
        match (self, down) {
            // A press on a latched chip is the operator taking it back off.
            (Self::Latched, true) => (Self::Unlatching, Some(false)),
            (_, true) => (Self::Keying { touch }, Some(true)),
            // A mouse click stays on the air; a finger, and a press held too
            // long or dragged away to count as a click, does not.
            (Self::Keying { touch: false }, false) if click => (Self::Latched, None),
            (Self::Keying { .. }, false) => (Self::Idle, Some(false)),
            // The release of an unlatching press, or a stray edge.
            (_, false) => (Self::Idle, None),
        }
    }
}

/// The System box's chips, a row at a time, in the order they are drawn.
///
/// **One array per row, and `system_chips_top` / `system_chips_bottom`
/// destructure their own exhaustively** — no `..` — so a chip added to a row
/// without a label added here does not compile. That is what keeps the width
/// reservation honest, and it has to be structural rather than a comment: a box
/// reserved narrower than its contents does not clip them. The row simply
/// carries on past the box, and whatever crosses the window edge is lost. That
/// is how SCAN, SETTINGS and HELP came to vanish on the layouts where the strip
/// put this box near the end of a row — and, later, how the public-SDR chip
/// did, drawn in the top row while a single split index still counted it in the
/// bottom one.
const SYSTEM_CHIPS_TOP: [&str; 7] = ["LOG", "SPOTS", "AWARDS", "BANDS", "SAT", "ISM", "PUBLIC SDR"];

/// The rest of them. See [`SYSTEM_CHIPS_TOP`].
const SYSTEM_CHIPS_BOTTOM: [&str; 5] = ["MAIL", "MEM", "SCAN", "⚙ SETTINGS", "? HELP"];

/// The Display box's top row: the solar view, then the chips that choose what
/// the panadapter draws — the last of those only on a front end with a
/// full-band lane. Peak hold used to stand here too and now lives in SPEC's
/// popup, beside the spectrum line it is drawn over. Visible to the app module
/// because [`SdroxideApp::solar_button`], which draws its first chip, lives in
/// `app::solar`.
pub(in crate::app) const DISPLAY_VIEW_CHIPS: [&str; 3] = ["☀ 3D", "SPEC", "WIDE"];

/// The Display box's bottom row: the level fit, centre tuning, the skimmers,
/// and the FFT/levels popup. Read by the measurement and by each chip's own
/// draw site.
const DISPLAY_TOOL_CHIPS: [&str; 4] = ["FIT", "CTR", "SKIM", "FFT"];

/// The keying chips' shared size: PTT and TUNE drawn to the wider of the two
/// labels, so the chips match and the level blocks beside them start on the
/// same column.
fn tx_key_chip_size(ui: &egui::Ui) -> egui::Vec2 {
    let w = crate::chrome::chip_width(ui, " PTT ", Some(15.0)).max(crate::chrome::chip_width(
        ui,
        " TUNE ",
        Some(15.0),
    ));
    egui::vec2(w, crate::chrome::chip_height(ui, Some(15.0)))
}

/// The fixed parts of the condensed TX box's two rows — everything but the
/// slider rail: (PTT, [keyer], the Drive label and its readout) and (TUNE,
/// the Tune label and its readout), gaps included. Free functions of the
/// keyer flag, like [`plan_short_strip`], so
/// `the_condensed_tx_box_fits_its_rows` can price both states without an app.
fn tx_rows_fixed_w(ui: &egui::Ui, keyer: bool) -> (f32, f32) {
    let g = MODULE_ROW_SPACING;
    let key_w = tx_key_chip_size(ui).x;
    let label =
        |s: &str| crate::chrome::text_width(ui, s, egui::TextStyle::Body.resolve(ui.style()));
    let keyer_w = if keyer { crate::chrome::chip_width(ui, " ▶ ", Some(15.0)) + g } else { 0.0 };
    let row1 = key_w + g + keyer_w + label("Drive") + g + g + TX_SLIDER_VALUE_W;
    let row2 = key_w + g + label("Tune") + g + g + TX_SLIDER_VALUE_W;
    (row1, row2)
}

/// The condensed TX box's natural width: the wider of its rows' fixed parts
/// plus a rail at [`STRIP_RAIL_W`], the side column and its padding, the box
/// margins, and a few points of rounding slack.
///
/// `side_col_w` is [`TX_MIC_COL_W`] or [`TX_LEVEL_COL_W`] — exactly one of the
/// two rails is drawn, and they are not the same width.
/// [`SdroxideApp::digi_tx_level_applies`] over the two things that decide it, so
/// the rule can be tested without an application around it.
fn digi_tx_level_applies_to(mode: Mode, caps: Option<&sdroxide_types::DeviceCaps>) -> bool {
    let Some(caps) = caps else { return false };
    if !(caps.audio_mode || caps.tx_audio) || !mode.takes_digi_tx_audio() {
        return false;
    }
    mode != Mode::Cw || caps.cw_audio_keyed
}

/// [`SdroxideApp::cessb_applies`] over the two things that decide it, so the
/// rule can be tested without an application around it.
fn cessb_applies_to(mode: Mode, caps: Option<&sdroxide_types::DeviceCaps>) -> bool {
    let Some(caps) = caps else { return false };
    matches!(mode, Mode::Usb | Mode::Lsb) && !caps.audio_mode && !caps.tx_audio
}

/// The envelope processor's setting as every one of its surfaces spells it:
/// decibels of compression, and "off" at the bottom of the rail rather than
/// "0 dB", which reads like an amount.
fn cessb_value_text(db: f32) -> String {
    if db < 0.05 { "off".into() } else { format!("{db:.0} dB") }
}

fn tx_rows_w_for(ui: &egui::Ui, keyer: bool, side_col_w: f32) -> f32 {
    let (row1, row2) = tx_rows_fixed_w(ui, keyer);
    row1.max(row2)
        + STRIP_RAIL_W
        + TX_MIC_GAP
        + side_col_w
        + 2.0 * crate::chrome::MODULE_MARGIN_X
        + 4.0
}

/// What egui draws a slider's or drag-value's readout as: the value's text in
/// button padding, never narrower than the style's interact size. Measured
/// from the style rather than pinned to a literal, because both figures move
/// with the tier.
fn value_field_w(ui: &egui::Ui, text: &str) -> f32 {
    let w = crate::chrome::text_width(ui, text, egui::TextStyle::Body.resolve(ui.style()))
        + 2.0 * ui.spacing().button_padding.x;
    w.max(ui.spacing().interact_size.x)
}

/// One of the RX box's dB rails — the front-end Gain and the manual gain the
/// AGC falls back to — with its readout beside it. The rail is pinned
/// ([`RX_DB_RAIL_W`]): these two don't take the box's stretch, the Vol and SQL
/// rails do.
///
/// The readout is priced at a sample wider than any rig's gain range formats
/// to rather than derived from the range, and deliberately: egui picks the
/// number of decimals it shows from the rail's *gradient* — dB per point —
/// so what the box would have to reserve moves with the rail length as well
/// as with the range. A few points of slack buys a figure that holds for
/// every rig, and one that does not change as the rail is dragged.
fn db_rail_w(ui: &egui::Ui) -> f32 {
    RX_DB_RAIL_W + MODULE_ROW_SPACING + value_field_w(ui, "-888.8 dB")
}

/// A chip on the RX box's chip run: everything that follows the SQL rail, in
/// the order it is drawn.
///
/// One list is measured ([`rx_rows`]) and drawn ([`SdroxideApp::rx_chip`]), so
/// a chip the box never reserved room for cannot be added to it. The DRM chip
/// was: the box drew it past its own right edge, pushed the boxes beside it
/// along the row and cost the System box its last chips over the edge of the
/// window (issue #152) — exactly what ANC and MONO had done before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RxChip {
    Nb,
    Anc,
    Nr,
    /// Binaural (pseudo-stereo) CW audio.
    Bin,
    Mute,
    Rec,
    /// WFM's stereo pilot.
    Stereo,
    /// WFM's RDS subcarrier.
    Rds,
    /// The DRM decoder's state.
    Drm,
    /// NFM's sub-audible tone.
    Tone,
}

impl RxChip {
    /// The widest label the chip ever wears, which is what the box reserves
    /// for it: a chip whose label follows what it is reading — the tone chip —
    /// must not change the width of the box around it as signals come and go.
    fn width_label(self) -> &'static str {
        match self {
            Self::Nb => "NB",
            Self::Anc => "ANC",
            Self::Nr => "NR",
            Self::Bin => "BIN",
            Self::Mute => "MUTE",
            Self::Rec => "REC",
            Self::Stereo => "ST",
            Self::Rds => "RDS",
            Self::Drm => "DRM",
            // Armed but silent — the dot marks the tone as a requirement
            // rather than a decode, and a DCS code reads longer than any
            // CTCSS tone.
            Self::Tone => "·D023N",
        }
    }

    fn width(self, ui: &egui::Ui) -> f32 {
        crate::chrome::chip_width(ui, self.width_label(), None)
    }
}

/// The width the DIV box wants: its two rows, whichever is wider.
///
/// A free function of nothing but the style, like [`tx_rows_fixed_w`], so
/// `the_diversity_box_fits_its_rows` can price it without an app around it.
/// The mode chip is measured at its longer label, because it cycles between
/// the two and a box that shrank under the shorter one would move everything
/// beside it every time the filter was switched.
fn div_rows_w(ui: &egui::Ui) -> f32 {
    let gap = MODULE_ROW_SPACING;
    let top = crate::chrome::text_width(ui, "DIV", egui::FontId::proportional(11.0))
        + gap
        + crate::chrome::chip_width(ui, DIV_MODE_LABELS[1], None)
        + gap
        + crate::chrome::chip_width(ui, "HOLD", None)
        + gap
        + crate::chrome::chip_width(ui, "RESTART", None);
    let bottom = crate::chrome::text_width(ui, "Adapt", egui::TextStyle::Body.resolve(ui.style()))
        + gap
        + STRIP_RAIL_W;
    top.max(bottom) + 2.0 * crate::chrome::MODULE_MARGIN_X
}

/// The RX box's chip run in a mode: the six every mode carries, then whatever
/// the mode itself brings — a subcarrier to read, a tone to gate on.
fn rx_chips(mode: Mode) -> Vec<RxChip> {
    // MONO is not among them: it is the *recording's* channel layout, and it
    // now sits beside the recording controls it belongs to, inside the REC
    // popup (issue #217). That is also one chip fewer on a strip that has to
    // fit on a 1366-pixel screen (issue #211).
    let mut chips = vec![RxChip::Nb, RxChip::Anc, RxChip::Nr, RxChip::Mute, RxChip::Rec];
    // Binaural audio goes where it is worth a permanent button: CW, where the
    // signal is a tone and so placing it by pitch places the signal, and SSB,
    // where what it buys is the decorrelated noise around the voice
    // (Mode::binaural_audio). It rides ahead of MUTE rather than on the end,
    // beside the other things done to the audio on its way to the ear.
    if mode.binaural_audio() {
        chips.insert(3, RxChip::Bin);
    }
    match mode {
        // Only WFM has a stereo pilot to lock or an RDS subcarrier to decode.
        Mode::Wfm => chips.extend([RxChip::Stereo, RxChip::Rds]),
        // Only DRM has a decoder whose state is worth a light of its own.
        Mode::Drm => chips.push(RxChip::Drm),
        // Only NFM carries a sub-audible tone.
        Mode::Nfm => chips.push(RxChip::Tone),
        _ => {}
    }
    chips
}

/// How the RX box lays itself out: where its chip run breaks between the two
/// rows, and what each row comes to once it has — gaps included, measured
/// against the live style.
#[derive(Debug, Clone, Copy)]
struct RxRows {
    /// How many of [`rx_chips`] ride up on the receive row. The rest follow
    /// the SQL rail on the row below.
    lifted: usize,
    receive: f32,
    noise: f32,
}

impl RxRows {
    /// The box is as wide as its wider row.
    fn w(&self) -> f32 {
        self.receive.max(self.noise)
    }
}

/// Measure the RX box's two rows and pick where the chip run breaks between
/// them. A free function of the state that changes them, like
/// [`tx_rows_fixed_w`], so `the_condensed_rx_box_fits_its_rows` can price
/// every combination without an app around it.
///
/// The rows are not a fixed division of the controls, because the receive row
/// is the one that varies with the *rig*: a front-end gain rail where there is
/// a gain to set, a decimation chip where there is a span to throw away, the
/// AGC chip and the manual rail behind it where the mode has an AGC at all. On
/// an SDR in SSB that row is full; on a CAT rig on a sound card in DRM — no
/// gain, no decimation, no AGC — it is a volume rail and nothing else, while
/// the row beneath it carries every chip in the box. The box hugs its wider
/// row, so that shape spends half of the box on nothing and charges the strip
/// for it: with the DRM chip on the end of the noise row it ran the RX box
/// into the boxes beside it and pushed the last of them off the window.
///
/// So the chips fill the receive row's tail while that leaves the box
/// narrower, and the run breaks wherever the wider row is narrowest. A rig
/// whose receive row is already full lifts nothing and is laid out exactly as
/// before; a bare one comes out a third narrower, which is often the
/// difference between the strip packing into two rows and taking a third.
fn rx_rows(ui: &egui::Ui, gain: bool, decim: bool, agc_off: bool, mode: Mode) -> RxRows {
    let g = MODULE_ROW_SPACING;
    // The Vol and SQL rails, which is what the box's stretch lengthens — so
    // they are priced at the floor they fall back to, not the style width.
    let rail = STRIP_RAIL_W;
    let label =
        |s: &str| crate::chrome::text_width(ui, s, egui::TextStyle::Body.resolve(ui.style()));
    let chip = |s: &str| crate::chrome::chip_width(ui, s, None);

    // Receive: volume, the front-end gain rail where the rig has one, the
    // decimation chip, the AGC chip, and the manual rail behind it.
    let mut receive = label("Vol") + g + rail;
    if gain {
        receive += g + label("Gain") + g + db_rail_w(ui);
    }
    if decim {
        // "DEC off" and "DEC /64" measure much the same, so the chip has one
        // width whatever it is set to — it either rides this row or, on a
        // radio with no span to spare, is not drawn at all.
        receive += g + chip("DEC off").max(chip("DEC /64"));
    }
    // At the widest of the four settings, so the box does not change width —
    // and the strip does not re-break its rows — as the AGC is cycled. In FM
    // and DRM the chain bypasses the AGC and neither the chip nor the manual
    // rail is drawn at all (Mode::audio_agc).
    if mode.audio_agc() {
        receive += g + AgcMode::ALL
            .iter()
            .map(|a| chip(&format!("AGC {}", a.label())))
            .fold(0.0, f32::max);
        if agc_off {
            receive += g + label("Man") + g + db_rail_w(ui);
        }
    }

    // Filter / noise: the squelch rail and its readout — the deepest threshold
    // is the longest the dBFS one reads, "off" at the bottom of the range being
    // shorter. Priced against the *radio's* readout as well, and always: which
    // of the two rails is drawn is the front end's answer, and a front end can
    // be swapped under a running window (`Engine::adopt_source`), so a box
    // sized for one of them would change width when the operator applied a new
    // interface.
    let sql_readout = format!("{:.0}", sdroxide_types::SQUELCH_OPEN_DB);
    let noise = label("SQL")
        + g
        + rail
        + g
        + value_field_w(ui, &sql_readout).max(value_field_w(ui, "100%"));

    // Then the run itself. Each chip is priced at its widest label, so the box
    // does not breathe as a decode comes and goes (see [`RxChip::width_label`])
    // and the run does not re-break under the operator's cursor.
    let chips: Vec<f32> = rx_chips(mode).iter().map(|c| g + c.width(ui)).collect();
    let all: f32 = chips.iter().sum();
    // Ties keep a chip where it is, so the layout every rig has always had —
    // the whole run under the SQL rail — is what comes back unless lifting a
    // chip out of it genuinely buys width.
    let mut best = RxRows { lifted: 0, receive, noise: noise + all };
    let mut up = 0.0;
    for (i, w) in chips.iter().enumerate() {
        up += w;
        let cand = RxRows { lifted: i + 1, receive: receive + up, noise: noise + all - up };
        if cand.w() < best.w() {
            best = cand;
        }
    }
    best
}

/// The natural width of the RIT/XIT offset row: the chips at their labels and
/// the Hz fields at their design width, with the box's row spacing between
/// them. A receive-only rig draws no XIT.
fn vfo_offsets_w(ui: &egui::Ui, tx_capable: bool) -> f32 {
    let g = MODULE_ROW_SPACING;
    let rit = crate::chrome::chip_width(ui, "RIT", None) + g + HZ_FIELD_W;
    if tx_capable {
        rit + g + crate::chrome::chip_width(ui, "XIT", None) + g + HZ_FIELD_W
    } else {
        rit
    }
}

/// How tall the band/mode chip is drawn: a plain chip plus
/// [`BAND_MODE_EXTRA_H`], or as much of it as the row it stands in can spare.
/// The compact strips cut their rows to the point — a phone's frequency box is
/// [`PHONE_FREQ_H`] and no taller — and a chip that outgrew one would push the
/// box out of line with the S-meter beside it rather than stand out.
fn band_mode_h(ui: &egui::Ui) -> f32 {
    let base = crate::chrome::chip_height(ui, Some(BAND_MODE_TEXT));
    (base + BAND_MODE_EXTRA_H).min(ui.available_height().max(base))
}

/// The labels on the VFO box's top row, in the order they are drawn.
///
/// DUPLEX is a transmit-side control and a receive-only front end does not pay
/// for it, exactly as it does not pay for XIT on the row below. TONE stays
/// either way: the receive tone squelch behind it belongs to the receiver.
fn vfo_chip_labels(tx_capable: bool) -> Vec<&'static str> {
    let mut labels = VFO_CHIPS.to_vec();
    if tx_capable {
        labels.push(DUPLEX_CHIP);
    }
    labels.push(TONE_CHIP);
    labels
}

/// [`SdroxideApp::readout`]'s arithmetic, over the state alone — so what the
/// readout says on the air can be tested without an app around it.
fn readout_for(
    state: &RadioState,
    tx_on: bool,
    cw_qrg: bool,
    cw_pitch_hz: f32,
) -> (f64, Option<Color32>, f64) {
    let dial = state.active_freq_hz();
    // The transmit case first: while a repeater shift is putting RF somewhere
    // else, where it is going matters more than anything below.
    if tx_on && state.repeater.shift != Shift::Simplex {
        let offset = state.tx_freq_hz() - dial;
        return (dial + offset, Some(crate::theme::ALERT()), offset);
    }
    // CW read as the signal rather than as the dial, when asked for. A CW dial
    // sits a sidetone pitch below what is being copied, so this is the number
    // both operators would quote — the same one `Mode::on_air_hz` answers and
    // the CW panel already shows. The offset rides back with it, so a wheel
    // turn or a typed frequency still moves the dial by what the operator
    // changed rather than jumping it by the pitch.
    if cw_qrg && state.rx[0].mode == Mode::Cw {
        return (dial + f64::from(cw_pitch_hz), None, f64::from(cw_pitch_hz));
    }
    (dial, None, 0.0)
}

/// One row of the SPEC popup: a caption naming the setting, then a chip per
/// step of it. Chips rather than a `ComboBox` for the reason
/// [`SdroxideApp::spectrum_controls`] gives — a combo opened inside a popup
/// counts as a click outside it and closes it under the operator.
///
/// The caption carries the explanation, and each chip only what that step
/// does: the caption is where an operator who does not yet know what the row
/// is for will point, and a paragraph repeated on every chip is a paragraph
/// nobody reads.
fn speed_row(ui: &mut egui::Ui, caption: &str, value: &mut Speed, steps: &[Speed], hint: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(caption).size(10.0).color(crate::theme::CYAN_DIM()))
            .on_hover_text(hint);
        for step in steps {
            if crate::chrome::chip(ui, value == step, step.label().to_uppercase()).clicked() {
                *value = *step;
            }
        }
    });
}

/// What a [`SpectrumDetail`] step is called on its chip: the column count it
/// asks for, which is the whole of what it means. Read off `columns()` rather
/// than written out again, so the chips can never disagree with the widths.
fn detail_chip_label(d: SpectrumDetail) -> String {
    match d.columns() {
        Some(c) => c.to_string(),
        None => "AUTO".to_string(),
    }
}

/// The width both of the SPEC popup's boxes are drawn at: the widest row
/// either of them holds, or as much of it as a popup may be wide. Priced
/// rather than left to auto-size, so the two come out matching — a short box
/// beside a long one reads as an unfinished one — and so their rows wrap
/// against a width the layout knows. The rows are measured with one gap per
/// chip rather than one between them, which leaves the margin that keeps the
/// longest row off the edge.
fn panadapter_group_w(ui: &egui::Ui) -> f32 {
    let gap = ui.spacing().item_spacing.x;
    let chips = |labels: &[&str]| -> f32 {
        labels.iter().map(|l| crate::chrome::chip_width(ui, l, None) + gap).sum()
    };
    let caption =
        |t: &str| crate::chrome::text_width(ui, t, egui::FontId::proportional(10.0)) + gap;
    let speeds = |steps: &[Speed]| -> f32 {
        steps
            .iter()
            .map(|s| crate::chrome::chip_width(ui, &s.label().to_uppercase(), None) + gap)
            .sum()
    };
    let detail: f32 = SpectrumDetail::ALL
        .iter()
        .map(|d| crate::chrome::chip_width(ui, &detail_chip_label(*d), None) + gap)
        .sum();
    let widest = [
        chips(&["SHOW SPECTRUM", "PEAK HOLD", "3D"]),
        caption("surface") + chips(&["LINES", "SOLID"]),
        caption("flow") + speeds(&Speed::SURFACE),
        caption("reaction") + speeds(&Speed::ALL),
        caption("scroll") + speeds(&Speed::WATERFALL),
        caption("detail") + detail + caption("8192 columns"),
    ]
    .into_iter()
    .fold(0.0f32, f32::max);
    // Never wider than a menu's body, which is capped at the screen and at 430
    // points whatever the screen (see `chrome::menu_popup`). The DISP menu
    // inlines these boxes, and a box priced past its body would be *clipped*
    // there rather than wrapped — on a phone, whose touch metrics make every
    // chip roomier, the detail row is exactly the row that would go over.
    // Taken from the screen rather than from `available_width`, which inside
    // an auto-sizing popup is the width the popup came out as last frame: a
    // box measured against that shrinks the popup, which shrinks the next
    // measurement, and it never grows back.
    let cap = (ui.ctx().content_rect().width() - 40.0).clamp(140.0, 414.0);
    widest.min(cap)
}

/// The natural width of a row of chips inside a condensed box: each chip at
/// its label, with the box's row spacing between them.
fn chip_row_w(ui: &egui::Ui, labels: &[&str]) -> f32 {
    let chips: f32 = labels.iter().map(|l| crate::chrome::chip_width(ui, l, None)).sum();
    chips + MODULE_ROW_SPACING * (labels.len() - 1) as f32
}

/// A chip stretched `extra` points past its label — for the condensed boxes'
/// rows, whose chips split the row's slack evenly. At `extra` 0 it draws
/// exactly what [`crate::chrome::chip`] would.
pub(in crate::app) fn chip_stretched(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    extra: f32,
) -> egui::Response {
    let size = egui::vec2(
        crate::chrome::chip_width(ui, label, None) + extra,
        crate::chrome::chip_height(ui, None),
    );
    crate::chrome::chip_sized(ui, selected, label, size)
}

/// [`chip_stretched`] with an accent fill — the SAT and SCAN chips while their
/// background work runs.
fn accent_chip_stretched(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    fill: Color32,
    ink: Color32,
    extra: f32,
) -> egui::Response {
    let size = egui::vec2(
        crate::chrome::chip_width(ui, label, None) + extra,
        crate::chrome::chip_height(ui, None),
    );
    crate::chrome::chip_accent_sized(ui, selected, label, fill, ink, size)
}

/// Width the System box needs for its chips over the two rows: the
/// wider row plus the box's side margins. Measured against the live style
/// rather than fixed, because a touched layout pads every chip out past its
/// desktop width — see `the_condensed_system_box_fits_its_chips`.
fn system_rows_w(ui: &egui::Ui) -> f32 {
    chip_row_w(ui, &SYSTEM_CHIPS_TOP).max(chip_row_w(ui, &SYSTEM_CHIPS_BOTTOM))
        + 2.0 * crate::chrome::MODULE_MARGIN_X
}

/// Why a band chip is greyed out, in the operator's terms.
///
/// The answer is always the same shape — the band is outside what this radio
/// receives — but the *remedy* is not: a range typed on the Radio page is one
/// the operator can widen or clear, while one the device published is a fact
/// about the hardware. Issue #272 was the version of this with no message at
/// all, where a stale receive range left over from another interface greyed out
/// the one band above HF a transceiver had.
fn disabled_band_reason(band: Band, caps: Option<&DeviceCaps>, stated: bool) -> String {
    let ranges = caps
        .map(|c| sdroxide_types::format_freq_ranges(&c.freq_ranges_rx))
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| "nothing".to_string());
    format!(
        "{} is outside what this radio receives ({ranges} MHz).\n\n{}",
        band.label(),
        if stated {
            "That range was typed in Settings \u{25b8} Radio \u{25b8} RX range, not reported by \
             the radio. Widen it, or empty the box to use whatever the device says about itself."
        } else {
            "That is what the device reports about itself. If the radio does cover this band, \
             state its real range in Settings \u{25b8} Radio \u{25b8} RX range."
        }
    )
}

/// The band + mode + digital chip rows: the body of the band/mode popup.
///
/// A free function taking the state it draws from, rather than a method, so a
/// test can lay the whole menu out on a phone-sized viewport without an app
/// around it — see `the_band_menu_fits_a_phone_screen`.
fn band_mode_menu(
    ui: &mut egui::Ui,
    mode: Mode,
    state: &RadioState,
    caps: Option<&DeviceCaps>,
    // Whether the receive range in force was typed on the Radio page rather
    // than published by the device — the two want different advice when a band
    // turns out to be unreachable, and only the caller knows which it is.
    ranges_stated: bool,
    // Passed in rather than read off the app, so the layout test above can
    // still build this menu without one. `None` is the normal state until the
    // solar window has been opened once, and colours nothing.
    conditions: Option<&sdroxide_solar::BandConditions>,
    daylight: bool,
    cmds: &mut Vec<Command>,
) {
    crate::chrome::menu_caption(ui, "Band");
    let digital = mode.is_digital();
    ui.horizontal_wrapped(|ui| {
        for b in Band::ALL {
            // A band the station's own band plan does not give this region gets
            // no button: 4 m is Region 1's alone and 1.25 m and 33 cm are the
            // Americas', and offering an operator a button that tunes outside
            // their own allocation — out of band, and with `tx_ham_only` set,
            // straight into a transmit lockout — would be offering them
            // something their licence has not got. GEN is the one bandless entry
            // that stays: it is the absence of a band.
            if b != Band::Gen && b.edges().is_none() {
                continue;
            }
            // In a digital mode, a band button tunes to the band's standard
            // dial frequency where the mode has one (SetVfo keeps the mode),
            // and the chip carries a cyan underline saying so. A band without
            // one — every band, in RF Paint's case — jumps to the band's
            // default frequency instead, still keeping the mode: any band can
            // be picked in any mode, standard frequency or not. Outside the
            // digital modes a click is a normal band change through the band
            // stack.
            let std_hz = if digital { digi_freq_for_band(mode, b) } else { None };
            let digi_hz = match std_hz {
                Some(hz) => Some(hz),
                None if digital => Some(b.default_entry().0),
                None => None,
            };
            // A radio that publishes no tuning range keeps every band button:
            // `may_rx_span` reads an empty range list as "the driver didn't
            // say", and greying out the whole bar would be a worse guess than
            // offering a band the radio turns out not to reach. Any *overlap*
            // is enough — a receiver that reaches into the band without
            // reaching either end of it still has the band (issue #272).
            let enabled =
                caps.is_none_or(|c| b.edges().is_none_or(|(lo, hi)| c.may_rx_span(lo, hi)));
            let active = match std_hz {
                Some(hz) => (state.active_freq_hz() - hz).abs() < 500.0,
                None => state.band == b,
            };
            // The published forecast, where there is one. Colour only: the
            // chip still says what band it is, and a band nothing is published
            // about — 160 m, 60 m, and everything above 10 m — looks exactly
            // as it did before rather than being given a verdict it has not
            // got. The words, the source and the age are in the tooltip.
            let verdict = conditions.and_then(|c| c.for_band(b, daylight));
            let tint = verdict
                .map(sdroxide_solar::BandRating::of)
                .and_then(crate::app::bands::rating_color);
            let resp = crate::chrome::chip_enabled_tinted(
                ui,
                enabled,
                active,
                b.label(),
                tint,
                std_hz.is_some(),
            );
            let resp = match verdict {
                Some(v) => resp.on_hover_text(format!(
                    "{}: {v} ({}) — forecast by HAMQSL.com from the solar indices, \
                     not a measurement of your own path.",
                    b.label(),
                    if daylight { "daytime" } else { "night" },
                )),
                None => resp,
            };
            // A chip that cannot be pressed has to say why. A band greyed out
            // with no explanation is what issue #272 was: an HF-plus-6 m
            // transceiver whose receive range said HF, and one dead button with
            // nothing on screen naming the range or where it came from.
            let resp = if enabled {
                resp
            } else {
                resp.on_disabled_hover_text(disabled_band_reason(b, caps, ranges_stated))
            };
            if resp.clicked() {
                match digi_hz {
                    Some(hz) => cmds.push(Command::SetVfo { vfo: state.active_vfo, hz }),
                    None => cmds.push(Command::SetBand(b)),
                }
            }
        }
    });
    ui.add_space(6.0);
    crate::chrome::menu_caption(ui, "Mode");
    ui.horizontal_wrapped(|ui| {
        for m in [
            Mode::Lsb,
            Mode::Usb,
            Mode::Cw,
            Mode::Am,
            Mode::Sam,
            Mode::Nfm,
            Mode::Wfm,
            // DRM belongs with the analog modes rather than under "Digital"
            // below: that heading is the modes the digi engine decodes and
            // transmits, and DRM is a broadcast to listen to — a demodulator,
            // like WFM beside it.
            Mode::Drm,
            Mode::Digu,
            Mode::Digl,
            Mode::Dsb,
            Mode::Isb,
            Mode::Spec,
        ] {
            if crate::chrome::chip(ui, mode == m, m.label()).clicked() {
                cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
            }
        }
    });
    ui.add_space(6.0);
    crate::chrome::menu_caption(ui, "Digital");
    ui.horizontal_wrapped(|ui| {
        // ADS-B, VDL2 and AIS ride along at the end of this row rather than in
        // [`Mode::DIGITAL`] itself: that list is what the digi engine decodes
        // and transmits, and neither of these is — each has its own lane, no
        // QSO and no transmitter. They are digital signals all the same, and
        // this is where an operator looks for one.
        for m in Mode::DIGITAL.into_iter().chain([Mode::Adsb, Mode::Vdl2, Mode::Ais]) {
            if crate::chrome::chip(ui, mode == m, m.label()).clicked() {
                cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
            }
        }
    });
}

/// The VFO A/B selector chips. In the frequency box on a desktop, in the VFO
/// menu on a phone — one definition either way.
fn vfo_ab_chips(ui: &mut egui::Ui, active: Vfo, cmds: &mut Vec<Command>) {
    for (v, label) in [(Vfo::A, "A"), (Vfo::B, "B")] {
        if crate::chrome::chip(ui, active == v, RichText::new(label).size(15.0)).clicked() {
            cmds.push(Command::SelectVfo(v));
        }
    }
}

/// The sub receiver's mode, picked from the audio modes it can wear. Returns
/// the newly chosen mode, or `None` if the current one still stands.
///
/// Audio modes only. The digital modes are wired to the main receiver alone
/// (one decoder, one TX), and SPEC produces no audio at all — a sub receiver
/// you cannot hear is a trap, not a setting. DRM is left out for the first of
/// those reasons: it would decode, but its status, its service picker and its
/// constellation are all read off the main receiver, so a sub receiver in DRM
/// would cost a whole second decoder and show nothing.
///
/// A combo where a fixed-width row has to hold ten of them; a wrapped row of
/// chips in a menu, where a combo would open a second popup layer and clicking
/// it would count as "outside" and close the menu underneath it. One list
/// either way, so the two cannot come to disagree about what the sub can do.
fn sub_mode_picker(ui: &mut egui::Ui, cur: Mode, narrow: bool) -> Option<Mode> {
    const MODES: [Mode; 10] = [
        Mode::Lsb,
        Mode::Usb,
        Mode::Cw,
        Mode::Am,
        Mode::Sam,
        Mode::Nfm,
        Mode::Wfm,
        Mode::Digu,
        Mode::Digl,
        Mode::Dsb,
    ];
    let mut picked = None;
    if narrow {
        ui.horizontal_wrapped(|ui| {
            for m in MODES {
                if crate::chrome::chip(ui, cur == m, m.label()).clicked() {
                    picked = Some(m);
                }
            }
        });
    } else {
        ComboBox::from_id_salt("sub-mode").selected_text(cur.label()).width(74.0).show_styled(
            ui,
            |ui| {
                for m in MODES {
                    if ui.selectable_label(cur == m, m.label()).clicked() {
                        picked = Some(m);
                    }
                }
            },
        );
    }
    picked
}

/// How many digit columns the frequency readout needs: [`freq_display::DIGITS`]
/// normally, [`freq_display::DIGITS_EXT`] once anything puts a real digit in the
/// ten-GHz column. Three independent tells, any one of them enough:
///
/// * `range_reaches_10ghz` — the receiver's published receive range (already
///   shifted by the configured converter/LNB offset) reaches 10 GHz or past it.
/// * `converter_offset_hz` — a 3-cm / 10 GHz converter is configured (LNB LO
///   9750–10600 MHz, i.e. `|offset|` of about 9.75 GHz and up — a QO-100
///   station's is 9.75 GHz exactly, well short of 10 GHz), so the column is
///   there *before* the operator tunes up rather than appearing mid-digit. A
///   driver that publishes no ranges at all (SoapySDR makes that optional)
///   leaves the first tell blind, and this is what still earns that station
///   its column. The `9e9` cut clears every 3-cm converter and stays above
///   the next transverter down (13 cm, ~2.3 GHz).
/// * `active_freq_hz` — the dial is already up there, converter or not (a
///   wideband direct sampler, a paired panadapter).
///
/// Without the extra column the dial reading 10489.750 MHz has nowhere to put
/// its leading "1" and is silently shown as "0489.750.000".
fn readout_digit_count(
    range_reaches_10ghz: bool,
    converter_offset_hz: f64,
    active_freq_hz: f64,
) -> u32 {
    let earns_extra =
        range_reaches_10ghz || converter_offset_hz.abs() >= 9e9 || active_freq_hz >= 1e10;
    if earns_extra { freq_display::DIGITS_EXT } else { freq_display::DIGITS }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Walk a chip through a sequence of pointer edges, collecting the PTT
    /// commands it asks for. `(down, touch, click)` per edge, as
    /// [`PttPress::on_pointer`] takes them.
    fn drive(edges: &[(bool, bool, bool)]) -> (PttPress, Vec<bool>) {
        let mut state = PttPress::default();
        let mut cmds = Vec::new();
        for &(down, touch, click) in edges {
            let (next, cmd) = state.on_pointer(down, touch, click);
            state = next;
            cmds.extend(cmd);
        }
        (state, cmds)
    }

    /// The rule the hold exists for, and the one thing that may not change:
    /// letting go of a finger always drops the transmitter. Not "usually" and
    /// not "unless it was quick" — a tap on a phone is a click by egui's
    /// reckoning too, and if that latched, the mis-tap this chip is guarded
    /// against would leave a radio transmitting into a pocket.
    #[test]
    fn a_finger_never_latches_the_transmitter() {
        // A tap: press and release, both inside egui's click window.
        let (state, cmds) = drive(&[(true, true, false), (false, true, true)]);
        assert_eq!(cmds, vec![true, false], "a tap keys and unkeys");
        assert_eq!(state, PttPress::Idle);
        assert!(!state.keying());

        // And a hold, which is what it is meant to be used as.
        let (state, cmds) = drive(&[(true, true, false), (false, false, false)]);
        assert_eq!(cmds, vec![true, false]);
        assert_eq!(state, PttPress::Idle);
    }

    /// A mouse gets the desktop chip's latch back: click on, click off. The
    /// second press is what ends the over — its release owes nothing, or it
    /// would unkey an over the operator had already started again.
    #[test]
    fn a_mouse_click_latches_and_the_next_press_lets_go() {
        let mut state = PttPress::default();
        let mut cmds = Vec::new();
        for &(down, click) in &[(true, false), (false, true), (true, false), (false, true)] {
            let (next, cmd) = state.on_pointer(down, false, click);
            state = next;
            cmds.extend(cmd);
        }
        assert_eq!(cmds, vec![true, false], "keyed on the first click, dropped on the second");
        assert_eq!(state, PttPress::Idle);

        // Mid-latch it is keying with nothing pressing it — which is what the
        // window-close backstop reads, and what the edge test must not mistake
        // for a pointer still being down.
        let (mid, _) = drive(&[(true, false, false), (false, false, true)]);
        assert_eq!(mid, PttPress::Latched);
        assert!(mid.keying() && !mid.pressed());
    }

    /// A hold outlasts egui's context-menu gesture. The finger is still on
    /// the chip, so the over is still on the air — the widget losing the press
    /// to a gesture nothing here uses may not be read as the finger lifting.
    #[test]
    fn a_long_touch_does_not_cut_the_over() {
        let (state, _) = drive(&[(true, true, false)]);
        assert_eq!(state, PttPress::Keying { touch: true });
        // The long-press frame: egui has taken the press off the widget, but
        // the button is still down.
        assert!(state.still_down(false, true), "the finger has not lifted");
        // And when it really does lift, both agree and the over ends.
        assert!(!state.still_down(false, false));

        // Never the other way round: a pointer pressed elsewhere while this
        // chip is idle or latched must not start or extend an over here.
        for idle in [PttPress::Idle, PttPress::Latched] {
            assert!(!idle.still_down(false, true), "{idle:?}");
        }
    }

    /// Push-to-talk still works with a mouse, and that is the case a latch
    /// must not steal: a press held past egui's click window is no longer a
    /// click, so the release ends the over instead of leaving it on the air.
    /// Same for a press dragged off the chip, which is how an accidental one
    /// is taken back.
    #[test]
    fn a_mouse_press_held_or_dragged_away_ends_the_over() {
        for release in [(false, false, false), (false, true, false)] {
            let (state, cmds) = drive(&[(true, false, false), release]);
            assert_eq!(cmds, vec![true, false], "{release:?}");
            assert_eq!(state, PttPress::Idle);
        }
    }

    /// Share Tech Mono advances 0.540 em and Chakra's " Hz" 1.392 em, so the
    /// readout costs 7.438 pt of width per point of digit size. Measured from
    /// the shipped fonts; the assertions below are what that buys at each of
    /// the viewport widths the tiers were drawn for.
    fn shipped() -> ReadoutFit {
        ReadoutFit { per_pt: 10.0 * 0.540 + 3.0 * 0.540 + 0.3 * 1.392, h_per_pt: 1.0, gaps: 13.0 }
    }

    #[test]
    fn fitting_a_size_and_measuring_it_back_agree() {
        let f = shipped();
        for size in [22.0f32, 25.0, 30.0, 34.0, 40.0] {
            let round_trip = f.fit(f.width(size));
            assert!((round_trip - size).abs() < 1e-3, "fit(width({size})) came back {round_trip}");
        }
    }

    #[test]
    fn the_desktop_readout_keeps_its_design_width() {
        // The box the desktop has always drawn: 512.5 pt wide overall.
        let f = shipped();
        let readout = f.width(crate::widgets::freq_display::DIGIT_SIZE);
        assert!((readout - 310.5).abs() < 0.5, "readout measured {readout}");
        let box_w = 8.0 + AB_W + 10.0 + readout + 12.0 + RIGHT_W + 8.0;
        assert!((box_w - 512.5).abs() < 0.5, "box measured {box_w}");
    }

    /// "20m \u{b7} USB" at 14 pt Chakra with a touched layout's chip padding.
    /// `freq_module` measures this rather than assuming [`RIGHT_W`], because it
    /// runs past that column's design width — which is what used to push the
    /// S-meter onto a row of its own on a tablet in portrait.
    const TOUCH_BAND_CHIP_W: f32 = 101.0;

    /// The digit size `freq_module` settles on for a `viewport` this wide, and
    /// the width of the box it builds around it.
    fn tablet_box(f: &ReadoutFit, viewport: f32) -> (f32, f32) {
        // Less the top panel's 8+8 and angled_frame's 10+10.
        let content = viewport - 36.0;
        let right_w = RIGHT_W.max(TOUCH_BAND_CHIP_W);
        let overhead = AB_W + right_w + 38.0;
        let size = f.fit(content - overhead - (SMETER_W + 12.0)).clamp(MIN_DIGIT, 40.0);
        (size, 8.0 + AB_W + 10.0 + f.width(size) + 12.0 + right_w + 8.0)
    }

    #[test]
    fn a_tablet_in_portrait_fits_the_readout_beside_the_smeter() {
        let f = shipped();
        let content = 768.0 - 36.0;
        let (size, box_w) = tablet_box(&f, 768.0);
        assert!((MIN_DIGIT..=40.0).contains(&size), "size {size} left its range");
        // Both boxes and the gap between them stay inside the row.
        assert!(box_w + 8.0 + SMETER_W <= content, "{box_w} + meter overflowed {content}");
    }

    #[test]
    fn a_landscape_tablet_gets_the_full_size_digits() {
        let f = shipped();
        let (size, box_w) = tablet_box(&f, 1024.0);
        assert_eq!(size, 40.0, "a 1024 pt tablet has room for the design size");
        assert!(box_w + 8.0 + SMETER_W <= 1024.0 - 36.0, "{box_w} + meter overflowed");
    }

    /// The needle's radius is set by the box *width* (its arc is a chord
    /// across it), so the arc gets taller as the box gets wider. A meter
    /// stretched across a phone would draw the ends of its scale below its own
    /// box; holding the aspect is what stops that.
    #[test]
    fn the_phone_smeter_keeps_its_scale_inside_its_box() {
        // The arc hangs from `13·k` below the top of the box (smeter.rs).
        let k = (PHONE_SMETER_H / 72.0).clamp(0.55, 2.0);
        let room = PHONE_SMETER_H - 13.0 * k;
        let half: f32 = 31.0_f32.to_radians();
        for w in [PHONE_SMETER_MIN_W, 150.0, PHONE_SMETER_MAX_W] {
            let rad = ((w - 14.0) / (2.0 * half.sin())).max(24.0);
            // How far the arc drops from its ends to its centre.
            let extent = rad * (1.0 - half.cos());
            assert!(extent <= room, "a {w} pt wide arc spans {extent} of {room} pt");
        }
    }

    /// Every menu chip the phone strip can carry, measured from the touched
    /// layout it is laid out with: the label at 14.5 pt Chakra in a chip padded
    /// 13 pt a side, so "TX" fills a 43 pt chip and "DISP" — the widest — a 57
    /// pt one. The PTT, at 15 pt and with its label's own spaces for padding,
    /// comes out 59; the band/mode chip runs to [`TOUCH_BAND_CHIP_W`].
    const P_MENU: [(&str, f32, f32); 6] = [
        ("RX", 44.1, 18.1),
        ("VFO", 52.8, 26.8),
        ("SUB", 54.1, 28.1),
        ("TX", 42.9, 16.9),
        ("DISP", 57.1, 31.1),
        ("SYS", 51.4, 25.4),
    ];
    const P_PTT_W: f32 = 59.2;
    const P_TEXT: f32 = 14.5;
    /// Every gap on the strip: the `item_spacing` [`SdroxideApp::top_bar`] sets
    /// for itself, not the style's own.
    const P_GAP: f32 = 8.0;
    /// The active-VFO tag at 13 pt — the same figure the short strip is
    /// measured with.
    const P_TAG_W: f32 = 9.0;

    /// The menu chips a rig in this state carries: what each measures, and the
    /// widest label's text width.
    fn p_menu(tx: bool, sub: bool) -> (Vec<f32>, f32) {
        let keep = |l: &str| match l {
            "SUB" => sub,
            "TX" => tx,
            _ => true,
        };
        let menu = P_MENU.iter().filter(|(l, ..)| keep(l)).map(|&(_, w, _)| w).collect();
        let text =
            P_MENU.iter().filter(|(l, ..)| keep(l)).map(|&(_, _, t)| t).fold(0.0f32, f32::max);
        (menu, text)
    }

    /// The whole phone strip as it lays itself out on a `viewport` this wide:
    /// what [`SdroxideApp::freq_module_compact`] decides about the readout and
    /// the band/mode chip, and what [`plan_phone_tail`] then makes of the row
    /// that is left — all of it in plain numbers, measured off the constants
    /// above rather than a live style.
    struct PhoneStrip {
        /// Content width of the strip: the viewport less the panel margins.
        content: f32,
        /// Was the band/mode chip kept in the frequency box?
        band_mode_shown: bool,
        tail: PhoneTail,
        /// Rows the whole strip takes, the frequency box's included.
        rows: usize,
        /// What the meter's row carries: the frequency box where it shares one
        /// with it, then the meter and the chips that stay beside it.
        meter_row: f32,
    }

    fn a_phone_strip(viewport: f32, tx: bool, sub: bool) -> PhoneStrip {
        let f = shipped();
        // Less the top panel's 8+8 and angled_frame's 10+10.
        let content = viewport - 36.0;
        // `freq_module_compact`, in the order it decides things.
        let fixed = 16.0 + P_TAG_W + 6.0;
        let with_chip = f.fit(content - fixed - 8.0 - TOUCH_BAND_CHIP_W).min(PHONE_DIGIT_MAX);
        let band_mode_shown = with_chip >= MIN_DIGIT;
        let size = if band_mode_shown {
            with_chip
        } else {
            f.fit(content - fixed).clamp(MIN_DIGIT, PHONE_DIGIT_MAX)
        };
        let box_w =
            fixed + f.width(size) + if band_mode_shown { 8.0 + TOUCH_BAND_CHIP_W } else { 0.0 };

        let (menu, text_w) = p_menu(tx, sub);
        let mut lead = (0usize, 0.0f32);
        if !band_mode_shown {
            lead = (1, TOUCH_BAND_CHIP_W + P_GAP);
        }
        if tx {
            lead = (lead.0 + 1, lead.1 + P_PTT_W + P_GAP);
        }
        let chips = PhoneChips { lead, menu: &menu, text_w, text_size: P_TEXT };
        // The wrapping layout puts the meter after the box, with a gap.
        let left = content - box_w - P_GAP;
        let tail = plan_phone_tail(content, left, &chips, P_GAP);

        // The meter shares the frequency box's row exactly when the plan gave
        // it more than what a fresh row would have left it.
        let shares = tail.meter_w + lead.1 + 6.0 <= left;
        let lead_w = lead.1 + lead.0 as f32 * tail.lead_extra;
        PhoneStrip {
            content,
            band_mode_shown,
            tail,
            rows: 1 + usize::from(!shares) + usize::from(tail.grid.is_some()),
            meter_row: if shares { box_w + P_GAP } else { 0.0 } + tail.meter_w + lead_w,
        }
    }

    /// The point of the plan: on every phone the tier dresses, the menu
    /// buttons are *one* row — never four of them across and a fifth alone
    /// under them, which costs the waterfall a whole row of height to show one
    /// button and leaves the rest of it empty.
    #[test]
    fn the_phone_menu_buttons_always_share_one_row() {
        for viewport in [320.0f32, 360.0, 375.0, 393.0, 412.0, 430.0, 480.0, 540.0, 599.0] {
            for (tx, sub) in [(true, false), (true, true), (false, false), (false, true)] {
                let s = a_phone_strip(viewport, tx, sub);
                let what = format!("{viewport} pt, tx={tx} sub={sub}");
                assert!(s.rows <= 3, "{what}: the strip took {} rows", s.rows);
                assert!(
                    s.meter_row <= s.content + 0.5,
                    "{what}: the meter's row wants {} of {}",
                    s.meter_row,
                    s.content
                );
                assert!(
                    (PHONE_SMETER_MIN_W..=PHONE_SMETER_MAX_W).contains(&s.tail.meter_w),
                    "{what}: the meter came out {} pt",
                    s.tail.meter_w
                );
                let Some(g) = s.tail.grid else {
                    continue;
                };
                let (menu, text_w) = p_menu(tx, sub);
                let n = menu.len() as f32;
                let row = n * g.cell_w + (n - 1.0) * P_GAP;
                assert!(row <= s.content + 0.5, "{what}: the buttons want {row} of {}", s.content);
                // And every label lands inside the cell it is centred in: a
                // chip drawn to an exact size does not clip its text, it lets
                // it hang over the edges.
                let ink = text_w * g.text.unwrap_or(P_TEXT) / P_TEXT;
                assert!(
                    ink + 2.0 * MENU_TEXT_PAD <= g.cell_w + 0.5,
                    "{what}: a {ink} pt label in a {} pt cell",
                    g.cell_w
                );
            }
        }
    }

    /// A 412x915 phone in portrait, the screen the strip is worn on most: the
    /// readout on one row, the meter and the PTT on the next, and five buttons
    /// across the third — each of them *wider* than its label asked for,
    /// because the row is divided between them rather than left part empty.
    #[test]
    fn a_phone_in_portrait_spends_the_row_on_its_buttons() {
        let s = a_phone_strip(412.0, true, false);
        assert!(s.band_mode_shown, "the frequency box had room for the band/mode chip");
        assert_eq!(s.rows, 3, "the strip took {} rows", s.rows);
        let g = s.tail.grid.expect("the buttons take a row of their own");
        assert!(g.text.is_none(), "the labels shrank to {:?} with room to spare", g.text);
        let widest = P_MENU.iter().map(|&(_, w, _)| w).fold(0.0f32, f32::max);
        assert!(g.cell_w > widest, "cells came out {} against a {widest} pt chip", g.cell_w);
        // The meter takes all it may of its row and the PTT — the only chip
        // beside it — takes the rest, so that row reaches the edge too.
        assert_eq!(s.tail.meter_w, PHONE_SMETER_MAX_W);
        assert!(s.tail.lead_extra > 0.0, "the PTT was left at its label width");
        assert!(s.meter_row >= s.content - 6.5, "{} of {} used", s.meter_row, s.content);
    }

    /// A phone in landscape — 852x393 — keeps the meter on the readout's row
    /// (three points short of holding the buttons there too), so the strip is
    /// two rows. The buttons still divide the second, but only up to twice the
    /// chip their labels asked for: a row 816 pt wide would otherwise put "RX"
    /// in a 163 pt button.
    #[test]
    fn a_phone_in_landscape_stops_stretching_its_buttons() {
        let s = a_phone_strip(852.0, true, false);
        assert_eq!(s.rows, 2, "the strip took {} rows", s.rows);
        let g = s.tail.grid.expect("the buttons take a row of their own");
        let widest = P_MENU.iter().map(|&(_, w, _)| w).fold(0.0f32, f32::max);
        assert_eq!(g.cell_w, CHIP_STRETCH_FACTOR * widest, "cells came out {}", g.cell_w);
        // The buttons' row still has to be one the layout will break onto:
        // wider than whatever the meter and the PTT left of the row above.
        let (menu, _) = p_menu(true, false);
        let n = menu.len() as f32;
        let grid_w = n * g.cell_w + (n - 1.0) * P_GAP;
        assert!(
            grid_w > s.content - s.meter_row,
            "a {grid_w} pt row of buttons fits the {} pt left of the row above it",
            s.content - s.meter_row
        );
    }

    /// The plan leans on two things the wrapping layout does, so they are
    /// checked against egui rather than assumed: a block as wide as the row is
    /// given a fresh row of its own rather than squeezed onto the end of the
    /// one it is on, and the cells inside it then divide that row without it
    /// breaking again under them. The strip's skeleton at phone metrics, with
    /// a spacer standing in for the frequency box and the real chips.
    #[test]
    fn the_button_row_is_broken_out_of_the_strip_whole() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        crate::layout::set_tier(&ctx, crate::layout::Tier::Phone);
        crate::theme::apply_metrics(&ctx, crate::layout::Tier::Phone);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(412.0, 915.0),
            )),
            ..Default::default()
        };
        const LABELS: [&str; 5] = ["RX", "VFO", "TX", "DISP", "SYS"];
        let (mut tops, mut right, mut edge) = (Vec::new(), 0.0f32, 0.0f32);
        ctx.run_ui(input, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
                |ui| {
                    let row = ui.max_rect().width();
                    edge = ui.max_rect().right();
                    let gap = ui.spacing().item_spacing.x;
                    let sense = egui::Sense::hover();
                    // The frequency box, which takes the row on a phone held
                    // upright — the case the buttons used to spill in.
                    ui.allocate_exact_size(egui::vec2(row, PHONE_FREQ_H), sense);

                    let font = egui::TextStyle::Button.resolve(ui.style());
                    let mut menu = Vec::new();
                    let mut text_w = 0.0f32;
                    for l in LABELS {
                        menu.push(crate::chrome::chip_width(ui, l, None));
                        text_w = text_w.max(crate::chrome::text_width(ui, l, font.clone()));
                    }
                    let ptt_w = crate::chrome::chip_width(ui, PTT_LABEL, Some(PTT_TEXT));
                    let left = row - (ui.cursor().min.x - ui.max_rect().min.x).max(0.0);
                    let chips = PhoneChips {
                        lead: (1, ptt_w + gap),
                        menu: &menu,
                        text_w,
                        text_size: font.size,
                    };
                    let tail = plan_phone_tail(row, left, &chips, gap);

                    ui.allocate_exact_size(egui::vec2(tail.meter_w, PHONE_SMETER_H), sense);
                    let ptt_h = crate::chrome::chip_height(ui, Some(PTT_TEXT));
                    ui.allocate_exact_size(egui::vec2(ptt_w + tail.lead_extra, ptt_h), sense);

                    let g = tail.grid.expect("the buttons take a row of their own");
                    let h = crate::chrome::chip_height(ui, None);
                    let n = LABELS.len() as f32;
                    let w = n * g.cell_w + (n - 1.0) * gap;
                    ui.allocate_ui_with_layout(
                        egui::vec2(w, h),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            for l in LABELS {
                                let cell = egui::vec2(g.cell_w, h);
                                let r = crate::chrome::chip_sized(ui, false, l, cell).rect;
                                tops.push(r.top());
                                right = right.max(r.right());
                            }
                        },
                    );
                },
            );
        })
        .drop_without_applying_deltas();
        let top = tops[0];
        assert!(tops.iter().all(|t| (t - top).abs() < 0.5), "the buttons landed at {tops:?}");
        assert!(right <= edge + 0.5, "the last button reaches {right} past a {edge} pt edge");
    }

    /// Where the row cannot afford a cell wide enough for "DISP" at the
    /// style's size, every label shrinks by the same factor rather than hang
    /// over the edges of the chip it is centred in. The narrowest phone keeps
    /// its size with five buttons up and gives a fraction of a point with the
    /// sub's sixth; a row narrower than any phone — a desktop window dragged
    /// down to a sliver, which the tier dresses as a phone — gives real
    /// ground, and never past the floor.
    #[test]
    fn a_row_too_narrow_for_the_labels_shrinks_them_into_their_cells() {
        assert!(
            a_phone_strip(320.0, true, false).tail.grid.is_some_and(|g| g.text.is_none()),
            "five buttons on the narrowest phone had to shrink their labels"
        );
        let six = a_phone_strip(320.0, true, true).tail.grid.expect("a row of their own");
        assert!(six.text.is_some_and(|s| (14.0..P_TEXT).contains(&s)), "six came out {six:?}");
        let (menu, text_w) = p_menu(true, true);
        let chips = PhoneChips {
            lead: (2, TOUCH_BAND_CHIP_W + P_GAP + P_PTT_W + P_GAP),
            menu: &menu,
            text_w,
            text_size: P_TEXT,
        };
        let tail = plan_phone_tail(240.0, 240.0, &chips, P_GAP);
        let g = tail.grid.expect("the buttons take a row of their own");
        let size = g.text.expect("the labels had to give");
        assert!((MENU_TEXT_MIN..P_TEXT).contains(&size), "the labels were set at {size} pt");
        assert!(
            text_w * size / P_TEXT + 2.0 * MENU_TEXT_PAD <= g.cell_w + 0.5,
            "a shrunk label still overhangs a {} pt cell",
            g.cell_w
        );
    }

    /// Metrics measured from the touched layout the short strip is laid out
    /// with: a chip stands 41 pt; "VFO" fills a 56 pt chip and "DISP" a 62 pt
    /// one; the band/mode chip runs to 101 pt at its widest label; the PTT
    /// comes out 76 pt wide; the active-VFO tag 9 pt.
    const T_CHIP_H: f32 = 41.0;
    const T_CELL1: f32 = 56.0;
    const T_CELL2: f32 = 62.0;
    const T_BM_W: f32 = 101.0;
    const T_PTT_W: f32 = 76.0;

    fn a_short_strip(avail: f32, tx: bool, sub: bool) -> ShortStrip {
        let f = shipped();
        let chips = StripChips {
            chip_h: T_CHIP_H,
            tag_w: 9.0,
            bm_w: T_BM_W,
            ptt_w: if tx { T_PTT_W } else { 0.0 },
            row1: if sub { (3, T_CELL1) } else { (2, T_CELL1) },
            row2: if tx { (3, T_CELL2) } else { (2, T_CELL2) },
        };
        plan_short_strip(avail, &f, &chips, 8.0, 8.0)
    }

    /// 600 pt is the narrowest viewport the tablet tier dresses; less the top
    /// panel's and `angled_frame`'s margins the strip gets 564. The box, the
    /// PTT and the grid at its minimum all have to land on the one row the
    /// strip is — a block that wrapped would take the very height the strip
    /// exists to give back.
    #[test]
    fn the_short_strip_fits_the_narrowest_short_window() {
        for (tx, sub) in [(true, false), (true, true), (false, false)] {
            let p = a_short_strip(564.0, tx, sub);
            let ptt = if tx { T_PTT_W + 8.0 } else { 0.0 };
            let total = p.box_w + 8.0 + ptt + p.grid_w;
            assert!(total <= 564.0, "tx={tx} sub={sub}: the strip wants {total} of 564");
            assert!(p.digit >= STRIP_DIGIT_MIN, "tx={tx} sub={sub}: digits fell to {}", p.digit);
            // Stretched cells never squeeze a chip below its own label.
            assert!(
                p.cell1_w + 0.5 >= T_CELL1 && p.cell2_w + 0.5 >= T_CELL2,
                "tx={tx} sub={sub}: cells squeezed to {} / {}",
                p.cell1_w,
                p.cell2_w
            );
        }
    }

    /// The screen the strip was drawn for: 1280x720. The readout reaches its
    /// cap and every point the box and the PTT do not take goes to the
    /// buttons — which is what "the buttons scale to fit the width" means —
    /// on a strip nearly half the height of the stacked rows it replaced.
    #[test]
    fn on_a_720p_screen_the_buttons_take_the_slack() {
        let p = a_short_strip(1280.0 - 36.0, true, false);
        assert_eq!(p.digit, STRIP_DIGIT_MAX, "room to spare caps the digits");
        assert!(p.cell1_w > 2.0 * T_CELL1, "row 1 cells stayed at {}", p.cell1_w);
        assert!(p.cell2_w > 2.0 * T_CELL2, "row 2 cells stayed at {}", p.cell2_w);
        // Everything on the strip shares this height. The old tablet layout
        // stacked a 74 pt frequency box over a 74 pt meter-and-menus row.
        assert!(p.box_h <= 90.0, "the strip stands {} pt", p.box_h);
    }

    /// A fresh context dressed with the Desktop tier's metrics on a roomy
    /// viewport — what the condensed-box layout tests draw in.
    fn desktop_ctx() -> (egui::Context, egui::RawInput) {
        let ctx = egui::Context::default();
        crate::layout::set_tier(&ctx, crate::layout::Tier::Desktop);
        crate::theme::apply_metrics(&ctx, crate::layout::Tier::Desktop);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(2560.0, 1440.0),
            )),
            ..Default::default()
        };
        (ctx, input)
    }

    /// Reserve the condensed System box the way the desktop strip does and
    /// draw its two chip rows into it the way `windows_condensed` does, at
    /// zero stretch. Hands back the width the box left for its contents, and
    /// how far each chip reached into it, both measured from the box's inner
    /// left edge.
    fn system_box_and_chips() -> (f32, Vec<(&'static str, f32)>) {
        let (ctx, input) = desktop_ctx();
        let mut out = None;
        ctx.run_ui(input, |ui| {
            let mut chips = Vec::new();
            let width = system_rows_w(ui);
            let room =
                crate::chrome::module_bare_h(ui, width, crate::chrome::MODULE_TALL_H, |ui| {
                    // Read before the rows are drawn: egui grows a Ui's
                    // max_rect to cover content that overflowed it, so
                    // afterwards it reports what the rows took rather than
                    // what the box offered.
                    let (left, room) = (ui.max_rect().left(), ui.max_rect().width());
                    // Left-aligned rather than centred so a chip's reach is
                    // measured from the box edge, the worst case.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.spacing_mut().item_spacing =
                            egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                        for row in [&SYSTEM_CHIPS_TOP[..], &SYSTEM_CHIPS_BOTTOM[..]] {
                            ui.horizontal(|ui| {
                                for label in row {
                                    let right = chip_stretched(ui, false, label, 0.0).rect.right();
                                    chips.push((*label, right - left));
                                }
                            });
                        }
                    });
                    room
                });
            out = Some((room, chips));
        })
        .drop_without_applying_deltas();
        out.expect("the box was drawn")
    }

    /// Every chip in the System box has to fit inside the width the box reserved
    /// for it.
    ///
    /// A module reserves its width before its contents are drawn, and a row that
    /// does not fit is not clipped to the box — it carries on past it, and
    /// whatever crosses the window edge is simply gone. The box was sized 285 pt
    /// by hand for five chips and kept that literal through three more (BANDS,
    /// SCAN, and the widened SETTINGS), by which point the row needed twice the
    /// box: SCAN, SETTINGS and HELP fell off the right-hand edge on any layout
    /// that left the box near the end of a row. Nothing about the chips said so
    /// — they were drawn every frame, just past the edge of the window.
    #[test]
    fn the_condensed_system_box_fits_its_chips() {
        let (room, chips) = system_box_and_chips();
        for (label, right) in chips {
            assert!(
                right <= room + 0.5,
                "{label} reaches {right} pt into a box with room for {room}"
            );
        }
    }

    /// Lay the condensed Display box's two rows out with the real chips at
    /// desktop metrics: each row has to fit the width [`chip_row_w`] prices
    /// for it, with and without the WIDE chip.
    #[test]
    fn the_condensed_display_box_fits_its_chips() {
        for has_wide in [false, true] {
            let (ctx, input) = desktop_ctx();
            ctx.run_ui(input, |ui| {
                let row1: &[&str] =
                    if has_wide { &DISPLAY_VIEW_CHIPS } else { &DISPLAY_VIEW_CHIPS[..2] };
                let room = chip_row_w(ui, row1).max(chip_row_w(ui, &DISPLAY_TOOL_CHIPS));
                for row in [row1, &DISPLAY_TOOL_CHIPS[..]] {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = MODULE_ROW_SPACING;
                        for label in row {
                            chip_stretched(ui, false, label, 0.0);
                        }
                        let took = ui.min_rect().width();
                        assert!(
                            took <= room + 0.5,
                            "has_wide={has_wide}: {row:?} took {took} of {room}"
                        );
                    });
                }
            })
            .drop_without_applying_deltas();
        }
    }

    /// The DIV box's rows have to fit the width it reserved, at both of the
    /// mode chip's labels — the box is planned once and the chip cycles under
    /// it, so pricing the shorter one would push RESTART off the right-hand
    /// edge every time the filter was switched to cancelling.
    #[test]
    fn the_diversity_box_fits_its_rows() {
        let (ctx, input) = desktop_ctx();
        ctx.run_ui(input, |ui| {
            let room = div_rows_w(ui) - 2.0 * crate::chrome::MODULE_MARGIN_X;
            for label in DIV_MODE_LABELS {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = MODULE_ROW_SPACING;
                    ui.label(RichText::new("DIV").size(11.0).strong());
                    crate::chrome::chip(ui, false, label);
                    crate::chrome::chip(ui, false, "HOLD");
                    crate::chrome::chip(ui, false, "RESTART");
                    let took = ui.min_rect().width();
                    assert!(took <= room + 0.5, "the {label} row took {took} of {room}");
                });
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = MODULE_ROW_SPACING;
                ui.spacing_mut().slider_width = STRIP_RAIL_W;
                ui.label("Adapt");
                let mut v = 0.5f32;
                crate::chrome::slider(ui, Slider::new(&mut v, 0.0..=1.0).show_value(false));
                let took = ui.min_rect().width();
                assert!(took <= room + 0.5, "the adaptation row took {took} of {room}");
            });
        })
        .drop_without_applying_deltas();
    }

    /// Which of the two rails the transmit box offers, and why each answer is
    /// the one that does something (issue #186).
    ///
    /// The rail replaces the mic one, so getting this wrong does not merely add
    /// a useless control — it takes away a working one, or leaves a dead one in
    /// place of the control the operator went looking for.
    #[test]
    fn the_transmit_audio_rail_appears_where_it_does_something() {
        use sdroxide_types::DeviceCaps;

        // A CAT rig on its sound card: it modulates what we hand it.
        let cat = DeviceCaps { tx_audio: true, ..DeviceCaps::default() };
        // The same rig, with CW going out through its own keyer instead.
        let cat_rig_keyer = DeviceCaps { tx_audio: true, ..DeviceCaps::default() };
        let cat_mcw = DeviceCaps { tx_audio: true, cw_audio_keyed: true, ..DeviceCaps::default() };
        // An SDR we modulate ourselves: the modulator and Drive own the level.
        let sdr = DeviceCaps::default();

        // The modes the report is about.
        for mode in [Mode::Ft8, Mode::Rtty, Mode::Psk, Mode::Aprs, Mode::Rade] {
            assert!(
                digi_tx_level_applies_to(mode, Some(&cat)),
                "{mode:?} transmits through this level and was not offered it"
            );
            assert!(
                !digi_tx_level_applies_to(mode, Some(&sdr)),
                "{mode:?} was offered a level that does nothing on an SDR"
            );
        }

        // Voice and the receive-only modes never reach it.
        for mode in [Mode::Usb, Mode::Lsb, Mode::Nfm, Mode::Am, Mode::Wefax, Mode::Drm] {
            assert!(
                !digi_tx_level_applies_to(mode, Some(&cat)),
                "{mode:?} was offered a level it never uses"
            );
        }

        // CW only where the sound card is in the path. With the rig's own keyer
        // sending, CW leaves as text over the control port.
        assert!(digi_tx_level_applies_to(Mode::Cw, Some(&cat_mcw)), "MCW was not offered a level");
        assert!(
            !digi_tx_level_applies_to(Mode::Cw, Some(&cat_rig_keyer)),
            "a rig-keyed CW mode was offered an audio level it never touches"
        );

        // And before the capabilities have arrived, nothing is offered.
        assert!(!digi_tx_level_applies_to(Mode::Ft8, None));
    }

    /// The envelope processor is offered on the two surfaces that can draw it
    /// exactly where it reaches the air: voice sideband on a radio whose
    /// sideband we make ourselves. The rail went missing from the desktop
    /// strip entirely (issue #294) because the two surfaces did not share this
    /// rule — or any rule.
    #[test]
    fn the_cessb_rail_appears_where_it_does_something() {
        use sdroxide_types::DeviceCaps;

        // An SDR we modulate ourselves — where the processor lives.
        let sdr = DeviceCaps::default();
        // A radio that modulates the audio we send it: a CAT rig on its sound
        // card, and one whose receive stream is demodulated audio too.
        let cat = DeviceCaps { tx_audio: true, ..DeviceCaps::default() };
        let cat_audio_rx = DeviceCaps { audio_mode: true, ..DeviceCaps::default() };

        for mode in [Mode::Usb, Mode::Lsb] {
            assert!(cessb_applies_to(mode, Some(&sdr)), "{mode:?} on an SDR was not offered CESSB");
            for caps in [&cat, &cat_audio_rx] {
                assert!(
                    !cessb_applies_to(mode, Some(caps)),
                    "{mode:?} was offered a processor the radio's own DSP is downstream of"
                );
            }
        }

        // Not in a mode whose payload is the envelope this flattens, nor in
        // one that does not transmit at all.
        for mode in [Mode::Ft8, Mode::Rtty, Mode::Rade, Mode::Nfm, Mode::Am, Mode::Cw, Mode::Drm] {
            assert!(
                !cessb_applies_to(mode, Some(&sdr)),
                "{mode:?} was offered an envelope processor with nothing to flatten"
            );
        }

        // And before the capabilities have arrived, nothing is offered.
        assert!(!cessb_applies_to(Mode::Usb, None));
    }

    /// Lay the condensed TX box's rows and mic column out with real widgets at
    /// desktop metrics and check each fits the width [`tx_rows_fixed_w`] and
    /// [`TX_MIC_COL_W`] price for it — which is what keeps
    /// [`TX_SLIDER_VALUE_W`] and the mic column honest against what egui
    /// actually draws.
    #[test]
    fn the_condensed_tx_box_fits_its_rows() {
        for keyer in [false, true] {
            let (ctx, input) = desktop_ctx();
            ctx.run_ui(input, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                let (fixed1, fixed2) = tx_rows_fixed_w(ui, keyer);
                // Both rows are drawn at the rail the box reserves for them.
                let rail = STRIP_RAIL_W;
                ui.spacing_mut().slider_width = rail;
                // 100% is the widest the readouts get, so the rows are laid
                // out at it.
                let (mut drive, mut tune, mut mic) = (1.0f32, 1.0f32, 1.0f32);
                let pct = |v: f64, _| format!("{:.0}%", v * 100.0);
                let row1 = ui
                    .horizontal(|ui| {
                        let size = tx_key_chip_size(ui);
                        crate::chrome::chip_accent_sized(
                            ui,
                            false,
                            RichText::new(" PTT ").size(15.0).strong().color(crate::theme::ALERT()),
                            crate::theme::ALERT(),
                            Color32::WHITE,
                            size,
                        );
                        if keyer {
                            crate::chrome::chip_accent(
                                ui,
                                false,
                                RichText::new(" ▶ ").size(15.0),
                                crate::theme::CYAN(),
                                crate::theme::INK_ON_CYAN(),
                            );
                        }
                        ui.label("Drive");
                        crate::chrome::slider(
                            ui,
                            Slider::new(&mut drive, 0.0..=1.0)
                                .show_value(true)
                                .custom_formatter(pct),
                        );
                        ui.min_rect().width()
                    })
                    .inner;
                let row2 = ui
                    .horizontal(|ui| {
                        let size = tx_key_chip_size(ui);
                        crate::chrome::chip_accent_sized(
                            ui,
                            false,
                            RichText::new(" TUNE ").size(15.0),
                            crate::theme::YELLOW(),
                            crate::theme::INK_ON_CYAN(),
                            size,
                        );
                        ui.label("Tune");
                        crate::chrome::slider(
                            ui,
                            Slider::new(&mut tune, 0.0..=1.0)
                                .show_value(true)
                                .custom_formatter(pct),
                        );
                        ui.min_rect().width()
                    })
                    .inner;
                let mic_w = ui
                    .horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(RichText::new("Mic").size(10.5));
                            ui.spacing_mut().slider_width = 45.0;
                            crate::chrome::slider(
                                ui,
                                Slider::new(&mut mic, 0.0..=1.0).vertical().show_value(false),
                            );
                        });
                        ui.min_rect().width()
                    })
                    .inner;
                // The other rail that can stand in that column (issue #186),
                // measured at the widest caption it can show — its caption is
                // its readout, so this is the figure `TX_LEVEL_COL_W` has to
                // cover.
                let mut db = sdroxide_types::TX_AUDIO_LEVEL_MIN_DB;
                let level_w = ui
                    .vertical(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(RichText::new(format!("{db:.0} dB")).size(10.5));
                            ui.spacing_mut().slider_width = 45.0;
                            crate::chrome::slider(
                                ui,
                                Slider::new(&mut db, sdroxide_types::TX_AUDIO_LEVEL_MIN_DB..=0.0)
                                    .vertical()
                                    .show_value(false),
                            );
                        });
                        ui.min_rect().width()
                    })
                    .inner;
                // The envelope rail that joins the mic one in voice sideband
                // (issue #294), measured at its caption — which is its name,
                // the widest thing in the column.
                let mut cessb = 0.0f32;
                let cessb_w = ui
                    .vertical(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(RichText::new("CESSB").size(10.5));
                            ui.spacing_mut().slider_width = 45.0;
                            crate::chrome::slider(
                                ui,
                                Slider::new(&mut cessb, 0.0..=sdroxide_types::CESSB_MAX_DB)
                                    .vertical()
                                    .show_value(false),
                            );
                        });
                        ui.min_rect().width()
                    })
                    .inner;
                let (room1, room2) = (fixed1 + rail, fixed2 + rail);
                assert!(row1 <= room1 + 0.5, "keyer={keyer}: row 1 took {row1} of {room1}");
                assert!(row2 <= room2 + 0.5, "keyer={keyer}: row 2 took {row2} of {room2}");
                assert!(mic_w <= TX_MIC_COL_W + 0.5, "the mic column took {mic_w}");
                assert!(
                    level_w <= TX_LEVEL_COL_W + 0.5,
                    "the transmit-audio column took {level_w} of {TX_LEVEL_COL_W}"
                );
                assert!(
                    cessb_w <= TX_CESSB_COL_W + 0.5,
                    "the CESSB column took {cessb_w} of {TX_CESSB_COL_W}"
                );
            })
            .drop_without_applying_deltas();
        }
    }

    /// The desktop meter's box has no ceiling — the needle face is what caps
    /// and centres its own scale (see `smeter::NEEDLE_FACE_MAX_W`), so however
    /// wide the packer draws the box, the ends of the scale stay inside a
    /// [`crate::chrome::MODULE_TALL_H`] box.
    #[test]
    fn the_stretched_desktop_smeter_keeps_its_scale_inside_its_box() {
        let h = crate::chrome::MODULE_TALL_H;
        let k = (h / 72.0).clamp(0.55, 2.0);
        let room = h - 13.0 * k;
        let half: f32 = 31.0_f32.to_radians();
        for w in [SMETER_W, 439.0, 1000.0, 4000.0] {
            let chord = w.min(crate::widgets::smeter::NEEDLE_FACE_MAX_W);
            let rad = ((chord - 14.0) / (2.0 * half.sin())).max(24.0);
            let extent = rad * (1.0 - half.cos());
            assert!(extent <= room, "a {w} pt wide box spans an arc of {extent} in {room} pt");
        }
    }

    /// Shorthand for the packer tests: a rigid box, and a flexible one.
    fn rigid(w: f32) -> StripBox {
        StripBox { w, flex: 0.0, max_w: w }
    }
    fn flexible(w: f32, flex: f32, max_w: f32) -> StripBox {
        StripBox { w, flex, max_w }
    }

    #[test]
    fn the_packer_keeps_one_row_and_fills_it() {
        let plan =
            plan_top_strip(1000.0, 8.0, &[rigid(300.0), flexible(200.0, 1.0, 400.0), rigid(300.0)]);
        assert_eq!(plan.rows.len(), 1, "everything fits one row");
        let row = &plan.rows[0];
        assert_eq!(row.boxes, 0..3);
        assert_eq!(row.widths[0], 300.0, "a rigid box never grows");
        assert_eq!(row.widths[2], 300.0);
        // The flexible box absorbed the slack: the row spans avail - 2.
        let total: f32 = row.widths.iter().sum::<f32>() + 2.0 * 8.0;
        assert!((total - 998.0).abs() < 0.6, "the row spans {total} of 998");
    }

    #[test]
    fn the_packer_prefers_balanced_rows_over_first_fit() {
        // First-fit would take three boxes on the first row and leave the
        // fourth almost alone: [300 300 300 | 550], leftovers 50 and 400.
        // Breaking after two spreads it: [300 300 | 300 550], 350 and 100.
        let boxes = [rigid(300.0), rigid(300.0), rigid(300.0), rigid(550.0)];
        let plan = plan_top_strip(950.0, 0.0, &boxes);
        assert_eq!(plan.rows.len(), 2, "two rows still suffice");
        assert_eq!(plan.rows[0].boxes, 0..2);
        assert_eq!(plan.rows[1].boxes, 2..4);
    }

    #[test]
    fn a_three_row_strip_comes_out_even() {
        // Six equal boxes over three rows land two per row — never a lone
        // box on the last row the way first-fit's tail would leave it.
        let plan = plan_top_strip(700.0, 0.0, &[rigid(300.0); 6]);
        assert_eq!(plan.rows.len(), 3);
        for (i, row) in plan.rows.iter().enumerate() {
            assert_eq!(row.boxes.len(), 2, "row {i} carries {:?}", row.boxes);
        }
    }

    #[test]
    fn the_packer_never_reorders() {
        let boxes = [rigid(500.0), rigid(200.0), rigid(400.0), rigid(100.0), rigid(450.0)];
        let plan = plan_top_strip(800.0, 8.0, &boxes);
        let mut next = 0;
        for row in &plan.rows {
            assert_eq!(row.boxes.start, next, "rows are contiguous and in order");
            next = row.boxes.end;
        }
        assert_eq!(next, boxes.len(), "every box was placed exactly once");
    }

    #[test]
    fn stretch_water_fills_against_the_caps() {
        // 98 points of slack for two flexible boxes with equal flex: the
        // capped one takes its 40 and hands the rest to the open one.
        let boxes = [rigid(300.0), flexible(200.0, 1.0, 240.0), flexible(200.0, 1.0, 1000.0)];
        let plan = plan_top_strip(800.0, 0.0, &boxes);
        let row = &plan.rows[0];
        assert_eq!(row.widths[0], 300.0);
        assert!((row.widths[1] - 240.0).abs() < 0.6, "the capped box stops at its cap");
        assert!((row.widths[2] - 258.0).abs() < 0.6, "the open box takes the rest");
        assert_eq!(row.extra_gap, 0.0);
    }

    #[test]
    fn a_row_of_capped_boxes_justifies_the_rest() {
        let boxes = [flexible(200.0, 1.0, 220.0), flexible(200.0, 1.0, 220.0)];
        let plan = plan_top_strip(600.0, 0.0, &boxes);
        let row = &plan.rows[0];
        assert_eq!(row.widths, vec![220.0, 220.0]);
        // 598 - 440 justified into the one gap between the two boxes.
        assert!((row.extra_gap - 158.0).abs() < 0.6, "extra gap came out {}", row.extra_gap);
    }

    #[test]
    fn an_oversize_box_gets_a_row_of_its_own() {
        let boxes = [rigid(300.0), rigid(900.0), rigid(300.0)];
        let plan = plan_top_strip(800.0, 8.0, &boxes);
        assert_eq!(plan.rows.len(), 3, "the oversize box forces its neighbours off");
        assert_eq!(plan.rows[1].boxes, 1..2);
        assert_eq!(plan.rows[1].widths[0], 900.0, "an oversize box keeps its width");
    }

    /// Leftover is only worth having on a row whose boxes can spend it.
    /// Natural balance would break these four evenly and leave 400 pt dead on
    /// each row; putting the bottomless box alone up top leaves 100 pt dead in
    /// total, and the packer has to prefer that.
    #[test]
    fn leftover_goes_where_it_can_be_absorbed() {
        let boxes = [flexible(300.0, 1.0, f32::INFINITY), rigid(300.0), rigid(300.0), rigid(300.0)];
        let plan = plan_top_strip(1000.0, 0.0, &boxes);
        assert_eq!(plan.rows.len(), 2);
        assert_eq!(plan.rows[0].boxes, 0..1, "the absorber takes the emptier row");
        assert_eq!(plan.rows[1].boxes, 1..4);
        assert!((plan.rows[0].widths[0] - 998.0).abs() < 0.6, "and swallows all of it");
    }

    /// The strip trades digit size for a whole row: design-width 450 forces a
    /// second row here, and 400 is the widest frequency box that packs into
    /// one — the search has to land just under it, and never lower.
    #[test]
    fn digits_shrink_just_enough_to_save_a_row() {
        let rest = [rigid(300.0), rigid(300.0)];
        let w = freq_w_for_fewest_rows(1000.0, 0.0, 450.0, 350.0, &rest);
        assert!((w - 400.0).abs() < 1.0, "the freq box came back {w}");
        let boxes = [rigid(w), rest[0], rest[1]];
        assert_eq!(rows_needed(1000.0, 0.0, &boxes), 1, "and one row now holds it");
    }

    /// When even the narrowest readout saves nothing — three rows either way
    /// here — the digits stay at their design size.
    #[test]
    fn digits_hold_their_design_size_when_no_row_is_saved() {
        let rest = [rigid(660.0), rigid(660.0)];
        let w = freq_w_for_fewest_rows(1000.0, 0.0, 450.0, 350.0, &rest);
        assert_eq!(w, 450.0);
    }

    /// Every box on the desktop strip, priced the way [`SdroxideApp::desktop_strip`]
    /// prices it, for a transmit-capable CAT rig with a full-band lane and no
    /// front-end gain or decimation of its own — an IC-705 over the LAN, which
    /// is the shape the strip first ran out of room on. Widths only; the boxes
    /// that need an app to measure (the frequency readout's side columns) are
    /// priced at their design figures, which is what a desktop gets.
    fn cat_rig_strip_boxes(ui: &egui::Ui, mode: Mode) -> Vec<StripBox> {
        let fit = ReadoutFit::measure(ui, freq_display::DIGITS);
        let freq_w = 8.0
            + AB_W
            + 10.0
            + fit.width(crate::layout::Tier::Desktop.digit_cap())
            + 12.0
            + RIGHT_W
            + 8.0;
        let vfo = chip_row_w(ui, &vfo_chip_labels(true)).max(vfo_offsets_w(ui, true))
            + 2.0 * crate::chrome::MODULE_MARGIN_X
            + 4.0;
        let rx =
            rx_rows(ui, false, false, false, mode).w() + 2.0 * crate::chrome::MODULE_MARGIN_X + 4.0;
        // A CAT rig modulates our audio, so a digital mode there draws the
        // transmit-audio rail rather than the mic one — and it is the wider of
        // the two. It is also why no CESSB rail joins them: the envelope is
        // built in the rig's own DSP, so the column is priced through the
        // strip's own rule rather than assumed away (issue #294).
        let caps = sdroxide_types::DeviceCaps { tx_audio: true, ..Default::default() };
        let side = if mode.takes_digi_tx_audio() { TX_LEVEL_COL_W } else { TX_MIC_COL_W }
            + if cessb_applies_to(mode, Some(&caps)) {
                MODULE_ROW_SPACING + TX_CESSB_COL_W
            } else {
                0.0
            };
        let tx = tx_rows_w_for(ui, mode.allows_voice_keyer(), side);
        let display = chip_row_w(ui, &DISPLAY_VIEW_CHIPS).max(chip_row_w(ui, &DISPLAY_TOOL_CHIPS))
            + 2.0 * crate::chrome::MODULE_MARGIN_X;
        let system = system_rows_w(ui);
        vec![
            StripBox { w: freq_w, flex: 0.0, max_w: freq_w },
            StripBox { w: SMETER_W, flex: 3.0, max_w: f32::INFINITY },
            StripBox { w: vfo, flex: 1.0, max_w: vfo + VFO_STRETCH_MAX },
            StripBox { w: rx, flex: 2.0, max_w: rx + RAIL_STRETCH_MAX },
            StripBox { w: tx, flex: 2.0, max_w: tx + RAIL_STRETCH_MAX },
            StripBox { w: display, flex: 1.0, max_w: display * CHIP_STRETCH_FACTOR },
            StripBox { w: system, flex: 1.0, max_w: system * CHIP_STRETCH_FACTOR },
        ]
    }

    /// The narrowest window the desktop tier takes still packs the whole strip
    /// into two rows — in every mode, including the ones that hang another
    /// chip or two on the RX box.
    ///
    /// It did not, and that is what [`STRIP_RAIL_W`] is for: with the Vol, SQL,
    /// Drive and Tune rails each reserved at the style's 84 pt, RX + TX +
    /// Display + System came to more than one row could hold, so System — and
    /// on a slightly narrower pane Display with it — spilled onto a third row
    /// while the S-meter above sat stretched over 300 pt of slack it had no use
    /// for. A third row costs the waterfall a whole module height; two shorter
    /// rails cost it nothing.
    ///
    /// The modes are here for the second half of issue #152. A CAT rig has no
    /// front-end gain and no decimation, so its receive row is a volume rail
    /// and — outside SSB, which is the only one of these four with an AGC —
    /// nothing else, while the chip run underneath grows a DRM light or an
    /// RDS one. Left whole under the squelch rail that run took the RX box to
    /// 498 pt and the strip to three rows; [`rx_rows`] breaks it across the
    /// two rows instead and the box comes back inside 330.
    #[test]
    fn the_desktop_strip_packs_a_cat_rig_into_two_rows() {
        let (ctx, input) = desktop_ctx();
        ctx.run_ui(input, |ui| {
            // `top_bar` sets the strip's own inter-box gap before it packs.
            let gap = 8.0;
            // 1400 pt is where `layout::tier_for` starts calling a window a
            // desktop; the top panel's 8+8 margin and `angled_frame`'s 10+10
            // come off it before the packer sees it.
            let avail = 1400.0 - 16.0 - 20.0;
            for mode in [Mode::Lsb, Mode::Nfm, Mode::Wfm, Mode::Drm] {
                let boxes = cat_rig_strip_boxes(ui, mode);
                let widths: Vec<f32> = boxes.iter().map(|b| b.w).collect();
                assert_eq!(
                    rows_needed(avail, gap, &boxes),
                    2,
                    "in {mode:?} the strip wants {rows} rows of a {avail} pt pane; \
                     boxes {widths:?}",
                    rows = rows_needed(avail, gap, &boxes),
                );
            }
        })
        .drop_without_applying_deltas();
    }

    /// The rail issue #294 put back costs the desktop strip no row: on the
    /// widest shape that draws it — an SDR in sideband with a front-end gain,
    /// a decimation chip and its AGC switched off, so every receive control is
    /// on the strip at once — the strip breaks into the same rows with the
    /// column as without it, at every width the desktop tier covers.
    ///
    /// A column is cheap and a row is not: a third row costs the waterfall a
    /// whole module height, which is what [`STRIP_RAIL_W`] exists to avoid.
    #[test]
    fn the_cessb_rail_costs_the_desktop_strip_no_row() {
        let (ctx, input) = desktop_ctx();
        ctx.run_ui(input, |ui| {
            let mut boxes = cat_rig_strip_boxes(ui, Mode::Lsb);
            // The receive box an SDR draws, in place of the CAT rig's.
            let rx = rx_rows(ui, true, true, true, Mode::Lsb).w()
                + 2.0 * crate::chrome::MODULE_MARGIN_X
                + 4.0;
            boxes[3] = StripBox { w: rx, flex: 2.0, max_w: rx + RAIL_STRETCH_MAX };
            let tx_w = |cessb: bool| {
                tx_rows_w_for(
                    ui,
                    true,
                    TX_MIC_COL_W + if cessb { MODULE_ROW_SPACING + TX_CESSB_COL_W } else { 0.0 },
                )
            };
            for avail in [1364.0, 1600.0, 1884.0, 2524.0, 3400.0] {
                let rows = |cessb: bool| {
                    let mut boxes = boxes.clone();
                    let w = tx_w(cessb);
                    boxes[4] = StripBox { w, flex: 2.0, max_w: w + RAIL_STRETCH_MAX };
                    rows_needed(avail, 8.0, &boxes)
                };
                assert_eq!(
                    rows(true),
                    rows(false),
                    "the CESSB column cost a {avail} pt strip a row: {} against {}",
                    rows(true),
                    rows(false),
                );
            }
        })
        .drop_without_applying_deltas();
    }

    /// And the box the packer's slack lands on stays a control box rather than
    /// becoming a 500 pt banner: whatever the row has spare, the VFO/RIT box
    /// takes at most [`VFO_STRETCH_MAX`] of it.
    #[test]
    fn the_vfo_box_does_not_swallow_a_rows_slack() {
        let (ctx, input) = desktop_ctx();
        ctx.run_ui(input, |ui| {
            let boxes = cat_rig_strip_boxes(ui, Mode::Lsb);
            let plan = plan_top_strip(1364.0, 8.0, &boxes);
            let vfo = plan
                .rows
                .iter()
                .find_map(|r| r.boxes.clone().position(|i| i == 2).map(|j| r.widths[j]))
                .expect("the VFO box was placed");
            assert!(
                vfo <= boxes[2].w + VFO_STRETCH_MAX + 0.5,
                "the VFO box came out {vfo} pt against a natural {}",
                boxes[2].w,
            );
        })
        .drop_without_applying_deltas();
    }

    /// Lay the condensed VFO/RIT box's rows out with real chips and drag
    /// values at desktop metrics and check both fit the width
    /// [`vfo_offsets_w`] and [`chip_row_w`] price — which is what keeps
    /// [`HZ_FIELD_W`] honest, in both the RX-only and the transmit-capable
    /// shape.
    #[test]
    fn the_condensed_vfo_box_fits_its_rows() {
        for tx_capable in [false, true] {
            let (ctx, input) = desktop_ctx();
            ctx.run_ui(input, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                let chips = vfo_chip_labels(tx_capable);
                let room = chip_row_w(ui, &chips).max(vfo_offsets_w(ui, tx_capable)) + 4.0;
                let row1 = ui
                    .horizontal(|ui| {
                        for label in chips {
                            chip_stretched(ui, false, label, 0.0);
                        }
                        ui.min_rect().width()
                    })
                    .inner;
                let row2 = ui
                    .horizontal(|ui| {
                        let mut hz = -9999i32;
                        crate::chrome::chip(ui, false, "RIT");
                        ui.add_sized(
                            [HZ_FIELD_W, 22.0],
                            DragValue::new(&mut hz).speed(5).range(-9999..=9999).suffix(" Hz"),
                        );
                        if tx_capable {
                            crate::chrome::chip(ui, false, "XIT");
                            ui.add_sized(
                                [HZ_FIELD_W, 22.0],
                                DragValue::new(&mut hz).speed(5).range(-9999..=9999).suffix(" Hz"),
                            );
                        }
                        ui.min_rect().width()
                    })
                    .inner;
                assert!(row1 <= room + 0.5, "tx={tx_capable}: row 1 took {row1} of {room}");
                assert!(row2 <= room + 0.5, "tx={tx_capable}: row 2 took {row2} of {room}");
            })
            .drop_without_applying_deltas();
        }
    }

    /// On the air through a repeater the readout follows the transmitter onto
    /// the repeater's input, and says so in the alert red. Off the air, and on
    /// simplex, it is the dial in the resting amber — which is every other
    /// moment of every other mode.
    #[test]
    fn the_readout_follows_the_transmitter_onto_a_repeaters_input() {
        let mut state = RadioState::default();
        state.vfo_a_hz = 145_712_500.0;
        state.vfo_b_hz = 145_712_500.0;

        // Simplex: the dial, whether or not anything is keyed.
        for tx in [false, true] {
            assert_eq!(
                readout_for(&state, tx, false, 700.0),
                (145_712_500.0, None, 0.0),
                "simplex, tx={tx}"
            );
        }

        state.repeater.shift = Shift::Minus;
        state.repeater.offset_hz = 600_000;
        // Shifted but listening: still the output, because that is what is
        // being listened to.
        assert_eq!(readout_for(&state, false, false, 700.0), (145_712_500.0, None, 0.0));
        // Keyed: the input, in the alert red, and the offset comes back so a
        // turn of the dial on those digits still moves the VFO by what was
        // turned rather than jumping it by the shift.
        let (shown, ink, offset) = readout_for(&state, true, false, 700.0);
        assert_eq!(shown, 145_112_500.0);
        assert_eq!(offset, -600_000.0);
        assert_eq!(ink, Some(crate::theme::ALERT()));
        assert_eq!(shown - offset, state.active_freq_hz(), "an edit maps back to the dial");

        // A shift stacked on XIT reads as the frequency that actually goes
        // out, not as the repeater's share of it.
        state.xit = sdroxide_types::OffsetState { enabled: true, hz: 250 };
        assert_eq!(readout_for(&state, true, false, 700.0).0, 145_112_750.0);
    }

    /// A CW dial sits a sidetone pitch below the signal, so an operator reading
    /// the dial is doing arithmetic to get the number they would put in the log
    /// or quote on the air. Asked for it, the readout does the arithmetic —
    /// and hands back the offset, so tuning those digits still moves the dial
    /// by what was turned rather than jumping it by the pitch.
    #[test]
    fn cw_can_read_the_signal_rather_than_the_dial() {
        let mut state = RadioState::default();
        state.vfo_a_hz = 14_050_000.0;
        state.vfo_b_hz = 14_050_000.0;
        state.rx[0].mode = Mode::Cw;

        // Off: the dial, as every other radio shows it.
        assert_eq!(readout_for(&state, false, false, 700.0), (14_050_000.0, None, 0.0));

        // On: the signal, which is the pitch above it.
        let (shown, ink, offset) = readout_for(&state, true, true, 700.0);
        assert_eq!(shown, 14_050_700.0);
        assert_eq!(offset, 700.0);
        assert_eq!(ink, None, "reading the signal is not an alert condition");
        assert_eq!(shown - offset, state.active_freq_hz(), "an edit maps back to the dial");

        // It follows the pitch the operator is actually copying at, not a
        // fixed 700 — the CW panel moves that with its ± buttons and with a
        // click on the waterfall.
        assert_eq!(readout_for(&state, false, true, 450.0).0, 14_050_450.0);

        // And it agrees with what the CW panel and the log already show.
        assert_eq!(shown, Mode::Cw.on_air_hz(state.active_freq_hz(), 700.0));

        // Only CW: the setting says nothing about any other mode.
        state.rx[0].mode = Mode::Usb;
        assert_eq!(readout_for(&state, false, true, 700.0), (14_050_000.0, None, 0.0));
    }

    /// The transmitter wins. A repeater shift has RF going somewhere else and
    /// that matters more than which end of the sidetone is being read.
    #[test]
    fn a_repeater_shift_outranks_the_cw_readout() {
        let mut state = RadioState::default();
        state.vfo_a_hz = 145_712_500.0;
        state.vfo_b_hz = 145_712_500.0;
        state.rx[0].mode = Mode::Cw;
        state.repeater.shift = Shift::Minus;
        state.repeater.offset_hz = 600_000;

        let (shown, ink, _) = readout_for(&state, true, true, 700.0);
        assert_eq!(shown, 145_112_500.0, "the transmit frequency, not the dial plus a pitch");
        assert_eq!(ink, Some(crate::theme::ALERT()));
    }

    /// Open the band/mode menu on a `screen`-sized viewport and measure the
    /// popup it produced.
    fn band_menu_rect(screen: egui::Vec2) -> egui::Rect {
        let ctx = egui::Context::default();
        let tier = crate::layout::tier_for(screen, sdroxide_types::LayoutMode::Auto);
        crate::layout::set_tier(&ctx, tier);
        crate::theme::apply_metrics(&ctx, tier);
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen)),
            ..Default::default()
        };
        let state = RadioState::default();
        let menu = |ui: &mut egui::Ui| {
            let btn = crate::chrome::chip(ui, false, "20m · USB");
            let id = egui::Popup::default_response_id(&btn);
            crate::chrome::menu_popup(ui, &btn, |ui| {
                band_mode_menu(
                    ui,
                    state.rx[0].mode,
                    &state,
                    None,
                    false,
                    None,
                    true,
                    &mut Vec::new(),
                );
            });
            id
        };
        // The first pass gives the chip an id; then it is opened and laid out.
        let mut id = None;
        ctx.run_ui(input(), |ui| id = Some(menu(ui))).drop_without_applying_deltas();
        let id = id.expect("the chip was drawn");
        egui::Popup::open_id(&ctx, id);
        ctx.run_ui(input(), |ui| {
            menu(ui);
        })
        .drop_without_applying_deltas();
        ctx.memory(|m| m.area_rect(id)).expect("the menu was shown")
    }

    /// The longest menu in the program, on the smallest screens it opens on.
    ///
    /// A popup is not laid out inside a panel: egui moves one that lands off an
    /// edge, but it cannot shrink one that is simply too big for the screen —
    /// the overflow hangs off the viewport where no finger can reach it. Forty
    /// chips in three sections is well past what a phone in landscape can show,
    /// so this menu only fits because [`crate::chrome::menu_popup`] bounds it
    /// and scrolls the rest.
    #[test]
    fn the_band_menu_fits_a_phone_screen() {
        for screen in [
            egui::vec2(360.0, 800.0),  // small phone, portrait
            egui::vec2(393.0, 852.0),  // common phone, portrait
            egui::vec2(852.0, 393.0),  // and in landscape
            egui::vec2(667.0, 375.0),  // a small phone in landscape: the tightest of all
            egui::vec2(768.0, 1024.0), // tablet, for company
        ] {
            let r = band_menu_rect(screen);
            assert!(
                r.width() <= screen.x && r.height() <= screen.y,
                "{screen:?}: the band menu came out {} x {}",
                r.width(),
                r.height()
            );
            assert!(
                r.left() >= 0.0
                    && r.right() <= screen.x
                    && r.top() >= 0.0
                    && r.bottom() <= screen.y,
                "{screen:?}: the band menu spans {r:?}"
            );
        }
    }

    /// Lay the condensed RX box's two rows out with the real widgets at
    /// desktop metrics, in every combination of the state that changes them,
    /// and check each fits the width [`rx_rows`] prices for it — including the
    /// break it picked for the chip run.
    ///
    /// The figure this replaced was a literal, and by the time the noise row
    /// had grown an ANC chip and a MONO chip it was 40 pt light. Nothing about
    /// the box said so: it drew its rows past its own right edge, pushing the
    /// TX, Display and System boxes along the row, and the System box — last
    /// on the row on a desktop layout — lost ISM and HELP over the edge of the
    /// window. The DRM chip did it again (issue #152), which is why the chips
    /// here come from [`rx_chips`] rather than from a list of their own: a
    /// chip the box does not know about cannot be drawn into it.
    #[test]
    fn the_condensed_rx_box_fits_its_rows() {
        for gain in [false, true] {
            for decim in [false, true] {
                for agc_off in [false, true] {
                    // One mode from each shape of the chip run: the plain
                    // five, CW's extra BIN, and the three that bring a chip of
                    // their own.
                    for mode in [Mode::Usb, Mode::Cw, Mode::Nfm, Mode::Wfm, Mode::Drm] {
                        let (ctx, input) = desktop_ctx();
                        ctx.run_ui(input, |ui| {
                            ui.spacing_mut().item_spacing =
                                egui::vec2(MODULE_ROW_SPACING, MODULE_ROW_SPACING);
                            // The box draws its Vol and SQL rails at what it
                            // reserved for them, so the rows are laid out here
                            // at the same figure.
                            ui.spacing_mut().slider_width = STRIP_RAIL_W;
                            let rows = rx_rows(ui, gain, decim, agc_off, mode);
                            let chips = rx_chips(mode);
                            let (mut vol, mut db) = (0.5f32, -88.8f32);
                            // The deepest threshold, which is the longest the
                            // readout beside the rail reads.
                            let mut sql = sdroxide_types::SQUELCH_OPEN_DB + 1.0;
                            let state = format!(
                                "gain={gain} decim={decim} agc_off={agc_off} mode={mode:?} \
                                 lifted={}",
                                rows.lifted
                            );
                            // Every chip at the widest label it wears, which is
                            // what the box reserved for it.
                            let draw = |ui: &mut egui::Ui, run: &[RxChip]| {
                                for c in run {
                                    crate::chrome::chip_accent(
                                        ui,
                                        false,
                                        c.width_label(),
                                        crate::theme::ALERT(),
                                        Color32::WHITE,
                                    );
                                }
                            };

                            let row1 = ui
                                .horizontal(|ui| {
                                    ui.label("Vol");
                                    crate::chrome::slider(
                                        ui,
                                        Slider::new(&mut vol, 0.0..=1.0).show_value(false),
                                    );
                                    if gain {
                                        ui.label("Gain");
                                        ui.scope(|ui| {
                                            ui.spacing_mut().slider_width = RX_DB_RAIL_W;
                                            crate::chrome::slider(
                                                ui,
                                                Slider::new(&mut db, -88.8..=0.0)
                                                    .step_by(0.1)
                                                    .suffix(" dB"),
                                            );
                                        });
                                    }
                                    if decim {
                                        crate::chrome::chip(ui, false, "DEC /64");
                                    }
                                    // The widest of the four AGC settings —
                                    // absent in FM and DRM, where the box draws
                                    // no AGC control (Mode::audio_agc).
                                    if mode.audio_agc() {
                                        crate::chrome::chip(ui, true, "AGC Slow");
                                        if agc_off {
                                            let mut man = sdroxide_types::MAX_MANUAL_GAIN_DB;
                                            ui.label("Man");
                                            ui.scope(|ui| {
                                                ui.spacing_mut().slider_width = RX_DB_RAIL_W;
                                                crate::chrome::slider(
                                                    ui,
                                                    Slider::new(
                                                        &mut man,
                                                        0.0..=sdroxide_types::MAX_MANUAL_GAIN_DB,
                                                    )
                                                    .step_by(1.0)
                                                    .suffix(" dB"),
                                                );
                                            });
                                        }
                                    }
                                    draw(ui, &chips[..rows.lifted]);
                                    ui.min_rect().width()
                                })
                                .inner;

                            let row2 = ui
                                .horizontal(|ui| {
                                    ui.label("SQL");
                                    crate::chrome::slider(
                                        ui,
                                        Slider::new(
                                            &mut sql,
                                            sdroxide_types::SQUELCH_OPEN_DB..=-30.0,
                                        )
                                        .show_value(true)
                                        .custom_formatter(|v, _| format!("{v:.0}")),
                                    );
                                    draw(ui, &chips[rows.lifted..]);
                                    ui.min_rect().width()
                                })
                                .inner;

                            let (room1, room2) = (rows.receive, rows.noise);
                            assert!(row1 <= room1 + 0.5, "{state}: row 1 took {row1} of {room1}");
                            assert!(row2 <= room2 + 0.5, "{state}: row 2 took {row2} of {room2}");
                        })
                        .drop_without_applying_deltas();
                    }
                }
            }
        }
    }

    #[test]
    fn the_readout_stays_at_ten_columns_for_an_ordinary_station() {
        // No range past 10 GHz, no converter, dial on HF / VHF / 23 cm.
        assert_eq!(readout_digit_count(false, 0.0, 14_074_000.0), freq_display::DIGITS);
        assert_eq!(readout_digit_count(false, 0.0, 1_296_000_000.0), freq_display::DIGITS);
        // A 13 cm transverter (~2.256 GHz offset) is the nearest converter
        // below a 3 cm one and must not trip the extra column.
        assert_eq!(
            readout_digit_count(false, -2_256_000_000.0, 144_000_000.0),
            freq_display::DIGITS
        );
    }

    #[test]
    fn a_qo100_converter_offset_earns_the_eleventh_column_before_the_dial_moves() {
        // A QO-100 LNB down-converts by 9.75 GHz — short of 10 GHz, which is
        // why the cut is 9e9 — while the dial is still parked on HF and the
        // driver may publish no ranges at all. The column has to be there
        // already, or it appears mid-digit the moment the operator tunes up.
        assert_eq!(
            readout_digit_count(false, -9_750_000_000.0, 14_074_000.0),
            freq_display::DIGITS_EXT
        );
        // A 10 GHz-LO LNB (some 3 cm setups) too.
        assert_eq!(
            readout_digit_count(false, -10_000_000_000.0, 14_074_000.0),
            freq_display::DIGITS_EXT
        );
    }

    #[test]
    fn a_dial_at_the_beacon_earns_the_column_on_its_own() {
        // A wideband front end tuned straight to 10489.750 MHz, no converter —
        // the exact reading the extra column exists to keep from truncating.
        assert_eq!(readout_digit_count(false, 0.0, 10_489_750_000.0), freq_display::DIGITS_EXT);
    }

    #[test]
    fn a_published_range_reaching_10_ghz_is_enough_by_itself() {
        assert_eq!(readout_digit_count(true, 0.0, 14_074_000.0), freq_display::DIGITS_EXT);
    }
}
