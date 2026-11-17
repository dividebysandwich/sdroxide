//! The sighting book: every aircraft heard, one line a day, kept for
//! [`KEEP_DAYS`].
//!
//! The table forgets an aircraft a minute after its last frame; the book keeps
//! when it came and went, what it was, where it flew and how far from home it
//! was heard. One CSV file per local day in the config folder (`adsb-log/`),
//! written once a minute, so a day can be opened in a spreadsheet as it is.
//!
//! The book also learns types: whenever the standing data knows the airframe
//! behind a callsign, the callsign remembers the type, and the airline counts
//! it. An aircraft the standing data does not know then gets the type its
//! flight number last flew with, else its airline's usual type, marked as a
//! guess. Airlines put the same flight on the same type most days, and many
//! fly one family; a wrong guess now and then is accepted (HDZ 2026-11-17:
//! „nur nicht overengineeren, Fehlerrate ist ok“).

use std::collections::{BTreeMap, HashMap};

use sdroxide_types::AdsbAircraft;

/// Days of files kept, and of learning read back at start.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub const KEEP_DAYS: i64 = 30;

/// An airline's usual type is offered only when it is at least this share of
/// what the airline has been seen with, over at least [`FLEET_MIN`] aircraft.
const FLEET_SHARE: f32 = 0.8;
const FLEET_MIN: u32 = 5;

/// What the panel knows of an aircraft beyond its own messages.
#[derive(Debug, Clone, Default)]
pub struct Known {
    /// ICAO type code from the standing data, empty when unknown.
    pub kind: String,
    /// The route by airport code, `EHAM›LPPT`, empty when unknown.
    pub route: String,
    /// ISO country of the address block.
    pub country: String,
}

/// One aircraft on one day.
#[derive(Debug, Clone, PartialEq)]
pub struct Sighting {
    pub icao: u32,
    pub call: String,
    pub kind: String,
    pub route: String,
    pub country: String,
    pub first: i64,
    pub last: i64,
    /// Nearest and farthest position from home, km; `NaN` without one.
    pub min_km: f32,
    pub max_km: f32,
    pub max_alt_ft: Option<i32>,
}

/// The day's sightings and what has been learnt about types.
#[derive(Default)]
pub struct SightingBook {
    /// The local day the sightings are for, `2026-11-17`.
    day: String,
    today: BTreeMap<u32, Sighting>,
    /// Callsign → the type last seen flying it.
    calls: HashMap<String, String>,
    /// Airline code → type → aircraft-days seen.
    fleet: HashMap<String, HashMap<String, u32>>,
    loaded: bool,
    dirty: bool,
    saved_at: i64,
}

/// The airline part of a callsign, `PGT` of `PGT49BX`.
fn airline_of(call: &str) -> Option<&str> {
    let b = call.as_bytes();
    (b.len() >= 4 && b[..3].iter().all(u8::is_ascii_alphabetic) && b[3].is_ascii_digit())
        .then(|| &call[..3])
}

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian
/// (H. Hinnant's `civil_from_days`). Here rather than chrono, which the
/// browser build does not have.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Local seconds since the epoch: the operator's clock.
fn local(t: i64) -> i64 {
    t + crate::time::local_offset_at(t)
}

pub(crate) fn local_day(t: i64) -> String {
    let (y, m, d) = civil(local(t).div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// `2026-11-17 13:24:05` local: date and time a spreadsheet reads as such.
pub(crate) fn local_stamp(t: i64) -> String {
    let s = local(t).rem_euclid(86_400);
    format!("{} {:02}:{:02}:{:02}", local_day(t), s / 3600, s % 3600 / 60, s % 60)
}

/// `HH:MM` local.
pub fn local_hm(t: i64) -> String {
    let s = local(t).rem_euclid(86_400);
    format!("{:02}:{:02}", s / 3600, s % 3600 / 60)
}

impl SightingBook {
    /// Take in a table snapshot. `known` looks up what the standing data has
    /// on an aircraft.
    pub fn note(
        &mut self,
        home: Option<(f64, f64)>,
        aircraft: &[AdsbAircraft],
        mut known: impl FnMut(&AdsbAircraft) -> Known,
        now: i64,
    ) {
        if !self.loaded {
            self.loaded = true;
            self.load(now);
        }
        let day = local_day(now);
        if day != self.day {
            if self.dirty {
                self.save(now);
            }
            self.day = day;
            self.today.clear();
        }
        for a in aircraft {
            // Heard today, not a leftover from before midnight.
            if a.last_at <= 0 || local_day(a.last_at) != self.day {
                continue;
            }
            let k = known(a);
            let call = a.callsign.trim().to_ascii_uppercase();
            let s = self.today.entry(a.icao).or_insert_with(|| Sighting {
                icao: a.icao,
                call: String::new(),
                kind: String::new(),
                route: String::new(),
                country: String::new(),
                first: if local_day(a.first_at) == self.day { a.first_at } else { a.last_at },
                last: a.last_at,
                min_km: f32::NAN,
                max_km: f32::NAN,
                max_alt_ft: None,
            });
            s.last = s.last.max(a.last_at);
            if !call.is_empty() {
                s.call = call.clone();
            }
            if !k.route.is_empty() {
                s.route = k.route;
            }
            if !k.country.is_empty() {
                s.country = k.country;
            }
            if let Some(alt) = a.altitude_ft {
                s.max_alt_ft = Some(s.max_alt_ft.map_or(alt, |m| m.max(alt)));
            }
            if let (Some(h), Some(pos)) = (home, a.lat.zip(a.lon)) {
                let km = sdroxide_types::distance_km(h, pos) as f32;
                s.min_km = if s.min_km.is_nan() { km } else { s.min_km.min(km) };
                s.max_km = if s.max_km.is_nan() { km } else { s.max_km.max(km) };
            }
            // Learnt once per aircraft and day, when the type first arrives.
            if !k.kind.is_empty() && s.kind.is_empty() {
                s.kind = k.kind;
                learn(&mut self.calls, &mut self.fleet, &s.call, &s.kind);
            }
            self.dirty = true;
        }
        if self.dirty && now - self.saved_at >= 60 {
            self.save(now);
        }
    }

    /// A type for a callsign the standing data has no airframe for: the one
    /// its flight number flew with last, else its airline's usual one.
    pub fn guess(&self, callsign: &str) -> Option<&str> {
        let call = callsign.trim().to_ascii_uppercase();
        if let Some(k) = self.calls.get(&call) {
            return Some(k);
        }
        let fleet = self.fleet.get(airline_of(&call)?)?;
        let total: u32 = fleet.values().sum();
        let (k, &n) = fleet.iter().max_by_key(|(k, n)| (**n, std::cmp::Reverse(k.as_str())))?;
        (total >= FLEET_MIN && n as f32 >= FLEET_SHARE * total as f32).then_some(k.as_str())
    }

    /// Today's sightings, latest first.
    pub fn today(&self) -> Vec<&Sighting> {
        let mut v: Vec<&Sighting> = self.today.values().collect();
        v.sort_by_key(|s| std::cmp::Reverse(s.last));
        v
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn dir() -> Option<std::path::PathBuf> {
        Some(sdroxide_config::config_dir().ok()?.join("adsb-log"))
    }

    fn save(&mut self, now: i64) {
        self.saved_at = now;
        self.dirty = false;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(dir) = Self::dir() else { return };
            if self.day.is_empty() || std::fs::create_dir_all(&dir).is_err() {
                return;
            }
            let _ = std::fs::write(dir.join(format!("{}.csv", self.day)), encode(&self.today));
        }
    }

    /// Today's file back into the day, every kept file into the learning,
    /// and files past [`KEEP_DAYS`] removed.
    fn load(&mut self, now: i64) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(dir) = Self::dir() else { return };
            let Ok(rd) = std::fs::read_dir(&dir) else { return };
            let today = local_day(now);
            let oldest = local_day(now - KEEP_DAYS * 86_400);
            let mut names: Vec<_> = rd
                .flatten()
                .filter_map(|e| {
                    Some((e.file_name().to_str()?.strip_suffix(".csv")?.to_string(), e.path()))
                })
                .collect();
            // Oldest first, so the latest flight wins a callsign's type.
            names.sort();
            for (day, path) in names {
                if day < oldest {
                    let _ = std::fs::remove_file(path);
                    continue;
                }
                let Ok(body) = std::fs::read_to_string(&path) else { continue };
                let rows = decode(&body);
                for s in rows.values() {
                    if !s.kind.is_empty() {
                        learn(&mut self.calls, &mut self.fleet, &s.call, &s.kind);
                    }
                }
                if day == today {
                    self.day = day;
                    self.today = rows;
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = now;
    }
}

fn learn(
    calls: &mut HashMap<String, String>,
    fleet: &mut HashMap<String, HashMap<String, u32>>,
    call: &str,
    kind: &str,
) {
    if call.is_empty() {
        return;
    }
    calls.insert(call.to_string(), kind.to_string());
    if let Some(al) = airline_of(call) {
        *fleet.entry(al.to_string()).or_default().entry(kind.to_string()).or_default() += 1;
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
/// The last two columns repeat `first` and `last` as local date and time, for
/// a reader with a spreadsheet; the program reads the epoch seconds.
/// Why: HDZ 2026-11-17 (Buch „mit Datum“) — epoch seconds do not read as a
///      date in Excel or Numbers.
const HEAD: &str =
    "first,last,icao,callsign,type,route,country,min_km,max_km,max_alt_ft,first_local,last_local";

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn encode(rows: &BTreeMap<u32, Sighting>) -> String {
    let mut out = format!("{HEAD}\n");
    let km = |v: f32| if v.is_nan() { String::new() } else { format!("{v:.0}") };
    for s in rows.values() {
        out.push_str(&format!(
            "{},{},{:06X},{},{},{},{},{},{},{},{},{}\n",
            s.first,
            s.last,
            s.icao,
            s.call,
            s.kind,
            s.route,
            s.country,
            km(s.min_km),
            km(s.max_km),
            s.max_alt_ft.map(|a| a.to_string()).unwrap_or_default(),
            local_stamp(s.first),
            local_stamp(s.last),
        ));
    }
    out
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn decode(body: &str) -> BTreeMap<u32, Sighting> {
    let mut out = BTreeMap::new();
    for line in body.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        // Ten columns before 2026-11-17, twelve since; the local times are
        // for people and not read back.
        let [first, last, icao, call, kind, route, country, min_km, max_km, alt, ..] = f[..] else {
            continue;
        };
        let (Ok(first), Ok(last), Ok(icao)) =
            (first.parse(), last.parse(), u32::from_str_radix(icao, 16))
        else {
            continue;
        };
        let km = |v: &str| v.parse().unwrap_or(f32::NAN);
        out.insert(
            icao,
            Sighting {
                icao,
                call: call.to_string(),
                kind: kind.to_string(),
                route: route.to_string(),
                country: country.to_string(),
                first,
                last,
                min_km: km(min_km),
                max_km: km(max_km),
                max_alt_ft: alt.parse().ok(),
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_file_carries_local_times_and_old_files_still_read() {
        let mut rows = BTreeMap::new();
        let s = Sighting {
            icao: 0x3E28E4,
            call: "DIGRK".into(),
            kind: "C25A".into(),
            route: String::new(),
            country: "DE".into(),
            first: 1_794_917_355,
            last: 1_794_917_649,
            min_km: 7.0,
            max_km: 21.0,
            max_alt_ft: Some(10_000),
        };
        rows.insert(s.icao, s.clone());
        let body = encode(&rows);
        let line = body.lines().nth(1).unwrap();
        let f: Vec<&str> = line.split(',').collect();
        assert_eq!(f.len(), 12);
        assert_eq!(f[10], local_stamp(s.first));
        assert!(f[10].starts_with(&local_day(s.first)) && f[10].len() == 19);
        assert_eq!(decode(&body)[&s.icao].last, s.last);
        // A file written before the local columns.
        let old = "h\n1794917355,1794917649,3E28E4,DIGRK,C25A,,DE,7,21,10000\n";
        assert_eq!(decode(old)[&0x3E28E4].call, "DIGRK");
    }

    fn ac(icao: u32, call: &str, seen: i64) -> AdsbAircraft {
        let mut a = AdsbAircraft::new(icao, seen);
        a.callsign = call.into();
        a.last_at = seen;
        a
    }

    fn book() -> SightingBook {
        // Loaded already, so the tests never touch the config folder.
        SightingBook { loaded: true, saved_at: i64::MAX / 2, ..Default::default() }
    }

    #[test]
    fn a_callsign_remembers_its_type_and_an_airline_its_usual_one() {
        let mut b = book();
        let t = 1_763_380_000;
        let fleet = ["PGT1", "PGT2", "PGT3", "PGT4", "PGT5"];
        let air: Vec<_> =
            fleet.iter().enumerate().map(|(i, c)| ac(0x4B_C800 + i as u32, c, t)).collect();
        b.note(None, &air, |_| Known { kind: "A21N".into(), ..Default::default() }, t);
        assert_eq!(b.guess("PGT1"), Some("A21N"));
        assert_eq!(b.guess("PGT77X"), Some("A21N"), "the airline's usual type");
        assert_eq!(b.guess("DLH1"), None);
        // A mixed fleet offers no usual type.
        b.note(
            None,
            &[ac(0x4B_C900, "PGT9", t)],
            |_| Known { kind: "B38M".into(), ..Default::default() },
            t,
        );
        assert_eq!(b.guess("PGT77X"), Some("A21N"), "5 of 6 is still over 80 %");
        b.note(
            None,
            &[ac(0x4B_CA00, "PGT10", t)],
            |_| Known { kind: "B38M".into(), ..Default::default() },
            t,
        );
        assert_eq!(b.guess("PGT77X"), None, "5 of 7 is not");
        assert_eq!(b.guess("PGT9"), Some("B38M"), "its own flight still knows");
    }

    #[test]
    fn a_day_survives_the_file() {
        let mut b = book();
        let t = 1_763_380_000;
        let mut a = ac(0x40_812E, "EZY83UK", t);
        a.altitude_ft = Some(37_025);
        a.lat = Some(51.5);
        a.lon = Some(6.0);
        b.note(
            Some((51.5, 7.5)),
            &[a],
            |_| Known { kind: "A20N".into(), route: "EHAM›LPPT".into(), country: "GB".into() },
            t,
        );
        let back = decode(&encode(&b.today));
        // Equal but for the distances, which the file keeps to the kilometre.
        let (x, y) = (&back[&0x40_812E], &b.today[&0x40_812E]);
        assert_eq!((&x.kind, &x.country, x.first, x.last), (&y.kind, &y.country, y.first, y.last));
        let s = &back[&0x40_812E];
        assert_eq!(
            (s.call.as_str(), s.route.as_str(), s.max_alt_ft),
            ("EZY83UK", "EHAM›LPPT", Some(37_025))
        );
        assert!((s.max_km - 104.0).abs() < 2.0, "{}", s.max_km);
    }

    #[test]
    fn the_calendar_is_right_across_years_and_leap_days() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_782), (2024, 2, 29));
        assert_eq!(civil(20_774), (2026, 11, 17));
        assert_eq!(civil(-1), (1969, 12, 31));
    }
}
