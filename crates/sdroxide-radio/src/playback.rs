//! A recording played back as if a sound-card radio were sending it (fork
//! discussion #10).
//!
//! [`PlaybackSource`] is an [`IqSource`] in audio mode: the engine adopts it in
//! place of the radio for the length of a playback, and everything a
//! demod-audio rig's audio goes through — the waterfall, the digital decoders,
//! noise reduction, the speakers — takes the file's audio instead. Nothing
//! downstream is told it is a file, which is the point: the SSTV decoder that
//! missed a picture on the air gets the same samples a second time.
//!
//! The file is decoded a packet at a time ([`AudioReader`]), never loaded whole
//! — a recording can run for hours — and paced to the wall clock, so a picture
//! paints at the speed it was sent. Paused, the source keeps time and hands
//! out silence, because the engine's loop is paced by its source.
//!
//! Each sample carries both ears: the first channel in `re`, the second in `im`
//! (the first again on a mono file). The decoders read `re` — the receiver —
//! and the speaker plays both when the recording is stereo, which is how a WFM
//! stereo broadcast comes back in stereo.

use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;

use sdroxide_types::{PlaybackCommand, PlaybackStatus, RecordingStamp};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::source::Throttle;
use crate::{Complex32, IqSource, RadioError, Result};

/// How much of the audio spectrum the waterfall shows: the same as a sound-card
/// radio's, which is where every audio decoder's tones sit.
const DISPLAY_BW_HZ: f64 = 4000.0;

fn err(msg: impl Into<String>) -> RadioError {
    RadioError::Msg(msg.into())
}

/// Most of one read: 10 ms, the size of block a sound card hands the engine.
///
/// The engine reads in blocks sized for an SDR (16 384 samples), which at an
/// audio rate is a third of a second at once — more than the speaker queue
/// holds (`MIXER_CAP`, ≈ ¼ s), so the overflow was thrown away and the next
/// third of a second arrived in one lump: audio with half of it missing, and
/// decoders fed in bursts. A sound card never delivers like that, and nothing
/// downstream is built for it.
const READ_SECONDS: f64 = 0.01;

/// A compressed or PCM audio file, decoded on demand into its first two
/// channels.
pub struct AudioReader {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    rate: u32,
    channels: u16,
    /// The file's length in frames, when it could be measured.
    frames: Option<u64>,
    buf: Option<SampleBuffer<f32>>,
}

impl AudioReader {
    /// Open `path` and find its audio track. The length is taken from the
    /// header where the file has one, and otherwise counted by walking the
    /// packets once — an MP3 straight from the recorder carries no length, and
    /// a transport bar that cannot say how long the file is cannot be seeked.
    pub fn open(path: &Path) -> Result<Self> {
        let mut reader = Self::open_once(path)?;
        if reader.frames.is_none() {
            reader.frames = Self::open_once(path).ok().and_then(|mut r| r.count_frames());
        }
        Ok(reader)
    }

    fn open_once(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions { enable_gapless: true, ..Default::default() },
                &MetadataOptions::default(),
            )
            .map_err(|e| err(format!("not an audio file this build can read ({e})")))?;
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| err("the file has no audio track"))?;
        let params = track.codec_params.clone();
        let track_id = track.id;
        let rate = params.sample_rate.ok_or_else(|| err("the file does not state its rate"))?;
        let channels = params.channels.map_or(1, |c| c.count()) as u16;
        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(|e| err(format!("no decoder for this file ({e})")))?;
        Ok(AudioReader {
            format,
            decoder,
            track_id,
            rate,
            channels,
            frames: params.n_frames,
            buf: None,
        })
    }

    /// Count the frames by reading every packet's duration, without decoding.
    fn count_frames(&mut self) -> Option<u64> {
        let mut total = 0u64;
        loop {
            match self.format.next_packet() {
                Ok(p) if p.track_id() == self.track_id => total += p.dur,
                Ok(_) => {}
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(_) => break,
            }
        }
        (total > 0).then_some(total)
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn frames(&self) -> Option<u64> {
        self.frames
    }

    /// Decode the next packet onto `out`, first channel in `re` and second in
    /// `im` (the first again on a mono file). `Ok(false)` at
    /// the end of the file. A packet that will not decode is skipped, as every
    /// player does: a damaged frame in the middle of a recording is no reason
    /// to lose the rest of it.
    pub fn decode_next(&mut self, out: &mut VecDeque<Complex32>) -> Result<bool> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(false);
                }
                Err(SymError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(err(format!("reading the file: {e}"))),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymError::DecodeError(_)) => continue,
                Err(e) => return Err(err(format!("decoding the file: {e}"))),
            };
            let spec = *decoded.spec();
            let need = decoded.capacity() as u64;
            if self.buf.as_ref().is_none_or(|b| (b.capacity() as u64) < need) {
                self.buf = Some(SampleBuffer::new(need, spec));
            }
            let Some(buf) = self.buf.as_mut() else { continue };
            buf.copy_interleaved_ref(decoded);
            let ch = spec.channels.count().max(1);
            // Both ears, for the speaker; the decoders take the first, which is
            // the receiver (the recorder puts our own transmit in the right).
            let right = if ch > 1 { 1 } else { 0 };
            out.extend(buf.samples().chunks_exact(ch).map(|f| Complex32::new(f[0], f[right])));
            return Ok(true);
        }
    }

    /// Go to `seconds` into the file. Returns the frame actually reached;
    /// samples between the packet the format landed on and the one asked for
    /// are dropped here, so the next decode starts where the operator pointed.
    pub fn seek(&mut self, seconds: f64, out: &mut VecDeque<Complex32>) -> Result<u64> {
        let seconds = seconds.max(0.0);
        let time = Time::new(seconds.trunc() as u64, seconds.fract());
        let seeked = self
            .format
            .seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(self.track_id) })
            .map_err(|e| err(format!("seeking: {e}")))?;
        self.decoder.reset();
        out.clear();
        // Timestamps on an audio track count frames at the track's own rate.
        let mut skip = seeked.required_ts.saturating_sub(seeked.actual_ts);
        while skip > 0 {
            if !self.decode_next(out)? {
                break;
            }
            let drop = (skip as usize).min(out.len());
            out.drain(..drop);
            skip -= drop as u64;
        }
        Ok(seeked.required_ts)
    }
}

/// The source the engine adopts for a playback.
pub struct PlaybackSource {
    name: String,
    stamp: Option<RecordingStamp>,
    reader: AudioReader,
    center_hz: f64,
    /// Decoded, not yet handed out.
    pending: VecDeque<Complex32>,
    /// Frames of the file handed out so far — the transport bar's position.
    pos_frames: u64,
    paused: bool,
    looping: bool,
    ended: bool,
    /// Set when the file broke mid-playback; it then plays silence and says why.
    fault: Option<String>,
    throttle: Throttle,
}

impl PlaybackSource {
    /// Open `path`, shown to the operator as `name`. The dial the engine shows
    /// starts at `center_hz`; the source has no knob of its own.
    pub fn open(path: &Path, name: &str, center_hz: f64) -> Result<Self> {
        let reader = AudioReader::open(path)?;
        let rate = reader.rate() as f64;
        if rate < 4000.0 {
            return Err(err(format!("{rate:.0} Hz is too low a rate to decode anything from")));
        }
        Ok(PlaybackSource {
            name: name.to_string(),
            stamp: sdroxide_types::playback::parse_recording_name(name),
            reader,
            center_hz,
            pending: VecDeque::new(),
            pos_frames: 0,
            paused: false,
            looping: false,
            ended: false,
            fault: None,
            throttle: Throttle::new(rate),
        })
    }

    /// Where playback is, for the state the engine publishes.
    pub fn status(&self) -> PlaybackStatus {
        let rate = self.reader.rate() as f64;
        PlaybackStatus {
            name: self.name.clone(),
            position_s: self.pos_frames as f64 / rate,
            duration_s: self.reader.frames().map(|f| f as f64 / rate),
            paused: self.paused,
            looping: self.looping,
            ended: self.ended,
            rate_hz: self.reader.rate(),
            channels: self.reader.channels(),
            stamp: self.stamp,
            fault: self.fault.clone(),
        }
    }

    /// Steer it. `Open`, `Stop` and `Delete` are the engine's, since they swap
    /// the source or touch the folder; everything else is answered here.
    pub fn control(&mut self, cmd: &PlaybackCommand) -> Result<()> {
        match cmd {
            PlaybackCommand::Pause(p) => self.paused = *p,
            PlaybackCommand::Loop(l) => self.looping = *l,
            PlaybackCommand::Seek(s) => {
                let to = match self.reader.frames() {
                    Some(f) => s.clamp(0.0, f as f64 / self.reader.rate() as f64),
                    None => s.max(0.0),
                };
                self.pos_frames = self.reader.seek(to, &mut self.pending)?;
                self.ended = false;
            }
            PlaybackCommand::Open { .. }
            | PlaybackCommand::Stop
            | PlaybackCommand::Delete { .. } => {}
        }
        Ok(())
    }

    /// Why playback went silent, if the file broke under it.
    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }

    /// Fill `out` with the next samples of the file — or silence while paused,
    /// at the end, or after a fault — without pacing. Split from `read` so the
    /// tests can run it faster than real time.
    fn fill(&mut self, out: &mut [Complex32]) {
        let mut n = 0;
        while n < out.len() {
            if self.paused || self.ended || self.fault.is_some() {
                out[n..].fill(Complex32::new(0.0, 0.0));
                return;
            }
            if self.pending.is_empty() {
                match self.reader.decode_next(&mut self.pending) {
                    Ok(true) => continue,
                    Ok(false) if self.looping && self.pos_frames > 0 => {
                        match self.reader.seek(0.0, &mut self.pending) {
                            Ok(at) => self.pos_frames = at,
                            Err(e) => self.fault = Some(e.to_string()),
                        }
                        continue;
                    }
                    Ok(false) => {
                        self.ended = true;
                        continue;
                    }
                    Err(e) => {
                        self.fault = Some(e.to_string());
                        continue;
                    }
                }
            }
            let take = (out.len() - n).min(self.pending.len());
            for (o, s) in out[n..n + take].iter_mut().zip(self.pending.drain(..take)) {
                *o = s;
            }
            n += take;
            self.pos_frames += take as u64;
        }
    }
}

impl IqSource for PlaybackSource {
    fn sample_rate(&self) -> f64 {
        self.reader.rate() as f64
    }

    fn center_hz(&self) -> f64 {
        self.center_hz
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        // Nothing to move: the dial is a label on a recording.
        self.center_hz = hz;
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(read_len(self.sample_rate()));
        self.fill(&mut buf[..n]);
        self.throttle.pace(n);
        Ok(n)
    }

    fn audio_stereo(&self) -> bool {
        self.reader.channels() > 1
    }

    fn describe(&self) -> String {
        format!("playback of {}", self.name)
    }

    fn open_status(&self) -> Option<String> {
        self.fault.as_ref().map(|f| format!("Playback stopped: {f}"))
    }

    fn display_bandwidth(&self) -> Option<f64> {
        Some(DISPLAY_BW_HZ.min(self.sample_rate() / 2.0))
    }

    fn playback_status(&self) -> Option<PlaybackStatus> {
        Some(self.status())
    }

    fn playback_control(&mut self, cmd: &PlaybackCommand) -> Result<()> {
        self.control(cmd)
    }
}

/// What the engine runs on when a playback ends and the radio will not come
/// back: silence at the radio's rate, asking to be replaced, so the reconnect
/// schedule takes over as it does for a radio that dropped out.
pub struct Standby {
    rate: f64,
    throttle: Throttle,
}

impl Standby {
    pub fn new(rate: f64) -> Self {
        let rate = if rate > 0.0 { rate } else { 48_000.0 };
        Standby { rate, throttle: Throttle::new(rate) }
    }
}

impl IqSource for Standby {
    fn sample_rate(&self) -> f64 {
        self.rate
    }

    fn center_hz(&self) -> f64 {
        0.0
    }

    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(read_len(self.rate));
        buf[..n].fill(Complex32::new(0.0, 0.0));
        self.throttle.pace(n);
        Ok(n)
    }

    fn describe(&self) -> String {
        "waiting for the radio after a playback".into()
    }

    fn open_status(&self) -> Option<String> {
        Some("Playback ended — the radio did not come back yet; reconnecting…".into())
    }

    fn needs_reopen(&self) -> bool {
        true
    }
}

/// How many samples one read hands over at `rate`: [`READ_SECONDS`] of them.
fn read_len(rate: f64) -> usize {
    ((rate * READ_SECONDS) as usize).max(1)
}

/// The capabilities a playback runs under: receive-only audio, and a dial that
/// can be set to anything, since it only labels the recording.
pub fn playback_caps(name: &str) -> sdroxide_types::DeviceCaps {
    sdroxide_types::DeviceCaps {
        driver: "playback".into(),
        label: format!("Playback — {name}"),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: true,
        freq_ranges_rx: vec![(0.0, 300e9)],
        ..sdroxide_types::DeviceCaps::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit WAV of `secs` seconds of a `hz` tone at `rate`.
    fn tone_wav(path: &Path, rate: u32, channels: u16, secs: f64, hz: f64) {
        let n = (rate as f64 * secs) as usize;
        let mut data = Vec::with_capacity(n * 2 * channels as usize);
        for i in 0..n {
            let v = (std::f64::consts::TAU * hz * i as f64 / rate as f64).sin() * 0.5;
            let s = (v * i16::MAX as f64) as i16;
            for ch in 0..channels {
                // The right ear carries something else entirely, so a test can
                // tell which channel was played.
                let s = if ch == 0 { s } else { i16::MAX / 10 };
                data.extend_from_slice(&s.to_le_bytes());
            }
        }
        let mut f = Vec::new();
        let block = 2 * channels as u32;
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&channels.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * block).to_le_bytes());
        f.extend_from_slice(&(block as u16).to_le_bytes());
        f.extend_from_slice(&16u16.to_le_bytes());
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(data.len() as u32).to_le_bytes());
        f.extend_from_slice(&data);
        std::fs::write(path, f).unwrap();
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sdroxide-playback-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// Mean absolute value of the first ear — a 0.5-amplitude sine averages
    /// about 0.3.
    fn level(s: &[Complex32]) -> f32 {
        s.iter().map(|x| x.re.abs()).sum::<f32>() / s.len() as f32
    }

    /// The whole file comes out, at its own rate and length, from its first
    /// channel only, and then silence — the end is held, not looped.
    #[test]
    fn a_file_plays_once_from_its_first_channel() {
        let path = scratch("once.wav");
        tone_wav(&path, 8000, 2, 1.0, 1000.0);
        let mut src = PlaybackSource::open(&path, "once.wav", 14.23e6).unwrap();
        assert_eq!(src.sample_rate(), 8000.0);
        let st = src.status();
        assert_eq!((st.channels, st.rate_hz), (2, 8000));
        assert!((st.duration_s.unwrap() - 1.0).abs() < 1e-3, "{:?}", st.duration_s);

        let mut out = vec![Complex32::default(); 8000];
        src.fill(&mut out);
        // A 1 kHz sine sampled at 8 kHz lands on eight exact phases, so its
        // mean |x| is 0.302 rather than the continuous 0.318; the right
        // channel is a constant 0.1, so either ear is unmistakable.
        let l = level(&out);
        assert!((l - 0.302).abs() < 0.01, "first channel level {l}");
        assert!((src.status().position_s - 1.0).abs() < 1e-3);

        src.fill(&mut out);
        assert!(src.status().ended);
        assert_eq!(level(&out), 0.0, "played past its end");
    }

    /// Paused, the position stands still and the decoders get silence; a seek
    /// lands where it was asked; looping goes round instead of ending.
    #[test]
    fn pause_seek_and_loop_do_what_they_say() {
        let path = scratch("steer.wav");
        tone_wav(&path, 8000, 1, 2.0, 700.0);
        let mut src = PlaybackSource::open(&path, "steer.wav", 0.0).unwrap();
        let mut out = vec![Complex32::default(); 4000];

        src.control(&PlaybackCommand::Pause(true)).unwrap();
        src.fill(&mut out);
        assert_eq!(level(&out), 0.0);
        assert_eq!(src.status().position_s, 0.0);

        src.control(&PlaybackCommand::Pause(false)).unwrap();
        src.control(&PlaybackCommand::Seek(1.5)).unwrap();
        assert!((src.status().position_s - 1.5).abs() < 0.01, "{}", src.status().position_s);
        src.fill(&mut out); // the last half second
        assert!(level(&out) > 0.25);
        assert!((src.status().position_s - 2.0).abs() < 0.01);

        src.control(&PlaybackCommand::Loop(true)).unwrap();
        src.fill(&mut out);
        assert!(!src.status().ended, "a looping file ended");
        assert!(level(&out) > 0.25, "the loop played silence");
        assert!(src.status().position_s < 1.0, "{}", src.status().position_s);

        // Past the end is clamped to the end, not an error.
        src.control(&PlaybackCommand::Loop(false)).unwrap();
        src.control(&PlaybackCommand::Seek(99.0)).unwrap();
        src.fill(&mut out);
        assert!(src.status().ended);
    }

    /// Power of `hz` in `s` at `rate` (Goertzel), for telling two tones apart.
    fn power_at(s: &[f32], rate: f64, hz: f64) -> f64 {
        let k = 2.0 * (std::f64::consts::TAU * hz / rate).cos();
        let (mut a, mut b) = (0.0f64, 0.0f64);
        for &x in s {
            let c = x as f64 + k * a - b;
            b = a;
            a = c;
        }
        a * a + b * b - k * a * b
    }

    /// The case the feature exists for: a file **the recorder itself wrote**.
    /// An MP3 from the recorder carries no length in its header, so this also
    /// proves the length is counted; and its right ear is our own transmit, so
    /// the receiver has to be the ear that is played.
    #[test]
    fn the_recorders_own_mp3_plays_back() {
        use crate::recorder::{Recorder, RecordingChannels};
        use sdroxide_types::RecordingFormat;
        let path = scratch("sdroxide_2026-10-06_10-00-05Z_14.230000MHz_SSTV.mp3");
        let _ = std::fs::remove_file(&path);
        let (rec, mut prod) = Recorder::start(
            path.clone(),
            48_000.0,
            RecordingChannels::Stereo,
            RecordingFormat::Mp3,
        )
        .unwrap();
        for i in 0..96_000 {
            let t = i as f32 / 48_000.0;
            let l = 0.5 * (std::f32::consts::TAU * 1000.0 * t).sin();
            let r = 0.5 * (std::f32::consts::TAU * 400.0 * t).sin();
            for s in [l, r] {
                while prod.push(s).is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        rec.stop();

        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let mut src = PlaybackSource::open(&path, &name, 0.0).unwrap();
        let st = src.status();
        assert_eq!((st.rate_hz, st.channels), (48_000, 2));
        let dur = st.duration_s.expect("the length was not counted");
        assert!((dur - 2.0).abs() < 0.15, "length {dur}");
        let stamp = st.stamp.expect("the name's stamp");
        assert_eq!(stamp.mode, Some(sdroxide_types::Mode::Sstv));
        assert!((stamp.freq_hz - 14_230_000.0).abs() < 0.5);

        let mut out = vec![Complex32::default(); 48_000];
        src.fill(&mut out);
        let left: Vec<f32> = out[4800..43_200].iter().map(|c| c.re).collect();
        let right: Vec<f32> = out[4800..43_200].iter().map(|c| c.im).collect();
        let (rx, tx) = (power_at(&left, 48_000.0, 1000.0), power_at(&left, 48_000.0, 400.0));
        assert!(rx > tx * 100.0, "left (rx) {rx:.3e} against right (tx) {tx:.3e}");
        // And the right ear comes back as the right ear, not a copy of the
        // left: a stereo recording plays back in stereo.
        let (r_own, r_left) =
            (power_at(&right, 48_000.0, 400.0), power_at(&right, 48_000.0, 1000.0));
        assert!(r_own > r_left * 100.0, "right {r_own:.3e} against left {r_left:.3e}");
        assert!(src.audio_stereo());

        // And a seek into an MP3 lands where it was asked.
        src.control(&PlaybackCommand::Seek(1.25)).unwrap();
        assert!((src.status().position_s - 1.25).abs() < 0.05, "{}", src.status().position_s);
    }

    /// The cause of the chopped audio reported on the first build: a read
    /// handed the engine its whole 16 384-sample block at once — a third of a
    /// second at 48 kHz, more than the speaker queue holds. A read is now a
    /// sound card's 10 ms, whatever the engine asks for, and a mono file plays
    /// the same sample in both ears.
    #[test]
    fn a_read_is_a_sound_cards_block_not_the_engines() {
        let path = scratch("block.wav");
        tone_wav(&path, 48_000, 1, 1.0, 1000.0);
        let mut src = PlaybackSource::open(&path, "block.wav", 0.0).unwrap();
        let mut buf = vec![Complex32::default(); 16_384];
        let n = src.read(&mut buf).unwrap();
        assert_eq!(n, 480, "one read handed over {n} samples");
        assert!(buf[..n].iter().all(|c| c.re == c.im), "a mono file is not the same in both ears");
        assert!(!src.audio_stereo());
    }

    /// Not every file is audio, and the operator is told rather than given a
    /// silent radio.
    #[test]
    fn a_file_that_is_not_audio_is_refused() {
        let path = scratch("notes.mp3");
        std::fs::write(&path, b"this is not an mp3 at all, just some text").unwrap();
        let e = PlaybackSource::open(&path, "notes.mp3", 0.0).err().expect("opened");
        assert!(e.to_string().contains("audio"), "{e}");
    }
}
