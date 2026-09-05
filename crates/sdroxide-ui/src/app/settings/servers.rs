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
use eframe::egui::{self, Color32, ComboBox, RichText};

/// Live status of the built-in TCI server, from `RadioEvent::TciServerStatus`.
#[derive(Clone, PartialEq)]
pub(in crate::app) struct TciServerStatus {
    pub(in crate::app) running: bool,
    pub(in crate::app) addr: String,
    pub(in crate::app) clients: usize,
    pub(in crate::app) error: Option<String>,
}

pub(in crate::app) fn settings_wsjtx_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::WsjtxConfig,
    seeded: bool,
    apply: &mut bool,
) {
    ui.label(RichText::new("WSJT-X UDP broadcast").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(4.0);
    if !seeded {
        ui.label(RichText::new("Waiting for the station's WSJT-X broadcast configuration…").weak());
        return;
    }
    ui.label(
        RichText::new(
            "Sends decodes, station status and logged QSOs the way WSJT-X does, so GridTracker, \
             JTAlert, N1MM+ and Log4OM work with sdroxide unchanged. Output only — nothing on \
             this socket can touch the radio.",
        )
        .weak(),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, "Enable");
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("wsjtx-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Send to");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.host)
                    .desired_width(160.0)
                    .hint_text("127.0.0.1"),
            )
            .on_hover_text(
                "127.0.0.1 reaches clients on this machine; a LAN address or a multicast \
                 group (224.0.0.1) reaches others",
            );
            ui.end_row();

            ui.label("Port");
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535))
                .on_hover_text("2237 is the port every client defaults to");
            ui.end_row();

            ui.label("Identify as");
            crate::chrome::field(ui, egui::TextEdit::singleline(&mut cfg.id).desired_width(160.0))
                .on_hover_text(
                    "The name clients see. Some loggers only accept traffic identifying itself \
                 as WSJT-X.",
                );
            ui.end_row();
        });
    });

    // ── N1MM+, the other dialect ──
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(
        RichText::new("N1MM+ contactinfo broadcast").size(13.0).strong().color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "The same news in N1MM's words: one XML datagram per logged contact, for the loggers \
             that listen for N1MM rather than for WSJT-X — the World Radio League's bridge takes \
             either. Contacts only; there is no decode stream in this protocol. Its own switch \
             and its own port, because a station may want both.",
        )
        .weak(),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.n1mm.enabled, "Enable");
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.n1mm.enabled, |ui| {
        egui::Grid::new("n1mm-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Send to");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.n1mm.host)
                    .desired_width(160.0)
                    .hint_text("127.0.0.1"),
            )
            .on_hover_text(
                "127.0.0.1 reaches clients on this machine. N1MM's own advice for a contest \
                 network is this subnet's broadcast address — 192.168.1.255 for a 192.168.1.n \
                 network — which reaches every position at once.",
            );
            ui.end_row();

            ui.label("Port");
            ui.add(egui::DragValue::new(&mut cfg.n1mm.port).range(1..=65535))
                .on_hover_text("12060 is the port N1MM's documentation recommends");
            ui.end_row();

            ui.label("Station name");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.n1mm.station).desired_width(160.0),
            )
            .on_hover_text(
                "What N1MM calls the StationName: the name of the computer that sent the \
                 packet. Loggers show it to tell one operating position from another.",
            );
            ui.end_row();
        });
    });

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(" APPLY ").strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text("Persist and (re)open the broadcast sockets")
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
        RichText::new("Hamlib rigctld server").size(14.0).strong().color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    if !seeded {
        ui.label(RichText::new("Waiting for the station's rigctld configuration…").weak());
        return;
    }
    ui.add_space(2.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, "Enable");
    ui.add_space(6.0);

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("rigctld-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Listen on");
            ComboBox::from_id_salt("rigctld_bind").selected_text(&cfg.bind).show_styled(ui, |ui| {
                for b in RigctldConfig::BINDS {
                    let label = if b == "127.0.0.1" {
                        "127.0.0.1 (this machine)"
                    } else {
                        "0.0.0.0 (whole network)"
                    };
                    if ui.selectable_label(cfg.bind == b, label).clicked() {
                        cfg.bind = b.to_string();
                    }
                }
            });
            ui.end_row();

            ui.label("Port");
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535))
                .on_hover_text("4532 is the port every rigctld client assumes");
            ui.end_row();

            ui.label("Rig name");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.rig_name)
                    .desired_width(160.0)
                    .hint_text("reported to clients"),
            );
            ui.end_row();

            ui.label("Max clients");
            ui.add(egui::DragValue::new(&mut cfg.max_clients).range(1..=32));
            ui.end_row();
        });
        ui.add_space(4.0);
        crate::chrome::checkbox(ui, &mut cfg.allow_tx, "Allow clients to transmit").on_hover_text(
            "Off refuses every key request and stops advertising a transmit range, so Hamlib \
             itself declines to key.",
        );
    });

    // Same hazard as TCI: the protocol has no authentication at all.
    if cfg.enabled && cfg.is_open_to_network() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "⚠ rigctld has no authentication. On 0.0.0.0, anyone who can reach this port can \
                 tune and key your transmitter.",
            )
            .color(Color32::from_rgb(230, 170, 60)),
        );
    }

    ui.add_space(8.0);
    match status {
        Some(s) if s.running => {
            let clients = match s.clients {
                1 => "1 client".to_string(),
                n => format!("{n} clients"),
            };
            ui.label(
                RichText::new(format!("● Listening on {} — {clients}", s.addr))
                    .color(Color32::from_rgb(90, 200, 110)),
            );
        }
        Some(s) => match &s.error {
            Some(e) => {
                ui.label(
                    RichText::new(format!("Failed: {e}")).color(Color32::from_rgb(230, 90, 80)),
                );
            }
            None => {
                ui.label(RichText::new("Not running.").weak());
            }
        },
        None => {
            ui.label(RichText::new("Status unknown — press APPLY.").weak());
        }
    }

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(" APPLY ").strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text("Persist and (re)bind the server")
    .clicked()
    {
        *apply = true;
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Lets any Hamlib-capable program drive this radio: frequency, mode, PTT, split, RIT \
             and power. In WSJT-X, fldigi or CQRLOG choose the rig \u{201c}Hamlib NET rigctl\u{201d} \
             (model 2) and point it at this address; in GPredict and N1MM enter the host and port \
             directly. Unlike the TCI server it carries control only \u{2014} no audio, no IQ.",
        )
        .weak(),
    );
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
        RichText::new("Antenna rotator (rotctld client)")
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    if !seeded {
        ui.label(RichText::new("Waiting for the station's rotator configuration…").weak());
        return;
    }
    ui.label(
        RichText::new(
            "Points a motorized antenna at the satellite you are locked onto, through a Hamlib \
             rotctld daemon (run e.g. \u{201c}rotctld -m 603 -r /dev/ttyUSB0\u{201d} for a GS-232B \
             rotator, or \u{201c}rotctld -m 1\u{201d} to try it without hardware). One daemon covers \
             every rotator Hamlib drives — EasyComm, GS-232, SPID and the rest.",
        )
        .weak(),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, "Enable");
    ui.add_space(6.0);
    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("rotator-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Host");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.host)
                    .desired_width(160.0)
                    .hint_text("127.0.0.1"),
            );
            ui.end_row();

            ui.label("Port");
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535))
                .on_hover_text("4533 is rotctld's default");
            ui.end_row();

            ui.label("Min elevation");
            ui.add(egui::DragValue::new(&mut cfg.min_el_deg).range(0.0..=45.0).suffix("°"))
                .on_hover_text(
                    "Below this the rotator parks instead of tracking — set it to your local \
                     horizon or roofline",
                );
            ui.end_row();

            ui.label("Azimuth offset");
            ui.add(egui::DragValue::new(&mut cfg.az_offset_deg).range(-180.0..=180.0).suffix("°"))
                .on_hover_text("Added to every command, for a rotator whose north is off");
            ui.end_row();

            ui.label("Min movement");
            ui.add(egui::DragValue::new(&mut cfg.min_move_deg).range(0.1..=10.0).suffix("°"))
                .on_hover_text(
                    "Steps smaller than this are not sent — chasing tenths of a degree wears \
                     motors for nothing",
                );
            ui.end_row();

            ui.label("Park position");
            let mut has_park = cfg.park.is_some();
            if ui
                .checkbox(&mut has_park, "drive to a position when idle")
                .on_hover_text("Off leaves the antenna wherever the last pass ended")
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
                RichText::new(format!("● Connected — antenna at {az:.0}° az, {el:.0}° el"))
                    .color(Color32::from_rgb(90, 200, 110)),
            );
        }
        Some((false, _, _, Some(e))) => {
            ui.label(RichText::new(format!("Failed: {e}")).color(Color32::from_rgb(230, 90, 80)));
        }
        Some((false, _, _, None)) => {
            ui.label(RichText::new("Not connected.").weak());
        }
        None if cfg.enabled => {
            ui.label(RichText::new("Status unknown — press APPLY.").weak());
        }
        None => {}
    }

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(" APPLY ").strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text("Persist and (re)connect to the daemon")
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
        ui.label(RichText::new("Waiting for the station's TCI server configuration…").weak());
        return;
    }

    ui.label(RichText::new("Built-in TCI server").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, "Enable");
    ui.add_space(6.0);

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("tci-srv-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Listen on");
            ComboBox::from_id_salt("tci_srv_bind").selected_text(&cfg.bind).show_styled(ui, |ui| {
                for b in TciServerConfig::BINDS {
                    let label = if b == "127.0.0.1" {
                        "127.0.0.1 (this machine)"
                    } else {
                        "0.0.0.0 (whole network)"
                    };
                    if ui.selectable_label(cfg.bind == b, label).clicked() {
                        cfg.bind = b.to_string();
                    }
                }
            });
            ui.end_row();

            ui.label("Port");
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535));
            ui.end_row();

            ui.label("Device name");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.device_name)
                    .desired_width(160.0)
                    .hint_text("reported to clients"),
            );
            ui.end_row();

            ui.label("Max clients");
            ui.add(egui::DragValue::new(&mut cfg.max_clients).range(1..=32));
            ui.end_row();
        });
        ui.add_space(4.0);
        crate::chrome::checkbox(ui, &mut cfg.allow_tx, "Allow clients to transmit").on_hover_text(
            "Off leaves control and the receive streams working, but every key request \
             is refused.",
        );
    });

    // Security: TCI has no authentication at all, so binding wide open hands
    // the transmitter to anyone who can reach the port.
    if cfg.enabled && cfg.is_open_to_network() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "⚠ TCI has no authentication. On 0.0.0.0, anyone who can reach this port can \
                 tune and key your transmitter.",
            )
            .color(Color32::from_rgb(230, 170, 60)),
        );
    }

    ui.add_space(8.0);
    match status {
        Some(s) if s.running => {
            let clients = match s.clients {
                1 => "1 client".to_string(),
                n => format!("{n} clients"),
            };
            ui.label(
                RichText::new(format!("● Listening on {} — {clients}", s.addr))
                    .color(Color32::from_rgb(90, 200, 110)),
            );
        }
        Some(s) => match &s.error {
            Some(e) => {
                let msg = RichText::new(format!("Failed: {e}"));
                ui.label(msg.color(Color32::from_rgb(230, 90, 80)));
            }
            None => {
                ui.label(RichText::new("Not running.").weak());
            }
        },
        None => {
            ui.label(RichText::new("Status unknown — press APPLY.").weak());
        }
    }

    ui.add_space(8.0);
    if crate::chrome::chip_accent(
        ui,
        false,
        RichText::new(" APPLY ").strong(),
        crate::theme::GREEN(),
        crate::theme::INK_ON_CYAN(),
    )
    .on_hover_text("Persist and (re)bind the server")
    .clicked()
    {
        *apply = true;
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Lets TCI-capable programs drive this radio: frequency and mode control, wideband IQ, \
             receive audio, and transmit audio. In WSJT-X choose the SunSDR (TCI) rig type, point \
             it at this address, and set PTT to TCI.",
        )
        .weak(),
    );
}
