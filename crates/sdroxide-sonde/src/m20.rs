//! Meteomodem M20 decoder.
//!
//! The signal: FSK at 9600 symbols/s, Manchester coded with a differential
//! data bit: the two symbols of a pair always differ and give a level, and the
//! data bit is 1 when that level equals the previous one. A frame is 70 bytes
//! (length byte 0x45; a 0x43 variant is 68), most significant bit first:
//! `45 20`, telemetry, a 16-bit checksum over all the bytes before it (the
//! same sum as the M10's). GPS fields are big-endian: latitude and longitude
//! in 1e-6 degrees, altitude in 24 bits of 1/100 m, velocities in 1/100 m/s.
//!
//! Provenance: the layout is the public description of the M20 telemetry by
//! rs1729/RS; the line coding and the sync word were checked on a real
//! recording (projecthorus sample set, a sonde on the ground at Adelaide):
//! 60 frames in 60 s, all pass the checksum, positions are a plausible fixed
//! place and a slowly changing altitude. The M20 has no serial number that
//! the description reads, so the registry names it by its channel. No code of
//! auto_rx, SDRangel or rs1729 (GPL) is used.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::fsk::{FskSlicer, Threshold};
use crate::m10::checksum;
use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 9600.0;
/// 32 raw symbols that end the preamble; the frame follows at once.
const SYNC: u32 = 0b1001_1001_1001_1001_0100_1100_1001_1001;
const SYNC_TOLERANCE: u32 = 2;
const MAX_VIOLATIONS: u32 = 3;
const MAX_BYTES: usize = 0x46;

fn be_i32(b: &[u8], o: usize) -> i32 {
    i32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn be_i16(b: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([b[o], b[o + 1]])
}

fn be_i24(b: &[u8], o: usize) -> i32 {
    let v = (i32::from(b[o]) << 16) | (i32::from(b[o + 1]) << 8) | i32::from(b[o + 2]);
    if v >> 23 == 1 { v - (1 << 24) } else { v }
}

/// A place the telemetry can really mean; the defaults of a GPS without fix
/// (latitude 0 and longitude 0, or the poles) are not.
fn position(b: &[u8]) -> Option<GpsPos> {
    let lat = f64::from(be_i32(b, 0x1C)) / 1e6;
    let lon = f64::from(be_i32(b, 0x20)) / 1e6;
    let alt = f64::from(be_i24(b, 0x08)) / 100.0;
    let ok = lat.abs() < 89.9
        && lon.abs() <= 180.0
        && !(lat.abs() < 1e-6 && lon.abs() < 1e-6)
        && (-500.0..60_000.0).contains(&alt);
    if !ok {
        return None;
    }
    let ve = f64::from(be_i16(b, 0x0B)) / 100.0;
    let vn = f64::from(be_i16(b, 0x0D)) / 100.0;
    let vu = f64::from(be_i16(b, 0x18)) / 100.0;
    Some(GpsPos {
        lat,
        lon,
        alt_m: alt,
        speed_ms: ve.hypot(vn),
        course_deg: ve.atan2(vn).to_degrees().rem_euclid(360.0),
        climb_ms: vu,
        num_sv: 0,
    })
}

/// A frame being collected after a sync word.
struct Collect {
    prev: u8,
    pending: Option<u8>,
    bits: Vec<u8>,
    violations: u32,
    /// Total bytes once the length byte is known.
    total: Option<usize>,
}

pub struct M20Decoder {
    slicer: FskSlicer,
    sr: u32,
    frame: Option<Collect>,
}

impl M20Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        Ok(Self {
            slicer: FskSlicer::new(input_rate, freq_offset_hz, BAUD, Threshold::Midrange)?,
            sr: 0,
            frame: None,
        })
    }

    fn symbol(&mut self, r: u8, out: &mut Vec<SondeFix>) {
        if let Some(f) = self.frame.as_mut() {
            let Some(first) = f.pending.take() else {
                f.pending = Some(r);
                return;
            };
            if first == r {
                f.violations += 1;
                if f.violations > MAX_VIOLATIONS {
                    self.frame = None;
                    return;
                }
            }
            f.bits.push(u8::from(first == f.prev));
            f.prev = first;
            if f.total.is_none() && f.bits.len() == 8 {
                let len = f.bits.iter().fold(0u8, |a, &b| (a << 1) | b);
                if len != 0x45 && len != 0x43 {
                    self.frame = None;
                    return;
                }
                f.total = Some(usize::from(len) + 1);
            }
            if let Some(total) = f.total
                && f.bits.len() == total * 8
            {
                let f = self.frame.take().unwrap();
                if let Some(fix) = decode_frame(&f.bits) {
                    out.push(fix);
                }
            }
            return;
        }
        self.sr = (self.sr << 1) | u32::from(r);
        if (self.sr ^ SYNC).count_ones() <= SYNC_TOLERANCE
            || (self.sr ^ !SYNC).count_ones() <= SYNC_TOLERANCE
        {
            self.frame = Some(Collect {
                // The first symbol of the last sync pair is the level the
                // first data bit is compared with.
                prev: ((self.sr >> 1) & 1) as u8,
                pending: None,
                bits: Vec::with_capacity(MAX_BYTES * 8),
                violations: 0,
                total: None,
            });
        }
    }
}

fn decode_frame(bits: &[u8]) -> Option<SondeFix> {
    let b: Vec<u8> = bits.chunks(8).map(|c| c.iter().fold(0u8, |a, &x| (a << 1) | x)).collect();
    let n = usize::from(b[0]);
    if b.len() != n + 1 || b[1] != 0x20 {
        return None;
    }
    let stored = u16::from_be_bytes([b[n - 1], b[n]]);
    if checksum(&b[..n - 1]) != stored {
        return None;
    }
    Some(SondeFix {
        model: SondeModel::M20,
        // No serial number is read from the frame: the registry names the
        // sonde by its channel.
        serial: String::new(),
        frame_num: u32::from(b[0x15]),
        battery_v: None,
        meteo: Meteo::default(),
        pos: position(&b),
    })
}

impl ModelDecoder for M20Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::M20
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

    /// Frame 0 of the real recording (70 bytes).
    const REAL: &str = "4520eb58a405c10100244a0001000007fb4cf634044b0590ffff08abfdee33ef08444895283fd44501100000fd34227254f609e3288e0000bbe21d580020005e0199f7043302";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    fn with_checksum(mut f: Vec<u8>) -> Vec<u8> {
        let n = f.len() - 2;
        let c = checksum(&f[..n]);
        f[n..].copy_from_slice(&c.to_be_bytes());
        f
    }

    #[test]
    fn a_real_frame_passes_its_checksum_and_has_a_place() {
        let f = unhex(REAL);
        assert_eq!(f.len(), 70);
        assert_eq!(checksum(&f[..68]), u16::from_be_bytes([f[68], f[69]]));
        let p = position(&f).expect("a real fix");
        assert!((p.lat + 34.7208).abs() < 1e-3 && (p.lon - 138.6928).abs() < 1e-3);
        assert!((p.alt_m - 92.9).abs() < 0.05);
    }

    #[test]
    fn a_flipped_bit_fails_the_checksum() {
        let mut f = unhex(REAL);
        f[0x1C] ^= 0x01;
        assert_ne!(checksum(&f[..68]), u16::from_be_bytes([f[68], f[69]]));
    }

    fn frame_with_position(counter: u8) -> Vec<u8> {
        let mut f = vec![0u8; 70];
        f[0] = 0x45;
        f[1] = 0x20;
        f[0x08..0x0B].copy_from_slice(&(1_234_500_i32).to_be_bytes()[1..]);
        f[0x0B..0x0D].copy_from_slice(&300i16.to_be_bytes());
        f[0x0D..0x0F].copy_from_slice(&400i16.to_be_bytes());
        f[0x15] = counter;
        f[0x18..0x1A].copy_from_slice(&500i16.to_be_bytes());
        f[0x1C..0x20].copy_from_slice(&48_850_000i32.to_be_bytes());
        f[0x20..0x24].copy_from_slice(&2_350_000i32.to_be_bytes());
        with_checksum(f)
    }

    /// The sync word as on air, then the frame: Manchester pairs whose level
    /// repeats for a 1 and changes for a 0.
    fn symbols(bytes: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = (0..32).rev().map(|i| ((SYNC >> i) & 1) as u8).collect();
        let mut level = out[30];
        for &byte in bytes {
            for k in (0..8).rev() {
                if byte >> k & 1 == 0 {
                    level ^= 1;
                }
                out.push(level);
                out.push(level ^ 1);
            }
        }
        out
    }

    fn modulate(sym: &[u8], rate: f64) -> Vec<Complex32> {
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let mut ph = 0f64;
        for (k, &r) in sym.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let f = if r == 1 { 5000.0 } else { -5000.0 };
            for _ in 0..n {
                ph += 2.0 * std::f64::consts::PI * f / rate;
                out.push(Complex32::new(ph.cos() as f32, ph.sin() as f32));
            }
        }
        out
    }

    fn burst(counter: u8, rate: f64) -> Vec<Complex32> {
        let mut iq = vec![Complex32::new(0.0, 0.0); 3000];
        iq.extend(modulate(&symbols(&frame_with_position(counter)), rate));
        iq.extend(vec![Complex32::new(0.0, 0.0); 3000]);
        iq
    }

    #[test]
    fn a_synthetic_burst_gives_a_position() {
        for rate in [48_000.0, 96_000.0] {
            let mut d = M20Decoder::new(rate, 0.0).unwrap();
            let mut fixes = Vec::new();
            for counter in 1..4u8 {
                for chunk in burst(counter, rate).chunks(1000) {
                    fixes.extend(d.process(chunk));
                }
            }
            assert!(!fixes.is_empty(), "no fix at {rate}");
            let f = &fixes[0];
            assert_eq!(f.model, SondeModel::M20);
            let p = f.pos.as_ref().expect("a position");
            assert!((p.lat - 48.85).abs() < 1e-6 && (p.lon - 2.35).abs() < 1e-6);
            assert!((p.alt_m - 12_345.0).abs() < 1e-6);
            assert!((p.speed_ms - 5.0).abs() < 1e-6, "3 east and 4 north make 5");
            assert!((p.climb_ms - 5.0).abs() < 1e-6);
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = M20Decoder::new(48_000.0, 0.0).unwrap();
        let mut x = 4242u32;
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
