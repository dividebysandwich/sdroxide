//! Network spots: which ones are shown, and the SPOTS window that lists them.
//!
//! Spots arrive from several feeds (DX cluster, POTA, SOTA, PSK Reporter,
//! FreeDV Reporter) plus the bundled broadcast schedule, and are merged into
//! one list filtered by kind, by the current view span, and by a fuzzy search.
//! Clicking one tunes the radio and pre-fills the logbook.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Mode, RxId, Spot, SpotKind};

use crate::theme::ThemedScroll;
use crate::time::now_unix;

use crate::app::SdroxideApp;
use crate::app::logbook::LogEditForm;
use crate::app::settings::SettingsTab;
use crate::app::util::fmt_age;

/// Everything about a spot the search box should be able to find it by.
///
/// The frequency goes in twice, as kHz and as MHz, because a shortwave listener
/// thinks in `9420` and a ham in `9.420` and both should work. The kind label is
/// in there too, so typing `bc` narrows to the broadcast stations without having
/// to reach for the chips.
fn spot_haystack(s: &Spot) -> String {
    let mut h = String::with_capacity(96);
    h.push_str(&s.call);
    for extra in [
        s.kind.label(),
        &s.mode,
        &s.comment,
        s.reference.as_deref().unwrap_or(""),
        &s.spotter,
        s.grid.as_deref().unwrap_or(""),
    ] {
        if !extra.is_empty() {
            h.push(' ');
            h.push_str(extra);
        }
    }
    h.push_str(&format!(" {:.0} {:.4}", s.freq_hz / 1e3, s.freq_hz / 1e6));
    h
}

/// One clickable spot row for the spots window: kind badge, call, frequency,
/// mode, age or schedule, and the park/summit/transmitter reference or comment.
fn spot_row(ui: &mut egui::Ui, s: &Spot, now_utc: i64, needed: bool) -> egui::Response {
    let kind_col = crate::theme::data_ink(crate::theme::spot_color(s.kind));
    let gray = crate::theme::gray(150);
    let inner = egui::Frame::new()
        .fill(crate::theme::ROW_BG())
        .inner_margin(egui::Margin { left: 8, right: 6, top: 4, bottom: 4 })
        .show(ui, |ui| {
            ui.set_min_height(22.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let col = |ui: &mut egui::Ui, w: f32, lbl: egui::Label| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(w, 20.0), egui::Sense::hover());
                    ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    )
                    .add(lbl);
                };
                col(
                    ui,
                    44.0,
                    egui::Label::new(
                        RichText::new(s.kind.label()).size(10.0).strong().color(kind_col),
                    ),
                );
                col(
                    ui,
                    132.0,
                    egui::Label::new(
                        RichText::new(&s.call)
                            .size(14.0)
                            .strong()
                            .color(crate::theme::TEXT_STRONG()),
                    )
                    .truncate(),
                );
                col(
                    ui,
                    78.0,
                    egui::Label::new(
                        RichText::new(format!("{:.4}", s.freq_hz / 1e6))
                            .monospace()
                            .size(12.0)
                            .color(gray),
                    ),
                );
                col(
                    ui,
                    46.0,
                    egui::Label::new(RichText::new(&s.mode).monospace().size(11.0).color(gray)),
                );
                // A broadcast station is not a report that ages: it carries its
                // schedule (`"24h"`, `"1800-2100"`) in this column instead.
                let when = if s.kind == SpotKind::Broadcast {
                    s.spotter.clone()
                } else {
                    fmt_age(now_utc - s.when_utc)
                };
                col(
                    ui,
                    76.0,
                    egui::Label::new(RichText::new(when).size(10.5).color(crate::theme::gray(120))),
                );
                if needed {
                    col(
                        ui,
                        36.0,
                        egui::Label::new(
                            RichText::new("NEW").size(10.0).strong().color(crate::theme::GREEN()),
                        ),
                    );
                }
                let info = match &s.reference {
                    Some(r) if !s.comment.is_empty() => format!("{r} · {}", s.comment),
                    Some(r) => r.clone(),
                    None => s.comment.clone(),
                };
                ui.add(egui::Label::new(RichText::new(info).size(11.0).color(gray)).truncate());
            });
        });
    let resp = inner.response.interact(egui::Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

impl SdroxideApp {
    /// Whether a spot passes the operator's filters.
    ///
    /// The single place that decides this. The waterfall overlay, the SPOTS list
    /// and the world-map dots all go through here, so switching a category off
    /// cannot take effect in one view and be forgotten in another.
    ///
    /// The search query is deliberately *not* part of this: it narrows the list
    /// in the SPOTS window only. See [`App::spot_search`].
    pub(in crate::app) fn spot_visible(&self, s: &Spot) -> bool {
        if !self.view.spot_kinds_shown[s.kind.index()] {
            return false;
        }
        if self.spot_in_view_only
            && !(self.view.view_lo_hz..=self.view.view_hi_hz).contains(&s.freq_hz)
        {
            return false;
        }
        true
    }

    /// Rebuild the on-air broadcast station list if the UTC minute has rolled
    /// over since it was last built, and take delivery of a schedule download.
    /// Cheap enough to call every frame.
    pub(in crate::app) fn refresh_broadcast_spots(&mut self, now_utc: i64) {
        self.poll_schedule_fetch(now_utc);
        let minute = now_utc.div_euclid(60);
        if minute == self.broadcast_minute {
            return;
        }
        self.broadcast_minute = minute;
        self.broadcast_spots = sdroxide_types::broadcast::on_air(&self.broadcast, now_utc);
    }

    /// Collect a finished schedule download, and start one when the broadcasting
    /// season has turned over since the cache was filled.
    ///
    /// The season check is once a day rather than once a frame: it is a calendar
    /// event, and `broadcast_schedule_due` stats a file.
    fn poll_schedule_fetch(&mut self, now_utc: i64) {
        if let Some(rx) = &self.broadcast_fetch
            && let Ok(result) = rx.try_recv()
        {
            self.broadcast_fetch = None;
            self.broadcast_fetch_status = Some(match result {
                Ok(stations) => {
                    let msg = format!("{} transmissions", stations.len());
                    self.broadcast = sdroxide_types::broadcast::with_utilities(stations);
                    // Force the on-air list to be rebuilt from the new schedule
                    // rather than waiting up to a minute for the tick.
                    self.broadcast_minute = -1;
                    Ok(msg)
                }
                // The compiled-in schedule stays in use, so a failed download
                // costs nothing but freshness.
                Err(e) => Err(e),
            });
        }
        let day = now_utc.div_euclid(86_400);
        if self.broadcast_fetch.is_none() && day != self.broadcast_checked_day {
            self.broadcast_checked_day = day;
            self.broadcast_fetch = crate::app::persist::spawn_schedule_fetch(false);
        }
    }

    /// Download the current season's schedule now, whether or not one is cached.
    pub(in crate::app) fn refetch_broadcast_schedule(&mut self) {
        crate::app::persist::clear_broadcast_cache();
        self.broadcast_fetch_status = None;
        self.broadcast_fetch = crate::app::persist::spawn_schedule_fetch(true);
    }

    /// Re-read the operator's own station file and lay it over the schedule.
    pub(in crate::app) fn reload_broadcast_stations(&mut self) {
        self.broadcast =
            sdroxide_types::broadcast::with_utilities(crate::app::persist::load_broadcast_stations());
        self.broadcast_minute = -1;
    }

    /// Live network spots and the on-air broadcast stations, unfiltered.
    pub(in crate::app) fn all_spots(&self) -> impl Iterator<Item = &Spot> {
        self.spots.iter().chain(self.broadcast_spots.iter())
    }

    /// The same two sets as one owned list in frequency order, for the SPOTS
    /// window. `self.spots` arrives sorted from the feed manager, but the
    /// broadcast stations have to be merged into that order.
    fn merged_spots(&self) -> Vec<Spot> {
        let mut all: Vec<Spot> = self.all_spots().cloned().collect();
        all.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
        all
    }

    /// Open a fresh log entry pre-filled from a clicked spot, and kick a
    /// callsign lookup if auto-lookup is on.
    ///
    /// Broadcast stations are exempt: "BBC World Service" is not a callsign to
    /// log or look up on QRZ, so clicking one only tunes. Guarding here rather
    /// than at each call site covers both the SPOTS list and the panadapter.
    pub(in crate::app) fn prefill_from_spot(&mut self, spot: &Spot) {
        if spot.kind == SpotKind::Broadcast {
            return;
        }
        let mut form = LogEditForm::new_entry(now_unix(), spot.freq_hz, &spot.mode);
        form.call = spot.call.clone();
        if let Some(g) = &spot.grid {
            form.grid = g.clone();
        }
        if !spot.comment.is_empty() {
            form.comment = spot.comment.clone();
        }
        self.log_edit = Some(form);
        self.show_logbook = true;
        self.queue_lookup(spot.call.clone());
    }

    /// Tune the active VFO onto a spot (CW dialed a pitch below, so it lands in
    /// the CW passband), set its mode, and open a pre-filled log entry.
    fn select_spot(&mut self, spot: &Spot, cmds: &mut Vec<Command>) {
        match spot.radio_mode() {
            Some(Mode::Cw) => {
                let (lo, hi) = Mode::Cw.default_filter();
                let pitch = ((lo + hi) * 0.5) as f64;
                cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz: spot.freq_hz - pitch });
                cmds.push(Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
            }
            Some(m) => {
                cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz: spot.freq_hz });
                cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
            }
            None => cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz: spot.freq_hz }),
        }
        self.prefill_from_spot(spot);
    }

    /// The live-spots window: source filters, a fuzzy search box, a
    /// click-to-tune list of current DX-cluster / POTA / SOTA / PSK-Reporter
    /// spots and broadcast stations, and the feed status line.
    pub(in crate::app) fn spots_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let worked_entities = self.worked_entities().clone();
        let mut open = self.show_spots;
        let mut clicked: Option<Spot> = None;
        let mut open_setup = false;
        let now = now_unix();
        self.refresh_broadcast_spots(now);
        // Cloned out of `self` because the window closure needs `&mut self`.
        let spots = self.merged_spots();
        // Chip order has to match `SpotKind::index`: the loop below indexes
        // `spot_kinds_shown` positionally.
        let labels = [
            (SpotKind::DxCluster, "DX"),
            (SpotKind::Pota, "POTA"),
            (SpotKind::Sota, "SOTA"),
            (SpotKind::PskReporter, "PSK"),
            (SpotKind::FreeDv, "FREEDV"),
            (SpotKind::Broadcast, "BC"),
        ];
        let resp = egui::Window::new("SPOTS")
            .id(crate::layout::salted_id(ctx, "SPOTS"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 580.0))
            .default_height(crate::layout::window_h(ctx, 480.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.horizontal(|ui| {
                    for (i, (kind, label)) in labels.iter().enumerate() {
                        let chip = crate::chrome::chip(ui, self.view.spot_kinds_shown[i], *label);
                        let chip = if *kind == SpotKind::Broadcast {
                            chip.on_hover_text("Longwave & shortwave broadcast stations on air now")
                        } else {
                            chip
                        };
                        if chip.clicked() {
                            self.view.spot_kinds_shown[i] = !self.view.spot_kinds_shown[i];
                        }
                    }
                    if crate::chrome::chip(ui, self.spot_in_view_only, "IN VIEW")
                        .on_hover_text("Only spots inside the panadapter span")
                        .clicked()
                    {
                        self.spot_in_view_only = !self.spot_in_view_only;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::chrome::chip(ui, false, "⚙ SETUP")
                            .on_hover_text("Feeds, lookup & upload settings")
                            .clicked()
                        {
                            open_setup = true;
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.label(RichText::new("⌕").color(crate::theme::CYAN_DIM()).size(14.0));
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.spot_search)
                            .desired_width(200.0)
                            .hint_text("call, station, site, frequency")
                            .text_color(crate::theme::TEXT_STRONG()),
                    );
                    if !self.spot_search.trim().is_empty()
                        && ui.button("✕").on_hover_text("Clear the search").clicked()
                    {
                        self.spot_search.clear();
                    }
                });
                if let Some(s) = &self.net_status {
                    ui.label(RichText::new(s).size(11.0).color(crate::theme::gray(150)));
                }
                ui.separator();
                // Filter by the category chips, then rank by how well each row
                // matched the query. With no query the natural frequency order
                // is kept; with one, the best matches come first, because the
                // whole point of typing is to get the wanted row to the top.
                let query = self.spot_search.trim();
                let visible: Vec<&Spot> = spots.iter().filter(|s| self.spot_visible(s)).collect();
                let mut rows: Vec<(&Spot, i32)> = visible
                    .iter()
                    .filter_map(|s| {
                        crate::fuzzy::score_terms(&spot_haystack(s), query).map(|sc| (*s, sc))
                    })
                    .collect();
                if !query.is_empty() {
                    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
                    // Counted against what the chips let through, not against
                    // every spot held — "3 of 5" when three categories are off
                    // would look like the search had lost the rest.
                    let (text, colour) = match rows.len() {
                        0 => ("no match".to_string(), crate::theme::ALERT()),
                        n => (format!("{n} of {}", visible.len()), crate::theme::YELLOW()),
                    };
                    ui.label(RichText::new(text).color(colour).size(10.0));
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show_themed(ui, |ui| {
                    for (s, _) in &rows {
                        let needed = s.kind != SpotKind::Broadcast
                            && sdroxide_types::entity_name(&s.call)
                                .map(|n| !worked_entities.contains(n))
                                .unwrap_or(false);
                        if spot_row(ui, s, now, needed).clicked() {
                            clicked = Some((*s).clone());
                        }
                    }
                    if rows.is_empty() {
                        ui.add_space(8.0);
                        let msg = if query.is_empty() {
                            "no spots — enable a feed in ⚙ SETUP"
                        } else {
                            "nothing matches the search"
                        };
                        ui.label(RichText::new(msg).color(crate::theme::gray(120)));
                    }
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_spots = open;
        if open_setup {
            // Open the main Settings dialog on the Spots tab.
            self.show_settings = true;
            self.settings_tab = SettingsTab::Spots;
        }
        if let Some(s) = clicked {
            self.select_spot(&s, cmds);
        }
    }
}
