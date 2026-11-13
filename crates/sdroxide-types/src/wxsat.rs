//! Weather-satellite image decode (NOAA APT and Meteor LRPT): what the
//! operator asks the engine for and what it reports back. Pure data + serde,
//! shared by the native engine and the UI (native + WASM); the demodulation
//! lives in the native `sdroxide-wxsat` crate, the same split as HFDL's.
//!
//! It is not a radio mode: it rides its own
//! downconversion of the raw I/Q (the 137 MHz band) and the radio keeps its
//! mode.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The default channel: NOAA-19, the APT bird most often heard.
pub const WXSAT_DEFAULT_HZ: f64 = 137_100_000.0;

/// The default Meteor LRPT channel: Meteor-M2-3.
pub const WXSAT_LRPT_DEFAULT_HZ: f64 = 137_900_000.0;

/// Pixels across a Meteor LRPT picture, and lines in a row of segments.
pub const WXSAT_LRPT_WIDTH: usize = 1568;
pub const WXSAT_LRPT_ROW_LINES: usize = 8;

/// Lowest and highest frequency the lane accepts (the 137–138 MHz
/// meteorological-satellite downlink band, with a little margin).
pub const WXSAT_BAND_HZ: (f64, f64) = (136_000_000.0, 138_100_000.0);

/// Pixels in one APT line: both image strips with their sync, space and
/// telemetry. The image strips are 909 pixels each, at 86 and 1126.
pub const WXSAT_LINE_PIXELS: usize = 2080;

/// Rows of LRPT picture a status carries at most (three channels, eight rows).
pub const WXSAT_MAX_NEW_ROWS: usize = 24;

/// Lines a status carries at most: a client that fell behind loses the oldest,
/// the picture shows a gap instead of the link choking.
pub const WXSAT_MAX_NEW_LINES: usize = 240;

/// Which weather-satellite signal the decoder reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WxsatKind {
    /// NOAA-15/18/19: FM with a 2400 Hz AM subcarrier, two lines a second.
    #[default]
    Apt,
    /// Meteor-M2: QPSK at 72 ksym/s carrying digital image segments.
    Lrpt,
}

impl WxsatKind {
    /// Hz either side of its channel that the whole signal needs.
    pub fn half_width_hz(self) -> f64 {
        match self {
            WxsatKind::Apt => 25_000.0,
            WxsatKind::Lrpt => 70_000.0,
        }
    }

    /// The channel the decoder opens on.
    pub fn default_hz(self) -> f64 {
        match self {
            WxsatKind::Apt => WXSAT_DEFAULT_HZ,
            WxsatKind::Lrpt => WXSAT_LRPT_DEFAULT_HZ,
        }
    }
}

/// Whether a sample window of `rate_hz` centred on `center_hz` holds the
/// channel at `freq_hz` with room for the whole signal of `kind`. The lane
/// mixes from the window, so outside it there is nothing to decode.
pub fn window_covers(center_hz: f64, rate_hz: f64, freq_hz: f64, kind: WxsatKind) -> bool {
    let half = rate_hz.abs() / 2.0 - kind.half_width_hz();
    half > 0.0 && (freq_hz - center_hz).abs() <= half
}

/// What the operator asks the weather-satellite decoder to do.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WxsatSettings {
    /// Whether the decoder runs at all. Off by default: it costs a
    /// downconverter and a worker thread.
    pub enabled: bool,
    /// The downlink channel the lane centres on, in Hz.
    pub frequency_hz: f64,
    /// Which signal. Appended last: postcard numbers fields by position, and a
    /// `wxsat.json` written before it still loads (`default`: APT).
    #[serde(default)]
    pub kind: WxsatKind,
}

impl Default for WxsatSettings {
    fn default() -> Self {
        Self { enabled: false, frequency_hz: WXSAT_DEFAULT_HZ, kind: WxsatKind::Apt }
    }
}

impl WxsatSettings {
    /// The settings with the frequency inside the band, applied where they
    /// arrive (a config file an operator may have edited, a remote client)
    /// rather than where they are used.
    pub fn sane(mut self) -> WxsatSettings {
        self.frequency_hz = self.frequency_hz.clamp(WXSAT_BAND_HZ.0, WXSAT_BAND_HZ.1);
        self
    }
}

/// What the engine tells the window about the decoder. The picture itself
/// travels as the lines new since the last status, not whole.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WxsatStatus {
    /// Whether the downconverter and worker are actually running.
    pub running: bool,
    /// Channel level in dBFS, so a lane that is up but hears nothing says so.
    pub level_dbfs: f32,
    /// Whether a line sync is being followed right now.
    pub locked: bool,
    /// Correlation of the latest line with the sync pattern, 0..1.
    pub quality: f32,
    /// Counts up each time the decoder is switched on: a new number means a
    /// new picture, and the client starts a blank one.
    pub pass: u32,
    /// Lines decoded in this pass.
    pub total_lines: u32,
    /// Index, in the picture, of the first of `new_lines`.
    pub first_line: u32,
    /// Lines decoded since the previous status, [`WXSAT_LINE_PIXELS`] each.
    pub new_lines: Vec<Vec<u8>>,
    /// Which decoder these figures are from. Appended last.
    pub kind: WxsatKind,
    /// LRPT: frames that passed Reed-Solomon in this pass.
    pub frames_ok: u32,
    /// LRPT: the carrier's offset from the channel, Hz.
    pub carrier_hz: f32,
    /// LRPT: rows of picture finished since the previous status, one per
    /// channel and row.
    pub rows: Vec<WxsatRow>,
}

/// One finished row of one LRPT channel: eight lines of
/// [`WXSAT_LRPT_WIDTH`] pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WxsatRow {
    /// The channel's APID: 64 to 69.
    pub apid: u16,
    /// Row number from the top of the picture.
    pub row: u32,
    pub lines: Vec<u8>,
}

/// How many lines of a picture a client keeps: about 50 minutes, longer than
/// any pass. Past it the oldest are dropped.
pub const WXSAT_PICTURE_MAX_LINES: usize = 6000;

/// The picture a client builds from the statuses: lines placed by their
/// number, a missed one left empty, a new `pass` starting over.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WxsatPicture {
    pub pass: u32,
    /// One entry per line; empty where the line never arrived.
    pub lines: Vec<Vec<u8>>,
    /// Lines dropped from the front because the picture was full.
    pub dropped: usize,
    /// LRPT: each channel's rows by row number, `WXSAT_LRPT_ROW_LINES` lines of
    /// `WXSAT_LRPT_WIDTH` pixels each; empty where a row never came.
    pub channels: BTreeMap<u16, Vec<Vec<u8>>>,
}

/// How many rows of a channel a client keeps: about 50 minutes, longer than a
/// pass.
pub const WXSAT_LRPT_MAX_ROWS: usize = 2400;

impl WxsatPicture {
    /// Take what a status carries.
    pub fn apply(&mut self, st: &WxsatStatus) {
        if st.pass != self.pass {
            *self = WxsatPicture { pass: st.pass, ..Default::default() };
        }
        for (i, l) in st.new_lines.iter().enumerate() {
            if l.len() != WXSAT_LINE_PIXELS {
                continue;
            }
            let Some(at) = (st.first_line as usize + i).checked_sub(self.dropped) else { continue };
            if at >= self.lines.len() {
                self.lines.resize(at + 1, Vec::new());
            }
            self.lines[at] = l.clone();
        }
        for r in &st.rows {
            let row = r.row as usize;
            if r.lines.len() != WXSAT_LRPT_WIDTH * WXSAT_LRPT_ROW_LINES
                || row >= WXSAT_LRPT_MAX_ROWS
            {
                continue;
            }
            let ch = self.channels.entry(r.apid).or_default();
            if ch.len() <= row {
                ch.resize(row + 1, Vec::new());
            }
            ch[row] = r.lines.clone();
        }
        if self.lines.len() > WXSAT_PICTURE_MAX_LINES {
            let extra = self.lines.len() - WXSAT_PICTURE_MAX_LINES;
            self.lines.drain(..extra);
            self.dropped += extra;
        }
    }

    /// Forget the picture (the operator asked for a blank one).
    pub fn clear(&mut self) {
        self.lines.clear();
        self.dropped = 0;
        self.channels.clear();
    }

    /// Whether there is anything to show, APT lines or LRPT rows.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.channels.values().all(Vec::is_empty)
    }

    /// One LRPT channel as grey pixels, `WXSAT_LRPT_WIDTH` wide: the size and
    /// the pixels, a row that never came black.
    pub fn channel_gray(&self, apid: u16) -> Option<([usize; 2], Vec<u8>)> {
        let rows = self.channels.get(&apid)?;
        let row_px = WXSAT_LRPT_WIDTH * WXSAT_LRPT_ROW_LINES;
        let mut out = vec![0u8; rows.len() * row_px];
        for (i, r) in rows.iter().enumerate() {
            if r.len() == row_px {
                out[i * row_px..(i + 1) * row_px].copy_from_slice(r);
            }
        }
        Some(([WXSAT_LRPT_WIDTH, rows.len() * WXSAT_LRPT_ROW_LINES], out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_is_kept_in_the_band() {
        let s = WxsatSettings { enabled: true, frequency_hz: 90e6, kind: WxsatKind::Apt }.sane();
        assert_eq!(s.frequency_hz, WXSAT_BAND_HZ.0);
        let s = WxsatSettings { enabled: true, frequency_hz: 1e9, kind: WxsatKind::Lrpt }.sane();
        assert_eq!(s.frequency_hz, WXSAT_BAND_HZ.1);
        assert_eq!(WxsatSettings::default().sane(), WxsatSettings::default());
    }

    #[test]
    fn the_window_must_hold_the_whole_signal() {
        // 2.4 MS/s centred on the channel: fine; a channel at the very edge: not.
        assert!(window_covers(137.1e6, 2.4e6, 137.1e6, WxsatKind::Apt));
        assert!(!window_covers(137.1e6, 2.4e6, 138.3e6, WxsatKind::Apt));
        assert!(!window_covers(137.1e6, 2.4e6, 138.29e6, WxsatKind::Apt));
        // LRPT is wider: 70 kHz each side.
        assert!(window_covers(137.1e6, 2.4e6, 138.26e6, WxsatKind::Apt));
        assert!(!window_covers(137.1e6, 2.4e6, 138.26e6, WxsatKind::Lrpt));
        assert!(window_covers(137.1e6, 2.4e6, 137.1e6 + 1.1e6, WxsatKind::Apt));
        // A narrow window cannot hold a 40 kHz signal.
        assert!(!window_covers(137.1e6, 48_000.0, 137.1e6, WxsatKind::Apt));
    }

    fn status(pass: u32, first: u32, n: usize) -> WxsatStatus {
        WxsatStatus {
            pass,
            first_line: first,
            total_lines: first + n as u32,
            new_lines: (0..n)
                .map(|i| vec![(first as usize + i) as u8; WXSAT_LINE_PIXELS])
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn the_picture_places_lines_by_number_and_leaves_gaps() {
        let mut p = WxsatPicture::default();
        p.apply(&status(1, 0, 2));
        p.apply(&status(1, 5, 1)); // three statuses were missed
        assert_eq!(p.lines.len(), 6);
        assert!(p.lines[2].is_empty() && p.lines[4].is_empty());
        assert_eq!(p.lines[5][0], 5);
        // The same lines again change nothing.
        p.apply(&status(1, 0, 2));
        assert_eq!(p.lines.len(), 6);
    }

    #[test]
    fn a_new_pass_starts_a_blank_picture() {
        let mut p = WxsatPicture::default();
        p.apply(&status(1, 0, 3));
        p.apply(&status(2, 0, 1));
        assert_eq!((p.pass, p.lines.len()), (2, 1));
    }

    #[test]
    fn a_full_picture_drops_the_oldest_and_keeps_numbering() {
        let mut p = WxsatPicture::default();
        p.apply(&status(1, 0, 1));
        p.apply(&status(1, WXSAT_PICTURE_MAX_LINES as u32 + 9, 1));
        assert_eq!(p.lines.len(), WXSAT_PICTURE_MAX_LINES);
        assert_eq!(p.dropped, 10);
        assert_eq!(p.lines.last().unwrap()[0], (WXSAT_PICTURE_MAX_LINES + 9) as u8);
        // A line older than what is kept is ignored, not placed wrongly.
        p.apply(&status(1, 0, 1));
        assert_eq!(p.lines.len(), WXSAT_PICTURE_MAX_LINES);
    }

    fn row(apid: u16, n: u32, v: u8) -> WxsatRow {
        WxsatRow { apid, row: n, lines: vec![v; WXSAT_LRPT_WIDTH * WXSAT_LRPT_ROW_LINES] }
    }

    #[test]
    fn lrpt_rows_are_placed_by_number_per_channel() {
        let mut p = WxsatPicture::default();
        p.apply(&WxsatStatus {
            pass: 1,
            rows: vec![row(64, 0, 10), row(65, 0, 20), row(64, 2, 30)],
            ..Default::default()
        });
        assert!(!p.is_empty());
        let ([w, h], g) = p.channel_gray(64).unwrap();
        assert_eq!((w, h), (1568, 24));
        assert_eq!((g[0], g[8 * 1568], g[16 * 1568]), (10, 0, 30), "row 1 never came");
        assert_eq!(p.channel_gray(65).unwrap().0[1], 8);
        assert!(p.channel_gray(66).is_none());
        // A row of the wrong size is refused, a new pass starts over.
        p.apply(&WxsatStatus {
            pass: 1,
            rows: vec![WxsatRow { apid: 64, row: 5, lines: vec![1; 10] }],
            ..Default::default()
        });
        assert_eq!(p.channels[&64].len(), 3);
        p.apply(&WxsatStatus { pass: 2, ..Default::default() });
        assert!(p.is_empty());
    }

    #[test]
    fn an_old_settings_file_without_a_kind_reads_as_apt() {
        let old = r#"{"enabled":true,"frequency_hz":137620000.0}"#;
        let s: WxsatSettings = serde_json::from_str(old).unwrap();
        assert_eq!(s.kind, WxsatKind::Apt);
        assert_eq!(WxsatKind::Lrpt.default_hz(), WXSAT_LRPT_DEFAULT_HZ);
    }

    #[test]
    fn status_round_trips_through_serde_json() {
        let st = WxsatStatus {
            running: true,
            pass: 2,
            total_lines: 3,
            first_line: 1,
            new_lines: vec![vec![7; 4]],
            ..Default::default()
        };
        let back: WxsatStatus = serde_json::from_str(&serde_json::to_string(&st).unwrap()).unwrap();
        assert_eq!(back, st);
    }
}
