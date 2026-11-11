//! A radiosonde's flight as a file: KML for Google Earth and the like, CSV for
//! a spreadsheet. Pure text built from a [`SondeTrack`], so it is tested
//! without a radio, the way the aircraft KML is.
//!
//! The file holds what the decoder heard: the serial and model name the file
//! and the line, and every kept position with its time and altitude. A sonde
//! carries no operator or owner data, so there is nothing else to leave out.

use crate::{SondeTrack, utc_ymd_hms};

/// ISO-8601 UTC, `2026-11-06T12:34:56Z`.
fn iso_utc(unix: i64) -> String {
    let (y, mo, d, h, mi, s) = utc_ymd_hms(unix);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Text made safe to put between tags.
fn xml(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control())
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '"' => "&quot;".chars().collect(),
            c => vec![c],
        })
        .collect()
}

/// A serial made safe for a file name: letters and digits only.
fn safe(serial: &str) -> String {
    let s: String = serial.chars().filter(char::is_ascii_alphanumeric).collect();
    if s.is_empty() { "unknown".to_string() } else { s }
}

/// `sonde-N1234567-20261106.kml` (or `.csv`), dated by `now`.
pub fn file_name(s: &SondeTrack, now: i64, ext: &str) -> String {
    let (y, mo, d, ..) = utc_ymd_hms(now);
    format!("sonde-{}-{y:04}{mo:02}{d:02}.{ext}", safe(&s.serial))
}

/// The flight as a KML `gx:Track`, or `None` when no position is kept.
pub fn kml(s: &SondeTrack) -> Option<String> {
    if s.track.is_empty() {
        return None;
    }
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(
        "<kml xmlns=\"http://www.opengis.net/kml/2.2\" \
         xmlns:gx=\"http://www.google.com/kml/ext/2.2\">\n<Document>\n",
    );
    out.push_str(&format!("<name>{} {}</name>\n", xml(&s.model), xml(&s.serial)));
    out.push_str(&format!("<description>{:.3} MHz</description>\n", s.freq_hz / 1e6));
    out.push_str(
        "<Style id=\"t\"><LineStyle><color>ff00a5ff</color><width>3</width></LineStyle></Style>\n",
    );
    out.push_str("<Placemark><name>flight</name><styleUrl>#t</styleUrl>\n");
    out.push_str("<gx:Track><altitudeMode>absolute</altitudeMode>\n");
    for p in &s.track {
        out.push_str(&format!("<when>{}</when>\n", iso_utc(p.unix)));
    }
    for p in &s.track {
        out.push_str(&format!("<gx:coord>{:.6} {:.6} {:.1}</gx:coord>\n", p.lon, p.lat, p.alt_m));
    }
    out.push_str("</gx:Track>\n</Placemark>\n</Document>\n</kml>\n");
    Some(out)
}

/// The flight as CSV: a header, then one line per kept position.
pub fn csv(s: &SondeTrack) -> Option<String> {
    if s.track.is_empty() {
        return None;
    }
    let mut out = String::from(
        "time_utc,unix,latitude,longitude,altitude_m,temperature_c,humidity_pct,pressure_hpa\n",
    );
    for p in &s.track {
        // A reading the sonde did not send is an empty cell.
        let cell =
            |v: Option<f32>, digits: usize| v.map_or(String::new(), |v| format!("{v:.digits$}"));
        out.push_str(&format!(
            "{},{},{:.6},{:.6},{:.1},{},{},{}\n",
            iso_utc(p.unix),
            p.unix,
            p.lat,
            p.lon,
            p.alt_m,
            cell(p.temp_c, 1),
            cell(p.humidity_pct, 0),
            cell(p.pressure_hpa, 1)
        ));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SondePoint;

    fn sonde(points: &[(f64, f64, f32, i64)]) -> SondeTrack {
        SondeTrack {
            model: "RS41".into(),
            serial: "N1234567".into(),
            freq_hz: 403.0e6,
            frame: 1,
            lat: None,
            lon: None,
            alt_m: None,
            speed_ms: None,
            course_deg: None,
            climb_ms: None,
            num_sv: None,
            battery_v: None,
            temp_c: None,
            humidity_pct: None,
            pressure_hpa: None,
            last_unix: 0,
            frames: 1,
            track: points
                .iter()
                .map(|&(lat, lon, alt_m, unix)| SondePoint {
                    lat,
                    lon,
                    alt_m,
                    unix,
                    temp_c: None,
                    humidity_pct: None,
                    pressure_hpa: None,
                })
                .collect(),
        }
    }

    #[test]
    fn a_flight_with_no_position_makes_no_file() {
        let s = sonde(&[]);
        assert!(kml(&s).is_none());
        assert!(csv(&s).is_none());
    }

    #[test]
    fn the_kml_has_the_times_and_the_coordinates_in_lon_lat_alt_order() {
        let s = sonde(&[(48.5, 2.25, 120.0, 1_762_432_496), (48.6, 2.35, 220.5, 1_762_432_497)]);
        let k = kml(&s).unwrap();
        assert!(k.contains("<name>RS41 N1234567</name>"));
        assert!(k.contains("<when>2025-11-06T12:34:56Z</when>"), "{k}");
        assert!(k.contains("<when>2025-11-06T12:34:57Z</when>"));
        assert!(k.contains("<gx:coord>2.250000 48.500000 120.0</gx:coord>"));
        assert!(k.contains("<gx:coord>2.350000 48.600000 220.5</gx:coord>"));
        assert!(k.contains("403.000 MHz"));
        // Well formed enough to open: every opened tag closes.
        assert_eq!(k.matches("<gx:Track>").count(), k.matches("</gx:Track>").count());
        assert!(k.trim_end().ends_with("</kml>"));
    }

    #[test]
    fn the_csv_has_a_header_and_one_line_per_point() {
        let s = sonde(&[(48.5, 2.25, 120.0, 1_762_432_496), (48.6, 2.35, 220.5, 1_762_432_497)]);
        let c = csv(&s).unwrap();
        let lines: Vec<&str> = c.lines().collect();
        assert_eq!(
            lines[0],
            "time_utc,unix,latitude,longitude,altitude_m,temperature_c,humidity_pct,pressure_hpa"
        );
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], "2025-11-06T12:34:56Z,1762432496,48.500000,2.250000,120.0,,,");
    }

    #[test]
    fn the_readings_of_a_point_are_in_the_csv() {
        let mut s = sonde(&[(48.5, 2.25, 120.0, 1_762_432_496)]);
        s.track[0].temp_c = Some(-12.34);
        s.track[0].humidity_pct = Some(57.6);
        s.track[0].pressure_hpa = Some(812.34);
        let c = csv(&s).unwrap();
        assert_eq!(
            c.lines().nth(1).unwrap(),
            "2025-11-06T12:34:56Z,1762432496,48.500000,2.250000,120.0,-12.3,58,812.3"
        );
    }

    #[test]
    fn a_name_with_markup_cannot_break_the_file() {
        let mut s = sonde(&[(1.0, 2.0, 3.0, 0)]);
        s.serial = "A<b>&\"x".into();
        let k = kml(&s).unwrap();
        assert!(k.contains("A&lt;b&gt;&amp;&quot;x"));
        assert!(!k.contains("<b>"));
    }

    #[test]
    fn the_file_name_is_safe_and_dated() {
        let mut s = sonde(&[(1.0, 2.0, 3.0, 0)]);
        assert_eq!(file_name(&s, 1_762_432_496, "kml"), "sonde-N1234567-20251106.kml");
        s.serial = "../../x y".into();
        assert_eq!(file_name(&s, 1_762_432_496, "csv"), "sonde-xy-20251106.csv");
        s.serial = "///".into();
        assert_eq!(file_name(&s, 1_762_432_496, "csv"), "sonde-unknown-20251106.csv");
    }
}
