//! The General tab's sound-device pickers and the server's sign-in credentials.
//!
//! The operator's own card is always shown; the radio's is only relevant to
//! the CAT / Audio interface, since every other backend carries its audio
//! in-band.

use eframe::egui::{self, Color32, ComboBox, RichText};
use sdroxide_types::{
    CbPlan, Command, Region, RemoteAccess, SWR_LIMIT_MAX, SWR_LIMIT_MIN, swr_tune_limit,
};

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

/// The licence-free channel-plan dropdown: which country's (or service's)
/// channels the dial reads in — the 11 m citizens' band plans, and the 446 MHz
/// PMR446 ones. Only the channels and the channel the band opens on — the
/// band's edges are left wide, so switching plans never changes what receives
/// or transmits.
pub(in crate::app) fn cb_plan_combo(ui: &mut egui::Ui, plan: &mut CbPlan) {
    ComboBox::from_id_salt("cb-plan").width(360.0).selected_text(plan.label()).show_styled(
        ui,
        |ui| {
            for p in CbPlan::ALL {
                if ui.selectable_label(*plan == p, p.label()).clicked() {
                    *plan = p;
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

/// A device picker that can also be typed into.
///
/// [`device_combo`]'s list is a convenience, not the truth. A sound server can
/// expose a device the host API does not enumerate — a PipeWire node, a
/// capture device another program is holding, one that appears only once
/// something asks for it — and a picker that can only be pointed at leaves the
/// operator with no way to name it. That is not hypothetical here: enumerating
/// PipeWire nodes is the open half of the Bluetooth microphone work.
///
/// So the value is one editable line under the list, and typing wins. The field
/// is **controlled** — re-seeded from the value every frame rather than kept in
/// a buffer of its own — so a name picked from the list and a name typed in are
/// the same value and cannot drift apart, and there is nothing per-field to
/// keep alive between frames or to go stale when a pick arrives from elsewhere.
pub(in crate::app) fn device_field(
    ui: &mut egui::Ui,
    id: &str,
    names: &[String],
    selected: &mut Option<String>,
    what: &str,
) {
    // Stacked, not side by side: in a two-column grid row the second control
    // lands in the next cell and the entry field is clipped to a few characters
    // of whatever the name happens to start with.
    ui.vertical(|ui| {
        // The list reads the value and the closure writes it, so the value is
        // copied across rather than borrowed twice.
        let shown = selected.clone();
        device_combo(ui, id, names, &shown, |n| *selected = n);
        let mut text = selected.clone().unwrap_or_default();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut text)
                .id(egui::Id::new(id).with("typed"))
                .desired_width(super::audio::DEVICE_WIDTH)
                .hint_text("…or type the name — empty is the system default"),
        );
        if resp.changed() {
            *selected = (!text.is_empty()).then_some(text);
        }
        resp.on_hover_text(format!(
            "The exact name {what} is opened by. Pick it from the list when it is there; type it \
             when the list does not show it — a sound server can offer a device the host API \
             does not enumerate. Empty uses the system default."
        ));
    });
}

/// Which station this client is signed in to, and the way back out of it.
///
/// The other half of the sign-in cookie, and it lives here because a cookie
/// outlives a reload: an operator who signed in for a day has no reason to
/// think about it again until they want to, and "somebody else's browser" is
/// exactly the case where being signed in matters. A native client has no
/// cookie — it keeps a copy of the password instead — so the same button clears
/// that, and says which it is doing rather than showing a control that quietly
/// does the wrong half.
pub(in crate::app) fn signed_in_station(ui: &mut egui::Ui, station: &str) {
    let station_key = crate::login::station_key(station);
    ui.label(RichText::new("Signed-in station").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    ui.label(RichText::new(station_key.clone()).size(11.5).weak());
    ui.add_space(8.0);

    let mut sign_out = false;
    ui.horizontal(|ui| {
        sign_out = crate::chrome::chip_accent(
            ui,
            false,
            RichText::new(" SIGN OUT ").strong().size(12.0),
            crate::theme::ALERT(),
            crate::theme::INK_ON_CYAN(),
        )
        .clicked();
    });
    sign_out.then(|| crate::login::sign_out(&station_key));

    let fallback = crate::login::cookie_fallback_stations();
    if fallback.is_empty() {
        ui.add_space(4.0);
        super::note_at(
            ui,
            "Forgets this device's sign-in. The station has to ask again on the next \
                 connection; this one stays up until you close it.",
            Some(11.0),
        );
    } else {
        // The cookie could not be taken, so a password is being kept in the page
        // instead. Said here because the alternative is a browser holding a
        // station's password that the operator believes it does not.
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "Remembering a password in this browser for {}, because it answered no sign-in \
                 cookie. Older stations do this; it is not the same thing.",
                fallback.join(", ")
            ))
            .size(11.0)
            .color(crate::theme::gray(140)),
        );
    }
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
    super::note_at(
        ui,
        "What a remote client — the browser page, or another sdroxide started with \
             --connect — has to give before this station will let it operate. Applies in server \
             mode (--server); the next sign-in picks up a change, with no restart.",
        Some(11.5),
    );
    ui.add_space(8.0);
    egui::Grid::new("remote-access-grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .min_row_height(ui.spacing().interact_size.y)
        .show(ui, |ui| {
            ui.label("Username");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut access.username).desired_width(200.0),
            );
            ui.end_row();
            ui.label("Password");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut access.password)
                    .password(true)
                    .desired_width(200.0),
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
        ui.end_row();
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
    // Wrapped: an unwrapped label here runs the width of the page and is drawn
    // over the paragraph after it, which is what rendering this page showed.
    ui.add(
        egui::Label::new(
            RichText::new(
                "Extra gain on everything this radio sends to the speakers, on top of the \
                 volume control. Leave it at 0 dB unless the radio is quiet at full volume: the \
                 volume rail's top is the audio as it arrives, so it can turn a radio down but \
                 never up, and some transceivers' USB sound output sits well below full scale.\n\n\
                 Go up 6 dB at a time. Too much clips — the audio is limited at full scale rather \
                 than allowed to wrap round, so overdoing it sounds harsh rather than loud. \
                 Recordings are taken ahead of this and are not affected.",
            )
            .size(10.5)
            .color(crate::theme::gray(140)),
        )
        .wrap(),
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

    /// Take this station's settings away as one file, and put one back.
    ///
    /// The answer to "how do I copy all this to my other machine" being
    /// "screenshots" (issue #356). Native only, and only where the settings are
    /// on *this* machine: a browser client has no filesystem, and a remote one
    /// would be exporting its own laptop's configuration rather than the
    /// station's, which is the opposite of what was asked for.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn settings_transfer(
        &self,
        ui: &mut egui::Ui,
        export: &mut bool,
        import: &mut bool,
    ) {
        ui.label(RichText::new("Settings file").strong());
        ui.end_row();
        if self.ctrl.engine_is_remote() {
            ui.label(
                RichText::new(
                    "The settings are on the machine the radio is attached to. Export them \
                     there.",
                )
                .size(10.5)
                .color(crate::theme::gray(140)),
            );
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if crate::chrome::chip(ui, false, RichText::new("EXPORT…").size(10.5))
                .on_hover_text(
                    "Write every setting at this station — the radios, the modes, the servers, \
                     the memories, the band plan — to one file you can carry to another \
                     installation. Your logbook and any saved server password stay here.",
                )
                .clicked()
            {
                *export = true;
            }
            if crate::chrome::chip(ui, false, RichText::new("IMPORT…").size(10.5))
                .on_hover_text(
                    "Replace this station's settings with the ones in a file exported from \
                     another installation. Restart sdroxide afterwards.",
                )
                .clicked()
            {
                *import = true;
            }
        });
        if let Some(note) = &self.settings_transfer_note {
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(RichText::new(note).size(10.5).color(Color32::LIGHT_GREEN)).wrap(),
            );
        }
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                RichText::new(
                    "An import overwrites what is here, file for file, and takes effect the \
                     next time sdroxide starts — the settings already in memory would \
                     otherwise be written straight back over it. Anything the file does not \
                     mention is left as it is, so a bundle from a one-radio station does not \
                     remove a second radio here. Your logbook is never in the file: export it \
                     as ADIF from the LOG window if you want to move that too.",
                )
                .size(10.5)
                .color(crate::theme::gray(140)),
            )
            .wrap(),
        );
    }

    /// The browser client has no filesystem, and the settings it would export
    /// are on the engine's machine in any case.
    #[cfg(target_arch = "wasm32")]
    pub(in crate::app) fn settings_transfer(
        &self,
        _ui: &mut egui::Ui,
        _export: &mut bool,
        _import: &mut bool,
    ) {
    }

    /// Carry out what [`Self::settings_transfer`]'s buttons asked for, after
    /// the window closure has given `&mut self` back — see [`SettingsIo`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn run_settings_transfer(&mut self, export: bool, import: bool) {
        if export {
            self.settings_transfer_note = Some(match sdroxide_config::transfer::export_json() {
                Ok(json) => {
                    let note = sdroxide_config::transfer::export()
                        .map(|b| b.summary())
                        .unwrap_or_else(|_| "settings".into());
                    crate::download::save("sdroxide-settings.json", json.as_bytes());
                    format!("Exported {note} — choose where to save it.")
                }
                Err(e) => format!("Export failed: {e}"),
            });
        }
        if import {
            self.settings_transfer_note = None;
            crate::download::load_text(
                "sdroxide settings",
                &["json"],
                self.settings_import_inbox.clone(),
            );
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::app) fn run_settings_transfer(&mut self, _export: bool, _import: bool) {}

    /// Where recordings are written, and how to change it.
    ///
    /// A `config.toml` path on the machine that records, so it is offered where
    /// the settings are on this machine — like [`Self::settings_transfer`], and
    /// for the same reason: a remote client would be changing its own laptop's
    /// folder, not the station's.
    #[cfg(not(target_arch = "wasm32"))]
    /// The folder row alone, for a page that has already said what it is
    /// called — so the two headings do not sit one line apart saying the same
    /// word twice.
    pub(in crate::app) fn settings_recordings_dir(
        &self,
        ui: &mut egui::Ui,
        choose: &mut bool,
        reset: &mut bool,
    ) {
        if self.ctrl.engine_is_remote() {
            ui.label(
                RichText::new(
                    "Recordings are written on the machine the radio is attached to. Set the \
                     folder there.",
                )
                .size(10.5)
                .color(crate::theme::gray(140)),
            );
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Folder").size(11.0).color(crate::theme::gray(150)));
            ui.label(
                RichText::new(self.recordings_dir.display().to_string())
                    .size(10.5)
                    .monospace()
                    .color(crate::theme::gray(180)),
            );
            if crate::chrome::chip(ui, false, RichText::new("CHOOSE…").size(10.5))
                .on_hover_text("Pick the folder recordings are written to")
                .clicked()
            {
                *choose = true;
            }
            if crate::chrome::chip(ui, false, RichText::new("DEFAULT").size(10.5))
                .on_hover_text(
                    "Go back to the default — your music folder, or the config directory when \
                     the system has none",
                )
                .clicked()
            {
                *reset = true;
            }
        });
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                RichText::new(
                    "Where the audio recordings, the scheduled recordings and the raw I/Q captures \
                     are written. Default: your music folder's sdroxide subfolder, or the config \
                     directory's recordings folder when the system exposes no music folder.",
                )
                .size(10.5)
                .color(crate::theme::gray(140)),
            )
            .wrap(),
        );
    }

    /// The browser client has no filesystem and no engine of its own to record
    /// with.
    #[cfg(target_arch = "wasm32")]
    pub(in crate::app) fn settings_recordings_dir(
        &self,
        _ui: &mut egui::Ui,
        _choose: &mut bool,
        _reset: &mut bool,
    ) {
    }

    /// Carry out what [`Self::settings_recordings_dir`]'s buttons asked for,
    /// after the window closure has given `&mut self` back — see [`SettingsIo`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn handle_recordings_dir(&mut self, choose: bool, reset: bool) {
        if !choose && !reset {
            return;
        }
        let saved = if choose {
            let mut dialog = rfd::FileDialog::new();
            if let Some(parent) = self.recordings_dir.parent() {
                dialog = dialog.set_directory(parent);
            }
            match dialog.pick_folder() {
                Some(picked) => sdroxide_config::save_recordings_dir(Some(&picked)),
                // A cancelled picker changes nothing.
                None => Ok(()),
            }
        } else {
            sdroxide_config::save_recordings_dir(None)
        };
        if let Err(e) = saved {
            eprintln!("failed to save the recordings directory: {e}");
        }
        // Re-read whatever is now in effect, so the row shows the truth at once.
        self.recordings_dir = crate::app::persist::recordings_dir_for_display();
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::app) fn handle_recordings_dir(&mut self, _choose: bool, _reset: bool) {}

    /// Apply a settings bundle the operator picked, once the picker thread has
    /// delivered it. Drained every frame beside the ADIF import.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn poll_settings_import(&mut self) {
        let loaded = self.settings_import_inbox.lock().ok().and_then(|mut g| g.take());
        let Some(loaded) = loaded else { return };
        // Said on screen rather than on stderr: the operator who pressed the
        // button is looking at the window, and on Windows there is no console
        // behind it to print to.
        self.settings_transfer_note = Some(match loaded {
            Err(e) => format!("Import failed: {e}"),
            Ok(loaded) => match sdroxide_config::transfer::import(&loaded.text) {
                Err(e) => format!("Import failed: {e}"),
                Ok(report) => {
                    let mut msg = format!("{} — restart sdroxide to use them.", report.summary());
                    for (path, why) in report.skipped.iter().take(4) {
                        msg.push_str(&format!("\nSkipped {path}: {why}"));
                    }
                    msg
                }
            },
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::app) fn poll_settings_import(&mut self) {}

    /// The SWR guard: arm it, and set the ratio it stops transmitting at.
    ///
    /// Reads the live values out of the broadcast TX state rather than off
    /// disk, so a remote client shows the radio's setting and not its own
    /// machine's `config.toml`, which would be a different antenna entirely.
    /// The command is sent only on an actual change, since a `DragValue` reports
    /// its value every frame it is dragged and each one would be a config write.
    pub(in crate::app) fn settings_swr_guard(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.label(RichText::new("SWR guard").strong());
        ui.end_row();
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
        super::note_rich(
            ui,
            "Stops the transmission when the radio reports an SWR at or above this figure, and \
                 keeps transmit locked out until you acknowledge it. Catches a disconnected \
                 antenna, a failed feeder, or a switch left on the wrong port.\n\n\
                 Tuning is treated differently, because feeding a mismatch is the point of it: an \
                 antenna tuner gets double the limit and about five seconds before the guard \
                 applies at all. A manual tuner that takes longer than that wants the guard \
                 switched off for the session.\n\n\
                 Needs a rig that reports SWR over CAT. Ignores the first fifth of a second of \
                 each transmission, and does not wait for high power.",
            Some(10.5),
            None,
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

    /// Forget every mode's remembered settings overrides.
    ///
    /// The **DEFAULTS** chip beside the receiver controls puts one mode back;
    /// this is the way back from a long session of fiddling without walking
    /// every mode. A command rather than a config edit, so it reaches whichever
    /// engine is running — local or remote — and undoes itself.
    pub(in crate::app) fn settings_mode_defaults(
        &self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<sdroxide_types::Command>,
    ) {
        ui.label(RichText::new("Per-mode settings").strong());
        ui.end_row();
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if crate::chrome::chip(ui, false, RichText::new("RESET EVERY MODE").size(10.5))
                .on_hover_text(
                    "Forget every mode's remembered AGC, squelch, noise reduction, notch and \
                     stereo switches, and put each mode's own defaults back.",
                )
                .clicked()
            {
                cmds.push(sdroxide_types::Command::ResetModeDefaults { mode: None });
            }
        });
        ui.add_space(4.0);
        super::note_rich(
            ui,
            "Selecting a mode lays that mode's own starting values for AGC, squelch, noise \
                 reduction, the notch and the stereo switches on the receiver, and changing one \
                 remembers it for that mode alone. The circular-arrow chip at the end of the \
                 receiver's filter/noise row lists what has been changed and puts the current \
                 mode back.",
            Some(10.5),
            None,
        );
    }
}

#[cfg(test)]
mod device_entry_tests {
    use super::device_field;
    use eframe::egui;

    const LIST: &[&str] = &["Built-in Audio", "USB Audio"];
    const UNLISTED: &str = "bluez_input.AA_BB_CC_DD_EE_FF";

    fn field_id() -> egui::Id {
        egui::Id::new("t").with("typed")
    }

    /// One frame: draw the picker, and optionally feed the entry field keys.
    fn frame(ctx: &egui::Context, selected: &mut Option<String>, keys: &[egui::Event]) {
        let names: Vec<String> = LIST.iter().map(|s| (*s).into()).collect();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(700.0, 400.0),
            )),
            events: keys.to_vec(),
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| {
            device_field(ui, "t", &names, selected, "the test device");
        });
    }

    /// One frame that only draws and claims the keyboard, so the next frame's
    /// keys go to the entry field rather than to nothing.
    fn focus(ctx: &egui::Context, selected: &mut Option<String>) {
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(700.0, 400.0),
                )),
                ..Default::default()
            },
            |ui| {
                let names: Vec<String> = LIST.iter().map(|s| (*s).into()).collect();
                device_field(ui, "t", &names, selected, "the test device");
                ui.memory_mut(|m| m.request_focus(field_id()));
            },
        );
    }

    fn text(s: &str) -> egui::Event {
        egui::Event::Text(s.to_owned())
    }

    fn backspace() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// **A name the list does not carry is still enterable.**
    ///
    /// The whole point of the text field. A sound server can offer a device the
    /// host API does not enumerate — enumerating PipeWire nodes is the open half
    /// of the Bluetooth microphone work — and a picker that can only be pointed
    /// at leaves the operator with no way to name it at all. Typed with real key
    /// events, not by poking the value: a test that sets the field itself is
    /// asserting that a struct holds what it was given.
    #[test]
    fn a_device_the_list_does_not_show_can_still_be_typed() {
        let ctx = egui::Context::default();
        let mut selected = None;
        // Focus the entry field, then type into it as a person would.
        focus(&ctx, &mut selected);
        let keys: Vec<egui::Event> = std::iter::once(text(UNLISTED)).collect();
        frame(&ctx, &mut selected, &keys);

        assert_eq!(
            selected.as_deref(),
            Some(UNLISTED),
            "a device name that is not in the list must be enterable — the list is a \
             convenience, not the truth"
        );
        assert!(
            !LIST.contains(&UNLISTED),
            "the premise of this test: {UNLISTED} is deliberately not one of {LIST:?}"
        );
    }

    /// **An emptied field means the system default**, not a device named "".
    ///
    /// `None` is how the rest of the audio code spells "system default"
    /// everywhere, so a field left holding an empty string would ask the sound
    /// server for a device called nothing.
    #[test]
    fn clearing_the_field_goes_back_to_the_system_default() {
        let ctx = egui::Context::default();
        let mut selected = Some("USB Audio".to_string());
        focus(&ctx, &mut selected);
        // One backspace per character typed so far.
        let keys: Vec<egui::Event> = (0.."USB Audio".len()).map(|_| backspace()).collect();
        frame(&ctx, &mut selected, &keys);
        assert_eq!(
            selected, None,
            "an empty entry field is the system default, not an empty device name"
        );
    }
}
