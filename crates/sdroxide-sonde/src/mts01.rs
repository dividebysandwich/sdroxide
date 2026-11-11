//! Meteosis MTS01 decoder.
//!
//! The signal: FSK at 1200 baud, plain bits (no line code), most significant
//! bit first. A frame starts with the bytes `AA AA B4 2B`, then one marker
//! byte, 128 bytes of ASCII text and a CRC-16 over the text (polynomial
//! 0x8005 shifted left, start 0xFFFF, result with its bits reversed), stored
//! low byte first. The text is a list of comma-separated fields, padded with
//! zero bytes: `serial, ?, counter, YYMMDDhhmmss, battery mV, lat, lon, alt,
//! heading, speed, ?, thermistor, thermistor, ?, ...`. The two thermistor
//! numbers are equal; the temperature comes from them with the approximation
//! below. The signal is narrow (tones about 1.2 kHz apart), so the channel is
//! low-passed before the discriminator.
//!
//! Provenance: the layout is the public description of the MTS01 telemetry by
//! rs1729/RS (`demod/mod/mts01mod.c`, discussion RS#48). No code of auto_rx,
//! SDRangel or rs1729 (GPL) is used. Checked on a real recording (a sonde on
//! the ground near Istanbul, 2022-11-10): the CRC passes, the serial, counter,
//! time, battery, place and altitude read sensibly. The thermistor constants
//! are those of the description, not a calibration.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 1200.0;
const HEADER: u32 = 0xAAAA_B42B;
const HEADER_TOLERANCE: u32 = 2;
/// The marker byte, the text and the CRC.
const BODY_BYTES: usize = 1 + TEXT_BYTES + 2;
const TEXT_BYTES: usize = 128;

fn crc16(data: &[u8]) -> u16 {
    let mut c: u16 = 0xFFFF;
    for &b in data {
        c ^= u16::from(b) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x8005 } else { c << 1 };
        }
    }
    c.reverse_bits()
}

/// Temperature of the thermistor reading `r` (kilo-ohms, 15 at 0 degrees C):
/// `1/T = 1/T0 + ln(r/R0)/B` with B = 3000 K. The approximation of the public
/// description; it is not a calibration.
fn thermistor_c(r: f64) -> Option<f64> {
    if !(r > 0.0 && r.is_finite()) {
        return None;
    }
    let t = 1.0 / (1.0 / 273.15 + (r / 15.0).ln() / 3000.0) - 273.15;
    (-120.0..80.0).contains(&t).then_some(t)
}

/// One checked frame (marker, text and CRC bytes, in this order).
fn fix_of(body: &[u8]) -> Option<SondeFix> {
    if body.len() != BODY_BYTES {
        return None;
    }
    let text = &body[1..1 + TEXT_BYTES];
    if crc16(text) != u16::from_le_bytes([body[1 + TEXT_BYTES], body[2 + TEXT_BYTES]]) {
        return None;
    }
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    let text = std::str::from_utf8(&text[..end]).ok()?;
    if !text.bytes().all(|b| (0x20..0x7F).contains(&b)) {
        return None;
    }
    let f: Vec<&str> = text.split(',').collect();
    let num = |i: usize| f.get(i).and_then(|s| s.trim().parse::<f64>().ok());
    let id: String = f.first()?.trim().chars().take(8).collect();
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let frame_num = num(2).filter(|n| *n >= 0.0).map_or(0, |n| n as u32);
    let battery_v = num(4).filter(|mv| (0.0..20_000.0).contains(mv)).map(|mv| (mv / 1000.0) as f32);
    let pos = match (num(5), num(6), num(7)) {
        (Some(lat), Some(lon), Some(alt))
            if (-90.0..=90.0).contains(&lat)
                && (-180.0..=180.0).contains(&lon)
                && !(lat == 0.0 && lon == 0.0)
                && (-1000.0..80_000.0).contains(&alt) =>
        {
            Some(GpsPos {
                lat,
                lon,
                alt_m: alt,
                speed_ms: num(9).filter(|v| (0.0..500.0).contains(v)).unwrap_or(0.0),
                course_deg: num(8).filter(|v| (0.0..=360.0).contains(v)).unwrap_or(0.0),
                climb_ms: 0.0,
                num_sv: 0,
            })
        }
        _ => None,
    };
    let temp_c = num(11).and_then(thermistor_c).map(|t| t as f32);
    Some(SondeFix {
        model: SondeModel::Mts01,
        serial: format!("MTS01-{id}"),
        frame_num,
        battery_v,
        meteo: Meteo { temp_c, ..Meteo::default() },
        pos,
    })
}

struct Collect {
    bits: Vec<u8>,
    invert: bool,
}

pub struct Mts01Decoder {
    slicer: FskSlicer,
    sr: u32,
    frame: Option<Collect>,
}

impl Mts01Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        // A narrow signal with long runs of zero bytes: the min/max threshold
        // follows it, a mean would drift on the runs.
        let slicer = FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?
            .with_channel_lowpass(1500.0, input_rate)
            .with_hold(4.0)
            .with_smoothing(30.0 / 96_000.0, input_rate);
        Ok(Self { slicer, sr: 0, frame: None })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        if let Some(f) = self.frame.as_mut() {
            f.bits.push(r ^ u8::from(f.invert));
            if f.bits.len() == BODY_BYTES * 8 {
                let f = self.frame.take().unwrap();
                let body: Vec<u8> =
                    f.bits.chunks(8).map(|c| c.iter().fold(0u8, |a, &x| (a << 1) | x)).collect();
                out.extend(fix_of(&body));
            }
            return;
        }
        self.sr = (self.sr << 1) | u32::from(r);
        let straight = (self.sr ^ HEADER).count_ones() <= HEADER_TOLERANCE;
        let inverted = (self.sr ^ !HEADER).count_ones() <= HEADER_TOLERANCE;
        if straight || inverted {
            self.frame =
                Some(Collect { bits: Vec::with_capacity(BODY_BYTES * 8), invert: !straight });
        }
    }
}

impl ModelDecoder for Mts01Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::Mts01
    }

    fn process(&mut self, input: &[Complex32]) -> Vec<SondeFix> {
        let mut out = Vec::new();
        let mut syms = Vec::new();
        self.slicer.process(input, |b| syms.push(b));
        for s in syms {
            self.symbol(s, &mut out);
        }
        out
    }

    fn level_dbfs(&self) -> f32 {
        self.slicer.level_dbfs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str =
        "A1234567,0,123,251106123456,3700,48.123456,2.654321,12345.6,270.5,12.3,0,15,15,800,";

    fn body(text: &str) -> Vec<u8> {
        let mut t = text.as_bytes().to_vec();
        t.resize(TEXT_BYTES, 0);
        let mut b = vec![0x80];
        b.extend(&t);
        b.extend(crc16(&t).to_le_bytes());
        b
    }

    /// The text and CRC of a frame of the real recording.
    const REAL_TEXT: &str = "31303031303336352c332c313235312c3232313131303230313232372c343830302c34302e3932333138332c32392e3330323132382c3132352c302e302c302e302c392c372e32352c372e32352c3234322e39332c352c332c312c30003000000000000000000000000000000000000000000000000000000000000000000000";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn the_crc_of_a_real_frame_matches() {
        let t = unhex(REAL_TEXT);
        assert_eq!(t.len(), TEXT_BYTES);
        assert_eq!(crc16(&t), 0x8995);
    }

    #[test]
    fn a_real_frame_gives_serial_place_and_temperature() {
        let mut b = vec![0x80];
        b.extend(unhex(REAL_TEXT));
        b.extend(0x8995u16.to_le_bytes());
        let f = fix_of(&b).expect("a fix");
        assert_eq!(f.serial, "MTS01-10010365");
        assert_eq!(f.frame_num, 1251);
        assert!((f.battery_v.unwrap() - 4.8).abs() < 1e-6);
        let p = f.pos.expect("a place");
        assert!(
            (p.lat - 40.923183).abs() < 1e-9
                && (p.lon - 29.302128).abs() < 1e-9
                && (p.alt_m - 125.0).abs() < 1e-9
        );
        // 7.25 on the thermistor scale: a mild day, not a freezing one.
        let t = f.meteo.temp_c.unwrap();
        assert!((10.0..30.0).contains(&t), "{t}");
    }

    #[test]
    fn a_frame_gives_serial_place_battery_and_temperature() {
        let f = fix_of(&body(TEXT)).expect("a fix");
        assert_eq!(f.serial, "MTS01-A1234567");
        assert_eq!(f.frame_num, 123);
        assert!((f.battery_v.unwrap() - 3.7).abs() < 1e-6);
        let p = f.pos.expect("a place");
        assert!((p.lat - 48.123456).abs() < 1e-9 && (p.lon - 2.654321).abs() < 1e-9);
        assert!(
            (p.alt_m - 12345.6).abs() < 1e-9
                && (p.speed_ms - 12.3).abs() < 1e-9
                && (p.course_deg - 270.5).abs() < 1e-9
        );
        // 15 is the thermistor value at 0 degrees C.
        assert!(f.meteo.temp_c.unwrap().abs() < 0.01);
    }

    #[test]
    fn the_thermistor_gets_colder_with_a_higher_reading() {
        let warm = thermistor_c(10.0).unwrap();
        let cold = thermistor_c(40.0).unwrap();
        assert!(warm > 0.0 && cold < 0.0, "{warm} {cold}");
        assert_eq!(thermistor_c(0.0), None);
    }

    #[test]
    fn a_flipped_bit_fails_the_crc() {
        let mut b = body(TEXT);
        b[20] ^= 0x01;
        assert!(fix_of(&b).is_none());
    }

    #[test]
    fn a_place_off_the_earth_is_dropped_but_the_serial_stays() {
        let f = fix_of(&body("A1,0,1,251106123456,3700,95.0,2.0,100,0,0,0,15,15,0,")).unwrap();
        assert!(f.pos.is_none());
        assert_eq!(f.serial, "MTS01-A1");
    }

    /// Plain bits, most significant first.
    fn symbols(bytes: &[u8]) -> Vec<u8> {
        bytes.iter().flat_map(|&b| (0..8).rev().map(move |k| b >> k & 1)).collect()
    }

    fn modulate(sym: &[u8], rate: f64, invert: bool) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if (r == 1) != invert { 600.0 } else { -600.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    #[test]
    fn a_synthetic_burst_is_decoded_at_both_rates_and_both_polarities() {
        for rate in [48_000.0, 96_000.0] {
            for invert in [false, true] {
                let mut bytes = vec![0xAA; 6];
                for _ in 0..3 {
                    bytes.extend([0xAA, 0xAA, 0xB4, 0x2B]);
                    bytes.extend(body(TEXT));
                    bytes.extend([0xAA, 0xAA]);
                }
                let mut iq = vec![Complex32::new(0.0, 0.0); 3000];
                iq.extend(modulate(&symbols(&bytes), rate, invert));
                iq.extend(vec![Complex32::new(0.0, 0.0); 3000]);
                let mut d = Mts01Decoder::new(rate, 0.0).unwrap();
                let mut fixes = Vec::new();
                for chunk in iq.chunks(1000) {
                    fixes.extend(d.process(chunk));
                }
                assert!(fixes.len() >= 2, "{} fixes at {rate}, invert {invert}", fixes.len());
                assert_eq!(fixes[0].serial, "MTS01-A1234567");
                assert!((fixes[0].pos.as_ref().unwrap().lat - 48.123456).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = Mts01Decoder::new(48_000.0, 0.0).unwrap();
        let mut x = 31337u32;
        let noise: Vec<Complex32> = (0..48_000 * 4)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                let a = (x >> 8) as f32 / 8_388_608.0 - 1.0;
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                Complex32::new(a, (x >> 8) as f32 / 8_388_608.0 - 1.0)
            })
            .collect();
        assert!(d.process(&noise).is_empty());
    }
}
