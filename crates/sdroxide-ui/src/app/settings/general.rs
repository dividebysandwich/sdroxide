//! The General tab's sound-device pickers and the server's sign-in credentials.
//!
//! The operator's own card is always shown; the radio's is only relevant to
//! the CAT / Audio interface, since every other backend carries its audio
//! in-band.

use eframe::egui::{self, Color32, ComboBox, RichText};
use sdroxide_types::{Command, Region, RemoteAccess, SWR_LIMIT_MAX, SWR_LIMIT_MIN, swr_tune_limit};

use crate::app::SdroxideApp;
use crate::app::persist::band_plan_path;
use crate::chrome::StyledCombo;

/// The IARU region dropdown: the number the band plans are published under,
/// with the part of the world it covers next to it.
///
/// Both, because neither alone identifies it for most operators — "Region 2"
/// means nothing until you know it is the Americas, and the number is what
/// every band-plan document and contest rule actually says.
pub(in crate::app) fn region_combo(ui: &mut egui::Ui, region: &mut Region) {
    ComboBox::from_id_salt("iaru-region").width(360.0).selected_text(region.label()).show_styled(
        ui,
        |ui| {
            for r in Region::ALL {
                if ui.selectable_label(*region == r, r.label()).clicked() {
                    *region = r;
                }
            }
        },
    );
}

/// A device dropdown ("System default" + names); calls `pick(Some(name)|None)`.
pub(in crate::app) fn device_combo(
    ui: &mut egui::Ui,
    id: &str,
    names: &[String],
    selected: &Option<String>,
    mut pick: impl FnMut(Option<String>),
) {
    let shown = selected.clone().unwrap_or_else(|| "System default".into());
    ComboBox::from_id_salt(id).width(300.0).selected_text(shown).show_styled(ui, |ui| {
        if ui.selectable_label(selected.is_none(), "System default").clicked() {
            pick(None);
        }
        for n in names {
            if ui.selectable_label(selected.as_deref() == Some(n), n).clicked() {
                pick(Some(n.clone()));
            }
        }
    });
}

/// Who may connect to this machine's server (`[remote_access]` in
/// `config.toml`).
///
/// Only drawn when the engine is in this process. These are a file on the
/// machine the radio is attached to: a remote client has nothing here to read
/// them from, and offering it a box that writes to its own disk instead would
/// be worse than offering nothing — it would look as though the station's
/// password had been changed when it had not.
///
/// Written as it is typed, like the control bindings, rather than behind an
/// APPLY: the server re-reads the file for every sign-in, so there is no
/// separate step for an APPLY to stand for.
pub(in crate::app) fn remote_access_settings(ui: &mut egui::Ui, access: &mut RemoteAccess) {
    ui.label(RichText::new("Remote access").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "What a remote client — the browser page, or another sdroxide started with \
             --connect — has to give before this station will let it operate. Applies in server \
             mode (--server); the next sign-in picks up a change, with no restart.",
        )
        .size(11.5)
        .weak(),
    );
    ui.add_space(8.0);
    egui::Grid::new("remote-access-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Username");
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut access.username).desired_width(200.0),
        );
        ui.end_row();
        ui.label("Password");
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut access.password).password(true).desired_width(200.0),
        );
        ui.end_row();
    });
    ui.add_space(6.0);
    // The state of the door, said plainly. "Both boxes empty means anyone may
    // key my transmitter" is not something an operator should have to infer.
    if access.is_enforced() {
        if access.username.is_empty() {
            ui.label(
                RichText::new(
                    "Clients must give the password. Leaving the username empty is fine.",
                )
                .size(11.5)
                .color(crate::theme::GREEN()),
            );
        } else {
            ui.label(
                RichText::new("Clients must sign in.").size(11.5).color(crate::theme::GREEN()),
            );
        }
    } else {
        ui.label(
            RichText::new(
                "⚠ Empty: anyone who can reach the server's port can operate this radio, on \
                 your callsign. Set a password before forwarding the port.",
            )
            .size(11.5)
            .color(crate::theme::YELLOW()),
        );
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Stored in the clear in config.toml, like the other passwords sdroxide keeps.",
        )
        .size(10.5)
        .color(crate::theme::gray(140)),
    );
}

/// The fixed trim on this radio's receive audio.
///
/// Lives beside the sound-card pickers because that is what it is for: the AF
/// rail on the strip tops out at unity, which can turn a radio down and never
/// up, and a transceiver whose USB codec puts out a quiet signal is then quiet
/// at full volume in sdroxide *and* in the operating system (issue #315). It is
/// per radio, because what it corrects is that radio's interface rather than
/// how loudly anyone wants to listen.
///
/// Edited straight into `radio_edit`, which the settings window sends and saves
/// on any change — so it takes effect while the operator is listening, with no
/// Apply and no reopen.
pub(in crate::app) fn settings_rx_audio_gain(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::RadioConfig,
) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Receive audio gain").strong());
        ui.add(
            egui::DragValue::new(&mut cfg.rx_audio_gain_db)
                .speed(0.5)
                .range(-20.0..=30.0)
                .fixed_decimals(1)
                .suffix(" dB"),
        );
        if cfg.rx_audio_gain_db != 0.0 && ui.button("0 dB").clicked() {
            cfg.rx_audio_gain_db = 0.0;
        }
    });
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Extra gain on everything this radio sends to the speakers, on top of the volume \
             control. Leave it at 0 dB unless the radio is quiet at full volume: the volume \
             rail's top is the audio as it arrives, so it can turn a radio down but never up, \
             and some transceivers' USB sound output sits well below full scale.\n\n\
             Go up 6 dB at a time. Too much clips — the audio is limited at full scale rather \
             than allowed to wrap round, so overdoing it sounds harsh rather than loud. \
             Recordings are taken ahead of this and are not affected.",
        )
        .size(10.5)
        .color(crate::theme::gray(140)),
    );
}

impl SdroxideApp {
    /// The band-plan file: where it is, whether the station is on it, and the
    /// button that re-reads it.
    ///
    /// No editor here on purpose. A band plan is forty rows of numbers that
    /// exist to be pasted from a published table and compared against one; a
    /// text editor does that far better than any grid this dialog could hold,
    /// and the file is written to be readable. What the dialog owes the
    /// operator is the path, and a way to apply an edit without restarting.
    pub(in crate::app) fn settings_band_plan_file(
        &self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<sdroxide_types::Command>,
    ) {
        let custom = !sdroxide_types::band_plan().is_default();
        ui.horizontal_wrapped(|ui| {
            if crate::chrome::chip(ui, false, RichText::new("RELOAD BAND PLAN").size(10.5))
                .on_hover_text(
                    "Re-read bandplan.json on the machine the radio is attached to and apply \
                     it — band edges, sub-segments and skimmer windows — without restarting.",
                )
                .clicked()
            {
                cmds.push(sdroxide_types::Command::ReloadBandPlan);
            }
            ui.add(
                egui::Label::new(
                    RichText::new(if custom {
                        "Running on the station's own band plan."
                    } else {
                        "Running on the built-in IARU tables."
                    })
                    .size(10.5)
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.add_space(4.0);
        // Only where the file is actually on this machine. A remote client
        // showing its own config path would be pointing at the wrong computer.
        let path = (!self.ctrl.engine_is_remote())
            .then(band_plan_path)
            .flatten()
            .map(|p| p.display().to_string());
        ui.label(
            RichText::new(match &path {
                Some(p) => format!(
                    "Every band edge and sub-segment comes from {p}, written from the built-in \
                     IARU tables the first time and yours to edit after that — narrow a band to \
                     your licence and sdroxide will refuse to transmit outside it. Frequencies \
                     are in MHz; delete the file for a fresh copy of the defaults. This is the \
                     regional allocation, not your licence: your own conditions may be narrower, \
                     and national plans differ inside a region.",
                ),
                None => "Every band edge and sub-segment comes from bandplan.json on the machine \
                         the radio is attached to. This is the regional allocation, not your \
                         licence: your own conditions may be narrower, and national plans differ \
                         inside a region."
                    .to_string(),
            })
            .size(10.5)
            .color(crate::theme::gray(140)),
        );
    }

    /// The SWR guard: arm it, and set the ratio it stops transmitting at.
    ///
    /// Reads the live values out of the broadcast TX state rather than off
    /// disk, so a remote client shows the radio's setting and not its own
    /// machine's `config.toml`, which would be a different antenna entirely.
    /// The command is sent only on an actual change, since a `DragValue` reports
    /// its value every frame it is dragged and each one would be a config write.
    pub(in crate::app) fn settings_swr_guard(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.label(RichText::new("SWR guard").strong());
        ui.add_space(4.0);

        let mut enabled = self.state.tx.swr_guard;
        // The engine clamps this too; matching the range here keeps the widget
        // from offering a value that would come back changed.
        let mut limit = self.state.tx.swr_limit.clamp(SWR_LIMIT_MIN, SWR_LIMIT_MAX);

        ui.horizontal(|ui| {
            if crate::chrome::checkbox(ui, &mut enabled, "Stop transmitting on high SWR").changed()
            {
                cmds.push(Command::SetSwrGuard { enabled, limit });
            }
        });
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal(|ui| {
                ui.label("Trip at");
                let r = ui.add(
                    egui::DragValue::new(&mut limit)
                        .speed(0.1)
                        .range(SWR_LIMIT_MIN..=SWR_LIMIT_MAX)
                        .fixed_decimals(1)
                        .suffix(":1"),
                );
                // `drag_stopped` and `lost_focus`, not `changed`: one command
                // per settled value rather than one per frame of the drag.
                if (r.drag_stopped() || r.lost_focus()) && limit != self.state.tx.swr_limit {
                    cmds.push(Command::SetSwrGuard { enabled, limit });
                }
                // The tune limit is derived from this one rather than typed, so
                // it is shown here: an operator who has just had a tune-up
                // stopped is told a figure, and this is where they find out
                // where it came from.
                ui.label(
                    RichText::new(format!("(tuning: {:.1}:1)", swr_tune_limit(limit)))
                        .size(11.0)
                        .color(crate::theme::gray(140)),
                );
            });
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Stops the transmission when the radio reports an SWR at or above this figure, and \
                 keeps transmit locked out until you acknowledge it. Catches a disconnected \
                 antenna, a failed feeder, or a switch left on the wrong port.\n\n\
                 Tuning is treated differently, because feeding a mismatch is the point of it: an \
                 antenna tuner gets double the limit and about five seconds before the guard \
                 applies at all. A manual tuner that takes longer than that wants the guard \
                 switched off for the session.\n\n\
                 Needs a rig that reports SWR over CAT. Ignores the first fifth of a second of \
                 each transmission, and does not wait for high power.",
            )
            .size(10.5)
            .color(crate::theme::gray(140)),
        );
        if let Some(swr) = self.state.tx.swr_tripped {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "⚠ Currently tripped at {swr:.1}:1 — transmit is locked out."
                ))
                .size(11.0)
                .color(Color32::from_rgb(255, 190, 70)),
            );
        }
    }

    /// The user's own speakers / microphone (applied live).
    pub(in crate::app) fn settings_user_audio(
        &self,
        ui: &mut egui::Ui,
        audio_pick: &mut Option<(bool, Option<String>)>,
    ) {
        let Some(devs) = &self.audio_devices else {
            return;
        };
        ui.label(RichText::new("Your audio (speakers / microphone)").strong());
        egui::Grid::new("user-audio").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Output");
            device_combo(ui, "u-out", &devs.outputs, &devs.selected_output, |n| {
                *audio_pick = Some((true, n))
            });
            ui.end_row();
            ui.label("Input");
            device_combo(ui, "u-in", &devs.inputs, &devs.selected_input, |n| {
                *audio_pick = Some((false, n))
            });
            ui.end_row();
        });
    }
}
