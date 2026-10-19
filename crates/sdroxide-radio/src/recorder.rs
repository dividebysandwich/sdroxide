//! Off-thread audio recorder for a QSO: RX in the left channel, TX in the right
//! — or, in mono mode, both time-multiplexed onto a single channel. A stereo
//! recording with no second receiver to fill the right channel is written as
//! dual mono instead of one silent ear; the mixer decides that, this end only
//! ever sees interleaved frames.
//!
//! Two containers, chosen at [`Recorder::start`]:
//!
//! - **MP3**, the default, encoded with the pure-Rust `shine_rs` encoder — the
//!   small-file answer for playing a contact back.
//! - **WAV**, 16-bit PCM with no codec at all ([`crate::audio_wav`]), for anyone
//!   who wants the samples themselves: an external decoder, an audio editor, a
//!   look at the spectrum. It costs roughly 11 MB a minute stereo.
//!
//! The engine's audio loop pushes interleaved frames straight off the stereo
//! mixer into a lock-free ring; a dedicated thread drains it, resamples to
//! 48 kHz, and hands it to whichever writer was chosen. Encoding and file I/O
//! never touch the real-time audio thread.
//!
//! A fixed channel count for the life of the recording (chosen at
//! [`Recorder::start`]), so WFM stereo coming and going, or the sub receiver
//! being switched on, never has to reinitialise the encoder mid-file.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use rtrb::{Consumer, Producer, RingBuffer};
use shine_rs::encoder::{
    ShineConfig, ShineMpeg, ShineWave, shine_close, shine_encode_buffer, shine_flush,
    shine_initialise, shine_samples_per_pass,
};
use tracing::{error, info, warn};

use sdroxide_dsp::{MonoResampler, StereoResampler};
use sdroxide_types::RecordingFormat;

use crate::audio_wav::AudioWavWriter;

/// Every recording is written at this rate, whatever the receiver runs at — a
/// universally-valid rate for both containers.
const REC_RATE_HZ: u32 = 48_000;
/// MP3 encode target — `REC_RATE_HZ` as `shine` wants it.
const MP3_RATE: i32 = REC_RATE_HZ as i32;
/// Constant bitrate (kbps). Ample for communications audio, and joint stereo
/// spends almost none of it when L and R are identical.
const MP3_BITRATE: i32 = 192;
/// shine channel mode MPG_MD_JOINT_STEREO.
const MODE_JOINT_STEREO: i32 = 1;
/// shine channel mode MPG_MD_MONO.
const MODE_MONO: i32 = 3;

/// How many interleaved channels a recording carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingChannels {
    /// Two channels: RX in the left, TX in the right — or the same signal in
    /// both, where the caller has nothing to separate (see `StereoMixer`).
    Stereo,
    /// RX and TX time-multiplexed onto a single channel — for RX-only
    /// listening, or anyone who doesn't want split-ear audio.
    Mono,
}

impl RecordingChannels {
    fn count(self) -> usize {
        match self {
            RecordingChannels::Stereo => 2,
            RecordingChannels::Mono => 1,
        }
    }

    fn shine_mode(self) -> i32 {
        match self {
            RecordingChannels::Stereo => MODE_JOINT_STEREO,
            RecordingChannels::Mono => MODE_MONO,
        }
    }
}

/// How many times the encoder may be rebuilt around a panic before the
/// recording is given up on.
///
/// A handful, not one: a single bad frame is a bug in the encoder and worth
/// riding out, while a panic on every frame is a recording that is never going
/// to produce anything and a thread that would otherwise spin rebuilding an
/// encoder forever.
const MAX_ENCODER_RESTARTS: u32 = 8;

/// A running recording. Feed it through the paired [`Producer`] (held by the
/// mixer); drop-finalize by calling [`Recorder::stop`].
pub struct Recorder {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    /// Set when the encoder panicked and could not be brought back, so the
    /// recording has stopped short of where the operator thinks it has. Read
    /// by [`Recorder::failure`].
    failed: Arc<AtomicBool>,
    /// Set when the encoder panicked *and was restarted* — the file is intact
    /// either side of a short gap. Cleared by [`Recorder::failure`] once
    /// reported, so one glitch is mentioned once.
    glitched: Arc<AtomicBool>,
    /// The file being written (absolute path).
    pub path: PathBuf,
}

/// What [`Recorder::failure`] has to report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecorderFault {
    /// The encoder panicked and came back; the file continues after a gap of a
    /// few tens of milliseconds.
    Glitch,
    /// The encoder could not be brought back. Nothing more is being written
    /// and the file ends where it ends.
    Dead,
}

impl Recorder {
    /// Start recording interleaved audio arriving at `in_rate` Hz per channel
    /// to `path`, with `channels` interleaved channels per frame in the given
    /// `format`. Returns the recorder plus the producer the caller feeds frames
    /// into. Fails only if the file can't be created (encoder setup happens on
    /// the worker thread).
    pub fn start(
        path: PathBuf,
        in_rate: f64,
        channels: RecordingChannels,
        format: RecordingFormat,
    ) -> std::io::Result<(Recorder, Producer<f32>)> {
        let file = File::create(&path)?;
        // ~4 s of slack so a brief disk stall drops nothing.
        let cap = (in_rate as usize).max(REC_RATE_HZ as usize) * 4 * channels.count();
        let (prod, cons) = RingBuffer::<f32>::new(cap);
        let stop = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let glitched = Arc::new(AtomicBool::new(false));
        let health = Health { failed: failed.clone(), glitched: glitched.clone() };
        let stop_worker = stop.clone();
        let path_worker = path.clone();
        let join = std::thread::Builder::new()
            .name("audio-recorder".into())
            .spawn(move || {
                encode_loop(cons, file, in_rate, channels, format, stop_worker, path_worker, health)
            })
            .expect("spawn recorder thread");
        info!(path = %path.display(), format = ?format, "recording started");
        Ok((Recorder { stop, join: Some(join), failed, glitched, path }, prod))
    }

    /// Anything that has gone wrong in the encoder since this was last asked,
    /// so a caller can tell the operator rather than leaving them to find a
    /// truncated file afterwards (issue #443).
    ///
    /// A recording is the one part of the program whose failure is completely
    /// invisible while it happens: the audio keeps playing, the button stays
    /// lit, and the only sign is a file that stops early. Polled rather than
    /// pushed because the worker is a plain thread with no channel back, and
    /// because a glitch is worth mentioning once rather than every tick.
    ///
    /// [`RecorderFault::Dead`] latches — it is the state of the recording, not
    /// an event — while [`RecorderFault::Glitch`] is taken and cleared.
    pub fn failure(&self) -> Option<RecorderFault> {
        if self.failed.load(Ordering::Relaxed) {
            return Some(RecorderFault::Dead);
        }
        self.glitched.swap(false, Ordering::Relaxed).then_some(RecorderFault::Glitch)
    }

    /// Stop recording: signal the worker, wait for it to flush and close the
    /// file. Blocks briefly (a final encode + flush).
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        info!(path = %self.path.display(), "recording stopped");
    }
}

/// The two flags [`Recorder::failure`] reads, as the worker sees them.
struct Health {
    failed: Arc<AtomicBool>,
    glitched: Arc<AtomicBool>,
}

/// The recorder thread: run the writer for the chosen container.
fn encode_loop(
    cons: Consumer<f32>,
    file: File,
    in_rate: f64,
    channels: RecordingChannels,
    format: RecordingFormat,
    stop: Arc<AtomicBool>,
    path: PathBuf,
    health: Health,
) {
    match format {
        // PCM is written straight through, so there is no encoder to panic and
        // none of the supervision below applies. The header is patched on the
        // way out either way, and a write error is warned rather than starting
        // a second session over the same bytes.
        RecordingFormat::Wav => wav_session(cons, file, in_rate, channels, &stop, &path),
        RecordingFormat::Mp3 => mp3_session(cons, file, in_rate, channels, stop, path, health),
    }
}

/// The MP3 worker: run an encode session, and if the encoder panics, build a
/// fresh one and carry on appending to the same file.
///
/// An MP3 file is a sequence of self-contained frames, so a new encoder's
/// output appended to a half-written file plays: what is lost is the audio
/// that was in flight, a few tens of milliseconds, and the listener hears a
/// click. That is a far better answer than what used to happen — the thread
/// died, the panic went to stderr, the recording stopped, and nothing on the
/// client said so, so the operator found out when they played the file back
/// (issue #443).
///
/// The panic that prompted this was an integer underflow inside `shine-rs`
/// 0.1.3 (`labs(i32::MIN)` wrapping, then indexing a lookup table with the
/// result), fixed upstream in 0.1.4, which this crate now requires. The
/// supervision stays regardless: the encoder is the one piece of this path
/// that is not sdroxide's code, and a recording that stops silently is worse
/// than one with a click in it.
///
/// A WAV gets none of this on purpose — its frames are not self-contained, its
/// sizes are written at the end, and a second session would restart the header
/// over a file already part-written.
fn mp3_session(
    mut cons: Consumer<f32>,
    file: File,
    in_rate: f64,
    channels: RecordingChannels,
    stop: Arc<AtomicBool>,
    path: PathBuf,
    health: Health,
) {
    let mut file = BufWriter::new(file);
    let mut restarts = 0u32;
    loop {
        // `AssertUnwindSafe` because the state that crosses this boundary is
        // the writer and the ring — a `BufWriter` that may be missing its last
        // frame, and a consumer that has had samples taken out of it. Both are
        // exactly as valid after a panic as before one; nothing here holds an
        // invariant a half-finished encode could break. The encoder itself is
        // built inside and thrown away with the panic.
        let finished = std::panic::catch_unwind(AssertUnwindSafe(|| {
            encode_session(&mut cons, &mut file, in_rate, channels, &stop, &path)
        }));
        match finished {
            Ok(()) => break,
            Err(_) => {
                restarts += 1;
                if stop.load(Ordering::Relaxed) || restarts > MAX_ENCODER_RESTARTS {
                    health.failed.store(true, Ordering::Relaxed);
                    error!(
                        path = %path.display(),
                        restarts,
                        "the MP3 encoder panicked and could not be restarted; this recording \
                         ends here"
                    );
                    break;
                }
                health.glitched.store(true, Ordering::Relaxed);
                warn!(
                    path = %path.display(),
                    restarts,
                    "the MP3 encoder panicked; rebuilding it and carrying on — the recording \
                     has a short gap at this point"
                );
            }
        }
    }
    let _ = file.flush();
}

/// The WAV worker: resample to 48 kHz and write the samples, then patch the
/// header. One pass, no supervision — there is no encoder to fall over.
fn wav_session(
    mut cons: Consumer<f32>,
    file: File,
    in_rate: f64,
    channels: RecordingChannels,
    stop: &AtomicBool,
    path: &std::path::Path,
) {
    let ch = channels.count();
    let mut writer = match AudioWavWriter::new(file, path, REC_RATE_HZ, ch as u16) {
        Ok(w) => w,
        Err(e) => {
            warn!(path = %path.display(), "WAV header write failed: {e}; recording aborted");
            return;
        }
    };
    let mut rs = Resampler48::new(in_rate, ch);
    let mut drained: Vec<f32> = Vec::new();
    let mut resampled: Vec<f32> = Vec::new();
    loop {
        drained.clear();
        while let Ok(s) = cons.pop() {
            drained.push(s);
        }
        if drained.is_empty() {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        rs.push(&drained, &mut resampled);
        if let Err(e) = writer.write(&resampled) {
            warn!(path = %path.display(), "recording write failed: {e}");
        }
    }
    if let Err(e) = writer.finish() {
        warn!(path = %path.display(), "recording close failed: {e}");
    }
}

/// Resample interleaved frames arriving at some rate to `REC_RATE_HZ`, for
/// either channel count.
///
/// Both containers want 48 kHz and the receiver can run at anything, so the
/// choice of resampler is the same question in both workers: which channel
/// count, and is a resampler needed at all when the rates already match. Kept
/// in one place so the MP3 and WAV paths cannot drift.
struct Resampler48 {
    stereo: Option<StereoResampler>,
    mono: Option<MonoResampler>,
}

impl Resampler48 {
    fn new(in_rate: f64, channels: usize) -> Resampler48 {
        if channels == 2 {
            Resampler48 {
                stereo: StereoResampler::new(in_rate, f64::from(REC_RATE_HZ)),
                mono: None,
            }
        } else {
            Resampler48 { stereo: None, mono: MonoResampler::new(in_rate, f64::from(REC_RATE_HZ)) }
        }
    }

    /// Resample `input` into `out`, clearing it first. With no resampler live
    /// (the rates already match) this is a copy, which is what the resampler
    /// would do anyway.
    fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        out.clear();
        match (self.stereo.as_mut(), self.mono.as_mut()) {
            (Some(r), _) => r.push(input, out),
            (None, Some(r)) => r.push(input, out),
            (None, None) => out.extend_from_slice(input),
        }
    }
}

/// One encode session: build an encoder, drain and encode until asked to stop,
/// then write the tail. Returns normally only on a clean stop — a panic in the
/// encoder unwinds out of here and [`encode_loop`] catches it.
fn encode_session(
    cons: &mut Consumer<f32>,
    file: &mut BufWriter<File>,
    in_rate: f64,
    channels: RecordingChannels,
    stop: &AtomicBool,
    path: &std::path::Path,
) {
    let ch = channels.count();
    let cfg = ShineConfig {
        wave: ShineWave { channels: ch as i32, samplerate: MP3_RATE },
        mpeg: ShineMpeg {
            mode: channels.shine_mode(),
            bitr: MP3_BITRATE,
            emph: 0,
            copyright: 0,
            original: 1,
        },
    };
    let mut enc = match shine_initialise(&cfg) {
        Ok(e) => e,
        Err(e) => {
            warn!(path = %path.display(), "MP3 encoder init failed: {e}; recording aborted");
            return;
        }
    };
    let spp = shine_samples_per_pass(&enc) as usize; // samples per channel per frame

    // Exactly one of these is live, matching `ch`; the rates line up and the
    // push is a copy when they already match.
    let mut rs = Resampler48::new(in_rate, ch);
    let mut drained: Vec<f32> = Vec::new();
    let mut resampled: Vec<f32> = Vec::new();
    let mut pending: Vec<f32> = Vec::new(); // 48 kHz interleaved, awaiting a frame
    // shine takes one pointer per channel, so the frame is de-interleaved here.
    let mut pcm_l = vec![0i16; spp];
    let mut pcm_r = vec![0i16; spp];

    loop {
        drained.clear();
        while let Ok(s) = cons.pop() {
            drained.push(s);
        }
        if drained.is_empty() {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        rs.push(&drained, &mut resampled);
        pending.extend_from_slice(&resampled);
        while pending.len() >= spp * ch {
            encode_frame(file, &mut enc, &pending[..spp * ch], ch, &mut pcm_l, &mut pcm_r, path);
            pending.drain(..spp * ch);
        }
    }

    // Final partial frame (zero-padded) so no tail is dropped, then flush.
    if !pending.is_empty() {
        pending.resize(spp * ch, 0.0);
        encode_frame(file, &mut enc, &pending[..spp * ch], ch, &mut pcm_l, &mut pcm_r, path);
    }
    let (tail, n) = shine_flush(&mut enc);
    if n > 0 {
        let _ = file.write_all(tail);
    }
    let _ = file.flush();
    shine_close(enc);
}

/// De-interleave one frame of f32 samples into per-channel i16 and encode it,
/// writing the MP3 bytes. `pcm_r` is unused (but still sized) when `channels == 1`.
fn encode_frame(
    file: &mut BufWriter<File>,
    enc: &mut shine_rs::ShineGlobalConfig,
    frame: &[f32],
    channels: usize,
    pcm_l: &mut [i16],
    pcm_r: &mut [i16],
    path: &std::path::Path,
) {
    let result = if channels == 2 {
        for (i, lr) in frame.chunks_exact(2).enumerate() {
            pcm_l[i] = (lr[0].clamp(-1.0, 1.0) * 32767.0) as i16;
            pcm_r[i] = (lr[1].clamp(-1.0, 1.0) * 32767.0) as i16;
        }
        shine_encode_buffer(enc, &[pcm_l.as_ptr(), pcm_r.as_ptr()])
    } else {
        for (i, &s) in frame.iter().enumerate() {
            pcm_l[i] = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        }
        shine_encode_buffer(enc, &[pcm_l.as_ptr()])
    };
    match result {
        Ok((mp3, n)) => {
            if n > 0 {
                if let Err(e) = file.write_all(mp3) {
                    warn!(path = %path.display(), "recording write failed: {e}");
                }
            }
        }
        Err(e) => warn!(path = %path.display(), "MP3 encode failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn records_a_valid_mp3() {
        let path =
            std::env::temp_dir().join(format!("sdroxide-rec-test-{}.mp3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let (rec, mut prod) = Recorder::start(
            path.clone(),
            48_000.0,
            RecordingChannels::Stereo,
            RecordingFormat::Mp3,
        )
        .expect("start recorder");

        // ~1 s at 48 kHz: 1 kHz in the left ear, 400 Hz in the right, so the
        // two channels are genuinely different, fed as it drains.
        for i in 0..48_000 {
            let t = i as f32 / 48_000.0;
            let l = 0.5 * (std::f32::consts::TAU * 1000.0 * t).sin();
            let r = 0.5 * (std::f32::consts::TAU * 400.0 * t).sin();
            for s in [l, r] {
                while prod.push(s).is_err() {
                    std::thread::sleep(Duration::from_millis(1)); // ring full: let it drain
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
        rec.stop(); // flushes + closes the file

        let bytes = std::fs::read(&path).expect("read mp3");
        let _ = std::fs::remove_file(&path);
        assert!(bytes.len() > 2_000, "mp3 suspiciously small: {} bytes", bytes.len());
        // First frame must start with an 11-bit MPEG sync word (0xFFE..).
        assert_eq!(bytes[0], 0xFF, "no MP3 frame sync");
        assert_eq!(bytes[1] & 0xE0, 0xE0, "no MP3 frame sync in byte 1");
    }

    /// A WAV recording is plain PCM: a RIFF header, 16-bit samples, and the
    /// sizes patched in on close — the same drain/resample path as MP3 with a
    /// different writer behind it.
    #[test]
    fn records_a_valid_wav() {
        let path =
            std::env::temp_dir().join(format!("sdroxide-rec-test-{}.wav", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let (rec, mut prod) = Recorder::start(
            path.clone(),
            48_000.0,
            RecordingChannels::Stereo,
            RecordingFormat::Wav,
        )
        .expect("start recorder");

        for i in 0..48_000 {
            let t = i as f32 / 48_000.0;
            let l = 0.5 * (std::f32::consts::TAU * 1000.0 * t).sin();
            let r = 0.5 * (std::f32::consts::TAU * 400.0 * t).sin();
            for s in [l, r] {
                while prod.push(s).is_err() {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(rec.failure(), None, "a WAV has no encoder to fail");
        rec.stop();

        let raw = std::fs::read(&path).expect("read wav");
        let _ = std::fs::remove_file(&path);
        assert_eq!(&raw[..4], b"RIFF", "not a RIFF file");
        assert_eq!(&raw[8..12], b"WAVE");
        let riff = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff, raw.len() - 8, "the RIFF size was not patched in");
        // The data chunk's size is the audio, and it reaches the end of file.
        let at = find_chunk(&raw, b"data").expect("data chunk");
        let data = u32::from_le_bytes(raw[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(data > 100_000, "wav suspiciously small: {data} bytes");
        assert_eq!(at + 8 + data, raw.len());
    }

    #[test]
    fn records_a_valid_mono_mp3() {
        let path =
            std::env::temp_dir().join(format!("sdroxide-rec-test-mono-{}.mp3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let (rec, mut prod) =
            Recorder::start(path.clone(), 48_000.0, RecordingChannels::Mono, RecordingFormat::Mp3)
                .expect("start recorder");

        for i in 0..48_000 {
            let t = i as f32 / 48_000.0;
            let s = 0.5 * (std::f32::consts::TAU * 1000.0 * t).sin();
            while prod.push(s).is_err() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        std::thread::sleep(Duration::from_millis(100));
        rec.stop();

        let bytes = std::fs::read(&path).expect("read mp3");
        let _ = std::fs::remove_file(&path);
        assert!(bytes.len() > 1_000, "mp3 suspiciously small: {} bytes", bytes.len());
        assert_eq!(bytes[0], 0xFF, "no MP3 frame sync");
        assert_eq!(bytes[1] & 0xE0, 0xE0, "no MP3 frame sync in byte 1");
    }

    /// Full scale, both rails, for a second — the input that used to reach the
    /// underflow in `shine-rs` 0.1.3 (`labs(i32::MIN)` wrapping negative and
    /// then indexing a 10000-entry table with it), which killed the recorder
    /// thread and stopped the recording without a word to anyone (issue #443).
    ///
    /// A square wave rather than a sine: it is the signal that drives the MDCT
    /// coefficients hardest, and the whole point is to stand on the rails
    /// rather than approach them.
    #[test]
    fn a_recording_at_full_scale_neither_panics_nor_reports_a_fault() {
        let path =
            std::env::temp_dir().join(format!("sdroxide-rec-test-rail-{}.mp3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let (rec, mut prod) = Recorder::start(
            path.clone(),
            48_000.0,
            RecordingChannels::Stereo,
            RecordingFormat::Mp3,
        )
        .expect("start recorder");
        for i in 0..48_000u32 {
            // A 1 kHz square at exactly ±1.0, and the other ear its inverse so
            // the joint-stereo path sees the widest difference there is.
            let hi = (i / 24) % 2 == 0;
            let l = if hi { 1.0 } else { -1.0 };
            for sample in [l, -l] {
                while prod.push(sample).is_err() {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(rec.failure(), None, "the encoder fell over on full-scale audio");
        rec.stop();

        let bytes = std::fs::read(&path).expect("read mp3");
        let _ = std::fs::remove_file(&path);
        assert!(bytes.len() > 2_000, "mp3 suspiciously small: {} bytes", bytes.len());
        assert_eq!(bytes[0], 0xFF, "no MP3 frame sync");
    }

    /// `failure()`'s two answers behave differently on purpose: a glitch is an
    /// event and is reported once, a dead encoder is a *state* and keeps being
    /// reported, because the recording really is over.
    #[test]
    fn a_glitch_is_reported_once_and_a_dead_encoder_keeps_being_reported() {
        let path = std::env::temp_dir()
            .join(format!("sdroxide-rec-test-health-{}.mp3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let (rec, prod) =
            Recorder::start(path.clone(), 48_000.0, RecordingChannels::Mono, RecordingFormat::Mp3)
                .expect("start recorder");
        assert_eq!(rec.failure(), None);

        rec.glitched.store(true, Ordering::Relaxed);
        assert_eq!(rec.failure(), Some(RecorderFault::Glitch));
        assert_eq!(rec.failure(), None, "a glitch is mentioned once, not every tick");

        rec.failed.store(true, Ordering::Relaxed);
        assert_eq!(rec.failure(), Some(RecorderFault::Dead));
        assert_eq!(rec.failure(), Some(RecorderFault::Dead), "still not recording");

        drop(prod);
        rec.stop();
        let _ = std::fs::remove_file(&path);
    }

    /// Walk the chunk list of a RIFF file, the way a player does, so a test can
    /// find `data` without assuming a header layout.
    fn find_chunk(raw: &[u8], id: &[u8; 4]) -> Option<usize> {
        let mut at = 12;
        while at + 8 <= raw.len() {
            let size = u32::from_le_bytes(raw[at + 4..at + 8].try_into().unwrap()) as usize;
            if &raw[at..at + 4] == id {
                return Some(at);
            }
            at += 8 + size + (size & 1);
        }
        None
    }
}
