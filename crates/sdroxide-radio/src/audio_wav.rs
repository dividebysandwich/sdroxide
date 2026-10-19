//! A recording of the receiver audio as a plain WAV file.
//!
//! 16-bit PCM at 48 kHz — no codec between the receiver and the file, which is
//! what an external decoder, an audio editor or a look at the spectrum wants.
//! The I/Q capture in [`crate::iq_wav`] is a different animal (stereo float32,
//! tuned, named `_IQ.wav`) and the two are kept apart on purpose: this one is
//! audio the operator can play, that one is the raw band.
//!
//! # Why RF64
//!
//! A RIFF file's sizes are 32-bit, so a WAV stops at 4 GB. At 48 kHz stereo
//! that is roughly six hours, which a station recording an afternoon meets.
//! RF64 (EBU Tech 3306) is the standard answer and is written the way
//! [`crate::iq_wav`] already writes it: a 28-byte `JUNK` chunk reserved ahead
//! of `fmt `, an ordinary WAV while the file fits in one, and promotion on
//! close only when it does not — `RIFF`→`RF64`, the two sizes to `0xFFFFFFFF`,
//! and the reserved `JUNK` becomes `ds64`. Nothing has to be decided when the
//! recording starts, which matters because nothing knows then how long it will
//! run.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Bits per sample. Fixed: the engine hands over `f32` in `[-1, 1]` and this is
/// what every program that opens a WAV expects.
pub const BITS: u16 = 16;

/// Where the 32-bit `RIFF` size lives.
const RIFF_SIZE_AT: u64 = 4;

/// The reserved chunk's payload size: `ds64` carries three 64-bit sizes and a
/// table count, and `JUNK` has to be big enough to become one in place.
const DS64_PAYLOAD: usize = 28;

/// A recording in progress.
pub struct AudioWavWriter {
    file: BufWriter<File>,
    path: PathBuf,
    rate_hz: u32,
    channels: u16,
    /// Bytes one frame occupies — every channel's sample together.
    frame_bytes: u64,
    /// Byte offset of the `data` chunk's own 32-bit size field.
    data_size_at: u64,
    /// Frames (one sample per channel) written so far.
    frames: u64,
    scratch: Vec<u8>,
}

impl AudioWavWriter {
    /// Open `path` and write the header for `channels` channels of 16-bit PCM
    /// at `rate_hz`. `channels` is 1 or 2; anything else is clamped into that
    /// range, since the recorder only ever produces those two.
    pub fn create(path: &Path, rate_hz: u32, channels: u16) -> std::io::Result<AudioWavWriter> {
        let file = File::create(path)?;
        AudioWavWriter::new(file, path, rate_hz, channels)
    }

    /// The same, onto a file the caller has already opened — the recorder
    /// creates the file up front so a bad path is reported before the worker
    /// starts, and hands the handle over rather than reopening it.
    pub fn new(
        file: File,
        path: &Path,
        rate_hz: u32,
        channels: u16,
    ) -> std::io::Result<AudioWavWriter> {
        let channels = channels.clamp(1, 2);
        let mut w = BufWriter::with_capacity(1 << 16, file);
        let header = header_bytes(rate_hz, channels);
        w.write_all(&header.bytes)?;
        Ok(AudioWavWriter {
            file: w,
            path: path.to_path_buf(),
            rate_hz,
            channels,
            frame_bytes: u64::from(channels) * u64::from(BITS) / 8,
            data_size_at: header.data_size_at,
            frames: 0,
            scratch: Vec::new(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn rate_hz(&self) -> u32 {
        self.rate_hz
    }

    /// Frames written so far — one sample per channel.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Seconds of audio written so far.
    pub fn seconds(&self) -> f64 {
        if self.rate_hz > 0 { self.frames as f64 / f64::from(self.rate_hz) } else { 0.0 }
    }

    /// Append interleaved `f32` samples in `[-1, 1]`. The caller owns the
    /// channel count: a slice whose length is not a whole number of frames
    /// leaves the trailing samples out rather than writing a short frame.
    pub fn write(&mut self, samples: &[f32]) -> std::io::Result<()> {
        let per_frame = self.channels as usize;
        let frames = samples.len() / per_frame;
        self.scratch.clear();
        self.scratch.reserve(frames * per_frame * 2);
        for &s in &samples[..frames * per_frame] {
            self.scratch.extend_from_slice(&to_i16(s).to_le_bytes());
        }
        self.file.write_all(&self.scratch)?;
        self.frames += frames as u64;
        Ok(())
    }

    /// Close the file, patching the sizes in — and promoting it to RF64 where
    /// they no longer fit in 32 bits.
    ///
    /// A failure here leaves a file whose header says it is empty, which every
    /// player reads as a zero-length recording, so this returns rather than
    /// being a `Drop`.
    pub fn finish(mut self) -> std::io::Result<PathBuf> {
        self.file.flush()?;
        let data_bytes = self.frames * self.frame_bytes;
        let riff_bytes = self.data_size_at + 4 + data_bytes - 8;
        let f = self.file.get_mut();
        if needs_rf64(riff_bytes, data_bytes) {
            // Promote in place: the reserved chunk was put there for this.
            f.seek(SeekFrom::Start(0))?;
            f.write_all(b"RF64")?;
            f.write_all(&u32::MAX.to_le_bytes())?;
            f.seek(SeekFrom::Start(12))?;
            f.write_all(b"ds64")?;
            f.write_all(&(DS64_PAYLOAD as u32).to_le_bytes())?;
            f.write_all(&riff_bytes.to_le_bytes())?;
            f.write_all(&data_bytes.to_le_bytes())?;
            // Sample count per channel, and no chunk-size table.
            f.write_all(&self.frames.to_le_bytes())?;
            f.write_all(&0u32.to_le_bytes())?;
            f.seek(SeekFrom::Start(self.data_size_at))?;
            f.write_all(&u32::MAX.to_le_bytes())?;
        } else {
            f.seek(SeekFrom::Start(RIFF_SIZE_AT))?;
            f.write_all(&(riff_bytes as u32).to_le_bytes())?;
            f.seek(SeekFrom::Start(self.data_size_at))?;
            f.write_all(&(data_bytes as u32).to_le_bytes())?;
        }
        f.flush()?;
        Ok(self.path)
    }
}

/// Whether the sizes have outgrown a RIFF header and the file has to be
/// promoted. Either one is enough — a reader uses both.
fn needs_rf64(riff_bytes: u64, data_bytes: u64) -> bool {
    riff_bytes > u64::from(u32::MAX) || data_bytes > u64::from(u32::MAX)
}

/// `f32` in `[-1, 1]` to the 16-bit PCM form. Clamped, and scaled by 32767
/// rather than 32768 so the extremes land on the representable ends rather
/// than wrapping — a sample louder than full scale would otherwise come out as
/// noise of the opposite sign.
fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}

struct Header {
    bytes: Vec<u8>,
    /// Byte offset of the `data` chunk's own 32-bit size field.
    data_size_at: u64,
}

/// The whole header, up to and including the `data` chunk's size field.
fn header_bytes(rate_hz: u32, channels: u16) -> Header {
    let mut b: Vec<u8> = Vec::with_capacity(64);
    let block_align = u32::from(channels) * u32::from(BITS) / 8;
    let byte_rate = rate_hz * block_align;

    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes()); // patched on close
    b.extend_from_slice(b"WAVE");

    // The RF64 reservation, first of the chunks exactly as Tech 3306 requires.
    b.extend_from_slice(b"JUNK");
    b.extend_from_slice(&(DS64_PAYLOAD as u32).to_le_bytes());
    b.extend_from_slice(&[0u8; DS64_PAYLOAD]);

    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate_hz.to_le_bytes());
    b.extend_from_slice(&byte_rate.to_le_bytes());
    b.extend_from_slice(&(block_align as u16).to_le_bytes());
    b.extend_from_slice(&BITS.to_le_bytes());

    b.extend_from_slice(b"data");
    let data_size_at = b.len() as u64;
    b.extend_from_slice(&0u32.to_le_bytes()); // patched on close
    Header { bytes: b, data_size_at }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sdroxide-audiowav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// The header an ordinary stereo recording gets: PCM, two channels, 16
    /// bits, the rate it was opened with, and the sizes a player reads.
    #[test]
    fn a_stereo_recording_is_a_plain_pcm_wav() {
        let path = temp_path("stereo.wav");
        let mut w = AudioWavWriter::create(&path, 48_000, 2).unwrap();
        let samples: Vec<f32> = (0..2000).map(|i| (i as f32 / 2000.0) * 2.0 - 1.0).collect();
        w.write(&samples).unwrap();
        assert_eq!(w.frames(), 1000);
        assert_eq!(w.seconds(), 1000.0 / 48_000.0);
        w.finish().unwrap();

        let raw = std::fs::read(&path).unwrap();
        assert_eq!(&raw[..4], b"RIFF", "a recording that fits stays a WAV");
        assert_eq!(&raw[8..12], b"WAVE");
        let riff = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff, raw.len() - 8, "the RIFF size is everything after it");

        let fmt = find_chunk(&raw, b"fmt ").expect("fmt chunk");
        assert_eq!(u16::from_le_bytes(raw[fmt + 8..fmt + 10].try_into().unwrap()), 1, "PCM");
        assert_eq!(u16::from_le_bytes(raw[fmt + 10..fmt + 12].try_into().unwrap()), 2, "channels");
        assert_eq!(u32::from_le_bytes(raw[fmt + 12..fmt + 16].try_into().unwrap()), 48_000);
        assert_eq!(u32::from_le_bytes(raw[fmt + 16..fmt + 20].try_into().unwrap()), 192_000);
        assert_eq!(u16::from_le_bytes(raw[fmt + 20..fmt + 22].try_into().unwrap()), 4, "align");
        assert_eq!(u16::from_le_bytes(raw[fmt + 22..fmt + 24].try_into().unwrap()), 16, "bits");

        let at = find_chunk(&raw, b"data").expect("data chunk");
        let data = u32::from_le_bytes(raw[at + 4..at + 8].try_into().unwrap()) as usize;
        assert_eq!(data, 1000 * 2 * 2, "frames × channels × 2 bytes");
        assert_eq!(at + 8 + data, raw.len());

        // The samples are the ones written, in order, at the stated size.
        assert_eq!(i16::from_le_bytes(raw[at + 8..at + 10].try_into().unwrap()), i16::MIN + 1);
        let _ = std::fs::remove_file(&path);
    }

    /// A mono recording is one channel and half the bytes — this is the shape
    /// the MONO chip produces.
    #[test]
    fn a_mono_recording_is_one_channel() {
        let path = temp_path("mono.wav");
        let mut w = AudioWavWriter::create(&path, 48_000, 1).unwrap();
        w.write(&[0.0; 500]).unwrap();
        assert_eq!(w.frames(), 500);
        assert_eq!(w.channels(), 1);
        w.finish().unwrap();

        let raw = std::fs::read(&path).unwrap();
        let fmt = find_chunk(&raw, b"fmt ").expect("fmt chunk");
        assert_eq!(u16::from_le_bytes(raw[fmt + 10..fmt + 12].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(raw[fmt + 16..fmt + 20].try_into().unwrap()), 96_000);
        let at = find_chunk(&raw, b"data").expect("data chunk");
        assert_eq!(u32::from_le_bytes(raw[at + 4..at + 8].try_into().unwrap()), 1000);
        let _ = std::fs::remove_file(&path);
    }

    /// Full-scale samples do not wrap, and silence stays silence.
    #[test]
    fn the_samples_convert_without_wrapping() {
        assert_eq!(to_i16(1.0), i16::MAX);
        assert_eq!(to_i16(-1.0), i16::MIN + 1);
        assert_eq!(to_i16(2.0), i16::MAX, "clamped rather than wrapped");
        assert_eq!(to_i16(-2.0), i16::MIN + 1, "clamped rather than wrapped");
        assert_eq!(to_i16(0.0), 0);
    }

    /// The reservation for RF64 is there from the first byte, ahead of `fmt `,
    /// so a recording that outgrows 4 GB can be promoted without moving
    /// anything.
    #[test]
    fn the_rf64_reservation_is_written_ahead_of_the_format() {
        let path = temp_path("junk.wav");
        AudioWavWriter::create(&path, 48_000, 2).unwrap().finish().unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert_eq!(&raw[12..16], b"JUNK", "the ds64 reservation has to be the first chunk");
        assert_eq!(u32::from_le_bytes(raw[16..20].try_into().unwrap()), DS64_PAYLOAD as u32);
        assert!(find_chunk(&raw, b"fmt ").unwrap() > 12);
        let _ = std::fs::remove_file(&path);
    }

    /// The threshold itself: either size overflowing a RIFF field promotes the
    /// file, and a file just inside stays a plain WAV.
    #[test]
    fn rf64_is_promoted_only_when_a_size_overflows() {
        assert!(!needs_rf64(1000, 1000));
        assert!(!needs_rf64(u64::from(u32::MAX), u64::from(u32::MAX)));
        assert!(needs_rf64(u64::from(u32::MAX) + 1, 0));
        assert!(needs_rf64(0, u64::from(u32::MAX) + 1));
    }

    /// The I/Q capture reader must not claim an audio recording: it is PCM and
    /// would play at the wrong scale, so `probe` says no rather than reading
    /// noise.
    #[test]
    fn the_iq_reader_does_not_claim_an_audio_recording() {
        let path = temp_path("not_iq.wav");
        let mut w = AudioWavWriter::create(&path, 48_000, 2).unwrap();
        w.write(&[0.0; 64]).unwrap();
        w.finish().unwrap();
        assert!(crate::iq_wav::probe(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }

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
