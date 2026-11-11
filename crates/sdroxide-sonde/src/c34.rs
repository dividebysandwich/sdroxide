//! Shanghai Changwang C34 and C50 decoder.
//!
//! The signal: an FM carrier modulated by audio tones at 2400 baud, 3000 Hz
//! for a 1 and 4800 Hz for a 0 (AFSK). The bits make 8-byte packets: `00 FF`,
//! a packet id, four data bytes (big-endian), and a two-byte Fletcher check
//! (the sum of the id and data bytes, then the complement of the sum of
//! their running sums). Bytes are sent least significant bit first. The C34
//! sends each byte behind the four bits `1110`; the C50 uses plain 8N1
//! characters.
//!
//! Ids used: `14` date (ddmmyy), `15` time (hhmmss), `16` and `17` latitude
//! and longitude in NMEA form (`ddmm.mmmm` times 1e4 for the C34, times 1e5
//! for the C50), `18` altitude in decimetres, `64` the 16-bit serial number,
//! `03` the temperature as a float and `10` the humidity as a float (C50).
//! A fix is made when the altitude arrives and the other four fields were
//! read less than a second before.
//!
//! Provenance: the layout is the public description of the C34 and C50
//! telemetry by rs1729/RS (`c34/`). No code of auto_rx, SDRangel or rs1729
//! (GPL) is used. Checked on the example packets of that description (the
//! check bytes match) and on synthetic signals built from them; not on a
//! radio recording. Some C50 sondes of 2024 scramble the position with a mask
//! derived from the time; those give no fix (the place fails its range
//! check).

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 2400.0;
/// Tone detectors: a little off the nominal 3000 and 4800 Hz, as the
/// description does, to sit between the sondes' tolerances.
const F_ONE: f64 = 2900.0;
const F_ZERO: f64 = 4720.0;
/// `00 FF` as the C34 sends it (with the next byte's `1110`), and as the C50.
const HEADER_C34: &str = "1110000000001110111111111110";
const HEADER_C50: &str = "00000000010111111111";
/// Bits after the header: seven bytes of 12 (less the 4 of the first, in the
/// header) and of 10.
const BODY_C34: usize = 7 * 12 - 4;
const BODY_C50: usize = 7 * 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    C34,
    C50,
}

fn pattern(s: &str) -> (u64, u32) {
    (u64::from_str_radix(s, 2).unwrap(), s.len() as u32)
}

/// The Fletcher check of the id and four data bytes.
fn check(b: &[u8]) -> u16 {
    let sum1: u32 = b.iter().map(|&x| u32::from(x)).sum();
    let sum2: u32 = b.iter().enumerate().map(|(i, &x)| (b.len() - i) as u32 * u32::from(x)).sum();
    ((sum1 & 0xFF) << 8 | (!sum2 & 0xFF)) as u16
}

/// The seven bytes (id, data, check) of a packet from the bits after its
/// header.
fn packet_of(kind: Kind, bits: &[u8]) -> Option<[u8; 7]> {
    let mut b = [0u8; 7];
    for (j, byte) in b.iter_mut().enumerate() {
        let at = match kind {
            Kind::C34 => 12 * j,
            Kind::C50 => 10 * j + 1,
        };
        *byte = (0..8).fold(0u8, |a, k| a | (bits[at + k] << k));
    }
    (u16::from(b[5]) << 8 | u16::from(b[6]) == check(&b[..5])).then_some(b)
}

fn nmea(v: i32, scale: i32) -> f64 {
    let a = v.unsigned_abs() as i64;
    let deg = a / (100 * i64::from(scale));
    let min = (a - deg * 100 * i64::from(scale)) as f64 / f64::from(scale);
    (deg as f64 + min / 60.0) * f64::from(v.signum())
}

#[derive(Default)]
struct State {
    date: Option<u64>,
    time: Option<(u64, u32)>,
    lat: Option<(u64, f64)>,
    lon: Option<(u64, f64)>,
    serial: Option<u32>,
    temp: Option<f32>,
    humidity: Option<f32>,
}

pub struct C34Decoder {
    slicer: FskSlicer,
    rate: f64,
    bitlen: f64,
    /// Tone correlators: the products with the two tones over the window.
    win: usize,
    pos: usize,
    prod: [Vec<(f64, f64)>; 2],
    sum: [(f64, f64); 2],
    step: [f64; 2],
    n: u64,
    smooth: [f64; 3],
    cur: u8,
    changed_at: u64,
    sr: u64,
    body: Option<(Kind, Vec<u8>)>,
    state: State,
}

impl C34Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        let bitlen = input_rate / BAUD;
        let win = (1.3 * bitlen).round() as usize;
        let w = |f: f64| 2.0 * std::f64::consts::PI * f / input_rate;
        Ok(Self {
            slicer: FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Mean)?,
            rate: input_rate,
            bitlen,
            win,
            pos: 0,
            prod: [vec![(0.0, 0.0); win], vec![(0.0, 0.0); win]],
            sum: [(0.0, 0.0); 2],
            step: [w(F_ONE), w(F_ZERO)],
            n: 0,
            smooth: [0.0; 3],
            cur: 1,
            changed_at: 0,
            sr: u64::MAX,
            body: None,
            state: State::default(),
        })
    }

    /// One audio sample: update the two tone correlators and, when the
    /// stronger tone changes, hand the finished run of bits on.
    fn audio(&mut self, y: f32, out: &mut Vec<SondeFix>) {
        let n = self.n;
        self.n += 1;
        for k in 0..2 {
            let ph = self.step[k] * (n % 1_000_000_007) as f64;
            let v = (f64::from(y) * ph.cos(), -f64::from(y) * ph.sin());
            let old = std::mem::replace(&mut self.prod[k][self.pos], v);
            self.sum[k].0 += v.0 - old.0;
            self.sum[k].1 += v.1 - old.1;
        }
        self.pos = (self.pos + 1) % self.win;
        let mag = |s: (f64, f64)| s.0.hypot(s.1);
        self.smooth[(n % 3) as usize] = mag(self.sum[0]) - mag(self.sum[1]);
        if n < self.win as u64 + 3 {
            return;
        }
        let bit = u8::from(self.smooth.iter().sum::<f64>() >= 0.0);
        if bit != self.cur {
            let run = ((n - self.changed_at) as f64 / self.bitlen + 0.5) as usize;
            for _ in 0..run.min(64) {
                self.push(self.cur, out);
            }
            self.cur = bit;
            self.changed_at = n;
        }
    }

    fn push(&mut self, b: u8, out: &mut Vec<SondeFix>) {
        if let Some((kind, bits)) = self.body.as_mut() {
            bits.push(b);
            let need = if *kind == Kind::C34 { BODY_C34 } else { BODY_C50 };
            if bits.len() == need {
                let (kind, bits) = self.body.take().unwrap();
                if let Some(p) = packet_of(kind, &bits) {
                    self.packet(kind, p, out);
                }
            }
            return;
        }
        self.sr = (self.sr << 1) | u64::from(b);
        // The short C50 header must match exactly: a longer tolerance finds
        // it in the idle ones and the pauses of the C34.
        for (kind, h, tolerance) in [(Kind::C34, HEADER_C34, 1), (Kind::C50, HEADER_C50, 0)] {
            let (pat, len) = pattern(h);
            let mask = (1u64 << len) - 1;
            if (self.sr & mask ^ pat).count_ones() <= tolerance {
                self.body = Some((kind, Vec::with_capacity(BODY_C34)));
                return;
            }
        }
    }

    fn packet(&mut self, kind: Kind, p: [u8; 7], out: &mut Vec<SondeFix>) {
        let val = u32::from_be_bytes([p[1], p[2], p[3], p[4]]);
        let now = self.n;
        let scale = if kind == Kind::C34 { 10_000 } else { 100_000 };
        match p[0] {
            0x14 => self.state.date = Some(now),
            0x15 => {
                self.state.time =
                    Some((now, (val / 10_000) * 3600 + (val / 100 % 100) * 60 + val % 100))
            }
            0x16 => self.state.lat = Some((now, nmea(val as i32, scale))),
            0x17 => self.state.lon = Some((now, nmea(val as i32, scale))),
            0x64 => self.state.serial = Some(val & 0xFFFF),
            0x03 => {
                let t = f32::from_bits(val);
                self.state.temp = (-273.0..100.0).contains(&t).then_some(t);
            }
            0x10 => {
                let rh = f32::from_bits(val);
                self.state.humidity = (-0.4..110.0).contains(&rh).then(|| rh.max(0.0));
            }
            0x18 => {
                let fresh = |t: u64| now.saturating_sub(t) < self.rate as u64;
                let (Some(d), Some((t, sec)), Some((la, lat)), Some((lo, lon)), Some(sn)) = (
                    self.state.date,
                    self.state.time,
                    self.state.lat,
                    self.state.lon,
                    self.state.serial,
                ) else {
                    return;
                };
                let alt = f64::from(val as i32) / 10.0;
                if !(fresh(d) && fresh(t) && fresh(la) && fresh(lo))
                    || !(-90.0..=90.0).contains(&lat)
                    || !(-180.0..=180.0).contains(&lon)
                    || !(-500.0..60_000.0).contains(&alt)
                    || (lat == 0.0 && lon == 0.0)
                {
                    return;
                }
                let prefix = if kind == Kind::C34 { "C34" } else { "C50" };
                out.push(SondeFix {
                    model: SondeModel::C34,
                    serial: format!("{prefix}-{sn}"),
                    frame_num: sec,
                    battery_v: None,
                    meteo: Meteo {
                        temp_c: self.state.temp,
                        humidity_pct: self.state.humidity,
                        ..Meteo::default()
                    },
                    pos: Some(GpsPos {
                        lat,
                        lon,
                        alt_m: alt,
                        speed_ms: 0.0,
                        course_deg: 0.0,
                        climb_ms: 0.0,
                        num_sv: 0,
                    }),
                });
            }
            _ => {}
        }
    }
}

impl ModelDecoder for C34Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::C34
    }

    fn process(&mut self, input: &[Complex32]) -> Vec<SondeFix> {
        let mut out = Vec::new();
        let mut audio = Vec::with_capacity(input.len());
        self.slicer.process_fm(input, |y| audio.push(y));
        for y in audio {
            self.audio(y, &mut out);
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

    #[test]
    fn the_check_of_the_description_matches() {
        // 15 0001E5FE F9CB, and the one after, 15 0001E5FF FACA.
        assert_eq!(check(&[0x15, 0x00, 0x01, 0xE5, 0xFE]), 0xF9CB);
        assert_eq!(check(&[0x15, 0x00, 0x01, 0xE5, 0xFF]), 0xFACA);
        assert_eq!(check(&[0x15, 0x00, 0x01, 0xE6, 0x00]), 0xFCC7);
    }

    #[test]
    fn nmea_coordinates_are_read_with_the_scale_of_the_model() {
        // 47422619 -> 47 deg 42.2619 min (C34); 465659787 -> 46 deg 56.59787 min (C50).
        assert!((nmea(47_422_619, 10_000) - 47.704365).abs() < 1e-6);
        assert!((nmea(465_659_787, 100_000) - 46.943298).abs() < 1e-6);
        assert!((nmea(-47_422_619, 10_000) + 47.704365).abs() < 1e-6);
    }

    fn packet(id: u8, val: u32) -> Vec<u8> {
        let mut b = vec![id];
        b.extend(val.to_be_bytes());
        let c = check(&b);
        b.extend(c.to_be_bytes());
        b
    }

    /// The bits of a stream of packets (`00 FF` first), least significant bit
    /// first, as the C34 or the C50 frames the bytes.
    fn bits(kind: Kind, packets: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![1u8; 30];
        for p in packets {
            let mut bytes = vec![0x00, 0xFF];
            bytes.extend(p);
            for b in bytes {
                let data = (0..8).map(move |k| b >> k & 1);
                match kind {
                    Kind::C34 => {
                        out.extend([1, 1, 1, 0]);
                        out.extend(data);
                    }
                    Kind::C50 => {
                        out.push(0);
                        out.extend(data);
                        out.push(1);
                    }
                }
            }
            out.extend([1; 6]);
        }
        out
    }

    /// Tones of 3000 Hz (1) and 4800 Hz (0) on an FM carrier.
    fn modulate(bits: &[u8], rate: f64) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let (mut tone, mut ph) = (0f64, 0f64);
        for (k, &b) in bits.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if b == 1 { 3000.0 } else { 4800.0 };
            for _ in 0..n {
                tone += 2.0 * std::f64::consts::PI * f / rate;
                ph += 2.0 * std::f64::consts::PI * 3000.0 * tone.sin() / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    fn run(kind: Kind, rate: f64) -> Vec<SondeFix> {
        let (lat, lon) =
            if kind == Kind::C34 { (47_422_619, 7_124_483) } else { (465_659_787, 64_311_199) };
        let packets = vec![
            packet(0x64, 1234),
            packet(0x03, (-45.5f32).to_bits()),
            packet(0x14, 160_115),
            packet(0x15, 124_735),
            packet(0x16, lat),
            packet(0x17, lon),
            packet(0x18, 305_953),
            // The run of bits behind the last packet is only known at the next change of tone.
            packet(0x63, 0),
        ];
        let mut iq = vec![Complex32::new(0.0, 0.0); 2000];
        iq.extend(modulate(&bits(kind, &packets), rate));
        iq.extend(vec![Complex32::new(0.0, 0.0); 2000]);
        let mut d = C34Decoder::new(rate, 0.0).unwrap();
        let mut fixes = Vec::new();
        for chunk in iq.chunks(1000) {
            fixes.extend(d.process(chunk));
        }
        fixes
    }

    #[test]
    fn a_c34_burst_gives_the_place_of_the_description() {
        for rate in [48_000.0, 96_000.0] {
            let fixes = run(Kind::C34, rate);
            assert_eq!(fixes.len(), 1, "{} fixes at {rate}", fixes.len());
            let f = &fixes[0];
            assert_eq!(f.serial, "C34-1234");
            assert_eq!(f.frame_num, 12 * 3600 + 47 * 60 + 35);
            let p = f.pos.as_ref().unwrap();
            assert!((p.lat - 47.704365).abs() < 1e-5 && (p.alt_m - 30_595.3).abs() < 1e-6, "{p:?}");
            assert!((f.meteo.temp_c.unwrap() + 45.5).abs() < 1e-6);
        }
    }

    #[test]
    fn a_c50_burst_gives_the_place_of_the_description() {
        for rate in [48_000.0, 96_000.0] {
            let fixes = run(Kind::C50, rate);
            assert_eq!(fixes.len(), 1, "{} fixes at {rate}", fixes.len());
            assert_eq!(fixes[0].serial, "C50-1234");
            let p = fixes[0].pos.as_ref().unwrap();
            assert!((p.lat - 46.943298).abs() < 1e-5, "{p:?}");
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = C34Decoder::new(48_000.0, 0.0).unwrap();
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
