//! The CW panel: what the decoder is copying, over what the operator is
//! sending.
//!
//! Laid out like the keyboard modes' panel — a receive pane over a transmit
//! box — because it is worked the same way. What is different is the header: a
//! CW decoder has to say how confident it is and at what speed, because unlike
//! PSK or RTTY there is no framing to fail, and a decoder that is reading the
//! wrong speed produces confident nonsense rather than nothing.
//!
//! The pitch shown here is the waterfall cursor, and it is one number for two
//! jobs: the tone being copied and the tone being transmitted. In CW they are
//! the same frequency — a station is answered where it was heard — so there is
//! nothing to keep in step.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Command, CwEngine};

use crate::app::{SdroxideApp, tx_gated};
use crate::theme::ThemedScroll;

/// Speeds the WPM chip offers. The range an operator actually sets a keyer to.
const WPM_STEPS: &[f32] = &[10.0, 13.0, 15.0, 18.0, 20.0, 22.0, 25.0, 28.0, 30.0, 35.0, 40.0];

impl SdroxideApp {
    pub(in crate::app) fn cw_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let content_bottom = ui.cursor().top() + panel_h - 40.0;
        let status = self.digi_status.clone();
        let cw = status.as_ref().and_then(|s| s.cw).unwrap_or_default();
        let pitch = status.as_ref().map(|s| s.audio_hz).unwrap_or(700.0);
        let sent = status.as_ref().map(|s| s.tx_sent).unwrap_or(0);
        let tx_on = status.as_ref().map(|s| s.tx_next).unwrap_or(false);
        let transmitting = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        let rx_text = status.as_ref().map(|s| s.text_rx.clone()).unwrap_or_default();
        let my_call = status.as_ref().map(|s| s.config.my_call.clone()).unwrap_or_default();
        let on_air = self.on_air_freq_hz();

        // Header: where we are listening, what is being heard there, and how
        // fast we send.
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("CW").size(11.0).strong().color(crate::theme::CYAN()));
            ui.label(
                RichText::new(format!("{pitch:.0} Hz")).size(11.0).color(crate::theme::gray(150)),
            )
            .on_hover_text(
                "The tone being copied, and the tone transmitted — in CW they are the \
                 same frequency. Click the waterfall to move it onto a signal.",
            );
            if crate::chrome::chip(ui, false, "−").on_hover_text("Down 10 Hz").clicked() {
                cmds.push(Command::SetDigiAudioFreq((pitch - 10.0).clamp(200.0, 3000.0)));
            }
            if crate::chrome::chip(ui, false, "+").on_hover_text("Up 10 Hz").clicked() {
                cmds.push(Command::SetDigiAudioFreq((pitch + 10.0).clamp(200.0, 3000.0)));
            }
            crate::app::panels::on_air_readout(ui, on_air);

            // Whether the *main* readout says that number too, instead of the
            // dial a sidetone below it. Kept here rather than in the settings
            // window because it belongs with the pitch it is derived from: the
            // two are read together and adjusted together.
            // QRG: the Q-code for "your frequency is", which is exactly the
            // number this puts in the readout — and exactly the question a CW
            // dial cannot answer on its own. Named for the thing rather than
            // for the switch: an operator reads the label to find out what the
            // number will mean, not to learn that something has been turned on.
            let qrg = self.ui_settings.cw_qrg;
            if crate::chrome::chip(ui, qrg, "QRG")
                .on_hover_text(if qrg {
                    "QRG: the main readout and the tuning line are on the frequency being \
                     worked. Click for the dial instead, a sidetone pitch below it — what \
                     most radios show."
                } else {
                    "Put the main readout and the tuning line on the signal rather than on \
                     the dial, so the frequency shown is the one both operators would quote \
                     and the tuning line sits in the middle of the passband. Tuning is \
                     unchanged; only the numbers move.\n\nClicking a signal lands it on \
                     the cursor only as closely as the click step allows — Controls → \
                     click-to-tune rounding, 10 Hz by default. A coarse step leaves the \
                     signal off the pitch by up to half of it, and the readout will say so."
                })
                .clicked()
            {
                self.ui_settings.cw_qrg = !qrg;
                crate::app::persist::persist_ui_settings(&self.ui_settings);
            }
            ui.add_space(8.0);

            // Copy state. A CW decoder that is not locked is not "quiet", it is
            // guessing, and the difference has to be visible.
            let (lamp, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter_at(lamp).circle_filled(
                lamp.center(),
                4.5,
                if cw.locked { crate::theme::GREEN() } else { crate::theme::gray(48) },
            );
            if cw.locked {
                ui.label(
                    RichText::new(format!("{:.0} WPM", cw.wpm))
                        .size(11.0)
                        .strong()
                        .color(crate::theme::GREEN()),
                )
                .on_hover_text("Sending speed read off the signal");
                ui.label(
                    RichText::new(format!("{:+.0} dB", cw.snr_db))
                        .size(10.5)
                        .color(crate::theme::gray(140)),
                )
                .on_hover_text("Signal to noise in 500 Hz — the same figure a report quotes");
                // Only worth showing once it is a real mistune rather than a
                // fraction of a hertz of tracking.
                let off = cw.tone_hz - pitch;
                if off.abs() >= 3.0 {
                    ui.label(
                        RichText::new(format!("{off:+.0} Hz"))
                            .size(10.5)
                            .color(crate::theme::YELLOW()),
                    )
                    .on_hover_text(
                        "How far off the cursor the signal actually is. The decoder \
                         follows it; the passband does not, so nudge the cursor if it grows.",
                    );
                }
            } else {
                ui.label(RichText::new("— listening —").size(10.5).color(crate::theme::gray(100)));
            }

            crate::chrome::row_tail(ui, |ui| {
                if transmitting {
                    ui.label(
                        RichText::new("● TX").size(11.0).strong().color(crate::theme::ALERT()),
                    );
                    ui.add_space(6.0);
                }
                self.cw_speed_controls(ui, cmds);
                self.clear_rx_chip(ui, cmds);
            });
        });
        ui.add_space(4.0);

        // Receive pane over the transmit box, sized against the real panel
        // bottom so the controls are never pushed off a short panel.
        let btn_h = 32.0;
        let input_h = 56.0;
        let gap = 5.0;
        let bottom_pad = 12.0;
        // The message-button row underneath, which is there whether or not any
        // buttons have been made — the MSG chip that makes them lives on it.
        // Counted here or the row would be laid out past the bottom of the
        // panel, where it does not clip: it paints over whatever is below.
        let macro_h = 4.0 + crate::chrome::chip_height(ui, None);
        let rx_h = if self.ui_settings.swl {
            (content_bottom - ui.cursor().top() - bottom_pad).max(24.0)
        } else {
            (content_bottom
                - ui.cursor().top()
                - btn_h
                - macro_h
                - input_h
                - 2.0 * gap
                - bottom_pad)
                .max(24.0)
        };

        ui.allocate_ui(egui::vec2(ui.available_width(), rx_h), |ui| {
            egui::Frame::new()
                .fill(crate::theme::ROW_BG())
                .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP()))
                .inner_margin(egui::Margin { left: 8, right: 7, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    egui::ScrollArea::vertical()
                        .max_height((rx_h - 12.0).max(20.0))
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show_themed(ui, |ui| {
                            if rx_text.is_empty() {
                                ui.label(
                                    RichText::new("— nothing copied yet —")
                                        .monospace()
                                        .size(12.0)
                                        .color(crate::theme::gray(90)),
                                );
                            } else {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&rx_text)
                                            .monospace()
                                            .size(12.5)
                                            .color(crate::theme::GREEN()),
                                    )
                                    .wrap(),
                                );
                            }
                        });
                });
        });
        ui.add_space(gap);

        if self.ui_settings.swl {
            return;
        }

        // Transmit box. Characters already keyed are green, and they are keyed
        // as they are typed rather than a line at a time — which is how a CW
        // operator sends.
        let prev = self.text_tx.clone();
        let sent = sent.min(prev.chars().count());
        let prefix: String = prev.chars().take(sent).collect();
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
            let text = buf.as_str();
            let sent_byte = text.char_indices().nth(sent).map(|(i, _)| i).unwrap_or(text.len());
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = wrap;
            let mono = egui::FontId::monospace(13.0);
            if sent_byte > 0 {
                job.append(
                    &text[..sent_byte],
                    0.0,
                    egui::TextFormat {
                        font_id: mono.clone(),
                        color: crate::theme::GREEN(),
                        ..Default::default()
                    },
                );
            }
            if sent_byte < text.len() {
                job.append(
                    &text[sent_byte..],
                    0.0,
                    egui::TextFormat {
                        font_id: mono.clone(),
                        color: crate::theme::TEXT_STRONG(),
                        ..Default::default()
                    },
                );
            }
            ui.fonts_mut(|f| f.layout_job(job))
        };

        // Send on return: the key is taken before the box is built, or the
        // edit turns it into a newline first. The line break is still wanted on
        // screen — it goes in below, once the line is known to be committed —
        // and the keyer reads it as a word space.
        let send_on_enter = self.digi_cfg_edit.send_on_enter;
        let tx_id = ui.id().with("cw-tx-edit");
        // On a receiver the box goes grey with the buttons, and here that is
        // more than tidiness: typing into it *is* the instruction to send, so a
        // live box on a radio with no transmitter would key nothing on every
        // keystroke.
        let tx_ok = self.tx_capable();
        let entered = tx_ok && send_on_enter && crate::chrome::take_return(ui, tx_id);

        let resp = ui
            .add_enabled_ui(tx_ok, |ui| {
                ui.allocate_ui(egui::vec2(ui.available_width(), input_h), |ui| {
                    egui::Frame::new()
                        .fill(crate::theme::ROW_BG())
                        .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP()))
                        .inner_margin(egui::Margin::symmetric(6, 4))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.set_min_height(ui.available_height());
                            egui::ScrollArea::vertical()
                                .id_salt("cw-tx")
                                .max_height((input_h - 8.0).max(20.0))
                                .auto_shrink([false, false])
                                .stick_to_bottom(true)
                                .show_themed(ui, |ui| {
                                    crate::chrome::field(
                                        ui,
                                        egui::TextEdit::multiline(&mut self.text_tx)
                                            .id(tx_id)
                                            .layouter(&mut layouter)
                                            .frame(egui::Frame::NONE)
                                            .desired_width(f32::INFINITY)
                                            .hint_text(if send_on_enter {
                                                "Type a line, Return sends it…"
                                            } else {
                                                "Type here to send…"
                                            }),
                                    )
                                })
                                .inner
                        })
                        .inner
                })
                .inner
            })
            .inner;
        if resp.changed() {
            // What is already on the air cannot be unsent.
            if !self.text_tx.starts_with(&prefix) {
                self.text_tx = prev;
            }
            // Typing is itself the instruction to send: waiting for a separate
            // button press would put the first characters on the air late, and
            // a CW operator who has started a callsign has committed to it.
            //
            // Unless the operator asked for the other bargain, in which case
            // nothing leaves the box until it is committed — see `entered`.
            if !send_on_enter {
                cmds.push(Command::DigiTxText(self.text_tx.clone()));
                if !tx_on && !self.text_tx.is_empty() {
                    cmds.push(Command::DigiTxActive(true));
                }
            }
        }
        // Return commits the whole buffer at once. That is the point of the
        // mode on a rig that keys itself from text: each hand-off to its keyer
        // is a transmit-receive cycle, and a line given over in one piece costs
        // one switch where typing it live costs one per word.
        if entered {
            self.commit_tx_line(cmds);
        }
        ui.add_space(gap);

        ui.horizontal(|ui| {
            let label = if tx_on { "  TX ON  " } else { "   TX   " };
            if tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    tx_on,
                    RichText::new(label).size(14.0).strong(),
                    crate::theme::ALERT(),
                    Color32::WHITE,
                )
                .on_hover_text(if send_on_enter {
                    "Send what is in the box now, without waiting for Return"
                } else {
                    "Hold the key down between characters, so nothing typed waits"
                })
            })
            .clicked()
            {
                // In send-on-Return the box is held back until it is committed,
                // and pressing transmit is a commit — switching TX on over a
                // queue nothing was ever put into would send silence.
                if !tx_on && send_on_enter {
                    cmds.push(Command::DigiTxText(self.text_tx.clone()));
                }
                cmds.push(Command::DigiTxActive(!tx_on));
            }
            if tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(" CALL CQ ").size(13.0).strong(),
                    crate::theme::GREEN(),
                    crate::theme::INK_ON_CYAN(),
                )
            })
            .clicked()
            {
                let call = if my_call.is_empty() { "NOCALL".to_string() } else { my_call.clone() };
                let cq = format!("CQ CQ CQ DE {call} {call} {call} K ");
                cmds.push(Command::DigiAbortTx);
                self.text_tx = cq.clone();
                cmds.push(Command::DigiTxText(cq));
                cmds.push(Command::DigiTxActive(true));
            }
            if crate::chrome::chip(ui, false, " CLEAR ")
                .on_hover_text("Stop sending and drop whatever has not gone out")
                .clicked()
            {
                self.text_tx.clear();
                cmds.push(Command::DigiAbortTx);
                cmds.push(Command::DigiTxText(String::new()));
            }

            // Which bargain the operator wants: a character on the air as it is
            // typed, or a line held back until it is whole. It sits with the
            // sending controls rather than the decoder chips in the header,
            // because what it changes is what the TX button and the box do.
            crate::chrome::row_tail(ui, |ui| {
                self.send_on_return_chip(
                    ui,
                    cmds,
                    "Hold what is typed until Return, then send the line in one piece \
                     instead of keying each character as it is typed. Worth having on a \
                     transceiver that keys itself from text, where every hand-off to its \
                     keyer is another transmit-receive cycle.",
                );
            });
        });
        self.cw_macro_row(ui, cmds, tx_ok, &my_call);
        ui.add_space(bottom_pad);
    }

    /// The operator's own message buttons, and the chip that edits them.
    ///
    /// Each one sends its whole text in a single message rather than keying it
    /// as if it had been typed, which is the point of them on a radio that keys
    /// itself from text: one hand-off to the rig's keyer instead of one per
    /// word, exactly as SEND ON RETURN does for a typed line (issue #374).
    ///
    /// **F1–F9 press them** while the CW panel is up and nothing on screen holds
    /// the keyboard. That exclusion matters: an operator part-way through a
    /// callsign has the caret in the transmit box, and a function key that fired
    /// a message from under them would put the wrong thing on the air. The test
    /// is deliberately the blunt one — *any* focused widget, not just that box —
    /// because a message going out unbidden is the expensive mistake and a
    /// function key that does nothing is the cheap one.
    fn cw_macro_row(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        tx_ok: bool,
        my_call: &str,
    ) {
        let macros = self.digi_cfg_edit.cw_macros.clone();
        // Something on screen has the keyboard — the transmit box, a settings
        // field, a callsign being typed into the logbook. The function keys are
        // the operator's then, not ours.
        let typing = ui.memory(|m| m.focused().is_some());
        let mut fire: Option<usize> = None;
        if !typing && tx_ok {
            const KEYS: [egui::Key; 9] = [
                egui::Key::F1,
                egui::Key::F2,
                egui::Key::F3,
                egui::Key::F4,
                egui::Key::F5,
                egui::Key::F6,
                egui::Key::F7,
                egui::Key::F8,
                egui::Key::F9,
            ];
            for (i, key) in KEYS.iter().enumerate().take(macros.len()) {
                if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, *key)) {
                    fire = Some(i);
                }
            }
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            // A row with nothing in it yet is one the operator has added and
            // not filled in: it keeps its place and its function key in the
            // editor, but there is nothing for a chip on the panel to send.
            for (i, m) in macros.iter().enumerate().filter(|(_, m)| !m.text.trim().is_empty()) {
                let hint = if i < 9 {
                    format!("F{}: sends “{}”", i + 1, m.text.trim())
                } else {
                    format!("Sends “{}”", m.text.trim())
                };
                if tx_gated(ui, tx_ok, |ui| {
                    crate::chrome::chip(ui, false, m.chip_label()).on_hover_text(&hint)
                })
                .clicked()
                {
                    fire = Some(i);
                }
            }
            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(ui, self.cw_macro_edit, "MSG")
                    .on_hover_text(
                        "Your own message buttons — a contest exchange, a name-and-QTH reply, \
                         TNX 73 GL. Each sends its whole text in one go, and F1–F9 press the \
                         first nine. They travel with the station's configuration, so a \
                         remote client has them too.",
                    )
                    .clicked()
                {
                    self.cw_macro_edit = !self.cw_macro_edit;
                }
            });
        });
        if let Some(m) = fire.and_then(|i| macros.get(i)) {
            let call = if my_call.is_empty() { "NOCALL" } else { my_call };
            let grid = self.digi_cfg_edit.my_grid.clone();
            let text = m.expand(call, &grid);
            if !text.trim().is_empty() {
                // The same three steps CALL CQ takes, and in the same order:
                // whatever was going out is abandoned, the box shows what is
                // being sent, and the message goes as one piece.
                cmds.push(Command::DigiAbortTx);
                self.text_tx = text.clone();
                cmds.push(Command::DigiTxText(text));
                cmds.push(Command::DigiTxActive(true));
            }
        }
    }

    /// The editor: one row per button, plus somewhere to add another.
    ///
    /// A window rather than a fold-out inside the panel. The panel's receive
    /// pane is sized against the real panel bottom, so anything that can grow
    /// under it has to be budgeted for — and a table that grows by a row every
    /// time ADD is pressed cannot be. A window also survives the panel being
    /// short, which is the case an operator setting these up on a laptop is in.
    pub(in crate::app) fn cw_macro_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        use sdroxide_types::CwMacro;

        if !self.cw_macro_edit {
            return;
        }
        let mut open = true;
        let mut changed = false;
        let mut remove = None;
        let resp = egui::Window::new("CW MESSAGES")
            .id(crate::layout::salted_id(ctx, "CwMacros"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 560.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                // Claimed before the grid, because a `TextEdit` is never wider
                // than the space it is given however wide it asks to be — and
                // an auto-sized window takes its width from its widest child,
                // which without this is the paragraph below.
                ui.set_min_width(crate::layout::window_w(ctx, 520.0));
                ui.label(
                    RichText::new(
                        "Each button sends its whole text in one go, at the panel's WPM. \
                         F1–F9 press the first nine, so long as nothing on screen has the \
                         keyboard.",
                    )
                    .size(10.5)
                    .color(crate::theme::gray(150)),
                );
                ui.add_space(6.0);
                egui::Grid::new("cw-macros").num_columns(4).spacing([6.0, 4.0]).show(ui, |ui| {
                    ui.label(RichText::new("key").size(10.0).color(crate::theme::gray(140)));
                    ui.label(RichText::new("button").size(10.0).color(crate::theme::gray(140)));
                    ui.label(RichText::new("sends").size(10.0).color(crate::theme::gray(140)));
                    ui.label("");
                    ui.end_row();
                    for (i, m) in self.digi_cfg_edit.cw_macros.iter_mut().enumerate() {
                        ui.label(
                            RichText::new(if i < 9 {
                                format!("F{}", i + 1)
                            } else {
                                String::new()
                            })
                            .size(10.5)
                            .color(crate::theme::gray(150)),
                        );
                        // Sized rather than asked for: inside a `Grid` a
                        // `TextEdit`'s `desired_width` is clamped to a column
                        // that has not been measured yet, and both boxes come
                        // out a few characters wide.
                        changed |= crate::chrome::field_sized(
                            ui,
                            [80.0, 22.0],
                            egui::TextEdit::singleline(&mut m.label).hint_text("label"),
                        )
                        .changed();
                        changed |= crate::chrome::field_sized(
                            ui,
                            [320.0, 22.0],
                            egui::TextEdit::singleline(&mut m.text).hint_text("5NN 5NN {MYCALL}"),
                        )
                        .changed();
                        if crate::chrome::chip(ui, false, "×")
                            .on_hover_text("Remove this button")
                            .clicked()
                        {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let full = self.digi_cfg_edit.cw_macros.len() >= CwMacro::MAX;
                    if ui
                        .add_enabled(!full, egui::Button::new("ADD"))
                        .on_disabled_hover_text(format!("{} is the most", CwMacro::MAX))
                        .clicked()
                    {
                        self.digi_cfg_edit.cw_macros.push(CwMacro::default());
                        changed = true;
                    }
                    ui.label(
                        RichText::new(
                            "{MYCALL} and {MYGRID} are filled in as the message goes out.",
                        )
                        .size(10.5)
                        .color(crate::theme::gray(140)),
                    );
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        if let Some(i) = remove {
            self.digi_cfg_edit.cw_macros.remove(i);
            changed = true;
        }
        if changed && self.digi_cfg_seeded {
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
        self.cw_macro_edit = open;
    }

    /// Transmit speed, Farnsworth spacing, and whether the decoder is allowed to
    /// find the receive speed for itself.
    fn cw_speed_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let cfg = &mut self.digi_cfg_edit;
        let mut changed = false;

        // Lock the decoder to the transmit speed. Worth having on a signal too
        // weak for the search to settle when you already know how fast the
        // other station is sending — in a contest, everyone at once.
        let locked = cfg.cw_speed_lock;
        if crate::chrome::chip(ui, locked, RichText::new("LOCK").size(10.5))
            .on_hover_text(
                "Decode at the speed set here instead of reading it off the signal. \
                 Helps a signal too weak for the speed search to settle.",
            )
            .clicked()
        {
            cfg.cw_speed_lock = !locked;
            changed = true;
        }

        // Which decoder copies the receive window. Two values, so a chip that
        // cycles rather than a picker — and the label says which one is
        // running, not which one it would switch to.
        let engine = cfg.cw_engine;
        if crate::chrome::chip(
            ui,
            engine == CwEngine::Timing,
            RichText::new(engine.label()).size(10.5),
        )
        .on_hover_text(format!(
            "{}\n\nClick for the {} decoder. The neural one copies further down and \
                 reads hand-sent CW a timing fit will not accept; the timing one is the \
                 only one that copies the accented letters — Ä, Ö, Å, Ü, É — because the \
                 model has no output class for them.",
            engine.hint(),
            match engine {
                CwEngine::Neural => "timing",
                CwEngine::Timing => "neural",
            }
        ))
        .clicked()
        {
            cfg.cw_engine = match engine {
                CwEngine::Neural => CwEngine::Timing,
                CwEngine::Timing => CwEngine::Neural,
            };
            changed = true;
        }

        if !self.ui_settings.swl {
            // Farnsworth: elements at the sending speed, spacing stretched to this.
            let fw = cfg.cw_farnsworth_wpm;
            let fw_on = fw > 0.0 && fw < cfg.cw_wpm;
            let face = if fw_on { format!("FW {fw:.0}") } else { "FW".to_string() };
            let btn = crate::chrome::chip(ui, fw_on, RichText::new(face).size(10.5)).on_hover_text(
                "Farnsworth: send the characters at full speed and stretch only the gaps \
             between them, so they are heard at the right rhythm but arrive slowly enough \
             to write down.",
            );
            let mut pick_fw = None;
            let resp = egui::Popup::from_toggle_button_response(&btn)
                .frame(crate::chrome::window_frame())
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .show(|ui| {
                    crate::chrome::window_body_bg(ui);
                    ui.set_max_width(180.0);
                    if ui.selectable_label(!fw_on, "Off — normal spacing").clicked() {
                        pick_fw = Some(0.0);
                    }
                    for w in [5.0f32, 8.0, 10.0, 13.0, 15.0, 18.0] {
                        if w >= cfg.cw_wpm {
                            continue; // stretching to faster than the elements is not a thing
                        }
                        if ui
                            .selectable_label((fw - w).abs() < 0.5, format!("{w:.0} WPM"))
                            .clicked()
                        {
                            pick_fw = Some(w);
                        }
                    }
                });
            if let Some(r) = &resp {
                crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, 1.0);
            }
            if let Some(w) = pick_fw {
                cfg.cw_farnsworth_wpm = w;
                changed = true;
            }

            // Transmit speed.
            let wpm = cfg.cw_wpm;
            let btn =
                crate::chrome::chip(ui, false, RichText::new(format!("{wpm:.0} WPM")).size(11.0))
                    .on_hover_text("Keying speed");
            let mut pick = None;
            let resp = egui::Popup::from_toggle_button_response(&btn)
                .frame(crate::chrome::window_frame())
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .show(|ui| {
                    crate::chrome::window_body_bg(ui);
                    ui.set_max_width(140.0);
                    for w in WPM_STEPS {
                        if ui
                            .selectable_label((wpm - w).abs() < 0.5, format!("{w:.0} WPM"))
                            .clicked()
                        {
                            pick = Some(*w);
                        }
                    }
                });
            if let Some(r) = &resp {
                crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, 1.0);
            }
            if let Some(w) = pick {
                cfg.cw_wpm = w;
                // Farnsworth spacing slower than the elements is the only kind
                // there is; a speed drop that inverted them would send gibberish
                // timing.
                if cfg.cw_farnsworth_wpm >= w {
                    cfg.cw_farnsworth_wpm = 0.0;
                }
                changed = true;
            }
        }

        if changed && self.digi_cfg_seeded {
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
    }
}
