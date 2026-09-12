//! The FSQ panel: directed messages, the contact list, and image transfer.
//!
//! FSQ addresses stations by callsign rather than by frequency, so the panel
//! is a message view with a target field; an empty target is ALLCALL. The
//! contact list is the operator's own address book, persisted beside the rest
//! of the configuration.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Decode};

use crate::theme::ThemedScroll;

use crate::app::panels::widgets::pick_image;
use crate::app::{SdroxideApp, tx_gated};
use crate::time::now_unix;

/// How long an FSQ station stays lit on the maps after it was last heard.
///
/// Longer than any of the slot modes, and deliberately: FSQ is a leisurely
/// keyboard mode where a quarter of an hour between overs is an ordinary
/// conversation rather than a silence. A map that emptied at FT8's two minutes
/// would be blank for most of every QSO.
const FSQ_STATION_FADE_S: f64 = 1800.0;

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn fsq_load_contacts() -> Vec<sdroxide_types::FsqContact> {
    sdroxide_config::load_contacts()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn fsq_load_contacts() -> Vec<sdroxide_types::FsqContact> {
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
fn fsq_save_contacts(contacts: &[sdroxide_types::FsqContact]) {
    let _ = sdroxide_config::save_contacts(contacts);
}

#[cfg(target_arch = "wasm32")]
fn fsq_save_contacts(_contacts: &[sdroxide_types::FsqContact]) {}

impl SdroxideApp {
    /// FSQ mode settings (speed + directed-message callsign), shown inline in the
    /// FSQ panel header next to the tune buttons.
    fn fsq_params_row(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let cfg = &mut self.digi_cfg_edit;
        let mut changed = false;
        ui.add_space(6.0);
        // Wrapped, not a fixed row: it is emitted into the header's own wrapped
        // row, and a nested row that will not wrap runs its callsign field off
        // the edge of a phone rather than moving it to the next line.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.label(RichText::new("Speed").size(10.5).strong().color(crate::theme::CYAN_DIM()));
            for b in [2.0f32, 3.0, 4.5, 6.0] {
                let sel = (cfg.fsq_baud - b).abs() < 0.05;
                let lbl =
                    if (b - 4.5).abs() < 0.05 { "4.5".to_string() } else { format!("{b:.0}") };
                if ui.selectable_label(sel, lbl).clicked() {
                    cfg.fsq_baud = b;
                    changed = true;
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Call").size(10.5).strong().color(crate::theme::CYAN_DIM()));
            if crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.fsq_call).desired_width(76.0),
            )
            .on_hover_text("Callsign for directed (FSQCALL) messages; defaults to your callsign")
            .changed()
            {
                cfg.fsq_call = cfg.fsq_call.to_uppercase();
                changed = true;
            }
        });
        if changed {
            cmds.push(Command::SetDigiConfig(cfg.clone()));
        }
    }

    /// Feed the heard list to the maps, and ask the lookup service where the
    /// stations on it actually are.
    ///
    /// FSQ carries no locator on the air — the protocol has no field for one and
    /// no convention of putting one in the text — so unlike JS8 there is no
    /// on-air answer to fall back on: it is the callsign lookup or nothing. The
    /// stations that do resolve are handed over as [`Decode`]s, the shape both
    /// maps already speak, rather than teaching them a second kind of station.
    fn fsq_observe(&mut self, heard: &[sdroxide_types::FsqHeard], now_t: f64) {
        self.digi_stations.set_fade_s(FSQ_STATION_FADE_S);
        // FSQ's heard list has no expiry of its own — a station stays on it
        // until thirty others have pushed it off — so the window is applied
        // here. Without it, opening the panel would light up everyone heard
        // since the receiver was switched on, which is the one thing a map of
        // who is *on the band* must not say.
        let now = now_unix();
        let located: Vec<Decode> = heard
            .iter()
            .filter(|h| (now - h.last_utc) < FSQ_STATION_FADE_S as i64)
            .filter_map(|h| {
                let grid = self.fsq_grid_for(&h.call)?;
                Some(Decode {
                    // FSQ has no slots; `last_utc` is when the station was
                    // actually heard, which is what the fade counts from.
                    slot_utc: h.last_utc,
                    snr_db: 0,
                    dt: 0.0,
                    audio_hz: 0.0,
                    message: String::new(),
                    to: None,
                    from: Some(h.call.clone()),
                    grid: Some(grid),
                    is_cq: false,
                    cq_to: None,
                    rr73_to: None,
                    free_text: false,
                })
            })
            .collect();
        self.digi_stations.observe(&located, now_t, now);

        if !self.grid_lookup_due(now_t) {
            return;
        }
        let next = heard
            .iter()
            .map(|h| h.call.to_ascii_uppercase())
            .find(|c| !c.is_empty() && !self.grid_looked_up.contains(c));
        self.queue_grid_lookup(next, now_t);
    }

    /// Where an FSQ station is, if the callsign lookup has been told.
    fn fsq_grid_for(&self, call: &str) -> Option<String> {
        if call.is_empty() {
            return None;
        }
        self.callsign_cache
            .get(&call.to_ascii_uppercase())
            .and_then(|i| i.grid.clone())
            .filter(|g| sdroxide_types::grid_to_latlon(g).is_some())
    }

    pub(in crate::app) fn fsq_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let content_bottom = ui.cursor().top() + panel_h - 26.0;
        let status = self.digi_status.clone();
        let audio_hz = status.as_ref().map(|s| s.audio_hz).unwrap_or(1500.0);
        let transmitting = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        let text_rx = status.as_ref().map(|s| s.text_rx.clone()).unwrap_or_default();
        let heard = status.as_ref().map(|s| s.fsq_heard.clone()).unwrap_or_default();
        let messages = status.as_ref().map(|s| s.fsq_messages.clone()).unwrap_or_default();
        // The globe and the flat map draw whatever was last observed, so this
        // runs whether or not this panel is showing a map of its own.
        self.fsq_observe(&heard, ui.input(|i| i.time));
        let my_call = {
            let c = &self.digi_cfg_edit;
            if !c.fsq_call.is_empty() { c.fsq_call.clone() } else { c.my_call.clone() }
        };

        let tx_ok = self.tx_capable();

        // A picked image → transmit (the engine grayscales/scales it). The
        // picker is greyed on a receiver, but drain the inbox regardless: a
        // file chosen just before the interface was swapped would otherwise sit
        // there and go out the moment a transmitter appeared.
        if let Some(bytes) = self.fsq_img_inbox.lock().ok().and_then(|mut g| g.take())
            && tx_ok
        {
            cmds.push(Command::DigiAbortTx);
            cmds.push(Command::DigiImageTx { png: bytes });
        }

        // Header row. Label matches the RTTY/PSK panels (11 pt cyan).
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("FSQ").size(11.0).strong().color(crate::theme::CYAN()));
            ui.label(RichText::new(format!("{audio_hz:.0} Hz")).monospace());
            if crate::chrome::chip(ui, false, "−").clicked() {
                cmds.push(Command::SetDigiAudioFreq((audio_hz - 10.0).clamp(200.0, 3500.0)));
            }
            if crate::chrome::chip(ui, false, "+").clicked() {
                cmds.push(Command::SetDigiAudioFreq((audio_hz + 10.0).clamp(200.0, 3500.0)));
            }
            self.digi_freq_chip(ui, cmds);
            // Mode settings inline next to the tune buttons (moved here from the
            // setup dialog): the FSQ speed and the directed-message callsign.
            self.fsq_params_row(ui, cmds);
            crate::chrome::row_tail(ui, |ui| {
                if transmitting {
                    ui.label(RichText::new("● TX").color(crate::theme::ALERT()).strong());
                }
                if crate::chrome::chip(ui, self.fsq_show_contacts, "CONTACTS").clicked() {
                    self.fsq_show_contacts = !self.fsq_show_contacts;
                }
                self.digi_squelch_slider(ui, cmds);
                self.clear_rx_chip(ui, cmds);
            });
        });
        ui.add_space(4.0);
        // Everything below fits inside the (bounded) panel height. The left
        // column scrolls as a unit; the right column pins its compose controls to
        // the bottom so they're never clipped on a short panel.
        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        // A phone takes them one at a time: the heard column alone is 150 fixed
        // points, and what is left of a phone's width is not a message field.
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        ui.horizontal_top(|ui| {
            // ── Left: heard list + image, each in its own bounded scroll so the
            // heard list scrolls internally instead of pushing the ALLCALL/IMAGE
            // controls off-panel. Fixed room is reserved for the labels/buttons
            // between them; the split never overflows the column height. ──
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    ui.set_width(if pane.is_some() { ui.available_width() } else { 150.0 });
                    let fixed_h = 96.0; // HEARD/IMAGE labels + ALLCALL + Send + separator
                    let scrollable = (avail_h - fixed_h).max(44.0);
                    let heard_h = scrollable * 0.62;
                    let images_h = scrollable - heard_h;

                    ui.label(
                        RichText::new("HEARD").size(10.5).strong().color(crate::theme::CYAN_DIM()),
                    );
                    egui::ScrollArea::vertical()
                        .id_salt("fsq-heard")
                        .max_height(heard_h)
                        .auto_shrink([false, true])
                        .show_themed(ui, |ui| {
                            if heard.is_empty() {
                                ui.label(RichText::new("— none —").weak());
                            }
                            for h in &heard {
                                let sel = self.fsq_target.eq_ignore_ascii_case(&h.call);
                                if ui
                                    .selectable_label(sel, RichText::new(&h.call).monospace())
                                    .clicked()
                                {
                                    self.fsq_target = h.call.clone();
                                }
                            }
                        });
                    ui.add_space(4.0);
                    if crate::chrome::chip(ui, self.fsq_target.is_empty(), "ALLCALL").clicked() {
                        self.fsq_target.clear();
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("IMAGE")
                                .size(10.5)
                                .strong()
                                .color(crate::theme::CYAN_DIM()),
                        );
                        // Unlike SSTV and wefax, nothing here is on a disk to
                        // delete: an FSQ picture is a texture this client made
                        // and nothing else holds, so clearing it needs no
                        // confirmation and no round trip to the radio.
                        if !self.fsq_rx_images.is_empty()
                            && crate::chrome::chip(ui, false, RichText::new("CLEAR").size(9.5))
                                .on_hover_text("Forget the pictures received so far")
                                .clicked()
                        {
                            self.fsq_rx_images.clear();
                        }
                    });
                    if tx_gated(ui, tx_ok, |ui| crate::chrome::chip(ui, false, "Send image…"))
                        .clicked()
                    {
                        pick_image(self.fsq_img_inbox.clone());
                    }
                    let mut drop_at = None;
                    egui::ScrollArea::vertical()
                        .id_salt("fsq-images")
                        .max_height(images_h)
                        .auto_shrink([false, true])
                        .show_themed(ui, |ui| {
                            for (i, tex) in self.fsq_rx_images.iter().enumerate() {
                                ui.add(
                                    egui::Image::new(tex)
                                        .fit_to_exact_size(egui::vec2(140.0, 105.0))
                                        .corner_radius(2.0)
                                        .sense(egui::Sense::click()),
                                )
                                .on_hover_text("Right-click to remove")
                                .context_menu(|ui| {
                                    if ui.button("Remove this picture").clicked() {
                                        drop_at = Some(i);
                                        ui.close();
                                    }
                                });
                                ui.add_space(3.0);
                            }
                        });
                    if let Some(i) = drop_at {
                        // Dropping the handle is the whole of it: the texture
                        // goes when the last reference to it does.
                        drop(self.fsq_rx_images.remove(i));
                    }
                });
            }

            if pane.is_none() {
                ui.separator();
            }

            // ── Right: RX stream (fills) + compose controls (pinned bottom) ──
            if pane.is_none_or(|p| p != 0) {
                ui.vertical(|ui| {
                    // Two control rows (To:/message/SEND, then CQ/? /CLEAR) + gaps.
                    let controls_h = 74.0;
                    let rx_h = (avail_h - controls_h).max(24.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), rx_h), |ui| {
                        egui::Frame::new()
                            .fill(crate::theme::ROW_BG())
                            .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP()))
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_min_height(ui.available_height());
                                egui::ScrollArea::vertical()
                                    .id_salt("fsq-rx")
                                    .auto_shrink([false, false])
                                    .stick_to_bottom(true)
                                    .show_themed(ui, |ui| {
                                        if text_rx.is_empty() && messages.is_empty() {
                                            ui.label(RichText::new("— listening —").weak());
                                        }
                                        for m in
                                            messages.iter().filter(|m| m.to_me && !m.to.is_empty())
                                        {
                                            ui.label(
                                                RichText::new(format!(
                                                    "★ {} → {}: {}",
                                                    m.from, m.to, m.text
                                                ))
                                                .color(crate::theme::CYAN())
                                                .monospace(),
                                            );
                                        }
                                        ui.label(
                                            RichText::new(&text_rx)
                                                .monospace()
                                                .color(crate::theme::GREEN()),
                                        );
                                    });
                            });
                    });
                    ui.add_space(4.0);
                    if !self.ui_settings.swl {
                        let tgt = if self.fsq_target.is_empty() {
                            "ALLCALL".to_string()
                        } else {
                            self.fsq_target.clone()
                        };
                        // Row 1: To: target + message input + SEND.
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{tgt}:"))
                                    .monospace()
                                    .color(crate::theme::CYAN_DIM()),
                            );
                            let resp = crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut self.text_tx)
                                    .desired_width((ui.available_width() - 62.0).max(60.0))
                                    .hint_text("Message…"),
                            );
                            // Return is the same button, so it is shut off with it.
                            let entered = tx_ok
                                && resp.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let send = entered
                                || tx_gated(ui, tx_ok, |ui| {
                                    crate::chrome::chip_accent(
                                        ui,
                                        false,
                                        " SEND ",
                                        crate::theme::ALERT(),
                                        crate::theme::INK_ON_CYAN(),
                                    )
                                })
                                .clicked();
                            if send && !self.text_tx.trim().is_empty() {
                                let call = if my_call.is_empty() { "NOCALL" } else { &my_call };
                                let body = self.text_tx.trim();
                                let full = if self.fsq_target.is_empty() {
                                    format!("{call}: {body}\n")
                                } else {
                                    format!("{call}:{} {body}\n", self.fsq_target)
                                };
                                cmds.push(Command::DigiAbortTx);
                                cmds.push(Command::DigiTxText(full));
                                cmds.push(Command::DigiTxActive(true));
                                self.text_tx.clear();
                            }
                        });
                        // Row 2: CQ / ? heard / CLEAR.
                        ui.horizontal(|ui| {
                            if tx_gated(ui, tx_ok, |ui| crate::chrome::chip(ui, false, " CALL CQ "))
                                .clicked()
                            {
                                cmds.push(Command::DigiCallCq);
                            }
                            if !self.fsq_target.is_empty()
                                && tx_gated(ui, tx_ok, |ui| {
                                    crate::chrome::chip(ui, false, " ? heard ")
                                })
                                .clicked()
                            {
                                let call = if my_call.is_empty() { "NOCALL" } else { &my_call };
                                let full = format!("{call}:{}?\n", self.fsq_target);
                                cmds.push(Command::DigiAbortTx);
                                cmds.push(Command::DigiTxText(full));
                                cmds.push(Command::DigiTxActive(true));
                            }
                            if crate::chrome::chip(ui, false, " CLEAR ").clicked() {
                                self.text_tx.clear();
                                cmds.push(Command::DigiAbortTx);
                            }
                        });
                    }
                });
            }
        });

        self.fsq_contacts_window(ui.ctx());
    }

    /// Editable FSQ contacts book (add / select-as-target / delete).
    fn fsq_contacts_window(&mut self, ctx: &egui::Context) {
        let mut open = self.fsq_show_contacts;
        let mut changed = false;
        let mut set_target: Option<String> = None;
        egui::Window::new("FSQ Contacts")
            .id(crate::layout::salted_id(ctx, "FSQ Contacts"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 320.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.horizontal(|ui| {
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.fsq_new_contact)
                            .desired_width(140.0)
                            .hint_text("callsign"),
                    );
                    let can_add = !self.fsq_new_contact.trim().is_empty();
                    if ui.add_enabled(can_add, egui::Button::new("Add")).clicked() {
                        let id = self.fsq_contacts.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                        self.fsq_contacts.push(sdroxide_types::FsqContact {
                            id,
                            call: self.fsq_new_contact.trim().to_uppercase(),
                            name: String::new(),
                            note: String::new(),
                        });
                        self.fsq_new_contact.clear();
                        changed = true;
                    }
                });
                ui.separator();
                let mut to_delete: Option<u64> = None;
                egui::ScrollArea::vertical().max_height(260.0).show_themed(ui, |ui| {
                    for c in &mut self.fsq_contacts {
                        ui.horizontal(|ui| {
                            if ui.button("TO").clicked() {
                                set_target = Some(c.call.clone());
                            }
                            ui.label(RichText::new(&c.call).monospace().strong());
                            if crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut c.name)
                                    .hint_text("name")
                                    .desired_width(120.0),
                            )
                            .changed()
                            {
                                changed = true;
                            }
                            if crate::chrome::chip_accent(
                                ui,
                                false,
                                "DEL",
                                crate::theme::PINK(),
                                crate::theme::INK_ON_CYAN(),
                            )
                            .clicked()
                            {
                                to_delete = Some(c.id);
                            }
                        });
                    }
                });
                if let Some(id) = to_delete {
                    self.fsq_contacts.retain(|c| c.id != id);
                    changed = true;
                }
            });
        if let Some(t) = set_target {
            self.fsq_target = t;
            self.fsq_show_contacts = false;
        }
        if changed {
            fsq_save_contacts(&self.fsq_contacts);
        }
        self.fsq_show_contacts = open;
    }
}
