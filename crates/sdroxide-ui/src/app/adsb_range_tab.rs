//! ADS-B SETUP → Range log: the setups the range log ([`crate::adsb_range`])
//! has filed positions under, each with its days, laid out like the PANO
//! recorder's inventory ([`super::recorder_tab`]) — size, INIT, delete in two
//! clicks — plus the antenna label and NEW SETUP, which start a version.

use eframe::egui::{self, RichText};

use crate::adsb_range::{DayRange, MAX_MAX_MB, MIN_MAX_MB, RangeLog, SetupInfo};
use crate::i18n::tr;
use crate::recorder::mb;

/// How often the inventory is read back while the window is open, seconds.
// The data changes only on each minute's flush; the read is a whole-table
// aggregate, so not every frame or two.
const REFRESH_S: f64 = 15.0;

#[derive(Default)]
pub(in crate::app) struct RangeTab {
    log: Option<RangeLog>,
    error: Option<String>,
    setups: Vec<SetupInfo>,
    /// The expanded setup's days.
    days: Vec<DayRange>,
    expanded: Option<i64>,
    armed_delete: Option<i64>,
    armed_init: bool,
    /// The antenna field, for NEW SETUP and NAME.
    antenna: String,
    max_mb: f64,
    file_bytes: u64,
    refreshed: f64,
    dirty: bool,
}

/// `18.11.26` from a `2026-11-18`.
fn short_day(day: &str) -> String {
    let p: Vec<&str> = day.split('-').collect();
    match p[..] {
        [y, m, d] if y.len() == 4 => format!("{d}.{m}.{}", &y[2..]),
        _ => day.to_string(),
    }
}

impl RangeTab {
    fn ensure_open(&mut self) {
        if self.log.is_some() || self.error.is_some() {
            return;
        }
        match RangeLog::open_default() {
            Ok(l) => {
                self.max_mb = l.max_mb();
                self.log = Some(l);
                self.dirty = true;
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn fail(&mut self, e: rusqlite::Error) {
        self.error = Some(e.to_string());
    }

    /// Take in a table snapshot heard by `instrument`.
    pub fn note(
        &mut self,
        instrument: &str,
        home: Option<(f64, f64)>,
        aircraft: &[sdroxide_types::AdsbAircraft],
        now: i64,
    ) {
        self.ensure_open();
        if let Some(log) = self.log.as_mut()
            && let Err(e) = log.note(instrument, home, aircraft, now)
        {
            self.fail(e);
        }
    }

    fn refresh(&mut self, now: f64) {
        if !self.dirty && now - self.refreshed < REFRESH_S {
            return;
        }
        self.refreshed = now;
        self.dirty = false;
        let Some(log) = self.log.as_ref() else { return };
        let r = log.setups().and_then(|s| {
            let days = match self.expanded {
                Some(id) => log.days(id)?,
                None => Vec::new(),
            };
            Ok((s, days))
        });
        match r {
            Ok((s, d)) => {
                self.setups = s;
                self.days = d;
            }
            Err(e) => self.fail(e),
        }
        self.file_bytes = self.log.as_ref().map_or(0, |l| l.file_bytes());
    }

    /// The section's body. `instrument` is what new positions are filed
    /// under; `home` the operator's position, which a new setup needs.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        instrument: &str,
        home: Option<(f64, f64)>,
        now: f64,
    ) {
        self.ensure_open();
        if let Some(e) = &self.error {
            ui.label(
                RichText::new(tr!("adsb-range-db-error", err = e.clone()))
                    .color(crate::theme::ALERT()),
            );
            return;
        }
        self.refresh(now);
        let unix = crate::time::now_unix();
        let current = self.log.as_ref().and_then(RangeLog::current);

        // The file: size, limit, INIT.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr!(
                    "adsb-range-summary",
                    size = mb(self.file_bytes),
                    n = self.setups.len()
                ))
                .strong(),
            )
            .on_hover_text(
                self.log.as_ref().map_or(String::new(), |l| l.path().display().to_string()),
            );
            ui.label(tr!("adsb-range-limit"));
            let r = ui
                .add(
                    egui::DragValue::new(&mut self.max_mb)
                        .range(MIN_MAX_MB..=MAX_MAX_MB)
                        .speed(1.0)
                        .suffix(" MB"),
                )
                .on_hover_text(tr!("adsb-range-limit-tip"));
            if (r.lost_focus() || r.drag_stopped())
                && let Some(log) = self.log.as_mut()
            {
                match log.set_max_mb(self.max_mb) {
                    Ok(_) => self.dirty = true,
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            crate::chrome::row_tail(ui, |ui| {
                let resp = if self.armed_init {
                    crate::chrome::chip_accent(
                        ui,
                        true,
                        tr!("adsb-range-init-sure"),
                        crate::theme::PINK(),
                        crate::theme::INK_ON_CYAN(),
                    )
                } else {
                    ui.add_enabled_ui(!self.setups.is_empty(), |ui| {
                        crate::chrome::chip(ui, false, tr!("adsb-range-init"))
                    })
                    .inner
                    .on_hover_text(tr!("adsb-range-init-tip"))
                };
                if resp.clicked() {
                    if self.armed_init {
                        if let Some(log) = self.log.as_mut()
                            && let Err(e) = log.init()
                        {
                            self.error = Some(e.to_string());
                        }
                        self.armed_init = false;
                        self.expanded = None;
                        self.dirty = true;
                    } else {
                        self.armed_init = true;
                        self.armed_delete = None;
                    }
                }
            });
        });
        if self.armed_init {
            ui.label(
                RichText::new(tr!(
                    "adsb-range-init-warning",
                    n = self.setups.len(),
                    size = mb(self.file_bytes)
                ))
                .color(crate::theme::ALERT()),
            );
            if ui.small_button(tr!("adsb-range-cancel")).clicked() {
                self.armed_init = false;
            }
        }

        // The setup positions go to now, and the controls that change it.
        let open = current.and_then(|id| self.setups.iter().find(|s| s.id == id));
        ui.label(match open {
            Some(s) => tr!("adsb-range-now", setup = setup_name(s)),
            None if home.is_none() => tr!("adsb-range-no-home"),
            None => tr!("adsb-range-waiting", instrument = instrument.to_string()),
        });
        ui.horizontal(|ui| {
            ui.label(tr!("adsb-range-antenna"));
            ui.add(
                egui::TextEdit::singleline(&mut self.antenna)
                    .desired_width(150.0)
                    .hint_text(tr!("adsb-range-antenna-hint")),
            );
            let can_new = home.is_some() && !instrument.is_empty();
            if ui
                .add_enabled_ui(can_new, |ui| crate::chrome::chip(ui, false, tr!("adsb-range-new")))
                .inner
                .on_hover_text(tr!("adsb-range-new-tip"))
                .clicked()
                && let (Some(log), Some(h)) = (self.log.as_mut(), home)
            {
                match log.new_setup(instrument, &self.antenna, h, unix) {
                    Ok(_) => self.dirty = true,
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            if ui
                .add_enabled_ui(current.is_some(), |ui| {
                    crate::chrome::chip(ui, false, tr!("adsb-range-name"))
                })
                .inner
                .on_hover_text(tr!("adsb-range-name-tip"))
                .clicked()
                && let (Some(log), Some(id)) = (self.log.as_mut(), current)
            {
                match log.set_antenna(id, &self.antenna) {
                    Ok(_) => self.dirty = true,
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
        });

        if self.setups.is_empty() {
            ui.label(RichText::new(tr!("adsb-range-empty")).weak());
            return;
        }

        let mut toggle = None;
        let mut delete = None;
        egui::ScrollArea::vertical().max_height(240.0).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("adsb-range-setups")
                .num_columns(7)
                .striped(true)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    for h in [
                        tr!("adsb-range-col-setup"),
                        tr!("adsb-range-col-antenna"),
                        tr!("adsb-range-col-when"),
                        tr!("adsb-range-col-days"),
                        tr!("adsb-range-col-km"),
                        tr!("adsb-range-col-size"),
                        String::new(),
                    ] {
                        ui.label(RichText::new(h).weak());
                    }
                    ui.end_row();

                    for s in &self.setups {
                        let is_open = self.expanded == Some(s.id);
                        let arrow = if is_open { "▼" } else { "▶" };
                        let mut name = RichText::new(format!("{arrow} {}", setup_name(s)));
                        if current == Some(s.id) {
                            name = name.color(crate::theme::CYAN());
                        }
                        if ui
                            .add(egui::Label::new(name).sense(egui::Sense::click()))
                            .on_hover_text(tr!("adsb-range-open-days"))
                            .clicked()
                        {
                            toggle = Some(s.id);
                        }
                        ui.label(if s.antenna.is_empty() { "—" } else { &s.antenna });
                        ui.label(if s.first_day.is_empty() {
                            "—".to_string()
                        } else if s.first_day == s.last_day {
                            short_day(&s.first_day)
                        } else {
                            format!("{}–{}", short_day(&s.first_day), short_day(&s.last_day))
                        });
                        ui.label(format!("{}", s.days));
                        ui.label(if s.max_km > 0.0 {
                            format!("{:.0}", s.max_km)
                        } else {
                            "—".to_string()
                        });
                        ui.label(mb(s.bytes.max(0) as u64));
                        if self.armed_delete == Some(s.id) {
                            if crate::chrome::chip_accent(
                                ui,
                                true,
                                tr!("adsb-range-delete-sure"),
                                crate::theme::PINK(),
                                crate::theme::INK_ON_CYAN(),
                            )
                            .clicked()
                            {
                                delete = Some(s.id);
                            }
                        } else if crate::chrome::chip(ui, false, tr!("adsb-range-delete"))
                            .on_hover_text(tr!("adsb-range-delete-tip"))
                            .clicked()
                        {
                            self.armed_delete = Some(s.id);
                            self.armed_init = false;
                        }
                        ui.end_row();

                        if is_open {
                            for d in &self.days {
                                ui.label(
                                    RichText::new(format!("   └ {}", short_day(&d.day))).weak(),
                                );
                                ui.label(tr!("adsb-range-day-positions", n = d.positions));
                                ui.label(tr!(
                                    "adsb-range-day-sectors",
                                    n = d.sectors,
                                    all = crate::adsb_coverage::SECTORS
                                ));
                                ui.label("");
                                ui.label(format!("{:.0}", d.max_km)).on_hover_text(tr!(
                                    "adsb-range-day-far",
                                    median = format!("{:.0}", d.median_km),
                                    bearing = format!("{:.0}", d.max_bearing)
                                ));
                                ui.label("");
                                ui.label("");
                                ui.end_row();
                            }
                        }
                    }
                });
        });

        if let Some(id) = toggle {
            self.expanded = if self.expanded == Some(id) { None } else { Some(id) };
            self.dirty = true;
        }
        if let Some(id) = delete {
            if let Some(log) = self.log.as_mut()
                && let Err(e) = log.delete(id)
            {
                self.error = Some(e.to_string());
            }
            if self.expanded == Some(id) {
                self.expanded = None;
            }
            self.armed_delete = None;
            self.dirty = true;
        }
    }
}

/// `RSP1A v2`.
fn setup_name(s: &SetupInfo) -> String {
    format!("{} v{}", s.instrument, s.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_read_short() {
        assert_eq!(short_day("2026-11-18"), "18.11.26");
        assert_eq!(short_day("odd"), "odd");
    }
}
