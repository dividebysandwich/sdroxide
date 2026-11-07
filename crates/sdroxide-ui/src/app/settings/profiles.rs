//! The Profiles tab (issue #197): save the station's setup under a name, and
//! put it back on whole in one click.
//!
//! A profile captures the whole working setup — the dials and VFOs, the mode
//! and filters, the gains, drive and antennas, the callsign and message
//! templates, and the per-band registers — and applying one restores all of
//! it. The hardware (back end, audio devices, converters) is deliberately
//! left out, so a profile is the way the operator works the station rather
//! than the radio it runs on.
//!
//! The engine owns the store and answers every action with the fresh name
//! list, which is all this tab ever holds: the profiles themselves live with
//! everything else the radio remembers, wherever the radio happens to be.

use crate::i18n::tr;
use eframe::egui::{self, RichText};

use sdroxide_types::Command;

use crate::app::settings::SettingsIo;

pub(in crate::app) fn settings_profiles_tab(
    ui: &mut egui::Ui,
    io: &mut SettingsIo,
    cmds: &mut Vec<Command>,
    profiles: &[String],
) {
    use crate::theme;

    ui.label(
        RichText::new(tr!("settings-profiles-profiles-station-s-saved-setups"))
            .size(14.0)
            .strong()
            .color(theme::CYAN()),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-profiles-profile-captures-whole-working-setup")).weak());
    ui.add_space(10.0);

    ui.label(RichText::new(tr!("settings-profiles-save-current-setup")).strong());
    ui.horizontal(|ui| {
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(io.profile_name)
                .hint_text(tr!("settings-profiles-profile-name")),
        );
        if !io.profile_name.trim().is_empty() && ui.button(tr!("settings-profiles-save")).clicked()
        {
            cmds.push(Command::ProfileSave(io.profile_name.trim().to_string()));
            io.profile_name.clear();
        }
    });
    ui.label(
        RichText::new(tr!("settings-profiles-applies-whatever-sdroxide-attached-dials")).weak(),
    );
    ui.add_space(12.0);

    if profiles.is_empty() {
        ui.label(RichText::new(tr!("settings-profiles-no-profiles-saved-yet-name")).weak());
        return;
    }

    ui.label(RichText::new(tr!("settings-profiles-saved-profiles")).strong());
    ui.add_space(4.0);
    for name in profiles {
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).strong());
            if ui
                .button(tr!("settings-profiles-apply"))
                .on_hover_text(tr!("settings-profiles-apply-hint", name = name.as_str()))
                .clicked()
            {
                cmds.push(Command::ProfileApply(name.clone()));
                // The engine rewrites the digital identity in place; its answer
                // re-seeds this screen's copy of it, or the next edit here
                // would put the old callsign back.
                *io.digi_reseed = true;
            }
            if ui
                .button("✕")
                .on_hover_text(tr!("settings-profiles-delete", name = name.as_str()))
                .clicked()
            {
                cmds.push(Command::ProfileDelete(name.clone()));
            }
        });
        ui.add_space(2.0);
    }
}
