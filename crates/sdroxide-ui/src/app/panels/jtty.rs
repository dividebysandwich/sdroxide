//! The JTTY panel: the messages decoded, newest first.
//!
//! JTTY is a keyboard-to-keyboard text mode, so the panel is a log rather than
//! a table: one line per message with its UTC stamp, audio frequency and SNR.
//! A message is free text — a callsign CQ, a `599` exchange, a grid, a control
//! phrase — so there are no per-field columns to break it into; the line is the
//! text.
//!
//! Receive only: there is no transmit row here yet.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Command, JttyStatus};

use crate::app::{SdroxideApp, tx_gated};
use crate::theme;

impl SdroxideApp {
    pub(in crate::app) fn jtty_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        _panel_h: f32,
    ) {
        let st: Option<JttyStatus> = self.digi_status.as_ref().and_then(|s| s.jtty.clone());
        // The transmit row is drawn whatever the receiver has heard — on a
        // quiet band there are no messages, and returning early here left the
        // whole TX row missing, which is the only thing the operator wanted.
        if let Some(st) = &st {
            self.jtty_log(ui, st);
        } else {
            ui.label(RichText::new("starting the JTTY receiver…").weak());
        }
        self.jtty_tx_row(ui, cmds);
    }

    /// The rolling log of messages heard.
    fn jtty_log(&mut self, ui: &mut egui::Ui, st: &JttyStatus) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("JTTY").strong().color(theme::CYAN()));
            // The audio level: an asynchronous mode is bursts between silences,
            // and the meter tells "nothing on the channel" from "nothing
            // decoded".
            ui.add(
                egui::ProgressBar::new(st.level.clamp(0.0, 1.0))
                    .desired_width(70.0)
                    .fill(theme::CYAN_DIM()),
            )
            .on_hover_text("Receive audio level");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} rx", st.total)).size(10.0).color(theme::gray(120)),
                );
            });
        });
        ui.add_space(4.0);
        ui.separator();

        if st.messages.is_empty() {
            ui.label(
                RichText::new(
                    "Listening. JTTY transmissions start at any time — a message \
                     appears here a moment after it is heard.",
                )
                .size(10.0)
                .weak(),
            );
            return;
        }

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for m in st.messages.iter().rev() {
                let secs = m.at_unix.rem_euclid(86_400);
                let stamp = format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(stamp).monospace().size(11.0).color(theme::gray(120)));
                    ui.label(
                        RichText::new(format!("{:>5.0}", m.audio_hz))
                            .monospace()
                            .size(11.0)
                            .color(theme::gray(120)),
                    );
                    ui.label(
                        RichText::new(format!("{:>3}", m.snr_db))
                            .monospace()
                            .size(11.0)
                            .color(theme::CYAN_DIM()),
                    );
                    // A partial message (no end-of-message flag) is dimmed, so a
                    // run cut off by the next transmission reads as incomplete.
                    let color = if m.complete { theme::TEXT() } else { theme::gray(140) };
                    ui.label(RichText::new(&m.text).monospace().size(12.5).color(color));
                });
            }
        });
    }

    /// JTTY's transmit row, under its message log.
    ///
    /// A single line: the message, a key and CQ. JTTY is asynchronous, so a
    /// press sends the message once — there is no T/R period, no repeat, and no
    /// sequencing. The over ends on its own when the burst is done.
    fn jtty_tx_row(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let tx_on = self.digi_status.as_ref().is_some_and(|s| s.transmitting);
        let tx_ok = self.tx_capable();
        ui.add_space(4.0);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("TX").size(10.5).strong().color(theme::CYAN()));
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.text_tx)
                    .desired_width(260.0)
                    .hint_text("W1ABC W9XYZ FN42"),
            );
            if field.changed() {
                cmds.push(Command::DigiTxText(self.text_tx.clone()));
            }
            let label = if tx_on { " SENDING " } else { "   TX   " };
            if tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    tx_on,
                    RichText::new(label).size(13.0).strong(),
                    theme::ALERT(),
                    Color32::WHITE,
                )
            })
            .clicked()
            {
                cmds.push(Command::DigiTxText(self.text_tx.clone()));
                cmds.push(Command::DigiTxActive(!tx_on));
            }
            if tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(" CALL CQ ").size(12.0).strong(),
                    theme::GREEN(),
                    theme::INK_ON_CYAN(),
                )
            })
            .clicked()
            {
                let call = if self.digi_cfg_edit.my_call.is_empty() {
                    "NOCALL".into()
                } else {
                    self.digi_cfg_edit.my_call.clone()
                };
                let cq = format!("CQ {call} CQ");
                cmds.push(Command::DigiAbortTx);
                self.text_tx = cq.clone();
                cmds.push(Command::DigiTxText(cq));
                cmds.push(Command::DigiTxActive(true));
            }
        });
        ui.label(
            RichText::new(
                "The message is sent once when you press TX — JTTY is asynchronous, \
                 so there is no period to wait for and nothing repeats.",
            )
            .size(9.5)
            .color(theme::CYAN_DIM()),
        );
    }
}
