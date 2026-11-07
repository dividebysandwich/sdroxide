//! Settings → Uploads → Wavelog: the server, the certificate, the station
//! locations, and the QSOs it has not got.
//!
//! More than the other upload tabs hold, because a Wavelog is the operator's
//! own server rather than a public service: it can sit behind a certificate
//! nobody vouches for, it has station locations to choose between, and it is
//! the one service sdroxide sends a backlog to. The order on screen is the
//! order an operator sets it up in — where it is, does it answer, where QSOs
//! go, what is still owed.

use crate::i18n::tr;
use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Command, LoginTarget, UploadTarget, WavelogStation, fingerprint_display};

use super::SettingsIo;
use super::net::{net_row, net_secret};
use crate::app::SdroxideApp;
use crate::chrome::StyledCombo;

fn hint(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).size(10.5).color(crate::theme::gray(140)));
}

fn warn(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(format!("⚠ {}", text.into())).color(Color32::from_rgb(230, 170, 60)));
}

/// Whether `url` is plain HTTP to somewhere other than this machine or a
/// private network — where the API key would cross the internet in clear.
fn http_over_internet(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("http://") else { return false };
    let host = rest.split(['/', ':']).next().unwrap_or_default();
    let private = host == "localhost"
        || host.ends_with(".local")
        || host.ends_with(".lan")
        || host.ends_with(".home.arpa")
        || !host.contains('.')
        || match host.parse::<std::net::Ipv4Addr>() {
            Ok(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
            Err(_) => false,
        };
    !private
}

/// Why Wavelog will refuse QSOs logged as `call`/`grid` at `station`, if it
/// will. Mirrors the server's own check (`Logbook_model::import`): the
/// station callsign must match exactly, and the locator must be in the same
/// place — compared here on the four-character square, which is as far as
/// the two can be expected to agree.
fn mismatch(station: &WavelogStation, call: &str, grid: &str) -> Option<String> {
    let call = call.trim();
    if !call.is_empty() && !station.callsign.eq_ignore_ascii_case(call) {
        return Some(tr!(
            "settings-wavelog-call-mismatch",
            location = station.name.to_string(),
            station_call = station.callsign.to_string(),
            call = call
        ));
    }
    let (a, b) = (grid.trim().to_ascii_uppercase(), station.grid.trim().to_ascii_uppercase());
    if a.len() >= 4 && b.len() >= 4 && a[..4] != b[..4] {
        return Some(tr!(
            "settings-wavelog-grid-mismatch",
            location = station.name.to_string(),
            station_grid = station.grid.to_string(),
            grid = a
        ));
    }
    None
}

impl SdroxideApp {
    /// The Wavelog tab's body, below the shared auto-upload tick.
    pub(in crate::app) fn wavelog_settings(
        &self,
        ui: &mut egui::Ui,
        io: &mut SettingsIo,
        cmds: &mut Vec<Command>,
    ) {
        let wl = &mut io.net_edit.wavelog;
        net_row(ui, &tr!("settings-wavelog-address"), &mut wl.url, 260.0);
        net_secret(ui, &tr!("settings-wavelog-api-key"), &mut wl.api_key, 260.0);
        hint(ui, tr!("settings-wavelog-your-wavelog-s-front-page"));
        if http_over_internet(&wl.url) {
            warn(ui, tr!("settings-wavelog-plain-http-address-internet-api"));
        }

        // The pinned certificate, when there is one.
        if !wl.cert_sha256.trim().is_empty() {
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-wavelog-trusted-cert")));
                ui.label(
                    RichText::new(fingerprint_display(&wl.cert_sha256)).monospace().size(10.0),
                );
            });
            ui.horizontal(|ui| {
                ui.add_space(100.0);
                if ui
                    .small_button(tr!("settings-wavelog-forget-certificate"))
                    .on_hover_text(tr!("settings-wavelog-go-back-ordinary-checking-against"))
                    .clicked()
                {
                    wl.cert_sha256.clear();
                }
            });
        }

        ui.add_space(4.0);
        if self.ctrl.engine_is_remote() {
            hint(ui, tr!("settings-wavelog-connection-check-runs-machine-radio"));
        } else {
            self.login_test_row(ui, cmds, io.net_edit, LoginTarget::Wavelog);
        }

        let probe = self.wavelog_probe.as_ref();

        // A certificate nothing vouches for: show it, and offer to trust it.
        if let Some(fp) = probe.and_then(|p| p.untrusted_cert.as_ref()) {
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr!("settings-wavelog-server-presented-certificate-sha-256"))
                    .size(11.0),
            );
            ui.label(RichText::new(fingerprint_display(fp)).monospace().size(10.0).strong());
            hint(ui, tr!("settings-wavelog-compare-your-server-s-e"));
            if ui.button(tr!("settings-wavelog-trust-certificate")).clicked() {
                io.net_edit.wavelog.cert_sha256 = fp.clone();
                cmds.push(Command::SetNetworkConfig(io.net_edit.clone()));
                cmds.push(Command::TestLogin(LoginTarget::Wavelog));
            }
        }

        crate::app::settings::net::net_heading(ui, &tr!("settings-wavelog-station-location"));
        let stations: &[WavelogStation] = probe.map(|p| p.stations.as_slice()).unwrap_or(&[]);
        let label_of = |id: u32| -> String {
            match stations.iter().find(|s| s.id == id) {
                Some(s) => s.label(),
                None if id == 0 => tr!("settings-wavelog-none"),
                // Chosen in an earlier session; the list arrives with a test.
                None => tr!("settings-wavelog-location-n", id = id),
            }
        };
        if stations.is_empty() {
            hint(ui, tr!("settings-wavelog-press-test-above-list-station"));
        }

        ui.horizontal(|ui| {
            ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-wavelog-default")));
            let cur = &mut io.net_edit.wavelog.station_id;
            egui::ComboBox::from_id_salt("wavelog_station_default")
                .selected_text(label_of(*cur))
                .width(260.0)
                .show_styled(ui, |ui| {
                    for s in stations {
                        ui.selectable_value(cur, s.id, s.label());
                    }
                });
        })
        .response
        .on_hover_text(tr!("settings-wavelog-where-qsos-go-when-operating"));

        // The profile's own choice lives with the callsign it must match, so
        // it is edited on the identity, which a profile saves and restores.
        if io.digi_seeded {
            let call = io.digi_edit.my_call.trim().to_string();
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-wavelog-profile")));
                let cur = &mut io.digi_edit.wavelog_station_id;
                let shown =
                    if *cur == 0 { tr!("settings-wavelog-use-default") } else { label_of(*cur) };
                egui::ComboBox::from_id_salt("wavelog_station_profile")
                    .selected_text(shown)
                    .width(260.0)
                    .show_styled(ui, |ui| {
                        ui.selectable_value(cur, 0, tr!("settings-wavelog-use-default"));
                        for s in stations {
                            ui.selectable_value(cur, s.id, s.label());
                        }
                    });
            });
            hint(
                ui,
                tr!(
                    "settings-wavelog-saved-with-callsign",
                    call = if call.is_empty() {
                        tr!("settings-wavelog-not-set")
                    } else {
                        call.clone()
                    }
                ),
            );
            let effective = io.net_edit.wavelog.station_for(io.digi_edit.wavelog_station_id);
            if let Some(s) = stations.iter().find(|s| s.id == effective)
                && let Some(why) = mismatch(s, &call, &io.digi_edit.my_grid)
            {
                warn(ui, why);
            }
            if effective == 0 && io.net_edit.wavelog.is_configured() {
                warn(ui, tr!("settings-wavelog-no-station-location-chosen-uploads"));
            }
        }

        self.wavelog_download_settings(ui, io, cmds, stations);
        self.wavelog_radio_settings(ui, io);

        crate::app::settings::net::net_heading(ui, &tr!("settings-wavelog-qsos-not-wavelog"));
        let owed = self.wavelog_owed().count();
        if owed > 0 {
            ui.label(RichText::new(tr!("settings-wavelog-owed", n = owed)).size(11.0));
        }
        let unsent: Vec<_> = self.wavelog_unsent().collect();
        if unsent.is_empty() {
            hint(ui, tr!("settings-wavelog-every-qso-logbook-wavelog"));
            return;
        }
        let station = io.net_edit.wavelog.station_for(io.digi_edit.wavelog_station_id);
        let ready = io.net_edit.wavelog.is_configured() && station != 0;
        // Two clicks, because this one is not undone from here: a log sent to
        // the wrong location has to be cleaned up in Wavelog.
        let armed_id = ui.make_persistent_id("wavelog_backlog_armed");
        let armed = ui.data(|d| d.get_temp::<bool>(armed_id)).unwrap_or(false);
        if !armed {
            let btn = ui.add_enabled(
                ready,
                egui::Button::new(tr!("settings-wavelog-send-unsent", n = unsent.len())),
            );
            if btn
                .on_hover_text(tr!("settings-wavelog-everything-logbook-wavelog-has-not"))
                .on_disabled_hover_text(tr!("settings-wavelog-set-address-key-station-location"))
                .clicked()
            {
                ui.data_mut(|d| d.insert_temp(armed_id, true));
            }
            return;
        }
        warn(
            ui,
            tr!("settings-wavelog-send-confirm", n = unsent.len(), location = label_of(station)),
        );
        ui.horizontal(|ui| {
            if ui.button(tr!("settings-wavelog-yes-send-them")).clicked() {
                // Applied first, so the engine uploads with what is on screen.
                cmds.push(Command::SetNetworkConfig(io.net_edit.clone()));
                for q in &unsent {
                    cmds.push(Command::UploadQso {
                        qso_id: q.id,
                        adif: sdroxide_types::qso_log_to_adif(std::slice::from_ref(*q)),
                        targets: vec![UploadTarget::Wavelog],
                    });
                }
                ui.data_mut(|d| d.insert_temp(armed_id, false));
            }
            if ui.button(tr!("settings-wavelog-cancel")).clicked() {
                ui.data_mut(|d| d.insert_temp(armed_id, false));
            }
        });
    }
}

impl SdroxideApp {
    /// Which locations to download, how often, and the full resync.
    fn wavelog_download_settings(
        &self,
        ui: &mut egui::Ui,
        io: &mut SettingsIo,
        cmds: &mut Vec<Command>,
        stations: &[WavelogStation],
    ) {
        crate::app::settings::net::net_heading(ui, &tr!("settings-wavelog-download-into-logbook"));
        if self.ctrl.engine_is_remote() {
            hint(ui, tr!("settings-wavelog-downloading-runs-where-radio-attached"));
            return;
        }
        let wl = &mut io.net_edit.wavelog;
        ui.horizontal(|ui| {
            crate::chrome::checkbox(
                ui,
                &mut wl.download,
                tr!("settings-wavelog-download-qsos-start-every"),
            );
            ui.add(egui::DragValue::new(&mut wl.sync_interval_min).range(5..=1440).suffix(" min"));
        });
        hint(ui, tr!("settings-wavelog-downloaded-qsos-join-logbook-so"));

        // The locations to read: Wavelog's list once a test has fetched it,
        // otherwise whatever is ticked already, so it can still be unticked.
        let mut listed: Vec<(u32, String)> = stations.iter().map(|s| (s.id, s.label())).collect();
        for id in &wl.download_station_ids {
            if !listed.iter().any(|(l, _)| l == id) {
                listed.push((*id, tr!("settings-wavelog-location-n", id = id)));
            }
        }
        if listed.is_empty() {
            hint(ui, tr!("settings-wavelog-press-test-above-choose-which"));
        }
        for (id, label) in listed {
            let mut on = wl.download_station_ids.contains(&id);
            if crate::chrome::checkbox(ui, &mut on, label).changed() {
                wl.download_station_ids.retain(|x| *x != id);
                if on {
                    wl.download_station_ids.push(id);
                    wl.download_station_ids.sort_unstable();
                }
            }
        }

        let running = self.wavelog_sync_running.is_some();
        let ready = wl.is_configured() && !wl.download_station_ids.is_empty();
        ui.horizontal(|ui| {
            let now = ui.add_enabled(
                ready && !running,
                egui::Button::new(tr!("settings-wavelog-sync-now")),
            );
            if now.on_hover_text(tr!("settings-wavelog-fetch-what-wavelog-has-added")).clicked() {
                cmds.push(Command::SetNetworkConfig(io.net_edit.clone()));
                self.wavelog_sync_request.set(Some(false));
            }
            let full = ui.add_enabled(
                ready && !running && self.wavelog_resync.is_none(),
                egui::Button::new(tr!("settings-wavelog-full-resync")),
            );
            if full
                .on_hover_text(tr!("settings-wavelog-download-ticked-locations-whole-compare"))
                .clicked()
            {
                cmds.push(Command::SetNetworkConfig(io.net_edit.clone()));
                self.wavelog_sync_request.set(Some(true));
            }
            if running {
                ui.spinner();
            }
        });
        if let Some(status) = &self.wavelog_sync_status {
            ui.label(RichText::new(status).size(11.0));
        }

        if let Some((plan, _)) = &self.wavelog_resync {
            ui.add_space(4.0);
            ui.group(|ui| {
                ui.label(RichText::new(tr!("settings-wavelog-full-resync-changes-make")).strong());
                ui.label(tr!("settings-wavelog-plan-add", n = plan.add.len()));
                ui.label(tr!("settings-wavelog-plan-update", n = plan.update.len()));
                ui.label(tr!("settings-wavelog-plan-remove", n = plan.remove.len()));
                ui.label(tr!("settings-wavelog-plan-mark", n = plan.mark.len()));
                hint(ui, tr!("settings-wavelog-qsos-logged-here-imported-file"));
                ui.horizontal(|ui| {
                    if ui.button(tr!("settings-wavelog-apply")).clicked() {
                        self.wavelog_resync_answer.set(Some(true));
                    }
                    if ui.button(tr!("settings-wavelog-discard")).clicked() {
                        self.wavelog_resync_answer.set(Some(false));
                    }
                });
            });
        }
    }

    /// The live radio status Wavelog's logging form and bandmap follow.
    fn wavelog_radio_settings(&self, ui: &mut egui::Ui, io: &mut SettingsIo) {
        crate::app::settings::net::net_heading(ui, &tr!("settings-wavelog-live-radio-status"));
        let wl = &mut io.net_edit.wavelog;
        crate::chrome::checkbox(
            ui,
            &mut wl.radio_status,
            tr!("settings-wavelog-send-frequency-mode-wavelog-they"),
        );
        net_row(ui, &tr!("settings-wavelog-radio-name"), &mut wl.radio_name, 160.0);
        hint(ui, tr!("settings-wavelog-pick-radio-wavelog-s-live"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_http_is_only_flagged_when_it_leaves_the_house() {
        assert!(!http_over_internet("http://192.168.1.10/wavelog"));
        assert!(!http_over_internet("http://10.0.0.5:8086"));
        assert!(!http_over_internet("http://localhost/wavelog"));
        assert!(!http_over_internet("http://wavelog.local"));
        assert!(!http_over_internet("http://nas/wavelog"));
        assert!(!http_over_internet("https://log.example.org"));
        assert!(http_over_internet("http://log.example.org"));
        assert!(http_over_internet("http://203.0.113.7/wavelog"));
    }

    fn home() -> WavelogStation {
        WavelogStation {
            id: 1,
            name: "Home".into(),
            callsign: "EA5ABC".into(),
            grid: "IM99TL".into(),
            active: true,
        }
    }

    #[test]
    fn a_matching_identity_raises_no_warning() {
        assert_eq!(mismatch(&home(), "ea5abc", "IM99"), None);
        assert_eq!(mismatch(&home(), "EA5ABC", ""), None);
    }

    #[test]
    fn a_portable_call_at_the_home_location_is_warned_about() {
        assert!(mismatch(&home(), "EA5ABC/P", "IM99").unwrap().contains("EA5ABC/P"));
    }

    #[test]
    fn a_grid_somewhere_else_is_warned_about() {
        assert!(mismatch(&home(), "EA5ABC", "IN80").unwrap().contains("IN80"));
    }
}
