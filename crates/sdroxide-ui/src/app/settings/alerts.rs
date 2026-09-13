//! The Settings → Alerts tab.
//!
//! This is the audible-alarm side of `Settings → UI → Voice announcements`,
//! aimed the other way: those read out what *changed*; these ring when a
//! *decode matters*, which is unmissable by design — the whole point is to
//! reach the operator when they are not looking at sdroxide.

use eframe::egui::{self, Color32, RichText};

use crate::app::alerts::AlertStatus;
use crate::app::settings::enum_combo;
use crate::app::settings::general::device_combo;
use sdroxide_types::{AlertEvent, AlertSettings, AlertSound};

pub(in crate::app) fn alerts_settings(
    ui: &mut egui::Ui,
    cfg: &mut AlertSettings,
    outputs: &[String],
    status: &AlertStatus,
    test: &mut bool,
) {
    ui.label(RichText::new("Audible alerts").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, "Sound an alarm when a decode matters")
        .on_hover_text(
            "Plays over its own audio output, so it is heard even when the band is in a \
             different speaker than this screen.",
        );

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("alerts-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label("Volume");
            crate::chrome::slider(ui, egui::Slider::new(&mut cfg.volume, 0.0..=1.0).step_by(0.05));
            ui.end_row();

            ui.label("Output");
            // Same contract as the speech tab: the current selection is read
            // into a copy because the dropdown's closure hands the new one back.
            let cur = cfg.device.clone();
            device_combo(ui, "alerts-out", outputs, &cur, |n| cfg.device = n);
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Test").clicked() {
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
        egui::CollapsingHeader::new("What to sound").default_open(true).show(ui, |ui| {
            ui.add_space(4.0);
            // One row per event: what it is, whether it rings, and with
            // which of the sounds. The rows are shared with the decode
            // list's badges, so "A station is calling me" here is "badge
            // yellow on the list" there.
            for event in AlertEvent::ALL {
                let rule = event.rule_mut(&mut cfg.events);
                ui.horizontal(|ui| {
                    crate::chrome::checkbox(ui, &mut rule.enabled, event.label());
                    enum_combo(
                        ui,
                        &format!("alert-{}", event.as_str()),
                        &mut rule.sound,
                        &AlertSound::ALL,
                        AlertSound::label,
                    );
                });
                ui.add_space(2.0);
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Each station is quiet for a while after an alert, so a busy band \
                         does not ring every slot.",
                )
                .weak()
                .small(),
            );
        });
    });
}
