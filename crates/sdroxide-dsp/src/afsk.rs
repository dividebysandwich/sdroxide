//! AFSK for AX.25 packet: two tones, a bit stream, and nothing above it.
//!
//! Serves both audio-frequency packet speeds, which differ only in constants:
//!
//! | | mark | space | baud | where |
//! |---|---|---|---|---|
//! | HF | 1600 Hz | 1800 Hz | 300 | single sideband |
//! | VHF | 1200 Hz | 2200 Hz | 1200 | audio into an FM transmitter (Bell 202) |
//!
//! Those are Direwolf's defaults for `-B 300` and `-B 1200`, which is what
//! makes them the right ones: the deployed gateway population is what a modem
//! has to agree with.
//!
//! # Relationship to the RTTY modem
//!
//! The detector is [`crate::rtty::RttyRx`]'s, and deliberately so — it is the
//! same problem (non-coherent binary FSK on HF) already solved here against a
//! real signal. Mix to the centre tone, band-limit, rotate each tone to DC and
//! integrate over exactly one bit, then slice on the difference of the two
//! envelopes, each referred to its own recent level so a selective fade decides
//! nothing. That last part, automatic threshold correction, matters more at 300
//! baud on HF than anywhere else in this tree.
//!
//! What is **not** shared is everything above the bit:
//!
//! * RTTY re-acquires its bit clock on every start bit, because a UART gives it
//!   one 7.5 bits. HDLC is a continuous stream with no framing bits at all, so
//!   the clock here free-runs and is nudged by transitions. It can afford to:
//!   NRZI puts a transition on every zero bit, and HDLC's bit stuffing
//!   guarantees a zero at least every five bits. Clock recovery is *why*
//!   stuffing has that limit, not merely a consequence of it.
//! * There is no framing validation and no lock gate. RTTY needs one because a
//!   UART will manufacture a character out of noise; HDLC has a flag and a
//!   16-bit FCS to do that job properly, one layer up.
//!
//! So this emits raw line levels and judges nothing. `sdroxide-ax25`'s deframer
//! decides what was a frame.

use std::f64::consts::TAU;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};
use crate::rtty::Integrator;

/// The two speeds, with the tone pairs the rest of the world uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfskProfile {
    /// 300 baud, 1600/1800 Hz — HF packet on single sideband.
    Hf300,
    /// 1200 baud, 1200/2200 Hz — Bell 202, VHF packet through an FM rig.
    Vhf1200,
    /// 1200 baud, 1300/2100 Hz — the DSC pair (ITU-R M.493-15 §3.1). No NRZI:
    /// DSC is direct FSK, so the slicer's bit is the wire bit.
    Dsc,
    /// 100 baud, 1615/1785 Hz — MF/HF DSC (ITU-R M.493), a 170 Hz shift about
    /// a 1700 Hz subcarrier. Sense is settled by the DSC framer.
    DscHf,
}

impl AfskProfile {
    #[must_use]
    pub fn baud(self) -> f64 {
        match self {
            AfskProfile::Hf300 => 300.0,
            AfskProfile::DscHf => 100.0,
            AfskProfile::Vhf1200 | AfskProfile::Dsc => 1200.0,
        }
    }

    /// (mark, space) in Hz.
    #[must_use]
    pub fn tones(self) -> (f64, f64) {
        match self {
            AfskProfile::Hf300 => (1600.0, 1800.0),
            AfskProfile::Vhf1200 => (1200.0, 2200.0),
            AfskProfile::Dsc => (1300.0, 2100.0),
            AfskProfile::DscHf => (1615.0, 1785.0),
        }
    }

    #[must_use]
    pub fn centre_hz(self) -> f64 {
        let (m, s) = self.tones();
        (m + s) / 2.0
    }

    #[must_use]
    pub fn shift_hz(self) -> f64 {
        let (m, s) = self.tones();
        (s - m).abs()
    }
}

// ─────────────────────────────── transmit ───────────────────────────────

/// Peak amplitude [`AfskTx::next_block`] reaches. A steady tone, so the figure
/// is exact — the transmit chain divides it back out to put the over on the air
/// at full scale (`DigiEngine::tx_peak`).
pub const AFSK_TX_PEAK: f32 = 0.5;

/// Phase-continuous AFSK. Bits in, audio out.
///
/// Phase continuity is not decoration: a discontinuity is a click, it splatters
/// across the channel, and on FM it lands as a wideband transient that the
/// receiver's own discriminator turns into a burst of wrong bits.
pub struct AfskTx {
    rate: f64,
    mark_hz: f64,
    space_hz: f64,
    spb: f64,
    phase: f64,
    /// Bits queued, in transmission order.
    bits: std::collections::VecDeque<bool>,
    /// Samples still owed for the bit being rendered.
    left: f64,
    cur_mark: bool,
}

impl AfskTx {
    #[must_use]
    pub fn new(rate: f64, profile: AfskProfile) -> Self {
        let (mark_hz, space_hz) = profile.tones();
        AfskTx {
            rate,
            mark_hz,
            space_hz,
            spb: rate / profile.baud(),
            phase: 0.0,
            bits: std::collections::VecDeque::new(),
            left: 0.0,
            cur_mark: true,
        }
    }

    /// Queue line levels — `true` is mark. These come from the HDLC framer and
    /// are already NRZI-encoded and stuffed.
    pub fn push_bits(&mut self, bits: &[bool]) {
        self.bits.extend(bits.iter().copied());
    }

    /// Everything queued has been rendered.
    #[must_use]
    pub fn idle(&self) -> bool {
        self.bits.is_empty() && self.left <= 0.0
    }

    /// Bits still waiting, for a transmit-progress readout.
    #[must_use]
    pub fn pending_bits(&self) -> usize {
        self.bits.len()
    }

    /// Fill `out` with audio. Returns the number of samples that carry signal;
    /// the rest is silence, which is the caller's cue that the over is done.
    ///
    /// Amplitude is [`AFSK_TX_PEAK`] rather than full scale: this is the
    /// modulating signal, and the transmit chain's drive control is what sets
    /// the level on the air — it divides this headroom back out again, so half
    /// scale here is not half a transmitter on the air.
    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        for slot in out.iter_mut() {
            if self.left <= 0.0 {
                match self.bits.pop_front() {
                    Some(b) => {
                        self.cur_mark = b;
                        // Accumulated, never assigned: `left` is in (-1, 0]
                        // here, and that remainder is the part of the previous
                        // bit that did not fit in a whole sample. Assigning
                        // would throw it away and round every bit *up* to a
                        // whole sample — a baud rate of `rate / spb.ceil()`
                        // rather than the one the profile names.
                        //
                        // On a sound card that is invisible: 48 kHz is exactly
                        // 40 samples a bit and there is no remainder. On an SDR
                        // it is not, because the channel rate is whatever the
                        // decimation chain lands on — 44642.857 Hz on a HackRF
                        // at 10 Msps, which is 37.2 samples a bit and went out
                        // 2.1 % slow, past what a receiver's clock recovery
                        // will follow. See issue #180.
                        self.left += self.spb;
                    }
                    None => {
                        *slot = 0.0;
                        continue;
                    }
                }
            }
            let hz = if self.cur_mark { self.mark_hz } else { self.space_hz };
            self.phase = (self.phase + TAU * hz / self.rate) % TAU;
            *slot = AFSK_TX_PEAK * self.phase.sin() as f32;
            self.left -= 1.0;
            n += 1;
        }
        n
    }
}

// ─────────────────────────────── receive ────────────────────────────────

/// How hard an observed transition pulls the bit clock.
///
/// Gentler than a UART's, which can afford a hard reset on every start bit.
/// Here the clock has to hold through a whole frame, so it must average many
/// transitions rather than trust any one — but it cannot be so slow that it
/// fails to acquire within the opening flags. Two flags is 16 transitions,
/// which at this gain is comfortably enough.
const CLOCK_TRACK_GAIN: f32 = 0.10;

/// Where a transition is steered to, in fractions of a bit, given that the
/// strobe fires at the wrap. Half a bit puts the strobe in the middle of the
/// eye, which is where it belongs.
///
/// Swept against Direwolf's reference audio at 0.25 through 0.95: every value
/// from 0.5 to 0.85 decodes both bauds perfectly, so the eye is wide and this
/// sits in the middle of the working range rather than on the edge of it.
const STROBE_PHASE: f32 = 0.5;

/// Automatic threshold correction, as in the RTTY detector: each branch is
/// compared against its own recent level, so a frequency-selective fade that
/// leaves one tone well down still decodes.
const ATC_SMOOTH: f32 = 8.0e-3;
const ATC_RELAX: f32 = ATC_SMOOTH / 4.0;
/// Bits a branch may go without winning before its reference starts relaxing
/// towards the other. HDLC's stuffing bounds a run of ones at five, and a run
/// of zeros is unbounded in principle but rare; ten is clear of normal traffic.
const ATC_IDLE_BITS: f32 = 10.0;
/// Bound the correction to about 16 dB.
const ATC_MIN_RATIO: f32 = 0.15;

/// Non-coherent AFSK receiver: audio in, line levels out.
///
/// Emits one level per bit, `true` for mark. Nothing here knows about frames —
/// feed the levels to `sdroxide_ax25::Deframer`.
pub struct AfskRx {
    rate: f64,
    baud: f64,
    centre_hz: f64,
    shift_hz: f64,
    reverse: bool,

    // Front end: mix to the centre tone, then band-limit.
    ph: f32,
    ph_inc: f32,
    lpf: ComplexFir,

    // Tone branches: rotate ±shift/2 to DC and integrate over one bit.
    tone_ph: f32,
    tone_inc: f32,
    mark_int: Integrator,
    space_int: Integrator,

    // Bit clock: a free-running NCO nudged by observed transitions.
    spb: f32,
    clk: f32,
    last_bit: bool,
    have_last: bool,

    // Automatic threshold correction.
    ref_mark: f32,
    ref_space: f32,
    mark_idle: f32,
    space_idle: f32,

    mag: f32,
    /// Smoothed tone separation — how convincingly one branch is winning.
    /// Drives the data-carrier detect the CSMA layer needs.
    sep: f32,

    mixed: Vec<Complex32>,
    bb: Vec<Complex32>,
}

/// Smoothing for the separation figure, about 20 ms at 48 kHz — fast enough
/// that DCD follows a real transmission's start and end, slow enough not to
/// chatter between bits.
const SEP_SMOOTH: f32 = 1.0e-3;

impl AfskRx {
    #[must_use]
    pub fn new(rate: f64, profile: AfskProfile) -> Self {
        let mut rx = AfskRx {
            rate,
            baud: profile.baud(),
            centre_hz: profile.centre_hz(),
            shift_hz: profile.shift_hz(),
            reverse: false,
            ph: 0.0,
            ph_inc: 0.0,
            lpf: ComplexFir::new(Vec::new()),
            tone_ph: 0.0,
            tone_inc: 0.0,
            mark_int: Integrator::new(1),
            space_int: Integrator::new(1),
            spb: 1.0,
            clk: 0.0,
            last_bit: true,
            have_last: false,
            ref_mark: 0.0,
            ref_space: 0.0,
            mark_idle: 0.0,
            space_idle: 0.0,
            mag: 0.0,
            sep: 0.0,
            mixed: Vec::new(),
            bb: Vec::new(),
        };
        rx.configure();
        rx
    }

    fn configure(&mut self) {
        self.spb = (self.rate / self.baud) as f32;
        let len = self.spb.round().max(1.0) as usize;
        self.mark_int.resize(len);
        self.space_int.resize(len);
        self.tone_inc = (TAU * (self.shift_hz / 2.0) / self.rate) as f32;
        self.tone_ph = 0.0;
        self.ph_inc = (TAU * self.centre_hz / self.rate) as f32;
        // Pass both tones and the modulation either side of them. The matched
        // filters do the real rejection; this only keeps the neighbours out.
        let bw = (self.shift_hz / 2.0 + 2.0 * self.baud).max(400.0);
        self.lpf.set_taps(bandpass_taps(129, -bw, bw, self.rate));
        self.lpf.reset();
        self.have_last = false;
        self.ref_mark = 0.0;
        self.ref_space = 0.0;
        self.mark_idle = 0.0;
        self.space_idle = 0.0;
        self.sep = 0.0;
    }

    /// Swap the sense of the tones.
    ///
    /// A signal received on the opposite sideband from the one it was sent on
    /// arrives with mark and space exchanged. NRZI means the *data* survives
    /// that — which is most of why AX.25 uses it — so this exists for the
    /// display and for the rare peer that inverts, not because the decode
    /// depends on it.
    pub fn set_reverse(&mut self, reverse: bool) {
        self.reverse = reverse;
    }

    #[must_use]
    pub fn reverse(&self) -> bool {
        self.reverse
    }

    /// Smoothed signal magnitude.
    #[must_use]
    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    /// How convincingly one tone is winning, 0..1.
    ///
    /// This is the data-carrier detect: on an idle channel the two branches see
    /// the same noise and the figure sits near zero, and a real signal drives it
    /// close to one within a few bits. A CSMA layer must not key while it is
    /// high.
    #[must_use]
    pub fn separation(&self) -> f32 {
        self.sep
    }

    /// Demodulate `audio`, appending one line level per recovered bit.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<bool>) {
        self.mixed.clear();
        self.mixed.reserve(audio.len());
        for &a in audio {
            let z = Complex32::new(a * self.ph.cos(), -a * self.ph.sin());
            self.ph += self.ph_inc;
            if self.ph > std::f32::consts::TAU {
                self.ph -= std::f32::consts::TAU;
            }
            self.mixed.push(z);
        }
        self.bb.clear();
        let mut bb = std::mem::take(&mut self.bb);
        self.lpf.process(&self.mixed, &mut bb);

        for &z in &bb {
            let (s, c) = (self.tone_ph.sin(), self.tone_ph.cos());
            self.tone_ph += self.tone_inc;
            if self.tone_ph > std::f32::consts::TAU {
                self.tone_ph -= std::f32::consts::TAU;
            }
            // In **both** packet profiles mark is the *lower* tone — 1200
            // against 2200, and 1600 against 1800. That is the opposite of
            // amateur RTTY, where mark sits above the centre, so the two
            // rotations are the other way round from `rtty.rs`'s. Getting this
            // backwards swaps mark and space, which NRZI then hides completely:
            // the frames still decode, and only a raw bit comparison ever
            // notices. Hence `mark_is_the_lower_tone` below.
            let m = self.mark_int.push(z * Complex32::new(c, s));
            let sp = self.space_int.push(z * Complex32::new(c, -s));
            let (em, es) = (m.norm(), sp.norm());

            let total = em + es;
            self.mag += 0.02 * (total - self.mag);
            let sep = if total > 1e-20 { (em - es).abs() / total } else { 0.0 };
            self.sep += SEP_SMOOTH * (sep - self.sep);

            // Seed both references together on the first sample carrying
            // energy; starting from zero costs a few bits of transient, which
            // on a packet frame lands in the preamble where it is free.
            if self.ref_mark <= 0.0 && self.ref_space <= 0.0 && total > 1e-20 {
                self.ref_mark = total * 0.5;
                self.ref_space = total * 0.5;
            }
            let (dm, ds) = if self.ref_mark > 1e-20 && self.ref_space > 1e-20 {
                (em / self.ref_mark, es / self.ref_space)
            } else {
                (em, es)
            };

            let phys_mark = dm > ds;
            let bit = phys_mark != self.reverse;

            let idle_limit = ATC_IDLE_BITS * self.spb;
            if phys_mark {
                self.ref_mark += ATC_SMOOTH * (em - self.ref_mark);
                self.space_idle += 1.0;
                self.mark_idle = 0.0;
                if self.space_idle > idle_limit {
                    self.ref_space += ATC_RELAX * (self.ref_mark - self.ref_space);
                }
            } else {
                self.ref_space += ATC_SMOOTH * (es - self.ref_space);
                self.mark_idle += 1.0;
                self.space_idle = 0.0;
                if self.mark_idle > idle_limit {
                    self.ref_mark += ATC_RELAX * (self.ref_space - self.ref_mark);
                }
            }
            let floor = self.ref_mark.max(self.ref_space) * ATC_MIN_RATIO;
            self.ref_mark = self.ref_mark.max(floor);
            self.ref_space = self.ref_space.max(floor);

            self.advance_clock(bit, out);
        }
        self.bb = bb;
    }

    /// Release the band-pass filter's withheld tail, so a burst's last bits are
    /// not lost.
    ///
    /// [`ComplexFir`] buffers `taps.len() - 1` samples and only emits them once
    /// more input arrives. On a continuous stream that is invisible — the audio
    /// keeps coming and the tail falls out on its own — but a file or batch
    /// decode ends with the audio, and the withheld tail is exactly where a DSC
    /// sequence's end-of-sequence character sits. Pushing one group delay of
    /// silence runs the buffer dry and emits those last bits.
    pub fn flush(&mut self, out: &mut Vec<bool>) {
        let n = self.lpf.group_delay();
        if n == 0 {
            return;
        }
        let zeros = vec![0.0f32; n];
        self.process(&zeros, out);
    }

    /// Free-running bit clock, pulled towards the middle of the eye by every
    /// transition.
    ///
    /// Transitions are steered to phase 0.5 and the strobe fires at the wrap,
    /// so the two sit half a bit apart — the strobe lands in the middle of the
    /// eye, which is the whole object. Steering transitions to the strobe phase
    /// instead samples exactly on the edges, where any jitter flips the bit;
    /// that reads as a clock that never locks rather than as a clock that is
    /// half a bit out.
    fn advance_clock(&mut self, bit: bool, out: &mut Vec<bool>) {
        if self.have_last && bit != self.last_bit {
            let mut e = STROBE_PHASE - self.clk;
            if e > 0.5 {
                e -= 1.0;
            } else if e < -0.5 {
                e += 1.0;
            }
            self.clk = (self.clk + CLOCK_TRACK_GAIN * e).rem_euclid(1.0);
        }
        self.last_bit = bit;
        self.have_last = true;

        self.clk += 1.0 / self.spb;
        if self.clk < 1.0 {
            return;
        }
        self.clk -= 1.0;
        out.push(bit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Modulate a bit pattern and demodulate it back, comparing the *settled*
    /// stretch only.
    ///
    /// The leading bits are deliberately excluded. The receiver's clock has to
    /// acquire and its threshold references have to seed, and until they have
    /// it can emit one bit more or fewer than were sent — which on a raw
    /// comparison shifts everything after it and reads as a catastrophic error
    /// rate rather than as the one-bit slip it is. On the air that stretch is
    /// the preamble, where it costs nothing. This cost an hour of chasing a
    /// receiver bug that did not exist, so: compare the tail, and let
    /// `packet_interop.rs` judge correctness against Direwolf.
    fn tail_matches(profile: AfskProfile, bits: &[bool]) -> (usize, usize) {
        let rate = 48_000.0;
        let mut tx = AfskTx::new(rate, profile);
        let preamble: Vec<bool> = (0..64).map(|i| i % 2 == 0).collect();
        tx.push_bits(&preamble);
        tx.push_bits(bits);

        let spb = rate / profile.baud();
        let n = ((preamble.len() + bits.len()) as f64 * spb).ceil() as usize + 4096;
        let mut audio = vec![0.0f32; n];
        tx.next_block(&mut audio);

        let mut rx = AfskRx::new(rate, profile);
        let mut got = Vec::new();
        rx.process(&audio, &mut got);

        // Line the tail of what was sent up against the recovered stream,
        // allowing for the acquisition slip.
        let want = &bits[bits.len() / 2..];
        let mut best = usize::MAX;
        for off in 0..got.len().saturating_sub(want.len()) {
            let wrong = got[off..off + want.len()].iter().zip(want).filter(|(a, b)| a != b).count();
            best = best.min(wrong);
            if best == 0 {
                break;
            }
        }
        (best, want.len())
    }

    #[test]
    fn bell_202_round_trips() {
        let bits: Vec<bool> = (0..200).map(|i| (i * 7 + i / 3) % 5 != 0).collect();
        let (wrong, total) = tail_matches(AfskProfile::Vhf1200, &bits);
        assert_eq!(wrong, 0, "{wrong} of {total} bits wrong at 1200 baud");
    }

    #[test]
    fn hf_300_round_trips() {
        let bits: Vec<bool> = (0..120).map(|i| (i * 3 + i / 5) % 4 != 0).collect();
        let (wrong, total) = tail_matches(AfskProfile::Hf300, &bits);
        assert_eq!(wrong, 0, "{wrong} of {total} bits wrong at 300 baud");
    }

    /// The tone pairs are the ones Direwolf names for `-B 300` and `-B 1200`.
    /// Pinned because agreeing with the deployed network is the whole point,
    /// and a plausible-looking edit here would produce a modem that works
    /// perfectly against itself and against nothing else.
    #[test]
    fn the_tone_pairs_are_the_published_ones() {
        assert_eq!(AfskProfile::Hf300.tones(), (1600.0, 1800.0));
        assert_eq!(AfskProfile::Hf300.baud(), 300.0);
        assert_eq!(AfskProfile::Vhf1200.tones(), (1200.0, 2200.0));
        assert_eq!(AfskProfile::Vhf1200.baud(), 1200.0);
    }

    /// The detector's two rotations are built on mark being *below* space, the
    /// opposite of amateur RTTY. If a profile ever breaks that, the branches
    /// silently swap — and NRZI hides the swap so completely that frames still
    /// decode and nothing looks wrong until someone compares raw bits.
    #[test]
    fn mark_is_the_lower_tone() {
        for p in [AfskProfile::Hf300, AfskProfile::Vhf1200] {
            let (mark, space) = p.tones();
            assert!(mark < space, "{p:?}: the detector assumes mark below space");
        }
    }

    /// The baud rate a modem actually produces, measured from the samples it
    /// emitted for a known number of bits.
    fn measured_baud(rate: f64, profile: AfskProfile) -> f64 {
        let mut tx = AfskTx::new(rate, profile);
        let bits = 2_000usize;
        tx.push_bits(&vec![true; bits]);
        let mut sent = 0usize;
        loop {
            let mut block = [0.0f32; 480];
            let n = tx.next_block(&mut block);
            if n == 0 {
                break;
            }
            sent += n;
            if tx.idle() {
                break;
            }
        }
        rate / (sent as f64 / bits as f64)
    }

    /// Issue #180. The bit clock has to hold its fractional remainder, because
    /// on a software radio the rate it is handed is whatever the decimation
    /// chain landed on and is rarely a whole number of samples a bit.
    ///
    /// 44642.857 Hz is what a HackRF at 10 Msps gives — 37.2 samples a bit —
    /// and rounding that up to 38 put the over on the air at 1174.8 baud, which
    /// is past what a receiver's clock recovery will follow. Every rate that
    /// *is* a whole number of samples a bit, 48 kHz among them, was unaffected,
    /// which is why a sound-card station never saw it.
    #[test]
    fn the_baud_rate_survives_a_rate_that_is_not_whole_samples_a_bit() {
        for rate in [48_000.0, 50_000.0, 44_642.857_142_857_145, 62_500.0, 44_100.0, 48_828.125] {
            for profile in [AfskProfile::Vhf1200, AfskProfile::Hf300] {
                let got = measured_baud(rate, profile);
                let err = (got / profile.baud() - 1.0).abs();
                // A tenth of a percent. The receiver tolerates a couple of
                // percent, which is exactly why the fault was invisible at the
                // rates where it was small.
                assert!(
                    err < 0.001,
                    "{rate} Hz, {profile:?}: {got:.2} baud against {}",
                    profile.baud()
                );
            }
        }
    }

    /// An idle channel must not read as a carrier, or CSMA would never key.
    #[test]
    fn silence_is_not_a_carrier() {
        let mut rx = AfskRx::new(48_000.0, AfskProfile::Vhf1200);
        let mut out = Vec::new();
        rx.process(&vec![0.0; 48_000], &mut out);
        assert!(rx.separation() < 0.2, "silence read as a carrier: {}", rx.separation());
    }

    /// ...and a real signal must.
    #[test]
    fn a_signal_reads_as_a_carrier() {
        let mut tx = AfskTx::new(48_000.0, AfskProfile::Vhf1200);
        let bits: Vec<bool> = (0..600).map(|i| i % 3 != 0).collect();
        tx.push_bits(&bits);
        let mut audio = vec![0.0f32; 48_000];
        tx.next_block(&mut audio);

        let mut rx = AfskRx::new(48_000.0, AfskProfile::Vhf1200);
        let mut out = Vec::new();
        rx.process(&audio, &mut out);
        assert!(rx.separation() > 0.5, "a real signal read as idle: {}", rx.separation());
    }
}
