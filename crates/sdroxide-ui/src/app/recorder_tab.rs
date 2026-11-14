//! Scanner → PANO, lower half: record the full-band strip into the database,
//! list what is there, load a strip back, delete a session or everything.

use eframe::egui::{self, RichText};

use crate::i18n::tr;
use crate::recorder::{
    DEFAULT_INTERVAL_S, MIN_INTERVAL_S, Recorder, STRIP_ROWS, SessionInfo, StoredRow, StripInfo,
    Sweep, mb,
};

/// What the tab asks of the app after drawing.
pub(in crate::app) enum RecAction {
    /// Put these rows into the strip and hold it there.
    Load(Vec<StoredRow>),
    /// Back to the live strip.
    Live,
}

/// The tab's state: the database, the listing drawn from it, and what is
/// armed or loaded.
pub(in crate::app) struct RecTab {
    rec: Option<Recorder>,
    /// Opening the file failed; shown instead of the controls.
    error: Option<String>,
    interval_s: f64,
    sessions: Vec<SessionInfo>,
    file_bytes: u64,
    listed_at: f64,
    dirty: bool,
    expanded: Option<i64>,
    strips: Vec<StripInfo>,
    armed_delete: Option<i64>,
    armed_init: bool,
    /// LOAD pressed on a strip while a session records: the second press
    /// stops the recording and loads. Session id and strip.
    // Why: HDZ 2026-11-13: "LOAD muss die aktive Recording mit Warnung stoppen".
    armed_load: Option<(i64, i64)>,
    /// The session and strip shown in the strip instead of the live one, and
    /// the line that says so.
    viewing: Option<(i64, i64, String)>,
    /// A full-band frame has arrived from the source since it was opened.
    /// Kept apart from what the strip shows: a loaded recording holds the
    /// strip, and the source behind it is live all the same.
    live_seen: bool,
    /// The live strip's sweeps, oldest first, as many as it shows, each with
    /// the time it arrived: what ● REC takes along and STORE keeps. Emptied
    /// wherever the strip is, so it holds what the strip holds.
    // Why: HDZ 2026-11-13: "den scan stand des panoramas bei rec einfach
    //      übernehmen … das würde so wies ist zur verwirrung führen".
    recent: std::collections::VecDeque<Sweep>,
}

impl Default for RecTab {
    fn default() -> Self {
        RecTab {
            rec: None,
            error: None,
            interval_s: DEFAULT_INTERVAL_S,
            sessions: Vec::new(),
            file_bytes: 0,
            listed_at: f64::NEG_INFINITY,
            dirty: true,
            expanded: None,
            strips: Vec::new(),
            armed_delete: None,
            armed_init: false,
            armed_load: None,
            viewing: None,
            live_seen: false,
            recent: std::collections::VecDeque::new(),
        }
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn local(ms: i64, fmt: &str) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ms)
        .single()
        .map_or_else(String::new, |t| t.format(fmt).to_string())
}

/// How often a session wrote a row: its interval, or — for a stored strip,
/// interval zero — once per sweep, with the sweeps' mean spacing.
fn every(s: &SessionInfo) -> String {
    if s.interval_s > 0.0 {
        return tr!("rec-every", s = format!("{:.0}", s.interval_s));
    }
    let mean = if s.rows > 1 {
        (s.ended_ms - s.started_ms) as f64 / 1000.0 / (s.rows - 1) as f64
    } else {
        0.0
    };
    tr!("rec-every-sweep", s = format!("{mean:.1}"))
}

fn duration(ms: i64) -> String {
    let s = (ms.max(0) + 500) / 1000;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// The bands a range covers: those lying wholly inside it, or the one it
/// lies inside.
fn bands_of(lo: f64, hi: f64) -> (String, String) {
    if hi <= lo {
        return ("—".into(), String::new());
    }
    let all = crate::widgets::bandplan::allocations();
    let inside: Vec<&str> =
        all.iter().filter(|(l, h, _)| *l >= lo - 1.0 && *h <= hi + 1.0).map(|b| b.2).collect();
    if !inside.is_empty() {
        let full = inside.join(", ");
        let short = if inside.len() > 3 {
            format!("{} … (+{})", inside[..3].join(", "), inside.len() - 3)
        } else {
            full.clone()
        };
        return (short, full);
    }
    all.iter()
        .filter(|(l, h, _)| *l <= lo + 1.0 && *h >= hi - 1.0)
        .min_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
        .map_or(("—".into(), String::new()), |b| (b.2.to_string(), String::new()))
}

impl RecTab {
    fn ensure_open(&mut self) {
        if self.rec.is_some() || self.error.is_some() {
            return;
        }
        match Recorder::open_default() {
            Ok(r) => {
                self.interval_s = r.interval_s();
                self.rec = Some(r);
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn fail(&mut self, e: rusqlite::Error) {
        self.error = Some(e.to_string());
    }

    /// Whether the strip is showing a loaded recording rather than live rows.
    pub fn viewing(&self) -> bool {
        self.viewing.is_some()
    }

    /// The strip was cleared: back to live, and the sweeps it held are gone
    /// with it.
    pub fn forget_view(&mut self) {
        self.viewing = None;
        self.recent.clear();
    }

    /// A different source: nothing loaded on show, nothing heard from it yet.
    pub fn source_changed(&mut self) {
        self.forget_view();
        self.live_seen = false;
    }

    /// Whether the source delivers a full-band strip at all.
    pub fn live_seen(&self) -> bool {
        self.live_seen
    }

    pub fn recording(&self) -> bool {
        self.rec.as_ref().is_some_and(|r| r.active().is_some())
    }

    /// A full-band frame as it arrives: kept with the strip's sweeps while
    /// the strip is live, and recorded if a session is running.
    pub fn on_wide_frame(&mut self, f: &sdroxide_types::SpectrumFrame) {
        self.live_seen = true;
        let now = now_ms();
        // A loaded recording holds the strip: what arrives meanwhile is not on
        // it, and the strip starts afresh at LIVE.
        if self.viewing.is_none()
            && !f.bins.is_empty()
            && self.recent.back().is_none_or(|s| s.frame.seq != f.seq)
        {
            self.recent.push_back(Sweep::new(f, now));
            while self.recent.len() > STRIP_ROWS as usize {
                self.recent.pop_front();
            }
        }
        let Some(rec) = self.rec.as_mut() else { return };
        if rec.active().is_none() {
            return;
        }
        if let Err(e) = rec.on_frame(f, now) {
            self.fail(e);
        }
    }

    /// Once per pass: write a row whose interval is over.
    pub fn tick(&mut self) {
        let Some(rec) = self.rec.as_mut() else { return };
        if let Err(e) = rec.tick(now_ms()) {
            self.fail(e);
        }
    }

    fn refresh(&mut self, now: f64) {
        if !self.dirty && now - self.listed_at < 2.0 {
            return;
        }
        let Some(rec) = self.rec.as_ref() else { return };
        let listed = rec.sessions().and_then(|s| {
            let strips = match self.expanded {
                Some(id) => rec.strips(id)?,
                None => Vec::new(),
            };
            Ok((s, strips))
        });
        self.file_bytes = rec.file_bytes();
        match listed {
            Ok((s, strips)) => {
                self.sessions = s;
                self.strips = strips;
            }
            Err(e) => self.fail(e),
        }
        self.listed_at = now;
        self.dirty = false;
    }

    /// The tab's body. `radio` names the source a new session is filed under;
    /// `wide_live` says whether a full-band strip is arriving at all; `pano`
    /// whether a panorama scan is feeding it, whose own times space the rows.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        radio: &str,
        wide_live: bool,
        pano: bool,
        now: f64,
    ) -> Option<RecAction> {
        self.ensure_open();
        if let Some(e) = &self.error {
            ui.label(
                RichText::new(tr!("rec-db-error", err = e.clone())).color(crate::theme::ALERT()),
            );
            if ui.button(tr!("rec-retry")).clicked() {
                self.error = None;
                self.rec = None;
            }
            return None;
        }
        self.refresh(now);
        let mut action = None;

        // Recording and a loaded strip exclude each other: ● REC brings the
        // strip back to live, as LOAD stops a recording.
        if self.controls(ui, radio, wide_live, pano) && self.viewing.is_some() {
            action = Some(RecAction::Live);
        }
        if let Some((_, _, line)) = &self.viewing {
            ui.horizontal(|ui| {
                ui.label(RichText::new(line.as_str()).color(crate::theme::YELLOW()));
                crate::chrome::row_tail(ui, |ui| {
                    if crate::chrome::chip(ui, false, tr!("rec-live"))
                        .on_hover_text(tr!("rec-live-hint"))
                        .clicked()
                    {
                        action = Some(RecAction::Live);
                    }
                });
            });
        }
        ui.separator();
        if let Some(a) = self.inventory(ui) {
            action = Some(a);
        }
        if matches!(action, Some(RecAction::Live)) {
            self.viewing = None;
        }
        action
    }

    /// The REC switch, STORE CURRENT, the interval and the status line.
    /// Returns whether a recording was started.
    ///
    /// Under a panorama a recording keeps one row per sweep: its settle time
    /// and pause, on top of the page, already space the rows, and the strip's
    /// length is read off there. The interval is for a wideband receiver's own
    /// lane, which delivers several frames a second.
    fn controls(&mut self, ui: &mut egui::Ui, radio: &str, wide_live: bool, pano: bool) -> bool {
        let recording = self.recording();
        let mut started = false;
        ui.horizontal(|ui| {
            let label = if recording { tr!("rec-stop") } else { tr!("rec-record") };
            let resp = crate::chrome::chip_accent(
                ui,
                recording,
                label,
                crate::theme::PINK(),
                crate::theme::INK_ON_CYAN(),
            )
            .on_hover_text(tr!("rec-record-hint"));
            if resp.clicked()
                && let Some(rec) = self.rec.as_mut()
            {
                // The strip comes along, unless it shows a loaded recording:
                // then REC goes back to live and starts from now.
                let backlog: &[Sweep] =
                    if self.viewing.is_some() { &[] } else { self.recent.make_contiguous() };
                let r = if recording {
                    rec.stop(now_ms())
                } else if pano {
                    rec.start_per_sweep(radio, backlog, now_ms()).map(|_| ())
                } else {
                    rec.start_with(radio, self.interval_s, backlog, now_ms()).map(|_| ())
                };
                match r {
                    Ok(()) => started = !recording,
                    Err(e) => self.fail(e),
                }
                self.dirty = true;
                self.armed_init = false;
                self.armed_load = None;
            }

            // Beside ● REC: the strip as it is, kept as a session of its own,
            // one row per sweep, nothing folded.
            // Why: HDZ 2026-11-13: "den store knopf neben dem record anordnen".
            let can = !recording && self.viewing.is_none() && !self.recent.is_empty();
            let resp = ui
                .add_enabled_ui(can, |ui| crate::chrome::chip(ui, false, tr!("rec-store")))
                .inner
                .on_hover_text(tr!("rec-store-hint"))
                .on_disabled_hover_text(if recording {
                    tr!("rec-store-recording")
                } else if self.viewing.is_some() {
                    tr!("rec-store-loaded")
                } else {
                    tr!("rec-store-empty")
                });
            if resp.clicked()
                && let Some(rec) = self.rec.as_mut()
            {
                match rec.store(radio, self.recent.make_contiguous()) {
                    // Opened, so the new session is seen at once.
                    Ok(Some(id)) => self.expanded = Some(id),
                    Ok(None) => {}
                    Err(e) => self.error = Some(e.to_string()),
                }
                self.dirty = true;
                self.armed_init = false;
                self.armed_load = None;
            }

            // Under a panorama the times on top of the page space the rows.
            if !pano {
                ui.label(tr!("rec-interval"));
                let before = self.interval_s;
                ui.add_enabled(
                    !recording,
                    egui::DragValue::new(&mut self.interval_s)
                        .speed(1.0)
                        .range(MIN_INTERVAL_S..=f64::INFINITY)
                        .fixed_decimals(0)
                        .suffix(" s"),
                )
                .on_hover_text(tr!("rec-interval-hint"))
                .on_disabled_hover_text(tr!("rec-interval-locked"));
                if self.interval_s != before
                    && let Some(rec) = self.rec.as_ref()
                    && let Err(e) = rec.set_interval_s(self.interval_s)
                {
                    self.fail(e);
                }
                let strip_min = self.interval_s * STRIP_ROWS as f64 / 60.0;
                ui.label(
                    RichText::new(tr!("rec-strip-span", min = format!("{strip_min:.0}"))).weak(),
                );
            }
        });

        let status = match self.rec.as_ref().and_then(|r| r.active()) {
            Some((id, rows)) => {
                let started = self.sessions.iter().find(|s| s.id == id).map(|s| s.started_ms);
                let running = started.map_or(0, |t| now_ms() - t);
                tr!(
                    "rec-status-recording",
                    id = id,
                    rows = rows,
                    strip = rows / STRIP_ROWS + 1,
                    time = duration(running)
                )
            }
            None if wide_live && self.viewing.is_none() && !self.recent.is_empty() => {
                let first = self.recent.front().map_or(0, |s| s.t_ms);
                tr!(
                    "rec-status-ready-strip",
                    radio = radio.to_string(),
                    n = self.recent.len(),
                    since = local(first, "%H:%M:%S")
                )
            }
            None if wide_live => tr!("rec-status-ready", radio = radio.to_string()),
            // A panorama under way has a strip coming, just not yet: its first
            // sweep is still going.
            // Why: HDZ 2026-11-13: "fehlermeldung unnötig, sdr scannt gerade".
            None if pano => tr!("rec-status-first-sweep", radio = radio.to_string()),
            None => tr!("rec-status-no-strip"),
        };
        let colour = if recording {
            crate::theme::PINK()
        } else if wide_live || pano {
            crate::theme::TEXT()
        } else {
            crate::theme::ALERT()
        };
        ui.label(RichText::new(status).color(colour));
        started
    }

    fn inventory(&mut self, ui: &mut egui::Ui) -> Option<RecAction> {
        let mut action = None;
        let recording_id = self.rec.as_ref().and_then(|r| r.active()).map(|a| a.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr!(
                    "rec-db-summary",
                    size = mb(self.file_bytes),
                    n = self.sessions.len()
                ))
                .strong(),
            )
            .on_hover_text(
                self.rec.as_ref().map_or(String::new(), |r| r.path().display().to_string()),
            );
            crate::chrome::row_tail(ui, |ui| {
                let enabled = recording_id.is_none() && !self.sessions.is_empty();
                let resp = if self.armed_init {
                    crate::chrome::chip_accent(
                        ui,
                        true,
                        tr!("rec-init-sure"),
                        crate::theme::PINK(),
                        crate::theme::INK_ON_CYAN(),
                    )
                    .on_hover_text(tr!("rec-init-armed-hint"))
                } else {
                    ui.add_enabled_ui(enabled, |ui| crate::chrome::chip(ui, false, tr!("rec-init")))
                        .inner
                        .on_hover_text(tr!("rec-init-hint"))
                        .on_disabled_hover_text(tr!("rec-init-locked"))
                };
                if resp.clicked() {
                    if self.armed_init {
                        if let Some(rec) = self.rec.as_mut() {
                            match rec.init() {
                                Ok(_) => {
                                    if self.viewing.is_some() {
                                        action = Some(RecAction::Live);
                                    }
                                    self.expanded = None;
                                }
                                Err(e) => self.error = Some(e.to_string()),
                            }
                        }
                        self.armed_init = false;
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
                    "rec-init-warning",
                    n = self.sessions.len(),
                    size = mb(self.file_bytes)
                ))
                .color(crate::theme::ALERT()),
            );
            if ui.small_button(tr!("rec-cancel")).clicked() {
                self.armed_init = false;
            }
        }

        if let (Some(_), Some(rid)) = (self.armed_load, recording_id) {
            ui.label(RichText::new(tr!("rec-load-warning", id = rid)).color(crate::theme::ALERT()));
            if ui.small_button(tr!("rec-cancel")).clicked() {
                self.armed_load = None;
            }
        }
        if recording_id.is_none() {
            self.armed_load = None;
        }

        if self.sessions.is_empty() {
            ui.label(RichText::new(tr!("rec-empty")).weak());
            return action;
        }

        let mut toggle = None;
        let mut delete = None;
        let mut load = None;
        let mut arm_load = None;
        egui::ScrollArea::vertical().max_height(280.0).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("rec-sessions").num_columns(8).striped(true).spacing([10.0, 4.0]).show(
                ui,
                |ui| {
                    for h in [
                        tr!("rec-col-when"),
                        tr!("rec-col-radio"),
                        tr!("rec-col-bands"),
                        tr!("rec-col-mhz"),
                        tr!("rec-col-length"),
                        tr!("rec-col-size"),
                        tr!("rec-col-strips"),
                        String::new(),
                    ] {
                        ui.label(RichText::new(h).weak());
                    }
                    ui.end_row();

                    for s in &self.sessions {
                        let open = self.expanded == Some(s.id);
                        // ▶/▼: the ones the interface font has (▸/▾ draw as boxes).
                        let arrow = if open { "▼" } else { "▶" };
                        let when = format!("{arrow} {}", local(s.started_ms, "%d.%m.%y %H:%M"));
                        if ui
                            .add(egui::Label::new(when).sense(egui::Sense::click()))
                            .on_hover_text(tr!("rec-open-strips"))
                            .clicked()
                        {
                            toggle = Some(s.id);
                        }
                        ui.label(&s.radio);
                        let (bands, all) = bands_of(s.lo_hz, s.hi_hz);
                        let r = ui.label(bands);
                        if !all.is_empty() {
                            r.on_hover_text(all);
                        }
                        ui.label(if s.hi_hz > s.lo_hz {
                            format!("{:.3}–{:.3}", s.lo_hz / 1e6, s.hi_hz / 1e6)
                        } else {
                            "—".into()
                        });
                        ui.label(duration(s.ended_ms - s.started_ms)).on_hover_text(every(s));
                        ui.label(mb(s.bytes.max(0) as u64));
                        ui.label(format!("{}", s.strips()));
                        if recording_id == Some(s.id) {
                            ui.label(RichText::new("● REC").color(crate::theme::PINK()));
                        } else if self.armed_delete == Some(s.id) {
                            if crate::chrome::chip_accent(
                                ui,
                                true,
                                tr!("rec-delete-sure"),
                                crate::theme::PINK(),
                                crate::theme::INK_ON_CYAN(),
                            )
                            .on_hover_text(tr!("rec-delete-armed-hint"))
                            .clicked()
                            {
                                delete = Some(s.id);
                            }
                        } else if crate::chrome::chip(ui, false, tr!("rec-delete"))
                            .on_hover_text(tr!("rec-delete-hint"))
                            .clicked()
                        {
                            self.armed_delete = Some(s.id);
                            self.armed_init = false;
                        }
                        ui.end_row();

                        if open {
                            for st in &self.strips {
                                let showing = self
                                    .viewing
                                    .as_ref()
                                    .is_some_and(|v| v.0 == s.id && v.1 == st.k);
                                ui.label(RichText::new(format!("   └ {}", st.k + 1)).weak());
                                ui.label(format!(
                                    "{}–{}",
                                    local(st.first_ms, "%H:%M:%S"),
                                    local(st.last_ms + (s.interval_s * 1000.0) as i64, "%H:%M:%S")
                                ));
                                ui.label(RichText::new(tr!("rec-rows", n = st.rows)).weak());
                                ui.label("");
                                ui.label(duration(
                                    st.last_ms - st.first_ms + (s.interval_s * 1000.0) as i64,
                                ));
                                ui.label("");
                                ui.label("");
                                // While a session records, LOAD stops it — on a
                                // second press, after the warning above.
                                let armed = self.armed_load == Some((s.id, st.k));
                                let resp = if armed {
                                    crate::chrome::chip_accent(
                                        ui,
                                        true,
                                        tr!("rec-load-sure"),
                                        crate::theme::PINK(),
                                        crate::theme::INK_ON_CYAN(),
                                    )
                                    .on_hover_text(tr!("rec-load-armed-hint"))
                                } else {
                                    crate::chrome::chip(ui, showing, tr!("rec-load"))
                                        .on_hover_text(tr!("rec-load-hint"))
                                };
                                if resp.clicked() {
                                    if recording_id.is_some() && !armed {
                                        arm_load = Some((s.id, st.k));
                                    } else {
                                        load = Some((s.id, st.clone(), s.interval_s));
                                    }
                                }
                                ui.end_row();
                            }
                        }
                    }
                },
            );
        });

        if let Some(id) = toggle {
            self.expanded = if self.expanded == Some(id) { None } else { Some(id) };
            self.dirty = true;
        }
        if let Some(id) = delete {
            if let Some(rec) = self.rec.as_mut() {
                if let Err(e) = rec.delete(id) {
                    self.error = Some(e.to_string());
                }
            }
            if self.viewing.as_ref().is_some_and(|v| v.0 == id) {
                action = Some(RecAction::Live);
            }
            if self.expanded == Some(id) {
                self.expanded = None;
            }
            self.armed_delete = None;
            self.dirty = true;
        }
        if arm_load.is_some() {
            self.armed_load = arm_load;
            self.armed_init = false;
            self.armed_delete = None;
        }
        // Loading over a running session: it stops first, and is written
        // up to the row it was gathering.
        if load.is_some() && recording_id.is_some() {
            self.armed_load = None;
            if let Some(rec) = self.rec.as_mut()
                && let Err(e) = rec.stop(now_ms())
            {
                self.error = Some(e.to_string());
            }
            self.dirty = true;
        }
        if let Some((id, st, interval)) = load
            && let Some(rec) = self.rec.as_ref()
        {
            match rec.strip_rows(id, st.k) {
                Ok(rows) if !rows.is_empty() => {
                    let line = tr!(
                        "rec-viewing",
                        id = id,
                        strip = st.k + 1,
                        from = local(st.first_ms, "%d.%m. %H:%M:%S"),
                        to = local(st.last_ms + (interval * 1000.0) as i64, "%H:%M:%S")
                    );
                    self.viewing = Some((id, st.k, line));
                    // The live strip gives way, and its sweeps with it.
                    self.recent.clear();
                    action = Some(RecAction::Load(rows));
                }
                Ok(_) => {}
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_as_hours_minutes_seconds() {
        assert_eq!(duration(0), "0:00:00");
        assert_eq!(duration(1_200_000), "0:20:00");
        assert_eq!(duration(3_723_400), "1:02:03");
    }

    /// A sweep across several bands names them; a slice of one band names it.
    #[test]
    fn bands_are_named_from_the_plan() {
        let all = crate::widgets::bandplan::allocations();
        let (l, h, name) = all[all.len() / 2];
        let (inside, _) = bands_of(l, h);
        assert!(inside.contains(name), "{inside} should name {name}");
        let (within, _) = bands_of(l + (h - l) * 0.25, l + (h - l) * 0.5);
        assert!(within.contains(name), "{within} should name {name}");
        assert_eq!(bands_of(5.0, 5.0).0, "—");
    }
}
