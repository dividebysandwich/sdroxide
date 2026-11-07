//! NAVTEX: SITOR-B (ITU-R M.625 collective B-mode) at 100 baud, 170 Hz shift.
//!
//! The maritime safety broadcast every coast station in the world sends on
//! 518 kHz in English, on 490 kHz in the national language and on 4209.5 kHz in
//! the tropics. Navigational and meteorological warnings, search-and-rescue
//! bulletins, ice reports and pilot notices — one of the few utility services
//! still worth leaving a receiver on (issue #212).
//!
//! # What the mode is
//!
//! Frequency-shift keying, 170 Hz apart, 100 bits a second, in a **7-bit
//! constant-ratio alphabet**: every valid character has exactly four mark bits
//! and three space bits, so any single bit error makes a code that cannot
//! exist. That is the whole of the error *detection*, and the correction is
//! time diversity — **every character is sent twice, five character times
//! apart**, so the stream alternates between a leading copy and the repeat of a
//! character sent five slots earlier:
//!
//! ```text
//! N  ·  A  ·  U  N  T  A  I  U  C  T  A  I  L  C  _  A  _  L
//! ```
//!
//! ("NAUTICAL", with the odd slots five behind the even ones.) A character
//! whose leading copy is corrupt is taken from its repeat, and only when both
//! are bad is anything lost.
//!
//! # Why the front end is written out again
//!
//! It is the same shape as [`crate::rtty::RttyRx`]'s — mix, band-limit,
//! integrate two tones over a bit, decide against each tone's own recent level
//! — and it is deliberately not shared with it. RTTY's framing is asynchronous
//! start/stop and this is a continuous synchronous stream with no framing bits
//! at all, so only the front half would be common; and RTTY is a mode that
//! works, on the air, today. Refactoring underneath it to save eighty lines
//! here is a trade nobody asked for.

use std::collections::VecDeque;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};
use crate::rtty::Integrator;

/// The audio centre a NAVTEX signal is expected at, in Hz.
///
/// The two tones sit ±85 Hz either side. The channel frequencies are quoted as
/// the *assigned* frequency, which is this centre, so a receiver in USB tunes
/// its dial 1700 Hz below the channel — 516.300 kHz for the 518 kHz service —
/// and [`crate::Mode::Navtex`]'s tone offset is what does that arithmetic.
pub const NAVTEX_CENTER_HZ: f32 = 1700.0;
/// Shift between mark and space, in Hz.
pub const NAVTEX_SHIFT_HZ: f32 = 170.0;
/// Symbol rate.
pub const NAVTEX_BAUD: f64 = 100.0;

/// Bits in one character.
const BITS: usize = 7;
/// Character slots between a character and its repeat.
const FEC_CHARS: usize = 5;
/// Bits between a character and its repeat.
const FEC_BITS: usize = FEC_CHARS * BITS;
/// Bits between one leading character and the next: the stream alternates
/// leading copies and repeats, so a character's own slots are two apart.
const PAIR_BITS: usize = 2 * BITS;
/// Bits held so a character can be decoded once its repeat has arrived, plus
/// the window the phase search runs over.
const HISTORY_BITS: usize = FEC_BITS + BITS + PAIR_BITS * 12;

// ─────────────────────────── CCIR 476 ───────────────────────────

/// Letter shift.
pub const CODE_LTRS: u8 = 0x5a;
/// Figure shift.
pub const CODE_FIGS: u8 = 0x36;
/// Idle "alpha" — one half of the phasing pair, and the filler a transmitter
/// sends between characters.
pub const CODE_ALPHA: u8 = 0x0f;
/// Idle "beta".
pub const CODE_BETA: u8 = 0x33;
/// Idle "rep" — the other half of the phasing pair.
pub const CODE_REP: u8 = 0x66;
/// Signal repetition (character 32).
pub const CODE_CHAR32: u8 = 0x6a;

/// The CCIR 476 alphabet: `(code, letter, figure)`.
///
/// Transcribed from the ITU-R M.476 table as fldigi's `navtex.cxx` carries it,
/// and checked by [`the_alphabet_is_a_constant_ratio_code`]: all thirty-five
/// seven-bit patterns with four mark bits are accounted for — twenty-nine
/// characters here and the six control codes above.
const ALPHABET: [(u8, char, char); 29] = [
    (0x17, 'J', '\''),
    (0x1b, 'F', '!'),
    (0x1d, 'C', ':'),
    (0x1e, 'K', '('),
    (0x27, 'W', '2'),
    (0x2b, 'Y', '6'),
    (0x2d, 'P', '0'),
    (0x2e, 'Q', '1'),
    (0x35, 'G', '&'),
    (0x39, 'M', '.'),
    (0x3a, 'X', '/'),
    (0x3c, 'V', ';'),
    (0x47, 'A', '-'),
    (0x4b, 'S', '\u{7}'),
    (0x4d, 'I', '8'),
    (0x4e, 'U', '7'),
    (0x53, 'D', '$'),
    (0x55, 'R', '4'),
    (0x56, 'E', '3'),
    (0x59, 'N', ','),
    (0x5c, ' ', ' '),
    (0x63, 'Z', '"'),
    (0x65, 'L', ')'),
    (0x69, 'H', '#'),
    (0x6c, '\n', '\n'),
    (0x71, 'O', '9'),
    (0x72, 'B', '?'),
    (0x74, 'T', '5'),
    (0x78, '\r', '\r'),
];

/// Whether a 7-bit pattern is a legal CCIR 476 code: four mark bits, three
/// space. Every single-bit error breaks this, which is what the alphabet is
/// for.
#[must_use]
pub fn is_valid_code(code: u8) -> bool {
    code < 0x80 && code.count_ones() == 4
}

/// The character a code stands for in the current shift, or `None` for a
/// control code (and for anything that is not a character at all).
#[must_use]
pub fn code_to_char(code: u8, figures: bool) -> Option<char> {
    ALPHABET.iter().find(|&&(c, _, _)| c == code).map(|&(_, l, f)| if figures { f } else { l })
}

/// The code for a character in the letter shift — the transmit side, and what
/// the tests synthesise with.
#[must_use]
pub fn char_to_code(ch: char) -> Option<u8> {
    let up = ch.to_ascii_uppercase();
    ALPHABET.iter().find(|&&(_, l, _)| l == up).map(|&(c, _, _)| c)
}

/// The code for a character in the figure shift.
#[must_use]
pub fn figure_to_code(ch: char) -> Option<u8> {
    ALPHABET.iter().find(|&&(_, _, f)| f == ch).map(|&(c, _, _)| c)
}

// ─────────────────────────── the receiver ───────────────────────────

/// Decision references settle at this rate per bit.
const ATC_SMOOTH: f32 = 0.05;
/// …and a branch that has been idle a while relaxes towards the other at this
/// rate, so a long run of one tone does not leave the other's reference stale.
const ATC_RELAX: f32 = 0.02;
/// Bits of one tone before the other's reference starts relaxing.
const ATC_IDLE_BITS: f32 = 8.0;
/// How far below the louder reference the quieter one may sit.
const ATC_MIN_RATIO: f32 = 0.05;
/// Per sample decay of the level peak the signal is measured against — about a
/// second and a half to halve at 8 kHz.
const MAG_PEAK_DECAY: f32 = 0.999_94;
/// Below this fraction of that peak, the station has stopped rather than faded.
/// Thirteen decibels: a fade that deep has taken the message with it anyway,
/// and holding a character phase through one costs the *next* message.
const SIGNAL_GONE: f32 = 0.05;
/// How hard an observed transition pulls the bit clock.
const CLOCK_TRACK_GAIN: f32 = 0.05;

/// AFC loop gain per sample, and the widest tuning error it will chase.
///
/// The range is far wider than RTTY's ±90 Hz, and deliberately so. RTTY is
/// tuned by the operator to a station they can hear and adjust; NAVTEX is a
/// published service on assigned frequencies that every published tuning
/// instruction rounds differently — 516.5 kHz, 516.6 kHz, 1700 Hz below the
/// channel — so the dial is routinely a couple of hundred hertz from where the
/// decoder expects the tones. There is no Doppler to speak of at 518 kHz and
/// nothing for a loop here to chase into: the offset is the operator's own
/// arithmetic and stays where it was put. Issue #608, reported as a strong
/// steady signal decoding to mostly asterisks while fldigi read it cleanly.
const AFC_GAIN: f32 = 3.0e-4;
/// ±350 Hz, which covers every published NAVTEX dial figure and then some.
const AFC_LIMIT_HZ: f32 = 350.0;
/// AFC needs the integrator settled on a single tone: its phase means nothing
/// while the window straddles a transition. A quarter of a bit excludes exactly
/// that, and unlike a magnitude floor it keeps working when mistuning has
/// knocked both branches down the matched filter's skirt — which is the case
/// this loop exists for.
const AFC_SETTLE: f32 = 0.25;
/// Floor on branch separation for AFC, low enough not to limit pull-in and
/// only there to keep the loop out of pure noise.
const AFC_SEP_MIN: f32 = 0.2;

/// Half-width of the front-end band-pass, in Hz.
///
/// Both tones, the keying sidebands of a 100-baud square wave, and the whole
/// AFC pull-in range with a margin — the same sum RTTY builds, and for the
/// same reason: a filter narrower than the loop's range would be discarding the
/// outer tone of a signal the loop is still trying to find.
fn front_end_hz() -> f64 {
    f64::from(NAVTEX_SHIFT_HZ / 2.0 + AFC_LIMIT_HZ + 20.0) + 2.0 * NAVTEX_BAUD
}

/// Both bit phases are decoded at once — see [`BitPhase`].
///
/// Nudging a free-running clock towards "a transition sits at phase 0.5" locks
/// at either of **two** points: the right one, and one exactly half a bit away
/// where every strobe straddles two symbols. Which it falls into depends on
/// where the signal started, and the wrong one is stable — a run of identical
/// bits looks perfectly clean from there, so nothing local to the clock can
/// tell them apart. Measured: the decoder read a stream that began at sample
/// zero and read nothing at all when the same stream began after a second of
/// silence, which is every real signal (a NAVTEX slot is ten minutes of nothing
/// and then a station).
///
/// What *can* tell them apart is the character-phase search: it is looking for
/// a constant-ratio code, and from the wrong bit phase there is not one. So
/// rather than guess and retry — which costs the front of a message every time,
/// and the front of a NAVTEX message is the header that identifies it — both
/// are slicedapart and whichever finds a character phase is the one that is
/// read. The second costs one more bit history and one more search per bit;
/// the front end, which is all the arithmetic, is shared.
const PHASES: usize = 2;

/// Characters of the phase search that must be valid before the stream is
/// declared in phase. Eight of the fourteen candidate offsets are wrong by a
/// whole character, and a wrong one scores near zero on a constant-ratio code.
const SYNC_SCORE: i32 = 8;
/// Consecutive character slots with neither copy readable before the character
/// phase is given up and searched for again.
const LOSS_LIMIT: u32 = 24;
/// Consecutive slots whose two copies both parsed and disagreed before the same
/// happens. Shorter than the loss limit because it is much stronger evidence: a
/// pair of legal codes that differ cannot both be right, and in the wrong phase
/// it is the ordinary case rather than the exception.
const DISAGREE_LIMIT: u32 = 6;

/// The state of one character slot, for the panel's quality readout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharSource {
    /// The leading copy was a legal code.
    Direct,
    /// It was not, and the repeat five slots later was.
    Repeat,
    /// Neither was: the character is lost and stands in the text as `*`.
    Lost,
}

/// A NAVTEX / SITOR-B receiver: audio in, text out.
pub struct NavtexRx {
    rate: f64,
    center_hz: f32,
    reverse: bool,

    // Front end.
    ph: f32,
    ph_inc: f32,
    lpf: ComplexFir,
    tone_ph: f32,
    tone_inc: f32,
    mark_int: Integrator,
    space_int: Integrator,
    ref_mark: f32,
    ref_space: f32,
    mark_idle: f32,
    space_idle: f32,
    mag: f32,
    mag_peak: f32,

    /// AFC: the residual tuning error it has found so far, folded into
    /// `ph_inc` — see [`NavtexRx::afc_offset_hz`].
    afc_hz: f32,
    afc_enabled: bool,
    /// The winning branch's integrator output last sample, and which branch won.
    /// Their phase difference is the residual frequency, already averaged over
    /// a bit, so no separate discriminator is needed and nothing has to be
    /// aligned against the bit decision — the same trick RTTY uses.
    prev_win: crate::Complex32,
    prev_win_mark: bool,
    have_win: bool,
    /// Samples the same branch has gone on winning, for [`AFC_SETTLE`].
    win_run: f32,

    // Bit clock.
    spb: f32,
    clk: f32,
    last_bit: bool,
    have_last: bool,
    /// The symbol being integrated for each bit phase: the signed sum, and the
    /// sum of magnitudes beside it, so a caller could read how open the eye was.
    acc: [f32; PHASES],
    acc_abs: [f32; PHASES],

    /// The two bit phases, half a symbol apart — see [`PHASES`].
    phases: [BitPhase; PHASES],
    /// Which of them is being read. `None` until one finds a character phase.
    active: Option<usize>,
}

/// One bit phase: the sliced bits, the character phase found in them, and the
/// text that comes out.
///
/// Two of these run side by side, half a symbol apart — see [`PHASES`].
struct BitPhase {
    /// Bit history, newest last. `true` is a mark bit.
    bits: VecDeque<bool>,
    /// Bits seen since this phase started, so a latched character phase can be
    /// held as an absolute position rather than an index into a moving window.
    seen: u64,
    /// Bit index of the next leading copy to decode, once in phase.
    next_at: Option<u64>,
    lost_run: u32,
    /// Bit index past the last slot already decoded into text, so a re-sync
    /// cannot read the same bits twice.
    emitted_to: u64,
    /// Consecutive slots whose two copies were both legal and different — see
    /// [`BitPhase::decode_slot`].
    disagree: u32,

    figures: bool,
    /// Quality of the last character slot decoded.
    last_source: CharSource,
    direct: u64,
    repaired: u64,
    lost: u64,
}

impl BitPhase {
    fn new() -> Self {
        BitPhase {
            bits: VecDeque::with_capacity(HISTORY_BITS + BITS),
            seen: 0,
            next_at: None,
            lost_run: 0,
            emitted_to: 0,
            disagree: 0,
            figures: false,
            last_source: CharSource::Direct,
            direct: 0,
            repaired: 0,
            lost: 0,
        }
    }

    fn in_sync(&self) -> bool {
        self.next_at.is_some()
    }

    /// Forget the character phase — the signal it was found in has gone.
    fn drop_sync(&mut self) {
        self.next_at = None;
        self.lost_run = 0;
        self.disagree = 0;
    }
}

impl NavtexRx {
    pub fn new(rate: f64) -> Self {
        let mut rx = NavtexRx {
            rate,
            center_hz: NAVTEX_CENTER_HZ,
            reverse: false,
            ph: 0.0,
            ph_inc: 0.0,
            // Wide enough for both tones, the keying sidebands of a 100-baud
            // square wave, and the whole AFC pull-in range: a band-pass that
            // clipped an off-tune signal's outer tone would be filtering away
            // the very thing AFC exists to find. The matched filters below do
            // the real noise rejection, so all this has to do is keep
            // neighbours out.
            lpf: ComplexFir::new(bandpass_taps(129, -front_end_hz(), front_end_hz(), rate)),
            tone_ph: 0.0,
            tone_inc: 0.0,
            mark_int: Integrator::new((rate / NAVTEX_BAUD) as usize),
            space_int: Integrator::new((rate / NAVTEX_BAUD) as usize),
            ref_mark: 0.0,
            ref_space: 0.0,
            mark_idle: 0.0,
            space_idle: 0.0,
            mag: 0.0,
            mag_peak: 0.0,
            afc_hz: 0.0,
            afc_enabled: true,
            prev_win: crate::Complex32::new(0.0, 0.0),
            prev_win_mark: false,
            have_win: false,
            win_run: 0.0,
            spb: (rate / NAVTEX_BAUD) as f32,
            clk: 0.0,
            last_bit: false,
            have_last: false,
            acc: [0.0; PHASES],
            acc_abs: [0.0; PHASES],
            phases: [BitPhase::new(), BitPhase::new()],
            active: None,
        };
        rx.retune();
        rx
    }

    /// Swap the sense of the tones, for a signal received on the other
    /// sideband. The same control RTTY calls Reverse, and needed for the same
    /// reason.
    pub fn set_reverse(&mut self, on: bool) {
        self.reverse = on;
    }

    #[must_use]
    pub fn reverse(&self) -> bool {
        self.reverse
    }

    fn retune(&mut self) {
        self.ph_inc = std::f32::consts::TAU * self.center_hz / self.rate as f32;
        self.tone_inc = std::f32::consts::TAU * (NAVTEX_SHIFT_HZ / 2.0) / self.rate as f32;
    }

    /// Fold the AFC pull into the mixer reference. Only the reference moves,
    /// never the tone separation: the filter is still centred on the tones, so
    /// a corrected signal is filtered exactly as a well-tuned one is.
    fn apply_afc(&mut self) {
        self.ph_inc = std::f32::consts::TAU * (self.center_hz + self.afc_hz) / self.rate as f32;
    }

    /// Move the expected audio centre. The shift is fixed by the standard, but
    /// the dial's idea of where that is moves with the cursor, so a retune
    /// drops the AFC pull: the loop then re-finds the signal from wherever the
    /// operator has just put it.
    pub fn set_center_hz(&mut self, hz: f32) {
        self.center_hz = hz;
        self.afc_hz = 0.0;
        self.retune();
    }

    /// Enable automatic frequency correction. On by default; turning it off
    /// pins the decoder to exactly where the operator put the cursor.
    pub fn set_afc(&mut self, on: bool) {
        self.afc_enabled = on;
        if !on {
            self.afc_hz = 0.0;
            self.retune();
        }
    }

    /// Current AFC pull in Hz — how far off the cursor the signal actually is.
    ///
    /// Read this before telling anyone to retune: on this mode the answer is
    /// usually that the dial is right and the expected figure is the thing to
    /// question.
    #[must_use]
    pub fn afc_offset_hz(&self) -> f32 {
        self.afc_hz
    }

    /// Signal level at the two tones, for a squelch or a meter.
    #[must_use]
    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    /// Whether either bit phase has the character phase latched.
    #[must_use]
    pub fn in_sync(&self) -> bool {
        self.phases.iter().any(BitPhase::in_sync)
    }

    /// How the last character was obtained.
    #[must_use]
    pub fn last_source(&self) -> CharSource {
        self.phases[self.active.unwrap_or(0)].last_source
    }

    /// `(taken directly, repaired from the repeat, lost)` since the receiver
    /// started — the only honest quality figure a mode with no CRC has.
    ///
    /// From the phase being read: the other one is decoding the same signal off
    /// the wrong grid, and counting its failures would report a receiver that
    /// is working perfectly as one losing half of everything.
    #[must_use]
    pub fn counts(&self) -> (u64, u64, u64) {
        let p = &self.phases[self.active.unwrap_or(0)];
        (p.direct, p.repaired, p.lost)
    }

    /// Feed audio; returns whatever text it completed.
    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            let z = Complex32::new(a * self.ph.cos(), -a * self.ph.sin());
            self.ph += self.ph_inc;
            if self.ph > std::f32::consts::TAU {
                self.ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        for z in bb {
            let (s, c) = (self.tone_ph.sin(), self.tone_ph.cos());
            self.tone_ph += self.tone_inc;
            if self.tone_ph > std::f32::consts::TAU {
                self.tone_ph -= std::f32::consts::TAU;
            }
            let m_c = self.mark_int.push(z * Complex32::new(c, -s));
            let sp_c = self.space_int.push(z * Complex32::new(c, s));
            let m = m_c.norm();
            let sp = sp_c.norm();
            let total = m + sp;
            self.mag += 0.02 * (total - self.mag);
            // A slow peak to measure the level against. Rises at once and
            // falls over a second or two, so a station that has stopped is
            // recognisable as one — see `NavtexRx::strobe`.
            self.mag_peak = self.mag_peak.max(self.mag) * MAG_PEAK_DECAY;

            // Seed the decision references, and re-seed them when a signal
            // returns after a silence has decayed them away. A NAVTEX slot is
            // ten minutes of nothing and then a station, so this is not an edge
            // case — it is every transmission after the first. Left to decay,
            // the references reach the denormal floor and the ratios below come
            // back as noise: the receiver decoded the first broadcast of a
            // session and nothing after it.
            if total > 1e-12 && (self.ref_mark + self.ref_space) < total * 0.05 {
                self.ref_mark = total * 0.5;
                self.ref_space = total * 0.5;
            }
            // Each branch against its own recent level rather than against the
            // other, so a selective fade that leaves one tone well down still
            // decodes — the same rule RTTY uses, and on 518 kHz at night it is
            // the difference between a message and a page of stars.
            let (dm, ds) = if self.ref_mark > 1e-20 && self.ref_space > 1e-20 {
                (m / self.ref_mark, sp / self.ref_space)
            } else {
                (m, sp)
            };
            let phys_mark = dm > ds;
            let idle_limit = ATC_IDLE_BITS * self.spb;
            if phys_mark {
                self.ref_mark += ATC_SMOOTH * (m - self.ref_mark);
                self.space_idle += 1.0;
                self.mark_idle = 0.0;
                if self.space_idle > idle_limit {
                    self.ref_space += ATC_RELAX * (self.ref_mark - self.ref_space);
                }
            } else {
                self.ref_space += ATC_SMOOTH * (sp - self.ref_space);
                self.mark_idle += 1.0;
                self.space_idle = 0.0;
                if self.mark_idle > idle_limit {
                    self.ref_mark += ATC_RELAX * (self.ref_space - self.ref_mark);
                }
            }
            // Keep the two references in proportion: a branch that has been
            // idle for a long time must not sit so far below the other that
            // its first sample of signal reads as a hundred times the level it
            // is. The same floor the RTTY receiver keeps, for the same reason.
            let floor = self.ref_mark.max(self.ref_space) * ATC_MIN_RATIO;
            self.ref_mark = self.ref_mark.max(floor);
            self.ref_space = self.ref_space.max(floor);

            // AFC. The winning branch's integrator output is a phasor turning
            // at exactly the residual tuning error, already averaged over a
            // bit, so its phase advance *is* the error in Hz. Same measurement,
            // and the same reason for the settle and separation gates, as
            // RTTY's — see the constants.
            if self.have_win && phys_mark == self.prev_win_mark {
                self.win_run += 1.0;
            } else {
                self.win_run = 0.0;
            }
            let win = if phys_mark { m_c } else { sp_c };
            if self.afc_enabled
                && (dm - ds).abs() > AFC_SEP_MIN
                && self.win_run > AFC_SETTLE * self.spb
            {
                let d = win * self.prev_win.conj();
                if d.norm_sqr() > 1e-24 {
                    let hz_per_rad = self.rate as f32 / std::f32::consts::TAU;
                    let err = (d.arg() * hz_per_rad).clamp(-AFC_LIMIT_HZ, AFC_LIMIT_HZ);
                    let was = self.afc_hz;
                    self.afc_hz = (self.afc_hz + AFC_GAIN * err).clamp(-AFC_LIMIT_HZ, AFC_LIMIT_HZ);
                    if self.afc_hz != was {
                        self.apply_afc();
                    }
                }
            }
            // Tracked whether or not the loop is running, so turning AFC back on
            // starts from a current phasor instead of one from before it was
            // switched off.
            self.prev_win = win;
            self.prev_win_mark = phys_mark;
            self.have_win = true;

            self.advance_clock(dm - ds, phys_mark, &mut out);
        }
        out
    }

    /// One sample of the bit clock.
    ///
    /// Synchronous: there is no start bit to acquire on, so the clock
    /// free-runs and every observed transition nudges it towards putting the
    /// strobe where a moving-average matched filter has finished a symbol — a
    /// half-bit after the crossover it just saw.
    ///
    /// The decision is integrated over the whole symbol rather than taken at
    /// the strobe, and the same sum is what measures the eye: see [`EYE_OPEN`]
    /// for the half-bit ambiguity that measurement exists to break.
    fn advance_clock(&mut self, d: f32, mark: bool, out: &mut String) {
        for p in 0..PHASES {
            self.acc[p] += d;
            self.acc_abs[p] += d.abs();
        }
        if self.have_last && mark != self.last_bit {
            let mut e = 0.5 - self.clk;
            if e > 0.5 {
                e -= 1.0;
            } else if e < -0.5 {
                e += 1.0;
            }
            self.clk = (self.clk + CLOCK_TRACK_GAIN * e).rem_euclid(1.0);
        }
        self.last_bit = mark;
        self.have_last = true;

        let before = self.clk;
        self.clk += 1.0 / self.spb;
        // Phase 1 strobes half a symbol after phase 0, so its symbol window is
        // the one phase 0 straddles and the other way about — see [`PHASES`].
        if before < 0.5 && self.clk >= 0.5 {
            self.strobe(1, out);
        }
        if self.clk >= 1.0 {
            self.clk -= 1.0;
            self.strobe(0, out);
        }
    }

    /// Slice one symbol on bit phase `p` and hand it to that phase's framer.
    ///
    /// Only the phase that has found a character phase is read; the other is
    /// still decoded, because it is what the decoder falls back on when the
    /// signal restarts and the clock lands on the other lock point.
    fn strobe(&mut self, p: usize, out: &mut String) {
        // A signal that has gone takes its character phase with it. The gap
        // between two NAVTEX slots is minutes, and the transmission after it
        // shares no bit alignment with the one before — a phase held across the
        // gap reads the whole of the next message off the wrong grid, and the
        // loss counter alone takes tens of characters to notice. Those
        // characters are the header.
        if self.mag < self.mag_peak * SIGNAL_GONE {
            self.phases[p].drop_sync();
        }
        let bit = (self.acc[p] > 0.0) != self.reverse;
        self.acc[p] = 0.0;
        self.acc_abs[p] = 0.0;
        let mut text = String::new();
        self.phases[p].push_bit(bit, &mut text);
        // Whichever has a character phase is the one that is read. The one
        // already being read keeps it, so a moment where both are in sync does
        // not swap the stream mid-message.
        match self.active {
            Some(a) if self.phases[a].in_sync() => {
                if a == p {
                    out.push_str(&text);
                }
            }
            _ => {
                self.active = (0..PHASES).find(|&i| self.phases[i].in_sync());
                if self.active == Some(p) {
                    out.push_str(&text);
                }
            }
        }
    }
}

impl BitPhase {
    /// One decided bit into the history, and whatever that completes.
    fn push_bit(&mut self, bit: bool, out: &mut String) {
        self.bits.push_back(bit);
        self.seen += 1;
        while self.bits.len() > HISTORY_BITS {
            self.bits.pop_front();
        }
        // Index of the oldest bit still held.
        let base = self.seen - self.bits.len() as u64;

        match self.next_at {
            None => {
                if let Some(at) = self.find_phase(base) {
                    // Catch up over everything the window already holds. The
                    // search cannot answer until it has a window's worth of
                    // bits, and a transmission's first characters are inside
                    // that window — a decoder that started from `now` would
                    // eat the `ZCZC` and the message's own serial number every
                    // time.
                    // …but never back over bits that have already been turned
                    // into text. A re-sync — a transmission ending and another
                    // starting, which share no phase — would otherwise read the
                    // window a second time and print the last few characters
                    // twice, which puts a second `ZCZC` in the stream and
                    // frames a message out of the middle of one.
                    let mut at = at;
                    while at < self.emitted_to {
                        at += PAIR_BITS as u64;
                    }
                    while at + FEC_BITS as u64 + BITS as u64 <= self.seen {
                        if let Some(ch) = self.decode_slot(at, base) {
                            out.push(ch);
                        }
                        at += PAIR_BITS as u64;
                    }
                    self.next_at = Some(at);
                    self.emitted_to = at;
                    self.lost_run = 0;
                }
            }
            Some(at) => {
                // Decodable once the repeat has arrived, which is `FEC_BITS`
                // after the leading copy plus the character itself.
                if self.seen >= at + FEC_BITS as u64 + BITS as u64 {
                    if let Some(ch) = self.decode_slot(at, base) {
                        out.push(ch);
                    }
                    self.next_at = Some(at + PAIR_BITS as u64);
                    self.emitted_to = at + PAIR_BITS as u64;
                    if self.lost_run >= LOSS_LIMIT || self.disagree >= DISAGREE_LIMIT {
                        self.next_at = None;
                        self.lost_run = 0;
                        self.disagree = 0;
                    }
                }
            }
        }
    }

    /// The seven bits at absolute index `at` as a code, if they are all held.
    fn code_at(&self, at: u64, base: u64) -> Option<u8> {
        if at < base || at + BITS as u64 > base + self.bits.len() as u64 {
            return None;
        }
        let off = (at - base) as usize;
        let mut code = 0u8;
        for i in 0..BITS {
            // Least significant bit first in time, which is how the standard
            // sends it and how every other decoder reads it.
            if self.bits[off + i] {
                code |= 1 << i;
            }
        }
        Some(code)
    }

    /// One character slot: the leading copy, or its repeat, or neither.
    fn decode_slot(&mut self, at: u64, base: u64) -> Option<char> {
        let dx = self.code_at(at, base).filter(|&c| is_valid_code(c));
        let rx = self.code_at(at + FEC_BITS as u64, base).filter(|&c| is_valid_code(c));
        // Two legal codes that are not the same is not a fade — at most one of
        // them can be right. In the *wrong* phase it happens on nearly every
        // slot, which is what makes it the signal that the phase has to be
        // searched for again: a transmission that stops and another that starts
        // share no phase, and without this the receiver stays latched on the
        // old one until the loss counter runs out.
        //
        // The phasing pair is exempt. A transmitter idling sends alpha and rep
        // on alternating slots, so a slot and the one five later legitimately
        // disagree for as long as the idle lasts.
        let phasing = |c: u8| matches!(c, CODE_ALPHA | CODE_BETA | CODE_REP);
        if let (Some(d), Some(r)) = (dx, rx)
            && d != r
            && !phasing(d)
            && !phasing(r)
        {
            self.disagree += 1;
        } else {
            self.disagree = 0;
        }
        let (code, source) = match (dx, rx) {
            (Some(c), _) => (Some(c), CharSource::Direct),
            (None, Some(c)) => (Some(c), CharSource::Repeat),
            (None, None) => (None, CharSource::Lost),
        };
        self.last_source = source;
        match source {
            CharSource::Direct => {
                self.direct += 1;
                self.lost_run = 0;
            }
            CharSource::Repeat => {
                self.repaired += 1;
                self.lost_run = 0;
            }
            CharSource::Lost => {
                self.lost += 1;
                self.lost_run += 1;
            }
        }
        let Some(code) = code else {
            // A hole, marked rather than hidden: a warning with a character
            // missing has to look like one.
            return Some('*');
        };
        match code {
            CODE_LTRS => {
                self.figures = false;
                None
            }
            CODE_FIGS => {
                self.figures = true;
                None
            }
            // The idle and phasing codes carry no text. `CHAR32` is the
            // repetition signal and `BETA` the other half of the phasing pair;
            // neither prints.
            CODE_ALPHA | CODE_BETA | CODE_REP | CODE_CHAR32 => None,
            _ => code_to_char(code, self.figures),
        }
    }

    /// Find the character phase in the bits held.
    ///
    /// Fourteen candidates, not seven: the phase has to say which slot is a
    /// leading copy and which is a repeat as well as where a character starts,
    /// and those are two bits of information. A wrong offset scores near zero
    /// because a constant-ratio code read off the boundary is almost never
    /// legal — that is the property the search is built on, and it is why no
    /// preamble hunt is needed.
    ///
    /// Returns the absolute index of the next leading copy to decode.
    fn find_phase(&self, base: u64) -> Option<u64> {
        let held = self.bits.len();
        if held < HISTORY_BITS {
            return None;
        }
        let mut best: Option<(i32, u64)> = None;
        for off in 0..PAIR_BITS {
            let mut score = 0i32;
            let mut pairs = 0i32;
            let mut at = off;
            while at + FEC_BITS + BITS <= held {
                let dx = self.code_at(base + at as u64, base);
                let rx = self.code_at(base + (at + FEC_BITS) as u64, base);
                match (dx, rx) {
                    (Some(d), Some(r)) => {
                        if is_valid_code(d) {
                            score += 1;
                        }
                        if is_valid_code(d) && d == r {
                            // The pair agreeing is what tells a leading copy
                            // from a repeat, and so which half of the fourteen
                            // this is.
                            pairs += 1;
                        }
                    }
                    _ => break,
                }
                at += PAIR_BITS;
            }
            let total = score + pairs;
            if pairs >= 2 && total >= SYNC_SCORE && best.is_none_or(|(b, _)| total > b) {
                best = Some((total, base + off as u64));
            }
        }
        // The *earliest* slot of the winning phase, not the newest: the caller
        // decodes forward from here, which is what recovers the characters
        // already in the window.
        best.map(|(_, at)| at)
    }
}

/// Render text as a SITOR-B bit stream, for tests and for anyone building a
/// signal to check a receiver against.
///
/// The arrangement is the one the receiver's phase search is looking for: a
/// leading copy in every even slot and, five slots later, its repeat. Slots
/// with nothing to repeat carry the alternating phasing pair, which is what a
/// transmitter idles with.
#[must_use]
pub fn encode_bits(text: &str) -> Vec<bool> {
    // The character stream, with shifts inserted where the register changes.
    let mut codes: Vec<u8> = Vec::new();
    let mut figures = false;
    for ch in text.chars() {
        let shared = matches!(ch, ' ' | '\r' | '\n');
        if let Some(c) = figure_to_code(ch).filter(|_| !shared && char_to_code(ch).is_none()) {
            if !figures {
                codes.push(CODE_FIGS);
                figures = true;
            }
            codes.push(c);
        } else if let Some(c) = char_to_code(ch) {
            if figures && !shared {
                codes.push(CODE_LTRS);
                figures = false;
            }
            codes.push(c);
        }
    }
    // Idle in front so a receiver has something to run its clock on, and
    // behind so the last characters' repeats are actually sent.
    let lead = 20usize;
    let total = lead + 2 * codes.len() + FEC_CHARS + 8;
    let mut slots: Vec<Option<u8>> = vec![None; total];
    for (k, &code) in codes.iter().enumerate() {
        slots[lead + 2 * k] = Some(code);
        slots[lead + 2 * k + FEC_CHARS] = Some(code);
    }
    let mut bits = Vec::with_capacity(total * BITS);
    for (i, slot) in slots.iter().enumerate() {
        // The phasing pair on anything unfilled: alpha and rep alternating,
        // which is what makes the idle stream *not* look like a message to the
        // phase search — no slot agrees with the one five later.
        let code = slot.unwrap_or(if i % 2 == 0 { CODE_REP } else { CODE_ALPHA });
        for b in 0..BITS {
            bits.push(code & (1 << b) != 0);
        }
    }
    bits
}

/// An FSK audio signal carrying `bits` at the NAVTEX rate and shift.
///
/// A mark bit is the **upper** tone. ITU-R M.625 sends the code's B elements —
/// the ones — on the higher frequency, and a receiver in upper sideband keeps
/// that order, so a signal built here decodes with Reverse off. That is also
/// the reasoning behind the default; it has not been checked against a real
/// coast station.
#[must_use]
pub fn synth(bits: &[bool], rate: f64, center_hz: f32, amplitude: f32) -> Vec<f32> {
    let spb = (rate / NAVTEX_BAUD) as usize;
    let mut out = Vec::with_capacity(bits.len() * spb);
    let mut ph = 0.0f32;
    for &b in bits {
        let hz = center_hz + if b { NAVTEX_SHIFT_HZ / 2.0 } else { -NAVTEX_SHIFT_HZ / 2.0 };
        let inc = std::f32::consts::TAU * hz / rate as f32;
        for _ in 0..spb {
            out.push(ph.cos() * amplitude);
            ph += inc;
            if ph > std::f32::consts::TAU {
                ph -= std::f32::consts::TAU;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The alphabet is what makes the mode self-checking: every legal code has
    /// four mark bits of seven, all thirty-five such patterns are accounted
    /// for, and none is claimed twice.
    #[test]
    fn the_alphabet_is_a_constant_ratio_code() {
        let controls = [CODE_LTRS, CODE_FIGS, CODE_ALPHA, CODE_BETA, CODE_REP, CODE_CHAR32];
        let mut seen: Vec<u8> = ALPHABET.iter().map(|&(c, _, _)| c).collect();
        seen.extend_from_slice(&controls);
        for &c in &seen {
            assert!(is_valid_code(c), "0x{c:02x} is not a four-of-seven code");
        }
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "a code is used twice");
        // C(7,4) = 35: the alphabet uses the whole space, which is why an
        // unlisted valid code cannot exist.
        assert_eq!(seen.len(), 35);
        assert_eq!((0u8..128).filter(|&c| is_valid_code(c)).count(), 35);
    }

    /// A transmission that starts after a spell of silence decodes: a real slot
    /// is ten minutes of nothing and then a station, and a receiver that only
    /// worked from the first sample would never decode anything at all.
    #[test]
    fn a_signal_that_starts_after_silence_decodes() {
        let rate = 8000.0;
        let msg = "ZCZC FA12 GALE WARNING NNNN";
        let mut audio = vec![0.0f32; 8000];
        audio.extend(synth(&encode_bits(msg), rate, NAVTEX_CENTER_HZ, 0.4));
        let mut rx = NavtexRx::new(rate);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(
            got.contains("GALE WARNING"),
            "decoded {got:?} sync={} counts={:?}",
            rx.in_sync(),
            rx.counts()
        );
    }

    /// Two slots with silence between them: a coast station transmits for ten
    /// minutes every four hours and its neighbours fill the gaps, so a receiver
    /// left on hears exactly this. The second must decode as well as the first.
    #[test]
    fn a_second_transmission_after_a_gap_decodes_too() {
        let rate = 8000.0;
        let mut audio = vec![0.0f32; 8000];
        audio.extend(synth(&encode_bits("ZCZC PA01 ONE NNNN"), rate, NAVTEX_CENTER_HZ, 0.4));
        audio.extend(std::iter::repeat_n(0.0f32, 8000));
        audio.extend(synth(&encode_bits("ZCZC PA02 TWO NNNN"), rate, NAVTEX_CENTER_HZ, 0.4));
        let mut rx = NavtexRx::new(rate);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(got.contains("ZCZC PA01 ONE NNNN"), "first: {got:?}");
        assert!(got.contains("ZCZC PA02 TWO NNNN"), "second: {got:?}");
    }

    /// A clean signal decodes to the text that was sent.
    #[test]
    fn a_clean_broadcast_decodes() {
        let rate = 8000.0;
        let msg = "ZCZC FA12\r\nGALE WARNING\r\nNNNN";
        let bits = encode_bits(msg);
        let audio = synth(&bits, rate, NAVTEX_CENTER_HZ, 0.4);
        let mut rx = NavtexRx::new(rate);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(rx.in_sync(), "the phase was never found");
        assert!(got.contains("ZCZC FA12"), "decoded {got:?}");
        assert!(got.contains("GALE WARNING"), "decoded {got:?}");
        assert!(got.contains("NNNN"), "decoded {got:?}");
        let (_, _, lost) = rx.counts();
        assert_eq!(lost, 0, "a clean signal lost {lost} characters");
    }

    /// …and it still decodes with the noise a 518 kHz signal actually arrives
    /// under. Not a sensitivity claim — a synthetic signal cannot make one
    /// (see the DSP notes in `validate-dsp-against-real-signals`) — but a
    /// decoder that only works on a noiseless tone is not one.
    #[test]
    fn noise_does_not_stop_it() {
        let rate = 8000.0;
        let msg = "ZCZC PB07 NAVAREA ONE";
        let bits = encode_bits(msg);
        let mut audio = synth(&bits, rate, NAVTEX_CENTER_HZ, 0.4);
        // A deterministic pseudo-noise at a third of the signal's amplitude.
        let mut x = 0x2545_f491_4f6c_dd1du64;
        for a in &mut audio {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *a += ((x >> 40) as f32 / 8_388_608.0 - 1.0) * 0.13;
        }
        let mut rx = NavtexRx::new(rate);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(got.contains("NAVAREA ONE"), "decoded {got:?}");
    }

    /// The time diversity is the whole point: a burst that destroys the
    /// leading copy of a run of characters still comes back readable, because
    /// each of them is sent again five slots later.
    #[test]
    fn a_burst_over_the_leading_copies_is_repaired_from_the_repeats() {
        let rate = 8000.0;
        let msg = "ZCZC GB34 SECURITE";
        let mut bits = encode_bits(msg);
        // Corrupt one bit in every other character slot — the leading copies —
        // right through the message. A constant-ratio code cannot absorb a
        // single bit error, so every one of these is a hole the repeat has to
        // fill.
        let mut hit = 0;
        let mut slot = 0;
        while (slot + 1) * BITS <= bits.len() {
            if slot % 2 == 0 && slot > 40 && slot < 70 {
                let at = slot * BITS + 3;
                bits[at] = !bits[at];
                hit += 1;
            }
            slot += 1;
        }
        assert!(hit > 5, "the test corrupted nothing");
        let audio = synth(&bits, rate, NAVTEX_CENTER_HZ, 0.4);
        let mut rx = NavtexRx::new(rate);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(got.contains("SECURITE"), "decoded {got:?}");
        let (_, repaired, lost) = rx.counts();
        assert!(repaired > 0, "nothing came from the repeats, so nothing was being tested");
        assert_eq!(lost, 0, "{lost} characters were lost that the repeat should have covered");
    }

    /// A receiver on the other sideband hears the tones swapped, and Reverse
    /// is what puts them back — the same control, and the same failure, as
    /// RTTY.
    #[test]
    fn the_reverse_control_recovers_an_inverted_signal() {
        let rate = 8000.0;
        let msg = "ZCZC ZZ01 TEST";
        let bits: Vec<bool> = encode_bits(msg).into_iter().map(|b| !b).collect();
        let audio = synth(&bits, rate, NAVTEX_CENTER_HZ, 0.4);

        let mut plain = NavtexRx::new(rate);
        let mut wrong = String::new();
        for chunk in audio.chunks(512) {
            wrong.push_str(&plain.process(chunk));
        }
        assert!(!wrong.contains("TEST"), "an inverted signal decoded anyway: {wrong:?}");

        let mut rx = NavtexRx::new(rate);
        rx.set_reverse(true);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(got.contains("ZCZC ZZ01 TEST"), "decoded {got:?}");
    }

    /// Figures come back as figures: a warning is mostly positions and times,
    /// and a shift missed turns 51-03N into QA-ZDN.
    #[test]
    fn the_figure_shift_survives_the_round_trip() {
        let rate = 8000.0;
        let msg = "ZCZC OA01 5103N 00109E AT 1200 UTC";
        let bits = encode_bits(msg);
        let audio = synth(&bits, rate, NAVTEX_CENTER_HZ, 0.4);
        let mut rx = NavtexRx::new(rate);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(got.contains("5103N 00109E AT 1200 UTC"), "decoded {got:?}");
    }

    /// The dial is a couple of hundred hertz off the tones and the message
    /// still arrives whole. This is the whole of issue #608, reported as a
    /// strong steady signal decoding to mostly asterisks while fldigi read the
    /// same signal cleanly: every published NAVTEX tuning figure rounds
    /// differently, and a matched-filter detector with no frequency tracking
    /// decodes the rounding error rather than the message.
    #[test]
    fn a_mistuned_dial_is_tracked_rather_than_decoded() {
        let rate = 8000.0;
        let msg = "ZCZC FA12 GALE WARNING NNNN WIND SPEED 55 KTS";
        let bits = encode_bits(msg);
        // Both directions, and inside the pull-in range at each end.
        for off in [-300.0f32, -150.0, 150.0, 300.0] {
            let audio = synth(&bits, rate, NAVTEX_CENTER_HZ + off, 0.4);
            let mut rx = NavtexRx::new(rate);
            let mut got = String::new();
            for chunk in audio.chunks(512) {
                got.push_str(&rx.process(chunk));
            }
            assert!(
                got.contains("GALE WARNING") && got.contains("WIND SPEED 55 KTS"),
                "off by {off} Hz the message was lost: {got:?}"
            );
            assert!(
                (rx.afc_offset_hz() - off).abs() < 30.0,
                "AFC found {:?} Hz for a signal {off} Hz off",
                rx.afc_offset_hz()
            );
        }
    }

    /// The loop reads the signal, not the clock: with it off the same mistuned
    /// signal is unreadable, which is what makes the test above a measurement
    /// of the AFC rather than of the widened filter.
    #[test]
    fn without_the_loop_the_same_mistuned_signal_is_lost() {
        let rate = 8000.0;
        let bits = encode_bits("ZCZC FA12 GALE WARNING NNNN");
        let audio = synth(&bits, rate, NAVTEX_CENTER_HZ + 300.0, 0.4);
        let mut rx = NavtexRx::new(rate);
        rx.set_afc(false);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(
            !got.contains("GALE WARNING"),
            "a signal 300 Hz off decoded with no frequency tracking: {got:?}"
        );
        assert_eq!(rx.afc_offset_hz(), 0.0, "the loop moved with AFC off");
    }

    /// The wider front end is a price paid for the pull-in range, and a wider
    /// filter is more noise in principle. Measured against the pre-fix front
    /// end on the same noisy signals — 60 decodes each at 15, 12, 9 and 6 dB —
    /// it costs nothing: the message count is 60/48/35/29 against 59/55/37/30,
    /// so the wide one is ahead at every step but one and never behind by more
    /// than a single decode. Pinned as a floor rather than as the A/B it was
    /// measured from, because sixteen noisy decodes will not tie and the
    /// margin this leaves is wider than the difference it guards.
    #[test]
    fn the_wider_front_end_has_not_cost_the_tuned_case_its_sensitivity() {
        let rate = 8000.0;
        let bits = encode_bits("ZCZC PB07 NAVAREA ONE WIND 55 KTS");
        // Twelve decibels in band-to-band ratio, above the point where the mode
        // is used in anger.
        let noise = 0.4 * 10.0f32.powf(-12.0 / 20.0);
        let mut decoded = 0;
        for seed in 0..24u64 {
            let mut audio = synth(&bits, rate, NAVTEX_CENTER_HZ, 0.4);
            let mut x =
                0x2545_f491_4f6c_dd1du64.wrapping_add(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
            for a in &mut audio {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                *a += ((x >> 40) as f32 / 8_388_608.0 - 1.0) * noise;
            }
            let mut rx = NavtexRx::new(rate);
            let mut got = String::new();
            for chunk in audio.chunks(512) {
                got.push_str(&rx.process(chunk));
            }
            if got.contains("NAVAREA ONE") {
                decoded += 1;
            }
        }
        assert!(
            decoded >= 21,
            "only {decoded} of 24 decodes at 12 dB — the wider front end cost the tuned case"
        );
    }

    /// Moving the cursor drops the pull, so a retune cannot leave the loop
    /// holding a correction for a frequency the operator has just changed. The
    /// loop then re-finds the signal from where the cursor now is.
    #[test]
    fn a_retune_drops_the_pull_and_the_loop_re_finds_the_signal() {
        let rate = 8000.0;
        let bits = encode_bits("ZCZC FA12 GALE WARNING NNNN WIND SPEED 55 KTS");
        let audio = synth(&bits, rate, NAVTEX_CENTER_HZ + 250.0, 0.4);
        let mut rx = NavtexRx::new(rate);
        let split = (audio.len() * 3 / 4) / 512 * 512;
        let mut got = String::new();
        for chunk in audio[..split].chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(rx.afc_offset_hz() > 200.0, "the loop had not pulled in yet");

        rx.set_center_hz(NAVTEX_CENTER_HZ);
        assert_eq!(rx.afc_offset_hz(), 0.0, "a retune kept the old pull");
        for chunk in audio[split..].chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(got.contains("GALE WARNING"), "lost across a retune: {got:?}");
        // The signal is 250 Hz off the new centre, so it must have found it again.
        assert!(
            rx.afc_offset_hz() > 200.0,
            "the loop did not re-lock after the retune: {:?}",
            rx.afc_offset_hz()
        );
    }

    /// The pull-in range is bounded, and the front-end filter is wide enough to
    /// hold a signal at the edge of it — those two numbers are the same design
    /// decision, so pin them together rather than letting one drift.
    #[test]
    fn the_front_end_covers_the_pull_in_range() {
        assert!(front_end_hz() > f64::from(AFC_LIMIT_HZ + NAVTEX_SHIFT_HZ));
    }
}
