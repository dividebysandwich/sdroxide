//! The contest logger window.
//!
//! Mode-agnostic: the operator types the exchange by hand for CW, SSB or
//! anything else, and the FT8 side auto-fills the same entry when it drives one
//! of the contests that has an FT8 layout. A session is the operator's own
//! state (see [`sdroxide_types::ContestSession`]); the QSOs it logs go into the
//! ordinary logbook, tagged with the contest, so scoring and the Cabrillo
//! export read the same rows everything else does.

use eframe::egui;
use sdroxide_types::{Command, ContestId, ContestSession, Exchange, QsoRecord};

use super::SdroxideApp;

/// The in-progress entry: what the operator is typing for the station being
/// worked. Session-only, cleared after every logged QSO.
#[derive(Default)]
pub(in crate::app) struct ContestEntry {
    pub call: String,
    /// The report they gave us (we default ours to 59).
    pub rst_rcvd: String,
    /// Their exchange — the serial, zone, state, grid or CB text.
    pub exchange: String,
}

impl SdroxideApp {
    pub(in crate::app) fn contest_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.show_contest {
            return;
        }
        let mut open = self.show_contest;
        egui::Window::new("CONTEST")
            .id(crate::layout::salted_id(ctx, "CONTEST"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 660.0))
            .default_height(crate::layout::window_h(ctx, 540.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                self.contest_body(ui, cmds);
            });
        self.show_contest = open;
    }

    fn contest_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        if self.contest.is_none() {
            self.contest_setup(ui, cmds);
        } else {
            self.contest_run(ui, cmds);
        }
    }

    /// The pre-session setup: pick the contest and enter our own exchange.
    fn contest_setup(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.label(
            "Pick a contest and enter what you send. The logger works on any mode — \
             type the exchange for CW or SSB, and the FT8 side fills it in by itself \
             where the contest has an FT8 layout.",
        );
        ui.add_space(8.0);
        let mut picked = self.contest_pick;
        ui.horizontal_wrapped(|ui| {
            for c in ContestId::CHOICES {
                if crate::chrome::chip(ui, picked == c, c.label()).clicked() {
                    picked = c;
                }
            }
        });
        self.contest_pick = picked;
        let spec = picked.spec();
        ui.add_space(8.0);
        ui.label(egui::RichText::new(spec.name).strong());
        ui.label(format!("  you send: {}", exchange_hint(spec.sent)));
        ui.label(format!("  they send: {}", exchange_hint(spec.rcvd)));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Your exchange");
            crate::chrome::field(ui, egui::TextEdit::singleline(&mut self.contest_my_exchange));
        });
        ui.add_space(8.0);
        if crate::chrome::chip(ui, true, " START ").clicked() {
            let now = crate::time::now_unix_f64() as i64;
            self.contest =
                Some(ContestSession::new(picked, self.contest_my_exchange.trim().into(), now));
            self.contest_entry = Default::default();
            // Tell the digi engine which FT8 contest layout to send, when this
            // contest has one. A hand-typed CW/SSB contest sets `None` — there
            // is no message layout to choose — and the logger works regardless.
            cmds.push(sdroxide_types::Command::SetDigiConfig({
                let mut cfg = self.digi_cfg_edit.clone();
                cfg.contest = digi_contest_for(picked);
                cfg
            }));
        }
    }

    /// The running session: entry, dupes, score and the session's log.
    fn contest_run(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some(contest) = self.contest.as_ref().map(|s| s.contest) else { return };
        let spec = contest.spec();
        let mine = self.session_qsos();
        let score = sdroxide_types::score(&mine, contest);
        let now = crate::time::now_unix_f64() as i64;
        let r10 = sdroxide_types::rate(&mine, now, 600);
        let r60 = sdroxide_types::rate(&mine, now, 3600);

        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(contest.label()).strong());
            ui.label(format!(
                "QSO {} · {} pts · {} mult · SCORE {}",
                score.qsos, score.points, score.mults, score.total
            ));
            ui.label(format!("{r10} in 10 min · {r60}/hr"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::chrome::chip(ui, false, " STOP ").clicked() {
                    self.contest = None;
                }
            });
        });
        if spec.multiplier != sdroxide_types::Multiplier::None {
            ui.label(
                egui::RichText::new("score is an estimate — the sponsor adjudicates")
                    .size(10.0)
                    .color(crate::theme::gray(140)),
            );
        }
        ui.separator();

        // ── The entry form ────────────────────────────────────────────────
        let dupe = {
            let call = self.contest_entry.call.trim();
            !call.is_empty()
                && sdroxide_types::worked_before(
                    &self.qso_log,
                    call,
                    self.state.band.label(),
                    "",
                    0,
                )
        };
        ui.horizontal(|ui| {
            ui.label("CALL");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.contest_entry.call).desired_width(120.0),
            );
            if dupe {
                ui.label(egui::RichText::new("DUPE").strong().color(crate::theme::ALERT()));
            }
            ui.label("RST");
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.contest_entry.rst_rcvd).desired_width(46.0),
            );
            ui.label(spec.rcvd.last().map_or("EXCH", |e| e.label()));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.contest_entry.exchange).desired_width(80.0),
            );
        });
        ui.add_space(4.0);
        let serial = self.contest.as_ref().map(|s| s.next_serial).unwrap_or(1);
        ui.horizontal(|ui| {
            if spec.sent.contains(&Exchange::Serial) {
                ui.label(format!("sending 599 {serial}"));
            } else if !self.contest_my_exchange.trim().is_empty() {
                ui.label(format!("sending 599 {}", self.contest_my_exchange.trim()));
            }
            ui.label(format!("· {} {}", self.state.band.label(), self.state.rx[0].mode.label()));
            if crate::chrome::chip(ui, true, " LOG ").clicked() {
                self.log_contest_qso(cmds);
            }
            if crate::chrome::chip(ui, false, " CABRILLO ").clicked() {
                let cab = sdroxide_types::to_cabrillo(
                    contest,
                    &self.digi_cfg_edit.my_call,
                    &self.contest_my_exchange,
                    &mine,
                );
                crate::download::save("sdroxide.cab", cab.as_bytes());
            }
        });
        ui.separator();

        // ── The session's log ─────────────────────────────────────────────
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("contest-log-grid")
                .num_columns(5)
                .spacing([12.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    for q in mine.iter().rev().take(200) {
                        let (_, _, _, h, mi, _) = sdroxide_types::utc_ymd_hms(q.start_utc);
                        ui.label(format!("{h:02}{mi:02}"));
                        ui.label(&q.call);
                        ui.label(&q.band);
                        ui.label(&q.mode);
                        ui.label(&q.srx_string);
                        ui.end_row();
                    }
                });
        });
    }

    /// The current session's QSOs: the log's rows tagged with this contest and
    /// logged since the session started.
    fn session_qsos(&self) -> Vec<QsoRecord> {
        let Some(s) = self.contest.as_ref() else { return Vec::new() };
        let id = s.contest.label();
        self.qso_log
            .iter()
            .filter(|q| q.contest_id == id && q.start_utc >= s.started_utc)
            .cloned()
            .collect()
    }

    /// Log the typed entry and advance the serial.
    fn log_contest_qso(&mut self, cmds: &mut Vec<Command>) {
        let Some(session) = self.contest.as_ref() else { return };
        let call = self.contest_entry.call.trim().to_ascii_uppercase();
        if call.is_empty() {
            return;
        }
        let spec = session.contest.spec();
        let serial = spec.sent.contains(&Exchange::Serial).then_some(session.next_serial);
        let now = crate::time::now_unix_f64() as i64;
        let rec = QsoRecord {
            call,
            rst_sent: parse_rst(&self.contest_entry.rst_rcvd).or(Some(59)),
            rst_rcvd: parse_rst(&self.contest_entry.rst_rcvd).or(Some(59)),
            freq_hz: self.on_air_freq_hz(),
            mode: self.state.rx[0].mode.label().to_string(),
            band: self.state.band.label().to_string(),
            start_utc: now,
            end_utc: now,
            my_call: self.digi_cfg_edit.my_call.clone(),
            contest_id: session.contest.label().to_string(),
            stx: serial,
            srx: parse_serial(&self.contest_entry.exchange),
            stx_string: session.my_exchange.clone(),
            srx_string: self.contest_entry.exchange.trim().to_string(),
            ..Default::default()
        };
        cmds.push(Command::LogQso(Box::new(rec)));
        if let Some(s) = self.contest.as_mut() {
            if s.contest.sends_serial() {
                s.next_serial = sdroxide_types::next_contest_serial(s.next_serial);
            }
        }
        self.contest_entry = Default::default();
    }
}

/// "RST + SERIAL + GRID" — the exchange spelled out for the setup screen.
fn exchange_hint(fields: &[Exchange]) -> String {
    if fields.is_empty() {
        return "—".to_string();
    }
    fields.iter().map(|f| f.label()).collect::<Vec<_>>().join(" + ")
}

/// The FT8 message layout a contest logger session should put the digi engine
/// into: EU VHF for an EU VHF contest, and `None` for the rest, which are typed
/// by hand. (A serial FT8 contest would set the serial layout, but that layout
/// is not in `ContestMode` yet — see the contest-exchange work.)
fn digi_contest_for(c: ContestId) -> sdroxide_types::ContestMode {
    match c {
        ContestId::EuVhf => sdroxide_types::ContestMode::EuVhf,
        _ => sdroxide_types::ContestMode::None,
    }
}

fn parse_serial(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

fn parse_rst(s: &str) -> Option<i16> {
    s.trim().parse().ok()
}
