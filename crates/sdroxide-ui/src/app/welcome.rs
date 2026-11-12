//! The welcome walkthrough: a first start's guided tour of what has to be set,
//! what usually goes wrong, and what is in the program.
//!
//! A newcomer's trouble is rarely a fault. It is a callsign never entered, so
//! FT8 will not transmit; a clock two seconds out, so the band looks dead; an
//! RTL-SDR on Windows with the wrong driver bound to it; a transceiver whose
//! CAT family is set to Yaesu when it is a Kenwood. Each of those has an
//! answer somewhere in the manual, and the newcomer does not yet know which
//! page to read. The other half of the problem is the opposite one: a program
//! with forty modes, a logbook, maps, spot feeds, Winlink and a server mode
//! shows a first-time operator a waterfall, and most of it is never found.
//!
//! So this is seven short pages, each one thing:
//!
//! 1. **Welcome** — what this is, and the language it speaks.
//! 2. **Station** — callsign and locator, which everything else reads.
//! 3. **Radio** — what kind of radio, the pitfalls of *that* kind, and the
//!    live state of the link and the receiver, so the page says whether it has
//!    worked.
//! 4. **Before you transmit** — clock, levels, the band lockout, the T/R
//!    switch: the things that cost contacts or hardware.
//! 5. **Online** — callbook, spots, log uploads. All optional, all off until
//!    configured.
//! 6. **Tour** — the features worth knowing are there, each with a button
//!    that opens it.
//! 7. **Check** — the station check's verdict, and what is left to do.
//!
//! The window is not modal and stays open while what it opens is used: the
//! point is to configure the radio *with* the page that explains it beside
//! it. Nothing in it transmits; the tour's mode buttons only change what is
//! received.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{Action, Command, Mode};

use crate::app::SdroxideApp;
use crate::app::checkup::{self, CheckId, Fix, Level};
use crate::app::palette::Win;
use crate::app::settings::SettingsTab;
use crate::theme::ThemedScroll;

/// A message and its `.hint`, both checked at compile time like any `tr!`.
macro_rules! pair {
    ($id:literal) => {
        (tr!($id), tr!($id, "hint"))
    };
}

/// The pages, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Step {
    Welcome,
    Station,
    Radio,
    Transmit,
    Online,
    Tour,
    Check,
}

impl Step {
    pub(in crate::app) const ALL: [Step; 7] = [
        Step::Welcome,
        Step::Station,
        Step::Radio,
        Step::Transmit,
        Step::Online,
        Step::Tour,
        Step::Check,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    pub(in crate::app) fn next(self) -> Step {
        Self::ALL[(self.index() + 1).min(Self::ALL.len() - 1)]
    }

    pub(in crate::app) fn prev(self) -> Step {
        Self::ALL[self.index().saturating_sub(1)]
    }

    fn title(self) -> String {
        match self {
            Step::Welcome => tr!("welcome-step-welcome"),
            Step::Station => tr!("welcome-step-station"),
            Step::Radio => tr!("welcome-step-radio"),
            Step::Transmit => tr!("welcome-step-transmit"),
            Step::Online => tr!("welcome-step-online"),
            Step::Tour => tr!("welcome-step-tour"),
            Step::Check => tr!("welcome-step-check"),
        }
    }
}

/// The kind of radio the operator says they have. Not a backend: several
/// backends share one set of pitfalls, and the operator knows what is on the
/// desk before they know what sdroxide calls the interface to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum RadioKind {
    /// An SDR on USB: RTL-SDR, Airspy, HackRF, SDRplay, RX-888, Lime, Fobos.
    UsbSdr,
    /// A transceiver with a CAT port and a sound card (built in or external).
    CatRig,
    /// A radio on the network: Hermes Lite / ANAN, Flex, Icom LAN, Pluto,
    /// SunSDR / Thetis over TCI, rtl_tcp.
    Network,
    /// Another sdroxide, running as a server.
    Remote,
    /// Nothing yet: listen to a public receiver on the internet.
    NoRadio,
}

impl RadioKind {
    pub(in crate::app) const ALL: [RadioKind; 5] = [
        RadioKind::UsbSdr,
        RadioKind::CatRig,
        RadioKind::Network,
        RadioKind::Remote,
        RadioKind::NoRadio,
    ];

    fn label(self) -> String {
        match self {
            RadioKind::UsbSdr => tr!("welcome-kind-usb"),
            RadioKind::CatRig => tr!("welcome-kind-cat"),
            RadioKind::Network => tr!("welcome-kind-net"),
            RadioKind::Remote => tr!("welcome-kind-remote"),
            RadioKind::NoRadio => tr!("welcome-kind-none"),
        }
    }

    fn examples(self) -> String {
        match self {
            RadioKind::UsbSdr => tr!("welcome-kind-usb", "hint"),
            RadioKind::CatRig => tr!("welcome-kind-cat", "hint"),
            RadioKind::Network => tr!("welcome-kind-net", "hint"),
            RadioKind::Remote => tr!("welcome-kind-remote", "hint"),
            RadioKind::NoRadio => tr!("welcome-kind-none", "hint"),
        }
    }

    /// The things that go wrong with this kind of radio, most common first:
    /// each a short lead and its explanation.
    pub(in crate::app) fn tips(self) -> Vec<(String, String)> {
        match self {
            RadioKind::UsbSdr => vec![
                pair!("welcome-tip-usb-driver"),
                pair!("welcome-tip-usb-busy"),
                pair!("welcome-tip-usb-port"),
                pair!("welcome-tip-usb-gain"),
                pair!("welcome-tip-usb-sdrplay"),
            ],
            RadioKind::CatRig => vec![
                pair!("welcome-tip-cat-port"),
                pair!("welcome-tip-cat-family"),
                pair!("welcome-tip-cat-audio"),
                pair!("welcome-tip-cat-data"),
                pair!("welcome-tip-cat-pipewire"),
            ],
            RadioKind::Network => vec![
                pair!("welcome-tip-net-subnet"),
                pair!("welcome-tip-net-firewall"),
                pair!("welcome-tip-net-icom"),
                pair!("welcome-tip-net-one"),
            ],
            RadioKind::Remote => vec![
                pair!("welcome-tip-remote-server"),
                pair!("welcome-tip-remote-port"),
                pair!("welcome-tip-remote-browser"),
            ],
            RadioKind::NoRadio => {
                vec![pair!("welcome-tip-none-public"), pair!("welcome-tip-none-later")]
            }
        }
    }

    /// Where this kind of radio is set up.
    fn setup(self) -> Go {
        match self {
            RadioKind::NoRadio => Go::PublicSdrs,
            #[cfg(not(target_arch = "wasm32"))]
            RadioKind::Remote => Go::Settings(SettingsTab::Remote),
            _ => Go::Settings(SettingsTab::Radio),
        }
    }
}

/// Where one of the walkthrough's buttons goes. Everything here is somewhere
/// the program already has a control for; the walkthrough only opens it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::app) enum Go {
    Settings(SettingsTab),
    Window(Win),
    Act(Action),
    Fix(Fix),
    PublicSdrs,
    Mail,
    Check,
    Palette,
}

/// The window's width, in points (less on a smaller screen).
const WIDTH: f32 = 470.0;

/// The tour: only what a newcomer is unlikely to find by looking at the main
/// screen (the modes, the logbook and the spots are on it already), what each
/// is for, and where its button goes.
pub(in crate::app) fn tour() -> Vec<(String, String, Go)> {
    let m = |(a, b): (String, String), go| (a, b, go);
    vec![
        m(pair!("welcome-tour-palette"), Go::Palette),
        m(pair!("welcome-tour-servers"), Go::Settings(SettingsTab::Servers)),
        m(pair!("welcome-tour-public"), Go::PublicSdrs),
        m(pair!("welcome-tour-profiles"), Go::Settings(SettingsTab::Profiles)),
        m(pair!("welcome-tour-controls"), Go::Settings(SettingsTab::Controls)),
        m(pair!("welcome-tour-bands"), Go::Window(Win::Bands)),
        m(pair!("welcome-tour-winlink"), Go::Mail),
        m(pair!("welcome-tour-help"), Go::Act(Action::ToggleHelp)),
    ]
}

/// The walkthrough's own state. Nothing here is saved but `welcomed`, which
/// lives in the UI settings: a walkthrough reopened is reopened at the start.
#[derive(Default)]
pub(in crate::app) struct Welcome {
    /// Opened by hand (Settings → General, or the palette), rather than by a
    /// first start. Shown regardless of the callsign.
    pub(in crate::app) asked: bool,
    /// Put aside for this session, by its close button.
    pub(in crate::app) later: bool,
    /// Has been on screen this session, so it stays until it is answered
    /// even once a callsign has been typed into it.
    pub(in crate::app) shown: bool,
    pub(in crate::app) step: Option<Step>,
    pub(in crate::app) kind: Option<RadioKind>,
    /// Dragged away by hand: from then on it stays where it was put rather
    /// than being kept in the middle of the application.
    pub(in crate::app) moved: bool,
}

impl Welcome {
    /// Open from the start, by hand.
    pub(in crate::app) fn open(&mut self) {
        self.asked = true;
        self.later = false;
        self.step = Some(Step::Welcome);
    }
}

/// Whether the walkthrough is on screen this frame.
///
/// A first start is a station with no callsign, once the engine's saved
/// settings have arrived — for the first frames of every start there is none
/// *here*, and an operator of ten years' standing must not be welcomed each
/// morning.
pub(in crate::app) fn should_show(
    w: &Welcome,
    welcomed: bool,
    seeded: bool,
    call_empty: bool,
) -> bool {
    if w.asked {
        return true;
    }
    if welcomed || w.later || !seeded {
        return false;
    }
    w.shown || call_empty
}

/// How a page has come out so far, for the marks on the step strip: done,
/// something to look at, or nothing to judge.
pub(in crate::app) fn step_level(step: Step, checks: &[checkup::Check]) -> Option<Level> {
    let ids: &[CheckId] = match step {
        Step::Station => &[CheckId::Callsign, CheckId::Grid],
        Step::Radio => &[CheckId::Link, CheckId::Receiver, CheckId::Level, CheckId::Audio],
        Step::Transmit => &[CheckId::Clock, CheckId::Transmit],
        Step::Online => &[CheckId::Lookup, CheckId::Spots],
        Step::Check => return checks.iter().map(|c| c.level).max(),
        Step::Welcome | Step::Tour => return None,
    };
    checks.iter().filter(|c| ids.contains(&c.id)).map(|c| c.level).max()
}

fn level_mark(level: Level) -> (&'static str, egui::Color32) {
    match level {
        Level::Ok => ("✓", crate::theme::GREEN()),
        Level::Info => ("·", crate::theme::gray(150)),
        Level::Warn => ("!", crate::theme::YELLOW()),
        Level::Fail => ("✕", crate::theme::PINK()),
    }
}

fn heading(ui: &mut egui::Ui, text: String) {
    ui.add_space(10.0);
    ui.label(RichText::new(text).size(12.5).strong().color(crate::theme::CYAN()));
    ui.add_space(3.0);
}

fn para(ui: &mut egui::Ui, text: String) {
    ui.label(RichText::new(text).size(12.0).color(crate::theme::TEXT()));
}

fn note(ui: &mut egui::Ui, text: String) {
    ui.label(RichText::new(text).size(11.0).color(crate::theme::gray(140)));
}

/// A bulleted pitfall: a bold lead-in and its explanation.
fn tip(ui: &mut egui::Ui, (lead, detail): (String, String)) {
    ui.horizontal_top(|ui| {
        ui.label(RichText::new("▸").size(11.5).color(crate::theme::YELLOW()));
        ui.vertical(|ui| {
            ui.label(RichText::new(lead).size(11.5).strong().color(crate::theme::TEXT_STRONG()));
            ui.label(RichText::new(detail).size(11.0).color(crate::theme::gray(160)));
        });
    });
    ui.add_space(3.0);
}

/// The station check's lines named by `ids`, drawn compactly, each with its
/// fix button. Returns the button pressed.
fn live_lines(ui: &mut egui::Ui, checks: &[checkup::Check], ids: &[CheckId]) -> Option<Go> {
    let mut go = None;
    for c in checks.iter().filter(|c| ids.contains(&c.id)) {
        if let Some(g) = check_row(ui, c) {
            go = Some(g);
        }
    }
    go
}

fn check_row(ui: &mut egui::Ui, c: &checkup::Check) -> Option<Go> {
    let mut go = None;
    let (mark, col) = level_mark(c.level);
    egui::Frame::new()
        .fill(crate::theme::ROW_BG())
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(mark).size(13.0).strong().color(col));
                ui.label(
                    RichText::new(&c.title).size(11.5).strong().color(crate::theme::TEXT_STRONG()),
                );
                if let Some(f) = c.fix {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = match f {
                            Fix::Settings(_) => tr!("check-fix-settings"),
                            Fix::Unmute => tr!("check-fix-unmute"),
                        };
                        if crate::chrome::chip(ui, false, RichText::new(label).size(10.5)).clicked()
                        {
                            go = Some(Go::Fix(f));
                        }
                    });
                }
            });
            ui.label(RichText::new(&c.detail).size(10.5).color(crate::theme::gray(150)));
        });
    ui.add_space(2.0);
    go
}

/// A button that opens something, with its explanation beside it.
fn open_row(ui: &mut egui::Ui, label: String, hint: String, go: Go, out: &mut Option<Go>) {
    ui.horizontal_wrapped(|ui| {
        if crate::chrome::chip(ui, false, RichText::new(label).size(11.0)).clicked() {
            *out = Some(go);
        }
        note(ui, hint);
    });
    ui.add_space(2.0);
}

impl SdroxideApp {
    /// Do what one of the walkthrough's buttons says. The walkthrough stays
    /// open: what it opens is meant to be used beside it.
    fn welcome_go(&mut self, go: Go, now: f64, cmds: &mut Vec<Command>) {
        match go {
            Go::Settings(t) => {
                self.settings_tab = t;
                self.show_settings = true;
            }
            Go::Window(w) => {
                let flag = match w {
                    Win::Scanner => &mut self.show_scanner,
                    Win::Bands => &mut self.show_bands,
                    Win::Awards => &mut self.show_awards,
                    Win::Satellite => &mut self.show_sat,
                    Win::Windows => &mut self.show_windows,
                };
                *flag = true;
            }
            Go::Act(a) => self.run_action(a, now, cmds),
            Go::Fix(f) => self.apply_fix(f, cmds),
            Go::PublicSdrs => self.show_public_sdrs = true,
            Go::Mail => self.mail.open = true,
            Go::Check => self.show_check = true,
            Go::Palette => {
                if !self.palette.open {
                    self.palette.toggle();
                }
            }
        }
    }

    /// Whether a tour button can do anything on this radio. A mode the radio
    /// cannot be put in is greyed, as the MODE menu leaves it out.
    fn tour_available(&self, go: Go) -> bool {
        match go {
            Go::Act(Action::ModeSelect(m)) => self.state.mode_unavailable(m).is_none(),
            Go::Window(Win::Windows) => !cfg!(target_arch = "wasm32"),
            _ => true,
        }
    }

    /// The walkthrough window.
    pub(in crate::app) fn welcome_window(
        &mut self,
        ctx: &egui::Context,
        now: f64,
        cmds: &mut Vec<Command>,
    ) {
        let call_empty = self.digi_cfg_edit.my_call.trim().is_empty();
        if !should_show(&self.welcome, self.ui_settings.welcomed, self.digi_cfg_seeded, call_empty)
        {
            return;
        }
        self.welcome.shown = true;
        let step = *self.welcome.step.get_or_insert(Step::Welcome);
        let checks = checkup::run_checks(&self.check_input(now));

        let mut open = true;
        let mut go: Option<Go> = None;
        let mut goto: Option<Step> = None;
        let mut finish = false;
        let mut save_station = false;
        let mut language_changed = false;

        // One fixed width: the pages are wrapped text, so the window is as
        // wide as this and no wider, whatever page is showing.
        let width = crate::layout::window_w(ctx, WIDTH);
        let mut win = egui::Window::new(tr!("welcome-title"))
            .id(crate::layout::salted_id(ctx, "welcome"))
            .frame(crate::chrome::window_frame())
            .collapsible(false)
            .resizable(false)
            .min_width(width)
            .max_width(width)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center());
        // Kept in the middle of the application until it is dragged away.
        if !self.welcome.moved {
            win = win.current_pos(ctx.content_rect().center());
        }
        let resp = win.open(&mut open).show(ctx, |ui| {
            crate::chrome::window_body_bg(ui);
            // `width` is the window's outer width (egui counts the frame in
            // it); the content fills what is left inside, so it is exactly as
            // wide as the title bar above it.
            ui.set_width(ui.available_width());

            // The step strip: every page by name, each a button, with a
            // mark saying how the things it covers stand right now.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (n, s) in Step::ALL.iter().enumerate() {
                    let mut text = format!("{} {}", n + 1, s.title());
                    if let Some(l) = step_level(*s, &checks) {
                        text.push(' ');
                        text.push_str(level_mark(l).0);
                    }
                    if crate::chrome::chip(ui, *s == step, RichText::new(text).size(10.5)).clicked()
                    {
                        goto = Some(*s);
                    }
                }
            });
            ui.add_space(6.0);
            ui.separator();

            let body_h = (ctx.content_rect().height() - 200.0).clamp(200.0, 430.0);
            egui::ScrollArea::vertical()
                .id_salt(("welcome-body", step.index()))
                .max_height(body_h)
                .show_themed(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match step {
                        Step::Welcome => {
                            language_changed = self.welcome_page_intro(ui);
                        }
                        Step::Station => {
                            save_station = self.welcome_page_station(ui, &checks, &mut go);
                        }
                        Step::Radio => self.welcome_page_radio(ui, &checks, &mut go),
                        Step::Transmit => welcome_page_transmit(ui, &checks, &mut go),
                        Step::Online => welcome_page_online(ui, &checks, &mut go),
                        Step::Tour => self.welcome_page_tour(ui, &mut go),
                        Step::Check => welcome_page_check(ui, &checks, &mut go),
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if step != Step::Welcome
                    && crate::chrome::chip(ui, false, tr!("welcome-back")).clicked()
                {
                    goto = Some(step.prev());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if step == Step::Check {
                        if crate::chrome::chip_accent(
                            ui,
                            false,
                            RichText::new(tr!("welcome-finish")).strong(),
                            crate::theme::GREEN(),
                            crate::theme::INK_ON_CYAN(),
                        )
                        .on_hover_text(tr!("welcome-finish", "hint"))
                        .clicked()
                        {
                            finish = true;
                        }
                    } else if crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new(tr!("welcome-next")).strong(),
                        crate::theme::CYAN(),
                        crate::theme::INK_ON_CYAN(),
                    )
                    .clicked()
                    {
                        goto = Some(step.next());
                    }
                    note(ui, tr!("welcome-reopen-note"));
                });
            });
        });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
            if r.response.dragged() {
                self.welcome.moved = true;
            }
        }

        if save_station {
            self.digi_cfg_edit.my_call = self.digi_cfg_edit.my_call.trim().to_uppercase();
            self.digi_cfg_edit.my_grid = self.digi_cfg_edit.my_grid.trim().to_string();
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
        if language_changed {
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
        if let Some(s) = goto {
            self.welcome.step = Some(s);
        }
        if let Some(g) = go {
            self.welcome_go(g, now, cmds);
        }
        if finish || !open {
            // FINISH or the cross: either way the operator has seen it, so a
            // listener who never sets a callsign is not welcomed on every
            // start. Settings → UI opens it again.
            self.ui_settings.welcomed = true;
            crate::app::persist::persist_ui_settings(&self.ui_settings);
            self.welcome = Welcome::default();
        }
    }

    /// Page 1: what this is, and its language. Returns whether the language
    /// was changed.
    fn welcome_page_intro(&mut self, ui: &mut egui::Ui) -> bool {
        ui.label(
            RichText::new(tr!("welcome-hello"))
                .size(16.0)
                .strong()
                .color(crate::theme::TEXT_STRONG()),
        );
        ui.add_space(4.0);
        para(ui, tr!("welcome-intro"));
        ui.add_space(4.0);
        para(ui, tr!("welcome-intro-how"));

        heading(ui, tr!("settings-ui-language"));
        let before = self.ui_settings.language;
        ui.horizontal(|ui| {
            crate::app::settings::ui_tab::language_combo(ui, &mut self.ui_settings.language);
        });
        note(ui, tr!("welcome-language-note"));

        heading(ui, tr!("welcome-intro-safe"));
        note(ui, tr!("welcome-intro-safe", "hint"));
        before != self.ui_settings.language
    }

    /// Page 2: callsign and locator. Returns whether SAVE was pressed.
    fn welcome_page_station(
        &mut self,
        ui: &mut egui::Ui,
        checks: &[checkup::Check],
        go: &mut Option<Go>,
    ) -> bool {
        let mut save = false;
        para(ui, tr!("welcome-station-why"));
        ui.add_space(8.0);
        let cfg = &mut self.digi_cfg_edit;
        egui::Grid::new("welcome-station").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-callsign"));
            if crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.my_call)
                    .hint_text("EA4ABC")
                    .desired_width(120.0),
            )
            .changed()
            {
                cfg.my_call = cfg.my_call.to_uppercase();
            }
            ui.end_row();
            ui.label(tr!("settings-grid-square"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.my_grid)
                    .hint_text("IN80dk")
                    .desired_width(120.0),
            );
            ui.end_row();
        });
        ui.add_space(6.0);
        let ready = checkup::callsign_shaped(&cfg.my_call)
            && (cfg.my_grid.trim().is_empty() || checkup::grid_shaped(&cfg.my_grid));
        ui.horizontal(|ui| {
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
            note(ui, tr!("welcome-station-save-note"));
        });
        heading(ui, tr!("welcome-station-grid-how"));
        note(ui, tr!("welcome-station-grid-how", "hint"));
        heading(ui, tr!("welcome-now"));
        if let Some(g) = live_lines(ui, checks, &[CheckId::Callsign, CheckId::Grid]) {
            *go = Some(g);
        }
        save
    }

    /// Page 3: the radio — its kind, that kind's pitfalls, and whether it is
    /// working yet.
    fn welcome_page_radio(
        &mut self,
        ui: &mut egui::Ui,
        checks: &[checkup::Check],
        go: &mut Option<Go>,
    ) {
        para(ui, tr!("welcome-radio-intro"));
        ui.add_space(6.0);
        for k in RadioKind::ALL {
            #[cfg(target_arch = "wasm32")]
            if k == RadioKind::Remote {
                // A browser client is already attached to a server.
                continue;
            }
            let sel = self.welcome.kind == Some(k);
            let r = egui::Frame::new()
                .fill(if sel { crate::theme::ROW_BG() } else { egui::Color32::TRANSPARENT })
                .stroke(egui::Stroke::new(
                    1.0,
                    if sel { crate::theme::CYAN() } else { crate::theme::gray(70) },
                ))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(k.label()).size(12.0).strong().color(if sel {
                        crate::theme::CYAN()
                    } else {
                        crate::theme::TEXT_STRONG()
                    }));
                    ui.label(RichText::new(k.examples()).size(10.5).color(crate::theme::gray(140)));
                })
                .response
                .interact(egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if r.clicked() {
                self.welcome.kind = Some(k);
            }
            ui.add_space(3.0);
        }

        let Some(kind) = self.welcome.kind else {
            ui.add_space(4.0);
            note(ui, tr!("welcome-radio-pick"));
            return;
        };
        heading(ui, tr!("welcome-radio-pitfalls"));
        for t in kind.tips() {
            tip(ui, t);
        }
        ui.add_space(4.0);
        let (label, hint) = match kind {
            RadioKind::NoRadio => (tr!("welcome-open-public"), tr!("welcome-open-public", "hint")),
            RadioKind::Remote => (tr!("welcome-open-remote"), tr!("welcome-open-remote", "hint")),
            _ => (tr!("welcome-open-radio"), tr!("welcome-open-radio", "hint")),
        };
        open_row(ui, label, hint, kind.setup(), go);
        note(ui, tr!("welcome-radio-manual"));

        heading(ui, tr!("welcome-now"));
        if let Some(g) = live_lines(
            ui,
            checks,
            &[CheckId::Link, CheckId::Receiver, CheckId::Level, CheckId::Audio, CheckId::Muted],
        ) {
            *go = Some(g);
        }
    }

    /// Page 6: the tour, one feature per line, its OPEN button first.
    fn welcome_page_tour(&mut self, ui: &mut egui::Ui, go: &mut Option<Go>) {
        para(ui, tr!("welcome-tour-intro"));
        ui.add_space(6.0);
        for (name, what, target) in tour() {
            let enabled = self.tour_available(target);
            ui.horizontal_top(|ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let open = RichText::new(tr!("welcome-tour-open")).size(10.5);
                    if crate::chrome::chip(ui, false, open).clicked() {
                        *go = Some(target);
                    }
                });
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(name).size(12.0).strong().color(crate::theme::TEXT_STRONG()),
                    );
                    ui.label(RichText::new(what).size(11.0).color(crate::theme::gray(160)));
                });
            });
            ui.add_space(5.0);
        }
    }
}

/// Page 4: before transmitting.
fn welcome_page_transmit(ui: &mut egui::Ui, checks: &[checkup::Check], go: &mut Option<Go>) {
    para(ui, tr!("welcome-tx-intro"));
    ui.add_space(4.0);
    for t in [
        pair!("welcome-tip-tx-clock"),
        pair!("welcome-tip-tx-level"),
        pair!("welcome-tip-tx-drive"),
        pair!("welcome-tip-tx-lockout"),
        pair!("welcome-tip-tx-trswitch"),
        pair!("welcome-tip-tx-cable"),
    ] {
        tip(ui, t);
    }
    ui.add_space(4.0);
    open_row(
        ui,
        tr!("welcome-open-region"),
        tr!("welcome-open-region", "hint"),
        Go::Settings(SettingsTab::General),
        go,
    );
    open_row(
        ui,
        tr!("welcome-open-trswitch"),
        tr!("welcome-open-trswitch", "hint"),
        Go::Settings(SettingsTab::TrSwitch),
        go,
    );
    heading(ui, tr!("welcome-now"));
    if let Some(g) = live_lines(ui, checks, &[CheckId::Transmit, CheckId::Clock]) {
        *go = Some(g);
    }
}

/// Page 5: the online services.
fn welcome_page_online(ui: &mut egui::Ui, checks: &[checkup::Check], go: &mut Option<Go>) {
    para(ui, tr!("welcome-online-intro"));
    ui.add_space(4.0);
    for ((label, hint), tab) in [
        (pair!("welcome-online-lookup"), SettingsTab::Uploads),
        (pair!("welcome-online-spots"), SettingsTab::Spots),
        (pair!("welcome-online-uploads"), SettingsTab::Uploads),
        (pair!("welcome-online-alerts"), SettingsTab::Alerts),
    ] {
        open_row(ui, label, hint, Go::Settings(tab), go);
    }
    ui.add_space(4.0);
    note(ui, tr!("welcome-online-privacy"));
    heading(ui, tr!("welcome-now"));
    if let Some(g) = live_lines(ui, checks, &[CheckId::Lookup, CheckId::Spots]) {
        *go = Some(g);
    }
}

/// Page 7: the verdict, and what is left.
fn welcome_page_check(ui: &mut egui::Ui, checks: &[checkup::Check], go: &mut Option<Go>) {
    let worst = checks.iter().map(|c| c.level).max().unwrap_or(Level::Ok);
    let (verdict, col) = match worst {
        Level::Ok | Level::Info => (tr!("check-verdict-ok"), crate::theme::GREEN()),
        Level::Warn => (tr!("check-verdict-warn"), crate::theme::YELLOW()),
        Level::Fail => (tr!("check-verdict-fail"), crate::theme::PINK()),
    };
    ui.label(RichText::new(verdict).size(13.5).strong().color(col));
    ui.add_space(6.0);
    let left: Vec<&checkup::Check> = checks.iter().filter(|c| c.level >= Level::Warn).collect();
    if left.is_empty() {
        para(ui, tr!("welcome-check-all-good"));
    } else {
        para(ui, tr!("welcome-check-left"));
        ui.add_space(4.0);
        for c in left {
            if let Some(g) = check_row(ui, c) {
                *go = Some(g);
            }
        }
    }
    ui.add_space(6.0);
    open_row(ui, tr!("check-open"), tr!("welcome-check-hint"), Go::Check, go);

    heading(ui, tr!("welcome-next-steps"));
    for t in [
        pair!("welcome-next-listen"),
        pair!("welcome-next-ft8"),
        pair!("welcome-next-palette"),
        pair!("welcome-next-manual"),
    ] {
        tip(ui, t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_run_in_order_and_stop_at_the_ends() {
        assert_eq!(Step::Welcome.prev(), Step::Welcome);
        assert_eq!(Step::Check.next(), Step::Check);
        let mut s = Step::Welcome;
        let mut seen = vec![s];
        while s != Step::Check {
            s = s.next();
            seen.push(s);
        }
        assert_eq!(seen, Step::ALL);
        for s in Step::ALL {
            assert_eq!(s.next().prev() == s, s != Step::Check, "{s:?}");
        }
    }

    #[test]
    fn a_first_start_is_welcomed_and_an_old_station_is_not() {
        let w = Welcome::default();
        // Not before the saved settings have arrived, whatever the callsign.
        assert!(!should_show(&w, false, false, true));
        // A first start: seeded, no callsign.
        assert!(should_show(&w, false, true, true));
        // A station with a callsign is never asked of its own accord...
        assert!(!should_show(&w, false, true, false));
        // ...nor once it has finished the walkthrough.
        assert!(!should_show(&w, true, true, true));
        // Typing the callsign into the open walkthrough does not close it.
        let shown = Welcome { shown: true, ..Default::default() };
        assert!(should_show(&shown, false, true, false));
        // Closing it puts it aside for the session.
        let later = Welcome { shown: true, later: true, ..Default::default() };
        assert!(!should_show(&later, false, true, true));
    }

    #[test]
    fn opened_by_hand_it_shows_whatever_the_station() {
        let mut w = Welcome { later: true, ..Default::default() };
        w.open();
        assert!(should_show(&w, true, true, false));
        assert_eq!(w.step, Some(Step::Welcome));
        assert!(!w.later);
    }

    #[test]
    fn every_radio_kind_has_pitfalls_and_somewhere_to_set_it_up() {
        for k in RadioKind::ALL {
            assert!(!k.tips().is_empty(), "{k:?}");
            let _ = k.setup();
        }
        assert_eq!(RadioKind::NoRadio.setup(), Go::PublicSdrs);
        assert_eq!(RadioKind::UsbSdr.setup(), Go::Settings(SettingsTab::Radio));
    }

    #[test]
    fn the_step_marks_follow_the_checks_they_cover() {
        let fresh = checkup::run_checks(&checkup::CheckInput::default());
        assert_eq!(step_level(Step::Station, &fresh), Some(Level::Fail));
        assert_eq!(step_level(Step::Online, &fresh), Some(Level::Info));
        assert_eq!(step_level(Step::Welcome, &fresh), None);
        assert_eq!(step_level(Step::Tour, &fresh), None);
        assert_eq!(step_level(Step::Check, &fresh), Some(Level::Fail));
    }

    #[test]
    fn nothing_on_the_tour_transmits() {
        for (name, _, go) in tour() {
            if let Go::Act(a) = go {
                assert_ne!(a.group(), "Transmit", "{name}");
            }
        }
    }
}
