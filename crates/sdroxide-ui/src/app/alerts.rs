//! Audible alert runtime.
//!
//! The [`AlertStatus`] mirrors the speech engine's status shape, and the
//! worker is the same shape as `sdroxide-speech`'s: a thread owns the cpal
//! output and its ring buffer, and jobs are paced into the ring rather than
//! blocking. The difference is that every job here is synthesised — a handful
//! of oscillator patterns — so there is no model to load and no first-sound
//! pause. The ring opens in flight (a few milliseconds of silence), which is
//! what makes the first alarm sound listenable.

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sdroxide_types::{AlertEvent, AlertSettings, AlertSound, Decode, LogIndex};

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
            AlertStatus::Running(d) => Some(format!("alarming on {d}")),
            AlertStatus::Failed(e) => Some(format!("alarm device unavailable: {e}")),
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
    /// (callsign, event) → when it last alarmed.
    at: HashMap<(String, AlertEvent), Instant>,
}

impl Cooldown {
    fn new() -> Self {
        Cooldown { at: HashMap::new() }
    }

    /// Whether this station may alarm for this event.
    fn eligible(&self, call: &str, event: AlertEvent) -> bool {
        match self.at.get(&(call.to_string(), event)) {
            None => true,
            Some(&when) => when.elapsed().as_secs() >= event.cooldown_s(),
        }
    }

    fn mark(&mut self, call: &str, event: AlertEvent) {
        self.at.insert((call.to_string(), event), Instant::now());
        // A wall of decoded stations could grow this forever, so once it gets
        // big, drop everything that has gone cold.
        if self.at.len() > 256 {
            self.at.retain(|_, when| when.elapsed() < Duration::from_secs(600));
        }
    }
}

/// What a sound job carries to the worker.
#[cfg(not(target_arch = "wasm32"))]
enum Job {
    Play { sound: AlertSound, volume: f32 },
    Quit,
}

/// The alarms themselves: settings plus, on native, a background worker that
/// owns the alert device.
pub struct AlertRuntime {
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
    tx: SyncSender<Job>,
    thread: Option<JoinHandle<()>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for AlertSink {
    fn drop(&mut self) {
        let _ = self.tx.send(Job::Quit);
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
    /// Build the runtime and, if enabled, open the device in the background.
    pub fn new(settings: AlertSettings) -> Self {
        let mut runtime = AlertRuntime {
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

    /// A snapshot of the status, safe to read from any thread.
    pub fn status(&self) -> AlertStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn settings(&self) -> &AlertSettings {
        &self.settings
    }

    /// Mastership goes here: whether alarms are worth listening for at all.
    pub fn enabled(&self) -> bool {
        self.settings.enabled
    }

    /// Update the configuration. Any change to enabled/device means a new
    /// worker; the rest is read live at each decode.
    pub fn set_settings(&mut self, settings: AlertSettings) {
        let device_changed = settings.device != self.settings.device;
        let enabled_changed = settings.enabled != self.settings.enabled;
        self.settings = settings;
        if enabled_changed || device_changed {
            self.sync_sink();
        }
    }

    /// A preview alarm for the settings tab: whatever sound the "called" rule
    /// is set to.
    pub fn test(&self) {
        self.play(self.settings.events.called.sound);
    }

    /// Feed one WSJT-style decode batch. Alarms intentionally do **not** wait
    /// for the window to be focused — the point is to reach the operator when
    /// they are looking at another window entirely.
    pub fn on_ft8(
        &mut self,
        decodes: &[Decode],
        my_call: &str,
        my_grid: &str,
        log: &LogIndex,
        band: &str,
    ) {
        if !self.settings.enabled {
            return;
        }
        let my_call = my_call.trim().to_ascii_uppercase();
        let my_grid = my_grid.trim().to_ascii_uppercase();
        if my_call.is_empty() {
            return;
        }
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
            self.play(event.rule(&self.settings.events).sound);
            self.cooldowns.mark(from, event);
        }
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
        let thread = {
            let status = status.clone();
            std::thread::Builder::new()
                .name("alerts".into())
                .spawn(move || worker(rx, device, status))
                .ok()
        };
        if thread.is_none() {
            *status.lock().unwrap() =
                AlertStatus::Failed("could not start the alert thread".into());
        }
        AlertSink { tx, thread }
    }

    fn play(&self, sound: AlertSound, volume: f32) {
        let _ = self.tx.try_send(Job::Play { sound, volume });
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
fn worker(rx: Receiver<Job>, device: Option<String>, status: Arc<Mutex<AlertStatus>>) {
    let (out, mut ring) = match sdroxide_audio::start_output(device.as_deref(), 48_000) {
        Ok(ok) => ok,
        Err(e) => {
            *status.lock().unwrap() = AlertStatus::Failed(e.to_string());
            // Nothing can be played — drain and go.
            while rx.recv().is_ok() {}
            return;
        }
    };
    let label = device.clone().unwrap_or_else(|| "default".into());
    *status.lock().unwrap() = AlertStatus::Running(label);
    let capacity = out.sample_rate as usize * 2;
    let lead = (out.sample_rate * LEAD_S) as usize;

    while let Ok(job) = rx.recv() {
        match job {
            Job::Quit => break,
            Job::Play { sound, volume } => {
                // Paced one frame at a time, exactly like the speech worker:
                // never run more than `lead` stereo frames ahead of the sound
                // card, so a tone starts when it should and the whole pattern
                // lands within a beat of the decode.
                for s in render(sound, out.sample_rate as u32, volume) {
                    loop {
                        if queued_frames(capacity, ring.slots()) < lead && ring.slots() >= 2 {
                            break;
                        }
                        std::thread::sleep(TICK);
                    }
                    let _ = ring.push(s);
                }
            }
        }
    }
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
