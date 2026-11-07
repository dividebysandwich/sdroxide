//! The FreeDV / RADE digital-voice panel.
//!
//! There is no text to show and no decode list to draw, so the panel is a
//! status view: sync state, SNR, and the reporting the mode does to FreeDV
//! Reporter on the operator's behalf.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::Command;

use crate::app::{SdroxideApp, tx_gated};

impl SdroxideApp {
    pub(in crate::app) fn rade_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        _panel_h: f32,
    ) {
        let status = self.digi_status.clone();
        let rade = status.as_ref().and_then(|s| s.rade).unwrap_or_default();
        let transmitting = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        // The last callsign decoded from a remote End-of-Over frame. The
        // controller has always carried it in `dx_call` (and reported it to
        // FreeDV Reporter from the same place) — this is where it reaches the
        // operator. It is `None` while transmitting: our own over is not a
        // station being heard.
        let dx_call = status.as_ref().and_then(|s| s.dx_call.clone());

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr!("rade-rade")).size(11.0).strong().color(crate::theme::CYAN()),
            );
            ui.label(
                RichText::new(tr!("rade-freedv-v1-digital-voice"))
                    .size(10.5)
                    .color(crate::theme::CYAN_DIM()),
            );
            crate::chrome::row_tail(ui, |ui| {
                if transmitting {
                    ui.label(
                        RichText::new(tr!("rade-tx"))
                            .size(11.0)
                            .strong()
                            .color(crate::theme::ALERT()),
                    );
                    ui.add_space(8.0);
                }
                // Silence the raw signal, leaving only decoded speech audible.
                let muted = self.digi_cfg_edit.rade_mute_analog;
                let resp = crate::chrome::chip(
                    ui,
                    muted,
                    RichText::new(tr!("rade-mute-analog")).size(10.5),
                );
                if resp.clicked() && self.digi_cfg_seeded {
                    self.digi_cfg_edit.rade_mute_analog = !muted;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
                resp.on_hover_text(tr!("rade-mute-demodulated-audio-so-only"));
            });
        });
        ui.add_space(8.0);

        // Sync lamp + link readouts.
        ui.horizontal(|ui| {
            let (lamp, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            let lit = rade.sync && !transmitting;
            ui.painter_at(lamp).circle_filled(
                lamp.center(),
                5.5,
                if lit { crate::theme::GREEN() } else { crate::theme::gray(48) },
            );
            ui.label(
                RichText::new(if transmitting {
                    tr!("rade-transmitting")
                } else if rade.sync {
                    tr!("rade-sync")
                } else {
                    String::from("searching")
                })
                .size(12.0)
                .strong()
                .color(if lit {
                    crate::theme::GREEN()
                } else {
                    crate::theme::gray(130)
                }),
            );
            ui.add_space(16.0);
            let dim = crate::theme::gray(150);
            if rade.sync && !transmitting {
                ui.label(
                    RichText::new(tr!("rade-snr-db", snr_db = format!("{:.0}", rade.snr_db)))
                        .size(12.0)
                        .color(crate::theme::TEXT_STRONG()),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(tr!(
                        "rade-offset-hz",
                        freq_offset_hz = format!("{:+.0}", rade.freq_offset_hz)
                    ))
                    .size(11.0)
                    .color(dim),
                )
                .on_hover_text(tr!("rade-how-far-received-signal-sits"));
            } else {
                ui.label(RichText::new(tr!("rade-snr")).size(12.0).color(dim));
            }
            // Who we last heard, from the End-of-Over frame that closed their
            // over — the only time a RADE station names itself. Held for a
            // minute past the signal, then dropped by the controller.
            if let Some(call) = &dx_call {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(tr!("rade-heard", call = call))
                        .size(12.0)
                        .strong()
                        .color(crate::theme::GREEN()),
                )
                .on_hover_text(tr!("rade-callsign-station-put-end-over"));
            }
        });
        ui.add_space(8.0);

        // Decoded-speech level.
        {
            let (bar, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
            let p = ui.painter_at(bar);
            p.rect_filled(bar, 0.0, crate::theme::gray(22));
            let level = rade.rx_level.clamp(0.0, 1.0);
            if level > 0.0 && !transmitting {
                let mut fill = bar;
                fill.set_width(bar.width() * level);
                p.rect_filled(fill, 0.0, crate::theme::CYAN());
            }
        }
        ui.add_space(12.0);

        // Transmit. `DigiTxActive` is the same command the main PTT button ends
        // up sending in this mode, so the two stay in step.
        let tx_ok = self.tx_capable();
        ui.horizontal(|ui| {
            let label = if transmitting { tr!("rade-stop-talking") } else { tr!("rade-talk-ptt") };
            let resp = tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    transmitting,
                    RichText::new(label).size(13.0).strong(),
                    crate::theme::ALERT(),
                    crate::theme::TEXT_STRONG(),
                )
            });
            if resp.clicked() {
                cmds.push(Command::DigiTxActive(!transmitting));
            }
            resp.on_hover_text(tr!("rade-open-close-rade-over-modem"));
            ui.add_space(10.0);
        });
        ui.add_space(12.0);

        ui.label(
            RichText::new(tr!("rade-rade-v1-occupies-roughly-1060"))
                .size(10.5)
                .color(crate::theme::gray(125)),
        );
        if rade.dropped > 0 {
            ui.add_space(6.0);
            ui.label(
                RichText::new(tr!(
                    "rade-samples-dropped-between-receiver-decoder",
                    dropped = rade.dropped
                ))
                .size(10.5)
                .color(crate::theme::YELLOW()),
            );
        }
    }
}
