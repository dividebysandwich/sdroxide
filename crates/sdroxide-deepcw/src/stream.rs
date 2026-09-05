//! Turning a window decoder into a running transcript.
//!
//! The model reads a whole window and returns the whole transcript for it, so
//! re-decoding as audio arrives means the text keeps changing behind the
//! operator's eyes — the tail especially, where the last character is usually
//! still half-sent. What makes it readable is committing the part that has
//! stopped moving.
//!
//! The cut is made at a word gap, and only at one far enough from the end that
//! more audio cannot reasonably revise it. Word gaps are the right seam because
//! CTC is most confident there and because a word is the unit the reader is
//! already tracking; cutting mid-character would commit exactly the decision the
//! next second of audio is most likely to overturn.
//!
//! Everything before the cut is emitted once and never revisited. Everything
//! after it stays in the buffer, gets decoded again next time, and is offered
//! separately as a preview.
//!
//! Committed audio is not thrown away immediately: a second of it is kept in
//! front of the buffer as run-up. A window that begins abruptly at a cut costs
//! the model the context it reads characters from, and it answers with a
//! spurious character at the seam — usually an `E`, the shortest thing it can
//! say. Giving it the second before the cut and then discarding whatever it
//! decodes there removes the artifact without emitting anything twice.

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};

use crate::ctc::Decoded;
use crate::model::{Decoder, Error};
use crate::spectrogram::{FRAME_S, SAMPLE_RATE, frame_to_sample};

/// Longest window handed to the model. The reference implementation documents
/// 5–20 s; the top of that range is where we sit, because the model reads the
/// whole window as context and more of it copies better through a fade.
const MAX_WINDOW_S: f64 = 20.0;
/// Don't decode until there is at least this much to decode.
const MIN_WINDOW_S: f64 = 2.0;
/// Never commit inside this much of the end. The last second is where the
/// transcript is still moving.
const TAIL_GUARD_S: f64 = 1.25;
/// Never commit less than this, so the committed text does not arrive one
/// character at a time.
const MIN_COMMIT_S: f64 = 2.0;
/// New audio needed before re-decoding. Bounds inference to roughly one run per
/// second of audio no matter how small the blocks arriving are.
const ANALYSIS_INTERVAL_S: f64 = 1.0;
/// Already-committed audio kept in front of the buffer as run-up, so no window
/// ever starts on a hard cut. Whatever the model decodes here is discarded.
const LEAD_S: f64 = 1.0;

fn samples(seconds: f64) -> usize {
    (seconds * SAMPLE_RATE) as usize
}

/// What one decode round produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Update {
    /// Text that will not change again. Empty when nothing settled this round.
    pub committed: String,
    /// The unsettled tail. Replaced wholesale by the next update, so display it
    /// after `committed` rather than appending it to anything.
    pub pending: String,
}

impl Update {
    pub fn is_empty(&self) -> bool {
        self.committed.is_empty() && self.pending.is_empty()
    }
}

/// A rolling transcript over a growing audio buffer.
///
/// Synchronous and self-contained: [`Stream::push`] is cheap, [`Stream::poll`]
/// is where inference happens. Drive it from a thread — see [`Worker`].
pub struct Stream {
    decoder: Decoder,
    pending: Vec<f32>,
    /// Samples at the front of `pending` that were already committed. They are
    /// decoded again every round for context and their text is thrown away.
    lead: usize,
    /// Samples pushed since the last decode, against `ANALYSIS_INTERVAL_S`.
    since_analysis: usize,
    analyzed: bool,
}

impl Stream {
    pub fn new() -> Result<Self, Error> {
        Ok(Stream {
            decoder: Decoder::new()?,
            pending: Vec::new(),
            lead: 0,
            since_analysis: 0,
            analyzed: false,
        })
    }

    /// First frame of the window that is not run-up.
    fn lead_frame(&self) -> usize {
        sample_to_frame(self.lead)
    }

    /// Feed audio at [`SAMPLE_RATE`], mono, signal inside 400–1200 Hz.
    pub fn push(&mut self, audio: &[f32]) {
        self.pending.extend_from_slice(audio);
        self.since_analysis += audio.len();

        // A decode drains at least `MIN_COMMIT_S` and costs tens of
        // milliseconds, so the buffer cannot grow in normal use. This is the
        // backstop for the case where it does anyway — a machine so loaded that
        // inference stops keeping up. Losing the oldest audio keeps the recent
        // timeline contiguous, which is what the decoder is about to read.
        let cap = samples(MAX_WINDOW_S) * 2;
        if self.pending.len() > cap {
            let excess = self.pending.len() - cap;
            tracing::warn!(
                seconds = excess as f64 / SAMPLE_RATE,
                "DeepCW is behind, dropping audio"
            );
            self.pending.drain(..excess);
            self.lead = self.lead.saturating_sub(excess);
        }
    }

    /// Decode whatever is buffered and commit all of it, however little there
    /// is. For the end of a transmission or a mode change, where the last words
    /// would otherwise sit in the tail forever waiting for a gap that never
    /// comes.
    pub fn flush(&mut self) -> Option<Result<Update, Error>> {
        if self.pending.len() < crate::spectrogram::MIN_SAMPLES {
            self.reset();
            return None;
        }
        // More than one window can be waiting, so work through the buffer a
        // window at a time rather than decoding the first one and dropping the
        // rest on the floor.
        let mut committed = String::new();
        while self.pending.len() >= crate::spectrogram::MIN_SAMPLES {
            let window = self.pending.len().min(samples(MAX_WINDOW_S));
            match self.decoder.decode(&self.pending[..window]) {
                Ok(decoded) => {
                    let text = decoded.slice(self.lead_frame(), usize::MAX).normalized();
                    if !text.is_empty() {
                        if !committed.is_empty() {
                            committed.push(' ');
                        }
                        committed.push_str(&text);
                    }
                }
                Err(e) => {
                    self.reset();
                    return Some(Err(e));
                }
            }
            // Each further pass keeps the tail of this window as its own run-up.
            self.pending.drain(..window);
            self.lead = 0;
        }
        self.reset();
        (!committed.is_empty()).then(|| Ok(Update { committed, pending: String::new() }))
    }

    /// Forget everything buffered. The committed text already handed out stays
    /// with the caller; this only resets what is still being decoded.
    pub fn reset(&mut self) {
        self.pending.clear();
        self.lead = 0;
        self.since_analysis = 0;
        self.analyzed = false;
    }

    /// Seconds of not-yet-committed audio. Excludes the run-up.
    pub fn buffered_s(&self) -> f64 {
        (self.pending.len() - self.lead) as f64 / SAMPLE_RATE
    }

    /// Decode if enough new audio has arrived, and return what changed.
    ///
    /// `None` means nothing was decoded this call — not that nothing was heard.
    pub fn poll(&mut self) -> Option<Result<Update, Error>> {
        if self.pending.len() - self.lead < samples(MIN_WINDOW_S) {
            return None;
        }
        // The first window is decoded as soon as there is one; after that, only
        // once the interval's worth of new audio has landed.
        if self.analyzed && self.since_analysis < samples(ANALYSIS_INTERVAL_S) {
            return None;
        }
        self.since_analysis = 0;
        self.analyzed = true;

        let window = self.pending.len().min(samples(MAX_WINDOW_S));
        let decoded = match self.decoder.decode(&self.pending[..window]) {
            Ok(d) => d,
            Err(e) => return Some(Err(e)),
        };

        let update = self.split(&decoded, window);
        Some(Ok(update))
    }

    /// Split one decode into settled text and a preview, dropping the audio
    /// behind whatever settled.
    fn split(&mut self, decoded: &Decoded, window: usize) -> Update {
        let first = self.lead_frame();
        let pending_text = decoded.slice(first, usize::MAX).normalized();
        // A full window has to give something up, or the buffer never drains and
        // the oldest audio is decoded forever. Past that point we will cut at the
        // last gap anywhere, and failing that at the window edge.
        let full = self.pending.len() >= samples(MAX_WINDOW_S);

        let cut = self
            .commit_point(decoded, window, false)
            .or_else(|| full.then(|| self.commit_point(decoded, window, true)).flatten());

        let (commit_len, commit_frame) = match cut {
            Some((sample, frame)) => (sample, frame),
            None if full => (samples(MAX_WINDOW_S), sample_to_frame(samples(MAX_WINDOW_S))),
            None => return Update { committed: String::new(), pending: pending_text },
        };

        if commit_len.saturating_sub(self.lead) < samples(MIN_COMMIT_S) {
            return Update { committed: String::new(), pending: pending_text };
        }

        let committed = decoded.slice(first, commit_frame).normalized();

        // Keep the last second of what was just committed as run-up for the next
        // window, so it does not begin on a hard cut.
        let keep = samples(LEAD_S).min(commit_len);
        self.pending.drain(..(commit_len - keep).min(self.pending.len()));
        self.lead = keep.min(self.pending.len());

        // The preview is dropped once its text has been committed, so a moment
        // of showing the same words twice cannot happen. The next poll rebuilds
        // it from whatever audio is left.
        Update { committed, pending: String::new() }
    }

    /// The last word gap that is safely inside the window, as
    /// `(sample to cut at, last frame to keep)`.
    fn commit_point(
        &self,
        decoded: &Decoded,
        window: usize,
        to_the_end: bool,
    ) -> Option<(usize, usize)> {
        let latest = if to_the_end {
            window
        } else {
            window.saturating_sub(samples(TAIL_GUARD_S)).max(self.lead + samples(MIN_COMMIT_S))
        };

        decoded.word_gaps.iter().rev().find_map(|gap| {
            // Cut in the middle of the gap: it is the point furthest from either
            // neighbouring character, so neither can be clipped by rounding.
            let mid = gap.start_frame + (gap.end_frame - gap.start_frame) / 2;
            let sample = frame_to_sample(mid);
            (sample >= self.lead + samples(MIN_COMMIT_S) && sample <= latest)
                .then_some((sample, gap.end_frame))
        })
    }
}

fn sample_to_frame(sample: usize) -> usize {
    (sample as f64 / (FRAME_S * SAMPLE_RATE)) as usize
}

// -- threaded ---------------------------------------------------------------

enum Msg {
    Audio(Vec<f32>),
    Flush,
    Reset,
}

/// A [`Stream`] on its own thread.
///
/// Inference takes tens of milliseconds and the callers here are on audio paths
/// that cannot lose that, so it happens elsewhere and updates are collected when
/// they are ready.
///
/// The handoff is lossless on purpose. Dropping a block would splice two
/// non-adjacent pieces of audio together, and the decoder would read the join as
/// a character — a corrupted copy is worse than a late one. Falling behind is
/// not a realistic risk anyway: a decode drains seconds of audio in tens of
/// milliseconds, and [`Stream::push`] caps the buffer if it ever were.
pub struct Worker {
    audio: Sender<Msg>,
    updates: Receiver<Result<Update, Error>>,
    /// Messages handed over and not yet dealt with — see [`Worker::queued`].
    queued: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    _thread: std::thread::JoinHandle<()>,
}

impl Worker {
    pub fn new() -> Result<Self, Error> {
        // Load on the caller's thread so a broken model is an error here rather
        // than a silent thread that never produces anything.
        crate::model::preload()?;

        let (audio_tx, audio_rx) = unbounded::<Msg>();
        let (update_tx, update_rx) = unbounded();

        let queued = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let thread_queued = std::sync::Arc::clone(&queued);
        let thread = std::thread::Builder::new()
            .name("sdroxide-deepcw".into())
            .spawn(move || run(audio_rx, update_tx, &thread_queued))
            .map_err(|e| Error::Load(e.to_string()))?;

        Ok(Worker { audio: audio_tx, updates: update_rx, queued, _thread: thread })
    }

    /// Hand over a block.
    pub fn push(&self, audio: &[f32]) {
        if !audio.is_empty() {
            self.send(Msg::Audio(audio.to_vec()));
        }
    }

    /// Commit the tail without waiting for a word gap. For the end of a
    /// transmission, where the last words would otherwise never settle.
    pub fn flush(&self) {
        self.send(Msg::Flush);
    }

    /// Clear the buffered audio and the tail being decoded.
    pub fn reset(&self) {
        self.send(Msg::Reset);
    }

    /// How much of what has been handed over the worker has not dealt with yet.
    ///
    /// Zero means everything sent before the call has been through the model
    /// and whatever it produced is already on the update channel — so a caller
    /// that has finished pushing audio can tell "the decoder has nothing left
    /// to say" from "the decoder has not got to it yet". Those are
    /// indistinguishable from the outside otherwise, and waiting a fixed time
    /// to tell them apart is a guess that a loaded machine loses: a decode is
    /// tens of milliseconds of arithmetic, but nothing bounds how long this
    /// thread waits to be scheduled.
    pub fn queued(&self) -> usize {
        self.queued.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Count it out, then hand it over. In that order, so the count is never
    /// seen low for a message that is already on its way.
    fn send(&self, msg: Msg) {
        self.queued.fetch_add(1, std::sync::atomic::Ordering::Release);
        if self.audio.send(msg).is_err() {
            // The thread is gone and nothing will ever mark this done.
            self.queued.fetch_sub(1, std::sync::atomic::Ordering::Release);
        }
    }

    /// Anything the worker has finished since the last call.
    pub fn poll(&self) -> Vec<Result<Update, Error>> {
        self.updates.try_iter().collect()
    }
}

fn run(
    audio: Receiver<Msg>,
    updates: Sender<Result<Update, Error>>,
    queued: &std::sync::atomic::AtomicUsize,
) {
    let mut stream = match Stream::new() {
        Ok(s) => s,
        Err(e) => {
            let _ = updates.send(Err(e));
            return;
        }
    };

    while let Ok(first) = audio.recv() {
        let mut flush = false;
        // Taken in, not yet dealt with: the count comes down after the model
        // has run and the update is on the channel, which is what makes zero
        // mean "there is nothing more to come" — see [`Worker::queued`].
        let mut taken = 0usize;
        let mut apply = |msg, flush: &mut bool| match msg {
            Msg::Audio(block) => stream.push(&block),
            Msg::Flush => *flush = true,
            Msg::Reset => stream.reset(),
        };
        apply(first, &mut flush);
        taken += 1;
        // Take everything already waiting before spending 25 ms in the model, so
        // a decode always sees the newest audio rather than replaying a backlog.
        loop {
            match audio.try_recv() {
                Ok(msg) => {
                    apply(msg, &mut flush);
                    taken += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        let update = if flush { stream.flush() } else { stream.poll() };
        let sent = match update {
            Some(update) => updates.send(update).is_ok(),
            None => true,
        };
        queued.fetch_sub(taken, std::sync::atomic::Ordering::Release);
        if !sent {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_signal;

    /// Accumulate a transcript the way a caller would: committed text in order,
    /// with the live tail shown after it.
    #[derive(Default)]
    struct Transcript {
        committed: String,
        pending: String,
    }

    impl Transcript {
        fn apply(&mut self, update: Update) {
            if !update.committed.is_empty() {
                if !self.committed.is_empty() {
                    self.committed.push(' ');
                }
                self.committed.push_str(&update.committed);
            }
            self.pending = update.pending;
        }

        fn text(&self) -> String {
            [self.committed.trim(), self.pending.trim()]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    /// Feed a rendered message through in realistic blocks, then flush the tail.
    fn copy(audio: &[f32], block: usize) -> String {
        let mut stream = Stream::new().expect("stream");
        let mut transcript = Transcript::default();
        for chunk in audio.chunks(block) {
            stream.push(chunk);
            while let Some(update) = stream.poll() {
                transcript.apply(update.expect("decode"));
            }
        }
        if let Some(update) = stream.flush() {
            transcript.apply(update.expect("decode"));
        }
        transcript.text()
    }

    const MSG: &str = "CQ CQ DE W1AW W1AW K UR RST 599 599 QTH BOSTON MA 73";

    #[test]
    fn a_clean_signal_copies_through_the_stream() {
        let audio = test_signal::render(MSG, 25.0, 800.0, 30.0, 1);
        let got = copy(&audio, 512);
        assert!(test_signal::accuracy(MSG, &got) > 0.95, "sent {MSG:?}\n got {got:?}");
    }

    #[test]
    fn text_is_committed_before_the_signal_ends() {
        // A long message must produce committed text along the way, not one
        // block at the end — that is the whole point of the split.
        let audio = test_signal::render(MSG, 20.0, 800.0, 30.0, 2);
        let mut stream = Stream::new().expect("stream");
        let mut commits = 0;
        for chunk in audio.chunks(512) {
            stream.push(chunk);
            while let Some(u) = stream.poll() {
                if !u.expect("decode").committed.is_empty() {
                    commits += 1;
                }
            }
        }
        assert!(commits > 0, "nothing was ever committed");
    }

    #[test]
    fn the_buffer_does_not_grow_without_bound() {
        // Unbroken noise has no word gaps to cut at; the window cap has to drain
        // it anyway or the decoder ends up re-reading an ever-longer buffer.
        let mut stream = Stream::new().expect("stream");
        let mut rng = test_signal::Rng(7);
        let block: Vec<f32> = (0..1600).map(|_| rng.normal() * 0.05).collect();
        for _ in 0..80 {
            stream.push(&block);
            while stream.poll().is_some() {}
        }
        assert!(
            stream.buffered_s() <= MAX_WINDOW_S + 1.0,
            "buffered {:.1}s, cap is {MAX_WINDOW_S}s",
            stream.buffered_s()
        );
    }

    #[test]
    fn nothing_is_decoded_until_there_is_a_window() {
        let mut stream = Stream::new().expect("stream");
        stream.push(&vec![0.0; samples(MIN_WINDOW_S) - 1]);
        assert!(stream.poll().is_none());
        stream.push(&[0.0; 2]);
        assert!(stream.poll().is_some());
    }

    #[test]
    fn reset_drops_the_buffer() {
        let mut stream = Stream::new().expect("stream");
        stream.push(&vec![0.0; samples(5.0)]);
        stream.reset();
        assert_eq!(stream.buffered_s(), 0.0);
        assert!(stream.poll().is_none());
    }

    #[test]
    fn the_worker_copies_what_the_stream_copies() {
        let audio = test_signal::render(MSG, 25.0, 800.0, 30.0, 3);
        let worker = Worker::new().expect("worker");
        let mut transcript = Transcript::default();

        for chunk in audio.chunks(512) {
            worker.push(chunk);
            for update in worker.poll() {
                transcript.apply(update.expect("decode"));
            }
        }
        worker.flush();

        // The worker is a thread behind, and this test hands it the whole message
        // at once, so it has several windows queued. Wait for the first update
        // however long it takes — the suite runs tests in parallel and inference
        // is not the only thing on the machine — then wait for quiescence.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let mut seen = false;
        let mut idle = 0;
        while std::time::Instant::now() < deadline {
            let updates = worker.poll();
            if updates.is_empty() {
                if seen {
                    idle += 1;
                    if idle >= 100 {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            seen = true;
            idle = 0;
            for update in updates {
                transcript.apply(update.expect("decode"));
            }
        }
        assert!(seen, "the worker produced nothing within the deadline");

        let got = transcript.text();
        assert!(test_signal::accuracy(MSG, &got) > 0.95, "sent {MSG:?}\n got {got:?}");
    }

    #[test]
    fn flushing_commits_a_tail_that_never_settled() {
        // The tail guard means the last words never settle on their own; they
        // sit in the preview until something asks for them.
        let audio = test_signal::render("CQ DE W1AW K", 25.0, 800.0, 30.0, 4);
        let mut stream = Stream::new().expect("stream");
        stream.push(&audio);

        let mut transcript = Transcript::default();
        while let Some(update) = stream.poll() {
            transcript.apply(update.expect("decode"));
        }
        assert!(stream.buffered_s() > 0.0, "expected an unsettled tail to flush");

        let flushed = stream.flush().expect("tail buffered").expect("decode");
        transcript.apply(flushed);
        assert_eq!(transcript.text(), "CQ DE W1AW K");
        assert_eq!(stream.buffered_s(), 0.0);
        assert!(stream.flush().is_none(), "flushing twice should find nothing");
    }
}
