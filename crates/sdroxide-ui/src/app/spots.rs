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

/// How close a broadcast station has to be to the dial to be named on the
/// panadapter.
///
/// The schedule lists a **carrier**, and a receiver sitting on it is rarely
/// exactly on it — a few hundred hertz off is the same transmission. So this is
/// a "the same station" window, not a tuning tolerance: wide enough that a dial
/// parked beside a broadcaster still names it, narrow enough that the next
/// channel up does not get one too.
pub(in crate::app) const BROADCAST_TUNED_HZ: f64 = 1_000.0;
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

/// One band opening as a compact row — a state tag, the path, the surge
/// factor, the distinct caller count and how long it has held. Styled like a
/// spot row so the OPENINGS section reads as part of the same list rather than
/// a banner (the first version's horizontal strip left the window's
/// non-broadcast, non-list look, which read wrong against everything else in
/// it).
fn opening_row(ui: &mut egui::Ui, o: &sdroxide_types::BandOpening, now: i64) {
    let (state_tag, state_col) = match o.state {
        sdroxide_types::OpeningState::Opening => ("OPEN", crate::theme::CYAN()),
        sdroxide_types::OpeningState::Active => ("ACTIVE", crate::theme::GREEN()),
        sdroxide_types::OpeningState::Closing => ("CLOSING", crate::theme::gray(170)),
    };
    let gray = crate::theme::gray(140);
    let factor = o.factor.map(|f| format!("{f:.1}×")).unwrap_or_else(|| "∞×".to_string());
    egui::Frame::new()
        .fill(crate::theme::ROW_BG())
        .inner_margin(egui::Margin { left: 8, right: 6, top: 3, bottom: 3 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let col = |ui: &mut egui::Ui, w: f32, lbl: egui::Label| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
                    ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    )
                    .add(lbl);
                };
                col(
                    ui,
                    56.0,
                    egui::Label::new(RichText::new(state_tag).size(10.0).strong().color(state_col)),
                );
                col(
                    ui,
                    46.0,
                    egui::Label::new(RichText::new(o.band.label()).size(11.0).strong().color(gray)),
                );
                col(
                    ui,
                    126.0,
                    egui::Label::new(
                        RichText::new(format!("{} → {}", o.from_continent, o.to_continent))
                            .size(11.0)
                            .strong()
                            .color(crate::theme::TEXT_STRONG()),
                    ),
                );
                col(
                    ui,
                    62.0,
                    egui::Label::new(
                        RichText::new(format!("{factor}"))
                            .monospace()
                            .size(11.0)
                            .color(crate::theme::YELLOW()),
                    ),
                );
                col(
                    ui,
                    74.0,
                    egui::Label::new(
                        RichText::new(format!("{} calls", o.short_calls)).size(10.5).color(gray),
                    ),
                );
                col(
                    ui,
                    52.0,
                    egui::Label::new(
                        RichText::new(fmt_age(now - o.since_utc))
                            .size(10.5)
                            .color(crate::theme::gray(120)),
                    ),
                );
            })
            .response
            .on_hover_text(format!(
                "{} band path from {} to {}: the last 15-minute window surged past the \
                 path's 3-hour baseline — recent callers: {}",
                o.band.label(),
                o.from_continent,
                o.to_continent,
                if o.sample_calls.is_empty() {
                    "none left in the window".to_string()
                } else {
                    o.sample_calls.join(", ")
                },
            ));
        });
}

/// The SPOTS list pane: the "n of m" count once a search is active, then the
/// rows themselves (or the understated empty message). Shared by the split
/// layout (openings above) and the plain full-window list, so the two cannot
/// drift apart on what a row does.
fn spot_rows_pane(
    ui: &mut egui::Ui,
    rows: &[(&Spot, i32)],
    query: &str,
    visible: usize,
    now: i64,
    worked_entities: &std::collections::HashSet<String>,
    clicked: &mut Option<Spot>,
) {
    if !query.is_empty() {
        // Counted against what the chips let through, not against every spot
        // held — "3 of 5" when three categories are off would look like the
        // search had lost the rest.
        let (text, colour) = match rows.len() {
            0 => ("no match".to_string(), crate::theme::ALERT()),
            n => (format!("{n} of {visible}"), crate::theme::YELLOW()),
        };
        ui.label(RichText::new(text).color(colour).size(10.0));
    }
    let rows: Vec<&Spot> = rows.iter().map(|(s, _)| *s).collect();
    let any = !rows.is_empty();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_themed(ui, |ui| {
        for s in rows {
            let needed = s.kind != SpotKind::Broadcast
                && sdroxide_types::entity_name(&s.call)
                    .map(|n| !worked_entities.contains(n))
                    .unwrap_or(false);
            if spot_row(ui, s, now, needed).clicked() {
                *clicked = Some((*s).clone());
            }
        }
        if !any {
            ui.add_space(8.0);
            let msg = if query.is_empty() {
                "no spots — enable a feed in ⚙ SETUP"
            } else {
                "nothing matches the search"
            };
            ui.label(RichText::new(msg).color(crate::theme::gray(120)));
        }
    });
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
    /// Only the tuned broadcaster is named on the panadapter, and a `Spot` that
    /// is not it is not a spot — pinned as a pure function of the dial.
    ///
    /// The rule is bebepip's, from fork #69, and it has two halves: a station is
    /// shown when the dial is **on it**, and a dial 1 kHz away is a different
    /// station. The window is wide enough that a receiver parked beside a
    /// carrier still names it, because a schedule row is a carrier and a dial is
    /// rarely exactly on one — and narrow enough that the next channel up does
    /// not get a name it does not own.
    fn broadcast_named_at(dial_hz: f64, station_hz: f64) -> bool {
        (station_hz - dial_hz).abs() <= BROADCAST_TUNED_HZ
    }

    /// With ALL on, a favourite is named wherever it sits on the dial; without
    /// it the ±1 kHz gate applies to every broadcast station.
    pub(in crate::app) fn broadcast_visible_at(
        dial_hz: f64,
        station_hz: f64,
        call: &str,
        favs: &[String],
        all: bool,
    ) -> bool {
        if all && favs.iter().any(|n| n == call) {
            return true;
        }
        Self::broadcast_named_at(dial_hz, station_hz)
    }

    /// **The broadcast name is drawn once, however many sources carry it.**
    ///
    /// bebepip is having EiBi, Aoki and HFCC merged into one master store and
    /// says outright that duplicates in *that list* are fine — *"the resulting
    /// list will simply be long and somewhat repetitive"* — because the Schedule
    /// window is where the detail lives. He flagged the one place they are not:
    /// **the waterfall**, where a station listed by two sources prints its name
    /// twice, one over the other.
    ///
    /// So the dedupe is here and **nowhere else**. Both records stay in the
    /// store, both are editable and both export; only the panadapter collapses
    /// them to one label. Same broadcaster at the same frequency is the same
    /// thing to say twice.
    ///
    /// The key is name **and** frequency: two different broadcasters on one
    /// carrier are both on air and both are worth naming, and a station carried
    /// by three sources at three slightly different rounded frequencies still
    /// collapses, because a schedule row is a carrier and nobody publishes one to
    /// the hertz.
    pub(in crate::app) fn broadcast_label_dedupe(&self, spots: &[Spot]) -> Vec<Spot> {
        broadcast_label_dedupe(spots)
    }
}

/// The pure half of [`SdroxideApp::broadcast_label_dedupe`], which is a thin
/// wrapper because it never needed the app and a method that does not read its
/// receiver is a lie about what it needs.
pub(in crate::app) fn broadcast_label_dedupe(spots: &[Spot]) -> Vec<Spot> {
    {
        let mut seen: Vec<(String, i64)> = Vec::new();
        let mut out: Vec<Spot> = Vec::with_capacity(spots.len());
        for s in spots {
            if s.kind == SpotKind::Broadcast {
                let key = (s.call.clone(), (s.freq_hz / 10.0).round() as i64);
                if seen.contains(&key) {
                    continue;
                }
                seen.push(key);
            }
            out.push(s.clone());
        }
        out
    }
}

impl SdroxideApp {
    pub(in crate::app) fn spot_visible(&self, s: &Spot) -> bool {
        // "Who heard me" is a map overlay, not a station to tune: it never
        // belongs in the spot list or on the panadapter, where a dot placed at
        // the *reporter* would be a marker at the wrong end of the report.
        if s.kind == SpotKind::HeardMe {
            return false;
        }
        // SWL mode drops the ham feeds that need a licence to be useful — POTA
        // and SOTA — but keeps the receive-only networks (PSK Reporter, FreeDV
        // Reporter), the broadcast stations and the DX cluster, which are exactly
        // what a listener is here for. The cluster is a *listening* feed: a
        // listener wants to know what is being worked on the band, and it carries
        // no transmit obligation, so it is shown whether SWL mode is on or not
        // (F4NRM: "SWL ou pas, dans les 2 cas, le cluster doit s'afficher").
        if self.swl_mode() && matches!(s.kind, SpotKind::Pota | SpotKind::Sota) {
            return false;
        }
        // **Only the station you are tuned to, on the broadcast overlay.**
        //
        // bebepip's answer on fork #69, and it settles the request he opened the
        // issue with — *"a checkbox so only the station you are tuned to is
        // shown, rather than the whole band labelled."* He then said what it
        // should mean, and it is narrower than a checkbox: **the name on the
        // tuned frequency and nothing else**, no neighbouring frequencies, with
        // everything else — country, city, coordinates — left to the SCHEDULE
        // window, which is where that detail belongs.
        //
        // A band panel already labels a couple of dozen stations at once, and on
        // 11 m that is a wall of names over the passband you are trying to read.
        // One name, for the thing you are listening to.
        if s.kind == SpotKind::Broadcast {
            return Self::broadcast_visible_at(
                self.state.rx_freq_hz(),
                s.freq_hz,
                &s.call,
                &self.broadcast_favs,
                self.broadcast_all,
            );
        }
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

    /// Re-fetch **one** source, leaving the others' caches alone.
    ///
    /// Per-source because a single "download" over three sources reports one
    /// count for three downloads: a source that fails looks exactly like one that
    /// worked, and the fix for that is a control per source rather than a longer
    /// status line.
    pub(in crate::app) fn refetch_broadcast_source(
        &mut self,
        source: sdroxide_types::ScheduleSource,
    ) {
        if let Some(rx) = crate::app::persist::spawn_one_schedule_fetch(source) {
            self.broadcast_fetch = Some(rx);
            self.broadcast_fetch_status = None;
        } else {
            self.broadcast_fetch_status =
                Some(Err(format!("{source:?} could not be downloaded on this platform")));
        }
    }

    /// Re-read the operator's own station file and lay it over the schedule.
    pub(in crate::app) fn reload_broadcast_stations(&mut self) {
        self.broadcast = sdroxide_types::broadcast::with_utilities(
            crate::app::persist::load_broadcast_stations(),
        );
        self.broadcast_minute = -1;
    }

    /// Live network spots and the on-air broadcast stations, unfiltered.
    pub(in crate::app) fn all_spots(&self) -> impl Iterator<Item = &Spot> {
        self.spots.iter().chain(self.broadcast_spots.iter())
    }

    /// The "who heard me" reporters to draw, when the overlay is on: the
    /// [`SpotKind::HeardMe`] spots from the snapshot, cloned out. Empty when
    /// the overlay is off, so the map draws nothing.
    pub(in crate::app) fn heard_me_reporters(&self) -> Vec<Spot> {
        if !self.view.psk_heard_me {
            return Vec::new();
        }
        self.all_spots().filter(|s| s.kind == SpotKind::HeardMe).cloned().collect()
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
    pub(in crate::app) fn select_spot(&mut self, spot: &Spot, cmds: &mut Vec<Command>) {
        match spot.radio_mode() {
            Some(Mode::Cw) => {
                // The operator's *own* sidetone pitch, not the mode's 700 Hz
                // default: a CW spot is tuned a pitch below its signal so it
                // lands on the CW cursor, and that cursor sits wherever the
                // operator put it. The skimmer overlay already tunes this way
                // (`spectrum_view`); the SPOTS list used the default and so
                // tuned a 600 Hz operator ~100 Hz out (Thieu, 2.0.9.x).
                let pitch = f64::from(self.cw_pitch_hz());
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
    /// spots and broadcast stations, and the feed status line. The body carries
    /// its two asks — a spot picked, and the setup chip — back to the tail.
    pub(in crate::app) fn spots_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut clicked: Option<Spot> = None;
        let mut open_setup = false;
        let open =
            self.tool_window(ctx, "spots", "SPOTS", [580.0, 480.0], self.show_spots, |me, ui| {
                me.spots_body(ui, &mut clicked, &mut open_setup)
            });
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

    /// The spots list's body, split out so the shell can draw it in a window of
    /// its own.
    fn spots_body(&mut self, ui: &mut egui::Ui, clicked: &mut Option<Spot>, open_setup: &mut bool) {
        crate::chrome::window_body_bg(ui);
        let worked_entities = self.worked_entities().clone();
        let now = now_unix();
        self.refresh_broadcast_spots(now);
        // Cloned out of `self` because the rest of the body needs `&mut self`.
        let spots = self.merged_spots();
        // Chip order has to match `SpotKind::index`: the loop below indexes
        // `spot_kinds_shown` positionally. HeardMe has no chip: it is the map
        // overlay toggled from the map's own row, not a spot category — see
        // `spot_visible`, which always hides it from the list.
        let labels = [
            (SpotKind::DxCluster, "DX"),
            (SpotKind::Pota, "POTA"),
            (SpotKind::Sota, "SOTA"),
            (SpotKind::PskReporter, "PSK"),
            (SpotKind::FreeDv, "FREEDV"),
            (SpotKind::Broadcast, "BC"),
            (SpotKind::Rbn, "RBN"),
        ];
        ui.horizontal(|ui| {
            for (i, (kind, label)) in labels.iter().enumerate() {
                // SWL mode has no POTA / SOTA feed to show, so it shows no chip
                // for one. The DX cluster is a listening feed and is shown in
                // both modes, so its chip stays (F4NRM).
                if self.swl_mode() && matches!(kind, SpotKind::Pota | SpotKind::Sota) {
                    continue;
                }
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
            if crate::chrome::chip(ui, self.view.spots_openings, "OPENINGS")
                .on_hover_text(
                    "Band-opening detections: paths whose recent activity surged \
                     past their own 3-hour baseline",
                )
                .clicked()
            {
                self.view.spots_openings = !self.view.spots_openings;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::chrome::chip(ui, false, "KML")
                    .on_hover_text(
                        "Save the spots you can see as a KML file — Google Earth, and any \
                         other mapping tool that reads it. Only the filters switched on, and \
                         only the spots that carry a position, since a pin with no \
                         coordinates is a pin at the wrong end of the world.",
                    )
                    .clicked()
                {
                    // What is on screen, not the whole buffer: exporting all
                    // seven kinds when four chips are lit would hand the
                    // operator a file that does not match the window it came
                    // from.
                    let spots: Vec<Spot> =
                        self.spots.iter().filter(|s| self.spot_visible(s)).cloned().collect();
                    if spots.is_empty() {
                        self.kml_note = Some("No spots to save — the filters are all off".into());
                    } else {
                        let placed = spots.iter().filter(|s| s.loc.is_some()).count();
                        let kml = sdroxide_types::spots_to_kml(&spots);
                        let name = if placed == spots.len() {
                            "sdroxide-spots.kml".to_string()
                        } else {
                            format!("sdroxide-spots-{placed}-of-{}.kml", spots.len())
                        };
                        // Say what was dropped rather than letting a file
                        // quietly hold fewer pins than the window showed.
                        self.kml_note = (placed < spots.len()).then(|| {
                            format!(
                                "{placed} of {} saved — the rest carry no position and cannot \
                                 be pinned",
                                spots.len()
                            )
                        });
                        crate::download::save(&name, kml.as_bytes());
                    }
                }
                if crate::chrome::chip(ui, false, "⚙ SETUP")
                    .on_hover_text("Feeds, lookup & upload settings")
                    .clicked()
                {
                    *open_setup = true;
                }
            });
        });
        // **The KML result is a line of its own, not a member of the chip row.**
        //
        // It was drawn inside the row's right-to-left layout beside the two
        // buttons, which is where it shoved **⚙ SETUP** out of the window: a
        // right-to-left row places each widget from the right edge, so a label
        // of any width pushes the one after it off the left. And because the
        // label does not wrap, it also laid out past the window's clip — which
        // is why the text appeared to drop *behind* everything rather than
        // simply being cut off.
        if let Some(note) = self.kml_note.clone() {
            // A **ribbon**, per the operator: a yellow bar of its own with a ✕.
            // It was a small cyan line among the buttons, and being among them
            // was the whole of the fault — it pushed ⚙ SETUP sideways and
            // painted over the chips rather than being cut off, which read as a
            // rendering fault rather than as a message. A bar has its own line,
            // so it cannot reach the row above it, and it is dismissed for this
            // session rather than for ever: the next export says something new.
            if crate::chrome::ribbon(ui, &note, crate::theme::YELLOW()) {
                self.kml_note = None;
            }
        }
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
        // Upload and lookup results — the rolling `net_log`, which had
        // no reader anywhere. Collapsed to one row by default, since the
        // SPOTS window is about the spots; opened, it answers "did that
        // QSO upload, and if not, why".
        if !self.net_log.is_empty() {
            egui::CollapsingHeader::new(
                RichText::new(format!("NET LOG ({})", self.net_log.len()))
                    .size(11.0)
                    .color(crate::theme::CYAN_DIM()),
            )
            .id_salt("spots-net-log")
            .default_open(false)
            .show(ui, |ui| {
                for line in &self.net_log {
                    ui.label(RichText::new(line).monospace().size(10.5));
                }
            });
        }
        ui.separator();
        // Band-opening detections from the same feeds (adapted from
        // OpenHamClock): a path whose recent activity surged past its
        // own 3-hour baseline. Behind the OPENINGS chip, and split
        // from the spot list by a draggable handle, so a band surge
        // can be given most of the window or squeezed back to a
        // sliver. Strongest first — opening, then active, then closing
        // sloughing off. `avail_h` is everything left below this
        // point, and the openings share and the handle subtract from
        // it, leaving the rest to the spots.
        let show_openings = self.view.spots_openings && !self.band_openings.is_empty();
        let avail_h = ui.available_height().max(130.0);
        const HANDLE_H: f32 = 7.0;
        let openings_h = if show_openings {
            (avail_h * self.view.spots_openings_fraction)
                .clamp(70.0, (avail_h - HANDLE_H - 120.0).max(70.0))
        } else {
            0.0
        };
        // Filter by the category chips, then rank by how well each row
        // matched the query. With no query the natural frequency order
        // is kept; with one, the best matches come first, because the
        // whole point of typing is to get the wanted row to the top.
        let query = self.spot_search.trim();
        let visible: Vec<&Spot> = spots.iter().filter(|s| self.spot_visible(s)).collect();
        let mut rows: Vec<(&Spot, i32)> = visible
            .iter()
            .filter_map(|s| crate::fuzzy::score_terms(&spot_haystack(s), query).map(|sc| (*s, sc)))
            .collect();
        if !query.is_empty() {
            rows.sort_by_key(|r| std::cmp::Reverse(r.1));
        }
        if show_openings {
            let openings = self.band_openings.clone();
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), openings_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("OPENINGS")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for o in &openings {
                                opening_row(ui, o, now);
                                ui.add_space(2.0);
                            }
                        });
                },
            );
            let h =
                crate::chrome::split_handle(ui, egui::vec2(ui.available_width(), HANDLE_H), None);
            if h.dragged() {
                self.view.spots_openings_fraction =
                    ((openings_h + h.drag_delta().y) / avail_h.max(1.0)).clamp(0.08, 0.8);
            }
            let rest = (avail_h - openings_h - HANDLE_H).max(0.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), rest),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    spot_rows_pane(ui, &rows, query, visible.len(), now, &worked_entities, clicked);
                },
            );
        } else {
            spot_rows_pane(ui, &rows, query, visible.len(), now, &worked_entities, clicked);
        }
    }
}

#[cfg(test)]
mod broadcast_only_the_tuned_tests {
    use super::SdroxideApp;

    /// The rule bebepip asked for: **the name on the tuned frequency and nothing
    /// else.** He asked for it as a checkbox and then said what it should mean —
    /// no neighbouring frequencies — because a band panel labelling a dozen
    /// stations at once is a wall of names over the passband being listened to.
    #[test]
    fn only_the_station_you_are_tuned_to_is_named() {
        let dial = 9_580_000.0;
        assert!(SdroxideApp::broadcast_named_at(dial, 9_580_000.0), "on it");
        // Beside it is still it — a schedule row is a carrier and a dial is
        // rarely exactly on one.
        assert!(
            SdroxideApp::broadcast_named_at(dial, 9_580_400.0),
            "400 Hz off is the same station"
        );
        assert!(
            SdroxideApp::broadcast_named_at(dial, 9_579_600.0),
            "and the same on the other side"
        );
        // The next channel is not. Shortwave channels are 4–5 kHz apart, so a
        // 2 kHz-wide window in total has room for the dial to sit off the
        // carrier and still stops well short of the neighbour.
        assert!(!SdroxideApp::broadcast_named_at(dial, 9_582_000.0), "2 kHz up is another station");
        assert!(!SdroxideApp::broadcast_named_at(dial, 9_578_000.0), "and 2 kHz down");
        // The boundary, stated rather than felt: **±1 kHz inclusive**, so the
        // width is a number this test holds rather than a feeling in the code.
        assert!(
            SdroxideApp::broadcast_named_at(dial, 9_581_000.0),
            "exactly 1 kHz is the edge, and it is inside"
        );
        assert!(!SdroxideApp::broadcast_named_at(dial, 9_581_001.0), "1 Hz past it is out");
    }

    /// ALL off: a favourite 2 kHz off the dial is still unnamed, exactly as
    /// before.
    #[test]
    fn a_favourite_outside_the_window_is_unnamed_without_all() {
        let dial = 9_580_000.0;
        let favs = vec!["BBC World Service".to_string()];
        assert!(
            !SdroxideApp::broadcast_visible_at(
                dial,
                9_582_000.0,
                "BBC World Service",
                &favs,
                false
            ),
            "2 kHz off, ALL off: unnamed"
        );
    }

    /// ALL on: the same favourite is named wherever it sits.
    #[test]
    fn a_favourite_is_named_with_all_even_off_the_dial() {
        let dial = 9_580_000.0;
        let favs = vec!["BBC World Service".to_string()];
        assert!(
            SdroxideApp::broadcast_visible_at(dial, 9_582_000.0, "BBC World Service", &favs, true),
            "2 kHz off, ALL on, favourite: named"
        );
    }

    /// ALL on does not name a station that is not a favourite.
    #[test]
    fn a_non_favourite_stays_gated_even_with_all() {
        let dial = 9_580_000.0;
        let favs: Vec<String> = Vec::new();
        assert!(
            !SdroxideApp::broadcast_visible_at(dial, 9_582_000.0, "BBC World Service", &favs, true),
            "non-favourite off the dial stays unnamed"
        );
    }
}

#[cfg(test)]
mod broadcast_dedupe_tests {
    use sdroxide_types::{Spot, SpotKind};

    /// Every field `Spot` has but a broadcast label does not. It does not derive
    /// `Default`, which is the point: a spot is built once, by the code that has
    /// the values, and a test that invents half of one proves nothing.
    fn spot_shell() -> Spot {
        Spot {
            id: 0,
            kind: SpotKind::Broadcast,
            freq_hz: 0.0,
            call: String::new(),
            spotter: String::new(),
            mode: String::new(),
            comment: String::new(),
            reference: None,
            grid: None,
            loc: None,
            snr_db: None,
            when_utc: 0,
        }
    }

    fn bc(name: &str, hz: f64) -> Spot {
        Spot { kind: SpotKind::Broadcast, freq_hz: hz, call: name.into(), ..spot_shell() }
    }

    /// The fault bebepip raised himself, pinned: a merged master store lists the
    /// same station under EiBi, under Aoki and under HFCC, and the waterfall
    /// must print **one** name rather than three on top of each other.
    #[test]
    fn one_broadcaster_from_three_sources_is_named_once() {
        let spots = vec![
            bc("BBC World Service", 15_490_000.0),
            bc("BBC World Service", 15_490_000.0),
            bc("BBC World Service", 15_490_004.0),
        ];
        let kept = super::broadcast_label_dedupe(&spots);
        assert_eq!(kept.len(), 1, "three sources, one name: {kept:?}");
        assert_eq!(kept[0].call, "BBC World Service");
    }

    /// **Two different broadcasters on one carrier are both on air** and both are
    /// worth naming — the dedupe is by name *and* frequency, never by frequency
    /// alone. Two rows that differed only in frequency would be one station
    /// published imprecisely; two that differ in name are two facts.
    #[test]
    fn two_broadcasters_on_one_carrier_both_stay() {
        let kept = super::broadcast_label_dedupe(&[
            bc("Radio Prague", 5_010_000.0),
            bc("WRMI", 5_010_000.0),
        ]);
        assert_eq!(kept.len(), 2, "two names on one carrier are both true");
    }

    /// And the dedupe touches **nothing else**: a DX-cluster spot is never
    /// collapsed, whatever its call.
    #[test]
    fn other_kinds_are_never_collapsed() {
        let mut dx = bc("DL1ABC", 14_200_000.0);
        dx.kind = SpotKind::DxCluster;
        let kept = super::broadcast_label_dedupe(&[dx.clone(), dx.clone(), dx.clone()]);
        assert_eq!(kept.len(), 3, "a callsign reported three times is three reports");
    }
}

// No regression test here, and that is a finding rather than an omission.
//
// The fault was real and the operator saw it twice — docked and in its own
// window — but it is **not visible as ink outside the window**: egui clips the
// chip row, so what actually happens is the note painting *over* the chips and
// shoving them sideways *inside* the clip. Three versions of a test were written
// against that, measuring ink coordinates and then the chip row's layout, and
// all three passed on the broken code. A test that passes before and after a fix
// is testing nothing, so none of them was kept.
//
// The change itself is the one that cannot fail this way: the note is drawn by
// `chrome::ribbon`, which is a full-width bar on its own line, so there is no
// longer anything for it to be laid out beside.
