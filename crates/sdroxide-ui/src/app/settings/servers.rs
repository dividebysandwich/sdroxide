//! The Servers tab: the three interfaces this app offers to other software.
//!
//! Hamlib rigctld and the TCI server are control surfaces third-party clients
//! connect to; the WSJT-X UDP broadcast is one-way, feeding decodes, status
//! and logged QSOs to GridTracker, JTAlert, N1MM+ and Log4OM. All three are
//! bound by the engine, which is also what persists their configuration and
//! reports back whether the bind succeeded.
//!
//! That makes these three settings the *station's*, not the screen's: the
//! engine announces what it has, and this tab edits that. `seeded` is whether
//! the announcement has arrived yet — a client that has not been told cannot
//! be allowed to apply defaults over the real thing.

use crate::chrome::StyledCombo;
use crate::i18n::{Tr, tr};
use eframe::egui::{self, Color32, ComboBox, RichText};

/// Live status of the built-in TCI server, from `RadioEvent::TciServerStatus`.
#[derive(Clone, PartialEq)]
pub(in crate::app) struct TciServerStatus {
    pub(in crate::app) running: bool,
    pub(in crate::app) addr: String,
    pub(in crate::app) clients: usize,
    pub(in crate::app) error: Option<String>,
    /// Outbound TCI client socket. The rigctld tab reuses this struct and
    /// leaves both of these at rest.
    pub(in crate::app) client_connected: bool,
    pub(in crate::app) client_error: Option<String>,
}

pub(in crate::app) fn settings_wsjtx_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::WsjtxConfig,
    seeded: bool,
    apply: &mut bool,
) {
    ui.label(
        RichText::new(tr!("settings-servers-wsjt-x-udp-broadcast"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    if !seeded {
        ui.label(RichText::new(tr!("settings-servers-waiting-station-s-wsjt-x")).weak());
        return;
    }
    ui.label(RichText::new(tr!("settings-servers-sends-decodes-station-status-logged")).weak());
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, tr!("settings-servers-enable"));
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("wsjtx-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-servers-send"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.host)
                    .desired_width(160.0)
                    .hint_text("127.0.0.1"),
            )
            .on_hover_text(tr!("settings-servers-t-127-0-0-1-reaches"));
            ui.end_row();

            ui.label(tr!("settings-servers-port"));
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535))
                .on_hover_text(tr!("settings-servers-t-2237-port-every-client-defaults"));
            ui.end_row();

            ui.label(tr!("settings-servers-identify"));
            crate::chrome::field(ui, egui::TextEdit::singleline(&mut cfg.id).desired_width(160.0))
                .on_hover_text(tr!("settings-servers-name-clients-see-some-loggers"));
            ui.end_row();
        });
    });

    // ── N1MM+, the other dialect ──
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(
        RichText::new(tr!("settings-servers-n1mm-contactinfo-broadcast"))
            .size(13.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-servers-same-news-n1mm-s-words")).weak());
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.n1mm.enabled, tr!("settings-servers-enable"));
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.n1mm.enabled, |ui| {
        egui::Grid::new("n1mm-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-servers-send"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.n1mm.host)
                    .desired_width(160.0)
                    .hint_text("127.0.0.1"),
            )
            .on_hover_text(tr!("settings-servers-t-127-0-0-1-reaches-2"));
            ui.end_row();

            ui.label(tr!("settings-servers-port"));
            ui.add(egui::DragValue::new(&mut cfg.n1mm.port).range(1..=65535))
                .on_hover_text(tr!("settings-servers-t-12060-port-n1mm-s-documentation"));
            ui.end_row();

            ui.label(tr!("settings-servers-station-name"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.n1mm.station).desired_width(160.0),
            )
            .on_hover_text(tr!("settings-servers-what-n1mm-calls-stationname-name"));
            ui.end_row();
        });
    });

    // ── Decoded CW, as plain text ──
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(
        RichText::new(tr!("settings-servers-decoded-cw-over-udp"))
            .size(13.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-servers-cw-panel-s-copy-plain")).weak());
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.cw_text.enabled, tr!("settings-servers-enable"));
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.cw_text.enabled, |ui| {
        egui::Grid::new("cw-text-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-servers-send"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.cw_text.host)
                    .desired_width(160.0)
                    .hint_text("127.0.0.1"),
            )
            .on_hover_text(tr!("settings-servers-t-127-0-0-1-reaches-3"));
            ui.end_row();

            ui.label(tr!("settings-servers-port"));
            ui.add(egui::DragValue::new(&mut cfg.cw_text.port).range(1..=65535))
                .on_hover_text(tr!("settings-servers-t-9999-default"));
            ui.end_row();
        });
    });

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(tr!("settings-servers-apply")).strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text(tr!("settings-servers-persist-re-open-broadcast-sockets"))
    .clicked()
    {
        *apply = true;
    }
}

pub(in crate::app) fn settings_rigctld_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::RigctldConfig,
    seeded: bool,
    status: &Option<TciServerStatus>,
    apply: &mut bool,
) {
    use sdroxide_types::RigctldConfig;

    ui.label(
        RichText::new(tr!("settings-servers-hamlib-rigctld-server"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    if !seeded {
        ui.label(
            RichText::new(tr!("settings-servers-waiting-station-s-rigctld-configuration")).weak(),
        );
        return;
    }
    ui.add_space(2.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, tr!("settings-servers-enable"));
    ui.add_space(6.0);

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("rigctld-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-servers-listen"));
            ComboBox::from_id_salt("rigctld_bind").selected_text(&cfg.bind).show_styled(ui, |ui| {
                for b in RigctldConfig::BINDS {
                    let label = if b == "127.0.0.1" {
                        tr!("settings-servers-t-127-0-0-1-machine")
                    } else {
                        tr!("settings-servers-t-0-0-0-0-whole")
                    };
                    if ui.selectable_label(cfg.bind == b, label).clicked() {
                        cfg.bind = b.to_string();
                    }
                }
            });
            ui.end_row();

            ui.label(tr!("settings-servers-port"));
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535))
                .on_hover_text(tr!("settings-servers-t-4532-port-every-rigctld-client"));
            ui.end_row();

            ui.label(tr!("settings-servers-rig-name"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.rig_name)
                    .desired_width(160.0)
                    .hint_text(tr!("settings-servers-reported-clients")),
            );
            ui.end_row();

            ui.label(tr!("settings-servers-max-clients"));
            ui.add(egui::DragValue::new(&mut cfg.max_clients).range(1..=32));
            ui.end_row();
        });
        ui.add_space(4.0);
        crate::chrome::checkbox(
            ui,
            &mut cfg.allow_tx,
            tr!("settings-servers-allow-clients-transmit"),
        )
        .on_hover_text(tr!("settings-servers-off-refuses-every-key-request"));
    });

    // Same hazard as TCI: the protocol has no authentication at all.
    if cfg.enabled && cfg.is_open_to_network() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr!("settings-servers-rigctld-has-no-authentication-0"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
    }

    ui.add_space(8.0);
    match status {
        Some(s) if s.running => {
            let clients = match s.clients {
                1 => tr!("settings-servers-t-1-client").to_string(),
                n => tr!("settings-servers-n-clients", n = n),
            };
            ui.label(
                RichText::new(tr!(
                    "settings-servers-listening",
                    addr = s.addr.to_string(),
                    clients = clients
                ))
                .color(Color32::from_rgb(90, 200, 110)),
            );
        }
        Some(s) => match &s.error {
            Some(e) => {
                ui.label(
                    RichText::new(tr!("settings-servers-failed", error = e.to_string()))
                        .color(Color32::from_rgb(230, 90, 80)),
                );
            }
            None => {
                ui.label(RichText::new(tr!("settings-servers-not-running")).weak());
            }
        },
        None => {
            ui.label(RichText::new(tr!("settings-servers-status-unknown-press-apply")).weak());
        }
    }

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(tr!("settings-servers-apply")).strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text(tr!("settings-servers-persist-re-bind-server"))
    .clicked()
    {
        *apply = true;
    }

    ui.add_space(8.0);
    ui.label(RichText::new(tr!("settings-servers-lets-any-hamlib-capable-program")).weak());
}

/// The rotctld *client* — the one interface on this tab where sdroxide dials
/// out instead of listening. Driven by the satellite lock's az/el.
pub(in crate::app) fn settings_rotator_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::RotatorConfig,
    seeded: bool,
    status: &Option<(bool, f64, f64, Option<String>)>,
    apply: &mut bool,
) {
    ui.label(
        RichText::new(tr!("settings-servers-antenna-rotator"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    if !seeded {
        ui.label(
            RichText::new(tr!("settings-servers-waiting-station-s-rotator-configuration")).weak(),
        );
        return;
    }
    ui.label(RichText::new(tr!("settings-servers-points-motorized-antenna-satellite-you")).weak());
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, tr!("settings-servers-enable"));
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("rotator-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-servers-program"));
            let was = cfg.protocol;
            super::enum_combo(
                ui,
                "rotator-protocol",
                &mut cfg.protocol,
                &sdroxide_types::RotatorProtocol::ALL,
                sdroxide_types::RotatorProtocol::tr,
            );
            // A port still on the old program's default follows to the new one's.
            if cfg.protocol != was && cfg.port == was.default_port() {
                cfg.port = cfg.protocol.default_port();
            }
            ui.end_row();

            ui.label(tr!("settings-servers-host"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.host)
                    .desired_width(160.0)
                    .hint_text("127.0.0.1"),
            );
            ui.end_row();

            ui.label(tr!("settings-servers-port"));
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535))
                .on_hover_text(tr!("settings-servers-t-4533-rotctld-s-default-pstrotator"));
            ui.end_row();

            ui.label(tr!("settings-servers-min-elevation"));
            ui.add(egui::DragValue::new(&mut cfg.min_el_deg).range(0.0..=45.0).suffix("°"))
                .on_hover_text(tr!("settings-servers-below-rotator-parks-instead-tracking"));
            ui.end_row();

            ui.label(tr!("settings-servers-azimuth-offset"));
            ui.add(egui::DragValue::new(&mut cfg.az_offset_deg).range(-180.0..=180.0).suffix("°"))
                .on_hover_text(tr!("settings-servers-added-every-command-rotator-whose"));
            ui.end_row();

            ui.label(tr!("settings-servers-min-movement"));
            ui.add(egui::DragValue::new(&mut cfg.min_move_deg).range(0.1..=10.0).suffix("°"))
                .on_hover_text(tr!("settings-servers-steps-smaller-than-are-not"));
            ui.end_row();

            ui.label(tr!("settings-servers-park-position"));
            let mut has_park = cfg.park.is_some();
            if ui
                .checkbox(&mut has_park, tr!("settings-servers-drive-position-when-idle"))
                .on_hover_text(tr!("settings-servers-off-leaves-antenna-wherever-last"))
                .changed()
            {
                cfg.park = has_park.then_some((0.0, 0.0));
            }
            ui.end_row();

            if let Some((az, el)) = cfg.park.as_mut() {
                ui.label("");
                ui.horizontal(|ui| {
                    ui.label("az");
                    ui.add(egui::DragValue::new(az).range(0.0..=360.0).suffix("°"));
                    ui.label("el");
                    ui.add(egui::DragValue::new(el).range(0.0..=90.0).suffix("°"));
                });
                ui.end_row();
            }
        });
    });

    ui.add_space(8.0);
    match status {
        Some((true, az, el, _)) => {
            ui.label(
                RichText::new(tr!(
                    "settings-servers-rotator-at",
                    az = format!("{az:.0}"),
                    el = format!("{el:.0}")
                ))
                .color(Color32::from_rgb(90, 200, 110)),
            );
        }
        Some((false, _, _, Some(e))) => {
            ui.label(
                RichText::new(tr!("settings-servers-failed", error = e.to_string()))
                    .color(Color32::from_rgb(230, 90, 80)),
            );
        }
        Some((false, _, _, None)) => {
            ui.label(RichText::new(tr!("settings-servers-not-connected")).weak());
        }
        None if cfg.enabled => {
            ui.label(RichText::new(tr!("settings-servers-status-unknown-press-apply")).weak());
        }
        None => {}
    }

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(tr!("settings-servers-apply")).strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text(tr!("settings-servers-persist-re-connect-daemon"))
    .clicked()
    {
        *apply = true;
    }
}

pub(in crate::app) fn settings_tci_server_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::TciServerConfig,
    seeded: bool,
    status: &Option<TciServerStatus>,
    apply: &mut bool,
) {
    use sdroxide_types::TciServerConfig;

    if !seeded {
        ui.label(RichText::new(tr!("settings-servers-waiting-station-s-tci-server")).weak());
        return;
    }

    ui.label(
        RichText::new(tr!("settings-servers-built-tci-server"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, tr!("settings-servers-enable"));
    ui.add_space(6.0);

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("tci-srv-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-servers-listen"));
            ComboBox::from_id_salt("tci_srv_bind").selected_text(&cfg.bind).show_styled(ui, |ui| {
                for b in TciServerConfig::BINDS {
                    let label = if b == "127.0.0.1" {
                        tr!("settings-servers-t-127-0-0-1-machine")
                    } else {
                        tr!("settings-servers-t-0-0-0-0-whole")
                    };
                    if ui.selectable_label(cfg.bind == b, label).clicked() {
                        cfg.bind = b.to_string();
                    }
                }
            });
            ui.end_row();

            ui.label(tr!("settings-servers-port"));
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535));
            ui.end_row();

            ui.label(tr!("settings-servers-device-name"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.device_name)
                    .desired_width(160.0)
                    .hint_text(tr!("settings-servers-reported-clients")),
            );
            ui.end_row();

            ui.label(tr!("settings-servers-max-clients"));
            ui.add(egui::DragValue::new(&mut cfg.max_clients).range(1..=32));
            ui.end_row();
        });
        ui.add_space(4.0);
        crate::chrome::checkbox(
            ui,
            &mut cfg.allow_tx,
            tr!("settings-servers-allow-clients-transmit"),
        )
        .on_hover_text(tr!("settings-servers-off-leaves-control-receive-streams"));
    });

    // Security: TCI has no authentication at all, so binding wide open hands
    // the transmitter to anyone who can reach the port.
    if cfg.enabled && cfg.is_open_to_network() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr!("settings-servers-tci-has-no-authentication-0"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
    }

    ui.add_space(8.0);
    match status {
        Some(s) if s.running => {
            let clients = match s.clients {
                1 => tr!("settings-servers-t-1-client").to_string(),
                n => tr!("settings-servers-n-clients", n = n),
            };
            ui.label(
                RichText::new(tr!(
                    "settings-servers-listening",
                    addr = s.addr.to_string(),
                    clients = clients
                ))
                .color(Color32::from_rgb(90, 200, 110)),
            );
        }
        Some(s) => match &s.error {
            Some(e) => {
                let msg = RichText::new(tr!("settings-servers-failed", error = e.to_string()));
                ui.label(msg.color(Color32::from_rgb(230, 90, 80)));
            }
            None => {
                ui.label(RichText::new(tr!("settings-servers-not-running")).weak());
            }
        },
        None => {
            ui.label(RichText::new(tr!("settings-servers-status-unknown-press-apply")).weak());
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        RichText::new(tr!("settings-servers-tci-client"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.client, tr!("settings-servers-tci-client"))
        .on_hover_text(tr!("settings-servers-when-you-tune-send-radio"));
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.client, |ui| {
        egui::Grid::new("tci-client-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-servers-server"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.client_address)
                    .desired_width(180.0)
                    .hint_text("127.0.0.1:50001"),
            );
            ui.end_row();
        });
    });
    if cfg.client && cfg.client_reaches_this_server() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr!("settings-servers-same-address-server-above-client"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
    }
    if cfg.client {
        ui.add_space(6.0);
        match status {
            Some(s) if s.client_connected => {
                ui.label(
                    RichText::new(tr!("settings-servers-tci-client-connected"))
                        .color(Color32::from_rgb(90, 200, 110)),
                );
            }
            Some(s) => match &s.client_error {
                Some(e) => {
                    ui.label(
                        RichText::new(tr!(
                            "settings-servers-tci-client-error",
                            error = e.to_string()
                        ))
                        .color(Color32::from_rgb(230, 90, 80)),
                    );
                }
                None => {
                    ui.label(RichText::new(tr!("settings-servers-tci-client-connecting")).weak());
                }
            },
            None => {
                ui.label(RichText::new(tr!("settings-servers-tci-client-press-apply")).weak());
            }
        }
    }

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(tr!("settings-servers-apply")).strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text(tr!("settings-servers-persist-re-bind-server-connect"))
    .clicked()
    {
        *apply = true;
    }

    ui.add_space(8.0);
    ui.label(RichText::new(tr!("settings-servers-server-lets-tci-capable-programs")).weak());
}
