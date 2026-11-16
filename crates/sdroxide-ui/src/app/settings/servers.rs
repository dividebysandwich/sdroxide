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

/// One radio of this station as the TCI server section's picker shows it.
///
/// The server is per radio — each engine binds its own, from its own
/// `tciserver.json` — so the page edits the server of the radio whose dialog
/// it is, and the picker moves the dialog to another radio rather than
/// editing that radio's file from here.
pub(in crate::app) struct TciRadio {
    pub(in crate::app) id: u32,
    pub(in crate::app) name: String,
    pub(in crate::app) tci: Option<crate::app::TciBrief>,
    /// The radio whose server this page edits.
    pub(in crate::app) own: bool,
}

/// The other radios whose TCI server is switched on at `port`. Every radio
/// counts, switched off or not: its engine runs either way and holds the port.
pub(in crate::app) fn tci_port_holders(port: u16, radios: &[TciRadio]) -> Vec<&str> {
    radios
        .iter()
        .filter(|r| !r.own && r.tci.is_some_and(|b| b.enabled && b.port == port))
        .map(|r| r.name.as_str())
        .collect()
}

/// The lowest port from [`sdroxide_types::TciServerConfig::BASE_PORT`] up that
/// no other radio's server is switched on at.
pub(in crate::app) fn first_free_tci_port(radios: &[TciRadio]) -> u16 {
    let mut port = sdroxide_types::TciServerConfig::BASE_PORT;
    while port < u16::MAX && !tci_port_holders(port, radios).is_empty() {
        port += 1;
    }
    port
}

/// A radio's picker chip: its name behind a mark for its server's state
/// (● on, ○ off, ⚠ bind failed), plain while its engine has not announced
/// one. The name stays the radio's name.
// Why: HDZ 2026-11-15: "der Status aus ist in den Namen gerutscht" — "RX-888 · aus"
//      read as if the radio were called that; the port moved to the tooltip.
fn tci_chip_text(r: &TciRadio) -> String {
    match r.tci {
        Some(b) if b.enabled && b.failed => tr!("settings-servers-tci-radio-failed", name = r.name),
        Some(b) if b.enabled => tr!("settings-servers-tci-radio-on", name = r.name),
        Some(_) => tr!("settings-servers-tci-radio-off", name = r.name),
        None => r.name.clone(),
    }
}

/// First tooltip line of a radio's chip: its server's state with the port.
fn tci_chip_state(r: &TciRadio) -> Option<String> {
    let b = r.tci?;
    Some(if b.enabled && b.failed {
        tr!("settings-servers-tci-radio-state-failed", port = b.port.to_string())
    } else if b.enabled {
        tr!("settings-servers-tci-radio-state-on", port = b.port.to_string())
    } else {
        tr!("settings-servers-tci-radio-state-off")
    })
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

#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_tci_server_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::TciServerConfig,
    seeded: bool,
    applied: Option<&sdroxide_types::TciServerConfig>,
    status: &Option<TciServerStatus>,
    radios: &[TciRadio],
    apply: &mut bool,
    requests: &mut Vec<crate::app::RadioTabRequest>,
) {
    use sdroxide_types::TciServerConfig;

    // Which radio's server this is, with more than one to choose from. Each
    // radio binds its own, so this is the radio strip of Settings → Radio
    // again, each chip carrying its port: clicking one moves the dialog to
    // that radio, on this page.
    // Why: this section showed one radio's server with nothing to say which,
    //      and the others were only reached by switching tabs first. HDZ
    //      2026-11-14: "nicht so kompliziert" — a picker, no overview table.
    if radios.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr!("settings-servers-tci-radio")).strong());
            for r in radios {
                let mut text = RichText::new(tci_chip_text(r)).size(12.5);
                if r.own {
                    text = text.strong().color(crate::theme::INK_ON_CYAN());
                }
                let action = if r.own {
                    tr!("settings-servers-tci-radio-own-tip")
                } else {
                    tr!("settings-servers-tci-radio-other-tip")
                };
                let tip = match tci_chip_state(r) {
                    Some(state) => format!("{state}\n{action}"),
                    None => action,
                };
                let chip = crate::chrome::chip(ui, r.own, text).on_hover_text(tip);
                if chip.clicked() && !r.own {
                    requests.push(crate::app::RadioTabRequest::FocusServers(r.id));
                }
            }
        });
        ui.add_space(6.0);
    }

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
    let was_on = cfg.enabled;
    crate::chrome::checkbox(ui, &mut cfg.enabled, tr!("settings-servers-enable"));
    // Switched on at a port another radio's server already holds — a radio
    // seeded before ports were spread out sits on the station's 50001 — moves
    // to the first free one. Shown before APPLY, so it is the operator's to
    // change back.
    if cfg.enabled && !was_on && !tci_port_holders(cfg.port, radios).is_empty() {
        cfg.port = first_free_tci_port(radios);
    }
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

    // Two radios on one port: the second to bind fails. Said here, before
    // APPLY, rather than left to the status line after it.
    let holders = tci_port_holders(cfg.port, radios);
    if cfg.enabled && !holders.is_empty() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr!(
                "settings-servers-tci-port-taken",
                port = cfg.port.to_string(),
                radios = holders.join(", "),
                free = first_free_tci_port(radios).to_string()
            ))
            .color(Color32::from_rgb(230, 170, 60)),
        );
    }

    // Security: TCI has no authentication at all, so binding wide open hands
    // the transmitter to anyone who can reach the port.
    if cfg.enabled && cfg.is_open_to_network() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr!("settings-servers-tci-has-no-authentication-0"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
    }

    // APPLY beside the server's own status as well as at the foot of the page:
    // the one at the foot sits under the TCI client, out of sight once the
    // client is open, and a server switched on but never applied looks like
    // one that is running.
    // Why: HDZ 2026-11-16: "du musst am Server noch einen Übernehmen-Knopf
    //      eine Reihe höher machen" — RTL-SDR showed Aktivieren and 50003 while
    //      nothing had been applied.
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if tci_apply_chip(ui).clicked() {
            *apply = true;
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
    });
    // The form differs from what this radio's engine runs: say so beside the
    // status, which describes the running server, not the form above it.
    // Why: Befund 2026-11-16 — RTL-SDR showed Aktivieren and 50003 while its
    //      server was off; the ticks had never been applied, and nothing on
    //      the page told the form from the server.
    if tci_form_unapplied(cfg, applied) {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-servers-tci-not-applied"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
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
    if tci_apply_chip(ui).clicked() {
        *apply = true;
    }

    ui.add_space(8.0);
    ui.label(RichText::new(tr!("settings-servers-server-lets-tci-capable-programs")).weak());
}

/// Whether the TCI form holds changes this radio's engine has not applied.
/// `None` (the engine has not announced its config yet) counts as no change:
/// the status line already says the state is unknown.
fn tci_form_unapplied(
    edit: &sdroxide_types::TciServerConfig,
    applied: Option<&sdroxide_types::TciServerConfig>,
) -> bool {
    applied.is_some_and(|a| a != edit)
}

/// The TCI page's APPLY chip, shown twice: beside the server's status and at
/// the foot of the page.
fn tci_apply_chip(ui: &mut egui::Ui) -> egui::Response {
    crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(tr!("settings-servers-apply")).strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text(tr!("settings-servers-persist-re-bind-server-connect"))
}

#[cfg(test)]
mod tci_port_tests {
    #[test]
    fn the_tci_form_is_unapplied_only_when_it_differs_from_the_engine() {
        use sdroxide_types::TciServerConfig;
        let applied = TciServerConfig { enabled: false, port: 50003, ..Default::default() };
        let mut edit = applied.clone();
        assert!(!super::tci_form_unapplied(&edit, Some(&applied)));
        assert!(!super::tci_form_unapplied(&edit, None));
        edit.enabled = true;
        assert!(super::tci_form_unapplied(&edit, Some(&applied)));
        assert!(!super::tci_form_unapplied(&edit, None));
    }

    use super::*;
    use crate::app::TciBrief;

    fn radio(id: u32, name: &str, tci: Option<(bool, u16)>, own: bool) -> TciRadio {
        TciRadio {
            id,
            name: name.into(),
            tci: tci.map(|(enabled, port)| TciBrief { enabled, port, failed: false }),
            own,
        }
    }

    #[test]
    fn a_port_is_taken_only_by_another_radio_whose_server_is_on() {
        let radios = [
            radio(0, "Station", Some((true, 50001)), false),
            radio(11, "RSP1a", Some((false, 50001)), true),
            radio(12, "Bench", Some((false, 50003)), false),
            radio(13, "Unheard", None, false),
        ];
        assert_eq!(tci_port_holders(50001, &radios), vec!["Station"]);
        assert!(tci_port_holders(50003, &radios).is_empty(), "a server that is off holds nothing");
        assert!(tci_port_holders(50002, &radios).is_empty());
        // The radio being edited never clashes with itself.
        let alone = [radio(11, "RSP1a", Some((true, 50001)), true)];
        assert!(tci_port_holders(50001, &alone).is_empty());
    }

    #[test]
    fn the_first_free_port_skips_every_server_that_is_on() {
        let radios = [
            radio(0, "Station", Some((true, 50001)), false),
            radio(1, "Two", Some((true, 50002)), false),
            radio(2, "Three", Some((false, 50003)), false),
            radio(3, "Edited", Some((true, 50004)), true),
        ];
        assert_eq!(first_free_tci_port(&radios), 50003);
        assert_eq!(first_free_tci_port(&[]), 50001);
    }
}
