//! Radiosonde (Vaisala RS41) decode: what the operator asks the engine for and
//! what it reports back. Pure data + serde, shared by the native engine and
//! the UI (native + WASM). The demodulation lives in the native
//! `sdroxide-sonde` crate, the same split as [`crate::hfdl`].
//!
//! Like HFDL and QO-100, the sonde lane is not a radio mode: it rides its own
//! downconversion of the raw I/Q (the 400–406 MHz meteorological band) and the
//! radio keeps its mode. Only RS41 is decoded for now; the model name travels
//! with every track so the other types can follow without a wire change.

use serde::{Deserialize, Serialize};

/// The default channel: the middle of the 400.15–406 MHz meteorological band,
/// where most European launches sit (403.0 MHz is a common RS41 frequency).
pub const SONDE_DEFAULT_HZ: f64 = 403_000_000.0;

/// Lowest and highest frequency the lane accepts (the 400.15–406 MHz
/// meteorological-aids band, with a little margin).
pub const SONDE_BAND_HZ: (f64, f64) = (400_000_000.0, 406_100_000.0);

/// Whether a sample window of `rate_hz` centred on `center_hz` reaches any of
/// the sonde band. The decoder lanes mix from the window, so outside it there
/// is nothing to decode and nothing worth scanning: the engine feeds them only
/// while this is true (a window parked on 1090 MHz for ADS-B, or on the
/// shortwave for HFDL, used to be scanned for sondes all the same).
pub fn window_reaches_band(center_hz: f64, rate_hz: f64) -> bool {
    let half = rate_hz.abs() / 2.0;
    center_hz + half > SONDE_BAND_HZ.0 && center_hz - half < SONDE_BAND_HZ.1
}

/// How many sondes the status carries at once (the most recently heard).
pub const SONDE_MAX_TRACKS: usize = 8;

/// How many track points a sonde keeps (one per frame, thinned when full).
pub const SONDE_TRACK_DEPTH: usize = 3600;

/// What the operator asks the radiosonde decoder to do.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SondeSettings {
    /// Whether the decoder runs at all. Off by default: a sometimes-reached
    /// utility lane that costs a downconverter and a worker thread.
    pub enabled: bool,
    /// The sonde channel the lane centres on, in Hz.
    pub frequency_hz: f64,
    /// Look for sondes across the whole sample window instead of listening on
    /// one channel: a few are decoded at once, one lane per carrier found.
    /// Appended last: postcard numbers fields by position, and the config file
    /// of a version without it still loads (`default`).
    #[serde(default)]
    pub scan: bool,
}

impl Default for SondeSettings {
    fn default() -> Self {
        Self { enabled: false, frequency_hz: SONDE_DEFAULT_HZ, scan: false }
    }
}

impl SondeSettings {
    /// The settings with the frequency inside the sonde band, applied where
    /// they arrive (a config file an operator may have edited, a remote
    /// client) rather than where they are used.
    pub fn sane(mut self) -> SondeSettings {
        self.frequency_hz = self.frequency_hz.clamp(SONDE_BAND_HZ.0, SONDE_BAND_HZ.1);
        self
    }

    /// The settings after the operator moved the dial to `dial_hz`, or `None`
    /// when the channel stays. With the decoder on and one channel (no scan) the
    /// dial and the channel are two controls for one thing, as in HFDL: tuning
    /// the dial onto a sonde in the band must make the lane decode *that*
    /// signal, not sit on the channel box's old value while the operator hears
    /// packets and nothing is decoded.
    pub fn following_dial(&self, dial_hz: f64) -> Option<SondeSettings> {
        let in_band = (SONDE_BAND_HZ.0..=SONDE_BAND_HZ.1).contains(&dial_hz);
        (self.enabled && !self.scan && in_band && (self.frequency_hz - dial_hz).abs() >= 1.0)
            .then_some(SondeSettings { frequency_hz: dial_hz, ..*self })
    }
}

/// One point of a sonde's flight.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SondePoint {
    pub lat: f64,
    pub lon: f64,
    pub alt_m: f32,
    /// Unix second the frame was heard.
    pub unix: i64,
    /// The latest air readings when the point was kept, when the model sends them.
    pub temp_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
}

/// One radiosonde the decoder has heard: its identity, its latest frame and
/// the track it has flown since it was first heard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SondeTrack {
    /// The sonde type, e.g. "RS41".
    pub model: String,
    /// The serial number printed on the sonde, e.g. "N1234567".
    pub serial: String,
    /// The channel it was heard on, in Hz.
    pub freq_hz: f64,
    /// The sonde's own frame counter.
    pub frame: u32,
    /// Latest position; `None` until the GPS has a fix.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub alt_m: Option<f32>,
    /// Horizontal speed, m/s.
    pub speed_ms: Option<f32>,
    /// Course over ground, degrees.
    pub course_deg: Option<f32>,
    /// Climb rate (positive up), m/s.
    pub climb_ms: Option<f32>,
    /// GPS satellites in the solution.
    pub num_sv: Option<u8>,
    /// Battery voltage, V.
    pub battery_v: Option<f32>,
    /// Air temperature, °C, when the model sends it and it is calibrated.
    pub temp_c: Option<f32>,
    /// Relative humidity, %.
    pub humidity_pct: Option<f32>,
    /// Air pressure, hPa.
    pub pressure_hpa: Option<f32>,
    /// Unix second the latest frame was heard.
    pub last_unix: i64,
    /// Frames decoded from this sonde since it was first heard.
    pub frames: u32,
    /// Where it has been, oldest first, bounded by [`SONDE_TRACK_DEPTH`].
    pub track: Vec<SondePoint>,
}

/// What the engine tells the window about the decoder's own state. Re-sent
/// whenever it changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SondeStatus {
    /// Whether the downconverter and worker are actually running.
    pub running: bool,
    /// Channel level in dBFS, so a lane that is up but hears nothing says so.
    pub level_dbfs: f32,
    /// Frames decoded since the lane was switched on.
    pub frames: u64,
    /// The sondes heard, most recently heard first.
    pub sondes: Vec<SondeTrack>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_channel_follows_the_dial_only_when_one_channel_is_decoded_in_the_band() {
        let on = SondeSettings { enabled: true, frequency_hz: 402_700_000.0, scan: false };
        let moved = on.following_dial(401_002_000.0).expect("follows");
        assert_eq!(moved.frequency_hz, 401_002_000.0);
        assert!(moved.enabled && !moved.scan);
        assert!(on.following_dial(402_700_000.4).is_none(), "already there");
        assert!(on.following_dial(145_500_000.0).is_none(), "outside the band");
        assert!(SondeSettings { enabled: false, ..on }.following_dial(401e6).is_none());
        assert!(SondeSettings { scan: true, ..on }.following_dial(401e6).is_none());
    }

    #[test]
    fn only_a_window_on_the_band_is_worth_decoding() {
        assert!(window_reaches_band(403_000_000.0, 2_500_000.0));
        assert!(window_reaches_band(399_000_000.0, 2_500_000.0), "its upper edge reaches in");
        assert!(!window_reaches_band(1_090_000_000.0, 2_500_000.0), "ADS-B window");
        assert!(!window_reaches_band(8_912_000.0, 2_500_000.0), "HFDL window");
        assert!(!window_reaches_band(136_800_000.0, 2_500_000.0), "VDL2 window");
    }

    #[test]
    fn the_saved_channel_is_brought_into_the_sonde_band() {
        assert_eq!(SondeSettings::default().sane().frequency_hz, SONDE_DEFAULT_HZ);
        assert_eq!(
            SondeSettings { frequency_hz: 6.0e9, ..Default::default() }.sane().frequency_hz,
            SONDE_BAND_HZ.1
        );
        assert_eq!(
            SondeSettings { frequency_hz: 0.0, ..Default::default() }.sane().frequency_hz,
            SONDE_BAND_HZ.0
        );
        let s = SondeSettings { enabled: true, frequency_hz: 404_500_000.0, scan: true };
        assert_eq!(s.sane(), s);
    }

    #[test]
    fn a_status_round_trips_through_postcard() {
        let st = SondeStatus {
            running: true,
            level_dbfs: -40.0,
            frames: 3,
            sondes: vec![SondeTrack {
                model: "RS41".into(),
                serial: "N1234567".into(),
                freq_hz: 403.0e6,
                frame: 77,
                lat: Some(48.8),
                lon: Some(2.3),
                alt_m: Some(1234.5),
                speed_ms: Some(5.0),
                course_deg: Some(90.0),
                climb_ms: Some(5.2),
                num_sv: Some(9),
                battery_v: Some(2.7),
                temp_c: None,
                humidity_pct: None,
                pressure_hpa: None,
                last_unix: 1,
                frames: 3,
                track: vec![SondePoint {
                    lat: 48.8,
                    lon: 2.3,
                    alt_m: 1234.5,
                    unix: 1,
                    temp_c: None,
                    humidity_pct: None,
                    pressure_hpa: None,
                }],
            }],
        };
        let bytes = postcard::to_stdvec(&st).unwrap();
        let back: SondeStatus = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, st);
    }
}
