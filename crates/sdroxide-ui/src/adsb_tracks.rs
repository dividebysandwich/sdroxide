//! Flight tracks: where each aircraft went, thinned, one CSV file per day.
//!
//! The sighting log keeps one line per aircraft and day; this keeps the path.
//! Every new position is considered, and a line is written when
//! [`EVERY_S`] have passed since the last kept point, or the track has turned
//! more than [`TURN_DEG`], or the altitude has moved more than [`CLIMB_FT`]
//! since then. A straight cruise is one point in fifteen seconds; a turn or a
//! climb keeps its shape. The day's lines are appended to
//! `adsb-tracks/<date>.csv` in the config folder once a minute, and files older
//! than [`crate::adsb_log::KEEP_DAYS`] are removed at start.
//!
//! Why: HDZ 2026-11-17: Flugspuren „ausgedünnt“, „alle 15 Sekunden“. Kept apart
//! from the sighting log so the log stays one readable line per aircraft; a
//! busy day is about one to three megabytes here.

use std::collections::HashMap;
use std::fmt::Write as _;

use sdroxide_types::AdsbAircraft;

use crate::adsb_log::{KEEP_DAYS, local_day, local_stamp};

/// A straight track keeps one point in this many seconds.
pub const EVERY_S: i64 = 15;
/// A turn of more than this since the last kept point keeps a point.
pub const TURN_DEG: f32 = 5.0;
/// A climb or descent of more than this since the last kept point keeps one.
pub const CLIMB_FT: i32 = 500;

const HEAD: &str = "icao,callsign,t,t_local,lat,lon,alt_ft,gs_kt,trk_deg";

/// How long the kept points stay in memory for the map's altitude trail (chip
/// ALT), seconds.
pub const RECENT_S: i64 = 600;

use crate::adsb_map::{Recent, TrailPoint};

/// The last point written for an aircraft: what the next one is measured from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Kept {
    t: i64,
    alt_ft: Option<i32>,
    trk_deg: Option<f32>,
}

/// Whether a new position at `t` is worth a line, against the last kept one.
fn keep(prev: Option<Kept>, t: i64, alt_ft: Option<i32>, trk_deg: Option<f32>) -> bool {
    let Some(p) = prev else { return true };
    if t - p.t >= EVERY_S {
        return true;
    }
    if let (Some(a), Some(b)) = (p.trk_deg, trk_deg) {
        // Shortest way round: 355° to 5° is a turn of ten degrees.
        if ((b - a + 540.0).rem_euclid(360.0) - 180.0).abs() > TURN_DEG {
            return true;
        }
    }
    matches!((p.alt_ft, alt_ft), (Some(a), Some(b)) if (b - a).abs() > CLIMB_FT)
}

/// One line of the day file.
fn line(a: &AdsbAircraft, t: i64, lat: f64, lon: f64) -> String {
    let opt = |v: Option<String>| v.unwrap_or_default();
    format!(
        "{:06X},{},{},{},{:.5},{:.5},{},{},{}\n",
        a.icao,
        a.callsign.trim(),
        t,
        local_stamp(t),
        lat,
        lon,
        opt(a.altitude_ft.map(|v| v.to_string())),
        opt(a.ground_speed_kt.map(|v| format!("{v:.0}"))),
        opt(a.track_deg.map(|v| format!("{v:.0}"))),
    )
}

/// The day's tracks, written as they grow.
#[derive(Default)]
pub struct TrackBook {
    /// The local day the pending lines belong to.
    day: String,
    /// Last kept point per aircraft.
    kept: HashMap<u32, Kept>,
    /// Last position time seen per aircraft, so a snapshot repeated twice a
    /// second is one position, not two.
    seen: HashMap<u32, i64>,
    /// Lines not yet on disk.
    pending: String,
    started: bool,
    saved_at: i64,
    /// The points written in the last [`RECENT_S`], for the map.
    recent: Recent,
}

impl TrackBook {
    /// Take in a table snapshot.
    pub fn note(&mut self, aircraft: &[AdsbAircraft], now: i64) {
        if !self.started {
            self.started = true;
            self.saved_at = now;
            if let Some(dir) = Self::dir() {
                prune(&dir, now);
            }
        }
        let day = local_day(now);
        if day != self.day {
            self.flush();
            self.day = day;
        }
        for a in aircraft {
            let (Some(lat), Some(lon)) = (a.lat, a.lon) else { continue };
            let t = a.last_pos_at;
            if t <= 0 || self.seen.get(&a.icao).is_some_and(|&p| t <= p) {
                continue;
            }
            self.seen.insert(a.icao, t);
            // A position from before midnight belongs to yesterday's file.
            if local_day(t) != self.day {
                continue;
            }
            if !keep(self.kept.get(&a.icao).copied(), t, a.altitude_ft, a.track_deg) {
                continue;
            }
            self.kept.insert(a.icao, Kept { t, alt_ft: a.altitude_ft, trk_deg: a.track_deg });
            self.pending.push_str(&line(a, t, lat, lon));
            self.recent.entry(a.icao).or_default().push_back(TrailPoint {
                t,
                lat: lat as f32,
                lon: lon as f32,
                alt_ft: a.altitude_ft,
            });
        }
        for pts in self.recent.values_mut() {
            while pts.front().is_some_and(|q| now - q.t > RECENT_S) {
                pts.pop_front();
            }
        }
        self.recent.retain(|_, pts| !pts.is_empty());
        // Forget aircraft gone for ten minutes, so the maps stay small.
        self.seen.retain(|_, &mut t| now - t < 600);
        self.kept.retain(|_, k| now - k.t < 600);
        if now - self.saved_at >= 60 {
            self.saved_at = now;
            self.flush();
        }
    }

    /// The kept points of the last [`RECENT_S`] per aircraft, the same thinned
    /// points the day file gets: for the map's altitude-coloured trail.
    pub fn recent(&self) -> &Recent {
        &self.recent
    }

    fn dir() -> Option<std::path::PathBuf> {
        Some(sdroxide_config::config_dir().ok()?.join("adsb-tracks"))
    }

    /// Append the pending lines to the day's file.
    fn flush(&mut self) {
        if self.pending.is_empty() || self.day.is_empty() {
            return;
        }
        if let Some(dir) = Self::dir() {
            append(&dir, &self.day, &self.pending);
        }
        self.pending.clear();
    }
}

/// Append `lines` to `<dir>/<day>.csv`, with the header if the file is new.
fn append(dir: &std::path::Path, day: &str, lines: &str) {
    use std::io::Write as _;
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let path = dir.join(format!("{day}.csv"));
    let fresh = !path.exists();
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let mut out = String::new();
    if fresh {
        let _ = writeln!(out, "{HEAD}");
    }
    out.push_str(lines);
    let _ = f.write_all(out.as_bytes());
}

/// Remove day files older than [`KEEP_DAYS`].
fn prune(dir: &std::path::Path, now: i64) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let oldest = local_day(now - KEEP_DAYS * 86_400);
    for e in rd.flatten() {
        let name = e.file_name();
        let Some(day) = name.to_str().and_then(|n| n.strip_suffix(".csv")) else { continue };
        if day < oldest.as_str() {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(t: i64, alt: i32, trk: f32) -> Option<Kept> {
        Some(Kept { t, alt_ft: Some(alt), trk_deg: Some(trk) })
    }

    #[test]
    fn a_straight_cruise_keeps_one_point_in_fifteen_seconds() {
        let p = kept(1000, 35_000, 90.0);
        assert!(keep(None, 1000, Some(35_000), Some(90.0)));
        assert!(!keep(p, 1014, Some(35_010), Some(91.0)));
        assert!(keep(p, 1015, Some(35_000), Some(90.0)));
    }

    #[test]
    fn a_turn_or_a_climb_keeps_a_point_sooner() {
        let p = kept(1000, 3_000, 355.0);
        // Across north: 355° to 6° is eleven degrees.
        assert!(keep(p, 1002, Some(3_000), Some(6.0)));
        assert!(!keep(p, 1002, Some(3_000), Some(359.0)));
        assert!(keep(p, 1004, Some(3_600), Some(355.0)));
        assert!(!keep(p, 1004, Some(3_400), Some(355.0)));
    }

    #[test]
    fn the_map_gets_the_kept_points_of_the_last_ten_minutes_with_altitude() {
        let mut b = TrackBook { started: true, saved_at: i64::MAX, ..Default::default() };
        let t = 1_794_917_355;
        b.day = local_day(t);
        let mut a = AdsbAircraft::new(0x3E28E4, t);
        a.lat = Some(51.4448);
        a.lon = Some(7.0889);
        a.altitude_ft = Some(4675);
        a.last_pos_at = t;
        b.note(std::slice::from_ref(&a), t);
        a.altitude_ft = Some(5900);
        a.last_pos_at = t + 16;
        b.note(std::slice::from_ref(&a), t + 16);
        let pts = &b.recent()[&0x3E28E4];
        let alts: Vec<_> = pts.iter().map(|q| q.alt_ft).collect();
        assert_eq!(alts, vec![Some(4675), Some(5900)], "the same points the file gets");
        // Ten minutes after the first point it goes; after the last, all.
        b.note(&[], t + RECENT_S + 1);
        assert_eq!(b.recent()[&0x3E28E4].len(), 1);
        b.note(&[], t + 16 + RECENT_S + 1);
        assert!(b.recent().is_empty());
    }

    #[test]
    fn a_repeated_snapshot_is_one_position_and_lines_carry_local_time() {
        let mut b = TrackBook { started: true, saved_at: i64::MAX, ..Default::default() };
        let t = 1_794_917_355;
        b.day = local_day(t);
        let mut a = AdsbAircraft::new(0x3E28E4, t);
        a.callsign = "DIGRK   ".into();
        a.lat = Some(51.4448);
        a.lon = Some(7.0889);
        a.altitude_ft = Some(4675);
        a.track_deg = Some(283.0);
        a.last_pos_at = t;
        b.note(std::slice::from_ref(&a), t);
        b.note(std::slice::from_ref(&a), t);
        a.last_pos_at = t + 5;
        b.note(std::slice::from_ref(&a), t + 5);
        a.last_pos_at = t + 16;
        b.note(std::slice::from_ref(&a), t + 16);
        let lines: Vec<&str> = b.pending.lines().collect();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].starts_with(&format!(
            "3E28E4,DIGRK,{t},{},51.44480,7.08890,4675,,283",
            local_stamp(t)
        )));
    }

    #[test]
    fn a_day_file_gets_its_header_once_and_old_days_go() {
        let dir = std::env::temp_dir().join(format!("sdx-tracks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        append(&dir, "2026-11-17", "a\n");
        append(&dir, "2026-11-17", "b\n");
        let body = std::fs::read_to_string(dir.join("2026-11-17.csv")).unwrap();
        assert_eq!(body, format!("{HEAD}\na\nb\n"));
        append(&dir, "2026-09-01", "old\n");
        prune(&dir, 1_794_917_355);
        assert!(!dir.join("2026-09-01.csv").exists());
        assert!(dir.join("2026-11-17.csv").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
