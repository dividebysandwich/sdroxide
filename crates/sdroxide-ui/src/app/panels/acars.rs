//! The ACARS panel: the airline datalink frames decoded, newest first.
//!
//! One list, because that is the whole shape of the mode: a frame is a single
//! self-contained burst of a second or two, complete in itself, and there is no
//! session to reassemble and nothing being copied *now* to pin beside it. What
//! a listener watches is the channel's level between bursts and the addresses
//! and text as they arrive.
//!
//! The channel row is the one control. ACARS lives on a handful of shared
//! airband frequencies and an operator moves between them by hand all evening,
//! so the published ones are chips rather than something to type into the dial.

use eframe::egui::{self, RichText};
use sdroxide_types::{AcarsStatus, Command, Vfo};

use crate::app::SdroxideApp;
use crate::theme;

/// The airline channels ACARS is commonly found on. Widely published: 131.550
/// is the primary almost everywhere, Europe works 131.525, 131.725 and 131.825
/// (the three acarsdec's own examples listen to), and the rest are North
/// American.
const CHANNELS: &[(u32, &str)] = &[
    (129_125_000, "129.125"),
    (130_025_000, "130.025"),
    (130_450_000, "130.450"),
    (131_125_000, "131.125"),
    (131_525_000, "131.525"),
    (131_550_000, "131.550"),
    (131_725_000, "131.725"),
    (131_825_000, "131.825"),
    (136_700_000, "136.700"),
];

impl SdroxideApp {
    pub(in crate::app) fn acars_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<AcarsStatus> = self.digi_status.as_ref().and_then(|s| s.acars.clone());
        let Some(st) = st else {
            ui.label(RichText::new("starting the ACARS receiver…").weak());
            return;
        };

        ui.horizontal(|ui| {
            ui.label(RichText::new("ACARS").strong().color(theme::CYAN()));
            // The audio level: ACARS speaks in bursts, and the meter is how an
            // operator tells "nothing on this channel" from "nothing decoded".
            ui.add(
                egui::ProgressBar::new(st.level.clamp(0.0, 1.0))
                    .desired_width(70.0)
                    .fill(theme::CYAN_DIM()),
            )
            .on_hover_text("Audio in the decoder's passband.");
            ui.label(
                RichText::new(format!("{} frames · {} bad", st.frames, st.bad))
                    .size(10.0)
                    .color(if st.bad > 0 { theme::YELLOW() } else { theme::CYAN_DIM() }),
            )
            .on_hover_text("Blocks that framed, and blocks whose check sequence failed.");
            self.clear_rx_chip(ui, cmds);
            self.save_rx_chip(ui);
        });

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("CHANNEL").size(10.0).weak());
            for (hz, label) in CHANNELS {
                if crate::chrome::chip(ui, false, RichText::new(*label).size(10.0))
                    .on_hover_text("Tune the dial here.")
                    .clicked()
                {
                    cmds.push(Command::SetVfo { vfo: Vfo::A, hz: *hz as f64 });
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("acars-frames")
            .max_height((panel_h - 60.0).max(60.0))
            .show(ui, |ui| {
                if st.messages.is_empty() {
                    ui.label(
                        RichText::new(
                            "Nothing yet. ACARS is quiet between aircraft — leave it on a \
                             channel and wait for a burst.",
                        )
                        .weak(),
                    );
                    return;
                }
                for m in st.messages.iter().rev() {
                    ui.horizontal(|ui| {
                        let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(m.at);
                        ui.label(
                            RichText::new(format!("{h:02}:{mi:02}:{s:02}"))
                                .monospace()
                                .size(10.0)
                                .color(theme::CYAN_DIM()),
                        );
                        ui.label(
                            RichText::new(&m.address).monospace().strong().color(theme::CYAN()),
                        );
                        if !m.mode.is_empty() {
                            ui.label(RichText::new(format!("mode {}", m.mode)).size(9.5).weak());
                        }
                        if !m.label.is_empty() {
                            ui.label(RichText::new(&m.label).monospace().size(10.0).weak());
                        }
                        if !m.ack.is_empty() {
                            ui.label(RichText::new(&m.ack).monospace().size(9.5).weak());
                        }
                        if !m.block_id.is_empty() {
                            ui.label(RichText::new(&m.block_id).size(9.5).weak());
                        }
                        if !m.crc_ok {
                            ui.label(
                                RichText::new("check failed").size(9.5).color(theme::YELLOW()),
                            )
                            .on_hover_text(
                                "The block-check sequence did not match — text may be wrong.",
                            );
                        }
                    });
                    if m.text.trim().is_empty() {
                        ui.label(RichText::new("(no text)").size(10.0).weak());
                    } else {
                        // Unwrapped: an ACARS message is columns and line
                        // breaks as the sender wrote them.
                        ui.label(RichText::new(&m.text).monospace().size(11.5));
                    }
                    ui.add_space(3.0);
                }
            });
    }
}
