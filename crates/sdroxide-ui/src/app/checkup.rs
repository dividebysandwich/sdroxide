//! The station check, and the welcome a first start gets.
//!
//! Most of what goes wrong with a new installation is not a fault. It is a
//! callsign that was never entered, so FT8 will not transmit and nothing says
//! why; a clock two seconds out, so the band looks dead; a receiver that is
//! muted; a radio that never answered. Each of those is already known
//! somewhere in the program — in a greyed-out button's hover text, a banner, a
//! figure in the corner of one panel — and the operator's problem is that they
//! do not know which of them to go and look at.
//!
//! The station check is all of them on one page, each with what it means and a
//! button that goes to where it is fixed. It measures nothing of its own and
//! transmits nothing: every line is read from state the program already holds,
//! which is also why it is honest about what it cannot see — a clock that has
//! not been measured is reported as not measured, not as fine.
//!
//! The welcome is the same idea at the one moment it matters most: a first
//! start, with no callsign, where the three things to do are enter one, choose
//! the radio, and run the check.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::Command;

use crate::app::SdroxideApp;
use crate::app::settings::SettingsTab;
use crate::theme::ThemedScroll;

/// How a check came out. Ordered worst last, so the page's verdict is the
/// maximum over its lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::app) enum Level {
    /// As it should be.
    Ok,
    /// Not a problem, but worth knowing: receive-only, nothing configured yet.
    Info,
    /// Works, with something wrong with it.
    Warn,
    /// Something will not work until this is dealt with.
    Fail,
}

/// Which check a line is — what the tests name them by, so they do not depend
/// on the wording or the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum CheckId {
    Callsign,
    Grid,
    Link,
    Receiver,
    Level,
    Audio,
    Muted,
    Transmit,
    Clock,
    Lookup,
    Spots,
}

/// Where a line's button goes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Fix {
    Settings(SettingsTab),
    Unmute,
}

pub(in crate::app) struct Check {
    /// Read by the tests only: the page shows every line and has no need to
    /// tell them apart.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(in crate::app) id: CheckId,
    pub(in crate::app) level: Level,
    title: String,
    detail: String,
    fix: Option<Fix>,
}

/// Everything the checks read, gathered into plain values so the judging can
/// be tested without a running application.
#[derive(Default)]
pub(in crate::app) struct CheckInput {
    pub(in crate::app) my_call: String,
    pub(in crate::app) my_grid: String,
    /// The connection's error, if it has one.
    pub(in crate::app) link_error: Option<String>,
    /// The radio has said what it is.
    pub(in crate::app) has_caps: bool,
    /// Seconds since the last spectrum frame; `None` if there has never been one.
    pub(in crate::app) spectrum_age_s: Option<f64>,
    pub(in crate::app) adc_overloaded: bool,
    /// The standing notice about the radio's audio, if there is one.
    pub(in crate::app) radio_notice: Option<String>,
    pub(in crate::app) muted: bool,
    pub(in crate::app) tx_capable: bool,
    /// What FT8's decodes say about this machine's clock, where they have.
    pub(in crate::app) clock_offset_s: Option<f32>,
    pub(in crate::app) lookup_configured: bool,
    pub(in crate::app) feeds_on: usize,
}

/// A spectrum frame older than this means the receiver has stopped: frames
/// arrive many times a second on every backend.
const STALLED_AFTER_S: f64 = 3.0;

/// Whether `call` has the shape of a callsign: letters and digits (and `/` for
/// a portable or a prefix), at least one of each, three characters or more.
/// Not a licence check — only a catch for a name or a grid typed in the wrong
/// box.
pub(in crate::app) fn callsign_shaped(call: &str) -> bool {
    let c = call.trim();
    c.len() >= 3
        && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '/')
        && c.chars().any(|ch| ch.is_ascii_digit())
        && c.chars().any(|ch| ch.is_ascii_alphabetic())
}

/// Whether `grid` is a Maidenhead locator of four or six characters.
pub(in crate::app) fn grid_shaped(grid: &str) -> bool {
    let g = grid.trim();
    matches!(g.len(), 4 | 6) && sdroxide_types::grid_to_latlon(g).is_some()
}

/// Judge the station. Always the same lines in the same order, so the page
/// does not rearrange itself as things come and go.
pub(in crate::app) fn run_checks(i: &CheckInput) -> Vec<Check> {
    let mut v = Vec::with_capacity(11);
    let mut push = |id, level, title: String, detail: String, fix| {
        v.push(Check { id, level, title, detail, fix });
    };
    let general = Some(Fix::Settings(SettingsTab::General));

    let call = i.my_call.trim();
    let (level, detail) = if call.is_empty() {
        (Level::Fail, tr!("check-callsign-missing"))
    } else if !callsign_shaped(call) {
        (Level::Warn, tr!("check-callsign-odd", call = call))
    } else {
        (Level::Ok, call.to_string())
    };
    push(
        CheckId::Callsign,
        level,
        tr!("check-callsign"),
        detail,
        (level != Level::Ok).then_some(general).flatten(),
    );

    let grid = i.my_grid.trim();
    let (level, detail) = if grid.is_empty() {
        (Level::Warn, tr!("check-grid-missing"))
    } else if !grid_shaped(grid) {
        (Level::Fail, tr!("check-grid-odd", grid = grid))
    } else {
        (Level::Ok, grid.to_string())
    };
    push(
        CheckId::Grid,
        level,
        tr!("check-grid"),
        detail,
        (level != Level::Ok).then_some(general).flatten(),
    );

    let radio = Some(Fix::Settings(SettingsTab::Radio));
    let (level, detail) = match (&i.link_error, i.has_caps) {
        (Some(e), _) => (Level::Fail, e.clone()),
        (None, false) => (Level::Warn, tr!("check-link-silent")),
        (None, true) => (Level::Ok, tr!("check-link-ok")),
    };
    push(
        CheckId::Link,
        level,
        tr!("check-link"),
        detail,
        (level != Level::Ok).then_some(radio).flatten(),
    );

    let (level, detail) = match i.spectrum_age_s {
        None => (Level::Warn, tr!("check-receiver-never")),
        Some(age) if age > STALLED_AFTER_S => {
            (Level::Fail, tr!("check-receiver-stalled", seconds = age.round() as u64))
        }
        Some(_) => (Level::Ok, tr!("check-receiver-ok")),
    };
    push(
        CheckId::Receiver,
        level,
        tr!("check-receiver"),
        detail,
        (level != Level::Ok).then_some(radio).flatten(),
    );

    let (level, detail) = if i.adc_overloaded {
        (Level::Warn, tr!("check-level-overload"))
    } else {
        (Level::Ok, tr!("check-level-ok"))
    };
    push(
        CheckId::Level,
        level,
        tr!("check-level"),
        detail,
        (level != Level::Ok).then_some(radio).flatten(),
    );

    let (level, detail) = match &i.radio_notice {
        Some(n) => (Level::Warn, n.clone()),
        None => (Level::Ok, tr!("check-audio-ok")),
    };
    push(
        CheckId::Audio,
        level,
        tr!("check-audio"),
        detail,
        (level != Level::Ok).then_some(radio).flatten(),
    );

    let (level, detail, fix) = if i.muted {
        (Level::Warn, tr!("check-muted-on"), Some(Fix::Unmute))
    } else {
        (Level::Ok, tr!("check-muted-off"), None)
    };
    push(CheckId::Muted, level, tr!("check-muted"), detail, fix);

    let (level, detail) = if i.tx_capable {
        (Level::Ok, tr!("check-transmit-ok"))
    } else {
        (Level::Info, tr!("check-transmit-rx-only"))
    };
    push(CheckId::Transmit, level, tr!("check-transmit"), detail, None);

    let (level, detail) = match i.clock_offset_s {
        None => (Level::Info, tr!("check-clock-unknown")),
        Some(off) => {
            use sdroxide_types::ClockHealth::*;
            let shown = format!("{off:+.1}");
            match sdroxide_types::clock_health(off) {
                Good => (Level::Ok, tr!("check-clock-ok", offset = shown)),
                Marginal => (Level::Warn, tr!("check-clock-marginal", offset = shown)),
                Bad => (Level::Fail, tr!("check-clock-bad", offset = shown)),
            }
        }
    };
    push(CheckId::Clock, level, tr!("check-clock"), detail, None);

    let (level, detail, fix) = if i.lookup_configured {
        (Level::Ok, tr!("check-lookup-ok"), None)
    } else {
        (Level::Info, tr!("check-lookup-none"), Some(Fix::Settings(SettingsTab::Uploads)))
    };
    push(CheckId::Lookup, level, tr!("check-lookup"), detail, fix);

    let (level, detail, fix) = if i.feeds_on > 0 {
        (Level::Ok, tr!("check-spots-ok", n = i.feeds_on), None)
    } else {
        (Level::Info, tr!("check-spots-none"), Some(Fix::Settings(SettingsTab::Spots)))
    };
    push(CheckId::Spots, level, tr!("check-spots"), detail, fix);

    v
}

impl SdroxideApp {
    fn check_input(&self, now: f64) -> CheckInput {
        let net = &self.net_cfg_edit;
        CheckInput {
            my_call: self.digi_cfg_edit.my_call.clone(),
            my_grid: self.my_grid(),
            link_error: self.error.clone(),
            has_caps: self.caps.is_some(),
            spectrum_age_s: (self.last_spectrum_at > 0.0).then_some(now - self.last_spectrum_at),
            adc_overloaded: self.meters.as_ref().is_some_and(|m| m.adc_overloaded()),
            radio_notice: self.radio_notice.clone(),
            muted: self.state.rx[0].muted,
            tx_capable: self.tx_capable(),
            clock_offset_s: self.digi_status.as_ref().and_then(|s| s.clock_offset_s),
            lookup_configured: net.lookup_provider != sdroxide_types::LookupProvider::None,
            feeds_on: [net.cluster.enabled, net.pota.enabled, net.sota.enabled, net.psk.enabled]
                .into_iter()
                .filter(|on| *on)
                .count(),
        }
    }

    fn apply_fix(&mut self, fix: Fix, cmds: &mut Vec<Command>) {
        match fix {
            Fix::Settings(tab) => {
                self.settings_tab = tab;
                self.show_settings = true;
            }
            Fix::Unmute => {
                cmds.push(Command::SetMute { rx: sdroxide_types::RxId::Main, muted: false })
            }
        }
    }

    /// The station check window.
    pub(in crate::app) fn check_window(
        &mut self,
        ctx: &egui::Context,
        now: f64,
        cmds: &mut Vec<Command>,
    ) {
        if !self.show_check {
            return;
        }
        let checks = run_checks(&self.check_input(now));
        let worst = checks.iter().map(|c| c.level).max().unwrap_or(Level::Ok);
        let mut open = true;
        let mut fix: Option<Fix> = None;
        let resp = egui::Window::new(tr!("check-title"))
            .id(crate::layout::salted_id(ctx, "station-check"))
            .frame(crate::chrome::window_frame())
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.set_width(crate::layout::window_w(ctx, 560.0));
                let (verdict, col) = match worst {
                    Level::Ok | Level::Info => (tr!("check-verdict-ok"), crate::theme::GREEN()),
                    Level::Warn => (tr!("check-verdict-warn"), crate::theme::YELLOW()),
                    Level::Fail => (tr!("check-verdict-fail"), crate::theme::PINK()),
                };
                ui.label(RichText::new(verdict).size(13.0).strong().color(col));
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(460.0).show_themed(ui, |ui| {
                    for c in &checks {
                        let (mark, col) = match c.level {
                            Level::Ok => ("✓", crate::theme::GREEN()),
                            Level::Info => ("·", crate::theme::gray(150)),
                            Level::Warn => ("!", crate::theme::YELLOW()),
                            Level::Fail => ("✕", crate::theme::PINK()),
                        };
                        egui::Frame::new()
                            .fill(crate::theme::ROW_BG())
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    // A fixed column for the mark, so the titles
                                    // line up whichever mark a row has.
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(16.0, 18.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        mark,
                                        egui::FontId::proportional(14.0),
                                        col,
                                    );
                                    ui.label(
                                        RichText::new(&c.title)
                                            .size(12.5)
                                            .strong()
                                            .color(crate::theme::TEXT_STRONG()),
                                    );
                                    if let Some(f) = c.fix {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let label = match f {
                                                    Fix::Settings(_) => tr!("check-fix-settings"),
                                                    Fix::Unmute => tr!("check-fix-unmute"),
                                                };
                                                if crate::chrome::chip(
                                                    ui,
                                                    false,
                                                    RichText::new(label).size(11.0),
                                                )
                                                .clicked()
                                                {
                                                    fix = Some(f);
                                                }
                                            },
                                        );
                                    }
                                });
                                ui.horizontal_wrapped(|ui| {
                                    ui.add_space(24.0);
                                    ui.label(
                                        RichText::new(&c.detail)
                                            .size(11.5)
                                            .color(crate::theme::gray(165)),
                                    );
                                });
                            });
                        ui.add_space(2.0);
                    }
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr!("check-footnote")).size(10.5).color(crate::theme::gray(120)),
                );
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_check = open;
        if let Some(f) = fix {
            self.apply_fix(f, cmds);
        }
    }

    /// The welcome a first start gets: shown while there is no callsign and it
    /// has not been dismissed for good.
    ///
    /// Waits for the engine's saved settings to arrive before deciding there
    /// is no callsign — for the first frames of every start there is none
    /// *here*, and an operator of ten years' standing must not be welcomed
    /// each morning.
    pub(in crate::app) fn welcome_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if self.ui_settings.welcomed || self.welcome_later || !self.digi_cfg_seeded {
            return;
        }
        if !self.welcome_shown && !self.digi_cfg_edit.my_call.trim().is_empty() {
            // A station that already has a callsign is not a first start, and
            // is never asked. Once the window *is* showing it stays until it
            // is answered, or typing the callsign into it would close it.
            return;
        }
        self.welcome_shown = true;
        let (mut done, mut later, mut save) = (false, false, false);
        let (mut radio, mut check) = (false, false);
        let resp = egui::Window::new(tr!("welcome-title"))
            .id(crate::layout::salted_id(ctx, "welcome"))
            .frame(crate::chrome::window_frame())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.set_width(crate::layout::window_w(ctx, 470.0));
                ui.label(RichText::new(tr!("welcome-intro")).color(crate::theme::TEXT()));
                let step = |ui: &mut egui::Ui, n: u8, title: String| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("{n}  {title}"))
                            .size(12.5)
                            .strong()
                            .color(crate::theme::CYAN()),
                    );
                    ui.add_space(4.0);
                };

                step(ui, 1, tr!("welcome-step-station"));
                let cfg = &mut self.digi_cfg_edit;
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-callsign"));
                    if crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut cfg.my_call).desired_width(110.0),
                    )
                    .changed()
                    {
                        cfg.my_call = cfg.my_call.to_uppercase();
                    }
                    ui.label(tr!("settings-grid-square"));
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut cfg.my_grid)
                            .hint_text("JN48")
                            .desired_width(70.0),
                    );
                    let ready = callsign_shaped(&cfg.my_call)
                        && (cfg.my_grid.trim().is_empty() || grid_shaped(&cfg.my_grid));
                    ui.add_enabled_ui(ready, |ui| {
                        if crate::chrome::chip_accent(
                            ui,
                            false,
                            RichText::new(tr!("logbook-save")).strong(),
                            crate::theme::GREEN(),
                            crate::theme::INK_ON_CYAN(),
                        )
                        .clicked()
                        {
                            save = true;
                        }
                    });
                });
                ui.label(
                    RichText::new(tr!("welcome-station-hint"))
                        .size(11.0)
                        .color(crate::theme::gray(140)),
                );

                step(ui, 2, tr!("welcome-step-radio"));
                ui.horizontal_wrapped(|ui| {
                    if crate::chrome::chip(ui, false, tr!("welcome-open-radio")).clicked() {
                        radio = true;
                    }
                    ui.label(
                        RichText::new(tr!("welcome-radio-hint"))
                            .size(11.0)
                            .color(crate::theme::gray(140)),
                    );
                });

                step(ui, 3, tr!("welcome-step-check"));
                ui.horizontal_wrapped(|ui| {
                    if crate::chrome::chip(ui, false, tr!("welcome-open-check")).clicked() {
                        check = true;
                    }
                    ui.label(
                        RichText::new(tr!("welcome-check-hint"))
                            .size(11.0)
                            .color(crate::theme::gray(140)),
                    );
                });

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if crate::chrome::chip(ui, false, tr!("welcome-done"))
                        .on_hover_text(tr!("welcome-done", "hint"))
                        .clicked()
                    {
                        done = true;
                    }
                    if crate::chrome::chip(ui, false, tr!("welcome-later"))
                        .on_hover_text(tr!("welcome-later", "hint"))
                        .clicked()
                    {
                        later = true;
                    }
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        if save {
            self.digi_cfg_edit.my_call = self.digi_cfg_edit.my_call.trim().to_uppercase();
            self.digi_cfg_edit.my_grid = self.digi_cfg_edit.my_grid.trim().to_string();
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
        if radio {
            self.settings_tab = SettingsTab::Radio;
            self.show_settings = true;
        }
        if check {
            self.show_check = true;
        }
        // Opening Settings or the check puts this window aside for the
        // session rather than leaving it on top of what it just opened.
        if later || radio || check {
            self.welcome_later = true;
        }
        if done {
            self.ui_settings.welcomed = true;
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A station with nothing wrong with it.
    fn healthy() -> CheckInput {
        CheckInput {
            my_call: "W1AW".into(),
            my_grid: "FN31pr".into(),
            link_error: None,
            has_caps: true,
            spectrum_age_s: Some(0.05),
            adc_overloaded: false,
            radio_notice: None,
            muted: false,
            tx_capable: true,
            clock_offset_s: Some(0.1),
            lookup_configured: true,
            feeds_on: 2,
        }
    }

    fn level(i: &CheckInput, id: CheckId) -> Level {
        run_checks(i).into_iter().find(|c| c.id == id).unwrap().level
    }

    #[test]
    fn a_healthy_station_passes_every_check() {
        let checks = run_checks(&healthy());
        assert_eq!(checks.len(), 11);
        for c in &checks {
            assert_eq!(c.level, Level::Ok, "{:?}", c.id);
            assert!(c.fix.is_none(), "{:?} offers a fix for nothing", c.id);
        }
    }

    #[test]
    fn a_fresh_install_is_told_what_is_missing_and_where_to_fix_it() {
        let fresh = CheckInput::default();
        assert_eq!(level(&fresh, CheckId::Callsign), Level::Fail);
        assert_eq!(level(&fresh, CheckId::Grid), Level::Warn);
        assert_eq!(level(&fresh, CheckId::Link), Level::Warn);
        assert_eq!(level(&fresh, CheckId::Receiver), Level::Warn);
        // Not having these is a choice, not a fault.
        assert_eq!(level(&fresh, CheckId::Transmit), Level::Info);
        assert_eq!(level(&fresh, CheckId::Lookup), Level::Info);
        assert_eq!(level(&fresh, CheckId::Spots), Level::Info);
        // And a clock nobody has measured is not reported as fine.
        assert_eq!(level(&fresh, CheckId::Clock), Level::Info);
        let checks = run_checks(&fresh);
        let call = checks.iter().find(|c| c.id == CheckId::Callsign).unwrap();
        assert!(call.fix == Some(Fix::Settings(SettingsTab::General)));
    }

    #[test]
    fn each_fault_is_reported_on_its_own_line() {
        let with = |f: fn(&mut CheckInput)| {
            let mut i = healthy();
            f(&mut i);
            i
        };
        assert_eq!(level(&with(|i| i.my_call = "Alice".into()), CheckId::Callsign), Level::Warn);
        assert_eq!(level(&with(|i| i.my_grid = "ZZ99".into()), CheckId::Grid), Level::Fail);
        assert_eq!(
            level(&with(|i| i.link_error = Some("refused".into())), CheckId::Link),
            Level::Fail
        );
        assert_eq!(level(&with(|i| i.spectrum_age_s = Some(12.0)), CheckId::Receiver), Level::Fail);
        assert_eq!(level(&with(|i| i.adc_overloaded = true), CheckId::Level), Level::Warn);
        assert_eq!(
            level(&with(|i| i.radio_notice = Some("mono card".into())), CheckId::Audio),
            Level::Warn
        );
        assert_eq!(level(&with(|i| i.muted = true), CheckId::Muted), Level::Warn);
        assert_eq!(level(&with(|i| i.clock_offset_s = Some(-0.9)), CheckId::Clock), Level::Warn);
        assert_eq!(level(&with(|i| i.clock_offset_s = Some(2.4)), CheckId::Clock), Level::Fail);
        // One fault leaves every other line alone.
        let muted = run_checks(&with(|i| i.muted = true));
        assert_eq!(muted.iter().filter(|c| c.level != Level::Ok).count(), 1);
    }

    #[test]
    fn callsigns_and_grids_are_recognised_by_shape() {
        for ok in ["W1AW", "EA5/G4XYZ/P", "9A1A", "ja1abc"] {
            assert!(callsign_shaped(ok), "{ok}");
        }
        for bad in ["", "AB", "ALICE", "12345", "W1 AW", "FN31!"] {
            assert!(!callsign_shaped(bad), "{bad}");
        }
        assert!(grid_shaped("JN48"));
        assert!(grid_shaped("fn31pr"));
        assert!(!grid_shaped("JN4"));
        assert!(!grid_shaped("ZZ99"));
        assert!(!grid_shaped("W1AW"));
    }
}
