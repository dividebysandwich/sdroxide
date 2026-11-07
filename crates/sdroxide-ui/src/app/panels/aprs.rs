//! The APRS panel: who is out there, where they are, and what they said.
//!
//! Three things an operator watches, and they move independently, so each gets
//! its own space rather than sharing one:
//!
//! - **STATIONS** — everything heard, newest activity first, with the detail
//!   card for whichever one is selected.
//! - **MAP** — the same stations placed, drawn with the icon each one's symbol
//!   asks for. See [`crate::aprs_map`].
//! - **MESSAGES** — the conversation, and the box to answer from. A `RAW` chip
//!   swaps it for every frame on the channel, which is where somebody else's
//!   traffic and anything the codec could not read both end up.
//!
//! Unlike the packet panel there *is* text entry here, because unlike packet
//! there is a conversation to have: APRS messages are addressed, acknowledged
//! and retried, and answering one is a button.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{AprsEntryKind, AprsMsgState, AprsStation, AprsStatus, Command};

use crate::app::util::fmt_age;
use crate::app::{SdroxideApp, tx_gated};
use crate::theme;
use crate::theme::ThemedScroll;

/// Longest message the format carries.
const MSG_MAX: usize = 67;

impl SdroxideApp {
    pub(in crate::app) fn aprs_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<AprsStatus> =
            self.digi_status.as_ref().and_then(|s| s.aprs.as_ref().map(|a| (**a).clone()));
        let Some(st) = st else {
            ui.label(RichText::new(tr!("aprs-starting-aprs-modem")).weak());
            return;
        };
        let tx_ok = self.tx_capable();
        let now = crate::time::now_unix();
        let content_bottom = ui.cursor().top() + panel_h - 26.0;

        self.aprs_header(ui, cmds, &st, tx_ok);
        ui.add_space(3.0);

        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let full_w = ui.available_width();

        ui.horizontal_top(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(
                        if pane.is_some() { full_w } else { (full_w * 0.34).clamp(190.0, 340.0) },
                        avail_h,
                    ),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.aprs_station_list(ui, &st, now, avail_h, tx_ok),
                );
            }
            if pane.is_none() {
                ui.separator();
            }
            // The map and the messages share the rest, the map taking the
            // larger share: it is the reason this mode has a panel of its own.
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        let map_h = (avail_h * 0.56).max(crate::aprs_map::MIN_HEIGHT);
                        self.aprs_map_pane(ui, &st, now, map_h);
                        ui.add_space(2.0);
                        self.aprs_message_pane(ui, cmds, &st, avail_h - map_h - 6.0, tx_ok);
                    } else {
                        self.aprs_message_pane(ui, cmds, &st, avail_h, tx_ok);
                    }
                });
            }
            if pane == Some(2) {
                ui.vertical(|ui| self.aprs_map_pane(ui, &st, now, avail_h));
            }
        });
    }

    /// The channel, the beacon and the buttons.
    ///
    /// # Why the readouts are painted into fixed slots
    ///
    /// Everything in this row that changes on its own does so in a slot of a
    /// fixed width. Carrier detect follows the channel and flips several times
    /// a second, and the row is a `horizontal_wrapped`: a label that comes and
    /// goes changes the row's width, which on a window near the wrap threshold
    /// tips the whole tail onto a second line and back again. The buttons
    /// dance sideways, and every pane below the header moves by a row height,
    /// twice a second. Reserving the space costs a few dozen points of header
    /// and makes the layout independent of what the channel is doing.
    fn aprs_header(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &AprsStatus,
        tx_ok: bool,
    ) {
        let transmitting = self.digi_status.as_ref().is_some_and(|s| s.transmitting);
        // What this station would transmit as, by the same rule the engine
        // uses. Empty means it cannot transmit at all, which is a thing the
        // operator has to be told rather than left to discover by pressing a
        // button that does nothing.
        let call = self.digi_cfg_edit.aprs_call();
        let have_call = !call.is_empty();
        let have_pos = st.my_pos.is_some();

        ui.horizontal_wrapped(|ui| {
            // A floor under the row, so a height that varies by a point with
            // its content cannot ripple into every pane below it either.
            ui.set_min_height(22.0);
            ui.label(RichText::new(tr!("aprs-aprs")).size(11.0).strong().color(theme::CYAN()));
            ui.label(RichText::new("1200 baud").weak().size(10.5));
            self.digi_freq_chip(ui, cmds);
            aprs_level_bar(ui, st.level, st.dcd);
            aprs_channel_slot(ui, st);
            // The one thing an operator must not have to guess at: this
            // station cannot transmit, and here is the reason. Static — it
            // changes only when the operator changes a setting — so it needs
            // no slot of its own.
            if !have_call {
                ui.label(
                    RichText::new(tr!("aprs-no-callsign"))
                        .strong()
                        .size(10.5)
                        .color(theme::ALERT()),
                )
                .on_hover_text(tr!("aprs-nothing-will-transmitted-no-beacon"));
            } else if !have_pos {
                ui.label(RichText::new(tr!("aprs-no-position")).size(10.5).color(theme::YELLOW()))
                    .on_hover_text(tr!("aprs-beacon-needs-somewhere-report-fill"));
            }

            crate::chrome::row_tail(ui, |ui| {
                self.clear_rx_chip(ui, cmds);
                self.save_rx_chip(ui);
                if crate::chrome::chip(
                    ui,
                    self.show_digi_settings,
                    RichText::new(tr!("aprs-setup")).size(9.5),
                )
                .on_hover_text(tr!("aprs-callsign-symbol-digipeater-path-position"))
                .clicked()
                {
                    self.show_digi_settings = !self.show_digi_settings;
                }
                // The beacon interval, here rather than only in the setup
                // dialog: how often an unattended transmitter keys is the
                // setting an operator reaches for while watching the channel,
                // not while in a dialog.
                let cfg = &mut self.digi_cfg_edit;
                let before = cfg.aprs_beacon_minutes;
                let resp = ui
                    .add_enabled(
                        // Not gated on having a callsign or a position:
                        // setting the interval before filling those in is a
                        // perfectly ordinary order to do things in, and the
                        // warning to the left already says why nothing is
                        // going out yet. Only a receiver, which cannot beacon
                        // at all, greys it.
                        tx_ok,
                        egui::DragValue::new(&mut cfg.aprs_beacon_minutes)
                            .range(0..=120)
                            .speed(0.25)
                            .custom_formatter(|n, _| {
                                if n < 1.0 { "off".into() } else { format!("{n:.0} min", n = n) }
                            }),
                    )
                    .on_hover_text(tr!("aprs-how-often-beacon-your-position"));
                if resp.changed() && cfg.aprs_beacon_minutes != before {
                    let cfg = cfg.clone();
                    cmds.push(Command::SetDigiConfig(cfg));
                }
                if tx_gated(ui, tx_ok && have_call && have_pos, |ui| {
                    crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new(tr!("aprs-beacon")).strong(),
                        theme::GREEN(),
                        theme::INK_ON_CYAN(),
                    )
                    .on_hover_text(if !have_call {
                        tr!("aprs-station-has-no-callsign-see")
                    } else if !have_pos {
                        tr!("aprs-station-has-no-position-report")
                    } else {
                        tr!("aprs-send-one-position-report-now")
                    })
                })
                .clicked()
                {
                    cmds.push(Command::AprsBeacon);
                }
                aprs_tx_slot(ui, st, transmitting);
            });
        });
    }

    /// Everything heard, and the card for whichever one is selected.
    ///
    /// A table rather than a row of widgets: every column starts at a fixed
    /// offset so callsigns and ages line up down the list, and the whole row is
    /// one click target. Built the way the FT8 decode list is
    /// ([`super::decodes`]) — painted into an allocated rectangle, with
    /// `ui.interact` over the whole of it — because a row assembled out of
    /// labels is only clickable where the labels happen to be.
    fn aprs_station_list(
        &mut self,
        ui: &mut egui::Ui,
        st: &AprsStatus,
        now: i64,
        avail_h: f32,
        tx_ok: bool,
    ) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr!("aprs-stations")).size(10.5).strong().color(theme::CYAN_DIM()),
            );
            crate::chrome::row_tail(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.aprs_filter)
                        .hint_text("filter")
                        .desired_width(74.0),
                );
            });
        });

        let filter = self.aprs_filter.trim().to_ascii_uppercase();
        // Newest activity first: on a channel where everything is a beacon,
        // "who just moved" is the question a list answers.
        let mut rows: Vec<&AprsStation> = st
            .stations
            .iter()
            .filter(|s| {
                filter.is_empty()
                    || s.name.to_ascii_uppercase().contains(&filter)
                    || s.comment.to_ascii_uppercase().contains(&filter)
            })
            .collect();
        rows.sort_by_key(|s| -s.last_heard);

        let selected = self.aprs_map.selected.clone();
        let card = selected.as_ref().and_then(|n| st.stations.iter().find(|s| &s.name == n));
        // The card sits at the bottom of the column and stays there, whether
        // the list above it holds two stations or two hundred. It used to be
        // laid out straight after the list, which put it half way up the panel
        // on a quiet channel and moved it every time somebody new was heard.
        let card_h = if card.is_some() { (avail_h * 0.42).clamp(120.0, 260.0) } else { 0.0 };
        let list_h = (avail_h - card_h - 24.0).max(48.0);

        let mut pick = None;
        egui::ScrollArea::vertical()
            .id_salt("aprs-stations")
            .max_height(list_h)
            .min_scrolled_height(list_h)
            // Hold the full height rather than shrinking onto the rows, so the
            // card below is pinned to the bottom of the column.
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if rows.is_empty() {
                    ui.label(RichText::new(tr!("aprs-nothing-heard-yet")).weak());
                }
                for (i, s) in rows.iter().enumerate() {
                    if let Some(name) = self.aprs_row(ui, s, st, now, selected.as_deref(), i) {
                        pick = Some(name);
                    }
                }
            });
        if let Some(name) = pick {
            let same = self.aprs_map.selected.as_deref() == Some(name.as_str());
            self.aprs_map.selected = if same { None } else { Some(name.clone()) };
            if !same
                && let Some(s) = st.stations.iter().find(|s| s.name == name)
                && s.entry == AprsEntryKind::Station
            {
                // Selecting a station is also how you address a message to it,
                // which saves typing a callsign that is already on screen.
                self.aprs_target = name;
            }
        }

        if let Some(s) = card {
            ui.separator();
            self.aprs_station_card(ui, s, st, now, card_h, tx_ok);
        }
    }

    /// One row of the station table. Returns its name if it was clicked.
    fn aprs_row(
        &mut self,
        ui: &mut egui::Ui,
        s: &AprsStation,
        st: &AprsStatus,
        now: i64,
        selected: Option<&str>,
        i: usize,
    ) -> Option<String> {
        const ROW_H: f32 = 18.0;
        // Column offsets from the left of the row. Fixed, so the table reads
        // down as well as across.
        const ACCENT_W: f32 = 2.5;
        const ICON_X: f32 = 6.0;
        const ICON_W: f32 = 15.0;
        const NAME_X: f32 = 25.0;
        const NAME_W: f32 = 82.0;
        const AGE_W: f32 = 30.0;

        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::hover());
        if !ui.is_rect_visible(rect) {
            return None;
        }
        let is_sel = selected == Some(s.name.as_str());
        let p = ui.painter_at(rect);

        // The row's own colour: what it is takes precedence over how it got
        // here, and a killed object over both.
        let (accent, ink) = if s.killed {
            (theme::gray(90), theme::gray(120))
        } else if is_sel {
            (theme::YELLOW(), theme::YELLOW())
        } else if s.entry == AprsEntryKind::Object {
            (theme::map().preview, theme::CYAN())
        } else if s.direct {
            // Heard direct: on a channel where nearly everything arrives
            // through a digipeater, the ones you hear yourself are the ones
            // actually in range, and that is worth a colour rather than a
            // punctuation mark.
            (theme::GREEN(), theme::CYAN())
        } else {
            (theme::CYAN_DIM(), theme::CYAN())
        };
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

        let icon = egui::Rect::from_center_size(
            egui::pos2(rect.left() + ICON_X + ICON_W / 2.0, rect.center().y),
            egui::vec2(ICON_W, ICON_W),
        );
        self.aprs_icons.paint(ui, icon, s.symbol.kind(), ink);

        let p = ui.painter_at(rect);
        let name = egui::Rect::from_min_max(
            egui::pos2(rect.left() + NAME_X, rect.top()),
            egui::pos2(rect.left() + NAME_X + NAME_W, rect.bottom()),
        );
        p.with_clip_rect(name).text(
            egui::pos2(name.left(), rect.center().y),
            egui::Align2::LEFT_CENTER,
            &s.name,
            egui::FontId::monospace(11.0),
            ink,
        );

        // Whatever the station last said, in the space between the callsign and
        // the age — clipped rather than wrapped, because a table row is one
        // line high.
        let note = if !s.comment.is_empty() { &s.comment } else { &s.status };
        let note_rect = egui::Rect::from_min_max(
            egui::pos2(name.right() + 6.0, rect.top()),
            egui::pos2(rect.right() - AGE_W - 6.0, rect.bottom()),
        );
        if !note.is_empty() && note_rect.width() > 20.0 {
            p.with_clip_rect(note_rect).text(
                egui::pos2(note_rect.left(), rect.center().y),
                egui::Align2::LEFT_CENTER,
                note,
                egui::FontId::proportional(10.0),
                theme::gray(140),
            );
        }
        p.text(
            egui::pos2(rect.right() - 2.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            fmt_age(now - s.last_heard),
            egui::FontId::monospace(9.5),
            theme::gray(130),
        );

        // One click target, the whole row wide. Registered after everything
        // above it and with an id of its own, which is what makes the callsign
        // as clickable as the icon.
        let hit = ui.interact(rect, ui.id().with(("aprs-row", i)), egui::Sense::click());
        let hit = hit.on_hover_ui(|ui| {
            ui.label(RichText::new(&s.name).monospace().strong().color(theme::YELLOW()));
            ui.label(RichText::new(s.symbol.kind().label()).size(10.5));
            if s.entry == AprsEntryKind::Object && !s.reported_by.is_empty() {
                ui.label(
                    RichText::new(tr!("aprs-object", reported_by = s.reported_by))
                        .size(10.0)
                        .weak(),
                );
            }
            if !s.comment.is_empty() {
                ui.label(RichText::new(&s.comment).size(10.5));
            }
            if let (Some(me), Some(q)) = (st.my_pos, s.pos) {
                let km = sdroxide_types::distance_km((me.lat, me.lon), (q.lat, q.lon));
                ui.label(RichText::new(format!("{km:.0} km")).size(10.0).weak());
            }
            ui.label(
                RichText::new(if s.direct {
                    tr!("aprs-heard-direct").to_string()
                } else if s.via.is_empty() {
                    String::new()
                } else {
                    tr!("aprs-via", via = s.via.join(","))
                })
                .size(10.0)
                .weak(),
            );
        });
        hit.clicked().then(|| s.name.clone())
    }

    /// Everything one station has told us.
    #[allow(clippy::too_many_arguments)]
    fn aprs_station_card(
        &mut self,
        ui: &mut egui::Ui,
        s: &AprsStation,
        st: &AprsStatus,
        now: i64,
        h: f32,
        tx_ok: bool,
    ) {
        let name = s.name.clone();
        egui::ScrollArea::vertical().id_salt("aprs-card").max_height(h).show_themed(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&s.name).monospace().strong().color(theme::YELLOW()));
                ui.label(RichText::new(s.symbol.kind().label()).size(10.0).weak());
                ui.label(RichText::new(s.symbol.text()).size(9.5).monospace().weak())
                    .on_hover_text(tr!("aprs-two-characters-station-sent-say"));
                if s.killed {
                    ui.label(RichText::new(tr!("aprs-killed")).size(9.5).color(theme::ALERT()))
                        .on_hover_text(tr!("aprs-station-put-object-here-has"));
                }
            });
            if s.entry == AprsEntryKind::Object && !s.reported_by.is_empty() {
                ui.label(
                    RichText::new(tr!("aprs-object-reported", reported_by = s.reported_by))
                        .size(10.0)
                        .weak(),
                );
            }
            if !s.comment.is_empty() {
                ui.label(RichText::new(&s.comment).size(10.5));
            }
            if !s.status.is_empty() {
                ui.label(RichText::new(&s.status).size(10.5).color(theme::GREEN()));
            }
            if let Some(q) = s.pos {
                ui.label(
                    RichText::new(format!("{:.5}, {:.5}", q.lat, q.lon)).monospace().size(10.0),
                );
                if q.ambiguity > 0 {
                    ui.label(
                        RichText::new(tr!("aprs-digit-s-blanked", ambiguity = q.ambiguity))
                            .size(9.5)
                            .weak(),
                    )
                    .on_hover_text(tr!("aprs-sender-deliberately-reported-square-rather"));
                }
                if let Some(me) = st.my_pos {
                    let km = sdroxide_types::distance_km((me.lat, me.lon), (q.lat, q.lon));
                    let bear = sdroxide_types::bearing_deg((me.lat, me.lon), (q.lat, q.lon));
                    ui.label(
                        RichText::new(format!("{km:.1} km   {bear:.0}°")).monospace().size(10.0),
                    );
                }
            }
            let mut motion = Vec::new();
            if let Some(c) = s.course_deg {
                motion.push(format!("{c:03}°"));
            }
            if let Some(v) = s.speed_kn {
                motion.push(format!("{v:.0} kn ({:.0} km/h)", v * 1.852));
            }
            if let Some(a) = s.altitude_m {
                motion.push(format!("{a:.0} m"));
            }
            if !motion.is_empty() {
                ui.label(RichText::new(motion.join("   ")).monospace().size(10.0));
            }
            if let Some(w) = s.weather.filter(|w| !w.is_empty()) {
                let mut bits = Vec::new();
                if let Some(t) = w.temp_c {
                    bits.push(format!("{t:.1} °C"));
                }
                if let (Some(d), Some(v)) = (w.wind_dir_deg, w.wind_speed_ms) {
                    bits.push(tr!(
                        "aprs-wind-m-s",
                        value = format!("{:03}", d),
                        value2 = format!("{:.1}", v)
                    ));
                }
                if let Some(g) = w.wind_gust_ms {
                    bits.push(tr!("aprs-gust", value = format!("{:.1}", g)));
                }
                if let Some(hu) = w.humidity_pct {
                    bits.push(format!("{hu}% RH"));
                }
                if let Some(p) = w.pressure_hpa {
                    bits.push(format!("{value:.1} hPa", value = p));
                }
                if let Some(r) = w.rain_1h_mm {
                    bits.push(tr!("aprs-rain-mm-h", value = format!("{:.1}", r)));
                }
                ui.label(RichText::new(bits.join("   ")).size(10.0).color(theme::CYAN_DIM()));
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr!(
                        "aprs-frame-s-last-ago",
                        packets = s.packets,
                        last_heard = fmt_age(now - s.last_heard)
                    ))
                    .size(9.5)
                    .weak(),
                );
            });
            if !s.via.is_empty() {
                ui.label(
                    RichText::new(tr!("aprs-via-2", via = s.via.join(",")))
                        .size(9.5)
                        .monospace()
                        .weak(),
                )
                .on_hover_text(tr!("aprs-digipeaters-last-frame-came-through"));
            }
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                if s.entry == AprsEntryKind::Station
                    && tx_gated(ui, tx_ok, |ui| {
                        crate::chrome::chip(
                            ui,
                            false,
                            RichText::new(tr!("aprs-message")).size(10.0),
                        )
                        .on_hover_text(tr!("aprs-address-message-box-station"))
                    })
                    .clicked()
                {
                    self.aprs_target = name.clone();
                }
                if s.pos.is_some()
                    && crate::chrome::chip(ui, false, RichText::new(tr!("aprs-center")).size(10.0))
                        .on_hover_text(tr!("aprs-put-station-middle-map-double"))
                        .clicked()
                    && let Some(q) = s.pos
                {
                    // Holding the view, not just re-fitting it: the auto-fit
                    // frames everything heard, which is the opposite of what
                    // somebody asking to centre on one station wants.
                    self.aprs_map.view.centre_on(q.lat, q.lon);
                }
            });
        });
    }

    fn aprs_map_pane(&mut self, ui: &mut egui::Ui, st: &AprsStatus, now: i64, h: f32) {
        let ttl = self.digi_cfg_edit.aprs_station_ttl_min;
        let icons = &mut self.aprs_icons;
        let state = &mut self.aprs_map;
        let picked = crate::aprs_map::show(ui, state, icons, &st.stations, st.my_pos, now, ttl, h);
        if let Some(name) = picked {
            if st.stations.iter().any(|s| s.name == name && s.entry == AprsEntryKind::Station) {
                self.aprs_target = name;
            }
        }
    }

    /// The conversation, or the raw channel, plus the box to answer from.
    fn aprs_message_pane(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &AprsStatus,
        h: f32,
        tx_ok: bool,
    ) {
        // The same rule the engine transmits under, so a message of ours is
        // shown as ours whichever field the callsign came from.
        let me = self.digi_cfg_edit.aprs_call();
        let have_call = !me.is_empty();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.aprs_show_traffic {
                    tr!("aprs-channel")
                } else {
                    tr!("aprs-messages")
                })
                .size(10.5)
                .strong()
                .color(theme::CYAN_DIM()),
            );
            // The two counts that say why a channel is not decoding, beside the
            // raw view they are about. They were in the panel header and are
            // not any more: they change on their own, and the header has to
            // keep a constant width (see `aprs_header`).
            if self.aprs_show_traffic {
                if st.bad_frames > 0 {
                    ui.label(
                        RichText::new(tr!("aprs-bad", bad_frames = st.bad_frames)).size(9.5).weak(),
                    )
                    .on_hover_text(tr!("aprs-frames-arrived-failed-their-check"));
                }
                if st.non_aprs > 0 {
                    ui.label(
                        RichText::new(tr!("aprs-other", non_aprs = st.non_aprs)).size(9.5).weak(),
                    )
                    .on_hover_text(tr!("aprs-frames-read-cleanly-off-channel"));
                }
            }
            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(ui, self.aprs_show_traffic, RichText::new("RAW").size(9.5))
                    .on_hover_text(tr!("aprs-every-frame-channel-arrived-other"))
                    .clicked()
                {
                    self.aprs_show_traffic = !self.aprs_show_traffic;
                }
            });
        });

        let compose_h = 30.0;
        let list_h = (h - compose_h - 22.0).max(40.0);
        egui::ScrollArea::vertical()
            .id_salt("aprs-messages")
            .max_height(list_h)
            .min_scrolled_height(list_h)
            .stick_to_bottom(true)
            // Fill the height it was given rather than shrinking to the
            // messages in it, so the box you type into stays where it was
            // instead of climbing up the panel as the conversation grows.
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if self.aprs_show_traffic {
                    if st.traffic.is_empty() {
                        ui.label(RichText::new(tr!("aprs-nothing-channel-yet")).weak());
                    }
                    for t in &st.traffic {
                        let via = if t.via.is_empty() {
                            String::new()
                        } else {
                            format!(",{}", t.via.join(","))
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let colour = if t.sent { theme::GREEN() } else { theme::CYAN_DIM() };
                            ui.label(
                                RichText::new(format!("{}>{}{via}", t.from, t.to))
                                    .monospace()
                                    .size(10.0)
                                    .color(colour),
                            );
                            ui.label(RichText::new(&t.kind).size(9.0).weak());
                            ui.label(RichText::new(&t.info).monospace().size(10.0));
                        });
                    }
                    return;
                }
                if st.messages.is_empty() {
                    ui.label(RichText::new(tr!("aprs-no-messages")).weak().size(10.5));
                }
                for m in &st.messages {
                    let ours = m.from.eq_ignore_ascii_case(&me) && !me.is_empty();
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(
                            RichText::new(crate::app::util::time_str(m.at))
                                .monospace()
                                .size(9.0)
                                .weak(),
                        );
                        let who = if ours { format!("→{}", m.to) } else { m.from.clone() };
                        ui.label(RichText::new(who).monospace().size(10.5).color(if ours {
                            theme::GREEN()
                        } else {
                            theme::YELLOW()
                        }));
                        ui.label(RichText::new(&m.text).size(10.5));
                        if ours {
                            // Words, not symbols. The packaged font has no
                            // glyph for a tick or an up arrow — they come out
                            // as empty boxes — and the state of a message an
                            // unattended transmitter is still retrying is not
                            // something to leave to a font fallback.
                            let (face, colour, tip) = match m.state {
                                AprsMsgState::Queued => {
                                    ("…".to_string(), theme::gray(150), tr!("aprs-waiting-channel"))
                                }
                                AprsMsgState::Sent if m.tries > 1 => (
                                    tr!("aprs-sent", tries = m.tries),
                                    theme::CYAN_DIM(),
                                    tr!("aprs-sent-retried-still-waiting-acknowledged"),
                                ),
                                AprsMsgState::Sent => (
                                    "sent".to_string(),
                                    theme::CYAN_DIM(),
                                    tr!("aprs-air-waiting-acknowledged"),
                                ),
                                AprsMsgState::Acked => (
                                    "ack".to_string(),
                                    theme::GREEN(),
                                    tr!("aprs-acknowledged-far-end"),
                                ),
                                AprsMsgState::Rejected => (
                                    "rejected".to_string(),
                                    theme::ALERT(),
                                    tr!("aprs-refused-far-end-not-retried"),
                                ),
                                AprsMsgState::Failed => (
                                    tr!("aprs-no-ack").to_string(),
                                    theme::ALERT(),
                                    tr!("aprs-no-answer-after-every-retry"),
                                ),
                                AprsMsgState::Received => {
                                    (String::new(), theme::gray(150), String::new())
                                }
                            };
                            if !face.is_empty() {
                                ui.label(RichText::new(face).size(9.5).color(colour))
                                    .on_hover_text(tip);
                            }
                        } else if !m.id.is_empty()
                            && crate::chrome::chip(ui, false, RichText::new("reply").size(9.0))
                                .clicked()
                        {
                            self.aprs_target = m.from.clone();
                        }
                    });
                }
            });

        ui.add_space(2.0);
        let mut send = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let to = ui.add(
                egui::TextEdit::singleline(&mut self.aprs_target)
                    .hint_text("to")
                    .desired_width(78.0)
                    .font(egui::TextStyle::Monospace),
            );
            if to.changed() {
                self.aprs_target = self.aprs_target.to_ascii_uppercase();
            }
            let room = (ui.available_width() - 56.0).max(60.0);
            let text = ui.add(
                egui::TextEdit::singleline(&mut self.aprs_draft)
                    .hint_text("message")
                    .desired_width(room)
                    .char_limit(MSG_MAX),
            );
            if text.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send = true;
            }
            let ready = !self.aprs_target.trim().is_empty() && !self.aprs_draft.trim().is_empty();
            if tx_gated(ui, tx_ok && ready && have_call, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(tr!("aprs-send")).size(10.0).strong(),
                    theme::GREEN(),
                    theme::INK_ON_CYAN(),
                )
                .on_hover_text(if !have_call {
                    tr!("aprs-station-has-no-callsign-so")
                } else if !ready {
                    tr!("aprs-needs-station-address-something-say")
                } else {
                    tr!("aprs-send-keep-retrying-until-far")
                })
            })
            .clicked()
            {
                send = true;
            }
        });
        // Enter in the message box goes through the same gate as the button: a
        // keystroke must not do what a greyed-out button refuses to.
        if send
            && have_call
            && !self.aprs_target.trim().is_empty()
            && !self.aprs_draft.trim().is_empty()
            && tx_ok
        {
            cmds.push(Command::AprsSendMessage {
                to: self.aprs_target.trim().to_ascii_uppercase(),
                text: self.aprs_draft.trim().to_string(),
            });
            self.aprs_draft.clear();
        }
    }
}

/// Paint `segs` left to right inside `rect`, clipped to it.
fn paint_segments(ui: &egui::Ui, rect: egui::Rect, segs: &[(String, egui::Color32)]) {
    let p = ui.painter_at(rect);
    let font = egui::FontId::proportional(10.5);
    let mut x = rect.left();
    for (text, col) in segs {
        let galley = p.layout_no_wrap(text.clone(), font.clone(), *col);
        let y = rect.center().y - galley.size().y / 2.0;
        x += galley.size().x + 6.0;
        p.galley(egui::pos2(x - galley.size().x - 6.0, y), galley, *col);
    }
}

/// The station count and the carrier detect, in a slot of fixed width.
///
/// Painted rather than laid out because the width has to be the same whether
/// the channel is busy or clear — see [`SdroxideApp::aprs_header`].
fn aprs_channel_slot(ui: &mut egui::Ui, st: &AprsStatus) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(86.0, 15.0), egui::Sense::hover());
    let mut segs = vec![(tr!("aprs-stn", stations = st.stations.len()), theme::CYAN_DIM())];
    if st.dcd {
        segs.push((tr!("aprs-busy").to_string(), theme::ALERT()));
    }
    paint_segments(ui, rect, &segs);
    resp.on_hover_text(tr!("aprs-stations-heard-still-inside-window"));
}

/// What the transmitter is doing, in a slot of fixed width: the over in
/// progress, then the frames waiting for the channel, then the countdown to
/// the next beacon.
fn aprs_tx_slot(ui: &mut egui::Ui, st: &AprsStatus, transmitting: bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(58.0, 15.0), egui::Sense::hover());
    let mut segs = Vec::new();
    if transmitting {
        segs.push((tr!("aprs-tx").to_string(), theme::ALERT()));
    } else if st.tx_queue > 0 {
        segs.push((tr!("aprs-queued", tx_queue = st.tx_queue), theme::YELLOW()));
    } else if let Some(secs) = st.next_beacon_s {
        segs.push((format!("{}:{:02}", secs / 60, secs % 60), theme::CYAN_DIM()));
    }
    paint_segments(ui, rect, &segs);
    resp.on_hover_text(tr!("aprs-over-progress-frames-waiting-channel"));
}

/// Receive level, so "nothing is decoding" can be told from "nothing is
/// arriving".
///
/// The first question anyone asks of a silent packet channel is whether the
/// audio is reaching the modem at all, and without this the panel cannot
/// answer it: a rig on the wrong connector, a muted data port or a squelch
/// that never opens all look exactly like a dead band.
fn aprs_level_bar(ui: &mut egui::Ui, level: f32, dcd: bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(48.0, 9.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, theme::gray(20));
    // Log scale (~ -60..0 dBFS) so a weak but perfectly decodable signal is
    // still visibly above the floor.
    let db = 20.0 * level.max(1e-6).log10();
    let frac = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    let mut fill = rect;
    fill.set_width(rect.width() * frac);
    let col = if dcd {
        theme::GREEN()
    } else if frac > 0.06 {
        theme::CYAN_DIM()
    } else {
        theme::gray(45)
    };
    p.rect_filled(fill, 2.0, col);
    p.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, theme::gray(60)), egui::StrokeKind::Inside);
    resp.on_hover_text(tr!("aprs-audio-reaching-modem-flat-means"));
}
