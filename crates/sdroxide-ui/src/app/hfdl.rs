//! The HFDL ground-network decoder's window.
//!
//! The choice the other not-a-mode lanes put in a window — enable, tune, then
//! read the decode log — but for one channel instead of a sketch map: HFDL
//! ground stations are spread across the shortwave band one channel each, so
//! the panel is a frequency, a run switch, and the rolling log of what the
//! channel decoded. State comes from [`sdroxide_types::HfdlStatus`] (the
//! engine's re-sent snapshot), settings from [`sdroxide_types::HfdlSettings`]
//! edited as a copy and diffed at the end — the same apply-by-diff convention
//! the AIS and ADS-B windows keep, because the engine echoes the accepted
//! config back and the two copies would otherwise drift.
//!
//! The window is split: the decode log on the left, and the aircraft map on the
//! right — every position a downlink has carried, kept per aircraft by
//! [`crate::hfdl_map`]. The divider is the same draggable handle the ADS-B
//! panel uses, and where it sits is remembered with the rest of the view state.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, HfdlDecode, HfdlStatus, Mode};

use crate::app::SdroxideApp;

/// The HFDL assigned-frequency plan, in kHz — the channels the ground-network
/// stations transmit on, ARINC/AviaSat's published allocation. Riverhead's
/// 21 931 kHz primary is first: it is the default channel and the reference
/// off-air capture that validated the decoder was recorded on it.
const HFDL_PLAN_KHZ: &[u32] = &[
    21_931, 2_941, 2_992, 3_455, 3_917, 5_451, 6_562, 6_640, 8_843, 10_060, 10_084, 11_184, 11_387,
    13_306, 13_309, 13_312, 13_315, 13_318, 17_904, 17_910, 17_913, 17_916, 17_919, 17_922, 17_925,
    21_934, 21_940,
];

/// The HFDL panel: run switch, channel, and the split decode log / aircraft
/// map — docked under the waterfall like the other lane panels (ADS-B, AIS).
impl SdroxideApp {
    pub(in crate::app) fn hfdl_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let now = crate::time::now_unix();
        // Edited as a copy and diffed at the end, the way the AIS panel does: the
        // engine echoes the accepted settings back, so there is no apply step and
        // no way for the two copies to drift.
        let mut cfg = self.state.hfdl;
        let content_bottom = ui.cursor().top() + panel_h - 4.0;

        hfdl_status_strip(ui, self.hfdl_status.as_ref(), self.hfdl_map.len());
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            if crate::chrome::chip_enabled(ui, true, cfg.enabled, "LISTEN")
                .on_hover_text(
                    "Switches the engine's HFDL downconverter and decoder on or \
                             off. Decoding costs a 24 kHz lane and a worker thread whether \
                             the window is open or not.",
                )
                .clicked()
            {
                cfg.enabled = !cfg.enabled;
            }
            let mut khz = (cfg.frequency_hz / 1e3).round();
            let old_khz = khz;
            ui.add(
                egui::DragValue::new(&mut khz).range(2_800.0..=30_000.0).speed(1.0).suffix(" kHz"),
            )
            .on_hover_text(
                "The channel (the assigned frequency) to listen on. Anything in the \
                         band works — HFDL stations transmit on 2.8 to 22 MHz — against a \
                         fixed 24 kHz lane centred here.",
            );
            if khz != old_khz {
                cfg.frequency_hz = khz * 1e3;
            }
        });
        ui.add_space(4.0);

        // The plan as pick chips, so a spot of a few kHz on the band is a
        // single click away rather than a DragValue spin.
        ui.horizontal_wrapped(|ui| {
            for &khz in HFDL_PLAN_KHZ {
                let on = (cfg.frequency_hz / 1e3).round() as u32 == khz;
                if crate::chrome::chip(ui, on, format!("{:.3} M", khz as f32 / 1e3))
                    .on_hover_text(if khz == 21_931 {
                        "Riverhead (northern Atlantic/north America) — the default, \
                                 and the channel the decoder was validated on."
                    } else {
                        "An HFDL assigned frequency."
                    })
                    .clicked()
                {
                    cfg.frequency_hz = f64::from(khz) * 1e3;
                }
            }
        });

        // Selecting the mode builds the lane and lets the waterfall and the
        // level meter run whether or not the decoder is switched on, so a
        // station can sit on a real channel with a live picture and get nothing
        // — and the only other clue is a small word in the status strip. Say it
        // plainly, where the controls that fix it are (reported on issue #497:
        // "bons signaux, aucune décode").
        if !cfg.enabled {
            let ink = crate::theme::HAZARD();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("⚠").size(13.0).color(ink));
                ui.label(
                    RichText::new("HFDL is selected but decoding is off — press LISTEN above")
                        .size(11.5)
                        .color(ink),
                );
            });
        }
        ui.add_space(4.0);

        ui.separator();

        // The log keeps a draggable share of the width; the rest is the map, as
        // in the ADS-B panel. Floors so neither pane can be dragged away to
        // nothing. On a phone the pane chips choose one or the other.
        let avail_h = (content_bottom - ui.cursor().top()).max(90.0);
        let pane = self.phone_pane(ui, Mode::Hfdl);
        let full_w = ui.available_width();
        const HANDLE_W: f32 = 7.0;
        let log_w = (full_w * self.view.hfdl_split_fraction)
            .clamp(220.0, (full_w - HANDLE_W - 180.0).max(220.0));

        ui.horizontal_top(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(if pane.is_some() { full_w } else { log_w }, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.hfdl_log_pane(ui, avail_h),
                );
            }
            if pane.is_none() {
                let h = crate::chrome::split_handle(ui, egui::vec2(HANDLE_W, avail_h), None);
                if h.dragged() {
                    self.view.hfdl_split_fraction =
                        ((log_w + h.drag_delta().x) / full_w.max(1.0)).clamp(0.15, 0.85);
                }
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| self.hfdl_map_pane(ui, now, avail_h));
            }
        });

        // A channel change moves the dial with it, so the panadapter keeps
        // showing what the lane is decoding; the lane follows its own frequency
        // either way.
        if cfg.frequency_hz != self.state.hfdl.frequency_hz {
            cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz: cfg.frequency_hz });
        }
        if cfg != self.state.hfdl {
            cmds.push(Command::SetHfdlConfig(cfg));
        }
    }

    /// The decode log, newest first, with a filter over it.
    fn hfdl_log_pane(&mut self, ui: &mut egui::Ui, avail_h: f32) {
        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(20.0);
            ui.label(RichText::new("DECODES").size(10.5).strong().color(crate::theme::CYAN_DIM()));
            ui.add(
                egui::TextEdit::singleline(&mut self.hfdl_filter)
                    .hint_text("filter")
                    .desired_width(90.0),
            );
        });

        let filter = self.hfdl_filter.trim().to_ascii_uppercase();
        // The log is carried whole in every status snapshot, oldest *last*.
        let log = self.hfdl_status.as_ref().map(|s| s.log.as_slice()).unwrap_or(&[]);
        egui::ScrollArea::vertical()
            .id_salt("hfdl-log")
            .max_height((avail_h - 24.0).max(48.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut shown = 0;
                for d in log.iter().rev() {
                    if !filter.is_empty() && !hfdl_matches(d, &filter) {
                        continue;
                    }
                    hfdl_log_row(ui, d);
                    shown += 1;
                }
                if shown == 0 {
                    let text = if log.is_empty() {
                        "Nothing decoded yet — a ground station's squitter repeats every \
                         ~32 s once one is in the channel."
                    } else {
                        "No decode matches the filter."
                    };
                    ui.label(RichText::new(text).size(10.5).color(crate::theme::gray(120)));
                }
            });
    }

    /// The aircraft map, fed by the app's plot table.
    fn hfdl_map_pane(&mut self, ui: &mut egui::Ui, now: i64, avail_h: f32) {
        let home = self.hfdl_home();
        crate::hfdl_map::show(ui, &mut self.hfdl_map, home, now, avail_h);
    }

    /// The operator's own position, from the grid in the digital-mode setup —
    /// the same source the FT8, APRS and ADS-B maps use, so they never
    /// disagree about where the station is.
    fn hfdl_home(&self) -> Option<(f64, f64)> {
        let grid = self.digi_cfg_edit.my_grid.trim();
        (!grid.is_empty()).then(|| sdroxide_types::grid_to_latlon(grid)).flatten()
    }
}

/// The decoder's own state, as fixed-width slots so nothing reflows the header
/// when a number grows a digit. The aircraft count is the map's table, not the
/// status's: it outlives the log's rolling window.
fn hfdl_status_strip(ui: &mut egui::Ui, status: Option<&HfdlStatus>, aircraft: usize) {
    let (running, level, bursts, decodes) = match status {
        Some(s) => (s.running, Some(s.level_dbfs), s.bursts, s.decodes),
        None => (false, None, 0, 0),
    };
    // "OFF" is amber, not grey: with the lane building the waterfall and the
    // level meter regardless, an idle-looking word here was the only thing
    // saying the decoder was not running, and it read as "nothing on the
    // channel" instead of "not switched on" (issue #497). The word itself says
    // which: DECODING rather than a bare RUNNING/OFF.
    let (run_text, run_ink) = if running {
        ("DECODING", crate::theme::GREEN())
    } else {
        ("DECODING OFF", crate::theme::HAZARD())
    };
    slot(ui, 108.0, run_text, run_ink);
    let level_text = match level {
        Some(l) if l > -90.0 => format!("{l:.0} dBFS"),
        _ => "— dBFS".to_string(),
    };
    slot(ui, 76.0, &level_text, crate::theme::gray(150));
    slot(ui, 76.0, &format!("{} aircraft", count(aircraft as u64)), crate::theme::CYAN());
    slot(ui, 64.0, &format!("{} bursts", count(bursts)), crate::theme::gray(150));
    slot(ui, 64.0, &format!("{} decodes", count(decodes)), crate::theme::gray(120));
}

/// One decoded event, as a row of the log: time, kind, the ground station it
/// names, the channel and the burst's signal report — then the payload, which
/// for a position record is the fix itself, and otherwise the parser's fields.
fn hfdl_log_row(ui: &mut egui::Ui, d: &HfdlDecode) {
    let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(d.unix);
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("{h:02}:{mi:02}:{s:02}"))
                .size(10.5)
                .color(crate::theme::gray(120)),
        );
        ui.label(RichText::new(&d.kind).size(10.5).color(crate::theme::CYAN()));
        if let Some(gs) = &d.gs {
            ui.label(RichText::new(gs).size(10.5).strong());
        }
        ui.label(
            RichText::new(format!("{:.3} MHz", d.freq_khz as f32 / 1e3))
                .size(10.5)
                .color(crate::theme::gray(150)),
        );
        if let Some(snr) = d.snr_db {
            ui.label(RichText::new(format!("{snr:.0} dB")).size(10.5).color(if snr >= 12.0 {
                crate::theme::GREEN()
            } else {
                crate::theme::YELLOW()
            }));
        }
        if let Some(fec) = d.fec_corrected.filter(|f| *f > 0) {
            ui.label(
                RichText::new(format!("{} fixed", count(u64::from(fec))))
                    .size(10.5)
                    .color(crate::theme::gray(120)),
            );
        }
        // The position, where there is one, as the fix itself rather than the
        // JSON it arrived in.
        if let Some(fix) = &d.position {
            ui.label(
                RichText::new(format!("{:.4}, {:.4}", fix.lat, fix.lon))
                    .size(10.5)
                    .color(crate::theme::CYAN()),
            );
            ui.label(RichText::new(fix.label()).size(10.5).color(crate::theme::gray(150)));
        } else if !d.details.is_empty() && d.details != "null" {
            ui.label(
                RichText::new(tidy_details(&d.details)).size(10.0).color(crate::theme::gray(120)),
            );
        }
    });
}

/// Whether a decode matches the log's filter text: the kind, the ground station,
/// the channel, a position aircraft's label, or the raw payload.
fn hfdl_matches(d: &HfdlDecode, filter: &str) -> bool {
    d.kind.to_ascii_uppercase().contains(filter)
        || d.gs.as_deref().is_some_and(|g| g.to_ascii_uppercase().contains(filter))
        || format!("{:.3}", d.freq_khz as f32 / 1e3).contains(filter)
        || d.position.as_ref().is_some_and(|f| {
            f.label().to_ascii_uppercase().contains(filter)
                || f.icao.as_deref().is_some_and(|i| i.to_ascii_uppercase().contains(filter))
        })
        || d.details.to_ascii_uppercase().contains(filter)
}

/// The payload JSON made readable: no braces, no quotes, one space after each
/// separator. The panel shows the fields xng parsed rather than pretty-printing
/// its structure, which at a glance is noise either way.
fn tidy_details(details: &str) -> String {
    details.trim().trim_start_matches('{').trim_end_matches('}').replace('"', "").replace(',', "  ")
}

/// A counter, short enough that it cannot outgrow its slot.
fn count(n: u64) -> String {
    if n < 10_000 {
        n.to_string()
    } else if n < 995_000 {
        format!("{:.0}k", n as f64 / 1e3)
    } else if n < 999_500_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else {
        format!("{:.2}G", n as f64 / 1e9)
    }
}

/// A readout in a slot of fixed width, so a number that grows a digit cannot
/// re-flow the header.
fn slot(ui: &mut egui::Ui, w: f32, text: &str, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 16.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    ui.painter_at(rect).text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::monospace(10.5),
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{HFDL_DEFAULT_HZ, HfdlFix};

    fn dec(kind: &str, gs: Option<&str>) -> HfdlDecode {
        HfdlDecode {
            unix: 1_700_000_000,
            kind: kind.to_string(),
            gs: gs.map(|s| s.to_string()),
            freq_khz: 21_931,
            snr_db: Some(14.5),
            freq_skew_hz: None,
            fec_corrected: None,
            details: r#"{"aircraft":"A9C-DM"}"#.to_string(),
            position: None,
        }
    }

    #[test]
    fn default_channel_is_in_the_plan() {
        assert!(HFDL_PLAN_KHZ.contains(&(HFDL_DEFAULT_HZ as u32 / 1000)));
    }

    #[test]
    fn the_filter_matches_kind_station_and_payload() {
        let d = dec("squitter", Some("Riverhead (GS 4)"));
        assert!(hfdl_matches(&d, "SQUIT"));
        assert!(hfdl_matches(&d, "RIVERHEAD"));
        assert!(hfdl_matches(&d, "A9C-DM"), "the payload details match too");
        assert!(!hfdl_matches(&d, "POSITION"));
    }

    #[test]
    fn the_filter_matches_a_position_aircraft() {
        let mut d = dec("performance-data", None);
        d.details = "{}".into();
        d.position = Some(HfdlFix {
            lat: 40.88,
            lon: -72.64,
            aircraft_id: Some(0x42),
            icao: Some("040087".into()),
            flight: Some("BAW123".into()),
        });
        assert!(hfdl_matches(&d, "BAW123"), "by flight");
        assert!(hfdl_matches(&d, "040087"), "by ICAO");
        assert!(!hfdl_matches(&d, "KLM"));
    }

    #[test]
    fn details_are_tidied_for_the_row() {
        assert_eq!(tidy_details(r#"{"gs_id":4,"lpdus":3}"#), "gs_id:4  lpdus:3");
    }
}
