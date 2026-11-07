//! The RDS window: what a broadcast FM station says about itself.
//!
//! Two tabs. **STATION** is the one to read — the name, the programme type, the
//! radio text, and what is playing. **DIAGNOSTICS** is the one to look at when
//! the first is empty or flickering, and answers the only question that matters
//! then: is anything actually arriving, and how much of it survives.
//!
//! Everything here is rendered from raw codes, so the RDS/RBDS selector re-labels
//! what has already been received rather than waiting for the station to send it
//! again. See [`sdroxide_types::RdsStandard`].

use crate::i18n::{Tr, tr};
use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{RdsData, RdsGroupLog, RdsStandard, pty_name, rt_plus_class};

use crate::app::SdroxideApp;
use crate::chrome::StyledCombo;

/// How many groups the diagnostics log keeps. About a minute at the eleven
/// groups a second the standard runs at — enough to watch a marginal station
/// come and go, short enough to stay scrollable.
pub const RDS_LOG_ROWS: usize = 600;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum RdsTab {
    #[default]
    Station,
    Diagnostics,
}

/// Field labels and anything the reader is not meant to look at first.
fn dim_ink() -> Color32 {
    crate::theme::gray(110)
}

impl SdroxideApp {
    /// Fold a snapshot from the engine into the window's state.
    ///
    /// The group log arrives as a delta with an absolute index, so a batch lost
    /// between here and the engine — a slow client, a reconnect — leaves a
    /// visible gap instead of silently closing up and making the log look
    /// continuous when it is not.
    pub(in crate::app) fn on_rds(&mut self, mut data: RdsData) {
        let groups = std::mem::take(&mut data.groups);
        if data.group_seq < self.rds_next_seq {
            // The decoder restarted — a retune, or a mode change. Its counter
            // went backwards, so the old log is somebody else's.
            self.rds_log.clear();
        }
        self.rds_next_seq = data.group_seq + groups.len() as u64;
        for g in groups {
            if self.rds_log.len() >= RDS_LOG_ROWS {
                self.rds_log.pop_front();
            }
            self.rds_log.push_back(g);
        }
        if !data.sync && data.is_empty() {
            // A cleared snapshot: the dial left the station.
            self.rds_log.clear();
            self.rds_next_seq = 0;
        }
        self.rds = Some(data);
    }

    /// Which table this station's codes are read against, with `Auto` settled.
    fn rds_standard(&self) -> RdsStandard {
        let (pi, ecc) = match self.rds.as_ref() {
            Some(d) => (d.pi, d.ecc),
            None => (None, None),
        };
        self.view.rds_standard.resolve(pi, ecc)
    }

    pub(in crate::app) fn rds_window(&mut self, ctx: &egui::Context) {
        if !self.show_rds {
            return;
        }
        let mut open = self.show_rds;
        let tool_title = "RDS";
        let mut tool_st = self.take_tool_state("RDS");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("RDS", tool_title, [460.0, 420.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| self.rds_body(ui),
        );
        self.put_tool_state("RDS", tool_st);
        self.show_rds = open;
    }

    fn rds_body(&mut self, ui: &mut egui::Ui) {
        let standard = self.rds_standard();
        // Read before the selector can change it, so the page below and the
        // combo above agree about the same frame.
        let chosen = self.view.rds_standard;

        crate::chrome::tab_bar(ui, |ui, bar| {
            for (tab, label) in [
                (RdsTab::Station, tr!("rds-station")),
                (RdsTab::Diagnostics, tr!("rds-diagnostics")),
            ] {
                if bar.tab(ui, self.rds_tab == tab, label).clicked() {
                    self.rds_tab = tab;
                }
            }
            // The selector is not a tab and the baseline runs on under it.
            bar.end_tabs(ui);

            // The standard selector. A display choice, not a command: the raw
            // codes are already here, so this re-labels everything at once.
            egui::ComboBox::from_id_salt("rds-standard")
                .selected_text(if chosen == RdsStandard::Auto {
                    tr!("rds-auto", standard = standard.tr())
                } else {
                    chosen.tr().to_string()
                })
                .show_styled(ui, |ui| {
                    for s in RdsStandard::ALL {
                        ui.selectable_value(&mut self.view.rds_standard, s, s.tr());
                    }
                })
                .response
                .on_hover_text(tr!("rds-which-programme-type-table-read"));
        });
        // No separator: the strip's own baseline is the line between the tabs
        // and the page they open.
        ui.add_space(8.0);

        match self.rds_tab {
            RdsTab::Station => self.rds_station(ui, standard, chosen),
            RdsTab::Diagnostics => self.rds_diagnostics(ui),
        }
    }

    /// `standard` is the resolved one, which every code is read against.
    /// `chosen` is what the operator actually selected, and is only consulted
    /// about the call sign — see [`RdsStandard::names_from_pi`].
    fn rds_station(&mut self, ui: &mut egui::Ui, standard: RdsStandard, chosen: RdsStandard) {
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());
        let Some(d) = self.rds.as_ref() else {
            ui.label(dim(&tr!("rds-waiting-receiver")));
            return;
        };

        if !d.sync && d.is_empty() {
            ui.label(dim(&if self.state.rx[0].mode == sdroxide_types::Mode::Wfm {
                tr!("rds-no-rds-station-not-every")
            } else {
                tr!("rds-rds-only-carried-wfm-broadcast")
            }));
            return;
        }

        // The station's name, big, with what it is playing under it.
        ui.horizontal(|ui| {
            let title = d.title(chosen).unwrap_or_else(|| "—".to_string());
            ui.label(RichText::new(title).size(22.0).strong());
            if !d.sync {
                ui.label(RichText::new(tr!("rds-holding")).size(9.5).color(crate::theme::ALERT()))
                    .on_hover_text(tr!("rds-block-sync-has-dropped-last"));
            }
        });

        if let Some(rt) = d.rt_plus.as_ref() {
            let now = [rt.artist.as_deref(), rt.title.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" — ");
            if !now.is_empty() {
                ui.label(RichText::new(now).size(13.0).color(crate::theme::CYAN_DIM()));
            }
        }
        ui.add_space(8.0);

        egui::ScrollArea::vertical().id_salt("rds-station").show(ui, |ui| {
            egui::Grid::new("rds_grid").num_columns(2).spacing([14.0, 4.0]).striped(true).show(
                ui,
                |ui| {
                    let row = |ui: &mut egui::Ui, k: &str, v: String| {
                        ui.label(dim(k));
                        ui.label(RichText::new(v).size(11.0));
                        ui.end_row();
                    };

                    if let Some(pi) = d.pi {
                        let mut v = format!("{pi:04X}");
                        if let Some(call) = d.call_sign(chosen) {
                            v = format!("{pi:04X}  ({call})");
                        }
                        row(ui, &tr!("rds-identity"), v);
                    }
                    if let Some(pty) = d.pty {
                        row(
                            ui,
                            &tr!("rds-programme"),
                            format!("{}  ({pty})", pty_name(pty, standard)),
                        );
                    }
                    if let Some(n) = d.ptyn.as_ref().filter(|n| !n.is_empty()) {
                        row(ui, &tr!("rds-described"), n.clone());
                    }
                    if let Some(m) = d.music {
                        row(
                            ui,
                            &tr!("rds-content"),
                            if m { tr!("rds-music") } else { tr!("rds-speech") }.to_string(),
                        );
                    }
                    row(
                        ui,
                        &tr!("rds-traffic"),
                        match (d.tp, d.ta) {
                            (_, true) => tr!("rds-announcement-air-now").to_string(),
                            (true, false) => tr!("rds-carried-station").to_string(),
                            (false, false) => tr!("rds-not-carried").to_string(),
                        },
                    );
                    if let Some(rt) = d.radiotext.as_ref().filter(|t| !t.is_empty()) {
                        ui.label(dim(&tr!("rds-radio-text")));
                        ui.label(RichText::new(rt.clone()).size(11.0));
                        ui.end_row();
                    }
                    if let Some(c) = d.clock {
                        row(ui, &tr!("rds-station-clock"), c.label());
                    }
                    if let Some(ecc) = d.ecc {
                        row(ui, &tr!("rds-country-code"), format!("{ecc:02X}"));
                    }
                    if !d.af.is_empty() {
                        let list: Vec<String> =
                            d.af.iter().map(|hz| format!("{:.1}", *hz as f64 / 1e6)).collect();
                        ui.label(dim(&tr!("rds-also")));
                        ui.label(
                            RichText::new(format!("{list} MHz", list = list.join(", "))).size(11.0),
                        );
                        ui.end_row();
                    }
                },
            );

            // Anything the station tagged beyond artist and title.
            let extra = d.rt_plus.as_ref().map(|r| r.other.clone()).unwrap_or_default();
            if !extra.is_empty() {
                ui.add_space(8.0);
                ui.label(dim(&tr!("rds-also-tagged")));
                for (class, text) in extra {
                    let name = rt_plus_class(class)
                        .map(str::to_string)
                        .unwrap_or_else(|| tr!("rds-type", class = class));
                    ui.label(RichText::new(format!("{name}: {text}")).size(10.5));
                }
            }
        });
    }

    fn rds_diagnostics(&mut self, ui: &mut egui::Ui) {
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());
        let Some(d) = self.rds.as_ref() else {
            ui.label(dim(&tr!("rds-waiting-receiver")));
            return;
        };
        let stats = d.stats;

        ui.horizontal(|ui| {
            let (label, colour) = if d.sync {
                (tr!("rds-sync"), crate::theme::CYAN_DIM())
            } else {
                (tr!("rds-no-sync"), crate::theme::ALERT())
            };
            ui.label(RichText::new(label).size(11.0).strong().color(colour));
            ui.label(dim(&tr!("rds-groups", groups = stats.groups)));
            let ber = stats.block_error_rate();
            ui.label(dim(&tr!("rds-blocks-error", ber = format!("{:.1}", ber * 100.0))))
                .on_hover_text(tr!("rds-blocks-failed-their-check-plus"));
        });
        ui.label(dim(&tr!(
            "rds-clean-corrected-lost",
            blocks_ok = stats.blocks_ok,
            blocks_corrected = stats.blocks_corrected,
            blocks_bad = stats.blocks_bad
        )));

        // The cause, when it is this one. A block error rate is a symptom, and
        // the symptom on its own sends the operator hunting for a better aerial
        // — which is the wrong direction, because the station is already too
        // strong. Nothing they can hear will tell them: the data subcarrier
        // sits 28-34 dB below peak deviation up at 57 kHz, where FM's noise is
        // some 15 dB worse than down where the programme lives, so it goes
        // while the audio is still clean and the pilot still locked.
        if self.meters.is_some_and(|m| m.adc_overloaded()) {
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr!("rds-front-end-overloading"))
                    .size(10.5)
                    .strong()
                    .color(crate::theme::ALERT()),
            )
            .on_hover_text(tr!("rds-converter-clipping-which-puts-distortion"));
        }
        ui.add_space(6.0);

        // Which group types this station actually sends. Empty columns are as
        // informative as full ones — a station with no 2A sends no radio text,
        // and that is the answer to "why is the text blank".
        let seen: Vec<String> = stats
            .group_types
            .iter()
            .enumerate()
            .filter(|&(_, &n)| n > 0)
            .map(|(i, n)| format!("{}{}×{}", i / 2, if i % 2 == 0 { 'A' } else { 'B' }, n))
            .collect();
        ui.label(dim(&tr!("rds-group-types")));
        ui.label(
            RichText::new(if seen.is_empty() { "—".to_string() } else { seen.join("  ") })
                .size(10.5)
                .monospace(),
        );
        ui.add_space(6.0);

        ui.label(dim(&tr!("rds-recent-groups-newest-last")));
        egui::ScrollArea::vertical().id_salt("rds-log").stick_to_bottom(true).show(ui, |ui| {
            for g in &self.rds_log {
                ui.label(rds_log_line(g));
            }
            if self.rds_log.is_empty() {
                ui.label(dim(&tr!("rds-nothing-decoded-yet")));
            }
        });
    }
}

/// One group as a line of hex, with the blocks that did not survive marked.
///
/// A lost block is shown as `····` rather than as the bits that arrived: those
/// bits failed their check, so printing them would put four plausible hex digits
/// on screen that mean nothing. A corrected block is shown with its value and a
/// marker, because that value *is* trustworthy — it just cost a repair.
fn rds_log_line(g: &RdsGroupLog) -> RichText {
    let mut s = String::with_capacity(40);
    for i in 0..4 {
        if g.valid & (1 << i) == 0 {
            s.push_str("····");
        } else {
            s.push_str(&format!("{:04X}", g.blocks[i]));
        }
        s.push(if g.corrected & (1 << i) != 0 { '*' } else { ' ' });
    }
    if let Some(kind) = g.kind() {
        s.push_str(&format!(" {kind}"));
    }
    let colour = if g.valid == 0b1111 && g.corrected == 0 {
        crate::theme::gray(170)
    } else if g.valid == 0b1111 {
        crate::theme::gray(130)
    } else {
        crate::theme::ALERT()
    };
    RichText::new(s).size(10.0).monospace().color(colour)
}
