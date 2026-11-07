//! The Hardrock amplifier: which one, on which port, and what it should do on
//! its own. The live panel is the AMP window; this tab only sets the link up.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, ComboBox, RichText};
use sdroxide_types::{AMP_BAUD_RATES, AmpConfig, AmpModel, AmpStatus};

use crate::chrome::StyledCombo;
use crate::{chrome, theme};

pub(in crate::app) fn settings_amp_tab(
    ui: &mut egui::Ui,
    cfg: &mut AmpConfig,
    seeded: bool,
    status: &AmpStatus,
    serial_ports: &[String],
    apply: &mut bool,
) {
    ui.label(RichText::new(tr!("settings-amp-amplifier")).size(14.0).strong().color(theme::CYAN()));
    ui.add_space(4.0);
    if !seeded {
        ui.label(
            RichText::new(tr!("settings-amp-waiting-station-s-amplifier-configuration")).weak(),
        );
        return;
    }
    ui.label(RichText::new(tr!("settings-amp-hobbypcb-hardrock-500-hardrock-50")).weak());
    ui.add_space(8.0);

    chrome::checkbox(ui, &mut cfg.enabled, tr!("settings-amp-enable"));
    ui.add_space(4.0);
    egui::Grid::new("amp-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-amp-model"));
        super::enum_combo(ui, "amp-model", &mut cfg.model, &AmpModel::ALL, AmpModel::tr);
        ui.end_row();
        if !cfg.model.tested() {
            ui.label("");
            ui.label(
                RichText::new(tr!("settings-amp-untested-panel-has-only-been"))
                    .color(theme::YELLOW()),
            );
            ui.end_row();
        }

        ui.label(tr!("settings-amp-port"));
        let shown = if cfg.serial.path.is_empty() {
            tr!("settings-amp-choose")
        } else {
            cfg.serial.path.clone()
        };
        // The list is of the engine's machine, like the T/R switch's.
        ComboBox::from_id_salt("amp-port").width(200.0).selected_text(shown).show_styled(
            ui,
            |ui| {
                for p in serial_ports {
                    if ui.selectable_label(&cfg.serial.path == p, p).clicked() {
                        cfg.serial.path = p.clone();
                    }
                }
            },
        );
        ui.end_row();

        ui.label(tr!("settings-amp-baud"));
        ComboBox::from_id_salt("amp-baud")
            .width(200.0)
            .selected_text(cfg.serial.baud.to_string())
            .show_styled(ui, |ui| {
                for b in AMP_BAUD_RATES {
                    if ui.selectable_label(cfg.serial.baud == b, b.to_string()).clicked() {
                        cfg.serial.baud = b;
                    }
                }
            })
            .response
            .on_hover_text(tr!("settings-amp-must-match-amplifier-s-menu"));
        ui.end_row();
    });
    ui.add_space(6.0);
    chrome::checkbox(ui, &mut cfg.follow_band, tr!("settings-amp-follow-radio-s-band"))
        .on_hover_text(tr!("settings-amp-sends-transmit-frequency-amplifier-every"));
    chrome::checkbox(ui, &mut cfg.feed_tx_meter, tr!("settings-amp-show-power-swr-tx-meter"))
        .on_hover_text(tr!("settings-amp-only-where-radio-does-not"));
    ui.add_space(8.0);

    let (text, col) = if !status.configured {
        (tr!("settings-amp-off"), theme::TEXT())
    } else if status.connected {
        (tr!("settings-amp-connected-to", model = status.model.tr()), theme::GREEN())
    } else if let Some(e) = &status.error {
        (e.clone(), theme::ALERT())
    } else {
        (tr!("settings-amp-connecting"), theme::YELLOW())
    };
    ui.horizontal(|ui| {
        ui.label(tr!("settings-amp-status"));
        ui.label(RichText::new(text).color(col));
    });
    ui.add_space(6.0);
    if chrome::chip_accent(
        ui,
        false,
        RichText::new(tr!("settings-amp-apply")).strong(),
        theme::GREEN(),
        theme::INK_ON_CYAN(),
    )
    .on_hover_text(tr!("settings-amp-save-re-open-amplifier-s"))
    .clicked()
    {
        *apply = true;
    }
}
