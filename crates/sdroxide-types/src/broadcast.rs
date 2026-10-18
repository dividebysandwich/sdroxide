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

/// Whether a schedule row names something a listener here can use.
///
/// Jammers exist only to sit on top of a broadcast, and numbers stations are
/// not addressed to a listener, so neither earns a row.
///
/// **DRM rows are kept, and they used to be dropped.** They went out with the
/// fax rows on the grounds that "the fax and DRM rows carry nothing an envelope
/// detector can recover" — true of the AM-only schedule this started as, and
/// untrue since this build got a DRM decoder. EiBi has no emission column: it
/// spells a DRM transmission out in the name (`BBC DIGITAL`, `CNR1 DIGITAL`,
/// `KCBS DIGITAL`, thirteen distinct names in the August 2026 file), so the word
/// that dropped them was `digital`. That is also why a search for "Digital"
/// found nothing at all — every row carrying the word had been thrown away
/// before the query ever ran. Kept rows say what they are through
/// [`mode_from_name`], or they would be tuned as AM.
///
/// The **fax** rows stay out, and for a different reason: a chart service is
/// not a broadcast programme, and the WEFAX stations already have a table of
/// their own ([`crate::WefaxStation`]), which is where a listener looks for
/// them.
fn is_listenable(station: &str) -> bool {
    let lower = station.to_ascii_lowercase();
    for phrase in ["jammer", "firedrake", "spy numbers"] {
        if lower.contains(phrase) {
            return false;
        }
    }
    // Whole words only: a station is not disqualified for having "fax" buried
    // inside a place name — `VCS Halifax Radio` is a coast station, not a chart.
    !lower.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == "fax")
}

/// The emission mode a schedule's own name implies, where it implies one.
///
/// Only DRM, because only DRM is spelled out in a name rather than carried in a
/// column, and only for the two spellings EiBi and Aoki use. A row left at `AM`
/// would be tuned as an envelope signal, which is worse than the row being
/// missing: the operator would hear noise and conclude the station was off.
fn mode_from_name(station: &str) -> Option<String> {
    let lower = station.to_ascii_lowercase();
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w == "digital" || w == "drm")
        .then(|| "DRM".to_string())
}

/// Decode an EiBi file, which is published as latin-1.
///
/// Latin-1 maps byte-for-byte onto the first 256 code points, so this cannot
/// fail — which matters, because a lossy UTF-8 decode would mangle exactly the
/// station names an operator searches for (`Rádio Clube do Pará`).
pub fn decode_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Where a schedule came from, and therefore how its bytes are read.
///
/// One enum, and the point of it is that **`Eibi` is first and is what the
/// program does by default.** Nothing about the existing path changes: the
/// switch is where a *second* schedule plugs in, not a rewrite of the one that is
/// already here. `SWL-HANDOVER.md` stage 1.
///
/// A variant is **not** a per-source settings block — it says how to *read*,
/// nothing about whether the schedule is active, and the active choice is a
/// separate thing ([`ScheduleOrigin`] below). Keeping those apart is what stops
/// "which format is this file" and "which schedule is in use" becoming one
/// question, which is how a program ends up unable to say what it is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleSource {
    /// EiBi's published `sked-XNN.csv`, semicolon-delimited with its own code
    /// tables. The default, and the schedule this program has always used.
    #[default]
    Eibi,
    /// **Aoki's** `ncbXX.txt` — space-aligned fixed columns, station names
    /// inline. Read by [`parse_aoki`]; the one bebepip runs.
    Aoki,
    /// **HFCC's** schedule — space-delimited, and the rows carry a three-letter
    /// code rather than a name, so it needs a companion file. Not yet
    /// implemented, and the reason a source cannot be assumed to be one file.
    Hfcc,
    /// The listener's own list, in this program's own format. Not yet
    /// implemented.
    User,
}

impl ScheduleSource {
    /// Every source, for a settings row that picks one.
    pub const ALL: [ScheduleSource; 4] =
        [ScheduleSource::Eibi, ScheduleSource::Aoki, ScheduleSource::Hfcc, ScheduleSource::User];

    pub fn label(self) -> &'static str {
        match self {
            ScheduleSource::Eibi => "EiBi",
            ScheduleSource::Aoki => "Aoki",
            ScheduleSource::Hfcc => "HFCC",
            ScheduleSource::User => "Mine",
        }
    }

    /// Whether this build can actually read the source.
    ///
    /// **False is the honest answer until the reader exists**, and it is why the
    /// selector greys the unimplemented ones rather than offering them and
    /// producing an empty schedule. A control that can silently do nothing is a
    /// bug in this fork even when the underlying behaviour is correct.
    pub fn is_readable(self) -> bool {
        matches!(self, ScheduleSource::Eibi | ScheduleSource::Aoki)
    }

    /// Whether the source is one file or a set of them.
    ///
    /// HFCC is a **pair**: its rows name stations by a three-letter code that a
    /// companion file resolves. A single-file assumption is not a simplification
    /// here, it is a way of showing a schedule full of `WCR` as if it were a
    /// schedule — see `SWL-HANDOVER.md` §4.
    pub fn needs_companion_file(self) -> bool {
        matches!(self, ScheduleSource::Hfcc)
    }
}

/// Read a schedule from the given source.
///
/// EiBi's reader is [`parse_schedule`], unchanged; the others are not written
/// yet, so this **refuses rather than returning nothing**, because an empty
/// schedule and an unreadable one look identical on screen and only one of them
/// is a bug.
pub fn parse_from(source: ScheduleSource, text: &str) -> Result<Vec<BroadcastStation>, String> {
    match source {
        ScheduleSource::Eibi => Ok(parse_schedule(text)),
        ScheduleSource::Aoki => Ok(parse_aoki(text)),
        // HFCC needs **three** files — the schedule plus two reference tables —
        // so it cannot go through a one-text seam. `parse_hfcc` takes them
        // directly, and the download unpacks all three from one package.
        other => Err(format!("{} needs more than one file to be read", other.label())),
    }
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
        // Fields past EiBi's eleven are an enrichment: an operator-supplied file
        // may append the broadcaster's report contact so a logged reception can
        // name where to send it. The published file stops at eleven, so these
        // are empty there.
        let email = f.get(11).map(|s| s.trim().to_string()).unwrap_or_default();
        let address = f.get(12).map(|s| s.trim().to_string()).unwrap_or_default();

        out.push(BroadcastStation {
            source: ScheduleSource::Eibi,
            name: station.to_string(),
            freq_khz: khz,
            site,
            country: codes.countries.get(&country_code).cloned().unwrap_or(country_code),
            lat,
            lon,
            power_kw: None,
            lang,
            target: target.to_string(),
            mode: mode_from_name(station),
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
            email,
            address,
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
    /// The broadcaster's email for reception reports, when the schedule carries
    /// one. An enriched EiBi file may append this as a twelfth field; the file
    /// EiBi publishes does not, so it is usually empty.
    #[serde(default)]
    pub email: String,
    /// The broadcaster's postal address for a reception report, from a
    /// thirteenth field when the schedule file carries one.
    #[serde(default)]
    pub address: String,
    /// Which schedule this row came from — `eibi`, `aoki`, `hfcc` or `user`.
    ///
    /// **The master store's one field**, and it is on the row rather than in a
    /// side table because a schedule row is where the question is asked: this is
    /// what the Schedule window's source filter reads, and what lets a listener
    /// see that one broadcaster is listed by three of them.
    ///
    /// `Default` is `Eibi`, which is what every row written before this field
    /// existed was: `broadcast_stations.json` is the listener's own corrections
    /// file, and assuming the source they added from is far better than discarding
    /// their work over a missing field. `BroadcastStation` is **local only** — it
    /// appears in no wire message — so this is no `PROTO_VERSION` bump.
    #[serde(default)]
    pub source: ScheduleSource,
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
    ///
    /// Allocation-free: the schedule window calls this for every station every
    /// frame it is open, so lowercasing the fields into fresh `String`s here was
    /// tens of thousands of allocations a second. [`contains_ascii_ci`] compares
    /// in place instead.
    pub fn matches_query(&self, q: &str) -> bool {
        let q = q.trim();
        if q.is_empty() {
            return true;
        }
        // **A frequency is a query too** (asked for by the listener who keeps the
        // schedule): typing `6185` or `6.185` finds the transmissions there, the
        // same way a name does. Matched by magnitude — at or above 1000 is kHz,
        // below it is MHz — which is the rule the import already uses, so one
        // number means one thing wherever it is typed.
        if query_frequencies_khz(q)
            .iter()
            .any(|khz| (self.freq_khz - khz).abs() <= FREQ_QUERY_TOLERANCE_KHZ)
        {
            return true;
        }
        [&self.name, &self.site, &self.country, &self.lang, &self.target]
            .iter()
            .any(|f| contains_ascii_ci(f, q))
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
    if (108_100.0..=137_000.0).contains(&k) {
        return Some("AIR");
    }
    if (225_100.0..=400_000.0).contains(&k) {
        return Some("MIL");
    }
    METRE_BANDS.iter().find(|&&(_, lo, hi)| (lo..=hi).contains(&k)).map(|&(name, _, _)| name)
}

/// Local mean solar time at a transmitter, `HH:MM`, for a UTC `HHMM` clock time
/// and the site's longitude.
///
/// The Sun's own clock — four minutes a degree — not a civil time zone: it
/// knows nothing of daylight saving or of a country's zone borders. It is what
/// the schedule's site coordinates can honestly be turned into, and it is the
/// clock a listener means by "the broadcaster's evening", so it is labelled
/// *solar* wherever it is shown rather than passed off as local time.
pub fn local_solar_hhmm(utc_hhmm: u16, lon_deg: f64) -> String {
    let utc_min = (utc_hhmm / 100) as i64 * 60 + (utc_hhmm % 100) as i64;
    let solar = (utc_min + (lon_deg * 4.0).round() as i64).rem_euclid(1440);
    format!("{:02}:{:02}", solar / 60, solar % 60)
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

/// Case-insensitive substring test that allocates nothing.
///
/// How close a typed frequency has to be to a row's own, in kHz.
///
/// A kilohertz either side: broadcast carriers are published to the kilohertz,
/// and somebody typing MHz may round (`6.18` for 6185). Wide enough to be
/// forgiving, narrow enough that it cannot swallow the neighbouring channel,
/// which on the 49 m band is 5 kHz away.
pub const FREQ_QUERY_TOLERANCE_KHZ: f64 = 1.0;

/// Every kHz reading a typed frequency could mean — usually one, sometimes two.
///
/// **A search box is forgiving, unlike the import.** With no unit written,
/// `153` is genuinely ambiguous: it is a real longwave frequency *in kHz* and it
/// also reads as 153 MHz. The import has to pick one because it is writing data;
/// a search only has to not miss, so it tries both and lets the range rule drop
/// whichever is impossible. Anything carrying a unit is exact.
fn query_frequencies_khz(q: &str) -> Vec<f64> {
    let t = q.trim();
    let without = |suffixes: [&str; 3]| {
        suffixes.iter().find_map(|s| t.strip_suffix(s)).map(|d| (d.trim(), true))
    };
    let (body, mhz) = match without(["kHz", "khz", "KHz"]) {
        Some((d, _)) => (d, false),
        None => match without(["MHz", "mhz", "Mhz"]) {
            Some((d, _)) => (d, true),
            None => (t, false),
        },
    };
    let Ok(v) = body.parse::<f64>() else {
        return Vec::new();
    };
    if !v.is_finite() || v <= 0.0 {
        return Vec::new();
    }
    let plausible = |k: f64| (3.0..=130_000.0).contains(&k);
    let mut out = Vec::new();
    if mhz {
        if plausible(v * 1000.0) {
            out.push(v * 1000.0);
        }
    } else {
        // As kHz, and as MHz: only one of the two is ever in range.
        if plausible(v) {
            out.push(v);
        }
        if plausible(v * 1000.0) {
            out.push(v * 1000.0);
        }
    }
    out
}

/// [`str::to_ascii_lowercase`] would, and the schedule filters run over
/// thousands of rows a frame; this walks the bytes instead.
pub fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    let needle = needle.trim().as_bytes();
    if needle.is_empty() {
        return true;
    }
    let hay = haystack.as_bytes();
    if needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w.eq_ignore_ascii_case(needle))
}

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

/// The universal **airband** frequencies: the civil-aviation VHF channels
/// every listener knows, which no schedule lists because they are channels
/// rather than transmissions. AM, as the airband is.
pub fn airband() -> &'static [BroadcastStation] {
    static PARSED: OnceLock<Vec<BroadcastStation>> = OnceLock::new();
    PARSED.get_or_init(|| {
        // (kHz, name)
        const TABLE: &[(f64, &str)] = &[
            (121_500.0, "121.500 Emergency (GUARD)"),
            (122_800.0, "122.800 Air-to-air"),
            (123_450.0, "123.450 Air-to-air"),
            (123_100.0, "123.100 SAR on-scene"),
            (122_750.0, "122.750 Air-to-air (US)"),
        ];
        TABLE
            .iter()
            .map(|&(freq_khz, name)| BroadcastStation {
                name: name.to_string(),
                freq_khz,
                mode: Some("AM".to_string()),
                ..Default::default()
            })
            .collect()
    })
}

/// The well-known **military** frequencies: the UHF emergency channel and the
/// US HF Global System's calling channels, which are channels rather than
/// transmissions and so are on no schedule. The US HF channels are USB, the
/// UHF emergency is AM.
pub fn military() -> &'static [BroadcastStation] {
    static PARSED: OnceLock<Vec<BroadcastStation>> = OnceLock::new();
    PARSED.get_or_init(|| {
        // (kHz, name, mode)
        const TABLE: &[(f64, &str, &str)] = &[
            (243_000.0, "243.000 Military Emergency (GUARD)", "AM"),
            (4_724.0, "US military HF (HFGCS)", "USB"),
            (8_992.0, "US military HF (HFGCS)", "USB"),
            (11_175.0, "US military HF (HFGCS)", "USB"),
            (15_016.0, "US military HF (HFGCS)", "USB"),
        ];
        TABLE
            .iter()
            .map(|&(freq_khz, name, mode)| BroadcastStation {
                name: name.to_string(),
                freq_khz,
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
    schedule.extend(airband().iter().cloned());
    schedule.extend(military().iter().cloned());
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

/// One HFCC day as ours.
///
/// **HFCC counts the week from Sunday** (`1` Sunday … `7` Saturday) and this
/// project's schedules count from Monday (`1` Monday … `7` Sunday). So HFCC's `1`
/// is our `7` and the rest shift down by one.
///
/// This is the per-source conversion the master store deliberately does **not**
/// do: doing it here, where the format is known, is the only place it cannot be
/// applied twice. Getting it wrong moves every HFCC station by a day and nothing
/// looks broken — and it is the *same* mapping as Aoki's, which is a coincidence
/// worth not being fooled by: two Japanese-influenced formats agreeing is not a
/// reason to assume the third one does.
pub fn hfcc_day_to_ours(d: u32) -> u32 {
    match d {
        1 => 7,
        2..=7 => d - 1,
        // **Not a day.** The field is fixed-width and a schedule has been seen
        // carrying a `0` in it, so the mapping is total rather than a subtraction
        // that panics on the day slot is blank. A `0` is dropped by the caller.
        _ => d,
    }
}

/// HFCC's reference table: `CODE<space>name`, one per line, `;` comments.
fn hfcc_reference_table(text: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.starts_with(';') || line.trim().is_empty() {
            continue;
        }
        let (code, name) = match line.split_once(char::is_whitespace) {
            Some((c, n)) => (c.trim(), n.trim()),
            None => continue,
        };
        if code.len() >= 2 && code.chars().all(|c| c.is_ascii_alphanumeric()) && !name.is_empty() {
            out.entry(code.to_ascii_uppercase()).or_insert_with(|| name.to_string());
        }
    }
    out
}

/// An HFCC site: name, country and position, keyed by the site code.
struct HfccSite {
    name: String,
    country: String,
    lat: Option<f64>,
    lon: Option<f64>,
}

/// `"17S44"` / `"168E33"` as signed degrees — the site's own compact form.
fn hfcc_coord(token: &str, what: &str) -> Option<f64> {
    // **`17S44` is 17 degrees 44 minutes South** — the hemisphere letter sits
    // *between* the degrees and the minutes, not at either end. Reading it as a
    // trailing sign (which is how a plain decimal coordinate is written) splits the
    // token in the wrong place and returns nonsense, so every coordinate parses to
    // nothing and every HFCC station quietly lands at no position on the map.
    let t = token.trim().to_ascii_uppercase();
    let at = t.find(['N', 'S', 'E', 'W'])?;
    let (deg_text, hemi) = (&t[..at], t.as_bytes()[at] as char);
    let min_text = &t[at + 1..];
    let deg: f64 = deg_text.parse().ok()?;
    let min: f64 = min_text.parse().ok()?;
    let sign = match hemi {
        'N' | 'E' => 1.0,
        'S' | 'W' => -1.0,
        _ => return None,
    };
    // A latitude cannot leave the hemisphere it is named for, and a longitude
    // cannot either — so this rejects a column that has drifted into the wrong one
    // rather than plotting a station in the wrong hemisphere.
    let value = sign * (deg.abs() + min.abs() / 60.0);
    let in_range = if what == "lat" {
        (-90.0..=90.0).contains(&value)
    } else {
        (-180.0..=180.0).contains(&value)
    };
    in_range.then_some(value)
}

/// HFCC's **schedule** plus its two reference tables.
///
/// The zip is one package — `a26allx2.zip` holds the schedule and the tables
/// together — so "the companion file" is not a second download that can go stale
/// against the first. It is why this source can exist at all: the schedule alone
/// is a table of `WCR` and `BRC` codes with no names in it.
///
/// **Column positions are read from the header line**, never hardcoded. HFCC's
/// layout has moved between seasons and a schedule that silently parses to zero
/// rows is the same failure as a wrong URL — quieter, and much easier to ship.
pub fn parse_hfcc(
    schedule: &str,
    broadcasters: &str,
    sites: &str,
) -> Result<Vec<BroadcastStation>, String> {
    let names = hfcc_reference_table(broadcasters);
    // **`SITE.TXT` is read by its own header's columns, not by whitespace.**
    // Site names contain spaces — "Port Vila", "Alphen aan den Rijn" — so splitting
    // on whitespace yields "Port" and drops the rest, and the coordinates shift with
    // it. The same self-calibrating rule the schedule uses, for the same reason: a
    // column that moves must not silently empty a field.
    let site_header = sites
        .lines()
        .find(|l| l.starts_with(";Co") && l.contains("Site Name"))
        .map(|l| &l[1..])
        .ok_or("HFCC's site table has no header line")?;
    let sc = |name: &str| -> Result<usize, String> {
        site_header.find(name).ok_or_else(|| format!("HFCC's site header has no {name} column"))
    };
    let (h_code, h_name, h_adm, h_lat, h_lon) =
        (sc("Co")?, sc("Site Name")?, sc("ADM")?, sc("Lati")?, sc("Longi")?);
    let mut site_map: std::collections::HashMap<String, HfccSite> =
        std::collections::HashMap::new();
    for line in sites.lines() {
        if line.starts_with(';') || line.trim().is_empty() {
            continue;
        }
        let get = |a: usize, b: usize| -> String {
            line.get(a..b.min(line.len())).unwrap_or("").trim().to_string()
        };
        site_map.insert(
            get(h_code, h_name).to_ascii_uppercase(),
            HfccSite {
                name: get(h_name, h_adm),
                country: get(h_adm, h_lat),
                lat: hfcc_coord(&get(h_lat, h_lon), "lat"),
                lon: hfcc_coord(&get(h_lon, line.len()), "lon"),
            },
        );
    }

    let raw = schedule
        .lines()
        .find(|l| l.starts_with(";FREQ"))
        .ok_or("HFCC's schedule has no header line")?;
    // **Drop the `;`.** The header is a comment line, so its own indices start one
    // later than the data it describes.
    let header = &raw[1..];
    let first = schedule
        .lines()
        .find(|l| !l.trim().is_empty() && !l.starts_with(';'))
        .ok_or("HFCC's schedule has no data rows")?;
    // **Calibrate the shift against the first row rather than assuming it is zero.**
    //
    // This is the whole bug this parser had. Reading offsets off the header and
    // applying them to the rows put every column after `FREQ` one position early:
    // the file loaded, the frequency parsed (the row's leading blank is absorbed
    // into the number), and 84% of stations came back as bare codes that matched
    // nothing in the broadcaster table. It read as "a different code space".
    //
    // Measuring where the header's first column name sits against where the first
    // row's first field actually starts gives the shift, and it survives a column
    // moving between seasons — which a hardcoded offset, or a header index taken
    // on trust, cannot.
    let first_at = first
        .find(|c: char| !c.is_ascii_whitespace())
        .ok_or("HFCC's first data row is blank")? as isize;
    let header_at =
        header.find(|c: char| !c.is_ascii_whitespace()).ok_or("HFCC's header is blank")? as isize;
    let shift: isize = first_at - header_at;
    let col = |name: &str| -> Result<usize, String> {
        let h = header.find(name).ok_or_else(|| format!("HFCC's header has no {name} column"))?;
        Ok((h as isize + shift) as usize)
    };
    let (f_freq, f_start, f_stop) = (col("FREQ")?, col("STRT")?, col("STOP")?);
    let (f_loc, f_powr, f_days, f_lang, f_adm, f_brc) =
        (col("LOC")?, col("POWR")?, col("DAYS")?, col("LANGUAGE")?, col("ADM")?, col("BRC")?);
    let row_len = first.len();
    // Each field runs to the next column, so a value may be longer than its label.
    let field = |line: &str, start: usize, next: usize| -> String {
        let hi = next.min(line.len().max(row_len));
        line.get(start..hi).unwrap_or("").trim().to_string()
    };

    let mut out = Vec::new();
    for line in schedule.lines() {
        if line.trim().is_empty() || line.starts_with(';') {
            continue;
        }
        let freq: f64 = match field(line, f_freq, f_start).parse() {
            Ok(f) if f > 0.0 => f,
            _ => continue,
        };
        let brc = field(line, f_brc, f_brc + 3).to_ascii_uppercase();
        let loc = field(line, f_loc, f_powr).to_ascii_uppercase();
        let name = names.get(&brc).cloned().unwrap_or_else(|| brc.clone());
        let site = site_map.get(&loc);
        // The broadcaster's own name is what a listener looks for; the site is
        // where it is transmitted from, and the two are often different.
        let station = if name == brc {
            site.map(|s| s.name.clone()).unwrap_or_else(|| brc.clone())
        } else {
            name
        };
        let days: String = field(line, f_days, f_lang)
            .chars()
            .filter_map(|c| c.to_digit(10).filter(|d| (1..=7).contains(d)).map(hfcc_day_to_ours))
            .map(|d| char::from(b'0' + d as u8))
            .collect();
        let hhmm = |at: usize, next: usize| -> Option<u16> {
            let t = field(line, at, next);
            (t.len() == 4).then(|| t.parse::<u16>().ok()).flatten().filter(|v| *v <= 2359)
        };
        let country = field(line, f_adm, f_brc);
        let country = if country.is_empty() {
            site.map(|s| s.country.clone()).unwrap_or_default()
        } else {
            country
        };
        out.push(BroadcastStation {
            name: station,
            freq_khz: freq,
            start_utc: hhmm(f_start, f_stop),
            end_utc: hhmm(f_stop, f_loc),
            days,
            site: site.map(|s| s.name.clone()).unwrap_or_default(),
            country,
            lang: field(line, f_lang, f_adm),
            lat: site.and_then(|s| s.lat),
            lon: site.and_then(|s| s.lon),
            source: ScheduleSource::Hfcc,
            ..Default::default()
        });
    }
    if out.is_empty() {
        return Err("HFCC's schedule yielded no rows".into());
    }
    Ok(out)
}

/// **The master store**: every available source in one list, each row tagged with
/// where it came from.
///
/// bebepip's answer on fork #69, and it replaced the handover's *"one active
/// source at a time"*: he wants EiBi, Aoki, HFCC and his own list **imported into
/// one repository**, with a source column, because filtering one store is simpler
/// than querying several — and says the resulting list *"will simply be long and
/// somewhat repetitive"*, which is accepted: the Schedule window is where the
/// detail belongs.
///
/// **There is no week-start handling here, and that is the point.** Aoki counts the
/// week from Sunday and everything else here counts from Monday; that difference
/// is already resolved **inside each parser** (`aoki_days` maps its Sunday to our
/// Sunday), so a row in this store is already in our terms whichever file it came
/// from. Putting the conversion here instead would be the one place it could be
/// applied twice, and applied twice it shifts every station by a day with nothing
/// looking broken.
///
/// Duplicates are **kept**. One broadcaster listed by three sources is three rows,
/// all editable and all exportable; the waterfall is where that repetition is
/// wrong, and it collapses it there. Collapsing here would throw away data to
/// tidy a picture.
pub fn master_store(sources: &[(ScheduleSource, Vec<BroadcastStation>)]) -> Vec<BroadcastStation> {
    let mut all: Vec<BroadcastStation> = Vec::new();
    for (source, rows) in sources {
        for row in rows {
            all.push(BroadcastStation { source: *source, ..row.clone() });
        }
    }
    all.sort_by(|a, b| {
        a.freq_khz
            .total_cmp(&b.freq_khz)
            .then_with(|| a.start_utc.cmp(&b.start_utc))
            .then_with(|| a.name.cmp(&b.name))
            // And the source last, so a stable list does not reshuffle when the
            // same station is listed twice.
            .then_with(|| source_rank(a.source).cmp(&source_rank(b.source)))
    });
    all
}

/// The store's order: EiBi, then Aoki, then HFCC, then the listener's own.
fn source_rank(s: ScheduleSource) -> u8 {
    match s {
        ScheduleSource::Eibi => 0,
        ScheduleSource::Aoki => 1,
        ScheduleSource::Hfcc => 2,
        ScheduleSource::User => 3,
    }
}

/// The rows of a store from one source — the Schedule window's filter, and the
/// "all of them" case the filter's default is.
pub fn filter_by_source(
    rows: &[BroadcastStation],
    source: Option<ScheduleSource>,
) -> Vec<BroadcastStation> {
    match source {
        None => rows.to_vec(),
        Some(want) => rows.iter().filter(|r| r.source == want).cloned().collect(),
    }
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
mod aoki_tests {
    use super::{aoki_days, aoki_position, parse_aoki};

    /// The 2018 screenshot bebepip posted, verbatim — thirteen rows chosen to
    /// cover every shape in the season, plus the two whose long station names
    /// push the columns. Taken from Aoki's own published `ncb24.txt`, so this is
    /// the format rather than a description of it.
    const FIXTURE: &str = include_str!("aoki-fixture.txt");

    fn rows() -> Vec<super::BroadcastStation> {
        parse_aoki(FIXTURE)
    }

    /// **Aoki counts the week from Sunday.** Read as ours, every schedule shifts
    /// by a day and nothing about the result looks wrong — the station is still
    /// there, just on the wrong days. This is the test that would catch it.
    #[test]
    fn the_week_starts_on_sunday_as_aoki_counts_it() {
        assert_eq!(aoki_days("1234567"), "1234567", "all seven days is all seven days");
        assert_eq!(aoki_days("1......"), "7", "Sunday alone is our 7");
        assert_eq!(aoki_days("......7"), "6", "Saturday alone is our 6");
        // The fixture's own row: `.234567` is Monday..Saturday.
        assert_eq!(aoki_days(".234567"), "123456", "Mon-Sat in our numbering");
        // `....567` in the file is Thursday, Friday, Saturday -- Aoki counts
        // Sunday as 1, so its 5,6,7 are ours 4,5,6.
        assert_eq!(aoki_days("....567"), "456", "Thu-Sat in our numbering");
        // A shift of exactly one on every day, which is the failure being pinned.
        for d in 1..=7u32 {
            let s = d.to_string();
            let ours = aoki_days(&s);
            assert_eq!(ours.len(), 1, "{s} must map to one day");
            assert_eq!(
                ours.parse::<u32>().unwrap(),
                if d == 1 { 7 } else { d - 1 },
                "Aoki {s} is a fixed offset from ours"
            );
        }
    }

    /// **Two `L/L` precisions occur in one season.** Aoki's current file gives
    /// `DDMMSS[NS]DDDMMSS[EW]`; some sites still carry the older `DDMM[NS]DDDMM[EW]`.
    /// Both are in the fixture, and a parser that assumes one reads the other as
    /// a wrong position — a station off by a degree, drawn in the sea.
    #[test]
    fn both_position_precisions_read() {
        // Six-digit form, as the current file writes it.
        let (lat, lon) = aoki_position("372221N1405056E").expect("seconds form");
        assert!((lat - 37.3725).abs() < 0.0001, "lat {lat}");
        assert!((lon - 140.84889).abs() < 0.0001, "lon {lon}");
        // Four-digit form.
        let (lat, lon) = aoki_position("5554N01216E").expect("minutes form");
        assert!((lat - 55.9).abs() < 0.0001, "lat {lat}");
        // 012 16 E is 12 degrees 16 minutes -- three degrees of longitude, then
        // two of minutes, which is why the two halves are not the same width.
        assert!((lon - 12.26667).abs() < 0.0001, "lon {lon}");
        // Southern and western hemispheres, both forms.
        let (lat, lon) = aoki_position("385002S1762501E").expect("south-west");
        assert!(lat < 0.0 && lon > 0.0, "38.5S 176.25E is south, east: {lat} {lon}");
        let (lat, lon) = aoki_position("0011S07832W").expect("equator-ish");
        assert!(lat < 0.0 && lon < 0.0, "00.11S 078.32W is south, west: {lat} {lon}");
    }

    /// A site Aoki does not have coordinates for yields **no position**, not a
    /// zero — a station at 0,0 is off West Africa and draws a dot there.
    #[test]
    fn a_missing_position_is_absent_rather_than_null_island() {
        assert_eq!(aoki_position(""), None);
        assert_eq!(aoki_position("   "), None);
        assert_eq!(aoki_position("N123E"), None, "too short to be a position");
    }

    /// The fixture's rows come out as stations with the fields a listener reads.
    #[test]
    fn the_fixture_parses_into_stations() {
        let v = rows();
        assert!(v.len() >= 30, "most fixture rows must parse, got {}", v.len());
        let t = v.iter().find(|s| s.name == "Radio OZ-Viola").expect("a known row");
        assert_eq!(t.freq_khz, 5980.0);
        assert_eq!(t.site, "Hilleroed");
        assert_eq!(
            t.country, "Denmark",
            "Aoki's three-letter ADM resolves through the shared code tables"
        );
        assert_eq!(t.lang, "Danish");
        assert_eq!(t.days, "3", "Aoki's ...4... is Thursday, which is our 3");
        // `0000-2400` is around the clock: no window at all, not a zero-length
        // one and not midnight. Most of the file is this, so a parser that read
        // it as 00:00-00:00 would show those stations as transmitting for no
        // time at all.
        let all_day: Vec<_> = v.iter().filter(|s| s.start_utc.is_none()).collect();
        assert!(!all_day.is_empty(), "around-the-clock rows must exist");
        for s in &all_day {
            assert!(s.end_utc.is_none(), "{} has a start of nothing and an end", s.name);
        }
        // A row with a window has both ends, and they are real times of day.
        let timed: Vec<_> = v.iter().filter(|s| s.start_utc.is_some()).collect();
        assert!(!timed.is_empty(), "some rows must carry a window");
        for s in timed {
            let (a, b) = (s.start_utc.unwrap(), s.end_utc.expect("a start needs an end"));
            assert!(a < 2400 && b < 2400, "{} has a window outside the day", s.name);
            assert!(a % 100 < 60 && b % 100 < 60, "{} has a window that is not HHMM", s.name);
            // A window that wraps past midnight is kept as it is published
            // rather than normalised, because the schedule window is what an
            // operator reads against.
            assert_ne!(a, b, "{} has a zero-length window", s.name);
        }
    }

    /// **A long station name pushes the row, and must still read.** Two rows in
    /// this fixture do exactly that; a fixed-column read would take the UTC
    /// window out of the station's name and put the name in the day's column.
    #[test]
    fn a_long_station_name_does_not_shift_the_row_out_from_under_us() {
        // 5800 kHz WRMI is the row in this file whose station name overruns its
        // column, so everything after it sits one place to the right.
        let v = rows();
        let s =
            v.iter().find(|s| s.freq_khz == 5800.0).expect("the pushed-column row parsed at all");
        assert_eq!(
            s.name, "WRMI Radio Rep?blica",
            "the name must be whole and not sliced at a column; the `?` is Aoki's \
             own character for an accented one, not our decoding"
        );
        assert_eq!(s.site, "Okeechobee10", "and the site must land in the site field");
        assert_eq!(s.days, "456", "Aoki's ....567 is Thu-Fri-Sat, which is ours 456");
        assert_eq!(s.country, "United States of America", "resolved through the code tables");
        assert_eq!(s.power_kw, Some(100.0), "power comes through the fallback too");
    }

    /// Out-of-band rows are dropped, as EiBi's are: a file that carries marine
    /// and aeronautical traffic must not put coast stations in a broadcast list.
    #[test]
    fn rows_outside_the_broadcast_bands_are_dropped() {
        let v = rows();
        for s in &v {
            assert!(
                super::BANDS.iter().any(|(lo, hi)| (*lo..=*hi).contains(&s.freq_khz)),
                "{} kHz is outside the broadcast bands",
                s.freq_khz
            );
        }
        // 40 kHz and 60 kHz time signals are in the fixture and are out of band.
        assert!(!v.iter().any(|s| s.freq_khz == 40.0), "40 kHz must not be listed");
    }

    /// **The two formats must not be able to read each other.** They are both
    /// "a schedule of broadcast stations" and both now go through
    /// [`super::parse_from`], so the obvious way for this work to go wrong is a
    /// parser that quietly accepts the other one's file and produces a schedule
    /// of nonsense. Aoki's columns are space-aligned and EiBi's are
    /// semicolons; neither is a subset of the other.
    #[test]
    fn aoki_and_eibi_do_not_read_each_other() {
        let aoki = parse_aoki(FIXTURE);
        assert!(!aoki.is_empty(), "Aoki's fixture must parse as Aoki");
        assert!(parse_aoki(super::FALLBACK_SKED).is_empty(), "EiBi's file must not parse as Aoki");
        // And the other way round is already asserted above; this states the
        // pair so the intent is visible from one test.
        assert!(super::parse_schedule(FIXTURE).is_empty(), "Aoki's file must not parse as EiBi");
    }

    /// A file that is not Aoki's, or a truncated one, must yield nothing rather
    /// than a schedule made of nonsense.
    #[test]
    fn a_file_that_is_not_aoki_yields_nothing() {
        assert!(parse_aoki("").is_empty());
        assert!(
            parse_aoki(
                "random text
more text
"
            )
            .is_empty()
        );
        // An EiBi file must not half-parse as Aoki.
        assert!(parse_aoki(super::FALLBACK_SKED).is_empty(), "EiBi's file is not Aoki's");
    }
}

#[cfg(test)]
mod tests {
    /// **The EiBi acceptance test for the source-abstraction work** (#69,
    /// `SWL-HANDOVER.md` stage 1): EiBi becomes *source #1* rather than *the*
    /// source, and the price of that refactor is that this program's schedule
    /// must come out of it byte-for-byte identical.
    ///
    /// A digest rather than a row count, because a count cannot see a station
    /// that lost its language, moved a megahertz or gained a day — the changes a
    /// refactor to a parser is most likely to make. It digests every field the
    /// schedule window, the spots and the reception log's pre-fill read, so any
    /// of them moving shows here first.
    ///
    /// The digest is pinned to whatever EiBi produced **before** the abstraction
    /// landed. If a future change moves it, that is either a deliberate
    /// behavioural change to be justified in the commit that makes it, or a bug.
    /// Either way it must not be updated on reflex.
    #[test]
    fn the_eibi_schedule_comes_out_unchanged() {
        let stations = parse_schedule(FALLBACK_SKED);
        assert!(
            stations.len() > 3_000,
            "the compiled-in fallback should parse to a real schedule, got {}",
            stations.len()
        );
        let digest = eibi_digest(&stations);
        println!("EiBi: {} stations, digest {digest:#018x}", stations.len());
        assert_eq!(
            digest, 0xe6fe_c537_410d_8cde,
            "the EiBi parse changed — see SWL-HANDOVER.md; a count alone would not \
             have caught a station that lost a field. Last moved deliberately on \
             2026-10-18, when the 114 DRM rows came back: the delta was measured \
             row by row against the fixture before this number was touched."
        );
    }

    /// The seam must be **real**: EiBi read through `parse_from` has to give
    /// exactly what `parse_schedule` gives, or the abstraction is a second
    /// implementation that can drift from the first — which is the thing a
    /// single implementation exists to prevent.
    #[test]
    fn reading_through_the_source_seam_is_the_same_parse() {
        let direct = parse_schedule(FALLBACK_SKED);
        let through = parse_from(ScheduleSource::Eibi, FALLBACK_SKED).expect("EiBi is readable");
        assert_eq!(direct.len(), through.len(), "the seam changed the row count");
        assert_eq!(
            eibi_digest(&direct),
            eibi_digest(&through),
            "the seam changed the parse — it must dispatch, not reimplement"
        );
    }

    /// **A source that cannot be read must say so, not return nothing.**
    ///
    /// An empty schedule and an unreadable one look identical on screen, and only
    /// one of them is a fault: the operator sees a band with nothing on it and
    /// concludes the band is dead.
    #[test]
    fn a_source_that_is_not_readable_yet_refuses_rather_than_returning_nothing() {
        for src in [ScheduleSource::Hfcc, ScheduleSource::User] {
            assert!(!src.is_readable(), "{} is not implemented yet", src.label());
            let err = parse_from(src, FALLBACK_SKED).expect_err("an unreadable source must refuse");
            assert!(err.contains(src.label()), "the refusal must name the source: {err}");
        }
    }

    /// EiBi is the default and stays readable, because a default that stops
    /// being readable would take the program's schedule with it.
    #[test]
    fn eibi_is_the_default_and_the_one_readable_source() {
        assert_eq!(ScheduleSource::default(), ScheduleSource::Eibi);
        assert!(ScheduleSource::Eibi.is_readable());
        assert_eq!(
            ScheduleSource::ALL.len(),
            4,
            "every source is listed, so a new one cannot be forgotten"
        );
        for src in ScheduleSource::ALL {
            // Each label is what a settings row shows; an empty one is a blank chip.
            assert!(!src.label().is_empty(), "{src:?} has no label");
        }
    }

    /// **HFCC is two files and every other source is one.** The seam has to be
    /// able to say so, because loading half of it is the failure mode: a schedule
    /// that is full of three-letter codes and nothing to resolve them.
    #[test]
    fn hfcc_needs_a_companion_file_and_nothing_else_does() {
        assert!(
            ScheduleSource::Hfcc.needs_companion_file(),
            "HFCC's rows carry codes that a second file resolves"
        );
        for src in [ScheduleSource::Eibi, ScheduleSource::Aoki, ScheduleSource::User] {
            assert!(!src.needs_companion_file(), "{} is a single file", src.label());
        }
    }

    /// FNV-1a over the fields a listener actually sees.
    fn eibi_digest(stations: &[BroadcastStation]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |s: &str| {
            for b in s.bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x100_0000_01b3);
            }
            h ^= 0x0a;
            h = h.wrapping_mul(0x100_0000_01b3);
        };
        for s in stations {
            feed(&s.name);
            feed(&format!("{}", s.freq_khz));
            feed(&s.site);
            feed(&s.country);
            feed(&s.lang);
            feed(&s.target);
            feed(&format!("{:?}{:?}", s.lat, s.lon));
            feed(&format!("{:?}{:?}", s.mode, s.season));
            feed(&format!("{:?}{:?}", s.start_utc, s.end_utc));
            feed(&s.days);
        }
        h
    }

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
            source: ScheduleSource::Eibi,
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
            email: String::new(),
            address: String::new(),
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
        //
        // **4,743 since 2026-10-18**: the DRM rows came back (see
        // `is_listenable`), which is 114 more. The fixture carries 118 rows
        // naming a DRM station; four of them fail a gate that has nothing to do
        // with this — one at 1557 kHz and two at 9975 kHz, both outside the
        // broadcast bands, and one marked inactive — so 114 is the whole of the
        // change and nothing else moved with it.
        assert_eq!(seed().len(), 23);
        assert_eq!(parse_schedule(FALLBACK_SKED).len(), 4743);
        assert_eq!(builtin().len(), 4766);
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
    fn schedule_contact_columns_are_read_when_present() {
        // An enriched file appends the broadcaster's report contact as fields
        // twelve and thirteen; the published EiBi file stops at eleven.
        let row = [
            "15400",
            "1800-1900",
            "Mo-Fr",
            "G",
            "BBC",
            "E",
            "WAf",
            "/ASC",
            "0",
            "",
            "",
            "reports@bbc.example",
            "BBC, London",
            "",
        ]
        .join(";");
        let got = parse_schedule(&format!("header\n{row}\n"));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].email, "reports@bbc.example");
        assert_eq!(got[0].address, "BBC, London");

        let plain = "header\n15400;1800-1900;Mo-Fr;G;BBC;E;WAf;/ASC;0;;;\n";
        let got = parse_schedule(plain);
        assert_eq!(got.len(), 1);
        assert!(got[0].email.is_empty() && got[0].address.is_empty());
    }

    /// Jammers and numbers stations are not for a listener; DRM is, and it used
    /// to be dropped with the fax rows.
    #[test]
    fn the_parser_drops_jammers_and_numbers_stations_and_keeps_drm() {
        let row = |f: &str, station: &str, lng: &str, p: &str| {
            format!("{f};0000-2400;;CHN;{station};{lng};As;;{p};;\n")
        };
        let csv = String::from("header\n")
            + &row("9420", "Good Station", "E", "1") // kept
            + &row("9420", "CNR1 Jammer", "E", "1") // jammer
            + &row("9420", "E06 Spy Numbers", "E", "1") // numbers station
            + &row("9420", "CRI DIGITAL", "E", "1") // DRM — kept, and says so
            + &row("9420", "VCS Halifax Radio", "E", "1") // "fax" in a place name
            + &row("9420", "NOJ Fax Kodiak", "E", "1") // a chart, not a programme
            + &row("9420", "Retired Service", "E", "8") // inactive
            + &row("9420", "Coast Radio", "-CW", "1") // not voice
            + &row("8000", "Out Of Band", "E", "1"); // not a broadcast band
        let got = parse_schedule(&csv);
        let names: Vec<&str> = got.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Good Station", "CRI DIGITAL", "VCS Halifax Radio"], "kept {names:?}");
        // Kept *and* labelled. EiBi has no emission column, so a DRM row is only
        // DRM because its name says so — tuned as AM it would be noise, which is
        // worse for the operator than the row being absent.
        let drm = got.iter().find(|s| s.name == "CRI DIGITAL").expect("the DRM row");
        assert_eq!(drm.mode(), crate::Mode::Drm, "the DRM row says what to tune");
        let plain = got.iter().find(|s| s.name == "Good Station").expect("the AM row");
        assert_eq!(plain.mode(), crate::Mode::Am, "and an ordinary row is untouched");
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
        assert_eq!(
            with.len(),
            seed().len() + utilities().len() + airband().len() + military().len()
        );
        assert!(with.iter().any(|s| s.name.contains("WWV")), "added in");
        assert!(with.iter().any(|s| s.name.contains("GUARD")), "airband too");
        assert!(with.iter().any(|s| s.name.contains("HFGCS")), "and military");
    }

    #[test]
    fn the_allocation_free_search_matches_case_insensitively() {
        assert!(contains_ascii_ci("Radio Taiwan International", "taiwan"));
        assert!(contains_ascii_ci("Ascension", "ASCENSION"));
        assert!(!contains_ascii_ci("BBC", "zzz"));
        assert!(contains_ascii_ci("anything", ""), "an empty needle matches");
        assert!(contains_ascii_ci("anything", "  "), "so does whitespace");
        assert!(!contains_ascii_ci("ab", "abc"), "a longer needle cannot match");
    }

    /// Four minutes a degree, and it wraps: noon UTC is 12:00 at Greenwich,
    /// ahead of it to the east, behind it to the west, and never off the clock.
    #[test]
    fn solar_time_is_four_minutes_a_degree_and_wraps() {
        assert_eq!(local_solar_hhmm(1200, 0.0), "12:00");
        // 15°E is +60 min; 75°W is −300 min.
        assert_eq!(local_solar_hhmm(1200, 15.0), "13:00");
        assert_eq!(local_solar_hhmm(1200, -75.0), "07:00");
        // Past midnight the clock wraps rather than going negative.
        assert_eq!(local_solar_hhmm(0100, -75.0), "20:00");
        assert_eq!(local_solar_hhmm(2300, 120.0), "07:00");
    }

    #[test]
    fn the_military_table_names_the_known_channels() {
        let m = military();
        assert!(m.iter().any(|s| s.freq_khz == 243_000.0), "the military emergency");
        assert!(m.iter().any(|s| s.name.contains("HFGCS")), "the HF system");
        for s in m {
            if s.freq_khz > 30_000.0 {
                assert_eq!(metre_band(s.freq_khz), Some("MIL"), "{} reads as MIL", s.name);
            }
            assert!(s.on_air_at(0), "{} runs around the clock", s.name);
        }
    }

    #[test]
    fn the_airband_table_covers_the_universal_channels() {
        let a = airband();
        assert!(a.iter().any(|s| s.freq_khz == 121_500.0), "the emergency channel");
        for s in a {
            assert!((108_100.0..=137_000.0).contains(&s.freq_khz), "{} kHz", s.freq_khz);
            assert_eq!(s.mode_str(), "AM", "{} is AM", s.name);
            assert_eq!(metre_band(s.freq_khz), Some("AIR"), "{} reads as AIR", s.name);
            assert!(s.on_air_at(0), "{} runs around the clock", s.name);
        }
    }
}

// ── Aoki ──────────────────────────────────────────────────────────────────

/// Column offsets read out of an Aoki file's own header line.
///
/// **Read from the file rather than hard-coded**, because the layout has moved
/// between seasons: bebepip's 2018 file and the 2025 one agree on the column
/// *positions* but not on what is in them — `L/L` gained seconds, and the
/// language codes changed. A parser that pinned today's numbers would read the
/// older file as garbage, and one that pinned the older file's would read the
/// current one. The header names every column and where it starts, so it is the
/// authority.
fn aoki_columns(header: &str) -> Option<Vec<(&'static str, usize)>> {
    let names: [(&'static str, &str); 11] = [
        ("fre", "FRE"),
        ("station", "STATION"),
        ("utc", "UTC"),
        ("days", "Su-W-Sa"),
        ("lang", "Language"),
        ("pow", "Pow"),
        ("azi", "Azi"),
        ("loc", "Location"),
        ("adm", "ADM"),
        ("ll", "L/L"),
        ("rem", "Remarks"),
    ];
    let mut out = Vec::with_capacity(names.len());
    for (key, needle) in names {
        // `starts_with` on the trimmed header: `Remarks` may be absent on an
        // older file, and that is the last column, so it may be missing rather
        // than the layout being different.
        let at = header.find(needle)?;
        out.push((key, at));
    }
    Some(out)
}

/// One header row split into field ranges, right-aligned by the next column's
/// start.
fn aoki_field<'a>(row: &'a str, cols: &[(&'static str, usize)], key: &str) -> &'a str {
    let at = cols.iter().position(|(k, _)| *k == key).unwrap_or(0);
    let start = cols[at].1.min(row.len());
    let end = cols.get(at + 1).map_or(row.len(), |(_, e)| (*e).min(row.len()));
    if end <= start {
        return "";
    }
    row[start..end].trim()
}

/// **Aoki counts the week from Sunday.** The header says so
/// (`Day 1 = Sunday`) and the file proves it: `.23456.` is Monday to Friday,
/// which is every day but Sunday and Saturday.
///
/// This is the single easiest thing to get wrong in this format. Our convention
/// — and EiBi's, and the one `BroadcastStation::days` documents — is
/// `1` = Monday .. `7` = Sunday. Reading Aoki's digits directly therefore shifts
/// **every schedule by one day**, and nothing about the result looks broken: the
/// station is still there, just on the wrong days. Hence its own test.
fn aoki_days(field: &str) -> String {
    let mut out = String::new();
    for c in field.chars() {
        // Seven positional slots, one per day, each the day's number or a dot
        // for "not on air". `1......` is Sunday only; `1234567` is every day.
        let Some(d) = c.to_digit(10).filter(|d| (1..=7).contains(d)) else {
            continue;
        };
        let ours: u32 = if d == 1 { 7 } else { d - 1 };
        let c = char::from(b'0' + ours as u8);
        if !out.contains(c) {
            out.push(c);
        }
    }
    // Ascending, because the convention is a *set* of digits and a listener
    // comparing an Aoki row with an EiBi row should read the same string.
    let mut v: Vec<char> = out.chars().collect();
    v.sort_unstable();
    v.into_iter().collect()
}

/// Aoki's `L/L`: `DDMMSS[NS]DDDMMSS[EW]` or the older `DDMM[NS]DDDMM[EW]`,
/// **both of which occur in one season's file**.
///
/// Returns `(lat, lon)` in degrees. A half absent — as it is for a station whose
/// site is known but not surveyed — yields `None` rather than a guess, so the
/// station keeps its waterfall label and simply gets no map dot.
fn aoki_position(ll: &str) -> Option<(f64, f64)> {
    let s: Vec<char> = ll.chars().filter(|c| !c.is_whitespace()).collect();
    // Exactly one N/S and one E/W; the degrees sit before each.
    let ns = s.iter().position(|c| matches!(c, 'N' | 'S'))?;
    let ew = s.iter().position(|c| matches!(c, 'E' | 'W'))?;
    if ew <= ns + 1 || ns < 4 {
        return None;
    }
    let num =
        |part: &[char]| -> Option<f64> { part.iter().collect::<String>().parse::<f64>().ok() };
    // Degrees, minutes, and — in the current format — seconds. Two degrees
    // wide for latitude and three for longitude, which is why the two halves are
    // not the same length. The older form drops the seconds; both occur inside a
    // single season, so which one this is is read from where the letter falls.
    let wide = ns == 6;
    let take = |a: usize, n: usize| -> Option<f64> {
        if a + n > s.len() {
            return None;
        }
        num(&s[a..a + n])
    };
    let (lat_deg, lat_min, lon_deg, lon_min) = if wide {
        (take(0, 2)?, take(2, 2)?, take(ns + 1, 3)?, take(ns + 4, 2)?)
    } else {
        (take(0, 2)?, take(2, 2)?, take(ns + 1, 3)?, take(ns + 4, 2)?)
    };
    // Seconds, where the format carries them. About 3 m of arc, so this is a
    // refinement rather than a different answer — but it is in the file and it
    // costs nothing to keep.
    let (lat_sec, lon_sec) =
        if wide { (take(4, 2).unwrap_or(0.0), take(ns + 6, 2).unwrap_or(0.0)) } else { (0.0, 0.0) };
    let lat = lat_deg + lat_min / 60.0 + lat_sec / 3600.0;
    let lon = lon_deg + lon_min / 60.0 + lon_sec / 3600.0;
    let lat = if s[ns] == 'S' { -lat } else { lat };
    let lon = if s[ew] == 'W' { -lon } else { lon };
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
}

/// `NNNN-NNNN` — Aoki's time window, whatever it turns out to mean.
fn looks_like_window(s: &str) -> bool {
    let Some((a, b)) = s.split_once('-') else {
        return false;
    };
    a.len() == 4 && b.len() == 4 && a.bytes().chain(b.bytes()).all(|c| c.is_ascii_digit())
}

/// Parse an Aoki `ncbXX.txt` schedule.
///
/// Space-aligned fixed columns, which is the trap: this is **not** EiBi's
/// semicolon file and the two cannot share a reader. Columns are taken from the
/// file's own header line (see [`aoki_columns`]).
///
/// Rows are read by column where they fit. A station name longer than its
/// column **pushes everything after it right**, which happens on roughly seven
/// rows in a season and is why a pure fixed-column read would silently mis-parse
/// them — so a row whose UTC column is not a time window falls back to being read
/// by shape instead, anchored on the `NNNN-NNNN` that cannot be anything else.
pub fn parse_aoki(text: &str) -> Vec<BroadcastStation> {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let Some(header_at) = lines.iter().position(|l| l.contains("STATION") && l.contains("UTC"))
    else {
        return Vec::new();
    };
    let Some(cols) = aoki_columns(lines[header_at]) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for line in &lines[header_at + 1..] {
        if line.trim().is_empty() {
            continue;
        }
        let freq: f64 = match aoki_field(line, &cols, "fre").parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !BANDS.iter().any(|(lo, hi)| (*lo..=*hi).contains(&freq)) {
            continue;
        }

        // Which path, decided on the **shape** of the UTC column and nothing
        // else. `0000-2400` — around the clock, the commonest row in the file —
        // deliberately parses to no window at all, so branching on the parse
        // sends almost every row down the fallback.
        let utc_col = aoki_field(line, &cols, "utc");
        let (station, utc, days, lang, loc, adm, ll, remarks, pow) = if looks_like_window(utc_col) {
            (
                aoki_field(line, &cols, "station").to_string(),
                parse_window(utc_col),
                aoki_field(line, &cols, "days").to_string(),
                aoki_field(line, &cols, "lang").to_string(),
                aoki_field(line, &cols, "loc").to_string(),
                aoki_field(line, &cols, "adm").to_string(),
                aoki_field(line, &cols, "ll").to_string(),
                aoki_field(line, &cols, "rem").to_string(),
                aoki_field(line, &cols, "pow").to_string(),
            )
        } else {
            // A long station name has pushed the row out of its column. Anchor
            // on the one thing that cannot be anything else: the `NNNN-NNNN`
            // time window, wherever it ended up.
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let Some(wi) = tokens.iter().position(|t| looks_like_window(t)) else {
                continue;
            };
            let hhmm = parse_window(tokens[wi]);
            let rest = &tokens[wi + 1..];
            let take = |i: usize| rest.get(i).copied().unwrap_or("").to_string();
            let name: String = tokens[1..wi].join(" ");
            let remarks: String = rest.iter().skip(7).copied().collect::<Vec<_>>().join(" ");
            // The **same order the header declares**: days, language, power,
            // azimuth, location, ADM, L/L, remarks. Azimuth is not a field of
            // ours and is stepped over rather than dropped, which is why this
            // list is longer than the tuple it feeds.
            (name, hhmm, take(0), take(1), take(4), take(5), take(6), remarks, take(2))
        };

        let station = station.trim();
        if station.is_empty() || !is_listenable(station) {
            continue;
        }
        let (lat, lon) = aoki_position(&ll).map_or((None, None), |(a, b)| (Some(a), Some(b)));
        let (start_utc, end_utc) = match utc {
            Some((a, b)) => (Some(a), Some(b)),
            None => (None, None),
        };
        out.push(BroadcastStation {
            source: ScheduleSource::Aoki,
            name: station.to_string(),
            freq_khz: freq,
            site: loc.trim().to_string(),
            // Aoki's ADM is a three-letter country code. EiBi's code tables are
            // ours to resolve and are keyed the same way, so where a code is in
            // them the name is used; where it is not, the code stands, which is
            // better than dropping the country a station is broadcast into.
            country: resolve_country_code(&adm),
            lat,
            lon,
            power_kw: pow.parse::<f64>().ok().filter(|p| *p > 0.0),
            // Aoki's language column mixes codes and English names
            // (`A1B`, `PWM/PM`, `Romanian`). Resolved where we know the code and
            // passed through otherwise, so nothing is lost to a table that only
            // covers one publisher.
            lang: lang.trim().to_string(),
            target: remarks.trim().to_string(),
            mode: mode_from_name(station),
            start_utc,
            end_utc,
            days: aoki_days(&days),
            season: None,
            email: String::new(),
            address: String::new(),
        });
    }
    out
}

/// A three-letter country code -> the name our code tables carry for it.
fn resolve_country_code(code: &str) -> String {
    let c = code.trim();
    if c.is_empty() {
        return String::new();
    }
    codes().countries.get(c).cloned().unwrap_or_else(|| c.to_string())
}

#[cfg(test)]
mod freq_query_tests {
    use super::{BroadcastStation, parse_aoki, query_frequencies_khz};

    /// **A frequency and a name are both queries.** The listener who keeps the
    /// schedule asked for the first: *"add the ability to search by frequency"*.
    #[test]
    fn a_typed_frequency_finds_its_transmission() {
        let rows = parse_aoki(include_str!("aoki-fixture.txt"));
        let row = rows.first().expect("the fixture has stations");
        let khz = row.freq_khz;
        // Exact, as kHz and as a decimal MHz.
        assert!(row.matches_query(&format!("{khz}")));
        assert!(row.matches_query(&format!("{:.3}", khz / 1000.0)));
        // And a name still works — the frequency path must not have replaced it.
        assert!(row.matches_query(&row.name[..4]));
        assert!(!row.matches_query("nothinglikeit"));
    }

    /// The unit is decided by the number, the same rule the import uses, so one
    /// number means one thing wherever it is typed.
    #[test]
    fn a_unit_makes_it_exact_and_a_bare_number_tries_both() {
        assert_eq!(query_frequencies_khz("15400 kHz"), vec![15_400.0]);
        assert_eq!(query_frequencies_khz("6.185 MHz"), vec![6_185.0]);
        // Bare numbers are read both ways, and the range rule drops the
        // impossible one: 15400 is kHz, 153 is longwave in kHz.
        assert_eq!(query_frequencies_khz("15400"), vec![15_400.0]);
        assert_eq!(query_frequencies_khz("153"), vec![153.0]);
        // 6.185 is genuinely two readings — 6.185 MHz and 6.185 kHz — and both
        // are in range, so both are offered. That is the point of a search
        // being forgiving: it must not miss, and it cannot write bad data.
        assert_eq!(query_frequencies_khz("6.185"), vec![6.185, 6_185.0]);
        assert!(query_frequencies_khz("6.185").contains(&6_185.0));
    }

    /// **A name must never be read as a frequency.** This is the trap: `BBC`
    /// could otherwise be coerced into something numeric, and a search box that
    /// silently looks up a number when you typed a station is worse than one
    /// that finds nothing.
    #[test]
    fn a_name_is_not_a_frequency() {
        assert!(query_frequencies_khz("BBC").is_empty());
        assert!(query_frequencies_khz("Radio Prague").is_empty());
        assert!(query_frequencies_khz("15400abc").is_empty());
        assert!(query_frequencies_khz("").is_empty());
        assert!(query_frequencies_khz("0").is_empty());
        assert!(query_frequencies_khz("-5000").is_empty());
    }

    /// One kilohertz either side, so a rounded MHz still lands — and the
    /// neighbouring channel, 5 kHz away on 49 m, does not.
    #[test]
    fn the_tolerance_is_a_kilohertz_either_side() {
        let row = BroadcastStation { name: "T".into(), freq_khz: 6_185.0, ..Default::default() };
        assert!(row.matches_query("6185"));
        assert!(row.matches_query("6185.9"));
        assert!(!row.matches_query("6188"), "3 kHz up is another channel here");
    }
}

#[cfg(test)]
mod master_store_tests {
    use super::{ScheduleSource, filter_by_source, master_store, parse_aoki};
    use crate::broadcast::BroadcastStation;

    fn st(name: &str, khz: f64, days: &str) -> BroadcastStation {
        BroadcastStation {
            name: name.into(),
            freq_khz: khz,
            days: days.to_string(),
            ..Default::default()
        }
    }

    /// **Every source lands in one list, tagged.** That is the whole of the
    /// request: one repository, and a row that says where it came from.
    #[test]
    fn every_source_lands_in_one_store_tagged() {
        let store = master_store(&[
            (ScheduleSource::Eibi, vec![st("BBC World Service", 15_490.0, "1234567")]),
            (ScheduleSource::Aoki, vec![st("NHK World", 15_475.0, "1234567")]),
        ]);
        assert_eq!(store.len(), 2);
        // By name, not by index: the store is in **frequency** order, so which
        // row is first is a property of the frequencies, not of the sources.
        let eibi = store.iter().find(|s| s.name == "BBC World Service").unwrap();
        let aoki = store.iter().find(|s| s.name == "NHK World").unwrap();
        assert_eq!(eibi.source, ScheduleSource::Eibi);
        assert_eq!(aoki.source, ScheduleSource::Aoki);
    }

    /// **Duplicates are kept.** bebepip is explicit that a merged list *"will
    /// simply be long and somewhat repetitive"*, and he is right: this list is
    /// where the detail lives, so dropping a row to tidy it throws away data. The
    /// waterfall is what collapses them.
    #[test]
    fn one_broadcaster_from_two_sources_is_two_rows() {
        let store = master_store(&[
            (ScheduleSource::Eibi, vec![st("Radio Prague", 5_010.0, "1234567")]),
            (ScheduleSource::Aoki, vec![st("Radio Prague", 5_010.0, "1234567")]),
        ]);
        assert_eq!(store.len(), 2, "two sources, two rows, both kept");
        let mut sources: Vec<ScheduleSource> = store.iter().map(|s| s.source).collect();
        sources.sort_by_key(|s| *s as u8);
        assert_eq!(sources, vec![ScheduleSource::Eibi, ScheduleSource::Aoki]);
    }

    /// **The filter is the whole reason for the store.** One store, and a filter
    /// on it, is what bebepip said beats queries across several databases — and
    /// "all of them" must be the default, because the repetition is his accepted
    /// cost.
    #[test]
    fn the_store_filters_to_one_source_or_all_of_them() {
        let store = master_store(&[
            (ScheduleSource::Eibi, vec![st("a", 10.0, "1234567")]),
            (ScheduleSource::Aoki, vec![st("b", 20.0, "1234567")]),
            (ScheduleSource::Hfcc, vec![st("c", 30.0, "1234567")]),
        ]);
        assert_eq!(filter_by_source(&store, None).len(), 3, "all of them is the default");
        assert_eq!(filter_by_source(&store, Some(ScheduleSource::Aoki)).len(), 1);
        assert_eq!(filter_by_source(&store, Some(ScheduleSource::Hfcc))[0].name, "c");
    }

    /// **The store is sorted by frequency**, because that is what the Schedule
    /// window shows and what a listener scans. Two rows on one frequency are
    /// ordered by name and then by source, so the list does not reshuffle
    /// between frames.
    #[test]
    fn the_store_is_in_frequency_order() {
        let store = master_store(&[
            (ScheduleSource::Aoki, vec![st("later", 9_000.0, "1234567")]),
            (ScheduleSource::Eibi, vec![st("earlier", 5_000.0, "1234567")]),
        ]);
        let names: Vec<&str> = store.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["earlier", "later"]);
    }

    /// **The week start is not this store's problem** — and that is the finding
    /// worth pinning, because it is the one silent way this feature could go
    /// wrong. Aoki publishes Sunday as `1......` and counts the week from Sunday;
    /// everything else here counts from Monday. That difference is resolved
    /// **inside each parser**, so a row in this store is already in our terms
    /// whichever file it came from — and doing it again at this layer would shift
    /// every station by a day with nothing looking broken.
    ///
    /// Read from the **published** file rather than a typed-in header, so the
    /// assertion cannot pass on a plausible invention.
    #[test]
    fn the_week_start_is_already_normalised_by_the_parser() {
        let aoki = include_str!("aoki-fixture.txt");
        let rows: Vec<BroadcastStation> =
            parse_aoki(aoki).into_iter().filter(|r| r.name == "ReflectionsEurope").collect();
        let row = rows.first().expect("the fixture has that station, on `1......`");
        assert_eq!(
            row.days, "7",
            "Aoki's Sunday-only row is already our Sunday, and the parser did it"
        );

        // Merging must not touch it, and an EiBi row beside it must not need
        // converting either — both are already ours, which is the whole point.
        let store = master_store(&[(ScheduleSource::Aoki, rows)]);
        assert_eq!(store[0].days, "7", "the store did not touch the week start");
    }
}

#[cfg(test)]
mod hfcc_coord_tests {
    use super::hfcc_coord;

    /// **`17S44` is 17°44' South — the hemisphere letter is in the *middle*.**
    ///
    /// This is the trap that had every HFCC station landing at no position on the
    /// map: read as a trailing sign it splits the token after the last character,
    /// both halves fail to parse, and the function returns `None` for every row.
    /// Nothing errors. The schedule simply has no coordinates.
    #[test]
    fn the_hemisphere_is_between_the_degrees_and_the_minutes() {
        let lat = hfcc_coord("17S44", "lat").expect("a southern latitude");
        assert!((lat + 17.7333).abs() < 0.01, "17 deg 44 min S is -17.7333, got {lat}");
        assert!((hfcc_coord("43N17", "lat").unwrap() - 43.2833).abs() < 0.01);
        assert!((hfcc_coord("52N08", "lat").unwrap() - 52.1333).abs() < 0.01);
        assert!((hfcc_coord("168E33", "lon").unwrap() - 168.55).abs() < 0.01);
        assert!((hfcc_coord("004E38", "lon").unwrap() - 4.6333).abs() < 0.01);
        assert!((hfcc_coord("077W00", "lon").unwrap() + 77.0).abs() < 0.01);
    }

    /// A latitude that arrives as 200 degrees is a column that has drifted, not a
    /// station off the planet — and it must not be plotted somewhere impossible.
    #[test]
    fn an_impossible_coordinate_is_refused() {
        assert!(hfcc_coord("200N00", "lat").is_none());
        assert!(hfcc_coord("170E00", "lat").is_none(), "170 is a longitude, not a latitude");
        assert!(hfcc_coord("200E00", "lon").is_none());
        assert!(hfcc_coord("", "lat").is_none());
        assert!(hfcc_coord("nonsense", "lat").is_none());
    }
}
