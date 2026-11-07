//! Morse trainer: a reference translator, a listening practice player, and a
//! Koch-style drill.
//!
//! Learning Morse by ear: play characters at full speed with wide (Farnsworth)
//! spacing, type what you hear, and add the next character once the ones in
//! hand are copied reliably. The curriculum and scoring are
//! [`sdroxide_types::MorseProgress`]'s; the tone is [`sdroxide_dsp::CwTx`]'s,
//! the same keyer the CW transmit path uses, played **locally only** — nothing
//! here keys a radio, and it works on a receive-only one.
//!
//! Audio is native only, through the same `sdroxide_audio` output the audible
//! alerts use; the browser build gets the reference and the drill without a
//! speaker.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{Answer, MorseProgress};

#[cfg(not(target_arch = "wasm32"))]
use sdroxide_audio::start_output;
#[cfg(not(target_arch = "wasm32"))]
use sdroxide_dsp::{CwTx, text_duration_s};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

/// Which pane of the trainer is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Translate,
    Practice,
    Learn,
}

impl Tab {
    fn label(self) -> String {
        match self {
            Tab::Translate => tr!("morse-translate"),
            Tab::Practice => tr!("morse-practice"),
            Tab::Learn => tr!("morse-learn"),
        }
    }
}

/// Everything the trainer window holds. Session state lives here; the score
/// rides [`MorseProgress`] and is persisted.
pub(in crate::app) struct MorseState {
    pub(in crate::app) show: bool,
    pub(in crate::app) progress: MorseProgress,
    tab: Tab,
    /// Reference pane: text to render as Morse. The translator is native-only.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    text: String,
    /// Reference pane: Morse to render as text.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    code: String,
    /// Practice topic: what the PLAY button sends.
    play_text: String,
    pitch_hz: f32,
    wpm: f32,
    farnsworth_wpm: f32,
    /// Learn pane: the character being asked, what was typed, and the verdict.
    target: Option<char>,
    answer: String,
    feedback: Option<(bool, char)>,
    /// Whether the character under test is shown (off while the drill runs).
    reveal: bool,
    /// A tiny xorshift, so the next target does not need an RNG dependency.
    rng: u64,
    #[cfg(not(target_arch = "wasm32"))]
    audio: Option<MorseSink>,
    /// A failed device open, said in the pane rather than swallowed.
    #[cfg(not(target_arch = "wasm32"))]
    audio_error: Option<String>,
}

impl MorseState {
    pub(in crate::app) fn new(progress: MorseProgress) -> Self {
        MorseState {
            show: false,
            progress,
            tab: Tab::Translate,
            text: "CQ CQ CQ DE NOCALL".into(),
            code: String::new(),
            play_text: "CQ CQ CQ DE NOCALL".into(),
            pitch_hz: 700.0,
            wpm: 20.0,
            farnsworth_wpm: 10.0,
            target: None,
            answer: String::new(),
            feedback: None,
            reveal: false,
            rng: crate::time::now_unix().max(1) as u64,
            #[cfg(not(target_arch = "wasm32"))]
            audio: None,
            #[cfg(not(target_arch = "wasm32"))]
            audio_error: None,
        }
    }

    /// A pseudo-random character from the unlocked set, and the new state.
    fn next_target(&mut self) -> char {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let set = self.progress.set();
        set[(self.rng as usize) % set.len()]
    }

    /// A character is played three times with a breath between, which is how a
    /// single letter is copied rather than guessed from one hearing.
    fn play_letter(&mut self, c: char, device: Option<String>) {
        self.play(&format!("{c} {c} {c}"), device);
    }

    /// Play `text` locally (native; a no-op in the browser), on `device` — the
    /// alert output, so the tone goes where the operator already chose to hear
    /// the app's own sounds rather than to the system default, which on some
    /// stations is the rig's USB codec.
    ///
    /// The worker renders the tone a block at a time at the rate the device
    /// actually opened at: rendering here at a fixed 48 kHz played at the wrong
    /// pitch and speed on a 44.1 kHz card, and a long text froze the window
    /// while it rendered.
    #[cfg(not(target_arch = "wasm32"))]
    fn play(&mut self, text: &str, device: Option<String>) {
        if text.trim().is_empty() {
            return;
        }
        if self.audio.is_none() {
            match MorseSink::start(device) {
                Ok(s) => {
                    self.audio_error = None;
                    self.audio = Some(s);
                }
                Err(e) => {
                    self.audio_error = Some(e);
                    return;
                }
            }
        }
        if let Some(sink) = self.audio.as_ref() {
            sink.play(Tone {
                text: text.to_string(),
                pitch_hz: self.pitch_hz,
                wpm: self.wpm,
                farnsworth_wpm: self.farnsworth_wpm,
            });
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn play(&mut self, _text: &str, _device: Option<String>) {}

    /// Stop playback and release the device.
    fn stop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.audio = None;
        }
    }
}

impl Drop for MorseState {
    fn drop(&mut self) {
        self.stop();
    }
}

// ─── Audio worker (native) ────────────────────────────────────────────────────

/// A message to play, and how.
#[cfg(not(target_arch = "wasm32"))]
struct Tone {
    text: String,
    pitch_hz: f32,
    wpm: f32,
    farnsworth_wpm: f32,
}

#[cfg(not(target_arch = "wasm32"))]
enum Job {
    Play(Tone),
    Quit,
}

/// Owns the worker and the sound card. One at a time, opened when the operator
/// first plays and held until the window closes or STOP is pressed (closing
/// the window calls [`MorseState::stop`]).
#[cfg(not(target_arch = "wasm32"))]
struct MorseSink {
    tx: Option<SyncSender<Job>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    error: Arc<std::sync::Mutex<Option<String>>>,
}

#[cfg(not(target_arch = "wasm32"))]
const TICK: Duration = Duration::from_millis(20);
/// Keep this much queued ahead of the device, as the alerts worker does.
#[cfg(not(target_arch = "wasm32"))]
const LEAD_S: f64 = 0.08;

#[cfg(not(target_arch = "wasm32"))]
impl MorseSink {
    fn start(device: Option<String>) -> Result<Self, String> {
        let (tx, rx) = sync_channel::<Job>(8);
        let stop = Arc::new(AtomicBool::new(false));
        let error = Arc::new(std::sync::Mutex::new(None));
        let (stop2, error2) = (Arc::clone(&stop), Arc::clone(&error));
        let thread = std::thread::Builder::new()
            .name("morse-trainer".into())
            .spawn(move || worker(rx, device, &stop2, &error2))
            .map_err(|e| tr!("morse-could-not-start-morse-audio", error = e.to_string()))?;
        Ok(MorseSink { tx: Some(tx), stop, thread: Some(thread), error })
    }

    fn play(&self, tone: Tone) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(Job::Play(tone));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for MorseSink {
    fn drop(&mut self) {
        // Signal first, then let go of the sender, then join — holding it across
        // the join deadlocks when the device failed to open.
        self.stop.store(true, Ordering::Relaxed);
        if let Some(tx) = self.tx.take() {
            let _ = tx.try_send(Job::Quit);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn worker(
    rx: Receiver<Job>,
    device: Option<String>,
    stop: &AtomicBool,
    error: &std::sync::Mutex<Option<String>>,
) {
    let (out, mut ring) =
        match start_output(device.as_deref(), 48_000, sdroxide_types::AudioRouting::Both) {
            Ok(v) => v,
            Err(e) => {
                *error.lock().unwrap() = Some(tr!("morse-no-audio-output", error = e.to_string()));
                // Drain so a send does not block forever.
                while let Ok(job) = rx.recv() {
                    if matches!(job, Job::Quit) {
                        break;
                    }
                }
                return;
            }
        };
    let rate = out.sample_rate;
    let capacity = rate as usize * 2;
    let lead = (rate * LEAD_S) as usize;
    let mut next = rx.recv().ok();
    while let Some(job) = next.take() {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let tone = match job {
            Job::Quit => break,
            Job::Play(t) => t,
        };
        let mut tx = CwTx::new(rate, tone.pitch_hz as f64, tone.wpm);
        tx.set_params(tone.pitch_hz as f64, tone.wpm, tone.farnsworth_wpm);
        tx.push_text(&tone.text);
        let mut block = [0.0f32; 256];
        let (mut i, mut n) = (0, 0);
        loop {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            // A newer PLAY replaces the message still sounding rather than
            // queueing behind it.
            if let Ok(job) = rx.try_recv() {
                next = Some(job);
                break;
            }
            if i == n {
                if tx.drained() {
                    break;
                }
                tx.next_block(&mut block);
                (i, n) = (0, block.len());
            }
            let queued = capacity.saturating_sub(ring.slots()) / 2;
            if queued < lead && ring.slots() >= 2 {
                let s = block[i];
                let _ = ring.push(s);
                let _ = ring.push(s); // interleaved stereo
                i += 1;
            } else {
                std::thread::sleep(TICK);
            }
        }
        if next.is_none() {
            next = rx.recv().ok();
        }
    }
}

// ─── Rendering helpers ────────────────────────────────────────────────────────

/// A message as its Morse, characters separated by spaces and words by ` / `.
///
/// The table is `sdroxide-dsp`'s — the same ITU alphabet the decoder and keyer
/// use, so the reference cannot drift from what goes on the air. That crate is
/// native-only, so the translator is too; the drill and the practice pane do
/// not need it.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn to_morse(text: &str) -> String {
    use sdroxide_dsp::morse_encode;
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push_str(" / ");
        }
        for (i, ch) in word.chars().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            match morse_encode(ch) {
                Some(code) => out.push_str(code),
                None => out.push('?'),
            }
        }
    }
    out
}

/// The text for a Morse string: `.`/`-` groups separated by spaces, ` / ` a
/// word break. Unknown groups read as `?`.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn from_morse(code: &str) -> String {
    use sdroxide_dsp::morse_decode;
    let mut out = String::new();
    for tok in code.split_whitespace() {
        if tok == "/" {
            out.push(' ');
            continue;
        }
        match morse_decode(tok) {
            Some(c) => out.push(c),
            None => out.push('?'),
        }
    }
    out
}

// ─── The window ───────────────────────────────────────────────────────────────

impl super::SdroxideApp {
    pub(in crate::app) fn morse_window(&mut self, ctx: &egui::Context) {
        if !self.morse.show {
            // Closed by its × or by the TRAINER chip: either way a message still
            // sounding stops with it, and the device is let go.
            self.morse.stop();
            return;
        }
        // Another tab may have trained since this one last looked.
        if let Some(p) = super::persist::shared_morse_progress() {
            self.morse.progress = p;
        }
        let mut open = self.morse.show;
        let resp = egui::Window::new(tr!("morse-morse"))
            .id(crate::layout::salted_id(ctx, "Morse"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 520.0))
            .default_height(crate::layout::window_h(ctx, 430.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                self.morse_body(ui);
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.morse.show = open;
    }

    fn morse_body(&mut self, ui: &mut egui::Ui) {
        // The worker opens the device asynchronously; surface a failure once it
        // has landed rather than leaving the pane looking like it played.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let failed = self.morse.audio.as_ref().and_then(|s| s.error.lock().unwrap().clone());
            if failed.is_some() {
                self.morse.audio = None;
                self.morse.audio_error = failed;
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for tab in [Tab::Translate, Tab::Practice, Tab::Learn] {
                if crate::chrome::chip(ui, self.morse.tab == tab, tab.label()).clicked() {
                    self.morse.tab = tab;
                }
            }
        });
        ui.separator();
        match self.morse.tab {
            Tab::Translate => self.morse_translate(ui),
            Tab::Practice => self.morse_practice(ui),
            Tab::Learn => self.morse_learn(ui),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn morse_translate(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(tr!("morse-text-morse")).size(11.0).color(crate::theme::gray(170)));
        crate::chrome::field(ui, egui::TextEdit::multiline(&mut self.morse.text).desired_rows(3));
        let morse = to_morse(&self.morse.text);
        ui.add_space(2.0);
        ui.label(RichText::new(morse).monospace().size(14.0).color(crate::theme::CYAN()));
        ui.add_space(10.0);
        ui.label(RichText::new(tr!("morse-morse-text")).size(11.0).color(crate::theme::gray(170)));
        crate::chrome::field(
            ui,
            egui::TextEdit::multiline(&mut self.morse.code)
                .desired_rows(3)
                .hint_text(".... . .-.. .-.. --- / .-- --- .-. .-.. -.."),
        );
        ui.label(RichText::new(from_morse(&self.morse.code)).size(14.0));
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr!("morse-per-character-space-between-characters")).size(9.5).weak(),
        );
    }

    #[cfg(target_arch = "wasm32")]
    fn morse_translate(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(tr!("morse-reference-translator-available-desktop-build"))
                .size(11.0)
                .weak(),
        );
    }

    fn morse_practice(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(tr!("morse-play-what-you-type-your"))
                .size(11.0)
                .color(crate::theme::gray(170)),
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::multiline(&mut self.morse.play_text).desired_rows(3),
        );
        self.morse_tone_controls(ui);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if crate::chrome::chip(ui, true, tr!("morse-play")).clicked() {
                let text = self.morse.play_text.clone();
                self.morse.play(&text, self.alerts.settings().device);
            }
            if crate::chrome::chip(ui, false, tr!("morse-stop")).clicked() {
                self.morse.stop();
            }
            #[cfg(not(target_arch = "wasm32"))]
            ui.label(
                RichText::new(tr!(
                    "morse-about-s",
                    farnsworth_wpm = format!(
                        "{:.0}",
                        text_duration_s(
                            &self.morse.play_text,
                            self.morse.wpm,
                            self.morse.farnsworth_wpm
                        )
                    )
                ))
                .size(10.0)
                .weak(),
            );
        });
        self.morse_audio_note(ui);
    }

    fn morse_learn(&mut self, ui: &mut egui::Ui) {
        let set =
            self.morse.progress.set().iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
        let (correct, total, streak, complete) = {
            let p = &self.morse.progress;
            (p.correct, p.total, p.streak, p.complete())
        };
        ui.label(RichText::new(tr!("morse-characters-play", set = set)).size(11.0));
        ui.label(
            RichText::new(tr!(
                "morse-score-run",
                correct = correct,
                total = total,
                streak = streak.min(sdroxide_types::ADVANCE_RUN),
                value = sdroxide_types::ADVANCE_RUN
            ))
            .size(11.0)
            .color(crate::theme::CYAN_DIM()),
        );
        if complete {
            ui.label(
                RichText::new(tr!("morse-every-character-learned-well-done"))
                    .color(crate::theme::GREEN()),
            );
        }
        ui.add_space(4.0);
        self.morse_tone_controls(ui);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            // The drill is copying by ear, and the browser build has no
            // speaker: a NEW there would score guesses.
            if cfg!(target_arch = "wasm32") {
                ui.label(
                    RichText::new(tr!("morse-drill-needs-sound-which-desktop")).size(10.5).weak(),
                );
            } else if crate::chrome::chip(ui, false, "NEW").clicked() {
                let c = self.morse.next_target();
                self.morse.target = Some(c);
                self.morse.answer.clear();
                self.morse.feedback = None;
                self.morse.play_letter(c, self.alerts.settings().device);
            }
            if crate::chrome::chip(ui, self.morse.reveal, tr!("morse-reveal")).clicked() {
                self.morse.reveal = !self.morse.reveal;
            }
            if crate::chrome::chip(ui, false, tr!("morse-reset")).clicked() {
                self.morse.progress.reset();
                self.morse.target = None;
                self.morse.answer.clear();
                self.morse.feedback = None;
                super::persist::persist_morse_progress(&self.morse.progress);
            }
            if let Some(c) = self.morse.target {
                if self.morse.reveal {
                    ui.label(RichText::new(c.to_string()).size(16.0).strong());
                }
                if crate::chrome::chip(ui, false, tr!("morse-replay")).clicked() {
                    self.morse.play_letter(c, self.alerts.settings().device);
                }
            }
        });
        ui.add_space(4.0);
        let target = self.morse.target;
        if let Some(want) = target {
            ui.horizontal(|ui| {
                ui.label(tr!("morse-what-did-you-hear"));
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.morse.answer)
                        .desired_width(80.0)
                        .hint_text("?"),
                );
                let submitted = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if crate::chrome::chip(ui, false, tr!("morse-check")).clicked() || submitted {
                    let got = self.morse.answer.clone();
                    let before = self.morse.progress.unlocked;
                    let verdict = self.morse.progress.answer(want, &got);
                    self.morse.feedback = Some((verdict != Answer::Wrong, want));
                    let promoted = self.morse.progress.unlocked > before;
                    self.morse.answer.clear();
                    if promoted {
                        self.morse.target = Some(self.morse.progress.newest());
                    }
                    super::persist::persist_morse_progress(&self.morse.progress);
                }
            });
            if let Some((ok, c)) = self.morse.feedback {
                let (ink, text) = if ok {
                    (crate::theme::GREEN(), tr!("morse-correct", value = c))
                } else {
                    (crate::theme::ALERT(), tr!("morse-was", value = c))
                };
                ui.label(RichText::new(text).color(ink));
            }
        } else {
            ui.label(RichText::new(tr!("morse-press-new-hear-character")).weak());
        }
        self.morse_audio_note(ui);
    }

    fn morse_tone_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr!("morse-tone"));
            ui.add(
                egui::DragValue::new(&mut self.morse.pitch_hz)
                    .speed(5.0)
                    .range(300.0..=1200.0)
                    .suffix(" Hz"),
            );
            ui.label(tr!("morse-speed"));
            ui.add(
                egui::DragValue::new(&mut self.morse.wpm)
                    .speed(0.5)
                    .range(5.0..=40.0)
                    .suffix(" wpm"),
            );
            // Farnsworth spacing: characters at full speed, gaps stretched.
            // Never above the character speed, which is what "no Farnsworth"
            // means to the keyer.
            self.morse.farnsworth_wpm = self.morse.farnsworth_wpm.min(self.morse.wpm);
            ui.label(tr!("morse-spacing"));
            ui.add(
                egui::DragValue::new(&mut self.morse.farnsworth_wpm)
                    .speed(0.5)
                    .range(5.0..=self.morse.wpm)
                    .suffix(" wpm"),
            )
            .on_hover_text(tr!("morse-characters-go-out-speed-above"));
        });
    }

    fn morse_audio_note(&self, ui: &mut egui::Ui) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(e) = &self.morse.audio_error {
            ui.label(RichText::new(e).size(10.0).color(crate::theme::ALERT()));
        }
        #[cfg(target_arch = "wasm32")]
        ui.label(RichText::new(tr!("morse-playback-available-desktop-build")).size(10.0).weak());
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips_through_morse() {
        for text in ["CQ CQ DE W1ABC", "HELLO WORLD", "TEST 123"] {
            let code = to_morse(text);
            assert_eq!(from_morse(&code).trim(), text, "{text} -> {code}");
        }
    }

    #[test]
    fn an_unknown_character_reads_as_a_question_mark() {
        assert_eq!(to_morse("A~B"), ".- ? -...");
        assert_eq!(from_morse(".- ..-.- -..."), "A?B");
    }

    #[test]
    fn word_breaks_survive_the_round_trip() {
        assert_eq!(to_morse("E T"), ". / -");
        assert_eq!(from_morse(". / -"), "E T");
    }
}
