//! The AIS panel: what is on the water, and where.
//!
//! Two things a watchkeeper watches, so two columns:
//!
//! - **VESSELS** — every station being tracked, newest activity first, with the
//!   detail card for whichever one is selected pinned below the list.
//! - **CHART** — the same stations as a marine traffic picture. See
//!   [`crate::ais_map`].
//!
//! No third column, because unlike APRS there is nothing to say back: this is a
//! receive-only safety service and the ships are not listening to us.
//!
//! # The header says what the receiver is doing
//!
//! An empty vessel list has four quite different causes — nothing afloat within
//! range, a receiver on the wrong frequency, a stream too narrow to hold the
//! channels, and a receiver a few kilohertz off frequency — and only one of
//! them is anything to do with the decoder. The header carries the slot and
//! message counters, the per-channel state, and, where the decoder has measured
//! one, **how far off frequency the ships are**: that last is the difference
//! between "there are no ships" and "set the front end's frequency correction",
//! and nothing else in the program can tell an operator which.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, RichText};
use sdroxide_types::{AisKind, AisStatus, AisVessel, Command};

use crate::app::SdroxideApp;
use crate::app::util::fmt_age;
use crate::theme;
use crate::theme::ThemedScroll;

/// How the vessel table is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum AisSort {
    /// Most recently heard first. The default: on a display of things that
    /// move, "what just changed" is the question a list answers.
    #[default]
    Heard,
    Name,
    Speed,
    /// Nearest first. Only offered once the operator's own position is known —
    /// there is no distance without one.
    Range,
    Signal,
}

/// Below this the trailing columns come off the table. A row that has run out
/// of room prints its columns on top of each other, which is worse than not
/// printing them.
const NARROW_W: f32 = 340.0;

/// Above this there is room for the signal column too.
const WIDE_W: f32 = 440.0;

/// Past this the panel says the receiver is off frequency rather than leaving
/// the operator to conclude the sea is empty.
///
/// A GMSK signal is decoded out to about ±5 kHz — see
/// `sdroxide_ais::demod::CHANNEL_CUTOFF_HZ` — so two is where it is worth
/// mentioning and well before anything is being lost.
const OFFSET_WARN_HZ: f32 = 2_000.0;

impl SdroxideApp {
    pub(in crate::app) fn ais_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: AisStatus = match self.ais_status.as_ref() {
            Some(s) => (**s).clone(),
            None => {
                ui.label(RichText::new(tr!("ais-starting-ais-decoder")).weak());
                return;
            }
        };
        let now = crate::time::now_unix();
        let content_bottom = ui.cursor().top() + panel_h - 26.0;

        self.ais_header(ui, cmds, &st);
        ui.add_space(3.0);

        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let full_w = ui.available_width();

        // The list keeps a draggable share of the width. The floor is the
        // narrow table; the ceiling leaves the chart enough to still be a
        // chart rather than a strip.
        const HANDLE_W: f32 = 7.0;
        let list_w = (full_w * self.view.ais_split_fraction)
            .clamp(240.0, (full_w - HANDLE_W - 200.0).max(240.0));

        ui.horizontal_top(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(if pane.is_some() { full_w } else { list_w }, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.ais_list(ui, &st, now, avail_h),
                );
            }
            if pane.is_none() {
                let h = crate::chrome::split_handle(ui, egui::vec2(HANDLE_W, avail_h), None);
                if h.dragged() {
                    self.view.ais_split_fraction =
                        ((list_w + h.drag_delta().x) / full_w.max(1.0)).clamp(0.15, 0.85);
                }
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| self.ais_map_pane(ui, &st, now, avail_h));
            }
        });
    }

    /// Where the receiver is looking, what the decoder is finding, and the one
    /// button that fixes the commonest problem.
    ///
    /// Every volatile readout occupies a fixed-width slot, for the reason the
    /// ADS-B header's do: this is a `horizontal_wrapped`, and a number that
    /// grows a digit tips the whole tail onto a second line and moves every
    /// pane below it.
    fn ais_header(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, st: &AisStatus) {
        let dial = self.state.rx_freq_hz();
        let on_channel = (dial - sdroxide_types::AIS_PLAN_CENTER_HZ).abs() < 60_000.0
            && st.unavailable.is_none();

        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(22.0);
            ui.label(RichText::new("AIS").size(11.0).strong().color(theme::CYAN()));
            ui.label(RichText::new("161.975 / 162.025 MHz").weak().size(10.5));

            // The one preset there is. A chip rather than a note, because the
            // fix for "nothing is decoding" is nearly always this.
            if crate::chrome::chip(ui, on_channel, "162.000")
                .on_hover_text(tr!("ais-tune-between-two-ais-channels"))
                .clicked()
            {
                cmds.push(Command::SetVfo {
                    vfo: self.state.active_vfo,
                    hz: sdroxide_types::AIS_PLAN_CENTER_HZ,
                });
            }

            ui.separator();
            slot(ui, 74.0, &tr!("ais-vessels-3", vessels = st.vessels.len()), theme::CYAN());
            slot(ui, 84.0, &tr!("ais-messages", messages = count(st.messages)), theme::gray(150));
            // A high slot count with no messages is the honest picture of a
            // band busy with something the decoder cannot read. Worth showing
            // rather than leaving the panel looking broken.
            slot(ui, 68.0, &tr!("ais-slots", bursts = count(st.bursts)), theme::gray(120));
            slot(ui, 78.0, &tr!("ais-bad-fcs", bad_fcs = count(st.bad_fcs)), theme::gray(120));

            // Which channels are alive, as two chips: a ship alternates between
            // them, so one dark channel halves every vessel's reporting rate
            // and looks exactly like a quiet sea.
            for c in &st.channels {
                let tip = match &c.reason {
                    Some(r) => tr!(
                        "ais-ais-mhz",
                        value = c.label,
                        mhz = format!("{:.3}", c.freq_hz / 1e6),
                        value2 = r
                    ),
                    None => tr!(
                        "ais-ais-mhz-slots-messages-floor",
                        value = c.label,
                        mhz = format!("{:.3}", c.freq_hz / 1e6),
                        bursts = c.bursts,
                        messages = c.messages,
                        floor_dbfs = format!("{:.0}", c.floor_dbfs)
                    ),
                };
                crate::chrome::chip(ui, c.live, tr!("ais-ais", value = c.label)).on_hover_text(tip);
            }

            if st.window_rate_hz > 0.0 {
                slot(
                    ui,
                    140.0,
                    &format!(
                        "{e6:.3} MHz / {e3:.0} kHz",
                        e6 = st.window_center_hz / 1e6,
                        e3 = st.window_rate_hz / 1e3
                    ),
                    theme::gray(120),
                );
            }

            ui.separator();
            if crate::chrome::chip(ui, self.show_ais_setup, tr!("ais-setup"))
                .on_hover_text(tr!("ais-channels-timeouts-trail-length-how"))
                .clicked()
            {
                self.show_ais_setup = !self.show_ais_setup;
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
                        tr!("ais-tune", mhz = format!("{:.3}", hz / 1e6)),
                    )
                    .clicked()
                {
                    cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
                }
            });
        }
        // Running, but on half the plan or too few samples a bit. Said out loud
        // because the symptom — vessels reporting at half the rate — is exactly
        // what a quiet sea looks like.
        if let Some(why) = &st.degraded {
            ui.label(RichText::new(why).size(10.5).color(theme::YELLOW()));
        }
        // The one diagnosis nothing else in the program can make. A frequency
        // discriminator measures the carrier offset for free, and every ship
        // being three kilohertz off in the same direction is this receiver.
        if let Some(off) = st.offset_hz.filter(|o| o.abs() >= OFFSET_WARN_HZ) {
            ui.label(
                RichText::new(tr!(
                    "ais-ships-heard-are-khz-off",
                    khz = format!("{:+.1}", off / 1e3),
                    mhz = format!(
                        "{:+.0}",
                        f64::from(off) / sdroxide_types::AIS_PLAN_CENTER_HZ * 1e6
                    )
                ))
                .size(10.5)
                .color(theme::YELLOW()),
            );
        }
    }

    /// The vessel list, with the detail card pinned to the bottom of the
    /// column.
    ///
    /// Pinned rather than laid out after the list, for the reason the ADS-B
    /// card is: on a quiet coast it would otherwise sit half way up the panel
    /// and jump every time another ship came into range.
    fn ais_list(&mut self, ui: &mut egui::Ui, st: &AisStatus, now: i64, avail_h: f32) {
        let home = self.ais_home();
        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(20.0);
            ui.label(RichText::new(tr!("ais-vessels")).strong().size(10.5).color(theme::CYAN()));
            ui.add(
                egui::TextEdit::singleline(&mut self.ais_filter)
                    .hint_text("filter")
                    .desired_width(70.0),
            );
        });

        let filter = self.ais_filter.trim().to_ascii_uppercase();
        let mut rows: Vec<&AisVessel> = st
            .vessels
            .iter()
            .filter(|v| {
                filter.is_empty()
                    || v.name.to_ascii_uppercase().contains(&filter)
                    || v.call_sign.to_ascii_uppercase().contains(&filter)
                    || v.destination.to_ascii_uppercase().contains(&filter)
                    || v.mmsi.to_string().contains(&filter)
            })
            .collect();
        sort_rows(&mut rows, self.ais_sort, self.ais_sort_desc, home);

        let selected = self.ais_map.selected;
        let card = selected.and_then(|m| st.vessels.iter().find(|v| v.mmsi == m));
        const HANDLE_H: f32 = 7.0;
        let card_h = if card.is_some() {
            (avail_h * self.view.ais_card_fraction).clamp(90.0, (avail_h - 120.0).max(90.0))
        } else {
            0.0
        };
        let list_h =
            (avail_h - card_h - 46.0 - if card.is_some() { HANDLE_H } else { 0.0 }).max(48.0);

        let drop_map_s = self.state.ais.drop_map_s;
        // Outside the scroll area, so a busy estuary does not scroll the column
        // headings off the top of the table it is describing — and so they stay
        // where they can be clicked to re-order it.
        ais_head_row(ui, home.is_some(), &mut self.ais_sort, &mut self.ais_sort_desc);
        let mut pick = None;
        egui::ScrollArea::vertical()
            .id_salt("ais-vessels")
            .max_height(list_h)
            .min_scrolled_height(list_h)
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if rows.is_empty() {
                    ui.label(RichText::new(tr!("ais-nothing-heard-yet")).weak());
                }
                for (i, v) in rows.iter().enumerate() {
                    if ais_row(ui, v, now, selected, home, drop_map_s, i) {
                        pick = Some(v.mmsi);
                    }
                }
            });
        if let Some(mmsi) = pick {
            self.ais_map.selected = (selected != Some(mmsi)).then_some(mmsi);
        }

        if let Some(v) = card {
            let w = ui.available_width();
            let h = crate::chrome::split_handle(ui, egui::vec2(w, HANDLE_H), None);
            if h.dragged() {
                self.view.ais_card_fraction =
                    ((card_h - h.drag_delta().y) / avail_h.max(1.0)).clamp(0.12, 0.8);
            }
            self.ais_card(ui, v, now, card_h, home);
        }
    }

    /// Everything one station has said.
    fn ais_card(
        &mut self,
        ui: &mut egui::Ui,
        v: &AisVessel,
        now: i64,
        h: f32,
        home: Option<(f64, f64)>,
    ) {
        egui::ScrollArea::vertical()
            .id_salt("ais-card")
            .max_height(h)
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(v.label())
                            .monospace()
                            .strong()
                            .size(13.0)
                            .color(if v.is_alarm() { theme::HAZARD() } else { theme::YELLOW() }),
                    );
                    ui.label(RichText::new(v.mmsi.to_string()).monospace().size(10.5).weak());
                    ui.label(RichText::new(v.kind.tr()).size(10.5).color(theme::CYAN_DIM()));
                    if v.virtual_aid {
                        ui.label(
                            RichText::new(tr!("ais-virtual-nothing-there"))
                                .size(10.5)
                                .color(theme::HAZARD()),
                        );
                    }
                    if v.lat.is_some() && v.pos_stale(now, self.state.ais.drop_map_s) {
                        ui.label(
                            RichText::new(tr!("ais-position-stale"))
                                .size(10.5)
                                .color(theme::HAZARD())
                                .italics(),
                        );
                    }
                });
                if v.kind == AisKind::Sart {
                    ui.label(
                        RichText::new(tr!("ais-search-rescue-transmitter-man-overboard"))
                            .strong()
                            .color(theme::HAZARD()),
                    );
                }

                let mut rows: Vec<(String, String)> = Vec::new();
                if let Some(t) = v.type_label() {
                    let hazard = v.ship_type.and_then(sdroxide_types::ship_type_hazard);
                    rows.push((
                        tr!("ais-row-type"),
                        match hazard {
                            Some(h) => format!("{t} — {h}"),
                            None => t.to_string(),
                        },
                    ));
                }
                if let Some(s) = v.nav_status.map(sdroxide_types::nav_status_label) {
                    rows.push((tr!("ais-row-status"), s.to_string()));
                }
                if !v.call_sign.is_empty() {
                    rows.push((tr!("ais-call-sign"), v.call_sign.clone()));
                }
                if let Some(imo) = v.imo {
                    rows.push(("IMO".into(), imo.to_string()));
                }
                if let Some(kt) = v.sog_kt {
                    rows.push((tr!("ais-row-speed"), format!("{kt:.1} kt")));
                }
                if let Some(c) = v.cog_deg {
                    rows.push((
                        tr!("ais-row-course"),
                        tr!("ais-over-ground", value = format!("{:.0}", c)),
                    ));
                }
                if let Some(hd) = v.heading_deg {
                    rows.push((tr!("ais-row-heading"), format!("{hd:.0}°")));
                }
                if let Some(r) = v.turn_rate_deg_min.filter(|r| r.abs() > 0.5) {
                    rows.push((tr!("ais-row-turning"), format!("{value:+.0}°/min", value = r)));
                }
                if let Some(a) = v.altitude_m {
                    rows.push((tr!("ais-row-altitude"), format!("{a} m")));
                }
                let size = v.fmt_size();
                if !size.is_empty() {
                    rows.push((tr!("ais-row-size"), size));
                }
                if let Some(d) = v.draught_m {
                    rows.push((tr!("ais-row-draught"), format!("{d:.1} m")));
                }
                if !v.destination.is_empty() {
                    rows.push((tr!("ais-row-destination"), v.destination.clone()));
                }
                if !v.eta.is_empty() {
                    rows.push(("ETA".into(), format!("{eta} UTC", eta = v.eta)));
                }
                if !v.utc.is_empty() {
                    rows.push((tr!("ais-station-time"), format!("{utc} UTC", utc = v.utc)));
                }
                if let (Some((hlat, hlon)), Some((lat, lon))) = (home, v.lat.zip(v.lon)) {
                    let km = sdroxide_types::distance_km((hlat, hlon), (lat, lon));
                    let bear = sdroxide_types::bearing_deg((hlat, hlon), (lat, lon));
                    rows.push((
                        tr!("ais-row-range"),
                        tr!(
                            "ais-range-bearing",
                            km = format!("{km:.1}"),
                            bearing = format!("{bear:.0}")
                        ),
                    ));
                }
                if let Some((lat, lon)) = v.lat.zip(v.lon) {
                    rows.push((
                        tr!("ais-row-position"),
                        format!(
                            "{lat:.5}, {lon:.5}{}",
                            if v.accuracy { tr!("ais-dgnss") } else { String::from("") }
                        ),
                    ));
                }
                rows.push((
                    tr!("ais-row-signal"),
                    tr!(
                        "ais-dbfs-db-snr",
                        rssi_dbfs = format!("{:.0}", v.rssi_dbfs),
                        snr_db = format!("{:.0}", v.snr_db)
                    ),
                ));
                rows.push((
                    tr!("ais-row-messages"),
                    tr!(
                        "ais-last-type-ais",
                        messages = v.messages,
                        last_type = v.last_type,
                        channel = v.channel
                    ),
                ));
                rows.push((tr!("ais-first-heard"), fmt_age(now - v.first_at)));

                egui::Grid::new("ais-card-grid").num_columns(2).spacing([10.0, 1.0]).show(
                    ui,
                    |ui| {
                        for (k, val) in rows {
                            ui.label(RichText::new(k).size(10.0).weak());
                            ui.label(RichText::new(val).monospace().size(10.5));
                            ui.end_row();
                        }
                    },
                );

                ui.horizontal_wrapped(|ui| {
                    if v.has_position()
                        && crate::chrome::chip(
                            ui,
                            false,
                            RichText::new(tr!("ais-center")).size(10.0),
                        )
                        .on_hover_text(tr!("ais-put-vessel-middle-chart"))
                        .clicked()
                        && let Some((lat, lon)) = v.lat.zip(v.lon)
                    {
                        self.ais_map.view.centre_on(lat, lon);
                    }
                });
                // The sentence the message arrived as. It is here so the decode
                // can be *checked*: this decoder was written from the standard
                // rather than from a recording, and an `!AIVDM` line is a form
                // every other AIS tool accepts.
                if !v.nmea.is_empty() {
                    ui.label(
                        RichText::new(&v.nmea).monospace().size(9.0).color(theme::gray(110)).weak(),
                    )
                    .on_hover_text(tr!("ais-last-message-station-nmea-sentences"));
                }
            });
    }

    fn ais_map_pane(&mut self, ui: &mut egui::Ui, st: &AisStatus, now: i64, h: f32) {
        let home = self.ais_home();
        let cfg = self.state.ais;
        let state = &mut self.ais_map;
        crate::ais_map::show(ui, state, &st.vessels, home, now, cfg, h);
    }

    /// The operator's own position, from the grid in the digital-mode setup.
    ///
    /// The same source the FT8, APRS and ADS-B maps use, so the four never
    /// disagree about where the station is.
    fn ais_home(&self) -> Option<(f64, f64)> {
        let grid = self.digi_cfg_edit.my_grid.trim();
        (!grid.is_empty()).then(|| sdroxide_types::grid_to_latlon(grid)).flatten()
    }
}

impl SdroxideApp {
    /// How the decoder behaves: the channels, the two timeouts, the trail
    /// window and the vector time.
    ///
    /// Its own window rather than a section of the digimode setup dialog, for
    /// the reason the ADS-B one is: that dialog edits a `DigiConfig` and exists
    /// to hold an operator identity and message templates, and AIS has neither
    /// a callsign nor anything to say.
    pub(in crate::app) fn ais_setup_window(
        &mut self,
        ctx: &egui::Context,
        cmds: &mut Vec<Command>,
    ) {
        if !self.show_ais_setup {
            return;
        }
        let mut open = self.show_ais_setup;
        // Edited as a copy and diffed at the end, the way the ADS-B window
        // does: the engine persists whatever arrives and echoes it back in the
        // state, so there is no apply step and no way for the two copies to
        // drift.
        let mut cfg = self.state.ais;
        let resp = egui::Window::new(tr!("ais-ais-setup"))
            .id(crate::layout::salted_id(ctx, "AisSetup"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 420.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr!("ais-channels"));
                    for (i, ch) in sdroxide_ais_channels().iter().enumerate() {
                        let on = cfg.channel_enabled(i);
                        if crate::chrome::chip(ui, on, tr!("ais-ais-2", ch = ch.0))
                            .on_hover_text(tr!(
                                "ais-mhz-marine-channel-ship-alternates",
                                mhz = format!("{:.3}", ch.1 / 1e6),
                                ch = ch.2
                            ))
                            .clicked()
                        {
                            cfg.channels ^= 1 << i;
                        }
                    }
                });
                ui.add_space(4.0);
                egui::Grid::new("ais-cfg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr!("ais-drop-chart-after"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.drop_map_s).range(10..=3600).suffix(" s"),
                        );
                        ui.label(
                            RichText::new(tr!("ais-without-position-report")).size(9.5).weak(),
                        );
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(
                        RichText::new(tr!("ais-five-minutes-default-not-ads")).size(10.0).weak(),
                    );
                    ui.end_row();

                    ui.label(tr!("ais-drop-list-after"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.drop_list_s)
                                .range(i64::from(cfg.drop_map_s)..=21_600)
                                .suffix(" s"),
                        );
                        ui.label(RichText::new(tr!("ais-nothing-heard-all")).size(9.5).weak());
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(
                        RichText::new(tr!("ais-message-carrying-ship-s-name")).size(10.0).weak(),
                    );
                    ui.end_row();

                    ui.label(tr!("ais-trail-length"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.trail_minutes)
                                .range(0..=360)
                                .suffix(" min"),
                        );
                        ui.label(
                            RichText::new(tr!("ais-history-behind-each-target")).size(9.5).weak(),
                        );
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(
                        RichText::new(tr!("ais-minutes-rather-than-points-because"))
                            .size(10.0)
                            .weak(),
                    );
                    ui.end_row();

                    ui.label(tr!("ais-speed-vector"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.vector_minutes)
                                .speed(0.5)
                                .range(0.0..=60.0)
                                .suffix(" min"),
                        );
                        ui.label(
                            RichText::new(tr!("ais-how-far-ahead-vector-reaches")).size(9.5).weak(),
                        );
                    });
                    ui.end_row();

                    ui.label(tr!("ais-slot-threshold"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.threshold_db).range(3..=30).suffix(" dB"),
                        );
                        ui.label(
                            RichText::new(tr!("ais-above-channel-s-learned-noise"))
                                .size(9.5)
                                .weak(),
                        );
                    });
                    ui.end_row();

                    ui.label(tr!("ais-track-most"));
                    ui.add(
                        egui::DragValue::new(&mut cfg.max_vessels)
                            .range(10..=5000)
                            .suffix(tr!("ais-vessels-2")),
                    );
                    ui.end_row();
                });
                ui.separator();
                ui.label(RichText::new(tr!("ais-ais-receive-only-here-always")).size(10.0).weak());
                ui.label(RichText::new(tr!("ais-fill-my-grid-digimode-setup")).size(10.0).weak());
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        let cfg = cfg.sane();
        if cfg != self.state.ais {
            cmds.push(Command::SetAisConfig(cfg));
        }
        self.show_ais_setup = open;
    }
}

/// The two channels, for the setup window's chips: label, frequency, and the
/// marine channel number an operator would set on a bridge radio.
///
/// A table here rather than a dependency on `sdroxide_ais`: this crate builds
/// for wasm, and that one is native-only.
fn sdroxide_ais_channels() -> [(&'static str, f64, &'static str); 2] {
    [("A", sdroxide_types::AIS_CHANNEL_A_HZ, "87B"), ("B", sdroxide_types::AIS_CHANNEL_B_HZ, "88B")]
}

/// The column headings, drawn with the same offsets the rows use.
fn ais_head_row(ui: &mut egui::Ui, have_home: bool, sort: &mut AisSort, desc: &mut bool) {
    const L: egui::Align2 = egui::Align2::LEFT_CENTER;
    const R: egui::Align2 = egui::Align2::RIGHT_CENTER;
    let cols = columns(ui.available_width(), have_home);
    crate::app::panels::widgets::sort_head_row(
        ui,
        &[
            (cols.name, L, &tr!("ais-name"), Some(AisSort::Name)),
            // The MMSI is the name column's fallback, not an order anybody
            // wants a harbour in.
            (cols.mmsi, L, &tr!("ais-mmsi"), None),
            (cols.kind, L, &tr!("ais-type"), None),
            (cols.spd, R, "KT", Some(AisSort::Speed)),
            (cols.cog, R, "COG", None),
            (cols.sig, R, "SIG", Some(AisSort::Signal)),
            (cols.range, R, "KM", Some(AisSort::Range)),
            (cols.age, R, "AGE", Some(AisSort::Heard)),
        ],
        sort,
        desc,
    );
}

/// Column x offsets, in points from the left of a row.
struct Cols {
    name: f32,
    mmsi: f32,
    kind: f32,
    spd: f32,
    cog: f32,
    sig: f32,
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
    let after_range = if range.is_nan() { age - 34.0 } else { range - 44.0 };
    let sig = if w < WIDE_W { f32::NAN } else { after_range };
    let cog = if sig.is_nan() { after_range } else { sig - 34.0 };
    let spd = cog - 40.0;
    // The two left-hand extras go first on a narrow panel. A ship's name is up
    // to twenty characters where an aircraft's callsign is seven, so the name
    // column alone needs the room those two would take — and of the three, the
    // name is the one nobody can do without.
    let mmsi = if narrow { f32::NAN } else { 118.0 };
    let kind = if narrow { f32::NAN } else { 190.0 };
    Cols { name: 5.0, mmsi, kind, spd, cog, sig, range, age }
}

/// One row of the vessel table. Returns true if it was clicked.
fn ais_row(
    ui: &mut egui::Ui,
    v: &AisVessel,
    now: i64,
    selected: Option<u32>,
    home: Option<(f64, f64)>,
    drop_map_s: u16,
    i: usize,
) -> bool {
    const ROW_H: f32 = 17.0;
    const ACCENT_W: f32 = 2.5;

    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return false;
    }
    let is_sel = selected == Some(v.mmsi);
    let stale = v.pos_stale(now, drop_map_s);
    let p = ui.painter_at(rect);

    // A target whose position has aged out is greyed, because it is no longer
    // on the chart and a row that looked live would be the only thing saying it
    // was. A distress beacon or a vessel not under command outranks everything.
    let (accent, ink) = if v.is_alarm() {
        (theme::HAZARD(), theme::HAZARD())
    } else if is_sel {
        (theme::YELLOW(), theme::YELLOW())
    } else if stale {
        (theme::gray(80), theme::gray(115))
    } else if !v.kind.is_underway() {
        // Marks and shore stations: they are not traffic, and a chart full of
        // buoys drawn like ships is a chart nobody can scan.
        (theme::GREEN(), theme::CYAN_DIM())
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

    put(cols.name, egui::Align2::LEFT_CENTER, v.label(), mono.clone(), ink);
    // The MMSI, but only where it is not already the label — repeating it would
    // take a column and say nothing.
    if !v.name.is_empty() {
        put(cols.mmsi, egui::Align2::LEFT_CENTER, v.mmsi.to_string(), small.clone(), dim);
    }
    put(cols.kind, egui::Align2::LEFT_CENTER, v.kind.short().to_string(), small.clone(), dim);
    put(cols.spd, egui::Align2::RIGHT_CENTER, v.fmt_speed(), mono.clone(), ink);
    put(cols.cog, egui::Align2::RIGHT_CENTER, v.fmt_course(), small.clone(), dim);
    put(cols.sig, egui::Align2::RIGHT_CENTER, format!("{:.0}", v.rssi_dbfs), small.clone(), dim);
    if let (Some(h), Some((lat, lon))) = (home, v.lat.zip(v.lon)) {
        put(
            cols.range,
            egui::Align2::RIGHT_CENTER,
            format!("{:.1}", sdroxide_types::distance_km(h, (lat, lon))),
            small.clone(),
            dim,
        );
    }
    put(cols.age, egui::Align2::RIGHT_CENTER, fmt_age(now - v.last_at), small, dim);

    // One click target, the whole row wide, registered after everything above
    // it — which is what makes the name as clickable as the empty space.
    let hit = ui.interact(rect, ui.id().with(("ais-row", i)), egui::Sense::click());
    hit.on_hover_text(match v.type_label() {
        Some(t) => format!("{} — {} — {t}", v.mmsi, v.kind.tr()),
        None => format!("{} — {}", v.mmsi, v.kind.tr()),
    })
    .clicked()
}

/// Order the table. A missing value always sorts last, whichever way the arrow
/// points: a vessel that has not said its speed yet is not the slowest one
/// afloat.
fn sort_rows(rows: &mut [&AisVessel], by: AisSort, desc: bool, home: Option<(f64, f64)>) {
    let key = |v: &AisVessel| -> Option<f64> {
        match by {
            AisSort::Heard => Some(v.last_at as f64),
            AisSort::Name => None,
            AisSort::Speed => v.sog_kt.map(f64::from),
            AisSort::Range => {
                let h = home?;
                let (lat, lon) = v.lat.zip(v.lon)?;
                // Negated so "descending" is nearest-first, which is the way
                // round anybody actually wants a range column.
                Some(-sdroxide_types::distance_km(h, (lat, lon)))
            }
            AisSort::Signal => Some(f64::from(v.rssi_dbfs)),
        }
    };
    if by == AisSort::Name {
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

    fn ship(mmsi: u32, name: &str, kt: Option<f32>, last: i64) -> AisVessel {
        let mut v = AisVessel::new(mmsi, last);
        v.name = name.to_string();
        v.sog_kt = kt;
        v.last_at = last;
        v
    }

    /// A vessel that has not reported a speed yet is not the slowest one afloat
    /// — it sorts last whichever way the column is pointing.
    #[test]
    fn a_missing_value_sorts_last_in_both_directions() {
        let a = ship(1, "AAA", Some(12.0), 10);
        let b = ship(2, "BBB", None, 20);
        let c = ship(3, "CCC", Some(3.0), 30);
        for desc in [true, false] {
            let mut rows = vec![&a, &b, &c];
            sort_rows(&mut rows, AisSort::Speed, desc, None);
            assert_eq!(rows.last().unwrap().mmsi, 2, "desc={desc}");
        }
    }

    /// Nearest first is what a range column is for, so "descending" — the
    /// default direction for every other column — has to mean nearest here.
    #[test]
    fn the_range_column_puts_the_nearest_vessel_at_the_top() {
        let mut near = ship(1, "NEAR", None, 0);
        near.lat = Some(52.38);
        near.lon = Some(4.90);
        let mut far = ship(2, "FAR", None, 0);
        far.lat = Some(51.0);
        far.lon = Some(1.0);
        let mut rows = vec![&far, &near];
        sort_rows(&mut rows, AisSort::Range, true, Some((52.37, 4.89)));
        assert_eq!(rows[0].mmsi, 1);
    }

    /// The header counters run all session; they must not grow.
    #[test]
    fn a_counter_never_outgrows_its_slot() {
        for n in [0u64, 9_999, 10_000, 999_999, 1_000_000, 999_999_999, 42_000_000_000] {
            assert!(count(n).len() <= 6, "{n} formatted as {}", count(n));
        }
        assert_eq!(count(1_234), "1234");
        assert_eq!(count(12_345), "12k");
    }

    /// The columns must not overlap at any width the panel can be dragged to,
    /// and the trailing ones drop out rather than printing on top of each
    /// other.
    #[test]
    fn the_columns_never_overprint_however_narrow_the_panel_gets() {
        let mut w = 240.0f32;
        while w < 900.0 {
            for have_home in [true, false] {
                let c = columns(w, have_home);
                let mut xs: Vec<f32> = [c.kind, c.spd, c.cog, c.sig, c.range, c.age]
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
                    assert!(c.sig.is_nan() && c.range.is_nan(), "narrow panels drop the tail");
                    assert!(c.mmsi.is_nan() && c.kind.is_nan(), "...and the left-hand extras");
                }
                // Whatever else goes, the name and the age never do: one says
                // which vessel the row is and the other whether it is still
                // there.
                assert!(!c.name.is_nan() && !c.age.is_nan());
            }
            w += 7.0;
        }
    }

    /// The channel table the setup window draws has to be the one the decoder
    /// uses, in the order its bits are numbered — a chip that switched off the
    /// wrong channel would be a silent, permanent halving of the traffic.
    #[test]
    fn the_setup_chips_are_the_channels_the_decoder_uses() {
        let chans = sdroxide_ais_channels();
        assert_eq!(chans[0].1, sdroxide_types::AIS_CHANNEL_A_HZ);
        assert_eq!(chans[1].1, sdroxide_types::AIS_CHANNEL_B_HZ);
        let cfg = sdroxide_types::AisSettings { channels: 0b01, ..Default::default() };
        assert!(cfg.channel_enabled(0) && !cfg.channel_enabled(1));
    }
}
