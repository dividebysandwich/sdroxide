//! The UVPacket panel: the frames decoded, newest first, and the one selected.
//!
//! Two panes, like DSC and for the same reason. The list is a rolling view of a
//! channel that is mostly quiet, and a frame carries an application tag and a
//! payload whose reading wants the room: a private group may be sending a
//! sentence, a JSON blob or a packed binary struct, and only the frame's own
//! width tells you which.
//!
//! The reading pane shows the header fields the summary line leaves out and the
//! payload two ways — as text when every byte is printable, and always as hex.
//! Guessing which one a listener wanted would be worse than showing both.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Mode, UvPacketFrame, UvPacketStatus};

use crate::app::SdroxideApp;
use crate::theme;

impl SdroxideApp {
    pub(in crate::app) fn uvpacket_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<UvPacketStatus> = self.digi_status.as_ref().and_then(|s| s.uvpacket.clone());
        let Some(st) = st else {
            ui.label(RichText::new("starting the UVPacket receiver…").weak());
            return;
        };

        let pane = self.phone_pane(ui, Mode::UvPacket);

        ui.horizontal(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        ui.set_width(ui.available_width() * 0.5);
                    }
                    self.uvpacket_list(ui, &st, cmds, panel_h);
                });
            }
            if pane.is_none() {
                ui.separator();
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    self.uvpacket_reading(ui, &st, panel_h);
                });
            }
        });
    }

    fn uvpacket_list(
        &mut self,
        ui: &mut egui::Ui,
        st: &UvPacketStatus,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("UVPACKET").strong().color(theme::CYAN()));
            // The audio level: a packet channel is bursts between long silences,
            // and the meter is how an operator tells "nothing on this channel"
            // from "nothing decoded".
            ui.add(
                egui::ProgressBar::new(st.level.clamp(0.0, 1.0))
                    .desired_width(70.0)
                    .fill(theme::CYAN_DIM()),
            )
            .on_hover_text("Audio in the decoder's passband.");
            ui.label(
                RichText::new(format!("{} frames", st.frames_total))
                    .size(10.0)
                    .color(theme::CYAN_DIM()),
            )
            .on_hover_text("Frames decoded since the receiver started.");
            self.clear_rx_chip(ui, cmds);
        });

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(
                    "A packet mode for private VHF/UHF groups. Tune the dial to \
                     the group's channel; the sub-mode is detected from each \
                     frame.",
                )
                .size(10.0)
                .weak(),
            );
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("uvpacket-frames")
            .max_height((panel_h - 60.0).max(60.0))
            .show(ui, |ui| {
                if st.frames.is_empty() {
                    ui.label(
                        RichText::new(
                            "Nothing yet. UVPacket is quiet between transmissions — \
                             leave it on a channel and wait for a burst.",
                        )
                        .weak(),
                    );
                    return;
                }
                let mut pick: Option<usize> = None;
                for (i, f) in st.frames.iter().enumerate().rev() {
                    let resp = ui.horizontal(|ui| {
                        let (_, _, _, hh, mi, ss) = sdroxide_types::utc_ymd_hms(f.at);
                        ui.label(
                            RichText::new(format!("{hh:02}:{mi:02}:{ss:02}"))
                                .monospace()
                                .size(10.0)
                                .color(theme::CYAN_DIM()),
                        );
                        ui.label(
                            RichText::new(f.mode.label()).monospace().strong().color(theme::CYAN()),
                        );
                        ui.label(
                            RichText::new(format!("app {} · seq {}", f.app_type, f.sequence))
                                .monospace()
                                .size(10.0),
                        );
                        ui.label(
                            RichText::new(format!("{} B", f.payload.len()))
                                .monospace()
                                .size(10.0)
                                .color(theme::CYAN_DIM()),
                        );
                        ui.label(
                            RichText::new(format!("{:+} dB", f.snr_db))
                                .monospace()
                                .size(10.0)
                                .color(theme::CYAN_DIM()),
                        );
                        ui.label(RichText::new(preview(f)).size(10.0).weak());
                    });
                    if resp.response.interact(egui::Sense::click()).clicked() {
                        pick = Some(i);
                    }
                    resp.response.on_hover_text("Click to read the whole frame.");
                    ui.add_space(3.0);
                }
                if let Some(i) = pick {
                    self.uvpacket_open = Some(i);
                }
            });
    }

    fn uvpacket_reading(&mut self, ui: &mut egui::Ui, st: &UvPacketStatus, panel_h: f32) {
        let selected: Option<&UvPacketFrame> =
            self.uvpacket_open.and_then(|i| st.frames.get(i)).or_else(|| st.frames.last());
        let Some(f) = selected else {
            ui.label(RichText::new("Nothing received yet.").weak());
            return;
        };

        ui.horizontal(|ui| {
            ui.label(RichText::new("FRAME").strong().color(theme::CYAN()));
            ui.label(RichText::new(f.mode.label()).monospace().color(theme::CYAN()));
            ui.label(
                RichText::new(format!("{} bps net", f.mode.net_bps()))
                    .size(10.0)
                    .color(theme::CYAN_DIM()),
            );
        });

        let (_, _, _, hh, mi, ss) = sdroxide_types::utc_ymd_hms(f.at);
        egui::ScrollArea::vertical()
            .id_salt("uvpacket-reading")
            .max_height((panel_h - 50.0).max(60.0))
            .show(ui, |ui| {
                field(ui, "Received", &format!("{hh:02}:{mi:02}:{ss:02}Z"));
                field(ui, "Sub-mode", f.mode.label());
                field(ui, "App type", &f.app_type.to_string());
                field(ui, "Sequence", &f.sequence.to_string());
                field(ui, "Blocks", &f.block_count.to_string());
                field(ui, "Payload", &format!("{} bytes", f.payload.len()));
                field(ui, "SNR", &format!("{:+} dB", f.snr_db));

                ui.add_space(4.0);
                if let Some(text) = f.as_text() {
                    ui.label(RichText::new("Text").size(10.0).weak());
                    ui.label(RichText::new(text).monospace().size(10.5));
                    ui.add_space(4.0);
                }
                ui.label(RichText::new("Hex").size(10.0).weak());
                ui.label(RichText::new(hex_pairs(&f.payload)).monospace().size(10.5));
            });
    }
}

/// A short one-line preview of a frame's payload for the list: the text when it
/// is printable, otherwise the leading bytes in hex.
fn preview(f: &UvPacketFrame) -> String {
    match f.as_text() {
        Some(t) => {
            let one_line: String =
                t.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
            let mut s: String = one_line.chars().take(48).collect();
            if one_line.chars().count() > 48 {
                s.push('…');
            }
            s
        }
        None => {
            let mut s = String::new();
            for (i, b) in f.payload.iter().take(8).enumerate() {
                use std::fmt::Write as _;
                if i > 0 {
                    s.push(' ');
                }
                let _ = write!(s, "{b:02x}");
            }
            if f.payload.len() > 8 {
                s.push_str(" …");
            }
            s
        }
    }
}

/// Bytes as space-separated hex pairs, so the reading pane's dump is legible.
fn hex_pairs(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        use std::fmt::Write as _;
        if i > 0 {
            s.push(' ');
        }
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn field(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{label:>11}")).size(10.0).weak().monospace());
        ui.label(RichText::new(value).monospace());
    });
}
