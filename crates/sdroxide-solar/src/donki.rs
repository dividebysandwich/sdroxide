//! NASA CCMC's DONKI space-weather database: coronal mass ejections and solar
//! flares.
//!
//! <https://ccmc.gsfc.nasa.gov/tools/DONKI/> — the web service needs no API key.
//! When CCMC's own host is unreachable, which has lasted weeks at a time
//! (#895), the same data comes from NASA's API gateway, [`MIRROR`], with the
//! shared `DEMO_KEY`: same query, same JSON.
//!
//! Parsing here is defensive because the live data demands it, not out of
//! habit. In a representative 30-day window: 44 of 209 CME analyses carry a
//! null `longitude` (plane-of-sky measurements with no 3D direction), 98 of 165
//! events have an empty `sourceLocation`, 123 have a null `activeRegionNum`,
//! and `linkedEvents` is null more often than not. A strict schema fails on the
//! first record.

use serde::{Deserialize, Serialize};

use crate::helio::parse_location;
use crate::timefmt;

pub const BASE: &str = "https://kauai.ccmc.gsfc.nasa.gov/DONKI/WS/get";

/// NASA's API gateway in front of the same service. `DEMO_KEY` allows about
/// 50 requests a day per address, which is why the feed only falls back to it
/// and polls it no more than hourly there.
pub const MIRROR: &str = "https://api.nasa.gov/DONKI";

/// Typical half-width when an analysis omits it, degrees.
const DEFAULT_HALF_ANGLE: f64 = 25.0;
/// Typical CME speed when an analysis omits it, km/s.
const DEFAULT_SPEED: f64 = 450.0;

pub fn cme_url(start_unix: i64, end_unix: i64) -> String {
    format!("{BASE}/CME?startDate={}&endDate={}", timefmt::ymd(start_unix), timefmt::ymd(end_unix))
}

pub fn flare_url(start_unix: i64, end_unix: i64) -> String {
    format!("{BASE}/FLR?startDate={}&endDate={}", timefmt::ymd(start_unix), timefmt::ymd(end_unix))
}

/// The [`MIRROR`] URL for the same query as `primary` (one of [`cme_url`] or
/// [`flare_url`]); `None` for anything else.
pub fn mirror_url(primary: &str) -> Option<String> {
    let rest = primary.strip_prefix(BASE)?;
    Some(format!("{MIRROR}{rest}&api_key=DEMO_KEY"))
}

/// A CME with enough information to place a cone in space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CmeAnalysis {
    /// When the leading edge passed 21.5 solar radii — the epoch the speed is
    /// measured from, and the instant whose solar frame the direction is in.
    pub t21_5_unix: i64,
    pub lat_deg: f64,
    /// Stonyhurst longitude, **West-positive**.
    pub lon_west_deg: f64,
    pub half_angle_deg: f64,
    pub speed_km_s: f64,
    /// DONKI's speed class: S(low), C(ommon), O(ccasional), R(are), ER(extremely rare).
    pub kind: String,
    /// True when the direction came from the event's `sourceLocation` string
    /// rather than a fitted analysis — a coarser number, worth flagging.
    pub estimated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CmeEvent {
    pub id: String,
    pub start_unix: i64,
    pub active_region: Option<u32>,
    pub note: String,
    pub link: String,
    /// `None` for plane-of-sky-only events: they are real and still listed, but
    /// there is no direction to draw a cone along.
    pub analysis: Option<CmeAnalysis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlareEvent {
    pub id: String,
    pub begin_unix: i64,
    pub peak_unix: i64,
    pub end_unix: Option<i64>,
    /// GOES class, e.g. `M1.1`, `X2.4`.
    pub class: String,
    /// `(lat, lon_west)` degrees, when the source region is known.
    pub location: Option<(f64, f64)>,
    pub active_region: Option<u32>,
    pub link: String,
}

/// Rough energy ordering of a GOES flare class: A=0, B=1, C=2, M=3, X=4, plus
/// the mantissa as a fraction. Shared with the live X-ray readout in
/// [`crate::indices`], which reports the same classes.
pub fn flare_class_severity(class: &str) -> f64 {
    let mut c = class.trim().chars();
    let base = match c.next().map(|l| l.to_ascii_uppercase()) {
        Some('A') => 0.0,
        Some('B') => 1.0,
        Some('C') => 2.0,
        Some('M') => 3.0,
        Some('X') => 4.0,
        _ => return 0.0,
    };
    let mag: f64 = c.as_str().parse().unwrap_or(1.0);
    base + (mag.max(1.0).log10()).clamp(0.0, 1.0)
}

impl FlareEvent {
    /// See [`flare_class_severity`].
    pub fn severity(&self) -> f64 {
        flare_class_severity(&self.class)
    }
}

// ── Wire types ──────────────────────────────────────────────────────────────
// Every nullable field really is null in the live feed; `#[serde(default)]`
// throughout, and never `deny_unknown_fields` — DONKI adds fields over time.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RawCme {
    // Not `activityId`: DONKI capitalises the whole initialism, so the
    // `camelCase` rule alone silently produces an empty id.
    #[serde(rename = "activityID")]
    activity_id: String,
    start_time: Option<String>,
    source_location: Option<String>,
    active_region_num: Option<u32>,
    note: Option<String>,
    link: Option<String>,
    cme_analyses: Option<Vec<RawAnalysis>>,
}

impl Default for RawCme {
    fn default() -> Self {
        RawCme {
            activity_id: String::new(),
            start_time: None,
            source_location: None,
            active_region_num: None,
            note: None,
            link: None,
            cme_analyses: None,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct RawAnalysis {
    is_most_accurate: Option<bool>,
    #[serde(rename = "time21_5")]
    time21_5: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    half_angle: Option<f64>,
    speed: Option<f64>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct RawFlare {
    #[serde(rename = "flrID")]
    flr_id: String,
    begin_time: Option<String>,
    peak_time: Option<String>,
    end_time: Option<String>,
    class_type: Option<String>,
    source_location: Option<String>,
    active_region_num: Option<u32>,
    link: Option<String>,
}

/// Parse a DONKI `/CME` response. Unparseable records are skipped rather than
/// failing the batch — one malformed event should not cost the other 164.
pub fn parse_cmes(json: &str) -> Result<Vec<CmeEvent>, serde_json::Error> {
    let raw: Vec<RawCme> = serde_json::from_str(json)?;
    Ok(raw.into_iter().filter_map(convert_cme).collect())
}

fn convert_cme(r: RawCme) -> Option<CmeEvent> {
    let start_unix = timefmt::parse_unix(r.start_time.as_deref()?)?;
    let source = r.source_location.as_deref().and_then(parse_location);
    let analyses = r.cme_analyses.unwrap_or_default();

    // Prefer the analysis DONKI marks most accurate; fall back to the last one
    // submitted, which is the most recently refined.
    let has_direction = |a: &&RawAnalysis| a.latitude.is_some() && a.longitude.is_some();
    let chosen = analyses
        .iter()
        .find(|a| a.is_most_accurate.unwrap_or(false) && has_direction(a))
        .or_else(|| analyses.iter().filter(has_direction).next_back());

    let analysis = match chosen {
        Some(a) => Some(CmeAnalysis {
            t21_5_unix: a.time21_5.as_deref().and_then(timefmt::parse_unix).unwrap_or(start_unix),
            lat_deg: a.latitude?,
            lon_west_deg: a.longitude?,
            half_angle_deg: a.half_angle.unwrap_or(DEFAULT_HALF_ANGLE),
            speed_km_s: a.speed.unwrap_or(DEFAULT_SPEED),
            kind: a.kind.clone().unwrap_or_default(),
            estimated: false,
        }),
        // No fitted direction, but the event names its source region: that is
        // a usable (if coarse) axis, so draw it and mark it estimated.
        None => source.map(|(lat, lon)| {
            let fallback = analyses.iter().find(|a| a.speed.is_some());
            CmeAnalysis {
                t21_5_unix: fallback
                    .and_then(|a| a.time21_5.as_deref())
                    .and_then(timefmt::parse_unix)
                    .unwrap_or(start_unix),
                lat_deg: lat,
                lon_west_deg: lon,
                half_angle_deg: fallback.and_then(|a| a.half_angle).unwrap_or(DEFAULT_HALF_ANGLE),
                speed_km_s: fallback.and_then(|a| a.speed).unwrap_or(DEFAULT_SPEED),
                kind: fallback.and_then(|a| a.kind.clone()).unwrap_or_default(),
                estimated: true,
            }
        }),
    };

    Some(CmeEvent {
        id: r.activity_id,
        start_unix,
        active_region: r.active_region_num,
        note: r.note.unwrap_or_default(),
        link: r.link.unwrap_or_default(),
        analysis,
    })
}

/// Parse a DONKI `/FLR` response.
pub fn parse_flares(json: &str) -> Result<Vec<FlareEvent>, serde_json::Error> {
    let raw: Vec<RawFlare> = serde_json::from_str(json)?;
    Ok(raw
        .into_iter()
        .filter_map(|r| {
            let begin_unix = timefmt::parse_unix(r.begin_time.as_deref()?)?;
            Some(FlareEvent {
                id: r.flr_id,
                begin_unix,
                peak_unix: r
                    .peak_time
                    .as_deref()
                    .and_then(timefmt::parse_unix)
                    .unwrap_or(begin_unix),
                end_unix: r.end_time.as_deref().and_then(timefmt::parse_unix),
                class: r.class_type.unwrap_or_default(),
                location: r.source_location.as_deref().and_then(parse_location),
                active_region: r.active_region_num,
                link: r.link.unwrap_or_default(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mirror_asks_the_same_question() {
        let p = cme_url(0, 86_400);
        assert_eq!(
            mirror_url(&p).unwrap(),
            "https://api.nasa.gov/DONKI/CME?startDate=1970-01-01&endDate=1970-01-02&api_key=DEMO_KEY"
        );
        assert!(mirror_url("https://services.swpc.noaa.gov/x.json").is_none());
    }

    const CME_JSON: &str = include_str!("../tests/fixtures/cme.json");
    const FLR_JSON: &str = include_str!("../tests/fixtures/flr.json");

    #[test]
    fn parses_the_real_null_heavy_feed() {
        let events = parse_cmes(CME_JSON).expect("fixture must parse");
        assert!(events.len() >= 8, "only {} events", events.len());
        assert!(events.iter().all(|e| !e.id.is_empty() && e.start_unix > 0));
        // The fixture deliberately contains plane-of-sky events with no
        // direction and fully populated ones, so both paths are exercised.
        assert!(events.iter().any(|e| e.analysis.is_some()), "no event had a direction");
        for e in &events {
            if let Some(a) = &e.analysis {
                assert!((-90.0..=90.0).contains(&a.lat_deg), "{}: lat {}", e.id, a.lat_deg);
                assert!((-360.0..=360.0).contains(&a.lon_west_deg));
                assert!(a.half_angle_deg > 0.0 && a.half_angle_deg <= 90.0);
                assert!(a.speed_km_s > 0.0 && a.speed_km_s < 10_000.0);
                assert!(a.t21_5_unix > 0);
            }
        }
    }

    /// The sign exam for DONKI's numeric longitude.
    ///
    /// Where an event carries both a fitted `longitude` and a `sourceLocation`
    /// string, the two describe the same thing — and the string is
    /// self-describing (`W` really means west). If the numeric field were
    /// East-positive, these would disagree by roughly twice the longitude.
    #[test]
    fn donki_longitude_is_west_positive() {
        #[derive(serde::Deserialize)]
        struct Probe {
            #[serde(rename = "sourceLocation")]
            source: Option<String>,
            #[serde(rename = "cmeAnalyses")]
            analyses: Option<Vec<RawAnalysis>>,
        }
        let probes: Vec<Probe> = serde_json::from_str(CME_JSON).unwrap();
        let mut checked = 0;
        for p in &probes {
            let Some((slat, slon)) = p.source.as_deref().and_then(parse_location) else {
                continue;
            };
            for a in p.analyses.iter().flatten() {
                let (Some(lat), Some(lon)) = (a.latitude, a.longitude) else { continue };
                // A CME's fitted apex need not sit exactly over its source
                // region, but it lands on the same side of the disk.
                assert!(
                    (lon - slon).abs() < 60.0,
                    "longitude {lon} disagrees with sourceLocation {slon} — sign flipped?"
                );
                assert!((lat - slat).abs() < 60.0, "latitude {lat} vs {slat}");
                checked += 1;
            }
        }
        assert!(checked > 0, "fixture has no event with both forms — test is vacuous");
    }

    #[test]
    fn parses_flares_and_orders_them_by_energy() {
        let flares = parse_flares(FLR_JSON).expect("fixture must parse");
        assert!(!flares.is_empty());
        assert!(flares.iter().all(|f| f.begin_unix > 0 && f.peak_unix >= f.begin_unix));
        // Some fixture records have no source region and no end time.
        assert!(flares.iter().any(|f| f.location.is_some()));

        let sev = |c: &str| {
            FlareEvent {
                id: String::new(),
                begin_unix: 0,
                peak_unix: 0,
                end_unix: None,
                class: c.into(),
                location: None,
                active_region: None,
                link: String::new(),
            }
            .severity()
        };
        assert!(sev("X2.4") > sev("M1.1"));
        assert!(sev("M9.9") > sev("M1.0"));
        assert!(sev("C1.0") > sev("B5.0"));
        assert_eq!(sev(""), 0.0);
        assert_eq!(sev("garbage"), 0.0);
    }

    #[test]
    fn empty_and_malformed_input_is_survivable() {
        assert_eq!(parse_cmes("[]").unwrap().len(), 0);
        assert_eq!(parse_flares("[]").unwrap().len(), 0);
        assert!(parse_cmes("not json").is_err());
        // A record missing every optional field must still parse.
        assert_eq!(
            parse_cmes(r#"[{"activityID":"x","startTime":"2026-01-01T00:00Z"}]"#).unwrap().len(),
            1
        );
        // ...and one missing the *required* start time is skipped, not fatal.
        assert_eq!(parse_cmes(r#"[{"activityID":"x"}]"#).unwrap().len(), 0);
    }

    #[test]
    fn urls_carry_the_requested_window() {
        let u = cme_url(1_784_851_200, 1_784_937_600);
        assert_eq!(
            u,
            "https://kauai.ccmc.gsfc.nasa.gov/DONKI/WS/get/CME?startDate=2026-07-24&endDate=2026-07-25"
        );
        assert!(flare_url(0, 0).contains("/FLR?startDate=1970-01-01"));
    }
}
