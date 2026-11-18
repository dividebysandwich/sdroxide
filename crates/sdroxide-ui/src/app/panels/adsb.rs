//! The ADS-B panel: what is in the air, and where.
//!
//! Two things an operator watches, so two columns:
//!
//! - **AIRCRAFT** — every target being tracked, newest activity first, with the
//!   detail card for whichever one is selected pinned below the list.
//! - **MAP** — the same targets as a radar picture. See [`crate::adsb_map`].
//!
//! No third column, because unlike APRS there is nothing to say back: this is a
//! receive-only surveillance downlink and the aircraft are not listening.
//!
//! # The header says what the receiver is doing
//!
//! An empty aircraft list has three quite different causes — a quiet sky, a
//! receiver on the wrong frequency, and a stream too narrow to slice a
//! half-microsecond chip — and only one of them is anything to do with the
//! decoder. The header carries the preamble and frame counters and, when the
//! lane cannot run at all, the sentence saying why, with the retune that fixes
//! it beside it.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{AdsbAircraft, AdsbStatus, Command};

use crate::app::SdroxideApp;
use crate::app::util::fmt_age;
use crate::theme;
use crate::theme::ThemedScroll;

/// How the aircraft table is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum AdsbSort {
    /// Most recently heard first. The default: on a display of things that move,
    /// "what just changed" is the question a list answers.
    #[default]
    Heard,
    Callsign,
    Altitude,
    Speed,
    /// Nearest first. Only offered once the operator's own position is known —
    /// there is no distance without one.
    Range,
    Signal,
}

/// How the sighting log is ordered.
/// Why: HDZ 2026-11-17: „das Buch muss sortierbare Header bekommen“.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum LogSort {
    /// Last heard first, as the log opens.
    #[default]
    Seen,
    Callsign,
    Address,
    Type,
    Route,
    /// Farthest first.
    Range,
    /// Highest first.
    Altitude,
}

/// Column offsets of the sighting log, heading and rows alike.
const LOG_X: [f32; 8] = [4.0, 84.0, 152.0, 168.0, 220.0, 262.0, 360.0, 420.0];

/// Order the sighting log. Text sorts A–Z ascending; an empty field or a
/// missing number goes last both ways round, as in the aircraft table.
fn sort_log(rows: &mut [&crate::adsb_log::Sighting], by: LogSort, desc: bool) {
    use crate::adsb_log::Sighting;
    use std::cmp::Ordering;
    let text = |x: &Sighting| -> String {
        match by {
            LogSort::Callsign => x.call.clone(),
            LogSort::Address => format!("{:06X}", x.icao),
            LogSort::Type => x.kind.clone(),
            LogSort::Route => x.route.clone(),
            _ => String::new(),
        }
    };
    let num = |x: &Sighting| -> Option<f64> {
        match by {
            LogSort::Seen => Some(x.last as f64),
            LogSort::Range => (!x.max_km.is_nan()).then_some(f64::from(x.max_km)),
            LogSort::Altitude => x.max_alt_ft.map(f64::from),
            _ => None,
        }
    };
    let textual =
        matches!(by, LogSort::Callsign | LogSort::Address | LogSort::Type | LogSort::Route);
    rows.sort_by(|a, b| {
        if textual {
            let (ta, tb) = (text(a), text(b));
            return match (ta.is_empty(), tb.is_empty()) {
                (false, true) => Ordering::Less,
                (true, false) => Ordering::Greater,
                (true, true) => Ordering::Equal,
                (false, false) => {
                    let o = ta.cmp(&tb);
                    if desc { o.reverse() } else { o }
                }
            };
        }
        match (num(a), num(b)) {
            (Some(x), Some(y)) => {
                let o = x.partial_cmp(&y).unwrap_or(Ordering::Equal);
                if desc { o.reverse() } else { o }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    });
}

/// Below this the two signal columns come off the table. A row that has run out
/// of room prints its columns on top of each other, which is worse than not
/// printing them.
const NARROW_W: f32 = 330.0;

/// Above this there is room for the signal column too.
///
/// It is the sort key with no other home — every other one names a column the
/// table already had — so it is drawn wherever it fits rather than left
/// unreachable, and the divider beside the list is draggable for the widths
/// where it does not.
const WIDE_W: f32 = 430.0;

impl SdroxideApp {
    pub(in crate::app) fn adsb_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: AdsbStatus = match self.adsb_status.as_ref() {
            Some(s) => (**s).clone(),
            None => {
                ui.label(RichText::new(tr!("adsb-starting-ads-b-decoder")).weak());
                return;
            }
        };
        let now = crate::time::now_unix();
        let content_bottom = ui.cursor().top() + panel_h - 26.0;

        self.adsb_header(ui, cmds, &st);
        ui.add_space(3.0);
        self.adsb_routes.poll(ui.ctx());

        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let full_w = ui.available_width();

        // The list keeps a draggable share of the width. The floor is the
        // narrow table (callsign, altitude and age); the ceiling leaves the map
        // enough to still be a radar picture rather than a strip.
        const HANDLE_W: f32 = 7.0;
        // The map in a window of its own leaves the panel to the list.
        let map_out = self.view.adsb_map_out && cfg!(not(target_arch = "wasm32"));
        let list_w = (full_w * self.view.adsb_split_fraction)
            .clamp(240.0, (full_w - HANDLE_W - 200.0).max(240.0));

        ui.horizontal_top(|ui| {
            if map_out || pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(if pane.is_some() || map_out { full_w } else { list_w }, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.adsb_list(ui, &st, now, avail_h),
                );
            }
            if map_out {
                return;
            }
            if pane.is_none() {
                let h = crate::chrome::split_handle(ui, egui::vec2(HANDLE_W, avail_h), None);
                if h.dragged() {
                    self.view.adsb_split_fraction =
                        ((list_w + h.drag_delta().x) / full_w.max(1.0)).clamp(0.15, 0.85);
                }
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| self.adsb_map_pane(ui, &st, now, avail_h, false));
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        if map_out {
            self.adsb_map_window(ui.ctx(), &st, now);
        }
    }

    /// The map in an OS window of its own. Closing the window, or its dock
    /// chip, brings the map back into the panel.
    #[cfg(not(target_arch = "wasm32"))]
    fn adsb_map_window(&mut self, ctx: &egui::Context, st: &AdsbStatus, now: i64) {
        const ID: &str = "ADSB-MAP";
        let mut open = true;
        let mut tool_st = self.take_tool_state(ID);
        tool_st.undocked = true;
        let child = crate::app::undocked::ChildWindow::own(self);
        let title = tr!("adsb-map-window");
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new(ID, &title, [960.0, 680.0]),
            &mut tool_st,
            child,
            &mut open,
            |ui| {
                let h = ui.available_height();
                self.adsb_map_pane(ui, st, now, h, true);
            },
        );
        let docked = !tool_st.undocked;
        tool_st.undocked = true;
        self.put_tool_state(ID, tool_st);
        if !open || docked {
            self.view.adsb_map_out = false;
        }
    }

    /// The switches over the map: cities, night, the coverage diagram with
    /// its window, and the way out into a window of its own.
    fn adsb_map_chips(&mut self, ui: &mut egui::Ui, in_window: bool) {
        let have_home = self.adsb_home().is_some();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let small = |t: String| RichText::new(t).size(9.5);
            if crate::chrome::chip(ui, self.view.adsb_map_cities, small(tr!("adsb-chip-cities")))
                .on_hover_text(tr!("adsb-chip-cities-tip"))
                .clicked()
            {
                self.view.adsb_map_cities = !self.view.adsb_map_cities;
            }
            if crate::chrome::chip(
                ui,
                self.view.adsb_map_airports,
                small(tr!("adsb-chip-airports")),
            )
            .on_hover_text(tr!("adsb-chip-airports-tip"))
            .context_menu_link("https://ourairports.com/data/")
            .clicked()
            {
                self.view.adsb_map_airports = !self.view.adsb_map_airports;
            }
            if crate::chrome::chip(ui, self.view.map_night, small(tr!("panels-night")))
                .on_hover_text(tr!("panels-shade-where-sun-down-twilight"))
                .clicked()
            {
                self.view.map_night = !self.view.map_night;
            }
            if crate::chrome::chip(ui, self.view.adsb_map_pointer, small(tr!("adsb-chip-pointer")))
                .on_hover_text(if have_home {
                    tr!("adsb-chip-pointer-tip")
                } else {
                    tr!("adsb-chip-range-no-home")
                })
                .clicked()
            {
                self.view.adsb_map_pointer = !self.view.adsb_map_pointer;
            }
            let range = self.view.adsb_map_range;
            let tip =
                if have_home { tr!("adsb-chip-range-tip") } else { tr!("adsb-chip-range-no-home") };
            if crate::chrome::chip(ui, range, small(tr!("adsb-chip-range")))
                .on_hover_text(tip)
                .clicked()
            {
                self.view.adsb_map_range = !range;
            }
            if range {
                for (i, label) in
                    [tr!("adsb-range-1h"), tr!("adsb-range-24h"), tr!("adsb-range-7d")]
                        .into_iter()
                        .enumerate()
                {
                    if crate::chrome::chip(ui, self.view.adsb_range_window == i as u8, small(label))
                        .clicked()
                    {
                        self.view.adsb_range_window = i as u8;
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            if !in_window
                && crate::chrome::chip_window_icon(ui, crate::chrome::WindowIcon::Out)
                    .on_hover_text(tr!("adsb-map-out-tip"))
                    .clicked()
            {
                self.view.adsb_map_out = true;
            }
            #[cfg(target_arch = "wasm32")]
            let _ = in_window;
        });
    }

    /// Where the receiver is looking, what the demodulator is finding, and the
    /// one button that fixes the commonest problem.
    ///
    /// Every volatile readout occupies a fixed-width slot, for the reason the
    /// APRS header's do: this is a `horizontal_wrapped`, and a number that grows
    /// a digit tips the whole tail onto a second line and moves every pane below
    /// it. On a busy band these counters change several times a second.
    fn adsb_header(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, st: &AdsbStatus) {
        let dial = self.state.rx_freq_hz();
        let on_channel =
            (dial - sdroxide_types::ADSB_FREQ_HZ).abs() < 100_000.0 && st.unavailable.is_none();

        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(22.0);
            ui.label(RichText::new(tr!("adsb-ads-b")).size(11.0).strong().color(theme::CYAN()));
            ui.label(RichText::new(tr!("adsb-t-1090-mhz-mode-s")).weak().size(10.5));

            // The one preset there is. A chip rather than a note, because the
            // fix for "nothing is decoding" is nearly always this. A left
            // click tunes and sets the gains saved for ADS-B on this device;
            // saving is the right click's menu, so a quick click can never
            // overwrite a good setting with whatever happens to be on.
            // Why: HDZ 2026-11-17: „links store LNA settings (alle Verstärker),
            //      rechts recall … mach so wie du vorgeschlagen hast“.
            let key = self.adsb_gain_key();
            let saved = key.as_ref().and_then(|k| self.view.adsb_gain_presets.get(k)).cloned();
            let now_gains = self.adsb_gain_snapshot();
            let now = crate::time::now_unix();
            // The tooltip says what a click will set and what is on now, so a
            // recall that changes nothing reads as such (HDZ 2026-11-17: „wenn
            // gespeichert muss der Tooltip zeigen, was im Speicher ist … ich
            // sehe keinen Effekt beim Recall“).
            let tip = match &saved {
                Some(g) => {
                    tr!("adsb-tune-gains-tip", gains = fmt_gains(g), now = fmt_gains(&now_gains))
                }
                None => tr!("adsb-tune-gains-none-tip", now = fmt_gains(&now_gains)),
            };
            let resp = crate::chrome::chip(ui, on_channel, "1090.000").on_hover_text(tip);
            if resp.clicked() {
                cmds.push(Command::SetVfo {
                    vfo: self.state.active_vfo,
                    hz: sdroxide_types::ADSB_FREQ_HZ,
                });
                if let Some(g) = &saved {
                    tracing::info!("ADS-B gain preset recalled for {key:?}: {}", fmt_gains(g));
                    self.apply_gains(g, cmds);
                    self.adsb_gain_note = Some((tr!("adsb-gains-recalled"), now));
                }
            }
            resp.context_menu(|ui| {
                ui.label(
                    RichText::new(tr!("adsb-gains-now", gains = fmt_gains(&now_gains)))
                        .monospace()
                        .size(9.5),
                );
                if ui.button(tr!("adsb-gains-save")).clicked() {
                    match (&key, now_gains.is_empty()) {
                        (Some(k), false) => {
                            tracing::info!(
                                "ADS-B gain preset saved for {k}: {}",
                                fmt_gains(&now_gains)
                            );
                            self.view.adsb_gain_presets.insert(k.clone(), now_gains.clone());
                            self.adsb_gain_note = Some((tr!("adsb-gains-saved"), now));
                        }
                        _ => {
                            tracing::warn!(
                                "ADS-B gain preset: nothing to save (device {key:?}, gains {:?})",
                                self.state.gains
                            );
                            self.adsb_gain_note = Some((tr!("adsb-gains-none"), now));
                        }
                    }
                    ui.close();
                }
                if let Some(g) = &saved {
                    ui.label(
                        RichText::new(tr!("adsb-gains-stored", gains = fmt_gains(g)))
                            .monospace()
                            .size(9.5)
                            .weak(),
                    );
                    if ui.button(tr!("adsb-gains-forget")).clicked()
                        && let Some(k) = &key
                    {
                        self.view.adsb_gain_presets.remove(k);
                        ui.close();
                    }
                }
            });
            // A short note beside the chip after a save or a recall.
            if let Some((msg, at)) = &self.adsb_gain_note {
                if now - at < 4 {
                    ui.label(RichText::new(msg).size(10.0).color(theme::GREEN()));
                    crate::repaint::after_ms(ui.ctx(), 500);
                } else {
                    self.adsb_gain_note = None;
                }
            }

            ui.separator();
            slot(ui, 76.0, &tr!("adsb-aircraft-3", aircraft = st.aircraft.len()), theme::CYAN());
            slot(ui, 74.0, &tr!("adsb-frames", frames = count(st.frames)), theme::gray(150));
            // A high preamble count with no frames is the honest picture of a
            // band that is busy with something the decoder cannot read, or of a
            // receiver picking up its own noise. Worth showing rather than
            // leaving the panel looking broken.
            slot(
                ui,
                88.0,
                &tr!("adsb-preambles", preambles = count(st.preambles)),
                theme::gray(120),
            );
            slot(ui, 78.0, &tr!("adsb-bad-crc", bad_crc = count(st.bad_crc)), theme::gray(120));

            if st.window_rate_hz > 0.0 {
                slot(
                    ui,
                    148.0,
                    &format!(
                        "{e6:.3} MHz / {e62:.2} Msps",
                        e6 = st.window_center_hz / 1e6,
                        e62 = st.window_rate_hz / 1e6
                    ),
                    theme::gray(120),
                );
            }

            ui.separator();
            if crate::chrome::chip(ui, self.show_adsb_setup, tr!("adsb-setup"))
                .on_hover_text(tr!("adsb-timeouts-trail-length-how-far"))
                .clicked()
            {
                self.show_adsb_setup = !self.show_adsb_setup;
            }
        });

        if let Some(why) = &st.unavailable {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(why).size(10.5).color(theme::HAZARD()));
                if let Some(hz) = st.suggest_center_hz
                    && (dial - hz).abs() > 1.0
                    && crate::chrome::chip(
                        ui,
                        false,
                        tr!("adsb-tune", mhz = format!("{:.3}", hz / 1e6)),
                    )
                    .clicked()
                {
                    cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
                }
            });
        }
        // Running, but on a stream too narrow to carry the waveform properly.
        // Said out loud because the symptom — a short list — is exactly what a
        // quiet sky looks like, and the fix is one setting away on most
        // receivers.
        if let Some(why) = &st.degraded {
            ui.label(RichText::new(why).size(10.5).color(theme::YELLOW()));
        }
    }

    /// The target list, with the detail card pinned to the bottom of the column.
    ///
    /// Pinned rather than laid out after the list, for the reason the APRS
    /// card is: on a quiet sky it would otherwise sit half way up the panel and
    /// jump every time another aeroplane came into range.
    fn adsb_list(&mut self, ui: &mut egui::Ui, st: &AdsbStatus, now: i64, avail_h: f32) {
        let home = self.adsb_home();
        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(20.0);
            ui.label(RichText::new(tr!("adsb-aircraft")).strong().size(10.5).color(theme::CYAN()));
            ui.add(
                egui::TextEdit::singleline(&mut self.adsb_filter)
                    .hint_text("filter")
                    .desired_width(70.0),
            );
            if crate::chrome::chip(
                ui,
                self.view.adsb_show_log,
                RichText::new(tr!("adsb-chip-log")).size(9.5),
            )
            .on_hover_text(tr!("adsb-chip-log-tip"))
            .clicked()
            {
                self.view.adsb_show_log = !self.view.adsb_show_log;
            }
        });
        if self.view.adsb_show_log {
            self.adsb_log_list(ui, st, avail_h - 26.0);
            return;
        }

        let filter = self.adsb_filter.trim().to_ascii_uppercase();
        let mut rows: Vec<&AdsbAircraft> = st
            .aircraft
            .iter()
            .filter(|a| {
                filter.is_empty()
                    || a.callsign.to_ascii_uppercase().contains(&filter)
                    || a.hex().contains(&filter)
            })
            .collect();
        sort_rows(&mut rows, self.adsb_sort, self.adsb_sort_desc, home);
        // Asked for here, row by row, so only the airlines actually heard are
        // fetched; a row shows its route once the airline's file is in.
        let routes: Vec<Option<Vec<String>>> =
            rows.iter().map(|a| self.adsb_routes.route(&a.callsign)).collect();
        // The same route by place. Asking for the end airports here fetches
        // their files for every row with a route, not only the selected one.
        let mut places: Vec<String> = routes
            .iter()
            .map(|r| match r {
                Some(r) => crate::adsb_route::short_places(r, |c| {
                    self.adsb_routes.airport(c).map(|ap| ap.place)
                }),
                None => String::new(),
            })
            .collect();
        // The airline by name, from the callsign alone, so a row without a
        // known route still says whose aircraft it is; the country from the
        // address block; the type from the standing data, else the class the
        // aircraft sends about itself.
        let extras: Vec<RowExtra> = rows
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let info = self.adsb_routes.aircraft(a.icao);
                let known = info.as_ref().is_some_and(|x| !x.type_code.is_empty());
                RowExtra {
                    places: std::mem::take(&mut places[i]),
                    airline: self.adsb_routes.airline(&a.callsign).unwrap_or_default(),
                    // Unknown airframe: the type the sighting book has seen
                    // on this flight or airline, with a question mark; else
                    // the class the aircraft sends.
                    kind: match info {
                        Some(x) if known => x.type_code,
                        _ => match self.adsb_log.guess(&a.callsign) {
                            Some(g) => format!("{g}?"),
                            None => a.category.clone().unwrap_or_default(),
                        },
                    },
                    kind_known: known,
                    country: self.adsb_routes.country(a.icao),
                }
            })
            .collect();

        let selected = self.adsb_map.selected;
        let card = selected.and_then(|i| st.aircraft.iter().find(|a| a.icao == i));
        // The card takes a draggable share of the column, but only while there
        // is one: with nothing selected the list has the lot and there is no
        // divider to grab.
        const HANDLE_H: f32 = 7.0;
        let card_h = if card.is_some() {
            (avail_h * self.view.adsb_card_fraction).clamp(90.0, (avail_h - 120.0).max(90.0))
        } else {
            0.0
        };
        let list_h =
            (avail_h - card_h - 46.0 - if card.is_some() { HANDLE_H } else { 0.0 }).max(48.0);

        let drop_map_s = self.state.adsb.drop_map_s;
        // Outside the scroll area, so a busy sector does not scroll the column
        // headings off the top of the table it is describing — and so they stay
        // where they can be clicked to re-order it.
        adsb_head_row(ui, home.is_some(), &mut self.adsb_sort, &mut self.adsb_sort_desc);
        let mut pick = None;
        egui::ScrollArea::vertical()
            .id_salt("adsb-aircraft")
            .max_height(list_h)
            .min_scrolled_height(list_h)
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if rows.is_empty() {
                    ui.label(RichText::new(tr!("adsb-nothing-heard-yet")).weak());
                }
                for (i, a) in rows.iter().enumerate() {
                    let route = routes[i].as_deref();
                    let extra = &extras[i];
                    let flags = &mut self.flags;
                    match adsb_row(ui, a, route, extra, flags, now, selected, home, drop_map_s, i) {
                        Some(RowAction::Select) => pick = Some(a.icao),
                        Some(RowAction::Centre) => {
                            if let Some((lat, lon)) = a.lat.zip(a.lon) {
                                self.adsb_map.selected = Some(a.icao);
                                self.adsb_map.view.centre_on(lat, lon);
                            }
                        }
                        None => {}
                    }
                }
            });
        if let Some(icao) = pick {
            self.adsb_map.selected = (selected != Some(icao)).then_some(icao);
        }

        if let Some(a) = card {
            let w = ui.available_width();
            let h = crate::chrome::split_handle(ui, egui::vec2(w, HANDLE_H), None);
            if h.dragged() {
                // Drag up grows the card, which is the half the pointer is
                // heading for.
                self.view.adsb_card_fraction =
                    ((card_h - h.drag_delta().y) / avail_h.max(1.0)).clamp(0.12, 0.8);
            }
            self.adsb_card(ui, a, now, card_h, home);
        }
    }

    /// A day's sightings: the filter row, a summary line, then one row an
    /// aircraft, latest first. A click on one still in the air selects it on
    /// the map.
    ///
    /// Filters: the day (slider over the kept files, right end today), the
    /// panel's text filter (callsign, address, type, route, country), a least
    /// distance, and LIVE — only those in the table now.
    /// Why: HDZ 2026-11-18: „das aircraft log braucht noch ein paar mehr
    ///      filter. Z.B. einen datumsregler“, with ~20 aircraft on the roster.
    fn adsb_log_list(&mut self, ui: &mut egui::Ui, st: &AdsbStatus, h: f32) {
        let live: std::collections::HashSet<u32> = st.aircraft.iter().map(|a| a.icao).collect();
        let today = crate::adsb_log::local_day(crate::time::now_unix());
        let mut days = self.adsb_log.days();
        if days.last() != Some(&today) {
            days.push(today.clone());
        }
        // A day that has aged out, or today chosen by hand, is today: `None`
        // follows midnight, a fixed date would not.
        if self.adsb_log_day.as_ref().is_some_and(|d| *d == today || !days.contains(d)) {
            self.adsb_log_day = None;
        }
        let day = self.adsb_log_day.clone().unwrap_or_else(|| today.clone());
        let is_today = day == today;
        self.adsb_log.open_day(&day);
        let far_out = self
            .adsb_log
            .rows_of(&day)
            .iter()
            .filter(|s| !s.max_km.is_nan())
            .fold(0.0f32, |m, s| m.max(s.max_km));
        ui.horizontal(|ui| {
            if days.len() > 1 {
                let last = days.len() - 1;
                let mut i = days.iter().position(|d| *d == day).unwrap_or(last);
                if ui
                    .add(egui::Slider::new(&mut i, 0..=last).show_value(false))
                    .on_hover_text(tr!("adsb-log-day-tip", n = days.len().to_string()))
                    .changed()
                {
                    self.adsb_log_day = (i < last).then(|| days[i].clone());
                }
            }
            let label = if is_today { tr!("adsb-log-today-word") } else { day_label(&day) };
            ui.label(RichText::new(label).monospace().size(10.0).color(theme::CYAN()));
            if far_out > 0.0 {
                // Up to the day's farthest, in 10 km steps; past it nothing
                // would pass.
                let top = (far_out / 10.0).ceil() * 10.0;
                self.adsb_log_min_km = self.adsb_log_min_km.min(top);
                ui.add_space(8.0);
                ui.add(
                    egui::Slider::new(&mut self.adsb_log_min_km, 0.0..=top)
                        .step_by(10.0)
                        .show_value(false),
                )
                .on_hover_text(tr!("adsb-log-km-tip"));
                let km = if self.adsb_log_min_km > 0.0 {
                    format!("≥{:.0} km", self.adsb_log_min_km)
                } else {
                    tr!("adsb-log-km-any")
                };
                ui.label(RichText::new(km).monospace().size(10.0));
            }
            if is_today
                && crate::chrome::chip(
                    ui,
                    self.adsb_log_live,
                    RichText::new(tr!("adsb-chip-live")).size(9.5),
                )
                .on_hover_text(tr!("adsb-chip-live-tip"))
                .clicked()
            {
                self.adsb_log_live = !self.adsb_log_live;
            }
        });
        let text = self.adsb_filter.trim().to_ascii_uppercase();
        let all = self.adsb_log.rows_of(&day);
        let total = all.len();
        let live_only = is_today && self.adsb_log_live;
        let mut rows: Vec<&crate::adsb_log::Sighting> = all
            .into_iter()
            .filter(|s| {
                crate::adsb_log::passes(s, &text, self.adsb_log_min_km)
                    && (!live_only || live.contains(&s.icao))
            })
            .collect();
        sort_log(&mut rows, self.adsb_log_sort, self.adsb_log_desc);
        // The summary: how many, the farthest, and the commonest
        // destinations and types.
        let top = |keys: Vec<&str>| -> String {
            let mut n: std::collections::HashMap<&str, u32> = Default::default();
            for k in keys.into_iter().filter(|k| !k.is_empty()) {
                *n.entry(k).or_default() += 1;
            }
            let mut v: Vec<_> = n.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
            v.iter().take(5).map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(" · ")
        };
        let dests = top(rows.iter().map(|s| s.route.rsplit('›').next().unwrap_or("")).collect());
        let kinds = top(rows.iter().map(|s| s.kind.as_str()).collect());
        let far =
            rows.iter().filter(|s| !s.max_km.is_nan()).max_by(|a, b| a.max_km.total_cmp(&b.max_km));
        let mut head = if is_today {
            tr!("adsb-log-today", n = total.to_string())
        } else {
            tr!("adsb-log-on-day", day = day_label(&day), n = total.to_string())
        };
        if rows.len() != total {
            head.push_str(&format!(" · {}", tr!("adsb-log-shown", n = rows.len().to_string())));
        }
        if let Some(f) = far {
            let who = if f.call.is_empty() { format!("{:06X}", f.icao) } else { f.call.clone() };
            head.push_str(&format!(
                "   {}",
                tr!("adsb-log-farthest", km = format!("{:.0}", f.max_km), call = who)
            ));
        }
        ui.label(RichText::new(head).monospace().size(10.0).color(theme::CYAN()));
        if !dests.is_empty() {
            ui.label(
                RichText::new(format!("{}  {dests}", tr!("adsb-log-dests")))
                    .monospace()
                    .size(9.5)
                    .weak(),
            );
        }
        if !kinds.is_empty() {
            ui.label(
                RichText::new(format!("{}  {kinds}", tr!("adsb-log-types")))
                    .monospace()
                    .size(9.5)
                    .weak(),
            );
        }
        {
            const L: egui::Align2 = egui::Align2::LEFT_CENTER;
            crate::app::panels::widgets::sort_head_row(
                ui,
                &[
                    (LOG_X[0], L, &tr!("adsb-log-time"), Some(LogSort::Seen)),
                    (LOG_X[1], L, &tr!("adsb-call"), Some(LogSort::Callsign)),
                    (LOG_X[3], L, &tr!("adsb-icao"), Some(LogSort::Address)),
                    (LOG_X[4], L, &tr!("adsb-type-col"), Some(LogSort::Type)),
                    (LOG_X[5], L, &tr!("adsb-route-col"), Some(LogSort::Route)),
                    (LOG_X[6], L, "KM", Some(LogSort::Range)),
                    (LOG_X[7], L, "ALT", Some(LogSort::Altitude)),
                ],
                &mut self.adsb_log_sort,
                &mut self.adsb_log_desc,
            );
        }
        let selected = self.adsb_map.selected;
        let mut pick = None;
        egui::ScrollArea::vertical()
            .id_salt("adsb-log")
            .max_height((h - 64.0).max(60.0))
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if total == 0 && is_today {
                    ui.label(RichText::new(tr!("adsb-log-empty")).weak());
                } else if total == 0 {
                    ui.label(RichText::new(tr!("adsb-log-empty-day")).weak());
                } else if rows.is_empty() {
                    ui.label(RichText::new(tr!("adsb-log-none")).weak());
                }
                for s in &rows {
                    let w = ui.available_width();
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(w, 16.0), egui::Sense::click());
                    if !ui.is_rect_visible(rect) {
                        continue;
                    }
                    let p = ui.painter_at(rect);
                    let on = live.contains(&s.icao);
                    if selected == Some(s.icao) {
                        p.rect_filled(rect, 0.0, theme::gray(34));
                    } else if resp.hovered() {
                        p.rect_filled(rect, 0.0, theme::gray(26));
                    }
                    let ink = if on { theme::CYAN() } else { theme::gray(150) };
                    let dim = theme::gray(120);
                    let font = egui::FontId::monospace(10.0);
                    let y = rect.center().y;
                    let put = |x: f32, t: String, c| {
                        p.text(
                            egui::pos2(rect.left() + x, y),
                            egui::Align2::LEFT_CENTER,
                            t,
                            font.clone(),
                            c,
                        );
                    };
                    put(
                        LOG_X[0],
                        format!(
                            "{}–{}",
                            crate::adsb_log::local_hm(s.first),
                            crate::adsb_log::local_hm(s.last)
                        ),
                        dim,
                    );
                    put(LOG_X[1], s.call.clone(), ink);
                    if !s.country.is_empty() {
                        self.flags.paint(
                            ui.ctx(),
                            &p,
                            &s.country,
                            egui::pos2(rect.left() + LOG_X[2], y),
                            8.0,
                        );
                    }
                    put(LOG_X[3], format!("{:06X}", s.icao), dim);
                    put(LOG_X[4], s.kind.clone(), ink);
                    put(LOG_X[5], s.route.clone(), dim);
                    let km = if s.max_km.is_nan() {
                        String::new()
                    } else {
                        format!("{:>4.0} km", s.max_km)
                    };
                    put(LOG_X[6], km, dim);
                    let fl =
                        s.max_alt_ft.map(|a| format!("F{:03}", (a + 50) / 100)).unwrap_or_default();
                    put(LOG_X[7], fl, dim);
                    if resp.clicked() && on {
                        pick = Some(s.icao);
                    }
                    resp.context_menu(|ui| log_row_menu(ui, s));
                }
            });
        if let Some(icao) = pick {
            self.adsb_map.selected = (selected != Some(icao)).then_some(icao);
        }
    }

    /// Everything one aircraft has said.
    fn adsb_card(
        &mut self,
        ui: &mut egui::Ui,
        a: &AdsbAircraft,
        now: i64,
        h: f32,
        home: Option<(f64, f64)>,
    ) {
        egui::ScrollArea::vertical()
            .id_salt("adsb-card")
            .max_height(h)
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(a.label()).monospace().strong().size(13.0).color(
                        if a.emergency.is_some() { theme::HAZARD() } else { theme::YELLOW() },
                    ));
                    ui.label(RichText::new(a.hex()).monospace().size(10.5).weak());
                    if let Some(c) = &a.category {
                        ui.label(RichText::new(c).size(10.5).color(theme::CYAN_DIM()));
                    }
                    if a.on_ground {
                        ui.label(
                            RichText::new(tr!("adsb-ground")).size(10.5).color(theme::GREEN()),
                        );
                    }
                    if a.lat.is_some() && a.pos_stale(now, self.state.adsb.drop_map_s) {
                        ui.label(
                            RichText::new(tr!("adsb-position-stale"))
                                .size(10.5)
                                .color(theme::HAZARD())
                                .italics(),
                        );
                    }
                });
                if let Some(e) = &a.emergency {
                    ui.label(RichText::new(e).strong().color(theme::HAZARD()));
                }

                let mut rows: Vec<(String, String)> = Vec::new();
                // The usual route of the callsign, airports named where their
                // file is in. Typical, not filed: see `adsb_route`.
                if let Some(route) = self.adsb_routes.route(&a.callsign) {
                    let names: Vec<String> = route
                        .iter()
                        .map(|c| match self.adsb_routes.airport(c) {
                            Some(ap) if !ap.name.is_empty() => format!("{c} {}", ap.name),
                            _ => c.clone(),
                        })
                        .collect();
                    rows.push((tr!("adsb-row-route"), names.join(" → ")));
                }
                // What the standing data knows of the airframe, where it does.
                if let Some(info) = self.adsb_routes.aircraft(a.icao) {
                    for (key, v) in [
                        (tr!("adsb-row-registration"), info.registration),
                        (tr!("adsb-row-type"), info.manufacturer_model),
                        (tr!("adsb-row-operator"), info.operator),
                        (tr!("adsb-row-built"), info.year),
                    ] {
                        if !v.is_empty() {
                            rows.push((key, v));
                        }
                    }
                }
                if let Some((cc, mil)) = self.adsb_routes.country(a.icao) {
                    let v = if mil { format!("{cc} · {}", tr!("adsb-military")) } else { cc };
                    rows.push((tr!("adsb-row-country"), v));
                }
                if let Some(ft) = a.altitude_ft {
                    rows.push((tr!("adsb-row-altitude"), tr!("adsb-ft-barometric", ft = ft)));
                }
                if let Some(ft) = a.gnss_altitude_ft {
                    rows.push((tr!("adsb-gnss-altitude"), format!("{ft} ft")));
                }
                if let Some(kt) = a.ground_speed_kt {
                    rows.push((tr!("adsb-ground-speed"), format!("{kt:.0} kt")));
                }
                if let Some(t) = a.track_deg {
                    rows.push((tr!("adsb-row-track"), format!("{t:.0}°")));
                }
                if a.turn_rate_deg_s.abs() > 0.2 {
                    rows.push((tr!("adsb-row-turning"), format!("{:.1}°/s", a.turn_rate_deg_s)));
                }
                if let Some(v) = a.vertical_rate_fpm {
                    rows.push((tr!("adsb-vertical-rate"), format!("{value:+} ft/min", value = v)));
                }
                // What the aircraft reports from its own systems; each row only
                // once a message carrying it has arrived.
                let av = &a.avionics;
                if let Some(h) = av.heading_deg {
                    rows.push((tr!("adsb-row-heading"), format!("{h:03.0}°")));
                }
                let speeds: Vec<String> = [("IAS", av.ias_kt), ("TAS", av.tas_kt)]
                    .iter()
                    .filter_map(|(k, v)| v.map(|v| format!("{k} {v:.0} kt")))
                    .collect();
                if !speeds.is_empty() {
                    rows.push((tr!("adsb-row-airspeed"), speeds.join(" · ")));
                }
                if let Some(ft) = av.selected_alt_ft {
                    // The field counts in 32 ft steps, so FL350 arrives as
                    // 35 008; what the crew set is the nearest hundred.
                    let ft = (ft as f32 / 100.0).round() as i32 * 100;
                    let src = if av.selected_alt_fms { "FMS" } else { "MCP" };
                    rows.push((tr!("adsb-row-selected-alt"), format!("{ft} ft ({src})")));
                }
                if let Some(h) = av.selected_heading_deg {
                    rows.push((tr!("adsb-row-selected-heading"), format!("{h:03.0}°")));
                }
                if let Some(q) = av.qnh_hpa {
                    // 0.8 hPa steps: 1012.8 is the code nearest the standard
                    // 1013.25, so it reads as STD, the setting above the
                    // transition altitude.
                    let std = if (q - 1013.25).abs() < 0.41 { " (STD)" } else { "" };
                    rows.push(("QNH".to_string(), format!("{q:.1} hPa{std}")));
                }
                if let Some(ap) = av.autopilot {
                    let mut parts = vec![if ap { tr!("adsb-on") } else { tr!("adsb-off") }];
                    parts.extend(av.modes.iter().cloned());
                    if av.tcas == Some(true) {
                        parts.push("TCAS".to_string());
                    }
                    rows.push((tr!("adsb-row-autopilot"), parts.join(" · ")));
                }
                if let Some(v) = av.version {
                    let mut parts = vec![format!("v{v}")];
                    if let Some(n) = av.nac_p {
                        parts.push(format!("NACp {n}"));
                    }
                    if let Some(n) = av.sil {
                        parts.push(format!("SIL {n}"));
                    }
                    rows.push(("ADS-B".to_string(), parts.join(" · ")));
                }
                if a.squawk.is_some() {
                    rows.push((tr!("adsb-row-squawk"), a.fmt_squawk()));
                }
                if let (Some((hlat, hlon)), Some((lat, lon))) = (home, a.lat.zip(a.lon)) {
                    let km = sdroxide_types::distance_km((hlat, hlon), (lat, lon));
                    let bear = sdroxide_types::bearing_deg((hlat, hlon), (lat, lon));
                    rows.push((
                        tr!("adsb-row-range"),
                        tr!(
                            "adsb-range-bearing",
                            km = format!("{km:.0}"),
                            bearing = format!("{bear:.0}")
                        ),
                    ));
                }
                if let Some((lat, lon)) = a.lat.zip(a.lon) {
                    rows.push((tr!("adsb-row-position"), format!("{lat:.4}, {lon:.4}")));
                }
                rows.push((
                    tr!("adsb-row-signal"),
                    format!("{rssi_dbfs:.0} dBFS", rssi_dbfs = a.rssi_dbfs),
                ));
                rows.push((tr!("adsb-row-frames"), format!("{} ({})", a.frames, a.source.label())));
                rows.push((tr!("adsb-first-heard"), fmt_age(now - a.first_at)));

                egui::Grid::new("adsb-card-grid").num_columns(2).spacing([10.0, 1.0]).show(
                    ui,
                    |ui| {
                        for (k, v) in rows {
                            ui.label(RichText::new(k).size(10.0).weak());
                            ui.label(RichText::new(v).monospace().size(10.5));
                            ui.end_row();
                        }
                    },
                );

                ui.horizontal_wrapped(|ui| {
                    if a.has_position()
                        && crate::chrome::chip(
                            ui,
                            false,
                            RichText::new(tr!("adsb-center")).size(10.0),
                        )
                        .on_hover_text(tr!("adsb-put-aircraft-middle-map"))
                        .clicked()
                        && let Some((lat, lon)) = a.lat.zip(a.lon)
                    {
                        self.adsb_map.view.centre_on(lat, lon);
                    }
                });
                if !a.raw_hex.is_empty() {
                    ui.label(
                        RichText::new(&a.raw_hex)
                            .monospace()
                            .size(9.0)
                            .color(theme::gray(110))
                            .weak(),
                    )
                    .on_hover_text(tr!("adsb-last-frame-accepted-aircraft"));
                }
            });
    }

    fn adsb_map_pane(
        &mut self,
        ui: &mut egui::Ui,
        st: &AdsbStatus,
        now: i64,
        h: f32,
        in_window: bool,
    ) {
        let top = ui.cursor().top();
        self.adsb_map_chips(ui, in_window);
        let h = (h - (ui.cursor().top() - top)).max(60.0);
        let home = self.adsb_home();
        let cfg = self.state.adsb;
        // The selected aircraft's route, with every airport whose place is in.
        let route: Vec<crate::adsb_route::Airport> = self
            .adsb_map
            .selected
            .and_then(|i| st.aircraft.iter().find(|a| a.icao == i))
            .and_then(|a| self.adsb_routes.route(&a.callsign))
            .map(|r| r.iter().filter_map(|c| self.adsb_routes.airport(c)).collect())
            .unwrap_or_default();
        let night = self.night_texture(ui.ctx());
        let hours = match self.view.adsb_range_window {
            0 => 1,
            1 => 24,
            _ => crate::adsb_coverage::KEEP_H,
        };
        let coverage =
            if self.view.adsb_map_range { self.adsb_coverage.view(hours, now) } else { None };
        let airports =
            if self.view.adsb_map_airports { self.adsb_routes.map_airports() } else { None };
        let layers = crate::adsb_map::Layers {
            night,
            cities: self.view.adsb_map_cities,
            coverage: coverage.as_ref(),
            pointer: self.view.adsb_map_pointer,
            airports,
        };
        let state = &mut self.adsb_map;
        crate::adsb_map::show(ui, state, &st.aircraft, &route, home, now, cfg, h, layers);
    }

    /// Which device a gain preset belongs to: driver and label, so an RSP1a
    /// and an RTL-SDR keep their own.
    fn adsb_gain_key(&self) -> Option<String> {
        self.caps.as_ref().map(|c| format!("{}:{}", c.driver, c.label))
    }

    /// The receive gains to keep: every gain stage the device offers, its
    /// hardware AGC and its amplifier and range switches, as the engine last
    /// echoed them. Settings that ride the same command but are not gain
    /// (ppm, bias tee, sampling mode) stay out.
    fn adsb_gain_snapshot(&self) -> Vec<(String, f64)> {
        let stages: Vec<&str> = self
            .caps
            .as_ref()
            .map(|c| {
                c.gains
                    .iter()
                    .filter(|g| g.direction == sdroxide_types::Direction::Rx)
                    .map(|g| g.name.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let picked: Vec<(String, f64)> = self
            .state
            .gains
            .iter()
            .filter(|(n, _)| {
                stages.contains(&n.as_str())
                    || n.contains("AGC")
                    || ["AMP", "VGARANGE", "RSPLNA", "SPYAUTOG"].contains(&n.as_str())
            })
            .cloned()
            .collect();
        // A device whose gain names match none of the above still gets its
        // gains kept, less the settings that are not gain.
        if picked.is_empty() {
            const NOT_GAIN: [&str; 6] = ["PPM", "BIAS", "BIAST", "DIRECT", "OFFSET", "DECIM"];
            return self
                .state
                .gains
                .iter()
                .filter(|(n, _)| !NOT_GAIN.iter().any(|x| n.to_ascii_uppercase().contains(x)))
                .cloned()
                .collect();
        }
        picked
    }

    /// Set saved gains: the stages first, the AGC switches last, since a
    /// manual stage write switches some tuners' AGC off.
    fn apply_gains(&mut self, gains: &[(String, f64)], cmds: &mut Vec<Command>) {
        let mut order: Vec<&(String, f64)> = gains.iter().collect();
        order.sort_by_key(|(n, _)| n.contains("AGC"));
        for (name, db) in order {
            match self.state.gains.iter_mut().find(|(n, _)| n == name) {
                Some((_, d)) => *d = *db,
                None => self.state.gains.push((name.clone(), *db)),
            }
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: name.clone(),
                db: *db,
            });
        }
    }

    /// The operator's own position, from the grid in the digital-mode setup.
    ///
    /// The same source the FT8 and APRS maps use, so the three never disagree
    /// about where the station is. `None` until it has been filled in, which is
    /// why the range column and the RANGE sort are conditional.
    pub(in crate::app) fn adsb_home(&self) -> Option<(f64, f64)> {
        let grid = self.digi_cfg_edit.my_grid.trim();
        (!grid.is_empty()).then(|| sdroxide_types::grid_to_latlon(grid)).flatten()
    }
}

impl SdroxideApp {
    /// How the decoder behaves: the two timeouts, the trail length and the
    /// leader-line time.
    ///
    /// Its own window rather than a section of the digimode setup dialog, which
    /// is what every other panel's SETUP chip opens: that one edits a
    /// `DigiConfig` and exists to hold an operator identity and a set of
    /// message templates, and ADS-B has neither a callsign nor anything to say.
    pub(in crate::app) fn adsb_setup_window(
        &mut self,
        ctx: &egui::Context,
        cmds: &mut Vec<Command>,
    ) {
        if !self.show_adsb_setup {
            return;
        }
        let mut open = self.show_adsb_setup;
        // Edited as a copy and diffed at the end, the way the ISM window does:
        // the engine persists whatever arrives and echoes it back in the state,
        // so there is no apply step and no way for the two copies to drift.
        let mut cfg = self.state.adsb;
        let coverage = &mut self.adsb_coverage;
        let resp = egui::Window::new(tr!("adsb-ads-b-setup"))
            .id(crate::layout::salted_id(ctx, "AdsbSetup"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 400.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                egui::Grid::new("adsb-cfg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr!("adsb-drop-map-after"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.drop_map_s).range(2..=600).suffix(" s"),
                        );
                        ui.label(
                            RichText::new(tr!("adsb-without-position-report")).size(9.5).weak(),
                        );
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(
                        RichText::new(tr!("adsb-past-aircraft-comes-off-map")).size(10.0).weak(),
                    );
                    ui.end_row();

                    ui.label(tr!("adsb-drop-list-after"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.drop_list_s)
                                .range(i64::from(cfg.drop_map_s)..=3600)
                                .suffix(" s"),
                        );
                        ui.label(RichText::new(tr!("adsb-nothing-heard-all")).size(9.5).weak());
                    });
                    ui.end_row();

                    ui.label(tr!("adsb-trail-length"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.history_points)
                                .range(0..=sdroxide_types::ADSB_TRACK_MAX as u16)
                                .suffix(tr!("adsb-points")),
                        );
                        ui.label(
                            RichText::new(tr!("adsb-history-dots-behind-each-target"))
                                .size(9.5)
                                .weak(),
                        );
                    });
                    ui.end_row();

                    ui.label(tr!("adsb-speed-vector"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.vector_minutes)
                                .speed(0.1)
                                .range(0.0..=10.0)
                                .suffix(" min"),
                        );
                        ui.label(
                            RichText::new(tr!("adsb-how-far-ahead-leader-line")).size(9.5).weak(),
                        );
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(
                        RichText::new(tr!("adsb-one-minute-usual-radar-convention"))
                            .size(10.0)
                            .weak(),
                    );
                    ui.end_row();

                    ui.label(tr!("adsb-track-most"));
                    ui.add(
                        egui::DragValue::new(&mut cfg.max_aircraft)
                            .range(10..=2000)
                            .suffix(tr!("adsb-aircraft-2")),
                    );
                    ui.end_row();
                });
                ui.separator();
                ui.label(
                    RichText::new(tr!("adsb-aircraft-ground-are-placed-against")).size(10.0).weak(),
                );
                ui.separator();
                // Reset in two clicks, as it throws away up to a week.
                let arm = ui.id().with("adsb-coverage-reset");
                let armed = ui.data(|d| d.get_temp::<bool>(arm)).unwrap_or(false);
                ui.horizontal(|ui| {
                    ui.label(tr!("adsb-coverage-kept", hours = coverage.hours_kept().to_string()));
                    if !armed {
                        if ui
                            .button(tr!("adsb-coverage-reset"))
                            .on_hover_text(tr!("adsb-coverage-reset-tip"))
                            .clicked()
                        {
                            ui.data_mut(|d| d.insert_temp(arm, true));
                        }
                    } else {
                        if ui
                            .button(
                                RichText::new(tr!("adsb-coverage-reset-sure"))
                                    .color(theme::HAZARD()),
                            )
                            .clicked()
                        {
                            coverage.reset();
                            ui.data_mut(|d| d.insert_temp(arm, false));
                        }
                        if ui.button(tr!("adsb-coverage-reset-keep")).clicked() {
                            ui.data_mut(|d| d.insert_temp(arm, false));
                        }
                    }
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        let cfg = cfg.sane();
        if cfg != self.state.adsb {
            cmds.push(Command::SetAdsbConfig(cfg));
        }
        self.show_adsb_setup = open;
    }
}

/// A right-click on a data chip offers the source's page, where its licence
/// is written down; a tooltip cannot hold a link that can be clicked.
trait ContextMenuLink: Sized {
    fn context_menu_link(self, url: &str) -> Self;
}

impl ContextMenuLink for egui::Response {
    fn context_menu_link(self, url: &str) -> Self {
        self.context_menu(|ui| {
            ui.hyperlink_to(tr!("adsb-source-open"), url).on_hover_text(url);
        });
        self
    }
}

/// Gains as one line: `LNA 30 · MIX 10 · AGC 0`.
fn fmt_gains(g: &[(String, f64)]) -> String {
    g.iter().map(|(n, v)| format!("{n} {v:.0}")).collect::<Vec<_>>().join(" · ")
}

/// The column headings, drawn with the same offsets the rows use.
fn adsb_head_row(ui: &mut egui::Ui, have_home: bool, sort: &mut AdsbSort, desc: &mut bool) {
    const L: egui::Align2 = egui::Align2::LEFT_CENTER;
    const R: egui::Align2 = egui::Align2::RIGHT_CENTER;
    let cols = columns(ui.available_width(), have_home);
    crate::app::panels::widgets::sort_head_row(
        ui,
        &[
            (cols.call, L, &tr!("adsb-call"), Some(AdsbSort::Callsign)),
            // The address is the callsign column's fallback, not an order
            // anybody wants a sky in.
            (cols.icao, L, &tr!("adsb-icao"), None),
            (cols.alt, R, "ALT", Some(AdsbSort::Altitude)),
            (cols.spd, R, "GS", Some(AdsbSort::Speed)),
            (cols.route, L, &tr!("adsb-route-col"), None),
            (cols.places, L, &tr!("adsb-places-col"), None),
            (cols.airline, L, &tr!("adsb-airline-col"), None),
            (cols.kind, L, &tr!("adsb-type-col"), None),
            (cols.sig, R, "SIG", Some(AdsbSort::Signal)),
            (cols.trk, R, "TRK", None),
            (cols.vs, R, &tr!("adsb-v-s"), None),
            (cols.sqk, R, "SQK", None),
            (cols.range, R, "KM", Some(AdsbSort::Range)),
            (cols.age, R, "AGE", Some(AdsbSort::Heard)),
        ],
        sort,
        desc,
    );
}

/// Where the route column starts, and the room it needs.
const ROUTE_X: f32 = 206.0;
const ROUTE_W: f32 = 66.0;
/// Where the route by place starts, and the least room worth drawing it in.
const PLACES_X: f32 = ROUTE_X + ROUTE_W + 10.0;
const PLACES_MIN_W: f32 = 90.0;
const PLACES_MAX_W: f32 = 200.0;
/// The room the type gets: `B38M`, or the class it sends, `rotorcraft`.
const KIND_W: f32 = 62.0;
/// Room before the address for its country's flag.
const FLAG_W: f32 = 15.0;
/// The room the airline gets, right of the places: `TUI fly Netherlands`.
const AIRLINE_W: f32 = 116.0;

/// Column x offsets, in points from the left of a row.
struct Cols {
    call: f32,
    icao: f32,
    alt: f32,
    spd: f32,
    route: f32,
    /// Left edge of the route by place, and the room up to the next column.
    places: f32,
    places_w: f32,
    /// Left edge of the airline, and its room.
    airline: f32,
    airline_w: f32,
    /// Left edge of the aircraft type.
    kind: f32,
    sig: f32,
    trk: f32,
    vs: f32,
    sqk: f32,
    range: f32,
    age: f32,
}

/// Where each column sits, given the width available.
///
/// Fixed offsets rather than a layout, so the table reads down as well as
/// across; the trailing columns drop out below [`NARROW_W`] rather than
/// overprinting each other. `f32::NAN` means "not drawn".
fn columns(w: f32, have_home: bool) -> Cols {
    let age = w - 4.0;
    let narrow = w < NARROW_W;
    let range = if have_home && !narrow { age - 34.0 } else { f32::NAN };
    let after_range = if range.is_nan() { age - 34.0 } else { range - 40.0 };
    let sqk = if narrow { f32::NAN } else { after_range };
    let vs = if narrow { f32::NAN } else { sqk - 40.0 };
    let trk = if vs.is_nan() { after_range } else { vs - 44.0 };
    // The last column to appear and the first to go: it is the only one whose
    // heading has to clear the fixed left-hand group, and at [`WIDE_W`] it
    // just does — with or without a range column between it and the edge.
    let sig = if w < WIDE_W { f32::NAN } else { trk - 34.0 };
    // The route sits after the speed, left-aligned, only where the first of the
    // right-hand columns leaves it room for `EDDK›…›GCLP`.
    let first_right = if sig.is_nan() { trk } else { sig };
    let route = if first_right - 28.0 - ROUTE_X >= ROUTE_W { ROUTE_X } else { f32::NAN };
    // Right of the codes: the places, the airline and the type, in whatever
    // is left before the first right-hand column. The places give way first,
    // then the airline; the type is short and stays longest.
    let free = first_right - 34.0 - PLACES_X;
    let none = (f32::NAN, 0.0);
    let (places, airline, kind) = if route.is_nan() {
        (none, none, f32::NAN)
    } else if free >= PLACES_MIN_W + AIRLINE_W + KIND_W + 20.0 {
        // Capped, so airline and type stay beside the places on a wide table
        // instead of drifting over to the right-hand columns.
        let pw = (free - 20.0 - AIRLINE_W - KIND_W).min(PLACES_MAX_W);
        let ax = PLACES_X + pw + 10.0;
        ((PLACES_X, pw), (ax, AIRLINE_W), ax + AIRLINE_W + 10.0)
    } else if free >= AIRLINE_W + KIND_W + 10.0 {
        (none, (PLACES_X, AIRLINE_W), PLACES_X + AIRLINE_W + 10.0)
    } else if free >= KIND_W {
        (none, none, PLACES_X)
    } else {
        (none, none, f32::NAN)
    };
    let ((places, places_w), (airline, airline_w)) = (places, airline);
    Cols {
        call: 5.0,
        icao: 62.0,
        alt: 158.0,
        spd: 196.0,
        route,
        places,
        places_w,
        airline,
        airline_w,
        kind,
        sig,
        trk,
        vs,
        sqk,
        range,
        age,
    }
}

/// What a row shows beyond the aircraft's own messages, looked up by the
/// panel before the rows are drawn.
struct RowExtra {
    /// The route by place, `London › Budapest`.
    places: String,
    airline: String,
    /// The ICAO type code, or the class the aircraft sends when the type is
    /// not known.
    kind: String,
    /// `kind` is a type code rather than a class.
    kind_known: bool,
    /// Country of the address block, ISO 3166 (`DE`), and whether the block
    /// is military.
    country: Option<(String, bool)>,
}

/// What a row asks of the panel.
enum RowAction {
    /// Clicked: select it, or let go of it.
    Select,
    /// "Show on map" from its menu.
    Centre,
}

/// The pages that know an aircraft by its address or callsign, as SDRangel
/// offers them. Links only: nothing of theirs is fetched or kept.
fn aircraft_links(a: &AdsbAircraft) -> Vec<(String, String)> {
    let hex = a.hex();
    let low = hex.to_ascii_lowercase();
    let mut v = vec![
        (tr!("adsb-link-planespotters"), format!("https://www.planespotters.net/hex/{hex}")),
        (tr!("adsb-link-adsbx"), format!("https://globe.adsbexchange.com/?icao={low}")),
        (tr!("adsb-link-adsblol"), format!("https://adsb.lol/?icao={low}")),
        (tr!("adsb-link-opensky"), format!("https://map.opensky-network.org/?icao={low}")),
    ];
    let call = a.callsign.trim();
    if !call.is_empty() {
        v.push((tr!("adsb-link-fr24"), format!("https://www.flightradar24.com/{call}")));
    }
    v
}

/// Pages that still have a flight after it has landed: its track on the day
/// it was logged, and the flight's history by callsign. Unlike the live map
/// links of [`aircraft_links`], each of these answers for a past sighting.
/// Links only: nothing of theirs is fetched or kept.
fn sighting_links(s: &crate::adsb_log::Sighting) -> Vec<(String, String)> {
    let hex = format!("{:06X}", s.icao);
    let low = hex.to_ascii_lowercase();
    let day = crate::adsb_log::utc_day(s.first);
    let mut v = vec![(
        tr!("adsb-link-adsbx-trace", day = day.clone()),
        format!("https://globe.adsbexchange.com/?icao={low}&showTrace={day}"),
    )];
    let call = s.call.trim();
    if !call.is_empty() {
        v.push((
            tr!("adsb-link-flightaware", call = call),
            format!("https://www.flightaware.com/live/flight/{call}"),
        ));
    }
    v.push((tr!("adsb-link-planespotters"), format!("https://www.planespotters.net/hex/{hex}")));
    v
}

/// `Mo 17.11.` for `2026-11-17`: the day the sighting log shows, as its
/// slider and summary name it. The date itself when it does not parse.
fn day_label(day: &str) -> String {
    let Some((wd, d, m)) = crate::adsb_log::day_parts(day) else { return day.to_string() };
    let wd = match wd {
        0 => tr!("adsb-wd-mo"),
        1 => tr!("adsb-wd-tu"),
        2 => tr!("adsb-wd-we"),
        3 => tr!("adsb-wd-th"),
        4 => tr!("adsb-wd-fr"),
        5 => tr!("adsb-wd-sa"),
        _ => tr!("adsb-wd-su"),
    };
    tr!("adsb-log-day-label", wd = wd, d = format!("{d:02}"), m = format!("{m:02}"))
}

/// The right-click menu of a sighting-log row.
fn log_row_menu(ui: &mut egui::Ui, s: &crate::adsb_log::Sighting) {
    let call = s.call.trim();
    if !call.is_empty() && ui.button(tr!("adsb-copy-callsign", call = call)).clicked() {
        ui.ctx().copy_text(call.to_string());
        ui.close();
    }
    let hex = format!("{:06X}", s.icao);
    if ui.button(tr!("adsb-copy-icao", icao = hex.clone())).clicked() {
        ui.ctx().copy_text(hex);
        ui.close();
    }
    ui.separator();
    for (label, url) in sighting_links(s) {
        if ui.button(label).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            ui.close();
        }
    }
}

/// The right-click menu of a row.
fn row_menu(ui: &mut egui::Ui, a: &AdsbAircraft) -> Option<RowAction> {
    let mut out = None;
    let call = a.callsign.trim();
    if !call.is_empty() && ui.button(tr!("adsb-copy-callsign", call = call)).clicked() {
        ui.ctx().copy_text(call.to_string());
        ui.close();
    }
    if ui.button(tr!("adsb-copy-icao", icao = a.hex())).clicked() {
        ui.ctx().copy_text(a.hex());
        ui.close();
    }
    ui.separator();
    for (label, url) in aircraft_links(a) {
        if ui.button(label).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            ui.close();
        }
    }
    ui.separator();
    // Where the type and registration this panel shows come from; an aircraft
    // missing there can be added by anyone.
    if ui.button(tr!("adsb-link-vrs-edit")).on_hover_text(tr!("adsb-link-vrs-edit-tip")).clicked() {
        ui.ctx().copy_text(a.hex());
        ui.ctx()
            .open_url(egui::OpenUrl::new_tab("https://sdm.virtualradarserver.co.uk/Edit/Aircraft"));
        ui.close();
    }
    if a.lat.is_some() && ui.button(tr!("adsb-show-on-map")).clicked() {
        out = Some(RowAction::Centre);
        ui.close();
    }
    out
}

/// One row of the aircraft table. Returns what it asks of the panel.
#[allow(clippy::too_many_arguments)]
fn adsb_row(
    ui: &mut egui::Ui,
    a: &AdsbAircraft,
    route: Option<&[String]>,
    extra: &RowExtra,
    flags: &mut crate::flags::Flags,
    now: i64,
    selected: Option<u32>,
    home: Option<(f64, f64)>,
    drop_map_s: u16,
    i: usize,
) -> Option<RowAction> {
    const ROW_H: f32 = 17.0;
    const ACCENT_W: f32 = 2.5;

    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return None;
    }
    let is_sel = selected == Some(a.icao);
    let stale = a.pos_stale(now, drop_map_s);
    let p = ui.painter_at(rect);

    // A target whose position has aged out is greyed, because it is no longer
    // on the map and a row that looked live would be the only thing saying it
    // was. An emergency outranks everything.
    let (accent, ink) = if a.emergency.is_some() {
        (theme::HAZARD(), theme::HAZARD())
    } else if is_sel {
        (theme::YELLOW(), theme::YELLOW())
    } else if stale {
        (theme::gray(80), theme::gray(115))
    } else if a.on_ground {
        (theme::GREEN(), theme::CYAN())
    } else {
        (theme::CYAN_DIM(), theme::CYAN())
    };
    let dim = if stale { theme::gray(95) } else { theme::gray(140) };
    if is_sel {
        p.rect_filled(rect, 0.0, theme::gray(34));
    }
    p.rect_filled(
        egui::Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.left() + ACCENT_W, rect.bottom()),
        ),
        0.0,
        accent,
    );

    let cols = columns(w, home.is_some());
    let mono = egui::FontId::monospace(10.5);
    let small = egui::FontId::monospace(9.5);
    let y = rect.center().y;
    let put = |x: f32, align: egui::Align2, text: String, font: egui::FontId, c| {
        if x.is_nan() || text.is_empty() {
            return;
        }
        p.text(egui::pos2(rect.left() + x, y), align, text, font, c);
    };

    put(cols.call, egui::Align2::LEFT_CENTER, a.label(), mono.clone(), ink);
    // The address, but only where it is not already the label — repeating it
    // would take a column and say nothing.
    if let Some((cc, _)) = &extra.country {
        flags.paint(ui.ctx(), &p, cc, egui::pos2(rect.left() + cols.icao, y), 8.0);
    }
    if !a.callsign.is_empty() {
        put(cols.icao + FLAG_W, egui::Align2::LEFT_CENTER, a.hex(), small.clone(), dim);
    }
    put(cols.alt, egui::Align2::RIGHT_CENTER, a.fmt_altitude(), mono.clone(), ink);
    put(cols.spd, egui::Align2::RIGHT_CENTER, a.fmt_speed(), mono.clone(), ink);
    put(
        cols.route,
        egui::Align2::LEFT_CENTER,
        route.map(crate::adsb_route::short).unwrap_or_default(),
        small.clone(),
        dim,
    );
    if !cols.places.is_nan() {
        let t = fit(&p, &extra.places, &small, cols.places_w);
        put(cols.places, egui::Align2::LEFT_CENTER, t, small.clone(), dim);
    }
    if !cols.airline.is_nan() {
        let t = fit(&p, &extra.airline, &small, cols.airline_w);
        put(cols.airline, egui::Align2::LEFT_CENTER, t, small.clone(), dim);
    }
    if !cols.kind.is_nan() {
        let t = fit(&p, &extra.kind, &small, KIND_W);
        // A class is a guess at the size, not a type: drawn fainter.
        let c = if extra.kind_known { ink } else { theme::gray(95) };
        put(cols.kind, egui::Align2::LEFT_CENTER, t, small.clone(), c);
    }
    put(cols.sig, egui::Align2::RIGHT_CENTER, format!("{:.0}", a.rssi_dbfs), small.clone(), dim);
    put(
        cols.trk,
        egui::Align2::RIGHT_CENTER,
        a.track_deg.map(|t| format!("{t:03.0}")).unwrap_or_default(),
        small.clone(),
        dim,
    );
    put(
        cols.vs,
        egui::Align2::RIGHT_CENTER,
        a.vertical_rate_fpm.filter(|v| v.abs() >= 64).map(|v| format!("{v:+}")).unwrap_or_default(),
        small.clone(),
        dim,
    );
    put(cols.sqk, egui::Align2::RIGHT_CENTER, a.fmt_squawk(), small.clone(), dim);
    if let (Some(h), Some((lat, lon))) = (home, a.lat.zip(a.lon)) {
        put(
            cols.range,
            egui::Align2::RIGHT_CENTER,
            format!("{:.0}", sdroxide_types::distance_km(h, (lat, lon))),
            small.clone(),
            dim,
        );
    }
    put(cols.age, egui::Align2::RIGHT_CENTER, fmt_age(now - a.last_at), small, dim);

    // One click target, the whole row wide, registered after everything above
    // it — which is what makes the callsign as clickable as the empty space.
    let hit = ui.interact(rect, ui.id().with(("adsb-row", i)), egui::Sense::click());
    let mut tip = a.hex();
    if let Some((cc, mil)) = &extra.country {
        tip.push_str(&format!(" · {cc}"));
        if *mil {
            tip.push_str(&format!(" · {}", tr!("adsb-military")));
        }
    }
    if let Some(c) = &a.category {
        tip.push_str(&format!(" — {c}"));
    }
    let hit = hit.on_hover_text(tip);
    let mut out = hit.clicked().then_some(RowAction::Select);
    hit.context_menu(|ui| {
        if let Some(act) = row_menu(ui, a) {
            out = Some(act);
        }
    });
    out
}

/// `text` cut to `max_w` points in `font`, with `…` where it was cut.
fn fit(p: &egui::Painter, text: &str, font: &egui::FontId, max_w: f32) -> String {
    let width =
        |t: &str| p.layout_no_wrap(t.to_string(), font.clone(), egui::Color32::WHITE).size().x;
    if text.is_empty() || width(text) <= max_w {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut: String = chars.iter().collect::<String>().trim_end().to_string() + "…";
        if width(&cut) <= max_w {
            return cut;
        }
    }
    String::new()
}

/// Order the table. A missing value always sorts last, whichever way the arrow
/// points: an aircraft that has not said its altitude yet is not the lowest one
/// in the sky.
fn sort_rows(rows: &mut [&AdsbAircraft], by: AdsbSort, desc: bool, home: Option<(f64, f64)>) {
    let key = |a: &AdsbAircraft| -> Option<f64> {
        match by {
            AdsbSort::Heard => Some(a.last_at as f64),
            AdsbSort::Callsign => None,
            AdsbSort::Altitude => a.altitude_ft.map(f64::from),
            AdsbSort::Speed => a.ground_speed_kt.map(f64::from),
            AdsbSort::Range => {
                let h = home?;
                let (lat, lon) = a.lat.zip(a.lon)?;
                // Negated so "descending" is nearest-first, which is the way
                // round anybody actually wants a range column.
                Some(-sdroxide_types::distance_km(h, (lat, lon)))
            }
            AdsbSort::Signal => Some(f64::from(a.rssi_dbfs)),
        }
    };
    if by == AdsbSort::Callsign {
        rows.sort_by(|x, y| {
            let o = x.label().cmp(&y.label());
            if desc { o.reverse() } else { o }
        });
        return;
    }
    rows.sort_by(|x, y| {
        match (key(x), key(y)) {
            (Some(a), Some(b)) => {
                let o = a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
                if desc { o.reverse() } else { o }
            }
            // Missing last, both ways round.
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
}

/// A counter, short enough that it cannot outgrow its slot.
///
/// The frame and preamble counts run for as long as the decoder does — an
/// afternoon on a busy sector is millions — and a header readout that grew a
/// digit an hour would eventually push everything beside it off the row.
fn count(n: u64) -> String {
    let m = n as f64;
    if n < 10_000 {
        n.to_string()
    } else if n < 995_000 {
        format!("{:.0}k", m / 1e3)
    } else if n < 999_500_000 {
        format!("{:.1}M", m / 1e6)
    } else {
        format!("{:.0}G", m / 1e9)
    }
}

/// A readout in a slot of fixed width, so a number that grows a digit cannot
/// re-flow the header.
fn slot(ui: &mut egui::Ui, w: f32, text: &str, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 16.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    ui.painter_at(rect).text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::monospace(10.0),
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ac(icao: u32, call: &str, alt: Option<i32>, last: i64) -> AdsbAircraft {
        let mut a = AdsbAircraft::new(icao, last);
        a.callsign = call.to_string();
        a.altitude_ft = alt;
        a.last_at = last;
        a
    }

    /// An aircraft that has not reported an altitude yet is not the lowest one
    /// in the sky — it sorts last whichever way the column is pointing.
    #[test]
    fn a_missing_value_sorts_last_in_both_directions() {
        let a = ac(1, "AAA", Some(35_000), 10);
        let b = ac(2, "BBB", None, 20);
        let c = ac(3, "CCC", Some(3_000), 30);
        for desc in [true, false] {
            let mut rows = vec![&a, &b, &c];
            sort_rows(&mut rows, AdsbSort::Altitude, desc, None);
            assert_eq!(rows.last().unwrap().icao, 2, "desc={desc}");
        }
    }

    /// Nearest first is what a range column is for, so "descending" — the
    /// default direction for every other column — has to mean nearest here.
    #[test]
    fn the_range_column_puts_the_nearest_aircraft_at_the_top() {
        let mut near = ac(1, "NEAR", None, 0);
        near.lat = Some(48.3);
        near.lon = Some(16.4);
        let mut far = ac(2, "FAR", None, 0);
        far.lat = Some(52.0);
        far.lon = Some(4.0);
        let mut rows = vec![&far, &near];
        sort_rows(&mut rows, AdsbSort::Range, true, Some((48.2, 16.37)));
        assert_eq!(rows[0].icao, 1);
    }

    /// The header counters run all session; they must not grow.
    #[test]
    fn a_counter_never_outgrows_its_slot() {
        for n in [0u64, 9_999, 10_000, 999_999, 1_000_000, 999_999_999, 42_000_000_000] {
            assert!(count(n).len() <= 6, "{n} formatted as {}", count(n));
        }
        assert_eq!(count(1_234), "1234");
        assert_eq!(count(12_345), "12k");
        assert_eq!(count(999_999_999), "1G");
        assert_eq!(count(4_500_000), "4.5M");
    }

    /// The columns must not overlap at any width the panel can be dragged to,
    /// and the trailing ones drop out rather than printing on top of each other.
    #[test]
    fn the_log_sorts_by_any_column_and_puts_the_unknown_last() {
        use crate::adsb_log::Sighting;
        let mk = |icao: u32, call: &str, km: f32, last: i64| Sighting {
            icao,
            call: call.into(),
            kind: String::new(),
            route: String::new(),
            country: String::new(),
            first: last - 60,
            last,
            min_km: km,
            max_km: km,
            max_alt_ft: None,
        };
        let (a, b, c) =
            (mk(1, "KLM1", 48.0, 300), mk(2, "", f32::NAN, 200), mk(3, "BAW8", 22.0, 100));
        let calls = |v: &[&Sighting]| v.iter().map(|s| s.icao).collect::<Vec<_>>();
        let mut v = vec![&a, &b, &c];
        sort_log(&mut v, LogSort::Range, true);
        assert_eq!(calls(&v), [1, 3, 2]);
        sort_log(&mut v, LogSort::Range, false);
        assert_eq!(calls(&v), [3, 1, 2]);
        sort_log(&mut v, LogSort::Callsign, false);
        assert_eq!(calls(&v), [3, 1, 2]);
        sort_log(&mut v, LogSort::Callsign, true);
        assert_eq!(calls(&v), [1, 3, 2]);
        sort_log(&mut v, LogSort::Seen, true);
        assert_eq!(calls(&v), [1, 2, 3]);
    }

    /// A logged flight's links answer for the day it was logged — the trace
    /// on that UTC day — and the callsign's history only where there is a
    /// callsign.
    #[test]
    fn a_sighting_links_to_its_own_day_and_its_flight() {
        let s = crate::adsb_log::Sighting {
            icao: 0x3C_6444,
            call: "DLH400".into(),
            kind: String::new(),
            route: String::new(),
            country: String::new(),
            // 2026-11-16 23:30 UTC: the local day may already be the 17th.
            first: 1_794_871_800,
            last: 1_794_872_400,
            min_km: f32::NAN,
            max_km: f32::NAN,
            max_alt_ft: None,
        };
        let urls: Vec<String> = sighting_links(&s).into_iter().map(|(_, u)| u).collect();
        assert_eq!(urls[0], "https://globe.adsbexchange.com/?icao=3c6444&showTrace=2026-11-16");
        assert_eq!(urls[1], "https://www.flightaware.com/live/flight/DLH400");
        assert!(urls[2].ends_with("/hex/3C6444"));
        let anon = crate::adsb_log::Sighting { call: String::new(), ..s };
        assert!(sighting_links(&anon).iter().all(|(_, u)| !u.contains("flightaware")));
    }

    #[test]
    fn places_airline_and_type_end_before_the_right_hand_columns() {
        let mut w = 200.0f32;
        while w < 1600.0 {
            for have_home in [true, false] {
                let c = columns(w, have_home);
                let first_right = if c.sig.is_nan() { c.trk } else { c.sig };
                // The right-hand columns are right-aligned: their text starts
                // up to about 28 points left of their x.
                let limit = first_right - 28.0;
                for (x, cw) in [(c.places, c.places_w), (c.airline, c.airline_w), (c.kind, KIND_W)]
                {
                    if !x.is_nan() {
                        assert!(
                            x + cw <= limit,
                            "column at {x} (+{cw}) runs into {first_right} at width {w}"
                        );
                    }
                }
                if !c.places.is_nan() && !c.airline.is_nan() {
                    assert!(
                        c.places + c.places_w < c.airline,
                        "places run into the airline at {w}"
                    );
                }
                if !c.airline.is_nan() && !c.kind.is_nan() {
                    assert!(c.airline + c.airline_w < c.kind, "airline runs into the type at {w}");
                }
            }
            w += 7.0;
        }
    }

    #[test]
    fn the_columns_never_overprint_however_narrow_the_panel_gets() {
        let mut w = 200.0f32;
        while w < 900.0 {
            for have_home in [true, false] {
                let c = columns(w, have_home);
                let mut xs: Vec<f32> = [c.trk, c.vs, c.sqk, c.range, c.age]
                    .into_iter()
                    .filter(|x| !x.is_nan())
                    .collect();
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                for pair in xs.windows(2) {
                    assert!(
                        pair[1] - pair[0] >= 28.0,
                        "columns {pair:?} collide at width {w} (home={have_home})"
                    );
                }
                if w < NARROW_W {
                    assert!(c.sqk.is_nan() && c.vs.is_nan(), "narrow panels drop the tail");
                }
            }
            w += 7.0;
        }
    }
}
