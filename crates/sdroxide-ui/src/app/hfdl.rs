//! The HFDL ground-network decoder's window.
//!
//! The choice the other not-a-mode lanes put in a window — enable, tune, then
//! read the decode log — but for one channel instead of a sketch map: HFDL
//! ground stations are spread across the shortwave band one channel each, so
//! the panel is a frequency, a run switch, and the rolling log of what the
//! channel decoded. State comes from [`sdroxide_types::HfdlStatus`] (the
//! engine's re-sent snapshot), settings from [`sdroxide_types::HfdlSettings`]
//! edited as a copy and diffed at the end — the same apply-by-diff convention
//! the AIS and ADS-B windows keep, because the engine echoes the accepted
//! config back and the two copies would otherwise drift.
//!
//! The window is split: the decode log on the left, and the aircraft map on the
//! right — every position a downlink has carried, kept per aircraft by
//! [`crate::hfdl_map`]. The divider is the same draggable handle the ADS-B
//! panel uses, and where it sits is remembered with the rest of the view state.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{
    Command, HfdlAcars, HfdlAircraft, HfdlDecode, HfdlStation, HfdlStatus, Mode,
    hfdl_acars_label_name, hfdl_plan_khz, hfdl_stations_on, hfdl_working_khz,
};

use crate::app::SdroxideApp;

/// The HFDL assigned-frequency plan, in kHz — the channels the ground-network
/// stations transmit on, ARINC/AviaSat's published allocation. Riverhead's
/// 21 931 kHz primary is first: it is the default channel and the reference
/// off-air capture that validated the decoder was recorded on it.
const HFDL_PLAN_KHZ: &[u32] = &[
    21_931, 2_941, 2_992, 3_455, 3_917, 5_451, 6_562, 6_640, 8_843, 10_060, 10_084, 11_184, 11_387,
    13_306, 13_309, 13_312, 13_315, 13_318, 17_904, 17_910, 17_913, 17_916, 17_919, 17_922, 17_925,
    21_934, 21_940,
];

/// The HFDL panel: run switch, channel, and the split decode log / aircraft
/// map — docked under the waterfall like the other lane panels (ADS-B, AIS).
impl SdroxideApp {
    pub(in crate::app) fn hfdl_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let now = crate::time::now_unix();
        // Edited as a copy and diffed at the end, the way the AIS panel does: the
        // engine echoes the accepted settings back, so there is no apply step and
        // no way for the two copies to drift.
        let mut cfg = self.state.hfdl;
        // The panel asks for `panel_h`, but the space the parent actually gave
        // can be shorter (the split's arithmetic does not see every margin the
        // frame adds), so the log must end at the rect it was given, not at the
        // requested height — otherwise its last rows hang below the window.
        let content_bottom = (ui.cursor().top() + panel_h - 4.0).min(ui.max_rect().bottom() - 4.0);

        hfdl_status_strip(ui, self.hfdl_status.as_ref(), self.hfdl_map.len());
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            if crate::chrome::chip_enabled(ui, true, cfg.enabled, &tr!("hfdl-listen"))
                .on_hover_text(tr!("hfdl-switches-engine-s-hfdl-downconverter"))
                .clicked()
            {
                cfg.enabled = !cfg.enabled;
            }
            let mut khz = (cfg.frequency_hz / 1e3).round();
            let old_khz = khz;
            ui.add(
                egui::DragValue::new(&mut khz).range(2_800.0..=30_000.0).speed(1.0).suffix(" kHz"),
            )
            .on_hover_text(tr!("hfdl-channel-assigned-frequency-listen-anything"));
            if khz != old_khz {
                cfg.frequency_hz = khz * 1e3;
            }
            crate::chrome::row_tail(ui, |ui| {
                // What SAVE writes follows the tab: the messages tab saves the
                // ACARS messages as a CSV (the same columns' worth of detail
                // the ACARS pane saves), every other tab the decode log.
                if self.view.hfdl_tab == HFDL_TAB_MESSAGES {
                    let msgs = self.hfdl_status.as_ref().map(|s| s.acars.as_slice()).unwrap_or(&[]);
                    crate::app::panels::save_text_chip(
                        ui,
                        !msgs.is_empty(),
                        "sdroxide-hfdl-acars.csv",
                        &tr!("hfdl-save-hfdl-acars-messages-csv"),
                        || crate::app::save_text::hfdl_acars_csv(msgs),
                    );
                } else {
                    let log = self.hfdl_status.as_ref().map(|s| s.log.as_slice()).unwrap_or(&[]);
                    crate::app::panels::save_text_chip(
                        ui,
                        !log.is_empty(),
                        "sdroxide-hfdl-log.txt",
                        &tr!("hfdl-save-hfdl-decode-log-file"),
                        || crate::app::save_text::hfdl_log_text(log),
                    );
                }
            });
        });
        ui.add_space(4.0);

        // The plan as pick chips, so a spot of a few kHz on the band is a
        // single click away rather than a DragValue spin. The channels are the
        // network's own once its system table is known (received over the air,
        // or kept from last time), the published plan until then. A channel a
        // ground station is known to be working lights green: heard on, or
        // listed in a fresh squitter's "in use" bitmap. The operator's own
        // saved channels follow, then a "+" to save the one in force — the
        // busy assigned frequencies that are not in the plan.
        let stations: &[HfdlStation] =
            self.hfdl_status.as_ref().map(|s| s.stations.as_slice()).unwrap_or(&[]);
        let plan = hfdl_plan_khz(stations, HFDL_PLAN_KHZ);
        let working = hfdl_working_khz(stations, now);
        let mut remove_khz: Option<u32> = None;
        let mut add_now = false;
        ui.horizontal_wrapped(|ui| {
            for &khz in &plan {
                let on = (cfg.frequency_hz / 1e3).round() as u32 == khz;
                let live = working.contains(&khz);
                let text = RichText::new(format!("{:.3} M", khz as f32 / 1e3));
                let text = if live { text.color(crate::theme::GREEN()) } else { text };
                if crate::chrome::chip(ui, on, text)
                    .on_hover_text(hfdl_chip_hint(stations, khz, live, now))
                    .clicked()
                {
                    cfg.frequency_hz = f64::from(khz) * 1e3;
                }
            }
            let cur_khz = (cfg.frequency_hz / 1e3).round() as u32;
            for &khz in &self.hfdl_channels {
                let resp =
                    crate::chrome::chip(ui, cur_khz == khz, format!("{:.3} M", khz as f32 / 1e3))
                        .on_hover_text(tr!("hfdl-one-your-saved-channels-click"));
                if resp.clicked() {
                    cfg.frequency_hz = f64::from(khz) * 1e3;
                }
                if resp.secondary_clicked() {
                    remove_khz = Some(khz);
                }
            }
            let already = self.hfdl_channels.contains(&cur_khz) || plan.contains(&cur_khz);
            let add = crate::chrome::chip_enabled(ui, !already, false, "+");
            let add = if already {
                add.on_disabled_hover_text(tr!("hfdl-khz-already-has-chip", cur_khz = cur_khz))
            } else {
                add.on_hover_text(tr!("hfdl-save-current-channel-khz-one", cur_khz = cur_khz))
            };
            if add.clicked() {
                add_now = true;
            }
        });
        if let Some(khz) = remove_khz {
            self.hfdl_channels.retain(|&k| k != khz);
            crate::app::persist::persist_hfdl_channels(&self.hfdl_channels);
        }
        if add_now {
            let cur_khz = (cfg.frequency_hz / 1e3).round() as u32;
            if !plan.contains(&cur_khz) && add_hfdl_channel(&mut self.hfdl_channels, cur_khz) {
                crate::app::persist::persist_hfdl_channels(&self.hfdl_channels);
            }
        }

        // Selecting the mode builds the lane and lets the waterfall and the
        // level meter run whether or not the decoder is switched on, so a
        // station can sit on a real channel with a live picture and get nothing
        // — and the only other clue is a small word in the status strip. Say it
        // plainly, where the controls that fix it are (reported on issue #497:
        // "bons signaux, aucune décode").
        if !cfg.enabled {
            let ink = crate::theme::HAZARD();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("⚠").size(13.0).color(ink));
                ui.label(
                    RichText::new(tr!("hfdl-hfdl-selected-but-decoding-off")).size(11.5).color(ink),
                );
            });
            ui.add_space(4.0);
        }

        ui.separator();

        // The log keeps a draggable share of the width; the rest is the map, as
        // in the ADS-B panel. Floors so neither pane can be dragged away to
        // nothing. On a phone the pane chips choose one or the other.
        let avail_h = (content_bottom - ui.cursor().top()).max(90.0);
        let pane = self.phone_pane(ui, Mode::Hfdl);
        let full_w = ui.available_width();
        const HANDLE_W: f32 = 7.0;
        let log_w = (full_w * self.view.hfdl_split_fraction)
            .clamp(220.0, (full_w - HANDLE_W - 180.0).max(220.0));

        ui.horizontal_top(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(if pane.is_some() { full_w } else { log_w }, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if let Some(khz) = self.hfdl_left_pane(ui, now, cfg.frequency_hz) {
                            cfg.frequency_hz = f64::from(khz) * 1e3;
                        }
                    },
                );
            }
            if pane.is_none() {
                let h = crate::chrome::split_handle(ui, egui::vec2(HANDLE_W, avail_h), None);
                if h.dragged() {
                    self.view.hfdl_split_fraction =
                        ((log_w + h.drag_delta().x) / full_w.max(1.0)).clamp(0.15, 0.85);
                }
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| self.hfdl_map_pane(ui, now, avail_h));
            }
        });

        // A channel change moves the dial with it, so the panadapter keeps
        // showing what the lane is decoding; the lane follows its own frequency
        // either way.
        if cfg.frequency_hz != self.state.hfdl.frequency_hz {
            cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz: cfg.frequency_hz });
        }
        if cfg != self.state.hfdl {
            cmds.push(Command::SetHfdlConfig(cfg));
        }
    }

    /// The left pane: the decode log or the ground stations, by the chips in
    /// its header. Returns a channel (kHz) the operator clicked in the stations
    /// list, for the panel to tune to.
    fn hfdl_left_pane(&mut self, ui: &mut egui::Ui, now: i64, cur_hz: f64) -> Option<u32> {
        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(20.0);
            for (tab, label) in [
                (HFDL_TAB_DECODES, tr!("hfdl-decodes")),
                (HFDL_TAB_STATIONS, tr!("hfdl-stations")),
                (HFDL_TAB_MESSAGES, tr!("hfdl-messages")),
                (HFDL_TAB_AIRCRAFT, tr!("hfdl-aircraft-list")),
            ] {
                if crate::chrome::chip(ui, self.view.hfdl_tab == tab, label.to_string()).clicked() {
                    self.view.hfdl_tab = tab;
                }
            }
            if matches!(self.view.hfdl_tab, HFDL_TAB_DECODES | HFDL_TAB_MESSAGES) {
                ui.add(
                    egui::TextEdit::singleline(&mut self.hfdl_filter)
                        .hint_text("filter")
                        .desired_width(90.0),
                );
            }
        });
        match self.view.hfdl_tab {
            HFDL_TAB_STATIONS => self.hfdl_stations_pane(ui, now, (cur_hz / 1e3).round() as u32),
            HFDL_TAB_MESSAGES => {
                self.hfdl_messages_pane(ui);
                None
            }
            HFDL_TAB_AIRCRAFT => {
                self.hfdl_aircraft_pane(ui, now);
                None
            }
            _ => {
                self.hfdl_log_pane(ui);
                None
            }
        }
    }

    /// The ground stations the decoder knows of: when each was last heard and
    /// how strongly, and its channels — green where it is working them now. A
    /// channel is a click away from being tuned.
    fn hfdl_stations_pane(&self, ui: &mut egui::Ui, now: i64, cur_khz: u32) -> Option<u32> {
        let stations: &[HfdlStation] =
            self.hfdl_status.as_ref().map(|s| s.stations.as_slice()).unwrap_or(&[]);
        let room = ui.available_height().max(48.0);
        let mut picked = None;
        egui::ScrollArea::vertical()
            .id_salt("hfdl-stations")
            .max_height(room)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if stations.is_empty() {
                    ui.label(
                        RichText::new(tr!("hfdl-no-station-heard-yet"))
                            .size(10.5)
                            .color(crate::theme::gray(120)),
                    );
                    return;
                }
                for s in hfdl_station_order(stations) {
                    if let Some(khz) = hfdl_station_row(ui, s, now, cur_khz) {
                        picked = Some(khz);
                    }
                }
            });
        picked
    }

    /// Every aircraft the decoder has heard from, as a table — including the
    /// ones that never sent a position, which the map cannot show. The header
    /// sorts it. The registration blinks the aircraft's plot on the map; the
    /// arrow at the start of a row filters the decode log to that aircraft.
    fn hfdl_aircraft_pane(&mut self, ui: &mut egui::Ui, now: i64) {
        let home = self.hfdl_home();
        let fleet: &[HfdlAircraft] =
            self.hfdl_status.as_ref().map(|s| s.aircraft.as_slice()).unwrap_or(&[]);
        let (col, asc) = (self.view.hfdl_aircraft_sort, self.view.hfdl_aircraft_asc);
        let room = ui.available_height().max(48.0);
        let mut act = AircraftAction::None;
        if fleet.is_empty() {
            ui.label(
                RichText::new(tr!("hfdl-no-aircraft-heard-yet"))
                    .size(10.5)
                    .color(crate::theme::gray(120)),
            );
            return;
        }
        // Sideways too: the table is wider than the half of the window the
        // log pane gets, and a cut-off column is worse than a scroll bar.
        egui::ScrollArea::both()
            .id_salt("hfdl-aircraft")
            .max_height(room)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("hfdl-aircraft-grid")
                    .num_columns(10)
                    .spacing([10.0, 3.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(c) = hfdl_aircraft_header(ui, col, asc) {
                            act = AircraftAction::Sort(c);
                        }
                        ui.end_row();
                        for a in hfdl_aircraft_sorted(fleet, col, asc, home) {
                            let plotted =
                                a.map_key.as_deref().is_some_and(|k| self.hfdl_map.has(k));
                            let selected = plotted && self.hfdl_map.selected == a.map_key;
                            let next = hfdl_aircraft_row(ui, a, now, home, plotted, selected);
                            if !matches!(next, AircraftAction::None) {
                                act = next;
                            }
                            ui.end_row();
                        }
                    });
            });
        match act {
            AircraftAction::Blink(key) => {
                let t = ui.input(|i| i.time);
                self.hfdl_map.blink(&key, t);
            }
            AircraftAction::Filter(text) => {
                self.hfdl_filter = text;
                self.view.hfdl_tab = HFDL_TAB_DECODES;
            }
            AircraftAction::Sort(c) => {
                // The same column again flips the direction; a new one starts
                // where that column's first look is most useful — newest first
                // for time, A to Z for names, biggest first for counts.
                if self.view.hfdl_aircraft_sort == c {
                    self.view.hfdl_aircraft_asc = !self.view.hfdl_aircraft_asc;
                } else {
                    self.view.hfdl_aircraft_sort = c;
                    self.view.hfdl_aircraft_asc = matches!(
                        c,
                        AIRCRAFT_COL_REG
                            | AIRCRAFT_COL_FLIGHT
                            | AIRCRAFT_COL_GS
                            | AIRCRAFT_COL_DIST
                    );
                }
            }
            AircraftAction::None => {}
        }
    }

    /// The ACARS messages the data link carried, newest first: who sent it,
    /// what kind it is, what the application layer made of it, and the text.
    fn hfdl_messages_pane(&mut self, ui: &mut egui::Ui) {
        let filter = self.hfdl_filter.trim().to_ascii_uppercase();
        let msgs = self.hfdl_status.as_ref().map(|s| s.acars.as_slice()).unwrap_or(&[]);
        let room = ui.available_height().max(48.0);
        egui::ScrollArea::vertical()
            .id_salt("hfdl-messages")
            .max_height(room)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut shown = 0;
                for d in msgs {
                    if !filter.is_empty() && !hfdl_matches(d, &filter) {
                        continue;
                    }
                    hfdl_message_row(ui, d);
                    shown += 1;
                }
                if shown == 0 {
                    let text = if msgs.is_empty() {
                        tr!("hfdl-no-acars-yet")
                    } else {
                        tr!("hfdl-no-decode-matches-filter")
                    };
                    ui.label(RichText::new(text).size(10.5).color(crate::theme::gray(120)));
                }
            });
    }

    /// The decode log, newest first, with a filter over it.
    fn hfdl_log_pane(&mut self, ui: &mut egui::Ui) {
        let filter = self.hfdl_filter.trim().to_ascii_uppercase();
        // The log is carried whole in every status snapshot, newest first —
        // shown in that order, as the manual says, with the latest at the top.
        let log = self.hfdl_status.as_ref().map(|s| s.log.as_slice()).unwrap_or(&[]);
        // What is left under the header, measured rather than assumed: the
        // header's real height is a font and a touch-metric question.
        let room = ui.available_height().max(48.0);
        egui::ScrollArea::vertical()
            .id_salt("hfdl-log")
            .max_height(room)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut shown = 0;
                for d in log {
                    if !filter.is_empty() && !hfdl_matches(d, &filter) {
                        continue;
                    }
                    hfdl_log_row(ui, d);
                    shown += 1;
                }
                if shown == 0 {
                    let text = if log.is_empty() {
                        tr!("hfdl-nothing-decoded-yet-ground-station")
                    } else {
                        tr!("hfdl-no-decode-matches-filter")
                    };
                    ui.label(RichText::new(text).size(10.5).color(crate::theme::gray(120)));
                }
            });
    }

    /// The aircraft map, fed by the app's plot table.
    fn hfdl_map_pane(&mut self, ui: &mut egui::Ui, now: i64, avail_h: f32) {
        let home = self.hfdl_home();
        crate::hfdl_map::show(ui, &mut self.hfdl_map, home, now, avail_h);
    }

    /// The operator's own position, from the grid in the digital-mode setup —
    /// the same source the FT8, APRS and ADS-B maps use, so they never
    /// disagree about where the station is.
    fn hfdl_home(&self) -> Option<(f64, f64)> {
        let grid = self.digi_cfg_edit.my_grid.trim();
        (!grid.is_empty()).then(|| sdroxide_types::grid_to_latlon(grid)).flatten()
    }
}

/// The decoder's own state, as fixed-width slots so nothing reflows the header
/// when a number grows a digit. The aircraft count is the map's table, not the
/// status's: it outlives the log's rolling window.
///
/// One row. The slots were always sized to sit side by side, but the panel's
/// layout is vertical, so each allocation took a line of its own and the strip
/// came out as five short lines with the width of the panel empty beside them.
/// Wrapped here rather than at the call site so the function keeps its own
/// shape wherever it is used; `horizontal_wrapped` rather than `horizontal` so
/// a panel too narrow for all five folds them instead of running them off the
/// edge.
fn hfdl_status_strip(ui: &mut egui::Ui, status: Option<&HfdlStatus>, aircraft: usize) {
    ui.horizontal_wrapped(|ui| hfdl_status_slots(ui, status, aircraft));
}

fn hfdl_status_slots(ui: &mut egui::Ui, status: Option<&HfdlStatus>, aircraft: usize) {
    let (running, level, bursts, decodes) = match status {
        Some(s) => (s.running, Some(s.level_dbfs), s.bursts, s.decodes),
        None => (false, None, 0, 0),
    };
    // "OFF" is amber, not grey: with the lane building the waterfall and the
    // level meter regardless, an idle-looking word here was the only thing
    // saying the decoder was not running, and it read as "nothing on the
    // channel" instead of "not switched on" (issue #497). The word itself says
    // which: DECODING rather than a bare RUNNING/OFF.
    let (run_text, run_ink) = if running {
        (tr!("hfdl-decoding"), crate::theme::GREEN())
    } else {
        (tr!("hfdl-decoding-off"), crate::theme::HAZARD())
    };
    slot(ui, 108.0, &run_text, run_ink);
    let level_text = match level {
        Some(l) if l > -90.0 => format!("{l:.0} dBFS", l = l),
        _ => tr!("hfdl-dbfs").to_string(),
    };
    slot(ui, 76.0, &level_text, crate::theme::gray(150));
    slot(ui, 76.0, &tr!("hfdl-aircraft", aircraft = count(aircraft as u64)), crate::theme::CYAN());
    slot(ui, 64.0, &tr!("hfdl-bursts", bursts = count(bursts)), crate::theme::gray(150));
    slot(ui, 64.0, &tr!("hfdl-decodes-2", decodes = count(decodes)), crate::theme::gray(120));
}

/// One decoded event, as a row of the log: time, kind, the ground station it
/// names, the channel and the burst's signal report — then the payload, which
/// for a position record is the fix itself, and otherwise the parser's fields.
fn hfdl_log_row(ui: &mut egui::Ui, d: &HfdlDecode) {
    let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(d.unix);
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("{h:02}:{mi:02}:{s:02}"))
                .size(10.5)
                .color(crate::theme::gray(120)),
        );
        ui.label(RichText::new(&d.kind).size(10.5).color(crate::theme::CYAN()));
        if let Some(gs) = &d.gs {
            ui.label(RichText::new(gs).size(10.5).strong());
        }
        ui.label(
            RichText::new(format!("{e3:.3} MHz", e3 = d.freq_khz as f32 / 1e3))
                .size(10.5)
                .color(crate::theme::gray(150)),
        );
        if let Some(snr) = d.snr_db {
            ui.label(RichText::new(format!("{snr:.0} dB")).size(10.5).color(if snr >= 12.0 {
                crate::theme::GREEN()
            } else {
                crate::theme::YELLOW()
            }));
        }
        if let Some(fec) = d.fec_corrected.filter(|f| *f > 0) {
            ui.label(
                RichText::new(tr!("hfdl-fixed", fec = count(u64::from(fec))))
                    .size(10.5)
                    .color(crate::theme::gray(120)),
            );
        }
        // The position, where there is one, as the fix itself rather than the
        // JSON it arrived in.
        if let Some(fix) = &d.position {
            ui.label(
                RichText::new(format!("{:.4}, {:.4}", fix.lat, fix.lon))
                    .size(10.5)
                    .color(crate::theme::CYAN()),
            );
            ui.label(RichText::new(fix.label()).size(10.5).color(crate::theme::gray(150)));
        } else if let Some(a) = &d.acars {
            hfdl_acars_summary(ui, a);
        } else if !d.details.is_empty() && d.details != "null" {
            ui.label(
                RichText::new(tidy_details(&d.details)).size(10.0).color(crate::theme::gray(120)),
            );
        }
    });
}

/// Whether a decode matches the log's filter text: the kind, the ground station,
/// the channel, a position aircraft's label, or the raw payload.
fn hfdl_matches(d: &HfdlDecode, filter: &str) -> bool {
    d.kind.to_ascii_uppercase().contains(filter)
        || d.gs.as_deref().is_some_and(|g| g.to_ascii_uppercase().contains(filter))
        || format!("{:.3}", d.freq_khz as f32 / 1e3).contains(filter)
        || d.position.as_ref().is_some_and(|f| {
            f.label().to_ascii_uppercase().contains(filter)
                || f.icao.as_deref().is_some_and(|i| i.to_ascii_uppercase().contains(filter))
        })
        || d.acars.as_ref().is_some_and(|a| {
            [a.registration.as_str(), a.flight.as_str(), a.label.as_str(), a.text.as_str()]
                .iter()
                .any(|f| f.to_ascii_uppercase().contains(filter))
                || a.icao.as_deref().is_some_and(|i| i.to_ascii_uppercase().contains(filter))
                || a.decoded.as_deref().is_some_and(|x| x.to_ascii_uppercase().contains(filter))
        })
        || d.details.to_ascii_uppercase().contains(filter)
}

/// The payload JSON made readable: no braces, no quotes, one space after each
/// separator. The panel shows the fields xng parsed rather than pretty-printing
/// its structure, which at a glance is noise either way.
fn tidy_details(details: &str) -> String {
    details.trim().trim_start_matches('{').trim_end_matches('}').replace('"', "").replace(',', "  ")
}

/// A counter, short enough that it cannot outgrow its slot.
fn count(n: u64) -> String {
    if n < 10_000 {
        n.to_string()
    } else if n < 995_000 {
        format!("{:.0}k", n as f64 / 1e3)
    } else if n < 999_500_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else {
        format!("{:.2}G", n as f64 / 1e9)
    }
}

/// A readout in a slot of fixed width, so a number that grows a digit cannot
/// re-flow the header.
fn slot(ui: &mut egui::Ui, w: f32, text: &str, color: egui::Color32) {
    // At least `w`, so the slots stand still as the counts change, but never
    // narrower than the text: the widths were measured on English, and
    // „0 Dekodierungen“ was cut off after „0 Dekodie“ (HDZ 2026-11-08).
    let font = egui::FontId::monospace(10.5);
    let w = w.max(crate::chrome::text_width(ui, text, font.clone()) + 4.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 16.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    ui.painter_at(rect).text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
}

/// The left pane's tabs, as stored in the view state.
pub(in crate::app) const HFDL_TAB_DECODES: u8 = 0;
pub(in crate::app) const HFDL_TAB_STATIONS: u8 = 1;
pub(in crate::app) const HFDL_TAB_MESSAGES: u8 = 2;
pub(in crate::app) const HFDL_TAB_AIRCRAFT: u8 = 3;

/// The stations in the order the list shows them: those heard most recently
/// first, and the ones never heard (known only from the saved table) after,
/// by id.
fn hfdl_station_order(stations: &[HfdlStation]) -> Vec<&HfdlStation> {
    let mut v: Vec<&HfdlStation> = stations.iter().collect();
    v.sort_by_key(|s| (std::cmp::Reverse(s.heard_at), s.gs_id));
    v
}

/// What a channel chip says on hover: which ground stations the table gives
/// the channel to, and whether any is working it now.
fn hfdl_chip_hint(stations: &[HfdlStation], khz: u32, live: bool, now: i64) -> String {
    let on = hfdl_stations_on(stations, khz);
    if on.is_empty() {
        return if khz == 21_931 {
            tr!("hfdl-riverhead-northern-atlantic-north-america").to_string()
        } else {
            tr!("hfdl-hfdl-assigned-frequency").to_string()
        };
    }
    let names: Vec<&str> = on.iter().map(|s| s.name.as_str()).collect();
    let mut tip = tr!("hfdl-chip-stations", stations = names.join(", ")).to_string();
    if live {
        // The stations known to be working it, and how long ago that was heard.
        for s in &on {
            if s.working_khz(now).contains(&khz) {
                tip.push('\n');
                tip.push_str(
                    &tr!(
                        "hfdl-chip-working",
                        station = s.name.clone(),
                        age = crate::hfdl_map::fmt_age(now - s.heard_at.max(s.active_at))
                    )
                    .to_string(),
                );
            }
        }
    }
    tip
}

/// One ground station in the stations list: its name and how recently and how
/// well it was heard, then its channels. Returns a channel the operator clicked.
fn hfdl_station_row(ui: &mut egui::Ui, s: &HfdlStation, now: i64, cur_khz: u32) -> Option<u32> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&s.name).size(11.0).strong());
        if s.heard_at > 0 {
            let fresh = s.is_fresh(now);
            ui.label(
                RichText::new(tr!(
                    "hfdl-station-heard",
                    age = crate::hfdl_map::fmt_age(now - s.heard_at),
                    mhz = format!("{:.3}", s.heard_khz as f32 / 1e3)
                ))
                .size(10.5)
                .color(if fresh {
                    crate::theme::GREEN()
                } else {
                    crate::theme::gray(120)
                }),
            );
            if let Some(snr) = s.snr_db {
                ui.label(RichText::new(format!("{snr:.0} dB")).size(10.5).color(if snr >= 12.0 {
                    crate::theme::GREEN()
                } else {
                    crate::theme::YELLOW()
                }));
            }
            if s.squitters > 0 {
                ui.label(
                    RichText::new(tr!("hfdl-station-squitters", n = count(u64::from(s.squitters))))
                        .size(10.5)
                        .color(crate::theme::gray(120)),
                );
            }
        } else {
            ui.label(
                RichText::new(tr!("hfdl-station-not-heard"))
                    .size(10.5)
                    .color(crate::theme::gray(100)),
            );
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.add_space(10.0);
        if s.has_table() {
            let working = s.working_khz(now);
            for f in &s.freqs {
                let khz = f.khz();
                let live = working.contains(&khz);
                let mut t = RichText::new(format!("{:.3}", khz as f32 / 1e3)).size(10.5);
                t = if live {
                    t.color(crate::theme::GREEN())
                } else {
                    t.color(crate::theme::gray(130))
                };
                if khz == cur_khz {
                    t = t.underline().strong();
                }
                let resp = ui
                    .add(egui::Label::new(t).sense(egui::Sense::click()))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(tr!(
                        "hfdl-station-channel-hint",
                        mhz = format!("{:.3}", khz as f32 / 1e3)
                    ));
                if resp.clicked() {
                    picked = Some(khz);
                }
            }
        } else {
            ui.label(
                RichText::new(tr!("hfdl-station-channels-unknown"))
                    .size(10.0)
                    .color(crate::theme::gray(100)),
            );
        }
    });
    ui.add_space(3.0);
    picked
}

/// The ACARS message in a decode-log row: who, the label, and a short lead of
/// what it says (the whole of it is on the messages tab).
fn hfdl_acars_summary(ui: &mut egui::Ui, a: &HfdlAcars) {
    const LEAD: usize = 110;
    ui.label(RichText::new(hfdl_direction(a)).size(10.5).color(crate::theme::CYAN()));
    ui.label(RichText::new(a.who()).size(10.5).strong());
    ui.label(RichText::new(format!("[{}]", a.label)).size(10.5).color(crate::theme::gray(150)));
    let lead = match &a.decoded {
        Some(d) => d.clone(),
        None => hfdl_lead(&a.one_line(), LEAD),
    };
    ui.label(RichText::new(lead).size(10.0).color(crate::theme::gray(130)));
}

/// `↓` for an aircraft's message, `↑` for a ground station's.
fn hfdl_direction(a: &HfdlAcars) -> &'static str {
    if a.downlink { "↓" } else { "↑" }
}

/// `text` cut to at most `max` characters, with an ellipsis when it was.
fn hfdl_lead(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// One ACARS message on the messages tab: a line saying when, from whom, what
/// kind and how well it was received, then what the application layer made of
/// it, then the text.
fn hfdl_message_row(ui: &mut egui::Ui, d: &HfdlDecode) {
    let Some(a) = &d.acars else { return };
    let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(d.unix);
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("{h:02}:{mi:02}:{s:02}"))
                .size(10.5)
                .color(crate::theme::gray(120)),
        );
        ui.label(RichText::new(hfdl_direction(a)).size(11.0).color(crate::theme::CYAN()))
            .on_hover_text(if a.downlink {
                tr!("hfdl-acars-downlink")
            } else {
                tr!("hfdl-acars-uplink")
            });
        let who = a.who();
        if !who.is_empty() {
            ui.label(RichText::new(who).size(11.0).strong());
        }
        if !a.flight.trim().is_empty() && a.registration.trim() != a.flight.trim() {
            ui.label(RichText::new(a.flight.trim()).size(10.5).color(crate::theme::gray(160)));
        }
        let label = match &a.sublabel {
            Some(sub) => format!("{}/{sub}", a.label),
            None => a.label.clone(),
        };
        let r = ui.label(RichText::new(label).size(10.5).color(crate::theme::CYAN_DIM()));
        if let Some(name) = hfdl_acars_label_name(&a.label) {
            r.on_hover_text(name);
        }
        ui.label(
            RichText::new(format!("{:.3} MHz", d.freq_khz as f32 / 1e3))
                .size(10.5)
                .color(crate::theme::gray(130)),
        );
        if let Some(snr) = d.snr_db {
            ui.label(RichText::new(format!("{snr:.0} dB")).size(10.5).color(if snr >= 12.0 {
                crate::theme::GREEN()
            } else {
                crate::theme::YELLOW()
            }));
        }
        if !a.crc_ok {
            ui.label(
                RichText::new(tr!("hfdl-acars-crc-bad")).size(10.5).color(crate::theme::HAZARD()),
            )
            .on_hover_text(tr!("hfdl-acars-crc-bad-hint"));
        }
        if a.more {
            ui.label(
                RichText::new(tr!("hfdl-acars-more")).size(10.5).color(crate::theme::gray(120)),
            )
            .on_hover_text(tr!("hfdl-acars-more-hint"));
        }
    });
    if let Some(name) = hfdl_acars_label_name(&a.label)
        && a.decoded.is_none()
    {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new(name).size(10.0).color(crate::theme::gray(110)));
        });
    }
    if let Some(dec) = &a.decoded {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new(dec).size(10.5).color(crate::theme::CYAN()));
        });
    }
    let text = a.one_line();
    if !text.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new(text).monospace().size(10.5).color(crate::theme::gray(190)));
        });
    }
    ui.add_space(3.0);
}

/// What clicking the aircraft table asked for.
#[derive(Debug, Clone, PartialEq)]
enum AircraftAction {
    None,
    /// Blink (and select) the aircraft's plot on the map, by the map's key.
    Blink(String),
    /// Filter the decode log to this text and show it.
    Filter(String),
    /// Sort by this column.
    Sort(u8),
}

/// The aircraft table's sortable columns, as stored in the view state.
const AIRCRAFT_COL_HEARD: u8 = 0;
const AIRCRAFT_COL_REG: u8 = 1;
const AIRCRAFT_COL_FLIGHT: u8 = 2;
const AIRCRAFT_COL_GS: u8 = 3;
const AIRCRAFT_COL_MHZ: u8 = 4;
const AIRCRAFT_COL_SNR: u8 = 5;
const AIRCRAFT_COL_MSGS: u8 = 6;
const AIRCRAFT_COL_DIST: u8 = 7;

/// The longest last message shown in its cell; the whole of it is the tooltip.
const AIRCRAFT_LAST_LEAD: usize = 90;

/// How far the aircraft's last position is from `home`, in km.
fn hfdl_aircraft_km(a: &HfdlAircraft, home: Option<(f64, f64)>) -> Option<f64> {
    let (lat, lon) = a.lat.zip(a.lon)?;
    home.map(|h| sdroxide_types::distance_km(h, (lat, lon)))
}

/// The station's town, for a narrow cell: `Riverhead, New York` → `Riverhead`.
fn hfdl_short_gs(name: &str) -> &str {
    name.split(',').next().unwrap_or(name).trim()
}

/// The aircraft in the order the table shows them. An aircraft with no value
/// in the sorted column (no position for a distance, no station) goes last
/// whichever way it runs, and ties break by who was heard last.
fn hfdl_aircraft_sorted(
    fleet: &[HfdlAircraft],
    col: u8,
    asc: bool,
    home: Option<(f64, f64)>,
) -> Vec<&HfdlAircraft> {
    use std::cmp::Ordering;
    let text = |s: &Option<String>| s.as_deref().map(|t| t.trim().to_ascii_uppercase());
    let cmp = |a: &HfdlAircraft, b: &HfdlAircraft| -> Option<Ordering> {
        match col {
            AIRCRAFT_COL_REG => {
                text(&a.registration).zip(text(&b.registration)).map(|(x, y)| x.cmp(&y))
            }
            AIRCRAFT_COL_FLIGHT => text(&a.flight).zip(text(&b.flight)).map(|(x, y)| x.cmp(&y)),
            AIRCRAFT_COL_GS => text(&a.gs).zip(text(&b.gs)).map(|(x, y)| x.cmp(&y)),
            AIRCRAFT_COL_MHZ => Some(a.freq_khz.cmp(&b.freq_khz)),
            AIRCRAFT_COL_SNR => a.snr_db.zip(b.snr_db).map(|(x, y)| x.total_cmp(&y)),
            AIRCRAFT_COL_MSGS => Some(a.msgs.cmp(&b.msgs)),
            AIRCRAFT_COL_DIST => hfdl_aircraft_km(a, home)
                .zip(hfdl_aircraft_km(b, home))
                .map(|(x, y)| x.total_cmp(&y)),
            _ => Some(a.last_heard.cmp(&b.last_heard)),
        }
    };
    let has = |a: &HfdlAircraft| match col {
        AIRCRAFT_COL_REG => a.registration.is_some(),
        AIRCRAFT_COL_FLIGHT => a.flight.is_some(),
        AIRCRAFT_COL_GS => a.gs.is_some(),
        AIRCRAFT_COL_SNR => a.snr_db.is_some(),
        AIRCRAFT_COL_DIST => hfdl_aircraft_km(a, home).is_some(),
        _ => true,
    };
    let mut v: Vec<&HfdlAircraft> = fleet.iter().collect();
    v.sort_by(|a, b| {
        // Empty cells last, in either direction.
        match (has(a), has(b)) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let o = cmp(a, b).unwrap_or(Ordering::Equal);
        let o = if asc { o } else { o.reverse() };
        o.then(b.last_heard.cmp(&a.last_heard))
    });
    v
}

/// The table's header row. Returns the column the operator clicked to sort by.
fn hfdl_aircraft_header(ui: &mut egui::Ui, sorted: u8, asc: bool) -> Option<u8> {
    let mut clicked = None;
    ui.label(""); // over the log arrows
    for (col, label) in [
        (AIRCRAFT_COL_REG, tr!("hfdl-col-reg")),
        (AIRCRAFT_COL_FLIGHT, tr!("hfdl-col-flight")),
        (AIRCRAFT_COL_HEARD, tr!("hfdl-col-heard")),
        (AIRCRAFT_COL_GS, tr!("hfdl-col-station")),
        (AIRCRAFT_COL_MHZ, tr!("hfdl-col-mhz")),
        (AIRCRAFT_COL_SNR, tr!("hfdl-col-snr")),
        (AIRCRAFT_COL_MSGS, tr!("hfdl-col-msgs")),
        (AIRCRAFT_COL_DIST, tr!("hfdl-col-dist")),
    ] {
        let arrow = if col != sorted {
            ""
        } else if asc {
            " ▲"
        } else {
            " ▼"
        };
        let ink = if col == sorted { crate::theme::CYAN() } else { crate::theme::CYAN_DIM() };
        let r = ui.add(
            egui::Label::new(
                RichText::new(format!("{label}{arrow}")).size(10.0).strong().color(ink),
            )
            .sense(egui::Sense::click()),
        );
        if r.on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tr!("hfdl-aircraft-sort-hint"))
            .clicked()
        {
            clicked = Some(col);
        }
    }
    ui.label(
        RichText::new(tr!("hfdl-col-last")).size(10.0).strong().color(crate::theme::CYAN_DIM()),
    );
    clicked
}

/// What a position cell's tooltip says: where it last said it was, how long
/// ago, and — from our own position, where we have one — how far and which way.
fn hfdl_aircraft_where(a: &HfdlAircraft, now: i64, home: Option<(f64, f64)>) -> Option<String> {
    let (lat, lon) = a.lat.zip(a.lon)?;
    let mut out = tr!(
        "hfdl-aircraft-position",
        lat = format!("{lat:.3}"),
        lon = format!("{lon:.3}"),
        age = crate::hfdl_map::fmt_age(now - a.fix_at)
    )
    .to_string();
    if let Some(h) = home {
        let km = sdroxide_types::distance_km(h, (lat, lon));
        let bear = sdroxide_types::bearing_deg(h, (lat, lon));
        out.push_str(" · ");
        out.push_str(&tr!(
            "hfdl-aircraft-from-you",
            km = format!("{km:.0}"),
            bearing = format!("{bear:.0}")
        ));
    }
    Some(out)
}

/// One aircraft as a row of the table (ten cells; the caller ends the row).
fn hfdl_aircraft_row(
    ui: &mut egui::Ui,
    a: &HfdlAircraft,
    now: i64,
    home: Option<(f64, f64)>,
    plotted: bool,
    selected: bool,
) -> AircraftAction {
    let mut action = AircraftAction::None;
    let dim = crate::theme::gray(130);
    let fine = crate::theme::gray(190);

    // Log arrow: filter the decode log to this aircraft.
    let filter = a.filter_text();
    if filter.is_empty() {
        ui.label("");
    } else if ui
        .add(
            egui::Label::new(RichText::new("▸").size(11.0).color(crate::theme::CYAN_DIM()))
                .sense(egui::Sense::click()),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tr!("hfdl-aircraft-filter-hint"))
        .clicked()
    {
        action = AircraftAction::Filter(filter);
    }

    // Registration — or the best name there is, dimmer — blinks the plot.
    let (reg, reg_known) = match (&a.registration, &a.icao) {
        (Some(r), _) => (r.trim().to_string(), true),
        (None, Some(i)) => (i.clone(), false),
        (None, None) => (a.label(), false),
    };
    let ink = if selected {
        crate::theme::CYAN()
    } else if a.logged_off {
        crate::theme::gray(110)
    } else if reg_known {
        crate::theme::gray(235)
    } else {
        crate::theme::gray(160)
    };
    let mut text = RichText::new(reg).size(11.0).strong().color(ink);
    if a.logged_off {
        text = text.strikethrough();
    }
    let r = ui.add(egui::Label::new(text).sense(if plotted {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    }));
    // The hand only where a click does something: an aircraft with no plot has
    // nothing to blink.
    let r = if plotted { r.on_hover_cursor(egui::CursorIcon::PointingHand) } else { r };
    let mut tip = String::new();
    if let Some(i) = &a.icao {
        tip.push_str(&format!("ICAO {i}\n"));
    }
    if a.logged_off {
        tip.push_str(&tr!("hfdl-aircraft-logged-off"));
        tip.push('\n');
    }
    tip.push_str(&if plotted {
        tr!("hfdl-aircraft-blink-hint").to_string()
    } else {
        tr!("hfdl-aircraft-no-position-hint").to_string()
    });
    if r.on_hover_text(tip).clicked()
        && plotted
        && let Some(k) = &a.map_key
    {
        action = AircraftAction::Blink(k.clone());
    }

    ui.label(RichText::new(a.flight.as_deref().unwrap_or("").trim()).size(10.5).color(fine));

    let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(a.last_heard);
    let age = now - a.last_heard;
    ui.label(RichText::new(crate::hfdl_map::fmt_age(age)).size(10.5).color(if age < 300 {
        crate::theme::GREEN()
    } else {
        dim
    }))
    .on_hover_text(format!("{h:02}:{mi:02}:{s:02} UTC"));

    ui.label(RichText::new(a.gs.as_deref().map_or("", hfdl_short_gs)).size(10.5).color(fine))
        .on_hover_text(a.gs.as_deref().unwrap_or(""));
    ui.label(RichText::new(format!("{:.3}", a.freq_khz as f32 / 1e3)).size(10.5).color(dim));
    ui.label(
        RichText::new(a.snr_db.map_or(String::new(), |v| format!("{v:.0}"))).size(10.5).color(
            match a.snr_db {
                Some(v) if v >= 12.0 => crate::theme::GREEN(),
                Some(_) => crate::theme::YELLOW(),
                None => dim,
            },
        ),
    );
    ui.label(RichText::new(count(u64::from(a.msgs))).size(10.5).color(dim));

    let dist = ui.label(
        RichText::new(hfdl_aircraft_km(a, home).map_or(String::new(), |km| format!("{km:.0}")))
            .size(10.5)
            .color(crate::theme::CYAN_DIM()),
    );
    if let Some(w) = hfdl_aircraft_where(a, now, home) {
        dist.on_hover_text(w);
    }

    // The last message: what it said, or — for an aircraft that has sent no
    // ACARS — only what kind of event was last heard from it, dimmer.
    match &a.last_text {
        Some(t) => {
            ui.label(
                RichText::new(hfdl_lead(t, AIRCRAFT_LAST_LEAD))
                    .monospace()
                    .size(10.0)
                    .color(crate::theme::gray(200)),
            )
            .on_hover_text(t.as_str());
        }
        None => {
            ui.label(RichText::new(&a.last_kind).size(10.0).color(crate::theme::gray(100)));
        }
    }
    action
}

/// Save the channel in force as one of the operator's own chips, keeping the
/// list sorted and refusing a channel already on it — so the file is written
/// only on a real edit and the chip row does not reshuffle under the pointer.
fn add_hfdl_channel(list: &mut Vec<u32>, khz: u32) -> bool {
    if list.contains(&khz) {
        return false;
    }
    list.push(khz);
    list.sort_unstable();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{HFDL_DEFAULT_HZ, HfdlFix, HfdlStationFreq};

    /// Saving the channel in force is idempotent and keeps the chips sorted, so
    /// the file is only written on a real edit and the row does not reshuffle.
    #[test]
    fn saving_a_channel_is_idempotent_and_sorted() {
        let mut list = Vec::new();
        assert!(add_hfdl_channel(&mut list, 11_184));
        assert!(add_hfdl_channel(&mut list, 5_451));
        assert!(add_hfdl_channel(&mut list, 8_843));
        assert_eq!(list, vec![5_451, 8_843, 11_184]);
        assert!(!add_hfdl_channel(&mut list, 8_843), "the same channel changes nothing");
        assert_eq!(list, vec![5_451, 8_843, 11_184]);
    }

    fn dec(kind: &str, gs: Option<&str>) -> HfdlDecode {
        HfdlDecode {
            unix: 1_700_000_000,
            kind: kind.to_string(),
            gs: gs.map(|s| s.to_string()),
            freq_khz: 21_931,
            snr_db: Some(14.5),
            freq_skew_hz: None,
            fec_corrected: None,
            details: r#"{"aircraft":"A9C-DM"}"#.to_string(),
            position: None,
            acars: None,
        }
    }

    /// The status slots are sized to sit side by side, but a panel lays out
    /// vertically — so without a horizontal layout each took a line of its own
    /// and the header came out five lines tall with the panel's width empty
    /// beside it. Measured, because "it looks right" is what let it ship.
    #[test]
    fn the_status_strip_is_one_row() {
        let status = HfdlStatus {
            running: true,
            level_dbfs: -59.0,
            bursts: 30,
            decodes: 44,
            ..Default::default()
        };
        let mut tall = 0.0;
        egui::__run_test_ui(|ui| {
            let before = ui.min_rect().height();
            hfdl_status_strip(ui, Some(&status), 3);
            tall = ui.min_rect().height() - before;
        });
        // One row of 16pt slots, plus whatever the style puts around it. Five
        // rows could not fit under this however the spacing is set.
        assert!(tall > 0.0, "the strip drew nothing, so this measures nothing");
        assert!(tall < 40.0, "the status strip is {tall} points tall — it has stacked again");
    }

    fn gs(id: u8, name: &str, khz: &[u32], heard_at: i64) -> HfdlStation {
        let mut s = HfdlStation::new(id, Some(name));
        s.freqs = khz
            .iter()
            .enumerate()
            .map(|(i, k)| HfdlStationFreq { freq_hz: k * 1000, slot: i as u8, active: false })
            .collect();
        s.heard_at = heard_at;
        s
    }

    /// The most recently heard station leads; the ones never heard follow by id.
    #[test]
    fn the_stations_list_leads_with_the_latest_heard() {
        let list = [
            gs(7, "Shannon", &[], 0),
            gs(4, "Riverhead", &[], 900),
            gs(2, "Molokai", &[], 0),
            gs(3, "Reykjavik", &[], 950),
        ];
        let ids: Vec<u8> = hfdl_station_order(&list).iter().map(|s| s.gs_id).collect();
        assert_eq!(ids, vec![3, 4, 2, 7]);
    }

    #[test]
    fn a_chip_names_the_stations_that_use_its_channel() {
        let mut riverhead = gs(4, "Riverhead", &[5_451, 21_931], 1_000);
        riverhead.heard_khz = 21_931;
        let list = [riverhead, gs(7, "Shannon", &[3_455, 5_451], 0)];
        // Two stations carry 5 451 kHz; neither is known to be working it yet.
        let tip = hfdl_chip_hint(&list, 5_451, false, 1_010);
        assert!(tip.contains("Riverhead") && tip.contains("Shannon"), "{tip}");
        assert!(!tip.contains("working"), "{tip}");
        // Riverhead was just heard on 21 931 kHz: the hint says so.
        let tip = hfdl_chip_hint(&list, 21_931, true, 1_010);
        assert!(tip.contains("Riverhead") && tip.contains("working"), "{tip}");
        // A channel the table does not give to anyone falls back to the plain text.
        assert_eq!(hfdl_chip_hint(&list, 9_999, false, 1_010), tr!("hfdl-hfdl-assigned-frequency"));
    }

    /// A station row is two lines of wrapped labels, not one tall stack — with
    /// the whole 20-channel table of a busy station it must not run away.
    #[test]
    fn a_station_row_stays_compact_and_reports_a_clicked_channel() {
        let mut s =
            gs(4, "Riverhead, New York", &[2_941, 5_451, 8_912, 11_184, 13_306, 17_919], 990);
        s.heard_khz = 8_912;
        s.snr_db = Some(14.0);
        s.squitters = 6;
        let mut tall = 0.0;
        let mut picked = Some(0);
        egui::__run_test_ui(|ui| {
            let before = ui.min_rect().height();
            picked = hfdl_station_row(ui, &s, 1_000, 8_912);
            tall = ui.min_rect().height() - before;
        });
        assert_eq!(picked, None, "nothing was clicked");
        assert!(tall > 0.0 && tall < 90.0, "a station row is {tall} points tall");
    }

    #[test]
    fn a_station_without_a_table_says_its_channels_are_unknown() {
        let s = gs(4, "Riverhead", &[], 0);
        let mut tall = 0.0;
        egui::__run_test_ui(|ui| {
            let before = ui.min_rect().height();
            let _ = hfdl_station_row(ui, &s, 1_000, 21_931);
            tall = ui.min_rect().height() - before;
        });
        assert!(tall > 0.0, "a station with nothing known still gets a row");
    }

    fn msg(text: &str, decoded: Option<&str>) -> HfdlDecode {
        let mut d = dec("acars", None);
        d.details = r#"{"who":{"dir":"downlink"}}"#.into();
        d.acars = Some(HfdlAcars {
            downlink: true,
            registration: "N123AB".into(),
            flight: "UAL012".into(),
            label: "H1".into(),
            text: text.into(),
            crc_ok: true,
            decoded: decoded.map(str::to_owned),
            ..Default::default()
        });
        d
    }

    /// An ACARS message is found by its aircraft, its label and its words — not
    /// only by the link-layer envelope the log used to show.
    #[test]
    fn the_filter_finds_an_acars_message_by_its_text() {
        let d = msg("REQUEST DIRECT TO BIRD", Some("ETA 1530"));
        assert!(hfdl_matches(&d, "BIRD"), "the message text");
        assert!(hfdl_matches(&d, "N123AB"), "the registration");
        assert!(hfdl_matches(&d, "UAL012"), "the flight");
        assert!(hfdl_matches(&d, "H1"), "the label");
        assert!(hfdl_matches(&d, "ETA 1530"), "what was decoded from it");
        assert!(!hfdl_matches(&d, "WILCO"));
    }

    #[test]
    fn a_long_lead_is_cut_with_an_ellipsis() {
        assert_eq!(hfdl_lead("short", 10), "short");
        let cut = hfdl_lead("abcdefghijklmnop", 10);
        assert_eq!(cut.chars().count(), 10);
        assert!(cut.ends_with('…'));
        // Cut on characters, not bytes: a multi-byte one is never split.
        assert_eq!(hfdl_lead("⏎⏎⏎⏎⏎⏎", 4).chars().count(), 4);
    }

    /// A message with its text, a decoded line and a long body stays a handful
    /// of lines — and a CRC-failed one and a multi-block one draw too.
    #[test]
    fn a_message_row_has_a_bounded_height() {
        let long = "WORD ".repeat(60);
        let mut d = msg(&long, Some("ETA 1530"));
        d.acars.as_mut().unwrap().crc_ok = false;
        d.acars.as_mut().unwrap().more = true;
        let mut tall = 0.0;
        egui::__run_test_ui(|ui| {
            let before = ui.min_rect().height();
            hfdl_message_row(ui, &d);
            tall = ui.min_rect().height() - before;
        });
        assert!(tall > 20.0, "the row drew nothing: {tall}");
        assert!(tall < 220.0, "a message row is {tall} points tall");
    }

    #[test]
    fn a_log_row_with_an_acars_message_draws_it_not_the_envelope() {
        // No panic, and not the empty-details fallback: a row is drawn.
        let d = msg("HELLO", None);
        let mut tall = 0.0;
        egui::__run_test_ui(|ui| {
            let before = ui.min_rect().height();
            hfdl_log_row(ui, &d);
            tall = ui.min_rect().height() - before;
        });
        assert!(tall > 0.0);
    }

    fn plane() -> HfdlAircraft {
        let mut a = HfdlAircraft::new("icao:A1B2C3".into(), 900);
        a.icao = Some("A1B2C3".into());
        a.registration = Some("N123AB".into());
        a.flight = Some("UAL012".into());
        a.gs = Some("Riverhead, New York".into());
        a.freq_khz = 8_912;
        a.last_heard = 990;
        a.msgs = 7;
        a.snr_db = Some(12.0);
        a.lat = Some(40.0);
        a.lon = Some(-50.0);
        a.fix_at = 700;
        a.map_key = Some("icao:A1B2C3".into());
        a.last_kind = "acars".into();
        a.last_text = Some("[H1] REQUEST HIGHER".into());
        a
    }

    fn fleet_of(rows: &[(&str, i64, Option<&str>, u32, Option<f64>)]) -> Vec<HfdlAircraft> {
        rows.iter()
            .map(|(icao, heard, reg, msgs, lat)| {
                let mut a = HfdlAircraft::new(format!("icao:{icao}"), *heard);
                a.icao = Some((*icao).into());
                a.last_heard = *heard;
                a.registration = reg.map(str::to_owned);
                a.msgs = *msgs;
                a.lat = *lat;
                a.lon = lat.map(|_| 0.0);
                a.fix_at = lat.map_or(0, |_| *heard);
                a
            })
            .collect()
    }

    fn order(v: &[&HfdlAircraft]) -> Vec<String> {
        v.iter().map(|a| a.icao.clone().unwrap()).collect()
    }

    #[test]
    fn the_table_sorts_by_the_chosen_column_and_empty_cells_go_last() {
        let f = fleet_of(&[
            ("AAA", 100, Some("N2"), 5, Some(10.0)),
            ("BBB", 300, None, 9, None),
            ("CCC", 200, Some("N1"), 1, Some(50.0)),
        ]);
        let home = Some((0.0, 0.0));
        // Heard: newest first by default, oldest first reversed.
        assert_eq!(
            order(&hfdl_aircraft_sorted(&f, AIRCRAFT_COL_HEARD, false, home)),
            ["BBB", "CCC", "AAA"]
        );
        assert_eq!(
            order(&hfdl_aircraft_sorted(&f, AIRCRAFT_COL_HEARD, true, home)),
            ["AAA", "CCC", "BBB"]
        );
        // Registration A to Z — BBB has none, so it is last either way.
        assert_eq!(
            order(&hfdl_aircraft_sorted(&f, AIRCRAFT_COL_REG, true, home)),
            ["CCC", "AAA", "BBB"]
        );
        assert_eq!(
            order(&hfdl_aircraft_sorted(&f, AIRCRAFT_COL_REG, false, home)),
            ["AAA", "CCC", "BBB"]
        );
        // Messages, biggest first.
        assert_eq!(
            order(&hfdl_aircraft_sorted(&f, AIRCRAFT_COL_MSGS, false, home)),
            ["BBB", "AAA", "CCC"]
        );
        // Distance needs a position *and* a home; without home all tie and fall
        // back to who was heard last.
        assert_eq!(
            order(&hfdl_aircraft_sorted(&f, AIRCRAFT_COL_DIST, true, home)),
            ["AAA", "CCC", "BBB"]
        );
        assert_eq!(
            order(&hfdl_aircraft_sorted(&f, AIRCRAFT_COL_DIST, true, None)),
            ["BBB", "CCC", "AAA"]
        );
    }

    #[test]
    fn a_station_is_shown_by_its_town() {
        assert_eq!(hfdl_short_gs("Riverhead, New York"), "Riverhead");
        assert_eq!(hfdl_short_gs("GS 12"), "GS 12");
        assert_eq!(hfdl_short_gs(""), "");
    }

    #[test]
    fn a_position_tooltip_reads_from_where_we_are() {
        let a = plane();
        let w = hfdl_aircraft_where(&a, 1_000, Some((40.0, -72.0))).unwrap();
        assert!(w.contains("40.000, -50.000"), "{w}");
        assert!(w.contains("km"), "{w}");
        let blind = hfdl_aircraft_where(&a, 1_000, None).unwrap();
        assert!(!blind.contains("km"), "{blind}");
        let none = HfdlAircraft { lat: None, lon: None, fix_at: 0, ..a };
        assert_eq!(hfdl_aircraft_where(&none, 1_000, None), None);
        assert_eq!(hfdl_aircraft_km(&none, Some((0.0, 0.0))), None);
    }

    /// Draw the header and one row into a grid, as the pane does.
    fn run_table(a: &HfdlAircraft, plotted: bool) -> (f32, usize, AircraftAction) {
        let mut out = (0.0, 0, AircraftAction::None);
        egui::__run_test_ui(|ui| {
            let before = ui.min_rect().height();
            let mut act = AircraftAction::None;
            egui::Grid::new("t").num_columns(10).show(ui, |ui| {
                assert_eq!(hfdl_aircraft_header(ui, AIRCRAFT_COL_HEARD, false), None);
                ui.end_row();
                act = hfdl_aircraft_row(ui, a, 1_000, Some((40.0, -72.0)), plotted, false);
                ui.end_row();
            });
            out = (ui.min_rect().height() - before, 0, act);
        });
        out
    }

    #[test]
    fn a_table_row_is_one_line_and_does_nothing_until_clicked() {
        let (tall, _, act) = run_table(&plane(), true);
        assert_eq!(act, AircraftAction::None);
        assert!(tall > 20.0 && tall < 70.0, "header and row are {tall} points tall");
        // An aircraft that never sent a position, ACARS or a resolved ICAO,
        // and has logged off, still draws one row.
        let mut bare = HfdlAircraft::new("gs4:ac12".into(), 900);
        bare.aircraft_id = Some(12);
        bare.logged_off = true;
        bare.last_kind = "performance-data".into();
        let (tall, _, act) = run_table(&bare, false);
        assert_eq!(act, AircraftAction::None);
        assert!(tall > 20.0 && tall < 70.0, "{tall}");
        // A long message stays one line: it is cut for the cell.
        let mut chatty = plane();
        chatty.last_text = Some(format!("[H1] {}", "WORD ".repeat(80)));
        let (tall, _, _) = run_table(&chatty, true);
        assert!(tall < 70.0, "a long last message made the row {tall} points tall");
    }

    #[test]
    fn default_channel_is_in_the_plan() {
        assert!(HFDL_PLAN_KHZ.contains(&(HFDL_DEFAULT_HZ as u32 / 1000)));
    }

    #[test]
    fn the_filter_matches_kind_station_and_payload() {
        let d = dec("squitter", Some("Riverhead (GS 4)"));
        assert!(hfdl_matches(&d, "SQUIT"));
        assert!(hfdl_matches(&d, "RIVERHEAD"));
        assert!(hfdl_matches(&d, "A9C-DM"), "the payload details match too");
        assert!(!hfdl_matches(&d, "POSITION"));
    }

    #[test]
    fn the_filter_matches_a_position_aircraft() {
        let mut d = dec("performance-data", None);
        d.details = "{}".into();
        d.position = Some(HfdlFix {
            lat: 40.88,
            lon: -72.64,
            aircraft_id: Some(0x42),
            icao: Some("040087".into()),
            flight: Some("BAW123".into()),
        });
        assert!(hfdl_matches(&d, "BAW123"), "by flight");
        assert!(hfdl_matches(&d, "040087"), "by ICAO");
        assert!(!hfdl_matches(&d, "KLM"));
    }

    #[test]
    fn details_are_tidied_for_the_row() {
        assert_eq!(tidy_details(r#"{"gs_id":4,"lpdus":3}"#), "gs_id:4  lpdus:3");
    }
}
