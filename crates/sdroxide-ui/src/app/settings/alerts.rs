//! The Settings → Alerts tab.
//!
//! This is the audible-alarm side of `Settings → UI → Voice announcements`,
//! aimed the other way: those read out what *changed*; these ring when a
//! *decode matters*, which is unmissable by design — the whole point is to
//! reach the operator when they are not looking at sdroxide.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, Color32, RichText};

use crate::app::alerts::AlertStatus;
use crate::app::settings::enum_combo;
use crate::app::settings::general::device_combo;
use sdroxide_types::{AlertEvent, AlertReply, AlertSettings, AlertSound};

pub(in crate::app) fn alerts_settings(
    ui: &mut egui::Ui,
    cfg: &mut AlertSettings,
    outputs: &[String],
    status: &AlertStatus,
    test: &mut bool,
) {
    ui.label(
        RichText::new(tr!("settings-alerts-audible-alerts"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(
        ui,
        &mut cfg.enabled,
        tr!("settings-alerts-sound-alarm-when-decode-matters"),
    )
    .on_hover_text(tr!("settings-alerts-plays-over-own-audio-output"));

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("alerts-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label(tr!("settings-alerts-volume"));
            crate::chrome::slider(ui, egui::Slider::new(&mut cfg.volume, 0.0..=1.0).step_by(0.05));
            ui.end_row();

            ui.label(tr!("settings-alerts-output"));
            // Same contract as the speech tab: the current selection is read
            // into a copy because the dropdown's closure hands the new one back.
            let cur = cfg.device.clone();
            device_combo(ui, "alerts-out", outputs, &cur, |n| cfg.device = n);
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(tr!("settings-alerts-test")).clicked() {
                *test = true;
            }
            if let Some(note) = status.note() {
                let text = RichText::new(note);
                ui.label(if status.is_failed() {
                    text.color(Color32::from_rgb(0xE0, 0x6C, 0x4B))
                } else {
                    text.weak()
                });
            }
        });

        ui.add_space(4.0);
        egui::CollapsingHeader::new(tr!("settings-alerts-what-sound"))
            .id_salt("alerts-what")
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(4.0);
                // One row per event: what it is, whether it rings, and with
                // which of the sounds. The rows are shared with the decode
                // list's badges, so "A station is calling me" here is "badge
                // yellow on the list" there.
                for event in AlertEvent::ALL {
                    let rule = event.rule_mut(&mut cfg.events);
                    let tone = rule.reply.plays_tone();
                    // Wrapped: a checkbox and two combos are wider than a phone.
                    ui.horizontal_wrapped(|ui| {
                        crate::chrome::checkbox(ui, &mut rule.enabled, event.tr());
                        // The sound only matters when the reply makes one; a
                        // voice-only rule greys it rather than hiding it, so the
                        // row keeps its shape.
                        ui.add_enabled_ui(tone, |ui| {
                            enum_combo(
                                ui,
                                &format!("alert-{}", event.as_str()),
                                &mut rule.sound,
                                &AlertSound::ALL,
                                AlertSound::tr,
                            );
                        });
                        enum_combo(
                            ui,
                            &format!("alert-reply-{}", event.as_str()),
                            &mut rule.reply,
                            &AlertReply::ALL,
                            AlertReply::tr,
                        );
                    });
                    ui.add_space(2.0);
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr!("settings-alerts-each-station-quiet-while-after"))
                        .weak()
                        .small(),
                );
            });
    });
}
