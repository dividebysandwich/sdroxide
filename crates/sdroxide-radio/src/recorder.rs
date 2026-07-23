//! Off-thread MP3 recorder for the receiver audio.
//!
//! The engine's audio loop pushes mono samples (a downmix of the stereo mixer
//! output) into a lock-free ring; a dedicated thread drains it, resamples to
//! 48 kHz, and encodes to MP3 with the pure-Rust `shine_rs` encoder, writing to
//! the file. Encoding and file I/O never touch the real-time audio thread.

use std::fs::File;
use std::io::{BufWriter, Write};
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
use tracing::{info, warn};

use sdroxide_dsp::MonoResampler;

/// MP3 encode target — a universally-valid MPEG-1 Layer III rate.
const MP3_RATE: i32 = 48_000;
/// Constant bitrate (kbps). Plenty for a communications-audio recording.
const MP3_BITRATE: i32 = 128;
/// shine channel mode for a single channel (MPG_MD_MONO).
const MODE_MONO: i32 = 3;

/// A running recording. Feed it through the paired [`Producer`] (held by the
/// mixer); drop-finalize by calling [`Recorder::stop`].
pub struct Recorder {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    /// The file being written (absolute path).
    pub path: PathBuf,
}

impl Recorder {
    /// Start recording mono audio arriving at `in_rate` Hz to `path`. Returns the
    /// recorder plus the producer the caller feeds samples into. Fails only if
    /// the file can't be created (encoder setup happens on the worker thread).
    pub fn start(path: PathBuf, in_rate: f64) -> std::io::Result<(Recorder, Producer<f32>)> {
        let file = File::create(&path)?;
        // ~4 s of slack so a brief disk stall drops nothing.
        let cap = (in_rate as usize).max(MP3_RATE as usize) * 4;
        let (prod, cons) = RingBuffer::<f32>::new(cap);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_worker = stop.clone();
        let path_worker = path.clone();
        let join = std::thread::Builder::new()
            .name("mp3-recorder".into())
            .spawn(move || encode_loop(cons, file, in_rate, stop_worker, path_worker))
            .expect("spawn recorder thread");
        info!(path = %path.display(), "recording started");
        Ok((Recorder { stop, join: Some(join), path }, prod))
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

fn encode_loop(
    mut cons: Consumer<f32>,
    file: File,
    in_rate: f64,
    stop: Arc<AtomicBool>,
    path: PathBuf,
) {
    let cfg = ShineConfig {
        wave: ShineWave { channels: 1, samplerate: MP3_RATE },
        mpeg: ShineMpeg {
            mode: MODE_MONO,
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

    let mut file = BufWriter::new(file);
    let mut resampler = MonoResampler::new(in_rate, MP3_RATE as f64);
    let mut drained: Vec<f32> = Vec::new();
    let mut resampled: Vec<f32> = Vec::new();
    let mut pending: Vec<f32> = Vec::new(); // 48 kHz mono awaiting a full frame
    let mut pcm = vec![0i16; spp];

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
        match resampler.as_mut() {
            Some(r) => {
                resampled.clear();
                r.push(&drained, &mut resampled);
                pending.extend_from_slice(&resampled);
            }
            None => pending.extend_from_slice(&drained),
        }
        while pending.len() >= spp {
            encode_frame(&mut file, &mut enc, &pending[..spp], &mut pcm, &path);
            pending.drain(..spp);
        }
    }

    // Final partial frame (zero-padded) so no tail is dropped, then flush.
    if !pending.is_empty() {
        pending.resize(spp, 0.0);
        encode_frame(&mut file, &mut enc, &pending[..spp], &mut pcm, &path);
    }
    let (tail, n) = shine_flush(&mut enc);
    if n > 0 {
        let _ = file.write_all(tail);
    }
    let _ = file.flush();
    shine_close(enc);
}

/// Convert one frame of f32 mono to i16 and encode it, writing the MP3 bytes.
fn encode_frame(
    file: &mut BufWriter<File>,
    enc: &mut shine_rs::ShineGlobalConfig,
    frame: &[f32],
    pcm: &mut [i16],
    path: &std::path::Path,
) {
    for (o, &s) in pcm.iter_mut().zip(frame) {
        *o = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
    }
    let ptr = pcm.as_ptr();
    match shine_encode_buffer(enc, &[ptr]) {
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
        let path = std::env::temp_dir().join(format!("sdroxide-rec-test-{}.mp3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let (rec, mut prod) = Recorder::start(path.clone(), 48_000.0).expect("start recorder");

        // ~1 s of a 1 kHz tone at 48 kHz mono, fed as it drains.
        for i in 0..48_000 {
            let s = 0.5 * (std::f32::consts::TAU * 1000.0 * i as f32 / 48_000.0).sin();
            while prod.push(s).is_err() {
                std::thread::sleep(Duration::from_millis(1)); // ring full: let it drain
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
}
