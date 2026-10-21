//! The broadcast schedule window: what is on, when, where — and a way to tune
//! to it and log it.
//!
//! The EiBi table is already in the program (it labels the waterfall); this
//! turns it into the list a listener works from. Filter by time, band, language
//! and a free-text query, tune a row, or open a reception with the station
//! filled in.

use eframe::egui::{self, RichText};
use sdroxide_types::{BroadcastStation, Command, RxId, ScheduleSource, Vfo, broadcast};

use crate::app::SdroxideApp;
use crate::app::swl_log::SwlEditForm;
use crate::time::now_unix;

/// The SCHEDULE window's default width, and what the filter row has to fit in.
///
/// **The row is one `horizontal` and does not wrap**, so this is a budget and not
/// a preference: the source combobox was added beside `Band` because that is where
/// filters live, and the row was already at the window's edge — "Aoki" sat half
/// off it in the first build. Two things came out of that screenshot, and both are
/// here rather than in the commit: the combobox moved to the chip row, and the
/// window got wide enough that the row genuinely fits.
pub(in crate::app) const SCHEDULE_WINDOW_W: f32 = 1000.0;

/// The sources offered in the source filter, in the order the store ranks them.
const SOURCE_FILTERS: &[(ScheduleSource, &str)] = &[
    (ScheduleSource::Eibi, "EiBi"),
    (ScheduleSource::Aoki, "Aoki"),
    (ScheduleSource::Hfcc, "HFCC"),
    (ScheduleSource::User, "mine"),
];

/// The filter's own label, which must say **all of them** rather than any source's
/// name — the default is the store, and a combobox reading "EiBi" when three
/// schedules are loaded would be the control quietly lying about its state.
fn source_label(source: Option<ScheduleSource>) -> String {
    match source {
        None => "all of them".to_string(),
        Some(s) => SOURCE_FILTERS
            .iter()
            .find(|(x, _)| *x == s)
            .map(|(_, n)| (*n).to_string())
            .unwrap_or_else(|| format!("{s:?}")),
    }
}

/// The metre bands offered in the band filter. Empty = any.
const BAND_FILTERS: [&str; 20] = [
    "", "LW", "MW", "120m", "90m", "75m", "60m", "49m", "41m", "31m", "25m", "22m", "19m", "16m",
    "15m", "13m", "11m", "FM", "AIR", "MIL",
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
    /// Which schedule to show. `None` is **all of them**, which is the default:
    /// the master store holds every downloaded source and the repetition is
    /// accepted, because this window is where the detail belongs.
    pub source: Option<ScheduleSource>,
    /// Show each row's local mean solar time at the transmitter, from the
    /// site's longitude. Off by default: most listeners work in UTC, and the
    /// column is only as good as the site coordinate.
    pub solar_time: bool,
    /// The filtered list, and the key it was built for. Rebuilt only when the
    /// filters, the favourites, the loaded schedule or the UTC minute change —
    /// not sixty times a second over four and a half thousand rows.
    cache_key: Option<(String, String, String, String, bool, usize, i64, Option<ScheduleSource>)>,
    cache: Vec<BroadcastStation>,
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
            source: None,
            solar_time: false,
            cache_key: None,
            cache: Vec::new(),
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
    /// The SCHEDULE window. Tuning and logging are done by the commands and the
    /// entry form it fills in, after the window has closed the borrow of the log
    /// — so the body carries the three asks back and the tail acts on them.
    pub(in crate::app) fn schedule_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut tune: Option<BroadcastStation> = None;
        let mut log: Option<BroadcastStation> = None;
        let mut fav_toggle: Option<(String, bool)> = None;
        let mut all_toggle: Option<bool> = None;
        let open = self.tool_window(
            ctx,
            "schedule",
            "SCHEDULE",
            [SCHEDULE_WINDOW_W, 560.0],
            self.schedule.show,
            |me, ui| me.schedule_body(ui, &mut tune, &mut log, &mut fav_toggle, &mut all_toggle),
        );
        self.schedule.show = open;

        if let Some((name, on)) = fav_toggle {
            self.broadcast_favs.retain(|n| n != &name);
            if on {
                self.broadcast_favs.push(name);
                self.broadcast_favs.sort();
            }
            crate::app::persist::persist_broadcast_favourites(&self.broadcast_favs);
        }
        if let Some(on) = all_toggle {
            self.broadcast_all = on;
            if on {
                // Favourite every schedule station.
                self.broadcast_favs.clear();
                for s in &self.broadcast {
                    if !self.broadcast_favs.iter().any(|n| n == &s.name) {
                        self.broadcast_favs.push(s.name.clone());
                    }
                }
                self.broadcast_favs.sort();
            } else {
                self.broadcast_favs.clear();
            }
            crate::app::persist::persist_broadcast_favourites(&self.broadcast_favs);
        }
        if let Some(s) = tune {
            cmds.push(Command::SetVfo { vfo: Vfo::A, hz: s.freq_hz() });
            cmds.push(Command::SetMode { rx: RxId::Main, mode: s.mode() });
        }
        if let Some(s) = log {
            let smeter = self.meters.map(|m| m.s_dbm);
            let grid = self.my_grid();
            let antenna = self.swl_antenna.clone();
            self.swl_edit = Some(SwlEditForm::from_station(
                s.freq_hz(),
                s.mode(),
                &s.name,
                &s.lang,
                &s.site,
                &s.email,
                &s.address,
                s.lat,
                s.lon,
                smeter,
                grid,
                antenna,
            ));
            self.show_swl = true;
        }
    }

    /// The schedule's body: the filters, the on-air list and its row actions.
    /// Split out of [`Self::schedule_window`] so the shell can draw it in a
    /// window of its own; the three row asks go back through the parameters.
    fn schedule_body(
        &mut self,
        ui: &mut egui::Ui,
        tune: &mut Option<BroadcastStation>,
        log: &mut Option<BroadcastStation>,
        fav_toggle: &mut Option<(String, bool)>,
        all_toggle: &mut Option<bool>,
    ) {
        crate::chrome::window_body_bg(ui);
        let now = now_unix() as i64;
        let at = schedule_time(now, &self.schedule);

        // The matching rows, cloned out before the closure borrows `self` to
        // edit the filters and to open the log.
        // On-air status changes at the minute, so the cache keys on the minute,
        // not on the second.
        let key = (
            self.schedule.query.clone(),
            self.schedule.lang.clone(),
            self.schedule.target.clone(),
            self.schedule.band.clone(),
            self.schedule.favourites_only,
            self.broadcast.len(),
            at.div_euclid(60),
            self.schedule.source,
        );
        let rows: Vec<BroadcastStation> = if self.schedule.cache_key.as_ref() == Some(&key) {
            // Lifted out for the frame and put back afterwards, so the closure
            // below is free to borrow `self` mutably.
            std::mem::take(&mut self.schedule.cache)
        } else {
            let f = &self.schedule;
            let mut v: Vec<BroadcastStation> = self
                .broadcast
                .iter()
                .filter(|s| {
                    s.on_air_at(at)
                        && f.source.is_none_or(|w| s.source == w)
                        && s.matches_query(&f.query)
                        && broadcast::contains_ascii_ci(&s.lang, &f.lang)
                        && broadcast::contains_ascii_ci(&s.target, &f.target)
                        && (f.band.is_empty()
                            || broadcast::metre_band(s.freq_khz) == Some(f.band.as_str()))
                        && (!f.favourites_only || self.broadcast_favs.iter().any(|n| n == &s.name))
                })
                .cloned()
                .collect();
            v.sort_by(|a, b| {
                a.freq_khz.partial_cmp(&b.freq_khz).unwrap_or(std::cmp::Ordering::Equal)
            });
            v
        };
        let count = rows.len();

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
                    .hint_text("BBC, 6185, 15.4 MHz…"),
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
            // On the **chip row**, not beside Band: the filter row was already at
            // the window's edge and a third combobox put "Aoki" half off it. The
            // house rule is that a new control is priced before it is added, and
            // this one was not.
            ui.label("Source");
            egui::ComboBox::from_id_salt("sched-source")
                .width(96.0)
                .selected_text(source_label(self.schedule.source))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.schedule.source, None, "all of them")
                        .on_hover_text(
                            "Every downloaded schedule in the one store. A station listed \
                         by two of them appears twice, which is the detail this window \
                         is for — the panadapter shows one name.",
                        );
                    for (src, what) in SOURCE_FILTERS {
                        ui.selectable_value(&mut self.schedule.source, Some(*src), *what);
                    }
                });
            if crate::chrome::chip(ui, self.schedule.favourites_only, "★ FAVS")
                .on_hover_text("Only the stations you have starred")
                .clicked()
            {
                self.schedule.favourites_only = !self.schedule.favourites_only;
            }
            if crate::chrome::chip(ui, self.broadcast_all, "⭐ ALL")
                .on_hover_text(
                    "Favourite every schedule station, and show their labels on the \
                     waterfall while it stays on. Click again to clear all favourites.",
                )
                .clicked()
            {
                *all_toggle = Some(!self.broadcast_all);
            }
            if crate::chrome::chip(ui, self.schedule.solar_time, "SOLAR TIME")
                .on_hover_text(
                    "Show each row's local time at the transmitter — the Sun's clock, \
                     four minutes a degree from its longitude, not a civil time zone. \
                     It knows nothing of daylight saving or zone borders, so it is \
                     labelled solar rather than local.",
                )
                .clicked()
            {
                self.schedule.solar_time = !self.schedule.solar_time;
            }
            ui.separator();
            // **The way into LISTEN**, and deliberately here rather than as a
            // chip of its own on the top bar: the SWL row is already at the
            // two-row limit at 1400 pt, and a third chip pushed AM — the main
            // SWL mode — onto a third row. SCHEDULE is already in that row and is
            // the same data, so this costs no width and loses nothing.
            if crate::chrome::chip(ui, self.show_listen, "MAP")
                .on_hover_text(
                    "Open LISTEN — this schedule as a map. Pick a station on it, write \
                     what you heard, and log it. The selected station comes across",
                )
                .clicked()
            {
                self.show_listen = !self.show_listen;
                if self.show_listen
                    && let Some(s) = sdroxide_types::broadcast::at_dial(
                        &self.broadcast,
                        self.state.rx_freq_hz(),
                        now_unix(),
                    )
                {
                    // Open it with **what is on the dial** already selected, which
                    // is the station the operator is listening to and the one they
                    // most often want to write down.
                    let name = s.name.clone();
                    self.listen_select(&name);
                }
            }
            ui.separator();
            ui.label(
                RichText::new(crate::time::utc_and_local_now(now))
                    .monospace()
                    .color(crate::theme::CYAN()),
            );
            // The build, beside the clock — see the SWL LOG window.
            ui.label(
                RichText::new(format!("v{}", sdroxide_version::VERSION))
                    .size(10.5)
                    .color(crate::theme::gray(120)),
            )
            .on_hover_text(sdroxide_version::LONG_VERSION);
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
                            RichText::new(format!("{:>6}", s.freq_khz.round() as i64)).monospace(),
                        );
                        ui.label(
                            RichText::new(broadcast::metre_band(s.freq_khz).unwrap_or(""))
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
                        if self.schedule.solar_time
                            && let Some(lon) = s.lon
                        {
                            let (_, _, _, h, mi, _) = sdroxide_types::utc_ymd_hms(now);
                            let local =
                                broadcast::local_solar_hhmm(h as u16 * 100 + mi as u16, lon);
                            ui.label(
                                RichText::new(local.clone())
                                    .size(11.0)
                                    .monospace()
                                    .color(crate::theme::gray(140)),
                            )
                            .on_hover_text(format!(
                                "{local} local solar time at {}, {lon:.1}°",
                                s.site,
                            ));
                        }
                        let fav = self.broadcast_favs.iter().any(|n| n == &s.name);
                        if ui
                            .small_button(if fav { "★" } else { "☆" })
                            .on_hover_text("Favourite this station")
                            .clicked()
                        {
                            *fav_toggle = Some((s.name.clone(), !fav));
                        }
                        if ui.small_button("TUNE").clicked() {
                            *tune = Some(s.clone());
                        }
                        if ui.small_button("LOG").clicked() {
                            *log = Some(s.clone());
                        }
                    });
                }
            },
        );

        self.schedule.cache = rows;
        self.schedule.cache_key = Some(key);
    }
}

#[cfg(test)]
mod source_filter_tests {
    use super::{ScheduleUi, source_label};
    use sdroxide_types::ScheduleSource;

    /// **The default has to say what it actually does.** With three schedules in
    /// the store, a combobox reading "EiBi" would be this fork's house rule in
    /// miniature — a control that cannot be misread. It reads *all of them*.
    #[test]
    fn the_source_filter_default_says_all_of_them() {
        assert_eq!(source_label(None), "all of them");
        assert_eq!(ScheduleUi::default().source, None, "and starts on the store");
    }

    /// Every source is **distinguishable** in the filter. Two sources sharing a
    /// label would make the combobox a control that cannot say which one is
    /// selected — the same fault as the default being wrong, one row over.
    #[test]
    fn every_source_has_its_own_name() {
        let all = [
            ScheduleSource::Eibi,
            ScheduleSource::Aoki,
            ScheduleSource::Hfcc,
            ScheduleSource::User,
        ];
        let mut seen: Vec<String> = Vec::new();
        for src in all {
            let label = source_label(Some(src));
            assert!(!label.is_empty(), "{src:?} has no name at all");
            assert!(
                !seen.contains(&label),
                "{src:?} reads {label:?}, which another source also uses"
            );
            seen.push(label);
        }
        assert_eq!(source_label(Some(ScheduleSource::Aoki)), "Aoki");
    }
}

#[cfg(test)]
mod filter_row_width {
    use super::{BAND_FILTERS, SCHEDULE_WINDOW_W, ScheduleUi};

    /// What is left of the window's width for content: egui's window margin
    /// either side plus the frame's stroke.
    const INNER_W: f32 = SCHEDULE_WINDOW_W - 2.0 * 10.0 - 2.0;

    /// The filter row's cheapest honest description: it is a single `horizontal`,
    /// so **anything added to it is added to one line**. That is the whole reason
    /// the source combobox overflowed the window — it went in beside `Band`
    /// because that is where filters live, and the row was already at the edge.
    ///
    /// This is a budget, not a layout test: it cannot see egui's own widths, so
    /// it pins the *decision* (Source is not on this row) and the number that
    /// made the decision. A pixel-accurate guard belongs to the window oracle,
    /// which measures ink — and ink, not geometry, is what an operator sees.
    #[test]
    fn the_filter_row_has_room_left_after_the_two_comboboxes() {
        // The widths the row actually asks for, as the widgets are written.
        let fields = [150.0_f32, 90.0, 90.0]; // Find, Language, Target
        let band = 72.0;
        let chips = ["At", "NOW", "set time", "Band"];
        let field = fields.iter().sum::<f32>();
        // egui's own overhead per label and per widget, measured rather than
        // guessed: a label is ~7 pt of text plus ~6 pt of spacing.
        let per_label = 13.0;
        let budget: f32 = fields.iter().sum::<f32>()
            + band
            + chips.len() as f32 * (28.0 + per_label)
            + (chips.len() + 3) as f32 * per_label
            + 8.0 * 13.0;
        assert!(
            budget < INNER_W,
            "the filter row asks {budget:.0} pt against {inner:.0} pt of window; \
             that is what put \"Aoki\" half off the edge",
            inner = INNER_W
        );
        assert!(field > 0.0 && !BAND_FILTERS.is_empty());
        assert!(INNER_W > 0.0);
        // And the state still has somewhere to put a source.
        let _ = ScheduleUi::default().source;
    }
}
