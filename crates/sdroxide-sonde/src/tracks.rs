//! The book of sondes heard: one [`SondeTrack`] per serial number, updated
//! frame by frame. Pure data handling, no threads, so it is tested directly.

#[cfg(test)]
use crate::model::Meteo;
use crate::model::SondeFix;
use sdroxide_types::{SONDE_MAX_TRACKS, SONDE_TRACK_DEPTH, SondePoint, SondeTrack};

/// Every sonde heard since the lane started, most recently heard first.
#[derive(Debug, Default)]
pub struct TrackBook {
    sondes: Vec<SondeTrack>,
}

impl TrackBook {
    /// Fold one decoded frame in. `freq_hz` is the channel it was heard on and
    /// `unix` the time it was heard.
    pub fn push(&mut self, f: &SondeFix, freq_hz: f64, unix: i64) {
        // A model whose frame has no readable serial is named by its channel.
        let serial = if f.serial.is_empty() {
            format!("{}@{:.3}", f.model.name(), freq_hz / 1e6)
        } else {
            f.serial.clone()
        };
        let idx = match self.sondes.iter().position(|s| s.serial == serial) {
            Some(i) => i,
            None => {
                self.sondes.insert(
                    0,
                    SondeTrack {
                        model: f.model.name().to_string(),
                        serial: serial.clone(),
                        freq_hz,
                        frame: 0,
                        lat: None,
                        lon: None,
                        alt_m: None,
                        speed_ms: None,
                        course_deg: None,
                        climb_ms: None,
                        num_sv: None,
                        battery_v: None,
                        temp_c: None,
                        humidity_pct: None,
                        pressure_hpa: None,
                        last_unix: unix,
                        frames: 0,
                        track: Vec::new(),
                    },
                );
                self.sondes.truncate(SONDE_MAX_TRACKS);
                0
            }
        };
        let s = &mut self.sondes[idx];
        s.freq_hz = freq_hz;
        s.frame = f.frame_num;
        s.battery_v = f.battery_v.filter(|v| v.is_finite() && *v > 0.0);
        // A reading stays until the model sends a newer one: a sonde sends
        // temperature and humidity in different frames than the position.
        if let Some(t) = f.meteo.temp_c.filter(|t| t.is_finite() && (-120.0..80.0).contains(t)) {
            s.temp_c = Some(t);
        }
        if let Some(h) = f.meteo.humidity_pct.filter(|h| h.is_finite() && (0.0..=100.0).contains(h))
        {
            s.humidity_pct = Some(h);
        }
        if let Some(p) = f.meteo.pressure_hpa.filter(|p| p.is_finite() && (0.0..1200.0).contains(p))
        {
            s.pressure_hpa = Some(p);
        }
        s.last_unix = unix;
        s.frames = s.frames.saturating_add(1);
        if let Some(p) = f.pos.as_ref().filter(|p| valid_fix(p.lat, p.lon)) {
            s.lat = Some(p.lat);
            s.lon = Some(p.lon);
            s.alt_m = Some(p.alt_m as f32);
            s.speed_ms = Some(p.speed_ms as f32);
            s.course_deg = Some(p.course_deg as f32);
            s.climb_ms = Some(p.climb_ms as f32);
            s.num_sv = Some(p.num_sv);
            // One point per second; a repeat of the same second adds nothing.
            let same = s.track.last().is_some_and(|l| l.unix == unix);
            if !same {
                if s.track.len() >= SONDE_TRACK_DEPTH {
                    thin(&mut s.track);
                }
                s.track.push(SondePoint {
                    lat: p.lat,
                    lon: p.lon,
                    alt_m: p.alt_m as f32,
                    unix,
                    temp_c: s.temp_c,
                    humidity_pct: s.humidity_pct,
                    pressure_hpa: s.pressure_hpa,
                });
            }
        }
        // Most recently heard first.
        if idx != 0 {
            let s = self.sondes.remove(idx);
            self.sondes.insert(0, s);
        }
    }

    pub fn snapshot(&self) -> Vec<SondeTrack> {
        self.sondes.clone()
    }
}

/// A GPS fix that is a place on Earth. A sonde without a lock sends zeros (or
/// the geocentre, which converts to nonsense), and neither belongs on a map.
fn valid_fix(lat: f64, lon: f64) -> bool {
    lat.is_finite()
        && lon.is_finite()
        && lat.abs() <= 90.0
        && lon.abs() <= 180.0
        && !(lat.abs() < 1e-6 && lon.abs() < 1e-6)
}

/// Drop every other point (keeping the newest), so a long flight keeps its
/// whole shape in bounded memory instead of losing its start.
fn thin(track: &mut Vec<SondePoint>) {
    let last = track.len().saturating_sub(1);
    let mut i = 0;
    track.retain(|_| {
        let keep = i % 2 == 0 || i == last;
        i += 1;
        keep
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SondeModel;
    use xng_mode_sonde::GpsPos;

    fn frame(serial: &str, n: u16, lat: f64, lon: f64, alt: f64) -> SondeFix {
        SondeFix {
            model: SondeModel::Rs41,
            serial: serial.to_string(),
            frame_num: u32::from(n),
            battery_v: Some(2.7),
            meteo: Meteo::default(),
            pos: Some(GpsPos {
                lat,
                lon,
                alt_m: alt,
                speed_ms: 4.0,
                course_deg: 90.0,
                climb_ms: 5.0,
                num_sv: 9,
            }),
        }
    }

    #[test]
    fn frames_of_one_serial_build_one_track() {
        let mut b = TrackBook::default();
        for i in 0..5u16 {
            let lat = 48.0 + f64::from(i) * 1e-3;
            b.push(&frame("N1", i, lat, 2.0, 100.0 * f64::from(i)), 403e6, 10 + i64::from(i));
        }
        let s = b.snapshot();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].frames, 5);
        assert_eq!(s[0].track.len(), 5);
        assert_eq!(s[0].frame, 4);
        assert_eq!(s[0].alt_m, Some(400.0));
        assert_eq!(s[0].climb_ms, Some(5.0));
    }

    #[test]
    fn the_latest_heard_sonde_comes_first_and_the_book_is_bounded() {
        let mut b = TrackBook::default();
        b.push(&frame("A", 1, 48.0, 2.0, 1.0), 403e6, 1);
        b.push(&frame("B", 1, 49.0, 2.0, 1.0), 403e6, 2);
        b.push(&frame("A", 2, 48.0, 2.0, 1.0), 403e6, 3);
        assert_eq!(b.snapshot()[0].serial, "A");
        for i in 0..20 {
            b.push(&frame(&format!("S{i}"), 1, 48.0, 2.0, 1.0), 403e6, 10 + i);
        }
        assert_eq!(b.snapshot().len(), SONDE_MAX_TRACKS);
    }

    #[test]
    fn a_model_without_a_serial_is_named_by_its_channel() {
        let mut b = TrackBook::default();
        let mut f = frame("", 1, 48.0, 2.0, 1.0);
        f.model = SondeModel::M20;
        b.push(&f, 403.5e6, 1);
        b.push(&f, 403.5e6, 2);
        b.push(&f, 404.1e6, 3);
        let s = b.snapshot();
        assert_eq!(s.len(), 2, "two channels, two sondes");
        assert_eq!(s[0].serial, "M20@404.100");
        assert_eq!(s[1].serial, "M20@403.500");
        assert_eq!(s[1].frames, 2);
    }

    #[test]
    fn a_frame_without_a_real_fix_adds_no_point() {
        let mut b = TrackBook::default();
        b.push(&frame("N1", 1, 0.0, 0.0, 0.0), 403e6, 1);
        let s = b.snapshot();
        assert_eq!(s[0].frames, 1, "the sonde was heard");
        assert!(s[0].lat.is_none() && s[0].track.is_empty(), "but it has no place yet");
    }

    #[test]
    fn a_long_flight_is_thinned_not_truncated() {
        let mut b = TrackBook::default();
        for i in 0..(SONDE_TRACK_DEPTH as i64 + 10) {
            b.push(&frame("N1", (i % 60000) as u16, 48.0, 2.0 + i as f64 * 1e-5, 1.0), 403e6, i);
        }
        let t = &b.snapshot()[0].track;
        assert!(t.len() <= SONDE_TRACK_DEPTH);
        assert_eq!(t[0].unix, 0, "the launch point is kept");
        assert_eq!(t.last().unwrap().unix, SONDE_TRACK_DEPTH as i64 + 9, "and the newest");
    }

    #[test]
    fn a_reading_stays_until_a_newer_one_comes() {
        let mut b = TrackBook::default();
        let mut f = frame("A1", 1, 48.0, 2.0, 100.0);
        f.meteo = Meteo { temp_c: Some(12.5), humidity_pct: Some(55.0), pressure_hpa: Some(980.0) };
        b.push(&f, 403e6, 10);
        b.push(&frame("A1", 2, 48.0, 2.0, 110.0), 403e6, 11);
        let s = &b.sondes[0];
        assert_eq!(
            (s.temp_c, s.humidity_pct, s.pressure_hpa),
            (Some(12.5), Some(55.0), Some(980.0))
        );
        // The point kept with a frame carries the readings at that time.
        assert_eq!(s.track.last().unwrap().temp_c, Some(12.5));
        // Nonsense is dropped, the last good value stays.
        let mut g = frame("A1", 3, 48.0, 2.0, 120.0);
        g.meteo =
            Meteo { temp_c: Some(900.0), humidity_pct: Some(-4.0), pressure_hpa: Some(f32::NAN) };
        b.push(&g, 403e6, 12);
        assert_eq!(b.sondes[0].temp_c, Some(12.5));
    }
}
