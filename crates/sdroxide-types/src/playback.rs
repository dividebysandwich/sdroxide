//! Playing a recording back into the decoders (fork discussion #10).
//!
//! The recorder writes demodulated audio and names every file with the moment,
//! the frequency and the mode it was made at. Playback is the other half: the
//! engine stands its radio down for the length of the playback, plays the file
//! through the same path a sound-card radio's audio takes — waterfall,
//! decoders, speakers — and puts the radio back when the operator stops.
//!
//! This module is the part both ends need: reading the stamp back out of a
//! name, the list of files the station has, and the control and status that
//! cross the wire. Nothing here touches a file.

use serde::{Deserialize, Serialize};

use crate::Mode;

/// What a recording's own name says about it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RecordingStamp {
    /// When the recording started, Unix seconds UTC.
    pub unix: i64,
    /// The dial frequency the radio was on, in hertz.
    pub freq_hz: f64,
    /// The mode it was made in, when the name carries one this build knows.
    pub mode: Option<Mode>,
    /// The radio that made it: 0 for the first, `n` for `_radio{n}`.
    pub radio: u32,
}

/// How a mode is written into a recording's name: its label with the
/// characters a file name should not carry taken out. The engine names files
/// with this and [`parse_recording_name`] reads it back, so the two cannot
/// drift apart.
pub fn recording_mode_tag(mode: Mode) -> String {
    mode.label().replace(['/', ' '], "")
}

/// Read the stamp back out of a recording's name:
/// `sdroxide[_radioN]_YYYY-MM-DD_HH-MM-SSZ_<MHz>MHz_<MODE>.<ext>`.
///
/// Something in front of `sdroxide_` (a download's prefix) is ignored. Returns
/// `None` for any other name — a file somebody copied in, or one of the I/Q
/// captures. A mode tag this build does not know leaves `mode` empty
/// rather than refusing the whole stamp: the time and the frequency are still
/// worth showing.
pub fn parse_recording_name(name: &str) -> Option<RecordingStamp> {
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    // Anything in front is a copy's own doing (a download prefix, "copy of").
    let rest = &stem[stem.find("sdroxide_")? + "sdroxide".len()..];
    let (radio, rest) = match rest.strip_prefix("_radio") {
        Some(r) => {
            let end = r.find('_')?;
            (r[..end].parse::<u32>().ok()?, &r[end..])
        }
        None => (0, rest),
    };
    let mut parts = rest.strip_prefix('_')?.splitn(4, '_');
    let date = parts.next()?;
    let time = parts.next()?.strip_suffix('Z')?;
    let freq = parts.next()?.strip_suffix("MHz")?;
    let mode_tag = parts.next()?;

    let mut d = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, mo, da) = (d.next()??, d.next()??, d.next()??);
    let mut t = time.split('-').map(|x| x.parse::<i64>().ok());
    let (h, mi, s) = (t.next()??, t.next()??, t.next()??);
    if d.next().is_some() || t.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&mo) || !(1..=31).contains(&da) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let mhz: f64 = freq.parse().ok()?;
    if !mhz.is_finite() || mhz < 0.0 {
        return None;
    }
    let mode =
        Mode::ALL.iter().copied().find(|m| recording_mode_tag(*m).eq_ignore_ascii_case(mode_tag));
    Some(RecordingStamp {
        unix: days_from_civil(y, mo, da) * 86_400 + h * 3600 + mi * 60 + s,
        freq_hz: mhz * 1e6,
        mode,
        radio,
    })
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The file extensions playback can read: the recorder's MP3, and WAV, OGG
/// Vorbis and FLAC for audio made elsewhere.
pub const PLAYABLE_EXTENSIONS: [&str; 4] = ["mp3", "wav", "ogg", "flac"];

/// Whether a name looks like something playback can open — by extension, and
/// never a path. The engine refuses anything else before it touches the disk;
/// a remote client names a file, it does not get to name a directory.
pub fn playable_name(name: &str) -> bool {
    if name.is_empty()
        || name.contains(['/', '\\', '\0'])
        || name.starts_with('.')
        || name.contains("..")
    {
        return false;
    }
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| PLAYABLE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

/// One file in the station's recordings directory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordingFile {
    /// The file's name, which is also how [`PlaybackCommand::Open`] asks for it.
    pub name: String,
    /// Its size on disk, in bytes.
    pub bytes: u64,
    /// When it was last written, Unix seconds UTC (0 when unknown).
    pub modified_unix: i64,
    /// An I/Q capture rather than audio. Listed so the operator is told why it
    /// cannot be played here, rather than wondering where it went.
    pub iq: bool,
}

impl RecordingFile {
    /// The stamp in this file's name, if it carries one.
    pub fn stamp(&self) -> Option<RecordingStamp> {
        parse_recording_name(&self.name)
    }
}

/// The answer to [`crate::Command::ListRecordings`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordingListing {
    /// The directory, as the station sees it — shown, so the operator knows
    /// where the files are.
    pub dir: String,
    /// Newest first.
    pub files: Vec<RecordingFile>,
    /// Why the directory could not be read, when it could not.
    pub error: Option<String>,
}

/// What the operator asks of playback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlaybackCommand {
    /// Play this file from the recordings directory, from the start. Also
    /// tunes the dial and picks the mode the file's name records, when it
    /// records them.
    Open { name: String },
    /// Pause (`true`) or carry on (`false`).
    Pause(bool),
    /// Go to this many seconds into the file.
    Seek(f64),
    /// Start again from the top when the file ends, rather than stopping there.
    Loop(bool),
    /// End playback and put the radio back as it was.
    Stop,
    /// Delete this file from the recordings directory, for good — the
    /// recordings pile up and nothing else on the station removes them. Refused
    /// for the file playing back and for one still being written. Appended
    /// last: this enum rides the wire.
    Delete { name: String },
}

/// Where playback is, for the transport bar. `None` on the state when nothing
/// is playing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackStatus {
    /// The file's name.
    pub name: String,
    /// Seconds into the file.
    pub position_s: f64,
    /// The file's length in seconds, when it could be measured.
    pub duration_s: Option<f64>,
    pub paused: bool,
    pub looping: bool,
    /// The file has played to its end and is holding there (looping off).
    pub ended: bool,
    /// The file's own sample rate.
    pub rate_hz: u32,
    /// How many channels the file has. Only the first is played: on a stereo
    /// recording that is the receiver, the right being our own transmit.
    pub channels: u16,
    /// What the file's name records, if anything.
    pub stamp: Option<RecordingStamp>,
    /// Why playback went silent, when the file broke under it.
    pub fault: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name the engine writes reads back to what it was written from —
    /// every mode, so a label that lost its round trip (a mode whose tag
    /// collides with another's) fails here rather than playing a file in the
    /// wrong decoder.
    #[test]
    fn every_mode_reads_back_from_its_own_name() {
        for mode in Mode::ALL {
            let name = format!(
                "sdroxide_2026-10-06_10-00-05Z_14.230000MHz_{}.mp3",
                recording_mode_tag(mode)
            );
            let stamp = parse_recording_name(&name).unwrap_or_else(|| panic!("{name} unread"));
            assert_eq!(stamp.mode, Some(mode), "{name}");
        }
    }

    #[test]
    fn the_stamp_carries_time_frequency_and_radio() {
        let s = parse_recording_name("sdroxide_radio2_2026-10-06_10-00-05Z_27.265000MHz_FT8.mp3")
            .unwrap();
        assert_eq!(s.radio, 2);
        assert_eq!(s.mode, Some(Mode::Ft8));
        assert!((s.freq_hz - 27_265_000.0).abs() < 0.5);
        // 2026-10-06 10:00:05 UTC.
        assert_eq!(s.unix, 1_791_280_805);

        let first =
            parse_recording_name("sdroxide_1970-01-01_00-00-00Z_0.010000MHz_HDRADIO.mp3").unwrap();
        assert_eq!((first.unix, first.radio, first.mode), (0, 0, Some(Mode::HdRadio)));
    }

    /// An unknown mode keeps the rest of the stamp; anything else that is not
    /// our pattern is not a stamp at all.
    #[test]
    fn foreign_names_are_not_stamps() {
        let s =
            parse_recording_name("sdroxide_2026-10-06_10-00-05Z_7.040000MHz_NEWMODE.mp3").unwrap();
        assert_eq!(s.mode, None);
        for name in [
            "capture.mp3",
            "SDRoxide_20261006_100005Z_14200000Hz_192000sps_IQ.wav",
            "sdroxide_2026-13-06_10-00-05Z_7.040000MHz_USB.mp3",
            "sdroxide_2026-10-06_10-00-05_7.040000MHz_USB.mp3",
            "sdroxide_2026-10-06_10-00-05Z_7.04kHz_USB.mp3",
            "sdroxide_radioX_2026-10-06_10-00-05Z_7.040000MHz_USB.mp3",
        ] {
            assert_eq!(parse_recording_name(name), None, "{name}");
        }
    }

    /// A downloaded or uploaded copy arrives with something in front of the
    /// name (`487e8df6-sdroxide_…`); the stamp is still the file's own, and
    /// without it the mode and the dial are left alone and nothing decodes.
    #[test]
    fn a_prefixed_copy_keeps_its_stamp() {
        let s = parse_recording_name("487e8df6-sdroxide_2026-10-18_14-39-56Z_2.185800MHz_DSC.mp3")
            .expect("the prefixed name is unread");
        assert_eq!(s.mode, Some(Mode::Dsc));
        assert!((s.freq_hz - 2_185_800.0).abs() < 0.5);
        assert_eq!(
            parse_recording_name(
                "copy of sdroxide_radio2_2026-10-06_10-00-05Z_27.265000MHz_FT8.mp3"
            )
            .map(|s| (s.radio, s.mode)),
            Some((2, Some(Mode::Ft8)))
        );
    }

    /// A name, never a path: the engine opens files only inside its own
    /// recordings directory, whoever asks.
    #[test]
    fn only_plain_audio_names_are_playable() {
        assert!(playable_name("sdroxide_2026-10-06_10-00-05Z_14.230000MHz_SSTV.mp3"));
        assert!(playable_name("pass.WAV"));
        assert!(playable_name("net.ogg"));
        for bad in [
            "",
            "../secret.mp3",
            "a/b.mp3",
            "a\\b.mp3",
            ".hidden.mp3",
            "x..mp3",
            "notes.txt",
            "noext",
        ] {
            assert!(!playable_name(bad), "{bad:?} accepted");
        }
    }
}
