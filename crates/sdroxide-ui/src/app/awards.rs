//! The awards dashboard: DXCC, WAS, WAZ and grid tallies over the logbook.
//!
//! The tally is expensive enough to cache (keyed by log length and the band
//! filter) and is shared with the 3D view's award layer by `Arc` rather than
//! copied, because the window republishes it every frame.

// The globe's award layer is native-only, and it is the sole user here.
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

use eframe::egui::{self, RichText};

use crate::theme::ThemedScroll;

use crate::app::SdroxideApp;

/// One "N worked / M confirmed" summary line for an award map.
fn award_summary<K>(
    ui: &mut egui::Ui,
    name: &str,
    map: &std::collections::BTreeMap<K, sdroxide_types::AwardStatus>,
) {
    let (w, c) = sdroxide_types::counts(map);
    ui.horizontal(|ui| {
        ui.add_sized([90.0, 20.0], egui::Label::new(RichText::new(name).strong()));
        ui.label(RichText::new(format!("{w} worked")).color(crate::theme::YELLOW()).monospace());
        ui.label(RichText::new(format!("{c} confirmed")).color(crate::theme::GREEN()).monospace());
    });
}

/// A wrapping grid of fixed-width award cells: grey = not worked, amber =
/// worked, green = confirmed.
fn award_cell_grid(
    ui: &mut egui::Ui,
    items: impl Iterator<Item = (String, sdroxide_types::AwardStatus)>,
    cell_w: f32,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for (label, st) in items {
            let (bg, fg) = if st.confirmed {
                (crate::theme::GREEN(), crate::theme::INK_ON_BRIGHT())
            } else if st.worked {
                (crate::theme::YELLOW(), crate::theme::INK_ON_BRIGHT())
            } else {
                (crate::theme::gray(38), crate::theme::gray(110))
            };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(cell_w, 20.0), egui::Sense::hover());
            let p = ui.painter_at(rect);
            p.rect_filled(rect, 3.0, bg);
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::monospace(11.0),
                fg,
            );
        }
    });
}

impl SdroxideApp {
    /// The cached award tally for the current band filter (recomputed when the
    /// log length or the band filter changes).
    pub(in crate::app) fn ensure_awards(&mut self) {
        let len = self.qso_log.len();
        let band = self.awards_band.clone();
        let stale =
            self.awards_cache.as_ref().map(|(l, b, _)| *l != len || *b != band).unwrap_or(true);
        if stale {
            let filter = (!band.is_empty()).then_some(band.as_str());
            let awards = sdroxide_types::compute_awards(&self.qso_log, filter, None);
            self.awards_cache = Some((len, band, awards));
        }
    }

    /// Award coverage placed on the globe, for the 3D view's award layer. Built
    /// from the same tally the dashboard shows and cached the same way, so the
    /// two can never tell different stories about the same log.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn award_heat(&mut self) -> Arc<Vec<sdroxide_types::EntitySlot>> {
        let len = self.qso_log.len();
        let band = self.awards_band.clone();
        let stale =
            self.awards_heat.as_ref().map(|(l, b, _)| *l != len || *b != band).unwrap_or(true);
        if stale {
            self.ensure_awards();
            let slots = self
                .awards_cache
                .as_ref()
                .map(|(_, _, a)| sdroxide_types::entity_coverage(a))
                .unwrap_or_default();
            self.awards_heat = Some((len, band, Arc::new(slots)));
        }
        Arc::clone(&self.awards_heat.as_ref().expect("just filled").2)
    }

    pub(in crate::app) fn awards_window(&mut self, ctx: &egui::Context) {
        crate::layout::arm_center_on_open(
            self.show_awards,
            &mut self.system_was_open.awards,
            &mut self.system_center_pending.awards,
        );
        if !self.show_awards && !self.system_docked_as(crate::app::SystemDock::Awards) {
            return;
        }
        self.ensure_awards();
        let bands =
            ["", "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m"];
        let mut open = self.show_awards;
        let mut new_band: Option<String> = None;
        let awards = self.awards_cache.as_ref().map(|(_, _, a)| a.clone()).unwrap_or_default();
        let size = egui::vec2(
            crate::layout::window_w(ctx, 540.0),
            crate::layout::window_h(ctx, 560.0),
        );
        let id = crate::layout::salted_id(ctx, "AWARDS");
        if self.system_docked_as(crate::app::SystemDock::Awards) {
            if let Some(win) = self.system_dock_host("AWARDS", id, crate::app::SystemDock::Awards) {
                win.show(ctx, |ui| {
                    crate::chrome::window_body_bg(ui);
                    self.clip_system_dock_body(ui, crate::app::SystemDock::Awards);
                    self.awards_body(ui, &bands, &awards, &mut new_band);
                });
            }
            if let Some(b) = new_band {
                self.awards_band = b;
            }
            return;
        }
        let pending = self.system_center_pending.awards;
        let resp = crate::layout::center_on_open(
            egui::Window::new("AWARDS")
                .id(id)
                .open(&mut open)
                .frame(crate::chrome::window_frame())
                .resizable(true)
                .default_width(size.x)
                .default_height(size.y),
            ctx,
            pending,
            size,
        )
        .show(ctx, |ui| {
            crate::chrome::window_body_bg(ui);
            self.awards_body(ui, &bands, &awards, &mut new_band);
        });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
            self.system_center_pending.awards = false;
        }
        if resp
            .as_ref()
            .is_some_and(|r| self.system_title_pin(ctx, &r.response))
        {
            self.dock_system(crate::app::SystemDock::Awards);
        }
        self.show_awards = open;
        if let Some(b) = new_band {
            self.awards_band = b;
        }
    }

    fn awards_body(
        &mut self,
        ui: &mut egui::Ui,
        bands: &[&str],
        awards: &sdroxide_types::Awards,
        new_band: &mut Option<String>,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Band").size(11.0).color(crate::theme::gray(150)));
            for b in bands {
                let label = if b.is_empty() { "All" } else { b };
                if crate::chrome::chip(ui, self.awards_band == *b, label).clicked() {
                    *new_band = Some((*b).to_string());
                }
            }
        });
        ui.separator();
        // Summary counts.
        award_summary(ui, "DXCC", &awards.dxcc);
        award_summary(ui, "WAZ", &awards.waz);
        award_summary(ui, "WAS", &awards.was);
        award_summary(ui, "Grids", &awards.grids);
        ui.add_space(6.0);

        egui::ScrollArea::vertical().auto_shrink([false, false]).show_themed(ui, |ui| {
            // WAS state grid.
            ui.label(
                RichText::new("Worked All States")
                    .size(12.0)
                    .strong()
                    .color(crate::theme::CYAN()),
            );
            award_cell_grid(
                ui,
                sdroxide_types::US_STATES.iter().map(|s| {
                    (s.to_string(), awards.was.get(*s).copied().unwrap_or_default())
                }),
                44.0,
            );
            ui.add_space(8.0);
            // WAZ zone grid (1..40).
            ui.label(
                RichText::new("CQ Zones (WAZ)")
                    .size(12.0)
                    .strong()
                    .color(crate::theme::CYAN()),
            );
            award_cell_grid(
                ui,
                (1u8..=40).map(|z| {
                    (format!("{z:02}"), awards.waz.get(&z).copied().unwrap_or_default())
                }),
                34.0,
            );
            ui.add_space(8.0);
            // DXCC worked list (confirmed marked).
            ui.label(
                RichText::new("DXCC entities")
                    .size(12.0)
                    .strong()
                    .color(crate::theme::CYAN()),
            );
            for (name, st) in &awards.dxcc {
                let col = if st.confirmed {
                    crate::theme::GREEN()
                } else {
                    crate::theme::YELLOW()
                };
                ui.label(
                    RichText::new(format!(
                        "{} {name}",
                        if st.confirmed { "✓" } else { "•" }
                    ))
                    .size(11.5)
                    .color(col),
                );
            }
        });
    }
}
