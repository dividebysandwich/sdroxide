//! Widgets shared by more than one panel.
//!
//! The decode list and the JS8 heard list draw the same station card and the
//! same fixed-width row cells, every slotted mode draws the same slot clock,
//! and every panel that transmits an image opens the same file picker, so those
//! live here rather than in whichever panel happened to grow them first.

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Decode, SlotTiming};

/// The hover card behind a decode row: everything the entity file, the log and
/// the operator's own grid already know about this station, said in full.
///
/// The row can only afford a callsign, a grid and two numbers; all of this is
/// resolved for it anyway, so the card costs nothing but the space to show it.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn station_card(
    ui: &mut egui::Ui,
    flags: &mut crate::flags::Flags,
    d: &Decode,
    entity: Option<sdroxide_types::EntityInfo>,
    dist_km: Option<f64>,
    my_grid: &str,
    novelty: sdroxide_types::Novelty,
    band: &str,
    queued: bool,
    cq_for_us: bool,
) {
    ui.set_max_width(300.0);
    let dim = crate::theme::gray(140);
    match d.from.as_deref() {
        Some(call) => {
            ui.label(RichText::new(call).size(16.0).strong().color(crate::theme::TEXT_STRONG()));
        }
        None if d.free_text => {
            ui.label(RichText::new("free text").size(13.0).italics().color(dim));
        }
        // A hashed callsign nobody on this receiver has heard in full yet.
        None => {
            ui.label(RichText::new("hashed callsign, not yet heard in full").size(13.0).color(dim));
        }
    }

    match entity {
        Some(e) => {
            // Flag beside the name rather than instead of it: the card is
            // where there is room to say both, and a flag on its own asks the
            // operator to recognise 265 of them.
            ui.horizontal(|ui| {
                flags.show(ui, e.flag, 13.0);
                ui.label(
                    RichText::new(e.name)
                        .size(13.0)
                        .strong()
                        .color(crate::theme::continent_color(e.continent)),
                );
            });
            ui.label(
                RichText::new(format!(
                    "{} · CQ zone {} · ITU zone {}",
                    e.continent, e.cq_zone, e.itu_zone
                ))
                .size(11.5)
                .color(dim),
            );
        }
        None if d.from.is_some() => {
            ui.label(RichText::new("entity unknown").size(11.5).color(dim));
        }
        None => {}
    }

    // Where they are, from their grid: distance and the beam heading to point.
    if let Some(g) = d.grid.as_deref() {
        let bearing =
            (!my_grid.is_empty()).then(|| sdroxide_types::grid_bearing(my_grid, g)).flatten();
        let mut line = g.to_string();
        if let Some(km) = dist_km {
            line.push_str(&format!(" · {km:.0} km"));
        }
        if let Some(b) = bearing {
            line.push_str(&format!(" · {b:.0}°"));
        }
        ui.label(RichText::new(line).size(12.0).color(crate::theme::YELLOW()));
    }

    ui.separator();
    // Worked before? The one thing that decides whether this decode is worth
    // acting on, spelled out rather than compressed into a four-letter badge.
    let (worked, col) = worked_before_line(novelty, band);
    ui.label(RichText::new(worked).size(12.0).color(col));

    if let Some(target) = d.cq_to.as_deref() {
        ui.label(
            RichText::new(if cq_for_us {
                format!("Calling CQ {target} — that includes you")
            } else {
                format!("Calling CQ {target} — not aimed at you")
            })
            .size(11.5)
            .color(if cq_for_us { crate::theme::GREEN() } else { dim }),
        );
    }
    if queued {
        ui.label(RichText::new("In the call queue").size(11.5).color(crate::theme::GREEN()));
    }
    ui.label(
        RichText::new(format!("{:+} dB · {:.0} Hz · DT {:+.1} s", d.snr_db, d.audio_hz, d.dt))
            .size(11.0)
            .monospace()
            .color(dim),
    );
}

/// One fixed-width column of a station row, shared by the FT8 decode list and
/// the JS8 heard list so the two line up field for field.
///
/// The width is *reserved*, not requested: a plain `allocate_ui` shrinks to its
/// content, so a short callsign would collapse the column and shift everything
/// after it out of alignment down the list.
/// The one line that says what working this station would be worth against the
/// log, and in what colour. Pure, because the wording is the thing that goes
/// wrong: it is the sentence an operator reads before deciding whether to
/// answer.
pub(in crate::app) fn worked_before_line(
    novelty: sdroxide_types::Novelty,
    band: &str,
) -> (String, egui::Color32) {
    let band_label = if band.is_empty() { "this band".to_string() } else { band.to_string() };
    if novelty.new_dxcc {
        ("New entity — never worked, on any band".to_string(), crate::theme::PINK())
    } else if novelty.new_dxcc_band {
        (format!("New entity on {band_label}"), crate::theme::YELLOW())
    } else if novelty.new_grid {
        ("New grid square".to_string(), crate::theme::CYAN())
    } else if novelty.new_call {
        ("Not worked before".to_string(), crate::theme::CYAN_DIM())
    } else if novelty.dupe {
        (format!("Worked before on {band_label}"), crate::theme::gray(130))
    } else if band.is_empty() {
        // No ADIF band for this one, so "not on this band" is a question with no
        // answer — and on 11 m it is a nonsense one, since a CB callsign exists
        // only on the citizens' band and there is no other band it could have
        // been worked on. ADIF has no 11 m enumeration (`adif_band` returns ""
        // for 24.99–28.0 MHz on purpose), which is also why a worked-before CB
        // station could never reach the dupe branch above.
        ("Worked before".to_string(), crate::theme::gray(150))
    } else {
        ("Worked before, but not on this band".to_string(), crate::theme::gray(150))
    }
}

pub(in crate::app) fn row_cell(
    ui: &mut egui::Ui,
    w: f32,
    h: f32,
    align_right: bool,
    lbl: egui::Label,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let layout = if align_right {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout)).add(lbl);
}

// ── Sortable table headings ──

/// The marker on the column a table is ordered by.
fn sort_arrow(desc: bool) -> &'static str {
    if desc { " ▼" } else { " ▲" }
}

/// A painted table heading row whose columns sort the table when clicked.
///
/// The tables that use this paint their rows with a `Painter` at fixed x
/// offsets rather than laying them out as widgets — that is what keeps a column
/// of numbers reading straight down the list — so their headings are painted
/// the same way, and the click targets have to be worked out from the same
/// offsets rather than come free with a layout.
///
/// `cols` is `(x offset, alignment, heading, sort key)`. An `f32::NAN` offset
/// is a column the width has dropped; a `None` key is a heading there is
/// nothing to sort on (an aircraft's track, a station's type), which is drawn
/// but not clickable.
///
/// Clicking the column already sorted on reverses it, which is what every table
/// in every other program does; clicking another sorts on that one, descending,
/// because on every column here the interesting end is the top.
pub(in crate::app) fn sort_head_row<K: Copy + PartialEq>(
    ui: &mut egui::Ui,
    cols: &[(f32, egui::Align2, &str, Option<K>)],
    sort: &mut K,
    desc: &mut bool,
) {
    const H: f32 = 14.0;
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, H), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter_at(rect);
    let font = egui::FontId::monospace(8.5);
    let pointer = resp.hover_pos();
    let mut hit = None;

    for (x, align, text, key) in cols {
        if x.is_nan() {
            continue;
        }
        let on = key.is_some_and(|k| k == *sort);
        let label = if on { format!("{text}{}", sort_arrow(*desc)) } else { (*text).to_string() };
        // Laid out with no colour of its own, so the one worked out below is
        // what `Painter::galley` paints it in — a colour baked in here would
        // make every heading look like the active one.
        let galley = p.layout_no_wrap(label, font.clone(), Color32::PLACEHOLDER);
        let anchor = egui::pos2(rect.left() + x, rect.center().y);
        let text_rect = align.anchor_size(anchor, galley.size());
        // A heading is four or five characters of 8.5 pt type. Widened a little
        // and taken to the full height of the row, it is a target a pointer can
        // actually land on.
        let target = egui::Rect::from_x_y_ranges(
            (text_rect.left() - 4.0)..=(text_rect.right() + 4.0),
            rect.y_range(),
        );
        let hovered = key.is_some() && pointer.is_some_and(|p| target.contains(p));
        if hovered {
            hit = *key;
        }
        let ink = if on {
            crate::theme::CYAN()
        } else if hovered {
            crate::theme::gray(180)
        } else {
            crate::theme::gray(110)
        };
        p.galley(text_rect.min, galley, ink);
    }

    if hit.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if let Some(key) = hit.filter(|_| resp.clicked()) {
        if key == *sort {
            *desc = !*desc;
        } else {
            *sort = key;
            *desc = true;
        }
    }
}

/// [`row_cell`] for a heading that sorts the table when it is clicked.
///
/// The [`sort_head_row`] of the tables that lay their columns out as widgets
/// rather than painting them at fixed offsets. Same rule on the click: the
/// active column reverses, any other becomes the new one, descending.
pub(in crate::app) fn sort_head_cell<K: Copy + PartialEq>(
    ui: &mut egui::Ui,
    w: f32,
    align_right: bool,
    text: &str,
    key: Option<K>,
    sort: &mut K,
    desc: &mut bool,
) {
    const H: f32 = 14.0;
    let on = key.is_some_and(|k| k == *sort);
    let label = if on { format!("{text}{}", sort_arrow(*desc)) } else { text.to_string() };
    let sense = if key.is_some() { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, H), sense);
    let ink = if on {
        crate::theme::CYAN()
    } else if resp.hovered() {
        crate::theme::gray(180)
    } else {
        crate::theme::CYAN_DIM()
    };
    let layout = if align_right {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout))
        .add(egui::Label::new(RichText::new(label).size(9.5).color(ink).monospace()).truncate());
    if key.is_some() && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if let Some(key) = key.filter(|_| resp.clicked()) {
        if key == *sort {
            *desc = !*desc;
        } else {
            *sort = key;
            *desc = true;
        }
    }
}

/// [`row_cell`] for a column that draws something other than a label — the
/// flag image, which is a texture rather than text but has to reserve its
/// width the same way or the columns after it walk about between rows.
pub(in crate::app) fn row_cell_ui(
    ui: &mut egui::Ui,
    w: f32,
    h: f32,
    add: impl FnOnce(&mut egui::Ui),
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    add(&mut ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout)));
}

// ── The slot clock ──

/// What the station is doing with the turn the bar is measuring, which is what
/// its colour says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum SlotState {
    /// Receiving — the ordinary state, and the one the bar is most use in.
    Listening,
    /// Working out what was heard. Only WSPR reports this: its decode takes
    /// long enough to be worth naming.
    Decoding,
    /// Keyed.
    Transmitting,
}

/// How far into the current slot the clock is, in seconds.
///
/// `engine_slot_utc` is the slot the radio says it is working on, and it is the
/// anchor whenever this window's clock agrees that we are inside it. When the
/// two disagree — a remote UI on a machine some way off the radio's clock, or
/// before the first status has arrived — the local boundary is used instead.
/// Pass 0 for the modes whose status carries no slot: a slot *is* a fixed block
/// of UTC counted from the epoch, so a clock that is wrong then draws a bar
/// quietly wrong by the same amount, rather than one that pins at full for a
/// while and jumps.
pub(in crate::app) fn slot_phase_s(now: f64, slot_s: f64, engine_slot_utc: i64) -> f64 {
    let into = now - engine_slot_utc as f64;
    if engine_slot_utc > 0 && (0.0..slot_s).contains(&into) { into } else { now.rem_euclid(slot_s) }
}

/// The phase of a slot as it reaches us over the receive path.
///
/// `offset_s` is the DT the stations we hear show against our own slot
/// boundary, which already includes the audio and network chain. A frame is on
/// the air from the boundary, but it is in our audio `offset_s` later, so the
/// bar that follows what is being received is that far behind the wall clock.
/// `None` (nothing measured yet) is the wall clock, as before.
pub(in crate::app) fn slot_phase_heard_s(now: f64, slot_s: f64, offset_s: Option<f32>) -> f64 {
    slot_phase_s(now - offset_s.unwrap_or(0.0) as f64, slot_s, 0)
}

/// Where the closing stretch of an FT8 cycle begins, as a fraction of the slot.
///
/// The bar turns green from the end of the 12.64 s burst to the end of the 15 s
/// slot. It marks the end of the cycle and says nothing about how long a decode
/// takes. Only FT8's slot has it: the other slotted modes keep one colour.
pub(in crate::app) fn slot_green_start_frac(t: SlotTiming) -> Option<f32> {
    (t.slot_s == 15.0 && t.burst_s == 12.64).then(|| (t.burst_s / t.slot_s) as f32)
}

/// The slot clock: how far through its turn this station is, whichever way
/// round it is spending it.
///
/// Every slotted mode runs on the same two facts — a turn of a fixed length,
/// and a burst that fills part of it — so they all get the same bar. A station
/// spends most of its slots listening, and what an operator wants to know while
/// it does is when this one ends: that is when the recording goes to the
/// decoder and when the next transmit decision lands. A bar that only appeared
/// under a carrier would be missing in exactly the state it is most use in, so
/// it is always drawn and the colour says what is happening: pink on the air,
/// yellow working out what was heard, dim cyan listening.
///
/// Returns the bar's response, so a caller can hang hover text on it.
pub(in crate::app) fn slot_bar(
    ui: &mut egui::Ui,
    t: SlotTiming,
    into_slot: f64,
    state: SlotState,
) -> egui::Response {
    // The clock runs on the wall clock rather than on anything arriving, so the
    // bar has to ask for its own frames. About a four-hundredth of the bar per
    // frame is enough to read as moving without asking a phone to redraw the
    // panel sixty times a second for a strip five pixels tall.
    let step_ms = (t.slot_s * 1000.0 / 400.0).clamp(80.0, 250.0);
    crate::repaint::after_ms(ui.ctx(), step_ms as u64);

    let (bar, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 5.0), egui::Sense::hover());
    let p = ui.painter_at(bar);
    p.rect_filled(bar, 0.0, crate::theme::gray(24));

    let frac = (into_slot / t.slot_s).clamp(0.0, 1.0) as f32;
    let tail_from = slot_green_start_frac(t);
    // The part already drawn keeps the state's colour; the bar goes on from
    // there in green once the cycle enters its closing stretch.
    let head = tail_from.map_or(frac, |g| frac.min(g));
    let mut fill = bar;
    fill.set_width(bar.width() * head);
    p.rect_filled(
        fill,
        0.0,
        match state {
            SlotState::Transmitting => crate::theme::ALERT(),
            SlotState::Decoding => crate::theme::YELLOW(),
            SlotState::Listening => crate::theme::CYAN_DIM(),
        },
    );
    if let Some(g) = tail_from.filter(|g| frac > *g) {
        let x0 = bar.left() + bar.width() * g;
        let x1 = bar.left() + bar.width() * frac;
        p.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x0, bar.top()), egui::pos2(x1, bar.bottom())),
            0.0,
            crate::theme::GREEN(),
        );
    }

    // While keyed, a mark where the carrier stops. FT8 stops 13.1 s into its
    // 15, WSPR 111.6 s into its 120; without the mark the rest of the bar reads
    // as a transmission that ended early rather than as the turnaround it is.
    // Painted over the fill, and only while transmitting: in a receive slot the
    // recording runs the whole turn and the mark would mean nothing.
    if state == SlotState::Transmitting {
        let x = bar.left() + bar.width() * t.burst_end_frac() as f32;
        p.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, bar.top()), egui::vec2(1.0, bar.height())),
            0.0,
            crate::theme::gray(150),
        );
    }
    resp
}

/// Colour a decode's SNR: green for strong, cyan mid, dimmed for weak.
pub(in crate::app) fn snr_color(snr_db: i16) -> Color32 {
    if snr_db >= 0 {
        crate::theme::GREEN()
    } else if snr_db >= -12 {
        crate::theme::CYAN()
    } else {
        crate::theme::CYAN_DIM()
    }
}

// ── File picker (native thread / wasm async) ──

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn pick_image(inbox: Arc<Mutex<Option<Vec<u8>>>>) {
    std::thread::Builder::new()
        .name("sdroxide-pick".into())
        .spawn(move || {
            if let Some(path) =
                rfd::FileDialog::new().add_filter("Image", &["png", "jpg", "jpeg"]).pick_file()
            {
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Ok(mut g) = inbox.lock() {
                        *g = Some(bytes);
                    }
                }
            }
        })
        .ok();
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn pick_image(inbox: Arc<Mutex<Option<Vec<u8>>>>) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(file) = rfd::AsyncFileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg"])
            .pick_file()
            .await
        {
            let bytes = file.read().await;
            if let Ok(mut g) = inbox.lock() {
                *g = Some(bytes);
            }
        }
    });
}

/// A titled, bordered section box of a fixed size, for the SSTV panel's LIVE /
/// RECEIVED / TRANSMIT areas.
pub(in crate::app) fn sstv_section<R>(
    ui: &mut egui::Ui,
    title: &str,
    size: egui::Vec2,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    // Force a top-down layout: `allocate_ui` would otherwise inherit the parent's
    // horizontal layout (we're inside a `horizontal_top`), laying the section's
    // contents out side by side instead of stacked.
    ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
        egui::Frame::new()
            .fill(crate::theme::ROW_BG())
            .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
            .inner_margin(egui::Margin { left: 8, right: 8, top: 5, bottom: 7 })
            .show(ui, |ui| {
                ui.set_min_size(egui::vec2(size.x - 18.0, size.y - 14.0));
                ui.set_max_width(size.x - 18.0);
                ui.label(RichText::new(title).size(9.5).strong().color(crate::theme::CYAN_DIM()));
                ui.add_space(3.0);
                add(ui)
            })
            .inner
    })
    .inner
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_heard_phase_follows_the_receive_path_not_the_wall_clock() {
        // Audio arrives 2 s behind our boundary: at wall 7 s into a 15 s
        // slot the frame in the audio is 5 s in, not 7.
        assert!((slot_phase_heard_s(112.0, 15.0, Some(2.0)) - 5.0).abs() < 1e-9);
        assert!((slot_phase_heard_s(112.0, 15.0, None) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn the_heard_phase_follows_the_receive_path_on_ft4_and_ft2_slots() {
        // FT4 (7.5 s): wall 7 s in, audio 2 s behind -> 5 s.
        assert!((slot_phase_heard_s(112.0, 7.5, Some(2.0)) - 5.0).abs() < 1e-9);
        assert!((slot_phase_heard_s(112.0, 7.5, None) - 7.0).abs() < 1e-9);
        // The offset reaching back past the boundary wraps into the previous slot.
        assert!((slot_phase_heard_s(113.0, 7.5, Some(2.0)) - 6.0).abs() < 1e-9);
        // FT2 (3.75 s).
        assert!((slot_phase_heard_s(112.0, 3.75, Some(2.0)) - 1.25).abs() < 1e-9);
        assert!((slot_phase_heard_s(112.0, 3.75, None) - 3.25).abs() < 1e-9);
        assert!((slot_phase_heard_s(113.0, 3.75, Some(2.0)) - 2.25).abs() < 1e-9);
    }

    use super::*;
    use sdroxide_types::Mode;

    /// A real WSPR slot boundary: 1_785_760_440 is divisible by 120, and so by
    /// every shorter slot length here.
    const SLOT: i64 = 1_785_760_440;

    fn wspr() -> SlotTiming {
        Mode::Wspr.slot_timing().expect("WSPR is slotted")
    }

    #[test]
    fn the_bar_measures_from_the_slot_the_engine_says_it_is_working_on() {
        let s = wspr().slot_s;
        assert_eq!(slot_phase_s(SLOT as f64, s, SLOT), 0.0);
        assert_eq!(slot_phase_s(SLOT as f64 + 43.5, s, SLOT), 43.5);
        // Right up to the boundary, and not past it.
        assert!(slot_phase_s(SLOT as f64 + 119.9, s, SLOT) > 119.0);
    }

    /// A window whose clock is nowhere near the radio's must not pin the bar at
    /// full and then jump: it falls back to its own boundary, which is what a
    /// slot is defined by, and is then wrong by exactly the clock error.
    #[test]
    fn a_clock_that_disagrees_with_the_engine_still_gets_a_moving_bar() {
        let s = wspr().slot_s;
        // Six minutes — three slots — ahead of the one the engine reported.
        let now = SLOT as f64 + 360.0 + 20.0;
        assert_eq!(slot_phase_s(now, s, SLOT), 20.0);
        // And behind it, where the difference would otherwise be negative.
        assert_eq!(slot_phase_s(SLOT as f64 - 100.0, s, SLOT), 20.0);
    }

    /// The modes whose status carries no slot pass 0, and the bar still has to
    /// move — that is every mode but WSPR.
    #[test]
    fn with_no_engine_slot_the_bar_runs_off_the_local_boundary() {
        assert_eq!(slot_phase_s(SLOT as f64 + 61.0, wspr().slot_s, 0), 61.0);
        // FT8: 61 s past a two-minute boundary is one second into a slot.
        let ft8 = Mode::Ft8.slot_timing().unwrap();
        assert_eq!(slot_phase_s(SLOT as f64 + 61.0, ft8.slot_s, 0), 1.0);
        // FT2's slots are four times shorter again.
        let ft2 = Mode::Ft2.slot_timing().unwrap();
        assert_eq!(slot_phase_s(SLOT as f64 + 5.0, ft2.slot_s, 0), 1.25);
    }

    /// Whatever the two clocks are doing, in whatever mode, the fraction the
    /// bar draws is one.
    #[test]
    fn the_phase_is_always_inside_the_slot() {
        for mode in Mode::ALL {
            let Some(t) = mode.slot_timing() else { continue };
            for k in -10..600 {
                let now = SLOT as f64 + k as f64 * 7.3;
                let p = slot_phase_s(now, t.slot_s, SLOT);
                assert!((0.0..t.slot_s).contains(&p), "{mode:?}: {p} is not a position in a slot");
            }
        }
        // JS8, whose slot length comes from the speed rather than the mode.
        for speed in sdroxide_types::Js8Speed::ALL {
            let t = speed.slot_timing();
            let p = slot_phase_s(SLOT as f64 + 3.7, t.slot_s, 0);
            assert!((0.0..t.slot_s).contains(&p), "{}: {p}", speed.label());
        }
    }

    fn bar_fills(into_s: f64, t: SlotTiming) -> Vec<(egui::Rect, Color32)> {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            slot_bar(ui, t, into_s, SlotState::Listening);
        });
        let mut fills: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Rect(r) if r.rect.height() == 5.0 => Some((r.rect, r.fill)),
                _ => None,
            })
            .collect();
        fills.remove(0); // the track
        fills.retain(|f| f.0.width() > 0.0);
        fills
    }

    #[test]
    fn only_the_ft8_slot_has_a_green_tail() {
        let ft8 = Mode::Ft8.slot_timing().unwrap();
        assert!((slot_green_start_frac(ft8).unwrap() - 12.64 / 15.0).abs() < 1e-6);
        assert_eq!(slot_green_start_frac(Mode::Ft4.slot_timing().unwrap()), None);
        assert_eq!(slot_green_start_frac(Mode::Ft2.slot_timing().unwrap()), None);
        assert_eq!(slot_green_start_frac(wspr()), None);
    }

    #[test]
    fn the_ft8_bar_continues_in_green_after_12_64_s() {
        let t = Mode::Ft8.slot_timing().unwrap();
        let before = bar_fills(12.0, t);
        assert_eq!(before.len(), 1, "no green before 12.64 s");
        assert_eq!(before[0].1, crate::theme::CYAN_DIM());

        let after = bar_fills(14.0, t);
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].1, crate::theme::CYAN_DIM(), "drawn part keeps its colour");
        assert_eq!(after[1].1, crate::theme::GREEN());
        let (a, b) = (after[0].0, after[1].0);
        assert!((a.width() / (a.width() + b.width()) - 12.64 / 14.0).abs() < 1e-3);
        assert!((a.right() - b.left()).abs() < 1e-3, "no gap between the two");

        let end = bar_fills(15.0, t);
        assert_eq!(end.len(), 2);
        assert!(
            (end[1].0.right() - end[0].0.left() - 1.0 * (end[0].0.width() + end[1].0.width()))
                .abs()
                < 1e-3
        );
        assert!(bar_fills(0.0, t).is_empty());
    }

    #[test]
    fn other_slots_stay_one_colour() {
        let t = Mode::Ft4.slot_timing().unwrap();
        assert_eq!(bar_fills(t.slot_s * 0.95, t).len(), 1);
    }
}

#[cfg(test)]
mod worked_before_tests {
    use super::worked_before_line;
    use sdroxide_types::Novelty;

    /// A station worked before, on a band ADIF does not enumerate, must not be
    /// told it was worked "but not on this band".
    ///
    /// The operator's report, on 11 m: *"it says 'worked before but not on this
    /// band' which is wrong. CB callsigns are only on 11M so what other band
    /// should that station have been worked on?"* — right, and unreachable to
    /// fix any other way, because `adif_band` deliberately returns `""` there
    /// (ADIF's list runs 12m, 10m, 8m with nothing in between), so `novelty()`
    /// sets `per_band` false and `dupe` can never be true on the citizens' band.
    /// Every worked-before CB station therefore fell into this last branch.
    #[test]
    fn on_a_band_with_no_adif_name_it_says_worked_before_and_nothing_more() {
        let n = Novelty::default(); // worked before, not a dupe, nothing new
        let (line, _) = worked_before_line(n, "");
        assert_eq!(line, "Worked before", "11 m has no ADIF band to differ from");
        assert!(!line.contains("band"), "the band clause is the whole bug: {line:?}");
    }

    /// ...and it must not swallow the cases that *do* have a band.
    #[test]
    fn a_real_band_still_says_which_band() {
        let n = Novelty::default();
        let (line, _) = worked_before_line(n, "20m");
        assert_eq!(line, "Worked before, but not on this band");
        let dupe = Novelty { dupe: true, ..Novelty::default() };
        let (line, _) = worked_before_line(dupe, "20m");
        assert_eq!(line, "Worked before on 20m");
    }

    /// A station never heard before is unaffected — it is a new call, and that
    /// outranks every wording question here.
    #[test]
    fn a_new_call_still_says_so() {
        let n = Novelty { new_call: true, ..Novelty::default() };
        assert_eq!(worked_before_line(n, "").0, "Not worked before");
        assert_eq!(worked_before_line(n, "20m").0, "Not worked before");
    }
}
