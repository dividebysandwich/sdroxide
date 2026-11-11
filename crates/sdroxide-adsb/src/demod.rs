//! The 1090 MHz pulse-position demodulator: complex baseband in, candidate
//! Mode S messages out.
//!
//! # The waveform
//!
//! Mode S downlink (ICAO Annex 10 Volume IV, RTCA DO-260B) is on-off keying at
//! one megabit a second, and every reply has the same two parts:
//!
//! * an **8 µs preamble** — 0.5 µs pulses beginning at 0.0, 1.0, 3.5 and
//!   4.5 µs, with everything between and after them dark until the data
//!   starts. The odd spacing is the point: it is a pattern noise does not make.
//! * a **data block** of either 56 or 112 bits, each bit 1 µs long and split
//!   into two half-microsecond chips. Energy in the first chip is a `1`, energy
//!   in the second is a `0`. Which length it is comes from the first five bits,
//!   the downlink format: 16 and above are the long ones.
//!
//! So a short reply is 8 + 56 = 64 µs and a long one 120 µs, and everything
//! this module does is measured in microseconds rather than in samples.
//!
//! # Why it is written in microseconds, and why the positions are fractional
//!
//! There is no resampler in front of this. The engine hands over whatever its
//! downconverter settled on — 2.4 Msps from an RTL-SDR, 2.025 from an RX-888's
//! wideband path, 2.5 from an Airspy — and the decoder works in *time*, not in
//! samples: [`Demod::chip`] integrates the envelope over the half-microsecond
//! window a chip actually occupies, wherever that falls between samples.
//!
//! That last part is not a refinement, it is the whole thing working or not.
//! A transponder's burst arrives at an arbitrary moment, so the chip boundaries
//! fall wherever they like relative to the sample grid. Round them to the
//! nearest sample and the error is up to half a sample — which at two samples
//! per bit is **half a chip**, and half the bits in the message get decided on
//! the wrong side. Measured, on clean full-scale bursts swept across one sample
//! period: 5 % recovered at 2.025 Msps, 53 % at 2.4, against 100 % above 4.
//! Everything below 4 Msps was effectively broken, and the front end most
//! people point at 1090 MHz runs at 2.4.
//!
//! So the correlator searches at half-sample steps, refines the burst's start
//! time to a sixteenth of a sample against the preamble, and slices with
//! fractional windows from there.
//!
//! # Power, not magnitude
//!
//! Every comparison this module makes is between two envelope levels, and
//! `a > b` has the same answer for `|z|` as for `|z|²`. So the hot path squares
//! and never takes a root; the one square root per accepted message is in
//! [`Candidate::rssi_dbfs`], where a decibel is actually wanted.
//!
//! # Blocks are not messages
//!
//! A long reply is 120 µs — about 288 samples at 2.4 Msps — and the engine's
//! blocks are not aligned to anything. [`Demod::push`] therefore keeps the tail
//! of the previous block in front of the new one, so a message that straddles a
//! boundary is still seen whole. Dropping those would not look like a bug; it
//! would look like a receiver a few decibels deaf.

use sdroxide_dsp::Complex32;

/// Bit period, microseconds.
const BIT_US: f64 = 1.0;
/// Preamble length, microseconds — where the data starts.
const PREAMBLE_US: f64 = 8.0;
/// The four preamble pulse positions, microseconds from the start.
const PULSES_US: [f64; 4] = [0.0, 1.0, 3.5, 4.5];
/// The gaps between and after them that must be dark.
const SPACES_US: [f64; 6] = [0.5, 1.5, 2.0, 3.0, 4.0, 5.0];
/// Long-message length in bits (DF >= 16); the short one is 56.
const LONG_BITS: usize = 112;
const SHORT_BITS: usize = 56;

/// Total span a long message occupies, microseconds.
const MSG_US: f64 = PREAMBLE_US + LONG_BITS as f64 * BIT_US;

/// How much of the previous block to carry forward, microseconds.
///
/// One whole long message plus a little, so a preamble that begins in the last
/// microsecond of a block still has its data available on the next pass.
const TAIL_US: f64 = MSG_US + 4.0;

/// How far above the noise floor the weakest preamble pulse must sit.
///
/// The preamble's own shape does nearly all the work — four pulses at
/// irregular spacing, each of which has to beat every one of the six dark
/// chips — so this is not the sensitivity limit. It is what stops the
/// correlator spending 112 bit comparisons on a stretch of pure noise whose
/// samples happen to fall in the right order, which at 2.4 million samples a
/// second is often.
///
/// Measured against the noise's **mean** power ([`Demod::noise_floor`]), and
/// 2.5 is where it costs nothing: on complex Gaussian noise the correlator's
/// false candidates fall from ~98 000 a second at 2.4 Msps to under a hundred,
/// and the worker from ~90 % of a core to under 10 % (from ~280 % to ~22 % at
/// 8 Msps), while recall on a DF17 swept across every arrival phase is the same
/// to the message at 2.4 Msps from 12 dB SNR and at 4 Msps from 10 dB; only at
/// the very edge (8 dB) does it give up one or two in forty. The contrast gates
/// below are what set the sensitivity; this one only keeps them from being
/// asked about noise.
const PULSE_OVER_NOISE: f32 = 2.5;

/// What the lower quartile of the envelope power is, as a fraction of its mean,
/// on receiver noise.
///
/// The noise a receiver hands over is complex Gaussian, so its power is
/// exponentially distributed and its lower quartile sits at `ln(4/3)` ≈ 0.288
/// of the mean. [`Demod::track_noise`] measures the quartile — it is what stays
/// put under traffic — and divides by this to get the mean back.
///
/// Reading the quartile *as* the floor, which is what this decoder first did,
/// put the gate at 3 × 0.288 = 0.86 of the mean noise power: below the average
/// noise sample. Gaussian noise crosses that about 40 % of the time, so the
/// first-chip rejection let through nearly half of every second, each of them
/// paid for the refine and up to seven 112-bit slices, and the worker sat at
/// 100 % of a core on an empty sky (Kevin's Pluto, fork discussion #16). The
/// test that should have caught it used *uniform* noise, whose power has no
/// tail to cross.
const Q1_OF_MEAN: f32 = 0.287_682;

/// How well the preamble has to match, as a normalised contrast between the
/// four lit chips and the six dark ones — see [`Demod::contrast`].
///
/// # Why this is not a pulse-to-space ratio
///
/// It used to be one: the weakest pulse had to be twice the loudest space.
/// That number is unreachable. A 0.5 µs pulse sampled every 0.5 µs is exactly
/// at the limit, and at the worst arrival phase each pulse straddles two
/// samples that are half lit — so the 0.5 µs gap between the preamble's first
/// two pulses is *filled in* by two half-lit samples and reads exactly as
/// strongly as the pulses either side of it. The ratio is 1.0 no matter what
/// the signal-to-noise is, so no threshold on it can pass a perfect signal,
/// and the min/max score it was refined against is flat — there is not even a
/// gradient to find the alignment with.
///
/// A contrast between the *sums* has neither problem: it stays positive
/// through the worst phase (about 0.26 for a noiseless burst at 2 Msps), it
/// varies smoothly with alignment, and it is what the refinement climbs.
const CONTRAST_MIN: f32 = 0.24;

/// The same for the coarse pass, which is up to a quarter of a sample out and
/// therefore reads a duller preamble than the refined pass will.
const COARSE_CONTRAST: f32 = 0.14;

/// Sub-sample offsets tried when the coarse pass fires, as a fraction of a
/// sample either side.
///
/// The coarse search is at half-sample steps, so the truth is within a quarter
/// of a sample; a third either way covers it with margin. Sixteenths, because
/// what this has to deliver is a chip window aligned to a fraction of its own
/// width — at 2 Msps a sixteenth of a sample is 0.03 µs against a 0.5 µs chip.
const REFINE_SPAN: f64 = 0.34;
const REFINE_STEP: f64 = 1.0 / 16.0;

/// Extra slicing alignments to try, in samples either side of the one the
/// preamble chose, when the first read does not check out.
///
/// Ordered outward, so the likeliest correction is tried first. They are only
/// reached on a format whose check sequence can settle the question, which is
/// also what keeps them off the hot path: a candidate raised by noise slices to
/// an unverifiable format nine times in ten and returns immediately.
const SLICE_PHASES: [f64; 6] = [-0.1, 0.1, -0.2, 0.2, -0.35, 0.35];

/// A message the slicer produced, before anything has checked it.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// 7 or 14 bytes, most significant bit of byte 0 first.
    pub bytes: Vec<u8>,
    /// Mean `|a - b| / (a + b)` over the sliced bits: 1.0 when every bit was
    /// unambiguous, near 0 when the slicer was guessing. Reported rather than
    /// acted on — the CRC is the gate, and a low-confidence message that passes
    /// a 24-bit check is a real message.
    pub confidence: f32,
    /// Peak envelope of the preamble, dBFS. Negative on any real signal.
    pub rssi_dbfs: f32,
}

/// Rolling demodulator over one receiver window.
pub struct Demod {
    rate_hz: f64,
    /// Samples per microsecond — the only rate-dependent number in the module.
    sps_us: f64,
    /// Envelope power, with [`TAIL_US`] of the previous block still in front.
    power: Vec<f32>,
    /// Running sum of `power`, one longer: `psum[j]` is the total before sample
    /// `j`.
    ///
    /// The power is constant across a sample, so this is piecewise linear and
    /// interpolating it gives the integral over a *fractional* range exactly —
    /// which turns every chip from a loop over the samples it touches into two
    /// interpolations. That is what makes the fractional windows affordable:
    /// the loop version cost half a core at 2 Msps and could not keep up at 4.
    ///
    /// `f64` because it accumulates across a whole block, and the differences
    /// taken from it are individual samples.
    psum: Vec<f64>,
    /// How many samples of `power` are carried over from last time.
    carried: usize,
    /// Slowly-tracked mean noise power, in power units.
    noise: f32,
    /// The floor has seen at least one block.
    ///
    /// Without this the tracker eases down from its initial guess over several
    /// blocks, and a decoder that has just started — or has just had its window
    /// rebuilt by a retune — is deaf to weak aircraft for the whole of that.
    primed: bool,
    /// Preambles accepted, and messages sliced out of them.
    pub preambles: u64,
    /// Samples seen since the decoder started, so a position can be compared
    /// across blocks after the buffer has been trimmed.
    seen: u64,
    /// The last message emitted and where, to suppress the duplicate a strong
    /// burst produces from two neighbouring alignments.
    last_out: Option<(Vec<u8>, u64)>,
    /// Absolute position to resume scanning at, after a message that checked
    /// out. Carried across blocks, because a message read at the end of one
    /// runs into the next.
    skip_to: u64,
}

impl Demod {
    /// Build a demodulator for a stream at `rate_hz`.
    ///
    /// Any rate is accepted; [`sdroxide_types::ADSB_MIN_RATE_HZ`] is the caller's
    /// business, because "this receiver cannot do ADS-B" is a thing to tell the
    /// operator rather than a panic.
    pub fn new(rate_hz: f64) -> Demod {
        Demod {
            rate_hz,
            sps_us: rate_hz / 1e6,
            power: Vec::new(),
            psum: vec![0.0],
            carried: 0,
            // Starts pessimistic and tracks down: a floor that begins at zero
            // would accept everything until the first block replaces it.
            noise: 1e-3,
            primed: false,
            preambles: 0,
            seen: 0,
            last_out: None,
            skip_to: 0,
        }
    }

    pub fn rate_hz(&self) -> f64 {
        self.rate_hz
    }

    /// The mean noise power the gate is measuring against, in power units. For
    /// the tests and the replay tool.
    pub fn noise_floor(&self) -> f32 {
        self.noise
    }

    /// Feed one block and collect every message found in it.
    ///
    /// The tail of the previous block is searched again as far as one message
    /// length in, which is what makes a straddling reply decodable; the scan
    /// stops one message length before the end of the new data so the same
    /// message is not found twice from the two sides of a boundary.
    pub fn push(&mut self, iq: &[Complex32], out: &mut Vec<Candidate>) {
        if iq.is_empty() {
            return;
        }
        // Keep the tail, drop everything older.
        let tail = (TAIL_US * self.sps_us).ceil() as usize;
        if self.power.len() > tail {
            self.power.drain(..self.power.len() - tail);
        }
        self.carried = self.power.len();
        self.power.reserve(iq.len());
        for z in iq {
            self.power.push(z.re * z.re + z.im * z.im);
        }
        self.seen += iq.len() as u64;
        self.psum.clear();
        self.psum.reserve(self.power.len() + 1);
        let mut acc = 0.0f64;
        self.psum.push(0.0);
        for &p in &self.power {
            acc += f64::from(p);
            self.psum.push(acc);
        }
        self.track_noise();

        let span = (MSG_US * self.sps_us).ceil() as usize;
        if self.power.len() <= span {
            return;
        }
        let last = self.power.len() - span;
        // Absolute position of `power[0]` in the stream, so the de-duplicator's
        // memory survives the buffer being trimmed between blocks.
        let origin = self.seen.saturating_sub(self.power.len() as u64);
        let mut skip_to = self.skip_to;
        for i in 0..last {
            if origin + (i as u64) < skip_to {
                continue;
            }
            let Some(c) = self.try_at(i) else { continue };
            self.preambles += 1;
            // One sample at a time, never skipping a message length on a hit.
            //
            // Skipping was the obvious economy and it was wrong: a burst is not
            // the only thing that trips the correlator, and a false trip on
            // noise a few microseconds ahead of a real aeroplane would step
            // straight over it. With a gate loose enough to catch every arrival
            // phase, that was happening often enough to hide most of the sky.
            //
            // What skipping was really for is the duplicate a strong burst
            // produces when two neighbouring alignments both resolve it, and
            // that is better answered by recognising the duplicate.
            let at = origin + i as u64;
            let dup = self
                .last_out
                .as_ref()
                .is_some_and(|(b, p)| *b == c.bytes && at.saturating_sub(*p) < span as u64);
            if dup {
                continue;
            }
            // A message whose own check sequence comes out is one we have
            // finished with, and its 112 bits of data will otherwise trip the
            // correlator dozens more times on the way past — which is most of
            // what this loop costs on a busy band. Skipping only on *that*
            // gets the economy without the blindness: nothing is stepped over
            // except a message already read.
            //
            // Only the formats that carry a plain check sequence can say so. A
            // surveillance reply's parity has an address mixed into it and
            // means nothing here, so those are read the slow way.
            let df = c.bytes[0] >> 3;
            let done = matches!(df, 11 | 17 | 18) && crate::crc::syndrome(&c.bytes) == 0;
            self.last_out = Some((c.bytes.clone(), at));
            out.push(c);
            if done {
                skip_to = at + span as u64;
            }
        }
        self.skip_to = skip_to;
        // Anything from `last` on stays for the next block to look at.
        self.carried = 0;
    }

    /// Total energy up to a fractional sample position, by interpolating the
    /// running sum.
    #[inline]
    fn upto(&self, x: f64) -> f64 {
        let n = self.power.len();
        if x <= 0.0 {
            return 0.0;
        }
        if x >= n as f64 {
            return self.psum[n];
        }
        let i = x as usize;
        self.psum[i] + (x - i as f64) * f64::from(self.power[i])
    }

    /// Mean envelope power over a *fractional* range of sample indices.
    ///
    /// The partial samples at each end contribute in proportion to how much of
    /// them the range covers, which is the reverse of what the ADC did to the
    /// pulse on the way in. This is the whole reason the decoder works below
    /// 4 Msps, and doing it in two interpolations rather than a loop is what
    /// makes it affordable.
    #[inline]
    fn energy(&self, x0: f64, x1: f64) -> f32 {
        let w = x1 - x0;
        if w <= 0.0 {
            return 0.0;
        }
        ((self.upto(x1) - self.upto(x0)) / w) as f32
    }

    /// Envelope power over the half-microsecond chip beginning `us` after the
    /// burst starts at fractional sample position `base`.
    #[inline]
    fn chip(&self, base: f64, us: f64) -> f32 {
        self.energy(base + us * self.sps_us, base + (us + 0.5) * self.sps_us)
    }

    /// How well a preamble sits at `base`: its contrast, the weakest of its
    /// four lit chips, and the peak.
    ///
    /// The contrast is `(lit - dark) / (lit + dark)` over the mean of each
    /// group — 1.0 for a noiseless burst perfectly aligned, 0 for anything with
    /// no preamble in it, and negative where the pattern is inverted. It is a
    /// difference of sums rather than a ratio of extremes for the reason
    /// [`CONTRAST_MIN`] gives: the extremes are equal at the worst arrival
    /// phase however strong the signal, and the sums are not.
    #[inline]
    fn contrast(&self, base: f64) -> (f32, f32, f32) {
        let mut lit = 0.0f32;
        let mut weakest = f32::MAX;
        let mut peak = 0.0f32;
        for us in PULSES_US {
            let p = self.chip(base, us);
            lit += p;
            weakest = weakest.min(p);
            peak = peak.max(p);
        }
        let mut dark = 0.0f32;
        for us in SPACES_US {
            dark += self.chip(base, us);
        }
        let lit = lit / PULSES_US.len() as f32;
        let dark = dark / SPACES_US.len() as f32;
        let total = lit + dark;
        let q = if total > 0.0 { (lit - dark) / total } else { 0.0 };
        (q, weakest, peak)
    }

    /// Try to read a message whose preamble starts near sample `i`.
    ///
    /// Three passes over the same four pulses and six spaces: a coarse one at
    /// half-sample steps to find candidates cheaply, a refinement to a
    /// sixteenth of a sample to place the burst, and the real test at that
    /// place. Only then is anything sliced.
    fn try_at(&self, i: usize) -> Option<Candidate> {
        // ── coarse ──
        //
        // Half-sample steps: at whole ones the worst case is a quarter of a
        // chip out at 2 Msps, which is enough to dull the contrast past the
        // point where a threshold loose enough to catch it would fire on
        // anything.
        let floor = self.noise * PULSE_OVER_NOISE;
        let mut base = f64::NAN;
        let mut best_coarse = f32::NEG_INFINITY;
        for half in [0.0f64, 0.5] {
            let b = i as f64 + half;
            // The cheapest possible rejection, and the one that decides what
            // this costs: a preamble begins with a pulse, so if the very first
            // lit chip is in the noise there is nothing here. Two
            // interpolations, and it turns away all but a fraction of a percent
            // of the samples in a quiet band before any of the other nine chips
            // are touched.
            if self.chip(b, PULSES_US[0]) < floor {
                continue;
            }
            let (q, weakest, _) = self.contrast(b);
            if weakest < floor || q < COARSE_CONTRAST {
                continue;
            }
            // The better of the two, not the first that passes: taking the
            // first put the refinement's window on the wrong side of the truth
            // whenever both were good enough to fire.
            if q > best_coarse {
                best_coarse = q;
                base = b;
            }
        }
        if base.is_nan() {
            return None;
        }

        // ── refine ──
        //
        // Climb the contrast. It is smooth in the alignment and peaks where the
        // lit windows hold whole pulses and the dark ones hold none.
        let mut best = (f32::NEG_INFINITY, base);
        let mut d = -REFINE_SPAN;
        while d <= REFINE_SPAN + 1e-9 {
            let b = base + d;
            let (q, _, _) = self.contrast(b);
            if q > best.0 {
                best = (q, b);
            }
            d += REFINE_STEP;
        }
        let base = best.1;

        // ── the real test, at the place the burst actually starts ──
        let (q, weakest, peak) = self.contrast(base);
        if weakest < floor || q < CONTRAST_MIN {
            return None;
        }

        // ── slice, letting the check sequence choose the alignment ──
        //
        // The preamble says where the burst starts, but only to the accuracy
        // eight microseconds of it can support, and the message runs for a
        // hundred and twelve more. A start that best fits the preamble is not
        // always the one that reads the data correctly, and at 2.4 Msps a
        // twentieth of a sample is the difference.
        //
        // So try the neighbourhood and let the check sequence say which was
        // right — it is a 24-bit test, and a wrong alignment does not pass it.
        // This is what a comparison against dump1090_rs on its own off-air
        // recordings turned up: the preamble-only alignment found 11 of its 14
        // messages, and the three it missed were not weak, they were sliced
        // from a start a fraction of a sample out.
        let first = self.slice(base, peak)?;
        let df = first.bytes[0] >> 3;
        // Nothing to choose with, on a format whose parity carries an address.
        if !matches!(df, 11 | 17 | 18) || crate::crc::syndrome(&first.bytes) == 0 {
            return Some(first);
        }
        for d in SLICE_PHASES {
            let Some(c) = self.slice(base + d, peak) else { continue };
            let df = c.bytes[0] >> 3;
            if matches!(df, 11 | 17 | 18) && crate::crc::syndrome(&c.bytes) == 0 {
                return Some(c);
            }
        }
        Some(first)
    }

    /// Read the message out from a given start, as bits.
    fn slice(&self, base: f64, peak: f32) -> Option<Candidate> {
        let mut bytes = [0u8; 14];
        let mut conf_sum = 0.0f32;
        let mut nbits = LONG_BITS;
        let mut k = 0usize;
        while k < nbits {
            let t = PREAMBLE_US + k as f64 * BIT_US;
            let a = self.chip(base, t);
            let b = self.chip(base, t + 0.5);
            if a > b {
                bytes[k / 8] |= 0x80 >> (k % 8);
            }
            let sum = a + b;
            conf_sum += if sum > 0.0 { (a - b).abs() / sum } else { 0.0 };
            k += 1;
            if k == 5 {
                // Downlink format: bit 0 of the five is worth 16.
                let df = bytes[0] >> 3;
                nbits = if df >= 16 { LONG_BITS } else { SHORT_BITS };
            }
        }

        // A message of all zeroes or all ones is what an unmodulated carrier
        // and a dead channel look like; neither is worth a CRC.
        let len = nbits / 8;
        let msg = &bytes[..len];
        if msg.iter().all(|&b| b == 0) || msg.iter().all(|&b| b == 0xff) {
            return None;
        }

        // dBFS against a full-scale complex sample, which is what every source
        // in this tree normalises to.
        let rssi_dbfs = 10.0 * peak.max(1e-12).log10();
        Some(Candidate {
            bytes: msg.to_vec(),
            confidence: conf_sum / nbits as f32,
            rssi_dbfs: rssi_dbfs.min(0.0),
        })
    }

    /// Track the channel's noise floor from the block just pushed.
    ///
    /// The floor is taken as a low quantile of a coarse sample of the block
    /// rather than as its mean: half of a busy second at 1090 MHz is other
    /// people's transmissions, and a mean over those is not a noise floor at
    /// all. Tracking is asymmetric for the reason the ISM gate's is — down
    /// quickly, up slowly — so a burst of traffic cannot walk the threshold up
    /// behind itself and deafen the receiver for the next second.
    fn track_noise(&mut self) {
        let fresh = &self.power[self.carried..];
        if fresh.len() < 64 {
            return;
        }
        // Every 37th sample: coprime with any plausible period in the signal,
        // so the sample is not synchronised to the traffic it is measuring.
        let mut sample: Vec<f32> = fresh.iter().step_by(37).copied().collect();
        sample.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // The quartile, scaled up to the mean it implies — see `Q1_OF_MEAN`.
        let q = sample[sample.len() / 4] / Q1_OF_MEAN;
        if !self.primed {
            self.primed = true;
            self.noise = q.max(1e-12);
            return;
        }
        if q < self.noise {
            self.noise = 0.7 * self.noise + 0.3 * q;
        } else {
            self.noise = 0.99 * self.noise + 0.01 * q;
        }
        self.noise = self.noise.max(1e-12);
    }
}

/// Modulate a Mode S reply the way a transponder does: the preamble, then one
/// on-off keyed chip per half-microsecond, into `nf` of complex noise.
///
/// Starts the burst 10 µs in. See [`modulate_at`] for why that is a parameter
/// worth having.
pub fn modulate(msg: &[u8], rate_hz: f64, amp: f32, nf: f32, seed: u64) -> Vec<Complex32> {
    modulate_at(msg, rate_hz, 10.0, amp, nf, seed)
}

/// The same, starting at an arbitrary — and deliberately fractional — time.
///
/// # Why the pulses are integrated rather than stamped onto samples
///
/// A transponder does not know where this receiver's sample instants are. Its
/// pulses begin whenever they begin, and each sample the ADC delivers is the
/// energy over that sample's own window — so a pulse edge falling in the middle
/// of a sample gives a *half-height* sample, and every burst on the air lands
/// at a different sub-sample phase.
///
/// A generator that instead rounded each pulse to the nearest sample would
/// produce signals no aircraft transmits, and — because the decoder used to
/// round the same way — would have agreed with it perfectly while both were
/// wrong. That is exactly what happened: this crate's tests passed at every
/// rate while the decoder recovered 5 % of real bursts at 2.025 Msps. The
/// integration below is what makes the round trip mean something.
///
/// Public because the tests here, the engine's integration test and the
/// `adsb_iq` example all need a transmitter, and it has to be *the same* one.
/// `seed` makes the noise deterministic; there is no `rand` in this tree, and a
/// decoder test that fails one run in fifty is worse than no test at all.
pub fn modulate_at(
    msg: &[u8],
    rate_hz: f64,
    t0_us: f64,
    amp: f32,
    nf: f32,
    seed: u64,
) -> Vec<Complex32> {
    let sps_us = rate_hz / 1e6;
    let bits = msg.len() * 8;
    // A whole long-message span of quiet behind: the scan deliberately stops
    // one message short of the end of what it has, so a burst any closer to the
    // end than that waits for a block a caller may never send.
    let total_us = t0_us + PREAMBLE_US + bits as f64 + MSG_US + 8.0;
    let n = (total_us * sps_us).ceil() as usize;

    // Every pulse's start time, in microseconds. Each is 0.5 µs long.
    let mut pulses: Vec<f64> = PULSES_US.iter().map(|u| t0_us + u).collect();
    for k in 0..bits {
        let bit = msg[k / 8] & (0x80 >> (k % 8)) != 0;
        let t = t0_us + PREAMBLE_US + k as f64 * BIT_US;
        pulses.push(if bit { t } else { t + 0.5 });
    }

    let mut st = seed | 1;
    let mut rnd = move || {
        st ^= st << 13;
        st ^= st >> 7;
        st ^= st << 17;
        ((st >> 40) as f32 / 8_388_608.0) - 1.0
    };
    (0..n)
        .map(|i| {
            let a = i as f64 / sps_us;
            let b = (i + 1) as f64 / sps_us;
            // How much of this sample's window the pulses cover. The pulses
            // never overlap, so this is a plain sum.
            let mut cover = 0.0;
            for &p in &pulses {
                let lo = p.max(a);
                let hi = (p + 0.5).min(b);
                if hi > lo {
                    cover += hi - lo;
                }
            }
            let level = amp * (cover / (b - a)).clamp(0.0, 1.0) as f32;
            Complex32::new(level + nf * rnd(), nf * rnd())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real DF17 position squitter, from the published test vectors.
    const DF17: [u8; 14] =
        [0x8D, 0x40, 0x62, 0x1D, 0x58, 0xC3, 0x82, 0xD6, 0x90, 0xC8, 0xAC, 0x28, 0x63, 0xA7];

    fn decode_at(rate: f64) -> Vec<Candidate> {
        let iq = modulate(&DF17, rate, 1.0, 0.02, 0x9E37_79B9);
        let mut d = Demod::new(rate);
        let mut out = Vec::new();
        // Two passes so the noise tracker has run before the message arrives —
        // exactly what happens on air.
        d.push(&vec![Complex32::new(0.0, 0.0); 4096], &mut out);
        out.clear();
        d.push(&iq, &mut out);
        out
    }

    /// The demodulator is written in microseconds, so the same burst has to
    /// come back at every rate a front end in this tree might deliver.
    #[test]
    fn one_burst_decodes_at_every_rate_a_front_end_delivers() {
        for rate in [2_400_000.0, 2_500_000.0, 3_200_000.0, 4_050_000.0, 8_000_000.0] {
            let out = decode_at(rate);
            assert!(!out.is_empty(), "nothing found at {rate}");
            assert!(
                out.iter().any(|c| c.bytes == DF17),
                "the message came back wrong at {rate}: {:02X?}",
                out[0].bytes
            );
        }
    }

    /// Sweep a burst across one whole sample period, at every rate.
    ///
    /// **The test this decoder was shipped without, and should not have been.**
    /// A transponder has no idea where the receiver's sample instants are, so a
    /// burst arrives at a uniformly random sub-sample phase: this is not an
    /// edge case, it is the ordinary situation forty times over.
    ///
    /// The first cut rounded every chip position to the nearest sample and
    /// recovered 2 of 40 at 2.025 Msps and 21 of 40 at 2.4, on clean
    /// full-scale bursts. Every other test passed, because the generator
    /// rounded the same way — see [`modulate_at`].
    ///
    /// At and above [`sdroxide_types::ADSB_GOOD_RATE_HZ`] the answer has to be
    /// all of them. Below it the waveform is critically sampled and no
    /// implementation recovers every phase (see [`recall_at_a_marginal_rate_is_
    /// limited_by_the_sample_rate`]), which is why that is where the good rate
    /// is drawn.
    #[test]
    fn a_burst_decodes_wherever_it_falls_between_two_samples() {
        for rate in [2_400_000.0f64, 3_200_000.0, 4_050_000.0, 8_000_000.0] {
            const N: usize = 40;
            let mut hits = 0;
            for k in 0..N {
                let t0 = 10.0 + (k as f64 / N as f64) * (1e6 / rate);
                let iq = modulate_at(&DF17, rate, t0, 1.0, 0.02, 0x9E37_79B9 + k as u64);
                let mut d = Demod::new(rate);
                let mut out = Vec::new();
                d.push(&vec![Complex32::new(0.0, 0.0); 8192], &mut out);
                out.clear();
                d.push(&iq, &mut out);
                if out.iter().any(|c| c.bytes == DF17) {
                    hits += 1;
                }
            }
            assert_eq!(hits, N, "only {hits}/{N} phases decoded at {rate:.0} sps");
        }
    }

    /// The same sweep at a *marginal* rate, recording what physics allows.
    ///
    /// A Mode S chip is 0.5 µs, so at 2 Msps the chip and the sample are the
    /// same width and the channel is critically sampled. At the worst arrival
    /// phase each chip is split equally across two samples and reads exactly as
    /// strongly as its neighbour: the bit is a coin toss, and no amount of
    /// arithmetic downstream puts the information back. At 2.025 Msps it is
    /// worse again, because the two clocks beat — the alignment walks through
    /// the degenerate phase every 40 bits, so *every* message has a few bands
    /// of bits decided on the wrong side of a boundary.
    ///
    /// This is why [`sdroxide_types::ADSB_GOOD_RATE_HZ`] is 2.4 Msps and why
    /// the panel says so when a receiver delivers less. The numbers are pinned
    /// low so a regression is still caught; they are not a target to improve.
    #[test]
    fn recall_at_a_marginal_rate_is_limited_by_the_sample_rate() {
        for (rate, floor) in [(2_000_000.0f64, 30usize), (2_025_000.0, 12)] {
            const N: usize = 40;
            let mut hits = 0;
            for k in 0..N {
                let t0 = 10.0 + (k as f64 / N as f64) * (1e6 / rate);
                let iq = modulate_at(&DF17, rate, t0, 1.0, 0.02, 0x9E37_79B9 + k as u64);
                let mut d = Demod::new(rate);
                let mut out = Vec::new();
                d.push(&vec![Complex32::new(0.0, 0.0); 8192], &mut out);
                out.clear();
                d.push(&iq, &mut out);
                if out.iter().any(|c| c.bytes == DF17) {
                    hits += 1;
                }
            }
            assert!(hits >= floor, "only {hits}/{N} phases decoded at {rate:.0} sps");
        }
    }

    /// At a rate that can carry the waveform, recall has to hold up as the
    /// signal comes down — and it must not depend on the arrival phase, or the
    /// decoder works on half the sky and looks like propagation.
    #[test]
    fn a_weak_burst_still_decodes_at_every_phase() {
        const N: usize = 40;
        for rate in [2_400_000.0f64, 4_050_000.0] {
            for (amp, floor) in [(0.15f32, N), (0.08, N - 1)] {
                let mut hits = 0;
                for k in 0..N {
                    let t0 = 10.0 + (k as f64 / N as f64) * (1e6 / rate);
                    let iq = modulate_at(&DF17, rate, t0, amp, 0.01, 0x1234_5678 + k as u64);
                    let mut d = Demod::new(rate);
                    let mut out = Vec::new();
                    // Primed on noise, not on silence: a floor eased down from
                    // digital zero would make this test measure the tracker.
                    let mut st = 99u64;
                    let mut rnd = || {
                        st ^= st << 13;
                        st ^= st >> 7;
                        st ^= st << 17;
                        ((st >> 40) as f32 / 8_388_608.0) - 1.0
                    };
                    let warm: Vec<Complex32> =
                        (0..16384).map(|_| Complex32::new(0.01 * rnd(), 0.01 * rnd())).collect();
                    d.push(&warm, &mut out);
                    out.clear();
                    d.push(&iq, &mut out);
                    if out.iter().any(|c| c.bytes == DF17) {
                        hits += 1;
                    }
                }
                assert!(hits >= floor, "only {hits}/{N} at amplitude {amp} and {rate:.0} sps");
            }
        }
    }

    /// A message split across two blocks must still decode. Without the carried
    /// tail this silently costs a few percent of every aircraft's frames, which
    /// looks like a deaf receiver rather than like a bug.
    #[test]
    fn a_burst_straddling_a_block_boundary_still_decodes() {
        let rate = 2_400_000.0;
        let iq = modulate(&DF17, rate, 1.0, 0.02, 0x1234_5678);
        // Cut in the middle of the data block.
        let cut = iq.len() / 2;
        let mut d = Demod::new(rate);
        let mut out = Vec::new();
        d.push(&vec![Complex32::new(0.0, 0.0); 4096], &mut out);
        out.clear();
        d.push(&iq[..cut], &mut out);
        d.push(&iq[cut..], &mut out);
        assert!(
            out.iter().any(|c| c.bytes == DF17),
            "the split message was lost: {} candidates",
            out.len()
        );
    }

    /// Noise alone must not produce messages. The CRC is the real gate, but a
    /// correlator that fires on every other sample would hand it millions of
    /// candidates a second and cost more than the whole receive chain.
    ///
    /// **Gaussian** noise, because that is what a receiver produces. This test
    /// used uniform noise and passed while the decoder raised ~98 000 candidates
    /// a second on a real empty sky and pinned its worker at 100 % of a core:
    /// uniform I and Q give an envelope power with no tail, so it never crossed a
    /// floor the Gaussian tail crosses 40 % of the time. See [`Q1_OF_MEAN`].
    #[test]
    fn noise_alone_yields_almost_no_candidates() {
        for rate in [2_400_000.0f64, 4_000_000.0, 8_000_000.0] {
            let mut g = Gauss(0xDEAD_BEEF);
            // A quarter of a second to settle the tracker, then one second.
            let warm: Vec<Complex32> = (0..rate as usize / 4).map(|_| g.next(1e-4)).collect();
            let iq: Vec<Complex32> = (0..rate as usize).map(|_| g.next(1e-4)).collect();
            let mut d = Demod::new(rate);
            let mut out = Vec::new();
            for chunk in warm.chunks(16_384) {
                d.push(chunk, &mut out);
            }
            out.clear();
            for chunk in iq.chunks(16_384) {
                d.push(chunk, &mut out);
            }
            // The check sequence is what decides whether a candidate is a
            // message, so a few is harmless; what is not harmless is the work
            // each one costs. Measured at under a hundred a second at every rate
            // here, against ~98 000 at 2.4 Msps before the floor was a mean.
            assert!(
                out.len() < 2_000,
                "the correlator fired {} times on a second of Gaussian noise at {rate:.0} sps",
                out.len()
            );
        }
    }

    /// The other half of the noise test: the floor that stopped the correlator
    /// firing on noise must not have cost the weak aircraft.
    ///
    /// A DF17 at every arrival phase in **Gaussian** noise, near where the
    /// decoder runs out. On these seeds the old gate and this one score the same
    /// at 2.4 Msps / 15 dB (35/40) and 4 Msps / 10 dB (37/40); at 8 Msps / 8 dB
    /// the old one had 40 and this 39. Each floor is that, less one.
    #[test]
    fn the_noise_floor_costs_no_weak_aircraft() {
        let np = 1e-4f64;
        for (rate, snr_db, floor) in
            [(2_400_000.0f64, 15.0f64, 34usize), (4_000_000.0, 10.0, 36), (8_000_000.0, 8.0, 38)]
        {
            let amp = (np * 10f64.powf(snr_db / 10.0)).sqrt() as f32;
            const N: usize = 40;
            let mut hits = 0;
            for k in 0..N {
                let t0 = 10.0 + (k as f64 / N as f64) * (1e6 / rate);
                let clean = modulate_at(&DF17, rate, t0, amp, 0.0, 1);
                let mut g = Gauss(0x5555_0000 + k as u64);
                let warm: Vec<Complex32> = (0..32_768).map(|_| g.next(np as f32)).collect();
                let iq: Vec<Complex32> = clean.iter().map(|z| z + g.next(np as f32)).collect();
                let mut d = Demod::new(rate);
                let mut out = Vec::new();
                d.push(&warm, &mut out);
                out.clear();
                d.push(&iq, &mut out);
                if out.iter().any(|c| c.bytes == DF17) {
                    hits += 1;
                }
            }
            assert!(hits >= floor, "only {hits}/{N} at {snr_db} dB and {rate:.0} sps");
        }
    }

    /// Complex Gaussian noise of a given mean power, deterministically: there
    /// is no `rand` in this tree.
    struct Gauss(u64);

    impl Gauss {
        fn uniform(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }

        fn next(&mut self, power: f32) -> Complex32 {
            let r = (-f64::from(power) * self.uniform().ln()).sqrt();
            let th = std::f64::consts::TAU * self.uniform();
            Complex32::new((r * th.cos()) as f32, (r * th.sin()) as f32)
        }
    }

    /// A short reply is 56 bits, and the length comes from the first five —
    /// reading a DF4 as 112 bits would run the CRC over eight bytes of the next
    /// aircraft's silence.
    #[test]
    fn the_downlink_format_chooses_the_length() {
        let rate = 2_400_000.0;
        // DF4 (00100...) surveillance altitude reply, seven bytes.
        let short = [0x20u8, 0x00, 0x11, 0x91, 0xAB, 0xCD, 0xEF];
        let iq = modulate(&short, rate, 1.0, 0.02, 0xABCD_0123);
        let mut d = Demod::new(rate);
        let mut out = Vec::new();
        d.push(&vec![Complex32::new(0.0, 0.0); 4096], &mut out);
        out.clear();
        d.push(&iq, &mut out);
        assert!(
            out.iter().any(|c| c.bytes == short),
            "a short reply did not come back as seven bytes: {:02X?}",
            out.first().map(|c| &c.bytes)
        );
    }
}
