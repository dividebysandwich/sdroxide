//! The DSC panel: the sequences decoded, newest first, and the one selected.
//!
//! Two panes, like NAVTEX and for the same reason. The list is a rolling view
//! of a channel that is mostly quiet, and a distress alert carries enough —
//! MMSI, nature, position, time — that it wants reading on its own rather than
//! squeezed beside a hundred other rows.
//!
//! The reading pane shows every field the parser recovered, including the ones
//! the summary line leaves out, and flags a marginal decode: DSC's BCH check
//! catches a single-bit error but not always, and a sequence that arrived with
//! a bad character is surfaced rather than dropped. That is the honest answer
//! for a distress alert — a receiver that hid a marginal one would be worse
//! than useless on the channel it exists for.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, DscFormat, DscHeard, DscStatus, Vfo};

use crate::app::SdroxideApp;
use crate::theme;

/// The DSC channels the panel offers, as dial frequencies — see
/// `panels::mode_dials`. The MF/HF service is the one a shortwave listener
/// reaches; channel 70 is FM and included for completeness.
const CHANNELS: &[(f64, &str)] = &[
    (156_523_300.0, "70"),
    (2_185_800.0, "2187.5"),
    (4_205_800.0, "4207.5"),
    (6_310_300.0, "6312"),
    (8_412_800.0, "8414.5"),
    (12_575_300.0, "12577"),
    (16_802_800.0, "16804.5"),
];

impl SdroxideApp {
    pub(in crate::app) fn dsc_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Option<DscStatus> = self.digi_status.as_ref().and_then(|s| s.dsc.clone());
        let Some(st) = st else {
            ui.label(RichText::new("starting the DSC receiver…").weak());
            return;
        };

        let pane = self.phone_pane(ui, sdroxide_types::Mode::Dsc);

        ui.horizontal(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.vertical(|ui| {
                    if pane.is_none() {
                        ui.set_width(ui.available_width() * 0.46);
                    }
                    self.dsc_list(ui, &st, cmds, panel_h);
                });
            }
            if pane.is_none() {
                ui.separator();
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| {
                    self.dsc_reading(ui, &st, panel_h);
                });
            }
        });
    }

    fn dsc_list(
        &mut self,
        ui: &mut egui::Ui,
        st: &DscStatus,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("DSC").strong().color(theme::CYAN()));
            // The audio level: DSC speaks in bursts, and the meter is how an
            // operator tells "nothing on this channel" from "nothing decoded".
            ui.add(
                egui::ProgressBar::new(st.level.clamp(0.0, 1.0))
                    .desired_width(70.0)
                    .fill(theme::CYAN_DIM()),
            )
            .on_hover_text("Audio in the decoder's passband.");
            ui.label(
                RichText::new(format!("{} seq · tone {:.0}%", st.sequences, st.separation * 100.0))
                    .size(10.0)
                    .color(theme::CYAN_DIM()),
            )
            .on_hover_text(
                "Sequences that reached their end marker, and how cleanly \
                            the two tones separate.",
            );
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
                    cmds.push(Command::SetVfo { vfo: Vfo::A, hz: *hz });
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("dsc-messages")
            .max_height((panel_h - 60.0).max(60.0))
            .show(ui, |ui| {
                if st.messages.is_empty() {
                    ui.label(
                        RichText::new(
                            "Nothing yet. DSC is quiet between vessels — leave it on a \
                             channel and wait for a burst.",
                        )
                        .weak(),
                    );
                    return;
                }
                let mut pick: Option<usize> = None;
                for (i, h) in st.messages.iter().enumerate().rev() {
                    let resp = ui.horizontal(|ui| {
                        let (_, _, _, hh, mi, ss) = sdroxide_types::utc_ymd_hms(h.at);
                        ui.label(
                            RichText::new(format!("{hh:02}:{mi:02}:{ss:02}"))
                                .monospace()
                                .size(10.0)
                                .color(theme::CYAN_DIM()),
                        );
                        let ink = if h.message.clean { theme::CYAN() } else { theme::YELLOW() };
                        ui.label(
                            RichText::new(h.message.format.label()).monospace().strong().color(ink),
                        );
                        if h.message.self_mmsi != 0 {
                            ui.label(
                                RichText::new(format!("{:09}", h.message.self_mmsi)).monospace(),
                            );
                        }
                        if h.message.format == DscFormat::Distress {
                            let nature = h.message.nature.label();
                            if !nature.is_empty() {
                                ui.label(RichText::new(nature).size(10.0).color(theme::ALERT()));
                            }
                        }
                        if !h.message.clean {
                            ui.label(RichText::new("marginal").size(9.5).color(theme::YELLOW()))
                                .on_hover_text(
                                    "A character failed its BCH check — the fields shown may be \
                                     wrong.",
                                );
                        }
                    });
                    if resp.response.interact(egui::Sense::click()).clicked() {
                        pick = Some(i);
                    }
                    resp.response.on_hover_text("Click to read the whole sequence.");
                    ui.add_space(3.0);
                }
                if let Some(i) = pick {
                    self.dsc_open = Some(i);
                }
            });
    }

    fn dsc_reading(&mut self, ui: &mut egui::Ui, st: &DscStatus, panel_h: f32) {
        let selected: Option<&DscHeard> =
            self.dsc_open.and_then(|i| st.messages.get(i)).or_else(|| st.messages.last());
        let Some(h) = selected else {
            ui.label(RichText::new("Nothing received yet.").weak());
            return;
        };
        let m = &h.message;

        ui.horizontal(|ui| {
            ui.label(RichText::new("READING").strong().color(theme::CYAN()));
            ui.label(RichText::new(m.format.label()).monospace().color(if m.clean {
                theme::CYAN()
            } else {
                theme::YELLOW()
            }));
            if !m.clean {
                ui.label(RichText::new("MARGINAL").size(10.0).color(theme::YELLOW()))
                    .on_hover_text(
                        "At least one character failed its BCH check. The sequence is shown \
                         because a distress alert that arrived imperfectly is still worth \
                         reading.",
                    );
            }
        });

        let (_, _, _, hh, mi, ss) = sdroxide_types::utc_ymd_hms(h.at);
        egui::ScrollArea::vertical()
            .id_salt("dsc-reading")
            .max_height((panel_h - 50.0).max(60.0))
            .show(ui, |ui| {
                field(ui, "Received", &format!("{hh:02}:{mi:02}:{ss:02}Z"));
                field(ui, "Format", m.format.label());
                field(ui, "Category", m.category.label());
                if m.format == DscFormat::Distress {
                    let nature = m.nature.label();
                    field(ui, "Nature", if nature.is_empty() { "—" } else { nature });
                    field(ui, "Caller MMSI", &mmsi(m.self_mmsi));
                    if let Some((lat, lon)) = m.position {
                        field(ui, "Position", &format!("{lat:.4}°, {lon:.4}°"));
                    } else if m.format == DscFormat::Distress {
                        field(ui, "Position", "not stated");
                    }
                    if let Some((th, tm)) = m.time_utc {
                        field(ui, "Alert time", &format!("{th:02}:{tm:02}Z"));
                    }
                } else {
                    field(ui, "Target MMSI", &mmsi(m.target_mmsi));
                    field(ui, "Caller MMSI", &mmsi(m.self_mmsi));
                    if m.working_channel != 0 {
                        field(ui, "Working ch", &m.working_channel.to_string());
                    }
                }
                ui.add_space(4.0);
                ui.label(RichText::new("Raw symbols").size(10.0).weak());
                // Monospace and wrapped-by-hand: the symbol run is what to look
                // at when a decode is marginal, so it is shown as it came in.
                ui.label(
                    RichText::new(
                        m.raw_symbols.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" "),
                    )
                    .monospace()
                    .size(10.5),
                );
            });
    }
}

fn mmsi(v: u64) -> String {
    if v == 0 { "(none)".to_string() } else { format!("{v:09}") }
}

fn field(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{label:>11}")).size(10.0).weak().monospace());
        ui.label(RichText::new(value).monospace());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{DSC_TONE_HZ, Mode};

    /// A DSC channel is quoted as the centre of the 1615/1785 Hz MF/HF tone pair, and
    /// the receiver is upper sideband, so the dial sits `DSC_TONE_HZ` below it:
    /// 2187.5 kHz is tuned at 2185.8. Both chip lists must agree with that, and
    /// with each other, or one of them tunes the tones 1.7 kHz out of the
    /// decoder's passband.
    #[test]
    fn every_hf_channel_is_tuned_1700_hz_below_its_assigned_frequency() {
        let hf = |c: &&(f64, &str)| c.0 < 30e6;
        let panel: Vec<_> = CHANNELS.iter().filter(hf).collect();
        assert_eq!(panel.len(), 6, "2187.5, 4207.5, 6312, 8414.5, 12577, 16804.5");
        for (dial, label) in panel {
            let channel_hz = label.parse::<f64>().unwrap() * 1000.0;
            assert!(
                (dial - (channel_hz - f64::from(DSC_TONE_HZ))).abs() < 0.5,
                "{label} kHz is tuned at {dial} Hz, not {} Hz",
                channel_hz - f64::from(DSC_TONE_HZ)
            );
            let (lo, hi) = Mode::Dsc.default_filter_at(*dial);
            assert!(lo > 0.0 && lo < 1300.0 && hi > 2100.0, "{label}: passband {lo}..{hi}");
        }
        let menu: Vec<_> = super::super::digi_dial_freqs(Mode::Dsc).iter().map(|c| c.1).collect();
        let chips: Vec<_> = CHANNELS.iter().map(|c| c.0).collect();
        assert_eq!(menu, chips, "the band menu and the DSC panel tune different dials");
    }

    /// The logged frequency is the channel, not the dial.
    #[test]
    fn the_channel_is_the_dial_plus_the_tone_pair_centre() {
        let dial = 2_185_800.0;
        assert_eq!(Mode::Dsc.on_air_hz(dial, DSC_TONE_HZ), 2_187_500.0);
    }
}
