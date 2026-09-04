//! The burst gate: turns a continuously running channel into discrete
//! transmissions.
//!
//! VDL2 is carrier-sense multiple access. A station listens, transmits for
//! somewhere between three and seventy milliseconds, and is quiet again; even
//! the Common Signalling Channel spends most of its time empty. So the gate
//! watches one channel's baseband power, notices when something arrives, and
//! hands the whole transmission — run-up included — to the decoder as one buffer
//! it can make several passes over.
//!
//! # Why a power gate rather than a continuous search
//!
//! Correlating for the synchronisation word at every sample of every channel
//! would cost fifteen complex multiply-accumulates per trial offset in two
//! spectral senses, on fourteen channels, forever — an order of magnitude more
//! than this costs, to find bursts on channels that are silent most of the time.
//! The gate is one band-limiting filter, `|z|²` per sample and one comparison
//! per thirty-two of them.
//!
//! # Why the power is measured through a filter
//!
//! The channel arrives here ten times as wide as the signal in it: the
//! downconverter ahead of the gate is designed against its own output rate, and
//! the plan is the whole 25 kHz raster, so both of a channel's neighbours are
//! sitting inside the stream at full strength. Measuring `|z|²` on that counts
//! them as this channel's own power, and a ground station a mile away opens the
//! gate on the two channels either side of the one it is transmitting on, which
//! then spend the correlator on nothing. On a recording from Brittany, six of
//! the fourteen channels showed between 39 and 103 bursts and not one
//! synchronisation between them: every one of those was a neighbour.
//!
//! So the power is measured on a copy of the stream band-limited to the channel.
//! It costs a dot product per sample, which is what the rest of this module is
//! arranged to avoid — and it buys back more than it costs, because the
//! correlator is no longer run on other stations' transmissions. It also makes
//! the gate *more* sensitive, not less: the noise in 25 kHz is some 6 dB below
//! the noise in the whole stream, and the threshold is measured against the
//! floor either way.
//!
//! # The noise floor
//!
//! A threshold means nothing without a floor, and the floor has to be learned
//! from a channel that is usually quiet and sometimes not. The obvious rule —
//! only update while the gate is shut — deadlocks both ways: seeded too low the
//! gate never shuts and the floor never updates; seeded too high it never opens,
//! and again never updates.
//!
//! So the floor tracks *down fast and up very slowly*. It converges on the
//! quietest power seen recently, which on a channel like this is the noise, and
//! a burst lasting a thousandth of the rise time barely moves it. Min-tracking
//! of a noisy quantity settles below its mean, so the estimate is scaled back up
//! by [`FLOOR_BIAS`] — a constant this module's own test *measures* against
//! noise of known power rather than reasons about and hopes for. The design is
//! the ISM decoder's, with VDL2's numbers; it is not shared code, because that
//! crate links C behind a feature flag and every one of its constants is wrong
//! here.

use sdroxide_dsp::{Complex32, FirDecim, lowpass_taps};

use crate::demod::{CUTOFF_SYMS, SYMBOL_RATE};

/// Taps in the filter the power is measured through.
///
/// Enough that the stopband has closed by the time it reaches the neighbouring
/// channel's centre 25 kHz away — a windowed sinc's transition is about four
/// sample rates over the tap count, which at a channel rate near 100 kHz and
/// this many taps is 8 kHz, so a cutoff at 12.6 kHz is done by 20 kHz. The
/// neighbour's own shoulder reaches down to 16.6 kHz, and what little of its
/// power lives out there is the far tail of its shaping filter.
///
/// [`tests::a_transmission_on_the_next_channel_does_not_open_the_gate`] is what
/// says the number is big enough.
const DET_TAPS: usize = 49;

/// Samples per power measurement.
///
/// At a channel rate near 100 kHz this is a third of a millisecond, about three
/// and a half symbols. Long enough that the mean of `|z|²` is a usable estimate,
/// and short enough that the shaped envelope's dip as the constellation passes
/// near the origin cannot look like a gap in the transmission.
const BLOCK: usize = 32;

/// Per-block smoothing when the measurement is below the current floor.
const FLOOR_FALL: f32 = 0.05;
/// Per-block smoothing when it is above — three orders of magnitude slower.
const FLOOR_RISE: f32 = 0.00005;

/// Correction from where min-tracking settles to the actual mean noise power.
///
/// Measured by [`tests::the_floor_estimate_lands_on_the_true_noise_power`].
/// Changing `BLOCK`, `FLOOR_FALL`, `FLOOR_RISE` or [`DET_TAPS`] changes this
/// number, and that test is what will say so. It grew when the power began
/// being measured through a filter: a block of 32 samples of a stream
/// band-limited to a quarter of its width holds a quarter as many independent
/// ones, the spread of the block powers is wider for it, and min-tracking
/// settles further below the mean.
const FLOOR_BIAS: f32 = 2.585;

/// How far the power must fall back below the open threshold before the
/// transmission is over, as a power ratio — 6 dB of hysteresis.
const CLOSE_RATIO: f32 = 0.25;

/// Blocks below the close threshold before the gate shuts. About three
/// milliseconds: long enough to ride a momentary dip, short enough not to weld
/// two transmissions from two stations into one.
const HANG_BLOCKS: u32 = 6;

/// Shortest transmission worth decoding, in seconds.
///
/// The smallest real one is the run-up, sixteen synchronisation symbols, nine
/// header symbols and the thirty-five a minimum AVLC frame needs — about three
/// milliseconds. Below that it is a click.
const MIN_BURST_S: f64 = 0.002;

/// Longest transmission kept, in seconds.
///
/// The header's length field tops out at 0x3FFF bits, which is 2048 data octets,
/// nine Reed-Solomon blocks, 5608 symbols and **535.7 ms** on the air. Anything
/// past that is not a VDL2 transmission, so the cap bounds what a stuck carrier
/// can cost — and because an over-long burst is *dropped* rather than emitted, a
/// permanently occupied channel does not hand the decoder the same rubbish over
/// and over.
///
/// The number matters more than it looks. The ISM gate's quarter of a second
/// would silently drop every frame over about a thousand octets, and the symptom
/// — long messages never arriving, short ones fine — reads exactly like a deaf
/// receiver.
const MAX_BURST_S: f64 = 0.7;

/// Pre-trigger history, in seconds.
///
/// Must hold the transmitter's ramp-up *and* the gate's own detection lag, since
/// the synchronisation word arrives immediately after the ramp and the
/// correlator needs all sixteen symbols of it. Sixteen symbols is 1.5 ms and a
/// block of lag is 0.3 ms, so five is ample.
const PRE_S: f64 = 0.005;

/// One captured transmission.
#[derive(Debug, Clone)]
pub struct Burst {
    /// Baseband samples, pre-trigger history first.
    pub iq: Vec<Complex32>,
    pub rate_hz: f64,
    /// Absolute RF centre of the channel it came from.
    pub center_hz: f64,
    /// Peak block power against the channel's learned floor, dB. Both are
    /// measured in the channel's own bandwidth rather than the stream's, so
    /// this is the transmission's signal-to-noise ratio and not a figure with
    /// nine tenths of a stream's noise in the denominator.
    pub snr_db: f32,
    /// Peak block power in absolute dBFS — the comparable figure when two
    /// channels hear the same station, since `snr_db` is referred to each
    /// channel's own floor.
    pub peak_dbfs: f32,
}

/// Burst gate for one channel.
pub struct Gate {
    rate_hz: f64,
    center_hz: f64,
    open_ratio: f32,

    inbuf: Vec<Complex32>,
    /// The stream band-limited to this channel, which is what the power is
    /// measured on. Only the measurement: what the decoder gets handed is the
    /// samples as they arrived, since its own receive filter is a better one
    /// and it wants to place it itself.
    det: FirDecim,
    detbuf: Vec<Complex32>,
    floor: f32,
    tracked: f32,
    seeded: bool,

    pre: Vec<Complex32>,
    pre_w: usize,
    pre_filled: usize,

    cur: Vec<Complex32>,
    cur_pre: usize,
    open: bool,
    hang: u32,
    peak: f32,

    min_samples: usize,
    max_samples: usize,

    /// Transmissions the gate opened on, and ones dropped for running past
    /// [`MAX_BURST_S`]. Reported so a channel sitting under a carrier looks like
    /// what it is rather than like a broken decoder.
    pub opened: u64,
    pub overlong: u64,
}

impl Gate {
    pub fn new(rate_hz: f64, center_hz: f64, threshold_db: f32) -> Gate {
        let pre = (PRE_S * rate_hz).round().max(BLOCK as f64) as usize;
        // The same band the receive filter passes, for the same reason: a
        // receiver a few tens of parts per million out slides the whole signal
        // sideways, and a detector cut to the signal's own shoulder would stop
        // hearing it before the decoder stopped reading it.
        let det_taps = lowpass_taps(DET_TAPS, CUTOFF_SYMS * SYMBOL_RATE / rate_hz);
        Gate {
            rate_hz,
            center_hz,
            open_ratio: 10f32.powf(threshold_db / 10.0),
            inbuf: Vec::with_capacity(BLOCK * 2),
            det: FirDecim::with_taps(det_taps, 1),
            detbuf: Vec::with_capacity(BLOCK * 2),
            floor: 0.0,
            tracked: 0.0,
            seeded: false,
            pre: vec![Complex32::default(); pre],
            pre_w: 0,
            pre_filled: 0,
            cur: Vec::new(),
            cur_pre: 0,
            open: false,
            hang: 0,
            peak: 0.0,
            min_samples: (MIN_BURST_S * rate_hz) as usize,
            max_samples: (MAX_BURST_S * rate_hz) as usize,
            opened: 0,
            overlong: 0,
        }
    }

    /// Change the threshold without losing the learned floor — the noise did not
    /// move because the operator dragged a slider.
    pub fn set_threshold_db(&mut self, db: f32) {
        self.open_ratio = 10f32.powf(db / 10.0);
    }

    /// Current noise-floor estimate as dBFS.
    pub fn floor_dbfs(&self) -> f32 {
        10.0 * self.floor.max(1e-30).log10()
    }

    /// Feed baseband samples; append any completed transmissions to `out`.
    pub fn push(&mut self, iq: &[Complex32], out: &mut Vec<Burst>) {
        let mut buf = std::mem::take(&mut self.inbuf);
        let mut det = std::mem::take(&mut self.detbuf);
        buf.extend_from_slice(iq);
        self.det.process(iq, &mut det);

        // The two queues carry the same samples and drain together, so they
        // stay in step: the filter holds its first taps back and never catches
        // up, which puts the power measurement half a tap length ahead of the
        // block it is measured against. Half a tap length is a fifth of a
        // millisecond, and [`PRE_S`] of run-up is kept in front of every burst.
        let mut pos = 0usize;
        let usable = buf.len().min(det.len());
        while pos + BLOCK <= usable {
            let power =
                det[pos..pos + BLOCK].iter().map(|z| z.norm_sqr()).sum::<f32>() / BLOCK as f32;
            self.advance(&buf[pos..pos + BLOCK], power, out);
            pos += BLOCK;
        }
        if pos > 0 {
            buf.drain(..pos);
            det.drain(..pos);
        }
        self.inbuf = buf;
        self.detbuf = det;
    }

    fn advance(&mut self, block: &[Complex32], power: f32, out: &mut Vec<Burst>) {
        if !self.seeded {
            self.tracked = power;
            self.seeded = true;
        } else {
            let alpha = if power < self.tracked { FLOOR_FALL } else { FLOOR_RISE };
            self.tracked += alpha * (power - self.tracked);
        }
        self.floor = self.tracked * FLOOR_BIAS;
        let open_at = self.floor * self.open_ratio;

        if !self.open {
            self.push_pre(block);
            if power > open_at {
                self.open = true;
                self.hang = 0;
                self.peak = power;
                self.opened += 1;
                self.cur.clear();
                self.take_pre_into_cur();
                self.cur_pre = self.cur.len();
            }
            return;
        }

        self.cur.extend_from_slice(block);
        self.peak = self.peak.max(power);

        if power < open_at * CLOSE_RATIO {
            self.hang += 1;
        } else {
            self.hang = 0;
        }

        let too_long = self.cur.len() > self.max_samples;
        if self.hang < HANG_BLOCKS && !too_long {
            return;
        }

        self.open = false;
        // The hangover is *not* trimmed, unlike the ISM gate's. There the burst
        // length is all the decoders have to go on; here the frame's length
        // comes from its own header, so trailing silence costs nothing and
        // cutting it risks clipping the last symbols of a frame that ended in a
        // run of small-envelope transitions.
        if too_long {
            self.overlong += 1;
        } else if self.cur.len().saturating_sub(self.cur_pre) >= self.min_samples {
            out.push(Burst {
                iq: std::mem::take(&mut self.cur),
                rate_hz: self.rate_hz,
                center_hz: self.center_hz,
                snr_db: 10.0 * (self.peak / self.floor.max(1e-30)).max(1e-30).log10(),
                peak_dbfs: 10.0 * self.peak.max(1e-30).log10(),
            });
        }
        self.cur.clear();
        // The pre-trigger ring is now the tail of the transmission just
        // emitted; dropping it stops the next one being prefixed with the last.
        self.pre_filled = 0;
        self.pre_w = 0;
    }

    fn push_pre(&mut self, block: &[Complex32]) {
        for &z in block {
            self.pre[self.pre_w] = z;
            self.pre_w = (self.pre_w + 1) % self.pre.len();
            self.pre_filled = (self.pre_filled + 1).min(self.pre.len());
        }
    }

    fn take_pre_into_cur(&mut self) {
        let n = self.pre_filled;
        let len = self.pre.len();
        let start = (self.pre_w + len - n) % len;
        for i in 0..n {
            self.cur.push(self.pre[(start + i) % len]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::Noise;

    /// [`FLOOR_BIAS`] is measured, not guessed: feed noise of known power and
    /// check the settled estimate against it.
    ///
    /// The power to check against is the noise in the *channel*, not in the
    /// stream: the gate measures through [`DET_TAPS`] taps of low-pass, which
    /// pass a fraction of a white spectrum equal to the sum of their squares.
    /// That is where six of the decibels between this figure and the stream's
    /// own noise power went, and it is the whole point — the floor a
    /// transmission is compared against is the noise it actually competes with.
    #[test]
    fn the_floor_estimate_lands_on_the_true_noise_power() {
        let rate = 100_000.0;
        let mut n = Noise::new(0x1234_5678);
        // Two components at sigma each, so the mean power is 2·sigma².
        let sigma = 0.03f32;
        let band: f32 =
            lowpass_taps(DET_TAPS, CUTOFF_SYMS * SYMBOL_RATE / rate).iter().map(|t| t * t).sum();
        let want = 2.0 * sigma * sigma * band;
        let mut g = Gate::new(rate, 136_975_000.0, 9.0);
        let mut out = Vec::new();
        let mut buf = vec![Complex32::default(); 4096];
        for _ in 0..40 {
            for s in buf.iter_mut() {
                *s = n.gaussian(sigma);
            }
            g.push(&buf, &mut out);
        }
        let err_db = 10.0 * (g.floor / want).log10();
        assert!(err_db.abs() < 1.0, "floor is {err_db} dB from the true noise power");
        assert!(out.is_empty(), "noise alone opened the gate {} times", out.len());
    }

    /// A transmission is caught whole, with its run-up in front of it — which
    /// is the only reason the synchronisation word survives the gate's own
    /// detection lag.
    #[test]
    fn a_burst_is_caught_with_its_run_up() {
        let rate = 100_000.0;
        let frame = vec![0x5au8; 40];
        let p = crate::tx::TxParams { sample_rate: rate, ..crate::tx::TxParams::default() };
        let burst = crate::tx::modulate(&frame, &p, 10.0);

        let mut n = Noise::new(99);
        let mut g = Gate::new(rate, 136_975_000.0, 9.0);
        let mut out = Vec::new();
        // Quiet, then the transmission, then quiet again.
        let mut quiet = vec![Complex32::default(); 20_000];
        n.add(&mut quiet, 0.01);
        g.push(&quiet, &mut out);
        let mut sig = burst.clone();
        n.add(&mut sig, 0.01);
        g.push(&sig, &mut out);
        let mut quiet2 = vec![Complex32::default(); 20_000];
        n.add(&mut quiet2, 0.01);
        g.push(&quiet2, &mut out);

        assert_eq!(out.len(), 1, "expected exactly one transmission");
        let b = &out[0];
        assert!(b.iq.len() >= burst.len(), "the burst was clipped");
        assert!(b.snr_db > 20.0, "signal-to-noise {}", b.snr_db);
        assert_eq!(b.center_hz, 136_975_000.0);
    }

    /// A transmission on the next channel up does not open this one's gate,
    /// however strong it is.
    ///
    /// The case that made this necessary is an operator living beside an
    /// airport: the ground station 25 kHz away arrives tens of decibels above
    /// everything else, and before the detection filter it was counted as a
    /// burst here — 46 of them in 25 seconds on one recording, none of which
    /// was ever going to synchronise.
    #[test]
    fn a_transmission_on_the_next_channel_does_not_open_the_gate() {
        let rate = 96_000.0;
        let frame = vec![0x5au8; 40];
        let p = crate::tx::TxParams { sample_rate: rate, ..crate::tx::TxParams::default() };
        let burst = crate::tx::modulate(&frame, &p, 10.0);

        // 40 dB above the noise the gate is learning its floor from, which is
        // what a local ground station looks like.
        let hear = |offset_hz: f64| -> usize {
            let mut n = Noise::new(0xbeef);
            let mut g = Gate::new(rate, 136_900_000.0, 9.0);
            let mut out = Vec::new();
            let mut quiet = vec![Complex32::default(); 40_000];
            n.add(&mut quiet, 0.001);
            g.push(&quiet, &mut out);

            let w = std::f64::consts::TAU * offset_hz / rate;
            let mut sig: Vec<Complex32> = burst
                .iter()
                .enumerate()
                .map(|(i, &z)| {
                    let ph = w * i as f64;
                    z * Complex32::new(ph.cos() as f32, ph.sin() as f32) * 0.1
                })
                .collect();
            n.add(&mut sig, 0.001);
            g.push(&sig, &mut out);
            let mut quiet2 = vec![Complex32::default(); 20_000];
            n.add(&mut quiet2, 0.001);
            g.push(&quiet2, &mut out);
            out.len()
        };

        assert_eq!(hear(0.0), 1, "the gate missed a transmission on its own channel");
        assert_eq!(hear(25_000.0), 0, "the channel above opened the gate");
        assert_eq!(hear(-25_000.0), 0, "the channel below opened the gate");
    }

    /// A carrier that never stops is dropped and counted, not accumulated until
    /// the process runs out of memory.
    #[test]
    fn a_stuck_carrier_is_dropped_and_counted() {
        let rate = 100_000.0;
        let mut g = Gate::new(rate, 136_975_000.0, 9.0);
        let mut out = Vec::new();
        let mut n = Noise::new(5);
        let mut quiet = vec![Complex32::default(); 8192];
        n.add(&mut quiet, 0.01);
        g.push(&quiet, &mut out);

        let carrier = vec![Complex32::new(1.0, 0.0); (MAX_BURST_S * rate) as usize + 8192];
        g.push(&carrier, &mut out);
        // Then silence, so it closes.
        g.push(&quiet, &mut out);
        assert!(out.is_empty(), "an endless carrier was emitted as a transmission");
        assert_eq!(g.overlong, 1);
    }

    /// The cap is long enough for the longest frame the standard can describe.
    #[test]
    fn the_cap_clears_the_longest_frame_the_standard_allows() {
        let max_symbols = (crate::header::HEADER_BITS + 2100 * 8).div_ceil(3);
        let seconds = max_symbols as f64 / crate::demod::SYMBOL_RATE;
        assert!(seconds < 0.54, "the longest frame is {seconds} s");
        assert!(MAX_BURST_S > seconds * 1.2, "MAX_BURST_S leaves no margin over {seconds} s");
    }
}
