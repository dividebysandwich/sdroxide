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
        RichText::new("Profiles: the station's saved setups")
            .size(14.0)
            .strong()
            .color(theme::CYAN()),
    );
    ui.add_space(4.0);
    super::note(
        ui,
        "A profile captures the whole working setup — the dials and VFOs, the mode \
             and filters, the gains, drive and antennas, your callsign and message \
             templates, and the per-band registers — and applies it back in one click. \
             The hardware (back end, audio devices, converters) is deliberately left \
             alone: a profile is the way you work the station, not the radio it runs on.",
    );
    ui.add_space(10.0);

    ui.label(RichText::new("Save the current setup").strong());
    ui.end_row();
    ui.horizontal(|ui| {
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(io.profile_name).hint_text("profile name"),
        );
        if !io.profile_name.trim().is_empty() && ui.button("Save").clicked() {
            cmds.push(Command::ProfileSave(io.profile_name.trim().to_string()));
            io.profile_name.clear();
        }
    });
    super::note(
        ui,
        "Applies to whatever sdroxide is attached to: dials and VFOs, mode and \
             filters, levels, gains, drive, antennas, the digital identity and the band \
             stacks.",
    );
    ui.add_space(12.0);

    if profiles.is_empty() {
        super::note(ui, "No profiles saved yet — name one above and press Save.");
        return;
    }

    ui.label(RichText::new("Saved profiles").strong());
    ui.end_row();
    ui.add_space(4.0);
    for name in profiles {
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).strong());
            if ui
                .button("Apply")
                .on_hover_text(format!(
                    "Put the station back onto \u{201c}{name}\u{201d}: dials, VFOs, mode, filters, \
                 gains, drive, antennas, identity and band stacks."
                ))
                .clicked()
            {
                cmds.push(Command::ProfileApply(name.clone()));
                // The engine rewrites the digital identity in place; its answer
                // re-seeds this screen's copy of it, or the next edit here
                // would put the old callsign back.
                *io.digi_reseed = true;
            }
            if ui.button("✕").on_hover_text(format!("Delete \u{201c}{name}\u{201d}")).clicked() {
                cmds.push(Command::ProfileDelete(name.clone()));
            }
        });
        ui.add_space(2.0);
    }
}
