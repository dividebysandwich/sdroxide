//! The full-band strip (WIDE) kept on disk: a recorder that writes its rows
//! into an SQLite file, by session, and hands them back a strip at a time.
//!
//! Whatever feeds the strip is recorded — a PANO sweep from a narrow-band
//! front end, a wideband receiver's own full-band lane — because the recorder
//! sits where the strip is fed, in the client. One row is kept per interval;
//! every sweep that arrives inside it is folded in, column by column, keeping
//! the strongest level, so a carrier that came up for one sweep between two
//! rows is still in the row.
//!
//! A session is cut into strips of [`STRIP_ROWS`] rows, the height of the
//! strip on screen: loading one fills the strip exactly, at the interval it
//! was recorded with. At the default ten seconds that is twenty minutes a
//! strip.
//!
//! The file is plain SQLite. Anything that reads SQLite (DuckDB included) can
//! open it while the radio is off; the layout is the two tables made in
//! [`Recorder::open`].
// Why: HDZ 2026-11-13: "PANO scanner, vierter Reiter, DB integrieren, SQLite,
//      record to DB, load from DB, DB-Inventar … delete session"; Intervall
//      1 s bis unbegrenzt, Vorgabe 10 s; Sessions in Pakete zu 120 Zeilen;
//      Anzeige der Megabytes je Session und der ganzen DB; DB INIT mit Warnung.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use sdroxide_types::SpectrumFrame;

/// Rows per strip: the height of the full-band strip on screen.
pub const STRIP_ROWS: i64 = 120;
/// Columns per stored row: the width of the full-band strip's history.
pub const COLS: usize = 1024;
/// The shortest interval between two stored rows.
pub const MIN_INTERVAL_S: f64 = 1.0;
/// The interval a fresh database starts with.
pub const DEFAULT_INTERVAL_S: f64 = 10.0;
/// The file's name in the configuration directory.
pub const FILE_NAME: &str = "recordings.sqlite";

/// Bytes a row costs beyond its levels: time, centre, span, scale, keys.
const ROW_OVERHEAD: i64 = 64;

/// A session as the inventory lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionInfo {
    pub id: i64,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub radio: String,
    /// The lowest and highest frequency any row covered, Hz; zero before the
    /// first row.
    pub lo_hz: f64,
    pub hi_hz: f64,
    pub interval_s: f64,
    pub rows: i64,
    /// What the rows take up, levels plus bookkeeping.
    pub bytes: i64,
}

impl SessionInfo {
    pub fn strips(&self) -> i64 {
        (self.rows + STRIP_ROWS - 1) / STRIP_ROWS
    }
}

/// One strip of a session: rows `k * STRIP_ROWS ..` and their time span.
#[derive(Debug, Clone, PartialEq)]
pub struct StripInfo {
    pub k: i64,
    pub first_ms: i64,
    pub last_ms: i64,
    pub rows: i64,
}

/// One stored row, ready to be fed back to the strip.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredRow {
    pub t_ms: i64,
    pub center_hz: f64,
    pub span_hz: f64,
    pub db_floor: f32,
    pub db_ceil: f32,
    pub bins: Vec<u8>,
}

impl StoredRow {
    /// The row as a spectrum frame, numbered `seq` so the strip takes it as new.
    pub fn frame(&self, seq: u32) -> SpectrumFrame {
        SpectrumFrame {
            seq,
            center_hz: self.center_hz,
            span_hz: self.span_hz,
            db_floor: self.db_floor,
            db_ceil: self.db_ceil,
            bins: self.bins.clone(),
            rows: Vec::new(),
            rows_clocked: false,
        }
    }
}

/// One frame of the strip as it arrived, squeezed to [`COLS`] columns: what
/// the recorder keeps of the strip while nothing records, so that ● REC and
/// STORE can take along what is already on screen.
// Why: HDZ 2026-11-13: "kannst du den scan stand des panoramas bei rec nicht
//      einfach übernehmen … die hast du doch im speicher"; "speicher beim pano
//      die zeit mit, weil mit der verzögerungszeit kann das variieren".
#[derive(Debug, Clone, PartialEq)]
pub struct Sweep {
    /// When the frame arrived: for PANO, when its sweep finished.
    pub t_ms: i64,
    pub frame: SpectrumFrame,
}

impl Sweep {
    /// The frame at `t_ms`, squeezed as the strip squeezes it: the strongest
    /// bin in each column, on the frame's own scale.
    pub fn new(frame: &SpectrumFrame, t_ms: i64) -> Self {
        let n = frame.bins.len();
        let bins = if n <= COLS {
            frame.bins.clone()
        } else {
            (0..COLS)
                .map(|c| {
                    let b0 = c * n / COLS;
                    let b1 = (((c + 1) * n / COLS).max(b0 + 1)).min(n);
                    frame.bins[b0..b1].iter().copied().max().unwrap_or(0)
                })
                .collect()
        };
        Sweep {
            t_ms,
            frame: SpectrumFrame {
                seq: frame.seq,
                center_hz: frame.center_hz,
                span_hz: frame.span_hz,
                db_floor: frame.db_floor,
                db_ceil: frame.db_ceil,
                bins,
                rows: Vec::new(),
                rows_clocked: false,
            },
        }
    }
}

/// The interval being gathered into one row.
#[derive(Debug, Clone)]
struct Pending {
    started_ms: i64,
    center_hz: f64,
    span_hz: f64,
    /// Strongest level per column, dB.
    db: Vec<f32>,
    floor: f32,
    ceil: f32,
}

#[derive(Debug, Clone)]
struct Active {
    session: i64,
    rows: i64,
    interval_ms: i64,
    last_seq: Option<u32>,
    pending: Option<Pending>,
}

/// The recorder and the file behind it.
pub struct Recorder {
    db: Connection,
    path: PathBuf,
    active: Option<Active>,
}

impl Recorder {
    /// The file in the configuration directory, made on first use.
    pub fn open_default() -> Result<Self, String> {
        let dir = sdroxide_config::config_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Self::open(&dir.join(FILE_NAME)).map_err(|e| e.to_string())
    }

    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let db = Connection::open(path)?;
        // Two radio tabs recording at once share the file; one waits briefly
        // for the other's write rather than failing it.
        db.busy_timeout(std::time::Duration::from_millis(500))?;
        // Must precede the first table, or it is ignored: freed pages go back
        // to the file system after a delete instead of lingering as free
        // space, so the size shown is the size on disk.
        db.execute_batch(
            "PRAGMA auto_vacuum = INCREMENTAL;
             PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS session (
                 id         INTEGER PRIMARY KEY,
                 started_ms INTEGER NOT NULL,
                 ended_ms   INTEGER NOT NULL,
                 radio      TEXT    NOT NULL,
                 lo_hz      REAL    NOT NULL DEFAULT 0,
                 hi_hz      REAL    NOT NULL DEFAULT 0,
                 interval_s REAL    NOT NULL,
                 rows       INTEGER NOT NULL DEFAULT 0,
                 bytes      INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS row (
                 session   INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE,
                 idx       INTEGER NOT NULL,
                 t_ms      INTEGER NOT NULL,
                 center_hz REAL    NOT NULL,
                 span_hz   REAL    NOT NULL,
                 db_floor  REAL    NOT NULL,
                 db_ceil   REAL    NOT NULL,
                 bins      BLOB    NOT NULL,
                 PRIMARY KEY (session, idx)
             );",
        )?;
        // A session the app did not get to close (a crash, a kill) keeps the
        // rows it wrote; its end is the last row's time, already stored.
        Ok(Recorder { db, path: path.to_path_buf(), active: None })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The interval stored with the database, clamped to the minimum.
    pub fn interval_s(&self) -> f64 {
        self.db
            .query_row("SELECT value FROM meta WHERE key = 'interval_s'", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .ok()
            .flatten()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .map_or(DEFAULT_INTERVAL_S, |v| v.max(MIN_INTERVAL_S))
    }

    pub fn set_interval_s(&self, s: f64) -> rusqlite::Result<()> {
        let s = if s.is_finite() { s.max(MIN_INTERVAL_S) } else { DEFAULT_INTERVAL_S };
        self.db.execute(
            "INSERT INTO meta (key, value) VALUES ('interval_s', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![s.to_string()],
        )?;
        Ok(())
    }

    /// The session being recorded and its rows so far.
    pub fn active(&self) -> Option<(i64, i64)> {
        self.active.as_ref().map(|a| (a.session, a.rows))
    }

    /// Open a new session with nothing behind it. A running one is closed
    /// first. The app always starts with the strip's backlog
    /// ([`Recorder::start_with`], [`Recorder::start_per_sweep`]).
    #[cfg(test)]
    pub fn start(&mut self, radio: &str, interval_s: f64, now_ms: i64) -> rusqlite::Result<i64> {
        self.start_with(radio, interval_s, &[], now_ms)
    }

    /// Open a new session that begins with `backlog`, the sweeps already on
    /// the strip, oldest first: they are folded into rows by their own times,
    /// exactly as if the recording had run while they came in, and the
    /// session starts with the oldest. A running session is closed first.
    pub fn start_with(
        &mut self,
        radio: &str,
        interval_s: f64,
        backlog: &[Sweep],
        now_ms: i64,
    ) -> rusqlite::Result<i64> {
        self.stop(now_ms)?;
        let interval_s = if interval_s.is_finite() {
            interval_s.max(MIN_INTERVAL_S)
        } else {
            DEFAULT_INTERVAL_S
        };
        let first = backlog.first().map_or(now_ms, |s| s.t_ms.min(now_ms));
        let session = self.begin(radio, interval_s, first)?;
        for s in backlog {
            self.on_frame(&s.frame, s.t_ms)?;
        }
        self.tick(now_ms)?;
        Ok(session)
    }

    /// Keep `sweeps` as a finished session, one row per sweep at the time it
    /// arrived — nothing folded, so loading it back shows the strip as it
    /// was. The session's interval is stored as zero: "per sweep". Refused
    /// (`None`) while a session records, which takes the strip along anyway,
    /// and when there is nothing to keep.
    // Why: HDZ 2026-11-13: "bei rec alle x sekunden ist der timer ja wichtig,
    //      bei einem normalen scan wäre die zeit ja nur die sweep zeit, die so
    //      schnell wies halt kam … das ist die trennung".
    pub fn store(&mut self, radio: &str, sweeps: &[Sweep]) -> rusqlite::Result<Option<i64>> {
        if self.active.is_some() || sweeps.is_empty() {
            return Ok(None);
        }
        let last = sweeps.last().map_or(0, |s| s.t_ms);
        let session = self.start_per_sweep(radio, sweeps, last)?;
        self.stop(last)?;
        Ok(Some(session))
    }

    /// Open a session that keeps one row per sweep, as it arrives — for a
    /// panorama, whose own times (settle and pause) already space its rows —
    /// beginning with `backlog` as [`Recorder::start_with`] does. A running
    /// session is closed first.
    // Why: HDZ 2026-11-13: "die zweite zeit da weg, weil die jetzt oben mit den
    //      beiden zeiten verfrühstückt ist".
    pub fn start_per_sweep(
        &mut self,
        radio: &str,
        backlog: &[Sweep],
        now_ms: i64,
    ) -> rusqlite::Result<i64> {
        self.stop(now_ms)?;
        let first = backlog.first().map_or(now_ms, |s| s.t_ms.min(now_ms));
        let session = self.begin(radio, 0.0, first)?;
        for s in backlog {
            // Interval zero: every frame closes the row before it.
            self.on_frame(&s.frame, s.t_ms)?;
        }
        // And the last one is written at once rather than with the next sweep.
        self.tick(now_ms)?;
        Ok(session)
    }

    fn begin(&mut self, radio: &str, interval_s: f64, started_ms: i64) -> rusqlite::Result<i64> {
        self.db.execute(
            "INSERT INTO session (started_ms, ended_ms, radio, interval_s) VALUES (?1, ?1, ?2, ?3)",
            params![started_ms, radio, interval_s],
        )?;
        let session = self.db.last_insert_rowid();
        self.active = Some(Active {
            session,
            rows: 0,
            interval_ms: (interval_s * 1000.0).round() as i64,
            last_seq: None,
            pending: None,
        });
        Ok(session)
    }

    /// Close the running session, writing the interval gathered so far.
    pub fn stop(&mut self, now_ms: i64) -> rusqlite::Result<()> {
        if self.active.as_ref().is_some_and(|a| a.pending.is_some()) {
            self.flush()?;
        }
        if let Some(a) = self.active.take() {
            self.db.execute(
                "UPDATE session SET ended_ms = MAX(ended_ms, ?2) WHERE id = ?1",
                params![a.session, now_ms],
            )?;
        }
        Ok(())
    }

    /// Fold one frame of the strip in. Frames already seen are ignored.
    pub fn on_frame(&mut self, frame: &SpectrumFrame, now_ms: i64) -> rusqlite::Result<()> {
        let Some(a) = self.active.as_mut() else { return Ok(()) };
        if a.last_seq == Some(frame.seq) || frame.bins.is_empty() || frame.span_hz <= 0.0 {
            return Ok(());
        }
        a.last_seq = Some(frame.seq);
        // Due, or the window moved: what was gathered goes out first, so a
        // row never mixes two intervals or two axes.
        let flush_first = a.pending.as_ref().is_some_and(|p| {
            now_ms - p.started_ms >= a.interval_ms
                || (p.center_hz - frame.center_hz).abs() > 1.0
                || (p.span_hz - frame.span_hz).abs() > 1.0
        });
        if flush_first {
            self.flush()?;
        }
        let a = self.active.as_mut().expect("still recording");
        let levels = columns_db(frame);
        match a.pending.as_mut() {
            Some(p) => {
                for (held, new) in p.db.iter_mut().zip(levels) {
                    *held = held.max(new);
                }
                p.floor = p.floor.min(frame.db_floor);
                p.ceil = p.ceil.max(frame.db_ceil);
            }
            None => {
                a.pending = Some(Pending {
                    started_ms: now_ms,
                    center_hz: frame.center_hz,
                    span_hz: frame.span_hz,
                    db: levels,
                    floor: frame.db_floor,
                    ceil: frame.db_ceil,
                });
            }
        }
        Ok(())
    }

    /// Write the gathered row once its interval is over, frames or not.
    pub fn tick(&mut self, now_ms: i64) -> rusqlite::Result<()> {
        let due = self.active.as_ref().is_some_and(|a| {
            a.pending.as_ref().is_some_and(|p| now_ms - p.started_ms >= a.interval_ms)
        });
        if due { self.flush() } else { Ok(()) }
    }

    fn flush(&mut self) -> rusqlite::Result<()> {
        let Some(a) = self.active.as_mut() else { return Ok(()) };
        let Some(p) = a.pending.take() else { return Ok(()) };
        let bins = quantize(&p.db, p.floor, p.ceil);
        let lo = p.center_hz - p.span_hz / 2.0;
        let hi = p.center_hz + p.span_hz / 2.0;
        let bytes = bins.len() as i64 + ROW_OVERHEAD;
        let tx = self.db.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO row (session, idx, t_ms, center_hz, span_hz, db_floor, db_ceil, bins)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![a.session, a.rows, p.started_ms, p.center_hz, p.span_hz, p.floor, p.ceil, bins],
        )?;
        tx.execute(
            "UPDATE session SET
                 rows = rows + 1,
                 bytes = bytes + ?2,
                 ended_ms = MAX(ended_ms, ?3),
                 lo_hz = CASE WHEN rows = 0 THEN ?4 ELSE MIN(lo_hz, ?4) END,
                 hi_hz = CASE WHEN rows = 0 THEN ?5 ELSE MAX(hi_hz, ?5) END
             WHERE id = ?1",
            params![a.session, bytes, p.started_ms + a.interval_ms, lo, hi],
        )?;
        tx.commit()?;
        a.rows += 1;
        Ok(())
    }

    /// Every session, newest first.
    pub fn sessions(&self) -> rusqlite::Result<Vec<SessionInfo>> {
        let mut st = self.db.prepare(
            "SELECT id, started_ms, ended_ms, radio, lo_hz, hi_hz, interval_s, rows, bytes
             FROM session ORDER BY started_ms DESC, id DESC",
        )?;
        st.query_map([], |r| {
            Ok(SessionInfo {
                id: r.get(0)?,
                started_ms: r.get(1)?,
                ended_ms: r.get(2)?,
                radio: r.get(3)?,
                lo_hz: r.get(4)?,
                hi_hz: r.get(5)?,
                interval_s: r.get(6)?,
                rows: r.get(7)?,
                bytes: r.get(8)?,
            })
        })?
        .collect()
    }

    /// A session's strips, oldest first.
    pub fn strips(&self, session: i64) -> rusqlite::Result<Vec<StripInfo>> {
        let mut st = self.db.prepare(
            "SELECT idx / ?2 AS k, MIN(t_ms), MAX(t_ms), COUNT(*)
             FROM row WHERE session = ?1 GROUP BY k ORDER BY k",
        )?;
        st.query_map(params![session, STRIP_ROWS], |r| {
            Ok(StripInfo { k: r.get(0)?, first_ms: r.get(1)?, last_ms: r.get(2)?, rows: r.get(3)? })
        })?
        .collect()
    }

    /// Strip `k` of a session, oldest row first.
    pub fn strip_rows(&self, session: i64, k: i64) -> rusqlite::Result<Vec<StoredRow>> {
        let mut st = self.db.prepare(
            "SELECT t_ms, center_hz, span_hz, db_floor, db_ceil, bins
             FROM row WHERE session = ?1 AND idx >= ?2 AND idx < ?3 ORDER BY idx",
        )?;
        st.query_map(params![session, k * STRIP_ROWS, (k + 1) * STRIP_ROWS], |r| {
            Ok(StoredRow {
                t_ms: r.get(0)?,
                center_hz: r.get(1)?,
                span_hz: r.get(2)?,
                db_floor: r.get(3)?,
                db_ceil: r.get(4)?,
                bins: r.get(5)?,
            })
        })?
        .collect()
    }

    /// Delete one session and its rows. The running one is refused.
    pub fn delete(&mut self, session: i64) -> rusqlite::Result<bool> {
        if self.active.as_ref().is_some_and(|a| a.session == session) {
            return Ok(false);
        }
        let tx = self.db.unchecked_transaction()?;
        tx.execute("DELETE FROM row WHERE session = ?1", params![session])?;
        let n = tx.execute("DELETE FROM session WHERE id = ?1", params![session])?;
        tx.commit()?;
        self.db.execute_batch("PRAGMA incremental_vacuum;")?;
        Ok(n > 0)
    }

    /// Delete every session and give the space back. Refused while recording.
    pub fn init(&mut self) -> rusqlite::Result<bool> {
        if self.active.is_some() {
            return Ok(false);
        }
        self.db.execute_batch(
            "DELETE FROM row;
             DELETE FROM session;
             VACUUM;
             PRAGMA wal_checkpoint(TRUNCATE);",
        )?;
        Ok(true)
    }

    /// The database's size on disk, write-ahead log included.
    pub fn file_bytes(&self) -> u64 {
        let size = |p: &Path| std::fs::metadata(p).map_or(0, |m| m.len());
        let mut wal = self.path.clone().into_os_string();
        wal.push("-wal");
        size(&self.path) + size(Path::new(&wal))
    }
}

/// A frame's levels in dB, resampled to [`COLS`] columns keeping the strongest
/// bin in each — the same squeeze the strip itself applies.
fn columns_db(frame: &SpectrumFrame) -> Vec<f32> {
    let n = frame.bins.len().max(1);
    let scale = (frame.db_ceil - frame.db_floor) / 255.0;
    (0..COLS)
        .map(|c| {
            let b0 = c * n / COLS;
            let b1 = (((c + 1) * n / COLS).max(b0 + 1)).min(n);
            let v = frame.bins.get(b0..b1).and_then(|s| s.iter().copied().max()).unwrap_or(0);
            frame.db_floor + f32::from(v) * scale
        })
        .collect()
}

fn quantize(db: &[f32], floor: f32, ceil: f32) -> Vec<u8> {
    let range = ceil - floor;
    db.iter()
        .map(|v| {
            if range > 0.0 {
                ((v - floor) / range * 255.0).round().clamp(0.0, 255.0) as u8
            } else {
                0
            }
        })
        .collect()
}

/// A size as shown: kilobytes below a tenth of a megabyte, where one decimal
/// of a megabyte would read 0.0, megabytes with one decimal above.
pub fn mb(bytes: u64) -> String {
    if bytes < 100_000 {
        format!("{} kB", bytes.div_ceil(1000))
    } else {
        format!("{:.1} MB", bytes as f64 / 1e6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(tag: &str) -> (PathBuf, Recorder) {
        let dir = std::env::temp_dir().join(format!(
            "sdroxide-rec-test-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        let rec = Recorder::open(&path).unwrap();
        (dir, rec)
    }

    fn frame(seq: u32, bins: Vec<u8>, floor: f32, ceil: f32) -> SpectrumFrame {
        SpectrumFrame {
            seq,
            center_hz: 8.5e6,
            span_hz: 15e6,
            db_floor: floor,
            db_ceil: ceil,
            bins,
            rows: Vec::new(),
            rows_clocked: false,
        }
    }

    /// The sandbox never reaches the operator's configuration.
    #[test]
    fn tests_write_only_their_own_file() {
        let (dir, rec) = temp_db("path");
        assert!(rec.path().starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(dir).ok();
    }

    /// Sweeps inside one interval become one row holding each column's
    /// strongest level; the next interval starts a new row.
    #[test]
    fn one_row_per_interval_keeps_the_strongest_level() {
        let (dir, mut rec) = temp_db("hold");
        rec.start("RTL-SDR", 10.0, 0).unwrap();
        let mut a = vec![0u8; COLS];
        a[100] = 200;
        let mut b = vec![0u8; COLS];
        b[500] = 255;
        rec.on_frame(&frame(1, a, -120.0, -20.0), 0).unwrap();
        rec.on_frame(&frame(2, b, -120.0, -20.0), 4_000).unwrap();
        assert_eq!(rec.active(), Some((1, 0)), "nothing written inside the interval");
        rec.on_frame(&frame(3, vec![0; COLS], -120.0, -20.0), 10_000).unwrap();
        assert_eq!(rec.active(), Some((1, 1)));
        let rows = rec.strip_rows(1, 0).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bins[100], 200, "first sweep's carrier kept");
        assert_eq!(rows[0].bins[500], 255, "second sweep's carrier kept");
        assert_eq!(rows[0].t_ms, 0);
        // Gegenprobe: the third sweep is in the pending row, not this one.
        rec.stop(12_000).unwrap();
        assert_eq!(rec.strip_rows(1, 0).unwrap().len(), 2, "stop writes what was gathered");
        std::fs::remove_dir_all(dir).ok();
    }

    fn sweep(seq: u32, t_ms: i64, col: usize, level: u8) -> Sweep {
        let mut bins = vec![0u8; COLS];
        bins[col] = level;
        Sweep::new(&frame(seq, bins, -120.0, -20.0), t_ms)
    }

    /// ● REC on a strip that already holds sweeps: they go into the session
    /// by their own times — PANO's sweeps come at uneven times — and the
    /// session starts with the oldest; live frames carry on from there.
    #[test]
    fn rec_takes_the_strip_along_by_its_own_times() {
        let (dir, mut rec) = temp_db("backlog");
        // Uneven sweep times, as a dwell delay makes them: 0, 7, 19, 23 s.
        let backlog = [
            sweep(1, 0, 10, 200),
            sweep(2, 7_000, 20, 210),
            sweep(3, 19_000, 30, 220),
            sweep(4, 23_000, 40, 230),
        ];
        let id = rec.start_with("RTL-SDR", 10.0, &backlog, 25_000).unwrap();
        // Rows: [0, 10) holds sweeps 1 and 2; [19, 29) holds 3 and 4, still open.
        assert_eq!(rec.active(), Some((id, 1)));
        let s = &rec.sessions().unwrap()[0];
        assert_eq!(s.started_ms, 0, "session starts with the oldest sweep");
        let rows = rec.strip_rows(id, 0).unwrap();
        assert_eq!((rows[0].t_ms, rows[0].bins[10], rows[0].bins[20]), (0, 200, 210));
        // Gegenprobe: sweep 3 is not in the first row.
        assert_eq!(rows[0].bins[30], 0);
        // A live frame inside the open interval joins the backlog's last row.
        let mut live = vec![0u8; COLS];
        live[50] = 240;
        rec.on_frame(&frame(5, live, -120.0, -20.0), 27_000).unwrap();
        rec.stop(28_000).unwrap();
        let rows = rec.strip_rows(id, 0).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[1].t_ms, rows[1].bins[30], rows[1].bins[40], rows[1].bins[50]),
            (19_000, 220, 230, 240)
        );
        std::fs::remove_dir_all(dir).ok();
    }

    /// A backlog whose last interval is over by the time REC is pressed is
    /// written at once, not with the next live frame.
    #[test]
    fn rec_writes_a_backlog_row_that_is_already_due() {
        let (dir, mut rec) = temp_db("due");
        let id = rec.start_with("RTL-SDR", 10.0, &[sweep(1, 0, 5, 100)], 60_000).unwrap();
        assert_eq!(rec.active(), Some((id, 1)));
        std::fs::remove_dir_all(dir).ok();
    }

    /// STORE keeps every sweep as its own row at its own time, nothing
    /// folded; the session says "per sweep" with an interval of zero.
    #[test]
    fn store_keeps_one_row_per_sweep() {
        let (dir, mut rec) = temp_db("store");
        let sweeps = [sweep(1, 1_000, 10, 200), sweep(2, 1_400, 20, 210), sweep(3, 9_100, 30, 220)];
        let id = rec.store("RTL-SDR", &sweeps).unwrap().expect("stored");
        assert_eq!(rec.active(), None, "nothing left recording");
        let rows = rec.strip_rows(id, 0).unwrap();
        assert_eq!(rows.iter().map(|r| r.t_ms).collect::<Vec<_>>(), vec![1_000, 1_400, 9_100]);
        // Gegenprobe: each row holds only its own sweep's carrier.
        assert_eq!((rows[0].bins[10], rows[0].bins[20]), (200, 0));
        assert_eq!((rows[1].bins[20], rows[1].bins[30]), (210, 0));
        let s = &rec.sessions().unwrap()[0];
        assert_eq!((s.started_ms, s.ended_ms, s.interval_s, s.rows), (1_000, 9_100, 0.0, 3));
        std::fs::remove_dir_all(dir).ok();
    }

    /// REC on a panorama: one row per sweep from the backlog on, each written
    /// as it arrives, and live sweeps carry on the same way.
    #[test]
    fn rec_per_sweep_writes_every_sweep_as_it_comes() {
        let (dir, mut rec) = temp_db("per-sweep");
        let backlog = [sweep(1, 0, 10, 200), sweep(2, 12_500, 20, 210)];
        let id = rec.start_per_sweep("RTL-SDR", &backlog, 13_000).unwrap();
        assert_eq!(rec.active(), Some((id, 2)), "both backlog sweeps written at once");
        rec.on_frame(&sweep(3, 25_100, 30, 220).frame, 25_100).unwrap();
        rec.tick(25_120).unwrap();
        assert_eq!(rec.active(), Some((id, 3)), "a live sweep is written on the next pass");
        let rows = rec.strip_rows(id, 0).unwrap();
        assert_eq!(rows.iter().map(|r| r.t_ms).collect::<Vec<_>>(), vec![0, 12_500, 25_100]);
        assert_eq!((rows[1].bins[10], rows[1].bins[20]), (0, 210), "nothing folded");
        assert_eq!(rec.sessions().unwrap()[0].interval_s, 0.0);
        std::fs::remove_dir_all(dir).ok();
    }

    /// STORE leaves a running recording alone, and an empty strip stores
    /// nothing.
    #[test]
    fn store_is_refused_while_recording_or_empty() {
        let (dir, mut rec) = temp_db("store-refused");
        assert_eq!(rec.store("RTL-SDR", &[]).unwrap(), None);
        let id = rec.start("RTL-SDR", 10.0, 0).unwrap();
        assert_eq!(rec.store("RTL-SDR", &[sweep(1, 0, 1, 1)]).unwrap(), None);
        assert_eq!(rec.active(), Some((id, 0)), "the recording runs on");
        assert_eq!(rec.sessions().unwrap().len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    /// A wide frame is squeezed to the stored width keeping each column's
    /// strongest bin; a narrow one is kept as it is.
    #[test]
    fn sweeps_are_squeezed_like_the_strip() {
        let mut bins = vec![0u8; COLS * 4];
        bins[4 * 300 + 3] = 250;
        let s = Sweep::new(&frame(1, bins, -120.0, -20.0), 0);
        assert_eq!(s.frame.bins.len(), COLS);
        assert_eq!((s.frame.bins[299], s.frame.bins[300], s.frame.bins[301]), (0, 250, 0));
        let narrow = Sweep::new(&frame(2, vec![7; 300], -120.0, -20.0), 0);
        assert_eq!(narrow.frame.bins, vec![7; 300]);
    }

    #[test]
    fn small_sizes_read_in_kilobytes() {
        assert_eq!(mb(0), "0 kB");
        assert_eq!(mb(5_440), "6 kB");
        assert_eq!(mb(99_999), "100 kB");
        assert_eq!(mb(100_000), "0.1 MB");
        assert_eq!(mb(31_000_000), "31.0 MB");
    }

    /// Rows scaled differently are compared in dB, not in bytes.
    #[test]
    fn differently_scaled_sweeps_merge_in_db() {
        let (dir, mut rec) = temp_db("scale");
        rec.start("RTL-SDR", 10.0, 0).unwrap();
        // −60 dB in a −120…−20 window is byte 153; in a −80…−40 window, 128.
        let mut a = vec![0u8; COLS];
        a[7] = 153;
        let mut b = vec![0u8; COLS];
        b[7] = 64; // −70 dB in −80…−40
        rec.on_frame(&frame(1, a, -120.0, -20.0), 0).unwrap();
        rec.on_frame(&frame(2, b, -80.0, -40.0), 1_000).unwrap();
        rec.stop(2_000).unwrap();
        let r = &rec.strip_rows(1, 0).unwrap()[0];
        assert_eq!((r.db_floor, r.db_ceil), (-120.0, -20.0));
        let db = r.db_floor + f32::from(r.bins[7]) * (r.db_ceil - r.db_floor) / 255.0;
        assert!((db + 60.0).abs() < 0.5, "kept −60 dB, got {db}");
        std::fs::remove_dir_all(dir).ok();
    }

    /// The interval never goes below a second, and a repeated frame is not a
    /// new sweep.
    #[test]
    fn interval_floor_and_repeated_frames() {
        let (dir, mut rec) = temp_db("floor");
        rec.start("RX-888", 0.1, 0).unwrap();
        for (i, t) in [0, 300, 600, 900].into_iter().enumerate() {
            rec.on_frame(&frame(i as u32 + 1, vec![9; COLS], -120.0, -20.0), t).unwrap();
        }
        assert_eq!(rec.active(), Some((1, 0)), "under a second: still one row gathering");
        rec.on_frame(&frame(4, vec![9; COLS], -120.0, -20.0), 1_000).unwrap();
        assert_eq!(rec.active(), Some((1, 0)), "same seq is the same frame");
        rec.on_frame(&frame(5, vec![9; COLS], -120.0, -20.0), 1_000).unwrap();
        assert_eq!(rec.active(), Some((1, 1)));
        assert_eq!(rec.sessions().unwrap()[0].interval_s, 1.0);
        rec.set_interval_s(0.2).unwrap();
        assert_eq!(rec.interval_s(), 1.0);
        rec.set_interval_s(3600.0).unwrap();
        assert_eq!(rec.interval_s(), 3600.0);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Ticking writes a due row without waiting for the next sweep.
    #[test]
    fn tick_writes_a_due_row() {
        let (dir, mut rec) = temp_db("tick");
        rec.start("RTL-SDR", 10.0, 0).unwrap();
        rec.on_frame(&frame(1, vec![9; COLS], -120.0, -20.0), 0).unwrap();
        rec.tick(9_999).unwrap();
        assert_eq!(rec.active(), Some((1, 0)));
        rec.tick(10_000).unwrap();
        assert_eq!(rec.active(), Some((1, 1)));
        std::fs::remove_dir_all(dir).ok();
    }

    /// A moved window closes the row early rather than mixing two axes.
    #[test]
    fn a_new_range_starts_a_new_row() {
        let (dir, mut rec) = temp_db("range");
        rec.start("RTL-SDR", 10.0, 0).unwrap();
        rec.on_frame(&frame(1, vec![9; COLS], -120.0, -20.0), 0).unwrap();
        let mut moved = frame(2, vec![9; COLS], -120.0, -20.0);
        moved.center_hz = 7.1e6;
        moved.span_hz = 0.2e6;
        rec.on_frame(&moved, 2_000).unwrap();
        assert_eq!(rec.active(), Some((1, 1)));
        rec.stop(3_000).unwrap();
        let s = &rec.sessions().unwrap()[0];
        assert_eq!((s.lo_hz, s.hi_hz), (1e6, 16e6), "range spans both windows");
        std::fs::remove_dir_all(dir).ok();
    }

    /// 250 rows are three strips: 120, 120 and 10, each handed back in order.
    #[test]
    fn sessions_are_cut_into_strips_of_120() {
        let (dir, mut rec) = temp_db("strips");
        rec.start("RX-888", 1.0, 0).unwrap();
        for i in 0..251u32 {
            rec.on_frame(
                &frame(i + 1, vec![(i % 256) as u8; COLS], -120.0, -20.0),
                i64::from(i) * 1000,
            )
            .unwrap();
        }
        assert_eq!(rec.active(), Some((1, 250)));
        rec.stop(251_000).unwrap();
        let s = rec.sessions().unwrap();
        assert_eq!(s[0].rows, 251);
        assert_eq!(s[0].strips(), 3);
        assert_eq!(s[0].bytes, 251 * (COLS as i64 + ROW_OVERHEAD));
        let strips = rec.strips(1).unwrap();
        let counts: Vec<i64> = strips.iter().map(|s| s.rows).collect();
        assert_eq!(counts, vec![120, 120, 11]);
        assert_eq!((strips[1].first_ms, strips[1].last_ms), (120_000, 239_000));
        let rows = rec.strip_rows(1, 1).unwrap();
        assert_eq!(rows.len(), 120);
        assert_eq!(rows[0].bins[0], 120);
        assert_eq!(rows[119].bins[0], 239);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Deleting takes one session's rows; INIT takes everything and gives the
    /// space back. Neither touches the session being recorded.
    #[test]
    fn delete_and_init() {
        let (dir, mut rec) = temp_db("delete");
        for s in 0..2 {
            rec.start("RTL-SDR", 1.0, s * 1_000_000).unwrap();
            for i in 0..400u32 {
                rec.on_frame(
                    &frame(i + 1, vec![77; COLS], -120.0, -20.0),
                    s * 1_000_000 + i64::from(i) * 1000,
                )
                .unwrap();
            }
            rec.stop(s * 1_000_000 + 500_000).unwrap();
        }
        let full = rec.file_bytes();
        assert!(full > 400 * COLS as u64, "{full}");
        assert!(rec.delete(1).unwrap());
        let left = rec.sessions().unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, 2);
        assert!(rec.strip_rows(1, 0).unwrap().is_empty());

        rec.start("RTL-SDR", 1.0, 9_000_000).unwrap();
        assert!(!rec.delete(3).unwrap(), "the running session stays");
        assert!(!rec.init().unwrap(), "no INIT while recording");
        rec.stop(9_000_001).unwrap();

        assert!(rec.init().unwrap());
        assert!(rec.sessions().unwrap().is_empty());
        let empty = rec.file_bytes();
        assert!(empty < full / 4, "space given back: {empty} of {full}");
        // The interval survives an INIT: it is a setting, not a recording.
        std::fs::remove_dir_all(dir).ok();
    }

    /// A reopened file still lists what was recorded.
    #[test]
    fn sessions_survive_reopening() {
        let (dir, mut rec) = temp_db("reopen");
        rec.start("KiwiSDR", 10.0, 0).unwrap();
        rec.on_frame(&frame(1, vec![1; COLS], -120.0, -20.0), 0).unwrap();
        rec.stop(5_000).unwrap();
        let path = rec.path().to_path_buf();
        drop(rec);
        let again = Recorder::open(&path).unwrap();
        let s = again.sessions().unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].radio, "KiwiSDR");
        assert_eq!(s[0].rows, 1);
        assert_eq!(s[0].ended_ms, 10_000, "end is the last row's interval end");
        std::fs::remove_dir_all(dir).ok();
    }
}
