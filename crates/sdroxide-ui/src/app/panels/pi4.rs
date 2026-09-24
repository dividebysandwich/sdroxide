//! The PI4 panel: beacon receptions, and where the one-minute cycle is.
//!
//! Two panes, unlike WSPR's three: `SPOTS` is the reception list and `STATUS`
//! is the cycle clock. There is no `MAP`, because a PI4 message carries no
//! grid square — see `sdroxide_types::Pi4Spot`'s doc comment — so there is no
//! path to place on one. And, as with WSPR, there is no QSO pane: PI4 is a
//! beacon, and this decoder only ever receives one.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Command, Mode, Pi4Spot};

use crate::app::SdroxideApp;
use crate::app::panels::widgets::{SlotState, row_cell, slot_bar, slot_phase_s};
use crate::app::util::fmt_age;
use crate::theme::ThemedScroll;
use crate::time::{now_unix, now_unix_f64};

/// How many receptions the panel keeps. A PI4 beacon is heard at most once a
/// minute, so this is a day and a half of them — generous against WSPR's
/// [`crate::app::panels::wspr::WSPR_SPOT_ROWS`] because the arrival rate is a
/// sixtieth of it.
pub(in crate::app) const PI4_SPOT_ROWS: usize = 2_000;

const PI4_ROW_H: f32 = 19.0;

/// See [`crate::app::panels::wspr::wspr_row_slot`]'s doc comment — the same
/// reasoning, the same fix.
fn pi4_row_slot(ui: &egui::Ui) -> f32 {
    PI4_ROW_H.max(ui.spacing().interact_size.y) + ui.spacing().item_spacing.y
}

impl SdroxideApp {
    pub(in crate::app) fn pi4_panel(
        &mut self,
        ui: &mut egui::Ui,
        _cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let pane = self.digi_pane(Mode::Pi4);
        let phone = crate::layout::tier(ui.ctx()) == crate::layout::Tier::Phone;
        const TWO_COLUMN_MIN_W: f32 = 420.0;
        if phone || ui.available_width() < TWO_COLUMN_MIN_W {
            match pane {
                0 => self.pi4_spot_list(ui),
                _ => self.pi4_status_pane(ui),
            }
            return;
        }
        let total_w = ui.available_width();
        let left_w =
            (total_w * self.view.digi_split_fraction).clamp(240.0, (total_w - 200.0).max(240.0));
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(left_w);
                ui.set_height(panel_h);
                self.pi4_spot_list(ui);
            });
            let resp = crate::chrome::split_handle(ui, egui::vec2(7.0, panel_h), None);
            if resp.dragged() {
                let dx = resp.drag_delta().x;
                self.view.digi_split_fraction = ((left_w + dx) / total_w).clamp(0.25, 0.75);
            }
            ui.vertical(|ui| {
                ui.set_height(panel_h);
                egui::ScrollArea::vertical()
                    .id_salt("pi4-status")
                    .auto_shrink([false, false])
                    .show_themed(ui, |ui| self.pi4_status_pane(ui));
            });
        });
    }

    fn pi4_spot_list(&mut self, ui: &mut egui::Ui) {
        let now = now_unix();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("RECEPTIONS").size(9.5).strong().color(crate::theme::CYAN_DIM()),
            );
            crate::chrome::row_tail(ui, |ui| {
                ui.label(
                    RichText::new(format!("{} rx", self.pi4_spots.len()))
                        .size(10.0)
                        .color(crate::theme::gray(120)),
                );
                crate::app::panels::save_text_chip(
                    ui,
                    !self.pi4_spots.is_empty(),
                    "sdroxide-pi4-spots.csv",
                    "Save the PI4 reception list as CSV",
                    || crate::app::save_text::pi4_spots_csv(&self.pi4_spots),
                );
            });
        });

        let spots = std::mem::take(&mut self.pi4_spots);
        egui::ScrollArea::vertical()
            .id_salt("pi4-receptions")
            .max_height(ui.available_height())
            .auto_shrink([false, false])
            .show_rows_themed(ui, pi4_row_slot(ui), spots.len(), |ui, rows| {
                for s in &spots[rows] {
                    pi4_row(ui, s, now);
                }
            });
        self.pi4_spots = spots;
    }

    fn pi4_status_pane(&mut self, ui: &mut egui::Ui) {
        let status = self.digi_status.clone();
        let p = status.as_ref().and_then(|s| s.pi4.clone()).unwrap_or_default();

        let timing = Mode::Pi4.slot_timing().expect("PI4 is slotted");
        let into_slot = slot_phase_s(now_unix_f64(), timing.slot_s, p.slot_utc);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("PI4").size(11.0).strong().color(crate::theme::CYAN()));
            ui.label(
                RichText::new("Next Generation Beacon").size(10.5).color(crate::theme::CYAN_DIM()),
            );
            crate::chrome::row_tail(ui, |ui| {
                let left = (timing.slot_s - into_slot).max(0.0).round() as i64;
                ui.label(
                    // Minutes and seconds the way `wspr.rs` writes them, not a
                    // hard-coded "0:" — a full minute left is 1:00, and a
                    // one-minute cycle reads that for the first half second of
                    // every slot.
                    RichText::new(format!("{}:{:02}", left / 60, left % 60))
                        .size(10.5)
                        .color(crate::theme::gray(140)),
                )
                .on_hover_text(
                    "Time left in this one-minute beacon cycle. The PI4 message is searched for \
                     once the window has had time to fill past where it would end.",
                );
                if p.decoding {
                    ui.label(RichText::new("decoding…").size(10.5).color(crate::theme::YELLOW()));
                }
            });
        });
        ui.add_space(4.0);
        slot_bar(
            ui,
            timing,
            into_slot,
            if p.decoding { SlotState::Decoding } else { SlotState::Listening },
        );
        ui.add_space(6.0);

        crate::chrome::red_panel(ui, |ui| {
            let dial = self.state.rx_freq_hz();
            row(ui, "Band", &format!("{:.6} MHz", dial / 1e6));
            row(
                ui,
                "Last cycle",
                &if p.decoding {
                    "decoding…".to_string()
                } else if p.last_slot_spots == 0 {
                    "nothing heard".to_string()
                } else {
                    format!(
                        "{} beacon{}",
                        p.last_slot_spots,
                        if p.last_slot_spots == 1 { "" } else { "s" }
                    )
                },
            );
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Receive only. Tune so the beacon's CW identification sits at 800 Hz audio — the \
                 network's own convention — and the four PI4 tones fall where this decoder \
                 searches for them by default.",
            )
            .size(9.5)
            .color(crate::theme::gray(120)),
        );
    }
}

fn pi4_row(ui: &mut egui::Ui, s: &Pi4Spot, now: i64) {
    egui::Frame::new()
        .fill(crate::theme::ROW_BG())
        .inner_margin(egui::Margin { left: 6, right: 6, top: 3, bottom: 3 })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.set_min_height(PI4_ROW_H);
                ui.spacing_mut().item_spacing.x = 5.0;
                row_cell(
                    ui,
                    74.0,
                    PI4_ROW_H,
                    false,
                    egui::Label::new(
                        RichText::new(&s.text)
                            .size(11.0)
                            .strong()
                            .color(crate::theme::TEXT_STRONG()),
                    )
                    .truncate(),
                );
                row_cell(
                    ui,
                    52.0,
                    PI4_ROW_H,
                    false,
                    egui::Label::new(
                        RichText::new(&s.variant).size(9.5).color(crate::theme::CYAN_DIM()),
                    ),
                );
                row_cell(
                    ui,
                    46.0,
                    PI4_ROW_H,
                    true,
                    egui::Label::new(
                        RichText::new(format!("{:+.0} dB", s.snr_db))
                            .size(10.5)
                            .strong()
                            .color(fit_color(s.fit)),
                    ),
                );
                // `fit`: how much of the received tone energy this message
                // accounts for. There is no checksum on a PI4 message, so this —
                // not a bare pass/fail — is what says the decode is real rather
                // than a well-formed guess out of noise.
                row_cell(
                    ui,
                    40.0,
                    PI4_ROW_H,
                    true,
                    egui::Label::new(
                        RichText::new(format!("{:.2}", s.fit))
                            .size(9.5)
                            .color(crate::theme::gray(150)),
                    ),
                );
                crate::chrome::row_tail(ui, |ui| {
                    let t = s.slot_utc.rem_euclid(86_400);
                    ui.label(
                        RichText::new(format!("{:02}:{:02}", t / 3600, (t % 3600) / 60))
                            .size(9.5)
                            .monospace()
                            .color(crate::theme::gray(120)),
                    )
                    .on_hover_text(format!(
                        "{:02}:{:02} UTC — {} ago",
                        t / 3600,
                        (t % 3600) / 60,
                        fmt_age(now - s.slot_utc)
                    ));
                });
            });
        });
}

/// Colour a reception by [`Pi4Spot::fit`] rather than by its SNR estimate:
/// every row shown here already cleared the decoder's fit floor, so this is
/// about how much margin it cleared it by, not whether it did.
fn fit_color(fit: f32) -> Color32 {
    if fit >= 0.6 {
        crate::theme::GREEN()
    } else if fit >= 0.35 {
        crate::theme::CYAN()
    } else {
        crate::theme::YELLOW()
    }
}

/// A label/value line in the status card — see
/// [`crate::app::panels::wspr::row`], which this copies rather than shares:
/// `wspr.rs`'s is private to that module.
fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.0).color(crate::theme::CYAN_DIM()));
        crate::chrome::row_tail(ui, |ui| {
            ui.label(RichText::new(value).size(10.5).color(crate::theme::TEXT()));
        });
    });
}
