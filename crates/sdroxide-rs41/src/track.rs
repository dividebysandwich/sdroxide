//! Sonde table: fold packages into [`Rs41Sonde`] rows and age them out.

use sdroxide_types::{Rs41Settings, Rs41Sonde, Rs41Status};

use crate::package::Package;

pub struct Tracker {
    cfg: Rs41Settings,
    sondes: Vec<Rs41Sonde>,
    window_center_hz: f64,
    window_rate_hz: f64,
    frames: u64,
    bad_crc: u64,
    unavailable: Option<String>,
}

impl Tracker {
    pub fn new(window_center_hz: f64, window_rate_hz: f64, cfg: Rs41Settings) -> Self {
        Tracker {
            cfg: cfg.sane(),
            sondes: Vec::new(),
            window_center_hz,
            window_rate_hz,
            frames: 0,
            bad_crc: 0,
            unavailable: None,
        }
    }

    pub fn set_config(&mut self, cfg: Rs41Settings) {
        self.cfg = cfg.sane();
        self.trim();
    }

    pub fn set_window(&mut self, center_hz: f64, rate_hz: f64) {
        self.window_center_hz = center_hz;
        self.window_rate_hz = rate_hz;
    }

    pub fn set_unavailable(&mut self, msg: Option<String>) {
        self.unavailable = msg;
    }

    pub fn note_counters(&mut self, frames: u64, bad_crc: u64) {
        self.frames = frames;
        self.bad_crc = bad_crc;
    }

    pub fn ingest(&mut self, pkg: Package, freq_hz: f64, rssi_dbfs: f32, now: i64) {
        let serial = pkg.serial.clone();
        if let Some(s) = self.sondes.iter_mut().find(|s| s.serial == serial) {
            apply(s, &pkg, freq_hz, rssi_dbfs, now, self.cfg.history_points);
        } else {
            let mut s = Rs41Sonde::new(serial, freq_hz, pkg.kind, now);
            apply(&mut s, &pkg, freq_hz, rssi_dbfs, now, self.cfg.history_points);
            self.sondes.push(s);
            self.trim();
        }
    }

    pub fn age_out(&mut self, now: i64) {
        let drop = i64::from(self.cfg.drop_list_s);
        self.sondes.retain(|s| now - s.last_at <= drop);
    }

    pub fn status(&self) -> Rs41Status {
        Rs41Status {
            sondes: self.sondes.clone(),
            unavailable: self.unavailable.clone(),
            suggest_center_hz: None,
            window_center_hz: self.window_center_hz,
            window_rate_hz: self.window_rate_hz,
            frames: self.frames,
            bad_crc: self.bad_crc,
        }
    }

    fn trim(&mut self) {
        let max = self.cfg.max_sondes as usize;
        if self.sondes.len() <= max {
            return;
        }
        self.sondes.sort_by_key(|s| s.last_at);
        let drop_n = self.sondes.len() - max;
        self.sondes.drain(..drop_n);
    }
}

fn apply(s: &mut Rs41Sonde, pkg: &Package, freq_hz: f64, rssi_dbfs: f32, now: i64, history: u16) {
    s.frequency_hz = freq_hz;
    s.rssi_dbfs = rssi_dbfs;
    s.frames = s.frames.saturating_add(1);
    s.last_at = now;
    s.kind = pkg.kind;
    s.manufacturer = pkg.manufacturer.clone();
    if let Some(ref st) = pkg.subtype {
        s.subtype = Some(st.clone());
    }
    if let Some(f) = pkg.frame {
        s.frame = Some(f);
    }
    if let Some(t) = pkg.temperature_c {
        s.temperature_c = Some(t);
    }
    if let Some(h) = pkg.humidity_pct {
        s.humidity_pct = Some(h);
    }
    if let Some(p) = pkg.pressure_hpa {
        s.pressure_hpa = Some(p);
    }
    if let Some(b) = pkg.battery_v {
        s.battery_v = Some(b);
    }
    if let Some(a) = pkg.ascent_rate_mps {
        s.ascent_rate_mps = Some(a);
    }
    if let Some(v) = pkg.vel_h_mps {
        s.vel_h_mps = Some(v);
    }
    if let Some(h) = pkg.heading_deg {
        s.heading_deg = Some(h);
    }
    if let Some(n) = pkg.sats {
        s.sats = Some(n);
    }
    if let Some(ref dt) = pkg.datetime {
        if !dt.is_empty() {
            s.datetime = Some(dt.clone());
        }
    }
    if let (Some(lat), Some(lon)) = (pkg.lat, pkg.lon) {
        push_pos(s, lat, lon, pkg.altitude_m, now, history);
    }
}

fn push_pos(s: &mut Rs41Sonde, lat: f64, lon: f64, alt: Option<f32>, now: i64, history: u16) {
    s.lat = Some(lat);
    s.lon = Some(lon);
    if let Some(a) = alt {
        s.altitude_m = Some(a);
    }
    s.last_pos_at = now;
    s.track.push((lat as f32, lon as f32));
    let cap = history as usize;
    if s.track.len() > cap {
        let n = s.track.len() - cap;
        s.track.drain(..n);
    }
}
