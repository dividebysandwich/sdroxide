//! One AIS channel, end to end: gate, demodulate, deframe, decode.
//!
//! ```text
//!  baseband ─▶ RxFilter ─▶ Gate ─▶ Demod::prepare ─▶ slice at a phase ─▶ Deframer ─▶ parse
//!               ±9 kHz                    │                   ▲              │
//!                                         └── timing, level ──┘         data field
//! ```
//!
//! The receive filter is ahead of the gate rather than behind it, which is
//! [`crate::demod`]'s note to make: it is what stops one channel's gate opening
//! on the other channel's ships.
//!
//! # The counters are the diagnosis
//!
//! An empty vessel list has half a dozen causes and they want completely
//! different answers: no aerial, an aerial for the wrong band, a receiver
//! parked elsewhere, a channel full of something that is not AIS, or a decoder
//! with a bit order backwards. [`Counters`] is arranged so that the *first*
//! stage that stops counting names the cause:
//!
//! - `bursts` zero — nothing is arriving. Aerial, or the receiver is not here.
//! - `bursts` up, `no_eye` equal to it — something is arriving and it is not
//!   GMSK. A carrier, a pager, a repeater's tail.
//! - `bursts` up, `bad_fcs` high, `frames` zero — it is a burst of *something*
//!   whose bits do not check. Marginal signal, or a decoder problem.
//! - `frames` up, `unsupported` high — real AIS, being read, carrying message
//!   types this decoder does not model. Nothing is wrong.
//!
//! # Why the bit phase is swept and why that is not a substitute for measuring it
//!
//! [`crate::demod`] measures the bit phase once per burst and is right about it
//! — [`crate::demod`]'s own test is what says so. The sweep here exists for the
//! bursts where the *measurement* is biased rather than wrong: one the gate
//! opened late on, one clipped by a fade at its tail, one with a second station
//! bleeding into its last slot. A fraction of a bit of bias is the difference
//! between every frame and none, and each extra try costs one pass of a slicer
//! over a couple of hundred bits.
//!
//! The order matters: the estimate is tried *first*, and the sweep stops at the
//! first phase that yields a frame whose check sequence passes. A phase that
//! produces a frame produced the right one — a sixteen-bit check standing over
//! a flag-delimited, octet-aligned run is not something a wrong sampling
//! instant arrives at.

use sdroxide_dsp::Complex32;

use crate::demod::{Demod, RxFilter};
use crate::gate::{Burst, Gate};
use crate::hdlc::{Deframer, Reject};
use crate::message::{self, Message};

/// What came off the air.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub message: Message,
    /// The data field as received, for the `!AIVDM` sentence the panel shows.
    pub bits: Vec<bool>,
    /// Absolute RF centre of the channel it arrived on.
    pub center_hz: f64,
    /// `'A'` or `'B'`.
    pub channel: char,
    pub snr_db: f32,
    pub rssi_dbfs: f32,
    /// How far off frequency the transmission was, in Hz — see
    /// [`crate::demod::Timing::offset_hz`].
    pub freq_offset_hz: f32,
}

/// Where every transmission ended up.
#[derive(Debug, Clone, Copy, Default)]
pub struct Counters {
    pub bursts: u64,
    /// Transmissions dropped for running past the gate's cap — a stuck
    /// carrier, or a run of slots with no gap the gate could find.
    pub overlong: u64,
    /// Transmissions with no eye in them: the gate opened on something that is
    /// not this waveform.
    pub no_eye: u64,
    /// Candidates between two flags that were not a whole number of octets —
    /// noise that happened to contain the flag pattern.
    pub unaligned: u64,
    /// ...and ones that were, and whose check sequence failed.
    pub bad_fcs: u64,
    /// Frames whose check sequence passed.
    pub frames: u64,
    /// ...of which this many carried a message type the decoder reads.
    pub messages: u64,
    /// ...and this many did not.
    pub unsupported: u64,
}

/// One channel's receiver.
pub struct ChannelRx {
    center_hz: f64,
    channel: char,
    filter: RxFilter,
    gate: Gate,
    demod: Demod,
    counters: Counters,

    /// Smoothed frequency offset of the transmissions that decoded here.
    ///
    /// Only from frames that *passed*, which is what makes it worth anything:
    /// a burst that did not decode has no measured offset, it has a
    /// measurement of whatever the gate opened on.
    offset_hz: Option<f32>,

    filtered: Vec<Complex32>,
    bursts: Vec<Burst>,
    levels: Vec<bool>,
    frames: Vec<Vec<bool>>,
}

impl ChannelRx {
    pub fn new(center_hz: f64, channel: char, rate_hz: f64, threshold_db: f32) -> ChannelRx {
        ChannelRx {
            center_hz,
            channel,
            filter: RxFilter::new(rate_hz),
            gate: Gate::new(rate_hz, center_hz, threshold_db),
            demod: Demod::new(rate_hz),
            counters: Counters::default(),
            offset_hz: None,
            filtered: Vec::new(),
            bursts: Vec::new(),
            levels: Vec::new(),
            frames: Vec::new(),
        }
    }

    pub fn counters(&self) -> Counters {
        self.counters
    }

    pub fn floor_dbfs(&self) -> f32 {
        self.gate.floor_dbfs()
    }

    /// How far off frequency the ships heard on this channel have been, in Hz.
    /// `None` until one has decoded.
    pub fn offset_hz(&self) -> Option<f32> {
        self.offset_hz
    }

    pub fn samples_per_bit(&self) -> f64 {
        self.demod.samples_per_bit()
    }

    pub fn set_threshold_db(&mut self, db: f32) {
        self.gate.set_threshold_db(db);
    }

    /// Feed baseband samples for this channel; append whatever decoded.
    pub fn push(&mut self, iq: &[Complex32], out: &mut Vec<Decoded>) {
        let mut filtered = std::mem::take(&mut self.filtered);
        filtered.clear();
        self.filter.process(iq, &mut filtered);
        let mut bursts = std::mem::take(&mut self.bursts);
        bursts.clear();
        self.gate.push(&filtered, &mut bursts);
        self.filtered = filtered;
        self.counters.bursts = self.gate.opened;
        self.counters.overlong = self.gate.overlong;
        for b in &bursts {
            self.decode(b, out);
        }
        self.bursts = bursts;
    }

    fn decode(&mut self, burst: &Burst, out: &mut Vec<Decoded>) {
        let Some(timing) = self.demod.prepare(&burst.iq) else {
            self.counters.no_eye += 1;
            return;
        };

        let mut frames = std::mem::take(&mut self.frames);
        let mut levels = std::mem::take(&mut self.levels);
        let mut rejects: Vec<Reject> = Vec::new();
        for (try_no, &offset) in Demod::phase_tries().iter().enumerate() {
            frames.clear();
            self.demod.slice(&timing, offset, &mut levels);
            let mut df = Deframer::new();
            for &lvl in levels.iter() {
                df.push_level(lvl, &mut frames);
            }
            // Whatever the accepted pass saw is what gets counted. Counting
            // every pass would multiply the failure counters by the length of
            // the sweep and make a healthy channel look broken.
            if try_no == 0 || !frames.is_empty() {
                rejects = std::mem::take(&mut df.rejects);
            }
            if !frames.is_empty() {
                break;
            }
        }
        for r in &rejects {
            match r {
                Reject::BadFcs => self.counters.bad_fcs += 1,
                Reject::Unaligned | Reject::Length => self.counters.unaligned += 1,
            }
        }
        for bits in frames.drain(..) {
            self.counters.frames += 1;
            // One part new to seven old: it is a standing property of the
            // receiver, not of this transmission, and one ship with a bad
            // oscillator must not move it.
            self.offset_hz = Some(match self.offset_hz {
                Some(prev) => prev + (timing.offset_hz() - prev) * 0.125,
                None => timing.offset_hz(),
            });
            let Some(message) = message::parse(&bits) else { continue };
            if message.known {
                self.counters.messages += 1;
            } else {
                self.counters.unsupported += 1;
            }
            out.push(Decoded {
                message,
                bits,
                center_hz: self.center_hz,
                channel: self.channel,
                snr_db: burst.snr_db,
                rssi_dbfs: burst.peak_dbfs,
                freq_offset_hz: timing.offset_hz(),
            });
        }
        self.frames = frames;
        self.levels = levels;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::{Noise, Payload, TxParams, modulate_bits};
    use sdroxide_types::AIS_CHANNEL_A_HZ;

    fn position_report(mmsi: u32, lat: f64, lon: f64) -> Vec<bool> {
        Payload::new(168)
            .put(0, 6, 1)
            .put(8, 30, u64::from(mmsi))
            .put(38, 4, 0)
            .put(50, 10, 121)
            .put_signed(61, 28, (lon * 600_000.0) as i64)
            .put_signed(89, 27, (lat * 600_000.0) as i64)
            .put(116, 12, 900)
            .put(128, 9, 90)
            .bits()
    }

    /// The whole channel, from samples to a decoded ship: a transmission built
    /// from the standard goes in and the vessel it describes comes out.
    #[test]
    fn a_transmission_becomes_a_position_report() {
        let rate = 75_000.0;
        let p = TxParams { sample_rate: rate, ..TxParams::default() };
        let bits = position_report(244_660_000, 52.3791, 4.8973);
        let mut rx = ChannelRx::new(AIS_CHANNEL_A_HZ, 'A', rate, 8.0);

        let mut n = Noise::new(1);
        let mut quiet = vec![Complex32::default(); 20_000];
        n.add(&mut quiet, 0.005);
        let mut out = Vec::new();
        rx.push(&quiet, &mut out);

        let mut sig = modulate_bits(&bits, &p);
        n.add(&mut sig, 0.005);
        rx.push(&sig, &mut out);
        let mut tail = vec![Complex32::default(); 8_000];
        n.add(&mut tail, 0.005);
        rx.push(&tail, &mut out);

        assert_eq!(out.len(), 1, "counters: {:?}", rx.counters());
        let d = &out[0];
        assert_eq!(d.message.mmsi, 244_660_000);
        assert_eq!(d.message.kind, 1);
        assert_eq!(d.channel, 'A');
        let f = d.message.fix.as_ref().expect("a fix");
        assert!((f.lat.unwrap() - 52.3791).abs() < 1e-4);
        assert!((f.lon.unwrap() - 4.8973).abs() < 1e-4);
        assert_eq!(rx.counters().messages, 1);
        assert_eq!(rx.counters().bad_fcs, 0);
    }

    /// It still works on a receiver nobody has calibrated. Four kilohertz at
    /// 162 MHz is twenty-five parts per million, which is an ordinary dongle,
    /// and it is nearly twice the whole frequency deviation.
    #[test]
    fn an_uncalibrated_receiver_still_decodes() {
        let rate = 75_000.0;
        let p = TxParams { sample_rate: rate, ..TxParams::default() };
        let bits = position_report(366_123_456, 40.7128, -74.0060);
        let mut sig = modulate_bits(&bits, &p);
        crate::tx::shift(&mut sig, 4_000.0, rate);

        let mut rx = ChannelRx::new(AIS_CHANNEL_A_HZ, 'A', rate, 8.0);
        let mut n = Noise::new(2);
        let mut quiet = vec![Complex32::default(); 20_000];
        n.add(&mut quiet, 0.005);
        let mut out = Vec::new();
        rx.push(&quiet, &mut out);
        n.add(&mut sig, 0.005);
        rx.push(&sig, &mut out);
        rx.push(&quiet, &mut out);

        assert_eq!(out.len(), 1, "counters: {:?}", rx.counters());
        let f = out[0].message.fix.as_ref().unwrap();
        assert!(f.lon.unwrap() < -73.0, "{:?}", f.lon);
        // ...and the receiver knows how far off it is, which is what the panel
        // needs to tell the operator to set a frequency correction.
        let off = rx.offset_hz().expect("a decoded frame measures the offset");
        assert!((off - 4_000.0).abs() < 500.0, "measured {off} Hz");
    }

    /// Two ships in adjacent slots both decode. This is the gate's separation
    /// and the per-burst measurement working together — one welded burst would
    /// give a timing and a slicing level that fitted neither.
    #[test]
    fn two_ships_in_adjacent_slots_both_decode() {
        let rate = 75_000.0;
        let sps = rate / sdroxide_types::AIS_BIT_RATE;
        let p = TxParams { sample_rate: rate, ..TxParams::default() };
        let a = modulate_bits(&position_report(111_111_111, 10.0, 20.0), &p);
        // The second ship is 12 dB weaker, which is ordinary: they are at
        // different ranges.
        let mut b = modulate_bits(&position_report(222_222_222, -10.0, -20.0), &p);
        for z in b.iter_mut() {
            *z *= 0.25;
        }

        let mut rx = ChannelRx::new(AIS_CHANNEL_A_HZ, 'A', rate, 8.0);
        let mut n = Noise::new(3);
        let mut quiet = vec![Complex32::default(); 20_000];
        n.add(&mut quiet, 0.004);
        let mut out = Vec::new();
        rx.push(&quiet, &mut out);

        let gap = ((256.0 * sps) as usize).saturating_sub(a.len());
        let mut run = a;
        run.extend(std::iter::repeat_n(Complex32::default(), gap));
        run.extend_from_slice(&b);
        n.add(&mut run, 0.004);
        rx.push(&run, &mut out);
        rx.push(&quiet, &mut out);

        let mmsis: Vec<u32> = out.iter().map(|d| d.message.mmsi).collect();
        assert!(mmsis.contains(&111_111_111), "{mmsis:?} — {:?}", rx.counters());
        assert!(mmsis.contains(&222_222_222), "{mmsis:?} — {:?}", rx.counters());
    }

    /// Noise is not a transmission either, and what refuses it is the frame
    /// check sequence rather than any measurement of the waveform — see
    /// [`crate::demod`]. Half a minute of it must produce nothing.
    #[test]
    fn noise_alone_produces_no_messages() {
        let rate = 75_000.0;
        let mut rx = ChannelRx::new(AIS_CHANNEL_A_HZ, 'A', rate, 8.0);
        let mut n = Noise::new(0xC0FFEE);
        let mut out = Vec::new();
        let mut buf = vec![Complex32::default(); 1 << 16];
        for k in 0..35 {
            // A level that wanders, so the gate is opening and shutting on it
            // rather than settling into a floor above everything.
            let sigma = if k % 3 == 0 { 0.05 } else { 0.005 };
            for s in buf.iter_mut() {
                *s = n.gaussian(sigma);
            }
            rx.push(&buf, &mut out);
        }
        assert!(out.is_empty(), "noise decoded as {} messages", out.len());
    }

    /// A carrier is not a transmission. The gate opens on it and the eye test
    /// throws it out, rather than a slicer manufacturing bits from a constant.
    #[test]
    fn a_bare_carrier_produces_nothing() {
        let rate = 75_000.0;
        let mut rx = ChannelRx::new(AIS_CHANNEL_A_HZ, 'A', rate, 8.0);
        let mut n = Noise::new(4);
        let mut quiet = vec![Complex32::default(); 20_000];
        n.add(&mut quiet, 0.005);
        let mut out = Vec::new();
        rx.push(&quiet, &mut out);

        let mut carrier = vec![Complex32::new(0.5, 0.0); 4_000];
        n.add(&mut carrier, 0.005);
        rx.push(&carrier, &mut out);
        rx.push(&quiet, &mut out);
        assert!(out.is_empty(), "a carrier decoded as {} messages", out.len());
        assert!(rx.counters().bursts > 0, "the gate should have opened on it");
    }
}

