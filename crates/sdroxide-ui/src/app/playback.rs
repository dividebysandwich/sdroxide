//! **PLAYBACK**: a recording played back into the decoders (fork discussion
//! #10).
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
                if st.radio > 0 { format!(" · radio {}", st.radio + 1) } else { String::new() }
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
            "no stamp in the name — plays in the mode you are in".into(),
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
        let mut out = Vec::new();
        let show = self.tool_window(
            ctx,
            "playback",
            "PLAYBACK — recorded audio",
            [640.0, 520.0],
            true,
            |me, ui| me.playback_body(ui, &mut out),
        );
        cmds.extend(out);
        self.playback.show = show;
    }

    fn playback_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::window_body_bg(ui);
        ui.label(
            RichText::new(
                "Play a recording back into the decoders — SSTV, CW, RTTY, PSK, FT8 and every \
                 other mode that decodes from audio. The radio stands down while it plays, the \
                 dial and mode go to what the recording was made on, and STOP puts the radio \
                 back. Change the mode while it plays to try another decoder.",
            )
            .size(11.0)
            .color(crate::theme::gray(160)),
        );
        ui.add_space(4.0);
        if let Some(st) = self.state.playback.clone() {
            self.playback_transport(ui, &st, cmds, true);
            ui.add_space(4.0);
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Find").size(11.0).color(crate::theme::gray(150)));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.playback.find)
                    .desired_width(200.0)
                    .hint_text("mode, MHz, date, name…"),
            );
            if crate::chrome::chip(ui, false, "REFRESH")
                .on_hover_text("Read the recordings folder again")
                .clicked()
            {
                self.playback.asked = false;
            }
        });
        let Some(listing) = self.playback.listing.clone() else {
            ui.label(RichText::new("Asking the station for its recordings…").size(11.0));
            return;
        };
        if !listing.dir.is_empty() {
            ui.label(
                RichText::new(format!("Folder: {}", listing.dir))
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
                    ui.label(
                        RichText::new(
                            "No recordings yet. Press REC → MP3 or WAV while something is on the \
                             air; the file appears here, named with its time, frequency and mode.",
                        )
                        .color(crate::theme::gray(150)),
                    );
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
                                    if current { "▶ ON" } else { "▶ PLAY" },
                                )
                            })
                            .inner
                            .on_hover_text(if file.iq {
                                "An I/Q capture, not audio. Playback takes the audio the recorder \
                                 writes; an I/Q capture is played with --file."
                            } else if transmitting {
                                "Not while transmitting"
                            } else {
                                "Play this recording into the decoders from the start"
                            })
                            .on_disabled_hover_text(if file.iq {
                                "An I/Q capture, not audio. Playback takes the audio the recorder \
                                 writes; an I/Q capture is played with --file."
                            } else {
                                "Not while transmitting"
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
                            Some("Playing back — stop it before deleting it")
                        } else if writing.iter().any(|w| w.as_deref() == Some(file.name.as_str())) {
                            Some("Still being recorded — stop REC before deleting it")
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
                                    if armed { "DELETE?" } else { "CLEAR" },
                                    crate::theme::ALERT(),
                                    egui::Color32::WHITE,
                                )
                            })
                            .inner
                            .on_hover_text(if armed {
                                "Press again to delete this file for good. It cannot be undone."
                            } else {
                                "Delete this recording from the folder (asks once more first)"
                            })
                            .on_disabled_hover_text(busy.unwrap_or_default());
                        if clear.clicked() {
                            if let Some(cmd) = self.playback.press_clear(&file.name) {
                                cmds.push(Command::Playback(cmd));
                            }
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
            ui.label(RichText::new("▶ PLAYBACK").strong().color(crate::theme::ALERT()));
            ui.label(
                RichText::new("not live — the radio is standing down")
                    .size(11.0)
                    .color(crate::theme::gray(170)),
            );
            let (label, hover) = if st.paused || st.ended {
                ("▶ PLAY", "Carry on playing")
            } else {
                ("⏸ PAUSE", "Pause — the decoders hear silence until you carry on")
            };
            if crate::chrome::chip(ui, false, label).on_hover_text(hover).clicked() {
                if st.ended {
                    cmds.push(Command::Playback(PlaybackCommand::Seek(0.0)));
                    cmds.push(Command::Playback(PlaybackCommand::Pause(false)));
                } else {
                    cmds.push(Command::Playback(PlaybackCommand::Pause(!st.paused)));
                }
            }
            if crate::chrome::chip(ui, false, "↺ AGAIN")
                .on_hover_text("Back to the start — another try at a picture or a decode")
                .clicked()
            {
                cmds.push(Command::Playback(PlaybackCommand::Seek(0.0)));
                cmds.push(Command::Playback(PlaybackCommand::Pause(false)));
            }
            if crate::chrome::chip(ui, st.looping, "LOOP")
                .on_hover_text("Start again from the top when the file ends")
                .clicked()
            {
                cmds.push(Command::Playback(PlaybackCommand::Loop(!st.looping)));
            }
            if crate::chrome::chip_accent(
                ui,
                true,
                "■ STOP",
                crate::theme::ALERT(),
                egui::Color32::WHITE,
            )
            .on_hover_text("End the playback and put the radio back where it was")
            .clicked()
            {
                cmds.push(Command::Playback(PlaybackCommand::Stop));
            }
            if !full && crate::chrome::chip(ui, self.playback.show, "FILES").clicked() {
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
                ui.label(RichText::new("end of file").size(11.0).color(crate::theme::gray(160)));
            } else if st.paused {
                ui.label(RichText::new("paused").size(11.0).color(crate::theme::gray(160)));
            }
        });
        let mut line = st.name.clone();
        if full {
            match st.stamp {
                Some(stamp) => {
                    line = format!(
                        "{line}\nRecorded on {:.4} MHz{} — the dial and mode were set from the \
                         file's name.",
                        stamp.freq_hz / 1e6,
                        stamp.mode.map_or(String::new(), |m| format!(" in {}", m.label())),
                    );
                }
                None => {
                    line = format!(
                        "{line}\nThe name records no dial or mode, so it plays in the mode you \
                         are in — pick the decoder from the mode menu."
                    );
                }
            }
            line.push_str(&format!(
                "\n{} Hz, {} — the {} is played.",
                st.rate_hz,
                if st.channels >= 2 { "stereo" } else { "mono" },
                if st.channels >= 2 { "left (receive) channel" } else { "one channel" }
            ));
        }
        ui.label(RichText::new(line).size(10.5).color(crate::theme::CYAN_DIM()));
        if let Some(f) = &st.fault {
            ui.label(
                RichText::new(format!("Stopped: {f}")).size(11.0).color(crate::theme::ALERT()),
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
