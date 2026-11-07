//! The Spots, Uploads and FreeDV tabs — the network cockpit's settings.
//!
//! All three edit one [`NetworkConfig`], which the engine persists when it
//! applies it, so each has an APPLY button rather than saving as you type:
//! these are credentials and connection details, and half-typed ones would
//! reconnect the feeds on every keystroke.

use crate::i18n::tr;
use eframe::egui::{self, Color32, RichText};

/// A dropdown over an enum's `ALL`, using its `label()`.
/// UI / display preferences: frame rate, waterfall scroll speed, spectrum speed.
/// Section heading for the Spots / Uploads settings tabs.
pub(in crate::app) fn net_heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).size(12.0).strong().color(crate::theme::CYAN()));
}

/// A labelled single-line text field for the network settings tabs.
pub(in crate::app) fn net_row(ui: &mut egui::Ui, label: &str, val: &mut String, w: f32) {
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 22.0], egui::Label::new(label));
        crate::chrome::field(ui, egui::TextEdit::singleline(val).desired_width(w));
    });
}

/// A labelled password field (masked) for the network settings tabs.
pub(in crate::app) fn net_secret(ui: &mut egui::Ui, label: &str, val: &mut String, w: f32) {
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 22.0], egui::Label::new(label));
        crate::chrome::field(ui, egui::TextEdit::singleline(val).password(true).desired_width(w));
    });
}

/// Where the operator's callsign and grid actually live, for the network tabs
/// that report under them but deliberately do not offer a second copy to edit.
pub(in crate::app) fn operator_identity_note(
    ui: &mut egui::Ui,
    digi: &sdroxide_types::DigiConfig,
    seeded: bool,
) {
    net_heading(ui, &tr!("settings-net-operator"));
    if !seeded {
        ui.label(RichText::new(tr!("settings-net-callsign-grid-are-set-general")).weak());
        return;
    }
    let (call, grid) = (digi.my_call.trim(), digi.my_grid.trim());
    if call.is_empty() || grid.is_empty() {
        ui.label(
            RichText::new(tr!("settings-net-set-your-callsign-grid-general"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
    } else {
        ui.label(
            RichText::new(tr!("settings-net-call-grid-set-on-general", call = call, grid = grid))
                .weak(),
        );
    }
}

/// FreeDV Reporter (<https://qso.freedv.org/>): announce this station and show
/// everyone else's as spots.
///
/// `call`/`grid` are the operator identity from the General tab, shown here but
/// not editable: reporting under a second copy would only let the two disagree.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_freedv_tab(
    ui: &mut egui::Ui,
    net: &mut sdroxide_types::NetworkConfig,
    call: &str,
    grid: &str,
    digi_seeded: bool,
    status: &Option<String>,
    apply: &mut bool,
) {
    ui.label(
        RichText::new(tr!("settings-net-freedv-reporter"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut net.freedv_reporter.enabled, tr!("settings-net-enable"))
        .on_hover_text(tr!("settings-net-connects-whenever-enabled-your-station"));
    ui.add_space(6.0);

    let enabled = net.freedv_reporter.enabled;

    ui.add_enabled_ui(enabled, |ui| {
        let c = &mut net.freedv_reporter;

        net_heading(ui, &tr!("settings-net-station"));
        net_row(ui, &tr!("settings-net-message"), &mut c.message, 260.0);
        crate::chrome::checkbox(
            ui,
            &mut c.rx_only,
            tr!("settings-net-receive-only-i-cannot-transmit"),
        );

        net_heading(ui, &tr!("settings-net-server"));
        net_row(ui, &tr!("settings-net-host"), &mut c.host, 220.0);
        ui.horizontal(|ui| {
            ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-net-port")));
            ui.add(egui::DragValue::new(&mut c.port).range(1..=65535));
        });
        ui.add_enabled_ui(false, |ui| {
            crate::chrome::checkbox(ui, &mut c.tls, tr!("settings-net-tls-wss"))
                .on_hover_text(tr!("settings-net-not-yet-implemented-freedv-gui"));
        });

        net_heading(ui, &tr!("settings-net-reporting"));
        crate::chrome::checkbox(ui, &mut c.report_rx, tr!("settings-net-report-stations-i-decode"))
            .on_hover_text(tr!("settings-net-sends-rx-report-each-callsign"));
        crate::chrome::checkbox(
            ui,
            &mut c.show_spots,
            tr!("settings-net-show-other-reporter-stations-spots"),
        )
        .on_hover_text(tr!("settings-net-adds-them-panadapter-overlay-world"));
    });

    ui.add_space(8.0);
    if enabled && digi_seeded && (call.is_empty() || grid.is_empty()) {
        ui.label(
            RichText::new(tr!("settings-net-set-your-callsign-grid-general-2"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
    } else if enabled && digi_seeded {
        ui.label(
            RichText::new(tr!(
                "settings-net-reporting-as",
                call = call,
                grid = grid,
                version = env!("CARGO_PKG_VERSION")
            ))
            .weak(),
        );
        ui.label(RichText::new(tr!("settings-net-callsign-grid-are-set-general")).weak());
    }
    // The status line is shared by every network feed, so only show it here
    // when it is actually ours.
    if let Some(s) = status {
        if s.starts_with("FreeDV Reporter") {
            ui.label(RichText::new(s).weak());
        }
    }

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(tr!("settings-net-apply")).strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text(tr!("settings-net-persist-re-connect"))
    .clicked()
    {
        *apply = true;
    }
}

/// The broadcast-station block on the Spots settings tab: where the schedule
/// came from, and the three things that can be done to it.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn broadcast_stations_settings(
    ui: &mut egui::Ui,
    reload: &mut bool,
    refetch: &mut bool,
    fetching: bool,
    status: Option<&Result<String, String>>,
) {
    let (season, cached) = sdroxide_config::broadcast_schedule_status();
    ui.label(
        RichText::new(tr!("settings-net-longwave-shortwave-stations-labelled-waterfall")).weak(),
    );
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(tr!("settings-net-season")).size(11.0).color(crate::theme::gray(150)),
        );
        ui.label(
            RichText::new(season.to_uppercase())
                .monospace()
                .size(11.0)
                .color(crate::theme::TEXT_STRONG()),
        );
        let (text, colour) = if fetching {
            (tr!("settings-net-downloading"), crate::theme::YELLOW())
        } else if cached {
            ("downloaded".to_string(), crate::theme::GREEN())
        } else {
            (tr!("settings-net-using-built-copy"), crate::theme::gray(150))
        };
        ui.label(RichText::new(text).size(11.0).color(colour));
    });
    if let Some(Err(e)) = status {
        // Worth showing, not worth alarming over: the built-in schedule is still
        // in use, so the only thing lost is freshness.
        ui.label(
            RichText::new(tr!("settings-net-schedule-download-failed", error = e.to_string()))
                .color(crate::theme::YELLOW())
                .size(11.0),
        );
    }

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-net-your-own-stations-corrections-go")).weak());
    if let Ok(p) = sdroxide_config::broadcast_stations_path() {
        ui.add(
            egui::Label::new(
                RichText::new(p.display().to_string())
                    .monospace()
                    .size(10.5)
                    .color(crate::theme::gray(150)),
            )
            .truncate(),
        );
    }
    ui.horizontal(|ui| {
        if ui
            .button(tr!("settings-net-reload"))
            .on_hover_text(tr!("settings-net-re-read-your-station-file"))
            .clicked()
        {
            *reload = true;
        }
        if ui
            .add_enabled(!fetching, egui::Button::new(tr!("settings-net-download-schedule-now")))
            .on_hover_text(tr!("settings-net-fetch-season-s-schedule-again"))
            .clicked()
        {
            *refetch = true;
        }
    });
}

/// The browser client reads the schedule compiled into the wasm bundle, so there
/// is nothing to download and no file to overlay.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn broadcast_stations_settings(
    ui: &mut egui::Ui,
    _reload: &mut bool,
    _refetch: &mut bool,
    _fetching: bool,
    _status: Option<&Result<String, String>>,
) {
    ui.label(RichText::new(tr!("settings-net-broadcast-stations-labelled-waterfall-come")).weak());
}

impl crate::app::SdroxideApp {
    /// One "Test" button and its answer, for a logging service's credentials.
    ///
    /// The point of it: today the first sign that a password is wrong is a QSO
    /// that failed to upload, hours after the contact. This asks the service
    /// directly, before anything depends on the answer.
    ///
    /// ⛔ It sends the EDITED config first. The engine tests what it has
    /// applied, so without that a freshly pasted key would be checked against
    /// the old one, and the operator would be told their new credentials were
    /// wrong when they had simply not been saved yet. Pressing Test therefore
    /// also applies the network settings, which is what someone pressing it
    /// means, and the label says so.
    ///
    /// Nothing is cached between sessions: a green tick from an hour ago says
    /// nothing about the password typed since.
    pub(in crate::app) fn login_test_row(
        &self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<sdroxide_types::Command>,
        edited: &sdroxide_types::NetworkConfig,
        target: sdroxide_types::LoginTarget,
    ) {
        ui.horizontal(|ui| {
            let pending = self.login_tests_pending.contains(&target);
            let btn = ui.add_enabled(
                !pending,
                egui::Button::new(
                    RichText::new(tr!("settings-net-test-service", service = target.label()))
                        .size(11.0),
                ),
            );
            if btn
                .on_hover_text(tr!("settings-net-ask-service-whether-these-credentials"))
                .clicked()
            {
                cmds.push(sdroxide_types::Command::SetNetworkConfig(edited.clone()));
                cmds.push(sdroxide_types::Command::TestLogin(target));
            }
            if pending {
                ui.label(RichText::new(tr!("settings-net-checking")).size(11.0).weak());
            } else if let Some(r) = self.login_tests.get(&target) {
                let (mark, colour) = if r.ok {
                    ("✔", crate::theme::GREEN())
                } else {
                    ("✖", Color32::from_rgb(255, 120, 120))
                };
                ui.label(RichText::new(format!("{mark} {}", r.message)).size(11.0).color(colour));
            }
        });
    }
}
