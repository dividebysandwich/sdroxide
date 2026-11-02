//! The VARA HF panel: what the modem is doing, and a terminal to talk on.
//!
//! LINK is the modem as any host sees it — the session state, the speed level
//! the link has climbed to, the far station's signal, the channel-busy
//! detector, and who is driving it: this panel, the MAIL window, or a program
//! on VARA's TCP ports. TERMINAL is a keyboard-to-keyboard session from here,
//! the way VarAC's chat is: call a station, type, read.
//!
//! One modem, one host at a time. Whichever claims it first has it, and the
//! others are told so — in this panel's transcript, where the operator is
//! looking.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{Command, VaraHolder, VaraLineKind, VaraStatus};

use crate::app::{SdroxideApp, tx_gated};
use crate::theme;

fn holder_name(h: &VaraHolder) -> String {
    match h {
        VaraHolder::Nobody => tr!("vara-nobody"),
        VaraHolder::Panel => tr!("vara-holder-panel"),
        VaraHolder::Winlink => tr!("vara-holder-winlink"),
        VaraHolder::Tcp(a) => tr!("vara-holder-tcp", addr = a.clone()),
    }
}

/// The VARA session states that mean "a link is being made or is up".
fn working(state: &str) -> bool {
    !matches!(state, "" | "Listen")
}

impl SdroxideApp {
    pub(in crate::app) fn vara_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<VaraStatus> = self.digi_status.as_ref().and_then(|s| s.vara.clone());
        let Some(st) = st else {
            ui.label(RichText::new(tr!("vara-starting")).weak());
            return;
        };
        let tx_ok = self.tx_capable();
        let pane = self.phone_pane(ui, sdroxide_types::Mode::VaraHf);

        ui.horizontal(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        ui.set_width((ui.available_width() * 0.42).clamp(220.0, 360.0));
                    }
                    self.vara_link(ui, cmds, &st, tx_ok);
                });
            }
            if pane.is_none() {
                ui.separator();
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    self.vara_terminal(ui, cmds, &st, panel_h, tx_ok);
                });
            }
        });
    }

    /// The modem's state, wrapped so a phone shows all of it.
    fn vara_link(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &VaraStatus,
        tx_ok: bool,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr!("vara-link")).strong().color(theme::CYAN()));
            let (label, colour) = if st.connected {
                (tr!("vara-connected"), theme::GREEN())
            } else if working(&st.state) {
                (tr!("vara-calling"), theme::YELLOW())
            } else {
                (tr!("vara-idle"), theme::CYAN_DIM())
            };
            ui.label(RichText::new(label).strong().color(colour))
                .on_hover_text(tr!("vara-session-state", state = st.state.clone()));
            if st.ptt {
                ui.label(RichText::new(tr!("vara-tx")).strong().color(theme::ALERT()));
            }
            if st.busy {
                ui.label(RichText::new(tr!("vara-busy")).strong().color(theme::YELLOW()))
                    .on_hover_text(tr!("vara-busy-hint"));
            }
            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(
                    ui,
                    self.show_digi_settings,
                    RichText::new(tr!("vara-setup")).size(9.5),
                )
                .on_hover_text(tr!("vara-setup-hint"))
                .clicked()
                {
                    self.show_digi_settings = !self.show_digi_settings;
                }
            });
        });

        egui::Grid::new("vara-link-grid").num_columns(2).spacing([8.0, 2.0]).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, k: String, v: RichText| {
                ui.label(RichText::new(k).weak());
                ui.label(v);
                ui.end_row();
            };
            if !st.peer.is_empty() {
                row(ui, tr!("vara-peer"), RichText::new(&st.peer).monospace().strong());
            }
            row(ui, tr!("vara-width"), RichText::new(format!("{} Hz", st.bw.hz())).monospace());
            if st.connected && st.level > 0 {
                row(
                    ui,
                    tr!("vara-speed"),
                    RichText::new(format!("L{}  {} bps", st.level, st.bps)).monospace(),
                );
            }
            if let Some(snr) = st.snr_db {
                row(ui, tr!("vara-snr"), RichText::new(format!("{snr:+.1} dB")).monospace());
            }
            if st.connected {
                row(
                    ui,
                    tr!("vara-bytes"),
                    RichText::new(format!("↓{}  ↑{}", st.rx_bytes, st.tx_bytes)).monospace(),
                );
            }
            if st.buffered > 0 {
                row(ui, tr!("vara-queued"), RichText::new(format!("{}", st.buffered)).monospace());
            }
            if !st.connected && st.attempts > 0 && working(&st.state) {
                row(ui, tr!("vara-attempt"), RichText::new(format!("{}", st.attempts)).monospace());
            }
            row(
                ui,
                tr!("vara-host"),
                RichText::new(holder_name(&st.holder)).color(theme::CYAN_DIM()),
            );
            let (ports, colour) = match (&st.host_listening, &st.host_error) {
                (Some(a), _) => (a.clone(), theme::GREEN()),
                (None, Some(e)) => (e.clone(), theme::ALERT()),
                (None, None) => (tr!("vara-ports-off"), theme::CYAN_DIM()),
            };
            row(ui, tr!("vara-ports"), RichText::new(ports).color(colour));
        });

        ui.horizontal_wrapped(|ui| {
            if tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip(ui, st.tuning, RichText::new(tr!("vara-tune")).size(10.5))
                    .on_hover_text(tr!("vara-tune-hint"))
            })
            .clicked()
            {
                cmds.push(Command::VaraTune { on: !st.tuning });
            }
            if (st.connected || working(&st.state))
                && crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(tr!("vara-abort")).size(10.5).strong(),
                    theme::ALERT(),
                    theme::INK_ON_CYAN(),
                )
                .on_hover_text(tr!("vara-abort-hint"))
                .clicked()
            {
                cmds.push(Command::VaraAbort);
            }
        });
    }

    /// Keyboard to keyboard: the call bar, the scrollback, and the line typed.
    fn vara_terminal(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &VaraStatus,
        panel_h: f32,
        tx_ok: bool,
    ) {
        let panel_holds = st.holder == VaraHolder::Panel;
        let other_holds = !matches!(st.holder, VaraHolder::Nobody | VaraHolder::Panel);
        let busy_link = st.connected || working(&st.state);
        let have_call = !self.digi_cfg_edit.vara_mycall.trim().is_empty()
            || !self.digi_cfg_edit.my_call.trim().is_empty();

        // ── call bar ──────────────────────────────────────────────────────
        let mut connect = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!("vara-terminal")).strong().color(theme::CYAN()));
            ui.add_enabled_ui(!busy_link, |ui| {
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.vara_target)
                        .desired_width(84.0)
                        .hint_text(tr!("vara-callsign")),
                );
                if resp.changed() {
                    self.vara_target = self.vara_target.to_uppercase();
                }
                connect |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            let ready = !self.vara_target.trim().is_empty();
            if busy_link && panel_holds {
                if crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(tr!("vara-disconnect")).strong(),
                    theme::ALERT(),
                    theme::INK_ON_CYAN(),
                )
                .on_hover_text(tr!("vara-disconnect-hint"))
                .clicked()
                {
                    cmds.push(Command::VaraDisconnect);
                }
            } else if tx_gated(
                ui,
                tx_ok && ready && have_call && !other_holds && !busy_link,
                |ui| {
                    crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new(tr!("vara-connect")).strong(),
                        theme::GREEN(),
                        theme::INK_ON_CYAN(),
                    )
                    .on_hover_text(if other_holds {
                        tr!("vara-held-by", holder = holder_name(&st.holder))
                    } else if !have_call {
                        tr!("vara-no-callsign")
                    } else {
                        tr!("vara-connect-hint")
                    })
                },
            )
            .clicked()
            {
                connect = true;
            }
            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(ui, false, RichText::new(tr!("vara-clear")).size(10.5))
                    .on_hover_text(tr!("vara-clear-hint"))
                    .clicked()
                {
                    cmds.push(Command::VaraTermClear);
                }
            });
        });
        if connect
            && tx_ok
            && have_call
            && !busy_link
            && !other_holds
            && !self.vara_target.trim().is_empty()
        {
            cmds.push(Command::VaraConnect { call: self.vara_target.trim().to_string() });
        }
        if other_holds {
            ui.label(
                RichText::new(tr!("vara-held-by", holder = holder_name(&st.holder)))
                    .weak()
                    .color(theme::YELLOW()),
            );
        }

        // ── scrollback ────────────────────────────────────────────────────
        let input_h = 26.0;
        egui::ScrollArea::vertical()
            .id_salt("vara-term")
            .max_height((panel_h - 76.0 - input_h).max(60.0))
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if st.transcript.is_empty() {
                    ui.label(RichText::new(tr!("vara-nothing-yet")).weak());
                }
                for l in &st.transcript {
                    let colour = match l.kind {
                        VaraLineKind::Rx => theme::TEXT(),
                        VaraLineKind::Tx => theme::GREEN(),
                        VaraLineKind::Event => theme::CYAN_DIM(),
                        VaraLineKind::Host => theme::gray(140),
                        VaraLineKind::Error => theme::ALERT(),
                    };
                    ui.add(
                        egui::Label::new(RichText::new(&l.text).monospace().color(colour)).wrap(),
                    );
                }
            });

        // ── the line you type on ──────────────────────────────────────────
        let can_send = st.connected && panel_holds;
        let mut send = false;
        // Right to left: the SEND chip is laid out first at its own width,
        // which is not the English one in every language (ОТПРАВИТЬ), and the
        // text field takes whatever is left.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if tx_gated(ui, tx_ok && can_send, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(tr!("vara-send")).size(10.0).strong(),
                    theme::GREEN(),
                    theme::INK_ON_CYAN(),
                )
            })
            .clicked()
            {
                send = true;
            }
            let room = ui.available_width().max(80.0);
            ui.add_enabled_ui(can_send, |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.vara_draft).desired_width(room).hint_text(
                        if can_send { tr!("vara-type-here") } else { tr!("vara-not-connected") },
                    ),
                );
                send |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        });
        if send && tx_ok && can_send {
            cmds.push(Command::VaraSend { text: self.vara_draft.trim_end().to_string() });
            self.vara_draft.clear();
        }
    }
}
