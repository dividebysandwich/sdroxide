//! ADS-B / Mode S domain types, shared by the native engine, the wire protocol
//! and the UI (native + WASM). Pure data + serde — the demodulator and the
//! frame decoder live in the native `sdroxide-adsb` crate.
//!
//! # Why an aircraft table rather than a message log
//!
//! 1090 MHz is a room full of transmitters repeating themselves: an airliner
//! sends its position twice a second, its velocity twice a second and its
//! callsign every five. A chronological log of every squitter is the same
//! twenty aircraft over and over, and the question an operator actually has is
//! "what is in the air, and where".
//!
//! So the decoder keeps one [`AdsbAircraft`] per ICAO address and re-sends the
//! whole table a couple of times a second, exactly as the ISM decoder re-sends
//! its device table. The address is the stable key that lets a panel row keep
//! its place and a map symbol keep its track.
//!
//! # The two clocks
//!
//! A target has two independent ages, because "I have not heard where it is"
//! and "I have not heard it at all" are different facts and want different
//! answers. [`AdsbSettings::drop_map_s`] governs the first: past it the target
//! leaves the map and its row greys, because a symbol drawn at a position that
//! is half a minute old is a lie told in the same ink as the truth.
//! [`AdsbSettings::drop_list_s`] governs the second: past it the aircraft is
//! gone.

use serde::{Deserialize, Serialize};

/// The ADS-B extended squitter downlink, worldwide. There is only one.
pub const ADSB_FREQ_HZ: f64 = 1_090_000_000.0;

/// Below this the chips cannot be resolved at all.
///
/// Mode S is 1 Mbit/s pulse-position modulation: every bit is 1 µs split into
/// two 0.5 µs chips, and deciding which half holds the energy needs at least
/// one sample in each. Two megasamples a second is the floor, not a preference
/// — under it there is nothing to slice, and the honest answer is to say so
/// rather than to run and decode nothing.
pub const ADSB_MIN_RATE_HZ: f64 = 2_000_000.0;

/// The rate below which recall suffers no matter how good the signal is.
///
/// A Mode S chip is 0.5 µs, so at 2 Msps a chip and a sample are the same width
/// and the channel is critically sampled: at the worst arrival phase a chip is
/// split equally between two samples and reads exactly as strongly as its
/// neighbour, and the bit is a coin toss. Nothing downstream can put that back.
/// At 2.4 Msps a chip is 1.2 samples and the worst case leaves a clear 3:2, and
/// measured recall goes from a fraction of the sky to all of it.
///
/// It is not a refusal — a receiver between this and [`ADSB_MIN_RATE_HZ`] still
/// decodes the strong aircraft — but it is worth telling the operator, because
/// on most receivers the window width is theirs to change.
pub const ADSB_GOOD_RATE_HZ: f64 = 2_400_000.0;

/// The most the lane will take, however wide the receiver is.
///
/// More samples per chip is strictly better for this waveform, so the window is
/// *not* decimated down to some preferred figure — it keeps whatever the front
/// end delivers, up to here. The cap is a CPU budget: the correlator touches
/// every sample, and at this rate a very busy sky costs about a quarter of a
/// core. An RX-888 handing over its full 32.4 MHz would otherwise cost four
/// times that for no gain worth having.
pub const ADSB_MAX_RATE_HZ: f64 = 9_000_000.0;

/// Longest track kept per aircraft, whatever the settings say.
pub const ADSB_TRACK_MAX: usize = 240;

/// Default for [`AdsbSettings::drop_map_s`].
pub const ADSB_DROP_MAP_S: u16 = 10;
/// Default for [`AdsbSettings::drop_list_s`].
pub const ADSB_DROP_LIST_S: u16 = 60;
/// Default for [`AdsbSettings::history_points`].
pub const ADSB_HISTORY_POINTS: u16 = 40;
/// Default for [`AdsbSettings::max_aircraft`].
pub const ADSB_MAX_AIRCRAFT: u16 = 300;
/// Default for [`AdsbSettings::vector_minutes`].
pub const ADSB_VECTOR_MINUTES: f32 = 1.0;

/// Which downlink format a report's most recent frame was.
///
/// Kept because the two halves of this decoder have very different standing: an
/// extended squitter carries its own CRC and proves itself, while a
/// surveillance reply's parity is overlaid with the aircraft's address and can
/// only be believed because that address was already known. An operator reading
/// a row should be able to tell which kind of claim it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AdsbSource {
    /// DF17/18 extended squitter, or a DF11 all-call — CRC verified outright.
    #[default]
    Squitter,
    /// DF0/4/5/16/20/21 surveillance reply, accepted because the address
    /// recovered from its overlaid parity matched an aircraft already heard.
    Reply,
}

impl AdsbSource {
    pub fn label(self) -> &'static str {
        match self {
            AdsbSource::Squitter => "ES",
            AdsbSource::Reply => "reply",
        }
    }
}

/// One aircraft, as everything heard from it so far.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdsbAircraft {
    /// The 24-bit ICAO address. Stable for the aircraft's life, unique
    /// worldwide, and the table's key.
    pub icao: u32,
    /// The flight identification, eight characters, as broadcast. Empty until
    /// an identification squitter arrives — which can take several seconds, so
    /// a target with no callsign is normal rather than broken.
    pub callsign: String,
    /// The emitter category the identification squitter declared, already
    /// worded ("heavy", "rotorcraft", "glider").
    pub category: Option<String>,
    /// Latest position, degrees. `None` until an even/odd CPR pair or a local
    /// decode has resolved one.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Where it has been, oldest first, capped by
    /// [`AdsbSettings::history_points`].
    ///
    /// `f32` on purpose: at ±180° that is about two metres, which is far below
    /// what a history dot on a map can show, and it halves what the whole table
    /// costs on the wire — this message is re-sent twice a second with every
    /// aircraft in it.
    pub track: Vec<(f32, f32)>,
    /// Barometric altitude, feet.
    pub altitude_ft: Option<i32>,
    /// GNSS (geometric) altitude, feet, where the aircraft broadcasts it.
    pub gnss_altitude_ft: Option<i32>,
    /// Ground speed, knots.
    pub ground_speed_kt: Option<f32>,
    /// Track over ground, degrees true.
    pub track_deg: Option<f32>,
    /// Rate of climb, feet per minute; negative is descending.
    pub vertical_rate_fpm: Option<i32>,
    /// Rate of turn, degrees per second, derived from successive tracks rather
    /// than broadcast. What bends the leader line on the map.
    pub turn_rate_deg_s: f32,
    /// The aircraft says it is on the ground.
    pub on_ground: bool,
    /// Mode A squawk — the four digits an operator reads out, held as the
    /// decimal number they spell (`7700`, not the thirteen bits it arrived in).
    /// Each digit is 0..7; the code is octal, but it is *written* in digits and
    /// storing it any other way makes every display site convert.
    pub squawk: Option<u16>,
    /// An emergency or special-position state the aircraft declared.
    pub emergency: Option<String>,
    /// Signal level of the last accepted frame, dBFS — negative.
    pub rssi_dbfs: f32,
    /// Frames accepted from this address this session. One frame may be a lucky
    /// CRC pass; two hundred is an aeroplane.
    pub frames: u32,
    /// What the most recent accepted frame was.
    pub source: AdsbSource,
    /// Unix seconds when first heard this session.
    pub first_at: i64,
    /// Unix seconds of the last accepted frame of any kind.
    pub last_at: i64,
    /// Unix seconds of the last frame that moved the position. Zero when there
    /// has never been one.
    pub last_pos_at: i64,
    /// The last accepted frame as hex, for identifying something the decoder
    /// only half understands.
    pub raw_hex: String,
    /// What the aircraft reports from its own systems beyond position and
    /// velocity: airspeed, heading, what the crew has selected, the ADS-B
    /// version. Each field stays `None` until a message carrying it arrives.
    pub avionics: AdsbAvionics,
    /// Where this transponder's carrier is, absolute Hz: the median of the
    /// last frames' measurements. A transponder may sit anywhere within
    /// ±1 MHz of 1090 MHz and each sits at its own oscillator's offset, which
    /// is what draws one trace per aircraft in a waterfall zoomed in on
    /// 1090. `None` until a frame has been measured.
    pub carrier_hz: Option<f64>,
}

/// What an aircraft reports from its own systems (TC 19 subtypes 3/4, TC 29,
/// TC 31). Most aircraft send only some of it: the airspeed message only when
/// there is no velocity over the ground to send, the target state only from
/// ADS-B version 2 transponders.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AdsbAvionics {
    /// Heading, degrees — where the nose points, which differs from the
    /// track by the wind.
    pub heading_deg: Option<f32>,
    /// Indicated airspeed, knots.
    pub ias_kt: Option<f32>,
    /// True airspeed, knots.
    pub tas_kt: Option<f32>,
    /// Selected altitude, feet.
    pub selected_alt_ft: Option<i32>,
    /// The selected altitude comes from the FMS rather than the MCP/FCU.
    pub selected_alt_fms: bool,
    /// Barometer setting, hPa.
    pub qnh_hpa: Option<f32>,
    /// Selected heading, degrees.
    pub selected_heading_deg: Option<f32>,
    /// Autopilot engaged.
    pub autopilot: Option<bool>,
    /// The autopilot modes on, in the order `VNAV ALT APP LNAV`.
    pub modes: Vec<String>,
    /// TCAS operational.
    pub tcas: Option<bool>,
    /// ADS-B version of the transponder (0, 1 or 2).
    pub version: Option<u8>,
    /// Navigation accuracy category for position, 0–11.
    pub nac_p: Option<u8>,
    /// Source integrity level, 0–3.
    pub sil: Option<u8>,
}

impl AdsbAircraft {
    /// A fresh entry for an address just heard.
    pub fn new(icao: u32, now: i64) -> AdsbAircraft {
        AdsbAircraft {
            icao,
            callsign: String::new(),
            category: None,
            lat: None,
            lon: None,
            track: Vec::new(),
            altitude_ft: None,
            gnss_altitude_ft: None,
            ground_speed_kt: None,
            track_deg: None,
            vertical_rate_fpm: None,
            turn_rate_deg_s: 0.0,
            on_ground: false,
            squawk: None,
            emergency: None,
            rssi_dbfs: -100.0,
            frames: 0,
            source: AdsbSource::Squitter,
            first_at: now,
            last_at: now,
            last_pos_at: 0,
            raw_hex: String::new(),
            avionics: AdsbAvionics::default(),
            carrier_hz: None,
        }
    }

    /// The address as the six hex digits every other tool prints it in.
    pub fn hex(&self) -> String {
        format!("{:06X}", self.icao & 0xff_ffff)
    }

    /// What to call it on screen: the callsign once it has arrived, else the
    /// address. Never empty, because a target with no label is a target the
    /// operator cannot talk about.
    pub fn label(&self) -> String {
        let call = self.callsign.trim();
        if call.is_empty() { self.hex() } else { call.to_string() }
    }

    /// Has a position at all.
    pub fn has_position(&self) -> bool {
        self.lat.is_some() && self.lon.is_some()
    }

    /// The position is too old to draw: no position report for `drop_map_s`.
    ///
    /// Answers `true` for an aircraft that has never had one, which is what the
    /// map wants — there is nothing to place — while the list still shows the
    /// row, because "heard, altitude known, position not yet" is real
    /// information.
    pub fn pos_stale(&self, now: i64, drop_map_s: u16) -> bool {
        if !self.has_position() {
            return true;
        }
        now - self.last_pos_at > i64::from(drop_map_s)
    }

    /// Altitude the way a controller's data block writes it: flight level above
    /// the transition altitude, hundreds of feet below it, `GND` on the ground.
    pub fn fmt_altitude(&self) -> String {
        if self.on_ground {
            return "GND".to_string();
        }
        match self.altitude_ft {
            Some(ft) if ft >= 18_000 => format!("F{:03}", (ft as f64 / 100.0).round() as i32),
            Some(ft) => format!("{:03}", (ft as f64 / 100.0).round() as i32),
            None => "---".to_string(),
        }
    }

    /// Ground speed as a three-digit knot figure, or dashes.
    pub fn fmt_speed(&self) -> String {
        match self.ground_speed_kt {
            Some(kt) => format!("{:03}", kt.round().max(0.0) as i32),
            None => "---".to_string(),
        }
    }

    /// The squawk as the four digits it is assigned and read in.
    pub fn fmt_squawk(&self) -> String {
        match self.squawk {
            Some(s) => format!("{s:04}"),
            None => String::new(),
        }
    }
}

/// What the engine tells the panel about the decoder itself.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AdsbStatus {
    /// Every aircraft still on the list, in no particular order — the panel
    /// sorts.
    pub aircraft: Vec<AdsbAircraft>,
    /// Why nothing is running, when nothing is running. A receiver that cannot
    /// reach 1090 MHz, or cannot deliver two megasamples a second, produces an
    /// empty panel either way; only this distinguishes that from a quiet sky.
    pub unavailable: Option<String>,
    /// Where the operator would have to tune for the decoder to work. `None`
    /// when the dial is already right.
    pub suggest_center_hz: Option<f64>,
    /// Where the decoder's own window is, and how wide, in Hz.
    ///
    /// Shown for the reason the ISM window's is: "your receiver is not looking
    /// at 1090 MHz" is a claim about numbers the operator cannot otherwise see,
    /// and on a wide front end the hardware centre and the dial are routinely
    /// megahertz apart.
    pub window_center_hz: f64,
    pub window_rate_hz: f64,
    /// Preambles the correlator accepted since the decoder started, and how
    /// many of those produced a frame that passed its check. A high preamble
    /// count with no frames is a band full of something else — worth showing
    /// rather than leaving the panel looking broken.
    pub preambles: u64,
    pub frames: u64,
    /// Frames whose CRC did not come out zero.
    pub bad_crc: u64,
    /// Surveillance replies whose recovered address belonged to no known
    /// aircraft, so they were dropped rather than believed.
    pub unmatched: u64,
    /// Why the decoder will do badly here even though it is running.
    ///
    /// A receiver below [`ADSB_GOOD_RATE_HZ`] decodes the strong aircraft and
    /// quietly loses the rest. Without this the operator sees a short list and
    /// concludes there is nothing overhead, when what is overhead is being
    /// thrown away by a window one setting from wide enough.
    ///
    /// Appended, for the usual reason: postcard numbers fields by position.
    pub degraded: Option<String>,
}

/// How the ADS-B decoder behaves. Owned by the engine (it lives in
/// [`crate::RadioState`]), edited from the panel's setup, and persisted across
/// restarts (`adsb.json`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdsbSettings {
    /// The decoder runs. Follows the mode — selecting ADS-B switches it on —
    /// but kept as a field so a front end that cannot feed it can switch it off
    /// and say so.
    pub enabled: bool,
    /// Seconds without a position report before a target leaves the map and its
    /// row greys.
    pub drop_map_s: u16,
    /// Seconds without any frame at all before the aircraft leaves the list.
    pub drop_list_s: u16,
    /// How many past positions to keep and draw behind each target.
    pub history_points: u16,
    /// How far ahead the leader line reaches, in minutes of flight at the
    /// current ground speed. One minute is the usual radar convention.
    pub vector_minutes: f32,
    /// Ceiling on the table, so a busy sector cannot grow the status message
    /// without bound. The oldest are dropped first.
    pub max_aircraft: u16,
}

impl Default for AdsbSettings {
    fn default() -> Self {
        AdsbSettings {
            enabled: true,
            drop_map_s: ADSB_DROP_MAP_S,
            drop_list_s: ADSB_DROP_LIST_S,
            history_points: ADSB_HISTORY_POINTS,
            vector_minutes: ADSB_VECTOR_MINUTES,
            max_aircraft: ADSB_MAX_AIRCRAFT,
        }
    }
}

impl AdsbSettings {
    /// The decoder switched off, for a front end that cannot run it.
    ///
    /// A separate value rather than `Default` with `enabled: false`, for the
    /// reason [`crate::IsmSettings::OFF`] is: the engine forces this into the
    /// live state on an audio-mode source, and that must not be mistaken for
    /// what the operator chose.
    pub const OFF: AdsbSettings = AdsbSettings {
        enabled: false,
        drop_map_s: ADSB_DROP_MAP_S,
        drop_list_s: ADSB_DROP_LIST_S,
        history_points: ADSB_HISTORY_POINTS,
        vector_minutes: ADSB_VECTOR_MINUTES,
        max_aircraft: ADSB_MAX_AIRCRAFT,
    };

    /// The settings with every field inside the range the decoder can honour.
    ///
    /// Applied where they arrive rather than where they are used: these come
    /// from a config file an operator may have edited and from a remote client,
    /// and a zero history length or a million-aircraft ceiling should be
    /// corrected once rather than defended against everywhere.
    pub fn sane(mut self) -> AdsbSettings {
        self.drop_map_s = self.drop_map_s.clamp(2, 600);
        self.drop_list_s = self.drop_list_s.clamp(self.drop_map_s, 3600);
        self.history_points = self.history_points.clamp(0, ADSB_TRACK_MAX as u16);
        self.vector_minutes = self.vector_minutes.clamp(0.0, 10.0);
        self.max_aircraft = self.max_aircraft.clamp(10, 2000);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The data block writes an altitude the way a controller reads one.
    #[test]
    fn altitude_reads_as_a_flight_level_above_the_transition() {
        let mut a = AdsbAircraft::new(0x3C6444, 0);
        a.altitude_ft = Some(38_000);
        assert_eq!(a.fmt_altitude(), "F380");
        a.altitude_ft = Some(4_500);
        assert_eq!(a.fmt_altitude(), "045");
        a.on_ground = true;
        assert_eq!(a.fmt_altitude(), "GND");
        a.on_ground = false;
        a.altitude_ft = None;
        assert_eq!(a.fmt_altitude(), "---");
    }

    /// A squawk is four digits with leading zeroes kept: 0021 is a real code
    /// and "21" is not one anybody would recognise.
    #[test]
    fn a_squawk_keeps_its_leading_zeroes() {
        let mut a = AdsbAircraft::new(0x3C6444, 0);
        a.squawk = Some(7700);
        assert_eq!(a.fmt_squawk(), "7700");
        a.squawk = Some(21);
        assert_eq!(a.fmt_squawk(), "0021");
    }

    /// An aircraft always has something to call it, from the first frame.
    #[test]
    fn a_target_is_never_unlabelled() {
        let mut a = AdsbAircraft::new(0x4CA1FA, 0);
        assert_eq!(a.label(), "4CA1FA");
        a.callsign = "RYR1234 ".to_string();
        assert_eq!(a.label(), "RYR1234");
    }

    /// The two clocks are independent, and an aircraft with no position at all
    /// is stale for the map's purposes while still being a real row.
    #[test]
    fn the_map_clock_and_the_list_clock_are_not_the_same_clock() {
        let mut a = AdsbAircraft::new(0x3C6444, 1_000);
        assert!(a.pos_stale(1_000, 10), "never positioned is nothing to draw");
        a.lat = Some(48.2);
        a.lon = Some(16.4);
        a.last_pos_at = 1_000;
        assert!(!a.pos_stale(1_005, 10));
        assert!(a.pos_stale(1_011, 10));
    }

    /// A hand-edited config cannot ask for a list window shorter than the map
    /// one — that would drop an aircraft before it had a chance to grey.
    #[test]
    fn the_list_window_is_never_shorter_than_the_map_window() {
        let s = AdsbSettings { drop_map_s: 30, drop_list_s: 5, ..AdsbSettings::default() }.sane();
        assert!(s.drop_list_s >= s.drop_map_s);
        let s = AdsbSettings { history_points: 60_000, ..AdsbSettings::default() }.sane();
        assert_eq!(s.history_points, ADSB_TRACK_MAX as u16);
    }
}
