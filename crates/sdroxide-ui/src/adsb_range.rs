//! The range log: how far this station hears aircraft, day by day, filed
//! under the setup that heard them.
//!
//! The coverage diagram ([`crate::adsb_coverage`]) answers "how does the
//! antenna do this week"; it keeps a week and starts over when it is reset.
//! The range log keeps the same cells — [`SECTORS`] slices of bearing by
//! [`RINGS`] rings, a position counted only when its aircraft gave another
//! less than [`CONTINUOUS_S`] before — by **setup** and **day**, for as long
//! as the size limit allows, so a setup can be set against the one before it:
//! the antenna on the roof against the one at the window, the Pi in a plastic
//! case against the metal one.
//!
//! A setup is an instrument — the receiver the positions came from, named
//! the way the radio's tab names it — with an antenna label and a home, and
//! a version number counted per instrument. A new version starts by itself
//! when the home moves, and by hand when the antenna or anything else about
//! the setup changes. Each instrument has its own open setup, so going from
//! one radio to another and back carries on where it was.
//!
//! The file is plain SQLite (`adsb-range.sqlite` in the configuration
//! folder), in the manner of the PANO recorder ([`crate::recorder`]): readable
//! by anything that reads SQLite while the radio is off, its size shown and
//! capped, a setup deletable on its own and the whole file emptied with INIT.
//! The view `range_log` gives one line per setup, day and sector with the
//! range in kilometres — the log as a table, for a spreadsheet or DuckDB.
// Why: HDZ 2026-11-18: „beim log wäre noch geil ein range log zu haben welches
//      evt noch instrument aware ist. bekommen wir die range observations noch
//      geversioniert?“ — „ein aufbau mit eigener versionierung und dann im sql
//      server oder als datei ein log. so wie das panorama log. nimm den
//      gleichen code dafür, der ist schon gut, größenbegrenzt, hat datum und
//      erlaubt löschen … bau ein extra modul an“.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use sdroxide_types::AdsbAircraft;

use crate::adsb_coverage::{BOUNDARY_MIN, CONTINUOUS_S, Cell, RING_KM, RINGS, SECTORS, cell_of};

/// The file's name in the configuration directory.
pub const FILE_NAME: &str = "adsb-range.sqlite";
/// The size limit a fresh file starts with, megabytes. A day with a
/// thousand cells takes some 80 kB with the index (measured 2026-11-18), so
/// this holds well over a year of busy days.
pub const DEFAULT_MAX_MB: f64 = 50.0;
/// The limit's range in the settings.
pub const MIN_MAX_MB: f64 = 1.0;
pub const MAX_MAX_MB: f64 = 10_000.0;
/// How often what has been gathered is written, seconds.
pub const FLUSH_S: i64 = 60;
/// A home further than this from the open setup's starts a new version, km.
pub const HOME_KM: f64 = 1.0;
/// What one stored cell costs on disk with its index entry, near enough, for
/// the per-setup size (84 bytes measured on a thousand cells a day).
const ROW_BYTES: i64 = 84;

/// A setup as the inventory lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct SetupInfo {
    pub id: i64,
    pub instrument: String,
    /// Counted per instrument, from 1.
    pub version: i64,
    pub antenna: String,
    pub home: (f64, f64),
    pub started_ms: i64,
    /// The last time something was written for it.
    pub ended_ms: i64,
    /// Whether a newer version of the same instrument has replaced it.
    pub closed: bool,
    pub days: i64,
    pub positions: i64,
    /// The farthest boundary of any of its days, km; 0 without one.
    pub max_km: f64,
    pub bytes: i64,
    /// First and last day with positions, `2026-11-18`; empty without any.
    pub first_day: String,
    pub last_day: String,
}

/// One day of one setup: the line of the log.
#[derive(Debug, Clone, PartialEq)]
pub struct DayRange {
    pub day: String,
    pub positions: i64,
    /// Sectors with a boundary: at least [`BOUNDARY_MIN`] positions in one
    /// cell.
    pub sectors: usize,
    /// The median of the sectors' ranges, km; 0 without any.
    pub median_km: f64,
    /// The farthest boundary, km, and its bearing, degrees.
    pub max_km: f64,
    pub max_bearing: f64,
}

/// The open setup the positions go to.
#[derive(Debug, Clone, PartialEq)]
struct Current {
    id: i64,
    instrument: String,
    home: (f64, f64),
}

/// The range log and the file behind it.
pub struct RangeLog {
    db: Connection,
    path: PathBuf,
    current: Option<Current>,
    /// Gathered since the last write: (setup, day, cell) → counts.
    pending: HashMap<(i64, String, u16), Cell>,
    /// Each aircraft's last position time, for the continuity test.
    last_pos: HashMap<u32, i64>,
    flushed_at: i64,
}

/// What is gathered but not yet written goes to the file on the way out.
impl Drop for RangeLog {
    fn drop(&mut self) {
        let _ = self.flush(crate::time::now_unix());
    }
}

impl RangeLog {
    /// The file in the configuration directory, made on first use.
    pub fn open_default() -> Result<Self, String> {
        let dir = sdroxide_config::config_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Self::open(&dir.join(FILE_NAME)).map_err(|e| e.to_string())
    }

    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let db = Connection::open(path)?;
        // Two radio tabs share the file; one waits briefly for the other.
        db.busy_timeout(std::time::Duration::from_millis(500))?;
        // auto_vacuum must precede the first table, as in the recorder: a
        // deleted setup gives its pages back, so the size shown is on disk.
        db.execute_batch(&format!(
            "PRAGMA auto_vacuum = INCREMENTAL;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS setup (
                 id         INTEGER PRIMARY KEY AUTOINCREMENT,
                 instrument TEXT    NOT NULL,
                 version    INTEGER NOT NULL,
                 antenna    TEXT    NOT NULL DEFAULT '',
                 home_lat   REAL    NOT NULL,
                 home_lon   REAL    NOT NULL,
                 started_ms INTEGER NOT NULL,
                 ended_ms   INTEGER NOT NULL,
                 closed     INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS cell (
                 setup    INTEGER NOT NULL REFERENCES setup(id) ON DELETE CASCADE,
                 day      TEXT    NOT NULL,
                 idx      INTEGER NOT NULL,
                 n        INTEGER NOT NULL,
                 rssi_sum REAL    NOT NULL,
                 rssi_max REAL    NOT NULL,
                 PRIMARY KEY (setup, day, idx)
             );
             CREATE INDEX IF NOT EXISTS cell_day ON cell(day);
             CREATE VIEW IF NOT EXISTS range_log AS
                 SELECT s.id AS setup, s.instrument, s.version, s.antenna,
                        c.day, c.idx / {RINGS} AS sector,
                        (c.idx / {RINGS}) * {deg} AS bearing_deg,
                        MAX(CASE WHEN c.n >= {BOUNDARY_MIN}
                                 THEN (c.idx % {RINGS} + 1) * {RING_KM} END) AS range_km,
                        SUM(c.n) AS positions,
                        MAX(c.rssi_max) AS rssi_max_dbfs
                 FROM cell c JOIN setup s ON s.id = c.setup
                 GROUP BY c.setup, c.day, c.idx / {RINGS};",
            deg = 360 / SECTORS,
        ))?;
        Ok(RangeLog {
            db,
            path: path.to_path_buf(),
            current: None,
            pending: HashMap::new(),
            last_pos: HashMap::new(),
            flushed_at: crate::time::now_unix(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The size limit stored with the file, megabytes.
    pub fn max_mb(&self) -> f64 {
        self.db
            .query_row("SELECT value FROM meta WHERE key = 'max_mb'", [], |r| r.get::<_, String>(0))
            .optional()
            .ok()
            .flatten()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .map_or(DEFAULT_MAX_MB, |v| v.clamp(MIN_MAX_MB, MAX_MAX_MB))
    }

    pub fn set_max_mb(&mut self, mb: f64) -> rusqlite::Result<usize> {
        self.db.execute(
            "INSERT INTO meta (key, value) VALUES ('max_mb', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![mb.clamp(MIN_MAX_MB, MAX_MAX_MB).to_string()],
        )?;
        self.trim((self.max_mb() * 1e6) as u64)
    }

    /// The open setup positions are filed under, if one is.
    pub fn current(&self) -> Option<i64> {
        self.current.as_ref().map(|c| c.id)
    }

    /// Take in a table snapshot heard by `instrument`. Each aircraft counts
    /// once per new position; what is gathered is written every
    /// [`FLUSH_S`].
    pub fn note(
        &mut self,
        instrument: &str,
        home: Option<(f64, f64)>,
        aircraft: &[AdsbAircraft],
        now: i64,
    ) -> rusqlite::Result<()> {
        let Some(home) = home else { return Ok(()) };
        if instrument.is_empty() {
            return Ok(());
        }
        let id = self.setup_for(instrument, home, now)?;
        for a in aircraft {
            let (Some(pos), t) = (a.lat.zip(a.lon), a.last_pos_at) else { continue };
            if t <= 0 {
                continue;
            }
            let prev = self.last_pos.insert(a.icao, t);
            let Some(prev) = prev else { continue };
            if t <= prev || t - prev > CONTINUOUS_S {
                continue;
            }
            let Some(idx) = cell_of(home, pos) else { continue };
            let day = crate::adsb_log::local_day(t);
            self.pending.entry((id, day, idx as u16)).or_default().add(a.rssi_dbfs);
        }
        self.last_pos.retain(|_, &mut t| now - t < 600);
        if now - self.flushed_at >= FLUSH_S {
            self.flush(now)?;
        }
        Ok(())
    }

    /// The open setup of `instrument` for `home`: the one already open, or a
    /// new version when there is none or the home has moved.
    fn setup_for(&mut self, instrument: &str, home: (f64, f64), now: i64) -> rusqlite::Result<i64> {
        if let Some(c) = &self.current
            && c.instrument == instrument
            && sdroxide_types::distance_km(c.home, home) <= HOME_KM
        {
            return Ok(c.id);
        }
        // Another instrument, or the first table: its continuity starts over.
        if self.current.as_ref().is_none_or(|c| c.instrument != instrument) {
            self.last_pos.clear();
        }
        let open: Option<(i64, f64, f64, String)> = self
            .db
            .query_row(
                "SELECT id, home_lat, home_lon, antenna FROM setup
                 WHERE instrument = ?1 AND closed = 0 ORDER BY id DESC LIMIT 1",
                params![instrument],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let id = match open {
            Some((id, la, lo, _)) if sdroxide_types::distance_km((la, lo), home) <= HOME_KM => id,
            // The home moved: the antenna went with it, so its label does.
            Some((_, _, _, antenna)) => self.begin(instrument, &antenna, home, now)?,
            None => self.begin(instrument, "", home, now)?,
        };
        self.current = Some(Current { id, instrument: instrument.to_string(), home });
        Ok(id)
    }

    /// Close `instrument`'s open setup and start its next version.
    fn begin(
        &mut self,
        instrument: &str,
        antenna: &str,
        home: (f64, f64),
        now: i64,
    ) -> rusqlite::Result<i64> {
        let tx = self.db.unchecked_transaction()?;
        tx.execute(
            "UPDATE setup SET closed = 1 WHERE instrument = ?1 AND closed = 0",
            params![instrument],
        )?;
        let version: i64 = tx.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM setup WHERE instrument = ?1",
            params![instrument],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO setup (instrument, version, antenna, home_lat, home_lon, started_ms, ended_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![instrument, version, antenna.trim(), home.0, home.1, now * 1000],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    /// Start a new version by hand: a new antenna, a new place on the roof,
    /// a new case. What was gathered for the old one is written first and
    /// kept.
    pub fn new_setup(
        &mut self,
        instrument: &str,
        antenna: &str,
        home: (f64, f64),
        now: i64,
    ) -> rusqlite::Result<i64> {
        self.flush(now)?;
        let id = self.begin(instrument, antenna, home, now)?;
        self.current = Some(Current { id, instrument: instrument.to_string(), home });
        Ok(id)
    }

    /// Give a setup its antenna label after the fact.
    pub fn set_antenna(&mut self, id: i64, antenna: &str) -> rusqlite::Result<bool> {
        Ok(self
            .db
            .execute("UPDATE setup SET antenna = ?2 WHERE id = ?1", params![id, antenna.trim()])?
            > 0)
    }

    /// Write what has been gathered, then keep the file under its limit.
    pub fn flush(&mut self, now: i64) -> rusqlite::Result<()> {
        self.flushed_at = now;
        if self.pending.is_empty() {
            return Ok(());
        }
        let tx = self.db.unchecked_transaction()?;
        {
            let mut up = tx.prepare_cached(
                "INSERT INTO cell (setup, day, idx, n, rssi_sum, rssi_max)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(setup, day, idx) DO UPDATE SET
                     n = n + excluded.n,
                     rssi_sum = rssi_sum + excluded.rssi_sum,
                     rssi_max = MAX(rssi_max, excluded.rssi_max)",
            )?;
            let mut touched = std::collections::BTreeSet::new();
            for ((setup, day, idx), c) in self.pending.drain() {
                // A setup deleted while its cells were gathering takes them
                // with it.
                let exists: bool = tx
                    .query_row("SELECT 1 FROM setup WHERE id = ?1", params![setup], |_| Ok(true))
                    .optional()?
                    .unwrap_or(false);
                if !exists {
                    continue;
                }
                up.execute(params![setup, day, idx, c.n, c.rssi_sum, c.rssi_max])?;
                touched.insert(setup);
            }
            for setup in touched {
                tx.execute(
                    "UPDATE setup SET ended_ms = MAX(ended_ms, ?2) WHERE id = ?1",
                    params![setup, now * 1000],
                )?;
            }
        }
        tx.commit()?;
        self.trim((self.max_mb() * 1e6) as u64)?;
        Ok(())
    }

    /// Drop the oldest days, across every setup, until the file is under
    /// `cap` bytes. The setups themselves stay listed: the history of what
    /// was set up when outlives the cells. Returns the days dropped.
    fn trim(&mut self, cap: u64) -> rusqlite::Result<usize> {
        let mut dropped = 0;
        self.db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        while self.file_bytes() > cap {
            let n =
                self.db.execute("DELETE FROM cell WHERE day = (SELECT MIN(day) FROM cell)", [])?;
            if n == 0 {
                break;
            }
            dropped += 1;
            self.give_back()?;
        }
        Ok(dropped)
    }

    /// Hand freed pages back to the file system and fold the write-ahead log
    /// in, so [`Self::file_bytes`] reads the size that is left.
    fn give_back(&self) -> rusqlite::Result<()> {
        // Stepped to the end: the pragma frees one page per step, and a
        // batch steps it only once.
        let mut st = self.db.prepare("PRAGMA incremental_vacuum")?;
        let mut rows = st.query([])?;
        while rows.next()?.is_some() {}
        self.db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
    }

    /// Every setup, newest first.
    pub fn setups(&self) -> rusqlite::Result<Vec<SetupInfo>> {
        let mut st = self.db.prepare(&format!(
            "SELECT s.id, s.instrument, s.version, s.antenna, s.home_lat, s.home_lon,
                    s.started_ms, s.ended_ms, s.closed,
                    COUNT(DISTINCT c.day), COALESCE(SUM(c.n), 0), COUNT(c.idx),
                    COALESCE(MIN(c.day), ''), COALESCE(MAX(c.day), ''),
                    COALESCE(MAX(CASE WHEN c.n >= {BOUNDARY_MIN}
                                      THEN (c.idx % {RINGS} + 1) * {RING_KM} END), 0)
             FROM setup s LEFT JOIN cell c ON c.setup = s.id
             GROUP BY s.id ORDER BY s.started_ms DESC, s.id DESC"
        ))?;
        st.query_map([], |r| {
            Ok(SetupInfo {
                id: r.get(0)?,
                instrument: r.get(1)?,
                version: r.get(2)?,
                antenna: r.get(3)?,
                home: (r.get(4)?, r.get(5)?),
                started_ms: r.get(6)?,
                ended_ms: r.get(7)?,
                closed: r.get::<_, i64>(8)? != 0,
                days: r.get(9)?,
                positions: r.get(10)?,
                bytes: r.get::<_, i64>(11)? * ROW_BYTES,
                first_day: r.get(12)?,
                last_day: r.get(13)?,
                max_km: r.get(14)?,
            })
        })?
        .collect()
    }

    /// A setup's log, newest day first.
    pub fn days(&self, setup: i64) -> rusqlite::Result<Vec<DayRange>> {
        let mut st = self
            .db
            .prepare("SELECT day, idx, n FROM cell WHERE setup = ?1 ORDER BY day DESC, idx")?;
        let rows = st.query_map(params![setup], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        })?;
        let mut out: Vec<DayRange> = Vec::new();
        let mut rings: Vec<Option<usize>> = vec![None; SECTORS];
        let mut positions = 0;
        let mut day = String::new();
        let close = |day: &str, positions: i64, rings: &[Option<usize>]| -> DayRange {
            let mut km: Vec<(f64, f64)> = rings
                .iter()
                .enumerate()
                .filter_map(|(s, r)| {
                    r.map(|r| ((r + 1) as f64 * RING_KM, s as f64 * 360.0 / SECTORS as f64))
                })
                .collect();
            km.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (max_km, max_bearing) = km.last().copied().unwrap_or((0.0, 0.0));
            DayRange {
                day: day.to_string(),
                positions,
                sectors: km.len(),
                median_km: if km.is_empty() { 0.0 } else { km[km.len() / 2].0 },
                max_km,
                max_bearing,
            }
        };
        for r in rows {
            let (d, idx, n) = r?;
            if d != day {
                if !day.is_empty() {
                    out.push(close(&day, positions, &rings));
                }
                day = d;
                rings.iter_mut().for_each(|r| *r = None);
                positions = 0;
            }
            positions += n;
            let (s, ring) = (idx as usize / RINGS, idx as usize % RINGS);
            if n >= i64::from(BOUNDARY_MIN) && s < SECTORS {
                rings[s] = Some(rings[s].map_or(ring, |r| r.max(ring)));
            }
        }
        if !day.is_empty() {
            out.push(close(&day, positions, &rings));
        }
        Ok(out)
    }

    /// Delete one setup and its days. Deleting the open one starts the
    /// instrument's next version with the next table.
    pub fn delete(&mut self, setup: i64) -> rusqlite::Result<bool> {
        self.pending.retain(|k, _| k.0 != setup);
        if self.current.as_ref().is_some_and(|c| c.id == setup) {
            self.current = None;
        }
        let tx = self.db.unchecked_transaction()?;
        tx.execute("DELETE FROM cell WHERE setup = ?1", params![setup])?;
        let n = tx.execute("DELETE FROM setup WHERE id = ?1", params![setup])?;
        tx.commit()?;
        self.give_back()?;
        Ok(n > 0)
    }

    /// Delete every setup and give the space back. The size limit stays: it
    /// is a setting, not a measurement.
    pub fn init(&mut self) -> rusqlite::Result<()> {
        self.pending.clear();
        self.current = None;
        self.last_pos.clear();
        self.db.execute_batch(
            "DELETE FROM cell;
             DELETE FROM setup;
             VACUUM;
             PRAGMA wal_checkpoint(TRUNCATE);",
        )
    }

    /// The file's size on disk, write-ahead log included.
    pub fn file_bytes(&self) -> u64 {
        let size = |p: &Path| std::fs::metadata(p).map_or(0, |m| m.len());
        let mut wal = self.path.clone().into_os_string();
        wal.push("-wal");
        size(&self.path) + size(Path::new(&wal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(tag: &str) -> (PathBuf, RangeLog) {
        let dir = std::env::temp_dir().join(format!(
            "sdroxide-range-test-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let log = RangeLog::open(&dir.join(FILE_NAME)).unwrap();
        (dir, log)
    }

    const HOME: (f64, f64) = (51.5, 7.5);

    fn plane(icao: u32, pos: (f64, f64), t: i64) -> AdsbAircraft {
        let mut a = AdsbAircraft::new(icao, t);
        a.lat = Some(pos.0);
        a.lon = Some(pos.1);
        a.last_pos_at = t;
        a.rssi_dbfs = -20.0;
        a
    }

    /// An aircraft flying away from home on `bearing`, one position every
    /// five seconds from `km0` out to `km1`, fed to the log as tables.
    fn fly(log: &mut RangeLog, inst: &str, icao: u32, bearing: f64, km0: f64, km1: f64, t0: i64) {
        let mut km = km0;
        let mut t = t0;
        while km <= km1 {
            let pos = crate::adsb_coverage::destination(HOME, bearing, km);
            log.note(inst, Some(HOME), &[plane(icao, pos, t)], t).unwrap();
            km += 1.0;
            t += 5;
        }
    }

    /// Noon on 18 November 2026, UTC: midday local in Europe as anywhere
    /// else, so the local day is the same day.
    const T0: i64 = 1_795_003_200;

    #[test]
    fn tests_write_only_their_own_file() {
        let (dir, log) = temp_db("own");
        assert!(log.path().starts_with(&dir));
        std::fs::remove_dir_all(dir).ok();
    }

    /// A track out to 85 km on bearing 92.5 gives that sector a range of 80 to
    /// 90 km; one stray position far out does not count.
    #[test]
    fn a_continuous_track_sets_the_days_range_and_a_stray_does_not() {
        let (dir, mut log) = temp_db("range");
        fly(&mut log, "RSP1A", 0xABC, 92.5, 5.0, 85.0, T0);
        // One position 300 km out, nothing before it: a lucky decode.
        let far = crate::adsb_coverage::destination(HOME, 180.0, 300.0);
        log.note("RSP1A", Some(HOME), &[plane(0xDEF, far, T0 + 1000)], T0 + 1000).unwrap();
        log.flush(T0 + 2000).unwrap();
        let s = log.setups().unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].instrument.as_str(), s[0].version), ("RSP1A", 1));
        assert!((s[0].max_km - 90.0).abs() < 1e-9, "{}", s[0].max_km);
        let d = log.days(s[0].id).unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].sectors, 1, "only the sector the track flew");
        assert!((d[0].max_km - 90.0).abs() < 1e-9, "{:?}", d[0]);
        assert!((d[0].max_bearing - 90.0).abs() < 1e-9);
        // 81 positions, the first of which has nothing before it.
        assert_eq!(d[0].positions, 80);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Two instruments keep two setups, and going back to the first carries
    /// on in its setup instead of starting a version.
    #[test]
    fn each_instrument_has_its_own_setup_and_comes_back_to_it() {
        let (dir, mut log) = temp_db("inst");
        fly(&mut log, "RSP1A", 1, 2.5, 5.0, 30.0, T0);
        let a = log.current().unwrap();
        fly(&mut log, "Beast 192.168.178.127:30005", 2, 2.5, 5.0, 60.0, T0 + 500);
        let b = log.current().unwrap();
        assert_ne!(a, b);
        fly(&mut log, "RSP1A", 3, 47.5, 5.0, 30.0, T0 + 1000);
        assert_eq!(log.current(), Some(a), "back to the RSP1A's open setup");
        log.flush(T0 + 2000).unwrap();
        let s = log.setups().unwrap();
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|s| s.version == 1 && !s.closed));
        let far = |id| log.days(id).unwrap()[0].max_km;
        assert!((far(a) - 30.0).abs() < 1e-9, "{}", far(a));
        assert!((far(b) - 60.0).abs() < 1e-9, "the Pi's range stays the Pi's: {}", far(b));
        std::fs::remove_dir_all(dir).ok();
    }

    /// A new setup by hand closes the old one and keeps what it gathered; a
    /// moved home starts one by itself, carrying the antenna label.
    #[test]
    fn versions_start_by_hand_and_when_home_moves() {
        let (dir, mut log) = temp_db("version");
        fly(&mut log, "RTL-SDR", 1, 92.5, 5.0, 40.0, T0);
        let v1 = log.current().unwrap();
        let v2 = log.new_setup("RTL-SDR", "plastic case", HOME, T0 + 600).unwrap();
        fly(&mut log, "RTL-SDR", 2, 92.5, 5.0, 70.0, T0 + 700);
        assert_eq!(log.current(), Some(v2));
        let moved = (HOME.0 + 0.1, HOME.1);
        log.note("RTL-SDR", Some(moved), &[], T0 + 1200).unwrap();
        let v3 = log.current().unwrap();
        assert_ne!(v3, v2);
        log.flush(T0 + 1300).unwrap();
        let s = log.setups().unwrap();
        let by = |id| s.iter().find(|x| x.id == id).unwrap();
        assert_eq!((by(v1).version, by(v1).closed), (1, true));
        assert_eq!(
            (by(v2).version, by(v2).closed, by(v2).antenna.as_str()),
            (2, true, "plastic case")
        );
        assert_eq!(
            (by(v3).version, by(v3).closed, by(v3).antenna.as_str()),
            (3, false, "plastic case")
        );
        assert!((log.days(v1).unwrap()[0].max_km - 40.0).abs() < 1e-9, "v1 kept");
        assert!((log.days(v2).unwrap()[0].max_km - 70.0).abs() < 1e-9);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Deleting takes one setup's days; INIT takes everything and gives the
    /// space back; the size limit survives both.
    #[test]
    fn delete_and_init() {
        let (dir, mut log) = temp_db("delete");
        log.set_max_mb(20.0).unwrap();
        fly(&mut log, "A", 1, 2.5, 5.0, 300.0, T0);
        let a = log.current().unwrap();
        fly(&mut log, "B", 2, 2.5, 5.0, 300.0, T0 + 2000);
        log.flush(T0 + 4000).unwrap();
        assert!(log.delete(a).unwrap());
        let left = log.setups().unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].instrument, "B");
        assert!(log.days(a).unwrap().is_empty());
        // Deleting the open setup: the next table starts a new one.
        let b = log.current().unwrap();
        assert!(log.delete(b).unwrap());
        fly(&mut log, "B", 3, 2.5, 5.0, 20.0, T0 + 5000);
        assert_ne!(log.current(), Some(b));
        log.init().unwrap();
        assert!(log.setups().unwrap().is_empty());
        assert_eq!(log.max_mb(), 20.0);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Over the limit, the oldest days go first, across setups; the setups
    /// stay listed.
    #[test]
    fn the_size_limit_drops_the_oldest_days() {
        let (dir, mut log) = temp_db("cap");
        // Ten days, each with every cell of a full sky.
        for d in 0..10 {
            let t = T0 + d * 86_400;
            let mut pending = HashMap::new();
            let id = log.setup_for("RTL-SDR", HOME, t).unwrap();
            for idx in 0..(SECTORS * RINGS) as u16 {
                pending.insert(
                    (id, crate::adsb_log::local_day(t), idx),
                    Cell { n: 5, rssi_sum: -100.0, rssi_max: -15.0 },
                );
            }
            log.pending = pending;
            log.flush(t).unwrap();
        }
        let full = log.file_bytes();
        let id = log.current().unwrap();
        assert_eq!(log.days(id).unwrap().len(), 10);
        let dropped = log.trim(full / 2).unwrap();
        assert!(dropped >= 4, "{dropped} days dropped from {full} bytes");
        let days = log.days(id).unwrap();
        assert_eq!(days.len(), 10 - dropped);
        assert_eq!(
            days.last().unwrap().day,
            crate::adsb_log::local_day(T0 + dropped as i64 * 86_400),
            "the oldest went first"
        );
        assert!(log.file_bytes() <= full / 2);
        assert_eq!(log.setups().unwrap().len(), 1, "the setup stays listed");
        std::fs::remove_dir_all(dir).ok();
    }

    /// What is gathered reaches the file on drop, and a reopened file goes on
    /// in the open setup.
    #[test]
    fn a_reopened_file_carries_on() {
        let (dir, mut log) = temp_db("reopen");
        fly(&mut log, "KiwiSDR", 1, 272.5, 5.0, 25.0, T0);
        let id = log.current().unwrap();
        let path = log.path().to_path_buf();
        drop(log);
        let mut again = RangeLog::open(&path).unwrap();
        assert_eq!(again.days(id).unwrap().len(), 1, "written on drop");
        fly(&mut again, "KiwiSDR", 2, 272.5, 5.0, 25.0, T0 + 600);
        assert_eq!(again.current(), Some(id));
        std::fs::remove_dir_all(dir).ok();
    }

    /// The view the file offers other readers gives the range per sector.
    #[test]
    fn the_range_log_view_reads_as_a_table() {
        let (dir, mut log) = temp_db("view");
        fly(&mut log, "RSP1A", 1, 92.5, 5.0, 85.0, T0);
        log.flush(T0 + 1000).unwrap();
        let (sector, bearing, km): (i64, i64, f64) = log
            .db
            .query_row(
                "SELECT sector, bearing_deg, MAX(range_km) FROM range_log WHERE range_km IS NOT NULL",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((sector, bearing), (18, 90));
        assert!((km - 90.0).abs() < 1e-9);
        std::fs::remove_dir_all(dir).ok();
    }
}
