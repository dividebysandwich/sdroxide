//! RS41 air temperature and humidity.
//!
//! The RS41 sends 12 raw 24-bit counts every frame (`Ptu::raw`) and one 16-byte
//! slice of its calibration table (51 slices, `Ptu::cal_index`). A reading is
//! only possible once the slices holding the coefficients have been heard, so
//! up to 51 frames (about a minute) after the sonde is first heard there is no
//! temperature. The table belongs to one sonde: it is dropped when the serial
//! changes.
//!
//! Provenance: where the coefficients sit in the table and the measurement
//! chain (two reference resistors give gain and offset, a platinum
//! resistance, a quadratic fit and a gain/offset/scale trim) follow the public
//! description of rs1729/RS; the humidity formula is its empirical one. The
//! values were compared with that program's output on the same recording
//! (see the test notes in TODO.md). No code is copied.

use crate::model::Meteo;
use xng_mode_sonde::Ptu;

const SLICES: usize = 51;

/// Table offsets (bytes) of each little-endian `f32`.
const RF1: usize = 61;
const RF2: usize = 65;
const CO1: usize = 77; // three floats
const CAL_T1: usize = 89; // three floats
const CAL_H: usize = 117; // two floats

#[derive(Default)]
pub struct Calibration {
    serial: String,
    bytes: Vec<u8>,
    seen: Vec<bool>,
}

impl Calibration {
    /// Keep the slice this frame carried; start over on another sonde.
    pub fn feed(&mut self, serial: &str, ptu: &Ptu) {
        if self.serial != serial || self.bytes.len() != SLICES * 16 {
            self.serial = serial.to_string();
            self.bytes = vec![0; SLICES * 16];
            self.seen = vec![false; SLICES];
        }
        let i = usize::from(ptu.cal_index);
        if i < SLICES && !self.seen[i] {
            self.bytes[i * 16..i * 16 + 16].copy_from_slice(&ptu.cal_bytes);
            self.seen[i] = true;
        }
    }

    /// The float at `off`, when both slices it lies in have been heard.
    fn f32_at(&self, off: usize) -> Option<f64> {
        if self.bytes.len() != SLICES * 16 || !self.seen[off / 16] || !self.seen[(off + 3) / 16] {
            return None;
        }
        let v = f32::from_le_bytes([
            self.bytes[off],
            self.bytes[off + 1],
            self.bytes[off + 2],
            self.bytes[off + 3],
        ]);
        v.is_finite().then_some(f64::from(v))
    }

    fn floats<const N: usize>(&self, off: usize) -> Option<[f64; N]> {
        let mut out = [0.0; N];
        for (k, o) in out.iter_mut().enumerate() {
            *o = self.f32_at(off + 4 * k)?;
        }
        Some(out)
    }

    /// Temperature and humidity of one frame, whichever the table allows.
    pub fn meteo(&self, ptu: &Ptu) -> Meteo {
        let t = self.temperature(ptu);
        Meteo {
            temp_c: t.map(|t| t as f32),
            humidity_pct: t.and_then(|t| self.humidity(ptu, t)).map(|h| h as f32),
            pressure_hpa: None,
        }
    }

    fn temperature(&self, ptu: &Ptu) -> Option<f64> {
        let (rf1, rf2) = (self.f32_at(RF1)?, self.f32_at(RF2)?);
        let co: [f64; 3] = self.floats(CO1)?;
        let cal: [f64; 3] = self.floats(CAL_T1)?;
        let (f, f1, f2) = (f64::from(ptu.raw[0]), f64::from(ptu.raw[1]), f64::from(ptu.raw[2]));
        if f2 == f1 || rf2 == rf1 {
            return None;
        }
        // Two known resistors give the converter's gain and offset.
        let gain = (f2 - f1) / (rf2 - rf1);
        let offset = (f1 * rf2 - f2 * rf1) / (f2 - f1);
        let r = (f / gain - offset) * cal[0];
        let t = (co[0] + co[1] * r + co[2] * r * r + cal[1]) * (1.0 + cal[2]);
        (-120.0..80.0).contains(&t).then_some(t)
    }

    fn humidity(&self, ptu: &Ptu, t: f64) -> Option<f64> {
        let cal: [f64; 2] = self.floats(CAL_H)?;
        let (f, f1, f2) = (f64::from(ptu.raw[3]), f64::from(ptu.raw[4]), f64::from(ptu.raw[5]));
        if f2 == f1 || cal[0] == 0.0 {
            return None;
        }
        let fh = (f - f1) / (f2 - f1);
        let mut rh = 100.0 * (350.0 / cal[0] * fh - 7.5);
        // Empirical temperature compensation.
        rh -= t / 5.5;
        if t < -25.0 {
            rh *= 1.0 + (-25.0 - t) / 90.0;
        }
        Some(rh.clamp(0.0, 100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table whose coefficients make the arithmetic easy to follow: the
    /// reference resistors are 750 and 1100 ohms, the temperature fit is
    /// T = R / 10 - 100, the trims are neutral.
    fn table() -> Vec<u8> {
        let mut t = vec![0u8; SLICES * 16];
        let mut put = |off: usize, v: f32| t[off..off + 4].copy_from_slice(&v.to_le_bytes());
        put(RF1, 750.0);
        put(RF2, 1100.0);
        put(CO1, -100.0);
        put(CO1 + 4, 0.1);
        put(CO1 + 8, 0.0);
        put(CAL_T1, 1.0);
        put(CAL_T1 + 4, 0.0);
        put(CAL_T1 + 8, 0.0);
        put(CAL_H, 350.0);
        t
    }

    fn ptu(raw: [u32; 12], i: u8, bytes: &[u8]) -> Ptu {
        let mut cal_bytes = [0u8; 16];
        cal_bytes.copy_from_slice(&bytes[usize::from(i) * 16..usize::from(i) * 16 + 16]);
        Ptu { raw, p_aux: 0, cal_index: i, cal_bytes }
    }

    #[test]
    fn nothing_is_read_before_the_table_is_heard() {
        let t = table();
        let mut c = Calibration::default();
        // Counts for 1000 ohms: gain 1 count/ohm, offset 0 -> f1 = 750, f2 = 1100.
        let raw = [1000, 750, 1100, 7925, 0, 1000, 0, 0, 0, 0, 0, 0];
        c.feed("X1", &ptu(raw, 3, &t));
        assert_eq!(c.meteo(&ptu(raw, 3, &t)), Meteo::default());
    }

    #[test]
    fn a_complete_table_gives_a_temperature_and_a_humidity() {
        let t = table();
        let mut c = Calibration::default();
        let raw = [1000, 750, 1100, 7925, 0, 1000, 0, 0, 0, 0, 0, 0];
        for i in 0..SLICES as u8 {
            c.feed("X1", &ptu(raw, i, &t));
        }
        let m = c.meteo(&ptu(raw, 0, &t));
        // R = 1000 ohms -> T = -100 + 0.1 * 1000 = 0 degrees.
        assert!(m.temp_c.unwrap().abs() < 1e-3, "{m:?}");
        // fh = 7.925, RH = 100 * (350 / 350 * 7.925 - 7.5) = 42.5 at 0 degrees.
        assert!((m.humidity_pct.unwrap() - 42.5).abs() < 1e-2, "{m:?}");
    }

    #[test]
    fn another_serial_starts_a_new_table() {
        let t = table();
        let mut c = Calibration::default();
        let raw = [1000, 750, 1100, 7925, 0, 1000, 0, 0, 0, 0, 0, 0];
        for i in 0..SLICES as u8 {
            c.feed("X1", &ptu(raw, i, &t));
        }
        c.feed("X2", &ptu(raw, 0, &t));
        assert_eq!(c.meteo(&ptu(raw, 0, &t)).temp_c, None);
    }
}
