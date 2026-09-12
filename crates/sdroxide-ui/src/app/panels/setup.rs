//! The digimode setup window.
//!
//! One dialog for the operator identity and the per-mode settings that all the
//! panels share, reachable from any of them. The identity half is the same
//! store the Settings dialog's General tab edits.

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
        let title = if mode.is_aprs() {
            "APRS Setup".to_string()
        } else if mode.is_packet() {
            "Packet Setup".to_string()
        } else if mode.is_text_modem() || mode.is_hell() || mode.is_js8() {
            format!("{} Setup", mode.label())
        } else {
            "FT8 / FT4 / FT2 Setup".to_string()
        };
        let resp = egui::Window::new(title.clone())
            .id(crate::layout::salted_id(ctx, &title))
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
                    ui.label("My callsign");
                    if crate::chrome::field(ui, egui::TextEdit::singleline(&mut cfg.my_call))
                        .changed()
                    {
                        cfg.my_call = cfg.my_call.to_uppercase();
                        changed = true;
                    }
                    ui.end_row();
                    ui.label("My grid");
                    if crate::chrome::field(ui, egui::TextEdit::singleline(&mut cfg.my_grid))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();
                    if mode.is_packet() {
                        ui.label("Station call");
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
                        ui.label(
                            RichText::new(
                                "With an SSID — OE3JJS-10. A packet station is conventionally \
                                 a different SSID from the operator, and nothing transmits \
                                 until this is set.",
                            )
                            .weak(),
                        );
                        ui.end_row();

                        // Speed is VHF's choice only: HF packet runs at 300 and
                        // the controller clamps it, so offering 9600 on 40 m
                        // would be a control that does nothing.
                        if mode == sdroxide_types::Mode::Packet {
                            ui.label("Speed");
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
                                    RichText::new(
                                        "9600 needs the radio's data port. A microphone and \
                                         speaker path destroys it at both ends — that is the \
                                         radio, not sdroxide.",
                                    )
                                    .weak(),
                                );
                                ui.end_row();
                            }
                        }

                        if !self.ui_settings.swl {
                            ui.label("TX delay");
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut cfg.packet_txdelay_ms)
                                            .range(50..=2000)
                                            .suffix(" ms"),
                                    )
                                    .on_hover_text(
                                        "Flags sent before a frame, so the far end can hear us and \
                                         lock its clock. On a CAT rig sdroxide alone spends 165–240 \
                                         ms getting on the air, and the rig's own transmit-ready \
                                         time is on top of that.",
                                    )
                                    .changed();
                            });
                            ui.end_row();

                            ui.label("Packet length");
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut cfg.packet_paclen)
                                            .range(16..=256)
                                            .suffix(" bytes"),
                                    )
                                    .on_hover_text(
                                        "The most a single frame carries. Shorter frames survive a \
                                         marginal path — only the frame that was hit is resent — and \
                                         cost more overhead. 128 on HF, 256 where the path is solid.",
                                    )
                                    .changed();
                                ui.label(RichText::new("window").weak());
                                let window_max = if cfg.packet_ext_seq { 127 } else { 7 };
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut cfg.packet_maxframe)
                                            .range(1..=window_max),
                                    )
                                    .on_hover_text(
                                        "Frames sent before waiting for an acknowledgement. More \
                                         fills a good path; on a bad one it is more to resend.",
                                    )
                                    .changed();
                            });
                            ui.end_row();

                            ui.label("Extended");
                            changed |= ui
                                .checkbox(&mut cfg.packet_ext_seq, "Ask for mod-128")
                                .on_hover_text(
                                    "Extended sequence numbers, for a window bigger than seven. Many \
                                     nodes refuse the request with a DM, which looks exactly like a \
                                     station that would not talk to you — so leave it off unless you \
                                     know the far end wants it.",
                                )
                                .changed();
                            ui.end_row();

                            ui.label("Answer calls");
                            changed |= ui
                                .checkbox(&mut cfg.packet_accept_incoming, "Accept connections")
                                .on_hover_text(
                                    "Off for a Winlink client, which dials out and has no reason to \
                                     answer. On to be reachable as a peer: a call arrives in the \
                                     terminal pane and you talk to whoever made it. A station whose \
                                     link is already busy with a Winlink session refuses calls until \
                                     that finishes.",
                                )
                                .changed();
                            ui.end_row();

                            ui.label("Connect text");
                            changed |= crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut cfg.packet_connect_text)
                                    .hint_text("what a caller is greeted with"),
                            )
                            .on_hover_text(
                                "Sent to a station that connects to us, once the link is up. Empty \
                                 sends nothing — which looks broken to whoever called.",
                            )
                            .changed();
                            ui.end_row();

                            ui.label("Default via");
                            changed |= crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut cfg.packet_connect_via)
                                    .hint_text("OE3XLR-1,OE3XMS-1"),
                            )
                            .on_hover_text(
                                "The digipeater path the terminal starts with. The path to your \
                                 local node is the same every time, and retyping it is how a hop \
                                 gets left off.",
                            )
                            .changed();
                            ui.end_row();

                            ui.label("Beacon");
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut cfg.packet_beacon_minutes)
                                            .range(0..=120)
                                            .suffix(" min"),
                                    )
                                    .on_hover_text("0 disables the timer")
                                    .changed();
                                changed |= crate::chrome::field(
                                    ui,
                                    egui::TextEdit::singleline(&mut cfg.packet_beacon_text)
                                        .hint_text("beacon text"),
                                )
                                .changed();
                            });
                            ui.end_row();

                            ui.label("KISS server");
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .checkbox(&mut cfg.packet_kiss_server, "Serve")
                                    .on_hover_text(
                                        "Offer this modem as a KISS TNC on a socket, so Pat, an \
                                         APRS client or the Linux AX.25 stack can use the radio.",
                                    )
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
                    }

                    if mode.is_aprs() {
                        ui.label("APRS call");
                        if crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.aprs_mycall),
                        )
                        .on_hover_text(
                            "The callsign this station beacons under, with its SSID — \
                             OE3JJS-9 for a car, -10 for an I-gate, -5 for a phone. Leave it \
                             empty to use the callsign above.",
                        )
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
                            RichText::new(
                                "No callsign: this station will not transmit at all — not a \
                                 beacon, not a message, not an acknowledgement.",
                            )
                            .size(10.0)
                            .color(crate::theme::ALERT())
                        } else {
                            RichText::new(format!("Transmits as {call}")).size(10.0).weak()
                        });
                        ui.end_row();

                        ui.label("Symbol");
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
                                .on_hover_text(
                                    "The two characters as the protocol carries them: a table \
                                     (`/` or `\\`) and a symbol code. A digit or a letter in \
                                     the first position is an overlay, drawn on top of the \
                                     alternate table's icon.",
                                )
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

                        ui.label("Path");
                        ui.horizontal(|ui| {
                            changed |= crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut cfg.aprs_path)
                                    .hint_text("WIDE1-1,WIDE2-1")
                                    .desired_width(150.0),
                            )
                            .on_hover_text(
                                "How far you ask to be repeated. This is the single most \
                                 consequential setting on the channel: every hop multiplies \
                                 the transmissions the whole network makes on one shared \
                                 frequency.",
                            )
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
                        ui.label("Position");
                        ui.horizontal(|ui| {
                            let was = cfg.aprs_use_grid;
                            changed |= ui
                                .checkbox(&mut cfg.aprs_use_grid, "From my grid")
                                .on_hover_text(
                                    "Beacon the centre of the locator above, reported with the \
                                     ambiguity a locator actually has — a six-character one is \
                                     a couple of kilometres across, and saying so is honest. \
                                     Untick to give exact coordinates instead.",
                                )
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
                                "Latitude",
                                &mut cfg.aprs_lat,
                                &mut self.aprs_lat_buf,
                                grid_lat,
                                90.0,
                                "south",
                            ),
                            (
                                "Longitude",
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
                                    ui.label(RichText::new("from the locator").size(9.5).weak());
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
                                    .on_hover_text(format!(
                                        "Decimal degrees, negative for {hemi}. Paste one \
                                         straight off a map."
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
                                        RichText::new("not a coordinate")
                                            .size(9.5)
                                            .color(crate::theme::ALERT()),
                                    );
                                }
                            });
                            ui.end_row();
                        }

                        ui.label("Comment");
                        changed |= crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut cfg.aprs_comment)
                                .hint_text("sent with every beacon")
                                .char_limit(43),
                        )
                        .changed();
                        ui.end_row();

                        if !self.ui_settings.swl {
                            ui.label("Beacon");
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut cfg.aprs_beacon_minutes)
                                            .range(0..=120)
                                            .suffix(" min"),
                                    )
                                    .on_hover_text(
                                        "0 — the default — never beacons on a timer. Thirty minutes \
                                         is the convention for a fixed station; a moving one \
                                         beacons oftener, but every beacon is somebody else's \
                                         channel time.",
                                    )
                                    .changed();
                                changed |= ui
                                    .checkbox(&mut cfg.aprs_compressed, "Compressed")
                                    .on_hover_text(
                                        "The compressed position format: a third of the air time \
                                         and more precise. Every receiver since the 1990s reads it.",
                                    )
                                    .changed();
                            });
                            ui.end_row();
                            if cfg.aprs_beacon_minutes > 0 {
                                ui.label("");
                                ui.label(
                                    RichText::new(
                                        "The first goes out one interval from now, not immediately.",
                                    )
                                    .size(10.0)
                                    .weak(),
                                );
                                ui.end_row();
                            }

                            ui.label("Messages");
                            changed |= ui
                                .checkbox(&mut cfg.aprs_ack_messages, "Acknowledge")
                                .on_hover_text(
                                    "Answer messages addressed to you. An acknowledgement is a \
                                     transmission this station makes without being asked, so a \
                                     receive-only setup should turn it off — and the beacon then \
                                     stops claiming to be reachable too.",
                                )
                                .changed();
                            ui.end_row();

                            ui.label("TX delay");
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut cfg.packet_txdelay_ms)
                                            .range(50..=2000)
                                            .suffix(" ms"),
                                    )
                                    .on_hover_text(
                                        "Flags sent ahead of every frame, so the far end's modem \
                                         hears the carrier and locks its clock before the data \
                                         starts. It has to outlast everything between pressing \
                                         transmit and radiating: sdroxide alone spends 165–240 ms \
                                         of it on a CAT rig, and a radio taking audio over a \
                                         network buffers more on top. Too little and the far end \
                                         never locks — the transmission is there and nothing \
                                         decodes it.",
                                    )
                                    .changed();
                                ui.label(RichText::new("shared with the packet mode").size(9.5).weak());
                            });
                            ui.end_row();
                        }

                        ui.label("Keep stations");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut cfg.aprs_station_ttl_min)
                                    .range(5..=1440)
                                    .suffix(" min"),
                            )
                            .on_hover_text(
                                "How long a station stays on the map after it was last heard. \
                                 Also what the map's fade is measured against, so a short \
                                 window shows only what is live.",
                            )
                            .changed();
                        ui.end_row();
                    }

                    if mode.is_js8() && !self.ui_settings.swl {
                        let turbo = cfg.js8_speed == sdroxide_types::Js8Speed::Turbo;
                        ui.label("Auto-reply");
                        changed |= ui
                            .checkbox(&mut cfg.js8_auto_reply, "Answer SNR? / GRID? / STATUS?")
                            .on_hover_text(
                                "Answer a direct question addressed to you or to @ALLCALL, with \
                                 the answer rather than an acknowledgement. Never answers another \
                                 station's traffic, and never answers itself.",
                            )
                            .changed();
                        ui.end_row();
                        ui.label("Heartbeat");
                        ui.horizontal(|ui| {
                            // The intervals JS8Call offers, plus off. A beacon
                            // is a commitment of air time, so the choice is a
                            // few sensible ones rather than a free number.
                            for (mins, label) in [
                                (0u32, "Off"),
                                (10, "10 min"),
                                (15, "15 min"),
                                (30, "30 min"),
                                (60, "60 min"),
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
                                "Turbo does not beacon — it is the local and VHF speed."
                            } else {
                                "Sends your callsign and grid so others know you are receivable. \
                                 The first goes out one interval from now, not immediately."
                            })
                            .size(10.5)
                            .weak(),
                        );
                        ui.end_row();
                        ui.label("Beacon frequency");
                        ui.horizontal(|ui| {
                            let sub_band = !cfg.js8_hb_anywhere;
                            if crate::chrome::chip(ui, sub_band, "500–1000 Hz")
                                .on_hover_text(
                                    "Move each beacon to a free slot in the heartbeat sub-band, \
                                     the way JS8Call does: it is where stations watching for \
                                     beacons look, and it keeps an unattended transmitter off \
                                     somebody else's QSO. The slot is chosen when the beacon \
                                     actually goes out, clear of everything being decoded.",
                                )
                                .clicked()
                                && !sub_band
                            {
                                cfg.js8_hb_anywhere = false;
                                changed = true;
                            }
                            if crate::chrome::chip(ui, !sub_band, "Working freq")
                                .on_hover_text(
                                    "Beacon where you are working instead. Against the band \
                                     convention, but it keeps everything you transmit in one \
                                     place.",
                                )
                                .clicked()
                                && sub_band
                            {
                                cfg.js8_hb_anywhere = true;
                                changed = true;
                            }
                        });
                        ui.end_row();
                        ui.label("Heartbeat reply");
                        ui.add_enabled_ui(cfg.js8_auto_reply && !turbo, |ui| {
                            changed |= ui
                                .checkbox(
                                    &mut cfg.js8_hb_ack,
                                    "Answer heartbeats with a signal report",
                                )
                                .on_hover_text(
                                    "Tell a station that beaconed how well you copied them. Off \
                                     by default: a busy band carries a heartbeat every slot, and \
                                     answering all of them would flood exactly the band \
                                     heartbeats exist to keep quiet. Rate-limited to one answer \
                                     per station every 15 minutes, and never while a message is \
                                     still arriving or while you have something queued to send.",
                                )
                                .changed();
                        });
                        ui.end_row();
                        ui.label("Status message");
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
                    if !mode.is_aprs() && !self.ui_settings.swl {
                        ui.label("TX period");
                        ui.horizontal(|ui| {
                            changed |=
                                ui.selectable_value(&mut cfg.tx_even, true, "Even").changed();
                            changed |=
                                ui.selectable_value(&mut cfg.tx_even, false, "Odd").changed();
                        });
                        ui.end_row();
                        ui.label("Auto-sequence");
                        changed |= crate::chrome::checkbox(ui, &mut cfg.auto_seq, "").changed();
                        ui.end_row();
                        ui.label("Auto TX frequency");
                        changed |= ui
                            .checkbox(&mut cfg.auto_tx_freq, "")
                            .on_hover_text(
                                "Choose the transmit frequency automatically: the quietest spot in \
                             the period you transmit in, rather than the frequency of the \
                             station you are answering. Off does NOT hold it — it answers on \
                             the frequency of the station being called. To hold, use Hold TX.",
                            )
                            .changed();
                        ui.end_row();
                        ui.label("Hold TX frequency");
                        changed |= ui
                            .checkbox(&mut cfg.hold_tx_freq, "")
                            .on_hover_text(
                                "Pin the transmit tone where it is. Nothing moves it: not \
                             answering a station, not the call queue, not calling CQ, not a \
                             click on a decode or the waterfall. Overrides Auto TX frequency. \
                             Changing band is the one exception: the offset you last set on \
                             the new band comes back with it. For where your licence is \
                             narrower than the band plan — on a UK 60 m dial of 5357 kHz the \
                             allocation ends at 5358.0, so the tone must stay under 1000 Hz.",
                            )
                            .changed();
                        ui.end_row();
                        ui.label("TX watchdog");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut cfg.tx_watchdog_min)
                                    .range(0..=60)
                                    .suffix(" min"),
                            )
                            .on_hover_text(
                                "Stop transmitting after this long with no reply and no action \
                             from you. 0 disables it.",
                            )
                            .changed();
                        ui.end_row();
                        ui.label("Give up after");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut cfg.max_tx_repeats)
                                    .range(0..=30)
                                    .suffix(" calls"),
                            )
                            .on_hover_text(
                                "Unanswered calls to one station before moving on. Calling CQ is \
                             exempt. 0 disables it.",
                            )
                            .changed();
                        ui.end_row();
                        // Special operating activity (issue #223). Above the
                        // DXpedition roles because it changes what every
                        // message in the exchange says, where those change who
                        // sends them.
                        ui.label("Contest");
                        ui.horizontal(|ui| {
                            for m in sdroxide_types::ContestMode::ALL {
                                changed |= ui
                                    .selectable_value(&mut cfg.contest, m, m.label())
                                    .on_hover_text(match m {
                                        sdroxide_types::ContestMode::None => {
                                            "The everyday exchange: grid, report, RR73."
                                        }
                                        sdroxide_types::ContestMode::EuVhf => {
                                            "European VHF contests: send a signal report, a \
                                             serial number and your SIX-character locator, in \
                                             the message layout WSJT-X calls EU VHF Contest. \
                                             CQ goes out as CQ TEST."
                                        }
                                    })
                                    .changed();
                            }
                        });
                        ui.end_row();
                        if cfg.contest == sdroxide_types::ContestMode::EuVhf {
                            ui.label("Serial");
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
                                    .on_hover_text(
                                        "The serial number the next exchange will carry. It \
                                         advances by itself as each contact is logged and is \
                                         remembered between sessions — set it by hand only to \
                                         start a contest, or to pick up where a paper log got \
                                         to.",
                                    );
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
                                        RichText::new("⚠ needs a 6-character grid")
                                            .size(11.0)
                                            .color(crate::theme::ALERT()),
                                    )
                                    .on_hover_text(
                                        "EU VHF Contest messages carry a six-character locator \
                                         (JN88DD, not JN88). Until My grid has one, the ordinary \
                                         exchange is sent instead.",
                                    );
                                }
                            });
                            ui.end_row();
                        }
                        ui.label("DXpedition");
                        ui.horizontal(|ui| {
                            for m in sdroxide_types::DxpedMode::ALL {
                                changed |= ui
                                .selectable_value(&mut cfg.dxped_mode, m, m.label())
                                .on_hover_text(match m {
                                    sdroxide_types::DxpedMode::Normal => "Ordinary FT8 operation.",
                                    sdroxide_types::DxpedMode::Hound => {
                                        "Calling a DXpedition running Fox mode: call from above \
                                         1000 Hz, move down onto the Fox when it answers, and \
                                         log on its RR73 without sending 73."
                                    }
                                    sdroxide_types::DxpedMode::Fox => {
                                        "Run the pile-up: several signals at once, a queue of \
                                         callers, worked strongest and rarest first. CALL CQ \
                                         starts it, STOP QSO stands it down."
                                    }
                                })
                                .changed();
                            }
                        });
                        ui.end_row();
                        if cfg.dxped_mode == sdroxide_types::DxpedMode::Fox {
                            ui.label("Fox signals");
                            changed |= ui
                            .add(
                                egui::DragValue::new(&mut cfg.fox_slots)
                                    .range(1..=sdroxide_types::FOX_MAX_SLOTS)
                                    .suffix(" at once"),
                            )
                            .on_hover_text(
                                "Simultaneous transmissions, spaced 60 Hz apart. They share the \
                                 transmitter's power, so more signals means each is weaker.",
                            )
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
                    if !self.ui_settings.swl {
                        ui.label("TX audio");
                        ui.horizontal(|ui| {
                            let mut db = sdroxide_types::tx_level_db(cfg.tx_level_for(mode)).round();
                            if ui
                                .add(
                                    egui::DragValue::new(&mut db)
                                        .range(sdroxide_types::TX_AUDIO_LEVEL_MIN_DB..=0.0)
                                        .suffix(" dB"),
                                )
                                .on_hover_text(if fm {
                                    "How loud this mode's burst is handed to a radio that \
                                     modulates it itself — and on FM that is the deviation, \
                                     because an FM transmitter turns audio level into frequency \
                                     swing and has no ALC to catch it. 1200 baud packet wants \
                                     about 3 kHz where voice wants 5, so full scale into a data \
                                     input set for voice over-deviates — which sounds completely \
                                     normal to a listener and decodes for nobody. Turn it down \
                                     until other stations report you, or set the level at the \
                                     radio.\n\nThis level belongs to this mode alone. A mode you \
                                     have never set starts from the FM figure, and the sideband \
                                     modes start from their own — so a deviation set for 1200 \
                                     baud never lands on FT8."
                                } else {
                                    "How loud this mode's over is handed to a radio that \
                                     modulates it itself — a CAT rig on its sound card, a FLEX, \
                                     an Icom on its network port. On sideband this is drive into \
                                     the modulator: bring it down until the rig's ALC is barely \
                                     moving and set the power at the radio, because ALC riding \
                                     on a constant-envelope mode is what splatters. Drive \
                                     reaches the rig's power register here, not its audio, so \
                                     this is the level.\n\nThis level belongs to this mode \
                                     alone: FT8, RTTY, PSK and MCW each keep their own, because \
                                     they do not load a modulator the same way. The same rail is \
                                     on the transmit strip, where you can reach it while you are \
                                     transmitting."
                                })
                                .changed()
                            {
                                let level = sdroxide_types::tx_level_from_db(db);
                                cfg.set_tx_level(mode, level);
                                cmds.push(Command::SetDigiTxLevel { mode, level });
                            }
                            ui.label(
                                RichText::new(if fm { "deviation, on FM" } else { "drive, on SSB" })
                                    .size(9.5)
                                    .weak(),
                            );
                        });
                        ui.end_row();
                    }
                });
                if !mode.is_aprs() && !self.ui_settings.swl {
                    ui.separator();
                    ui.label(
                        RichText::new("Message templates  {MYCALL} {MYGRID} {DX} {REPORT}")
                            .size(10.5)
                            .color(crate::theme::gray(150)),
                    );
                    egui::Grid::new("digi-msgs").num_columns(2).show(ui, |ui| {
                        for (label, field) in [
                            ("CQ", &mut cfg.msg_cq),
                            ("Grid", &mut cfg.msg_grid),
                            ("Report", &mut cfg.msg_report),
                            ("R+Report", &mut cfg.msg_rreport),
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
