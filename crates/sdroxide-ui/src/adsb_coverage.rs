//! Where this station hears aircraft: a polar diagram around home, kept by
//! the hour.
//!
//! The sky around the antenna is cut into [`SECTORS`] slices of bearing and
//! [`RINGS`] rings of [`RING_KM`]. Every position an aircraft reports lands in
//! one of those cells — but only if the same aircraft gave a position no more
//! than [`CONTINUOUS_S`] before it. A single frame from far away is a lucky
//! decode; a track that keeps coming is reception, and the diagram is meant to
//! show reception. Each cell keeps how many such positions fell in it and how
//! strong their frames were, so the picture says *how well* as well as *how
//! far*. Coverage is not round: hills, buildings and the antenna's own pattern
//! cut it per direction, and on some days the troposphere carries it far.
//!
//! The cells are kept per hour and on disk for [`KEEP_H`], one file an hour in
//! the config folder (`adsb-coverage/`), so the operator can set the last hour
//! (today's conditions) against the last week (what the antenna usually does).
//! The browser build keeps the session only.
//!
//! A different home is a different diagram: when the grid square moves, what
//! was gathered is dropped.

use std::collections::{BTreeMap, HashMap};

use sdroxide_types::AdsbAircraft;

/// Slices of bearing, 5° each.
pub const SECTORS: usize = 72;
/// Rings out from home.
pub const RINGS: usize = 40;
/// Width of one ring, km. With [`RINGS`], 400 km — past the radio horizon of
/// an airliner at FL450 for an antenna near the ground.
pub const RING_KM: f64 = 10.0;
/// Longest gap between two positions of one aircraft that still counts as
/// continuous reception, seconds.
pub const CONTINUOUS_S: i64 = 20;
/// Hours kept on disk.
pub const KEEP_H: i64 = 7 * 24;
/// Positions a cell needs before the boundary goes through it; fewer is a
/// stray.
pub const BOUNDARY_MIN: u32 = 3;

/// What one cell has gathered.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Cell {
    /// Continuous positions in this cell.
    pub n: u32,
    /// Sum of their frame levels, dBFS, for the mean.
    pub rssi_sum: f32,
    /// Strongest frame level, dBFS.
    pub rssi_max: f32,
}

impl Cell {
    pub fn mean_rssi(&self) -> f32 {
        if self.n == 0 { f32::NEG_INFINITY } else { self.rssi_sum / self.n as f32 }
    }

    pub(crate) fn add(&mut self, rssi: f32) {
        self.rssi_max = if self.n == 0 { rssi } else { self.rssi_max.max(rssi) };
        self.n += 1;
        self.rssi_sum += rssi;
    }

    fn merge(&mut self, o: &Cell) {
        if o.n == 0 {
            return;
        }
        self.rssi_max = if self.n == 0 { o.rssi_max } else { self.rssi_max.max(o.rssi_max) };
        self.n += o.n;
        self.rssi_sum += o.rssi_sum;
    }
}

/// The cell for a position seen from home, or `None` past the last ring.
pub fn cell_of(home: (f64, f64), pos: (f64, f64)) -> Option<usize> {
    let d = sdroxide_types::distance_km(home, pos);
    let ring = (d / RING_KM) as usize;
    if ring >= RINGS {
        return None;
    }
    let b = sdroxide_types::bearing_deg(home, pos);
    let sector = ((b / (360.0 / SECTORS as f64)) as usize) % SECTORS;
    Some(sector * RINGS + ring)
}

/// The point `km` from `from` on bearing `deg`, on a sphere.
pub fn destination(from: (f64, f64), deg: f64, km: f64) -> (f64, f64) {
    const R: f64 = 6371.0;
    let (p1, l1) = (from.0.to_radians(), from.1.to_radians());
    let (b, d) = (deg.to_radians(), km / R);
    let p2 = (p1.sin() * d.cos() + p1.cos() * d.sin() * b.cos()).asin();
    let l2 = l1 + (b.sin() * d.sin() * p1.cos()).atan2(d.cos() - p1.sin() * p2.sin());
    (p2.to_degrees(), (l2.to_degrees() + 540.0) % 360.0 - 180.0)
}

/// The diagram over a window of hours, ready to draw.
#[derive(Debug, Clone, Default)]
pub struct CoverageView {
    pub home: (f64, f64),
    /// `SECTORS * RINGS` cells, sector-major.
    pub cells: Vec<Cell>,
    /// Per sector, the outermost ring with at least [`BOUNDARY_MIN`]
    /// positions.
    pub boundary: Vec<Option<usize>>,
}

/// Everything gathered, by hour.
#[derive(Default)]
pub struct CoverageBook {
    home: Option<(f64, f64)>,
    hours: BTreeMap<i64, HashMap<u16, Cell>>,
    /// Each aircraft's last position time, for the continuity test.
    last_pos: HashMap<u32, i64>,
    loaded: bool,
    /// Hours changed since they were last written.
    dirty: std::collections::BTreeSet<i64>,
    saved_at: i64,
}

impl CoverageBook {
    /// Take in a table snapshot. Each aircraft counts once per new position.
    pub fn note(&mut self, home: Option<(f64, f64)>, aircraft: &[AdsbAircraft], now: i64) {
        let Some(home) = home else { return };
        if !self.loaded {
            self.loaded = true;
            self.load(home, now);
        }
        // A new home is a new diagram.
        if self.home.is_some_and(|h| sdroxide_types::distance_km(h, home) > 1.0) {
            self.hours.clear();
            self.dirty.clear();
            self.remove_files();
        }
        self.home = Some(home);
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
            let hour = t.div_euclid(3600);
            self.hours.entry(hour).or_default().entry(idx as u16).or_default().add(a.rssi_dbfs);
            self.dirty.insert(hour);
        }
        // Forget aircraft not heard for a while, so the map stays small.
        self.last_pos.retain(|_, &mut t| now - t < 600);
        let oldest = now.div_euclid(3600) - KEEP_H;
        self.hours.retain(|&h, _| h > oldest);
        if now - self.saved_at >= 60 {
            self.save(now);
        }
    }

    /// Hours with something in them, for the settings window.
    pub fn hours_kept(&self) -> usize {
        self.hours.len()
    }

    /// Forget everything gathered, on disk too: a new antenna, a new place
    /// on the roof (HDZ 2026-11-17: „muss im Settings-Dialog resettbar sein“).
    pub fn reset(&mut self) {
        self.hours.clear();
        self.dirty.clear();
        self.last_pos.clear();
        self.remove_files();
    }

    /// The diagram over the last `hours` hours.
    pub fn view(&self, hours: i64, now: i64) -> Option<CoverageView> {
        let home = self.home?;
        let from = now.div_euclid(3600) - hours + 1;
        let mut cells = vec![Cell::default(); SECTORS * RINGS];
        for (_, m) in self.hours.range(from..) {
            for (&i, c) in m {
                if let Some(slot) = cells.get_mut(i as usize) {
                    slot.merge(c);
                }
            }
        }
        let boundary = (0..SECTORS)
            .map(|s| (0..RINGS).rev().find(|&r| cells[s * RINGS + r].n >= BOUNDARY_MIN))
            .collect();
        Some(CoverageView { home, cells, boundary })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn dir() -> Option<std::path::PathBuf> {
        Some(sdroxide_config::config_dir().ok()?.join("adsb-coverage"))
    }

    /// Write the changed hours. One line a cell: `index n rssi_sum rssi_max`,
    /// under a `home lat lon` line.
    fn save(&mut self, now: i64) {
        self.saved_at = now;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (Some(dir), Some(home)) = (Self::dir(), self.home) else { return };
            if std::fs::create_dir_all(&dir).is_err() {
                return;
            }
            for h in std::mem::take(&mut self.dirty) {
                let Some(m) = self.hours.get(&h) else { continue };
                let _ = std::fs::write(dir.join(format!("{h}.txt")), encode(home, m));
            }
            let oldest = now.div_euclid(3600) - KEEP_H;
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let name = e.file_name();
                    let h = name.to_str().and_then(|n| n.strip_suffix(".txt")?.parse::<i64>().ok());
                    if h.is_some_and(|h| h <= oldest) {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
        }
    }

    /// Read back what is on disk for this home.
    fn load(&mut self, home: (f64, f64), now: i64) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(dir) = Self::dir() else { return };
            let Ok(rd) = std::fs::read_dir(&dir) else { return };
            let oldest = now.div_euclid(3600) - KEEP_H;
            for e in rd.flatten() {
                let name = e.file_name();
                let Some(h) =
                    name.to_str().and_then(|n| n.strip_suffix(".txt")?.parse::<i64>().ok())
                else {
                    continue;
                };
                if h <= oldest {
                    continue;
                }
                let Ok(body) = std::fs::read_to_string(e.path()) else { continue };
                if let Some((at, m)) = decode(&body)
                    && sdroxide_types::distance_km(at, home) <= 1.0
                {
                    self.hours.insert(h, m);
                }
            }
            self.home = Some(home);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (home, now);
    }

    fn remove_files(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dir) = Self::dir()
            && let Ok(rd) = std::fs::read_dir(&dir)
        {
            for e in rd.flatten() {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn encode(home: (f64, f64), m: &HashMap<u16, Cell>) -> String {
    let mut s = format!("home {:.5} {:.5}\n", home.0, home.1);
    let mut keys: Vec<_> = m.keys().copied().collect();
    keys.sort_unstable();
    for k in keys {
        let c = m[&k];
        s.push_str(&format!("{k} {} {:.1} {:.1}\n", c.n, c.rssi_sum, c.rssi_max));
    }
    s
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn decode(body: &str) -> Option<((f64, f64), HashMap<u16, Cell>)> {
    let mut lines = body.lines();
    let head: Vec<&str> = lines.next()?.split_whitespace().collect();
    let home = match head.as_slice() {
        ["home", la, lo] => (la.parse().ok()?, lo.parse().ok()?),
        _ => return None,
    };
    let mut m = HashMap::new();
    for l in lines {
        let f: Vec<&str> = l.split_whitespace().collect();
        if let [i, n, sum, max] = f.as_slice()
            && let (Ok(i), Ok(n), Ok(sum), Ok(max)) =
                (i.parse::<u16>(), n.parse(), sum.parse(), max.parse())
            && (i as usize) < SECTORS * RINGS
        {
            m.insert(i, Cell { n, rssi_sum: sum, rssi_max: max });
        }
    }
    Some((home, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: (f64, f64) = (51.5, 7.5);

    fn ac(icao: u32, pos: (f64, f64), t: i64, rssi: f32) -> AdsbAircraft {
        let mut a = AdsbAircraft::new(icao, t);
        a.lat = Some(pos.0);
        a.lon = Some(pos.1);
        a.last_pos_at = t;
        a.rssi_dbfs = rssi;
        a
    }

    #[test]
    fn a_cell_is_found_by_bearing_and_distance() {
        // 55 km due north: sector 0, ring 5.
        let north = destination(HOME, 0.0, 55.0);
        assert_eq!(cell_of(HOME, north), Some(5));
        // 105 km due east: sector 18 (90°/5°), ring 10.
        let east = destination(HOME, 90.0, 105.0);
        assert_eq!(cell_of(HOME, east), Some(18 * RINGS + 10));
        assert_eq!(cell_of(HOME, destination(HOME, 45.0, 450.0)), None);
    }

    #[test]
    fn only_continuous_positions_count() {
        let mut b = CoverageBook { loaded: true, saved_at: i64::MAX, ..Default::default() };
        let p = destination(HOME, 90.0, 105.0);
        let t0 = 1_800_000_000;
        // First sighting: no predecessor, not counted.
        b.note(Some(HOME), &[ac(1, p, t0, -10.0)], t0);
        // 5 s later: counted.
        b.note(Some(HOME), &[ac(1, p, t0 + 5, -12.0)], t0 + 5);
        // Same snapshot again: same position time, not counted twice.
        b.note(Some(HOME), &[ac(1, p, t0 + 5, -12.0)], t0 + 5);
        // After a 60 s gap: the track broke off, not counted.
        b.note(Some(HOME), &[ac(1, p, t0 + 65, -30.0)], t0 + 65);
        // And continuous again.
        b.note(Some(HOME), &[ac(1, p, t0 + 70, -8.0)], t0 + 70);
        let v = b.view(1, t0 + 70).unwrap();
        let c = v.cells[18 * RINGS + 10];
        assert_eq!(c.n, 2);
        assert_eq!(c.rssi_max, -8.0);
        assert!((c.mean_rssi() + 10.0).abs() < 1e-6);
    }

    #[test]
    fn the_boundary_needs_more_than_a_stray() {
        let mut b = CoverageBook { loaded: true, saved_at: i64::MAX, ..Default::default() };
        let t0 = 1_800_000_000;
        let near = destination(HOME, 2.0, 95.0); // sector 0, ring 9
        let far = destination(HOME, 2.0, 255.0); // sector 0, ring 25
        for k in 0..5 {
            let t = t0 + k * 5;
            b.note(Some(HOME), &[ac(1, near, t, -10.0), ac(2, far, t.min(t0 + 5), -20.0)], t);
        }
        let v = b.view(1, t0 + 25).unwrap();
        assert_eq!(v.cells[25].n, 1);
        assert_eq!(v.boundary[0], Some(9));
        assert_eq!(v.boundary[1], None);
    }

    #[test]
    fn an_hour_file_reads_back_as_written() {
        let mut m = HashMap::new();
        m.insert(42u16, Cell { n: 7, rssi_sum: -70.0, rssi_max: -4.5 });
        let (home, back) = decode(&encode(HOME, &m)).unwrap();
        assert!((home.0 - HOME.0).abs() < 1e-4 && (home.1 - HOME.1).abs() < 1e-4);
        assert_eq!(back, m);
    }

    #[test]
    fn hours_outside_the_window_are_left_out() {
        let mut b = CoverageBook { loaded: true, saved_at: i64::MAX, ..Default::default() };
        let p = destination(HOME, 90.0, 105.0);
        let t0 = 500_000 * 3600; // on the hour
        b.note(Some(HOME), &[ac(1, p, t0, -10.0)], t0);
        b.note(Some(HOME), &[ac(1, p, t0 + 5, -10.0)], t0 + 5);
        let later = t0 + 3 * 3600;
        assert_eq!(b.view(1, later).unwrap().cells[18 * RINGS + 10].n, 0);
        assert_eq!(b.view(24, later).unwrap().cells[18 * RINGS + 10].n, 1);
    }
}
