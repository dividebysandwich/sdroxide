//! The packet panel: what is on the channel, and what the link is doing.
//!
//! A packet operator watches two things that move independently — every frame
//! heard, and the session they are in — so they get a pane each rather than
//! sharing one.
//!
//! The terminal is the whole of connected mode from the operator's side:
//! CONNECT calls a node, a BBS or a peer, the scrollback is what the far end
//! has said, and the line at the bottom is what you say back. It is one link:
//! a Winlink session driven from the MAIL window and this pane are two ways to
//! use one radio, and whichever asks first gets it — the other is told so, here,
//! in this pane's own transcript.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Mode, PacketLinkOwner, PacketStatus, PacketTermKind};

use crate::app::{SdroxideApp, tx_gated};
use crate::theme;

impl SdroxideApp {
    pub(in crate::app) fn packet_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<PacketStatus> = self.digi_status.as_ref().and_then(|s| s.packet.clone());
        let Some(st) = st else {
            ui.label(RichText::new("starting the packet modem…").weak());
            return;
        };
        // Monitoring the channel is worth doing on a receiver; announcing
        // ourselves on it is not.
        let tx_ok = self.tx_capable();
        let mode = self.digi_status.as_ref().map_or(Mode::Packet, |s| s.mode);
        let pane = self.phone_pane(ui, mode);

        ui.horizontal(|ui| {
            // ── MONITOR ───────────────────────────────────────────────────
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        ui.set_width(ui.available_width() * 0.55);
                    }
                    self.packet_monitor(ui, cmds, &st, panel_h);
                });
            }

            if pane.is_none() {
                ui.separator();
            }

            // ── TERMINAL ──────────────────────────────────────────────────
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    self.packet_terminal(ui, cmds, &st, panel_h, tx_ok);
                });
            }
        });
    }

    /// Every frame on the channel, ours in a different colour.
    fn packet_monitor(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &PacketStatus,
        panel_h: f32,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("MONITOR").strong().color(theme::CYAN()));
            ui.label(RichText::new(format!("{} baud", st.baud.label())).weak());
            // Channel busy and bad frames are the two numbers that explain a
            // link that is not working: one says somebody else is talking, the
            // other says the path is marginal.
            if st.dcd {
                ui.label(RichText::new("BUSY").strong().color(theme::ALERT()));
            }
            if st.bad_frames > 0 {
                ui.label(
                    RichText::new(format!("{} bad", st.bad_frames)).weak().color(theme::ALERT()),
                )
                .on_hover_text(
                    "Frames that arrived but failed their check sequence — a collision, a fade, \
                     or a signal too weak to read.",
                );
            }
            crate::chrome::row_tail(ui, |ui| {
                self.clear_rx_chip(ui, cmds);
                // The only route to the packet settings there is. Everything
                // this mode needs before it can transmit at all — the station
                // callsign above all — lives in that window, and until this
                // chip existed nothing anywhere opened it: an operator told to
                // "set a station callsign in the packet settings first" had
                // nowhere to go and look (issue #159).
                if crate::chrome::chip(
                    ui,
                    self.show_digi_settings,
                    RichText::new("⚙ SETUP").size(9.5),
                )
                .on_hover_text(
                    "Station callsign, speed, TX delay, the digipeater path, the beacon and the \
                     KISS server",
                )
                .clicked()
                {
                    self.show_digi_settings = !self.show_digi_settings;
                }
            });
        });

        egui::ScrollArea::vertical()
            .id_salt("packet-monitor")
            .max_height(panel_h - 40.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if st.heard.is_empty() {
                    ui.label(RichText::new("nothing heard yet").weak());
                }
                for h in &st.heard {
                    let via = if h.via.is_empty() {
                        String::new()
                    } else {
                        format!(",{}", h.via.join(","))
                    };
                    // Our own traffic in a different colour: an operator needs
                    // to tell their beacon going out from somebody answering it.
                    let colour = if h.sent { theme::GREEN() } else { theme::CYAN_DIM() };
                    ui.horizontal_wrapped(|ui| {
                        // Clicking a station addresses the connect bar to it,
                        // which is how an operator finds out who is reachable:
                        // by watching the channel, not by typing callsigns from
                        // memory.
                        let who = RichText::new(format!("{}>{}{via}", h.from, h.to))
                            .monospace()
                            .color(colour);
                        if ui.add(egui::Label::new(who).sense(egui::Sense::click())).clicked()
                            && !h.sent
                            && !h.from.is_empty()
                        {
                            self.packet_target = h.from.clone();
                            self.packet_via = h.via.join(",");
                        }
                        ui.label(RichText::new(&h.kind).monospace().weak());
                        if !h.text.is_empty() {
                            ui.label(RichText::new(&h.text).monospace());
                        }
                    });
                }
            });
    }

    /// The connected session: who we are with, what was said, and the line to
    /// say something back on.
    fn packet_terminal(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &PacketStatus,
        panel_h: f32,
        tx_ok: bool,
    ) {
        let link = st.link.clone().unwrap_or_default();
        let connected = link.state == "Connected";
        let busy = link.owner == PacketLinkOwner::Session;
        // Anything that is not Disconnected and not Connected is on its way to
        // one of them: the SABM is out, or the DISC is, or the link is retrying.
        let working = !connected && link.state != "Disconnected";

        // ── status row ────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label(RichText::new("TERMINAL").strong().color(theme::CYAN()));
            let (label, colour) = if connected {
                (" CONNECTED ", theme::GREEN())
            } else if working {
                (" CALLING ", theme::YELLOW())
            } else {
                (" IDLE ", theme::CYAN_DIM())
            };
            ui.label(RichText::new(label).strong().color(colour))
                .on_hover_text(format!("The link layer is in {}", link.state));
            if let Some(peer) = &link.peer
                && (connected || working)
            {
                let via = if link.via.is_empty() {
                    String::new()
                } else {
                    format!(" via {}", link.via.join(","))
                };
                ui.label(RichText::new(format!("{peer}{via}")).monospace());
            }
            if link.ext {
                ui.label(RichText::new("mod-128").weak())
                    .on_hover_text("Extended sequence numbers — a window of up to 127 frames.");
            }
            // The two numbers that explain a session that has gone quiet.
            if link.unacked > 0 {
                ui.label(RichText::new(format!("{} unacked", link.unacked)).weak())
                    .on_hover_text("Frames sent and not yet acknowledged by the far end.");
            }
            if link.retries > 0 {
                ui.label(
                    RichText::new(format!("retry {}", link.retries)).weak().color(theme::ALERT()),
                )
                .on_hover_text(
                    "Retries against this link's limit. A count climbing while the unacknowledged \
                     frames stay put is what a fading path looks like from this side.",
                );
            }
        });

        if busy {
            ui.label(
                RichText::new("The MAIL window is using the link.").weak().color(theme::YELLOW()),
            )
            .on_hover_text(
                "One radio, one channel, one link. A Winlink session and this terminal are two \
                 ways to use it, and whichever asks first gets it.",
            );
        }

        // ── connect bar ───────────────────────────────────────────────────
        if !self.ui_settings.swl {
            let mut connect = false;
            // Nothing transmits without a station callsign, and the refusal used to
            // arrive only after CONNECT was pressed. Saying so on the button is the
            // difference between a setting to go and find and a mode that looks
            // broken (issue #159).
            let have_call = !self.digi_cfg_edit.packet_mycall.trim().is_empty();
            ui.horizontal(|ui| {
                let can_edit = !connected && !working;
                ui.add_enabled_ui(can_edit, |ui| {
                    let resp = crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.packet_target)
                            .desired_width(84.0)
                            .hint_text("callsign"),
                    );
                    if resp.changed() {
                        self.packet_target = self.packet_target.to_uppercase();
                    }
                    connect |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let resp = crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.packet_via)
                            .desired_width(120.0)
                            .hint_text("via"),
                    )
                    .on_hover_text(
                        "Digipeaters, nearest first, separated by commas — OE3XLR-1,OE3XMS-1. Leave \
                         it empty for a station you can hear directly, or to use the default path \
                         from the packet settings.",
                    );
                    if resp.changed() {
                        self.packet_via = self.packet_via.to_uppercase();
                    }
                    connect |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                });

                let ready = !self.packet_target.trim().is_empty();
                if connected || working {
                    if tx_gated(ui, tx_ok, |ui| {
                        crate::chrome::chip_accent(
                            ui,
                            false,
                            RichText::new(" DISCONNECT ").strong(),
                            theme::ALERT(),
                            theme::INK_ON_CYAN(),
                        )
                        .on_hover_text(
                            "Hang up properly, and wait for the far end to agree. Changing mode \
                             instead ends the session without telling anybody.",
                        )
                    })
                    .clicked()
                    {
                        cmds.push(Command::PacketDisconnect);
                    }
                } else if tx_gated(ui, tx_ok && ready && have_call && !busy, |ui| {
                    crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new(" CONNECT ").strong(),
                        theme::GREEN(),
                        theme::INK_ON_CYAN(),
                    )
                    .on_hover_text(if busy {
                        "The MAIL window has the link. Finish or stop that session first."
                    } else if !have_call {
                        "This station has no callsign yet — set one under SETUP, above the monitor \
                         pane. Nothing transmits until it is set."
                    } else if ready {
                        "Call this station in connected mode — a node, a BBS, or another operator."
                    } else {
                        "Needs a callsign to call."
                    })
                })
                .clicked()
                {
                    connect = true;
                }

                crate::chrome::row_tail(ui, |ui| {
                    if crate::chrome::chip(ui, false, RichText::new(" CLEAR ").size(10.5))
                        .on_hover_text(
                            "Empty the transcript. The link is untouched — a connected station stays \
                             connected.",
                        )
                        .clicked()
                    {
                        cmds.push(Command::PacketTermClear);
                    }
                });
            });

            if connect
                && tx_ok
                && have_call
                && !connected
                && !working
                && !self.packet_target.trim().is_empty()
            {
                cmds.push(Command::PacketConnect {
                    call: self.packet_target.trim().to_string(),
                    via: self.packet_via.trim().to_string(),
                    ext: self.digi_cfg_edit.packet_ext_seq,
                });
            }
        }

        // ── scrollback ────────────────────────────────────────────────────
        let input_h = 26.0;
        egui::ScrollArea::vertical()
            .id_salt("packet-term")
            .max_height((panel_h - 76.0 - input_h).max(60.0))
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if st.term.is_empty() && st.term_partial.is_empty() {
                    ui.label(
                        RichText::new(
                            "Not connected. Type a callsign above and press CONNECT to call a \
                             node or a BBS.",
                        )
                        .weak(),
                    );
                }
                for l in &st.term {
                    let colour = match l.kind {
                        PacketTermKind::Rx => theme::TEXT(),
                        PacketTermKind::Tx => theme::GREEN(),
                        PacketTermKind::Note => theme::CYAN_DIM(),
                    };
                    ui.label(RichText::new(&l.text).monospace().color(colour));
                }
                // The unterminated tail, on its own line and with no break
                // after it. This is the prompt: a BBS ends its question without
                // a carriage return and waits, so a pane that showed only whole
                // lines would show nothing at the moment it matters most.
                if !st.term_partial.is_empty() {
                    ui.label(RichText::new(&st.term_partial).monospace().color(theme::TEXT()));
                }
            });

        // ── the line you type on ──────────────────────────────────────────
        if !self.ui_settings.swl {
            let mut send = false;
            ui.horizontal(|ui| {
                let room = (ui.available_width() - 56.0).max(80.0);
                ui.add_enabled_ui(connected, |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.packet_draft)
                            .desired_width(room)
                            .hint_text(if connected { "type here" } else { "not connected" }),
                    );
                    if resp.has_focus() {
                        self.packet_recall(ui, &resp);
                    }
                    send |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                });
                if tx_gated(ui, tx_ok && connected, |ui| {
                    crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new(" SEND ").size(10.0).strong(),
                        theme::GREEN(),
                        theme::INK_ON_CYAN(),
                    )
                    .on_hover_text("Send the line. A node or a BBS reads one line at a time.")
                })
                .clicked()
                {
                    send = true;
                }
            });

            // Return goes through the same gate as the button: a keystroke must not
            // do what a greyed-out button refuses to.
            if send && tx_ok && connected {
                let line = self.packet_draft.trim_end().to_string();
                cmds.push(Command::PacketSend { text: line.clone() });
                // An empty line is a real thing to send — it is how you get a BBS
                // to reprint its prompt — but it is not worth remembering.
                if !line.trim().is_empty() && self.packet_history.last() != Some(&line) {
                    self.packet_history.push(line);
                    if self.packet_history.len() > 100 {
                        self.packet_history.remove(0);
                    }
                }
                self.packet_history_at = None;
                self.packet_draft.clear();
            }
        }
    }

    /// Up and down walk back through what has already been sent.
    ///
    /// A node's command line is retyped constantly — the same `L`, the same
    /// `C CALL` — and at 300 baud, retyping is where the typos come from.
    fn packet_recall(&mut self, ui: &egui::Ui, resp: &egui::Response) {
        let (up, down) =
            ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
        if !up && !down {
            // Anything else typed means they are writing something new, so the
            // next Up starts from the end again rather than from wherever the
            // last walk left off.
            if resp.changed() {
                self.packet_history_at = None;
            }
            return;
        }
        if self.packet_history.is_empty() {
            return;
        }
        let last = self.packet_history.len() - 1;
        self.packet_history_at = match (self.packet_history_at, up) {
            (None, true) => Some(last),
            (None, false) => None,
            (Some(0), true) => Some(0),
            (Some(n), true) => Some(n - 1),
            (Some(n), false) if n >= last => None,
            (Some(n), false) => Some(n + 1),
        };
        self.packet_draft = match self.packet_history_at {
            Some(n) => self.packet_history[n].clone(),
            None => String::new(),
        };
    }
}
