//! CW (Morse): a receive decoder that tracks one tone and reads its keying, the
//! envelope-domain timing engine behind it, and a keyed-sidetone transmitter.
//!
//! The decoder is deliberately *not* the usual "measure each mark as it ends,
//! call it a dit or a dah against a running average, and adapt" design. That
//! shape has to commit to a decision threshold and to a speed before it has seen
//! the evidence, and on a weak or fading signal both guesses are wrong in the
//! same direction: a threshold set too high clips the marks short, which drags
//! the speed estimate up, which reclassifies the next dah as a dit. It is a
//! feedback loop with no way back, and it is why such decoders produce long runs
//! of `E`/`T`/`I` on exactly the signals you most want copied.
//!
//! [`CwDecoder`] instead keeps a few seconds of the keying envelope and, on each
//! hop, *searches* for the (threshold, dit length, edge bias) that make the run
//! lengths best fit Morse's own timing model — 1 and 3 units keyed, 1, 3 and 7
//! units of silence. The residual of that fit is a real confidence number: noise
//! has no timing model to fit, so its best cost stays high and nothing is
//! emitted. Only then are characters read off, and only the ones that have not
//! been emitted before. Nothing is decided before the evidence is in, and one
//! wrong element cannot poison what follows.
//!
//! The idea of scoring a candidate (speed, threshold) pair by how well the
//! resulting intervals fit Morse timing is Georgi Gerganov's, from ggmorse. What
//! is done differently here: the intervals are built once per threshold rather
//! than once per (speed, threshold) pair, the threshold rides a locally
//! estimated floor/peak pair so QSB cannot walk the signal out from under it,
//! the shaped-keying edge bias is a fitted parameter instead of an after-the-fact
//! rescale, character and word gaps are scored one-sided so Farnsworth spacing
//! is not read as a speed error, and emission is keyed to an absolute sample
//! index so a character is emitted exactly once.
//!
//! [`CwDecoder`] works on any envelope stream at any rate, so the wideband
//! skimmer's per-bin STFT envelopes go through the same engine as the panel's
//! narrowband audio path.

use std::collections::VecDeque;
use std::f32::consts::TAU;

use crate::Complex32;

// ─── Morse alphabet ──────────────────────────────────────────────────────────

/// International Morse (ITU-R M.1677-1) plus the punctuation and prosigns that
/// turn up in ordinary QSOs. `.` is a dit, `-` a dah.
const TABLE: &[(char, &str)] = &[
    ('A', ".-"),
    ('B', "-..."),
    ('C', "-.-."),
    ('D', "-.."),
    ('E', "."),
    ('F', "..-."),
    ('G', "--."),
    ('H', "...."),
    ('I', ".."),
    ('J', ".---"),
    ('K', "-.-"),
    ('L', ".-.."),
    ('M', "--"),
    ('N', "-."),
    ('O', "---"),
    ('P', ".--."),
    ('Q', "--.-"),
    ('R', ".-."),
    ('S', "..."),
    ('T', "-"),
    ('U', "..-"),
    ('V', "...-"),
    ('W', ".--"),
    ('X', "-..-"),
    ('Y', "-.--"),
    ('Z', "--.."),
    ('0', "-----"),
    ('1', ".----"),
    ('2', "..---"),
    ('3', "...--"),
    ('4', "....-"),
    ('5', "....."),
    ('6', "-...."),
    ('7', "--..."),
    ('8', "---.."),
    ('9', "----."),
    ('.', ".-.-.-"),
    (',', "--..--"),
    ('?', "..--.."),
    ('/', "-..-."),
    ('=', "-...-"),
    ('+', ".-.-."),
    ('-', "-....-"),
    (':', "---..."),
    ('(', "-.--."),
    (')', "-.--.-"),
    ('"', ".-..-."),
    ('\'', ".----."),
    ('@', ".--.-."),
    // The accented letters ITU-R M.1677-1 lists beside the plain alphabet.
    // They are ordinary Morse, not an extension: a Nordic or German operator
    // sends them without comment, and a receiver that has never been told
    // about them copies each one as a `?` (issue #382).
    //
    // One character per code, because the table has to invert: several of
    // these are shared between languages — `.-.-` is A-umlaut and also
    // A-E-ligature, `---.` is O-umlaut and also O-slash — and which of them
    // was meant is not in the signal. [`fold`] is what lets the *transmit*
    // side accept the others and send the code they share.
    //
    // `----` is deliberately absent. M.1677 lists it as the digraph CH (and
    // Slovene/Croatian S-caron), and it is not a character this table can
    // hold — but more to the point, four dahs is exactly what a run of
    // mis-timed T's looks like, and turning that into a letter would be
    // turning noise into words.
    ('Ä', ".-.-"),
    ('Å', ".--.-"),
    ('Ç', "-.-.."),
    ('È', ".-..-"),
    ('É', "..-.."),
    ('Ñ', "--.--"),
    ('Ö', "---."),
    ('Ü', "..--"),
];

/// Decode a `.-` element string to a character, or `None` if it is not Morse.
pub fn morse_decode(code: &str) -> Option<char> {
    TABLE.iter().find(|(_, c)| *c == code).map(|(ch, _)| *ch)
}

/// Encode a character to its `.-` element string. Case-insensitive; `None` for
/// anything with no Morse representation.
pub fn morse_encode(ch: char) -> Option<&'static str> {
    let up = fold(ch);
    TABLE.iter().find(|(c, _)| *c == up).map(|(_, code)| *code)
}

/// The character [`TABLE`] actually carries for `ch`: upper case, with the
/// national letters that *share* one Morse code folded onto the one entry that
/// holds it.
///
/// Two things are going on here, and the second is the reason this is not
/// `to_ascii_uppercase`. The ASCII version leaves every accented letter alone,
/// so a lower-case `ä` never matched the table at all. And Morse gives one code
/// to letters several languages spell differently — `.-.-` is Ä to a German and
/// Æ to a Dane, `---.` is Ö and Ø, `.--.-` is Å and À — so the sender's own
/// spelling has to reach the one entry the table holds for that code.
///
/// A character whose upper case is more than one letter — German ß, which
/// upper-cases to "SS" — is left alone rather than truncated to its first
/// letter: sending S for ß would put a *different word* on the air, and no code
/// at all is the answer the keyer already had for it.
fn fold(ch: char) -> char {
    let mut up = ch.to_uppercase();
    let (Some(c), None) = (up.next(), up.next()) else { return ch };
    match c {
        // ITU-R M.1677-1 writes the first three of these as one row each —
        // "à å", "ä", "é" — and the national alphabets that use the same code
        // for their own letter follow from that: Danish and Norwegian Æ and Ø,
        // Polish Ą, Ć, Ł, Ń and Ę, Esperanto Ĉ and Ŭ.
        'Æ' | 'Ą' => 'Ä',
        'Ø' => 'Ö',
        'À' => 'Å',
        'Ć' | 'Ĉ' => 'Ç',
        'Ł' => 'È',
        'Ę' | 'Đ' => 'É',
        'Ń' => 'Ñ',
        'Ŭ' => 'Ü',
        c => c,
    }
}

// ─── the envelope-domain timing engine ───────────────────────────────────────

/// Seconds of envelope held for the fit. Long enough that a slow sender still
/// puts several characters in it (at 10 WPM a word is about 4 s), short enough
/// that a speed change is followed rather than averaged away.
const WINDOW_S: f32 = 6.0;
/// Re-fit and emit this often. The decode latency is roughly this plus the run
/// that terminates the character.
const HOP_S: f32 = 0.1;
/// Noise-floor estimation block. Short enough to follow a changing band, long
/// enough that a block of ordinary keying still contains silence to measure.
const BLOCK_S: f32 = 0.3;
/// Decision thresholds tried, in dB above the estimated noise level.
///
/// Referring them to the *noise* rather than to the signal is what makes QSB
/// survivable. The receiver's noise floor does not fade — only the signal on
/// top of it does — so one threshold a few dB above the floor holds good from
/// the peak of a fade to the trough, where a threshold set as a fraction of the
/// mark level would climb as the signal sank and go deaf exactly when it was
/// needed. The fit picks which of these to use.
const THRESHOLDS_DB: &[f32] = &[1.0, 2.0, 3.0, 4.5, 6.0, 8.0, 10.0, 13.0, 16.0, 20.0, 25.0, 31.0];
/// A Rayleigh envelope's 20th percentile sits this far below its mean (0.668σ
/// against 1.253σ). Scaling the measured percentile by it turns a robust order
/// statistic — one that a 50 % keying duty cycle cannot pull upwards — into the
/// noise level the thresholds above are quoted against.
const P20_TO_MEAN: f32 = 1.876;
/// How far above the whole-window noise estimate a single block's may sit
/// (a ratio, ~4 dB). Bounds how much a stretch of dense keying can lift the
/// floor under itself.
const FLOOR_HEADROOM: f32 = 1.6;
/// Speed search bounds (WPM). Below 6 the window holds too little to fit; above
/// 70 the envelope rate stops resolving a dit.
const WPM_MIN: f32 = 6.0;
const WPM_MAX: f32 = 70.0;
/// Coarse speed grid ratio, then the fine one used around the coarse winner.
const WPM_COARSE_STEP: f32 = 1.05;
const WPM_FINE_STEP: f32 = 1.008;
/// Fraction of a transmission's intervals that may fail to fit the timing model
/// before it stops being read as Morse. This is the decoder's squelch, and it
/// is what keeps noise off the panel.
const MAX_COST: f32 = 0.35;
/// Consecutive hops that must agree before anything is emitted. A run of noise
/// can fit *a* timing model, but not the same one twice running; a real signal
/// gives the same speed hop after hop. Costs one hop of latency.
const LOCK_HOPS: u32 = 2;
/// How far the fitted speed may move between hops and still count as the same
/// signal for [`LOCK_HOPS`].
const LOCK_WPM_TOL: f32 = 0.12;
/// Width, in units, of the window a run's timing counts as "fitting" within.
///
/// The fit is scored by how many intervals land inside it, softly — one minus
/// the mean of a Gaussian bump on each residual — rather than by the mean
/// squared residual. The difference matters entirely for rejecting noise. A
/// squared-error score can always be improved by finding an interpretation
/// under which a few intervals are outliers and then discounting them, and with
/// a threshold, a speed, a spacing unit and an edge bias to choose from, noise
/// finds one: measured, its best mean-squared fit came out *better* than a real
/// signal's at 0 dB. Here an interval that does not fit contributes its full
/// weight however badly it misses, so the score is the fraction of the
/// transmission the model actually explains, which noise cannot fake.
const FIT_SIGMA: f32 = 0.40;
/// How far below the mark level a decision threshold has to stay. Closer than
/// this and it is measuring the shape of the keying envelope's top, not the
/// keying.
const THRESHOLD_HEADROOM_DB: f32 = 6.0;
/// Highest threshold always worth trying, whatever the measured level.
const THRESHOLD_FLOOR_DB: f32 = 10.0;
/// Shortest run, as a fraction of the fitted dit, that is still an element
/// rather than the envelope crossing the threshold on its way past.
const DEGLITCH: f32 = 0.35;
/// Least signal-to-noise, in the detector's own bandwidth, that is read as a
/// signal at all.
///
/// A Rayleigh envelope's ninetieth percentile sits only 4.7 dB above the noise
/// level estimated from its twentieth, so pure noise cannot reach this however
/// well its run lengths happen to fit; a signal weak enough to be worth calling
/// marginal is already well past it.
const MIN_SNR_DB: f32 = 8.0;
/// Spacing-unit candidates, as a multiple of the element unit. 1.0 is ordinary
/// timing; the tail covers Farnsworth down to roughly a third of the character
/// speed, which is as slow as the practice sending goes.
const SPACE_RATIOS: &[f32] =
    &[1.0, 1.15, 1.35, 1.6, 1.9, 2.2, 2.6, 3.0, 3.5, 4.1, 4.8, 5.6, 6.5, 8.0];
/// Never emit more than this much backlog at once. A decoder that has just
/// found the speed after a spell of noise has a whole window of unemitted
/// characters behind it; dumping all six seconds reads as a glitch.
const CATCHUP_S: f32 = 3.0;
/// Longest element string still worth looking up. Anything longer is a burst of
/// noise inside one character, not a character.
const MAX_ELEMENTS: usize = 8;

/// One thresholded run of the envelope: `[start, end)` in window samples.
#[derive(Clone, Copy)]
struct Run {
    on: bool,
    start: u32,
    end: u32,
}

impl Run {
    fn len(&self) -> f32 {
        (self.end - self.start) as f32
    }
}

/// The timing model fitted to one run list.
#[derive(Clone, Copy, Debug)]
struct Fit {
    /// Dit length, in envelope samples.
    unit: f32,
    /// Spacing unit, in envelope samples: what a character gap divides by three
    /// and a word gap by seven.
    ///
    /// The same as `unit` for ordinary sending, and longer under Farnsworth,
    /// where the characters are sent at full speed and only the spacing is
    /// stretched. Fitting it separately is what keeps an 18-WPM-at-8-WPM signal
    /// from being read as 8 WPM with every dah turned into a dit.
    space: f32,
    /// Edge bias, in envelope samples. Thresholding a shaped keying envelope
    /// below its half-amplitude point lengthens *every* mark and shortens every
    /// gap by the same amount, so the two are one parameter rather than noise —
    /// fitting it is what lets a low threshold (good for weak signals) be used
    /// without the mark/gap classification drifting.
    bias: f32,
    /// Fraction of intervals the model does not explain. The fit quality, and
    /// the decoder's confidence.
    cost: f32,
    /// Character- and word-spacing gaps the spacing unit was fitted from.
    longs: u32,
}

/// A character read off the fitted run list.
///
/// Emission is keyed to window sample indices, so re-fitting the same audio
/// cannot emit the same character twice. `commit` is deliberately *not* the end
/// of the character: it sits a fixed unit into the silence that terminated it,
/// far from any run boundary, because a boundary is exactly what moves by a
/// sample or two when the next hop picks a slightly different threshold — and a
/// commit point that lands on one drops every other character.
struct Decoded {
    ch: char,
    start: u32,
    commit: u32,
}

/// Streaming Morse decoder over a keying-envelope signal.
///
/// Feed it an amplitude-like envelope (magnitude, not power) at a fixed rate
/// with [`CwDecoder::push`]; it returns newly decoded text. The envelope may be
/// anything that rises with the keying — a narrowband audio detector's output,
/// or one STFT bin of a wideband skimmer.
pub struct CwDecoder {
    rate: f32,
    /// Rolling envelope window.
    buf: Vec<f32>,
    /// Absolute sample index of `buf[0]`, so emission survives the window
    /// sliding underneath it.
    base: u64,
    /// Absolute index up to which characters have already been emitted.
    emitted_end: u64,
    since_hop: usize,
    hop: usize,

    /// Envelope in dB above the local noise level (`normalize`).
    norm: Vec<f32>,
    /// Per-block (noise, mark) percentiles, and the noise estimate widened over
    /// neighbouring blocks.
    blocks: Vec<(f32, f32)>,
    spread: Vec<f32>,
    /// Scratch for the per-block percentiles.
    pct: Vec<f32>,
    /// Runs before the de-glitch pass.
    raw: Vec<Run>,
    runs: Vec<Run>,
    best_runs: Vec<Run>,
    chars: Vec<Decoded>,

    /// Speed of the most recent fit, whether or not it was good enough to
    /// believe. Only the hop-to-hop agreement check uses this.
    fit_wpm: f32,
    /// Speed of the most recent *locked* fit — what the caller is told.
    /// Between transmissions the decoder goes on fitting whatever the noise
    /// offers, and a readout that followed it would be a random number
    /// generator with a WPM label on it.
    wpm: f32,
    cost: f32,
    snr_db: f32,
    /// Consecutive hops whose fit agreed with the one before it.
    good_hops: u32,
    /// Spacing unit as a multiple of the element unit, smoothed across hops.
    ///
    /// It comes off a coarse scan, so hop to hop it steps between grid points;
    /// used raw, the boundary it sets between "between characters" and "between
    /// words" moves with it and sprays word breaks through the middle of a
    /// callsign. What it is measuring — whether this operator sends Farnsworth
    /// spacing — does not change during an over, so it is worth averaging.
    /// `None` until a window with enough spacing in it has been seen: starting
    /// from an assumed 1.0 and averaging towards the truth would put word
    /// breaks through the first second of every Farnsworth transmission.
    space_ratio: Option<f32>,
    /// Operator-locked speed, when the panel's speed control is not on auto.
    lock_wpm: Option<f32>,
}

impl CwDecoder {
    /// `rate` is the envelope sample rate in Hz.
    pub fn new(rate: f32) -> Self {
        let cap = (WINDOW_S * rate) as usize + 1;
        CwDecoder {
            rate,
            buf: Vec::with_capacity(cap),
            base: 0,
            emitted_end: 0,
            since_hop: 0,
            hop: (HOP_S * rate) as usize,
            norm: Vec::with_capacity(cap),
            blocks: Vec::new(),
            spread: Vec::new(),
            pct: Vec::new(),
            raw: Vec::new(),
            runs: Vec::new(),
            best_runs: Vec::new(),
            chars: Vec::new(),
            fit_wpm: 20.0,
            wpm: 20.0,
            cost: f32::INFINITY,
            snr_db: -30.0,
            good_hops: 0,
            space_ratio: None,
            lock_wpm: None,
        }
    }

    /// How often to re-fit and emit, in seconds. The default suits one signal
    /// being read by an operator; a skimmer running this over hundreds of
    /// simultaneous signals trades latency for the CPU back.
    pub fn set_hop_s(&mut self, seconds: f32) {
        self.hop = (seconds.clamp(HOP_S, WINDOW_S / 2.0) * self.rate) as usize;
    }

    /// Pin the speed search to `wpm` (±10%), or `None` to search the full range.
    pub fn set_speed_lock(&mut self, wpm: Option<f32>) {
        self.lock_wpm = wpm.map(|w| w.clamp(WPM_MIN, WPM_MAX));
    }

    /// Forget everything: the window, the emitted-index bookkeeping and the
    /// fitted speed. For a retune, where the old signal's timing says nothing
    /// about the new one's.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.base = 0;
        self.emitted_end = 0;
        self.since_hop = 0;
        self.cost = f32::INFINITY;
        self.snr_db = -30.0;
        self.good_hops = 0;
        self.space_ratio = None;
    }

    /// Speed of the last good fit, in words per minute.
    pub fn wpm(&self) -> f32 {
        self.wpm
    }

    /// Mean squared timing residual of the last fit, in dits². Below
    /// [`MAX_COST`] the decoder is copying; well above it there is no Morse
    /// there.
    pub fn cost(&self) -> f32 {
        self.cost
    }

    /// True while the decoder is copying: the last fits were good enough and
    /// agreed with each other.
    pub fn locked(&self) -> bool {
        self.good_hops >= LOCK_HOPS
    }

    /// Signal-to-noise of the envelope, in dB: the fitted mark level over the
    /// noise floor, in whatever bandwidth the caller's detector has. The
    /// caller scales it to a reference bandwidth if it wants a comparable
    /// figure (see [`CwRx::snr_db`]).
    pub fn raw_snr_db(&self) -> f32 {
        self.snr_db
    }

    /// Feed envelope samples; returns whatever new text they completed.
    pub fn push(&mut self, env: &[f32]) -> String {
        self.buf.extend_from_slice(env);
        self.since_hop += env.len();
        let cap = (WINDOW_S * self.rate) as usize;
        if self.buf.len() > cap {
            let drop = self.buf.len() - cap;
            self.buf.drain(..drop);
            self.base += drop as u64;
        }
        if self.since_hop < self.hop {
            return String::new();
        }
        self.since_hop = 0;
        self.analyze()
    }

    fn analyze(&mut self) -> String {
        // Enough blocks that the noise estimate is worth having, and enough
        // signal that a character is not read off half of itself.
        if self.buf.len() < (4.0 * BLOCK_S * self.rate) as usize {
            return String::new();
        }
        self.normalize();
        if self.snr_db < MIN_SNR_DB {
            self.cost = f32::INFINITY;
            self.good_hops = 0;
            return String::new();
        }

        // The run list depends only on the threshold, and the timing fit only
        // on the run list — so build the runs once per threshold and score the
        // whole speed grid against each, rather than re-encoding the envelope
        // for every (speed, threshold) pair.
        let (wpm_lo, wpm_hi) = match self.lock_wpm {
            Some(w) => (w * 0.9, w * 1.1),
            None => (WPM_MIN, WPM_MAX),
        };
        // The speed does not change between hops, so last hop's unit sets how
        // short a run has to be to be a glitch. Before there is one, nothing is
        // absorbed.
        let min_run =
            if self.good_hops > 0 { (DEGLITCH * 1.2 * self.rate / self.wpm) as u32 } else { 0 };
        // A threshold within a few dB of the mark level does not measure the
        // keying, it clips it: the shaped top of a dit spends less time above
        // such a threshold than a dah's does, in a ratio that has nothing to do
        // with 1 and 3. Not trying them at all is cheaper than scoring them and
        // safer than hoping they score badly.
        // …but never rule out the low end: on a marginal signal the measured
        // level understates the marks, and a decoder that declines to try any
        // threshold at all copies nothing.
        let t_max = (self.snr_db - THRESHOLD_HEADROOM_DB).max(THRESHOLD_FLOOR_DB);
        let mut best: Option<Fit> = None;
        for &t in THRESHOLDS_DB.iter().take_while(|&&t| t <= t_max) {
            self.encode_runs(t, min_run);
            // Two truncated edge runs plus something to fit between them.
            if self.runs.len() < 6 {
                continue;
            }
            let Some(fit) = fit_runs(&self.runs[1..self.runs.len() - 1], self.rate, wpm_lo, wpm_hi)
            else {
                continue;
            };
            if best.is_none_or(|b| fit.cost < b.cost) {
                best = Some(fit);
                self.best_runs.clear();
                self.best_runs.extend_from_slice(&self.runs);
            }
        }

        let Some(fit) = best else {
            self.cost = f32::INFINITY;
            self.good_hops = 0;
            return String::new();
        };
        self.cost = fit.cost;
        let wpm = 1.2 * self.rate / fit.unit;
        // The first hop of a new signal has nothing to agree with, so it stands
        // as the reference for the next one rather than being thrown away.
        let agrees =
            self.good_hops == 0 || (wpm - self.fit_wpm).abs() < LOCK_WPM_TOL * self.fit_wpm;
        self.fit_wpm = wpm;
        if fit.cost > MAX_COST || !agrees {
            self.good_hops = 0;
            return String::new();
        }
        self.good_hops += 1;
        if self.good_hops < LOCK_HOPS {
            return String::new();
        }
        self.wpm = wpm;

        // Only from a window with enough spacing in it to have measured
        // anything: one or two gaps will fit whatever ratio the scan tries
        // first, and a single such hop dragging the average down is enough to
        // put a word break inside the next callsign.
        if fit.longs >= 2 {
            let ratio = fit.space / fit.unit;
            self.space_ratio = Some(self.space_ratio.map_or(ratio, |p| 0.85 * p + 0.15 * ratio));
        }
        self.decode_runs(&fit);
        self.emit()
    }

    /// Restate the envelope as dB above the locally estimated noise level.
    ///
    /// Everything downstream — the thresholds, the SNR readout — is then
    /// referred to the noise rather than to the signal, which is what makes a
    /// single threshold hold good across a fade.
    fn normalize(&mut self) {
        let n = self.buf.len();
        let bn = ((BLOCK_S * self.rate) as usize).max(8);
        // Whole blocks only, the last one swallowing the remainder. A short
        // tail block is the worst case there is for an order statistic: at the
        // window edge it can be thirty samples of one dah, whose 20th
        // percentile is the mark level rather than the noise.
        let nb = (n / bn).max(1);
        self.blocks.clear();
        for b in 0..nb {
            let s = b * bn;
            let e = if b + 1 == nb { n } else { (b + 1) * bn };
            self.pct.clear();
            self.pct.extend_from_slice(&self.buf[s..e]);
            let lo = percentile(&mut self.pct, 0.20);
            let hi = percentile(&mut self.pct, 0.90);
            self.blocks.push((lo, hi));
        }

        // A receiver's noise floor does not move much over six seconds, and
        // what does move it (AGC riding the keying) moves it *with* the signal,
        // which is the one thing an estimate that followed it closely would
        // cancel out. So take a whole-window figure and let the per-block one
        // only go below it: a run of dense keying can then no longer drag the
        // floor up over its own marks, which is what makes a threshold set
        // above it go deaf for a hop and split a character in two.
        self.pct.clear();
        self.pct.extend(self.blocks.iter().map(|b| b.0));
        let global = percentile(&mut self.pct, 0.25);
        // Never claim more dynamic range than a receiver has. Fed a synthetic
        // signal with no noise on it at all the floor estimate goes to zero,
        // every ring and every shaping tail is then hundreds of dB "above the
        // noise", and the thresholds lose all meaning.
        let ceil = self.blocks.iter().fold(0.0f32, |a, w| a.max(w.1));
        let min_floor = ceil * 1e-4;
        self.spread.clear();
        for b in 0..nb {
            let s = b.saturating_sub(2);
            let e = (b + 3).min(nb);
            let lo = self.blocks[s..e].iter().fold(f32::INFINITY, |a, w| a.min(w.0));
            let lo = lo.min(global * FLOOR_HEADROOM).max(min_floor);
            self.spread.push((lo * P20_TO_MEAN).max(1e-12));
        }

        // Reported against the whole window, not per block: the number the
        // panel shows should be the signal's, and a strong signal's SNR does
        // not drop because it happens to be between characters.
        self.pct.clear();
        self.pct.extend(self.blocks.iter().map(|b| b.1));
        let g_hi = percentile(&mut self.pct, 0.9);
        self.pct.clear();
        self.pct.extend(self.spread.iter().copied());
        let g_lo = percentile(&mut self.pct, 0.5);
        self.snr_db = 20.0 * (g_hi / g_lo).max(1.0).log10();

        // Interpolate the noise level between block centres so a band that
        // changes level mid-block does not step the normalised envelope.
        self.norm.clear();
        self.norm.resize(n, 0.0);
        let last = nb - 1;
        for i in 0..n {
            let x = i as f32 / bn as f32 - 0.5;
            let b0 = x.floor().max(0.0).min(last as f32) as usize;
            let b1 = (b0 + 1).min(last);
            let t = (x - b0 as f32).clamp(0.0, 1.0);
            let f = self.spread[b0] + t * (self.spread[b1] - self.spread[b0]);
            self.norm[i] = (20.0 * (self.buf[i].max(1e-12) / f).log10()).clamp(-30.0, 80.0);
        }
    }

    /// Run-length encode `norm > t` into [`Run`]s, absorbing any run shorter
    /// than `min_run` samples into the one before it.
    ///
    /// At the edge of copy the envelope of a mark dips through the threshold
    /// several times — three independent noise samples inside one dit is enough
    /// — and each dip costs two spurious intervals that fit nothing. Measured on
    /// a signal at +3 dB in 500 Hz, a six-second window held seventy-five
    /// intervals where clean keying holds thirty-three, and the fit was scored
    /// on the wreckage. Nothing that brief is an element at any speed the
    /// decoder searches, so it is not one.
    fn encode_runs(&mut self, t: f32, min_run: u32) {
        self.raw.clear();
        let n = self.norm.len();
        let mut on = self.norm[0] > t;
        let mut start = 0u32;
        for i in 1..n {
            let cur = self.norm[i] > t;
            if cur != on {
                self.raw.push(Run { on, start, end: i as u32 });
                on = cur;
                start = i as u32;
            }
        }
        self.raw.push(Run { on, start, end: n as u32 });

        self.runs.clear();
        for &r in &self.raw {
            match self.runs.last_mut() {
                Some(last) if last.on == r.on || r.end - r.start < min_run => last.end = r.end,
                _ => self.runs.push(r),
            }
        }
    }

    /// Read characters off the fitted run list.
    ///
    /// The first run is truncated by the window edge, so its length means
    /// nothing and it is skipped. The *last* is not skipped when it is silence:
    /// a gap that has already run three units is a finished character however
    /// much longer it turns out to be, and waiting for it to close would lose
    /// the last character of every over.
    fn decode_runs(&mut self, fit: &Fit) {
        self.chars.clear();
        let runs = &self.best_runs;
        if runs.len() < 3 {
            return;
        }
        let (u, v) = (fit.unit, fit.unit * self.space_ratio.unwrap_or(1.0));
        let mut sym = String::new();
        let mut start = 0u32;
        for (i, r) in runs.iter().enumerate().skip(1) {
            let last = i + 1 == runs.len();
            if r.on {
                if last {
                    break; // a mark still in progress is not yet an element
                }
                if sym.is_empty() {
                    start = r.start;
                }
                sym.push(if r.len() - fit.bias > 2.0 * u { '-' } else { '.' });
                if sym.chars().count() > MAX_ELEMENTS {
                    sym.clear();
                }
                continue;
            }
            // Gaps carry the bias the other way: the threshold that lengthened
            // the mark shortened the silence beside it by the same amount.
            let g = r.len() + fit.bias;
            if g < 2.0 * u {
                continue; // between elements of one character
            }
            if !sym.is_empty() {
                let ch = morse_decode(&sym).unwrap_or('?');
                sym.clear();
                self.chars.push(Decoded { ch, start, commit: r.start + u as u32 });
            }
            if g > 5.0 * v {
                // Between words. Placed at fixed offsets into the gap rather
                // than at its edges: the offsets are the same whether this gap
                // is five units long or five seconds, and whether it has closed
                // yet, so a later hop places it identically and it is emitted
                // exactly once.
                //
                // In element units, deliberately, even though what decided this
                // *was* a word gap is the spacing unit. The spacing unit comes
                // off a coarse scan and can step 15 % between hops; a marker
                // placed at a multiple of it would then move far enough to be
                // emitted twice — and to carry the emitted-index past the start
                // of the next character, swallowing it. The element unit moves
                // by a fraction of a percent. A word gap is at least five
                // element units wide, so three of them always lands inside it.
                self.chars.push(Decoded {
                    ch: ' ',
                    start: r.start + (2.0 * u) as u32,
                    commit: r.start + (3.0 * u) as u32,
                });
            }
        }
    }

    /// Emit the decoded characters this window has not already produced.
    fn emit(&mut self) -> String {
        let n = self.buf.len() as u64;
        let catchup = (CATCHUP_S * self.rate) as u64;
        self.emitted_end = self.emitted_end.max((self.base + n).saturating_sub(catchup));
        let mut out = String::new();
        for c in &self.chars {
            let a = self.base + c.start as u64;
            let b = self.base + c.commit as u64;
            // `start >= emitted_end` as well as `commit > emitted_end`: a re-fit
            // that merged two already-emitted characters into one would
            // otherwise emit the merged result a second time.
            if a >= self.emitted_end && b > self.emitted_end {
                out.push(c.ch);
                self.emitted_end = b;
            }
        }
        out
    }
}

/// Search the speed grid for the dit length that makes `runs` fit Morse timing
/// best. Coarse geometric pass, then a fine one around its winner.
fn fit_runs(runs: &[Run], rate: f32, wpm_lo: f32, wpm_hi: f32) -> Option<Fit> {
    let mut best: Option<Fit> = None;
    let mut wpm = wpm_lo;
    while wpm <= wpm_hi {
        if let Some(f) = eval_fit(runs, 1.2 * rate / wpm)
            && best.is_none_or(|b| f.cost < b.cost)
        {
            best = Some(f);
        }
        wpm *= WPM_COARSE_STEP;
    }
    let coarse = best?;
    let centre = 1.2 * rate / coarse.unit;
    let mut wpm = (centre / WPM_COARSE_STEP).max(wpm_lo);
    let hi = (centre * WPM_COARSE_STEP).min(wpm_hi);
    while wpm <= hi {
        if let Some(f) = eval_fit(runs, 1.2 * rate / wpm)
            && best.is_none_or(|b| f.cost < b.cost)
        {
            best = Some(f);
        }
        wpm *= WPM_FINE_STEP;
    }
    best
}

/// Score one candidate dit length against a run list: fit the edge bias and the
/// spacing unit, then take the fraction of intervals the model fails to explain.
fn eval_fit(runs: &[Run], unit: f32) -> Option<Fit> {
    // Below a couple of envelope samples per dit the run lengths are
    // quantisation noise and any speed "fits".
    if unit < 2.0 {
        return None;
    }

    // Fit the bias from the intervals whose nominal length is not in doubt:
    // marks (1 or 3 units, whichever is nearer) and inter-element gaps. Longer
    // gaps are left out because an operator's — or Farnsworth's — character and
    // word spacing is not held to the nominal 3 and 7.
    let mut bias = 0.0f32;
    for _ in 0..3 {
        let mut acc = 0.0f32;
        let mut n = 0u32;
        for r in runs {
            let l = r.len();
            if r.on {
                let c = if l - bias > 2.0 * unit { 3.0 } else { 1.0 };
                acc += l - c * unit;
                n += 1;
            } else if l + bias < 2.0 * unit {
                acc += unit - l;
                n += 1;
            }
        }
        if n == 0 {
            return None;
        }
        bias = (acc / n as f32).clamp(-0.45 * unit, 0.45 * unit);
    }

    // Marks and inter-element gaps: scored against the element unit, two-sided
    // and strictly. These are the intervals Morse actually fixes.
    let mut fit = 0.0f32;
    let mut n = 0u32;
    let (mut dits, mut dahs, mut longs) = (0u32, 0u32, 0u32);
    for r in runs {
        let l = r.len();
        if r.on {
            let dah = l - bias > 2.0 * unit;
            if dah {
                dahs += 1;
            } else {
                dits += 1;
            }
            fit += inlier((l - ((if dah { 3.0 } else { 1.0 }) * unit + bias)) / unit);
            n += 1;
        } else if l + bias <= 2.0 * unit {
            fit += inlier((l + bias - unit) / unit);
            n += 1;
        } else {
            longs += 1;
        }
    }
    if n < 4 || dits + dahs < 3 {
        return None;
    }

    // Everything longer than an element gap is character or word spacing, which
    // Farnsworth stretches — 18 WPM characters at 8 WPM spacing puts thirteen
    // dits between letters, not three. So the spacing unit is fitted separately
    // rather than assumed equal to the element unit, and by scanning rather
    // than by iterating from a seed: seeded refinement converges to whatever
    // cluster it started nearest, which for real text is the wrong one.
    let mut space = unit;
    if longs > 0 {
        let mut best = f32::NEG_INFINITY;
        for &ratio in SPACE_RATIOS {
            let s = unit * ratio;
            let mut acc = 0.0f32;
            for r in runs {
                let g = r.len() + bias;
                if !r.on && g > 2.0 * unit {
                    acc += inlier(gap_dev(g, s));
                }
            }
            // Prefer plain timing where it explains the gaps as well: stretched
            // spacing is the unusual case, and letting it win ties would hand a
            // noise fit a free parameter to hide behind.
            acc -= 0.02 * longs as f32 * ratio.ln();
            if acc > best {
                best = acc;
                space = s;
            }
        }
        fit += best;
        n += longs;
    }

    let mut cost = 1.0 - fit / n as f32;
    // Morse at the right speed uses both element lengths. A candidate that
    // explains everything with one of them is usually the true speed's third or
    // triple — real text breaks the tie, and this nudges it.
    if dits == 0 || dahs == 0 {
        cost += 0.25;
    }
    Some(Fit { unit, space, bias, cost, longs })
}

/// How well one interval fits: 1 dead on, falling to 0 over [`FIT_SIGMA`].
fn inlier(dev: f32) -> f32 {
    (-(dev * dev) / (2.0 * FIT_SIGMA * FIT_SIGMA)).exp()
}

/// Deviation of one character-or-word gap `g` from a spacing unit `s`, in
/// spacing units.
fn gap_dev(g: f32, s: f32) -> f32 {
    if g <= 5.0 * s {
        (g - 3.0 * s) / s
    } else {
        // Between words. An operator pausing to think sends a gap longer than
        // seven units and is still sending Morse, so over-length is charged at
        // a fraction of the rate — but it cannot be free. Exempting it entirely
        // lets the spacing unit collapse to the element unit and call every gap
        // in a Farnsworth transmission a word gap, which reads back as every
        // letter its own word.
        let d = (g - 7.0 * s) / s;
        if d > 0.0 { d * 0.55 } else { d }
    }
}

/// `q`-quantile of `v`, which is reordered in place.
fn percentile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let k = ((v.len() - 1) as f32 * q).round() as usize;
    let (_, m, _) = v.select_nth_unstable_by(k, |a, b| a.total_cmp(b));
    *m
}

// ─── narrowband receive front end ────────────────────────────────────────────

/// Envelope rate the audio front end decimates to. Ten samples across a dit at
/// 60 WPM, which is finer than the timing fit needs and cheap to analyse.
const TARGET_ENV_RATE: f32 = 500.0;
/// Detector noise bandwidths (Hz, two-sided) tried, in turn, before a speed is
/// known.
///
/// One width cannot acquire everything: at the edge of copy the bandwidth has
/// to be close to the signal's own keying rate before the envelope is clean
/// enough to fit a speed to, and the keying rate is the thing not yet known.
/// The first is right for the middle of the range, where most CW is; the second
/// acquires a contest operator at 45 WPM; the third is for slow, weak signals.
const ACQUIRE_ENBW_HZ: &[f32] = &[35.0, 75.0, 22.0];
/// Envelope samples spent on each acquisition width before trying the next.
const ACQUIRE_DWELL: u32 = 600;
/// Once the speed is known, the detector narrows to this many times the dit
/// *rate*. Two is about the last point at which the keying edges are still
/// crisp: narrower filters the marks themselves.
const ENBW_PER_DIT_RATE: f32 = 2.0;
/// Bounds on the adapted bandwidth.
const ENBW_MIN_HZ: f32 = 25.0;
const ENBW_MAX_HZ: f32 = 130.0;
/// How far the AFC may pull from the operator's pitch. Beyond this it is a
/// different signal, not a mistune.
const AFC_RANGE_HZ: f32 = 35.0;
/// Fraction of the measured frequency error corrected per hop.
const AFC_GAIN: f32 = 0.25;

/// A running-sum stage: the moving average of the last `n` samples, kept
/// incrementally so it costs one add and one subtract per sample however wide
/// it is.
struct Boxcar {
    ring: Vec<Complex32>,
    head: usize,
    sum: Complex32,
}

impl Boxcar {
    /// A stage `n` wide, every slot pre-loaded with `z` so that resizing one
    /// mid-stream does not punch a hole in the envelope while it refills.
    fn new(n: usize, z: Complex32) -> Self {
        let n = n.max(1);
        Boxcar { ring: vec![z; n], head: 0, sum: z * n as f32 }
    }

    fn width(&self) -> usize {
        self.ring.len()
    }

    fn push(&mut self, z: Complex32) -> Complex32 {
        self.sum += z - self.ring[self.head];
        self.ring[self.head] = z;
        self.head = (self.head + 1) % self.ring.len();
        self.sum / self.ring.len() as f32
    }
}

/// Width of each of the two cascaded boxcars giving a two-sided noise bandwidth
/// of `enbw` at `rate`. The cascade's bandwidth is ⅔·rate/n.
fn boxcar_width(rate: f64, enbw: f32) -> usize {
    ((2.0 / 3.0) * rate as f32 / enbw).round().max(2.0) as usize
}

/// The bandwidth a cascade of two boxcars `n` wide actually has.
fn enbw_for(rate: f64, n: usize) -> f32 {
    (2.0 / 3.0) * rate as f32 / n as f32
}

/// Narrowband CW receiver: mixes one audio tone to baseband, detects its keying
/// envelope, and reads it with a [`CwDecoder`].
pub struct CwRx {
    rate: f64,
    /// Operator's pitch — the tone the waterfall cursor is on.
    pitch: f32,
    /// AFC correction, added to `pitch` to make the mixer frequency.
    offset: f32,
    ph: f32,
    /// Two cascaded running sums — a triangular, and so *symmetric*, low-pass.
    ///
    /// A recursive low-pass would be cheaper and is the obvious choice, but its
    /// impulse response is one-sided: the envelope decays down its tail for
    /// milliseconds after the key opens, so every mark measures long by an
    /// amount that depends on where the threshold sits, and on a strong signal
    /// (where the threshold is far down the skirt) that is most of a dit. A
    /// symmetric response moves both edges of a mark the same way and leaves its
    /// length alone, which is the only thing the timing fit downstream cares
    /// about.
    box1: Boxcar,
    box2: Boxcar,
    /// Two-sided equivalent noise bandwidth of that pair, for the SNR readout.
    enbw_hz: f32,
    decim: usize,
    decim_n: usize,
    env_rate: f32,
    /// Decimated baseband, kept for the AFC's phase-difference estimate.
    bb: Vec<Complex32>,
    env: Vec<f32>,
    prev: Complex32,
    afc_acc: Complex32,
    peak: f32,
    mag: f32,
    /// Envelope samples spent without a lock, which is what walks the detector
    /// through [`ACQUIRE_ENBW_HZ`].
    searching: u32,
    dec: CwDecoder,
}

impl CwRx {
    pub fn new(rate: f64, pitch_hz: f32) -> Self {
        let decim = (rate as f32 / TARGET_ENV_RATE).round().max(1.0) as usize;
        let env_rate = rate as f32 / decim as f32;
        let n = boxcar_width(rate, ACQUIRE_ENBW_HZ[0]);
        CwRx {
            rate,
            pitch: pitch_hz,
            offset: 0.0,
            ph: 0.0,
            box1: Boxcar::new(n, Complex32::default()),
            box2: Boxcar::new(n, Complex32::default()),
            enbw_hz: enbw_for(rate, n),
            decim,
            decim_n: 0,
            env_rate,
            bb: Vec::new(),
            env: Vec::new(),
            prev: Complex32::default(),
            afc_acc: Complex32::default(),
            peak: 0.0,
            mag: 0.0,
            searching: 0,
            dec: CwDecoder::new(env_rate),
        }
    }

    /// Retune to a new pitch. The AFC offset and the decoder's window are only
    /// dropped on a real move — a repeated set of the same pitch (the panel
    /// pushes its config on every status update) must not restart the decode.
    pub fn set_pitch(&mut self, hz: f32) {
        if (hz - self.pitch).abs() < 0.5 {
            return;
        }
        self.pitch = hz;
        self.offset = 0.0;
        self.afc_acc = Complex32::default();
        self.dec.reset();
    }

    /// Pin the speed search, or `None` for auto.
    pub fn set_speed_lock(&mut self, wpm: Option<f32>) {
        self.dec.set_speed_lock(wpm);
    }

    /// The tone actually being copied: the operator's pitch plus whatever the
    /// AFC has pulled.
    pub fn tone_hz(&self) -> f32 {
        self.pitch + self.offset
    }

    pub fn wpm(&self) -> f32 {
        self.dec.wpm()
    }

    pub fn locked(&self) -> bool {
        self.dec.locked()
    }

    /// Signal-to-noise referred to the 500 Hz bandwidth CW reports are quoted
    /// in. The detector is far narrower than that, so what it measures has to be
    /// scaled back up to be comparable with anyone else's number.
    pub fn snr_db(&self) -> f32 {
        self.dec.raw_snr_db() + 10.0 * (self.enbw_hz / 500.0).log10()
    }

    /// Detector output level, for the panel's tuning indicator.
    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    pub fn reset(&mut self) {
        self.dec.reset();
        self.offset = 0.0;
        self.afc_acc = Complex32::default();
    }

    /// Feed audio at the construction rate; returns whatever new text it
    /// completed.
    pub fn process(&mut self, audio: &[f32]) -> String {
        self.bb.clear();
        self.env.clear();
        let inc = TAU * (self.pitch + self.offset) / self.rate as f32;
        for &a in audio {
            let z = Complex32::new(a * self.ph.cos(), -a * self.ph.sin());
            self.ph += inc;
            if self.ph > TAU {
                self.ph -= TAU;
            }
            let y = self.box2.push(self.box1.push(z));
            self.decim_n += 1;
            if self.decim_n == self.decim {
                self.decim_n = 0;
                self.bb.push(y);
                self.env.push(y.norm());
            }
        }

        // AFC: during key-down the baseband is a complex exponential at the
        // residual mistune, so the phase step between neighbouring samples *is*
        // that error. Accumulating the products and taking one argument at the
        // end is the same estimate averaged, without unwrapping anything.
        for (i, &z) in self.bb.iter().enumerate() {
            let m = self.env[i];
            self.mag = 0.995 * self.mag + 0.005 * m;
            self.peak = if m > self.peak { m } else { self.peak * 0.9995 };
            if m > 0.5 * self.peak {
                self.afc_acc += z * self.prev.conj();
            }
            self.prev = z;
        }
        if self.afc_acc.norm() > 0.0 {
            let df = self.afc_acc.arg() * self.env_rate / TAU;
            if df.is_finite() {
                self.offset = (self.offset + AFC_GAIN * df).clamp(-AFC_RANGE_HZ, AFC_RANGE_HZ);
            }
            self.afc_acc = Complex32::default();
        }

        let env = std::mem::take(&mut self.env);
        let out = self.dec.push(&env);
        self.env = env;
        self.retune_detector();
        out
    }

    /// Match the detector bandwidth to the speed being copied.
    ///
    /// Before the speed is known the detector has to be wide, and wide costs
    /// sensitivity: what breaks copy at the edge is not the mark being too weak
    /// to see but its envelope wobbling through the threshold, and the wobble
    /// is the noise admitted by the bandwidth. At 20 WPM, going from the
    /// acquisition width to twice the dit rate is about 4 dB — the difference
    /// between a signal that is copied and one that is nearly copied.
    fn retune_detector(&mut self) {
        let enbw = if self.dec.locked() {
            self.searching = 0;
            (ENBW_PER_DIT_RATE * self.dec.wpm() / 1.2).clamp(ENBW_MIN_HZ, ENBW_MAX_HZ)
        } else {
            // Not locked, so the fitted speed is not to be trusted to choose a
            // bandwidth from — and waiting for a lock before narrowing means
            // never narrowing, because at the edge of copy it is the bandwidth
            // that is stopping the lock. Walk the candidates instead.
            self.searching = self.searching.saturating_add(self.env.len() as u32);
            let i = (self.searching / ACQUIRE_DWELL) as usize % ACQUIRE_ENBW_HZ.len();
            ACQUIRE_ENBW_HZ[i]
        };
        let n = boxcar_width(self.rate, enbw);
        // Hysteresis: rebuilding the filter is only nearly seamless, and doing
        // it because the fitted speed twitched by a percent would cost more
        // than the bandwidth gained.
        let cur = self.box1.width();
        if (n as f32 - cur as f32).abs() < 0.2 * cur as f32 {
            return;
        }
        // Seed both stages with what they are currently putting out, so the
        // envelope carries straight on rather than dipping to nothing while the
        // new, wider rings fill.
        let held = self.box2.push(self.box1.push(Complex32::default()));
        self.box1 = Boxcar::new(n, held);
        self.box2 = Boxcar::new(n, held);
        self.enbw_hz = enbw_for(self.rate, n);
    }
}

// ─── keyed sidetone transmitter ──────────────────────────────────────────────

/// Rise and fall time of the keying envelope, in milliseconds. Long enough to
/// keep the click sidebands off the neighbours, short enough not to soften a
/// dit at contest speeds.
const SHAPE_MS: f32 = 5.0;

/// One keyed or unkeyed stretch of the outgoing signal.
struct Key {
    on: bool,
    samples: usize,
    /// Set on the last element of a character: the index of the character that
    /// has finished going out once this element does, which is what colours the
    /// sent prefix in the panel.
    char_done: Option<usize>,
}

/// CW transmitter: text in, a shaped keyed sidetone out.
///
/// Text is keyed as it arrives rather than a message at a time, so an operator
/// typing into the panel is heard typing — which is how CW is worked.
pub struct CwTx {
    rate: f64,
    pitch: f64,
    wpm: f32,
    /// Farnsworth character speed. Equal to (or below) `wpm` means no
    /// Farnsworth: elements and spacing are both at `wpm`.
    fw_wpm: f32,
    ph: f64,
    q: VecDeque<Key>,
    cur_left: usize,
    cur_on: bool,
    cur_done: Option<usize>,
    total_chars: usize,
    sent_chars: usize,
    /// Keying-envelope position, 0 (key up) to 1 (key down).
    shape: f32,
    shape_step: f32,
    /// True once anything has been queued, so the first character does not get
    /// an inter-character gap in front of it.
    started: bool,
    /// Manual keying (issue #322): a pc keyboard held down as a straight key.
    ///
    /// In manual mode the queue of timed elements is ignored — nothing from it
    /// may cut across the operator's hand — and `next_block` becomes a switch:
    /// tone while [`Self::held`] is down, silence while it is up, both through
    /// the same raised-cosine envelope.
    manual: bool,
    /// The straight key's present position while manual keying is engaged.
    held: bool,
}

impl CwTx {
    pub fn new(rate: f64, pitch_hz: f64, wpm: f32) -> Self {
        CwTx {
            rate,
            pitch: pitch_hz,
            wpm: wpm.clamp(5.0, 80.0),
            fw_wpm: 0.0,
            ph: 0.0,
            q: VecDeque::new(),
            cur_left: 0,
            cur_on: false,
            cur_done: None,
            total_chars: 0,
            sent_chars: 0,
            shape: 0.0,
            shape_step: 1.0 / (SHAPE_MS * 1e-3 * rate as f32).max(1.0),
            started: false,
            manual: false,
            held: false,
        }
    }

    /// Retune / change speed. Only what has not been queued yet is affected —
    /// re-timing elements already on their way out would stutter the sending.
    pub fn set_params(&mut self, pitch_hz: f64, wpm: f32, farnsworth_wpm: f32) {
        self.pitch = pitch_hz;
        self.wpm = wpm.clamp(5.0, 80.0);
        self.fw_wpm = farnsworth_wpm;
    }

    /// Dit length in samples at the character speed.
    fn dit(&self) -> usize {
        (self.rate * 1.2 / self.wpm as f64).round().max(1.0) as usize
    }

    /// Inter-character and inter-word gaps in samples.
    ///
    /// Plain timing is 3 and 7 dits. Farnsworth sends the characters at the
    /// full speed and stretches only the spacing, so a beginner has time to
    /// think between letters without learning them at a rhythm they will have
    /// to unlearn; the split of the extra time between the two gaps is the ARRL
    /// formula's.
    fn gaps(&self) -> (usize, usize) {
        let c = self.wpm;
        let s = self.fw_wpm;
        if s <= 0.0 || s >= c {
            let d = self.dit();
            return (3 * d, 7 * d);
        }
        let ta = (60.0 * c - 37.2 * s) / (c * s); // seconds of spacing per word
        let tc = 3.0 * ta / 19.0;
        let tw = 7.0 * ta / 19.0;
        ((self.rate * tc as f64) as usize, (self.rate * tw as f64) as usize)
    }

    /// Queue `text` for transmission. Characters with no Morse representation
    /// are dropped, but still counted, so the panel's sent-prefix colouring
    /// stays lined up with what the operator typed.
    pub fn push_text(&mut self, text: &str) {
        let dit = self.dit();
        let (char_gap, word_gap) = self.gaps();
        for ch in text.chars() {
            let ci = self.total_chars;
            self.total_chars += 1;
            if ch == ' ' || ch == '\n' || ch == '\r' {
                // A word gap is seven units *including* the three that separate
                // the characters either side of it, not seven on top of them —
                // so queue the difference and let the next character contribute
                // its own gap as usual.
                let samples = word_gap.saturating_sub(char_gap);
                self.q.push_back(Key { on: false, samples, char_done: Some(ci) });
                self.started = true;
                continue;
            }
            let Some(code) = morse_encode(ch) else {
                // Nothing to send: mark it done immediately so the colouring
                // does not stall on a character that will never go out.
                if let Some(last) = self.q.back_mut() {
                    last.char_done = Some(ci);
                } else {
                    self.sent_chars = ci + 1;
                }
                continue;
            };
            if self.started {
                self.q.push_back(Key { on: false, samples: char_gap, char_done: None });
            }
            self.started = true;
            let n = code.chars().count();
            for (i, el) in code.chars().enumerate() {
                let samples = if el == '-' { 3 * dit } else { dit };
                let last = i + 1 == n;
                self.q.push_back(Key {
                    on: true,
                    samples,
                    char_done: if last { Some(ci) } else { None },
                });
                if !last {
                    self.q.push_back(Key { on: false, samples: dit, char_done: None });
                }
            }
        }
    }

    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }

    /// Turn manual keying on or off (issue #322). Coming in, everything the
    /// timed keyer had queued or partially sent is dropped: the keyer is about
    /// to hand the operator the frequency, and a half-typed automatic message
    /// surfacing between dits would be the last thing they expected to hear.
    pub fn set_manual(&mut self, manual: bool) {
        self.clear();
        self.manual = manual;
        if !manual {
            self.held = false;
        }
    }

    /// The straight key up or down, while manual keying is engaged.
    pub fn set_held(&mut self, down: bool) {
        self.held = down && self.manual;
    }

    /// Whether the straight key is down.
    pub fn held(&self) -> bool {
        self.held
    }

    /// Samples of keying still to go out, the element in progress included.
    pub fn queued_samples(&self) -> usize {
        self.q.iter().map(|k| k.samples).sum::<usize>() + self.cur_left
    }

    pub fn total_chars(&self) -> usize {
        self.total_chars
    }

    /// True once every queued element has gone out and the envelope has closed.
    ///
    /// Manual keying settles the moment the straight key is up again and the
    /// envelope has closed — after that the keyer is just a switch neither
    /// closed nor opening.
    pub fn drained(&self) -> bool {
        if self.manual {
            return !self.held && self.shape <= 0.0;
        }
        self.q.is_empty() && self.cur_left == 0 && self.shape <= 0.0
    }

    pub fn clear(&mut self) {
        self.q.clear();
        self.cur_left = 0;
        self.cur_on = false;
        self.cur_done = None;
        self.total_chars = 0;
        self.sent_chars = 0;
        self.started = false;
    }

    /// Fill `out` with the next block of sidetone.
    pub fn next_block(&mut self, out: &mut [f32]) {
        let inc = std::f64::consts::TAU * self.pitch / self.rate;
        for s in out.iter_mut() {
            // Manual keying: the operator's hand is the timing. The dialogue
            // with the timed queue is deliberately skipped wholesale — nothing
            // queued before the key went down may surface between the
            // operator's elements.
            if self.manual {
                let target = if self.held { 1.0 } else { 0.0 };
                self.shape = if self.shape < target {
                    (self.shape + self.shape_step).min(target)
                } else {
                    (self.shape - self.shape_step).max(target)
                };
                let amp = 0.5 - 0.5 * (std::f32::consts::PI * self.shape).cos();
                self.ph += inc;
                if self.ph > std::f64::consts::TAU {
                    self.ph -= std::f64::consts::TAU;
                }
                *s = amp * self.ph.sin() as f32;
                continue;
            }
            if self.cur_left == 0 {
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                match self.q.pop_front() {
                    Some(k) => {
                        self.cur_on = k.on;
                        self.cur_left = k.samples;
                        self.cur_done = k.char_done;
                    }
                    None => {
                        self.cur_on = false;
                        self.cur_left = 1;
                        self.started = false;
                    }
                }
            }
            self.cur_left -= 1;
            let target = if self.cur_on { 1.0 } else { 0.0 };
            self.shape = if self.shape < target {
                (self.shape + self.shape_step).min(target)
            } else {
                (self.shape - self.shape_step).max(target)
            };
            // Raised cosine on the envelope, not a ramp: a linear key edge has a
            // corner in it, and a corner is what a click is.
            let amp = 0.5 - 0.5 * (std::f32::consts::PI * self.shape).cos();
            self.ph += inc;
            if self.ph > std::f64::consts::TAU {
                self.ph -= std::f64::consts::TAU;
            }
            *s = amp * self.ph.sin() as f32;
        }
    }
}

/// How long `text` takes to send, in seconds, at `wpm` (and `farnsworth_wpm`
/// spacing, 0 for none).
///
/// Answered by queueing the text into a [`CwTx`] and measuring the queue, so it
/// is the same timing the sidetone keyer would produce rather than a second,
/// drifting copy of the rules. What needs it is a rig keying itself from text
/// we handed it: nothing comes back to say when the rig finished, so the only
/// way to know when the next chunk may go — and when our own sending has
/// stopped landing in the decoder — is to have counted the elements.
pub fn text_duration_s(text: &str, wpm: f32, farnsworth_wpm: f32) -> f32 {
    const RATE: f64 = 8000.0;
    let mut tx = CwTx::new(RATE, 600.0, wpm);
    tx.set_params(600.0, wpm, farnsworth_wpm);
    tx.push_text(text);
    tx.queued_samples() as f32 / RATE as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── deterministic signal generation with impairments ────────────────────

    /// A repeatable Gaussian source. `rand` is not a dependency here and the
    /// tests must fail the same way twice, so this is a plain LCG through
    /// Box–Muller.
    struct Rng(u64);

    impl Rng {
        fn uniform(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 40) as f32 + 0.5) / (1u64 << 24) as f32
        }
        fn normal(&mut self) -> f32 {
            let u1 = self.uniform().max(1e-7);
            let u2 = self.uniform();
            (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
        }
    }

    /// Everything that makes a real CW signal harder to read than a test tone.
    #[derive(Clone, Copy)]
    struct Chan {
        /// Signal-to-noise in a 500 Hz bandwidth, dB — the figure CW reports
        /// are quoted in, so the numbers mean what an operator thinks they mean.
        snr_db: f32,
        wpm: f32,
        /// Farnsworth word speed; 0 for none.
        fw_wpm: f32,
        /// QSB: fade depth in dB and rate in Hz.
        qsb_db: f32,
        qsb_hz: f32,
        /// Per-element timing jitter as a fraction of a dit — a hand at a key.
        jitter: f32,
        /// Systematic weight error: dits and dahs sent long (+) or short (−) as
        /// a fraction of a dit, the commonest fault of a hand-set keyer.
        weight: f32,
        /// Carrier drift over the whole transmission, Hz.
        drift_hz: f32,
        pitch: f32,
        seed: u64,
    }

    impl Default for Chan {
        fn default() -> Self {
            Chan {
                snr_db: 30.0,
                wpm: 20.0,
                fw_wpm: 0.0,
                qsb_db: 0.0,
                qsb_hz: 0.0,
                jitter: 0.0,
                weight: 0.0,
                drift_hz: 0.0,
                pitch: 700.0,
                seed: 12_345,
            }
        }
    }

    const FS: f64 = 8000.0;

    /// The keyed element sequence for `text`: `(key_on, dits)` pairs, before any
    /// impairment is applied.
    fn elements(text: &str, ch: &Chan) -> Vec<(bool, f32)> {
        let (char_gap, word_gap) = if ch.fw_wpm > 0.0 && ch.fw_wpm < ch.wpm {
            let ta = (60.0 * ch.wpm - 37.2 * ch.fw_wpm) / (ch.wpm * ch.fw_wpm);
            let dit_s = 1.2 / ch.wpm;
            (3.0 * ta / 19.0 / dit_s, 7.0 * ta / 19.0 / dit_s)
        } else {
            (3.0, 7.0)
        };
        let mut out = Vec::new();
        let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
        for (wi, word) in words.iter().enumerate() {
            let chars: Vec<char> = word.chars().collect();
            for (ci, c) in chars.iter().enumerate() {
                let code = morse_encode(*c).unwrap();
                let n = code.chars().count();
                for (ei, el) in code.chars().enumerate() {
                    out.push((true, if el == '-' { 3.0 } else { 1.0 }));
                    if ei + 1 < n {
                        out.push((false, 1.0));
                    }
                }
                if ci + 1 < chars.len() {
                    out.push((false, char_gap));
                }
            }
            if wi + 1 < words.len() {
                out.push((false, word_gap));
            }
        }
        out
    }

    /// Render `text` to audio through `ch`.
    fn render(text: &str, ch: &Chan) -> Vec<f32> {
        let mut rng = Rng(ch.seed);
        let dit = FS * 1.2 / ch.wpm as f64;
        let shape = (SHAPE_MS * 1e-3 * FS as f32).max(1.0);

        // Key state per sample, with jitter and weight folded into the element
        // lengths. Leading and trailing silence give the decoder a noise floor
        // to measure and a chance to flush the last character.
        let mut key: Vec<f32> = Vec::new();
        let lead = (FS * 0.7) as usize;
        key.resize(lead, 0.0);
        for (on, dits) in elements(text, ch) {
            let mut n = dits as f64 * dit;
            if ch.jitter > 0.0 {
                n += (rng.normal() * ch.jitter) as f64 * dit;
            }
            // Weight lengthens marks at the expense of the gaps beside them,
            // which is what a mis-set keyer actually does.
            n += if on { ch.weight as f64 * dit } else { -(ch.weight as f64) * dit };
            let n = n.max(1.0) as usize;
            key.resize(key.len() + n, if on { 1.0 } else { 0.0 });
        }
        key.resize(key.len() + (FS * 1.5) as usize, 0.0);

        // Shape the key envelope (raised cosine) so the spectrum is a CW
        // signal's rather than a square wave's.
        let mut env = vec![0.0f32; key.len()];
        let mut s = 0.0f32;
        let step = 1.0 / shape;
        for (i, &k) in key.iter().enumerate() {
            s = if s < k { (s + step).min(k) } else { (s - step).max(k) };
            env[i] = 0.5 - 0.5 * (std::f32::consts::PI * s).cos();
        }

        // Noise power for the requested SNR, referred to 500 Hz: the tone's
        // key-down power is A²/2, and white noise of variance σ² over the full
        // audio band carries σ²·500/(fs/2) of it in the reference bandwidth.
        let sigma = (0.5 / 10f32.powf(ch.snr_db / 10.0) * (FS as f32 / 2.0) / 500.0).sqrt();

        let n = env.len();
        let mut out = Vec::with_capacity(n);
        let mut ph = 0.0f64;
        for (i, &e) in env.iter().enumerate() {
            let t = i as f32 / FS as f32;
            let f = ch.pitch as f64 + (ch.drift_hz * i as f32 / n as f32) as f64;
            ph += std::f64::consts::TAU * f / FS;
            // QSB as a slow sinusoid in dB, so the trough is `qsb_db` down.
            let fade = if ch.qsb_db > 0.0 {
                let d = ch.qsb_db * 0.5 * ((TAU * ch.qsb_hz * t).sin() - 1.0);
                10f32.powf(d / 20.0)
            } else {
                1.0
            };
            out.push(e * fade * ph.sin() as f32 + sigma * rng.normal());
        }
        out
    }

    /// Fraction of `sent` that survived, by edit distance — the figure that
    /// matters is how much of the message a reader gets, and both dropped and
    /// invented characters cost.
    fn accuracy(sent: &str, got: &str) -> f32 {
        let a: Vec<char> = sent.chars().collect();
        let b: Vec<char> = got.chars().collect();
        if a.is_empty() {
            return 1.0;
        }
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        let mut cur = vec![0usize; b.len() + 1];
        for i in 1..=a.len() {
            cur[0] = i;
            for j in 1..=b.len() {
                let sub = prev[j - 1] + usize::from(a[i - 1] != b[j - 1]);
                cur[j] = sub.min(prev[j] + 1).min(cur[j - 1] + 1);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        1.0 - (prev[b.len()] as f32 / a.len() as f32).min(1.0)
    }

    /// Run `text` through `ch` and the decoder, in realistic block sizes.
    fn copy(text: &str, ch: &Chan) -> String {
        let audio = render(text, ch);
        let mut rx = CwRx::new(FS, ch.pitch);
        let mut out = String::new();
        for block in audio.chunks(512) {
            out.push_str(&rx.process(block));
        }
        out
    }

    /// The message under test. Long enough for the fit to have something to
    /// work with and shaped like a real over: callsigns, numbers, prosigns.
    const MSG: &str = "CQ CQ DE W1AW W1AW K UR RST 599 599 QTH BOSTON MA = 73 TU E E";

    #[test]
    fn morse_table_is_invertible() {
        for &(ch, code) in TABLE {
            assert_eq!(morse_decode(code), Some(ch), "{ch}");
            assert_eq!(morse_encode(ch), Some(code), "{ch}");
        }
        // No two characters share an element string, or the table would decode
        // ambiguously depending on iteration order.
        for (i, a) in TABLE.iter().enumerate() {
            for b in &TABLE[i + 1..] {
                assert_ne!(a.1, b.1, "{} and {} share {}", a.0, b.0, a.1);
            }
        }
    }

    /// Issue #382: `ä`, `ö` and `å` are ordinary Morse — ITU-R M.1677-1 lists
    /// them — and copying them as `?` is a hole in the table, not a font.
    #[test]
    fn the_accented_letters_go_out_and_come_back() {
        for (ch, code) in
            [('Ä', ".-.-"), ('Å', ".--.-"), ('Ö', "---."), ('Ü', "..--"), ('É', "..-..")]
        {
            assert_eq!(morse_encode(ch), Some(code), "{ch} sent");
            assert_eq!(morse_decode(code), Some(ch), "{code} copied");
            // Lower case reaches the same entry, which `to_ascii_uppercase`
            // never did: it leaves every letter outside ASCII alone.
            let lower = ch.to_lowercase().next().expect("one lower-case letter");
            assert_eq!(morse_encode(lower), Some(code), "{lower} sent");
        }
        // The letters that share a code with one of those reach it too, so a
        // Danish operator's Æ and Ø go out as the code they have always had.
        assert_eq!(morse_encode('ø'), morse_encode('ö'));
        assert_eq!(morse_encode('æ'), morse_encode('ä'));
        assert_eq!(morse_encode('à'), morse_encode('å'));
        // ß upper-cases to two letters, and half of a word is worse than none.
        assert_eq!(morse_encode('ß'), None);
        // And a real over in it copies back as itself.
        let msg = "DE OH2ABC = QTH JYVASKYLA MEN HÄR ÄR DET KALLT = 73";
        assert_eq!(copy(msg, &Chan::default()).trim(), msg, "clean copy differs");
    }

    #[test]
    fn copies_a_clean_signal_exactly() {
        let got = copy(MSG, &Chan::default());
        assert_eq!(got.trim(), MSG, "clean copy differs");
    }

    #[test]
    fn copies_across_the_speed_range() {
        for wpm in [10.0f32, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0] {
            let ch = Chan { wpm, ..Default::default() };
            let got = copy(MSG, &ch);
            let acc = accuracy(MSG, got.trim());
            assert!(acc > 0.96, "{wpm} WPM: {acc:.3}  {got:?}");
            // …and the reported speed has to be right, or the panel's readout
            // and any speed lock built on it are lying.
            let mut rx = CwRx::new(FS, ch.pitch);
            for b in render(MSG, &ch).chunks(512) {
                rx.process(b);
            }
            let err = (rx.wpm() - wpm).abs() / wpm;
            assert!(err < 0.06, "{wpm} WPM measured as {:.1}", rx.wpm());
        }
    }

    #[test]
    fn copies_down_to_low_signal_to_noise() {
        // Quoted the way CW reports are, in 500 Hz. A good operator copies
        // solidly to about −8 dB; a decoder that holds full copy to +5 and most
        // of a message at +3 is doing well, and by 0 dB it is picking up what it
        // can between the holes.
        for (snr, want) in
            [(20.0f32, 0.99f32), (12.0, 0.99), (8.0, 0.97), (5.0, 0.95), (3.0, 0.85), (0.0, 0.45)]
        {
            let ch = Chan { snr_db: snr, ..Default::default() };
            let got = copy(MSG, &ch);
            let acc = accuracy(MSG, got.trim());
            assert!(acc >= want, "{snr} dB SNR: {acc:.3} < {want}  {got:?}");
        }
    }

    #[test]
    fn copies_through_deep_qsb() {
        // Referring the threshold to the noise floor rather than to the mark
        // level is what this is really testing: the floor is the one thing a
        // fade does not move, so one threshold set against it holds good from
        // the peak of the fade to the trough.
        //
        // A 15 dB fade on a 15 dB signal puts the trough at 0 dB, which is the
        // edge of copy — so what is asked for is that most of the message
        // survives, not all of it. Deeper fades take the trough below anything
        // readable and the decoder is then reduced to copying the peaks, which
        // `deep_qsb_still_copies_the_peaks` covers.
        for (depth, rate) in [(10.0f32, 0.2f32), (15.0, 0.15), (12.0, 0.5)] {
            let ch = Chan { snr_db: 15.0, qsb_db: depth, qsb_hz: rate, ..Default::default() };
            let got = copy(MSG, &ch);
            let acc = accuracy(MSG, got.trim());
            assert!(acc > 0.75, "{depth} dB QSB at {rate} Hz: {acc:.3}  {got:?}");
        }
    }

    #[test]
    fn deep_qsb_still_copies_the_peaks() {
        // 30 dB of fade spends part of every cycle with nothing there to copy.
        // What matters is that the decoder keeps copying either side of the
        // hole rather than losing lock and staying silent for the rest of the
        // over — the failure mode of a decoder that adapts its threshold to the
        // signal instead of to the noise.
        for wpm in [15.0f32, 25.0, 35.0] {
            let ch = Chan { wpm, snr_db: 15.0, qsb_db: 30.0, qsb_hz: 0.2, ..Default::default() };
            let got = copy(MSG, &ch);
            let acc = accuracy(MSG, got.trim());
            assert!(acc > 0.30, "{wpm} WPM through a 30 dB fade: {acc:.3}  {got:?}");
        }
    }

    #[test]
    fn copies_a_hand_sent_signal() {
        // Jitter is a fist; weight is a keyer set wrong. Both are everywhere on
        // the bands and neither should cost a character.
        for (jitter, weight) in [(0.12f32, 0.0f32), (0.0, 0.25), (0.0, -0.2), (0.10, 0.15)] {
            let ch = Chan { snr_db: 15.0, jitter, weight, ..Default::default() };
            let got = copy(MSG, &ch);
            let acc = accuracy(MSG, got.trim());
            assert!(acc > 0.90, "jitter {jitter} weight {weight}: {acc:.3}  {got:?}");
        }
    }

    #[test]
    fn copies_a_mistuned_and_drifting_signal() {
        // The operator's cursor is 20 Hz off and the other station's VFO walks
        // 15 Hz over the over — the AFC has to hold both.
        let ch = Chan { snr_db: 12.0, drift_hz: 15.0, ..Default::default() };
        let audio = render(MSG, &ch);
        let mut rx = CwRx::new(FS, ch.pitch - 20.0);
        let mut got = String::new();
        for b in audio.chunks(512) {
            got.push_str(&rx.process(b));
        }
        let acc = accuracy(MSG, got.trim());
        assert!(acc > 0.90, "mistuned + drifting: {acc:.3}  {got:?}");
        assert!(
            (rx.tone_hz() - (ch.pitch - 20.0 + 20.0)).abs() < 25.0,
            "AFC settled at {:.0} Hz",
            rx.tone_hz()
        );
    }

    #[test]
    fn copies_farnsworth_spacing() {
        // 18 WPM characters at 8 WPM spacing. The character speed is what the
        // element lengths say; scoring the stretched gaps two-sided would drag
        // the fit down to the word speed and turn every dah into a dit.
        let ch = Chan { wpm: 18.0, fw_wpm: 8.0, snr_db: 20.0, ..Default::default() };
        let got = copy(MSG, &ch);
        let acc = accuracy(MSG, got.trim());
        assert!(acc > 0.93, "Farnsworth: {acc:.3}  {got:?}");
    }

    #[test]
    fn silence_and_noise_decode_to_nothing() {
        // The confidence gate is the whole squelch: with no timing model to fit,
        // the best cost stays high and nothing is emitted. A decoder that
        // scribbles E and T across the panel between overs is unusable.
        let mut rng = Rng(7);
        for level in [0.0f32, 0.05, 0.5] {
            let audio: Vec<f32> = (0..(FS as usize * 8)).map(|_| level * rng.normal()).collect();
            let mut rx = CwRx::new(FS, 700.0);
            let mut got = String::new();
            for b in audio.chunks(512) {
                got.push_str(&rx.process(b));
            }
            assert!(got.trim().is_empty(), "noise at {level} decoded to {got:?}");
        }
    }

    #[test]
    fn a_second_signal_in_the_passband_is_not_copied() {
        // Two stations 120 Hz apart. The detector is 40 Hz wide, so the one the
        // cursor is not on must not get into the copy.
        let a = render("CQ DE W1AW K", &Chan { pitch: 700.0, snr_db: 20.0, ..Default::default() });
        let b = render(
            "TEST DE K5ZZ K5ZZ",
            &Chan { pitch: 820.0, snr_db: 20.0, wpm: 28.0, seed: 999, ..Default::default() },
        );
        let n = a.len().min(b.len());
        let mixed: Vec<f32> = (0..n).map(|i| a[i] + b[i]).collect();
        let mut rx = CwRx::new(FS, 700.0);
        let mut got = String::new();
        for c in mixed.chunks(512) {
            got.push_str(&rx.process(c));
        }
        assert!(got.contains("W1AW"), "wanted station missing: {got:?}");
        assert!(!got.contains("K5ZZ"), "neighbour bled through: {got:?}");
    }

    #[test]
    fn transmit_round_trips_through_the_decoder() {
        // The keyer and the decoder have to agree about Morse timing, and the
        // shaped envelope has to still be readable. Feeding one into the other
        // is the cheapest way to keep them honest.
        for wpm in [12.0f32, 22.0, 35.0] {
            let mut tx = CwTx::new(FS, 700.0, wpm);
            tx.push_text("CQ DE W1AW K ");
            let mut audio = vec![0.0f32; (FS * 0.7) as usize];
            while !tx.drained() {
                let mut blk = [0.0f32; 512];
                tx.next_block(&mut blk);
                audio.extend_from_slice(&blk);
            }
            audio.resize(audio.len() + (FS * 1.5) as usize, 0.0);
            // A receiver always has a noise floor, and the decoder measures its
            // thresholds against one. Digital silence is not a channel.
            let mut rng = Rng(4242);
            let sigma = (0.5 / 10f32.powf(25.0 / 10.0) * (FS as f32 / 2.0) / 500.0).sqrt();
            for a in audio.iter_mut() {
                *a += sigma * rng.normal();
            }

            let mut rx = CwRx::new(FS, 700.0);
            let mut got = String::new();
            for b in audio.chunks(512) {
                got.push_str(&rx.process(b));
            }
            assert_eq!(got.trim(), "CQ DE W1AW K", "{wpm} WPM round trip");
        }
    }

    #[test]
    fn transmit_reports_characters_as_they_go_out() {
        let mut tx = CwTx::new(FS, 700.0, 20.0);
        tx.push_text("AB");
        assert_eq!(tx.total_chars(), 2);
        assert_eq!(tx.sent_chars(), 0);
        let mut blk = [0.0f32; 256];
        let mut seen = 0;
        while !tx.drained() {
            tx.next_block(&mut blk);
            assert!(tx.sent_chars() >= seen, "sent count went backwards");
            seen = tx.sent_chars();
        }
        assert_eq!(tx.sent_chars(), 2);
        // Text arriving later is keyed straight on: an operator typing mid-over
        // must not have to wait for a message boundary.
        tx.push_text("C");
        assert_eq!(tx.total_chars(), 3);
        assert!(!tx.drained());
    }

    #[test]
    fn transmit_keys_at_the_requested_speed() {
        // A dit at 20 WPM is 60 ms. Measure it off the generated envelope
        // rather than trusting the queue arithmetic.
        let mut tx = CwTx::new(FS, 700.0, 20.0);
        tx.push_text("EE");
        let mut audio = Vec::new();
        while !tx.drained() {
            let mut blk = [0.0f32; 512];
            tx.next_block(&mut blk);
            audio.extend_from_slice(&blk);
        }
        // Half-amplitude crossings bracket each dit at its nominal length.
        let env: Vec<f32> =
            audio.chunks(8).map(|c| c.iter().fold(0.0f32, |a, b| a.max(b.abs()))).collect();
        let rate = FS / 8.0;
        let on: Vec<bool> = env.iter().map(|&e| e > 0.5).collect();
        let mut lens = Vec::new();
        let mut i = 0;
        while i < on.len() {
            if on[i] {
                let s = i;
                while i < on.len() && on[i] {
                    i += 1;
                }
                lens.push((i - s) as f64 / rate * 1000.0);
            } else {
                i += 1;
            }
        }
        assert_eq!(lens.len(), 2, "expected two dits, got {lens:?}");
        for l in lens {
            assert!((l - 60.0).abs() < 6.0, "dit was {l:.1} ms, wanted 60");
        }
    }

    /// Not an assertion — a report. `cargo test -p sdroxide-dsp cw_matrix --
    /// --ignored --nocapture` prints the accuracy surface the thresholds above
    /// were chosen from, which is how the decoder gets tuned without guessing.
    #[test]
    #[ignore = "reporting harness, not a pass/fail test"]
    fn cw_matrix() {
        println!("\n      SNR(500Hz) →");
        print!("{:>6}", "WPM");
        let snrs = [20.0f32, 12.0, 8.0, 5.0, 3.0, 0.0, -3.0, -6.0];
        for s in snrs {
            print!("{s:>7.0}");
        }
        println!();
        for wpm in [10.0f32, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0] {
            print!("{wpm:>6.0}");
            for snr in snrs {
                let ch = Chan { wpm, snr_db: snr, ..Default::default() };
                let acc = accuracy(MSG, copy(MSG, &ch).trim());
                print!("{:>7.2}", acc);
            }
            println!();
        }
        println!("\n      QSB depth (dB) at 0.2 Hz, 15 dB SNR →");
        print!("{:>6}", "WPM");
        let depths = [0.0f32, 10.0, 20.0, 30.0, 40.0];
        for d in depths {
            print!("{d:>7.0}");
        }
        println!();
        for wpm in [15.0f32, 25.0, 35.0] {
            print!("{wpm:>6.0}");
            for d in depths {
                let ch = Chan { wpm, snr_db: 15.0, qsb_db: d, qsb_hz: 0.2, ..Default::default() };
                let acc = accuracy(MSG, copy(MSG, &ch).trim());
                print!("{:>7.2}", acc);
            }
            println!();
        }
        println!();
    }
}
