//! The speaking half: a worker thread, a sound card, and the model.
//!
//! # Its own output stream
//!
//! Speech does **not** go through the radio's mixer. Anything summed in there
//! reaches the MP3 recorder and every remote listener of the station, and an
//! announcement belongs to the operator sitting at this screen. A second cpal
//! stream also lets speech go to a different device from the receiver —
//! announcements in the room, the band in the headphones.
//!
//! # Why the lead is short
//!
//! The worker never pushes more than [`LEAD_S`] of audio into the ring at a
//! time. Barge-in is the reason: it cannot retract what the sound card has
//! already been handed, so the amount handed over is what a stop costs. Eighty
//! milliseconds is short enough to be inaudible as a tail and long enough that
//! a thread waking every twenty cannot underrun.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use sdroxide_dsp::MonoResampler;

use super::{Phonemes, Synth, SynthError, Voice, find_voice};
use crate::sink::SpeechSink;

/// How far ahead of the sound card to run. See the module note.
const LEAD_S: f64 = 0.08;

/// Worker poll interval while pacing audio into the ring.
const TICK: Duration = Duration::from_millis(20);

/// How long [`SpeechEngine::drop`] waits for the worker to wind down. Long
/// enough for a tidy stop in the normal case — the worker answers the stop
/// counter inside a tick — short enough that a worker wedged inside a blocking
/// device open cannot hold the app's exit on an audio driver.
const CLOSE_GRACE: Duration = Duration::from_millis(1500);

/// Longest chunk handed to the model at once. VITS synthesizes a whole
/// utterance in one pass, so chunking gets the first word out sooner, bounds
/// peak memory, and makes barge-in cheap.
const CHUNK_CHARS: usize = 120;

/// What the speech backend is doing, for the settings dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechStatus {
    /// Reading the voice model and the dictionary.
    Loading,
    /// Ready, with the voice's name.
    Ready(String),
    /// Not usable, and why. Reported rather than panicked: a missing voice
    /// file has to leave the radio working.
    Failed(String),
}

#[derive(Debug)]
enum Job {
    Speak { text: String, seq: u64 },
    Quit,
}

#[derive(Debug, Default)]
struct Shared {
    /// Audio for a `speak` is still being generated or played.
    busy: AtomicBool,
    /// Generation of the last utterance that finished, or was stopped.
    done_seq: AtomicU64,
    /// Bumped by `stop`; the worker abandons anything older.
    stop_seq: AtomicU64,
    /// Live speech rate and volume, so a settings change takes effect on the
    /// next utterance without restarting the worker.
    rate_milli: AtomicU64,
    volume_milli: AtomicU64,
}

/// A running speech backend.
pub struct SpeechEngine {
    /// `None` once dropped: taking it closes the channel the worker reads from.
    tx: Option<Sender<Job>>,
    shared: Arc<Shared>,
    status: Arc<Mutex<SpeechStatus>>,
    thread: Option<JoinHandle<()>>,
}

impl SpeechEngine {
    /// Start the worker. Never fails: the model is loaded on the worker thread
    /// and any problem shows up through [`SpeechEngine::status`], so switching
    /// speech on cannot stall the UI for the length of a 60 MB mmap and a
    /// 126 000-word dictionary parse.
    pub fn start(
        dirs: Vec<std::path::PathBuf>,
        voice: String,
        device: Option<String>,
        rate: f32,
        volume: f32,
    ) -> Self {
        // Bounded: if the operator somehow queues faster than the worker
        // speaks, dropping is the same answer the announcer's own queue gives.
        let (tx, rx) = crossbeam_channel::bounded::<Job>(16);
        let shared = Arc::new(Shared::default());
        shared.rate_milli.store((rate.clamp(0.25, 4.0) * 1000.0) as u64, Ordering::Relaxed);
        shared.volume_milli.store((volume.clamp(0.0, 1.0) * 1000.0) as u64, Ordering::Relaxed);
        let status = Arc::new(Mutex::new(SpeechStatus::Loading));

        let thread = {
            let shared = shared.clone();
            let status = status.clone();
            std::thread::Builder::new()
                .name("speech".into())
                .spawn(move || worker(rx, shared, status, dirs, voice, device))
                .ok()
        };
        if thread.is_none() {
            *status.lock().unwrap() =
                SpeechStatus::Failed("could not start the speech thread".into());
        }
        SpeechEngine { tx: Some(tx), shared, status, thread }
    }

    pub fn status(&self) -> SpeechStatus {
        self.status.lock().unwrap().clone()
    }

    /// A cheap, cloneable view of the same worker.
    ///
    /// The engine itself has to be *owned* by whoever speaks through it — the
    /// `SpeechSink` methods take `&mut self` — so a caller that also wants to
    /// read the status, or nudge the rate, takes one of these instead of
    /// wrapping the whole thing in a lock.
    pub fn handle(&self) -> SpeechHandle {
        SpeechHandle { shared: self.shared.clone(), status: self.status.clone() }
    }
}

/// A read-and-nudge view of a running [`SpeechEngine`].
#[derive(Clone)]
pub struct SpeechHandle {
    shared: Arc<Shared>,
    status: Arc<Mutex<SpeechStatus>>,
}

impl SpeechHandle {
    pub fn status(&self) -> SpeechStatus {
        self.status.lock().unwrap().clone()
    }

    /// Update the rate and volume without restarting. Takes effect on the next
    /// utterance — the model bakes tempo into the audio it generates, so a
    /// change mid-sentence is not possible and would be jarring anyway.
    pub fn set_levels(&self, rate: f32, volume: f32) {
        self.shared.rate_milli.store((rate.clamp(0.25, 4.0) * 1000.0) as u64, Ordering::Relaxed);
        self.shared.volume_milli.store((volume.clamp(0.0, 1.0) * 1000.0) as u64, Ordering::Relaxed);
    }
}

impl Drop for SpeechEngine {
    fn drop(&mut self) {
        // Close the job channel *before* joining. `recv` then answers `Err`
        // once the queue drains, so a worker that is blocked waiting, speaking,
        // or stuck inside a device open while it builds its backend has a way
        // out and the join cannot wait on a thread that is waiting on this one
        // to release the channel. The `Quit` is a best-effort head start; the
        // disconnect is what guarantees the exit.
        self.shared.stop_seq.fetch_add(1, Ordering::SeqCst);
        if let Some(tx) = self.tx.as_ref() {
            let _ = tx.try_send(Job::Quit);
        }
        self.tx.take();
        if let Some(t) = self.thread.take() {
            // Bounded: a worker wedged opening a contended sound device has
            // nothing to be woken by, so a plain `join` could hold the app's
            // close on the audio driver's timetable. Give it a grace period,
            // then leave it to fall off with the process.
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = t.join();
                let _ = done_tx.send(());
            });
            let _ = done_rx.recv_timeout(CLOSE_GRACE);
        }
    }
}

impl SpeechSink for SpeechEngine {
    fn speak(&mut self, text: &str, seq: u64) {
        self.shared.busy.store(true, Ordering::SeqCst);
        // A send that finds the queue full or the channel closed (the engine is
        // shutting down) must not leave the announcer waiting on a `busy` that
        // never clears.
        let queued = self.tx.as_ref().is_some_and(|tx| {
            tx.try_send(Job::Speak { text: text.to_string(), seq }).is_ok()
        });
        if !queued {
            self.shared.busy.store(false, Ordering::SeqCst);
            self.shared.done_seq.store(seq, Ordering::SeqCst);
        }
    }

    fn stop(&mut self) {
        self.shared.stop_seq.fetch_add(1, Ordering::SeqCst);
    }

    fn busy(&self) -> bool {
        self.shared.busy.load(Ordering::SeqCst)
    }

    fn done_seq(&self) -> u64 {
        self.shared.done_seq.load(Ordering::SeqCst)
    }
}

/// Everything the worker needs, once it has managed to build it.
struct Backend {
    synth: Synth,
    g2p: Phonemes,
    /// Owns the cpal stream. Never read — but dropping it closes the stream,
    /// so it has to outlive the ring the callback is reading from.
    _out: sdroxide_audio::AudioOutput,
    ring: rtrb::Producer<f32>,
    resampler: Option<MonoResampler>,
    /// Interleaved stereo frames the ring holds when full.
    capacity: usize,
    /// Frames to keep queued ahead of the sound card.
    lead_frames: usize,
}

fn worker(
    rx: Receiver<Job>,
    shared: Arc<Shared>,
    status: Arc<Mutex<SpeechStatus>>,
    dirs: Vec<std::path::PathBuf>,
    voice: String,
    device: Option<String>,
) {
    let mut backend = match build(&dirs, &voice, device.as_deref()) {
        Ok(b) => {
            *status.lock().unwrap() = SpeechStatus::Ready(b.synth.voice().name.clone());
            b
        }
        Err(e) => {
            tracing::warn!(error = %e, "speech is unavailable");
            *status.lock().unwrap() = SpeechStatus::Failed(e.to_string());
            // Drain and discard, so a caller that keeps speaking does not block
            // on a full channel or wait forever on `busy`.
            while let Ok(job) = rx.recv() {
                if let Job::Speak { seq, .. } = job {
                    shared.done_seq.store(seq, Ordering::SeqCst);
                }
                shared.busy.store(false, Ordering::SeqCst);
                if matches!(job, Job::Quit) {
                    return;
                }
            }
            return;
        }
    };

    while let Ok(job) = rx.recv() {
        let Job::Speak { text, seq } = job else { return };
        let generation = shared.stop_seq.load(Ordering::SeqCst);
        speak_one(&mut backend, &shared, &text, generation);
        shared.done_seq.store(seq, Ordering::SeqCst);
        // Only clear `busy` if nothing else is already waiting: otherwise the
        // announcer would see an idle sink between two queued utterances and
        // hand over a third.
        if rx.is_empty() {
            shared.busy.store(false, Ordering::SeqCst);
        }
    }
}

fn build(
    dirs: &[std::path::PathBuf],
    voice: &str,
    device: Option<&str>,
) -> Result<Backend, SynthError> {
    let path = find_voice(dirs, voice)?;
    let synth = Synth::load(Voice::load(path)?)?;
    let g2p = Phonemes::new()?;
    let (out, ring) = sdroxide_audio::start_output(device, 48_000)
        .map_err(|e| SynthError::Inference(format!("opening the speech output: {e}")))?;

    let model_rate = synth.sample_rate() as f64;
    let resampler = MonoResampler::new(model_rate, out.sample_rate);
    // `start_output` sizes the ring at one second of interleaved stereo.
    let capacity = out.sample_rate as usize * 2;
    let lead_frames = (out.sample_rate * LEAD_S) as usize;
    tracing::info!(
        voice = %synth.voice().name,
        model_rate,
        out_rate = out.sample_rate,
        channels = out.channels,
        "speech output running"
    );
    Ok(Backend { synth, g2p, _out: out, ring, resampler, capacity, lead_frames })
}

/// Synthesize and play one utterance, abandoning it if `stop` is called.
fn speak_one(b: &mut Backend, shared: &Shared, text: &str, generation: u64) {
    let rate = shared.rate_milli.load(Ordering::Relaxed) as f32 / 1000.0;
    let volume = shared.volume_milli.load(Ordering::Relaxed) as f32 / 1000.0;

    for chunk in chunks(text, CHUNK_CHARS) {
        if shared.stop_seq.load(Ordering::SeqCst) != generation {
            return;
        }
        let phonemes = match b.g2p.to_ipa(&chunk) {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!(error = %e, chunk, "could not phonemize");
                continue;
            }
        };
        let pcm = match b.synth.speak_phonemes(&phonemes, rate) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "synthesis failed");
                continue;
            }
        };
        if pcm.is_empty() {
            continue;
        }

        let mut resampled = Vec::with_capacity(pcm.len());
        match b.resampler.as_mut() {
            Some(r) => r.push(&pcm, &mut resampled),
            None => resampled.extend_from_slice(&pcm),
        }
        if !push_paced(b, shared, &resampled, volume, generation) {
            return;
        }
    }

    // Wait for the sound card to actually finish what it was handed, so `busy`
    // does not clear while the last word is still coming out.
    while queued_frames(b) > 0 {
        if shared.stop_seq.load(Ordering::SeqCst) != generation {
            return;
        }
        std::thread::sleep(TICK);
    }
}

/// Push mono samples as interleaved stereo, never running more than the lead
/// ahead of the sound card. `false` if a stop cut it short.
fn push_paced(b: &mut Backend, shared: &Shared, pcm: &[f32], volume: f32, generation: u64) -> bool {
    for s in pcm {
        loop {
            if shared.stop_seq.load(Ordering::SeqCst) != generation {
                return false;
            }
            if queued_frames(b) < b.lead_frames && b.ring.slots() >= 2 {
                break;
            }
            std::thread::sleep(TICK);
        }
        let v = (s * volume).clamp(-1.0, 1.0);
        // Both ears: the radio's own audio uses the right channel for the sub
        // receiver, but speech has no such split and centring it is what makes
        // it sit "in front of" whatever is being received.
        let _ = b.ring.push(v);
        let _ = b.ring.push(v);
    }
    true
}

/// Stereo frames currently queued for the sound card.
fn queued_frames(b: &Backend) -> usize {
    (b.capacity - b.ring.slots()) / 2
}

/// Split text into chunks of at most `max` characters, preferring sentence and
/// clause boundaries so the voice's phrasing survives the cut.
fn chunks(text: &str, max: usize) -> Vec<String> {
    let text = text.trim();
    if text.chars().count() <= max {
        return if text.is_empty() { Vec::new() } else { vec![text.to_string()] };
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let would_be = cur.chars().count() + 1 + word.chars().count();
        if !cur.is_empty() && would_be > max {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
        // A clause boundary is a good place to stop even below the limit.
        if cur.chars().count() >= max / 2 && word.ends_with(['.', ',', '?', '!']) {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_one_chunk() {
        assert_eq!(chunks("twenty meters", 120), vec!["twenty meters"]);
        assert!(chunks("   ", 120).is_empty());
    }

    #[test]
    fn long_text_splits_on_word_boundaries() {
        let text = "word ".repeat(100);
        let cs = chunks(&text, 40);
        assert!(cs.len() > 1);
        for c in &cs {
            assert!(c.chars().count() <= 40, "chunk too long: {c:?}");
            assert!(!c.starts_with(' ') && !c.ends_with(' '), "ragged chunk: {c:?}");
        }
        // Nothing may be lost in the split.
        assert_eq!(cs.join(" ").split_whitespace().count(), 100);
    }

    #[test]
    fn clauses_are_preferred_break_points() {
        let cs = chunks(
            "forty meters, seven point one zero zero, lower sideband, split on transmit",
            40,
        );
        assert!(cs[0].ends_with(','), "did not break at a clause: {cs:?}");
    }
}
