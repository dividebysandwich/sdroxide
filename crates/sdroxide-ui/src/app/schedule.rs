//! The broadcast schedule window: what is on, when, where — and a way to tune
//! to it and log it.
//!
//! The EiBi table is already in the program (it labels the waterfall); this
//! turns it into the list a listener works from. Filter by time, band, language
//! and a free-text query, tune a row, or open a reception with the station
//! filled in.

use eframe::egui::{self, RichText};
use sdroxide_types::{BroadcastStation, Command, RxId, Vfo, broadcast};

use crate::app::SdroxideApp;
use crate::app::swl_log::SwlEditForm;
use crate::time::now_unix;

/// The metre bands offered in the band filter. Empty = any.
const BAND_FILTERS: [&str; 18] = [
    "", "LW", "MW", "120m", "90m", "75m", "60m", "49m", "41m", "31m", "25m", "22m", "19m", "16m",
    "15m", "13m", "11m", "FM",
];

/// The schedule window's state: whether it is open, and its filters.
pub(in crate::app) struct ScheduleUi {
    pub show: bool,
    pub query: String,
    pub lang: String,
    pub target: String,
    /// Empty = any; otherwise a metre band name from [`BAND_FILTERS`].
    pub band: String,
    /// The filter time as UTC `HHMM`, ignored while `use_now`.
    pub hhmm: u16,
    pub use_now: bool,
    /// Show only the listener's favourite stations.
    pub favourites_only: bool,
}

impl Default for ScheduleUi {
    fn default() -> Self {
        ScheduleUi {
            show: false,
            query: String::new(),
            lang: String::new(),
            target: String::new(),
            band: String::new(),
            hhmm: 0,
            use_now: true,
            favourites_only: false,
        }
    }
}

/// The UTC instant the schedule is filtered at: now, or today at `hhmm`.
fn schedule_time(now: i64, ui: &ScheduleUi) -> i64 {
    if ui.use_now {
        return now;
    }
    let day = now.div_euclid(86_400) * 86_400;
    day + (ui.hhmm as i64 / 100) * 3600 + (ui.hhmm as i64 % 100) * 60
}

fn hhmm_text(hhmm: u16) -> String {
    format!("{:02}:{:02}", hhmm / 100, hhmm % 100)
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    let n = needle.trim();
    n.is_empty() || haystack.to_ascii_lowercase().contains(&n.to_ascii_lowercase())
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

impl SdroxideApp {
    /// The SCHEDULE window. Returns nothing; tuning and logging are done by the
    /// commands and the entry form it fills in.
    pub(in crate::app) fn schedule_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.schedule.show {
            return;
        }
        let mut open = self.schedule.show;
        let now = now_unix() as i64;
        let at = schedule_time(now, &self.schedule);

        // The matching rows, cloned out before the closure borrows `self` to
        // edit the filters and to open the log.
        let rows: Vec<BroadcastStation> = {
            let f = &self.schedule;
            let mut v: Vec<BroadcastStation> = self
                .broadcast
                .iter()
                .filter(|s| {
                    s.on_air_at(at)
                        && s.matches_query(&f.query)
                        && contains_ci(&s.lang, &f.lang)
                        && contains_ci(&s.target, &f.target)
                        && (f.band.is_empty() || broadcast::metre_band(s.freq_khz) == Some(f.band.as_str()))
                        && (!f.favourites_only || self.broadcast_favs.iter().any(|n| n == &s.name))
                })
                .cloned()
                .collect();
            v.sort_by(|a, b| a.freq_khz.partial_cmp(&b.freq_khz).unwrap_or(std::cmp::Ordering::Equal));
            v
        };
        let count = rows.len();

        let mut tune: Option<BroadcastStation> = None;
        let mut log: Option<BroadcastStation> = None;
        let mut fav_toggle: Option<(String, bool)> = None;
        let resp = egui::Window::new("SCHEDULE")
            .id(crate::layout::salted_id(ctx, "SCHEDULE"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 820.0))
            .default_height(crate::layout::window_h(ctx, 560.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.horizontal_wrapped(|ui| {
                    ui.label("At");
                    if crate::chrome::chip(ui, self.schedule.use_now, "NOW").clicked() {
                        self.schedule.use_now = true;
                    }
                    let mut hh = self.schedule.hhmm as i32;
                    let r = ui.add_enabled(
                        !self.schedule.use_now,
                        egui::DragValue::new(&mut hh)
                            .speed(1.0)
                            .range(0..=2359)
                            .custom_formatter(|v, _| hhmm_text(v as u16)),
                    );
                    if r.changed() {
                        self.schedule.hhmm = hh.clamp(0, 2359) as u16;
                    }
                    if ui.button("set time").clicked() {
                        self.schedule.use_now = false;
                    }
                    ui.separator();
                    ui.label("Find");
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.schedule.query)
                            .desired_width(150.0)
                            .hint_text("BBC, Ascension…"),
                    );
                    ui.label("Language");
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.schedule.lang).desired_width(90.0),
                    );
                    ui.label("Target");
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.schedule.target).desired_width(90.0),
                    );
                    ui.label("Band");
                    egui::ComboBox::from_id_salt("sched-band")
                        .width(72.0)
                        .selected_text(if self.schedule.band.is_empty() {
                            "any".to_string()
                        } else {
                            self.schedule.band.clone()
                        })
                        .show_ui(ui, |ui| {
                            for b in BAND_FILTERS {
                                let label = if b.is_empty() { "any" } else { b };
                                ui.selectable_value(&mut self.schedule.band, b.to_string(), label);
                            }
                        });
                    if crate::chrome::chip(ui, self.schedule.favourites_only, "★ FAVS")
                        .on_hover_text("Only the stations you have starred")
                        .clicked()
                    {
                        self.schedule.favourites_only = !self.schedule.favourites_only;
                    }
                    ui.separator();
                    ui.label(
                        RichText::new(format!("{} UTC", crate::time::utc_clock(now)))
                            .monospace()
                            .color(crate::theme::CYAN()),
                    );
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!(
                        "{count} on air at {} UTC",
                        if self.schedule.use_now {
                            "now".to_string()
                        } else {
                            hhmm_text(self.schedule.hhmm)
                        }
                    ))
                    .size(11.0)
                    .color(crate::theme::gray(150)),
                );
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("sched-list").show(
                    ui,
                    |ui| {
                        for s in &rows {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{:>6}", s.freq_khz.round() as i64))
                                        .monospace(),
                                );
                                ui.label(
                                    RichText::new(
                                        broadcast::metre_band(s.freq_khz).unwrap_or(""),
                                    )
                                    .size(11.0)
                                    .color(crate::theme::CYAN()),
                                );
                                ui.label(RichText::new(truncate(&s.name, 30)).strong());
                                ui.label(
                                    RichText::new(truncate(&s.lang, 12))
                                        .size(11.0)
                                        .color(crate::theme::gray(160)),
                                );
                                ui.label(
                                    RichText::new(truncate(&s.target, 12))
                                        .size(11.0)
                                        .color(crate::theme::gray(160)),
                                );
                                ui.label(
                                    RichText::new(truncate(&s.site, 18))
                                        .size(11.0)
                                        .color(crate::theme::gray(140)),
                                );
                                let fav =
                                    self.broadcast_favs.iter().any(|n| n == &s.name);
                                if ui
                                    .small_button(if fav { "★" } else { "☆" })
                                    .on_hover_text("Favourite this station")
                                    .clicked()
                                {
                                    fav_toggle = Some((s.name.clone(), !fav));
                                }
                                if ui.small_button("TUNE").clicked() {
                                    tune = Some(s.clone());
                                }
                                if ui.small_button("LOG").clicked() {
                                    log = Some(s.clone());
                                }
                            });
                        }
                    },
                );
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.schedule.show = open;

        if let Some((name, on)) = fav_toggle {
            self.broadcast_favs.retain(|n| n != &name);
            if on {
                self.broadcast_favs.push(name);
                self.broadcast_favs.sort();
            }
            crate::app::persist::persist_broadcast_favourites(&self.broadcast_favs);
        }
        if let Some(s) = tune {
            cmds.push(Command::SetVfo { vfo: Vfo::A, hz: s.freq_hz() });
            cmds.push(Command::SetMode { rx: RxId::Main, mode: s.mode() });
        }
        if let Some(s) = log {
            let smeter = self.meters.map(|m| m.s_dbm);
            self.swl_edit = Some(SwlEditForm::from_station(
                s.freq_hz(),
                s.mode(),
                &s.name,
                &s.lang,
                &s.site,
                smeter,
            ));
            self.show_swl = true;
        }
    }
}
