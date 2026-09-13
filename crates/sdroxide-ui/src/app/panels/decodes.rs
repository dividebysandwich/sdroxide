//! The FT8/FT4/JS8 decode list, and the QSO sequencer area beside it.
//!
//! Rows are grouped into transmit slots and, within a slot, ordered by the
//! operator's choice of signal or distance. Each row resolves up front what it
//! needs to colour itself — distance, whether it is a CQ we may answer, what
//! working the station would be worth against the log — because the list is
//! rebuilt from scratch every frame.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Command, Decode, DecodeSort, Mode};

use crate::theme::ThemedScroll;
use crate::time::now_unix;

use crate::app::panels::widgets::{row_cell, row_cell_ui, snr_color, station_card};
use crate::app::{SdroxideApp, rx_only_hint, tx_gated};

/// One decode as the list draws it: the entry itself, plus everything the row
/// needs resolved once up front — how far away they are, whether this is a CQ
/// *we* may answer, what working them would be worth against the log, and
/// whether the message names our own station.
#[derive(Clone, Copy)]
struct DecodeRow<'a> {
    /// Index into `digi_decodes`, so rows keep their identity after sorting.
    idx: usize,
    d: &'a Decode,
    /// DXCC entity resolved from the callsign — the country name, its flag and
    /// its continent. Resolved once here because the row needs it to draw and
    /// the list needs it to sort.
    entity: Option<sdroxide_types::EntityInfo>,
    dist_km: Option<f64>,
    cq: bool,
    novelty: sdroxide_types::Novelty,
    to_me: bool,
}

/// Whether a spot's announced mode is the one this panel works.
///
/// Feeds are inconsistent about case and about the FT4 spelling — the clusters
/// carry both `FT4` and the older `FT8-4` — so this compares loosely rather than
/// going through [`sdroxide_types::Mode`], which would also fold every other
/// data mode into `USB`.
fn is_ft8_or_ft4(mode: &str) -> bool {
    let m = mode.trim();
    m.eq_ignore_ascii_case("FT8")
        || m.eq_ignore_ascii_case("FT4")
        || m.eq_ignore_ascii_case("FT8-4")
        || m.eq_ignore_ascii_case("FT2")
}

impl SdroxideApp {
    /// Touch-friendly decode list with a per-row REPLY button. Clicking a
    /// row moves the TX audio frequency to that signal; REPLY starts a QSO.
    pub(in crate::app) fn decode_list(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let phone = crate::layout::tier(ui.ctx()) == crate::layout::Tier::Phone;
        // The tab row above already says which view this is and how many
        // stations came in; a second header would be a row of a phone's screen
        // spent repeating it.
        if !phone {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("DECODES").size(9.5).strong().color(crate::theme::CYAN_DIM()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} rx", self.digi_decodes.len()))
                            .size(10.0)
                            .color(crate::theme::gray(120)),
                    );
                });
            });
        }
        // Two control rows, each one chip tall, so everything on a row shares
        // a baseline. This used to be a single wrapped row with the TX nudges
        // stacked two-high in the middle of it: every chip then centred against
        // a double-height neighbour, and on most pane widths the Hz box broke
        // onto a ragged second line by itself. Split by subject instead — what
        // the list shows on one row, what transmit does on the other — each
        // row keeps a uniform height, and wrapping happens between rows' items
        // rather than through the middle of a stacked group.
        //
        // Ordering, grouping and filters for the decode list. All five are
        // remembered in `[ui]` with the rest of this screen's preferences —
        // they say how the operator reads a band, and that does not change
        // because the program was restarted. Edited through locals and written
        // back once below, so a frame that touches nothing writes nothing.
        let was = (
            self.ui_settings.decode_sort,
            self.ui_settings.decode_sort_desc,
            self.ui_settings.decode_single_list,
            self.ui_settings.decode_cq_only,
            self.ui_settings.decode_new_only,
        );
        let (mut sort_by, mut sort_desc, mut single, mut cq_only, mut new_only) = was;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("Sort").size(9.5).color(crate::theme::CYAN_DIM()));
            for m in DecodeSort::ALL {
                let active = sort_by == m;
                // Active mode shows its direction; re-pressing it flips direction.
                let lbl = if active && m != DecodeSort::None {
                    format!("{} {}", m.label(), if sort_desc { "↓" } else { "↑" })
                } else {
                    m.label().to_string()
                };
                if crate::chrome::chip(ui, active, lbl).clicked() {
                    if active && m != DecodeSort::None {
                        sort_desc = !sort_desc;
                    } else {
                        sort_by = m;
                        // Strongest and farthest first, but countries A to Z:
                        // "descending" is the useful end of a number and the
                        // wrong end of an alphabet.
                        sort_desc = m != DecodeSort::Country;
                    }
                }
            }
            ui.add_space(8.0);
            // Grouping: odd/even turn blocks, or everything in one list.
            // Beside the Sort chips because it decides what they sort — each
            // turn on its own, or the whole list at once.
            if crate::chrome::chip(ui, single, "Single list")
                .on_hover_text(
                    "Every decode in one list, newest turn first, instead of odd/even turn \
                     blocks. The Sort chips then order the whole list rather than each turn, \
                     and each row carries its slot time — cyan for an even slot, gold for an \
                     odd one.",
                )
                .clicked()
            {
                single = !single;
            }
            ui.add_space(8.0);
            if crate::chrome::chip(ui, cq_only, "CQ only")
                .on_hover_text(
                    "Only stations calling CQ — and only the calls you may answer: a directed \
                     CQ (DX, EU, JA, POTA, TEST …) is listed when it names you and hidden when \
                     it names someone else.",
                )
                .clicked()
            {
                cq_only = !cq_only;
            }
            if crate::chrome::chip(ui, new_only, "New only")
                .on_hover_text("Only stations that would be new: entity, band-slot, grid, or call")
                .clicked()
            {
                new_only = !new_only;
            }
        });
        if (sort_by, sort_desc, single, cq_only, new_only) != was {
            self.ui_settings.decode_sort = sort_by;
            self.ui_settings.decode_sort_desc = sort_desc;
            self.ui_settings.decode_single_list = single;
            self.ui_settings.decode_cq_only = cq_only;
            self.ui_settings.decode_new_only = new_only;
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
        // The transmit row: who picks our frequency, the frequency itself, and
        // the list-clearing button at the end.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            // Whether the engine chooses our transmit frequency. Here rather
            // than in the setup window because it decides what clicking a
            // decode in this list does.
            // SWL mode: the whole TX frequency row is meaningless without a
            // transmitter, so hide it entirely.
            if self.digi_cfg_seeded && !self.ui_settings.swl {
                let held = self.digi_cfg_edit.hold_tx_freq;
                let auto = self.digi_cfg_edit.auto_tx_freq;
                // Greyed while held, because held wins: leaving it live would
                // offer a choice between two movers neither of which can run.
                // `chip_enabled`, not a bare `add_enabled_ui`: this row is
                // `horizontal_wrapped`, and a child Ui inside one does not wrap.
                let auto_chip =
                    crate::chrome::chip_enabled(ui, !held, auto && !held, "Auto TX FRQ")
                        .on_hover_text(if held {
                            "Overridden by Hold TX. Lift the hold to choose which way the transmit \
                         frequency moves."
                        } else {
                            "Pick our transmit frequency automatically: the quietest spot in the \
                         period we transmit in, rather than the frequency of whoever we are \
                         answering — they transmit in the other period, so theirs says nothing \
                         about who is there when we key. Off does NOT hold the frequency: it \
                         answers on the frequency of the station being called. To hold, use \
                         Hold TX."
                        });
                if auto_chip.clicked() {
                    self.digi_cfg_edit.auto_tx_freq = !auto;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
                // The third state neither setting above can express: don't move
                // at all. For the licence edges, where the band plan is wider
                // than what we are allowed to key into.
                if crate::chrome::chip(ui, held, "Hold TX")
                    .on_hover_text(
                        "Pin the transmit tone where it is. Nothing moves it: not answering a \
                         station, not the call queue, not calling CQ, not a click on a decode \
                         or on the waterfall. Turn it off to move, then on again.\n\nChanging \
                         band is the one exception, and it is your own act: the offset you last \
                         set on the new band comes back with it, so a 60 m figure is not carried \
                         onto 20 m and 20 m's is not carried onto 60 m.\n\nFor where \
                         the licence is narrower than the band plan — on a UK 60 m dial of \
                         5357 kHz the allocation ends at 5358.0, so the tone has to stay under \
                         1000 Hz, and either automatic mover will walk out of the band between \
                         one over and the next.",
                    )
                    .clicked()
                {
                    self.digi_cfg_edit.hold_tx_freq = !held;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
                // Our own transmit offset: readout, ±10 Hz, and a box to type
                // it into. FT8 and FT4 were the only slotted modes without one
                // — JS8, FSQ, CW and the text modem all have the nudge pair —
                // so the offset could be placed only by clicking the waterfall,
                // which is not a way to land on an exact figure. A hold is only
                // as good as the ability to set what is being held.
                //
                // No suggested value anywhere here, deliberately. Where the
                // ceiling is the licence's rather than the band plan's, the
                // whole range below it is the operator's to choose from, and a
                // number printed in a tooltip is a number everyone sits on.
                let our_hz = self.digi_status.as_ref().map(|s| s.audio_hz).unwrap_or(1500.0);
                // The nudges land on round figures rather than stepping a flat
                // 10 Hz, because a flat step preserves the last digit of
                // wherever the tone happens to be sitting. A tone left at 373 by
                // an automatic move or a waterfall click can then reach 363 and
                // 383 and never 370 at all, however many times it is pressed.
                // Reported from the station the day the box landed, having been
                // tried: "it would not give me exact 370 so its 373".
                //
                // Nothing is lost by it. The step is unchanged where the tone is
                // already round, and an exact figure of any kind still comes
                // from the box beneath.
                let step_down = ((our_hz / 10.0).ceil() - 1.0) * 10.0;
                let step_up = ((our_hz / 10.0).floor() + 1.0) * 10.0;
                // Label, nudges, figure and unit are one horizontal group —
                // the typed box between its own − and +, stepper-fashion —
                // rather than five items loose in the wrapping row. Loose,
                // they sat side by side on a wide panel and broke apart on a
                // narrow one, so where the box appeared depended on the panel
                // width and it could be separated from the chips that move the
                // same number. A child `Ui` inside a `horizontal_wrapped` row
                // does not wrap, which is exactly what the group wants: it is
                // placed — and wraps — as one item, a single chip tall, so the
                // row it lands on keeps a uniform height.
                //
                // Each control inside is still gated on its own rather than
                // the group being wrapped in an `add_enabled_ui`: greying out
                // is what `chip_enabled` and `field_enabled` are for.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(RichText::new("TX").size(11.0).color(crate::theme::gray(140)));
                    if crate::chrome::chip_enabled(ui, !held, false, "−").clicked() {
                        cmds.push(Command::SetDigiAudioFreq(step_down.clamp(200.0, 3500.0)));
                    }
                    // The readout and the entry are the same widget: two of them
                    // would be two things to disagree with each other.
                    //
                    // A `TextEdit` rather than the `DragValue` this started as.
                    // Speed 0 stopped a drag from changing anything, but it did not
                    // stop egui *advertising* one: the pointer still turned into the
                    // horizontal resize arrows on hover, so the box offered a
                    // gesture that had been deliberately killed. A text box promises
                    // only what it delivers, and its caret says "type here" without
                    // a tooltip having to say it.
                    //
                    // The buffer is refreshed from the engine only while the box is
                    // NOT focused, and that is what makes typing survive at all: the
                    // offset arrives afresh in every status frame, so reseeding on
                    // top of a half-typed figure would rewrite it under the
                    // operator's hands.
                    //
                    // Committed on LOST FOCUS (Enter, or clicking away) rather than
                    // on every change, because the text is reparsed as it is typed:
                    // committing on change would send "8", then "82", then "820" —
                    // three transmit-frequency changes for one figure, the first two
                    // of them wrong, and on 60 m the first two are the ones inside
                    // the band. Clamped once, when the number is finished, rather
                    // than continuously, which would fight the operator and turn
                    // "820" into "200" the moment the 8 was pressed.
                    //
                    // Anything unparsable is simply not sent, and the box puts
                    // itself right on the next unfocused frame from the engine's own
                    // value. There is no error state to explain or dismiss.
                    let tx_hz_id = ui.id().with("digi_tx_hz");
                    if !ui.memory(|m| m.has_focus(tx_hz_id)) {
                        let shown = format!("{our_hz:.0}");
                        if self.digi_tx_hz_edit != shown {
                            self.digi_tx_hz_edit = shown;
                        }
                    }
                    let resp = crate::chrome::field_enabled(
                        ui,
                        !held,
                        egui::TextEdit::singleline(&mut self.digi_tx_hz_edit)
                            .id(tx_hz_id)
                            .desired_width(40.0),
                    )
                    .on_hover_text(
                        "Type the transmit offset in Hz, then press Enter. Where \
                         your licence is narrower than the band plan the whole \
                         range below the edge is yours to pick from, so nothing \
                         is suggested here.",
                    );
                    if crate::chrome::chip_enabled(ui, !held, false, "+").clicked() {
                        cmds.push(Command::SetDigiAudioFreq(step_up.clamp(200.0, 3500.0)));
                    }
                    // The unit rides beside the box rather than inside it. As a
                    // suffix it would be text sitting in the buffer the
                    // operator is typing into, to be worked around on every
                    // edit and parsed back off again.
                    ui.label(RichText::new("Hz").size(11.0).color(crate::theme::gray(140)));
                    if resp.lost_focus() {
                        if let Ok(v) = self.digi_tx_hz_edit.trim().parse::<f32>() {
                            let v = v.clamp(200.0, 3500.0);
                            if (v - our_hz).abs() >= 0.5 {
                                cmds.push(Command::SetDigiAudioFreq(v));
                            }
                        }
                    }
                });
            }
            ui.add_space(8.0);
            // The same control the keyboard-mode panels carry, but it cannot be
            // `clear_rx_chip`: that pushes `DigiClearRx`, which reaches the
            // controller — and the FT8/FT4 decodes are held here in the client,
            // one copy per attached screen, so the engine has nothing to empty.
            if crate::chrome::chip(ui, false, RichText::new(" CLEAR RX ").size(10.5))
                .on_hover_text("Empty the decode list. Nothing that is on the air stops.")
                .clicked()
            {
                self.clear_digi_band_rx();
            }
        });
        ui.add_space(2.0);
        // Call of the currently previewed decode (cloned so the scroll closure
        // doesn't hold a borrow of `self` we need to write back afterwards).
        let preview_call = self.digi_preview.as_ref().map(|(c, _)| c.clone());
        // Own grid, for the per-decode great-circle distance column.
        let my_grid =
            self.digi_status.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default();
        // Own callsign, to spotlight decodes addressed to us.
        let my_call =
            self.digi_status.as_ref().map(|s| s.config.my_call.clone()).unwrap_or_default();
        // Stations already marked to work, so their queue button reads as set.
        let queued_calls: Vec<String> = self
            .digi_status
            .as_ref()
            .map(|s| s.call_queue.iter().map(|q| q.call.clone()).collect())
            .unwrap_or_default();
        // Staged preview change: `None` = no click this frame; `Some(v)` =
        // replace the preview with `v` (`Some(None)` clears it).
        let mut new_preview: Option<Option<(String, (f64, f64))>> = None;
        // Location of the row hovered this frame → yellow dot on the map.
        let mut hover_ll: Option<(f64, f64)> = None;
        let cq_only = self.ui_settings.decode_cq_only;
        let new_only = self.ui_settings.decode_new_only;
        // Read once for the whole list: every row's REPLY and queue chip asks
        // the same question of the same radio.
        let tx_ok = self.tx_capable();
        let auto_tx_freq = self.digi_status.as_ref().map(|s| s.config.auto_tx_freq).unwrap_or(true);
        let hold_tx_freq =
            self.digi_status.as_ref().map(|s| s.config.hold_tx_freq).unwrap_or(false);
        let sort = self.ui_settings.decode_sort;
        let desc = self.ui_settings.decode_sort_desc;
        let single = self.ui_settings.decode_single_list;
        // Turn parity needs the mode's slot length (FT8 15 s, FT4 7.5 s). JS8's
        // is an operator setting rather than implied by the mode, so it comes
        // from the status — otherwise Turbo draws one turn header per 2.5 turns.
        let period = self.slot_period_s();
        // The band we'd log a contact on, for the "is this one new?" judgement.
        let dial_hz = self.state.active_freq_hz();
        let band = if dial_hz > 0.0 { sdroxide_types::adif_band(dial_hz) } else { "" };
        // Refresh the log index (needs &mut self), then borrow it beside the
        // decodes for the per-row novelty lookups.
        self.log_index();
        let log_ix = &self.log_index_cache.as_ref().expect("just refreshed").1;
        // Filter (CQ-only / new-only) and precompute distance for sorting and
        // display. Entries stay newest-turn-first; same-slot decodes are
        // contiguous in the list. A "CQ DX" from a station we're local to is not
        // a CQ *we* can answer: it neither passes the filter nor gets the CQ
        // highlight.
        let mut items: Vec<DecodeRow> = self
            .digi_decodes
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                // A message addressed to our own station survives every filter.
                // It is the one decode in the list we owe an answer to, and
                // hiding it behind "CQ only" — a station calling us back is not
                // calling CQ — leaves no REPLY button anywhere to answer from.
                let to_me = !my_call.is_empty() && d.to.as_deref() == Some(my_call.as_str());
                let cq = sdroxide_types::cq_is_for_us(d, &my_call, &my_grid);
                if cq_only && !cq && !to_me {
                    return None;
                }
                let novelty =
                    log_ix.novelty(d.from.as_deref().unwrap_or(""), d.grid.as_deref(), band);
                if new_only && !novelty.is_new() && !to_me {
                    return None;
                }
                let dist_km = (!my_grid.is_empty())
                    .then(|| {
                        d.grid
                            .as_deref()
                            .and_then(|g| sdroxide_types::grid_distance_km(&my_grid, g))
                    })
                    .flatten();
                let entity = d.from.as_deref().and_then(sdroxide_types::resolve_callsign);
                Some(DecodeRow { idx: i, d, entity, dist_km, cq, novelty, to_me })
            })
            .collect();
        // Whether a row still has the width for the one-line layout. Its fixed
        // columns, margins and buttons come to ~510 points before the message
        // gets one — right of a phone's whole screen, which is where the REPLY
        // button used to end up. Under this the row takes two lines instead.
        // Width, not tier: a phone at a larger OS scale factor and a desktop
        // pane dragged narrow are the same problem.
        let narrow = ui.available_width() < 600.0;
        // The flag fits in any layout — it is twenty points wide and needs no
        // reading. Spelling the country out as well takes a column the width of
        // two callsigns, so that one waits until the pane is wide enough to
        // give it without squeezing the message.
        let name_country = ui.available_width() >= 820.0;
        // Borrowed out of `self` up front: the closure below already holds the
        // decodes and the log index, and taking the whole app into it would
        // collide with them.
        let flags = &mut self.flags;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_themed(ui, |ui| {
            let mut gi = 0;
            while gi < items.len() {
                // A turn is one slot: group the contiguous same-slot decodes.
                // The single-list view instead takes the whole list as one
                // group, so the same sort below runs across every turn at once.
                let end = if single {
                    items.len()
                } else {
                    let slot = items[gi].d.slot_utc;
                    let mut end = gi;
                    while end < items.len() && items[end].d.slot_utc == slot {
                        end += 1;
                    }
                    end
                };
                match sort {
                    DecodeSort::None => {}
                    DecodeSort::Signal => items[gi..end].sort_by(|a, b| {
                        let o = a.d.snr_db.cmp(&b.d.snr_db);
                        if desc { o.reverse() } else { o }
                    }),
                    DecodeSort::Distance => items[gi..end].sort_by(|a, b| {
                        // Decodes without a grid always sort last (push them to the
                        // far end of whichever direction is active).
                        let sentinel = if desc { f64::NEG_INFINITY } else { f64::INFINITY };
                        let ka = a.dist_km.unwrap_or(sentinel);
                        let kb = b.dist_km.unwrap_or(sentinel);
                        let o = ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal);
                        if desc { o.reverse() } else { o }
                    }),
                    DecodeSort::Country => items[gi..end].sort_by(|a, b| {
                        // Stations whose entity we cannot name stay at the
                        // bottom in either direction: they are not a country
                        // that sorts before A or after Z, they are an unknown.
                        let ka = a.entity.map(|e| e.name);
                        let kb = b.entity.map(|e| e.name);
                        match (ka, kb) {
                            (None, None) => std::cmp::Ordering::Equal,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (Some(x), Some(y)) => {
                                let o = x.cmp(y);
                                if desc { o.reverse() } else { o }
                            }
                        }
                    }),
                }
                if single {
                    ui.add_space(3.0);
                } else {
                    // Turn separator: even/odd parity + UTC timestamp.
                    let slot = items[gi].d.slot_utc;
                    let even = ((slot as f64 / period).round() as i64).rem_euclid(2) == 0;
                    let s = slot.rem_euclid(86_400);
                    let tstr = format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60);
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        let (ptxt, pcol) = if even {
                            ("EVEN", crate::theme::CYAN())
                        } else {
                            ("ODD", crate::theme::YELLOW())
                        };
                        ui.label(RichText::new(ptxt).size(9.0).strong().color(pcol));
                        ui.label(
                            RichText::new(format!("{tstr} UTC"))
                                .size(9.5)
                                .monospace()
                                .color(crate::theme::gray(130)),
                        );
                    });
                    ui.separator();
                }
                for k in gi..end {
                    let DecodeRow { idx: i, d, entity, dist_km, cq, novelty, to_me } = items[k];
                    // Free text names no sender, and a hashed callsign nobody
                    // has heard yet resolves to none either — say which it is
                    // rather than showing a bare "?".
                    let who = d
                        .from
                        .clone()
                        .unwrap_or_else(|| if d.free_text { "TEXT".into() } else { "?".into() });
                    // What this station would be worth working: one badge, and
                    // a dupe fades the row back so the new ones carry the eye.
                    let (badge, badge_col) = match novelty.highlight() {
                        Some(sdroxide_types::Highlight::NewDxcc) => ("DXCC", crate::theme::PINK()),
                        Some(sdroxide_types::Highlight::NewDxccBand) => {
                            ("BAND", crate::theme::YELLOW())
                        }
                        Some(sdroxide_types::Highlight::NewGrid) => ("GRID", crate::theme::CYAN()),
                        Some(sdroxide_types::Highlight::NewCall) => {
                            ("NEW", crate::theme::CYAN_DIM())
                        }
                        Some(sdroxide_types::Highlight::Dupe) => ("DUPE", crate::theme::gray(85)),
                        None => ("", Color32::TRANSPARENT),
                    };
                    let dupe = novelty.dupe;
                    let grid = d.grid.clone().unwrap_or_default();
                    // Where in the world they are, from the callsign alone —
                    // most decodes carry no grid, and the entity always knows.
                    let continent = entity.map(|e| e.continent).unwrap_or("");
                    let flag = entity.map(|e| e.flag).unwrap_or("");
                    let country = entity.map(|e| e.name).unwrap_or("");
                    let is_preview =
                        d.from.is_some() && preview_call.as_deref() == d.from.as_deref();
                    let queued = d
                        .from
                        .as_deref()
                        .is_some_and(|f| queued_calls.iter().any(|q| q.eq_ignore_ascii_case(f)));
                    let mut reply = false;
                    let mut queue = false;
                    // Left edge of the REPLY button, so the row-body click area can
                    // exclude it (otherwise the full-row interaction below sits on
                    // top of the button and swallows its clicks).
                    let mut reply_left: Option<f32> = None;

                    // Everything the row shows, built once and placed by
                    // whichever of the two layouts below runs.
                    //
                    // In the single-list view the turn headers are gone, so the
                    // row itself says when it arrived: its slot time, coloured
                    // by the slot's parity — the two facts the headers carried.
                    let time_lbl = single.then(|| {
                        let even = ((d.slot_utc as f64 / period).round() as i64).rem_euclid(2) == 0;
                        let s = d.slot_utc.rem_euclid(86_400);
                        egui::Label::new(
                            RichText::new(format!(
                                "{:02}:{:02}:{:02}",
                                s / 3600,
                                (s % 3600) / 60,
                                s % 60
                            ))
                            .monospace()
                            .size(10.5)
                            .color(if even {
                                crate::theme::CYAN()
                            } else {
                                crate::theme::YELLOW()
                            }),
                        )
                    });
                    let dist_txt = dist_km.map(|km| format!("{km:.0} km")).unwrap_or_default();
                    let snr_lbl = egui::Label::new(
                        RichText::new(format!("{:+}", d.snr_db))
                            .monospace()
                            .size(13.0)
                            .color(snr_color(d.snr_db)),
                    );
                    // Audio frequency (one-line layout only).
                    let freq_lbl = egui::Label::new(
                        RichText::new(format!("{:.0}", d.audio_hz))
                            .monospace()
                            .size(12.0)
                            .color(crate::theme::gray(120)),
                    );
                    // Callsign — wider proportional (button) font.
                    let call_lbl =
                        egui::Label::new(RichText::new(&who).size(15.0).strong().color(if to_me {
                            crate::theme::YELLOW()
                        } else if d.from.is_none() || dupe {
                            crate::theme::gray(105)
                        } else if cq {
                            crate::theme::GREEN()
                        } else {
                            crate::theme::TEXT_STRONG()
                        }))
                        .truncate();
                    // What they'd be worth: new entity / band / grid / call, or
                    // a dupe already in the log for this band.
                    let badge_lbl =
                        egui::Label::new(RichText::new(badge).size(9.5).strong().color(badge_col));
                    // Continent — the band's opening, readable down the column
                    // without reading a single callsign.
                    let cont_lbl = egui::Label::new(
                        RichText::new(continent).monospace().size(11.0).strong().color(if dupe {
                            crate::theme::gray(85)
                        } else {
                            crate::theme::continent_color(continent)
                        }),
                    );
                    // The entity in full. Deliberately plain grey next to the
                    // colour-coded continent beside it — two coloured columns
                    // side by side stop either one meaning anything.
                    let country_col = crate::theme::gray(if dupe { 90 } else { 155 });
                    let country_lbl =
                        egui::Label::new(RichText::new(country).size(11.0).color(country_col))
                            .truncate();
                    let grid_lbl = egui::Label::new(
                        RichText::new(&grid).monospace().size(12.0).color(crate::theme::CYAN_DIM()),
                    );
                    // Distance (km, great-circle from my grid).
                    let dist_lbl = egui::Label::new(
                        RichText::new(&dist_txt)
                            .monospace()
                            .size(11.0)
                            .color(crate::theme::YELLOW()),
                    );
                    let msg_lbl =
                        egui::Label::new(RichText::new(&d.message).monospace().size(12.5).color(
                            if dupe { crate::theme::gray(95) } else { crate::theme::TEXT() },
                        ))
                        .truncate();
                    // REPLY and the queue button, drawn right-to-left so they
                    // pin to the right edge in either layout. The queue chip
                    // marks a station for later; pressing it again drops the
                    // station, so one button both queues and un-queues.
                    //
                    // Both are greyed on a receiver: answering a station and
                    // lining one up to answer later are the same promise to
                    // transmit, and a list of stations you cannot work should
                    // say so on the row rather than only when the key fails.
                    let buttons = |ui: &mut egui::Ui| {
                        let resp = tx_gated(ui, tx_ok, |ui| {
                            crate::chrome::chip_accent(
                                ui,
                                false,
                                RichText::new("REPLY").size(12.0).strong(),
                                if to_me {
                                    crate::theme::YELLOW()
                                } else if cq {
                                    crate::theme::GREEN()
                                } else {
                                    crate::theme::CYAN()
                                },
                                crate::theme::INK_ON_CYAN(),
                            )
                        });
                        let qresp = tx_gated(ui, tx_ok, |ui| {
                            crate::chrome::chip(
                                ui,
                                queued,
                                RichText::new(if queued { "＋" } else { "+" }).size(12.0).strong(),
                            )
                            .on_hover_text(if queued {
                                "Queued — click to remove"
                            } else {
                                "Work this station after the current one"
                            })
                        });
                        (resp, qresp)
                    };

                    let inner = egui::Frame::new()
                        .fill(if to_me {
                            crate::theme::TOME_BG()
                        } else if cq {
                            crate::theme::CQ_BG()
                        } else {
                            crate::theme::ROW_BG()
                        })
                        .inner_margin(egui::Margin { left: 11, right: 6, top: 6, bottom: 6 })
                        .show(ui, |ui| {
                            // Fixed-width columns so every field lines up down the
                            // list. Right-aligned numbers, then callsign (wide
                            // proportional font), grid, and the message filling the
                            // rest with a right-pinned REPLY button.
                            let ch = 22.0;
                            let cell =
                                |ui: &mut egui::Ui, w: f32, align_right: bool, lbl: egui::Label| {
                                    row_cell(ui, w, ch, align_right, lbl)
                                };
                            if narrow {
                                // Two lines: who (and the buttons) above, what
                                // they said below. The audio-frequency column
                                // drops — clicking the row tunes there anyway —
                                // and grid + distance move into the message
                                // line's dim tail.
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.horizontal(|ui| {
                                    ui.set_min_height(ch);
                                    ui.spacing_mut().item_spacing.x = 7.0;
                                    cell(ui, 28.0, true, snr_lbl);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (resp, qresp) = buttons(ui);
                                            reply = resp.clicked();
                                            queue = qresp.clicked();
                                            reply_left =
                                                Some(resp.rect.left().min(qresp.rect.left()));
                                            ui.with_layout(
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    // The callsign gets what the
                                                    // badges leave, and truncates
                                                    // before it pushes them out.
                                                    let call_w = (ui.available_width()
                                                        - (34.0 + 22.0 + 24.0 + 3.0 * 7.0))
                                                        .max(40.0);
                                                    cell(ui, call_w, false, call_lbl);
                                                    cell(ui, 34.0, false, badge_lbl);
                                                    row_cell_ui(ui, 22.0, ch, |ui| {
                                                        flags.show(ui, flag, 12.0);
                                                    });
                                                    cell(ui, 24.0, false, cont_lbl);
                                                },
                                            );
                                        },
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 7.0;
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            // Slot time at the row's right edge
                                            // (single-list view only) — the
                                            // narrow layout has no column for
                                            // it, and here it scans down the
                                            // list like one.
                                            if let Some(t) = time_lbl {
                                                ui.add(t);
                                            }
                                            let tail = [country, grid.as_str(), dist_txt.as_str()]
                                                .iter()
                                                .filter(|s| !s.is_empty())
                                                .copied()
                                                .collect::<Vec<_>>()
                                                .join(" · ");
                                            if !tail.is_empty() {
                                                ui.label(
                                                    RichText::new(tail)
                                                        .monospace()
                                                        .size(11.0)
                                                        .color(crate::theme::CYAN_DIM()),
                                                );
                                            }
                                            ui.with_layout(
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.add(msg_lbl);
                                                },
                                            );
                                        },
                                    );
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    ui.set_min_height(ch);
                                    ui.spacing_mut().item_spacing.x = 7.0;
                                    if let Some(t) = time_lbl {
                                        cell(ui, 54.0, false, t);
                                    }
                                    cell(ui, 28.0, true, snr_lbl);
                                    cell(ui, 40.0, true, freq_lbl);
                                    cell(ui, 98.0, false, call_lbl);
                                    cell(ui, 34.0, false, badge_lbl);
                                    row_cell_ui(ui, 22.0, ch, |ui| {
                                        flags.show(ui, flag, 12.0);
                                    });
                                    cell(ui, 24.0, false, cont_lbl);
                                    if name_country {
                                        cell(ui, 112.0, false, country_lbl);
                                    }
                                    cell(ui, 44.0, false, grid_lbl);
                                    cell(ui, 58.0, true, dist_lbl);
                                    // Message fills the remaining width; REPLY and
                                    // the queue button pinned right.
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (resp, qresp) = buttons(ui);
                                            reply = resp.clicked();
                                            queue = qresp.clicked();
                                            reply_left =
                                                Some(resp.rect.left().min(qresp.rect.left()));
                                            ui.with_layout(
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.add(msg_lbl);
                                                },
                                            );
                                        },
                                    );
                                });
                            }
                        });

                    let r = inner.response.rect;
                    // Left-accent bar: gold (to us) / red (CQ) / cyan (other). Wider
                    // for a to-us decode so it really pops — and for a directed CQ
                    // (DX, EU, JA …) that names us, which is a better prospect than
                    // a plain CQ anyone in the world is free to answer.
                    let (accent, aw) = if to_me {
                        (crate::theme::YELLOW(), 4.0)
                    } else if cq {
                        (crate::theme::PINK(), if d.cq_to.is_some() { 4.0 } else { 2.5 })
                    } else {
                        (crate::theme::CYAN_DIM(), 2.5)
                    };
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            r.left_top(),
                            egui::pos2(r.left() + aw, r.bottom()),
                        ),
                        0.0,
                        accent,
                    );
                    // Row-body click (everything left of the REPLY button) tunes
                    // the audio freq. Excluding the button's rect keeps this
                    // interaction from covering — and stealing clicks from — REPLY.
                    let body_right = reply_left.map(|x| x - 2.0).unwrap_or(r.right());
                    let body_rect =
                        egui::Rect::from_min_max(r.left_top(), egui::pos2(body_right, r.bottom()));
                    let row =
                        ui.interact(body_rect, ui.id().with(("dec", i)), egui::Sense::click());
                    // Everything already resolved about this station, gathered
                    // where there is room to say it: the row itself has to fit
                    // twenty of these on screen.
                    let row = row.on_hover_ui(|ui| {
                        station_card(
                            ui, flags, d, entity, dist_km, &my_grid, novelty, band, queued, cq,
                        );
                    });
                    if is_preview {
                        // Amber outline ties this row to its faint map marker.
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(1.0, crate::theme::YELLOW()),
                            egui::StrokeKind::Inside,
                        );
                    } else if to_me {
                        // A message to our own station: a gold box so it can't be missed.
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(1.4, crate::theme::YELLOW()),
                            egui::StrokeKind::Inside,
                        );
                    } else if row.hovered() {
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(1.0, crate::theme::CYAN_DIM()),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if row.hovered() {
                        hover_ll = d.grid.as_deref().and_then(sdroxide_types::grid_to_latlon);
                    }
                    if reply {
                        if let Some(from) = &d.from {
                            // If they're neither calling CQ nor calling us, hold until
                            // they call CQ rather than barging into their exchange.
                            // Any CQ counts here, including a "CQ DX" we're local to:
                            // the operator asked to call them, and they are free now.
                            cmds.push(Command::DigiStartQso {
                                from: from.clone(),
                                grid: d.grid.clone(),
                                snr: d.snr_db,
                                audio_hz: d.audio_hz,
                                wait_for_cq: !d.is_cq && !to_me,
                            });
                        }
                        // Starting a QSO promotes the station to the active DX
                        // marker; drop the faint preview so they don't overlap.
                        new_preview = Some(None);
                        // On a phone the exchange happens in the other view, so
                        // answering somebody takes you there. Leaving the
                        // operator on the decode list would have them watching
                        // the sequencer they just started from behind a chip.
                        if phone {
                            self.view.digi_pane =
                                crate::app::panels::pane_index(self.state.rx[0].mode, "QSO");
                        }
                    } else if queue {
                        if let Some(from) = &d.from {
                            if queued {
                                cmds.push(Command::DigiQueueRemove(from.clone()));
                            } else {
                                cmds.push(Command::DigiQueueAdd {
                                    from: from.clone(),
                                    grid: d.grid.clone(),
                                    snr: d.snr_db,
                                    audio_hz: d.audio_hz,
                                    // Same judgement the reply button makes: a
                                    // station mid-exchange is not free yet.
                                    wait_for_cq: !d.is_cq && !to_me,
                                });
                            }
                        }
                    } else if row.clicked() {
                        // Moving our transmit onto theirs is exactly what Auto
                        // TX FRQ exists to avoid, so with it on a click only
                        // previews the station. Hold TX suppresses it too: the
                        // engine would refuse the command anyway, and sending
                        // one it drops on the floor is how a UI comes to
                        // disagree with the radio.
                        if !auto_tx_freq && !hold_tx_freq {
                            cmds.push(Command::SetDigiAudioFreq(d.audio_hz));
                        }
                        // Preview this station's location (if it sent a grid).
                        let ll = d.grid.as_deref().and_then(sdroxide_types::grid_to_latlon);
                        new_preview = Some(ll.map(|ll| (who.clone(), ll)));
                    }
                    ui.add_space(3.0);
                }
                gi = end;
            }
        });
        self.digi_hover_ll = hover_ll;
        if let Some(sel) = new_preview {
            self.digi_preview = sel;
        }
    }

    /// The QSO operating area to the right of the decode list: header row,
    /// world map, station card, transcript, and action buttons.
    pub(in crate::app) fn qso_area(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let status = self.digi_status.clone();

        let tier = crate::layout::tier(ui.ctx());
        let compact = tier.compact();
        let phone = tier == crate::layout::Tier::Phone;

        // Header: QSO left, session log + downloads centered, SETUP right.
        // The count is this run's; the export buttons still save the whole
        // logbook, which is what their hover text says.
        //
        // A phone keeps only the two that steer the mode — the frequency chip
        // and SETUP. The session count and the ADIF/TXT exports are a row of
        // screen for something the logbook window (SYS → LOG) already offers,
        // exports included.
        let session = self.session_qsos;
        let logged = self.qso_log.len();
        // Measured, not assumed: this row holds `Button`s and chips, and both
        // grow with the touch style. Allocating the desktop's 26 points for a
        // 34-point row is what pushed the buttons at the bottom of this column
        // off the end of it.
        let row_h = 26.0_f32.max(ui.spacing().interact_size.y);
        let (row, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        let third = row.width() / 3.0;
        let zone = |i: f32| {
            egui::Rect::from_min_size(
                egui::pos2(row.left() + i * third, row.top()),
                egui::vec2(third, row_h),
            )
        };
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(zone(0.0))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.label(RichText::new("QSO").size(9.5).strong().color(crate::theme::CYAN_DIM()));
                self.digi_freq_chip(ui, cmds);
            },
        );
        if !phone {
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(zone(1.0))
                    .layout(egui::Layout::top_down(egui::Align::Center)),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("Session: {session} QSO"))
                                .size(11.0)
                                .color(crate::theme::gray(150)),
                        )
                        .on_hover_text("QSOs worked since sdroxide was started");
                        if ui
                            .add_enabled(logged > 0, egui::Button::new("ADIF"))
                            .on_hover_text(format!("Save the whole logbook ({logged} QSO) as ADIF"))
                            .clicked()
                        {
                            let adif = sdroxide_types::qso_log_to_adif(&self.qso_log);
                            crate::download::save("sdroxide-log.adi", adif.as_bytes());
                        }
                        if ui
                            .add_enabled(logged > 0, egui::Button::new("TXT"))
                            .on_hover_text(format!("Save the whole logbook ({logged} QSO) as text"))
                            .clicked()
                        {
                            let txt = sdroxide_types::qso_log_to_text(&self.qso_log);
                            crate::download::save("sdroxide-log.txt", txt.as_bytes());
                        }
                    });
                },
            );
        }
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(zone(2.0))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                if crate::chrome::chip(ui, self.show_digi_settings, "⚙ SETUP").clicked() {
                    self.show_digi_settings = !self.show_digi_settings;
                }
            },
        );

        ui.add_space(5.0);
        // World map — its height is a user-draggable fraction of the QSO area,
        // clamped so the station card + a usable transcript + the action buttons
        // stay visible. On very short windows it shrinks and then disappears.
        //
        // A compact layout has no map at all. It is the largest thing in this
        // column and the only one that is purely nice to have: everything else
        // here is either the state of the QSO or a control that changes it, and
        // on a tablet the map was taking the room they needed. The same
        // stations are still on the panadapter and in the 3D view.
        let show_map = !compact;
        let gap = 8.0;
        // Only the map budget needs this: the rows below the transcript place
        // themselves (see the bottom-up layout further down). Measured rather
        // than fixed at 44 so it still means "a row of buttons" on a layout
        // whose chips are half again as tall.
        let btn_h = crate::chrome::chip_height(ui, Some(15.0));
        let map_handle_h = 7.0;
        const CARD_RESERVE: f32 = 60.0;
        /// Enough conversation box to be a conversation: three or four lines.
        /// It is also what egui's own scroll area asks for, so below this the
        /// box is being squeezed rather than merely small.
        const TRANSCRIPT_MIN: f32 = 64.0;
        let full_h = ui.available_height();
        let avail_w = ui.available_width();
        // The map height is a user-draggable fraction of a range: from MIN_HEIGHT
        // up to whatever still leaves the station card + action buttons room below
        // (the transcript can shrink to nothing). `digi_map_fraction` slides
        // linearly across this range, so the divider tracks the cursor 1:1 and the
        // map genuinely grows/shrinks. Height is capped at the width (aspect ≤ 1).
        let map_lo = crate::widgets::worldmap::MIN_HEIGHT;
        // What has to be left underneath the map: the station card, *both* rows
        // of controls pinned to the bottom edge with the gap between them, and
        // a transcript worth the name. The reserve used to count one button row
        // and nothing at all for the transcript, so a map at its default share
        // left the box a few points tall — and before the box learned to be
        // squeezed (see [`Self::transcript`]) it overflowed downward onto the
        // message row instead, which is how that row went missing (issue #231).
        let below_map = map_handle_h + CARD_RESERVE + 5.0 + gap + 2.0 * btn_h + TRANSCRIPT_MIN;
        let map_hi = (full_h - below_map).min(avail_w).max(map_lo);
        let map_budget = map_lo + (map_hi - map_lo) * self.view.digi_map_fraction;
        let my_grid = status.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default();
        // Everything from here to the map itself is only ever read by the map,
        // so a layout without one does none of it — including walking every
        // spot the client holds, once a frame.
        if show_map && map_budget >= crate::widgets::worldmap::MIN_HEIGHT {
            let hover_ll = self.digi_hover_ll;
            let home_ll = sdroxide_types::grid_to_latlon(&my_grid);
            let dx_grid = status.as_ref().and_then(|s| s.dx_grid.clone());
            let dx_ll = dx_grid.as_deref().and_then(sdroxide_types::grid_to_latlon);
            // A clicked (but not yet answered) decode shows as a faint preview.
            let preview_ll = self.digi_preview.as_ref().map(|(_, ll)| *ll);
            let tx_active = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
            // White station dots fade over 2 minutes since a station was last
            // decoded, then expire (dropped from the map and from the zoom fit).
            // Ages use egui frame time (monotonic, works native + wasm); each grid
            // remembers the frame it was last freshly decoded in.
            let now_t = ui.input(|i| i.time);
            self.digi_stations.observe(&self.digi_decodes, now_t, now_unix());
            let stations = self.digi_stations.stations(now_t);
            // Located network spots (filtered by the shown-kind toggles), as
            // kind-coloured dots on the map.
            //
            // Only spots announced as FT8 or FT4, whatever fed them: this is the map
            // beside the FT8/FT4 decode list, and it exists to show where the mode is
            // being worked right now. Broadcast stations, FreeDV Reporter's hundreds
            // of connected stations, and the SSB/CW half of the cluster all buried
            // that with traffic from bands the panel cannot answer. They are still on
            // the panadapter overlay and in the SPOTS window.
            let spot_dots: Vec<(f64, f64, (u8, u8, u8))> = self
                .all_spots()
                .filter(|s| is_ft8_or_ft4(&s.mode))
                .filter(|s| self.spot_visible(s))
                .filter_map(|s| {
                    s.loc.map(|(lat, lon)| (lat, lon, crate::theme::spot_color(s.kind)))
                })
                .collect();
            let heat = self.prop_texture(ui.ctx(), self.state.rx_freq_hz());
            self.prop_map_controls(ui);
            crate::widgets::worldmap::show(
                ui,
                &mut self.map_view,
                home_ll,
                dx_ll,
                preview_ll,
                hover_ll,
                &stations,
                &spot_dots,
                heat,
                tx_active,
                map_budget,
            );
            // Draggable border between the map and the QSO form below it.
            let hresp = crate::chrome::split_handle(
                ui,
                egui::vec2(ui.available_width(), map_handle_h),
                None,
            );
            if hresp.dragged() {
                // 1:1 with the cursor: a drag of `dy` px moves the map edge `dy` px.
                let df = hresp.drag_delta().y / (map_hi - map_lo).max(1.0);
                self.view.digi_map_fraction = (self.view.digi_map_fraction + df).clamp(0.0, 1.0);
            }
        }
        // Station card.
        crate::chrome::red_panel(ui, |ui| {
            match status.as_ref() {
                Some(s) => {
                    ui.horizontal(|ui| {
                        // The contact is over and logged once we are confirming;
                        // green says so, since "Confirming" on its own reads like
                        // an exchange still in progress.
                        let done = s.step == sdroxide_types::QsoStep::Confirming;
                        let step_col =
                            if done { crate::theme::GREEN() } else { crate::theme::CYAN() };
                        ui.label(RichText::new(s.step.label()).size(13.0).strong().color(step_col));
                        if done {
                            ui.label(
                                RichText::new("✓ QSO COMPLETE")
                                    .size(11.0)
                                    .strong()
                                    .color(crate::theme::GREEN()),
                            )
                            .on_hover_text(
                                "The contact is complete and in the log. It is held here for a \
                                 few minutes so the final message can be re-sent if the other \
                                 station repeats theirs — nothing more is owed.",
                            );
                        }
                        if s.transmitting {
                            ui.label(
                                RichText::new("● TX")
                                    .size(13.0)
                                    .strong()
                                    .color(crate::theme::ALERT()),
                            );
                        }
                        if s.config.dxped_mode != sdroxide_types::DxpedMode::Normal
                            && matches!(s.mode, Mode::Ft8 | Mode::Ft2)
                        {
                            ui.label(
                                RichText::new(s.config.dxped_mode.label().to_uppercase())
                                    .size(11.0)
                                    .strong()
                                    .color(crate::theme::PINK()),
                            )
                            .on_hover_text(
                                "DXpedition mode. The transmit frequency is held out of the \
                                 Fox's half of the passband (below 1000 Hz) until the Fox \
                                 answers.",
                            );
                        }
                        if s.tx_watchdog {
                            // The sequencer stood down on its own; say so, since
                            // an idle step alone looks like nothing happened.
                            ui.label(
                                RichText::new("WATCHDOG")
                                    .size(11.0)
                                    .strong()
                                    .color(crate::theme::YELLOW()),
                            )
                            .on_hover_text(
                                "Transmitting stopped: no reply and no action for the watchdog \
                                 period. Call CQ or pick a message to resume.",
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{:.0} Hz · {} slots",
                                    s.audio_hz,
                                    if s.tx_even { "even" } else { "odd" }
                                ))
                                .size(11.0)
                                .color(crate::theme::gray(140)),
                            );
                            // What everyone else's timing says about ours. A
                            // clock far enough out that nobody can decode us
                            // looks exactly like a dead band from this side, so
                            // it is worth a permanent readout.
                            if let Some(off) = s.clock_offset_s {
                                use sdroxide_types::ClockHealth::*;
                                let health = sdroxide_types::clock_health(off);
                                let col = match health {
                                    Good => crate::theme::gray(140),
                                    Marginal => crate::theme::YELLOW(),
                                    Bad => crate::theme::PINK(),
                                };
                                let txt = RichText::new(format!("DT {off:+.1} s")).size(11.0);
                                ui.label(if health == Good {
                                    txt.color(col)
                                } else {
                                    txt.strong().color(col)
                                })
                                .on_hover_text(format!(
                                    "Your slot timing against the stations you are hearing.\n\
                                     {}\n\nIt covers the whole receive path, so a slow audio or \
                                     network chain counts the same as a wrong clock. Under 0.5 s \
                                     is comfortable.",
                                    match health {
                                        Good => "Well inside tolerance.".to_string(),
                                        _ => format!(
                                            "You transmit {:.1} s {} everyone else — the usual \
                                             reason calls go unanswered. Sync your computer clock.",
                                            off.abs(),
                                            if off > 0.0 { "before" } else { "after" },
                                        ),
                                    }
                                ));
                            }
                        });
                    });
                    match &s.dx_call {
                        Some(dx) => {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(dx)
                                        .size(17.0)
                                        .strong()
                                        .color(crate::theme::TEXT_STRONG()),
                                );
                                if let Some(g) = &s.dx_grid {
                                    ui.label(
                                        RichText::new(g).size(13.0).color(crate::theme::CYAN_DIM()),
                                    );
                                }
                                if let (Some(hg), Some(dg)) = (
                                    (!my_grid.is_empty()).then_some(my_grid.as_str()),
                                    s.dx_grid.as_deref(),
                                ) {
                                    if let (Some(km), Some(brg)) = (
                                        sdroxide_types::grid_distance_km(hg, dg),
                                        sdroxide_types::grid_bearing(hg, dg),
                                    ) {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:.0} km · {:.0}°",
                                                        km, brg
                                                    ))
                                                    .size(12.0)
                                                    .color(crate::theme::YELLOW()),
                                                );
                                            },
                                        );
                                    }
                                }
                            });
                        }
                        None => {
                            ui.label(
                                RichText::new("no active QSO — pick a decode to reply, or Call CQ")
                                    .size(11.0)
                                    .color(crate::theme::gray(120)),
                            );
                        }
                    }
                }
                None => {
                    ui.label(
                        RichText::new("FT8 engine idle").size(12.0).color(crate::theme::gray(130)),
                    );
                }
            }
        });

        // Fox pile-up: who is being worked and who is waiting. Only a Fox has
        // one, so its presence is the mode indicator.
        let fox_queue = status.as_ref().map(|s| s.fox_queue.clone()).unwrap_or_default();
        if !fox_queue.is_empty() {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(
                    RichText::new(format!("PILE-UP {}", fox_queue.len()))
                        .size(9.5)
                        .strong()
                        .color(crate::theme::CYAN_DIM()),
                );
                for c in &fox_queue {
                    let col =
                        if c.working { crate::theme::GREEN() } else { crate::theme::gray(150) };
                    ui.label(RichText::new(&c.call).size(11.5).strong().color(col)).on_hover_text(
                        format!(
                            "{} · {:+} dB{}",
                            if c.working { "being worked" } else { "waiting" },
                            c.snr_db,
                            c.grid.as_deref().map(|g| format!(" · {g}")).unwrap_or_default(),
                        ),
                    );
                }
            });
        }

        // The call queue: stations marked to be worked, in the order they will
        // be taken. Clicking one drops it; CLEAR empties the lot.
        let call_queue = status.as_ref().map(|s| s.call_queue.clone()).unwrap_or_default();
        if !call_queue.is_empty() {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(
                    RichText::new(format!("QUEUE {}", call_queue.len()))
                        .size(9.5)
                        .strong()
                        .color(crate::theme::CYAN_DIM()),
                );
                for (i, q) in call_queue.iter().enumerate() {
                    // The one going next is the one worth reading first.
                    let col = if i == 0 { crate::theme::GREEN() } else { crate::theme::gray(150) };
                    if crate::chrome::chip(ui, false, RichText::new(&q.call).size(11.5).color(col))
                        .on_hover_text(format!(
                            "{} · {:+} dB · {:.0} Hz{}\nClick to remove",
                            if i == 0 { "next" } else { "waiting" },
                            q.snr_db,
                            q.audio_hz,
                            q.grid.as_deref().map(|g| format!(" · {g}")).unwrap_or_default(),
                        ))
                        .clicked()
                    {
                        cmds.push(Command::DigiQueueRemove(q.call.clone()));
                    }
                }
                if crate::chrome::chip(ui, false, RichText::new("CLEAR").size(10.0)).clicked() {
                    cmds.push(Command::DigiQueueRemove(String::new()));
                }
            });
        }

        // The rest of the column: the conversation, the message picker and the
        // action buttons.
        //
        // Laid out bottom-up, so the two rows that have to stay reachable are
        // placed first — from the bottom edge, where they belong — and the
        // conversation takes whatever is left above them. The alternative is to
        // predict their height and give the transcript the remainder, and every
        // prediction of it has been wrong on some screen: the rows carry chips
        // whose padding comes from the style, a text field that sizes from
        // `interact_size`, and a button row that wraps when three no longer fit
        // across. Asking the layout instead of guessing is what keeps STOP TX
        // on the screen.
        ui.add_space(5.0);
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            self.qso_controls(ui, cmds, gap);
            // Whatever is left above the controls is the conversation's.
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                self.transcript(ui, status.as_ref());
            });
        });
    }

    /// The conversation box: what has been sent and heard this QSO.
    fn transcript(&mut self, ui: &mut egui::Ui, status: Option<&sdroxide_types::DigiStatus>) {
        ui.allocate_ui(egui::vec2(ui.available_width(), ui.available_height()), |ui| {
            let inner = egui::Frame::new()
                .fill(crate::theme::ROW_BG())
                .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP()))
                .inner_margin(egui::Margin { left: 9, right: 7, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let budget = ui.available_height();
                    ui.set_min_height(budget);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        // A scroll area is 64 points tall by default whatever it
                        // is given, and this box's whole job is to take
                        // *whatever is left* above the controls. On a short
                        // screen that is less than 64, and the box then
                        // overflowed its allocation and painted over the message
                        // row pinned underneath it — so the row that chooses
                        // what goes out next was not on the screen at all, with
                        // nothing to say it was missing (issue #231). The floor
                        // is the budget, so the box is squeezed rather than the
                        // thing below it.
                        .min_scrolled_height(budget.max(0.0).min(64.0))
                        .stick_to_bottom(true)
                        .show_themed(ui, |ui| {
                            let mut any = false;
                            if let Some(s) = status.as_ref() {
                                for line in &s.transcript {
                                    any = true;
                                    // Pink marks traffic that isn't ours: the
                                    // station we called is working someone else.
                                    let (tag, col) = if line.done {
                                        ("✓", crate::theme::GREEN())
                                    } else if line.overheard {
                                        ("·", crate::theme::PINK())
                                    } else if line.tx {
                                        ("»", crate::theme::YELLOW())
                                    } else {
                                        ("«", crate::theme::GREEN())
                                    };
                                    let txt = RichText::new(format!("{tag} {}", line.text))
                                        .monospace()
                                        .size(12.5)
                                        .color(col);
                                    // Received messages are green too, so the
                                    // completion line needs more than colour to
                                    // stand out at the end of a run of them.
                                    ui.label(if line.done {
                                        txt.strong().background_color(crate::theme::DONE_BG())
                                    } else {
                                        txt
                                    });
                                }
                                if let Some(msg) = &s.tx_pending_msg {
                                    any = true;
                                    ui.label(
                                        RichText::new(format!("→ {msg}"))
                                            .monospace()
                                            .size(11.5)
                                            .color(crate::theme::gray(150)),
                                    );
                                }
                            }
                            if !any {
                                ui.label(
                                    RichText::new("— no messages —")
                                        .monospace()
                                        .size(11.5)
                                        .color(crate::theme::gray(90)),
                                );
                            }
                        });
                });
            // Red left-accent bar (matching chrome::red_panel).
            let r = inner.response.rect;
            ui.painter().rect_filled(
                egui::Rect::from_min_max(r.left_top(), egui::pos2(r.left() + 2.5, r.bottom())),
                0.0,
                crate::theme::PINK(),
            );
        });
    }

    /// The two rows under the conversation: the message picker and the action
    /// buttons. Drawn into a bottom-up layout, so they are added in the order
    /// they should sit from the bottom edge — buttons first.
    fn qso_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, gap: f32) {
        let status = self.digi_status.clone();
        let in_qso = status
            .as_ref()
            .map(|s| {
                !matches!(
                    s.step,
                    sdroxide_types::QsoStep::Idle | sdroxide_types::QsoStep::Confirming
                )
            })
            .unwrap_or(false);
        // Action buttons (larger for touch). Wrapped, so a column too narrow
        // for all three takes a second row rather than clipping the last one —
        // and STOP TX is the one that would have been clipped.
        // Only the two controls that put us on the air are gated on there being
        // a transmitter: STOP QSO and STOP TX stay live, because a stop is
        // worth having even where it should have nothing to stop.
        // SWL mode: hide the entire action-button row.
        let tx_ok = self.tx_capable();
        if !self.ui_settings.swl {
            ui.horizontal_wrapped(|ui| {
                let cq = ui.add_enabled_ui(!in_qso && tx_ok, |ui| {
                    rx_only_hint(
                        crate::chrome::chip_accent(
                            ui,
                            false,
                            RichText::new("  CALL CQ  ").size(15.0).strong(),
                            crate::theme::GREEN(),
                            crate::theme::INK_ON_CYAN(),
                        ),
                        tx_ok,
                    )
                });
                if cq.inner.clicked() {
                    cmds.push(Command::DigiCallCq);
                }
                if crate::chrome::chip(ui, false, RichText::new(" STOP QSO ").size(14.0)).clicked()
                {
                    cmds.push(Command::DigiStopQso);
                }
                if crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(" STOP TX ").size(15.0).strong(),
                    crate::theme::ALERT(),
                    Color32::WHITE,
                )
                .clicked()
                {
                    cmds.push(Command::DigiAbortTx);
                }
            });
        }
        ui.add_space(gap);
        // Message picker: choose by hand which message goes next (WSJT-X's
        // Tx1–Tx6), or send a line of free text in the next slot.
        // SWL mode: hide the message picker and free text entry.
        if self.ui_settings.swl {
            return;
        }
        let has_dx = status.as_ref().and_then(|s| s.dx_call.as_ref()).is_some();
        let step_now = status.as_ref().map(|s| s.step);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (step, label) in [
                (sdroxide_types::QsoStep::TxGrid, "GRID"),
                (sdroxide_types::QsoStep::TxReport, "RPT"),
                (sdroxide_types::QsoStep::TxRReport, "R+RPT"),
                (sdroxide_types::QsoStep::TxRr73, "RR73"),
                (sdroxide_types::QsoStep::Tx73, "73"),
            ] {
                let resp = ui.add_enabled_ui(has_dx, |ui| {
                    crate::chrome::chip(ui, step_now == Some(step), RichText::new(label).size(11.0))
                });
                if resp.inner.clicked() {
                    cmds.push(Command::DigiSetStep(step));
                }
            }
            ui.add_space(4.0);
            // Free text: 13 characters is all FT8 carries, so cap the entry
            // there rather than letting the operator type a message that would
            // be silently cut on the air.
            let entry = crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.digi_free_text)
                    .desired_width(ui.available_width() - 52.0)
                    .char_limit(13)
                    .hint_text("free text (13 chars)"),
            );
            let send = tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new("SEND").size(11.0).strong(),
                    crate::theme::CYAN(),
                    crate::theme::INK_ON_CYAN(),
                )
            });
            // Return is the same button: a receiver must not have a keyboard
            // path onto the air that the greyed chip beside it denies.
            let entered =
                tx_ok && entry.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (send.clicked() || entered) && !self.digi_free_text.trim().is_empty() {
                cmds.push(Command::DigiSendText(self.digi_free_text.clone()));
                self.digi_free_text.clear();
            }
        });
    }
}
