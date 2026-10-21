//! The NAVTEX panel: the messages received, and the one arriving.
//!
//! Two panes, because they move at different rates and are read differently.
//! The list on the left is a file — a station's four-hourly cycle collapsed to
//! one entry per message, newest first, marked by what kind of warning it is.
//! The pane on the right is whatever is being copied *now*, which on a mode
//! with a ten-minute slot is the thing an operator is actually watching.
//!
//! There is nothing to transmit and no callsign to set, so there is no setup
//! window and no transmit half — the two controls are which tone sense the
//! receiver reads and whether to show the loose text below the message list.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, NavtexMessage, NavtexStatus, Vfo};

use crate::app::SdroxideApp;
use crate::theme;

/// The NAVTEX channels the panel offers, as dial frequencies — see
/// `panels::mode_dials`. The MF/HF service is the one a shortwave listener
/// reaches.
const CHANNELS: &[(f64, &str)] = &[
    (6_314_000.0, "6314"),
    (8_416_500.0, "8416.5"),
    (12_579_000.0, "12579"),
    (16_806_500.0, "16806.5"),
    (22_376_000.0, "22376"),
];

/// The message list's header row: the count, the sync state, what the time
/// diversity repaired, SAVE and REV. Returns whether REV was pressed; `rev` is
/// its state and `status` the app's `digi_status`, which SAVE writes from.
///
/// Wrapped: on a phone the row is wider than the screen, and a plain one ran
/// REV off its edge.
pub(super) fn navtex_list_header(
    ui: &mut egui::Ui,
    st: &NavtexStatus,
    status: Option<&sdroxide_types::DigiStatus>,
    cmds: &mut Vec<Command>,
    rev: bool,
) -> bool {
    let mut flip = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("MESSAGES").strong().color(theme::CYAN()));
        ui.label(RichText::new(format!("{}", st.messages.len())).weak());
        // The mode's own carrier detect: a constant-ratio code either
        // frames or it does not, and there is no in-between to show.
        if st.in_sync {
            ui.label(RichText::new("SYNC").strong().color(theme::GREEN()))
                .on_hover_text("The character phase is locked — a signal is being read.");
        } else {
            ui.label(RichText::new("hunting").weak()).on_hover_text(
                "Nothing is being read yet.\n\n\
                 It is almost always the tuning. Do NOT type the advertised number (518) \
                 into the frequency box: NAVTEX is quoted as a channel, and this program \
                 listens just below it. Click the 518 button above instead — it sets the \
                 right dial for you (516.300).\n\n\
                 If there is clearly a strong signal but the text is still nonsense, press \
                 REV: it is arriving the other way round.\n\n\
                 Also remember NAVTEX is a timetable, not a conversation: a coast station \
                 sends for ten minutes every four hours, so silence is normal most of the \
                 time. You have to be listening when your station is due. And 518 kHz only \
                 comes alive at night — a quiet daytime band is not a broken decoder.",
            );
        }
        // What the time diversity actually did, which is the only quality
        // figure a mode with no checksum has.
        if st.repaired > 0 || st.lost > 0 {
            ui.label(
                RichText::new(format!("{} repaired · {} lost", st.repaired, st.lost))
                    .size(10.0)
                    .color(if st.lost > 0 { theme::YELLOW() } else { theme::CYAN_DIM() }),
            )
            .on_hover_text(
                "Characters whose first copy was corrupt and were taken from the repeat five \
                 slots later, and those where both were bad. A NAVTEX character is sent \
                 twice; that is the whole of its error correction.",
            );
        }
        crate::chrome::row_tail(ui, |ui| {
            super::clear_rx_chip_at(ui, cmds, !st.messages.is_empty() || st.live.is_some());
            super::save_rx_chip_for(ui, status);
            if crate::chrome::chip(ui, rev, RichText::new("REV").size(10.5))
                .on_hover_text(
                    "Swap the mark and space tones, for a signal received on the other \
                     sideband. Off is upper sideband on the channel frequency, which is what \
                     every published tuning instruction for the service says.",
                )
                .clicked()
            {
                flip = true;
            }
        });
    });
    flip
}

impl SdroxideApp {
    pub(in crate::app) fn navtex_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<NavtexStatus> = self.digi_status.as_ref().and_then(|s| s.navtex.clone());
        let Some(st) = st else {
            ui.label(RichText::new("starting the NAVTEX receiver…").weak());
            return;
        };
        // A NAVTEX receiver driven into overload hears a wall of noise, not
        // characters, and the panel would only say "hunting".
        self.front_end_overload_warning(ui);
        let pane = self.phone_pane(ui, sdroxide_types::Mode::Navtex);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("CHANNEL").size(10.0).weak());
            for (hz, label) in CHANNELS {
                if crate::chrome::chip(ui, false, RichText::new(*label).size(10.0))
                    .on_hover_text("Tune the dial here.")
                    .clicked()
                {
                    cmds.push(Command::SetVfo { vfo: Vfo::A, hz: *hz });
                }
            }
        });

        ui.horizontal(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        ui.set_width(ui.available_width() * 0.46);
                    }
                    self.navtex_list(ui, cmds, &st, panel_h);
                });
            }
            if pane.is_none() {
                ui.separator();
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    self.navtex_reading(ui, &st, panel_h);
                });
            }
        });
    }

    /// The messages received, newest first.
    fn navtex_list(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &NavtexStatus,
        panel_h: f32,
    ) {
        if navtex_list_header(ui, st, self.digi_status.as_ref(), cmds, self.digi_cfg_edit.navtex_reverse)
        {
            self.digi_cfg_edit.navtex_reverse = !self.digi_cfg_edit.navtex_reverse;
            if self.digi_cfg_seeded {
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
        }

        egui::ScrollArea::vertical()
            .id_salt("navtex-list")
            .max_height((panel_h - 40.0).max(60.0))
            .show(ui, |ui| {
                if st.messages.is_empty() && st.live.is_none() {
                    ui.label(
                        RichText::new(
                            "nothing yet — a station transmits for ten minutes every four hours",
                        )
                        .weak(),
                    );
                }
                for (i, m) in st.messages.iter().enumerate().rev() {
                    let on = self.navtex_open == Some(i);
                    let head =
                        format!("{}{}{:02}  {}", m.station, m.kind, m.serial, m.kind_label());
                    let colour = if m.is_mandatory() {
                        // The three classes a ship's receiver may not switch
                        // off. sdroxide does not offer to hide them either, and
                        // they are marked so an eye finds them first.
                        theme::ALERT()
                    } else if m.complete {
                        theme::CYAN()
                    } else {
                        theme::YELLOW()
                    };
                    let mut text = RichText::new(head).monospace().color(colour);
                    if on {
                        text = text.strong();
                    }
                    if ui.selectable_label(on, text).clicked() {
                        self.navtex_open = if on { None } else { Some(i) };
                    }
                    let first = m.text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
                    let mut line = format!("      {}", first.trim());
                    line.truncate(60);
                    ui.label(RichText::new(line).size(10.0).weak());
                    if !m.complete || m.lost > 0 {
                        let mut why = Vec::new();
                        if !m.complete {
                            why.push("cut short".to_string());
                        }
                        if m.lost > 0 {
                            why.push(format!("{} characters lost", m.lost));
                        }
                        ui.label(
                            RichText::new(format!("      {}", why.join(", ")))
                                .size(9.5)
                                .color(theme::YELLOW()),
                        );
                    }
                }
            });
    }

    /// The message being read: the one selected, or the one arriving.
    fn navtex_reading(&mut self, ui: &mut egui::Ui, st: &NavtexStatus, panel_h: f32) {
        let selected: Option<&NavtexMessage> =
            self.navtex_open.and_then(|i| st.messages.get(i)).or(st.live.as_ref());
        ui.horizontal(|ui| match selected {
            Some(m) if self.navtex_open.is_some() => {
                ui.label(RichText::new("MESSAGE").strong().color(theme::CYAN()));
                ui.label(
                    RichText::new(format!("{}{}{:02}", m.station, m.kind, m.serial))
                        .monospace()
                        .color(theme::CYAN_DIM()),
                );
                ui.label(RichText::new(m.kind_label()).size(10.5).weak());
                // The time the message states, pulled out of the body: a
                // warning is read against when it was issued, and finding the
                // figure in a column of positions by eye is the tedious part.
                // Display only — the system clock is never set from it.
                if let Some((h, min)) = m.body_time_utc() {
                    ui.label(
                        RichText::new(format!("{h:02}{min:02} UTC"))
                            .monospace()
                            .size(10.5)
                            .color(theme::GREEN()),
                    )
                    .on_hover_text("Time of day stated in the message (UTC): display only");
                }
            }
            Some(_) => {
                ui.label(RichText::new("RECEIVING").strong().color(theme::ALERT()));
            }
            None => {
                ui.label(RichText::new("MONITOR").strong().color(theme::CYAN()));
                ui.label(RichText::new("everything decoded, message or not").size(10.5).weak());
            }
        });

        let body = match selected {
            Some(m) => m.text.clone(),
            None => st.text.clone(),
        };
        egui::ScrollArea::vertical()
            .id_salt("navtex-body")
            .max_height((panel_h - 40.0).max(60.0))
            .stick_to_bottom(selected.is_none() || self.navtex_open.is_none())
            .show(ui, |ui| {
                if body.trim().is_empty() {
                    ui.label(RichText::new("—").weak());
                }
                // Monospace and unwrapped: a NAVTEX message is laid out in
                // columns by the station that sent it — positions, times and
                // tables — and reflowing it destroys the only formatting it has.
                ui.label(RichText::new(body).monospace().size(11.5));
            });
    }
}
