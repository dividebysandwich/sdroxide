//! Award tallies (DXCC / WAS / WAZ / grid squares) computed purely over the
//! logbook, using [`crate::entity`] to resolve a callsign's DXCC entity and CQ
//! zone when the record doesn't carry them explicitly. Worked-vs-confirmed is
//! driven by [`QsoRecord::is_confirmed`].

use std::collections::BTreeMap;

use crate::geo::grid4;
use crate::{QsoRecord, entity};

/// Worked / confirmed status for one award slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Status {
    pub worked: bool,
    pub confirmed: bool,
}

/// The full set of award tallies for a (filtered) log.
#[derive(Debug, Clone, Default)]
pub struct Awards {
    /// DXCC entities by name.
    pub dxcc: BTreeMap<String, Status>,
    /// Worked All States by 2-letter state.
    pub was: BTreeMap<String, Status>,
    /// Worked All Zones by CQ zone (1..40).
    pub waz: BTreeMap<u8, Status>,
    /// Grid squares (4-char).
    pub grids: BTreeMap<String, Status>,
}

/// (worked, confirmed) counts of a status map.
pub fn counts<K>(map: &BTreeMap<K, Status>) -> (usize, usize) {
    let worked = map.values().filter(|s| s.worked).count();
    let confirmed = map.values().filter(|s| s.confirmed).count();
    (worked, confirmed)
}

/// The 50 US states plus DC — the WAS target set.
pub const US_STATES: [&str; 51] = [
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC",
];

fn is_us_state(s: &str) -> bool {
    US_STATES.contains(&s)
}

/// Compute award tallies over `log`, optionally filtered to one `band` and/or
/// `mode` (case-insensitive; `None` = all).
pub fn compute_awards(log: &[QsoRecord], band: Option<&str>, mode: Option<&str>) -> Awards {
    let mut a = Awards::default();
    for q in log {
        if q.call.trim().is_empty() {
            continue;
        }
        if let Some(b) = band {
            if !q.band.eq_ignore_ascii_case(b) {
                continue;
            }
        }
        if let Some(m) = mode {
            if !q.mode.eq_ignore_ascii_case(m) {
                continue;
            }
        }
        let conf = q.is_confirmed();
        let ent = entity::resolve_callsign(&q.call);

        // DXCC entity (prefer the resolver; fall back to the record's country).
        let ent_name = ent.map(|e| e.name.to_string()).or_else(|| {
            let c = q.country.trim();
            (!c.is_empty()).then(|| c.to_string())
        });
        if let Some(name) = ent_name {
            let s = a.dxcc.entry(name).or_default();
            s.worked = true;
            s.confirmed |= conf;
        }

        // WAZ: record's CQ zone, else the resolver's.
        if let Some(z) =
            q.cq_zone.or_else(|| ent.map(|e| e.cq_zone)).filter(|&z| (1..=40).contains(&z))
        {
            let s = a.waz.entry(z).or_default();
            s.worked = true;
            s.confirmed |= conf;
        }

        // WAS: a US state on the record.
        let st = q.state.trim().to_ascii_uppercase();
        if is_us_state(&st) {
            let s = a.was.entry(st).or_default();
            s.worked = true;
            s.confirmed |= conf;
        }

        // Grid squares (4-char).
        if let Some(g4) = q.grid.as_deref().and_then(grid4) {
            let s = a.grids.entry(g4).or_default();
            s.worked = true;
            s.confirmed |= conf;
        }
    }
    a
}

/// The DXCC entity name for a callsign, if resolvable.
pub fn entity_name(call: &str) -> Option<&'static str> {
    entity::resolve_callsign(call).map(|e| e.name)
}

// ── "What is still missing?" — the award tallies, placed on the map ──

/// How far along one award slot is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// Never worked. The thing a heat map exists to show.
    Missing,
    /// In the log, but no QSL of any kind has come back.
    Worked,
    Confirmed,
}

/// One DXCC entity, where it is, and how far along it is in the log.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntitySlot {
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub continent: &'static str,
    pub coverage: Coverage,
}

/// Every DXCC entity the country file knows, placed and judged against
/// `awards` — the whole target set, not only the part already in the log,
/// because the gaps are the point.
///
/// Entities the log names but the country file does not (an imported log's own
/// country column, a deleted entity) are left out: there is nowhere to draw
/// them, and inventing a position would be worse than omitting one.
pub fn entity_coverage(awards: &Awards) -> Vec<EntitySlot> {
    entity::all_entities()
        .iter()
        .map(|e| EntitySlot {
            name: e.name,
            lat: e.lat,
            lon: e.lon,
            continent: e.continent,
            coverage: match awards.dxcc.get(e.name) {
                Some(s) if s.confirmed => Coverage::Confirmed,
                Some(s) if s.worked => Coverage::Worked,
                _ => Coverage::Missing,
            },
        })
        .collect()
}

/// (missing, worked, confirmed) over a coverage list.
pub fn coverage_counts(slots: &[EntitySlot]) -> (usize, usize, usize) {
    let count = |c: Coverage| slots.iter().filter(|s| s.coverage == c).count();
    (count(Coverage::Missing), count(Coverage::Worked), count(Coverage::Confirmed))
}

// ── "Is this one worth working?" — the live decode list's view of the log ──

/// What a heard station would be worth working, judged against the log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Novelty {
    /// Their DXCC entity has never been worked, on any band.
    pub new_dxcc: bool,
    /// Their entity is in the log, but not from this band.
    pub new_dxcc_band: bool,
    /// Their grid square has never been worked.
    pub new_grid: bool,
    /// Their callsign has never been worked.
    pub new_call: bool,
    /// Their callsign is already in the log for this band — a dupe.
    pub dupe: bool,
}

/// The single most interesting thing about a station, for a one-badge display.
/// Ordered rarest-first: a new entity outranks a new grid outranks a new call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Highlight {
    NewDxcc,
    NewDxccBand,
    NewGrid,
    NewCall,
    Dupe,
}

impl Novelty {
    /// The one badge to show for this station, or `None` when there's nothing
    /// to say (worked before, but not on this band — neither new nor a dupe).
    pub fn highlight(self) -> Option<Highlight> {
        if self.new_dxcc {
            Some(Highlight::NewDxcc)
        } else if self.new_dxcc_band {
            Some(Highlight::NewDxccBand)
        } else if self.new_grid {
            Some(Highlight::NewGrid)
        } else if self.new_call {
            Some(Highlight::NewCall)
        } else if self.dupe {
            Some(Highlight::Dupe)
        } else {
            None
        }
    }

    /// True when working this station would put something new in the log.
    pub fn is_new(self) -> bool {
        self.new_dxcc || self.new_dxcc_band || self.new_grid || self.new_call
    }
}

/// Membership sets over the log, so the decode list can ask "is this one new?"
/// for every row of every slot without re-walking thousands of QSOs. Built once
/// and rebuilt when the log changes.
#[derive(Debug, Clone, Default)]
pub struct LogIndex {
    calls: std::collections::HashSet<String>,
    call_band: std::collections::HashSet<(String, String)>,
    dxcc: std::collections::HashSet<String>,
    dxcc_band: std::collections::HashSet<(String, String)>,
    grids: std::collections::HashSet<String>,
    /// Whether *any* record carries a band. An imported log without band
    /// columns can't support per-band judgements, and claiming everything is
    /// new-on-this-band would be worse than saying nothing.
    has_bands: bool,
}

/// The DXCC entity for a logged QSO: the resolver first, the record's own
/// country field as a fallback (imported logs may carry one we can't resolve).
fn record_entity(q: &QsoRecord) -> Option<String> {
    entity_name(&q.call).map(str::to_string).or_else(|| {
        let c = q.country.trim();
        (!c.is_empty()).then(|| c.to_string())
    })
}

impl LogIndex {
    pub fn build(log: &[QsoRecord]) -> Self {
        let mut ix = LogIndex::default();
        for q in log {
            let call = q.call.trim().to_ascii_uppercase();
            if call.is_empty() {
                continue;
            }
            let band = q.band.trim().to_ascii_lowercase();
            ix.calls.insert(call.clone());
            if !band.is_empty() {
                ix.has_bands = true;
                ix.call_band.insert((call, band.clone()));
            }
            if let Some(ent) = record_entity(q) {
                ix.dxcc.insert(ent.clone());
                if !band.is_empty() {
                    ix.dxcc_band.insert((ent, band.clone()));
                }
            }
            if let Some(g) = q.grid.as_deref().and_then(grid4) {
                ix.grids.insert(g);
            }
        }
        ix
    }

    /// How interesting a heard station is. `band` is the ADIF band we'd work
    /// them on (see [`crate::adif_band`]); an empty band skips the per-band
    /// judgements, leaving only the all-time ones.
    pub fn novelty(&self, call: &str, grid: Option<&str>, band: &str) -> Novelty {
        let call = call.trim().to_ascii_uppercase();
        if call.is_empty() {
            return Novelty::default();
        }
        let band = band.trim().to_ascii_lowercase();
        let per_band = self.has_bands && !band.is_empty();
        let (new_dxcc, new_dxcc_band) = match entity_name(&call) {
            Some(e) if !self.dxcc.contains(e) => (true, false),
            Some(e) => {
                (false, per_band && !self.dxcc_band.contains(&(e.to_string(), band.clone())))
            }
            None => (false, false),
        };
        Novelty {
            new_dxcc,
            new_dxcc_band,
            new_grid: grid.and_then(grid4).is_some_and(|g| !self.grids.contains(&g)),
            new_call: !self.calls.contains(&call),
            dupe: per_band && self.call_band.contains(&(call, band)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso(call: &str, band: &str, grid: Option<&str>) -> QsoRecord {
        QsoRecord {
            call: call.into(),
            band: band.into(),
            grid: grid.map(str::to_string),
            mode: "FT8".into(),
            ..Default::default()
        }
    }

    #[test]
    fn novelty_ranks_entity_over_grid_over_call() {
        let ix = LogIndex::build(&[qso("W1AW", "20m", Some("FN31"))]);

        // A brand-new entity is the headline, whatever else is also new.
        let n = ix.novelty("DL1ABC", Some("JO62"), "20m");
        assert!(n.new_dxcc && n.new_grid && n.new_call && !n.dupe);
        assert_eq!(n.highlight(), Some(Highlight::NewDxcc));

        // Same entity, new band → the band-slot badge.
        let n = ix.novelty("K2XYZ", Some("FN31"), "40m");
        assert!(!n.new_dxcc && n.new_dxcc_band);
        assert_eq!(n.highlight(), Some(Highlight::NewDxccBand));

        // Same entity and band, but a grid we've never had.
        let n = ix.novelty("K2XYZ", Some("EM48"), "20m");
        assert_eq!(n.highlight(), Some(Highlight::NewGrid));

        // Nothing new but the callsign itself.
        let n = ix.novelty("K2XYZ", Some("FN31"), "20m");
        assert_eq!(n.highlight(), Some(Highlight::NewCall));

        // The one station we have worked, on the same band → a dupe.
        let n = ix.novelty("W1AW", Some("FN31"), "20m");
        assert!(!n.is_new() && n.dupe);
        assert_eq!(n.highlight(), Some(Highlight::Dupe));
        // …but on another band they're a new band-slot, not a dupe.
        let n = ix.novelty("W1AW", Some("FN31"), "15m");
        assert!(!n.dupe);
        assert_eq!(n.highlight(), Some(Highlight::NewDxccBand));
    }

    /// The heat map's job is to show the gaps, so the list has to carry every
    /// entity — including the hundreds nobody has worked yet — and rank a
    /// confirmed one above a merely worked one.
    #[test]
    fn coverage_places_every_entity_and_grades_the_log() {
        let mut worked = qso("DL1ABC", "20m", Some("JO62"));
        let mut confirmed = qso("JA1XYZ", "20m", Some("PM95"));
        confirmed.lotw_rcvd = true;
        worked.lotw_rcvd = false;
        let a = compute_awards(&[worked, confirmed], None, None);

        let slots = entity_coverage(&a);
        let find = |name: &str| *slots.iter().find(|s| s.name == name).expect(name);
        assert_eq!(find("Fed. Rep. of Germany").coverage, Coverage::Worked);
        assert_eq!(find("Japan").coverage, Coverage::Confirmed);
        assert_eq!(find("Mongolia").coverage, Coverage::Missing);

        let (missing, worked, confirmed) = coverage_counts(&slots);
        assert_eq!((worked, confirmed), (1, 1));
        assert_eq!(missing, slots.len() - 2, "the rest of the world should read as missing");
    }

    #[test]
    fn a_log_without_bands_makes_no_per_band_claims() {
        // An imported log with no BAND column: every station would otherwise
        // look like a new band-slot, which says nothing useful.
        let ix = LogIndex::build(&[qso("W1AW", "", Some("FN31"))]);
        let n = ix.novelty("K2XYZ", Some("FN31"), "20m");
        assert!(!n.new_dxcc && !n.new_dxcc_band && !n.dupe);
        assert_eq!(n.highlight(), Some(Highlight::NewCall));
        // Re-hearing the logged station claims neither new nor dupe.
        assert_eq!(ix.novelty("W1AW", Some("FN31"), "20m").highlight(), None);
    }

    #[test]
    fn unresolvable_prefixes_and_missing_grids_are_quiet() {
        let ix = LogIndex::build(&[qso("W1AW", "20m", None)]);
        // No entity for "QQ1QQ" → no entity claim either way, but the call and
        // grid judgements still stand.
        let n = ix.novelty("QQ1QQ", None, "20m");
        assert!(!n.new_dxcc && !n.new_dxcc_band && !n.new_grid && n.new_call);
        // An empty callsign is not a station.
        assert_eq!(ix.novelty("", None, "20m"), Novelty::default());
    }
}
