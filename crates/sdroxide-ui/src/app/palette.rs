//! The command palette: Ctrl+K (⌘K on a Mac), type a few letters, Enter.
//!
//! The program has more in it than a row of buttons can show: forty modes,
//! twenty bands, fifteen settings pages, a dozen tool windows, and every
//! switch the Controls tab can bind a key to. The palette is one box that
//! reaches all of them by name — `ft8`, `40m`, `uploads`, `scan`, `notch` —
//! and a typed frequency (`14.074`, `7100k`) tunes there.
//!
//! It adds no behaviour of its own. A row is an [`Action`] the key bindings
//! already run, a window flag a chip already flips, a settings page a tab
//! already opens, or a memory the memory window already recalls; choosing it
//! does exactly what that control does, through the same code.
//!
//! What it deliberately does not offer is anything that transmits. A search
//! box whose top hit moves with every keystroke is no place for PTT, and
//! Enter pressed a letter early must never be a carrier.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{Action, ActionKind, Command};

use crate::app::SdroxideApp;
use crate::app::settings::SettingsTab;
use crate::theme::ThemedScroll;

/// A tool window that has no [`Action`] of its own to toggle it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Win {
    Scanner,
    Bands,
    Awards,
    Satellite,
    Windows,
}

/// What choosing a row does.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::app) enum Go {
    Act(Action),
    Window(Win),
    Settings(SettingsTab),
    /// Recall the memory channel with this id.
    Memory(u32),
    /// Tune the active VFO to this many hertz.
    Tune(f64),
}

/// One row: what it is filed under, what it is called, what it does.
#[derive(Clone)]
pub(in crate::app) struct Entry {
    group: String,
    label: String,
    go: Go,
}

/// The palette's own state. Nothing here is saved: it opens empty each time,
/// because the last thing looked for is rarely the next.
#[derive(Default)]
pub(in crate::app) struct Palette {
    pub(in crate::app) open: bool,
    query: String,
    selected: usize,
    /// Set by whatever opened it, so the box takes the keyboard on its first
    /// frame and not on every one — which would make it impossible to click
    /// anywhere else.
    focus: bool,
}

impl Palette {
    pub(in crate::app) fn toggle(&mut self) {
        self.open = !self.open;
        self.query.clear();
        self.selected = 0;
        self.focus = self.open;
    }
}

/// Whether an action may be offered. Continuous ones are knobs and have no
/// "do it" to press; the hand keys are read held by the CW panel and do
/// nothing when dispatched; and anything that puts a signal on the air is left
/// to a control that cannot be reached by a mistyped letter.
fn offered(a: Action) -> bool {
    use Action::*;
    a.kind() == ActionKind::Momentary
        && !matches!(a, Ptt | TuneCarrier | CwStraight | VoicePlay(_) | ToneBurst | MemoryRecall(_))
}

/// A typed frequency, in hertz: `14.074` and `14,074` are megahertz, `7100k`
/// kilohertz, `145500000` hertz, with an optional `m`/`mhz`/`k`/`khz`/`hz`
/// suffix. A bare number is megahertz below 30 000 and hertz above — nobody
/// types 30 GHz in megahertz, and nobody types 29 kHz in hertz.
///
/// `None` for anything else, including a band name: `40m` is forty metres, a
/// row of its own, and must not be read as 40 MHz.
pub(in crate::app) fn parse_freq(text: &str) -> Option<f64> {
    let t = text.trim().to_ascii_lowercase().replace(',', ".");
    let (num, scale) = if let Some(n) = t.strip_suffix("mhz") {
        (n, Some(1e6))
    } else if let Some(n) = t.strip_suffix("khz") {
        (n, Some(1e3))
    } else if let Some(n) = t.strip_suffix("hz") {
        (n, Some(1.0))
    } else if let Some(n) = t.strip_suffix('k') {
        (n, Some(1e3))
    } else {
        (t.as_str(), None)
    };
    let num = num.trim();
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let v: f64 = num.parse().ok()?;
    let hz = match scale {
        Some(s) => v * s,
        None if v < 30_000.0 => v * 1e6,
        None => v,
    };
    (hz > 0.0 && hz < 1e12).then_some(hz)
}

/// The rows matching `query`, best first, as indices into `entries`.
///
/// Literal matches — every word of the query present as typed — come before
/// fuzzy ones however the fuzzy ones score: `usb` is a subsequence of a good
/// many labels by accident, and the mode called USB must not sit under them.
/// Equal scores keep the order the entries were listed in, so an empty box
/// shows them as filed.
pub(in crate::app) fn rank(entries: &[Entry], query: &str) -> Vec<usize> {
    let mut hits: Vec<(bool, i32, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let hay = format!("{} {}", e.label, e.group);
            let score = crate::fuzzy::score_terms(&hay, query)?;
            Some((crate::fuzzy::contains_terms(&hay, query), score, i))
        })
        .collect();
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    hits.into_iter().map(|h| h.2).collect()
}

impl SdroxideApp {
    /// Everything the palette can reach right now. Built when it is open and
    /// not otherwise: a few hundred short strings, against a list that changes
    /// with the memories and the language.
    fn palette_entries(&self) -> Vec<Entry> {
        let mut v = Vec::with_capacity(256);
        let windows = tr!("palette-group-window");
        for (w, label) in [
            (Win::Scanner, tr!("scanner-scanner")),
            (Win::Bands, tr!("palette-win-bands")),
            (Win::Awards, tr!("palette-win-awards")),
            (Win::Satellite, tr!("sat-satellite")),
            (Win::Windows, tr!("palette-win-windows")),
        ] {
            v.push(Entry { group: windows.clone(), label, go: Go::Window(w) });
        }
        let settings = tr!("palette-group-settings");
        #[allow(unused_mut)]
        let mut tabs = vec![
            (SettingsTab::General, tr!("settings-general")),
            (SettingsTab::Radio, tr!("settings-radio")),
            (SettingsTab::Ui, tr!("settings-ui")),
            (SettingsTab::Alerts, tr!("settings-alerts")),
            (SettingsTab::Controls, tr!("settings-controls")),
            (SettingsTab::Spots, tr!("settings-spots")),
            (SettingsTab::FreeDv, tr!("settings-freedv")),
            (SettingsTab::Uploads, tr!("settings-uploads")),
            (SettingsTab::Winlink, tr!("settings-winlink")),
            (SettingsTab::Servers, tr!("settings-servers")),
            (SettingsTab::TrSwitch, tr!("settings-t-r-switch")),
            (SettingsTab::Amp, tr!("settings-amplifier")),
            (SettingsTab::Tle, tr!("settings-tle")),
            (SettingsTab::Profiles, tr!("settings-profiles")),
        ];
        #[cfg(not(target_arch = "wasm32"))]
        tabs.push((SettingsTab::Remote, tr!("settings-remote")));
        for (t, label) in tabs {
            v.push(Entry { group: settings.clone(), label, go: Go::Settings(t) });
        }
        for a in Action::all().into_iter().filter(|a| offered(*a)) {
            // A mode this radio cannot be put in is not offered, as the MODE
            // menu does not offer it.
            if let Action::ModeSelect(m) = a
                && self.state.mode_unavailable(m).is_some()
            {
                continue;
            }
            v.push(Entry { group: a.group().to_string(), label: a.label(), go: Go::Act(a) });
        }
        let memory = tr!("palette-group-memory");
        for m in &self.memories {
            v.push(Entry {
                group: memory.clone(),
                label: format!("{} — {:.4} MHz {}", m.name, m.freq_hz / 1e6, m.mode.label()),
                go: Go::Memory(m.id),
            });
        }
        v
    }

    /// Do what a row says.
    fn palette_go(&mut self, go: Go, now: f64, cmds: &mut Vec<Command>) {
        match go {
            Go::Act(a) => self.run_action(a, now, cmds),
            Go::Window(w) => {
                let flag = match w {
                    Win::Scanner => &mut self.show_scanner,
                    Win::Bands => &mut self.show_bands,
                    Win::Awards => &mut self.show_awards,
                    Win::Satellite => &mut self.show_sat,
                    Win::Windows => &mut self.show_windows,
                };
                *flag = !*flag;
            }
            // Opened, never toggled: asking for a settings page while Settings
            // is already showing another one means "go there", not "close".
            Go::Settings(t) => {
                self.settings_tab = t;
                self.show_settings = true;
            }
            Go::Memory(id) => cmds.push(Command::RecallMemory(id)),
            Go::Tune(hz) => cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz }),
        }
    }

    /// Open or close the palette on Ctrl+K / ⌘K, taking the key so nothing
    /// bound to it runs as well. Called before the bindings are polled.
    ///
    /// Read whether or not a text field has the keyboard, unlike the bare-key
    /// shortcuts: a chord with the command modifier types nothing, and the
    /// palette's own box is a text field it has to be closable from.
    pub(in crate::app) fn palette_key(&mut self, ctx: &egui::Context) {
        let hit = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K));
        if hit {
            self.palette.toggle();
        }
    }

    /// Draw the palette, if it is open, and act on a row chosen in it.
    pub(in crate::app) fn palette_window(
        &mut self,
        ctx: &egui::Context,
        now: f64,
        cmds: &mut Vec<Command>,
    ) {
        if !self.palette.open {
            return;
        }
        let mut entries = self.palette_entries();
        // A typed frequency is a row of its own, at the top: it is the one
        // thing here that is not in the list until it has been typed.
        if let Some(hz) = parse_freq(&self.palette.query) {
            entries.insert(
                0,
                Entry {
                    group: tr!("palette-group-tune"),
                    label: tr!("palette-tune-to", mhz = format!("{:.6}", hz / 1e6)),
                    go: Go::Tune(hz),
                },
            );
        }
        let ranked = if parse_freq(&self.palette.query).is_some() {
            // Nothing else is ranked against a number: `14.074` is a
            // subsequence of no label worth showing under it.
            vec![0]
        } else {
            rank(&entries, &self.palette.query)
        };
        self.palette.selected = self.palette.selected.min(ranked.len().saturating_sub(1));

        // Keys first, taken from the events so the text box does not also move
        // its caret with the arrows or the bindings tune the radio with them.
        let (mut up, mut down, mut enter, mut esc) = (false, false, false, false);
        ctx.input_mut(|i| {
            up = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
            down = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
            enter = i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
            esc = i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        });
        if down && !ranked.is_empty() {
            self.palette.selected = (self.palette.selected + 1) % ranked.len();
        }
        if up && !ranked.is_empty() {
            self.palette.selected = (self.palette.selected + ranked.len() - 1) % ranked.len();
        }
        let mut chosen: Option<Go> = None;
        if enter {
            chosen = ranked.get(self.palette.selected).map(|&i| entries[i].go);
        }

        let moved = up || down;
        let resp = egui::Window::new("palette")
            .id(crate::layout::salted_id(ctx, "command-palette"))
            .frame(crate::chrome::window_frame())
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 70.0])
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                let w = crate::layout::window_w(ctx, 520.0);
                ui.set_width(w);
                let before = self.palette.query.clone();
                let edit = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.palette.query)
                        .hint_text(tr!("palette-hint"))
                        .desired_width(f32::INFINITY),
                );
                if std::mem::take(&mut self.palette.focus) {
                    edit.request_focus();
                }
                if self.palette.query != before {
                    // A new question: the best answer to it is the first row.
                    self.palette.selected = 0;
                }
                ui.add_space(4.0);
                if ranked.is_empty() {
                    ui.label(RichText::new(tr!("palette-nothing")).color(crate::theme::gray(120)));
                    return;
                }
                egui::ScrollArea::vertical().max_height(340.0).show_themed(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // Capped: past the first screens of it a list this long is
                    // not being read, it is being typed at.
                    for (row, &i) in ranked.iter().take(60).enumerate() {
                        let e = &entries[i];
                        let sel = row == self.palette.selected;
                        let (rect, r) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 24.0),
                            egui::Sense::click(),
                        );
                        if sel || r.hovered() {
                            ui.painter().rect_filled(rect, 0.0, crate::theme::ROW_BG());
                        }
                        if sel {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    rect.left_top(),
                                    egui::pos2(rect.left() + 2.0, rect.bottom()),
                                ),
                                0.0,
                                crate::theme::CYAN(),
                            );
                            if moved {
                                ui.scroll_to_rect(rect, None);
                            }
                        }
                        let p = ui.painter_at(rect);
                        p.text(
                            egui::pos2(rect.left() + 10.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &e.label,
                            egui::FontId::proportional(13.0),
                            if sel { crate::theme::TEXT_STRONG() } else { crate::theme::TEXT() },
                        );
                        p.text(
                            egui::pos2(rect.right() - 8.0, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            &e.group,
                            egui::FontId::proportional(10.5),
                            crate::theme::gray(120),
                        );
                        if r.clicked() {
                            chosen = Some(e.go);
                        }
                    }
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
            // A click anywhere else is "never mind".
            if r.response.clicked_elsewhere() {
                esc = true;
            }
        }
        if let Some(go) = chosen {
            self.palette.open = false;
            self.palette_go(go, now, cmds);
        } else if esc {
            self.palette.open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(group: &str, label: &str) -> Entry {
        Entry { group: group.into(), label: label.into(), go: Go::Window(Win::Bands) }
    }

    #[test]
    fn a_typed_frequency_is_read_in_the_unit_it_is_plainly_in() {
        assert_eq!(parse_freq("14.074"), Some(14_074_000.0));
        assert_eq!(parse_freq("14,074"), Some(14_074_000.0));
        assert_eq!(parse_freq("7100k"), Some(7_100_000.0));
        assert_eq!(parse_freq("7100 khz"), Some(7_100_000.0));
        assert_eq!(parse_freq("145.5 MHz"), Some(145_500_000.0));
        assert_eq!(parse_freq("145500000"), Some(145_500_000.0));
        assert_eq!(parse_freq("10489.75"), Some(10_489_750_000.0));
        assert_eq!(parse_freq("137000hz"), Some(137_000.0));
    }

    #[test]
    fn a_band_or_a_word_is_not_a_frequency() {
        // Forty metres, not 40 MHz.
        assert_eq!(parse_freq("40m"), None);
        assert_eq!(parse_freq("ft8"), None);
        assert_eq!(parse_freq(""), None);
        assert_eq!(parse_freq("."), None);
        assert_eq!(parse_freq("0"), None);
        assert_eq!(parse_freq("-7"), None);
    }

    #[test]
    fn literal_matches_rank_above_accidental_subsequences() {
        let entries = vec![
            entry("Display", "Undo span blanking"),
            entry("Receive", "Mode USB"),
            entry("Settings", "Uploads"),
        ];
        let r = rank(&entries, "usb");
        assert_eq!(r.first(), Some(&1));
        // The group is searched too.
        assert_eq!(rank(&entries, "settings upl"), vec![2]);
        // An empty box lists everything as filed.
        assert_eq!(rank(&entries, ""), vec![0, 1, 2]);
        assert!(rank(&entries, "zzzz").is_empty());
    }

    #[test]
    fn nothing_that_transmits_is_offered() {
        for a in Action::all() {
            if matches!(
                a,
                Action::Ptt
                    | Action::TuneCarrier
                    | Action::CwStraight
                    | Action::VoicePlay(_)
                    | Action::ToneBurst
            ) {
                assert!(!offered(a), "{a:?}");
            }
        }
        assert!(offered(Action::Mute));
        assert!(offered(Action::ToggleLogbook));
        assert!(!offered(Action::Volume));
    }
}
