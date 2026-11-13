//! The Mercury / Data2G panel: what the modem is doing, and a terminal to talk
//! on.
//!
//! LINK is the modem as any host sees it — whether a link is up, the speed it
//! has climbed to, the far station's signal, the channel-busy detector, and
//! who is driving it: this panel, the MAIL window, or a program on the TCP
//! host ports. TERMINAL is a keyboard-to-keyboard session from here, the way
//! VarAC's chat is: call a station, type, read.
//!
//! One modem, one host at a time. Whichever claims it first has it, and the
//! others are told so — in this panel's transcript, where the operator is
//! looking.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Mode, TncHolder, TncLineKind, TncStatus};

use crate::app::{SdroxideApp, tx_gated};
use crate::theme;

fn holder_name(h: &TncHolder) -> String {
    match h {
        TncHolder::Nobody => tr!("tnc-nobody"),
        TncHolder::Panel => tr!("tnc-holder-panel"),
        TncHolder::Winlink => tr!("tnc-holder-winlink"),
        TncHolder::Tcp(a) => tr!("tnc-holder-tcp", addr = a.clone()),
    }
}

/// A link is being made or is up: the panel calling, a call coming in.
fn working(st: &TncStatus) -> bool {
    st.connected || st.pending || !st.peer.is_empty()
}

impl SdroxideApp {
    pub(in crate::app) fn tnc_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let mode = self.digi_status.as_ref().map_or(Mode::Data2g, |s| s.mode);
        let st: Option<TncStatus> = self.digi_status.as_ref().and_then(|s| s.tnc.clone());
        let Some(st) = st else {
            ui.label(RichText::new(tr!("tnc-starting")).weak());
            return;
        };
        let tx_ok = self.tx_capable() && st.unavailable.is_none();
        let pane = self.phone_pane(ui, mode);

        ui.horizontal(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        ui.set_width((ui.available_width() * 0.42).clamp(220.0, 360.0));
                    }
                    self.tnc_link(ui, cmds, &st, tx_ok);
                });
            }
            if pane.is_none() {
                ui.separator();
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    self.tnc_terminal(ui, cmds, &st, panel_h, tx_ok);
                });
            }
        });
    }

    /// The modem's state, wrapped so a phone shows all of it.
    fn tnc_link(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &TncStatus,
        tx_ok: bool,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr!("tnc-link")).strong().color(theme::CYAN()));
            let (label, colour) = if st.connected {
                (tr!("tnc-connected"), theme::GREEN())
            } else if st.unavailable.is_some() {
                (tr!("tnc-unavailable"), theme::ALERT())
            } else if st.pending {
                (tr!("tnc-incoming"), theme::YELLOW())
            } else if working(st) {
                (tr!("tnc-calling"), theme::YELLOW())
            } else {
                (tr!("tnc-idle"), theme::CYAN_DIM())
            };
            ui.label(RichText::new(label).strong().color(colour)).on_hover_text(st.version.clone());
            if st.ptt {
                ui.label(RichText::new(tr!("tnc-tx")).strong().color(theme::ALERT()));
            }
            if st.busy {
                ui.label(RichText::new(tr!("tnc-busy")).strong().color(theme::YELLOW()))
                    .on_hover_text(tr!("tnc-busy-hint"));
            }
            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(
                    ui,
                    self.show_digi_settings,
                    RichText::new(tr!("tnc-setup")).size(9.5),
                )
                .on_hover_text(tr!("tnc-setup-hint"))
                .clicked()
                {
                    self.show_digi_settings = !self.show_digi_settings;
                }
            });
        });

        egui::Grid::new("tnc-link-grid").num_columns(2).spacing([8.0, 2.0]).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, k: String, v: RichText| {
                ui.label(RichText::new(k).weak());
                ui.label(v);
                ui.end_row();
            };
            if !st.peer.is_empty() {
                row(ui, tr!("tnc-peer"), RichText::new(&st.peer).monospace().strong());
            }
            if let Some(why) = &st.unavailable {
                row(ui, tr!("tnc-problem"), RichText::new(why).color(theme::ALERT()));
            }
            row(ui, tr!("tnc-width"), RichText::new(format!("{} Hz", st.bw.hz())).monospace());
            if st.connected && st.bps > 0 {
                row(
                    ui,
                    tr!("tnc-speed"),
                    RichText::new(format!("L{}  {} bps", st.level, st.bps)).monospace(),
                );
            } else if st.connected && !st.submode.is_empty() {
                row(ui, tr!("tnc-speed"), RichText::new(&st.submode).monospace());
            }
            if let Some(snr) = st.snr_db {
                row(ui, tr!("tnc-snr"), RichText::new(format!("{snr:+.1} dB")).monospace());
            }
            if st.connected {
                row(
                    ui,
                    tr!("tnc-bytes"),
                    RichText::new(format!("↓{}  ↑{}", st.rx_bytes, st.tx_bytes)).monospace(),
                );
            }
            if st.buffered > 0 {
                row(ui, tr!("tnc-queued"), RichText::new(format!("{}", st.buffered)).monospace());
            }
            row(
                ui,
                tr!("tnc-host"),
                RichText::new(holder_name(&st.holder)).color(theme::CYAN_DIM()),
            );
            let (ports, colour) = match (&st.host_listening, &st.host_error) {
                (Some(a), _) => (a.clone(), theme::GREEN()),
                (None, Some(e)) => (e.clone(), theme::ALERT()),
                (None, None) => (tr!("tnc-ports-off"), theme::CYAN_DIM()),
            };
            row(ui, tr!("tnc-ports"), RichText::new(ports).color(colour));
        });

        ui.horizontal_wrapped(|ui| {
            if tx_gated(ui, tx_ok, |ui| {
                crate::chrome::chip(ui, st.tuning, RichText::new(tr!("tnc-tune")).size(10.5))
                    .on_hover_text(tr!("tnc-tune-hint"))
            })
            .clicked()
            {
                cmds.push(Command::TncTune { on: !st.tuning });
            }
            if working(st)
                && crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(tr!("tnc-abort")).size(10.5).strong(),
                    theme::ALERT(),
                    theme::INK_ON_CYAN(),
                )
                .on_hover_text(tr!("tnc-abort-hint"))
                .clicked()
            {
                cmds.push(Command::TncAbort);
            }
        });
    }

    /// Keyboard to keyboard: the call bar, the scrollback, and the line typed.
    fn tnc_terminal(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &TncStatus,
        panel_h: f32,
        tx_ok: bool,
    ) {
        let panel_holds = st.holder == TncHolder::Panel;
        let other_holds = !matches!(st.holder, TncHolder::Nobody | TncHolder::Panel);
        let busy_link = working(st);
        let have_call = !self.digi_cfg_edit.tnc_mycall.trim().is_empty()
            || !self.digi_cfg_edit.my_call.trim().is_empty();

        // ── call bar ──────────────────────────────────────────────────────
        let mut connect = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!("tnc-terminal")).strong().color(theme::CYAN()));
            ui.add_enabled_ui(!busy_link, |ui| {
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.tnc_target)
                        .desired_width(84.0)
                        .hint_text(tr!("tnc-callsign")),
                );
                if resp.changed() {
                    self.tnc_target = self.tnc_target.to_uppercase();
                }
                connect |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            let ready = !self.tnc_target.trim().is_empty();
            if busy_link && panel_holds {
                if crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(tr!("tnc-disconnect")).strong(),
                    theme::ALERT(),
                    theme::INK_ON_CYAN(),
                )
                .on_hover_text(tr!("tnc-disconnect-hint"))
                .clicked()
                {
                    cmds.push(Command::TncDisconnect);
                }
            } else if tx_gated(
                ui,
                tx_ok && ready && have_call && !other_holds && !busy_link,
                |ui| {
                    crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new(tr!("tnc-connect")).strong(),
                        theme::GREEN(),
                        theme::INK_ON_CYAN(),
                    )
                    .on_hover_text(if other_holds {
                        tr!("tnc-held-by", holder = holder_name(&st.holder))
                    } else if !have_call {
                        tr!("tnc-no-callsign")
                    } else {
                        tr!("tnc-connect-hint")
                    })
                },
            )
            .clicked()
            {
                connect = true;
            }
            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(ui, false, RichText::new(tr!("tnc-clear")).size(10.5))
                    .on_hover_text(tr!("tnc-clear-hint"))
                    .clicked()
                {
                    cmds.push(Command::TncTermClear);
                }
            });
        });
        if connect
            && tx_ok
            && have_call
            && !busy_link
            && !other_holds
            && !self.tnc_target.trim().is_empty()
        {
            cmds.push(Command::TncConnect { call: self.tnc_target.trim().to_string() });
        }
        if other_holds {
            ui.label(
                RichText::new(tr!("tnc-held-by", holder = holder_name(&st.holder)))
                    .weak()
                    .color(theme::YELLOW()),
            );
        }

        // ── scrollback ────────────────────────────────────────────────────
        let input_h = 26.0;
        egui::ScrollArea::vertical()
            .id_salt("tnc-term")
            .max_height((panel_h - 76.0 - input_h).max(60.0))
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if st.transcript.is_empty() {
                    ui.label(RichText::new(tr!("tnc-nothing-yet")).weak());
                }
                for l in &st.transcript {
                    let colour = match l.kind {
                        TncLineKind::Rx => theme::TEXT(),
                        TncLineKind::Tx => theme::GREEN(),
                        TncLineKind::Event => theme::CYAN_DIM(),
                        TncLineKind::Host => theme::gray(140),
                        TncLineKind::Error => theme::ALERT(),
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
                    RichText::new(tr!("tnc-send")).size(10.0).strong(),
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
                    egui::TextEdit::singleline(&mut self.tnc_draft).desired_width(room).hint_text(
                        if can_send { tr!("tnc-type-here") } else { tr!("tnc-not-connected") },
                    ),
                );
                send |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        });
        if send && tx_ok && can_send {
            cmds.push(Command::TncSend { text: self.tnc_draft.trim_end().to_string() });
            self.tnc_draft.clear();
        }
    }
}
