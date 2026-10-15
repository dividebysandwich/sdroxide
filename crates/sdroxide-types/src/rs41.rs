//! Radiosonde telemetry types, shared by the native engine, the wire protocol
//! and the UI (native + WASM). Pure data + serde — the `*mod` subprocess
//! runner lives in the native `sdroxide-rs41` crate.
//!
//! A sonde table rather than a chronological log, for the same reason
//! [`crate::AdsbAircraft`] is a table: each sonde repeats itself every second
//! or so, and the question an operator asks is "what is in the air, and where",
//! not "what did frame 14_022 say".

use serde::{Deserialize, Serialize};

/// Typical meteorological radiosonde band centre (Hz). Individual sondes sit
/// elsewhere in ~400–406 MHz; the dial chooses the one being copied.
pub const RS41_BAND_LO_HZ: f64 = 400_000_000.0;
pub const RS41_BAND_HI_HZ: f64 = 406_000_000.0;
/// Where selecting SONDE mode parks the dial when it is outside the band.
pub const RS41_DEFAULT_HZ: f64 = 403_000_000.0;

/// Default DDC lane rate for RS41 / DFM / MTS01 (`*mod` at 48 kHz).
pub const RS41_LANE_RATE_HZ: f64 = 48_000.0;
/// DDC lane rate for M10 / M20 (`m10mod` / `m20mod` at 76.8 kHz).
pub const SONDE_M10_LANE_RATE_HZ: f64 = 76_800.0;

/// Default for [`Rs41Settings::drop_map_s`].
pub const RS41_DROP_MAP_S: u16 = 30;
/// Default for [`Rs41Settings::drop_list_s`].
pub const RS41_DROP_LIST_S: u16 = 180;
/// Default for [`Rs41Settings::history_points`].
pub const RS41_HISTORY_POINTS: u16 = 60;
/// Default for [`Rs41Settings::max_sondes`].
pub const RS41_MAX_SONDES: u16 = 40;

/// Which rs1729 `*mod` binary the decoder should run.
///
/// Append-only: postcard + serde both care about variant order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum SondeKind {
    #[default]
    Rs41 = 0,
    Dfm9 = 1,
    Dfm17 = 2,
    M10 = 3,
    M20 = 4,
    Mts01 = 5,
}

impl SondeKind {
    pub const ALL: &[SondeKind] = &[
        SondeKind::Rs41,
        SondeKind::Dfm9,
        SondeKind::Dfm17,
        SondeKind::M10,
        SondeKind::M20,
        SondeKind::Mts01,
    ];

    /// Short chip / list label.
    pub fn label(self) -> &'static str {
        match self {
            SondeKind::Rs41 => "RS41",
            SondeKind::Dfm9 => "DFM9",
            SondeKind::Dfm17 => "DFM17",
            SondeKind::M10 => "M10",
            SondeKind::M20 => "M20",
            SondeKind::Mts01 => "MTS01",
        }
    }

    /// SondeHub high-level `type` string.
    pub fn sondehub_type(self) -> &'static str {
        match self {
            SondeKind::Rs41 => "RS41",
            SondeKind::Dfm9 | SondeKind::Dfm17 => "DFM",
            SondeKind::M10 => "M10",
            SondeKind::M20 => "M20",
            SondeKind::Mts01 => "MTS01",
        }
    }

    /// SondeHub `manufacturer`.
    pub fn manufacturer(self) -> &'static str {
        match self {
            SondeKind::Rs41 => "Vaisala",
            SondeKind::Dfm9 | SondeKind::Dfm17 => "Graw",
            SondeKind::M10 | SondeKind::M20 => "Meteomodem",
            SondeKind::Mts01 => "Meteosis",
        }
    }

    /// rs1729 binary name on PATH.
    pub fn binary(self) -> &'static str {
        match self {
            SondeKind::Rs41 => "rs41mod",
            SondeKind::Dfm9 | SondeKind::Dfm17 => "dfm09mod",
            SondeKind::M10 => "m10mod",
            SondeKind::M20 => "m20mod",
            SondeKind::Mts01 => "mts01mod",
        }
    }

    /// IQ sample rate the `*mod` process expects.
    pub fn lane_rate_hz(self) -> f64 {
        match self {
            SondeKind::M10 | SondeKind::M20 => SONDE_M10_LANE_RATE_HZ,
            _ => RS41_LANE_RATE_HZ,
        }
    }

    pub fn is_dfm(self) -> bool {
        matches!(self, SondeKind::Dfm9 | SondeKind::Dfm17)
    }
}

/// One radiosonde, as everything heard from it so far.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rs41Sonde {
    /// Serial number printed on the sonde / in the status subframe (e.g. `U1234567`).
    pub serial: String,
    /// Absolute RF frequency the decoder last locked on, Hz.
    pub frequency_hz: f64,
    /// Which family produced this row.
    #[serde(default)]
    pub kind: SondeKind,
    /// Detailed model from the decoder JSON, when known (e.g. `RS41-SGP`, `DFM17`).
    #[serde(default)]
    pub subtype: Option<String>,
    /// Manufacturer string for SondeHub / the panel.
    #[serde(default)]
    pub manufacturer: String,
    /// Decoder frame counter (or unix-ish frame for DFM / M10), when known.
    #[serde(default)]
    pub frame: Option<u32>,
    /// Latest position, degrees. `None` until a GPS subframe arrives.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Where it has been, oldest first, capped by [`Rs41Settings::history_points`].
    pub track: Vec<(f32, f32)>,
    /// GPS altitude, metres above the ellipsoid (or WGS84 as broadcast).
    pub altitude_m: Option<f32>,
    /// Temperature, °C.
    pub temperature_c: Option<f32>,
    /// Relative humidity, percent.
    pub humidity_pct: Option<f32>,
    /// Pressure, hPa, when the subframe carries it.
    pub pressure_hpa: Option<f32>,
    /// Battery voltage, V, when known.
    pub battery_v: Option<f32>,
    /// Ascent rate, m/s (positive climbing), derived from successive altitudes.
    pub ascent_rate_mps: Option<f32>,
    /// Horizontal ground speed, m/s, when the decoder provides it.
    #[serde(default)]
    pub vel_h_mps: Option<f32>,
    /// Course over ground, degrees, when known.
    #[serde(default)]
    pub heading_deg: Option<f32>,
    /// GNSS satellites in solution, when known.
    #[serde(default)]
    pub sats: Option<u8>,
    /// Sonde-reported timestamp from `*mod` JSON (`datetime`), when present.
    /// SondeHub wants this in the upload `datetime` field (not receive time).
    #[serde(default)]
    pub datetime: Option<String>,
    /// Signal level of the last accepted frame, dBFS — negative.
    pub rssi_dbfs: f32,
    /// Frames accepted from this serial this session.
    pub frames: u32,
    /// Unix seconds when first heard this session.
    pub first_at: i64,
    /// Unix seconds of the last accepted frame.
    pub last_at: i64,
    /// Unix seconds of the last frame that moved the position. Zero when never.
    pub last_pos_at: i64,
}

impl Rs41Sonde {
    pub fn new(serial: String, frequency_hz: f64, kind: SondeKind, now: i64) -> Rs41Sonde {
        Rs41Sonde {
            serial,
            frequency_hz,
            kind,
            subtype: None,
            manufacturer: kind.manufacturer().to_string(),
            frame: None,
            lat: None,
            lon: None,
            track: Vec::new(),
            altitude_m: None,
            temperature_c: None,
            humidity_pct: None,
            pressure_hpa: None,
            battery_v: None,
            ascent_rate_mps: None,
            vel_h_mps: None,
            heading_deg: None,
            sats: None,
            datetime: None,
            rssi_dbfs: -100.0,
            frames: 0,
            first_at: now,
            last_at: now,
            last_pos_at: 0,
        }
    }

    pub fn has_position(&self) -> bool {
        self.lat.is_some() && self.lon.is_some()
    }

    pub fn pos_stale(&self, now: i64, drop_map_s: u16) -> bool {
        if !self.has_position() {
            return true;
        }
        now - self.last_pos_at > i64::from(drop_map_s)
    }

    pub fn label(&self) -> &str {
        if self.serial.is_empty() { "——" } else { self.serial.as_str() }
    }
}

/// What the engine tells the panel about the decoder itself.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Rs41Status {
    /// Every sonde still on the list.
    pub sondes: Vec<Rs41Sonde>,
    /// Why nothing is running, when nothing is running.
    pub unavailable: Option<String>,
    /// Dial the decoder would like (current sonde frequency), when useful.
    pub suggest_center_hz: Option<f64>,
    /// Absolute RF centre of the demod lane, Hz.
    pub window_center_hz: f64,
    /// Sample rate of the demod lane, Hz.
    pub window_rate_hz: f64,
    /// Frames the correlator accepted since start.
    pub frames: u64,
    /// Frames that failed CRC / Reed-Solomon / JSON parse.
    pub bad_crc: u64,
}

/// How the radiosonde decoder behaves. Owned by the engine ([`crate::RadioState`]),
/// edited from the panel's setup, and persisted across restarts (`rs41.json`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rs41Settings {
    /// The decoder runs. Selecting SONDE mode switches it on.
    pub enabled: bool,
    /// Which `*mod` family to run.
    #[serde(default)]
    pub kind: SondeKind,
    /// Seconds without a position report before a sonde leaves the map.
    pub drop_map_s: u16,
    /// Seconds without any frame before the sonde leaves the list.
    pub drop_list_s: u16,
    /// How many past positions to keep and draw behind each sonde.
    pub history_points: u16,
    /// Ceiling on the table.
    pub max_sondes: u16,
}

impl Default for Rs41Settings {
    fn default() -> Self {
        Rs41Settings {
            enabled: true,
            kind: SondeKind::Rs41,
            drop_map_s: RS41_DROP_MAP_S,
            drop_list_s: RS41_DROP_LIST_S,
            history_points: RS41_HISTORY_POINTS,
            max_sondes: RS41_MAX_SONDES,
        }
    }
}

impl Rs41Settings {
    /// Forced off — what the engine puts in [`crate::RadioState`] when the
    /// front end cannot feed the decoder (audio-mode CAT, too-narrow stream).
    pub const OFF: Rs41Settings = Rs41Settings {
        enabled: false,
        kind: SondeKind::Rs41,
        drop_map_s: RS41_DROP_MAP_S,
        drop_list_s: RS41_DROP_LIST_S,
        history_points: RS41_HISTORY_POINTS,
        max_sondes: RS41_MAX_SONDES,
    };

    /// Clamp operator-edited values into a usable range.
    pub fn sane(mut self) -> Self {
        self.drop_map_s = self.drop_map_s.clamp(5, 600);
        self.drop_list_s = self.drop_list_s.clamp(self.drop_map_s, 3_600);
        self.history_points = self.history_points.clamp(4, 240);
        self.max_sondes = self.max_sondes.clamp(1, 200);
        self
    }
}
