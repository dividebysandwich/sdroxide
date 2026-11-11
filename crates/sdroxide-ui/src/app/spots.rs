//! Network spots: which ones are shown, and the SPOTS window that lists them.
//!
//! Spots arrive from several feeds (DX cluster, POTA, SOTA, PSK Reporter,
//! FreeDV Reporter) plus the bundled broadcast schedule, and are merged into
//! one list filtered by kind, by the current view span, and by a fuzzy search.
//! Clicking one tunes the radio and pre-fills the logbook.

use crate::i18n::tr;
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
/// How close the dial must be to a spot's frequency for its row to show as
/// the one tuned: a clicked spot puts the dial on it exactly, and a few
/// hundred hertz leave room for a dial nudged after it.
const TUNED_HZ: f64 = 250.0;

/// Whether the dial at `dial_hz` is on `s`, within [`TUNED_HZ`].
fn spot_tuned(s: &Spot, dial_hz: f64) -> bool {
    (s.freq_hz - dial_hz).abs() <= TUNED_HZ
}

/// One row of the SPOTS list. `tuned`: the dial is on this spot; the row is
/// drawn in the selection background with a bar in the spot's colour down its
/// left edge, so the station being listened to stands out of the list.
// Why: HDZ 2026-11-07: "in Spots bei der Liste wird der Sender, der
//      abgestimmt wird und eingestellt und gewählt ist, nicht gehighlighted".
fn spot_row(
    ui: &mut egui::Ui,
    s: &Spot,
    now_utc: i64,
    needed: bool,
    tuned: bool,
) -> egui::Response {
    let kind_col = crate::theme::data_ink(crate::theme::spot_color(s.kind));
    let gray = crate::theme::gray(150);
    let inner = egui::Frame::new()
        .fill(if tuned { crate::theme::TOME_BG() } else { crate::theme::ROW_BG() })
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
                // A broadcast transmitter's power in a column of its own, so it
                // is not the part of the comment the truncation cuts off; empty
                // for the other kinds, so the columns stay aligned.
                // Why: HDZ 2026-11-07: "in der Listendarstellung die kW-Leistung
                //      bitte auch".
                let kw = spot_power_kw(s);
                col(
                    ui,
                    58.0,
                    egui::Label::new(
                        RichText::new(kw.map(fmt_kw).unwrap_or_default())
                            .monospace()
                            .size(11.0)
                            .color(gray),
                    ),
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
                // The power has its column; the comment without it.
                let comment = match kw {
                    Some(_) => s.comment.rsplit_once(" · ").map_or("", |(head, _)| head),
                    None => s.comment.as_str(),
                };
                let info = match &s.reference {
                    Some(r) if !comment.is_empty() => format!("{r} · {comment}"),
                    Some(r) => r.clone(),
                    None => comment.to_string(),
                };
                ui.add(egui::Label::new(RichText::new(info).size(11.0).color(gray)).truncate());
            });
        });
    if tuned {
        let r = inner.response.rect;
        ui.painter().rect_filled(
            egui::Rect::from_min_size(r.min, egui::vec2(3.0, r.height())),
            0.0,
            kind_col,
        );
    }
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
                    let msg = tr!("spots-transmissions", stations = stations.len());
                    self.broadcast = stations;
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
        // The automatic download only. "Download schedule now" in Settings →
        // Spots still fetches when it is off: that is the operator asking. The
        // day is not marked as checked while it is off, so switching it back on
        // checks on the next frame.
        if self.ui_settings.broadcast_schedule_download
            && self.broadcast_fetch.is_none()
            && day != self.broadcast_checked_day
        {
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
        self.broadcast = crate::app::persist::load_broadcast_stations();
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
        let night = if self.view.spots_map { self.night_texture(ctx) } else { None };
        let dial_hz = self.state.rx_freq_hz();
        let picked_ids: std::collections::HashSet<u64> =
            self.spots_map_pick.iter().map(|p| p.id).collect();
        // Chip order has to match `SpotKind::index`: the loop below indexes
        // `spot_kinds_shown` positionally.
        let labels = [
            (SpotKind::DxCluster, "DX"),
            (SpotKind::Pota, &tr!("spots-pota")),
            (SpotKind::Sota, &tr!("spots-sota")),
            (SpotKind::PskReporter, "PSK"),
            (SpotKind::FreeDv, &tr!("spots-freedv")),
            (SpotKind::Broadcast, "BC"),
        ];
        // The pick to bring into view, once LIST is shown.
        let focus_id = (self.spots_list_focus && !self.view.spots_map)
            .then(|| self.spots_map_pick.first().map(|p| p.id))
            .flatten();
        let tool_title = tr!("spots-spots");
        let mut tool_st = self.take_tool_state("SPOTS");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("SPOTS", &tool_title, [580.0, 480.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| {
                // Wide enough for the whole first row, IN VIEW and SETUP
                // included, from the first opening on and in every language;
                // narrower, egui cut the row off at the window's edge.
                // Why: HDZ 2026-11-11: "denke an die Mindestbreite beim ersten
                //      Öffnen, dass 'im Blick' mit sichtbar wird".
                let list_label = tr!("spots-view-list");
                let map_label = tr!("spots-view-map");
                let view_label = tr!("spots-view");
                let row: Vec<&str> = [list_label.as_str(), map_label.as_str()]
                    .into_iter()
                    .chain(labels.iter().map(|(_, l)| *l))
                    .chain([view_label.as_str()])
                    .collect();
                ui.set_min_width(header_row_width(ui, &row, &tr!("spots-setup")));
                ui.horizontal(|ui| {
                    // LIST | MAP: two views of the same filtered spots, as in
                    // the Public SDRs window. The chips right of them apply to
                    // both.
                    // Why: HDZ 2026-11-07: "Public SDR hat als zwei Toggles
                    //      vorne 'Liste, Karte', so können wir das doch auch
                    //      machen"; the map to answer "gibt's Koordinaten der
                    //      Sender auch, und könnte ich auf ein Label klicken, um
                    //      zur Senderlokalisation zu kommen?".
                    if crate::chrome::chip(ui, !self.view.spots_map, tr!("spots-view-list"))
                        .clicked()
                    {
                        self.view.spots_map = false;
                    }
                    if crate::chrome::chip(ui, self.view.spots_map, tr!("spots-view-map"))
                        .on_hover_text(tr!("spots-view-map-tip"))
                        .clicked()
                    {
                        self.view.spots_map = true;
                    }
                    ui.separator();
                    for (i, (kind, label)) in labels.iter().enumerate() {
                        let chip = crate::chrome::chip(ui, self.view.spot_kinds_shown[i], *label);
                        let chip = if *kind == SpotKind::Broadcast {
                            chip.on_hover_text(tr!(
                                "spots-longwave-shortwave-broadcast-stations-air"
                            ))
                        } else {
                            chip
                        };
                        if chip.clicked() {
                            self.view.spot_kinds_shown[i] = !self.view.spot_kinds_shown[i];
                        }
                    }
                    if crate::chrome::chip(ui, self.spot_in_view_only, tr!("spots-view"))
                        .on_hover_text(tr!("spots-only-spots-inside-panadapter-span"))
                        .clicked()
                    {
                        self.spot_in_view_only = !self.spot_in_view_only;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::chrome::chip(ui, false, tr!("spots-setup"))
                            .on_hover_text(tr!("spots-feeds-lookup-upload-settings"))
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
                            .hint_text(tr!("spots-call-station-site-frequency"))
                            .text_color(crate::theme::TEXT_STRONG()),
                    );
                    if !self.spot_search.trim().is_empty()
                        && ui.button("✕").on_hover_text(tr!("spots-clear-search")).clicked()
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
                        0 => (tr!("spots-no-match").to_string(), crate::theme::ALERT()),
                        n => (
                            tr!("spots-n-of-total", n = n, total = visible.len()),
                            crate::theme::YELLOW(),
                        ),
                    };
                    ui.label(RichText::new(text).color(colour).size(10.0));
                }
                if self.view.spots_map {
                    let shown: Vec<&Spot> = rows.iter().map(|(s, _)| *s).collect();
                    if let Some(s) = self.spots_map_body(ui, &shown, &night) {
                        clicked = Some(s);
                    }
                    return;
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show_themed(ui, |ui| {
                    for (s, _) in &rows {
                        let needed = s.kind != SpotKind::Broadcast
                            && sdroxide_types::entity_name(&s.call)
                                .map(|n| !worked_entities.contains(n))
                                .unwrap_or(false);
                        // The tuned row, and the map's picks: what is listened to and what
                        // was picked on the map stand out alike.
                        // Why: HDZ 2026-11-08: "muss sie in der Listenanzeige auch
                        //      hin und in den Fokus hinein".
                        let lit = spot_tuned(s, dial_hz) || picked_ids.contains(&s.id);
                        let row = spot_row(ui, s, now, needed, lit);
                        // A spot picked on the map or the waterfall is brought
                        // into view in the list.
                        // Why: HDZ 2026-11-08: "wenn auf der Karte eine
                        //      Connection geklickt wird, muss sie in der
                        //      Listenanzeige auch hin und in den Fokus hinein".
                        if focus_id == Some(s.id) {
                            row.scroll_to_me(Some(egui::Align::Center));
                        }
                        if row.clicked() {
                            clicked = Some((*s).clone());
                        }
                    }
                    if rows.is_empty() {
                        ui.add_space(8.0);
                        let msg = if query.is_empty() {
                            tr!("spots-no-spots-enable-feed-setup")
                        } else {
                            tr!("spots-nothing-matches-search")
                        };
                        ui.label(RichText::new(msg).color(crate::theme::gray(120)));
                    }
                });
            },
        );
        self.put_tool_state("SPOTS", tool_st);
        self.show_spots = open;
        if open_setup {
            // Open the main Settings dialog on the Spots tab.
            self.show_settings = true;
            self.settings_tab = SettingsTab::Spots;
        }
        if focus_id.is_some() {
            self.spots_list_focus = false;
        }
        if let Some(s) = clicked {
            // A row clicked in LIST is also the map's pick, for MAP; it is in
            // view already, so the list is not scrolled to it.
            self.pick_spot_on_map(&s);
            self.spots_list_focus = false;
            self.select_spot(&s, cmds);
        }
    }

    /// Mark `s` on the SPOTS window's map — picked in the list, on the map,
    /// or by its label on the waterfall — together with every other shown
    /// spot on the same channel (within [`SPOTS_MAP_CHANNEL_HZ`], at most
    /// [`SPOTS_MAP_MAX_PICKS`], `s` first): a broadcast frequency is often
    /// shared by two or three transmitters, and the one heard may be any of
    /// them. The map frames the picked ones; a double-click on the map hands
    /// the view back to the fit.
    // Why: HDZ 2026-11-07: "wenn du in der BC-Liste eine Frequenz anwählst,
    //      werden zwei drei Sender ausgewählt, in der Map nur ein Sender
    //      verbunden; du musst dann für drei Sender die Verbindungen knüpfen und
    //      die Daten, Richtung, Kennung usw. unten aufschreiben".
    pub(in crate::app) fn pick_spot_on_map(&mut self, s: &Spot) {
        let mut picks = vec![s.clone()];
        for o in self.merged_spots() {
            if picks.len() >= SPOTS_MAP_MAX_PICKS {
                break;
            }
            if o.id != s.id
                && (o.freq_hz - s.freq_hz).abs() <= SPOTS_MAP_CHANNEL_HZ
                && self.spot_visible(&o)
            {
                picks.push(o);
            }
        }
        let placed: Vec<(f64, f64)> = picks.iter().filter_map(|p| effective_loc(p).0).collect();
        frame_places(&mut self.spots_map_view, &placed);
        self.spots_map_pick = picks;
        self.spots_list_focus = true;
    }

    /// Let all the SPOTS map's picks go: no markers, no paths, and the view
    /// back to the fit (the whole world around home).
    fn release_spot_on_map(&mut self) {
        self.spots_map_pick.clear();
        self.spots_map_view.manual = false;
    }

    /// The SPOTS window's MAP view: one status line, the map with every shown
    /// spot that has a location, and one line about the picked spot.
    ///
    /// In: the spots the chips and the search let through, the grey-line
    /// texture. Out: a spot to tune to, when TUNE is pressed.
    fn spots_map_body(
        &mut self,
        ui: &mut egui::Ui,
        shown: &[&Spot],
        night: &Option<eframe::egui::TextureId>,
    ) -> Option<Spot> {
        // Inside the width the window has now; a map measuring itself by what
        // is free would widen the window frame by frame.
        ui.set_max_width(ui.available_width());
        // A spot its feed gives no place gets an approximate one where that
        // can be had honestly ([`approx_loc`]); it is drawn hollow.
        let placed_owned: Vec<Spot> = shown
            .iter()
            .filter_map(|s| effective_loc(s).0.map(|ll| Spot { loc: Some(ll), ..(*s).clone() }))
            .collect();
        let placed: Vec<&Spot> = placed_owned.iter().collect();
        let approx =
            shown.iter().filter(|s| s.loc.is_none()).count() - (shown.len() - placed.len());
        ui.horizontal(|ui| {
            if crate::chrome::chip(ui, !self.view.spots_map_names_off, tr!("spots-map-names"))
                .on_hover_text(tr!("spots-map-names-tip"))
                .clicked()
            {
                self.view.spots_map_names_off = !self.view.spots_map_names_off;
            }
            if crate::chrome::chip(ui, !self.view.spots_map_cities_off, tr!("spots-map-cities"))
                .on_hover_text(tr!("spots-map-cities-tip"))
                .clicked()
            {
                self.view.spots_map_cities_off = !self.view.spots_map_cities_off;
            }
            // Said rather than hidden: a spot with no location has no dot.
            ui.label(
                RichText::new(tr!(
                    "spots-map-status",
                    shown = placed.len(),
                    approx = approx,
                    unplaced = shown.len() - placed.len()
                ))
                .size(10.0)
                .color(crate::theme::gray(150)),
            );
            ui.label(
                RichText::new(tr!("spots-map-size-legend"))
                    .size(10.0)
                    .color(crate::theme::gray(150)),
            );
        });
        // The dots as stations: that is what the map hit-tests a click
        // against. Drawn over below, coloured by kind and sized by power.
        let stations: Vec<crate::digi_map::DigiStation> = placed
            .iter()
            .filter_map(|s| {
                s.loc.map(|(lat, lon)| crate::digi_map::DigiStation {
                    lat,
                    lon,
                    fade: 1.0,
                    call: Some(std::sync::Arc::from(s.call.as_str())),
                    band: None,
                })
            })
            .collect();
        let home = sdroxide_types::grid_to_latlon(&self.my_grid());
        // The line under the map first, as a bottom panel of the window, and
        // the map in all the height that leaves: the window's size is the
        // map's, in both directions (up to the map's own cap, as high as
        // wide), and nothing the map does feeds back into the window.
        // Why: HDZ 2026-11-07: "die Map-Begrenzung stößt in der Höhe an eine
        //      Grenze … und fortan kann ich nur noch breiter werden, das ist
        //      sehr awkward als Fensterverhalten".
        let pick = self.spots_map_pick.clone();
        let mut tune = None;
        egui::Panel::bottom(ui.id().with("spots_map_line"))
            .frame(egui::Frame::NONE)
            .resizable(false)
            .show(ui, |ui| {
                tune = spots_map_lines(ui, &pick, home);
            });
        let map_h = ui.available_height().max(crate::widgets::worldmap::MIN_HEIGHT);
        let map_w = ui.available_width();
        let map_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(map_w, map_h.min(map_w).max(crate::widgets::worldmap::MIN_HEIGHT)),
        );
        let hit = crate::widgets::worldmap::show(
            ui,
            &mut self.spots_map_view,
            home,
            None,
            None,
            None,
            &stations,
            &[],
            &[],
            None,
            *night,
            false,
            map_h,
            false,
            !self.view.spots_map_cities_off,
        );
        self.spots_map_dots(ui, map_rect, &placed, shown, home);
        // Connect and release on the map itself: a click on a dot picks it, a
        // click on the picked dot or a right-click anywhere on the map lets
        // it go, and the map hands its view back to the fit.
        // Why: HDZ 2026-11-07: "in der Karte konnte man nicht deselektieren,
        //      das wäre der Rechtsklick … in der Karte muss das Verbinden und
        //      Lösen auch gehen".
        let right_click = ui.input(|i| {
            i.pointer.button_clicked(egui::PointerButton::Secondary)
                && i.pointer.interact_pos().is_some_and(|p| map_rect.contains(p))
        });
        if right_click {
            self.release_spot_on_map();
        } else if let Some(s) = hit.and_then(|i| placed.get(i)) {
            let s = (*s).clone();
            if self.spots_map_pick.iter().any(|p| p.id == s.id) {
                self.release_spot_on_map();
            } else {
                self.pick_spot_on_map(&s);
            }
        }
        if hit.is_some() {
            // The line under the map was drawn before the click: draw again.
            ui.ctx().request_repaint();
        }
        tune
    }

    /// The spots on the SPOTS window's map, over what `worldmap::show` drew in
    /// `rect`: coloured by kind, a broadcast transmitter sized by its power
    /// ([`spot_dot_radius`]), the largest first so a small one beside it stays
    /// on top, and its name beside it when NAMES is on.
    // Why: HDZ 2026-11-07: "die Größe des Kullers mit der Sendeleistung?".
    fn spots_map_dots(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        placed: &[&Spot],
        shown: &[&Spot],
        home: Option<(f64, f64)>,
    ) {
        // The ids whose place is approximate: those their feed left unplaced.
        let approx_ids: std::collections::HashSet<u64> =
            shown.iter().filter(|s| s.loc.is_none()).map(|s| s.id).collect();
        if rect.width() < crate::widgets::worldmap::MIN_HEIGHT {
            return;
        }
        let v = &self.spots_map_view;
        let aspect = (rect.height() / rect.width()) as f64;
        let lat_span = v.lat_span(aspect);
        // The map's own projection (equirectangular, longitude wrapping).
        let project = |lat: f64, lon: f64| {
            let dlon = crate::widgets::worldmap::wrap180(lon - v.clon);
            egui::pos2(
                rect.left() + (0.5 + (dlon / v.lon_span) as f32) * rect.width(),
                rect.top() + (0.5 - ((lat - v.clat) / lat_span) as f32) * rect.height(),
            )
        };
        // (position, radius, colour, name, approximate)
        let mut dots: Vec<(egui::Pos2, f32, egui::Color32, &str, bool)> = placed
            .iter()
            .filter_map(|s| {
                let (lat, lon) = s.loc?;
                let ink = crate::theme::data_ink(crate::theme::spot_color(s.kind));
                Some((
                    project(lat, lon),
                    spot_dot_radius(spot_power_kw(s)),
                    ink,
                    s.call.as_str(),
                    approx_ids.contains(&s.id),
                ))
            })
            .collect();
        dots.sort_by(|a, b| b.1.total_cmp(&a.1));
        let p = ui.painter_at(rect);
        for (c, r, ink, _, approx) in &dots {
            if *approx {
                // Hollow: placed from its name or its country, not its site.
                p.circle_stroke(*c, *r, egui::Stroke::new(1.3, *ink));
            } else {
                p.circle_filled(*c, r + 1.5, crate::widgets::worldmap::alpha(*ink, 55.0));
                p.circle_filled(*c, *r, *ink);
            }
        }
        // The picks: a dotted great circle from home to each, a ring round
        // its dot and its number, the same number as its line under the map.
        let trail = crate::theme::map().trail;
        for (n, s) in self.spots_map_pick.iter().enumerate() {
            let Some(ll) = effective_loc(s).0 else { continue };
            if let Some(h) = home {
                for (lat, lon) in sdroxide_types::great_circle_points(h, ll, 90) {
                    p.circle_filled(
                        project(lat, lon),
                        1.2,
                        crate::widgets::worldmap::alpha(trail, 150.0),
                    );
                }
            }
            let c = project(ll.0, ll.1);
            let r = spot_dot_radius(spot_power_kw(s)) + 3.0;
            p.circle_stroke(c, r, egui::Stroke::new(1.5, trail));
            p.text(
                c + egui::vec2(-r - 2.0, -r),
                egui::Align2::RIGHT_BOTTOM,
                (n + 1).to_string(),
                egui::FontId::monospace(11.0),
                trail,
            );
        }
        if !self.view.spots_map_names_off {
            for (c, r, ink, name, _) in &dots {
                p.text(
                    *c + egui::vec2(r + 3.0, -1.0),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    egui::FontId::monospace(9.0),
                    crate::widgets::worldmap::alpha(*ink, 210.0),
                );
            }
        }
    }
}

/// Where an approximate place on the SPOTS map came from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Approx {
    /// A city named in the station's name, from the map's own city table.
    City(&'static str),
    /// The station's country: the transmitter site of that country that lies
    /// nearest to all its others (the medoid), named.
    Country(&'static str),
}

/// A spot's place for the map: its own, or an approximate one
/// ([`approx_loc`]) with where that came from.
fn effective_loc(s: &Spot) -> (Option<(f64, f64)>, Option<Approx>) {
    match s.loc {
        Some(ll) => (Some(ll), None),
        None => approx_loc(s).map_or((None, None), |(ll, a)| (Some(ll), Some(a))),
    }
}

/// An approximate place for a broadcast spot the schedule left unplaced:
/// first a city in the station's name (two words before one, at least four
/// letters, the most populous of that name), kept only within 2,500 km of the
/// country's sites so "Paris" in an American name stays out; else the
/// country's medoid site. `None` for other kinds and for a country the site
/// table does not know. Nothing is invented: both are real places from tables
/// the program ships.
// Why: HDZ 2026-11-07: "manche Sender haben keine Location, aber im Sendernamen
//      ist eine Stadt genannt, das sollte dann ja wenigstens zur Nearest-
//      Neighbor-Positionierung genügen". Measured on the built-in schedule:
//      2,259 of 9,360 lines unplaced, 1,624 of them Taiwan, whose site table
//      has 13 sites but no entry for the country alone; a city in the name
//      places 58.
fn approx_loc(s: &Spot) -> Option<((f64, f64), Approx)> {
    if s.kind != SpotKind::Broadcast {
        return None;
    }
    // The schedule writes the country code last in the reference.
    let country = s.reference.as_deref()?.rsplit(", ").next()?.trim();
    let medoid = country_medoids().get(country).copied();
    let words: Vec<&str> = s
        .call
        .split(|c: char| !(c.is_alphabetic() || c == '\'' || c == '-'))
        .filter(|w| !w.is_empty())
        .collect();
    for i in 0..words.len() {
        for w in [words.get(i..i + 2).map(|p| p.join(" ")), Some(words[i].to_string())]
            .into_iter()
            .flatten()
        {
            if w.chars().count() < 4 {
                continue;
            }
            if let Some(&(lat, lon, name)) = city_index().get(&w.to_lowercase()) {
                let near = medoid
                    .is_none_or(|(ll, _)| sdroxide_types::distance_km(ll, (lat, lon)) <= 2_500.0);
                if near {
                    return Some(((lat, lon), Approx::City(name)));
                }
            }
        }
    }
    medoid.map(|(ll, site)| (ll, Approx::Country(site)))
}

/// The map's city table by lower-case name, the most populous of each name.
fn city_index() -> &'static std::collections::HashMap<String, (f64, f64, &'static str)> {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<String, (f64, f64, &'static str)>> =
        std::sync::OnceLock::new();
    INDEX.get_or_init(|| {
        let mut best: std::collections::HashMap<String, (f64, f64, &'static str, u32)> =
            std::collections::HashMap::new();
        for c in crate::basemap::cities() {
            let e = best.entry(c.name.to_lowercase()).or_insert((c.lat, c.lon, c.name, c.pop));
            if c.pop > e.3 {
                *e = (c.lat, c.lon, c.name, c.pop);
            }
        }
        best.into_iter().map(|(k, v)| (k, (v.0, v.1, v.2))).collect()
    })
}

/// Each country's medoid transmitter site from the schedule's site table: the
/// site with the least summed distance to the country's others, with its name.
fn country_medoids() -> &'static std::collections::HashMap<String, ((f64, f64), &'static str)> {
    static MEDOIDS: std::sync::OnceLock<
        std::collections::HashMap<String, ((f64, f64), &'static str)>,
    > = std::sync::OnceLock::new();
    MEDOIDS.get_or_init(|| {
        let mut by_country: std::collections::HashMap<&str, Vec<(f64, f64, &'static str)>> =
            std::collections::HashMap::new();
        for (key, (name, lat, lon)) in &sdroxide_types::broadcast::codes().sites {
            let c = key.split('-').next().unwrap_or(key);
            by_country.entry(c).or_default().push((*lat, *lon, name.as_str()));
        }
        by_country
            .into_iter()
            .filter_map(|(c, sites)| {
                let cost = |a: &(f64, f64, &str)| -> f64 {
                    sites.iter().map(|b| sdroxide_types::distance_km((a.0, a.1), (b.0, b.1))).sum()
                };
                let m = sites.iter().min_by(|a, b| cost(a).total_cmp(&cost(b)))?;
                Some((c.to_string(), ((m.0, m.1), m.2)))
            })
            .collect()
    })
}

/// How close two spots' frequencies are to count as one channel on the SPOTS
/// map: picking one picks the others.
const SPOTS_MAP_CHANNEL_HZ: f64 = 250.0;
/// At most this many spots picked at once.
const SPOTS_MAP_MAX_PICKS: usize = 6;

/// Frame `places` on the map: centred on them and wide enough for all, with
/// the margin the auto-fit leaves; one place is centred at the current zoom.
/// Held until a double-click hands the view back.
fn frame_places(view: &mut crate::widgets::worldmap::MapView, places: &[(f64, f64)]) {
    let Some(&(lat0, lon0)) = places.first() else { return };
    if places.len() == 1 {
        view.centre_on(lat0, lon0);
        return;
    }
    let wrap = crate::widgets::worldmap::wrap180;
    // Longitudes about the first, so a set across the date line stays one.
    let dl: Vec<f64> = places.iter().map(|p| wrap(p.1 - lon0)).collect();
    let (lo, hi) = dl.iter().fold((0.0f64, 0.0f64), |a, d| (a.0.min(*d), a.1.max(*d)));
    let (la_lo, la_hi) =
        places.iter().fold((90.0f64, -90.0f64), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    view.centre_on((la_lo + la_hi) / 2.0, lon0 + (lo + hi) / 2.0);
    // The latitude spread counted double: the map is about twice as wide as
    // high.
    view.lon_span = ((hi - lo).max(2.0 * (la_hi - la_lo)) * 1.4).clamp(30.0, 360.0);
}

/// The lines under the SPOTS map: one per picked spot, numbered as its ring
/// on the map, with its name, place, power, distance and bearing and TUNE; or
/// how to pick. Out: the spot to tune to.
fn spots_map_lines(ui: &mut egui::Ui, picks: &[Spot], home: Option<(f64, f64)>) -> Option<Spot> {
    let mut tune = None;
    if picks.is_empty() {
        ui.label(RichText::new(tr!("spots-map-hint")).size(11.0).color(crate::theme::gray(130)));
        return None;
    }
    for (n, s) in picks.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}", n + 1))
                    .monospace()
                    .size(11.0)
                    .color(crate::theme::map().trail),
            );
            if crate::chrome::chip(ui, false, tr!("spots-map-tune")).clicked() {
                tune = Some(s.clone());
            }
            // The power after the place, where the schedule gives one.
            // Why: HDZ 2026-11-07: "wenn wir einen Sender angeklickt haben,
            //      die Leistung mit rein in die Info".
            let place = match (s.reference.clone(), spot_power_kw(s)) {
                (Some(r), Some(kw)) => format!("{r} · {}", fmt_kw(kw)),
                (Some(r), None) => r,
                (None, Some(kw)) => fmt_kw(kw),
                (None, None) => String::new(),
            };
            // An approximate place says where it came from, after a "≈".
            let (loc, how) = effective_loc(s);
            let place = match how {
                Some(Approx::City(city)) => {
                    format!("{place} · {}", tr!("spots-map-approx-city", city = city))
                }
                Some(Approx::Country(site)) => {
                    format!("{place} · {}", tr!("spots-map-approx-country", site = site))
                }
                None => place,
            };
            let text = match (loc, home) {
                (Some(ll), Some(h)) => tr!(
                    "spots-map-pick",
                    name = s.call.clone(),
                    place = place,
                    km = format!("{:.0}", sdroxide_types::distance_km(h, ll)),
                    deg = format!("{:.0}", sdroxide_types::bearing_deg(h, ll))
                ),
                _ => tr!("spots-map-pick-nowhere", name = s.call.clone()),
            };
            let line = format!("{:.1} kHz · {} · {text}", s.freq_hz / 1e3, s.mode);
            ui.label(RichText::new(line).size(11.0).color(crate::theme::TEXT_STRONG()));
        });
    }
    tune
}

/// A broadcast spot's transmitter power, kW, read back from its comment, where
/// [`sdroxide_types::BroadcastStation::to_spot`] writes it as the last part,
/// "… · 500 kW". `None` for other kinds and where the schedule gives none.
fn spot_power_kw(s: &Spot) -> Option<f64> {
    if s.kind != SpotKind::Broadcast {
        return None;
    }
    let last = s.comment.rsplit(" · ").next()?;
    last.strip_suffix(" kW")?.trim().parse::<f64>().ok().filter(|kw| *kw > 0.0)
}

/// A power as the list and the map line show it: "500 kW", "0.5 kW".
fn fmt_kw(kw: f64) -> String {
    if kw < 1.0 { format!("{kw:.1} kW") } else { format!("{kw:.0} kW") }
}

/// The dot radius on the SPOTS map, points: 2.2 for a spot without a power,
/// and growing with the power's logarithm, from 2.2 at 1 kW to 6.1 at 1000 kW,
/// because broadcast powers span three decades and an area in proportion would
/// cover Europe with one dot.
fn spot_dot_radius(kw: Option<f64>) -> f32 {
    match kw {
        Some(kw) => (2.2 + 1.3 * kw.max(1.0).log10() as f32).min(6.5),
        None => 2.2,
    }
}

#[cfg(test)]
mod spots_map_tests {
    use super::*;

    /// Proves: the power comes back out of the comment `to_spot` writes, and
    /// a spot of another kind or without a power has none.
    #[test]
    fn a_broadcast_spot_gives_back_its_power() {
        let mut s = Spot {
            id: 1,
            kind: SpotKind::Broadcast,
            freq_hz: 9_650_000.0,
            call: "Test".into(),
            spotter: String::new(),
            mode: "AM".into(),
            comment: "G · Eu · 500 kW".into(),
            reference: None,
            grid: None,
            loc: Some((52.6, 12.9)),
            when_utc: 0,
            snr_db: None,
        };
        assert_eq!(spot_power_kw(&s), Some(500.0));
        s.comment = "G · Eu".into();
        assert_eq!(spot_power_kw(&s), None);
        s.comment = "250 kW".into();
        assert_eq!(spot_power_kw(&s), Some(250.0));
        s.kind = SpotKind::DxCluster;
        assert_eq!(spot_power_kw(&s), None);
        assert_eq!(fmt_kw(500.0), "500 kW");
        assert_eq!(fmt_kw(0.5), "0.5 kW");
    }

    /// Proves: several picks are framed together, across the date line too,
    /// and a single one is only centred.
    #[test]
    fn the_picks_are_framed_together() {
        let mut v = crate::widgets::worldmap::MapView::default();
        frame_places(&mut v, &[(52.6, 12.9), (45.0, 25.0), (40.0, 117.0)]);
        assert!((v.clon - 64.95).abs() < 0.01, "centre lon {}", v.clon);
        assert!((v.lon_span - 145.74).abs() < 0.01, "span {}", v.lon_span);
        assert!(v.manual);
        let mut w = crate::widgets::worldmap::MapView::default();
        frame_places(&mut w, &[(10.0, 179.0), (12.0, -179.0)]);
        assert!(w.clon.abs() > 179.0, "across the date line, centre {}", w.clon);
        assert_eq!(w.lon_span, 30.0);
        let mut one = crate::widgets::worldmap::MapView::default();
        frame_places(&mut one, &[(52.6, 12.9)]);
        assert!((one.clat - 52.6).abs() < 1e-9 && (one.clon - 12.9).abs() < 1e-9);
        assert_eq!(one.lon_span, 360.0, "a single pick keeps the zoom");
    }

    /// Proves: an unplaced broadcast spot is placed by a city in its name when
    /// that city is near its country's sites, else at its country's medoid
    /// site; another kind, or an unknown country, stays unplaced.
    #[test]
    fn an_unplaced_station_gets_an_honest_approximate_place() {
        let mk = |call: &str, reference: &str| Spot {
            id: 1,
            kind: SpotKind::Broadcast,
            freq_hz: 6_000_000.0,
            call: call.into(),
            spotter: String::new(),
            mode: "AM".into(),
            comment: String::new(),
            reference: Some(reference.into()),
            grid: None,
            loc: None,
            when_utc: 0,
            snr_db: None,
        };
        let (ll, a) = approx_loc(&mk("Radio Tunis Chaine Int.", "TUN")).expect("Tunis");
        assert_eq!(a, Approx::City("Tunis"));
        assert!((ll.0 - 36.8).abs() < 0.5 && (ll.1 - 10.2).abs() < 0.5, "{ll:?}");
        let (ll, a) = approx_loc(&mk("Central Weather Bureau", "TWN")).expect("Taiwan");
        assert!(matches!(a, Approx::Country(_)), "{a:?}");
        assert!((21.0..26.5).contains(&ll.0) && (119.0..122.5).contains(&ll.1), "{ll:?}");
        // A city of that name far from the country is not taken.
        let (_, a) = approx_loc(&mk("Radio Paris Texas", "TWN")).expect("Taiwan");
        assert!(matches!(a, Approx::Country(_)), "{a:?}");
        assert!(approx_loc(&mk("Nowhere", "ZZZ")).is_none());
        let mut dx = mk("Radio Tunis", "TUN");
        dx.kind = SpotKind::DxCluster;
        assert!(approx_loc(&dx).is_none());
    }

    /// Proves: the dot grows with power, a decade at a time, within bounds.
    #[test]
    fn the_dot_grows_with_the_power() {
        assert_eq!(spot_dot_radius(None), 2.2);
        assert_eq!(spot_dot_radius(Some(1.0)), 2.2);
        assert!((spot_dot_radius(Some(1000.0)) - 6.1).abs() < 1e-4);
        assert!(spot_dot_radius(Some(10.0)) < spot_dot_radius(Some(100.0)));
        assert!(spot_dot_radius(Some(1e6)) <= 6.5);
    }
}

#[cfg(test)]
mod spot_tuned_tests {
    use super::*;

    /// Proves: the row of the spot the dial sits on counts as tuned, a
    /// neighbour a channel away does not.
    #[test]
    fn the_spot_under_the_dial_is_the_tuned_one() {
        let s = Spot {
            id: 1,
            kind: SpotKind::Broadcast,
            freq_hz: 9_650_000.0,
            call: "Test".into(),
            spotter: String::new(),
            mode: "AM".into(),
            comment: String::new(),
            reference: None,
            grid: None,
            loc: None,
            when_utc: 0,
            snr_db: None,
        };
        assert!(spot_tuned(&s, 9_650_000.0));
        assert!(spot_tuned(&s, 9_650_200.0));
        assert!(!spot_tuned(&s, 9_655_000.0));
    }
}

/// How wide the SPOTS window's first row is laid out: the chips left to right
/// with one separator after the first two (LIST | MAP), and the setup chip
/// right-aligned after them. The window's minimum width, so nothing in the row
/// is cut off.
fn header_row_width(ui: &egui::Ui, left: &[&str], right: &str) -> f32 {
    let gap = ui.spacing().item_spacing.x;
    let chips: f32 =
        left.iter().chain([&right]).map(|l| crate::chrome::chip_width(ui, l, None)).sum();
    // Every item after the first sits one gap on, the separator among them,
    // and egui's separator takes its style's spacing as its width.
    let separator = ui.style().separator_style(&Default::default(), Default::default()).spacing;
    chips + separator + gap * (left.len() + 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proves: the minimum width the SPOTS window asks for holds its first row
    /// as egui lays it out — LIST, MAP, a separator, the six source chips,
    /// IN VIEW and, right-aligned, SETUP — so IN VIEW is not cut off when the
    /// window first opens (HDZ 2026-11-11). Laid out in a window exactly that
    /// wide, the last chip of the row must end inside it, with the German
    /// labels, the longest set.
    #[test]
    fn the_spots_window_opens_wide_enough_for_its_first_row() {
        let left = ["LISTE", "KARTE", "DX", "POTA", "SOTA", "PSK", "FREEDV", "BC", "IM BLICK"];
        let right = "⚙ EINRICHTUNG";
        let ctx = egui::Context::default();
        let mut checked = false;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let w = header_row_width(ui, &left, right);
            let (_, rect) = ui.allocate_space(egui::vec2(w, 30.0));
            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            let mut last_left = 0.0f32;
            let mut setup = egui::Rect::NOTHING;
            row.horizontal(|ui| {
                for (i, l) in left.iter().enumerate() {
                    last_left = crate::chrome::chip(ui, false, *l).rect.right();
                    if i == 1 {
                        ui.separator();
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    setup = crate::chrome::chip(ui, false, right).rect;
                });
            });
            assert!(
                last_left <= setup.left() + 0.5,
                "IN VIEW ends at {last_left}, SETUP begins at {}",
                setup.left()
            );
            assert!(
                setup.right() <= rect.right() + 0.5,
                "SETUP ends at {} past {}",
                setup.right(),
                rect.right()
            );
            // Exactly as wide as needed: between IN VIEW and SETUP egui keeps
            // one item gap, no more and no less.
            let gap = setup.left() - last_left;
            let want = ui.spacing().item_spacing.x;
            assert!(
                (gap - want).abs() < 0.5,
                "{gap} pt between IN VIEW and SETUP, {want} expected"
            );
            checked = true;
        })
        .drop_without_applying_deltas();
        assert!(checked);
    }
}
