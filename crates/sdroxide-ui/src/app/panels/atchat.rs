//! The AtCHAT NET panel: the roster, the conversation, the transfers in flight
//! and a viewer for the images that have arrived over the air.
//!
//! Two panes an operator watches move independently. CHAT is who is on the net
//! and what has been said: a tab row sits above the transcript — the common
//! "CHAT" tab carries the ALL traffic, and a direct message from a station
//! opens a closable per-station tab beside it (one per correspondent). What is
//! typed goes wherever the active tab points: ALL from CHAT, that one station
//! from a DM tab. FILES is the block-CRC-ARQ transfers still running and the
//! received-image viewer, which shows who sent each picture and when, with
//! ◀ ▶ between them.
//!
//! The mode runs a whole NET protocol station on its own thread — master
//! election, roster ageing, ARQ — so this panel only reads
//! [`sdroxide_types::AtChatStatus`] and pushes the `Command::AtChat*`.

use eframe::egui::{self, RichText};
use sdroxide_types::{AtChatStatus, Command};

use crate::app::{SdroxideApp, tx_gated};
use crate::theme;

impl SdroxideApp {
    pub(in crate::app) fn atchat_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<Box<AtChatStatus>> =
            self.digi_status.as_ref().and_then(|s| s.atchat.clone());
        let Some(st) = st else {
            ui.label(RichText::new("starting the AtCHAT NET station…").weak());
            return;
        };
        // On the virtual TCP channel nothing keys a transmitter, so sending a
        // line or a file needs no TX-capable radio — only the on-air path does.
        // Gate SEND on that, not on [`tx_capable`] alone, or a radio-less dev
        // station can join the net but never speak on it.
        let can_send = self.tx_capable() || st.virtual_addr.is_some();

        self.atchat_header(ui, cmds, &st);
        ui.add_space(6.0);

        let pane = self.phone_pane(ui, sdroxide_types::Mode::AtChat);
        ui.horizontal_top(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        ui.set_width(ui.available_width() * 0.56);
                    }
                    self.atchat_chat_pane(ui, cmds, &st, panel_h, can_send);
                });
            }
            if pane.is_none() {
                ui.separator();
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    self.atchat_files_pane(ui, cmds, &st, panel_h, can_send);
                });
            }
        });
    }

    /// The station the active chat tab points at — empty for the common "CHAT"
    /// tab (ALL), a callsign for a direct-message tab.
    fn atchat_target(&self) -> String {
        self.atchat_chat_tab.clone().unwrap_or_default()
    }

    /// Title, join state, role, master, carrier, and the virtual-channel field.
    fn atchat_header(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, st: &AtChatStatus) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("ATCHAT").size(11.0).strong().color(theme::CYAN()));
            ui.label(
                RichText::new("NET — multi-station keyboard & file")
                    .size(10.5)
                    .color(theme::CYAN_DIM()),
            );

            // The join badge doubles as the leave/rejoin control: green "ON NET"
            // while joined, plain "REJOIN" once off. Clicking it drops the link
            // (state is kept) or rejoins — sometimes you just need to cycle.
            let (face, hover) = if st.connected {
                (" ON NET ", "On the net — click to leave (state is kept; click again to rejoin)")
            } else {
                (" REJOIN ", "Off the net — click to rejoin")
            };
            let resp = crate::chrome::chip_accent(
                ui,
                st.connected,
                RichText::new(face).size(10.5).strong(),
                theme::GREEN(),
                theme::INK_ON_CYAN(),
            );
            if resp.clicked() {
                cmds.push(if st.connected {
                    Command::AtChatDrop
                } else {
                    Command::AtChatReconnect
                });
            }
            resp.on_hover_text(hover);
            if !st.connected {
                ui.label(RichText::new("off net").size(10.0).color(theme::YELLOW()));
            }

            if let Some(role) = &st.role {
                let colour = match role.as_str() {
                    "MASTER" => theme::GREEN(),
                    "BACKUP" => theme::YELLOW(),
                    _ => theme::gray(140),
                };
                ui.label(RichText::new(role).size(10.5).strong().color(colour));
            }
            if let Some(m) = &st.master {
                ui.label(RichText::new(format!("master {m}")).size(10.5).color(theme::gray(150)));
            }
            if st.carrier {
                ui.label(RichText::new("● CARRIER").size(10.5).color(theme::YELLOW()))
                    .on_hover_text("Another station is transmitting — the channel is busy.");
            }
            if st.keyed {
                ui.label(RichText::new("● TX").size(10.5).strong().color(theme::ALERT()));
            }

            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(
                    ui,
                    self.show_digi_settings,
                    RichText::new("⚙ SETUP").size(9.5),
                )
                .on_hover_text("Station callsign and the virtual-channel address")
                .clicked()
                {
                    self.show_digi_settings = !self.show_digi_settings;
                }
            });
        });

        // Virtual channel: a channel_server-compatible TCP endpoint that stands
        // in for the RF path, for developing and testing without a radio.
        ui.horizontal_wrapped(|ui| {
            let on = self.digi_cfg_edit.atchat_virtual;
            let resp = crate::chrome::chip(ui, on, RichText::new("VIRTUAL CHANNEL").size(10.0));
            if resp.clicked() && self.digi_cfg_seeded {
                self.digi_cfg_edit.atchat_virtual = !on;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
            resp.on_hover_text(
                "Work the net over a TCP endpoint instead of the radio — a \
                 channel_server-compatible loopback for radio-less development. \
                 Off puts the station on the air.",
            );
            ui.add_enabled_ui(on, |ui| {
                let r = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.digi_cfg_edit.atchat_virtual_addr)
                        .desired_width(140.0)
                        .hint_text("127.0.0.1:6000"),
                );
                if r.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && self.digi_cfg_seeded
                {
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
            });
            if on {
                ui.label(
                    RichText::new(st.virtual_addr.as_deref().unwrap_or("—"))
                        .size(9.5)
                        .color(theme::gray(130)),
                );
            }
        });
    }

    /// The roster strip, the CHAT / per-station tab row, and the conversation.
    fn atchat_chat_pane(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &AtChatStatus,
        panel_h: f32,
        can_send: bool,
    ) {
        // Open a DM tab for any incoming private line newer than the newest we
        // have already turned into a tab for that peer. Closing a tab does not
        // clear that mark, so an old message never reopens it — a genuinely new
        // one does.
        for c in &st.chat {
            if c.private && !c.own {
                let seen = self.atchat_dm_seen.get(&c.from).copied().unwrap_or(0);
                if c.when > seen {
                    if !self.atchat_dm_tabs.iter().any(|p| p == &c.from) {
                        self.atchat_dm_tabs.push(c.from.clone());
                    }
                    self.atchat_dm_seen.insert(c.from.clone(), c.when);
                }
            }
        }
        // A tab whose peer has left the roster and holds no history is stale.
        self.atchat_dm_tabs.retain(|peer| {
            st.roster.iter().any(|r| &r.call == peer)
                || st.chat.iter().any(|c| c.private && (&c.from == peer || &c.dst == peer))
        });
        if let Some(cur) = self.atchat_chat_tab.clone()
            && !self.atchat_dm_tabs.iter().any(|p| p == &cur)
        {
            self.atchat_chat_tab = None;
        }

        // Roster strip — a click opens/focuses that station's DM tab.
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("ROSTER").strong().size(10.5).color(theme::CYAN()));
            if st.roster.is_empty() {
                ui.label(RichText::new("nobody heard yet").weak());
            }
            for r in &st.roster {
                let colour = if r.status == "active" { theme::GREEN() } else { theme::gray(110) };
                let face = RichText::new(&r.call).monospace().size(10.5).color(colour);
                if ui
                    .add(egui::Label::new(face).sense(egui::Sense::click()))
                    .on_hover_text(format!(
                        "{} — last heard {:.0}s ago · click to open a direct-message tab",
                        r.status, r.age_s
                    ))
                    .clicked()
                {
                    if !self.atchat_dm_tabs.iter().any(|p| p == &r.call) {
                        self.atchat_dm_tabs.push(r.call.clone());
                    }
                    self.atchat_chat_tab = Some(r.call.clone());
                    self.atchat_show_log = false;
                }
            }
        });
        ui.add_space(4.0);

        // Tab row: CHAT (common), one closable tab per DM correspondent, and a
        // permanent LOG tab last. A framed strip sets the tabs apart from the
        // roster above and the transcript below.
        let mut to_close: Option<String> = None;
        // A `Frame` auto-sizes by measuring its content with unbounded width, so
        // the wrapped row inside would never wrap. Pin it to the width there is.
        let strip_w = (ui.available_width() - 12.0).max(80.0);
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, theme::CYAN_DIM()))
            .corner_radius(egui::CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(6, 3))
            .show(ui, |ui| {
                ui.set_min_width(strip_w);
                ui.set_max_width(strip_w);
                ui.horizontal_wrapped(|ui| {
                    let chat_active = !self.atchat_show_log && self.atchat_chat_tab.is_none();
                    if crate::chrome::chip(ui, chat_active, RichText::new(" CHAT ").size(10.5))
                        .on_hover_text("The common channel — lines here go to everyone (ALL)")
                        .clicked()
                    {
                        self.atchat_chat_tab = None;
                        self.atchat_show_log = false;
                    }
                    for peer in self.atchat_dm_tabs.clone() {
                        let active = !self.atchat_show_log
                            && self.atchat_chat_tab.as_deref() == Some(peer.as_str());
                        // Unread when the newest incoming line from this peer is
                        // newer than the last one seen while its tab was open.
                        let newest_in = st
                            .chat
                            .iter()
                            .filter(|c| c.private && !c.own && c.from == peer)
                            .map(|c| c.when)
                            .max()
                            .unwrap_or(0);
                        let unread = !active
                            && newest_in > self.atchat_dm_read.get(&peer).copied().unwrap_or(0);
                        let face =
                            if unread { format!("● {peer} ") } else { format!(" {peer} ") };
                        if crate::chrome::chip(ui, active, RichText::new(face).size(10.5)).clicked()
                        {
                            self.atchat_chat_tab = Some(peer.clone());
                            self.atchat_show_log = false;
                        }
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new("✕").size(9.5).color(theme::gray(130)),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(format!("close the {peer} tab"))
                            .clicked()
                        {
                            to_close = Some(peer.clone());
                        }
                    }
                    ui.separator();
                    if crate::chrome::chip(
                        ui,
                        self.atchat_show_log,
                        RichText::new(" LOG ").size(10.5),
                    )
                    .on_hover_text(
                        "The station's own on-air activity — master election, \
                             roster ageing, ARQ retries",
                    )
                    .clicked()
                    {
                        self.atchat_show_log = true;
                    }
                });
            });
        if let Some(peer) = to_close {
            self.atchat_dm_tabs.retain(|p| p != &peer);
            if self.atchat_chat_tab.as_deref() == Some(peer.as_str()) {
                self.atchat_chat_tab = None;
            }
        }
        ui.add_space(4.0);

        // Transcript for the active tab — or the on-air activity log when the
        // LOG tab is up.
        let show_log = self.atchat_show_log;
        let active = self.atchat_chat_tab.clone();
        let salt = if show_log {
            "__log__".to_string()
        } else {
            active.clone().unwrap_or_else(|| "__common__".into())
        };
        let input_h = if show_log { 0.0 } else { 30.0 };
        egui::ScrollArea::vertical()
            .id_salt(("atchat-chat", salt))
            .max_height((panel_h - 128.0 - input_h).max(60.0))
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if show_log {
                    if st.log.is_empty() {
                        ui.label(RichText::new("No activity logged yet.").weak());
                    }
                    for line in &st.log {
                        ui.label(
                            RichText::new(line).monospace().size(10.0).color(theme::gray(150)),
                        );
                    }
                    return;
                }
                let mut shown = 0usize;
                for c in &st.chat {
                    let in_tab = match &active {
                        None => !c.private,
                        Some(peer) => {
                            c.private && ((c.own && &c.dst == peer) || (!c.own && &c.from == peer))
                        }
                    };
                    if !in_tab {
                        continue;
                    }
                    shown += 1;
                    let when = hms(c.when);
                    let who = if c.own { "me".to_string() } else { c.from.clone() };
                    let tag = format!("<{who}>");
                    let colour = if c.own { theme::GREEN() } else { theme::TEXT() };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(when).monospace().size(9.5).color(theme::gray(110)));
                        ui.label(RichText::new(tag).monospace().size(10.5).color(colour));
                        ui.label(RichText::new(&c.text).size(11.0).color(colour));
                    });
                }
                if shown == 0 {
                    ui.label(RichText::new("No messages yet.").weak());
                }
            });

        // Nothing to type on while the LOG tab is up — it is read-only.
        if show_log {
            return;
        }

        // Viewing a DM tab marks it read up to its newest incoming line.
        if let Some(peer) = &active {
            let newest_in = st
                .chat
                .iter()
                .filter(|c| c.private && !c.own && &c.from == peer)
                .map(|c| c.when)
                .max()
                .unwrap_or(0);
            self.atchat_dm_read.insert(peer.clone(), newest_in);
        }

        // The line to type on — its destination is the active tab.
        let mut send = false;
        let hint = match &active {
            None => "message to everyone (ALL)".to_string(),
            Some(p) => format!("direct message to {p}"),
        };
        ui.horizontal(|ui| {
            let room = (ui.available_width() - 52.0).max(80.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.atchat_draft)
                    .desired_width(room)
                    .hint_text(hint),
            );
            send |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if tx_gated(ui, can_send, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(" SEND ").size(10.0).strong(),
                    theme::GREEN(),
                    theme::INK_ON_CYAN(),
                )
            })
            .clicked()
            {
                send = true;
            }
        });

        if send && can_send && !self.atchat_draft.trim().is_empty() {
            cmds.push(Command::AtChatSendChat {
                to: self.atchat_target(),
                text: self.atchat_draft.trim().to_string(),
            });
            self.atchat_draft.clear();
        }
    }

    /// The transfers in flight and the received-image viewer.
    fn atchat_files_pane(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &AtChatStatus,
        panel_h: f32,
        can_send: bool,
    ) {
        let target = self.atchat_target();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("TRANSFERS").strong().size(10.5).color(theme::CYAN()));
            ui.label(
                RichText::new(if target.is_empty() {
                    "→ ALL".to_string()
                } else {
                    format!("→ {target}")
                })
                .size(9.5)
                .color(theme::gray(130)),
            )
            .on_hover_text("A sent file follows the active chat tab");
            crate::chrome::row_tail(ui, |ui| {
                let clicked = tx_gated(ui, can_send, |ui| {
                    crate::chrome::chip(ui, false, RichText::new(" SEND FILE ").size(10.0))
                })
                .on_hover_text("Send a file or image over the air, block-CRC-ARQ")
                .clicked();
                // Native file picker only — the browser client has no
                // filesystem dialog here, same as the log-import button.
                #[cfg(not(target_arch = "wasm32"))]
                if clicked && let Some(path) = rfd::FileDialog::new().pick_file() {
                    cmds.push(Command::AtChatSendFile {
                        to: target.clone(),
                        path: path.to_string_lossy().into_owned(),
                    });
                }
                #[cfg(target_arch = "wasm32")]
                let _ = (clicked, &cmds, &target);
            });
        });

        egui::ScrollArea::vertical()
            .id_salt("atchat-transfers")
            .max_height((panel_h * 0.32).max(48.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if st.transfers.is_empty() {
                    ui.label(RichText::new("nothing in flight").weak());
                }
                for t in &st.transfers {
                    let dir = if t.incoming { "◀" } else { "▶" };
                    let frac = if t.total == 0 { 0.0 } else { t.have as f32 / t.total as f32 };
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{dir} {}", t.filename))
                                .monospace()
                                .size(10.0)
                                .color(if t.complete { theme::GREEN() } else { theme::TEXT() }),
                        );
                        ui.label(
                            RichText::new(format!("{} {}/{}", t.peer, t.have, t.total))
                                .size(9.5)
                                .color(theme::gray(130)),
                        );
                    });
                    ui.add(egui::ProgressBar::new(frac).desired_height(4.0));
                }
            });

        ui.add_space(6.0);
        ui.label(RichText::new("IMAGES").strong().size(10.5).color(theme::CYAN()));

        // Only the images, oldest first — the viewer walks this subset.
        let images: Vec<&sdroxide_types::AtChatFile> =
            st.files.iter().filter(|f| f.is_image).collect();
        if images.is_empty() {
            ui.label(RichText::new("No images received.").weak());
            return;
        }
        if self.atchat_img_at >= images.len() {
            self.atchat_img_at = images.len() - 1;
        }

        ui.horizontal(|ui| {
            if ui.add_enabled(self.atchat_img_at > 0, egui::Button::new("◀")).clicked() {
                self.atchat_img_at -= 1;
            }
            ui.label(
                RichText::new(format!("{}/{}", self.atchat_img_at + 1, images.len()))
                    .size(10.0)
                    .color(theme::gray(140)),
            );
            if ui
                .add_enabled(self.atchat_img_at + 1 < images.len(), egui::Button::new("▶"))
                .clicked()
            {
                self.atchat_img_at += 1;
            }
            let f = images[self.atchat_img_at];
            ui.label(
                RichText::new(format!("{}  ·  {}  ·  {}", f.filename, f.from, hms(f.when)))
                    .size(10.0)
                    .color(theme::TEXT_STRONG()),
            );
        });

        let f = images[self.atchat_img_at];
        let tex = self.atchat_image_texture(ui.ctx(), &f.path);
        match tex {
            Some(tex) => {
                let avail = ui.available_size();
                let [w, h] = tex.size();
                let scale = (avail.x / w as f32).min((panel_h * 0.42) / h as f32).min(1.0);
                ui.add(
                    egui::Image::new(&tex)
                        .fit_to_exact_size(egui::vec2(w as f32 * scale, h as f32 * scale)),
                );
            }
            None => {
                ui.label(
                    RichText::new(format!("cannot display {}", f.path))
                        .size(10.0)
                        .color(theme::YELLOW()),
                );
            }
        }
    }

    /// Decode a received image from disk once and keep it as a texture.
    fn atchat_image_texture(
        &mut self,
        ctx: &egui::Context,
        path: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(slot) = self.atchat_img_cache.get(path) {
            return slot.clone();
        }
        let handle = std::fs::read(path)
            .ok()
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(|img| {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let ci = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    rgba.as_raw(),
                );
                ctx.load_texture("atchat-image", ci, egui::TextureOptions::LINEAR)
            });
        self.atchat_img_cache.insert(path.to_string(), handle.clone());
        handle
    }
}

/// Unix seconds → `HH:MM:SS` UTC.
fn hms(unix: u64) -> String {
    let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(unix as i64);
    format!("{h:02}:{mi:02}:{s:02}")
}
