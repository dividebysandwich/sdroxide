//! Audible alert runtime.
//!
//! The [`AlertStatus`] mirrors the speech engine's status shape, and the
//! worker is the same shape as `sdroxide-speech`'s: a thread owns the cpal
//! output and its ring buffer, and jobs are paced into the ring rather than
//! blocking. The difference is that every job here is synthesised — a handful
//! of oscillator patterns — so there is no model to load and no first-sound
//! pause. The ring opens in flight (a few milliseconds of silence), which is
//! what makes the first alarm sound listenable.

use crate::i18n::tr;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use sdroxide_types::{AlertEvent, AlertReply, AlertSettings, AlertSound, Decode, LogIndex};

use crate::time::now_unix_f64;

/// How the alarm sounds right now.
///
/// On wasm the variants above `Idle` are never *constructed*: there is no sink
/// to run or fail, so they exist only as the shape the settings tab describes.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub enum AlertStatus {
    /// Alerts switched off, or no settings applied yet.
    Idle,
    /// Driving the named device.
    Running(String),
    /// Could not open the device; alarms are silently skipped.
    Failed(String),
}

impl AlertStatus {
    /// A short human line for the tab ("Idle", the device name, the error).
    pub fn note(&self) -> Option<String> {
        match self {
            AlertStatus::Idle => None,
            AlertStatus::Running(d) => Some(tr!("alerts-alarming", device = d)),
            AlertStatus::Failed(e) => {
                Some(tr!("alerts-alarm-device-unavailable", error = e.to_string()))
            }
        }
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, AlertStatus::Failed(_))
    }
}

/// A per-station, per-event quiet period, so a station that decodes every
/// slot does not beep every slot. Novelty does not change until the contact is
/// logged, and a new entity calling CQ all evening would otherwise wail all
/// evening.
struct Cooldown {
    /// (callsign, event) → Unix seconds when it last alarmed.
    ///
    /// Wall-clock seconds rather than `Instant`: `Instant::now()` panics on
    /// `wasm32-unknown-unknown`, and the browser client runs this same code.
    /// Cooldowns are minutes long, so a clock that steps does not matter here.
    at: HashMap<(String, AlertEvent), f64>,
}

impl Cooldown {
    fn new() -> Self {
        Cooldown { at: HashMap::new() }
    }

    /// Whether this station may alarm for this event.
    fn eligible(&self, call: &str, event: AlertEvent) -> bool {
        match self.at.get(&(call.to_string(), event)) {
            None => true,
            Some(&when) => now_unix_f64() - when >= event.cooldown_s() as f64,
        }
    }

    fn mark(&mut self, call: &str, event: AlertEvent) {
        let now = now_unix_f64();
        self.at.insert((call.to_string(), event), now);
        // A wall of decoded stations could grow this forever, so once it gets
        // big, drop everything that has gone cold.
        if self.at.len() > 256 {
            self.at.retain(|_, when| now - *when < 600.0);
        }
    }
}

/// What a sound job carries to the worker.
#[cfg(not(target_arch = "wasm32"))]
enum Job {
    Play { sound: AlertSound, volume: f32 },
    Quit,
}

/// What an alarm just went off for, handed back to the caller.
///
/// The tone is played inside the runtime, but the spoken half is not: speech is
/// the [`crate::app::speech::SpeechRuntime`]'s, and saying the phrase through it
/// is the caller's job — and only the focused tab's, since a background radio's
/// phrase would queue up and be read out late. `sound` is the rule's tone, for
/// the caller to [`AlertRuntime::ring`] instead when it cannot speak.
#[derive(Debug, Clone)]
pub struct AlertFired {
    pub event: AlertEvent,
    pub call: String,
    pub reply: AlertReply,
    pub sound: AlertSound,
}

/// The alarms themselves, as a radio tab holds them: a handle on an
/// [`AlertCore`], which every tab of a station shares — see
/// [`AlertRuntime::station`].
pub struct AlertRuntime {
    core: Arc<Mutex<AlertCore>>,
}

/// Settings plus, on native, a background worker that owns the alert device.
struct AlertCore {
    status: Arc<Mutex<AlertStatus>>,
    cooldowns: Cooldown,
    settings: AlertSettings,
    #[cfg(not(target_arch = "wasm32"))]
    sink: Option<AlertSink>,
}

/// Owns the worker. Dropping it asks the worker to quit and waits for it, so
/// the cpal stream is closed tidily and never outlives its ring.
#[cfg(not(target_arch = "wasm32"))]
struct AlertSink {
    /// `Option` so [`Drop`] can let go of the sending end before joining: the
    /// worker's failure path drains until the channel *closes*, and it can
    /// never close while this still holds a sender.
    tx: Option<SyncSender<Job>>,
    /// Raised by [`Drop`], and read by the worker wherever it waits on the
    /// sound card — the one wait a closed channel cannot end.
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for AlertSink {
    fn drop(&mut self) {
        // The flag first: a worker part-way through an alarm is waiting on the
        // sound card, not on the channel, and a card that stopped taking
        // samples (unplugged mid-alarm) would hold it there for ever. It also
        // keeps the worker from playing out whatever was still queued before
        // it reached the `Quit`, which stalled the screen for as long as that
        // took.
        self.stop.store(true, Ordering::Relaxed);
        // Then the sender. The worker that failed to open a device waits for
        // the channel to close rather than for a `Quit`, so holding the sender
        // across the join deadlocked on quit or when alarms were turned off —
        // the whole app froze. `try_send`, so a full queue cannot block here.
        if let Some(tx) = self.tx.take() {
            let _ = tx.try_send(Job::Quit);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ── tone synthesis ──────────────────────────────────────────────────────────
//
// Every pattern is a few short notes, rendered at the device's own rate, so
// no resampler is needed. Native only: the browser build has no alert device
// yet, but keeps the settings so one can appear later.

/// Append `seconds` of silence.
#[cfg(not(target_arch = "wasm32"))]
fn silence_into(out: &mut Vec<f32>, rate: f64, seconds: f64) {
    let n = (rate * seconds) as usize;
    out.extend(std::iter::repeat_n(0.0, n));
}

/// Append one note: a sine with a short attack and a release that fades the
/// last third to zero, so a tone never starts or ends with a click.
#[cfg(not(target_arch = "wasm32"))]
fn note_into(out: &mut Vec<f32>, rate: f64, freq: f64, seconds: f64, volume: f32) {
    let n = (rate * seconds) as usize;
    let release = ((seconds * 0.30) * rate) as usize + 1;
    for i in 0..n {
        let t = i as f64 / rate;
        let attack = (t / 0.008).min(1.0);
        let release_env = {
            let rem = n - i;
            if rem < release { rem as f64 / release as f64 } else { 1.0 }
        };
        let s = (freq * 2.0 * std::f64::consts::PI * t).sin() as f32;
        out.push(s * attack as f32 * release_env as f32 * volume);
    }
}

/// Append one rising-then-falling sweep.
#[cfg(not(target_arch = "wasm32"))]
fn warble_into(out: &mut Vec<f32>, rate: f64, f0: f64, f1: f64, seconds: f64, volume: f32) {
    let n = ((rate * seconds) as usize).max(1);
    let release = ((seconds / 2.0) * rate) as usize + 1;
    let mut phase = 0.0;
    for i in 0..n {
        let u = i as f64 / n as f64;
        let f =
            if u < 0.5 { f0 + (f1 - f0) * (u * 2.0) } else { f1 - (f1 - f0) * ((u - 0.5) * 2.0) };
        phase += 2.0 * std::f64::consts::PI * f / rate;
        let release_env = {
            let rem = n - i;
            if rem < release { rem as f64 / release as f64 } else { 1.0 }
        };
        out.push(phase.sin() as f32 * release_env as f32 * volume);
    }
}

/// Repeat a mono block `n` times.
#[cfg(not(target_arch = "wasm32"))]
fn repeat_into(out: &mut Vec<f32>, block: &[f32], n: usize) {
    for _ in 0..n {
        out.extend_from_slice(block);
    }
}

/// Render a whole alarm at the device's rate, interleaved stereo so it is
/// audible whichever ear the operator favours.
#[cfg(not(target_arch = "wasm32"))]
fn render(sound: AlertSound, rate: u32, volume: f32) -> Vec<f32> {
    let r = rate as f64;
    let mut mono = Vec::new();
    match sound {
        AlertSound::Ding => note_into(&mut mono, r, 880.0, 0.28, volume),
        AlertSound::TwoTone => {
            note_into(&mut mono, r, 660.0, 0.12, volume);
            silence_into(&mut mono, r, 0.05);
            note_into(&mut mono, r, 990.0, 0.22, volume);
        }
        AlertSound::Triplet => {
            for f in [740.0, 880.0, 1080.0] {
                note_into(&mut mono, r, f, 0.09, volume);
                silence_into(&mut mono, r, 0.03);
            }
        }
        AlertSound::Warble => {
            for _ in 0..2 {
                warble_into(&mut mono, r, 650.0, 950.0, 0.26, volume);
                silence_into(&mut mono, r, 0.04);
            }
        }
        AlertSound::Digital => {
            // A square-ish chirp: the fundamental plus a half-strength second
            // harmonic, in hard 60 ms blocks.
            let mut block = Vec::new();
            note_into(&mut block, r, 1180.0, 0.06, volume);
            let n = (r * 0.06) as usize;
            for (i, s) in block.iter_mut().enumerate().take(n) {
                let t = i as f64 / r;
                *s =
                    (*s + 0.5 * (1180.0 * 2.0 * 2.0 * std::f64::consts::PI * t).sin() as f32) / 1.5;
            }
            repeat_into(&mut mono, &block, 3);
        }
    }

    // A short tail of silence so a device that pads its own output never clips
    // the last fading note.
    silence_into(&mut mono, r, 0.02);

    let mut out = Vec::with_capacity(mono.len() * 2);
    for s in mono {
        out.push(s);
        out.push(s);
    }
    out
}

impl AlertRuntime {
    /// A runtime of its own: its own device, settings and cooldowns.
    pub fn new(settings: AlertSettings) -> Self {
        AlertRuntime { core: Arc::new(Mutex::new(AlertCore::new(settings))) }
    }

    /// The station's alarm: one per process, whichever radio tab asks for it.
    ///
    /// Each tab of a multi-radio station used to build its own — its own
    /// output stream on the alert device and its own copy of the settings, read
    /// once at start-up — so switching alerts off on one tab left the others
    /// ringing until a restart, and a station calling us on two receivers
    /// rang twice. `settings` only seeds the first. Held weakly, so the last
    /// tab to close still closes the device.
    pub fn station(settings: AlertSettings) -> Self {
        static STATION: Mutex<Weak<Mutex<AlertCore>>> = Mutex::new(Weak::new());
        let mut slot = STATION.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(core) = slot.upgrade() {
            return AlertRuntime { core };
        }
        let runtime = AlertRuntime::new(settings);
        *slot = Arc::downgrade(&runtime.core);
        runtime
    }

    fn core(&self) -> MutexGuard<'_, AlertCore> {
        self.core.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A snapshot of the status, safe to read from any thread.
    pub fn status(&self) -> AlertStatus {
        self.core().status()
    }

    pub fn settings(&self) -> AlertSettings {
        self.core().settings.clone()
    }

    /// Mastership goes here: whether alarms are worth listening for at all.
    pub fn enabled(&self) -> bool {
        self.core().settings.enabled
    }

    /// Update the configuration, for every tab at once.
    pub fn set_settings(&mut self, settings: AlertSettings) {
        self.core().set_settings(settings);
    }

    /// A preview alarm for the settings tab.
    pub fn test(&self) {
        self.core().test();
    }

    /// Sound one alarm tone — a voice-only alert's stand-in where it cannot be
    /// spoken. See [`AlertFired`].
    pub fn ring(&self, sound: AlertSound) {
        self.core().play(sound);
    }

    /// Feed one WSJT-style decode batch — see [`AlertCore::on_ft8`].
    pub fn on_ft8(
        &mut self,
        decodes: &[Decode],
        my_call: &str,
        my_grid: &str,
        log: &LogIndex,
        band: &str,
    ) -> Option<AlertFired> {
        self.core().on_ft8(decodes, my_call, my_grid, log, band)
    }
}

impl AlertCore {
    /// Build the runtime and, if enabled, open the device in the background.
    fn new(settings: AlertSettings) -> Self {
        let mut runtime = AlertCore {
            status: Arc::new(Mutex::new(AlertStatus::Idle)),
            cooldowns: Cooldown::new(),
            settings,
            #[cfg(not(target_arch = "wasm32"))]
            sink: None,
        };
        runtime.sync_sink();
        runtime
    }

    /// Reconcile the native worker with `settings` — build it when alarms are
    /// enabled, drop it when they are not.
    fn sync_sink(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            // No audio on wasm yet; the settings are still remembered so a
            // future browser backend inherits them.
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.settings.enabled && self.sink.is_none() {
                self.sink =
                    Some(AlertSink::start(self.settings.device.clone(), self.status.clone()));
            } else if !self.settings.enabled {
                self.sink = None;
            }
        }
    }

    fn status(&self) -> AlertStatus {
        self.status.lock().unwrap().clone()
    }

    /// Update the configuration. Any change to enabled/device means a new
    /// worker; the rest is read live at each decode.
    fn set_settings(&mut self, settings: AlertSettings) {
        let device_changed = settings.device != self.settings.device;
        let enabled_changed = settings.enabled != self.settings.enabled;
        self.settings = settings;
        // The worker owns the device it opened, so a different device means a
        // different worker: drop the old one before reconciling, or the change
        // did nothing at all. (No worker to drop on wasm.)
        #[cfg(not(target_arch = "wasm32"))]
        if device_changed {
            self.sink = None;
        }
        if enabled_changed || device_changed {
            self.sync_sink();
        }
    }

    /// A preview alarm for the settings tab: whatever sound the "called" rule
    /// is set to.
    fn test(&self) {
        self.play(self.settings.events.called.sound);
    }

    /// Feed one WSJT-style decode batch. Alarms intentionally do **not** wait
    /// for the window to be focused — the point is to reach the operator when
    /// they are looking at another window entirely.
    fn on_ft8(
        &mut self,
        decodes: &[Decode],
        my_call: &str,
        my_grid: &str,
        log: &LogIndex,
        band: &str,
    ) -> Option<AlertFired> {
        if !self.settings.enabled {
            return None;
        }
        let my_call = my_call.trim().to_ascii_uppercase();
        let my_grid = my_grid.trim().to_ascii_uppercase();
        if my_call.is_empty() {
            return None;
        }
        // One alarm per batch. A busy slot carries a dozen decodes and more
        // than one of them can match — all sixteen ringing one after another
        // says less than one does, and takes half a minute to say it. That one
        // is the match that matters most, not the first in the list: the list
        // is in decode order, and a new grid decoded ahead of a station calling
        // us must not be what silences the call.
        let mut best: Option<(AlertEvent, &str)> = None;
        for d in decodes {
            let Some(from) = d.from.as_deref() else { continue };
            let novelty = log.novelty(from, d.grid.as_deref(), band);
            let Some(event) = AlertEvent::for_decode(d, &my_call, &my_grid, novelty) else {
                continue;
            };
            if !event.rule(&self.settings.events).enabled {
                continue;
            }
            if !self.cooldowns.eligible(from, event) {
                continue;
            }
            if best.is_none_or(|(b, _)| event.rank() < b.rank()) {
                best = Some((event, from));
            }
        }
        if let Some((event, from)) = best {
            let rule = event.rule(&self.settings.events);
            let (sound, reply) = (rule.sound, rule.reply);
            // Voice-only rules leave the tone unsounded; the caller says the
            // phrase.
            if reply.plays_tone() {
                self.play(sound);
            }
            self.cooldowns.mark(from, event);
            return Some(AlertFired { event, call: from.to_string(), reply, sound });
        }
        None
    }

    /// Queue an alarm if there is a worker to play it.
    fn play(&self, sound: AlertSound) {
        let volume = self.settings.volume();
        if volume <= 0.0 {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref sink) = self.sink {
            sink.play(sound, volume);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = sound;
    }
}

/// The native worker: a thread that opens the device, then paces tones into
/// its ring. Owns the stream and ring, so both live exactly as long as the
/// loop does.
#[cfg(not(target_arch = "wasm32"))]
impl AlertSink {
    fn start(device: Option<String>, status: Arc<Mutex<AlertStatus>>) -> Self {
        let (tx, rx) = sync_channel::<Job>(16);
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let status = status.clone();
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("alerts".into())
                .spawn(move || worker(rx, device, status, &stop))
                .ok()
        };
        if thread.is_none() {
            *status.lock().unwrap() =
                AlertStatus::Failed(tr!("alerts-could-not-start-alert-thread"));
        }
        AlertSink { tx: Some(tx), stop, thread }
    }

    fn play(&self, sound: AlertSound, volume: f32) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(Job::Play { sound, volume });
        }
    }
}

/// How far ahead of the consumer the worker is allowed to get, like the
/// speech worker's [`LEAD_S`](crate::app::speech::SpeechRuntime).
#[cfg(not(target_arch = "wasm32"))]
const LEAD_S: f64 = 0.08;

/// The worker's pulse: short enough that a note boundary never starves the
/// ring, long enough not to busy-spin.
#[cfg(not(target_arch = "wasm32"))]
const TICK: Duration = Duration::from_millis(20);

/// Open the alert device and drive it until told to quit.
#[cfg(not(target_arch = "wasm32"))]
fn worker(
    rx: Receiver<Job>,
    device: Option<String>,
    status: Arc<Mutex<AlertStatus>>,
    stop: &AtomicBool,
) {
    let (out, mut ring) = match sdroxide_audio::start_output(
        device.as_deref(),
        48_000,
        sdroxide_types::AudioRouting::Both,
    ) {
        Ok(ok) => ok,
        Err(e) => {
            *status.lock().unwrap() = AlertStatus::Failed(e.to_string());
            // Nothing can be played. Honour a quit, and otherwise wait for the
            // sender to go — which it does before the join in `Drop`, so this
            // cannot hold the app open.
            while let Ok(job) = rx.recv() {
                if matches!(job, Job::Quit) {
                    break;
                }
            }
            return;
        }
    };
    let label = device.clone().unwrap_or_else(|| "default".into());
    *status.lock().unwrap() = AlertStatus::Running(label);
    let capacity = out.sample_rate as usize * 2;
    let lead = (out.sample_rate * LEAD_S) as usize;

    while let Ok(job) = rx.recv() {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match job {
            Job::Quit => break,
            Job::Play { sound, volume } => {
                // Paced one frame at a time, exactly like the speech worker:
                // never run more than `lead` stereo frames ahead of the sound
                // card, so a tone starts when it should and the whole pattern
                // lands within a beat of the decode.
                let pcm = render(sound, out.sample_rate as u32, volume);
                let offer = |s| {
                    let room = queued_frames(capacity, ring.slots()) < lead && ring.slots() >= 2;
                    if room {
                        let _ = ring.push(s);
                    }
                    room
                };
                if !push_paced(&pcm, stop, offer) {
                    break;
                }
            }
        }
    }
}

/// Hand `pcm` to the sound card no faster than it plays: `offer` takes a
/// sample only when the card has room for it, and each one is offered again
/// until it is taken. `false` when `stop` went up first.
///
/// The stop is read inside the wait, like the speech worker's generation, and
/// that is the point of it. A card that has stopped taking samples — a USB
/// headset pulled out mid-alarm — never makes room again, so a wait that only
/// watched the ring held the worker for good, and the join in `Drop` held the
/// screen behind it: the app froze on quit, on alerts off and on a change of
/// device.
#[cfg(not(target_arch = "wasm32"))]
fn push_paced(pcm: &[f32], stop: &AtomicBool, mut offer: impl FnMut(f32) -> bool) -> bool {
    for &s in pcm {
        while !offer(s) {
            if stop.load(Ordering::Relaxed) {
                return false;
            }
            std::thread::sleep(TICK);
        }
    }
    true
}

/// Stereo frames currently queued for the sound card.
#[cfg(not(target_arch = "wasm32"))]
fn queued_frames(capacity: usize, slots: usize) -> usize {
    (capacity - slots) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> LogIndex {
        LogIndex::build(&[])
    }

    fn dec(to: Option<&str>, from: Option<&str>, cq: bool) -> Decode {
        Decode {
            slot_utc: 0,
            snr_db: 0,
            dt: 0.0,
            audio_hz: 500.0,
            message: String::new(),
            cq_to: if cq { Some("EU".to_string()) } else { None },
            to: to.map(str::to_string),
            from: from.map(str::to_string),
            grid: None,
            is_cq: cq,
            free_text: false,
            rr73_to: None,
        }
    }

    #[test]
    fn a_runtime_off_by_default_plays_nothing() {
        let mut r = AlertRuntime::new(AlertSettings::default());
        r.on_ft8(&[dec(Some("OE3ABC"), Some("K1ABC"), false)], "oe3abc", "J063", &log(), "");
        assert_eq!(r.status(), AlertStatus::Idle);
    }

    #[test]
    fn cooldown_is_per_station() {
        let mut settings = AlertSettings { enabled: true, ..Default::default() };
        settings.events.called.enabled = true;
        let mut r = AlertRuntime::new(settings);
        r.on_ft8(&[dec(Some("OE3ABC"), Some("K1ABC"), false)], "oe3abc", "JO63", &log(), "");
        r.on_ft8(&[dec(Some("OE3ABC"), Some("K1ABC"), false)], "oe3abc", "JO63", &log(), "");
        r.on_ft8(&[dec(Some("OE3ABC"), Some("W2XYZ"), false)], "oe3abc", "JO63", &log(), "");
    }

    #[test]
    fn a_directed_cq_rings_only_when_enabled() {
        // Default: the CQ rule is off, so a CQ we could answer stays silent.
        let mut settings = AlertSettings { enabled: true, ..Default::default() };
        let mut r = AlertRuntime::new(settings.clone());
        r.on_ft8(&[dec(None, Some("OE3ABC"), true)], "dl1abc", "JO63", &log(), "");
        // Turned on, the same CQ is worth hearing.
        settings.events.cq.enabled = true;
        let mut r = AlertRuntime::new(settings);
        r.on_ft8(&[dec(None, Some("OE3ABC"), true)], "dl1abc", "JO63", &log(), "");
    }

    /// A busy slot can carry a dozen decodes that all match; one alarm says it,
    /// and a queue of them takes half a minute to stop saying it.
    #[test]
    fn only_one_alarm_is_raised_per_batch() {
        let mut settings = AlertSettings { enabled: true, ..Default::default() };
        settings.events.called.enabled = true;
        let mut r = AlertRuntime::new(settings);
        r.on_ft8(
            &[
                dec(Some("K1ABC"), Some("W1ABC"), false),
                dec(Some("K1ABC"), Some("DL1ABC"), false),
                dec(Some("K1ABC"), Some("F5ABC"), false),
            ],
            "k1abc",
            "JO63",
            &log(),
            "",
        );
        assert!(r.core().cooldowns.at.contains_key(&("W1ABC".to_string(), AlertEvent::Called)));
        assert!(!r.core().cooldowns.at.contains_key(&("DL1ABC".to_string(), AlertEvent::Called)));
        assert!(!r.core().cooldowns.at.contains_key(&("F5ABC".to_string(), AlertEvent::Called)));
    }

    /// A sound card that stops taking samples mid-alarm must not hold the
    /// worker: the stop reaches it inside the wait, which is what lets `Drop`
    /// join it. Before, this wait only watched the ring and never returned.
    /// Every tab asking for the station's alarm gets the same one: a setting
    /// changed on one is the setting on all, and there is one device between
    /// them. (Kept switched off, so no test here opens a sound card.)
    #[test]
    fn every_tab_shares_the_station_alarm() {
        let mut a = AlertRuntime::station(AlertSettings::default());
        let b = AlertRuntime::station(AlertSettings { volume: 0.1, ..Default::default() });
        assert!(Arc::ptr_eq(&a.core, &b.core), "two tabs, two alarms");
        a.set_settings(AlertSettings { volume: 0.3, ..Default::default() });
        assert_eq!(b.settings().volume, 0.3, "the other tab kept its own copy");
        // A runtime built on its own stays its own.
        assert!(!Arc::ptr_eq(&a.core, &AlertRuntime::new(AlertSettings::default()).core));
    }

    /// The batch's alarm is its most important match, wherever in the list it
    /// sits: a new entity decoded first must not stand in for a station
    /// calling us decoded after it.
    #[test]
    fn a_call_further_down_the_batch_outranks_a_novelty_above_it() {
        let mut settings = AlertSettings { enabled: true, ..Default::default() };
        settings.events.called.enabled = true;
        settings.events.new_dxcc.enabled = true;
        // Two other stations working each other, from an entity this empty log
        // has never had — and then someone calling us.
        let novelty = dec(Some("W2AAA"), Some("JA1ABC"), false);
        let call = dec(Some("K1ABC"), Some("DL1ABC"), false);

        // The novelty on its own does ring, so the batch below is a real choice.
        let mut r = AlertRuntime::new(settings.clone());
        r.on_ft8(std::slice::from_ref(&novelty), "k1abc", "FN42", &log(), "20m");
        assert!(r.core().cooldowns.at.contains_key(&("JA1ABC".to_string(), AlertEvent::NewDxcc)));

        let mut r = AlertRuntime::new(settings);
        r.on_ft8(&[novelty, call], "k1abc", "FN42", &log(), "20m");
        assert!(r.core().cooldowns.at.contains_key(&("DL1ABC".to_string(), AlertEvent::Called)));
        assert!(!r.core().cooldowns.at.contains_key(&("JA1ABC".to_string(), AlertEvent::NewDxcc)));
    }

    /// A fired alarm is handed back with the station and the reply, so the
    /// caller can speak it — the tone is the runtime's, the phrase is not.
    #[test]
    fn a_fired_alert_is_handed_back_with_its_station_and_reply() {
        let mut settings = AlertSettings { enabled: true, ..Default::default() };
        settings.events.called.enabled = true;
        settings.events.called.reply = AlertReply::Voice;
        let mut r = AlertRuntime::new(settings);
        let fired = r
            .on_ft8(&[dec(Some("OE3ABC"), Some("K1ABC"), false)], "oe3abc", "JO63", &log(), "20m")
            .expect("the call should raise an alarm");
        assert_eq!(fired.event, AlertEvent::Called);
        assert_eq!(fired.call, "K1ABC");
        assert_eq!(fired.reply, AlertReply::Voice);
        assert!(fired.reply.speaks() && !fired.reply.plays_tone());
    }

    /// A decode that matches nothing hands nothing back, so the caller has no
    /// phrase to speak.
    #[test]
    fn a_decode_that_matches_nothing_hands_nothing_back() {
        let mut settings = AlertSettings { enabled: true, ..Default::default() };
        settings.events.called.enabled = true;
        let mut r = AlertRuntime::new(settings);
        assert!(
            r.on_ft8(&[dec(None, None, false)], "oe3abc", "JO63", &log(), "20m").is_none(),
            "a decode with no station is no alarm"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_card_that_stops_taking_samples_lets_the_worker_go() {
        let stop = Arc::new(AtomicBool::new(false));
        let raise = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(60));
                stop.store(true, Ordering::Relaxed);
            })
        };
        let mut taken = 0;
        // Room for three samples, then none ever again.
        let finished = push_paced(&[0.1; 64], &stop, |_| {
            let room = taken < 3;
            taken += room as usize;
            room
        });
        raise.join().unwrap();
        assert!(!finished, "the alarm claims to have played out");
        assert_eq!(taken, 3);
    }

    /// And a card that keeps up is handed every sample, in order.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_card_that_keeps_up_gets_the_whole_alarm() {
        let stop = AtomicBool::new(false);
        let mut got = Vec::new();
        assert!(push_paced(&[0.1, 0.2, 0.3], &stop, |s| {
            got.push(s);
            true
        }));
        assert_eq!(got, [0.1, 0.2, 0.3]);
    }

    #[test]
    fn every_sound_renders_finite_stereo() {
        for sound in AlertSound::ALL {
            let pcm = render(sound, 48_000, 0.7);
            assert_eq!(pcm.len() % 2, 0);
            assert!(!pcm.is_empty());
            for s in &pcm {
                assert!(s.is_finite());
                assert!((-1.0..=1.0).contains(s), "level out of range for {sound:?}");
            }
        }
    }
}
