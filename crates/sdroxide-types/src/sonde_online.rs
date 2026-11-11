//! Radiosondes other stations have heard, as SondeHub's public API tells it.
//!
//! The pieces that need no network and no window, shared by the station (which
//! asks SondeHub once for all its clients, see `sdroxide-server`) and by the
//! client (which draws them, and asks SondeHub itself only when it runs next to
//! the radio with no station in between).

/// The query radius, metres.
pub const RADIUS_M: u32 = 400_000;

/// A sonde as SondeHub last heard it.
///
/// Every field after `unix` is optional and defaults to nothing, so a station
/// running an older version, whose list lacks them, still reads.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OnlineSonde {
    pub serial: String,
    pub model: String,
    pub lat: f64,
    pub lon: f64,
    pub alt_m: Option<f64>,
    /// Vertical speed, m/s (positive up).
    pub climb_ms: Option<f64>,
    /// Horizontal speed, m/s.
    pub speed_ms: Option<f64>,
    pub freq_mhz: Option<f64>,
    /// Unix second of its last frame.
    pub unix: i64,
    pub manufacturer: Option<String>,
    /// The variant, e.g. "RS41-SG".
    pub subtype: Option<String>,
    pub heading_deg: Option<f64>,
    pub temp_c: Option<f64>,
    pub humidity_pct: Option<f64>,
    pub pressure_hpa: Option<f64>,
    pub battery_v: Option<f64>,
    pub sats: Option<u32>,
    pub snr_db: Option<f64>,
    /// The callsign of the station that heard it, its place and its antenna,
    /// and the program it ran.
    pub station: Option<String>,
    pub station_pos: Option<(f64, f64)>,
    pub antenna: Option<String>,
    pub software: Option<String>,
    /// Unix second SondeHub received that frame.
    pub received_unix: Option<i64>,
}

/// A text from the internet, fit to be drawn: control characters out, at most
/// 64 characters, and nothing if that leaves nothing.
fn text(v: &serde_json::Value, key: &str) -> Option<String> {
    let t: String = v.get(key)?.as_str()?.chars().filter(|c| !c.is_control()).take(64).collect();
    let t = t.trim().to_string();
    (!t.is_empty()).then_some(t)
}

/// `"51.0583,-0.1282"` → a place on Earth.
fn parse_place(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once(',')?;
    let (lat, lon) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
    place_ok(lat, lon).then_some((lat, lon))
}

/// The address to ask, the locator rounded to 0.1° so the exact place stays
/// with the operator.
pub fn url(lat: f64, lon: f64) -> String {
    format!("https://api.v2.sondehub.org/sondes?lat={:.1}&lon={:.1}&distance={RADIUS_M}", lat, lon)
}

/// Days since 1970-01-01 of a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-11-05T23:48:43.004000Z` → unix seconds.
fn parse_time(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-');
    let (y, m, day) = (d.next()?.parse().ok()?, d.next()?.parse().ok()?, d.next()?.parse().ok()?);
    let time = time.trim_end_matches('Z');
    let time = time.split(['.', '+']).next()?;
    let mut t = time.split(':');
    let (h, mi, sec): (i64, i64, i64) =
        (t.next()?.parse().ok()?, t.next()?.parse().ok()?, t.next()?.parse().ok()?);
    Some(days_from_civil(y, m, day) * 86_400 + h * 3600 + mi * 60 + sec)
}

/// The reply: an object keyed by serial, each value the sonde's latest frame.
/// An entry without a serial, a place on Earth or a time is left out.
pub fn parse(json: &str) -> Vec<OnlineSonde> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let mut out: Vec<OnlineSonde> = map
        .values()
        .filter_map(|v| {
            let serial = v.get("serial")?.as_str()?.trim().to_string();
            let lat = v.get("lat")?.as_f64()?;
            let lon = v.get("lon")?.as_f64()?;
            if serial.is_empty() || lat.abs() > 90.0 || lon.abs() > 180.0 {
                return None;
            }
            Some(OnlineSonde {
                serial,
                model: v.get("type").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                lat,
                lon,
                alt_m: v.get("alt").and_then(|x| x.as_f64()),
                climb_ms: v.get("vel_v").and_then(|x| x.as_f64()),
                speed_ms: v.get("vel_h").and_then(|x| x.as_f64()),
                freq_mhz: v.get("frequency").and_then(|x| x.as_f64()),
                unix: parse_time(v.get("datetime")?.as_str()?)?,
                manufacturer: text(v, "manufacturer"),
                subtype: text(v, "subtype"),
                heading_deg: v.get("heading").and_then(|x| x.as_f64()),
                temp_c: v.get("temp").and_then(|x| x.as_f64()),
                humidity_pct: v.get("humidity").and_then(|x| x.as_f64()),
                pressure_hpa: v.get("pressure").and_then(|x| x.as_f64()),
                battery_v: v.get("batt").and_then(|x| x.as_f64()),
                sats: v.get("sats").and_then(|x| x.as_u64()).and_then(|n| u32::try_from(n).ok()),
                snr_db: v.get("snr").and_then(|x| x.as_f64()),
                station: text(v, "uploader_callsign"),
                station_pos: v
                    .get("uploader_position")
                    .and_then(|p| p.as_str())
                    .and_then(parse_place),
                antenna: text(v, "uploader_antenna"),
                software: text(v, "software_name").map(|n| match text(v, "software_version") {
                    Some(ver) => format!("{n} {ver}"),
                    None => n,
                }),
                received_unix: v.get("time_received").and_then(|t| t.as_str()).and_then(parse_time),
            })
        })
        .collect();
    // Newest first, so a crowded map keeps the live ones.
    out.sort_by_key(|s| std::cmp::Reverse(s.unix));
    out
}

/// A coordinate in tenths of a degree, the grain the station is asked at and
/// the key its answers are kept under.
pub fn tenths(v: f64) -> i32 {
    (v * 10.0).round() as i32
}

/// A station position that can be asked about: finite and on Earth.
pub fn place_ok(lat: f64, lon: f64) -> bool {
    lat.is_finite() && lon.is_finite() && lat.abs() <= 90.0 && lon.abs() <= 180.0
}

/// A serial fit to put in an address: letters, digits, `-` and `_`, at most 32.
pub fn serial_ok(serial: &str) -> bool {
    !serial.is_empty()
        && serial.len() <= 32
        && serial.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// The address of one sonde's flight so far (every frame SondeHub holds).
pub fn flight_url(serial: &str) -> Option<String> {
    serial_ok(serial).then(|| format!("https://api.v2.sondehub.org/sonde/{serial}"))
}

/// The most points of a flight kept: the track is a line on a map, not data.
pub const FLIGHT_MAX_POINTS: usize = 600;

/// One sonde's flight as SondeHub answers (an array of frames, each with
/// `datetime`, `lat`, `lon`): the places in time order, thinned to at most
/// [`FLIGHT_MAX_POINTS`] with the first and last kept. A frame without a place
/// on Earth is left out; nonsense gives nothing.
pub fn parse_flight(json: &str) -> Vec<(f64, f64)> {
    let Ok(serde_json::Value::Array(frames)) = serde_json::from_str::<serde_json::Value>(json)
    else {
        return Vec::new();
    };
    let mut pts: Vec<(i64, f64, f64)> = frames
        .iter()
        .filter_map(|v| {
            let lat = v.get("lat")?.as_f64()?;
            let lon = v.get("lon")?.as_f64()?;
            if !place_ok(lat, lon) {
                return None;
            }
            Some((parse_time(v.get("datetime")?.as_str()?)?, lat, lon))
        })
        .collect();
    pts.sort_by_key(|p| p.0);
    pts.dedup_by_key(|p| p.0);
    let n = pts.len();
    if n <= FLIGHT_MAX_POINTS {
        return pts.into_iter().map(|(_, a, b)| (a, b)).collect();
    }
    (0..FLIGHT_MAX_POINTS)
        .map(|i| pts[i * (n - 1) / (FLIGHT_MAX_POINTS - 1)])
        .map(|(_, a, b)| (a, b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two entries in the shape the API answers with: the first is a real
    /// sample (a Vaisala RS41 over the Channel), the second has no serial.
    const SAMPLE: &str = r#"{
      "Y1432023": {"software_name": "radiosonde_auto_rx", "datetime": "2026-11-05T23:48:43.004000Z",
        "manufacturer": "Vaisala", "type": "RS41", "serial": "Y1432023", "subtype": "RS41-SG",
        "frame": 3490, "lat": 51.05377, "lon": -0.0041, "alt": 8799.39254, "temp": -47.6,
        "vel_v": 4.51079, "vel_h": 29.0039, "heading": 304.36783, "sats": 9, "batt": 2.8,
        "frequency": 405.799875},
      "x": {"datetime": "2026-11-05T23:48:43Z", "lat": 1.0, "lon": 2.0}
    }"#;

    #[test]
    fn a_reply_becomes_sondes_and_an_entry_without_a_serial_is_left_out() {
        let v = parse(SAMPLE);
        assert_eq!(v.len(), 1);
        let s = &v[0];
        assert_eq!(s.serial, "Y1432023");
        assert_eq!(s.model, "RS41");
        assert!((s.lat - 51.05377).abs() < 1e-9 && (s.lon + 0.0041).abs() < 1e-9);
        assert_eq!(s.alt_m, Some(8799.39254));
        assert_eq!(s.climb_ms, Some(4.51079));
        assert_eq!(s.freq_mhz, Some(405.799875));
        // 2026-11-05T23:48:43Z (checked with calendar.timegm)
        assert_eq!(s.unix, 1_793_922_523);
    }

    /// Needs the network: `cargo test -p sdroxide-ui --lib sonde_online -- --ignored --nocapture`.

    #[test]
    fn nonsense_gives_an_empty_list() {
        assert!(parse("").is_empty());
        assert!(parse("[]").is_empty());
        assert!(parse("<html>").is_empty());
        assert!(parse(r#"{"a": {"serial": "A", "lat": 95.0, "lon": 0.0, "datetime": "2026-01-01T00:00:00Z"}}"#).is_empty());
    }

    #[test]
    fn a_flight_is_read_in_time_order_and_a_bad_frame_is_left_out() {
        let j = r#"[
          {"datetime": "2026-11-05T10:00:20Z", "lat": 49.0, "lon": 5.0},
          {"datetime": "2026-11-05T10:00:00Z", "lat": 48.0, "lon": 4.0},
          {"datetime": "2026-11-05T10:00:10Z", "lat": 95.0, "lon": 4.5},
          {"datetime": "2026-11-05T10:00:10Z", "lat": 48.5, "lon": 4.5, "alt": 1000},
          {"lat": 1.0, "lon": 1.0}
        ]"#;
        assert_eq!(parse_flight(j), vec![(48.0, 4.0), (48.5, 4.5), (49.0, 5.0)]);
        assert!(parse_flight("").is_empty() && parse_flight("{}").is_empty());
    }

    #[test]
    fn a_long_flight_is_thinned_and_keeps_both_ends() {
        let frames: Vec<String> = (0..5000)
            .map(|i| {
                format!(
                    r#"{{"datetime": "2026-11-05T{:02}:{:02}:{:02}Z", "lat": {}, "lon": 2.0}}"#,
                    i / 3600,
                    i / 60 % 60,
                    i % 60,
                    10.0 + i as f64 / 1000.0
                )
            })
            .collect();
        let v = parse_flight(&format!("[{}]", frames.join(",")));
        assert_eq!(v.len(), FLIGHT_MAX_POINTS);
        assert_eq!(v[0].0, 10.0);
        assert!((v[v.len() - 1].0 - 14.999).abs() < 1e-9);
    }

    #[test]
    fn only_a_plain_serial_gets_an_address() {
        assert_eq!(
            flight_url("W4871102").as_deref(),
            Some("https://api.v2.sondehub.org/sonde/W4871102")
        );
        for bad in ["", "A/../B", "a b", "x?y=1", &"A".repeat(33)] {
            assert!(flight_url(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn the_time_parser_matches_known_dates() {
        assert_eq!(parse_time("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_time("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(parse_time("2026-11-05T23:48:43.004000Z"), Some(1_793_922_523));
        assert_eq!(parse_time("garbage"), None);
    }

    #[test]
    fn the_address_rounds_the_stations_place_and_names_no_one() {
        let u = url(48.8566, 2.3522);
        assert_eq!(u, "https://api.v2.sondehub.org/sondes?lat=48.9&lon=2.4&distance=400000");
        assert!(!u.contains("48.85") && !u.contains("2.352"));
    }

    /// A real record (a Vaisala RS41 over London), as the API answers it.
    const FULL: &str = r#"{"Y1432014": {"software_name": "radiosonde_auto_rx",
        "software_version": "1.8.2", "uploader_callsign": "M6BIK",
        "uploader_position": "51.0583,-0.1282", "uploader_antenna": "1/4 wave monopole",
        "time_received": "2026-11-06T14:08:01.176206Z", "datetime": "2026-11-06T14:08:16.999000Z",
        "manufacturer": "Vaisala", "type": "RS41", "serial": "Y1432014", "subtype": "RS41-SG",
        "frame": 11922, "lat": 51.53776, "lon": -0.44878, "alt": 589.46337, "temp": 6.2,
        "humidity": 95.1, "vel_v": -3.90183, "vel_h": 1.88131, "heading": 266.82564,
        "sats": 9, "batt": 2.7, "frequency": 405.799875, "snr": 6.7}}"#;

    #[test]
    fn a_full_record_keeps_what_the_card_shows() {
        let v = parse(FULL);
        let s = &v[0];
        assert_eq!(s.manufacturer.as_deref(), Some("Vaisala"));
        assert_eq!(s.subtype.as_deref(), Some("RS41-SG"));
        assert_eq!(s.heading_deg, Some(266.82564));
        assert_eq!((s.temp_c, s.humidity_pct, s.battery_v), (Some(6.2), Some(95.1), Some(2.7)));
        assert_eq!((s.sats, s.snr_db, s.pressure_hpa), (Some(9), Some(6.7), None));
        assert_eq!(s.station.as_deref(), Some("M6BIK"));
        assert_eq!(s.station_pos, Some((51.0583, -0.1282)));
        assert_eq!(s.antenna.as_deref(), Some("1/4 wave monopole"));
        assert_eq!(s.software.as_deref(), Some("radiosonde_auto_rx 1.8.2"));
        // 2026-11-06T14:08:01Z and :16Z (checked with calendar.timegm)
        assert_eq!(s.received_unix, Some(1_793_974_081));
        assert_eq!(s.unix, 1_793_974_096);
    }

    #[test]
    fn an_old_stations_short_list_still_reads() {
        let old = r#"[{"serial":"A","model":"RS41","lat":1.0,"lon":2.0,"alt_m":null,
            "climb_ms":null,"speed_ms":null,"freq_mhz":null,"unix":5}]"#;
        let v: Vec<OnlineSonde> = serde_json::from_str(old).unwrap();
        assert_eq!(v[0].serial, "A");
        assert!(v[0].station.is_none() && v[0].temp_c.is_none());
    }

    #[test]
    fn texts_from_the_internet_are_cleaned_before_they_are_drawn() {
        let j = serde_json::json!({"a": "  ok\u{7}\n ", "b": "x".repeat(200), "c": "\n"});
        assert_eq!(text(&j, "a").as_deref(), Some("ok"));
        assert_eq!(text(&j, "b").map(|t| t.len()), Some(64));
        assert_eq!(text(&j, "c"), None);
        assert_eq!(parse_place("95,0"), None);
        assert_eq!(parse_place("junk"), None);
    }

    #[test]
    fn places_are_kept_to_a_tenth_and_nonsense_is_refused() {
        assert_eq!(tenths(48.8566), 489);
        assert_eq!(tenths(-0.04), 0);
        assert!(place_ok(48.0, 2.0) && place_ok(-90.0, 180.0));
        assert!(!place_ok(91.0, 0.0) && !place_ok(0.0, f64::NAN) && !place_ok(f64::INFINITY, 0.0));
    }
}
