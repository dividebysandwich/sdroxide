//! **PLAYBACK**: a recording played back into the decoders.
//!
//! The recorder names each file with the moment, the dial and the mode it was
//! made at; this is the other half. Pick a recording and the station stands its
//! radio down, puts the dial and the mode back to what the file records, and
//! plays it through the path a sound-card radio's audio takes — so the SSTV
//! picture that was missed paints again, the waterfall moves, and the decode
//! log fills as though it were on the air. Stopping puts the radio back.
//!
//! The files are the station's, not the window's: on a browser this lists the
//! server's recordings folder and the engine opens them there.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{Command, PlaybackCommand, PlaybackStatus, RecordingFile, RecordingListing};

/// The window's state and the last list the station sent.
#[derive(Default)]
pub(in crate::app) struct PlaybackUi {
    pub(in crate::app) show: bool,
    listing: Option<RecordingListing>,
    /// The list has been asked for since the window last opened.
    asked: bool,
    find: String,
    /// The position under the seek bar while it is being dragged — sent on
    /// release, so a drag is one seek and not a hundred.
    scrub: Option<f64>,
    /// The file whose CLEAR has been pressed once. Deleting is for good, so it
    /// takes a second press on the same row, which then reads DELETE?.
    clear_armed: Option<String>,
}

impl PlaybackUi {
    pub(in crate::app) fn accept(&mut self, listing: RecordingListing) {
        self.listing = Some(listing);
    }

    /// What pressing CLEAR on `name` does: the first press arms it, the second
    /// press on the same file deletes it. Pressing another file's CLEAR moves
    /// the arming there, so a stray first press can never delete anything.
    fn press_clear(&mut self, name: &str) -> Option<PlaybackCommand> {
        if self.clear_armed.as_deref() == Some(name) {
            self.clear_armed = None;
            Some(PlaybackCommand::Delete { name: name.to_string() })
        } else {
            self.clear_armed = Some(name.to_string());
            None
        }
    }

    /// Open the window and ask for a fresh list.
    pub(in crate::app) fn open(&mut self) {
        self.show = true;
        self.asked = false;
    }
}

/// `m:ss`, or `h:mm:ss` past the hour.
pub(in crate::app) fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// A file size the way a person reads one.
fn size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    } else {
        format!("{:.0} kB", (b / 1024.0).max(1.0))
    }
}

/// What a file's row says it is: from its name's stamp when it has one, so a
/// recording reads as "SSTV on 14.230 MHz", not as a file name.
fn describe(file: &RecordingFile) -> (String, String) {
    match file.stamp() {
        Some(st) => {
            let when =
                crate::app::util::date_str(st.unix) + " " + &crate::app::util::time_str(st.unix);
            let what = format!(
                "{:.4} MHz · {}{}",
                st.freq_hz / 1e6,
                st.mode.map_or("?", |m| m.label()),
                if st.radio > 0 {
                    tr!("playback-radio-n", n = st.radio + 1)
                } else {
                    String::new()
                }
            );
            (format!("{when}Z"), what)
        }
        None => (
            if file.modified_unix > 0 {
                crate::app::util::date_str(file.modified_unix)
                    + " "
                    + &crate::app::util::time_str(file.modified_unix)
                    + "Z"
            } else {
                String::new()
            },
            tr!("playback-no-stamp"),
        ),
    }
}

/// Whether a file matches the window's find box: its name, or what its stamp
/// says (a mode, a frequency).
fn matches(file: &RecordingFile, find: &str) -> bool {
    let find = find.trim().to_ascii_lowercase();
    if find.is_empty() {
        return true;
    }
    let (when, what) = describe(file);
    [file.name.as_str(), &when, &what].iter().any(|s| s.to_ascii_lowercase().contains(&find))
}

impl super::SdroxideApp {
    /// Ask the station for its recordings when the window has just opened.
    pub(in crate::app) fn poll_playback_list(&mut self, cmds: &mut Vec<Command>) {
        if self.playback.show && !self.playback.asked {
            self.playback.asked = true;
            cmds.push(Command::ListRecordings);
        }
    }

    pub(in crate::app) fn playback_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.playback.show {
            return;
        }
        let mut open = self.playback.show;
        let tool_title = tr!("playback-title");
        let mut tool_st = self.take_tool_state("PLAYBACK");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("PLAYBACK", &tool_title, [640.0, 520.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| self.playback_body(ui, cmds),
        );
        self.put_tool_state("PLAYBACK", tool_st);
        self.playback.show = open;
    }

    fn playback_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::window_body_bg(ui);
        ui.label(RichText::new(tr!("playback-intro")).size(11.0).color(crate::theme::gray(160)));
        ui.add_space(4.0);
        if let Some(st) = self.state.playback.clone() {
            self.playback_transport(ui, &st, cmds, true);
            ui.add_space(4.0);
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!("playback-find")).size(11.0).color(crate::theme::gray(150)));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.playback.find)
                    .desired_width(200.0)
                    .hint_text(tr!("playback-find-hint")),
            );
            if crate::chrome::chip(ui, false, tr!("playback-refresh"))
                .on_hover_text(tr!("playback-refresh-hint"))
                .clicked()
            {
                self.playback.asked = false;
            }
        });
        let Some(listing) = self.playback.listing.clone() else {
            ui.label(RichText::new(tr!("playback-asking")).size(11.0));
            return;
        };
        if !listing.dir.is_empty() {
            ui.label(
                RichText::new(tr!("playback-folder", dir = listing.dir))
                    .size(10.0)
                    .color(crate::theme::gray(140)),
            );
        }
        if let Some(e) = &listing.error {
            ui.label(RichText::new(e).size(11.0).color(crate::theme::ALERT()));
        }
        ui.separator();
        let playing = self.state.playback.as_ref().map(|p| p.name.clone());
        let transmitting = self.state.tx.ptt || self.state.tx.tune;
        let writing = [self.state.recording_file.clone(), self.state.iq_recording_file.clone()];
        egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("playback-list").show(
            ui,
            |ui| {
                if listing.files.is_empty() {
                    ui.label(RichText::new(tr!("playback-none")).color(crate::theme::gray(150)));
                }
                let find = self.playback.find.clone();
                for file in listing.files.iter().filter(|f| matches(f, &find)) {
                    let (when, what) = describe(file);
                    let current = playing.as_deref() == Some(file.name.as_str());
                    ui.horizontal(|ui| {
                        let enabled = !file.iq && !transmitting;
                        let chip = ui
                            .add_enabled_ui(enabled, |ui| {
                                crate::chrome::chip(
                                    ui,
                                    current,
                                    if current { tr!("playback-on") } else { tr!("playback-play") },
                                )
                            })
                            .inner
                            .on_hover_text(if file.iq {
                                tr!("playback-iq-hint")
                            } else if transmitting {
                                tr!("playback-not-while-tx")
                            } else {
                                tr!("playback-play-hint")
                            })
                            .on_disabled_hover_text(if file.iq {
                                tr!("playback-iq-hint")
                            } else {
                                tr!("playback-not-while-tx")
                            });
                        if chip.clicked() {
                            cmds.push(Command::Playback(PlaybackCommand::Open {
                                name: file.name.clone(),
                            }));
                        }
                        // Deleting what is in use would pull the file out from
                        // under the player or the recorder; the chip says so
                        // rather than doing nothing.
                        let busy = if current {
                            Some(tr!("playback-busy-playing"))
                        } else if writing.iter().any(|w| w.as_deref() == Some(file.name.as_str())) {
                            Some(tr!("playback-busy-recording"))
                        } else {
                            None
                        };
                        let armed =
                            self.playback.clear_armed.as_deref() == Some(file.name.as_str());
                        let clear = ui
                            .add_enabled_ui(busy.is_none(), |ui| {
                                crate::chrome::chip_accent(
                                    ui,
                                    armed,
                                    if armed {
                                        tr!("playback-delete-ask")
                                    } else {
                                        tr!("decodes-clear")
                                    },
                                    crate::theme::ALERT(),
                                    egui::Color32::WHITE,
                                )
                            })
                            .inner
                            .on_hover_text(if armed {
                                tr!("playback-delete-hint-armed")
                            } else {
                                tr!("playback-delete-hint")
                            })
                            .on_disabled_hover_text(busy.unwrap_or_default());
                        if clear.clicked()
                            && let Some(cmd) = self.playback.press_clear(&file.name)
                        {
                            cmds.push(Command::Playback(cmd));
                        }
                        ui.label(RichText::new(when).monospace().size(11.0));
                        ui.label(RichText::new(what).size(11.0).strong());
                        ui.label(
                            RichText::new(if file.iq {
                                "I/Q".to_string()
                            } else {
                                size(file.bytes)
                            })
                            .size(10.5)
                            .color(crate::theme::gray(150)),
                        );
                    })
                    .response
                    .on_hover_text(&file.name);
                }
            },
        );
    }

    /// The strip above the waterfall while a recording plays, so nobody takes
    /// a recording for the live band. Drawn whatever window is open.
    pub(in crate::app) fn playback_strip(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some(st) = self.state.playback.clone() else { return };
        let (wash, rule, _, _) = super::frame::notice_banner_colors();
        egui::Frame::new()
            .fill(wash)
            .stroke(egui::Stroke::new(1.0, rule))
            .inner_margin(egui::Margin::symmetric(8, 5))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.playback_transport(ui, &st, cmds, false);
            });
        // The position has to keep moving between state frames.
        if !st.paused && !st.ended {
            crate::repaint::schedule_ms(ui.ctx(), 250);
        }
    }

    /// Play/pause, stop, loop, the seek bar and what is playing. `full` is the
    /// window's version, which can afford the dial and mode line.
    fn playback_transport(
        &mut self,
        ui: &mut egui::Ui,
        st: &PlaybackStatus,
        cmds: &mut Vec<Command>,
        full: bool,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr!("playback-strip-label")).strong().color(crate::theme::ALERT()),
            );
            ui.label(
                RichText::new(tr!("playback-not-live")).size(11.0).color(crate::theme::gray(170)),
            );
            let (label, hover) = if st.paused || st.ended {
                (tr!("playback-play"), tr!("playback-carry-on"))
            } else {
                (tr!("playback-pause"), tr!("playback-pause-hint"))
            };
            if crate::chrome::chip(ui, false, label).on_hover_text(hover).clicked() {
                if st.ended {
                    cmds.push(Command::Playback(PlaybackCommand::Seek(0.0)));
                    cmds.push(Command::Playback(PlaybackCommand::Pause(false)));
                } else {
                    cmds.push(Command::Playback(PlaybackCommand::Pause(!st.paused)));
                }
            }
            if crate::chrome::chip(ui, false, tr!("playback-again"))
                .on_hover_text(tr!("playback-again-hint"))
                .clicked()
            {
                cmds.push(Command::Playback(PlaybackCommand::Seek(0.0)));
                cmds.push(Command::Playback(PlaybackCommand::Pause(false)));
            }
            if crate::chrome::chip(ui, st.looping, tr!("playback-loop"))
                .on_hover_text(tr!("playback-loop-hint"))
                .clicked()
            {
                cmds.push(Command::Playback(PlaybackCommand::Loop(!st.looping)));
            }
            if crate::chrome::chip_accent(
                ui,
                true,
                tr!("playback-stop"),
                crate::theme::ALERT(),
                egui::Color32::WHITE,
            )
            .on_hover_text(tr!("playback-stop-hint"))
            .clicked()
            {
                cmds.push(Command::Playback(PlaybackCommand::Stop));
            }
            if !full && crate::chrome::chip(ui, self.playback.show, tr!("playback-files")).clicked()
            {
                if self.playback.show {
                    self.playback.show = false;
                } else {
                    self.playback.open();
                }
            }
        });
        ui.horizontal(|ui| {
            let pos = self.playback.scrub.unwrap_or(st.position_s);
            match st.duration_s {
                Some(total) if total > 0.0 => {
                    let mut at = pos.min(total);
                    let w = (ui.available_width() - 120.0).clamp(80.0, 520.0);
                    ui.spacing_mut().slider_width = w;
                    let r = ui.add(egui::Slider::new(&mut at, 0.0..=total).show_value(false));
                    if r.dragged() || r.changed() {
                        self.playback.scrub = Some(at);
                    }
                    if r.drag_stopped() || (r.changed() && !r.dragged()) {
                        cmds.push(Command::Playback(PlaybackCommand::Seek(at)));
                        self.playback.scrub = None;
                    }
                    ui.label(
                        RichText::new(format!("{} / {}", clock(at), clock(total)))
                            .monospace()
                            .size(11.0),
                    );
                }
                _ => {
                    ui.label(RichText::new(clock(pos)).monospace().size(11.0));
                }
            }
            if st.ended {
                ui.label(
                    RichText::new(tr!("playback-end-of-file"))
                        .size(11.0)
                        .color(crate::theme::gray(160)),
                );
            } else if st.paused {
                ui.label(
                    RichText::new(tr!("playback-paused")).size(11.0).color(crate::theme::gray(160)),
                );
            }
        });
        let mut line = st.name.clone();
        if full {
            match st.stamp {
                Some(stamp) => {
                    let mhz = format!("{:.4}", stamp.freq_hz / 1e6);
                    let text = match stamp.mode {
                        Some(m) => tr!("playback-recorded-on-mode", mhz = mhz, mode = m.label()),
                        None => tr!("playback-recorded-on", mhz = mhz),
                    };
                    line = format!("{line}\n{text}");
                }
                None => {
                    line = format!("{line}\n{}", tr!("playback-no-dial-mode"));
                }
            }
            line.push('\n');
            line.push_str(&if st.channels >= 2 {
                tr!("playback-format-stereo", rate = st.rate_hz)
            } else {
                tr!("playback-format-mono", rate = st.rate_hz)
            });
        }
        ui.label(RichText::new(line).size(10.5).color(crate::theme::CYAN_DIM()));
        if let Some(f) = &st.fault {
            ui.label(
                RichText::new(tr!("playback-stopped", fault = f))
                    .size(11.0)
                    .color(crate::theme::ALERT()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deleting is for good, so one press only arms it: the second press on
    /// the same file deletes, and a press on another file moves the arming
    /// rather than deleting either.
    #[test]
    fn clear_asks_once_more_before_deleting() {
        let mut ui = PlaybackUi::default();
        assert_eq!(ui.press_clear("a.mp3"), None);
        assert_eq!(ui.press_clear("b.mp3"), None, "a press on another file deleted something");
        assert_eq!(ui.press_clear("b.mp3"), Some(PlaybackCommand::Delete { name: "b.mp3".into() }));
        assert_eq!(ui.press_clear("b.mp3"), None, "the arming outlived the delete");
    }

    #[test]
    fn the_clock_reads_like_a_player() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(65.9), "1:05");
        assert_eq!(clock(3725.0), "1:02:05");
    }

    /// A recording is found by what it is, not only by its file name.
    #[test]
    fn find_reaches_the_stamp() {
        let file = RecordingFile {
            name: "sdroxide_2026-10-06_10-00-05Z_14.230000MHz_SSTV.mp3".into(),
            bytes: 1,
            modified_unix: 0,
            iq: false,
        };
        assert!(matches(&file, "sstv"));
        assert!(matches(&file, "14.230"));
        assert!(matches(&file, ""));
        assert!(!matches(&file, "rtty"));
    }
}
