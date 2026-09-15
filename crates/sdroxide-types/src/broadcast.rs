//! Longwave and shortwave broadcast stations, and the transmitter site each one
//! radiates from.
//!
//! The shortwave half is [EiBi](https://www.eibispace.de/)'s seasonal schedule,
//! parsed by [`parse_schedule`]. Schedules are reissued twice a year, so
//! `sdroxide-config` downloads the current season's file and caches it; the copy
//! compiled in here is the fallback for a first run with no network, and goes
//! through the same parser, so there is one implementation and one set of
//! behaviours rather than a build-time converter and a runtime one that drift.
//!
//! Longwave and the HF standard-time stations are not in EiBi's file — it starts
//! at 2300 kHz and skips time signals — so they are kept by hand in
//! `broadcast_seed.json` and merged in.
//!
//! The site coordinates and the language, country and target-area names live in
//! EiBi's human-readable README rather than in the schedule, so they are lifted
//! into `broadcast_codes.json` by `tools/gen_broadcast_codes.py` and compiled in.
//! They change very rarely, which is why they are not fetched.
//!
//! Modelled on [`crate::WefaxStation`] — reference data answering "what is this
//! carrier I am hearing". Stations become [`Spot`]s of kind
//! [`SpotKind::Broadcast`] via [`BroadcastStation::to_spot`], so the panadapter
//! overlay, the spot list and the world map render them through exactly the same
//! path as a cluster spot.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::{Spot, SpotKind};

/// One season of EiBi's schedule, for a first run that cannot reach the network.
const FALLBACK_SKED: &str = include_str!("sked-fallback.csv");
/// Hand-kept longwave broadcasters and HF standard-time stations.
const SEED_JSON: &str = include_str!("broadcast_seed.json");
/// Site coordinates and code names lifted from EiBi's README.
const CODES_JSON: &str = include_str!("broadcast_codes.json");

/// The ITU broadcasting allocations, plus the tropical bands.
///
/// EiBi's schedule also carries marine, aeronautical and military voice traffic;
/// restricting to the broadcast bands is what separates a broadcaster from a
/// coast station without needing a curated station list.
const BANDS: &[(f64, f64)] = &[
    (2300.0, 2495.0),
    (3200.0, 3400.0),
    (3900.0, 4000.0),
    (4750.0, 5060.0),
    (5800.0, 6200.0),
    (7200.0, 7600.0),
    (9250.0, 9900.0),
    (11500.0, 12160.0),
    (13570.0, 13870.0),
    (15100.0, 15830.0),
    (17480.0, 17900.0),
    (18900.0, 19020.0),
    (21450.0, 21850.0),
    (25670.0, 26100.0),
];

/// Lookup tables from EiBi's README: `"ROU-t"` -> Tiganesti and its position,
/// plus readable names for the language, country and target-area codes.
#[derive(Debug, Deserialize)]
pub struct BroadcastCodes {
    #[serde(default)]
    pub sites: HashMap<String, (String, f64, f64)>,
    #[serde(default)]
    pub languages: HashMap<String, String>,
    #[serde(default)]
    pub countries: HashMap<String, String>,
    #[serde(default)]
    pub targets: HashMap<String, String>,
}

/// The compiled-in code tables, parsed once.
pub fn codes() -> &'static BroadcastCodes {
    static PARSED: OnceLock<BroadcastCodes> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_json::from_str(CODES_JSON).unwrap_or(BroadcastCodes {
            sites: HashMap::new(),
            languages: HashMap::new(),
            countries: HashMap::new(),
            targets: HashMap::new(),
        })
    })
}

/// Whether a schedule row names something an AM receiver can listen to.
///
/// Jammers exist only to sit on top of a broadcast, numbers stations are not
/// addressed to a listener, and the fax and DRM rows carry nothing an envelope
/// detector can recover.
fn is_listenable(station: &str) -> bool {
    let lower = station.to_ascii_lowercase();
    for phrase in ["jammer", "firedrake", "spy numbers"] {
        if lower.contains(phrase) {
            return false;
        }
    }
    // Whole words only: a station is not disqualified for having "drm" or "fax"
    // buried inside a place name.
    !lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| matches!(w, "fax" | "drm" | "digital"))
}

/// Decode an EiBi file, which is published as latin-1.
///
/// Latin-1 maps byte-for-byte onto the first 256 code points, so this cannot
/// fail — which matters, because a lossy UTF-8 decode would mangle exactly the
/// station names an operator searches for (`Rádio Clube do Pará`).
pub fn decode_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Parse one EiBi `sked-XNN.csv` into stations.
///
/// The format is documented in EiBi's README: eleven semicolon-separated fields,
/// of which this uses frequency, time, days, home country, station, language,
/// target, transmitter-site code and persistence. Rows outside the broadcast
/// bands, marked inactive, or naming something other than a broadcaster are
/// dropped. Unparseable rows are skipped rather than failing the file — a
/// schedule with one bad line is still worth having.
pub fn parse_schedule(csv: &str) -> Vec<BroadcastStation> {
    let codes = codes();
    let mut out = Vec::new();
    for line in csv.lines().skip(1) {
        let f: Vec<&str> = line.split(';').collect();
        if f.len() < 9 {
            continue;
        }
        let Ok(khz) = f[0].trim().parse::<f64>() else { continue };
        if !BANDS.iter().any(|&(lo, hi)| (lo..=hi).contains(&khz)) {
            continue;
        }
        let Ok(persistence) = f[8].trim().parse::<u32>() else { continue };
        // 8 is an inactive entry; 90 and up mark a utility station.
        if persistence == 8 || persistence >= 90 {
            continue;
        }
        let station = f[4].trim();
        if station.is_empty() || !is_listenable(station) || f[5].trim().starts_with('-') {
            continue;
        }
        let (site, lat, lon, country_code) = resolve_site(f[7].trim(), f[3].trim(), codes);

        let lang = f[5]
            .split(',')
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(|c| codes.languages.get(c).map(String::as_str).unwrap_or(c))
            .collect::<Vec<_>>()
            .join("/");
        let target = f[6].trim();
        let target = codes.targets.get(target).map(String::as_str).unwrap_or(target);
        let (start_utc, end_utc) = match parse_window(f[1].trim()) {
            Some((a, b)) => (Some(a), Some(b)),
            None => (None, None),
        };

        out.push(BroadcastStation {
            name: station.to_string(),
            freq_khz: khz,
            site,
            country: codes.countries.get(&country_code).cloned().unwrap_or(country_code),
            lat,
            lon,
            power_kw: None,
            lang,
            target: target.to_string(),
            mode: None,
            start_utc,
            end_utc,
            days: parse_days(f[2].trim()),
            // EiBi's persistence codes 4 and 5 mark a transmission that runs in
            // only one broadcasting season. Everything else is left unseasoned,
            // so a cached file keeps working past the changeover instead of
            // emptying the band.
            season: match persistence {
                4 => Some("B".to_string()),
                5 => Some("A".to_string()),
                _ => None,
            },
        });
    }
    out
}

/// Resolve EiBi's transmitter-site code to a place.
///
/// `/CCC` or `/CCC-x` is a relay in another country; a bare suffix is a site in
/// the station's home country. An unresolvable code still yields the country, so
/// the transmission keeps its waterfall label and simply gets no map dot — better
/// than inventing a position for it.
fn resolve_site(
    code: &str,
    home: &str,
    codes: &BroadcastCodes,
) -> (String, Option<f64>, Option<f64>, String) {
    let (country, suffix) = match code.strip_prefix('/') {
        Some(body) => match body.split_once('-') {
            Some((c, s)) => (c, s),
            None => (body, ""),
        },
        None => (home, code),
    };
    for key in [format!("{country}-{suffix}"), country.to_string()] {
        if let Some((name, lat, lon)) = codes.sites.get(&key) {
            return (name.clone(), Some(*lat), Some(*lon), country.to_string());
        }
    }
    (String::new(), None, None, country.to_string())
}

/// `0800-1000` -> `(800, 1000)`; `0000-2400` and degenerate spans mean around
/// the clock, which this reports as `None`.
fn parse_window(spec: &str) -> Option<(u16, u16)> {
    let (a, b) = spec.split_once('-')?;
    if a.len() != 4 || b.len() != 4 {
        return None;
    }
    let a: u16 = a.parse().ok()?;
    let mut b: u16 = b.parse().ok()?;
    if (a, b) == (0, 2400) || a == b {
        return None;
    }
    if b == 2400 {
        b = 0;
    }
    if a > 2359 || b > 2359 || a % 100 > 59 || b % 100 > 59 {
        return None;
    }
    Some((a, b))
}

const DAY_NAMES: [(&str, u8); 7] =
    [("Mo", 1), ("Tu", 2), ("We", 3), ("Th", 4), ("Fr", 5), ("Sa", 6), ("Su", 7)];

fn day_number(token: &str) -> Option<u8> {
    DAY_NAMES.iter().find(|(n, _)| *n == token).map(|(_, d)| *d)
}

/// EiBi's day spec -> the `1`=Monday..`7`=Sunday digit mask used here.
///
/// Handles `Mo-Fr`, `We-Mo` (wrapping the week), `Tu,Fr`, `SaSu` and specs that
/// are already digits. Anything else — `irr`, `Ram`, `1.Sa`, a date — is not a
/// weekly pattern, so it becomes daily: showing a station that turns out not to
/// be transmitting costs a glance, hiding one that is costs the catch.
fn parse_days(spec: &str) -> String {
    if spec.is_empty() {
        return String::new();
    }
    let mut days: Vec<u8> = Vec::new();
    if spec.bytes().all(|b| b.is_ascii_digit()) {
        days.extend(spec.bytes().map(|b| b - b'0').filter(|d| (1..=7).contains(d)));
    } else if let Some((a, b)) = spec.split_once('-')
        && let (Some(from), Some(to)) = (day_number(a), day_number(b))
    {
        let mut d = from;
        loop {
            days.push(d);
            if d == to {
                break;
            }
            d = d % 7 + 1;
        }
    } else {
        // `Tu,Fr`, `SaSu`, `Mo We` — two-character names, however separated.
        let stripped: String = spec.chars().filter(|c| !matches!(c, ',' | ' ' | '/')).collect();
        // Day names are ASCII pairs. Anything else cannot be one, and bailing
        // here also keeps the byte-pair chunking below off a multi-byte char.
        if !stripped.is_ascii() || !stripped.len().is_multiple_of(2) {
            return String::new();
        }
        let mut chars = stripped.as_bytes().chunks(2);
        let mut all = Vec::new();
        let ok = chars.all(|c| match std::str::from_utf8(c).ok().and_then(day_number) {
            Some(d) => {
                all.push(d);
                true
            }
            None => false,
        });
        if !ok {
            return String::new();
        }
        days = all;
    }
    days.sort_unstable();
    days.dedup();
    if days.len() == 7 {
        // "every day" is the same as leaving the mask off, and shorter.
        return String::new();
    }
    days.iter().map(|d| char::from(b'0' + d)).collect()
}

/// The EiBi season file name for `unix`, e.g. `"a26"` or `"b26"`.
///
/// The winter season spans the new year — B26 runs from late October 2026 to late
/// March 2027 — so it keeps the year it started in.
pub fn season_file(unix: i64) -> String {
    let (_, _, days) = utc_parts(unix);
    let (year, month, _) = civil_from_days(days);
    let season = season_at(unix);
    let year = if season == "B" && month < 6 { year - 1 } else { year };
    format!("{}{:02}", season.to_ascii_lowercase(), year.rem_euclid(100))
}

/// One broadcast transmission: a station, a frequency, the site it comes from,
/// and optionally when it is on the air.
///
/// Only `name` and `freq_khz` are required — everything else defaults — so a
/// hand-added entry can be two fields long and still work.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BroadcastStation {
    /// Station or programme name, as an operator would look for it.
    pub name: String,
    /// Carrier frequency in kHz, the unit broadcast schedules are published in.
    pub freq_khz: f64,
    /// Transmitter site ("Solec Kujawski", "Cypress Creek, SC").
    #[serde(default)]
    pub site: String,
    /// Country the transmitter stands in (not the broadcaster's home country —
    /// a BBC transmission from Ascension says Ascension).
    #[serde(default)]
    pub country: String,
    /// Transmitter latitude in degrees, for the world map.
    #[serde(default)]
    pub lat: Option<f64>,
    /// Transmitter longitude in degrees.
    #[serde(default)]
    pub lon: Option<f64>,
    /// Radiated power in kW.
    #[serde(default)]
    pub power_kw: Option<f64>,
    /// Language(s) of the transmission.
    #[serde(default)]
    pub lang: String,
    /// Target area as the broadcaster describes it ("Africa", "Pacific").
    #[serde(default)]
    pub target: String,
    /// Emission mode, if it is not plain AM (`"SAM"`, `"USB"`, …).
    #[serde(default)]
    pub mode: Option<String>,
    /// Start of the transmission as UTC `HHMM`. `None` means around the clock.
    #[serde(default)]
    pub start_utc: Option<u16>,
    /// End of the transmission as UTC `HHMM`. A value below `start_utc` wraps
    /// past midnight.
    #[serde(default)]
    pub end_utc: Option<u16>,
    /// Days the transmission runs, as digits `1` (Monday) to `7` (Sunday) —
    /// the convention the published schedules use. Empty means daily.
    #[serde(default)]
    pub days: String,
    /// Broadcast season: `"A"` (northern summer) or `"B"` (northern winter).
    /// Absent means the transmission runs in both.
    #[serde(default)]
    pub season: Option<String>,
}

/// The file format: a version, some provenance, and the stations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BroadcastStations {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub updated: String,
    /// Where a generated file came from. Present only on tables sdroxide itself
    /// produced, which is how a copy of a shipped schedule is told apart from a
    /// list the operator wrote.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub stations: Vec<BroadcastStation>,
}

impl BroadcastStation {
    pub fn freq_hz(&self) -> f64 {
        self.freq_khz * 1000.0
    }

    /// The emission mode to tune, defaulting to AM.
    pub fn mode_str(&self) -> &str {
        self.mode.as_deref().filter(|m| !m.is_empty()).unwrap_or("AM")
    }

    /// The emission mode to tune, as a [`crate::Mode`]. The schedule's own
    /// strings mapped onto the modes; anything unrecognised is AM.
    pub fn mode(&self) -> crate::Mode {
        match self.mode_str().to_ascii_uppercase().as_str() {
            "SAM" => crate::Mode::Sam,
            "USB" => crate::Mode::Usb,
            "LSB" => crate::Mode::Lsb,
            "CW" => crate::Mode::Cw,
            "DRM" => crate::Mode::Drm,
            "FM" | "WFM" => crate::Mode::Wfm,
            _ => crate::Mode::Am,
        }
    }

    /// Whether the entry matches a free-text query over the fields a listener
    /// searches by: name, site, country, language and target.
    pub fn matches_query(&self, q: &str) -> bool {
        let q = q.trim().to_ascii_lowercase();
        q.is_empty()
            || [&self.name, &self.site, &self.country, &self.lang, &self.target]
                .iter()
                .any(|f| f.to_ascii_lowercase().contains(&q))
    }

    /// Whether this transmission is scheduled at `unix` (seconds since epoch).
    ///
    /// Three independent gates, any of which passes trivially when the entry
    /// leaves the corresponding field out: the day mask, the broadcast season,
    /// and the UTC window.
    pub fn on_air_at(&self, unix: i64) -> bool {
        let (hhmm, dow, _) = utc_parts(unix);
        if !self.days.is_empty() {
            // `dow` is 0 = Monday, so it maps onto the schedules' 1..=7 digits.
            let want = (b'1' + dow) as char;
            if !self.days.contains(want) {
                return false;
            }
        }
        if let Some(s) = &self.season
            && !s.is_empty()
            && !s.eq_ignore_ascii_case(season_at(unix))
        {
            return false;
        }
        match (self.start_utc, self.end_utc) {
            (Some(start), Some(end)) if start != end => {
                if start < end {
                    (start..end).contains(&hhmm)
                } else {
                    // Wraps past midnight: 2200-0200 is on air at 2300 and 0100.
                    hhmm >= start || hhmm < end
                }
            }
            // A half-specified or degenerate window is treated as around the
            // clock rather than as never — a partially filled-in hand edit
            // should still show the station.
            _ => true,
        }
    }

    /// A human-readable schedule for the spot list: `"24h"`, or `"1800-2100"`
    /// with the day mask appended when it is not daily.
    pub fn schedule_label(&self) -> String {
        match (self.start_utc, self.end_utc) {
            (Some(s), Some(e)) if s != e => {
                if self.days.is_empty() {
                    format!("{s:04}-{e:04}")
                } else {
                    format!("{s:04}-{e:04} d{}", self.days)
                }
            }
            _ => "24h".to_string(),
        }
    }

    /// Render as a [`Spot`] so the existing overlay, list and map render it.
    ///
    /// `when_utc` is set to `now` because a scheduled station has no age: the
    /// overlay's age fade and the feed manager's max-age prune are both about
    /// how stale a *report* is, and neither applies here.
    pub fn to_spot(&self, now_utc: i64) -> Spot {
        let freq_hz = self.freq_hz();
        let mut comment = String::new();
        for part in [self.lang.as_str(), self.target.as_str()] {
            if !part.is_empty() {
                if !comment.is_empty() {
                    comment.push_str(" · ");
                }
                comment.push_str(part);
            }
        }
        if let Some(kw) = self.power_kw {
            if !comment.is_empty() {
                comment.push_str(" · ");
            }
            comment.push_str(&format!("{kw:.0} kW"));
        }
        let reference = match (self.site.is_empty(), self.country.is_empty()) {
            (true, true) => None,
            (false, true) => Some(self.site.clone()),
            (true, false) => Some(self.country.clone()),
            (false, false) => Some(format!("{}, {}", self.site, self.country)),
        };
        Spot {
            id: Spot::make_id(SpotKind::Broadcast, &self.name, freq_hz),
            kind: SpotKind::Broadcast,
            freq_hz,
            call: self.name.clone(),
            // No feed reported this, so `spotter` is free; the spot list shows
            // it where a network spot's age would go, which is the useful thing
            // to know about a scheduled transmission.
            spotter: self.schedule_label(),
            mode: self.mode_str().to_string(),
            comment,
            reference,
            grid: None,
            loc: match (self.lat, self.lon) {
                (Some(lat), Some(lon)) => Some((lat, lon)),
                _ => None,
            },
            when_utc: now_utc,
            snr_db: None,
        }
    }
}

/// The hand-kept longwave and standard-time entries.
/// The metre band a broadcast frequency falls in — `"49m"`, `"MW"`, `"120m"` —
/// for the schedule's band filter and its rows. The edges are the conventional
/// ones the published schedules use; a frequency between bands has no name.
pub fn metre_band(khz: f64) -> Option<&'static str> {
    let k = khz;
    if (148.5..=283.5).contains(&k) {
        return Some("LW");
    }
    if (526.5..=1606.5).contains(&k) {
        return Some("MW");
    }
    if (87_500.0..=108_000.0).contains(&k) {
        return Some("FM");
    }
    METRE_BANDS
        .iter()
        .find(|&&(_, lo, hi)| (lo..=hi).contains(&k))
        .map(|&(name, _, _)| name)
}

/// The shortwave broadcast metre bands, `(name, low_khz, high_khz)` — the
/// conventional edges the schedules use.
///
/// One table, shared by [`metre_band`] and the band selector's metre
/// shortcuts, so the name the schedule shows and the name the band bar offers
/// cannot come to disagree.
pub const METRE_BANDS: &[(&str, f64, f64)] = &[
    ("120m", 2300.0, 2495.0),
    ("90m", 3200.0, 3400.0),
    ("75m", 3900.0, 4000.0),
    ("60m", 4750.0, 5060.0),
    ("49m", 5900.0, 6200.0),
    ("41m", 7200.0, 7450.0),
    ("31m", 9400.0, 9900.0),
    ("25m", 11_600.0, 12_100.0),
    ("22m", 13_570.0, 13_870.0),
    ("19m", 15_100.0, 15_800.0),
    ("16m", 17_480.0, 17_900.0),
    ("15m", 18_900.0, 19_020.0),
    ("13m", 21_450.0, 21_850.0),
    ("11m", 25_670.0, 26_100.0),
];

pub fn seed() -> &'static [BroadcastStation] {
    static PARSED: OnceLock<Vec<BroadcastStation>> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_json::from_str::<BroadcastStations>(SEED_JSON).map(|f| f.stations).unwrap_or_default()
    })
}

/// Merge a schedule with the hand-kept entries, in frequency order.
///
/// The schedule is whatever `sdroxide-config` last downloaded, or the compiled-in
/// fallback; the seed is always added because EiBi covers neither longwave nor
/// the time stations.
/// Utility stations worth labelling: the time signals and VOLMET broadcasts a
/// shortwave listener tunes to, which the EiBi *broadcast* schedule does not
/// carry. They run around the clock — no start or end — and carry no programme
/// language or target, which is what makes them utility rather than broadcast.
pub fn utilities() -> &'static [BroadcastStation] {
    static PARSED: OnceLock<Vec<BroadcastStation>> = OnceLock::new();
    PARSED.get_or_init(|| {
        // (name, kHz, site, country, mode)
        const TABLE: &[(&str, f64, &str, &str, &str)] = &[
            ("WWV time signal", 2500.0, "Fort Collins, CO", "United States", "AM"),
            ("WWV time signal", 5000.0, "Fort Collins, CO", "United States", "AM"),
            ("WWV time signal", 10000.0, "Fort Collins, CO", "United States", "AM"),
            ("WWV time signal", 15000.0, "Fort Collins, CO", "United States", "AM"),
            ("WWV time signal", 20000.0, "Fort Collins, CO", "United States", "AM"),
            ("WWVH time signal", 2500.0, "Kekaha, HI", "United States", "AM"),
            ("WWVH time signal", 5000.0, "Kekaha, HI", "United States", "AM"),
            ("WWVH time signal", 10000.0, "Kekaha, HI", "United States", "AM"),
            ("WWVH time signal", 15000.0, "Kekaha, HI", "United States", "AM"),
            ("CHU time signal", 3330.0, "Ottawa, ON", "Canada", "AM"),
            ("CHU time signal", 7850.0, "Ottawa, ON", "Canada", "AM"),
            ("CHU time signal", 14670.0, "Ottawa, ON", "Canada", "AM"),
            ("RWM time signal", 4996.0, "Moscow", "Russia", "AM"),
            ("RWM time signal", 9996.0, "Moscow", "Russia", "AM"),
            ("RWM time signal", 14996.0, "Moscow", "Russia", "AM"),
            ("BPM time signal", 2500.0, "Pucheng", "China", "AM"),
            ("BPM time signal", 5000.0, "Pucheng", "China", "AM"),
            ("BPM time signal", 10000.0, "Pucheng", "China", "AM"),
            ("BPM time signal", 15000.0, "Pucheng", "China", "AM"),
            ("Shannon VOLMET", 5505.0, "Shannon", "Ireland", "USB"),
            ("Shannon VOLMET", 8957.0, "Shannon", "Ireland", "USB"),
            ("Shannon VOLMET", 13264.0, "Shannon", "Ireland", "USB"),
            ("RAF VOLMET", 5450.0, "United Kingdom", "United Kingdom", "USB"),
            ("RAF VOLMET", 11253.0, "United Kingdom", "United Kingdom", "USB"),
            ("New York VOLMET", 3485.0, "New York, NY", "United States", "USB"),
            ("New York VOLMET", 6604.0, "New York, NY", "United States", "USB"),
            ("New York VOLMET", 10051.0, "New York, NY", "United States", "USB"),
            ("New York VOLMET", 13270.0, "New York, NY", "United States", "USB"),
            ("UVB-76 \"The Buzzer\"", 4625.0, "Moscow", "Russia", "AM"),
        ];
        TABLE
            .iter()
            .map(|&(name, freq_khz, site, country, mode)| BroadcastStation {
                name: name.to_string(),
                freq_khz,
                site: site.to_string(),
                country: country.to_string(),
                mode: Some(mode.to_string()),
                ..Default::default()
            })
            .collect()
    })
}

/// Append the built-in utility stations to a loaded schedule.
///
/// Separate from [`merge`] on purpose: `merge` is about EiBi rows and the
/// parser test pins its totals, so the utilities are added by the caller once
/// the schedule is in hand rather than folded into the count.
pub fn with_utilities(mut schedule: Vec<BroadcastStation>) -> Vec<BroadcastStation> {
    schedule.extend(utilities().iter().cloned());
    schedule
}

pub fn merge(schedule: Vec<BroadcastStation>) -> Vec<BroadcastStation> {
    let mut all = schedule;
    all.extend(seed().iter().cloned());
    all.sort_by(|a, b| {
        a.freq_khz
            .total_cmp(&b.freq_khz)
            .then_with(|| a.start_utc.cmp(&b.start_utc))
            .then_with(|| a.name.cmp(&b.name))
    });
    all
}

/// The compiled-in schedule, parsed once — the offline fallback, and what the
/// browser client uses since it has nowhere to cache a download.
pub fn builtin() -> &'static [BroadcastStation] {
    static PARSED: OnceLock<Vec<BroadcastStation>> = OnceLock::new();
    PARSED.get_or_init(|| merge(parse_schedule(FALLBACK_SKED)))
}

/// The stations on air at `unix`, as spots ready to render.
pub fn on_air(stations: &[BroadcastStation], unix: i64) -> Vec<Spot> {
    stations.iter().filter(|s| s.on_air_at(unix)).map(|s| s.to_spot(unix)).collect()
}

/// How far the dial may sit from a published carrier and still count as being on
/// that station, in Hz.
///
/// Kept below half the 5 kHz shortwave channel spacing so two adjacent channels
/// can never both claim the dial, but wide enough to survive the offset an
/// operator listening in ECSS — one sideband of an AM signal, to duck selective
/// fading — will have tuned in.
pub const NEAR_HZ: f64 = 2000.0;

/// The station the dial is sitting on, if any, and where it transmits from.
///
/// Frequencies are shared: WWV and WWVH are both on 5000 kHz, and three
/// Mongolian transmitters share 209 kHz. Prefer one that is on air now, then the
/// most powerful — between two signals on one channel, that is the one being
/// heard. Modelled on [`crate::WefaxStation::at_dial`].
pub fn at_dial(
    stations: &[BroadcastStation],
    dial_hz: f64,
    unix: i64,
) -> Option<&BroadcastStation> {
    stations.iter().filter(|s| (s.freq_hz() - dial_hz).abs() < NEAR_HZ).max_by(|a, b| {
        a.on_air_at(unix)
            .cmp(&b.on_air_at(unix))
            .then_with(|| a.power_kw.unwrap_or(0.0).total_cmp(&b.power_kw.unwrap_or(0.0)))
    })
}

// ── UTC civil-time helpers ───────────────────────────────────────────────────
//
// Just enough calendar to evaluate a broadcast schedule, so the types crate
// stays dependency-free and wasm-safe. `chrono` would do this too, but this
// crate deliberately carries nothing but `serde`.

/// `(HHMM, weekday, day-of-epoch)` in UTC, weekday 0 = Monday.
fn utc_parts(unix: i64) -> (u16, u8, i64) {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let hhmm = (secs / 3600) * 100 + (secs % 3600) / 60;
    // 1970-01-01 was a Thursday, which is index 3 with Monday at 0.
    let dow = (days + 3).rem_euclid(7) as u8;
    (hhmm as u16, dow, days)
}

/// Civil `(year, month, day)` from a day count since the epoch. Hinnant's
/// `civil_from_days`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The broadcast season at `unix`: `"A"` from the last Sunday in March to the
/// last Sunday in October, `"B"` for the rest of the year — the HFCC seasons
/// published schedules are keyed to.
pub fn season_at(unix: i64) -> &'static str {
    let (_, _, days) = utc_parts(unix);
    let (y, m, d) = civil_from_days(days);
    match m {
        4..=9 => "A",
        1..=2 | 11..=12 => "B",
        3 => {
            if d >= last_sunday(y, 3) {
                "A"
            } else {
                "B"
            }
        }
        // October: still A until the last Sunday.
        _ => {
            if d >= last_sunday(y, 10) {
                "B"
            } else {
                "A"
            }
        }
    }
}

/// Day-of-month of the last Sunday in `month` of `year`. Only ever called for
/// March and October, both of which have 31 days.
fn last_sunday(year: i64, month: u32) -> u32 {
    const LAST: u32 = 31;
    let days = days_from_civil(year, month, LAST);
    // weekday 0 = Monday, so Sunday is 6 and the 31st is `dow + 1` days past
    // the Sunday we want (wrapping when the 31st *is* a Sunday).
    let dow = (days + 3).rem_euclid(7) as u32;
    LAST - ((dow + 1) % 7)
}

/// Inverse of [`civil_from_days`].
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-07-30 12:34:00 UTC — a Thursday, day 20664 of the epoch.
    const THU_1234: i64 = 20_664 * 86_400 + 12 * 3600 + 34 * 60;

    #[test]
    fn utc_parts_splits_a_known_instant() {
        let (hhmm, dow, days) = utc_parts(THU_1234);
        assert_eq!(hhmm, 1234);
        assert_eq!(dow, 3, "Thursday is index 3 with Monday at 0");
        assert_eq!(civil_from_days(days), (2026, 7, 30));
    }

    #[test]
    fn the_epoch_was_a_thursday() {
        let (hhmm, dow, days) = utc_parts(0);
        assert_eq!((hhmm, dow), (0, 3));
        assert_eq!(civil_from_days(days), (1970, 1, 1));
    }

    #[test]
    fn civil_dates_round_trip() {
        for &(y, m, d) in &[(1970, 1, 1), (2000, 2, 29), (2026, 7, 30), (2038, 12, 31)] {
            assert_eq!(civil_from_days(days_from_civil(y, m, d)), (y, m, d));
        }
    }

    fn at(start: Option<u16>, end: Option<u16>, days: &str) -> BroadcastStation {
        BroadcastStation {
            name: "test".into(),
            freq_khz: 6070.0,
            site: String::new(),
            country: String::new(),
            lat: None,
            lon: None,
            power_kw: None,
            lang: String::new(),
            target: String::new(),
            mode: None,
            start_utc: start,
            end_utc: end,
            days: days.into(),
            season: None,
        }
    }

    #[test]
    fn a_station_without_a_window_is_always_on_air() {
        assert!(at(None, None, "").on_air_at(THU_1234));
        // A half-filled window counts as unspecified, not as never.
        assert!(at(Some(900), None, "").on_air_at(THU_1234));
        assert!(at(Some(900), Some(900), "").on_air_at(THU_1234));
    }

    #[test]
    fn a_plain_window_brackets_the_current_time() {
        assert!(at(Some(1200), Some(1300), "").on_air_at(THU_1234));
        assert!(!at(Some(1300), Some(1400), "").on_air_at(THU_1234));
        // The end is exclusive, so back-to-back slots never both claim a minute.
        assert!(!at(Some(1100), Some(1234), "").on_air_at(THU_1234));
        assert!(at(Some(1234), Some(1300), "").on_air_at(THU_1234));
    }

    #[test]
    fn a_window_may_wrap_past_midnight() {
        let overnight = at(Some(2200), Some(200), "");
        assert!(overnight.on_air_at(THU_1234 + 11 * 3600), "2334 is inside 2200-0200");
        assert!(overnight.on_air_at(THU_1234 + 13 * 3600), "0134 is inside 2200-0200");
        assert!(!overnight.on_air_at(THU_1234), "1234 is not");
    }

    #[test]
    fn the_day_mask_uses_schedule_numbering() {
        assert!(at(None, None, "4").on_air_at(THU_1234), "Thursday is 4");
        assert!(at(None, None, "1234567").on_air_at(THU_1234));
        assert!(at(None, None, "67").on_air_at(THU_1234 + 2 * 86_400), "Saturday is 6");
        assert!(!at(None, None, "67").on_air_at(THU_1234));
    }

    #[test]
    fn seasons_change_on_the_last_sunday_in_march_and_october() {
        // 2026: last Sunday in March is the 29th, last Sunday in October the 25th.
        let d = |y, m, day| days_from_civil(y, m, day) * 86_400 + 12 * 3600;
        assert_eq!(season_at(d(2026, 3, 28)), "B");
        assert_eq!(season_at(d(2026, 3, 29)), "A");
        assert_eq!(season_at(d(2026, 7, 30)), "A");
        assert_eq!(season_at(d(2026, 10, 24)), "A");
        assert_eq!(season_at(d(2026, 10, 25)), "B");
        assert_eq!(season_at(d(2026, 12, 31)), "B");
        assert_eq!(season_at(d(2027, 1, 1)), "B");
    }

    #[test]
    fn a_seasonal_entry_is_gated_by_the_season() {
        let mut s = at(None, None, "");
        s.season = Some("B".into());
        let summer = days_from_civil(2026, 7, 30) * 86_400;
        let winter = days_from_civil(2026, 12, 15) * 86_400;
        assert!(!s.on_air_at(summer));
        assert!(s.on_air_at(winter));
    }

    #[test]
    fn the_bundled_table_parses_and_looks_like_broadcast_data() {
        let all = builtin();
        assert!(all.len() > 100, "expected the curated table, got {} entries", all.len());
        for s in all {
            assert!(!s.name.is_empty(), "unnamed station at {} kHz", s.freq_khz);
            let khz = s.freq_khz;
            let lw = (148.5..=283.5).contains(&khz);
            let hf = (2300.0..=27_000.0).contains(&khz);
            assert!(lw || hf, "{} at {khz} kHz is neither longwave nor shortwave", s.name);
            if let Some(lat) = s.lat {
                assert!((-90.0..=90.0).contains(&lat), "{} has latitude {lat}", s.name);
            }
            if let Some(lon) = s.lon {
                assert!((-180.0..=180.0).contains(&lon), "{} has longitude {lon}", s.name);
            }
            // A site without coordinates would be invisible on the world map,
            // which defeats the point of recording the site at all.
            assert_eq!(s.lat.is_some(), s.lon.is_some(), "{} has half a coordinate pair", s.name);
            assert!(
                s.mode_str().parse::<crate::Mode>().is_ok(),
                "{} has mode {:?}",
                s.name,
                s.mode
            );
            if !s.days.is_empty() {
                assert!(
                    s.days.chars().all(|c| ('1'..='7').contains(&c)),
                    "{} has day mask {:?}",
                    s.name,
                    s.days
                );
            }
            for t in [s.start_utc, s.end_utc].into_iter().flatten() {
                assert!(t % 100 < 60 && t / 100 < 24, "{} has time {t}", s.name);
            }
        }
    }

    #[test]
    fn every_longwave_entry_is_a_station_that_is_still_transmitting() {
        // The published lists are full of closed longwave transmitters and it is
        // easy to copy one in by accident, so pin the ones that must not appear.
        let closed = ["Droitwich", "Kalundborg", "Lahti", "Gufuskalar", "Burg", "Saarlouis"];
        for s in builtin().iter().filter(|s| s.freq_khz < 300.0) {
            for c in closed {
                assert!(!s.site.contains(c), "{} lists the closed transmitter at {c}", s.name);
            }
        }
    }

    #[test]
    fn a_station_becomes_a_tunable_spot() {
        let s = builtin().iter().find(|s| s.freq_khz == 225.0).expect("225 kHz");
        let spot = s.to_spot(THU_1234);
        assert_eq!(spot.kind, SpotKind::Broadcast);
        assert_eq!(spot.freq_hz, 225_000.0);
        assert_eq!(spot.call, "Polskie Radio Program 1");
        assert_eq!(spot.reference.as_deref(), Some("Solec Kujawski, Poland"));
        assert!(spot.comment.contains("Polish"));
        assert!(spot.comment.contains("1000 kW"));
        assert_eq!(spot.radio_mode(), Some(crate::Mode::Am));
        assert!(spot.loc.is_some());
    }

    #[test]
    fn the_dial_finds_the_station_on_it() {
        let all = builtin();
        let s = at_dial(all, 225_000.0, THU_1234).expect("225 kHz");
        assert_eq!(s.name, "Polskie Radio Program 1");
        // Slightly off frequency still counts — an ECSS listener is never exact.
        assert!(at_dial(all, 225_000.0 + 1500.0, THU_1234).is_some());
        assert!(at_dial(all, 225_000.0 - 1500.0, THU_1234).is_some());
        // Well off it does not.
        assert!(at_dial(all, 240_000.0, THU_1234).is_none());
    }

    #[test]
    fn a_shared_frequency_picks_the_stronger_transmitter() {
        // Three Mongolian transmitters share 209 kHz at 75/75/30 kW.
        let s = at_dial(builtin(), 209_000.0, THU_1234).expect("209 kHz");
        assert_eq!(s.power_kw, Some(75.0), "picked {} at {:?} kW", s.site, s.power_kw);
    }

    #[test]
    fn adjacent_shortwave_channels_do_not_claim_each_others_dial() {
        // The 5 kHz channel raster, which the tolerance has to stay inside.
        let ch = |khz: f64| BroadcastStation { freq_khz: khz, ..at(None, None, "") };
        let three = [ch(9535.0), ch(9540.0), ch(9545.0)];
        for want in &three {
            let hit = at_dial(&three, want.freq_hz(), THU_1234).expect("its own channel");
            assert_eq!(hit.freq_khz, want.freq_khz);
        }
    }

    #[test]
    fn the_dial_never_matches_a_station_it_is_not_near() {
        // Real schedules do put two stations 1 kHz apart, so `at_dial` may well
        // return a neighbour rather than an exact hit — but never one further
        // off than the tolerance allows.
        for s in builtin() {
            let hit = at_dial(builtin(), s.freq_hz(), THU_1234).expect("at least itself");
            assert!(
                (hit.freq_hz() - s.freq_hz()).abs() < NEAR_HZ,
                "tuned {} kHz, matched {} on {} kHz",
                s.freq_khz,
                hit.name,
                hit.freq_khz
            );
        }
    }

    #[test]
    fn the_parser_reproduces_the_reference_conversion() {
        // The Python tool that used to do this conversion at build time turned
        // the A-26 file into 4,629 transmissions, plus 23 hand-kept entries.
        // Pinning the totals is what catches the parser silently losing rows.
        assert_eq!(seed().len(), 23);
        assert_eq!(parse_schedule(FALLBACK_SKED).len(), 4629);
        assert_eq!(builtin().len(), 4652);
    }

    #[test]
    fn a_schedule_row_becomes_a_station() {
        let csv = "kHz;Time(UTC);Days;ITU;Station;Lng;Target;Remarks;P;Start;Stop;\n\
                   15400;1800-1900;Mo-Fr;G;BBC;E;WAf;/ASC;0;;\n";
        let got = parse_schedule(csv);
        assert_eq!(got.len(), 1);
        let s = &got[0];
        assert_eq!(s.name, "BBC");
        assert_eq!(s.freq_khz, 15400.0);
        assert_eq!(s.site, "Ascension Island");
        assert_eq!(s.country, "Ascension Island");
        assert_eq!(s.lang, "English");
        assert_eq!(s.target, "West Africa");
        assert_eq!((s.start_utc, s.end_utc), (Some(1800), Some(1900)));
        assert_eq!(s.days, "12345");
        assert!(s.lat.is_some() && s.lon.is_some());
    }

    #[test]
    fn the_parser_drops_what_an_am_receiver_cannot_use() {
        let row = |f: &str, station: &str, lng: &str, p: &str| {
            format!("{f};0000-2400;;CHN;{station};{lng};As;;{p};;\n")
        };
        let csv = String::from("header\n")
            + &row("9420", "Good Station", "E", "1")     // kept
            + &row("9420", "CNR1 Jammer", "E", "1")      // jammer
            + &row("9420", "E06 Spy Numbers", "E", "1")  // numbers station
            + &row("9420", "CRI DIGITAL", "E", "1")      // DRM, no envelope
            + &row("9420", "Retired Service", "E", "8")  // inactive
            + &row("9420", "Coast Radio", "-CW", "1")    // not voice
            + &row("8000", "Out Of Band", "E", "1"); // not a broadcast band
        let got = parse_schedule(&csv);
        assert_eq!(got.len(), 1, "kept {:?}", got.iter().map(|s| &s.name).collect::<Vec<_>>());
        assert_eq!(got[0].name, "Good Station");
    }

    #[test]
    fn day_specs_convert_to_the_digit_mask() {
        assert_eq!(parse_days(""), "");
        assert_eq!(parse_days("Mo-Fr"), "12345");
        assert_eq!(parse_days("Su"), "7");
        assert_eq!(parse_days("Tu,Fr"), "25");
        assert_eq!(parse_days("SaSu"), "67");
        assert_eq!(parse_days("156"), "156");
        // A range may wrap the week: Saturday through Thursday is all but Friday.
        assert_eq!(parse_days("Sa-Th"), "123467");
        // Every day is the same as no mask at all.
        assert_eq!(parse_days("Mo-Su"), "");
        assert_eq!(parse_days("1234567"), "");
        // Not weekly patterns — treated as daily rather than as never.
        for spec in ["irr", "Ram", "1.Sa", "15Sep", "tent", "Last7"] {
            assert_eq!(parse_days(spec), "", "{spec}");
        }
    }

    #[test]
    fn windows_convert_and_round_the_clock_means_no_window() {
        assert_eq!(parse_window("0800-1000"), Some((800, 1000)));
        assert_eq!(parse_window("2200-0200"), Some((2200, 200)));
        // 2400 is midnight at the far end, and a full day is "no window".
        assert_eq!(parse_window("1800-2400"), Some((1800, 0)));
        assert_eq!(parse_window("0000-2400"), None);
        assert_eq!(parse_window("0900-0900"), None);
        assert_eq!(parse_window("nonsense"), None);
        assert_eq!(parse_window("0870-0900"), None, "70 is not a minute");
    }

    #[test]
    fn latin1_station_names_survive_decoding() {
        // 0xE1 is á in latin-1. A lossy UTF-8 decode would replace it and break
        // searching for the station by name.
        assert_eq!(decode_latin1(b"R\xe1dio Clube do Par\xe1"), "Rádio Clube do Pará");
        assert!(builtin().iter().any(|s| s.name.contains("Rádio")));
    }

    #[test]
    fn the_season_file_name_follows_the_broadcasting_year() {
        let at = |y, m, d| days_from_civil(y, m, d) * 86_400 + 12 * 3600;
        assert_eq!(season_file(at(2026, 7, 30)), "a26");
        assert_eq!(season_file(at(2026, 4, 1)), "a26");
        assert_eq!(season_file(at(2026, 10, 24)), "a26");
        // The winter season keeps the year it started in, across the new year.
        assert_eq!(season_file(at(2026, 10, 25)), "b26");
        assert_eq!(season_file(at(2026, 12, 31)), "b26");
        assert_eq!(season_file(at(2027, 1, 1)), "b26");
        assert_eq!(season_file(at(2027, 3, 27)), "b26");
        assert_eq!(season_file(at(2027, 3, 28)), "a27");
    }

    #[test]
    fn the_bundled_table_carries_real_transmit_windows() {
        // The point of generating from a published schedule rather than by hand:
        // most transmissions are time-limited, and the waterfall only labels the
        // ones on air. A table that had lost its windows would look fine but
        // would quietly show every station at every hour.
        let all = builtin();
        let windowed = all.iter().filter(|s| s.start_utc.is_some()).count();
        assert!(
            windowed * 2 > all.len(),
            "only {windowed} of {} entries have a transmit window",
            all.len()
        );
        assert!(all.iter().any(|s| !s.days.is_empty()), "no entry carries a day mask");
        // And the filter has to actually thin them out over the day.
        let day = days_from_civil(2026, 7, 30) * 86_400;
        let counts: Vec<usize> = (0..24).map(|h| on_air(all, day + h * 3600).len()).collect();
        let (lo, hi) = (*counts.iter().min().unwrap(), *counts.iter().max().unwrap());
        assert!(lo > 0, "nothing on air at some hour of the day");
        assert!(hi < all.len() / 2, "the schedule filter barely removes anything");
    }

    #[test]
    fn on_air_keeps_the_unscheduled_stations() {
        let spots = on_air(builtin(), THU_1234);
        assert!(spots.iter().any(|s| s.freq_hz == 225_000.0));
        assert!(spots.iter().all(|s| s.kind == SpotKind::Broadcast));
    }
}

#[cfg(test)]
mod schedule_query_tests {
    use super::*;

    #[test]
    fn metre_bands_are_named() {
        assert_eq!(metre_band(6185.0), Some("49m"));
        assert_eq!(metre_band(9410.0), Some("31m"));
        assert_eq!(metre_band(1000.0), Some("MW"));
        assert_eq!(metre_band(200.0), Some("LW"));
        assert_eq!(metre_band(50_000.0), None);
    }

    #[test]
    fn queries_match_the_fields_a_listener_searches() {
        let s: BroadcastStation = serde_json::from_str(
            r#"{"name":"BBC World Service","freq_khz":6185.0,"site":"Ascension",
                "country":"Ascension","lang":"English","target":"Africa"}"#,
        )
        .unwrap();
        assert!(s.matches_query("bbc"));
        assert!(s.matches_query("ENGLISH"));
        assert!(s.matches_query("ascension"));
        assert!(s.matches_query("africa"));
        assert!(!s.matches_query("romania"));
        assert!(s.matches_query(""), "an empty query matches everything");
    }

    #[test]
    fn modes_map_onto_the_receiver_with_am_as_the_default() {
        let mut s: BroadcastStation =
            serde_json::from_str(r#"{"name":"X","freq_khz":6000.0}"#).unwrap();
        assert_eq!(s.mode(), crate::Mode::Am);
        s.mode = Some("USB".into());
        assert_eq!(s.mode(), crate::Mode::Usb);
        s.mode = Some("SAM".into());
        assert_eq!(s.mode(), crate::Mode::Sam);
    }
}

#[cfg(test)]
mod utility_tests {
    use super::*;

    #[test]
    fn the_utility_table_is_sane() {
        let u = utilities();
        assert!(u.len() >= 20, "a small table, not empty: {}", u.len());
        assert!(u.iter().any(|s| s.name.contains("WWV")), "time signals are in");
        assert!(u.iter().any(|s| s.name.contains("VOLMET")), "and the VOLMETs");
        // Nothing outside HF/MF, and every one names a site.
        for s in u {
            assert!((200.0..=30_000.0).contains(&s.freq_khz), "{} kHz", s.freq_khz);
            assert!(!s.site.is_empty(), "{} has no site", s.name);
            assert_eq!(s.start_utc, None, "{} runs around the clock", s.name);
            assert!(s.on_air_at(0), "{} is on at any time", s.name);
        }
    }

    #[test]
    fn utilities_ride_along_with_a_loaded_schedule() {
        let with = with_utilities(seed().to_vec());
        assert_eq!(with.len(), seed().len() + utilities().len());
        assert!(with.iter().any(|s| s.name.contains("WWV")), "added in");
    }
}
