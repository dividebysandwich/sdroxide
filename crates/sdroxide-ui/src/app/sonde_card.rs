//! What the radiosonde view says about one sonde: its card.
//!
//! Two sources fill the same card — a sonde this radio decodes
//! ([`SondeTrack`]), and one only another station has heard (SondeHub's list,
//! [`OnlineSonde`]). The wording and the ordering are here, apart from the
//! drawing, so they can be tested. Nothing in this file reads the network: a
//! field nobody sent is left out, never drawn as a blank.

use sdroxide_types::SondeTrack;

use crate::i18n::tr;
use crate::sonde_online::OnlineSonde;

/// A sonde as the card shows it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct Card {
    /// "RS41 N1234567".
    pub title: String,
    pub serial: String,
    /// The pairs of label and value, in the order they are drawn.
    pub rows: Vec<(String, String)>,
    /// The channel it transmits on, Hz: drawn large, and a click on it tunes
    /// the radio there.
    pub freq_hz: Option<f64>,
    /// Where it is now, and its altitude: the head of the card.
    pub place: Option<(f64, f64, Option<f64>)>,
    /// This radio decodes it (so the card carries the radio's own rows).
    pub from_radio: bool,
}

/// A position as the card writes it: `48.8566° N, 2.3522° E`.
pub(in crate::app) fn fmt_position(lat: f64, lon: f64) -> String {
    let ns = if lat < 0.0 { 'S' } else { 'N' };
    let ew = if lon < 0.0 { 'W' } else { 'E' };
    format!("{:.4}° {ns}, {:.4}° {ew}", lat.abs(), lon.abs())
}

/// "42 s", "7 min", "3 h".
pub(in crate::app) fn ago(secs: i64) -> String {
    let s = secs.max(0);
    if s < 90 {
        format!("{s} s")
    } else if s < 5400 {
        format!("{} min", s / 60)
    } else {
        format!("{} h", s / 3600)
    }
}

/// Where to read about a sonde on the web: the site's name and the page. A
/// serial is only ever letters, digits, `-` and `_`; anything else is dropped,
/// so a serial from the internet cannot add to the address.
pub(in crate::app) fn links(serial: &str) -> Vec<(&'static str, String)> {
    let s: String =
        serial.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')).collect();
    if s.is_empty() {
        return Vec::new();
    }
    vec![
        ("SondeHub", format!("https://tracker.sondehub.org/#!f={s}&q={s}")),
        ("radiosondy.info", format!("https://radiosondy.info/sonde.php?sondenumber={s}")),
    ]
}

/// The label of the link button.
pub(in crate::app) fn open_label(site: &str) -> String {
    tr!("sonde-card-open", site = site)
}

fn push(rows: &mut Vec<(String, String)>, label: String, value: Option<String>) {
    if let Some(v) = value {
        rows.push((label, v));
    }
}

/// The distance and bearing from the station.
fn from_you(home: Option<(f64, f64)>, lat: f64, lon: f64) -> Option<String> {
    let (hlat, hlon) = home?;
    let km = sdroxide_types::distance_km((hlat, hlon), (lat, lon));
    let bear = sdroxide_types::bearing_deg((hlat, hlon), (lat, lon));
    Some(format!("{km:.0} km   {bear:.0}°"))
}

/// A sonde this radio decodes.
pub(in crate::app) fn own(s: &SondeTrack, home: Option<(f64, f64)>, now: i64) -> Card {
    let mut rows = vec![(tr!("sonde-card-source"), tr!("sonde-card-source-radio"))];
    rows.push((tr!("sonde-card-model"), s.model.clone()));
    rows.push((tr!("sonde-channel"), format!("{:.3} MHz", s.freq_hz / 1e6)));
    if let (Some(lat), Some(lon)) = (s.lat, s.lon) {
        rows.push((tr!("sonde-card-position"), fmt_position(lat, lon)));
        push(&mut rows, tr!("sonde-card-from-you"), from_you(home, lat, lon));
    }
    push(&mut rows, tr!("sonde-col-alt"), s.alt_m.map(|v| format!("{v:.0} m")));
    push(&mut rows, tr!("sonde-col-climb"), s.climb_ms.map(|v| format!("{v:+.1} m/s")));
    push(&mut rows, tr!("sonde-col-speed"), s.speed_ms.map(|v| format!("{:.0} km/h", v * 3.6)));
    push(&mut rows, tr!("sonde-card-heading"), s.course_deg.map(|v| format!("{v:.0}°")));
    push(&mut rows, tr!("sonde-card-satellites"), s.num_sv.map(|v| v.to_string()));
    push(&mut rows, tr!("sonde-col-battery"), s.battery_v.map(|v| format!("{v:.1} V")));
    push(&mut rows, tr!("sonde-col-temp"), s.temp_c.map(|v| format!("{v:.1} °C")));
    push(&mut rows, tr!("sonde-col-humidity"), s.humidity_pct.map(|v| format!("{v:.0} %")));
    push(&mut rows, tr!("sonde-col-pressure"), s.pressure_hpa.map(|v| format!("{v:.0} hPa")));
    rows.push((tr!("sonde-card-frames"), s.frames.to_string()));
    rows.push((tr!("sonde-col-heard"), ago(now - s.last_unix)));
    Card {
        title: format!("{} {}", s.model, s.serial),
        serial: s.serial.clone(),
        rows,
        freq_hz: (s.freq_hz > 0.0).then_some(s.freq_hz),
        place: s.lat.zip(s.lon).map(|(a, b)| (a, b, s.alt_m.map(f64::from))),
        from_radio: true,
    }
}

/// A sonde only SondeHub's list has.
pub(in crate::app) fn online(o: &OnlineSonde, home: Option<(f64, f64)>, now: i64) -> Card {
    let mut rows = vec![(tr!("sonde-card-source"), tr!("sonde-card-source-hub"))];
    rows.push((tr!("sonde-card-model"), o.model.clone()));
    push(&mut rows, tr!("sonde-card-maker"), o.manufacturer.clone());
    push(&mut rows, tr!("sonde-card-variant"), o.subtype.clone());
    push(&mut rows, tr!("sonde-channel"), o.freq_mhz.map(|f| format!("{f:.3} MHz")));
    rows.push((tr!("sonde-card-position"), fmt_position(o.lat, o.lon)));
    push(&mut rows, tr!("sonde-card-from-you"), from_you(home, o.lat, o.lon));
    push(&mut rows, tr!("sonde-col-alt"), o.alt_m.map(|v| format!("{v:.0} m")));
    push(&mut rows, tr!("sonde-col-climb"), o.climb_ms.map(|v| format!("{v:+.1} m/s")));
    push(&mut rows, tr!("sonde-col-speed"), o.speed_ms.map(|v| format!("{:.0} km/h", v * 3.6)));
    push(&mut rows, tr!("sonde-card-heading"), o.heading_deg.map(|v| format!("{v:.0}°")));
    push(&mut rows, tr!("sonde-card-satellites"), o.sats.map(|v| v.to_string()));
    push(&mut rows, tr!("sonde-col-battery"), o.battery_v.map(|v| format!("{v:.1} V")));
    push(&mut rows, tr!("sonde-col-temp"), o.temp_c.map(|v| format!("{v:.1} °C")));
    push(&mut rows, tr!("sonde-col-humidity"), o.humidity_pct.map(|v| format!("{v:.0} %")));
    push(&mut rows, tr!("sonde-col-pressure"), o.pressure_hpa.map(|v| format!("{v:.0} hPa")));
    push(&mut rows, tr!("sonde-card-snr"), o.snr_db.map(|v| format!("{v:.1} dB")));
    push(&mut rows, tr!("sonde-card-station"), o.station.clone());
    if let Some((lat, lon)) = o.station_pos {
        rows.push((tr!("sonde-card-station-position"), fmt_position(lat, lon)));
    }
    push(&mut rows, tr!("sonde-card-antenna"), o.antenna.clone());
    push(&mut rows, tr!("sonde-card-software"), o.software.clone());
    rows.push((tr!("sonde-col-heard"), ago(now - o.unix)));
    Card {
        title: format!("{} {}", o.model, o.serial),
        serial: o.serial.clone(),
        rows,
        freq_hz: o.freq_mhz.filter(|f| *f > 0.0).map(|f| f * 1e6),
        place: Some((o.lat, o.lon, o.alt_m)),
        from_radio: false,
    }
}

/// A sonde both sources know: this radio's rows first (its own frames are the
/// freshest), then whatever only SondeHub says — maker, variant, SNR, the
/// station that heard it, its antenna and software — and the fields this radio
/// did not decode (temperature, pressure…) when SondeHub has them.
fn merged(s: &SondeTrack, o: &OnlineSonde, home: Option<(f64, f64)>, now: i64) -> Card {
    let mut c = own(s, home, now);
    c.rows[0].1 = tr!("sonde-card-source-both");
    let heard = c.rows.pop();
    let heard_label = heard.as_ref().map(|(l, _)| l.clone());
    for (label, value) in online(o, home, now).rows {
        let known = c.rows.iter().any(|(l, _)| *l == label);
        if !known && Some(&label) != heard_label.as_ref() {
            c.rows.push((label, value));
        }
    }
    c.rows.extend(heard);
    // What this radio lacks, SondeHub fills: a sonde heard but with no GPS fix
    // yet still shows where the network last saw it.
    let hub = online(o, home, now);
    c.freq_hz = c.freq_hz.or(hub.freq_hz);
    c.place = c.place.or(hub.place);
    c
}

/// The card of a sonde from what is known of it: this radio's track, SondeHub's
/// line, or both. `None` when neither has it.
pub(in crate::app) fn compose(
    radio: Option<&SondeTrack>,
    hub: Option<&OnlineSonde>,
    home: Option<(f64, f64)>,
    now: i64,
) -> Option<Card> {
    match (radio, hub) {
        (Some(s), Some(o)) => Some(merged(s, o, home, now)),
        (Some(s), None) => Some(own(s, home, now)),
        (None, Some(o)) => Some(online(o, home, now)),
        (None, None) => None,
    }
}

/// Mark a card drawn from a kept copy: the sonde is no longer in any list, and
/// what is shown is the last thing known of it.
pub(in crate::app) fn mark_stale(card: &mut Card) {
    let at = 1.min(card.rows.len());
    card.rows.insert(at, (tr!("sonde-card-stale"), tr!("sonde-card-stale-value")));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radio_sonde() -> SondeTrack {
        SondeTrack {
            model: "RS41".into(),
            serial: "Y1432014".into(),
            freq_hz: 405_800_000.0,
            frame: 10,
            lat: Some(48.0),
            lon: Some(2.0),
            alt_m: Some(1000.0),
            speed_ms: None,
            course_deg: None,
            climb_ms: None,
            num_sv: None,
            battery_v: None,
            temp_c: None,
            humidity_pct: None,
            pressure_hpa: None,
            last_unix: 1_000,
            frames: 7,
            track: Vec::new(),
        }
    }

    #[test]
    fn a_sonde_both_sources_know_gets_one_complete_card() {
        let mut hub = hub();
        hub.temp_c = Some(1.4);
        hub.subtype = Some("RS41-SGP".into());
        let c = compose(Some(&radio_sonde()), Some(&hub), None, 1_030).expect("a card");
        // The radio's own rows stay, and say both sources have it…
        assert_eq!(
            value(&c, &tr!("sonde-card-source")),
            Some(tr!("sonde-card-source-both").as_str())
        );
        assert_eq!(value(&c, &tr!("sonde-card-frames")), Some("7"));
        assert_eq!(value(&c, &tr!("sonde-col-alt")), Some("1000 m"), "the radio's altitude wins");
        // …and what only SondeHub knows is added, once.
        assert_eq!(value(&c, &tr!("sonde-card-maker")), Some("Vaisala"));
        assert_eq!(value(&c, &tr!("sonde-card-variant")), Some("RS41-SGP"));
        assert_eq!(value(&c, &tr!("sonde-card-station")), Some("M6BIK"));
        assert_eq!(value(&c, &tr!("sonde-col-temp")), Some("1.4 °C"), "a field the radio lacks");
        for label in [tr!("sonde-card-position"), tr!("sonde-col-heard"), tr!("sonde-channel")] {
            assert_eq!(c.rows.iter().filter(|(l, _)| *l == label).count(), 1, "{label} once");
        }
        assert_eq!(c.rows.last().map(|(l, _)| l.as_str()), Some(tr!("sonde-col-heard").as_str()));
    }

    #[test]
    fn a_card_is_built_from_whichever_source_has_the_sonde() {
        assert!(compose(None, None, None, 0).is_none());
        let r = compose(Some(&radio_sonde()), None, None, 1_030).unwrap();
        assert_eq!(
            value(&r, &tr!("sonde-card-source")),
            Some(tr!("sonde-card-source-radio").as_str())
        );
        let h = compose(None, Some(&hub()), None, 1_060).unwrap();
        assert_eq!(
            value(&h, &tr!("sonde-card-source")),
            Some(tr!("sonde-card-source-hub").as_str())
        );
    }

    #[test]
    fn a_kept_copy_says_it_is_no_longer_in_any_list() {
        let mut c = compose(None, Some(&hub()), None, 1_060).unwrap();
        mark_stale(&mut c);
        assert_eq!(c.rows[1].0, tr!("sonde-card-stale"));
        assert_eq!(c.rows[1].1, tr!("sonde-card-stale-value"));
    }

    fn value<'a>(c: &'a Card, label: &str) -> Option<&'a str> {
        c.rows.iter().find(|(l, _)| *l == label).map(|(_, v)| v.as_str())
    }

    fn hub() -> OnlineSonde {
        OnlineSonde {
            serial: "Y1432014".into(),
            model: "RS41".into(),
            lat: 51.5,
            lon: -0.4,
            alt_m: Some(589.0),
            climb_ms: Some(-3.9),
            freq_mhz: Some(405.8),
            unix: 1_000,
            manufacturer: Some("Vaisala".into()),
            station: Some("M6BIK".into()),
            antenna: Some("1/4 wave monopole".into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_sondehub_card_names_who_heard_it_and_leaves_out_what_nobody_sent() {
        let c = online(&hub(), None, 1_060);
        assert_eq!(c.title, "RS41 Y1432014");
        assert_eq!(value(&c, &tr!("sonde-card-maker")), Some("Vaisala"));
        assert_eq!(value(&c, &tr!("sonde-card-station")), Some("M6BIK"));
        assert_eq!(value(&c, &tr!("sonde-card-antenna")), Some("1/4 wave monopole"));
        assert_eq!(value(&c, &tr!("sonde-col-alt")), Some("589 m"));
        assert_eq!(value(&c, &tr!("sonde-col-climb")), Some("-3.9 m/s"));
        assert_eq!(value(&c, &tr!("sonde-col-heard")), Some("60 s"));
        assert_eq!(value(&c, &tr!("sonde-col-humidity")), None);
        assert_eq!(value(&c, &tr!("sonde-card-from-you")), None, "no station position, no range");
    }

    #[test]
    fn the_range_and_bearing_come_from_the_stations_place() {
        let c = online(&hub(), Some((51.5, 0.6)), 1_060);
        let v = value(&c, &tr!("sonde-card-from-you")).expect("a range");
        assert!(v.contains('°'), "{v}");
    }

    #[test]
    fn a_decoded_sonde_card_says_it_is_this_radios() {
        let s = SondeTrack {
            model: "RS41".into(),
            serial: "N1234567".into(),
            freq_hz: 404_500_000.0,
            frame: 10,
            lat: Some(48.0),
            lon: Some(2.0),
            alt_m: Some(12_000.0),
            speed_ms: Some(10.0),
            course_deg: Some(90.0),
            climb_ms: Some(5.0),
            num_sv: Some(9),
            battery_v: Some(2.7),
            temp_c: None,
            humidity_pct: None,
            pressure_hpa: None,
            last_unix: 500,
            frames: 42,
            track: Vec::new(),
        };
        let c = own(&s, None, 530);
        assert_eq!(
            value(&c, &tr!("sonde-card-source")),
            Some(tr!("sonde-card-source-radio").as_str())
        );
        assert_eq!(value(&c, &tr!("sonde-channel")), Some("404.500 MHz"));
        assert_eq!(value(&c, &tr!("sonde-col-speed")), Some("36 km/h"));
        assert_eq!(value(&c, &tr!("sonde-card-frames")), Some("42"));
        assert_eq!(value(&c, &tr!("sonde-col-temp")), None);
    }

    #[test]
    fn the_links_carry_the_serial_and_nothing_a_serial_could_add() {
        let l = links("Y1432014");
        assert_eq!(
            l[0],
            ("SondeHub", "https://tracker.sondehub.org/#!f=Y1432014&q=Y1432014".into())
        );
        assert_eq!(l[1].1, "https://radiosondy.info/sonde.php?sondenumber=Y1432014");
        let evil = links("A/../?x=1&y=2#z");
        assert!(evil.iter().all(|(_, u)| !u.contains("..") && !u.contains("x=1")), "{evil:?}");
        assert!(links("///").is_empty());
    }

    #[test]
    fn ages_read_in_a_human_unit() {
        assert_eq!(
            (ago(5), ago(600), ago(7200), ago(-3)),
            ("5 s".into(), "10 min".into(), "2 h".into(), "0 s".into())
        );
    }
}
