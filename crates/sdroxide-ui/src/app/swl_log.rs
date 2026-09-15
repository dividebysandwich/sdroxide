//! The listener's reception log: what was **heard**, not worked.
//!
//! A window in the same family as the LOGBOOK, with its own file
//! (`swl_log.json`) and its own record ([`sdroxide_types::SwlEntry`]). It logs
//! a station, a frequency, a time and a SINPO or SIO judgement, and can print
//! the entry as a reception report to send to the broadcaster.
//!
//! Owned by the UI, exactly like the QSO log next door: the table lives in the
//! app, is loaded from the config directory, and is written back when it
//! changes. The engine is not involved.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Mode, SignalReport, Sio, Sinpo, SwlEntry};

use crate::app::SdroxideApp;
use crate::app::persist::persist_swl_log;
use crate::time::now_unix;

/// The modes a listener is likely to log, in the order that makes sense for a
/// picker. A short list rather than every mode: a reception is AM or a sideband
/// or CW or one of the broadcast ones, and forty entries would bury those.
const SWL_MODES: [Mode; 9] = [
    Mode::Am,
    Mode::Sam,
    Mode::Cquam,
    Mode::Lsb,
    Mode::Usb,
    Mode::Cw,
    Mode::Nfm,
    Mode::Wfm,
    Mode::Drm,
];

/// `2026-09-16 19:42 UTC` from Unix seconds.
fn utc_text(unix: u64) -> String {
    let (y, mo, d, h, mi, _s) = sdroxide_types::utc_ymd_hms(unix as i64);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02} UTC")
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

/// A reception being typed, the frequency kept as text so partial input never
/// fights the operator. Parsed into a [`SwlEntry`] on save.
#[derive(Clone)]
pub(in crate::app) struct SwlEditForm {
    /// 0 = new entry; otherwise the id of the entry being edited.
    id: u64,
    heard_at: u64,
    station: String,
    freq_khz: String,
    mode: Mode,
    language: String,
    /// True = SINPO (five figures), false = SIO (three).
    sinpo: bool,
    /// Whether a report was given at all. Off = "not judged".
    judged: bool,
    s: u8,
    i: u8,
    n: u8,
    p: u8,
    o: u8,
    smeter_dbm: Option<f32>,
    site: String,
    notes: String,
}

impl Default for SwlEditForm {
    fn default() -> Self {
        SwlEditForm {
            id: 0,
            heard_at: 0,
            station: String::new(),
            freq_khz: String::new(),
            mode: Mode::Am,
            language: String::new(),
            sinpo: true,
            judged: false,
            s: 3,
            i: 3,
            n: 3,
            p: 3,
            o: 3,
            smeter_dbm: None,
            site: String::new(),
            notes: String::new(),
        }
    }
}

impl SwlEditForm {
    /// A fresh entry, pre-filled from the dial so logging a station found by
    /// tuning is a name and a judgement.
    pub(in crate::app) fn new(freq_hz: f64, mode: Mode, smeter_dbm: Option<f32>) -> Self {
        SwlEditForm {
            id: 0,
            heard_at: now_unix().max(0) as u64,
            freq_khz: format!("{:.0}", freq_hz / 1e3),
            mode,
            smeter_dbm,
            sinpo: true,
            s: 3,
            i: 3,
            n: 3,
            p: 3,
            o: 3,
            ..Default::default()
        }
    }

    /// A fresh entry from a schedule row: the station, language and site are
    /// already known, so a reception from the schedule is a judgement away.
    pub(in crate::app) fn from_station(
        freq_hz: f64,
        mode: Mode,
        station: &str,
        language: &str,
        site: &str,
        smeter_dbm: Option<f32>,
    ) -> Self {
        let mut f = Self::new(freq_hz, mode, smeter_dbm);
        f.station = station.to_string();
        f.language = language.to_string();
        f.site = site.to_string();
        f
    }

    fn from_entry(e: &SwlEntry) -> Self {
        let (sinpo, judged, s, i, n, p, o) = match e.report {
            Some(SignalReport::Sinpo(r)) => (true, true, r.s, r.i, r.n, r.p, r.o),
            Some(SignalReport::Sio(r)) => (false, true, r.s, r.i, 3, 3, r.o),
            None => (true, false, 3, 3, 3, 3, 3),
        };
        SwlEditForm {
            id: e.id,
            heard_at: e.heard_at_unix,
            station: e.station.clone(),
            freq_khz: format!("{:.0}", e.freq_hz / 1e3),
            mode: e.mode,
            language: e.language.clone(),
            sinpo,
            judged,
            s,
            i,
            n,
            p,
            o,
            smeter_dbm: e.smeter_dbm,
            site: e.site.clone(),
            notes: e.notes.clone(),
        }
    }

    fn to_entry(&self) -> SwlEntry {
        let freq_hz = self
            .freq_khz
            .trim()
            .parse::<f64>()
            .ok()
            .map(|k| k * 1e3)
            .unwrap_or(0.0);
        let cl = |v: u8| v.clamp(1, 5);
        let report = self.judged.then(|| {
            if self.sinpo {
                SignalReport::Sinpo(Sinpo {
                    s: cl(self.s),
                    i: cl(self.i),
                    n: cl(self.n),
                    p: cl(self.p),
                    o: cl(self.o),
                })
            } else {
                SignalReport::Sio(Sio { s: cl(self.s), i: cl(self.i), o: cl(self.o) })
            }
        });
        SwlEntry {
            id: self.id,
            heard_at_unix: self.heard_at,
            station: self.station.trim().to_string(),
            freq_hz,
            mode: self.mode,
            language: self.language.trim().to_string(),
            report,
            smeter_dbm: self.smeter_dbm,
            site: self.site.trim().to_string(),
            notes: self.notes.trim().to_string(),
        }
    }
}

impl SdroxideApp {
    /// The LISTEN window: the reception log, its entry form and its report.
    pub(in crate::app) fn swl_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut open = self.show_swl;
        let resp = egui::Window::new("LISTEN")
            .id(crate::layout::salted_id(ctx, "LISTEN"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 720.0))
            .default_height(crate::layout::window_h(ctx, 520.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.horizontal(|ui| {
                    if crate::chrome::chip(ui, false, "+ NEW").clicked() {
                        let freq = self.on_air_freq_hz();
                        let mode = self.state.rx[0].mode;
                        let s = self.meters.map(|m| m.s_dbm);
                        self.swl_edit = Some(SwlEditForm::new(freq, mode, s));
                    }
                    if crate::chrome::chip(ui, false, "JOBS")
                        .on_hover_text("Scheduled recordings — record a band at a set time")
                        .clicked()
                    {
                        self.jobs.show = true;
                    }
                    let replay = self.state.replay;
                    if crate::chrome::chip(ui, replay, "REPLAY")
                        .on_hover_text(
                            "Play the last two minutes instead of live — catch the station id \
                             you just missed",
                        )
                        .clicked()
                    {
                        cmds.push(Command::SetReplay(!replay));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let selected = self
                            .swl_selected
                            .and_then(|id| self.swl_log.iter().find(|e| e.id == id));
                        ui.add_enabled_ui(selected.is_some(), |ui| {
                            if crate::chrome::chip(ui, false, "REPORT")
                                .on_hover_text(
                                    "Copy this reception as a report to send to the station",
                                )
                                .clicked()
                                && let Some(e) = selected
                            {
                                let grid = self.my_grid();
                                let text = e.report_text(&grid, "sdroxide", "");
                                crate::download::save("reception-report.txt", text.as_bytes());
                            }
                        });
                        ui.label(
                            RichText::new(format!("{} heard", self.swl_log.len()))
                                .size(11.0)
                                .color(crate::theme::gray(150)),
                        );
                    });
                });
                // The listener's tone control: shelves on the demodulated
                // audio, in front of the speakers. Broadcast audio wants a
                // tone control the ham speech chain never needed.
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Tone").size(11.0).color(crate::theme::gray(150)));
                    let mut tone = self.state.rx_tone.clone();
                    let before = tone.clone();
                    crate::chrome::checkbox(ui, &mut tone.enabled, "on");
                    let mut band = |ui: &mut egui::Ui, name: &str, b: &mut sdroxide_types::TxEqBand| {
                        ui.label(RichText::new(name).size(11.0));
                        ui.add(
                            egui::DragValue::new(&mut b.gain_db)
                                .speed(0.2)
                                .range(-12.0..=12.0)
                                .suffix(" dB"),
                        );
                    };
                    band(ui, "Bass", &mut tone.low);
                    band(ui, "Mid", &mut tone.mid);
                    band(ui, "Treble", &mut tone.high);
                    if tone != before {
                        self.state.rx_tone = tone.clone();
                        cmds.push(Command::SetRxTone(Box::new(tone)));
                    }
                });
                if self.swl_edit.is_some() {
                    ui.add_space(4.0);
                    self.swl_entry_form(ui);
                }
                ui.separator();
                self.swl_list(ui);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_swl = open;
    }

    /// The reception log list, newest first, grouped by day.
    fn swl_list(&mut self, ui: &mut egui::Ui) {
        // A copy, so the list can be drawn while `self` stays free to record a
        // selection or a delete.
        let mut rows = self.swl_log.clone();
        rows.sort_by_key(|e| std::cmp::Reverse(e.heard_at_unix));
        let mut selected = self.swl_selected;
        let mut edit: Option<u64> = None;
        let mut delete: Option<u64> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("swl-list").show(
            ui,
            |ui| {
                if rows.is_empty() {
                    ui.label(
                        RichText::new("Nothing logged yet — tune a station and press + NEW.")
                            .color(crate::theme::gray(150)),
                    );
                }
                let mut last_day = String::new();
                for e in &rows {
                    let utc = utc_text(e.heard_at_unix);
                    let day = utc[..10].to_string();
                    if day != last_day {
                        ui.add_space(4.0);
                        ui.label(RichText::new(&day).size(11.0).strong().color(crate::theme::CYAN()));
                        last_day = day;
                    }
                    let report = e
                        .report
                        .map(|r| format!("{} {}", r.label(), r.digits()))
                        .unwrap_or_default();
                    let label = format!(
                        "{:>6} kHz  {:<4}  {:<26} {:<10} {}",
                        format!("{:.0}", e.freq_hz / 1e3),
                        e.mode.label(),
                        truncate(&e.station, 26),
                        truncate(&e.language, 10),
                        report,
                    );
                    ui.horizontal(|ui| {
                        let is_sel = selected == Some(e.id);
                        if ui.selectable_label(is_sel, RichText::new(label).monospace()).clicked() {
                            selected = Some(e.id);
                        }
                        ui.label(
                            RichText::new(&utc).size(10.5).color(crate::theme::gray(140)),
                        );
                        if !e.notes.is_empty() {
                            ui.label(
                                RichText::new(truncate(&e.notes, 48))
                                    .size(10.5)
                                    .color(crate::theme::gray(160)),
                            );
                        }
                        if ui.small_button("edit").clicked() {
                            edit = Some(e.id);
                        }
                        if ui.small_button("del").clicked() {
                            delete = Some(e.id);
                        }
                    });
                }
            },
        );
        self.swl_selected = selected;
        if let Some(id) = edit
            && let Some(e) = self.swl_log.iter().find(|e| e.id == id)
        {
            self.swl_edit = Some(SwlEditForm::from_entry(e));
        }
        if let Some(id) = delete {
            self.swl_log.retain(|e| e.id != id);
            if self.swl_selected == Some(id) {
                self.swl_selected = None;
            }
            persist_swl_log(&self.swl_log);
        }
    }

    /// The reception entry form.
    fn swl_entry_form(&mut self, ui: &mut egui::Ui) {
        let mut save = false;
        let mut cancel = false;
        let mut heard_now = false;
        {
            let f = self.swl_edit.as_mut().unwrap();
            egui::Frame::new()
                .fill(crate::theme::ROW_BG())
                .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP()))
                .inner_margin(egui::Margin::same(9))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(if f.id == 0 { "NEW RECEPTION" } else { "EDIT RECEPTION" })
                            .size(11.0)
                            .strong()
                            .color(crate::theme::CYAN()),
                    );
                    ui.add_space(4.0);
                    egui::Grid::new("swl-form").num_columns(4).spacing([10.0, 6.0]).show(
                        ui,
                        |ui| {
                            ui.label("Station");
                            crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut f.station)
                                    .desired_width(220.0)
                                    .hint_text("Radio Taiwan International"),
                            );
                            ui.label("Language");
                            crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut f.language)
                                    .desired_width(120.0)
                                    .hint_text("English"),
                            );
                            ui.end_row();

                            ui.label("Frequency");
                            ui.horizontal(|ui| {
                                crate::chrome::field(
                                    ui,
                                    egui::TextEdit::singleline(&mut f.freq_khz).desired_width(84.0),
                                );
                                ui.label("kHz");
                                egui::ComboBox::from_id_salt("swl-mode")
                                    .width(84.0)
                                    .selected_text(f.mode.label())
                                    .show_ui(ui, |ui| {
                                        for m in SWL_MODES {
                                            ui.selectable_value(&mut f.mode, m, m.label());
                                        }
                                    });
                            });
                            ui.label("Site");
                            crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut f.site).desired_width(160.0),
                            );
                            ui.end_row();

                            ui.label("Heard");
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(utc_text(f.heard_at)).monospace());
                                if crate::chrome::chip(ui, false, "NOW").clicked() {
                                    heard_now = true;
                                }
                            });
                            ui.label("S-meter");
                            ui.label(match f.smeter_dbm {
                                Some(db) => format!("{db:.0} dBm"),
                                None => "—".into(),
                            });
                            ui.end_row();

                            ui.label("Report");
                            ui.horizontal(|ui| {
                                crate::chrome::checkbox(ui, &mut f.judged, "judged");
                                ui.add_enabled_ui(f.judged, |ui| {
                                    ui.selectable_value(&mut f.sinpo, true, "SINPO");
                                    ui.selectable_value(&mut f.sinpo, false, "SIO");
                                });
                            });
                            if f.judged {
                                ui.horizontal(|ui| {
                                    let mut fig = |ui: &mut egui::Ui, name: &str, v: &mut u8| {
                                        ui.label(name);
                                        ui.add(
                                            egui::DragValue::new(v).speed(0.1).range(1..=5u8),
                                        );
                                    };
                                    fig(ui, "S", &mut f.s);
                                    fig(ui, "I", &mut f.i);
                                    if f.sinpo {
                                        fig(ui, "N", &mut f.n);
                                        fig(ui, "P", &mut f.p);
                                    }
                                    fig(ui, "O", &mut f.o);
                                });
                                ui.label("");
                                ui.label("");
                            } else {
                                ui.label("");
                            }
                            ui.end_row();

                            ui.label("Notes");
                            crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut f.notes)
                                    .desired_width(320.0)
                                    .hint_text("programme notes"),
                            );
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        },
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if crate::chrome::chip(ui, true, "SAVE").clicked() {
                            save = true;
                        }
                        if crate::chrome::chip(ui, false, "CANCEL").clicked() {
                            cancel = true;
                        }
                    });
                });
        }
        if heard_now {
            let s = self.meters.map(|m| m.s_dbm);
            if let Some(f) = self.swl_edit.as_mut() {
                f.heard_at = now_unix().max(0) as u64;
                f.smeter_dbm = s;
            }
        }
        if save
            && let Some(f) = self.swl_edit.take()
        {
            let mut entry = f.to_entry();
            if entry.id == 0 {
                entry.id = self.swl_log.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                self.swl_log.push(entry);
            } else if let Some(slot) = self.swl_log.iter_mut().find(|e| e.id == entry.id) {
                *slot = entry;
            }
            persist_swl_log(&self.swl_log);
        }
        if cancel {
            self.swl_edit = None;
        }
    }
}
