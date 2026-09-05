//! The numbers an HF operator actually checks before calling CQ: the planetary
//! K and A indices, the 10.7 cm solar flux, the current GOES X-ray level, and a
//! maximum usable frequency near the operator's own location.
//!
//! All from NOAA SWPC except the MUF, which comes from the community ionosonde
//! network aggregated by <https://prop.kc2g.com/>. None needs an API key, and
//! every payload here is small — the largest is 42 kB — because these are
//! polled far more often than the imagery.

use serde::{Deserialize, Serialize};

use crate::timefmt;

pub const FLUX_URL: &str = "https://services.swpc.noaa.gov/products/summary/10cm-flux.json";
pub const KP_URL: &str = "https://services.swpc.noaa.gov/products/noaa-planetary-k-index.json";
pub const XRAY_URL: &str =
    "https://services.swpc.noaa.gov/json/goes/primary/xray-flares-latest.json";
pub const IONOSONDE_URL: &str = "https://prop.kc2g.com/api/stations.json";
/// N0NBH's band-conditions feed, the one product here that is somebody's
/// personal server rather than an institution's.
///
/// Its FAQ asks for hourly polling at most ("that is the update period") and
/// warns that the feed exists only as long as the author's ISP tolerates it.
/// [`crate::Source::BandConditions`] honours that; do not shorten it. Credit to
/// HAMQSL.com is requested and is shown wherever the verdicts are.
pub const BAND_CONDITIONS_URL: &str = "https://www.hamqsl.com/solarxml.php";

/// 10.7 cm solar radio flux, the standard proxy for ionising solar output.
/// Under about 70 is a dead band; over 150 opens the high bands.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolarFlux {
    pub sfu: f64,
    pub observed_unix: i64,
}

/// Planetary geomagnetic activity. `kp` is the quasi-logarithmic 0–9 index;
/// `a_running` is its linear equivalent, which is the one that reads
/// proportionally to how disturbed things are.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeomagneticIndex {
    pub kp: f64,
    pub a_index: f64,
    pub observed_unix: i64,
}

impl GeomagneticIndex {
    /// NOAA's G-scale storm level, 0–5.
    pub fn storm_level(&self) -> u8 {
        match self.kp {
            k if k >= 9.0 => 5,
            k if k >= 8.0 => 4,
            k if k >= 7.0 => 3,
            k if k >= 6.0 => 2,
            k if k >= 5.0 => 1,
            _ => 0,
        }
    }

    /// What it means for the bands, in the terms operators use.
    pub fn hf_effect(&self) -> &'static str {
        match self.kp {
            k if k >= 7.0 => "severe storm — HF blackout at high latitudes",
            k if k >= 5.0 => "storm — polar paths degraded, aurora possible",
            k if k >= 4.0 => "unsettled — high-latitude paths noisy",
            k if k >= 3.0 => "slightly unsettled",
            _ => "quiet",
        }
    }
}

/// The current GOES soft X-ray level, as its flare class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XrayLevel {
    /// e.g. `C1.1`, `M2.4`, `X1.0`.
    pub class: String,
    /// The strongest class seen in the current event, if one is in progress.
    pub max_class: Option<String>,
    pub observed_unix: i64,
}

impl XrayLevel {
    /// Ordering value: A=0, B=1, C=2, M=3, X=4, plus the mantissa as a fraction.
    pub fn severity(&self) -> f64 {
        crate::donki::flare_class_severity(&self.class)
    }

    /// An M-class flare or bigger is when the D layer starts absorbing HF on
    /// the daylit side.
    pub fn causes_hf_absorption(&self) -> bool {
        self.severity() >= 3.0
    }
}

/// One ionosonde's most recent scaling.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ionosonde {
    pub lat: f64,
    /// Degrees east, normalised to `[-180, 180]`.
    pub lon: f64,
    /// F2-layer critical frequency, MHz — the highest frequency reflected
    /// straight up.
    pub fof2: f64,
    /// MUF for a 3000 km path, MHz.
    pub mufd: f64,
    pub observed_unix: i64,
}

/// A MUF estimate for a particular place, interpolated from nearby soundings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MufEstimate {
    /// MUF for a 3000 km path, MHz.
    pub muf_mhz: f64,
    pub fof2_mhz: f64,
    /// Distance to the nearest contributing ionosonde, km. The further this is,
    /// the more of a guess the number is.
    pub nearest_km: f64,
    pub station_count: usize,
    pub observed_unix: i64,
}

impl MufEstimate {
    /// How much to trust it. An ionosonde a few hundred km away is close to a
    /// measurement; one 3000 km away, across the terminator, is not.
    pub fn confidence(&self) -> &'static str {
        match self.nearest_km {
            d if d < 500.0 => "measured nearby",
            d if d < 1500.0 => "interpolated",
            _ => "distant sounders — rough",
        }
    }
}

/// How good a verdict is, without the words.
///
/// The feed states conditions in English, and English is what should be shown;
/// this is only so a colour can be chosen without every caller matching on
/// strings. [`BandRating::Unknown`] is deliberate and is not an error: a
/// phenomenon the feed describes some other way ("50MHz ES", "High MUF") still
/// has text worth printing, and inventing a grade for it would be worse than
/// printing it plain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BandRating {
    Good,
    Fair,
    Poor,
    Closed,
    Unknown,
}

impl BandRating {
    pub fn of(verdict: &str) -> BandRating {
        let v = verdict.trim().to_ascii_lowercase();
        if v.contains("closed") {
            BandRating::Closed
        } else if v.contains("good") {
            BandRating::Good
        } else if v.contains("fair") || v.contains("moderate") {
            BandRating::Fair
        } else if v.contains("poor") {
            BandRating::Poor
        } else {
            BandRating::Unknown
        }
    }
}

/// One HF verdict as the feed states it: a band *group*, a time of day, and a
/// word.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HfBandCondition {
    /// The group name verbatim — `"80m-40m"`, `"30m-20m"`, `"17m-15m"`,
    /// `"12m-10m"`. Kept as the feed's own string rather than mapped to an
    /// enum here: the publisher may regroup, and a group this build does not
    /// recognise should still be displayable.
    pub group: String,
    pub day: bool,
    pub verdict: String,
}

/// One VHF phenomenon: sporadic-E for a region, or the auroral band.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VhfCondition {
    /// `"E-Skip"`, `"vhf-aurora"`.
    pub phenomenon: String,
    /// `"europe"`, `"north_america"`, `"europe_6m"`, `"northern_hemi"`, …
    pub location: String,
    pub status: String,
}

/// N0NBH's calculated band conditions, as published.
///
/// **What this is not**: a statement about this station, this path, or this
/// antenna. It is one globally-computed verdict per band group per half of the
/// day, derived from the solar indices, and it is shown labelled with its
/// source and its age for exactly that reason. The propagation field built
/// from real receptions is the measurement; this is the forecast.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BandConditions {
    pub hf: Vec<HfBandCondition>,
    pub vhf: Vec<VhfCondition>,
    /// When the publisher says the figures were calculated.
    pub observed_unix: i64,
}

impl BandConditions {
    /// The verdict for `band`, or `None` where the feed says nothing about it.
    ///
    /// `None` is the common case at both ends of the spectrum and must render
    /// as blank rather than as a guess: the four groups published cover 80 m
    /// through 10 m and nothing else, so 160 m, 60 m and everything above
    /// 10 m have no verdict here at all. 60 m in particular sits *inside* the
    /// published range and is still not covered — it is not part of the
    /// "80m-40m" group, and folding it in would be inventing data.
    pub fn for_band(&self, band: sdroxide_types::Band, daylight: bool) -> Option<&str> {
        let group = band_group(band)?;
        self.hf
            .iter()
            .find(|c| c.day == daylight && c.group.eq_ignore_ascii_case(group))
            .map(|c| c.verdict.as_str())
    }

    /// The same, graded.
    pub fn rating_for_band(
        &self,
        band: sdroxide_types::Band,
        daylight: bool,
    ) -> Option<BandRating> {
        self.for_band(band, daylight).map(BandRating::of)
    }
}

/// Which published group a band belongs to, if any.
fn band_group(band: sdroxide_types::Band) -> Option<&'static str> {
    use sdroxide_types::Band;
    match band {
        Band::M80 | Band::M40 => Some("80m-40m"),
        Band::M30 | Band::M20 => Some("30m-20m"),
        Band::M17 | Band::M15 => Some("17m-15m"),
        Band::M12 | Band::M10 => Some("12m-10m"),
        // Not published. 160 m and 60 m are below and inside the range
        // respectively; 6 m and up are covered — if at all — by the sporadic-E
        // and aurora entries, which are about a phenomenon rather than a band
        // and are not interchangeable with a Good/Fair/Poor verdict. The
        // microwave bands are not an HF forecast's business at all: what opens
        // them is rain scatter, aircraft and the troposphere, none of which a
        // solar-flux verdict knows anything about.
        Band::M160
        | Band::M60
        | Band::M6
        | Band::M4
        | Band::M2
        | Band::M125
        | Band::M70
        | Band::Cm3
        | Band::Cm33
        | Band::Cm23
        | Band::Cm13
        | Band::Cm9
        | Band::Cm6
        | Band::Gen => None,
    }
}

/// Everything in this module, as one snapshot.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpaceWeather {
    pub flux: Option<SolarFlux>,
    pub geomagnetic: Option<GeomagneticIndex>,
    pub xray: Option<XrayLevel>,
    pub ionosondes: Vec<Ionosonde>,
    /// N0NBH's calculated verdicts. Appended last: this rides inside
    /// `SolarServerMsg::Weather`, so its position is part of the wire format.
    pub band_conditions: Option<BandConditions>,
}

// ── Parsers ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawFlux {
    flux: Option<f64>,
    time_tag: Option<String>,
}

/// `[{"flux": 150, "time_tag": "2026-07-24T20:00:00"}]`
pub fn parse_flux(json: &str) -> Option<SolarFlux> {
    let raw: Vec<RawFlux> = serde_json::from_str(json).ok()?;
    let last = raw.into_iter().next_back()?;
    Some(SolarFlux {
        sfu: last.flux?,
        observed_unix: last.time_tag.as_deref().and_then(timefmt::parse_unix).unwrap_or(0),
    })
}

#[derive(Deserialize)]
struct RawKp {
    time_tag: Option<String>,
    #[serde(rename = "Kp")]
    kp: Option<f64>,
    a_running: Option<f64>,
}

/// The planetary K index series; the newest entry is the current one.
pub fn parse_kp(json: &str) -> Option<GeomagneticIndex> {
    let raw: Vec<RawKp> = serde_json::from_str(json).ok()?;
    let last = raw.into_iter().filter(|r| r.kp.is_some()).next_back()?;
    Some(GeomagneticIndex {
        kp: last.kp?,
        a_index: last.a_running.unwrap_or(0.0),
        observed_unix: last.time_tag.as_deref().and_then(timefmt::parse_unix).unwrap_or(0),
    })
}

#[derive(Deserialize)]
struct RawXray {
    time_tag: Option<String>,
    current_class: Option<String>,
    max_class: Option<String>,
}

pub fn parse_xray(json: &str) -> Option<XrayLevel> {
    let raw: Vec<RawXray> = serde_json::from_str(json).ok()?;
    let last = raw.into_iter().next_back()?;
    Some(XrayLevel {
        class: last.current_class?,
        max_class: last.max_class,
        observed_unix: last.time_tag.as_deref().and_then(timefmt::parse_unix).unwrap_or(0),
    })
}

#[derive(Deserialize)]
struct RawStation {
    latitude: Option<String>,
    longitude: Option<String>,
}

#[derive(Deserialize)]
struct RawSounding {
    time: Option<String>,
    fof2: Option<f64>,
    mufd: Option<f64>,
    /// GIRO autoscaling confidence score; −1 means "not scored".
    cs: Option<f64>,
    station: Option<RawStation>,
}

/// Minimum autoscaling confidence to accept a sounding.
const MIN_CONFIDENCE: f64 = 20.0;

/// Parse HamQSL's `<solar><solardata>` document.
///
/// Defensive in the same way the JSON feeds here are: an element that has
/// changed shape costs that one verdict, never the document. A feed that came
/// back as an error page, or with no `<calculatedconditions>` at all, yields
/// `None` so the last cached copy stays on screen rather than being replaced
/// by an empty one.
///
/// The `<fof2>`, `<muffactor>` and `<muf>` elements are deliberately ignored:
/// they are frequently empty or `NoRpt`, and this program has a real MUF from
/// the ionosonde network a few lines up.
pub fn parse_band_conditions(xml: &str) -> Option<BandConditions> {
    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("band conditions parse failed: {e}");
            return None;
        }
    };
    let mut out = BandConditions::default();
    for n in doc.descendants() {
        match n.tag_name().name() {
            "updated" => {
                out.observed_unix =
                    n.text().and_then(timefmt::parse_dmy_hhmm).unwrap_or(out.observed_unix);
            }
            "band" => {
                let (Some(group), Some(time), Some(verdict)) =
                    (n.attribute("name"), n.attribute("time"), n.text())
                else {
                    continue;
                };
                let day = match time.trim().to_ascii_lowercase().as_str() {
                    "day" => true,
                    "night" => false,
                    // A third time of day is not something this understands
                    // well enough to file under either.
                    other => {
                        tracing::debug!("band conditions: unknown time {other:?}");
                        continue;
                    }
                };
                out.hf.push(HfBandCondition {
                    group: group.trim().to_string(),
                    day,
                    verdict: verdict.trim().to_string(),
                });
            }
            "phenomenon" => {
                let (Some(name), Some(status)) = (n.attribute("name"), n.text()) else {
                    continue;
                };
                out.vhf.push(VhfCondition {
                    phenomenon: name.trim().to_string(),
                    location: n.attribute("location").unwrap_or("").trim().to_string(),
                    status: status.trim().to_string(),
                });
            }
            _ => {}
        }
    }
    // A document with neither is not this document — most likely an error page
    // or a captive portal, and replacing good cached verdicts with nothing
    // would be the worst of both.
    (!out.hf.is_empty() || !out.vhf.is_empty()).then_some(out)
}

pub fn parse_ionosondes(json: &str) -> Vec<Ionosonde> {
    let raw: Vec<RawSounding> = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("ionosonde feed parse failed: {e}");
            return Vec::new();
        }
    };
    raw.into_iter()
        .filter_map(|r| {
            // Autoscaled ionograms are often wrong; the confidence score is
            // there to be used, and a bad scaling is worse than no number.
            if r.cs.unwrap_or(-1.0) < MIN_CONFIDENCE {
                return None;
            }
            let s = r.station?;
            let lat: f64 = s.latitude?.parse().ok()?;
            let lon: f64 = s.longitude?.parse().ok()?;
            let (fof2, mufd) = (r.fof2?, r.mufd?);
            if !(0.5..40.0).contains(&fof2) || !(1.0..80.0).contains(&mufd) {
                return None;
            }
            Some(Ionosonde {
                lat,
                // The feed publishes 0–360 east; everything else here is ±180.
                lon: if lon > 180.0 { lon - 360.0 } else { lon },
                fof2,
                mufd,
                observed_unix: r.time.as_deref().and_then(timefmt::parse_unix)?,
            })
        })
        .collect()
}

/// Soundings older than this are ignored — the ionosphere changes far faster.
pub const MAX_SOUNDING_AGE_S: i64 = 3 * 3600;
/// Ionosondes beyond this contribute nothing; past it the interpolation is
/// meaningless, usually because it would be reaching across the terminator.
pub const MAX_SOUNDING_KM: f64 = 4000.0;

/// Interpolate a MUF for `(lat, lon)` from the surrounding soundings.
///
/// Inverse-distance weighting over everything fresh and within
/// [`MAX_SOUNDING_KM`]. This is the same approach the community propagation
/// maps use, and it carries the same caveat: the ionosphere changes sharply
/// across the day/night terminator, so a number interpolated from sounders on
/// the other side of it is a guess. [`MufEstimate::confidence`] says which case
/// you are in rather than hiding it.
pub fn estimate_muf(
    stations: &[Ionosonde],
    lat: f64,
    lon: f64,
    now_unix: i64,
) -> Option<MufEstimate> {
    let mut sum_w = 0.0;
    let mut sum_muf = 0.0;
    let mut sum_fof2 = 0.0;
    let mut nearest = f64::MAX;
    let mut count = 0usize;
    let mut newest = 0i64;

    for s in stations {
        if now_unix - s.observed_unix > MAX_SOUNDING_AGE_S || s.observed_unix > now_unix + 3600 {
            continue;
        }
        let d = sdroxide_types::distance_km((lat, lon), (s.lat, s.lon));
        if d > MAX_SOUNDING_KM {
            continue;
        }
        // 1/d², with a floor so a sounder you are sitting on does not divide by
        // zero and swamp everything.
        let w = 1.0 / (d * d).max(100.0);
        sum_w += w;
        sum_muf += w * s.mufd;
        sum_fof2 += w * s.fof2;
        nearest = nearest.min(d);
        newest = newest.max(s.observed_unix);
        count += 1;
    }

    (count > 0 && sum_w > 0.0).then(|| MufEstimate {
        muf_mhz: sum_muf / sum_w,
        fof2_mhz: sum_fof2 / sum_w,
        nearest_km: nearest,
        station_count: count,
        observed_unix: newest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real document, as fetched.
    const HAMQSL: &str = include_str!("../tests/fixtures/hamqsl.xml");

    #[test]
    fn parses_the_published_band_conditions() {
        let c = parse_band_conditions(HAMQSL).expect("the real document must parse");
        // Four groups, each stated for both halves of the day.
        assert_eq!(c.hf.len(), 8, "{:?}", c.hf);
        assert_eq!(c.vhf.len(), 5);
        assert!(c.observed_unix > 1_700_000_000, "the update stamp was not read");
        for group in ["80m-40m", "30m-20m", "17m-15m", "12m-10m"] {
            for day in [true, false] {
                assert!(
                    c.hf.iter().any(|h| h.group == group && h.day == day),
                    "{group} has no {} verdict",
                    if day { "daytime" } else { "night" }
                );
            }
        }
        let es = c.vhf.iter().find(|v| v.phenomenon == "E-Skip" && v.location == "europe");
        assert!(es.is_some(), "the European sporadic-E entry is missing");
    }

    /// A band group covers two bands, and both must read the same verdict —
    /// that is what "80m-40m" means.
    #[test]
    fn both_bands_of_a_group_read_the_same_verdict() {
        use sdroxide_types::Band;
        let c = parse_band_conditions(HAMQSL).unwrap();
        for (a, b) in [
            (Band::M80, Band::M40),
            (Band::M30, Band::M20),
            (Band::M17, Band::M15),
            (Band::M12, Band::M10),
        ] {
            for day in [true, false] {
                assert_eq!(c.for_band(a, day), c.for_band(b, day));
                assert!(c.for_band(a, day).is_some(), "{a:?} has no verdict");
            }
        }
    }

    /// The bands the feed says nothing about must say nothing. Inventing a
    /// verdict for 160 m or 6 m from the neighbouring group would be the one
    /// way this feature could actively mislead.
    ///
    /// Written as "everything outside the four published groups" rather than as
    /// a list, so a band added later is silent by default and has to be given a
    /// group deliberately to gain a verdict.
    #[test]
    fn unpublished_bands_have_no_verdict_rather_than_a_guess() {
        use sdroxide_types::Band;
        let c = parse_band_conditions(HAMQSL).unwrap();
        for b in Band::ALL.into_iter().filter(|b| {
            !matches!(
                b,
                Band::M80
                    | Band::M40
                    | Band::M30
                    | Band::M20
                    | Band::M17
                    | Band::M15
                    | Band::M12
                    | Band::M10
            )
        }) {
            assert_eq!(c.for_band(b, true), None, "{b:?} was given a verdict");
            assert_eq!(c.for_band(b, false), None, "{b:?} was given a verdict");
            assert_eq!(c.rating_for_band(b, true), None);
        }
    }

    /// Day and night are different answers, and reading the wrong one is a
    /// silent error — the words are the same shape either way.
    #[test]
    fn day_and_night_are_kept_apart() {
        use sdroxide_types::Band;
        let xml = r#"<solar><solardata>
            <calculatedconditions>
              <band name="80m-40m" time="day">Poor</band>
              <band name="80m-40m" time="night">Good</band>
            </calculatedconditions>
        </solardata></solar>"#;
        let c = parse_band_conditions(xml).unwrap();
        assert_eq!(c.for_band(Band::M40, true), Some("Poor"));
        assert_eq!(c.for_band(Band::M40, false), Some("Good"));
        assert_eq!(c.rating_for_band(Band::M40, true), Some(BandRating::Poor));
        assert_eq!(c.rating_for_band(Band::M40, false), Some(BandRating::Good));
    }

    #[test]
    fn a_page_that_is_not_this_document_yields_nothing_to_show() {
        assert_eq!(parse_band_conditions("<html><body>502 Bad Gateway</body></html>"), None);
        assert_eq!(parse_band_conditions("not xml at all <<<"), None);
        assert_eq!(parse_band_conditions("<solar><solardata></solardata></solar>"), None);
    }

    /// One malformed entry must not cost the rest of the document.
    #[test]
    fn a_broken_entry_costs_only_itself() {
        let xml = r#"<solar><solardata>
            <calculatedconditions>
              <band time="day">Poor</band>
              <band name="30m-20m">Fair</band>
              <band name="30m-20m" time="teatime">Fair</band>
              <band name="30m-20m" time="day">Good</band>
            </calculatedconditions>
        </solardata></solar>"#;
        let c = parse_band_conditions(xml).unwrap();
        assert_eq!(c.hf.len(), 1);
        assert_eq!(c.for_band(sdroxide_types::Band::M20, true), Some("Good"));
    }

    #[test]
    fn verdicts_are_graded_without_losing_their_words() {
        assert_eq!(BandRating::of("Good"), BandRating::Good);
        assert_eq!(BandRating::of("fair"), BandRating::Fair);
        assert_eq!(BandRating::of("Poor"), BandRating::Poor);
        assert_eq!(BandRating::of("Band Closed"), BandRating::Closed);
        // Something the publisher words differently keeps its text and gets no
        // invented grade.
        assert_eq!(BandRating::of("50MHz ES"), BandRating::Unknown);
    }

    #[test]
    fn parses_the_flux_summary() {
        let f = parse_flux(r#"[{"flux": 150, "time_tag": "2026-07-24T20:00:00"}]"#).unwrap();
        assert_eq!(f.sfu, 150.0);
        assert_eq!(f.observed_unix, 1_784_923_200);
        assert_eq!(parse_flux("[]"), None);
        assert_eq!(parse_flux("junk"), None);
        // A record with a null flux is not a flux reading.
        assert_eq!(parse_flux(r#"[{"time_tag":"2026-07-24T20:00:00"}]"#), None);
    }

    #[test]
    fn parses_the_k_index_series_and_takes_the_newest() {
        let json = r#"[
            {"time_tag": "2026-07-25T00:00:00", "Kp": 4.33, "a_running": 20, "station_count": 8},
            {"time_tag": "2026-07-25T03:00:00", "Kp": 0.67, "a_running": 3, "station_count": 8}
        ]"#;
        let k = parse_kp(json).unwrap();
        assert_eq!(k.kp, 0.67);
        assert_eq!(k.a_index, 3.0);
        assert_eq!(k.observed_unix, 1_784_948_400);
    }

    #[test]
    fn the_k_index_maps_to_storm_levels_and_plain_words() {
        let at = |kp| GeomagneticIndex { kp, a_index: 0.0, observed_unix: 0 };
        assert_eq!(at(1.0).storm_level(), 0);
        assert_eq!(at(5.0).storm_level(), 1);
        assert_eq!(at(7.5).storm_level(), 3);
        assert_eq!(at(9.0).storm_level(), 5);
        assert_eq!(at(0.3).hf_effect(), "quiet");
        assert!(at(5.5).hf_effect().contains("storm"));
        assert!(at(8.0).hf_effect().contains("blackout"));
    }

    #[test]
    fn parses_the_xray_summary() {
        let json = r#"[{"time_tag": "2026-07-25T06:29:00Z", "satellite": 18,
            "current_class": "C1.1", "max_class": "C1.9"}]"#;
        let x = parse_xray(json).unwrap();
        assert_eq!(x.class, "C1.1");
        assert_eq!(x.max_class.as_deref(), Some("C1.9"));
        assert!(!x.causes_hf_absorption(), "a C-class flare is not a blackout");

        let m = XrayLevel { class: "M5.0".into(), max_class: None, observed_unix: 0 };
        assert!(m.causes_hf_absorption());
        assert!(m.severity() > x.severity());
        let big = XrayLevel { class: "X8.2".into(), max_class: None, observed_unix: 0 };
        assert!(big.severity() > m.severity());
    }

    fn sonde(lat: f64, lon: f64, mufd: f64, t: i64) -> Ionosonde {
        Ionosonde { lat, lon, fof2: mufd / 3.2, mufd, observed_unix: t }
    }

    const NOW: i64 = 1_784_937_600;

    #[test]
    fn muf_interpolation_favours_the_nearest_sounder() {
        // One close by at 20 MHz, one far away at 40.
        let stations = [sonde(48.0, 16.0, 20.0, NOW), sonde(38.0, 16.0, 40.0, NOW)];
        let e = estimate_muf(&stations, 48.2, 15.8, NOW).unwrap();
        assert_eq!(e.station_count, 2);
        assert!(e.nearest_km < 60.0, "nearest {} km", e.nearest_km);
        assert!(
            (e.muf_mhz - 20.0).abs() < 0.5,
            "MUF {} should be dominated by the sounder 30 km away",
            e.muf_mhz
        );
        assert_eq!(e.confidence(), "measured nearby");
    }

    #[test]
    fn muf_ignores_stale_and_distant_soundings() {
        // Stale.
        let old = [sonde(48.0, 16.0, 20.0, NOW - 6 * 3600)];
        assert_eq!(estimate_muf(&old, 48.2, 15.8, NOW), None);
        // Beyond the cutoff: the antipode.
        let far = [sonde(-48.0, -164.0, 20.0, NOW)];
        assert_eq!(estimate_muf(&far, 48.2, 15.8, NOW), None);
        // A timestamp from the future is a broken feed, not a fresh sounding.
        let future = [sonde(48.0, 16.0, 20.0, NOW + 86_400)];
        assert_eq!(estimate_muf(&future, 48.2, 15.8, NOW), None);
        assert_eq!(estimate_muf(&[], 0.0, 0.0, NOW), None);
    }

    #[test]
    fn muf_confidence_degrades_with_distance() {
        let near = estimate_muf(&[sonde(48.0, 16.0, 20.0, NOW)], 48.1, 16.1, NOW).unwrap();
        let mid = estimate_muf(&[sonde(48.0, 16.0, 20.0, NOW)], 55.0, 16.0, NOW).unwrap();
        let far = estimate_muf(&[sonde(48.0, 16.0, 20.0, NOW)], 20.0, 16.0, NOW).unwrap();
        assert_eq!(near.confidence(), "measured nearby");
        assert_eq!(mid.confidence(), "interpolated");
        assert_eq!(far.confidence(), "distant sounders — rough");
        assert!(near.nearest_km < mid.nearest_km && mid.nearest_km < far.nearest_km);
    }

    #[test]
    fn ionosonde_parsing_filters_bad_scalings_and_normalises_longitude() {
        let json = r#"[
            {"time":"2026-07-25T06:25:01","fof2":6.25,"mufd":20.0,"cs":65.0,
             "station":{"latitude":"37.1","longitude":"353.3","name":"El Arenosillo"}},
            {"time":"2026-07-25T06:20:00","fof2":8.0,"mufd":26.0,"cs":-1.0,
             "station":{"latitude":"30.4","longitude":"262.3","name":"unscored"}},
            {"time":"2026-07-25T06:20:00","fof2":8.0,"mufd":26.0,"cs":0.0,
             "station":{"latitude":"30.4","longitude":"262.3","name":"zero confidence"}},
            {"time":"2026-07-25T06:20:00","fof2":900.0,"mufd":26.0,"cs":65.0,
             "station":{"latitude":"30.4","longitude":"262.3","name":"absurd fof2"}},
            {"time":"2026-07-25T06:20:00","cs":65.0,
             "station":{"latitude":"30.4","longitude":"262.3","name":"no measurement"}}
        ]"#;
        let s = parse_ionosondes(json);
        assert_eq!(s.len(), 1, "kept {s:?}");
        // 353.3° east is 6.7° west.
        assert!((s[0].lon + 6.7).abs() < 0.01, "longitude {}", s[0].lon);
        assert_eq!(s[0].mufd, 20.0);
        assert!(parse_ionosondes("not json").is_empty());
        assert!(parse_ionosondes("[]").is_empty());
    }
}
