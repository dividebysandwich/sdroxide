//! Finding the sondes in a wide window, and deciding which channels to decode.
//!
//! Radiosondes sit on 400.15–406 MHz, a few kilohertz wide each, one per
//! balloon. A wideband receiver sees several at once. [`Accumulator`] averages
//! the spectrum of the incoming I/Q and [`Accumulator::carriers`] picks the
//! peaks that stand out of the floor; [`LaneBook`] turns the carriers heard
//! into the channels to decode (a few at a time, dropping a channel whose
//! carrier has gone). Pure data and arithmetic, so it is tested without a
//! radio.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex};
use sdroxide_dsp::Complex32;
use sdroxide_types::{SONDE_BAND_HZ, SondeTrack};

/// The most channels decoded at once.
pub const MAX_LANES: usize = 4;
/// A peak must stand this far above the median of the band, dB.
const PEAK_ABOVE_FLOOR_DB: f32 = 9.0;
/// A sonde's occupied width is about 10 kHz: peaks closer than this are one.
const MIN_SPACING_HZ: f64 = 8_000.0;
/// The spectrum is smoothed over about this width before peaks are picked.
const SMOOTH_HZ: f64 = 5_000.0;
/// A carrier within this of a channel already decoded is that channel.
const SAME_CHANNEL_HZ: f64 = 4_000.0;
/// A channel whose carrier has not been seen for this long is dropped.
pub const LANE_IDLE_S: i64 = 120;
/// Frequency resolution wanted, Hz.
const BIN_HZ: f64 = 150.0;

/// A carrier found: where, and how far above the floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Carrier {
    pub freq_hz: f64,
    pub above_floor_db: f32,
}

/// Averages the spectrum of the wide I/Q a block at a time.
pub struct Accumulator {
    n: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    sum: Vec<f32>,
    blocks: u32,
    scratch: Vec<Complex<f32>>,
}

impl Accumulator {
    /// For a stream at `rate` Hz: the transform is sized to resolve about
    /// [`BIN_HZ`].
    pub fn new(rate: f64) -> Self {
        let n = ((rate / BIN_HZ) as usize).next_power_of_two().clamp(2048, 32_768);
        let window: Vec<f32> = (0..n)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos())
            .collect();
        Self {
            n,
            fft: FftPlanner::new().plan_fft_forward(n),
            window,
            sum: vec![0.0; n],
            blocks: 0,
            scratch: vec![Complex::new(0.0, 0.0); n],
        }
    }

    /// The transform length: the smallest slice [`Self::push`] will use.
    pub fn block_len(&self) -> usize {
        self.n
    }

    /// Fold one block of I/Q in. Takes the first [`Self::block_len`] samples;
    /// a shorter slice is ignored. The caller decides how often to call.
    pub fn push(&mut self, iq: &[Complex32]) {
        if iq.len() < self.n {
            return;
        }
        for (i, s) in iq[..self.n].iter().enumerate() {
            self.scratch[i] = Complex::new(s.re * self.window[i], s.im * self.window[i]);
        }
        self.fft.process(&mut self.scratch);
        for (acc, v) in self.sum.iter_mut().zip(&self.scratch) {
            *acc += v.norm_sqr();
        }
        self.blocks += 1;
    }

    /// How many blocks have been folded in since the last [`Self::carriers`].
    pub fn blocks(&self) -> u32 {
        self.blocks
    }

    /// The carriers in the sonde band, strongest first, and start over. `rate`
    /// and `center_hz` describe the stream the blocks came from.
    pub fn carriers(&mut self, rate: f64, center_hz: f64) -> Vec<Carrier> {
        let out = if self.blocks == 0 { Vec::new() } else { self.find(rate, center_hz) };
        self.sum.iter_mut().for_each(|x| *x = 0.0);
        self.blocks = 0;
        out
    }

    fn find(&self, rate: f64, center_hz: f64) -> Vec<Carrier> {
        let n = self.n;
        let bin_hz = rate / n as f64;
        // Power in dB, rotated so index 0 is the lowest frequency.
        let db: Vec<f32> = (0..n)
            .map(|k| {
                let bin = self.sum[(k + n / 2) % n] / self.blocks as f32;
                10.0 * (bin + 1e-20).log10()
            })
            .collect();
        let freq = |k: usize| center_hz + (k as f64 - (n / 2) as f64) * bin_hz;
        // Which bins are inside the sonde band.
        let inside: Vec<usize> =
            (0..n).filter(|&k| (SONDE_BAND_HZ.0..=SONDE_BAND_HZ.1).contains(&freq(k))).collect();
        if inside.len() < 8 {
            return Vec::new();
        }
        // Smooth over a sonde's width, in power (so a narrow spike does not
        // outweigh a real signal).
        let half = ((SMOOTH_HZ / bin_hz / 2.0) as usize).max(1);
        let lin: Vec<f32> = db.iter().map(|d| 10f32.powf(d / 10.0)).collect();
        let mut prefix = vec![0.0f64; n + 1];
        for k in 0..n {
            prefix[k + 1] = prefix[k] + f64::from(lin[k]);
        }
        let smooth = |k: usize| -> f32 {
            let (lo, hi) = (k.saturating_sub(half), (k + half + 1).min(n));
            let mean = (prefix[hi] - prefix[lo]) / (hi - lo) as f64;
            10.0 * (mean as f32 + 1e-20).log10()
        };
        let s: Vec<f32> = (0..n).map(smooth).collect();
        let mut band: Vec<f32> = inside.iter().map(|&k| s[k]).collect();
        band.sort_by(f32::total_cmp);
        let floor = band[band.len() / 2];

        let reach = ((MIN_SPACING_HZ / bin_hz) as usize).max(1);
        let mut peaks: Vec<Carrier> = Vec::new();
        for &k in &inside {
            let v = s[k];
            if v - floor < PEAK_ABOVE_FLOOR_DB {
                continue;
            }
            let (lo, hi) = (k.saturating_sub(reach), (k + reach + 1).min(n));
            if s[lo..hi].iter().any(|&o| o > v) {
                continue;
            }
            // A plateau yields its first bin only.
            if k > 0 && s[k - 1] == v {
                continue;
            }
            peaks.push(Carrier { freq_hz: freq(k), above_floor_db: v - floor });
        }
        peaks.sort_by(|a, b| b.above_floor_db.total_cmp(&a.above_floor_db));
        let mut out: Vec<Carrier> = Vec::new();
        for p in peaks {
            if out.iter().all(|o| (o.freq_hz - p.freq_hz).abs() >= MIN_SPACING_HZ) {
                out.push(p);
            }
        }
        out
    }
}

/// One channel being decoded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lane {
    pub freq_hz: f64,
    /// Unix second its carrier was last seen.
    pub seen: i64,
}

/// What [`LaneBook::update`] decided.
#[derive(Debug, Default, PartialEq)]
pub struct Change {
    pub add: Vec<f64>,
    pub remove: Vec<f64>,
}

/// The channels being decoded, at most [`MAX_LANES`].
#[derive(Debug, Default)]
pub struct LaneBook {
    lanes: Vec<Lane>,
}

impl LaneBook {
    pub fn lanes(&self) -> &[Lane] {
        &self.lanes
    }

    /// Fold in the carriers just heard (strongest first). A carrier on a
    /// channel already held keeps it alive; the strongest others fill the free
    /// places; a channel whose carrier has been missing for [`LANE_IDLE_S`] is
    /// dropped.
    pub fn update(&mut self, carriers: &[Carrier], now: i64) -> Change {
        let mut change = Change::default();
        for c in carriers {
            if let Some(l) =
                self.lanes.iter_mut().find(|l| (l.freq_hz - c.freq_hz).abs() < SAME_CHANNEL_HZ)
            {
                l.seen = now;
            } else if self.lanes.len() < MAX_LANES {
                self.lanes.push(Lane { freq_hz: c.freq_hz, seen: now });
                change.add.push(c.freq_hz);
            }
        }
        self.lanes.retain(|l| {
            let keep = now - l.seen <= LANE_IDLE_S;
            if !keep {
                change.remove.push(l.freq_hz);
            }
            keep
        });
        change
    }

    /// Forget every channel (the window moved, or the scan was switched off).
    pub fn clear(&mut self) -> Vec<f64> {
        self.lanes.drain(..).map(|l| l.freq_hz).collect()
    }
}

/// Fold the sondes one channel decoder knows into the merged list: the entry
/// with the later frame wins, and the list stays newest first and bounded.
pub fn merge(into: &mut Vec<SondeTrack>, from: Vec<SondeTrack>) {
    for s in from {
        match into.iter_mut().find(|o| o.serial == s.serial) {
            Some(o) if s.last_unix >= o.last_unix => *o = s,
            Some(_) => {}
            None => into.push(s),
        }
    }
    into.sort_by_key(|s| std::cmp::Reverse(s.last_unix));
    into.truncate(sdroxide_types::SONDE_MAX_TRACKS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use xng_mode_sonde::{modulate, whitening};

    const FRAME1: &str = "8635f44093df1a602c87e0fa0521e8943d9cef4c7a67393f6d39fb546461f2111b6447ab79a746c80350cda5344157f8c0c12234f46902220f792816174b313933303239331a00000300000a00002f0007322ce53e31991abf12dada3eb68468c16755d51c7a2a15310216060245f302000d08a31607821e08bb210219060243f302000000000000000000000000000000220d7c1e0807d03cdc071fd81ddb19d70a8d0eb602b60cb518d40692ff00ff00ff001c277d59b8d83301ff0f881f0f38f4fe18b283038735ff000000003eb8ff4947201e6e3aff55415f13fc6e005440440cf100009e9f7406f85800832b631719d70010bebc172a8b00000000000000000000000000000000000000000000a48b7b15366181193ef05d07e1245b1be0f721f801f60804107b0b76110000000000000000000000000000000000ecc7";

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    /// A little noise from a fixed sequence (no RNG dependency).
    fn noise(len: usize, level: f32) -> Vec<Complex32> {
        let mut x = 0x1234_5678u32;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x as f32 / u32::MAX as f32) - 0.5
        };
        (0..len).map(|_| Complex32::new(next() * level, next() * level)).collect()
    }

    /// Sondes at the given offsets (Hz from the window centre) and amplitudes,
    /// in noise, at `rate`.
    fn window(rate: f64, sondes: &[(f64, f32)]) -> Vec<Complex32> {
        let mut on_air = hex(FRAME1);
        whitening::xor_mask(&mut on_air, 0);
        let mut mix = noise((rate * 0.7) as usize, 0.02);
        for &(off, amp) in sondes {
            let burst = modulate::burst_iq(&on_air, rate, off, amp);
            for (m, b) in mix.iter_mut().zip(burst) {
                *m += Complex32::new(b.re, b.im);
            }
        }
        mix
    }

    fn scan(iq: &[Complex32], rate: f64, center: f64) -> Vec<Carrier> {
        let mut a = Accumulator::new(rate);
        let n = a.block_len();
        for chunk in iq.chunks(n).filter(|c| c.len() == n) {
            a.push(chunk);
        }
        a.carriers(rate, center)
    }

    #[test]
    fn two_sondes_in_one_window_are_two_carriers_in_the_right_places() {
        let (rate, center) = (240_000.0, 403_000_000.0);
        let iq = window(rate, &[(30_000.0, 0.5), (-50_000.0, 0.3)]);
        let c = scan(&iq, rate, center);
        assert_eq!(c.len(), 2, "{c:?}");
        // Strongest first.
        assert!((c[0].freq_hz - 403_030_000.0).abs() < 1_500.0, "{c:?}");
        assert!((c[1].freq_hz - 402_950_000.0).abs() < 1_500.0, "{c:?}");
        assert!(c[0].above_floor_db > c[1].above_floor_db);
    }

    #[test]
    fn noise_alone_has_no_carrier() {
        let rate = 240_000.0;
        let c = scan(&noise(240_000, 0.02), rate, 403_000_000.0);
        assert!(c.is_empty(), "{c:?}");
    }

    #[test]
    fn a_carrier_outside_the_sonde_band_is_ignored() {
        // Centre 399.9 MHz: the sonde at +30 kHz is at 399.93, under the band.
        let rate = 240_000.0;
        let iq = window(rate, &[(30_000.0, 0.5)]);
        assert!(scan(&iq, rate, 399_900_000.0).is_empty());
    }

    #[test]
    fn the_accumulator_starts_over_after_it_is_read() {
        let rate = 240_000.0;
        let iq = window(rate, &[(30_000.0, 0.5)]);
        let mut a = Accumulator::new(rate);
        let n = a.block_len();
        for chunk in iq.chunks(n).filter(|c| c.len() == n) {
            a.push(chunk);
        }
        assert!(a.blocks() > 0);
        assert_eq!(a.carriers(rate, 403e6).len(), 1);
        assert_eq!(a.blocks(), 0);
        assert!(a.carriers(rate, 403e6).is_empty(), "nothing folded in since");
    }

    fn car(f: f64) -> Carrier {
        Carrier { freq_hz: f, above_floor_db: 20.0 }
    }

    #[test]
    fn new_carriers_get_channels_up_to_the_limit() {
        let mut b = LaneBook::default();
        let cs: Vec<Carrier> = (0..6).map(|i| car(401e6 + f64::from(i) * 50_000.0)).collect();
        let ch = b.update(&cs, 100);
        assert_eq!(ch.add.len(), MAX_LANES);
        assert_eq!(b.lanes().len(), MAX_LANES);
        assert!(ch.remove.is_empty());
        // The strongest (first) are the ones kept.
        assert_eq!(ch.add[0], 401e6);
    }

    #[test]
    fn a_carrier_on_a_held_channel_keeps_it_and_adds_nothing() {
        let mut b = LaneBook::default();
        b.update(&[car(403.0e6)], 100);
        let ch = b.update(&[car(403.0e6 + 1_500.0)], 130);
        assert_eq!(ch, Change::default());
        assert_eq!(b.lanes().len(), 1);
        assert_eq!(b.lanes()[0].seen, 130);
    }

    #[test]
    fn a_channel_whose_carrier_is_gone_is_dropped_after_the_idle_time() {
        let mut b = LaneBook::default();
        b.update(&[car(403.0e6), car(404.0e6)], 100);
        let ch = b.update(&[car(404.0e6)], 100 + LANE_IDLE_S);
        assert!(ch.remove.is_empty(), "exactly at the limit it stays");
        let ch = b.update(&[car(404.0e6)], 100 + LANE_IDLE_S + 1);
        assert_eq!(ch.remove, vec![403.0e6]);
        assert_eq!(b.lanes().len(), 1);
        assert_eq!(b.clear(), vec![404.0e6]);
        assert!(b.lanes().is_empty());
    }

    fn track(serial: &str, last: i64, frames: u32) -> SondeTrack {
        SondeTrack {
            model: "RS41".into(),
            serial: serial.into(),
            freq_hz: 403e6,
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
            last_unix: last,
            frames,
            track: vec![],
        }
    }

    #[test]
    fn merging_keeps_the_later_frame_and_the_newest_first() {
        let mut all = vec![track("A", 10, 3), track("B", 20, 1)];
        merge(&mut all, vec![track("A", 30, 4), track("B", 5, 9), track("C", 25, 1)]);
        let order: Vec<&str> = all.iter().map(|s| s.serial.as_str()).collect();
        assert_eq!(order, ["A", "C", "B"]);
        assert_eq!(all[0].frames, 4, "A took the later entry");
        assert_eq!(all[2].frames, 1, "B kept its later entry");
    }
}
