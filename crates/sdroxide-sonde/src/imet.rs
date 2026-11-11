//! InterMet iMet-4 decoder.
//!
//! The signal: a narrow FM carrier modulated by Bell 202 style audio tones,
//! 1200 baud, 1200 Hz for a 1 (mark) and 2200 Hz for a 0 (space). The bits
//! are plain UART bytes (start bit 0, eight data bits least significant bit
//! first, stop bit 1; the line idles at 1). The bytes form packets
//! `01 <id> ... <crc>`, little-endian numbers, a CRC-16 (polynomial 0x1021,
//! start value 0x1D0F, no reflection) stored big-endian after the data:
//! PTU (id 1, 14 bytes), GPS (2, 18), XDATA (3), ePTU (4, 20), eGPS (5, 30).
//!
//! Provenance: the packet layout and the CRC are the public description of
//! the iMet-4 telemetry by rs1729/RS. The tone detection and the UART framing
//! were checked on a real recording (projecthorus sample set, 120 s): 40 s
//! of it decodes to 79 packets, all with a valid CRC, consecutive frame
//! numbers and consecutive GPS seconds. The positions in that recording are
//! those of a GPS simulator (a sonde at ground pressure reporting 10 km), so
//! they prove the decoding but not a real place. The iMet-4 frame carries no
//! serial number; the registry names it by its channel. No code of auto_rx,
//! SDRangel or rs1729 (GPL) is used.

use sdroxide_dsp::Complex32;
use xng_mode_sonde::GpsPos;

use crate::model::{Meteo, ModelDecoder, SondeFix, SondeModel};

const BAUD: f64 = 1200.0;
const MARK_HZ: f64 = 1200.0;
const SPACE_HZ: f64 = 2200.0;
const MAX_BUFFER: usize = 80;

/// CRC-16, polynomial 0x1021, start 0x1D0F, most significant bit first.
pub fn crc16(data: &[u8]) -> u16 {
    let mut c: u16 = 0x1D0F;
    for &b in data {
        c ^= u16::from(b) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
    }
    c
}

/// Length of the packet that starts with `01 id`, when it is known from the id
/// (XDATA also needs its own length byte).
fn packet_len(buf: &[u8]) -> Option<usize> {
    match buf[1] {
        1 => Some(14),
        2 => Some(18),
        4 => Some(20),
        5 => Some(30),
        3 => buf.get(2).map(|&n| usize::from(n) + 5),
        _ => None,
    }
}

fn le_f32(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// What the packets said so far, folded into fixes.
#[derive(Default)]
struct Assembler {
    frame: u32,
    battery_v: Option<f32>,
    meteo: Meteo,
}

impl Assembler {
    fn packet(&mut self, p: &[u8], out: &mut Vec<SondeFix>) {
        match p[1] {
            // PTU and ePTU share the first fields.
            1 | 4 => {
                self.frame = u32::from(u16::from_le_bytes([p[2], p[3]]));
                let v = f32::from(p[0x0B]) / 10.0;
                self.battery_v = (v > 0.0).then_some(v);
                // Pressure in 1/100 hPa (24 bits), temperature and humidity in
                // 1/100 °C and 1/100 % (16 bits each).
                let press = u32::from(p[4]) | u32::from(p[5]) << 8 | u32::from(p[6]) << 16;
                let temp = i16::from_le_bytes([p[7], p[8]]);
                let hum = u16::from_le_bytes([p[9], p[10]]);
                self.meteo = Meteo {
                    temp_c: Some(f32::from(temp) / 100.0),
                    humidity_pct: Some(f32::from(hum) / 100.0),
                    pressure_hpa: Some(press as f32 / 100.0),
                };
            }
            2 | 5 => {
                let lat = f64::from(le_f32(p, 2));
                let lon = f64::from(le_f32(p, 6));
                let alt = f64::from(u16::from_le_bytes([p[10], p[11]])) - 5000.0;
                let (ve, vn, vu) = if p[1] == 5 {
                    (
                        f64::from(le_f32(p, 0x0D)),
                        f64::from(le_f32(p, 0x11)),
                        f64::from(le_f32(p, 0x15)),
                    )
                } else {
                    (0.0, 0.0, 0.0)
                };
                let ok = lat.is_finite()
                    && lon.is_finite()
                    && lat.abs() <= 90.0
                    && lon.abs() <= 180.0
                    && !(lat.abs() < 1e-6 && lon.abs() < 1e-6)
                    && (-500.0..60_000.0).contains(&alt);
                out.push(SondeFix {
                    model: SondeModel::Imet4,
                    serial: String::new(),
                    frame_num: self.frame,
                    battery_v: self.battery_v,
                    meteo: self.meteo,
                    pos: ok.then(|| GpsPos {
                        lat,
                        lon,
                        alt_m: alt,
                        speed_ms: ve.hypot(vn),
                        course_deg: ve.atan2(vn).to_degrees().rem_euclid(360.0),
                        climb_ms: vu,
                        num_sv: p[12],
                    }),
                });
            }
            _ => {}
        }
    }
}

/// A sliding-window correlator against one tone.
struct Tone {
    step: f64,
    phase: f64,
    ring: Vec<(f32, f32)>,
    sum: (f32, f32),
}

impl Tone {
    fn new(hz: f64, rate: f64, window: usize) -> Self {
        Tone {
            step: 2.0 * std::f64::consts::PI * hz / rate,
            phase: 0.0,
            ring: vec![(0.0, 0.0); window],
            sum: (0.0, 0.0),
        }
    }

    /// Push one sample (index `n`); returns the energy over the window.
    fn push(&mut self, a: f32, n: usize) -> f32 {
        let (s, c) = self.phase.sin_cos();
        self.phase += self.step;
        if self.phase > std::f64::consts::TAU {
            self.phase -= std::f64::consts::TAU;
        }
        let z = (a * c as f32, -a * s as f32);
        let slot = n % self.ring.len();
        let old = self.ring[slot];
        self.ring[slot] = z;
        self.sum.0 += z.0 - old.0;
        self.sum.1 += z.1 - old.1;
        self.sum.0 * self.sum.0 + self.sum.1 * self.sum.1
    }
}

enum Uart {
    Idle,
    /// Samples since the start edge, and the bits taken so far.
    Bits {
        elapsed: f64,
        next: f64,
        bits: Vec<u8>,
    },
}

pub struct Imet4Decoder {
    rate: f64,
    osc: f64,
    osc_step: f64,
    prev: Complex32,
    dc: f32,
    n: usize,
    mark: Tone,
    space: Tone,
    last_level: u8,
    uart: Uart,
    buf: Vec<u8>,
    asm: Assembler,
    level: f32,
}

impl Imet4Decoder {
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        if input_rate < 8.0 * SPACE_HZ {
            return Err(format!("input rate {input_rate} too low for iMet"));
        }
        let window = (input_rate / BAUD).round() as usize;
        Ok(Self {
            rate: input_rate,
            osc: 0.0,
            osc_step: -2.0 * std::f64::consts::PI * freq_offset_hz / input_rate,
            prev: Complex32::new(1.0, 0.0),
            dc: 0.0,
            n: 0,
            mark: Tone::new(MARK_HZ, input_rate, window),
            space: Tone::new(SPACE_HZ, input_rate, window),
            last_level: 1,
            uart: Uart::Idle,
            buf: Vec::with_capacity(MAX_BUFFER),
            asm: Assembler::default(),
            level: -120.0,
        })
    }

    fn byte(&mut self, b: u8, out: &mut Vec<SondeFix>) {
        self.buf.push(b);
        loop {
            // Drop what comes before a packet start.
            match self.buf.iter().position(|&x| x == 1) {
                Some(0) => {}
                Some(i) => {
                    self.buf.drain(..i);
                }
                None => {
                    self.buf.clear();
                    return;
                }
            }
            if self.buf.len() < 2 {
                return;
            }
            let Some(len) = packet_len(&self.buf) else {
                self.buf.remove(0);
                continue;
            };
            if !(4..=MAX_BUFFER).contains(&len) {
                self.buf.remove(0);
                continue;
            }
            if self.buf.len() < len {
                return;
            }
            let ok = crc16(&self.buf[..len - 2])
                == u16::from_be_bytes([self.buf[len - 2], self.buf[len - 1]]);
            if ok {
                let p: Vec<u8> = self.buf.drain(..len).collect();
                self.asm.packet(&p, out);
            } else {
                self.buf.remove(0);
            }
        }
    }
}

impl ModelDecoder for Imet4Decoder {
    fn model(&self) -> SondeModel {
        SondeModel::Imet4
    }

    fn process(&mut self, input: &[Complex32]) -> Vec<SondeFix> {
        let mut out = Vec::new();
        let bit = self.rate / BAUD;
        for &x in input {
            let (s, c) = self.osc.sin_cos();
            self.osc += self.osc_step;
            if self.osc > std::f64::consts::TAU {
                self.osc -= std::f64::consts::TAU;
            } else if self.osc < -std::f64::consts::TAU {
                self.osc += std::f64::consts::TAU;
            }
            let z = x * Complex32::new(c as f32, s as f32);
            self.level += 0.0005 * (10.0 * (z.norm_sqr() + 1e-12).log10() - self.level);
            // The audio is what the discriminator gives, without its mean.
            let d = (z * self.prev.conj()).arg();
            self.prev = z;
            self.dc += 0.0005 * (d - self.dc);
            let a = d - self.dc;

            let em = self.mark.push(a, self.n);
            let es = self.space.push(a, self.n);
            self.n = self.n.wrapping_add(1);
            let lvl = u8::from(em > es);

            match &mut self.uart {
                Uart::Idle => {
                    if self.last_level == 1 && lvl == 0 {
                        self.uart = Uart::Bits {
                            elapsed: 0.0,
                            next: 0.5 * bit,
                            bits: Vec::with_capacity(10),
                        };
                    }
                }
                Uart::Bits { elapsed, next, bits } => {
                    *elapsed += 1.0;
                    if *elapsed >= *next {
                        bits.push(lvl);
                        *next += bit;
                        // The start bit must still be a 0.
                        if bits.len() == 1 && lvl != 0 {
                            self.uart = Uart::Idle;
                        } else if bits.len() == 10 {
                            let ok = bits[9] == 1;
                            let byte = (0..8).fold(0u8, |a, j| a | (bits[1 + j] << j));
                            self.uart = Uart::Idle;
                            if ok {
                                self.byte(byte, &mut out);
                            }
                        }
                    }
                }
            }
            self.last_level = lvl;
        }
        out
    }

    fn level_dbfs(&self) -> f32 {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real packets from the recording: an ePTU and a GPS packet.
    const EPTU: &str = "01044b005b8701530a460f3c190b190b870a68e7";
    const GPS: &str = "0102c8c730c27b33f8420e3c0005252ff5ab";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn real_packets_pass_their_crc() {
        for h in [EPTU, GPS] {
            let p = unhex(h);
            let n = p.len();
            assert_eq!(crc16(&p[..n - 2]), u16::from_be_bytes([p[n - 2], p[n - 1]]), "{h}");
        }
    }

    #[test]
    fn a_flipped_bit_fails_the_crc() {
        let mut p = unhex(GPS);
        p[5] ^= 0x04;
        let n = p.len();
        assert_ne!(crc16(&p[..n - 2]), u16::from_be_bytes([p[n - 2], p[n - 1]]));
    }

    #[test]
    fn a_real_pair_of_packets_gives_a_fix_with_its_frame_and_battery() {
        let mut a = Assembler::default();
        let mut out = Vec::new();
        a.packet(&unhex(EPTU), &mut out);
        a.packet(&unhex(GPS), &mut out);
        assert_eq!(out.len(), 1);
        let f = &out[0];
        assert_eq!(f.frame_num, 75);
        assert!((f.battery_v.unwrap() - 6.0).abs() < 1e-6);
        let m = f.meteo;
        assert!((m.temp_c.unwrap() - 26.43).abs() < 1e-4);
        assert!((m.humidity_pct.unwrap() - 39.10).abs() < 1e-4);
        assert!((m.pressure_hpa.unwrap() - 1001.87).abs() < 1e-3);
        let p = f.pos.as_ref().unwrap();
        assert!((p.lat + 44.1951).abs() < 1e-3 && (p.lon - 124.1006).abs() < 1e-3);
        assert!((p.alt_m - 10_374.0).abs() < 1e-6);
    }

    fn with_crc(mut p: Vec<u8>) -> Vec<u8> {
        let n = p.len() - 2;
        let c = crc16(&p[..n]);
        p[n..].copy_from_slice(&c.to_be_bytes());
        p
    }

    fn gps_packet(lat: f32, lon: f32, alt: u16) -> Vec<u8> {
        let mut p = vec![0u8; 18];
        p[0] = 1;
        p[1] = 2;
        p[2..6].copy_from_slice(&lat.to_le_bytes());
        p[6..10].copy_from_slice(&lon.to_le_bytes());
        p[10..12].copy_from_slice(&alt.to_le_bytes());
        p[12] = 9;
        with_crc(p)
    }

    /// Audio bits (UART 8N1, idle high) as FM: the instantaneous frequency
    /// follows a sine at the tone of each bit.
    fn modulate(bytes: &[u8], rate: f64) -> Vec<Complex32> {
        let mut bits: Vec<u8> = vec![1; 40];
        for &b in bytes {
            bits.push(0);
            for j in 0..8 {
                bits.push(b >> j & 1);
            }
            bits.push(1);
        }
        bits.extend(vec![1u8; 40]);
        let spb = rate / BAUD;
        let mut out = Vec::new();
        let (mut ph_a, mut ph_c) = (0f64, 0f64);
        for (k, &bit) in bits.iter().enumerate() {
            let n = ((k + 1) as f64 * spb).round() as usize - (k as f64 * spb).round() as usize;
            let tone = if bit == 1 { MARK_HZ } else { SPACE_HZ };
            for _ in 0..n {
                ph_a += 2.0 * std::f64::consts::PI * tone / rate;
                ph_c += 2.0 * std::f64::consts::PI * 3000.0 * ph_a.sin() / rate;
                out.push(Complex32::new(ph_c.cos() as f32, ph_c.sin() as f32));
            }
        }
        out
    }

    #[test]
    fn synthetic_packets_give_a_position() {
        for rate in [48_000.0, 96_000.0] {
            let mut bytes = Vec::new();
            for k in 0..4 {
                bytes.extend(gps_packet(48.85 + k as f32 * 0.001, 2.35, 5000 + 12_345));
            }
            let mut d = Imet4Decoder::new(rate, 0.0).unwrap();
            let mut fixes = Vec::new();
            for chunk in modulate(&bytes, rate).chunks(1000) {
                fixes.extend(d.process(chunk));
            }
            assert!(fixes.len() >= 3, "{} fixes at {rate}", fixes.len());
            let p = fixes[0].pos.as_ref().expect("a position");
            assert!((p.lat - 48.85).abs() < 1e-4 && (p.lon - 2.35).abs() < 1e-4);
            assert!((p.alt_m - 12_345.0).abs() < 1e-6);
            assert_eq!(p.num_sv, 9);
        }
    }

    #[test]
    fn noise_decodes_nothing() {
        let mut d = Imet4Decoder::new(48_000.0, 0.0).unwrap();
        let mut x = 99u32;
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
