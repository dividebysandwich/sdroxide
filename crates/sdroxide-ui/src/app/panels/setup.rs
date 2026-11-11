//! The digimode setup window.
//!
//! One dialog for the operator identity and the per-mode settings that all the
//! panels share, reachable from any of them. The identity half is the same
//! store the Settings dialog's General tab edits.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, RichText};
use sdroxide_types::Command;

use crate::app::SdroxideApp;

/// A six-character Maidenhead locator, in the alphabet the EU VHF contest
/// layout allows — the sub-square letters run A..X, not A..Y.
///
/// The same test the engine applies before it will send a contest exchange
/// (`sdroxide_digi::qso`), so the warning here and the behaviour there cannot
/// disagree.
fn is_grid6(g: &str) -> bool {
    let b = g.trim().to_ascii_uppercase().into_bytes();
    b.len() == 6
        && (b'A'..=b'R').contains(&b[0])
        && (b'A'..=b'R').contains(&b[1])
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && (b'A'..=b'X').contains(&b[4])
        && (b'A'..=b'X').contains(&b[5])
}

impl SdroxideApp {
    /// Own-call / grid / message-template editor (and RTTY parameters).
    pub(in crate::app) fn digi_settings_window(
        &mut self,
        ctx: &egui::Context,
        cmds: &mut Vec<Command>,
    ) {
        let mut open = self.show_digi_settings;
        let mode = self.state.rx[0].mode;
        // Per-mode parameters (RTTY/Olivia/THOR/FSQ) now live in each panel's
        // header, so this dialog only carries the shared identity + the
        // message templates the slotted QSO modes share.
        // The window is keyed by its English title, which is what egui keyed
        // it by before the title was translated: the position it was left
        // at survives, and a change of language does not move it.
        let (title, key) = if mode.is_aprs() {
            (tr!("setup-aprs-setup"), "APRS Setup".to_string())
        } else if mode.is_packet() {
            (tr!("setup-packet-setup"), "Packet Setup".to_string())
        } else if mode.is_text_modem() || mode.is_hell() || mode.is_js8() {
            (tr!("setup-mode-setup", mode = mode.label()), format!("{} Setup", mode.label()))
        } else {
            (tr!("setup-ft8-ft4-ft2-setup"), "FT8 / FT4 / FT2 Setup".to_string())
        };
        let resp = egui::Window::new(title)
            .id(crate::layout::salted_id(ctx, &key))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 420.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                // Edit the UI-owned copy so keystrokes aren't clobbered by the
                // engine's status echo; persist on any change.
                let cfg = &mut self.digi_cfg_edit;
                let mut changed = false;
                egui::Grid::new("digi-cfg").num_columns(2).show(ui, |ui| {
                    ui.label(tr!("setup-my-callsign"));
                    if crate::chrome::field(ui, egui::TextEdit::singleline(&mut cfg.my_call))
                        .changed()
                    {
                        cfg.my_call = cfg.my_call.to_uppercase();
                        changed = true;
                    }
                    ui.end_row();
                    ui.label(tr!("setup-my-grid"));
                    if crate::chrome::field(ui, egui::TextEdit::singleline(&mut cfg.my_grid))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();
                    if mode.is_packet() {
                        ui.label(tr!("setup-station-call"));
                        if crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.packet_mycall),
                        )
                        .changed()
                        {
                            cfg.packet_mycall = cfg.packet_mycall.to_uppercase();
                            changed = true;
                        }
                        ui.end_row();
                        ui.label("");
                        ui.label(RichText::new(tr!("setup-ssid-oe3jjs-10-packet-station")).weak());
                        ui.end_row();

                        // Speed is VHF's choice only: HF packet runs at 300 and
                        // the controller clamps it, so offering 9600 on 40 m
                        // would be a control that does nothing.
                        if mode == sdroxide_types::Mode::Packet {
                            ui.label(tr!("setup-speed"));
                            ui.horizontal(|ui| {
                                for b in [
                                    sdroxide_types::PacketBaud::Vhf1200,
                                    sdroxide_types::PacketBaud::Vhf9600,
                                ] {
                                    changed |= ui
                                        .selectable_value(&mut cfg.packet_baud, b, b.label())
                                        .changed();
                                }
                            });
                            ui.end_row();
                            if cfg.packet_baud == sdroxide_types::PacketBaud::Vhf9600 {
                                ui.label("");
                                ui.label(
                                    RichText::new(tr!("setup-t-9600-needs-radio-s-data")).weak(),
                                );
                                ui.end_row();
                            }
                        }

                        ui.label(tr!("setup-tx-delay"));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.packet_txdelay_ms)
                                        .range(50..=2000)
                                        .suffix(" ms"),
                                )
                                .on_hover_text(tr!("setup-flags-sent-before-frame-so"))
                                .changed();
                        });
                        ui.end_row();

                        ui.label(tr!("setup-packet-length"));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.packet_paclen)
                                        .range(16..=256)
                                        .suffix(tr!("setup-bytes")),
                                )
                                .on_hover_text(tr!("setup-most-single-frame-carries-shorter"))
                                .changed();
                            ui.label(RichText::new("window").weak());
                            let window_max = if cfg.packet_ext_seq { 127 } else { 7 };
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.packet_maxframe)
                                        .range(1..=window_max),
                                )
                                .on_hover_text(tr!(
                                    "setup-frames-sent-before-waiting-acknowledgement"
                                ))
                                .changed();
                        });
                        ui.end_row();

                        ui.label(tr!("setup-extended"));
                        changed |= ui
                            .checkbox(&mut cfg.packet_ext_seq, tr!("setup-ask-mod-128"))
                            .on_hover_text(tr!("setup-extended-sequence-numbers-window-bigger"))
                            .changed();
                        ui.end_row();

                        ui.label(tr!("setup-answer-calls"));
                        changed |= ui
                            .checkbox(
                                &mut cfg.packet_accept_incoming,
                                tr!("setup-accept-connections"),
                            )
                            .on_hover_text(tr!("setup-off-winlink-client-which-dials"))
                            .changed();
                        ui.end_row();

                        ui.label(tr!("setup-connect-text"));
                        changed |= crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.packet_connect_text)
                                .hint_text(tr!("setup-what-caller-greeted")),
                        )
                        .on_hover_text(tr!("setup-sent-station-connects-us-once"))
                        .changed();
                        ui.end_row();

                        ui.label(tr!("setup-default-via"));
                        changed |= crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.packet_connect_via)
                                .hint_text(tr!("setup-oe3xlr-1-oe3xms-1")),
                        )
                        .on_hover_text(tr!("setup-digipeater-path-terminal-starts-path"))
                        .changed();
                        ui.end_row();

                        ui.label(tr!("setup-beacon"));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.packet_beacon_minutes)
                                        .range(0..=120)
                                        .suffix(" min"),
                                )
                                .on_hover_text(tr!("setup-t-0-disables-timer"))
                                .changed();
                            changed |= crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut cfg.packet_beacon_text)
                                    .hint_text(tr!("setup-beacon-text")),
                            )
                            .changed();
                        });
                        ui.end_row();

                        ui.label(tr!("setup-kiss-server"));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .checkbox(&mut cfg.packet_kiss_server, tr!("setup-serve"))
                                .on_hover_text(tr!("setup-offer-modem-kiss-tnc-socket"))
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.packet_kiss_port)
                                        .range(1024..=65535),
                                )
                                .changed();
                        });
                        ui.end_row();
                    }

                    if mode.is_aprs() {
                        ui.label(tr!("setup-aprs-call"));
                        if crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.aprs_mycall),
                        )
                        .on_hover_text(tr!("setup-callsign-station-beacons-under-ssid"))
                        .changed()
                        {
                            cfg.aprs_mycall = cfg.aprs_mycall.to_uppercase();
                            changed = true;
                        }
                        ui.end_row();
                        ui.label("");
                        // What will actually go on the air, which is the only
                        // thing that matters and is not obvious when the field
                        // above is blank.
                        let call = cfg.aprs_call();
                        ui.label(if call.is_empty() {
                            RichText::new(tr!("setup-no-callsign-station-will-not"))
                                .size(10.0)
                                .color(crate::theme::ALERT())
                        } else {
                            RichText::new(tr!("setup-transmits-as", call = call.to_string()))
                                .size(10.0)
                                .weak()
                        });
                        ui.end_row();

                        ui.label(tr!("setup-symbol"));
                        ui.horizontal(|ui| {
                            // A picker over the whole 190-entry table would be
                            // a dialog of its own. These are the ones an
                            // amateur station actually is, and the two
                            // characters are editable beside them for the rest.
                            for (table, code, label) in APRS_COMMON_SYMBOLS {
                                let sym = sdroxide_types::AprsSymbol::new(*table, *code);
                                let on = cfg.aprs_symbol == sym;
                                let (r, resp) = ui.allocate_exact_size(
                                    egui::vec2(19.0, 19.0),
                                    egui::Sense::click(),
                                );
                                let tint = if on {
                                    crate::theme::YELLOW()
                                } else {
                                    crate::theme::CYAN_DIM()
                                };
                                self.aprs_icons.paint(ui, r, sym.kind(), tint);
                                if resp.on_hover_text(*label).clicked() {
                                    cfg.aprs_symbol = sym;
                                    changed = true;
                                }
                            }
                            let mut text = cfg.aprs_symbol.text();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut text)
                                        .desired_width(30.0)
                                        .char_limit(2)
                                        .font(egui::TextStyle::Monospace),
                                )
                                .on_hover_text(tr!("setup-two-characters-protocol-carries-them"))
                                .changed()
                            {
                                let mut c = text.chars();
                                if let (Some(t), Some(k)) = (c.next(), c.next()) {
                                    cfg.aprs_symbol = sdroxide_types::AprsSymbol::new(t, k);
                                    changed = true;
                                }
                            }
                        });
                        ui.end_row();

                        ui.label(tr!("setup-path"));
                        ui.horizontal(|ui| {
                            changed |= crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut cfg.aprs_path)
                                    .hint_text(tr!("setup-wide1-1-wide2-1"))
                                    .desired_width(150.0),
                            )
                            .on_hover_text(tr!("setup-how-far-you-ask-repeated"))
                            .changed();
                        });
                        ui.end_row();
                        // Advice, not enforcement: local practice varies, and a
                        // path that is wasteful in a European city is
                        // reasonable in the outback. Nothing here refuses to
                        // transmit.
                        if let Some(note) =
                            sdroxide_aprs::path_advice(&sdroxide_aprs::parse_path(&cfg.aprs_path))
                        {
                            ui.label("");
                            ui.label(RichText::new(note).size(10.0).color(crate::theme::YELLOW()));
                            ui.end_row();
                        }

                        // Where the beacon says this station is.
                        //
                        // The coordinate rows are always drawn, greyed while
                        // the locator is in charge and showing what it works
                        // out to. Hiding them behind the tick-box left an
                        // operator who wanted to type a position with no sign
                        // that they could, and no way to see what was going
                        // out instead.
                        let from_grid = sdroxide_aprs::position_from_grid(&cfg.my_grid);
                        ui.label(tr!("setup-position"));
                        ui.horizontal(|ui| {
                            let was = cfg.aprs_use_grid;
                            changed |= ui
                                .checkbox(&mut cfg.aprs_use_grid, tr!("setup-my-grid-2"))
                                .on_hover_text(tr!("setup-beacon-centre-locator-above-reported"))
                                .changed();
                            // Taking the coordinates over starts from where the
                            // locator had you, rather than from Null Island.
                            if was
                                && !cfg.aprs_use_grid
                                && let Some(q) = from_grid
                            {
                                cfg.aprs_lat = (q.lat * 1e6).round() / 1e6;
                                cfg.aprs_lon = (q.lon * 1e6).round() / 1e6;
                            }
                        });
                        ui.end_row();

                        let grid_lat = from_grid.map_or(0.0, |q| q.lat);
                        let grid_lon = from_grid.map_or(0.0, |q| q.lon);
                        // Text boxes, not spinners: a coordinate is something
                        // an operator reads off a map and pastes, and a spinner
                        // has to be discovered to be typeable at all.
                        for (label, value, buf, shown, limit, hemi) in [
                            (
                                tr!("setup-latitude"),
                                &mut cfg.aprs_lat,
                                &mut self.aprs_lat_buf,
                                grid_lat,
                                90.0,
                                "south",
                            ),
                            (
                                tr!("setup-longitude"),
                                &mut cfg.aprs_lon,
                                &mut self.aprs_lon_buf,
                                grid_lon,
                                180.0,
                                "west",
                            ),
                        ] {
                            ui.label(label);
                            ui.horizontal(|ui| {
                                if cfg.aprs_use_grid {
                                    // The locator's own answer, read-only:
                                    // editing it here would set a figure the
                                    // beacon then ignores. The buffer is kept
                                    // in step so switching over starts here.
                                    *buf = format!("{shown:.6}");
                                    ui.add_enabled(
                                        false,
                                        egui::TextEdit::singleline(buf).desired_width(96.0),
                                    );
                                    ui.label(RichText::new(tr!("setup-locator")).size(9.5).weak());
                                    return;
                                }
                                // Seed once, so the box is never blank.
                                if buf.is_empty() {
                                    *buf = format!("{:.6}", *value);
                                }
                                let parsed = buf
                                    .trim()
                                    .parse::<f64>()
                                    .ok()
                                    .filter(|d| d.is_finite() && d.abs() <= limit);
                                let edit = egui::TextEdit::singleline(buf)
                                    .desired_width(96.0)
                                    .text_color_opt(parsed.is_none().then(crate::theme::ALERT));
                                if crate::chrome::field(ui, edit)
                                    .on_hover_text(tr!(
                                        "setup-decimal-degrees",
                                        hemisphere = hemi.to_string()
                                    ))
                                    .changed()
                                    && let Some(d) = parsed
                                    && (d - *value).abs() > f64::EPSILON
                                {
                                    *value = d;
                                    changed = true;
                                }
                                if parsed.is_none() {
                                    ui.label(
                                        RichText::new(tr!("setup-not-coordinate"))
                                            .size(9.5)
                                            .color(crate::theme::ALERT()),
                                    );
                                }
                            });
                            ui.end_row();
                        }

                        ui.label(tr!("setup-comment"));
                        changed |= crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.aprs_comment)
                                .hint_text(tr!("setup-sent-every-beacon"))
                                .char_limit(43),
                        )
                        .changed();
                        ui.end_row();

                        ui.label(tr!("setup-beacon"));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.aprs_beacon_minutes)
                                        .range(0..=120)
                                        .suffix(" min"),
                                )
                                .on_hover_text(tr!("setup-t-0-default-never-beacons-timer"))
                                .changed();
                            changed |= ui
                                .checkbox(&mut cfg.aprs_compressed, tr!("setup-compressed"))
                                .on_hover_text(tr!("setup-compressed-position-format-third-air"))
                                .changed();
                        });
                        ui.end_row();
                        if cfg.aprs_beacon_minutes > 0 {
                            ui.label("");
                            ui.label(
                                RichText::new(tr!("setup-first-goes-out-one-interval"))
                                    .size(10.0)
                                    .weak(),
                            );
                            ui.end_row();
                        }

                        ui.label(tr!("setup-messages"));
                        changed |= ui
                            .checkbox(&mut cfg.aprs_ack_messages, tr!("setup-acknowledge"))
                            .on_hover_text(tr!(
                                "setup-answer-messages-addressed-you-acknowledgement"
                            ))
                            .changed();
                        ui.end_row();

                        ui.label(tr!("setup-tx-delay"));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.packet_txdelay_ms)
                                        .range(50..=2000)
                                        .suffix(" ms"),
                                )
                                .on_hover_text(tr!("setup-flags-sent-ahead-every-frame"))
                                .changed();
                            ui.label(
                                RichText::new(tr!("setup-shared-packet-mode")).size(9.5).weak(),
                            );
                        });
                        ui.end_row();

                        ui.label(tr!("setup-keep-stations"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut cfg.aprs_station_ttl_min)
                                    .range(5..=1440)
                                    .suffix(" min"),
                            )
                            .on_hover_text(tr!("setup-how-long-station-stays-map"))
                            .changed();
                        ui.end_row();
                    }

                    if mode.is_js8() {
                        let turbo = cfg.js8_speed == sdroxide_types::Js8Speed::Turbo;
                        ui.label(tr!("setup-auto-reply"));
                        changed |= ui
                            .checkbox(&mut cfg.js8_auto_reply, tr!("setup-answer-snr-grid-status"))
                            .on_hover_text(tr!("setup-answer-direct-question-addressed-you"))
                            .changed();
                        ui.end_row();
                        ui.label(tr!("setup-heartbeat"));
                        ui.horizontal(|ui| {
                            // The intervals JS8Call offers, plus off. A beacon
                            // is a commitment of air time, so the choice is a
                            // few sensible ones rather than a free number.
                            for (mins, label) in [
                                (0u32, tr!("setup-off")),
                                (10, String::from("10 min")),
                                (15, String::from("15 min")),
                                (30, String::from("30 min")),
                                (60, String::from("60 min")),
                            ] {
                                if crate::chrome::chip(ui, cfg.js8_heartbeat_min == mins, label)
                                    .clicked()
                                    && cfg.js8_heartbeat_min != mins
                                {
                                    cfg.js8_heartbeat_min = mins;
                                    changed = true;
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("");
                        // Off by default and worth saying why: a beacon that
                        // switches itself on is an on-air behaviour the
                        // operator never chose.
                        ui.label(
                            RichText::new(if turbo {
                                tr!("setup-turbo-does-not-beacon-local")
                            } else {
                                tr!("setup-sends-your-callsign-grid-so")
                            })
                            .size(10.5)
                            .weak(),
                        );
                        ui.end_row();
                        ui.label(tr!("setup-beacon-frequency"));
                        ui.horizontal(|ui| {
                            let sub_band = !cfg.js8_hb_anywhere;
                            if crate::chrome::chip(ui, sub_band, tr!("setup-t-500-1000-hz"))
                                .on_hover_text(tr!("setup-move-each-beacon-free-slot"))
                                .clicked()
                                && !sub_band
                            {
                                cfg.js8_hb_anywhere = false;
                                changed = true;
                            }
                            if crate::chrome::chip(ui, !sub_band, tr!("setup-working-freq"))
                                .on_hover_text(tr!("setup-beacon-where-you-are-working"))
                                .clicked()
                                && sub_band
                            {
                                cfg.js8_hb_anywhere = true;
                                changed = true;
                            }
                        });
                        ui.end_row();
                        ui.label(tr!("setup-heartbeat-reply"));
                        ui.add_enabled_ui(cfg.js8_auto_reply && !turbo, |ui| {
                            changed |= ui
                                .checkbox(
                                    &mut cfg.js8_hb_ack,
                                    tr!("setup-answer-heartbeats-signal-report"),
                                )
                                .on_hover_text(tr!("setup-tell-station-beaconed-how-well"))
                                .changed();
                        });
                        ui.end_row();
                        ui.label(tr!("setup-status-message"));
                        changed |= crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.js8_status),
                        )
                        .changed();
                        ui.end_row();
                    }
                    // Everything from here down belongs to the QSO modes: a
                    // transmit period, a sequencer, a watchdog and the message
                    // templates. APRS has none of them — it is a broadcast
                    // channel with a beacon on it — so the rows would be
                    // controls that do nothing under a dialog titled after it.
                    //
                    // A conditional block rather than an early `return`, and
                    // deliberately: this dialog edits a copy and sends it to
                    // the engine *once*, at the end. A `return` skips that, so
                    // every setting the operator typed is discarded the moment
                    // the window closes — which is what happened when this
                    // was written the other way (issue #150).
                    if !mode.is_aprs() {
                        ui.label(tr!("setup-tx-period"));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .selectable_value(&mut cfg.tx_even, true, tr!("setup-even"))
                                .changed();
                            changed |= ui
                                .selectable_value(&mut cfg.tx_even, false, tr!("setup-odd"))
                                .changed();
                        });
                        ui.end_row();
                        ui.label(tr!("setup-auto-sequence"));
                        changed |= crate::chrome::checkbox(ui, &mut cfg.auto_seq, "").changed();
                        ui.end_row();
                        if mode == sdroxide_types::Mode::Ft8 {
                            ui.label(tr!("setup-decode-depth"));
                            let row = ui.horizontal(|ui| {
                                let mut c = false;
                                c |= ui
                                    .selectable_value(
                                        &mut cfg.ft8_depth,
                                        sdroxide_types::Ft8Depth::Fast,
                                        tr!("setup-fast"),
                                    )
                                    .changed();
                                c |= ui
                                    .selectable_value(
                                        &mut cfg.ft8_depth,
                                        sdroxide_types::Ft8Depth::Normal,
                                        tr!("setup-normal"),
                                    )
                                    .changed();
                                c |= ui
                                    .selectable_value(
                                        &mut cfg.ft8_depth,
                                        sdroxide_types::Ft8Depth::Deep,
                                        tr!("setup-deep"),
                                    )
                                    .changed();
                                c
                            });
                            changed |= row.inner;
                            row.response.on_hover_text(tr!("setup-how-hard-ft8-decoder-works"));
                            ui.end_row();
                        }
                        ui.label(tr!("setup-auto-tx-frequency"));
                        changed |= ui
                            .checkbox(&mut cfg.auto_tx_freq, "")
                            .on_hover_text(tr!(
                                "setup-choose-transmit-frequency-automatically-quietest"
                            ))
                            .changed();
                        ui.end_row();
                        ui.label(tr!("setup-hold-tx-frequency"));
                        changed |= ui
                            .checkbox(&mut cfg.hold_tx_freq, "")
                            .on_hover_text(tr!("setup-pin-transmit-tone-where-nothing"))
                            .changed();
                        ui.end_row();
                        ui.label(tr!("setup-tx-watchdog"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut cfg.tx_watchdog_min)
                                    .range(0..=60)
                                    .suffix(" min"),
                            )
                            .on_hover_text(tr!("setup-stop-transmitting-after-long-no"))
                            .changed();
                        ui.end_row();
                        ui.label(tr!("setup-give-up-after"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut cfg.max_tx_repeats)
                                    .range(0..=30)
                                    .suffix(tr!("setup-calls")),
                            )
                            .on_hover_text(tr!("setup-unanswered-calls-one-station-before"))
                            .changed();
                        ui.end_row();
                        // Special operating activity (issue #223). Above the
                        // DXpedition roles because it changes what every
                        // message in the exchange says, where those change who
                        // sends them.
                        ui.label(tr!("setup-contest"));
                        ui.horizontal(|ui| {
                            for m in sdroxide_types::ContestMode::ALL {
                                changed |= ui
                                    .selectable_value(&mut cfg.contest, m, m.tr())
                                    .on_hover_text(match m {
                                        sdroxide_types::ContestMode::None => {
                                            tr!("setup-everyday-exchange-grid-report-rr73")
                                        }
                                        sdroxide_types::ContestMode::EuVhf => {
                                            tr!("setup-european-vhf-contests-send-signal")
                                        }
                                    })
                                    .changed();
                            }
                        });
                        ui.end_row();
                        if cfg.contest == sdroxide_types::ContestMode::EuVhf {
                            ui.label(tr!("setup-serial"));
                            ui.horizontal(|ui| {
                                // Straight out as its own command, never
                                // through this window's `changed` flag: the
                                // engine advances this number as each contact is
                                // logged and keeps it against a stale client
                                // copy (`keep_engine_owned`), so a
                                // `SetDigiConfig` carrying it would be ignored.
                                let n = ui
                                    .add(
                                        egui::DragValue::new(&mut cfg.contest_serial)
                                            .range(1..=sdroxide_types::CONTEST_SERIAL_MAX),
                                    )
                                    .on_hover_text(tr!("setup-serial-number-next-exchange-will"));
                                if n.changed() {
                                    cmds.push(Command::SetContestSerial(cfg.contest_serial));
                                }
                                // The layout has no room for a four-character
                                // locator, so this is a refusal and not a
                                // preference: say so where it is set rather than
                                // letting the ordinary exchange go out and
                                // leaving the operator to wonder.
                                if !is_grid6(&cfg.my_grid) {
                                    ui.label(
                                        RichText::new(tr!("setup-needs-6-character-grid"))
                                            .size(11.0)
                                            .color(crate::theme::ALERT()),
                                    )
                                    .on_hover_text(tr!("setup-eu-vhf-contest-messages-carry"));
                                }
                            });
                            ui.end_row();
                        }
                        ui.label(tr!("setup-dxpedition"));
                        ui.horizontal(|ui| {
                            for m in sdroxide_types::DxpedMode::ALL {
                                changed |= ui
                                    .selectable_value(&mut cfg.dxped_mode, m, m.tr())
                                    .on_hover_text(match m {
                                        sdroxide_types::DxpedMode::Normal => {
                                            tr!("setup-ordinary-ft8-operation")
                                        }
                                        sdroxide_types::DxpedMode::Hound => {
                                            tr!("setup-calling-dxpedition-running-fox-mode")
                                        }
                                        sdroxide_types::DxpedMode::Fox => {
                                            tr!("setup-run-pile-up-several-signals")
                                        }
                                    })
                                    .changed();
                            }
                        });
                        ui.end_row();
                        if cfg.dxped_mode == sdroxide_types::DxpedMode::Fox {
                            ui.label(tr!("setup-fox-signals"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut cfg.fox_slots)
                                        .range(1..=sdroxide_types::FOX_MAX_SLOTS)
                                        .suffix(tr!("setup-once")),
                                )
                                .on_hover_text(tr!("setup-simultaneous-transmissions-spaced-60-hz"))
                                .changed();
                            ui.end_row();
                        }
                    }

                    // Last, and for every digital mode: the level into a radio
                    // that modulates what we send it. This mode's own — the
                    // carrier caption below says what it inherited from if it
                    // has never been set. It used to live inside the APRS
                    // block, where an operator setting FM deviation could not
                    // see that they were also turning down their FT8.
                    //
                    // Written straight out as `SetDigiTxLevel` rather than
                    // through this window's `changed` flag: the map is one of
                    // the fields the engine keeps against a stale client copy
                    // (`keep_engine_owned`), so a `SetDigiConfig` carrying it
                    // would be ignored. One write route, and the rail on the
                    // transmit strip is the other end of it.
                    let fm = mode.is_fm_carrier();
                    ui.label(tr!("setup-tx-audio"));
                    ui.horizontal(|ui| {
                        let mut db = sdroxide_types::tx_level_db(cfg.tx_level_for(mode)).round();
                        if ui
                            .add(
                                egui::DragValue::new(&mut db)
                                    .range(sdroxide_types::TX_AUDIO_LEVEL_MIN_DB..=0.0)
                                    .suffix(" dB"),
                            )
                            .on_hover_text(if fm {
                                tr!("setup-how-loud-mode-s-burst")
                            } else {
                                tr!("setup-how-loud-mode-s-over")
                            })
                            .changed()
                        {
                            let level = sdroxide_types::tx_level_from_db(db);
                            cfg.set_tx_level(mode, level);
                            cmds.push(Command::SetDigiTxLevel { mode, level });
                        }
                        ui.label(
                            RichText::new(if fm {
                                tr!("setup-deviation-fm")
                            } else {
                                tr!("setup-drive-ssb")
                            })
                            .size(9.5)
                            .weak(),
                        );
                    });
                    ui.end_row();
                });
                if !mode.is_aprs() {
                    ui.separator();
                    ui.label(
                        RichText::new("Message templates  {MYCALL} {MYGRID} {DX} {REPORT}")
                            .size(10.5)
                            .color(crate::theme::gray(150)),
                    );
                    egui::Grid::new("digi-msgs").num_columns(2).show(ui, |ui| {
                        for (label, field) in [
                            ("CQ", &mut cfg.msg_cq),
                            (&tr!("setup-grid"), &mut cfg.msg_grid),
                            (&tr!("setup-report"), &mut cfg.msg_report),
                            (&tr!("setup-r-report"), &mut cfg.msg_rreport),
                            ("RR73", &mut cfg.msg_rr73),
                            ("73", &mut cfg.msg_73),
                        ] {
                            ui.label(label);
                            changed |= crate::chrome::field(ui, egui::TextEdit::singleline(field))
                                .changed();
                            ui.end_row();
                        }
                    });
                }
                // The one place this dialog's edits leave it. Nothing above may
                // return early past it.
                if changed {
                    cmds.push(Command::SetDigiConfig(cfg.clone()));
                }
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_digi_settings = open;
    }
}

/// The symbols an amateur station actually is, as a row of pickable icons.
///
/// The full table is 190 entries and picking from it belongs in a dialog of
/// its own; everything else is reachable by typing the two characters beside
/// these.
const APRS_COMMON_SYMBOLS: &[(char, char, &str)] = &[
    ('/', '-', "Home station"),
    ('/', '>', "Car"),
    ('/', 'k', "Truck"),
    ('/', 'v', "Van"),
    ('/', '<', "Motorcycle"),
    ('/', 'b', "Bicycle"),
    ('/', '[', "Person on foot"),
    ('/', 'R', "Motorhome"),
    ('/', 's', "Boat"),
    ('/', 'Y', "Yacht"),
    ('/', '\'', "Light aircraft"),
    ('/', 'O', "Balloon"),
    ('/', '#', "Digipeater"),
    ('/', '&', "I-gate"),
    ('/', 'r', "Repeater"),
    ('/', '_', "Weather station"),
    ('/', 'h', "Hospital"),
    ('/', ';', "Portable / campsite"),
];
