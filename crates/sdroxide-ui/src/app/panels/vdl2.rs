//! The VDL panel: what the aircraft overhead are *saying*.
//!
//! Two things an operator watches, so two columns:
//!
//! - **MESSAGES** — every AVLC frame decoded, newest at the bottom, with the
//!   full card for whichever one is selected pinned below the log.
//! - **STATIONS** — one row per 24-bit address heard, which is who is out there.
//!
//! No map, deliberately. VDL2 carries a position only occasionally — a ground
//! station's own location in an XID beacon, and whatever an operator's airline
//! puts in an ACARS position report — so a map would be mostly empty of the
//! traffic the panel exists to show. The aircraft themselves are on the ADS-B
//! map next door, and their address here is the same number.
//!
//! # The header says what the receiver is doing
//!
//! An empty log has half a dozen causes and only one of them is the decoder: no
//! aerial, an aerial on the wrong socket, a window that reaches none of the
//! channels, a quiet hour. The header carries the channel strip — every channel
//! of the plan, whether it is live and how many frames it has produced — and the
//! counters that separate "nothing arriving" from "arriving and not decoding".

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Vdl2Message, Vdl2Payload, Vdl2Station, Vdl2Status};

use crate::app::SdroxideApp;
use crate::app::util::fmt_age;
use crate::theme;
use crate::theme::ThemedScroll;

/// How the station table is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum Vdl2Sort {
    /// Most recently heard first. The default: on a list of a conversation's
    /// participants, "who just said something" is the question it answers.
    #[default]
    Heard,
    Name,
    Address,
    Messages,
    Signal,
}

impl SdroxideApp {
    pub(in crate::app) fn vdl2_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Vdl2Status = match self.vdl2_status.as_ref() {
            Some(s) => (**s).clone(),
            None => {
                ui.label(RichText::new("starting the VDL2 decoder…").weak());
                return;
            }
        };
        let now = crate::time::now_unix();
        let content_bottom = ui.cursor().top() + panel_h - 26.0;

        self.vdl2_header(ui, cmds, &st);
        ui.add_space(3.0);

        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let full_w = ui.available_width();

        ui.horizontal_top(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(
                        if pane.is_some() {
                            full_w
                        } else {
                            // The log is the wider of the two: it carries free
                            // text, and the station list is six short columns.
                            (full_w * 0.62).clamp(300.0, full_w - 260.0)
                        },
                        avail_h,
                    ),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.vdl2_messages(ui, &st, avail_h),
                );
            }
            if pane.is_none() {
                ui.separator();
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| self.vdl2_stations(ui, &st, now, avail_h));
            }
        });
    }

    /// Where the receiver is looking, what each channel is doing, and the one
    /// button that fixes the commonest problem.
    ///
    /// Every volatile readout occupies a fixed-width slot: this is a
    /// `horizontal_wrapped`, and a counter that grows a digit would tip the tail
    /// onto a second line and move every pane below it.
    fn vdl2_header(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, st: &Vdl2Status) {
        let dial = self.state.rx_freq_hz();
        let centred = (dial - sdroxide_types::VDL2_PLAN_CENTER_HZ).abs() < 100_000.0
            && st.unavailable.is_none();

        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(22.0);
            ui.label(RichText::new("VDL2").size(11.0).strong().color(theme::CYAN()));
            ui.label(RichText::new("aircraft datalink").weak().size(10.5));

            if crate::chrome::chip(ui, centred, "136.8125")
                .on_hover_text(
                    "Tune to the middle of the VDL2 group. The decoder's own window slides \
                     from there to take in as many of the fourteen channels as the receiver \
                     can reach.",
                )
                .clicked()
            {
                cmds.push(Command::SetVfo {
                    vfo: self.state.active_vfo,
                    hz: sdroxide_types::VDL2_PLAN_CENTER_HZ,
                });
            }

            ui.separator();
            slot(ui, 70.0, &format!("{} frames", count(st.frames)), theme::CYAN());
            slot(ui, 70.0, &format!("{} bursts", count(st.bursts)), theme::gray(150));
            // These are the diagnosis, in the order the chain fails in. Bursts
            // without syncs is a channel busy with something else; syncs
            // without headers is a decoder problem; headers without frames,
            // with this climbing, is a decoder problem one layer further in;
            // and a bad frame check is this decoder misreading a frame the
            // radio path delivered intact.
            slot(ui, 62.0, &format!("{} sync", count(st.syncs)), theme::gray(120));
            slot(ui, 76.0, &format!("{} HDLC bad", count(st.hdlc_bad)), theme::gray(120));
            slot(ui, 68.0, &format!("{} bad FCS", count(st.fcs_bad)), theme::gray(120));

            if st.window_rate_hz > 0.0 {
                slot(
                    ui,
                    150.0,
                    &format!(
                        "{:.3} MHz / {:.0} kHz",
                        st.window_center_hz / 1e6,
                        st.window_rate_hz / 1e3
                    ),
                    theme::gray(120),
                );
            }

            ui.separator();
            if crate::chrome::chip(ui, self.show_vdl2_setup, "SETUP")
                .on_hover_text(
                    "Which channels to listen on, how hard a burst has to be, \
                                and how much log to keep",
                )
                .clicked()
            {
                self.show_vdl2_setup = !self.show_vdl2_setup;
            }
        });

        // The channel strip. Fourteen of them, so a glance says which are being
        // listened to and which are carrying anything — the two questions an
        // empty log raises and nothing else answers. Fixed-width slots, for the
        // reason the counters above have them: a frame count growing a digit
        // would otherwise reflow the strip and move every pane below it.
        if !st.channels.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.set_min_height(18.0);
                for (i, c) in st.channels.iter().enumerate() {
                    let name = format!("{:.3}", c.freq_hz / 1e6);
                    let ink = if !c.live {
                        theme::gray(85)
                    } else if c.frames > 0 {
                        theme::GREEN()
                    } else if c.bursts > 0 {
                        theme::YELLOW()
                    } else {
                        theme::gray(130)
                    };
                    let text = if c.live && c.frames > 0 {
                        format!("{name} ({})", count(c.frames))
                    } else {
                        name
                    };
                    let what = sdroxide_types::VDL2_CHANNEL_LABELS
                        .get(i)
                        .copied()
                        .unwrap_or("VDL2 channel");
                    let hover = match &c.reason {
                        Some(r) => format!("{:.3} MHz, {what} — {r}", c.freq_hz / 1e6),
                        None => format!(
                            "{:.3} MHz, {what} — {} bursts, {} frames, noise floor {:.0} dBFS",
                            c.freq_hz / 1e6,
                            c.bursts,
                            c.frames,
                            c.floor_dbfs
                        ),
                    };
                    // Wide enough for the longest a channel ever reads —
                    // "136.975 (999k)" — since the painter clips to the slot
                    // and a clipped frame count is worse than a wrapped strip.
                    let resp = ui.allocate_response(egui::vec2(86.0, 16.0), egui::Sense::hover());
                    if ui.is_rect_visible(resp.rect) {
                        ui.painter_at(resp.rect).text(
                            egui::pos2(resp.rect.left(), resp.rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &text,
                            egui::FontId::monospace(9.5),
                            ink,
                        );
                    }
                    resp.on_hover_text(hover);
                }
            });
        }

        if let Some(why) = &st.unavailable {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(why).size(10.5).color(theme::HAZARD()));
                if let Some(hz) = st.suggest_center_hz
                    && (dial - hz).abs() > 1.0
                    && crate::chrome::chip(ui, false, format!("TUNE {:.3}", hz / 1e6)).clicked()
                {
                    cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
                }
            });
        }
        // Running, but not on all of it. Said out loud because the symptom — a
        // thin log — is exactly what a quiet hour looks like.
        if let Some(why) = &st.degraded {
            ui.label(RichText::new(why).size(10.5).color(theme::YELLOW()));
        }
    }

    /// The message log, with the full card pinned to the bottom of the column.
    fn vdl2_messages(&mut self, ui: &mut egui::Ui, st: &Vdl2Status, avail_h: f32) {
        // Out of `self` for the length of the draw so the frozen log can be
        // read while the selection beside it is written; back at the end.
        let mut hold = self.vdl2_hold.take();
        ui.horizontal(|ui| {
            ui.set_min_height(20.0);
            ui.label(RichText::new("MESSAGES").strong().size(10.5).color(theme::CYAN()));
            ui.add(
                egui::TextEdit::singleline(&mut self.vdl2_filter)
                    .hint_text("address, flight, registration or label")
                    .desired_width(190.0),
            );
            if !self.vdl2_filter.is_empty() && crate::chrome::chip(ui, false, "×").clicked() {
                self.vdl2_filter.clear();
            }
            // Stop the log, not the decoder: on a busy plate a message is
            // pushed off the bottom before it can be read, and the answer to
            // that is to hold the *view* still. Everything carries on behind
            // it, and what arrived meanwhile is counted on the chip.
            let held = hold.is_some();
            let label = match &hold {
                Some((_, at)) => {
                    let since = st.frames.saturating_sub(*at);
                    if since > 0 { format!("HELD  +{}", count(since)) } else { "HELD".to_string() }
                }
                None => "HOLD".to_string(),
            };
            if crate::chrome::chip(ui, held, label)
                .on_hover_text(
                    "Hold the message log where it is, so one can be read without the next \
                     transmission pushing it up the screen. The decoder keeps running and \
                     the counters keep moving; letting go shows everything that arrived \
                     meanwhile.",
                )
                .clicked()
            {
                // The two lists are different lists, so an index into one means
                // nothing in the other.
                self.vdl2_selected = None;
                hold = if held { None } else { Some((st.messages.clone(), st.frames)) };
            }
        });

        let log: &[Vdl2Message] = match &hold {
            Some((m, _)) => m,
            None => &st.messages,
        };
        let filter = self.vdl2_filter.trim().to_ascii_uppercase();
        let rows: Vec<(usize, &Vdl2Message)> = log
            .iter()
            .enumerate()
            .filter(|(_, m)| filter.is_empty() || matches_filter(m, &filter))
            .collect();

        let card = self.vdl2_selected.and_then(|i| log.get(i));
        let card_h = if card.is_some() { (avail_h * 0.42).clamp(130.0, 260.0) } else { 0.0 };
        let list_h = (avail_h - card_h - 28.0).max(48.0);

        let mut pick = None;
        egui::ScrollArea::vertical()
            .id_salt("vdl2-messages")
            .max_height(list_h)
            .min_scrolled_height(list_h)
            .auto_shrink([false, false])
            // Newest at the bottom, and follow it: a log is read the way a
            // conversation is heard. Not while held — the whole point of
            // holding is that the view stays where it was put.
            .stick_to_bottom(hold.is_none())
            .show_themed(ui, |ui| {
                if rows.is_empty() {
                    ui.label(
                        RichText::new(if log.is_empty() {
                            "nothing decoded yet"
                        } else {
                            "nothing matches the filter"
                        })
                        .weak(),
                    );
                }
                let w = ui.available_width();
                for (i, m) in rows {
                    if message_row(ui, m, self.vdl2_selected == Some(i), w) {
                        pick = Some(i);
                    }
                }
            });
        if let Some(i) = pick {
            self.vdl2_selected = (self.vdl2_selected != Some(i)).then_some(i);
        }

        if let Some(m) = card {
            ui.separator();
            vdl2_card(ui, m, card_h);
        }
        self.vdl2_hold = hold;
    }

    /// Who is out there.
    fn vdl2_stations(&mut self, ui: &mut egui::Ui, st: &Vdl2Status, now: i64, avail_h: f32) {
        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(20.0);
            ui.label(RichText::new("STATIONS").strong().size(10.5).color(theme::CYAN()));
        });

        let filter = self.vdl2_filter.trim().to_ascii_uppercase();
        let mut rows: Vec<&Vdl2Station> = st
            .stations
            .iter()
            .filter(|s| {
                filter.is_empty()
                    || s.hex().contains(&filter)
                    || s.registration.to_ascii_uppercase().contains(&filter)
                    || s.flight.to_ascii_uppercase().contains(&filter)
            })
            .collect();
        sort_stations(&mut rows, self.vdl2_sort, self.vdl2_sort_desc);

        station_head_row(ui, &mut self.vdl2_sort, &mut self.vdl2_sort_desc);
        egui::ScrollArea::vertical()
            .id_salt("vdl2-stations")
            .max_height((avail_h - 40.0).max(48.0))
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if rows.is_empty() {
                    ui.label(RichText::new("nobody heard yet").weak());
                }
                for (i, s) in rows.iter().enumerate() {
                    if station_row(ui, s, now, i) {
                        // Selecting a station is the same as filtering to it:
                        // the log beside it is what an operator wanted to see.
                        self.vdl2_filter = s.hex();
                    }
                }
            });
    }

    /// How the decoder behaves: which channels, how hard a burst has to be, and
    /// how much is kept.
    ///
    /// Its own window rather than a section of the digimode setup dialog, for
    /// the reason the ADS-B one is: that dialog edits a `DigiConfig` and exists
    /// to hold an operator identity and a set of message templates, and this
    /// mode has neither a callsign nor anything to say.
    pub(in crate::app) fn vdl2_setup_window(
        &mut self,
        ctx: &egui::Context,
        cmds: &mut Vec<Command>,
    ) {
        if !self.show_vdl2_setup {
            return;
        }
        let mut open = self.show_vdl2_setup;
        // Edited as a copy and diffed at the end: the engine persists whatever
        // arrives and echoes it back in the state, so there is no apply step and
        // no way for the two copies to drift.
        let mut cfg = self.state.vdl2;
        egui::Window::new("VDL2 Setup")
            .id(crate::layout::salted_id(ctx, "Vdl2Setup"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 420.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.label(RichText::new("Channels").strong());
                ui.label(
                    RichText::new(
                        "Every 25 kHz slot from 136.650 to 136.975 MHz. One downconverter \
                         each, all inside the same receiver window. Switching one off saves \
                         a little processor time; it does not make the others any more \
                         sensitive.",
                    )
                    .size(10.0)
                    .weak(),
                );
                ui.horizontal_wrapped(|ui| {
                    for (i, &hz) in sdroxide_types::VDL2_CHANNELS_HZ.iter().enumerate() {
                        let mut on = cfg.channel_enabled(i);
                        let label = format!("{:.3}", hz / 1e6);
                        let what = sdroxide_types::VDL2_CHANNEL_LABELS[i];
                        let tip = if hz == sdroxide_types::VDL2_CSC_HZ {
                            format!(
                                "{what} — in use worldwide, and where every link starts. \
                                 The one to keep if you keep only one."
                            )
                        } else {
                            format!(
                                "Assigned to an {what}. Which channels carry anything \
                                 depends on where you are: leave them all on unless you \
                                 know otherwise."
                            )
                        };
                        if ui.checkbox(&mut on, label).on_hover_text(tip).changed() {
                            if on {
                                cfg.channels |= 1 << i;
                            } else {
                                cfg.channels &= !(1 << i);
                            }
                        }
                    }
                });
                ui.add_space(6.0);

                egui::Grid::new("vdl2-cfg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Burst threshold");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.threshold_db).range(3..=40).suffix(" dB"),
                        );
                        ui.label(
                            RichText::new("above each channel's own noise floor").size(9.5).weak(),
                        );
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(
                        RichText::new(
                            "Lower catches weaker transmissions and costs processor time on \
                             noise; higher misses them. The floor is learned per channel and \
                             a change here does not throw it away.",
                        )
                        .size(10.0)
                        .weak(),
                    );
                    ui.end_row();

                    ui.label("Keep in the log");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.max_messages)
                                .range(10..=sdroxide_types::VDL2_MESSAGE_MAX)
                                .suffix(" messages"),
                        );
                    });
                    ui.end_row();

                    ui.label("Track at most");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.max_stations)
                                .range(10..=sdroxide_types::VDL2_STATION_MAX)
                                .suffix(" stations"),
                        );
                        ui.label(RichText::new("the longest silent go first").size(9.5).weak());
                    });
                    ui.end_row();

                    ui.label("Forget a station after");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut cfg.drop_list_s)
                                .range(30..=21_600)
                                .suffix(" s"),
                        );
                        ui.label(RichText::new("with nothing heard from it").size(9.5).weak());
                    });
                    ui.end_row();

                    ui.label("Show unread payloads");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut cfg.show_other, "as hex");
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(
                        RichText::new(
                            "Frames carrying X.25, CLNP or the datalink applications above \
                             them. SDRoxide names them and shows the bytes rather than \
                             reading them, and hiding them would hide how much of the \
                             traffic that is.",
                        )
                        .size(10.0)
                        .weak(),
                    );
                    ui.end_row();
                });
            });
        let cfg = cfg.sane();
        if cfg != self.state.vdl2 {
            cmds.push(Command::SetVdl2Config(cfg));
        }
        self.show_vdl2_setup = open;
    }
}

/// Whether a message matches the filter box, which searches everything an
/// operator might type: an address, a flight, a registration or a label.
fn matches_filter(m: &Vdl2Message, filter: &str) -> bool {
    if m.src_hex().contains(filter) || m.dst_hex().contains(filter) {
        return true;
    }
    match &m.payload {
        Vdl2Payload::Acars(a) => {
            a.registration.to_ascii_uppercase().contains(filter)
                || a.flight.to_ascii_uppercase().contains(filter)
                || a.label.to_ascii_uppercase().contains(filter)
                || a.text.to_ascii_uppercase().contains(filter)
        }
        Vdl2Payload::Xid(x) => x.kind.to_ascii_uppercase().contains(filter),
        _ => false,
    }
}

/// The fixed part of a log line — time, channel, the two addresses and the
/// frame type — in points, at the sizes below.
const ROW_PREFIX_W: f32 = 250.0;
/// Roughly how wide one character of the summary is, at 10 points.
const SUMMARY_CHAR_W: f32 = 5.4;

/// One line of the log. Returns true if it was clicked.
///
/// `avail_w` is the pane's width rather than the row's: an `egui` row does not
/// clip, so a long ACARS text would paint straight across the station list
/// beside it. The summary is cut to what will fit instead.
fn message_row(ui: &mut egui::Ui, m: &Vdl2Message, selected: bool, avail_w: f32) -> bool {
    let ink = match &m.payload {
        Vdl2Payload::Acars(_) => theme::YELLOW(),
        Vdl2Payload::Xid(_) => theme::CYAN(),
        Vdl2Payload::Other { .. } => theme::gray(120),
        Vdl2Payload::None => theme::gray(105),
    };
    let resp = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let (_, _, _, h, mi, sec) = sdroxide_types::utc_ymd_hms(m.at);
            ui.label(
                RichText::new(format!("{h:02}:{mi:02}:{sec:02}"))
                    .monospace()
                    .size(9.5)
                    .color(theme::gray(110)),
            );
            ui.label(
                RichText::new(format!("{:.3}", m.freq_hz / 1e6))
                    .monospace()
                    .size(9.5)
                    .color(theme::gray(120)),
            );
            // The direction arrow is the command/response bit, which is the only
            // thing in a frame that says which way it is going.
            ui.label(
                RichText::new(format!("{}→{}", m.src_hex(), m.dst_hex()))
                    .monospace()
                    .size(9.5)
                    .color(if selected { theme::YELLOW() } else { theme::gray(140) }),
            );
            ui.label(RichText::new(m.frame.label()).monospace().size(9.5).color(theme::gray(120)));
            let budget = ((avail_w - ROW_PREFIX_W) / SUMMARY_CHAR_W).max(8.0) as usize;
            ui.label(RichText::new(one_line(&m.summary(), budget)).size(10.0).color(ink));
        })
        .response;
    ui.interact(
        resp.rect,
        ui.id().with(("vdl2-msg", m.at, m.src, m.raw_hex.len())),
        egui::Sense::click(),
    )
    .clicked()
}

/// Everything one frame carried.
fn vdl2_card(ui: &mut egui::Ui, m: &Vdl2Message, h: f32) {
    egui::ScrollArea::vertical()
        .id_salt("vdl2-card")
        .max_height(h)
        .auto_shrink([false, false])
        .show_themed(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{} → {}", m.src_hex(), m.dst_hex()))
                        .monospace()
                        .strong()
                        .size(12.0)
                        .color(theme::YELLOW()),
                );
                ui.label(
                    RichText::new(format!("{} → {}", m.src_kind.short(), m.dst_kind.short()))
                        .size(10.0)
                        .color(theme::CYAN_DIM()),
                );
                ui.label(RichText::new(m.frame.label()).monospace().size(10.5));
                ui.label(
                    RichText::new(if m.command { "command" } else { "response" }).size(10.0).weak(),
                );
            });
            ui.label(
                RichText::new(format!(
                    "{:.3} MHz · {:.0} dB SNR · {:.0} dBFS · EVM {:.1}° · \
                     {:+.0} Hz · {} RS symbols fixed",
                    m.freq_hz / 1e6,
                    m.snr_db,
                    m.rssi_dbfs,
                    m.evm_deg,
                    m.freq_err_hz,
                    m.rs_corrected
                ))
                .size(9.5)
                .weak(),
            );
            ui.separator();

            match &m.payload {
                Vdl2Payload::Acars(a) => {
                    egui::Grid::new("vdl2-acars").num_columns(2).spacing([10.0, 3.0]).show(
                        ui,
                        |ui| {
                            kv(ui, "Registration", &a.registration);
                            kv(ui, "Flight", &a.flight);
                            kv(ui, "Label", &a.label);
                            kv(ui, "Block", &a.block_id.to_string());
                            kv(ui, "Sequence", &a.msn);
                            kv(ui, "Mode", &a.mode.to_string());
                            if a.more {
                                kv(ui, "", "more blocks follow");
                            }
                            kv(
                                ui,
                                "Check",
                                if a.crc_ok {
                                    "message CRC good"
                                } else {
                                    "message CRC not checked"
                                },
                            );
                            if a.parity_errors > 0 {
                                kv(ui, "Parity", &format!("{} characters", a.parity_errors));
                            }
                        },
                    );
                    if !a.text.trim().is_empty() {
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(&a.text).monospace().size(11.0).color(theme::TEXT()),
                        );
                    }
                }
                Vdl2Payload::Xid(x) => {
                    ui.label(RichText::new(&x.kind).strong().size(11.0).color(theme::CYAN()));
                    egui::Grid::new("vdl2-xid").num_columns(2).spacing([10.0, 3.0]).show(
                        ui,
                        |ui| {
                            for (k, v) in &x.params {
                                kv(ui, k, v);
                            }
                        },
                    );
                    if x.unknown > 0 {
                        ui.label(
                            RichText::new(format!(
                                "{} parameter{} SDRoxide does not read — shown above as \
                                 identifier and bytes",
                                x.unknown,
                                if x.unknown == 1 { "" } else { "s" }
                            ))
                            .size(9.5)
                            .weak(),
                        );
                    }
                }
                Vdl2Payload::Other { note, hex } => {
                    ui.label(RichText::new(note).size(10.5).color(theme::gray(150)));
                    ui.label(RichText::new(hex).monospace().size(9.5).weak());
                }
                Vdl2Payload::None => {
                    ui.label(RichText::new("link control, no payload").size(10.5).weak());
                }
            }

            ui.add_space(4.0);
            ui.label(RichText::new("frame").size(9.0).weak());
            ui.label(RichText::new(&m.raw_hex).monospace().size(9.0).color(theme::gray(110)));
        });
}

fn kv(ui: &mut egui::Ui, k: &str, v: &str) {
    if v.trim().is_empty() {
        return;
    }
    ui.label(RichText::new(k).size(10.0).weak());
    ui.label(RichText::new(v).monospace().size(10.5));
    ui.end_row();
}

/// A message summary on one line and no wider than `budget` characters.
///
/// An ACARS text can carry newlines and can be hundreds of characters long. A
/// row that grew to twenty lines would push every other message off the panel,
/// and one that grew sideways would paint across the pane next to it — an
/// `egui` row does not clip. The whole text is in the card a click away.
fn one_line(s: &str, budget: usize) -> String {
    let flat: String = s.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
    let flat = flat.trim();
    if flat.chars().count() > budget {
        let cut: String = flat.chars().take(budget.saturating_sub(1)).collect();
        format!("{cut}…")
    } else {
        flat.to_string()
    }
}

/// The station table's column headings, at the same offsets the rows use — and
/// what re-orders it.
fn station_head_row(ui: &mut egui::Ui, sort: &mut Vdl2Sort, desc: &mut bool) {
    const L: egui::Align2 = egui::Align2::LEFT_CENTER;
    const R: egui::Align2 = egui::Align2::RIGHT_CENTER;
    let cols = station_columns(ui.available_width());
    crate::app::panels::widgets::sort_head_row(
        ui,
        &[
            (cols.name, L, "NAME", Some(Vdl2Sort::Name)),
            (cols.addr, L, "ADDR", Some(Vdl2Sort::Address)),
            // Ground station or aircraft, which is two values: an order on it
            // would be a grouping, and the address column already gives one.
            (cols.kind, L, "TYPE", None),
            (cols.msgs, R, "MSGS", Some(Vdl2Sort::Messages)),
            (cols.sig, R, "SIG", Some(Vdl2Sort::Signal)),
            (cols.age, R, "AGE", Some(Vdl2Sort::Heard)),
        ],
        sort,
        desc,
    );
}

struct StationCols {
    name: f32,
    addr: f32,
    kind: f32,
    msgs: f32,
    sig: f32,
    age: f32,
}

/// Below this the signal column comes off the table, and below [`TINY_W`] the
/// type and count columns go too.
///
/// A row that has run out of room prints its columns on top of each other,
/// which is worse than not printing them at all — the operator cannot tell
/// which number they are reading.
const NARROW_W: f32 = 300.0;
const TINY_W: f32 = 220.0;

/// Where each column sits, given the width available.
///
/// Fixed offsets rather than a layout, so the table reads down as well as
/// across; the trailing columns drop out rather than overprinting each other.
/// `f32::NAN` means "not drawn".
fn station_columns(w: f32) -> StationCols {
    let age = w - 4.0;
    let sig = if w < NARROW_W { f32::NAN } else { age - 34.0 };
    let msgs = if w < TINY_W {
        f32::NAN
    } else if sig.is_nan() {
        age - 34.0
    } else {
        sig - 38.0
    };
    let kind = if w < TINY_W { f32::NAN } else { 118.0 };
    StationCols { name: 5.0, addr: 66.0, kind, msgs, sig, age }
}

/// One row of the station table. Returns true if it was clicked.
fn station_row(ui: &mut egui::Ui, s: &Vdl2Station, now: i64, i: usize) -> bool {
    const ROW_H: f32 = 17.0;
    const ACCENT_W: f32 = 2.5;

    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return false;
    }
    let p = ui.painter_at(rect);
    let ground = s.kind.is_ground();
    let (accent, ink) =
        if ground { (theme::CYAN_DIM(), theme::CYAN()) } else { (theme::YELLOW(), theme::TEXT()) };
    if i % 2 == 1 {
        p.rect_filled(rect, 0.0, theme::ROW_BG());
    }
    p.rect_filled(
        egui::Rect::from_min_size(rect.left_top(), egui::vec2(ACCENT_W, rect.height())),
        0.0,
        accent,
    );

    let font = egui::FontId::monospace(9.5);
    let cols = station_columns(w);
    let dim = theme::gray(130);
    for (x, align, text, color) in [
        (cols.name, egui::Align2::LEFT_CENTER, s.label(), ink),
        (cols.addr, egui::Align2::LEFT_CENTER, s.hex(), dim),
        (cols.kind, egui::Align2::LEFT_CENTER, s.kind.short().to_string(), dim),
        (cols.msgs, egui::Align2::RIGHT_CENTER, s.messages.to_string(), dim),
        (cols.sig, egui::Align2::RIGHT_CENTER, format!("{:.0}", s.last_snr_db), dim),
        (cols.age, egui::Align2::RIGHT_CENTER, fmt_age(now - s.last_at), dim),
    ] {
        if x.is_nan() {
            continue;
        }
        p.text(egui::pos2(rect.left() + x, rect.center().y), align, &text, font.clone(), color);
    }

    ui.interact(rect, ui.id().with(("vdl2-station", s.addr)), egui::Sense::click()).clicked()
}

fn sort_stations(rows: &mut [&Vdl2Station], sort: Vdl2Sort, desc: bool) {
    match sort {
        Vdl2Sort::Heard => rows.sort_by_key(|s| s.last_at),
        Vdl2Sort::Name => rows.sort_by_key(|s| s.label().to_ascii_uppercase()),
        Vdl2Sort::Address => rows.sort_by_key(|s| s.addr),
        Vdl2Sort::Messages => rows.sort_by_key(|s| s.messages),
        Vdl2Sort::Signal => rows.sort_by(|a, b| a.last_snr_db.total_cmp(&b.last_snr_db)),
    }
    if desc {
        rows.reverse();
    }
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
    use sdroxide_types::{Vdl2AddrKind, Vdl2Frame};

    fn station(addr: u32, flight: &str, msgs: u32, last: i64) -> Vdl2Station {
        let mut s = Vdl2Station::new(addr, Vdl2AddrKind::Aircraft, last);
        s.flight = flight.to_string();
        s.messages = msgs;
        s.last_at = last;
        s
    }

    fn msg(src: u32, payload: Vdl2Payload) -> Vdl2Message {
        Vdl2Message {
            at: 1000,
            freq_hz: 136_975_000.0,
            src,
            src_kind: Vdl2AddrKind::Aircraft,
            dst: 0x10_00_01,
            dst_kind: Vdl2AddrKind::GroundAdmin,
            command: true,
            frame: Vdl2Frame::Ui { pf: false },
            payload,
            snr_db: 20.0,
            rssi_dbfs: -30.0,
            evm_deg: 5.0,
            freq_err_hz: 10.0,
            rs_corrected: 0,
            raw_hex: "DE AD".to_string(),
        }
    }

    /// Sorting is stable in both directions, and clicking the chip again
    /// reverses rather than re-sorting arbitrarily.
    #[test]
    fn the_station_table_sorts_both_ways() {
        let a = station(3, "AUA1", 5, 100);
        let b = station(1, "BAW2", 9, 300);
        let c = station(2, "CFG3", 1, 200);
        let mut rows: Vec<&Vdl2Station> = vec![&a, &b, &c];
        sort_stations(&mut rows, Vdl2Sort::Heard, true);
        assert_eq!(rows.iter().map(|s| s.addr).collect::<Vec<_>>(), vec![1, 2, 3]);
        sort_stations(&mut rows, Vdl2Sort::Heard, false);
        assert_eq!(rows.iter().map(|s| s.addr).collect::<Vec<_>>(), vec![3, 2, 1]);
        sort_stations(&mut rows, Vdl2Sort::Messages, true);
        assert_eq!(rows.iter().map(|s| s.messages).collect::<Vec<_>>(), vec![9, 5, 1]);
        sort_stations(&mut rows, Vdl2Sort::Address, false);
        assert_eq!(rows.iter().map(|s| s.addr).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    /// The filter searches everything an operator might type, in either case.
    #[test]
    fn the_filter_searches_what_an_operator_would_type() {
        let acars = sdroxide_types::Vdl2Acars {
            registration: "OE-LWA".to_string(),
            flight: "AUA123".to_string(),
            label: "H1".to_string(),
            text: "REQUEST DESCENT".to_string(),
            ..sdroxide_types::Vdl2Acars::default()
        };
        let m = msg(0x44_0F_31, Vdl2Payload::Acars(acars));
        for f in ["440F31", "OE-LWA", "AUA123", "H1", "DESCENT", "10000"] {
            assert!(matches_filter(&m, f), "{f} did not match");
        }
        assert!(!matches_filter(&m, "NOTHERE"));
    }

    /// A counter never grows past its slot, however long the decoder runs.
    #[test]
    fn a_counter_stays_short() {
        for n in [0u64, 9_999, 10_000, 994_999, 1_000_000, 12_345_678_901] {
            assert!(count(n).len() <= 6, "{n} formats as {}", count(n));
        }
    }

    /// A log line stays one line and inside its budget, whatever an airline
    /// puts in an ACARS text.
    #[test]
    fn a_log_line_is_one_line_and_fits() {
        let s = one_line("first\r\nsecond\nthird", 80);
        assert!(!s.contains('\n') && !s.contains('\r'), "{s}");
        let long = "x".repeat(400);
        for budget in [8usize, 20, 96, 400] {
            assert!(one_line(&long, budget).chars().count() <= budget, "budget {budget}");
        }
        // A short message is left alone rather than padded or cut.
        assert_eq!(one_line("short", 80), "short");
    }

    /// The station columns never overlap, at any width the panel can take.
    #[test]
    fn the_station_columns_do_not_overlap() {
        for w in (140..=1400).step_by(10) {
            let c = station_columns(w as f32);
            let xs: Vec<f32> = [c.name, c.addr, c.kind, c.msgs, c.sig, c.age]
                .into_iter()
                .filter(|x| !x.is_nan())
                .collect();
            for pair in xs.windows(2) {
                assert!(pair[1] > pair[0], "columns cross at width {w}: {xs:?}");
            }
        }
    }
}
