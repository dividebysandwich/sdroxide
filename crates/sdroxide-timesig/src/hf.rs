//! Receive audio to the short-wave stations' seconds and time code.
//!
//! WWV and WWVH are AM broadcasts, heard here in USB with the carrier at
//! [`sdroxide_types::TIMESIG_TONE_HZ`]: everything they send is a tone some
//! distance above it. Two of those tones are what the decoder needs —
//!
//! * the **tick**, 5 ms of 1000 Hz (WWV) or 1200 Hz (WWVH) starting exactly on
//!   every second, inside a 40 ms zone kept clear of everything else; and
//! * the **100 Hz subcarrier**, held high for 0.2, 0.5 or 0.8 s to say the
//!   second's bit, and 15 dB lower for the rest.
//!
//! Each is mixed to zero from where it lands relative to the carrier and
//! averaged: 5 ms for the tick, the length of the burst and so its matched
//! filter; 10 ms for the subcarrier. Those lengths are not arbitrary. A 5 ms
//! average has its nulls every 200 Hz, and the other station's tick is 200 Hz
//! away; a 10 ms average has its nulls every 100 Hz, and every steady tone
//! either station sends — the carrier, the 500 and 600 Hz tones, the ticks —
//! is a whole number of hundreds of hertz from the subcarrier. So each output
//! hears its own part of the broadcast and very little of the rest.
//!
//! # Fading
//!
//! A short-wave signal fades, often by 20 dB in a few seconds, and the fades
//! are flat across a couple of kilohertz. Every output is divided by the
//! carrier, averaged over a fifth of a second centred on the same instant,
//! which takes the fade off and leaves the modulation.
//!
//! Dividing by something that can be nearly nothing has to be done with care.
//! Nothing comes out at all until the search has found the carrier — on
//! noise alone the "carrier" would be noise, and its near-zero moments would
//! turn into the tallest peaks the decoder ever sees — and the divisor is
//! never let below a tenth of the carrier's long-run level, so the bottom of
//! a deep fade is no worse than not dividing.
//!
//! # The subcarrier, coherently
//!
//! The 100 Hz subcarrier starts every second on a rising zero crossing, so its
//! phase against the carrier never changes; only its level does. Divided by
//! the carrier — its phase as well as its size — it is a phasor that points
//! the same way all the time, and that direction is learnt by averaging it.
//! The level is then read as the part along that direction: half the noise of
//! taking the magnitude, and no floor where a weak subcarrier's magnitude
//! would be mostly noise. Its low level, 15 dB down, would otherwise read as
//! the noise rather than itself.
//!
//! # Where each sample is
//!
//! As on long wave, every filter is a plain average, symmetric, and every
//! output carries the input sample it stands for: `pos` for the subcarrier,
//! `tpos` for the ticks, which are averaged over a different length.

use std::collections::VecDeque;

use num_complex::Complex32;
use sdroxide_types::TimeStation;

use crate::frontend::{ENV_RATE, EnvSample};
use crate::search::CarrierSearch;

/// The subcarrier's offset above the carrier, Hz.
const SUBCARRIER_HZ: f64 = 100.0;
/// The subcarrier's average, envelope samples: 10 ms.
const SUB_SMOOTH: usize = 10;
/// The ticks' average: 5 ms, the length of a tick.
const TICK_SMOOTH: usize = 5;
/// The carrier average outputs are divided by, envelope samples either side
/// of the centre: 100 ms, so 0.2 s in all.
const LEVEL_HALF: usize = 100;
/// The subcarrier's direction against the carrier is averaged over this many
/// envelope samples: ten seconds.
const SUB_PHASE_TC: f32 = 10_000.0;
/// The carrier's long-run level, over this many envelope samples: twenty
/// seconds, longer than any fade.
const LONG_LEVEL_TC: f32 = 20_000.0;
/// How far below its long-run level the carrier is divided by at most: 20 dB.
const LEVEL_FLOOR: f32 = 0.1;

/// One mixer at the input rate, and the average that takes it down to the
/// envelope rate: a triangle two blocks wide, two 1 ms averages in a row.
///
/// One average would do for the wanted signal. The triangle is for what
/// mixing a real signal also makes — an image of the carrier on the far side
/// of zero, some 2.5 kHz from every channel — which a single 1 ms average lets
/// through at a tenth, and which aliases down to wherever the receiver's
/// frequency error puts it. Squaring the average's response squares that.
struct Mixer {
    phase: f64,
    step: f64,
    /// This block's sum, and its sum weighted 1, 2, … m along the block.
    sum: Complex32,
    ramp: Complex32,
    last_ramp: Complex32,
}

impl Mixer {
    fn new(hz: f64, rate: f64) -> Self {
        let zero = Complex32::new(0.0, 0.0);
        Mixer {
            phase: 0.0,
            step: std::f64::consts::TAU * hz / rate,
            sum: zero,
            ramp: zero,
            last_ramp: zero,
        }
    }

    /// Moves the mixer to `hz` without a jump in its phase.
    fn tune(&mut self, hz: f64, rate: f64) {
        self.step = std::f64::consts::TAU * hz / rate;
    }

    /// Mixes one input sample, the `i`th (from 0) of its block.
    fn mix(&mut self, x: f32, i: usize) {
        let (s, c) = self.phase.sin_cos();
        let z = Complex32::new(x * c as f32, -x * s as f32);
        self.sum += z;
        self.ramp += z * (i + 1) as f32;
        self.phase += self.step;
        if self.phase > std::f64::consts::PI {
            self.phase -= std::f64::consts::TAU;
        }
    }

    /// The triangle ending with this block: rising over the last block and
    /// falling over this one, so centred on the last sample of the last.
    fn take(&mut self, m: usize) -> Complex32 {
        let m = m as f32;
        let y = (self.last_ramp + self.sum * m - self.ramp) / (m * m);
        self.last_ramp = self.ramp;
        self.sum = Complex32::new(0.0, 0.0);
        self.ramp = Complex32::new(0.0, 0.0);
        y
    }
}

/// A running average of the last `n` complex samples.
struct Boxcar {
    ring: Vec<Complex32>,
    at: usize,
    filled: usize,
}

impl Boxcar {
    fn new(n: usize) -> Self {
        Boxcar { ring: vec![Complex32::new(0.0, 0.0); n], at: 0, filled: 0 }
    }

    fn clear(&mut self) {
        self.filled = 0;
    }

    fn push(&mut self, z: Complex32) -> Option<Complex32> {
        self.ring[self.at] = z;
        self.at = (self.at + 1) % self.ring.len();
        self.filled = (self.filled + 1).min(self.ring.len());
        (self.filled == self.ring.len())
            .then(|| self.ring.iter().sum::<Complex32>() / self.ring.len() as f32)
    }
}

/// What one decimated block produced, waiting for the level average centred
/// on it.
#[derive(Clone, Copy)]
struct Pending {
    pos: f64,
    sub: Complex32,
    tpos: f64,
    tick: f32,
    sibling: f32,
}

pub struct HfFrontEnd {
    rate: f64,
    m: usize,
    tone_hz: f64,
    /// Carrier, subcarrier, this station's tick, the other's tick — each where
    /// it is above the carrier, tuned to follow the carrier as found.
    mixers: [Mixer; 4],
    /// How far above the carrier each of [`HfFrontEnd::mixers`] listens.
    above: [f64; 4],
    /// The offset the mixers are tuned for.
    tuned: f64,
    /// The search's own mixer, which stays on the tone: it is measuring where
    /// the carrier is, and has to measure from somewhere that does not move.
    search_mixer: Mixer,
    acc_n: usize,
    blocks: u64,
    search: CarrierSearch,
    carrier: Boxcar,
    sub: Boxcar,
    tick: Boxcar,
    sibling: Boxcar,
    /// The carrier, block by block, and their running sum — complex, so the
    /// average keeps the carrier's phase for the subcarrier to be read
    /// against.
    levels: VecDeque<Complex32>,
    level_sum: Complex32,
    /// The subcarrier's direction against the carrier.
    sub_phase: Complex32,
    pending: VecDeque<Pending>,
    /// The latest centred carrier amplitude, and its long-run average.
    level: f32,
    long_level: f32,
    /// The search's move count when the averages were last emptied.
    moves_seen: u32,
    /// The input sample the carrier last moved at.
    moved_at: f64,
}

impl HfFrontEnd {
    pub fn new(station: TimeStation, rate: f64, tone_hz: f64) -> Self {
        let m = (rate / ENV_RATE).round().max(1.0) as usize;
        let tick = station.tick_hz().unwrap_or(1000.0);
        let sibling = station.sibling().and_then(TimeStation::tick_hz).unwrap_or(1200.0);
        let above = [0.0, SUBCARRIER_HZ, tick, sibling];
        HfFrontEnd {
            rate,
            m,
            tone_hz,
            mixers: above.map(|a| Mixer::new(tone_hz + a, rate)),
            above,
            tuned: 0.0,
            search_mixer: Mixer::new(tone_hz, rate),
            acc_n: 0,
            blocks: 0,
            search: CarrierSearch::new(rate / m as f64, false),
            carrier: Boxcar::new(SUB_SMOOTH),
            sub: Boxcar::new(SUB_SMOOTH),
            tick: Boxcar::new(TICK_SMOOTH),
            sibling: Boxcar::new(TICK_SMOOTH),
            levels: VecDeque::new(),
            level_sum: Complex32::new(0.0, 0.0),
            sub_phase: Complex32::new(0.0, 0.0),
            pending: VecDeque::new(),
            level: 0.0,
            long_level: 0.0,
            moves_seen: 0,
            moved_at: 0.0,
        }
    }

    /// The input sample the carrier was last found somewhere new at: nothing
    /// before it was measured on the right frequency.
    pub fn moved_at(&self) -> f64 {
        self.moved_at
    }

    pub fn moves(&self) -> u32 {
        self.search.moves()
    }

    pub fn carrier_hz(&self) -> Option<f64> {
        self.search.offset_hz().map(|o| self.tone_hz + o)
    }

    /// The carrier's amplitude, averaged over the last fifth of a second.
    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn process(&mut self, audio: &[f32], out: &mut Vec<EnvSample>) {
        for &x in audio {
            for mx in &mut self.mixers {
                mx.mix(x, self.acc_n);
            }
            self.search_mixer.mix(x, self.acc_n);
            self.acc_n += 1;
            if self.acc_n == self.m {
                self.acc_n = 0;
                let m = self.m;
                let y = self.mixers.each_mut().map(|mx| mx.take(m));
                let s = self.search_mixer.take(m);
                self.block(s, y, out);
            }
        }
    }

    fn block(&mut self, s: Complex32, y: [Complex32; 4], out: &mut Vec<EnvSample>) {
        self.search.push(s);
        if let Some(off) = self.search.offset_hz()
            && off != self.tuned
        {
            // Every part of the broadcast is off by the same amount: the
            // receiver's error moves them all together. Tuned exactly, each
            // part sits on its channel's zero and every other part of the
            // broadcast on a null of the averages that follow.
            self.tuned = off;
            for (mx, a) in self.mixers.iter_mut().zip(self.above) {
                mx.tune(self.tone_hz + off + a, self.rate);
            }
            if self.search.moves() != self.moves_seen {
                // A real move, not a refinement: every average in hand was
                // taken on the wrong frequencies — the subcarrier's, off by
                // tens of hertz, hears the carrier itself — and goes.
                self.moves_seen = self.search.moves();
                self.moved_at = self.blocks as f64 * self.m as f64;
                for b in [&mut self.carrier, &mut self.sub, &mut self.tick, &mut self.sibling] {
                    b.clear();
                }
                self.levels.clear();
                self.level_sum = Complex32::new(0.0, 0.0);
                self.sub_phase = Complex32::new(0.0, 0.0);
                self.pending.clear();
            }
        }
        let k = self.blocks as f64;
        self.blocks += 1;
        let m = self.m as f64;
        // Block k's triangle is centred on input sample k·m − 1; an average
        // of the last n of them is centred (n − 1)/2 blocks further back.
        let at = |n: usize| (k - (n as f64 - 1.0) / 2.0) * m - 1.0;
        let carrier = self.carrier.push(y[0]);
        let sub = self.sub.push(y[1]);
        let tick = self.tick.push(y[2]);
        let sibling = self.sibling.push(y[3]);
        let (Some(carrier), Some(sub), Some(tick), Some(sibling)) = (carrier, sub, tick, sibling)
        else {
            return;
        };
        self.levels.push_back(carrier);
        self.level_sum += carrier;
        self.pending.push_back(Pending {
            pos: at(SUB_SMOOTH),
            sub,
            tpos: at(TICK_SMOOTH),
            tick: tick.norm(),
            sibling: sibling.norm(),
        });
        if self.levels.len() > 2 * LEVEL_HALF + 1 {
            self.level_sum -= self.levels.pop_front().unwrap_or_default();
        }
        if self.levels.len() < 2 * LEVEL_HALF + 1 {
            // Until the average is full, only the blocks that will sit at its
            // centre are worth keeping.
            if self.pending.len() > LEVEL_HALF {
                self.pending.pop_front();
            }
            return;
        }
        // The window ends at the newest block and holds LEVEL_HALF either
        // side of the one leaving the queue now.
        let p = self.pending.pop_front().expect("one pending per level");
        let carrier = self.level_sum / self.levels.len() as f32;
        self.level = carrier.norm();
        self.long_level = if self.long_level == 0.0 {
            self.level
        } else {
            self.long_level + (self.level - self.long_level) / LONG_LEVEL_TC
        };
        if self.search.offset_hz().is_none() {
            return;
        }
        let norm = 1.0 / self.level.max(LEVEL_FLOOR * self.long_level).max(f32::MIN_POSITIVE);
        // The subcarrier against the carrier, phase and all; its direction
        // learnt slowly, and its level read along it.
        let ratio = p.sub * carrier.conj() * (norm * norm);
        self.sub_phase += (ratio - self.sub_phase) / SUB_PHASE_TC;
        let dir = self.sub_phase.norm();
        let sub = if dir > 0.0 { (ratio * self.sub_phase.conj()).re / dir } else { ratio.norm() };
        out.push(EnvSample {
            pos: p.pos,
            mag: sub,
            tpos: p.tpos,
            timing: p.tick * norm,
            sibling: p.sibling * norm,
            iq: Complex32::new(0.0, 0.0),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A carrier `off` hertz from 1 kHz and one tone above it, as USB hands
    /// them over.
    fn audio(rate: f64, secs: f64, off: f64, tone_above: f64, tone_level: f64) -> Vec<f32> {
        (0..(rate * secs) as usize)
            .map(|n| {
                let t = n as f64 / rate;
                let fc = 1000.0 + off;
                let c = (std::f64::consts::TAU * fc * t).cos();
                let s = tone_level * (std::f64::consts::TAU * (fc + tone_above) * t).cos();
                (0.2 * (c + s)) as f32
            })
            .collect()
    }

    fn tail_at(rate: f64, off: f64, tone_above: f64, level: f64) -> EnvSample {
        let mut fe = HfFrontEnd::new(TimeStation::Wwv, rate, 1000.0);
        let mut out = Vec::new();
        fe.process(&audio(rate, 13.0, off, tone_above, level), &mut out);
        if off != 0.0 {
            let found = fe.carrier_hz().expect("carrier") - 1000.0;
            assert!((found - off).abs() < 0.2, "{rate}: found {found} for {off}");
        }
        *out.last().expect("output")
    }

    fn tail(rate: f64, tone_above: f64, level: f64) -> EnvSample {
        tail_at(rate, 0.0, tone_above, level)
    }

    #[test]
    fn a_receiver_off_frequency_moves_every_part_together() {
        for (rate, off) in [(12_000.0, 187.0), (48_000.0, -251.0)] {
            let e = tail_at(rate, off, 100.0, 0.09);
            assert!((e.mag - 0.09).abs() < 0.005, "{rate} {off}: subcarrier {}", e.mag);
            let e = tail_at(rate, off, 1000.0, 0.5);
            assert!((e.timing - 0.5).abs() < 0.03, "{rate} {off}: tick {}", e.timing);
        }
    }

    #[test]
    fn each_part_of_the_broadcast_hears_its_own() {
        for rate in [8_000.0, 12_000.0, 48_000.0] {
            // The subcarrier, as a ratio to the carrier.
            let e = tail(rate, 100.0, 0.09);
            assert!((e.mag - 0.09).abs() < 0.005, "{rate}: subcarrier {}", e.mag);
            assert!(e.timing < 0.01, "{rate}: the tick hears the subcarrier: {}", e.timing);
            // WWV's tick pitch, steady.
            let e = tail(rate, 1000.0, 0.5);
            assert!((e.timing - 0.5).abs() < 0.03, "{rate}: tick {}", e.timing);
            assert!(e.sibling < 0.02, "{rate}: WWVH's detector hears WWV: {}", e.sibling);
            assert!(e.mag < 0.01, "{rate}: the subcarrier hears the tick: {}", e.mag);
            // WWVH's, which this front end, set for WWV, must keep apart.
            let e = tail(rate, 1200.0, 0.5);
            assert!(e.timing < 0.02, "{rate}: WWV's detector hears WWVH: {}", e.timing);
            assert!((e.sibling - 0.5).abs() < 0.03, "{rate}: sibling {}", e.sibling);
            // The 500 Hz tone either station sends in alternate minutes: none
            // of it in the subcarrier, and in the ticks only what a 5 ms
            // average lets through 500 Hz away — which the stations keep out
            // of the 40 ms around each tick, where the tick is looked for.
            let e = tail(rate, 500.0, 0.5);
            assert!(e.mag < 0.01, "{rate}: 500 Hz in the subcarrier: {}", e.mag);
            assert!(e.timing < 0.08, "{rate}: 500 Hz in the ticks: {}", e.timing);
        }
    }

    #[test]
    fn a_fade_is_divided_out() {
        let rate = 12_000.0;
        let x: Vec<f32> = audio(rate, 9.0, 0.0, 100.0, 0.09)
            .iter()
            .enumerate()
            .map(|(n, &v)| {
                // Down 20 dB and back over two seconds.
                let t = n as f64 / rate;
                let g = 0.55 + 0.45 * (std::f64::consts::TAU * 0.5 * t).cos();
                v * g as f32
            })
            .collect();
        let mut fe = HfFrontEnd::new(TimeStation::Wwv, rate, 1000.0);
        let mut out = Vec::new();
        fe.process(&x, &mut out);
        for e in &out[out.len() - 3000..] {
            assert!((e.mag - 0.09).abs() < 0.012, "subcarrier ratio {} through the fade", e.mag);
        }
    }
}
