//! RDS/RBDS: the data subcarrier on a broadcast FM multiplex.
//!
//! The composite coming out of the WFM discriminator carries, above the stereo
//! difference channel, a suppressed-carrier 57 kHz subcarrier with 1187.5 bps of
//! data on it. 57 kHz is exactly three times the 19 kHz stereo pilot, and the
//! standard requires the two to be locked — which is worth something here, since
//! the pilot is an un-suppressed tone at 10 % injection while the data
//! subcarrier is suppressed at 2–4 %.
//!
//! ```text
//! mpx ──► DDC (mix 57 kHz → DC, decimate to ~15 kHz)
//!      ──► biphase matched filter, Costas carrier, Gardner timing
//!      ──► differential detection ──► bits
//!      ──► 26-bit blocks, (26,16) syndrome + offset words ──► groups
//!      ──► group parsing ──► RdsData
//! ```
//!
//! **Carrier recovery is independent of the pilot**, and only *aided* by it. The
//! pilot's tracked frequency, tripled, retunes the down-converter — which takes
//! the station's and our own clock error out of the Costas loop's job and lets it
//! stay narrow. Its phase is deliberately not used: stations transmit RDS without
//! a stereo pilot, and the standard permits the subcarrier to sit in phase *or in
//! quadrature* with the pilot's third harmonic, so a phase-derived reference
//! needs an ambiguity resolver and a second code path for the unaided case. One
//! path that always works beats two that each work sometimes.
//!
//! **The line code does half the work for free.** RDS differentially encodes
//! before biphase-shaping, so differential detection — the same `d = curr ·
//! conj(prev)` the PSK31 receiver uses — recovers the data bits directly and the
//! Costas loop's 180° ambiguity never has to be resolved.
//!
//! Everything downstream is bit-exact and self-checking: a block whose syndrome
//! does not match one of the five offset words is not a block, so a decoder
//! parked on noise stays silent rather than inventing a station.

use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;

use sdroxide_types::{RdsClock, RdsData, RdsGroupLog, RtPlus, af_code_hz};

use crate::Complex32;
use crate::ddc::Ddc;
use crate::fir::{ComplexFir, bandpass_taps};
use crate::psk::loop_gains;

/// Below this channel rate the 57 kHz subcarrier and its ±2.4 kHz of sidebands
/// do not survive the DDC ahead of the discriminator, so no decoder is built.
/// Sits above the bare Nyquist requirement (~120 kHz) for the same reason
/// `WFM_STEREO_MIN_RATE` does: at the edge the anti-alias skirt is already eating
/// the thing being decoded.
pub const RDS_MIN_RATE: f64 = 140_000.0;

const CARRIER_HZ: f64 = 57_000.0;
/// 57000/48, exactly, by construction of the standard.
const BITRATE: f64 = 1_187.5;

/// Target rate for the bit-level work. Lands 15–16 kHz for every WFM channel
/// rate the DDC produces, i.e. 12–14 samples per symbol — enough for the timing
/// loop to interpolate against, cheap enough to be free.
const WORK_TARGET: f64 = 15_000.0;

/// Half-bandwidth of the data signal, and so of the channel filter below.
///
/// The standard shapes the biphase data to nothing beyond this, so anything
/// further out is somebody else's. It is worth filtering explicitly rather than
/// leaving it to the down-converter: the DDC's anti-alias filter only has to hold
/// back what would fold at the *work rate*, which leaves ±7.5 kHz open — three
/// times wider than the signal, and every dB of that extra width is distortion
/// and noise handed straight to the carrier loop.
const DATA_BW_HZ: f64 = 2_400.0;
/// Taps for that filter. At the ~15 kHz work rate this puts the transition inside
/// 1 kHz, which is enough to be well down by the time the neighbouring subcarrier
/// slots begin, and 63 multiplies at 15 kHz costs nothing.
const CHANNEL_TAPS: usize = 63;

// ---------------------------------------------------------------------------
// The (26,16) block code
// ---------------------------------------------------------------------------

/// g(x) = x¹⁰ + x⁸ + x⁷ + x⁵ + x⁴ + x³ + 1, as its eleven coefficients.
const POLY: u32 = 0b101_1011_1001;

/// The five offset words, in the order they appear in a group: A, B, C, D, and
/// C′ — which stands in for C in a version-B group, where block three carries the
/// programme identification again instead of group data.
const OFFSET_A: u16 = 0b00_1111_1100;
const OFFSET_B: u16 = 0b01_1001_1000;
const OFFSET_C: u16 = 0b01_0110_1000;
const OFFSET_CP: u16 = 0b11_0101_0000;
const OFFSET_D: u16 = 0b01_1011_0100;

/// Every offset word, for the hunt that has no expectation to check against.
const OFFSETS: [(u16, usize); 5] =
    [(OFFSET_A, 0), (OFFSET_B, 1), (OFFSET_C, 2), (OFFSET_CP, 2), (OFFSET_D, 3)];

/// The offset word expected at each position in a group. Position 2 accepts
/// either C or C′.
fn expected(pos: usize) -> [u16; 2] {
    match pos {
        0 => [OFFSET_A, OFFSET_A],
        1 => [OFFSET_B, OFFSET_B],
        2 => [OFFSET_C, OFFSET_CP],
        _ => [OFFSET_D, OFFSET_D],
    }
}

/// Remainder of a received 26-bit block divided by g(x).
///
/// For a codeword with an offset word added to its check bits this comes out as
/// the offset word itself: the codeword divides cleanly by construction, and the
/// offset has degree 9, below the divisor's, so it is its own remainder. Which is
/// why the tables below compare syndromes against offset words directly rather
/// than against a second set of constants.
fn syndrome(block: u32) -> u16 {
    let mut reg = 0u32;
    for i in (0..26).rev() {
        reg = (reg << 1) | ((block >> i) & 1);
        if reg & (1 << 10) != 0 {
            reg ^= POLY;
        }
    }
    (reg & 0x3ff) as u16
}

/// Longest error burst the corrector will attempt.
///
/// The code's stated capability is five, and five is genuinely achievable — every
/// burst up to that length has a syndrome of its own, which a test asserts. Using
/// the full capability is still wrong, and the reason is worth writing down,
/// because "the standard says five" is how it gets set back.
///
/// A corrector that accepts *n* of the 1024 syndromes will accept that fraction
/// of pure noise as a valid block, and the only way this decoder ever notices it
/// has lost a station is a run of [`MAX_BAD_RUN`] blocks that do not check out:
///
/// | burst | syndromes used | accepted from noise | P(24 in a row fail) |
/// |------:|---------------:|--------------------:|--------------------:|
/// | 5 | 368 | 36 % | 1 in 60 000 |
/// | 4 | 192 | 19 % | 1 in 160 |
/// | 3 | 100 | 9.8 % | 1 in 12 |
/// | **2** | **52** | **5.1 %** | **1 in 3.5** |
/// | 1 | 27 | 2.6 % | 1 in 1.9 |
/// | 0 | 1 | 0.1 % | ~certain |
///
/// At five the decoder would hold "sync" on a dead frequency indefinitely and
/// pour invented groups into the display. Two keeps the corrections that matter —
/// single bit errors and adjacent pairs, which is most of what a marginal signal
/// produces — while leaving the detector enough syndrome space to notice the
/// station has gone.
///
/// None of this makes the offset words mutually distinguishable under correction;
/// no burst length does, C and D being one single-bit syndrome apart. That is why
/// the hunt for sync never corrects at all — see [`BlockSync::push`].
const MAX_BURST: usize = 2;

/// Syndrome → error pattern, for every burst of up to [`MAX_BURST`] bits.
///
/// A burst is a run of bit positions whose first and last are in error and whose
/// interior is anything; there are 16 such patterns per starting position. Built
/// once and shared, the way the CTCSS decoder's Golay tables are.
fn burst_table() -> &'static HashMap<u16, u32> {
    static TABLE: OnceLock<HashMap<u16, u32>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map: HashMap<u16, u32> = HashMap::new();
        for len in 1..=MAX_BURST {
            // Patterns of exactly `len` bits with both ends set.
            let interior = if len <= 2 { 1u32 } else { 1 << (len - 2) };
            for fill in 0..interior {
                let pattern = if len == 1 { 1u32 } else { 1 | (fill << 1) | (1 << (len - 1)) };
                for pos in 0..=(26 - len) {
                    let err = pattern << pos;
                    let syn = syndrome(err);
                    // Every burst up to the code's stated length of five owns its
                    // syndrome outright, so this never has to choose between two
                    // candidates. A test asserts that, at five, not just at the
                    // two actually used.
                    map.entry(syn).or_insert(err);
                }
            }
        }
        map
    })
}

/// Check a 26-bit block against the offset word(s) expected of it.
///
/// Returns the sixteen information bits and whether the corrector had to be used,
/// or `None` when the block is beyond repair.
fn check_block(block: u32, want: [u16; 2]) -> Option<(u16, bool)> {
    let syn = syndrome(block);
    if syn == want[0] || syn == want[1] {
        return Some(((block >> 10) as u16, false));
    }
    for w in want {
        if let Some(&err) = burst_table().get(&(syn ^ w)) {
            return Some((((block ^ err) >> 10) as u16, true));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Symbol layer
// ---------------------------------------------------------------------------

/// Costas/Gardner loop bandwidth as a fraction of the work rate. Wider than the
/// PSK31 receiver's: RDS runs 1187.5 bps rather than 31, so the same fraction of
/// the *symbol* rate is a much larger fraction of the sample rate, and the
/// subcarrier can sit a few hundred Hz out before the pilot hint arrives.
const LOOP_BW: f32 = 0.004;
/// Gardner gain on a magnitude-normalised error, as in [`crate::psk::BpskCore`].
const TIMING_GAIN: f32 = 0.05;

/// Biphase symbol recovery over complex baseband at `sps` samples per symbol.
///
/// The matched filter is the part that differs from an ordinary BPSK receiver.
/// A biphase symbol is positive over its first half and negative over its second,
/// so a flat or Hann-weighted filter integrates it to approximately zero — the
/// PSK31 core cannot be reused as it stands. One period of a sine has exactly the
/// right sign structure and tapers to nothing at both ends and at the mid-symbol
/// transition, which is a good approximation to the shaped pulse the standard
/// specifies and a much better one than a hard ±1.
///
/// **Every symbol decision produces a bit, unconditionally.** There is no quality
/// gate here and there must not be one: the block layer counts 26 bits between
/// one offset word and the next, so a bit withheld is not a bit lost but a *slip*,
/// and sync never comes back. That is not hypothetical — a confidence gate copied
/// across from the PSK31 core, where it withholds finished characters and is
/// harmless, put a permanent hole in this decoder over a band of subcarrier
/// levels where the confidence measure happened to hover at its threshold.
/// Whether a block is worth believing is decided by its syndrome, downstream,
/// where discarding one costs no framing.
struct SymbolSync {
    sps: f32,
    ph: f32,
    freq: f32,
    alpha: f32,
    beta: f32,
    acc: f32,
    mf_taps: Vec<f32>,
    mf_buf: Vec<Complex32>,
    mf_pos: usize,
    hist: VecDeque<Complex32>,
    prev_sym: Complex32,
}

impl SymbolSync {
    fn new(sps: f32) -> Self {
        let (alpha, beta) = loop_gains(LOOP_BW);
        let n = (sps.round() as usize).max(4);
        let mut taps: Vec<f32> =
            (0..n).map(|i| (std::f32::consts::TAU * (i as f32 + 0.5) / n as f32).sin()).collect();
        // Normalise on absolute sum so the filter's output stays comparable to
        // its input whatever the oversampling ratio.
        let sum: f32 = taps.iter().map(|t| t.abs()).sum();
        taps.iter_mut().for_each(|t| *t /= sum);

        let mut hist = VecDeque::new();
        hist.extend(std::iter::repeat_n(Complex32::new(0.0, 0.0), n + 2));
        SymbolSync {
            sps,
            ph: 0.0,
            freq: 0.0,
            alpha,
            beta,
            acc: 0.0,
            mf_taps: taps,
            mf_buf: vec![Complex32::new(0.0, 0.0); n],
            mf_pos: 0,
            hist,
            prev_sym: Complex32::new(1.0, 0.0),
        }
    }

    fn matched(&mut self, y: Complex32) -> Complex32 {
        self.mf_buf[self.mf_pos] = y;
        self.mf_pos = (self.mf_pos + 1) % self.mf_buf.len();
        let n = self.mf_buf.len();
        let mut acc = Complex32::new(0.0, 0.0);
        for (k, &t) in self.mf_taps.iter().enumerate() {
            acc += self.mf_buf[(self.mf_pos + k) % n] * t;
        }
        acc
    }

    /// Feed one complex baseband sample; append any recovered data bits to `out`.
    fn push(&mut self, z: Complex32, out: &mut Vec<u8>) {
        let rot = Complex32::new(self.ph.cos(), -self.ph.sin());
        let y = self.matched(z * rot);

        // Decision-directed BPSK phase error, normalised to radians so the loop
        // bandwidth does not follow the signal level.
        let n = y.norm();
        let e = if n > 1e-20 { (if y.re >= 0.0 { y.im } else { -y.im }) / n } else { 0.0 };
        self.freq = (self.freq + self.beta * e).clamp(-0.4, 0.4);
        self.ph += self.freq + self.alpha * e;
        if self.ph > std::f32::consts::PI {
            self.ph -= std::f32::consts::TAU;
        } else if self.ph < -std::f32::consts::PI {
            self.ph += std::f32::consts::TAU;
        }

        self.hist.push_back(y);
        if self.hist.len() > self.mf_buf.len() + 2 {
            self.hist.pop_front();
        }
        self.acc += 1.0;
        if self.acc < self.sps {
            return;
        }
        self.acc -= self.sps;
        let curr = y;

        // Gardner timing off the half-symbol point, in symbol fractions.
        let midi = self.hist.len().saturating_sub(self.sps as usize / 2 + 1);
        let mid = *self.hist.get(midi).unwrap_or(&curr);
        let et = (mid.conj() * (curr - self.prev_sym)).re;
        let scale = mid.norm() * curr.norm().max(self.prev_sym.norm());
        if scale > 1e-20 {
            self.acc += (TIMING_GAIN * et / scale).clamp(-0.5, 0.5);
        }

        // Differential detection. RDS encodes a data 1 as a *change* of state, so
        // no phase change is a zero — the opposite of PSK31's convention, and a
        // detail worth getting right: inverting every bit leaves a stream whose
        // syndromes match nothing, so the whole decoder simply goes quiet.
        let d = curr * self.prev_sym.conj();
        self.prev_sym = curr;
        out.push(u8::from(d.re < 0.0));
    }
}

// ---------------------------------------------------------------------------
// Block synchronisation
// ---------------------------------------------------------------------------

/// Consecutive unrecoverable blocks before sync is abandoned. About half a second
/// at the 45 blocks per second the standard runs at — long enough to ride out a
/// mobile flutter or a passing car, short enough that a retune does not leave the
/// previous station's data on screen.
const MAX_BAD_RUN: u32 = 24;

/// Good blocks before sync is *reported*. Collection starts on the first offset
/// word that matches, but a lone match happens by chance roughly once in
/// 200 bit positions, so nothing is claimed until two clean groups have run
/// through at the spacing the standard requires.
const CONFIRM_BLOCKS: u32 = 8;

#[derive(Clone, Copy, PartialEq)]
enum SyncState {
    Hunting,
    Locked,
}

struct BlockSync {
    reg: u32,
    filled: u32,
    state: SyncState,
    /// Bits still to arrive before the current block is complete.
    countdown: u32,
    /// Which block of the group is being collected, 0 = A.
    pos: usize,
    group: [u16; 4],
    valid: u8,
    corrected: u8,
    bad_run: u32,
    good_run: u32,
    /// This lock has once run [`CONFIRM_BLOCKS`] good blocks in a row. Unlike
    /// [`BlockSync::synced`] it survives an isolated bad block, and is only
    /// cleared when sync is abandoned.
    confirmed: bool,
}

impl BlockSync {
    fn new() -> Self {
        BlockSync {
            reg: 0,
            filled: 0,
            state: SyncState::Hunting,
            countdown: 0,
            pos: 0,
            group: [0; 4],
            valid: 0,
            corrected: 0,
            bad_run: 0,
            good_run: 0,
            confirmed: false,
        }
    }

    fn reset(&mut self) {
        *self = BlockSync::new();
    }

    fn synced(&self) -> bool {
        self.state == SyncState::Locked && self.good_run >= CONFIRM_BLOCKS
    }

    /// Shift in one bit; call `emit` with each completed group and whether this
    /// lock has been confirmed (see [`BlockSync::confirmed`]).
    fn push(&mut self, bit: u8, emit: &mut impl FnMut([u16; 4], u8, u8, bool)) {
        self.reg = ((self.reg << 1) | bit as u32) & 0x3ff_ffff;
        self.filled = self.filled.saturating_add(1);
        if self.filled < 26 {
            return;
        }

        match self.state {
            SyncState::Hunting => {
                let syn = syndrome(self.reg);
                // No error correction while hunting: a corrector applied to
                // 26 bits of noise finds a "burst" in roughly one window in six
                // and would synchronise the decoder onto nothing at all.
                let Some(&(_, pos)) = OFFSETS.iter().find(|(off, _)| *off == syn) else {
                    return;
                };
                self.state = SyncState::Locked;
                self.group = [0; 4];
                self.valid = 0;
                self.corrected = 0;
                self.group[pos] = (self.reg >> 10) as u16;
                self.valid = 1 << pos;
                self.good_run = 1;
                self.bad_run = 0;
                self.confirmed = false;
                self.finish_block(pos, emit);
            }
            SyncState::Locked => {
                self.countdown -= 1;
                if self.countdown > 0 {
                    return;
                }
                let pos = self.pos;
                match check_block(self.reg, expected(pos)) {
                    Some((info, fixed)) => {
                        self.group[pos] = info;
                        self.valid |= 1 << pos;
                        if fixed {
                            self.corrected |= 1 << pos;
                        }
                        self.good_run = self.good_run.saturating_add(1);
                        self.bad_run = 0;
                        if self.good_run >= CONFIRM_BLOCKS {
                            self.confirmed = true;
                        }
                    }
                    None => {
                        // Keep the raw information bits so the diagnostics view
                        // shows what arrived, but leave the valid bit clear so
                        // nothing downstream reads them.
                        self.group[pos] = (self.reg >> 10) as u16;
                        self.good_run = 0;
                        self.bad_run += 1;
                        if self.bad_run >= MAX_BAD_RUN {
                            self.reset();
                            return;
                        }
                    }
                }
                self.finish_block(pos, emit);
            }
        }
    }

    /// Advance past the block just decided, emitting the group when D is done.
    fn finish_block(&mut self, pos: usize, emit: &mut impl FnMut([u16; 4], u8, u8, bool)) {
        self.countdown = 26;
        if pos == 3 {
            emit(self.group, self.valid, self.corrected, self.confirmed);
            self.group = [0; 4];
            self.valid = 0;
            self.corrected = 0;
        }
        self.pos = (pos + 1) % 4;
    }
}

// ---------------------------------------------------------------------------
// Group parsing
// ---------------------------------------------------------------------------

/// Application identification for RadioText+, from a group 3A.
const AID_RT_PLUS: u16 = 0x4BD7;

/// The RDS basic code table (G0), IEC 62106 annex E table E.1, in order from
/// code 0x20 to 0xff.
///
/// Its lower half is nearly ASCII but not quite, and the four places it parts
/// company are the standard's, not typos: 0x24 is the international currency
/// sign — a dollar is at 0xab — 0x5e a horizontal bar, 0x60 a double vertical
/// line, 0x7e a macron, and 0x7f is blank. A station that sends ASCII anyway
/// only ever loses a piece of punctuation by it; reading the table the other way
/// would lose every accented letter in the upper half, which is what the letters
/// in a station's own name are made of.
///
/// One glyph the published table does not settle: 0x8d is drawn as a sharp s and
/// read either as ß or as a Greek β. RDS text is words in a language, and only
/// the German ß ever appears in one, so that is the reading taken here.
#[rustfmt::skip]
const RDS_G0: [char; 224] = [
    ' ', '!', '"', '#', '¤', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '―', '_',
    '‖', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '¯', ' ',
    'á', 'à', 'é', 'è', 'í', 'ì', 'ó', 'ò', 'ú', 'ù', 'Ñ', 'Ç', 'Ş', 'ß', '¡', 'Ĳ',
    'â', 'ä', 'ê', 'ë', 'î', 'ï', 'ô', 'ö', 'û', 'ü', 'ñ', 'ç', 'ş', 'ǧ', 'ı', 'ĳ',
    'ª', 'α', '©', '‰', 'Ǧ', 'ě', 'ň', 'ő', 'π', '€', '£', '$', '←', '↑', '→', '↓',
    'º', '¹', '²', '³', '±', 'İ', 'ń', 'ű', 'µ', '¿', '÷', '°', '¼', '½', '¾', '§',
    'Á', 'À', 'É', 'È', 'Í', 'Ì', 'Ó', 'Ò', 'Ú', 'Ù', 'Ř', 'Č', 'Š', 'Ž', 'Ð', 'Ŀ',
    'Â', 'Ä', 'Ê', 'Ë', 'Î', 'Ï', 'Ô', 'Ö', 'Û', 'Ü', 'ř', 'č', 'š', 'ž', 'đ', 'ŀ',
    'Ã', 'Å', 'Æ', 'Œ', 'ŷ', 'Ý', 'Õ', 'Ø', 'Þ', 'Ŋ', 'Ŕ', 'Ć', 'Ś', 'Ź', 'Ŧ', 'ð',
    'ã', 'å', 'æ', 'œ', 'ŵ', 'ý', 'õ', 'ø', 'þ', 'ŋ', 'ŕ', 'ć', 'ś', 'ź', 'ŧ', ' ',
];

/// Printable text from one RDS character code, through [`RDS_G0`].
///
/// Everything below 0x20 is a control code with no glyph in the table. Those
/// become a middle dot rather than a guess: a station padding with NUL and a
/// block the error correction let through with a wrong byte look the same from
/// here, and the dot says the character did not decode.
fn rds_char(code: u8) -> char {
    match code {
        0x20..=0xff => RDS_G0[(code - 0x20) as usize],
        _ => '·',
    }
}

/// Trim a decoded field the way it is meant to be read: the standard pads with
/// spaces to a fixed length, and those are not part of the name.
fn finish_text(buf: &[u8]) -> String {
    let s: String = buf.iter().map(|&c| rds_char(c)).collect();
    s.trim_end().to_string()
}

/// The window over which [`mjd_to_date`] is exact: 1900-03-01 to 2100-02-28.
///
/// The standard's arithmetic takes a year to be 365.25 days flat, with no
/// correction for the century years that are not leap years — so it is right
/// between one such boundary and the next and wrong outside, by two days before
/// 1900-03-01 and by one after 2100-02-28. Both ends were found by comparing
/// every day from 1900 to 2132 against a calendar; the failures are exactly those
/// two runs and nothing in between.
///
/// A station broadcasts today's date, so this only ever excludes a corrupted
/// field — which is the point of checking it.
const MJD_VALID: std::ops::RangeInclusive<u32> = 15_079..=88_127;

/// Modified Julian Date to a calendar date, by the standard's own arithmetic.
///
/// Signed throughout, and left for the caller to validate: outside [`MJD_VALID`]
/// this returns a month like 0 or 13 rather than an error, and casting that to a
/// `u8` in here would turn it into something that looks like a date.
fn mjd_to_date(mjd: u32) -> (i32, i32, i32) {
    let yp = ((mjd as f64 - 15_078.2) / 365.25) as i32;
    let mp = ((mjd as f64 - 14_956.1 - (yp as f64 * 365.25).floor()) / 30.6001) as i32;
    let day = mjd as i32
        - 14_956
        - (yp as f64 * 365.25).floor() as i32
        - (mp as f64 * 30.6001).floor() as i32;
    let k = i32::from(mp == 14 || mp == 15);
    (yp + k + 1900, mp - 1 - k * 12, day)
}

/// Accumulates groups into the station picture.
struct Assembler {
    data: RdsData,
    changed: bool,

    /// Programme service, and the candidate awaiting a second identical pass.
    ps_buf: [u8; 8],
    ps_seen: u8,
    ps_candidate: Option<String>,

    /// RadioText, its A/B flag, and how long this station's variant is.
    rt_buf: [u8; 64],
    rt_seen: u64,
    rt_len: usize,
    rt_ab: Option<bool>,

    ptyn_buf: [u8; 8],
    ptyn_seen: u8,

    /// Programme identification awaiting a second sighting.
    pi_candidate: Option<u16>,

    /// Extended country code awaiting a second sighting, for the same reason
    /// the programme identification does — see [`Assembler::group1a`].
    ecc_candidate: Option<u8>,

    /// The five-bit group code group 3A assigned to RadioText+, if any.
    rt_plus_group: Option<u8>,
}

impl Assembler {
    fn new() -> Self {
        Assembler {
            data: RdsData::default(),
            changed: false,
            ps_buf: [b' '; 8],
            ps_seen: 0,
            ps_candidate: None,
            rt_buf: [b' '; 64],
            rt_seen: 0,
            rt_len: 64,
            rt_ab: None,
            ptyn_buf: [b' '; 8],
            ptyn_seen: 0,
            pi_candidate: None,
            ecc_candidate: None,
            rt_plus_group: None,
        }
    }

    fn reset(&mut self) {
        let stats = self.data.stats;
        *self = Assembler::new();
        // Counters belong to the decoder run, not to the station.
        self.data.stats = stats;
        self.changed = true;
    }

    fn set<T: PartialEq>(field: &mut T, v: T, changed: &mut bool) {
        if *field != v {
            *field = v;
            *changed = true;
        }
    }

    fn group(&mut self, blocks: [u16; 4], valid: u8) {
        if valid & 0b0001 != 0 {
            self.programme_id(blocks[0]);
        }
        if valid & 0b0010 == 0 {
            // Without block B there is no group type, so there is nothing that
            // can be done with blocks C and D.
            return;
        }
        let b = blocks[1];
        let gtype = (b >> 12) as u8;
        let version_b = b & 0x0800 != 0;
        Self::set(&mut self.data.tp, b & 0x0400 != 0, &mut self.changed);
        Self::set(&mut self.data.pty, Some(((b >> 5) & 0x1f) as u8), &mut self.changed);

        match (gtype, version_b) {
            (0, _) => self.group0(b, blocks, valid, version_b),
            (1, false) => self.group1a(blocks, valid),
            (2, _) => self.group2(b, blocks, valid, version_b),
            (3, false) => self.group3a(blocks, valid),
            (4, false) => self.group4a(b, blocks, valid),
            (10, false) => self.group10a(b, blocks, valid),
            _ => {}
        }

        // RadioText+ rides on whichever group type this station's 3A nominated.
        let code = (gtype << 1) | u8::from(version_b);
        if self.rt_plus_group == Some(code) {
            self.rt_plus(b, blocks, valid);
        }
    }

    /// The programme identification, accepted only on a second sighting.
    ///
    /// It is the one field with no redundancy beyond its own block check, it
    /// never changes while a station is tuned, and it is what the whole display
    /// is keyed on — so one corrected block is not enough to rename a station.
    fn programme_id(&mut self, pi: u16) {
        if self.data.pi == Some(pi) {
            return;
        }
        if self.pi_candidate == Some(pi) {
            Self::set(&mut self.data.pi, Some(pi), &mut self.changed);
        } else {
            self.pi_candidate = Some(pi);
        }
    }

    /// Group 0A/0B: station name, traffic announcement flag, alternative
    /// frequencies.
    fn group0(&mut self, b: u16, blocks: [u16; 4], valid: u8, version_b: bool) {
        Self::set(&mut self.data.ta, b & 0x0010 != 0, &mut self.changed);
        Self::set(&mut self.data.music, Some(b & 0x0008 != 0), &mut self.changed);

        if !version_b && valid & 0b0100 != 0 {
            let c = blocks[2];
            for code in [(c >> 8) as u8, (c & 0xff) as u8] {
                if let Some(hz) = af_code_hz(code)
                    && !self.data.af.contains(&hz)
                {
                    self.data.af.push(hz);
                    self.data.af.sort_unstable();
                    self.changed = true;
                }
            }
        }

        if valid & 0b1000 == 0 {
            return;
        }
        let seg = (b & 0x0003) as usize;
        let d = blocks[3];
        self.ps_buf[seg * 2] = (d >> 8) as u8;
        self.ps_buf[seg * 2 + 1] = (d & 0xff) as u8;
        self.ps_seen |= 1 << seg;
        if self.ps_seen != 0x0f {
            return;
        }
        self.ps_seen = 0;
        let name = finish_text(&self.ps_buf);
        // Two identical passes before it is shown. Stations scroll text through
        // this eight-character field, and a single pass across a scroll — or
        // across a corrected block — puts a word fragment on screen as if it
        // were the station's name.
        if self.ps_candidate.as_deref() == Some(name.as_str()) {
            Self::set(&mut self.data.ps, Some(name), &mut self.changed);
        } else {
            self.ps_candidate = Some(name);
        }
    }

    /// Group 1A: the extended country code, accepted only on a second sighting.
    ///
    /// The same rule as [`Assembler::programme_id`], and for a sharper reason.
    /// This byte decides which of two entirely different 32-entry programme-type
    /// tables the station is read against — 5 is "Education" under one and
    /// "Rock" under the other — and whether its identity code is spelled out as
    /// a call sign. It has no redundancy beyond its own block check, it does not
    /// change while a station is tuned, and a block the corrector repaired is
    /// trusted like any other.
    ///
    /// Accepting it on one sighting is measurable, not theoretical: over a
    /// minute of a marginal station transmitting nothing but `E0`, the decoder
    /// published `E0`, `E1` and `92`. Land one of those on a high nibble of
    /// `0xA` — one chance in sixteen — and the whole window re-labels itself
    /// RBDS until the next real 1A arrives, which is how a station that never
    /// moved appears to flip between the two standards.
    fn group1a(&mut self, blocks: [u16; 4], valid: u8) {
        if valid & 0b0100 == 0 {
            return;
        }
        let c = blocks[2];
        if (c >> 12) & 0x7 != 0 {
            return;
        }
        let ecc = (c & 0xff) as u8;
        if self.data.ecc == Some(ecc) {
            return;
        }
        if self.ecc_candidate == Some(ecc) {
            Self::set(&mut self.data.ecc, Some(ecc), &mut self.changed);
        } else {
            self.ecc_candidate = Some(ecc);
        }
    }

    /// Group 2A/2B: RadioText, four characters per group or two in version B.
    fn group2(&mut self, b: u16, blocks: [u16; 4], valid: u8, version_b: bool) {
        let ab = b & 0x0010 != 0;
        if self.rt_ab != Some(ab) {
            // The flag toggling is the station saying "this is a new text".
            // Whatever was half-assembled belongs to the old one.
            if self.rt_ab.is_some() {
                self.rt_seen = 0;
                self.rt_buf = [b' '; 64];
            }
            self.rt_ab = Some(ab);
        }
        let seg = (b & 0x000f) as usize;
        let (len, chars) = if version_b { (32, 2) } else { (64, 4) };
        if self.rt_len != len {
            self.rt_len = len;
            self.rt_seen = 0;
            self.rt_buf = [b' '; 64];
        }

        let put = |i: usize, c: u8, seen: &mut u64, buf: &mut [u8; 64]| {
            if i < 64 {
                buf[i] = c;
                *seen |= 1 << i;
            }
        };
        let base = seg * chars;
        if !version_b {
            if valid & 0b0100 == 0 || valid & 0b1000 == 0 {
                return;
            }
            let (c, d) = (blocks[2], blocks[3]);
            put(base, (c >> 8) as u8, &mut self.rt_seen, &mut self.rt_buf);
            put(base + 1, (c & 0xff) as u8, &mut self.rt_seen, &mut self.rt_buf);
            put(base + 2, (d >> 8) as u8, &mut self.rt_seen, &mut self.rt_buf);
            put(base + 3, (d & 0xff) as u8, &mut self.rt_seen, &mut self.rt_buf);
        } else {
            if valid & 0b1000 == 0 {
                return;
            }
            let d = blocks[3];
            put(base, (d >> 8) as u8, &mut self.rt_seen, &mut self.rt_buf);
            put(base + 1, (d & 0xff) as u8, &mut self.rt_seen, &mut self.rt_buf);
        }
        self.radiotext_try_commit();
    }

    /// Show the text once every character up to its end has arrived.
    ///
    /// A carriage return ends it early — most stations use far less than the full
    /// field and mark where they stopped — so waiting for all sixteen segments
    /// would mean never showing anything for those stations.
    fn radiotext_try_commit(&mut self) {
        let stop = self.rt_buf[..self.rt_len].iter().position(|&c| c == 0x0d);
        let need = stop.map_or(self.rt_len, |i| i + 1);
        let have_all = (0..need).all(|i| self.rt_seen & (1 << i) != 0);
        if !have_all {
            return;
        }
        let text = finish_text(&self.rt_buf[..stop.unwrap_or(need)]);
        if text.is_empty() {
            return;
        }
        Self::set(&mut self.data.radiotext, Some(text), &mut self.changed);
    }

    /// Group 3A: which group type this station uses for which open application.
    fn group3a(&mut self, blocks: [u16; 4], valid: u8) {
        if valid & 0b0010 == 0 || valid & 0b1000 == 0 {
            return;
        }
        if blocks[3] == AID_RT_PLUS {
            self.rt_plus_group = Some((blocks[1] & 0x1f) as u8);
        }
    }

    /// Group 4A: the station's clock.
    fn group4a(&mut self, b: u16, blocks: [u16; 4], valid: u8) {
        if valid & 0b0100 == 0 || valid & 0b1000 == 0 {
            return;
        }
        let (c, d) = (blocks[2], blocks[3]);
        let mjd = ((b as u32 & 0x3) << 15) | (c as u32 >> 1);
        let hour = (((c & 1) << 4) | (d >> 12)) as u8;
        let minute = ((d >> 6) & 0x3f) as u8;
        let half_hours = (d & 0x1f) as i8;
        let offset = if d & 0x20 != 0 { -half_hours } else { half_hours };
        if hour > 23 || minute > 59 || !MJD_VALID.contains(&mjd) {
            return;
        }
        let (year, month, day) = mjd_to_date(mjd);
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return;
        }
        let clock = RdsClock {
            year,
            month: month as u8,
            day: day as u8,
            hour,
            minute,
            offset_half_hours: offset,
        };
        Self::set(&mut self.data.clock, Some(clock), &mut self.changed);
    }

    /// Group 10A: the programme type name, eight characters in two halves.
    fn group10a(&mut self, b: u16, blocks: [u16; 4], valid: u8) {
        if valid & 0b0100 == 0 || valid & 0b1000 == 0 {
            return;
        }
        let seg = (b & 0x1) as usize;
        let (c, d) = (blocks[2], blocks[3]);
        for (i, ch) in [(c >> 8) as u8, (c & 0xff) as u8, (d >> 8) as u8, (d & 0xff) as u8]
            .into_iter()
            .enumerate()
        {
            self.ptyn_buf[seg * 4 + i] = ch;
        }
        self.ptyn_seen |= 1 << seg;
        if self.ptyn_seen == 0b11 {
            let name = finish_text(&self.ptyn_buf);
            Self::set(&mut self.data.ptyn, Some(name), &mut self.changed);
        }
    }

    /// RadioText+: two tagged substrings of the current RadioText per group.
    fn rt_plus(&mut self, b: u16, blocks: [u16; 4], valid: u8) {
        if valid & 0b0100 == 0 || valid & 0b1000 == 0 {
            return;
        }
        let Some(text) = self.data.radiotext.clone() else {
            return;
        };
        let (c, d) = (blocks[2], blocks[3]);
        // The two tags straddle the block boundaries: six bits of content type
        // split 3/3 across B and C for the first, and 1/5 across C and D for the
        // second.
        let tags = [
            (
                (((b & 0x7) << 3) | (c >> 13)) as u8,
                ((c >> 7) & 0x3f) as usize,
                (((c >> 1) & 0x3f) as usize) + 1,
            ),
            (
                (((c & 0x1) << 5) | (d >> 11)) as u8,
                ((d >> 5) & 0x3f) as usize,
                ((d & 0x1f) as usize) + 1,
            ),
        ];

        let chars: Vec<char> = text.chars().collect();
        let mut out = RtPlus::default();
        for (class, start, len) in tags {
            if class == 0 || start >= chars.len() {
                continue;
            }
            let end = (start + len).min(chars.len());
            let value: String = chars[start..end].iter().collect();
            let value = value.trim().to_string();
            if value.is_empty() {
                continue;
            }
            match class {
                1 => out.title = Some(value),
                4 => out.artist = Some(value),
                _ => out.other.push((class, value)),
            }
        }
        if out.is_empty() {
            return;
        }
        // Carry forward whichever half this group did not carry: title and
        // artist arrive in the same group as a rule, but a station that splits
        // them across groups should not blank one to set the other.
        if let Some(prev) = self.data.rt_plus.as_ref() {
            if out.title.is_none() {
                out.title = prev.title.clone();
            }
            if out.artist.is_none() {
                out.artist = prev.artist.clone();
            }
        }
        Self::set(&mut self.data.rt_plus, Some(out), &mut self.changed);
    }
}

// ---------------------------------------------------------------------------
// The receiver
// ---------------------------------------------------------------------------

/// How many groups may queue up for the diagnostics view before the oldest are
/// dropped. At about eleven groups a second this is several seconds of slack for
/// a caller that polls twice a second; a drop shows up as a jump in the sequence
/// number rather than as a silent gap.
const MAX_PENDING_GROUPS: usize = 256;

/// RDS/RBDS receiver over the FM composite multiplex.
///
/// Feed it the discriminator output — *before* de-emphasis, which is 25 dB down
/// at 57 kHz — and poll [`RdsRx::take`] for what it has made of it.
pub struct RdsRx {
    ddc: Ddc,
    /// ±2.4 kHz around the recovered carrier — the data's own bandwidth and
    /// nothing else's. See [`DATA_BW_HZ`].
    channel: ComplexFir,
    /// Nominal carrier, and what was last programmed into the DDC, so a pilot
    /// hint that has not moved does not churn the NCO.
    tuned_hz: f64,
    wide: Vec<Complex32>,
    base: Vec<Complex32>,
    filtered: Vec<Complex32>,
    bits: Vec<u8>,
    sym: SymbolSync,
    blocks: BlockSync,
    asm: Assembler,
    pending: VecDeque<RdsGroupLog>,
    /// Groups emitted since the decoder started, dropped ones included.
    emitted: u64,
}

impl RdsRx {
    /// `None` when the channel rate is too low to carry the subcarrier — see
    /// [`RDS_MIN_RATE`].
    pub fn new(channel_rate: f64) -> Option<Self> {
        if channel_rate < RDS_MIN_RATE {
            return None;
        }
        let mut ddc = Ddc::new(channel_rate, WORK_TARGET);
        ddc.set_offset_hz(CARRIER_HZ);
        let work_rate = Ddc::rate_for(channel_rate, WORK_TARGET);
        Some(RdsRx {
            ddc,
            channel: ComplexFir::new(bandpass_taps(
                CHANNEL_TAPS,
                -DATA_BW_HZ,
                DATA_BW_HZ,
                work_rate,
            )),
            tuned_hz: CARRIER_HZ,
            wide: Vec::new(),
            base: Vec::new(),
            filtered: Vec::new(),
            bits: Vec::new(),
            sym: SymbolSync::new((work_rate / BITRATE) as f32),
            blocks: BlockSync::new(),
            asm: Assembler::new(),
            pending: VecDeque::new(),
            emitted: 0,
        })
    }

    /// Forget this station: called on a retune, so the previous station's name
    /// does not sit on screen under the new one's audio.
    pub fn reset(&mut self) {
        self.blocks.reset();
        self.asm.reset();
        self.asm.data.stats = Default::default();
        self.pending.clear();
    }

    /// Retune the down-converter from the stereo pilot, whose third harmonic the
    /// subcarrier is locked to. `None` while the pilot is not locked, which
    /// leaves the nominal 57 kHz and the Costas loop to find the rest.
    pub fn set_pilot_hz(&mut self, pilot_hz: Option<f64>) {
        let want = pilot_hz.map_or(CARRIER_HZ, |hz| hz * 3.0);
        // A hint that has not moved a whole Hz is not worth an NCO write, and the
        // pilot PLL's estimate dithers continuously.
        if (want - self.tuned_hz).abs() < 1.0 {
            return;
        }
        self.tuned_hz = want;
        self.ddc.set_offset_hz(want);
    }

    /// Consume composite multiplex at the channel rate.
    pub fn process(&mut self, mpx: &[f32]) {
        self.wide.clear();
        self.wide.extend(mpx.iter().map(|&x| Complex32::new(x, 0.0)));
        self.base.clear();
        self.ddc.process(&self.wide, &mut self.base);
        self.filtered.clear();
        self.channel.process(&self.base, &mut self.filtered);

        self.bits.clear();
        for &z in &self.filtered {
            self.sym.push(z, &mut self.bits);
        }

        let (blocks, asm, pending) = (&mut self.blocks, &mut self.asm, &mut self.pending);
        let emitted = &mut self.emitted;
        for &bit in &self.bits {
            blocks.push(bit, &mut |group, valid, corrected, confirmed| {
                let stats = &mut asm.data.stats;
                stats.groups += 1;
                stats.blocks_ok += (valid & !corrected).count_ones() as u64;
                stats.blocks_corrected += corrected.count_ones() as u64;
                stats.blocks_bad += 4 - valid.count_ones() as u64;
                if valid & 0b0010 != 0 {
                    let b = group[1];
                    let idx = ((b >> 12) << 1) | ((b >> 11) & 1);
                    stats.group_types[idx as usize] += 1;
                }
                // Only a confirmed lock feeds the station picture. A lone offset
                // word matches by chance about once in 200 bit positions, and
                // the groups collected on such a lock are programme material —
                // two identical ones were enough to invent a station identity
                // on a stereo broadcast with no data subcarrier at all. The
                // diagnostics log below still gets every group.
                if confirmed {
                    asm.group(group, valid);
                }

                *emitted += 1;
                if pending.len() >= MAX_PENDING_GROUPS {
                    pending.pop_front();
                }
                pending.push_back(RdsGroupLog { blocks: group, valid, corrected });
            });
        }
    }

    /// The station picture and the groups decoded since the last call, or `None`
    /// when neither has moved.
    pub fn take(&mut self) -> Option<RdsData> {
        let synced = self.blocks.synced();
        let changed = self.asm.changed || synced != self.asm.data.sync;
        if !changed && self.pending.is_empty() {
            return None;
        }
        self.asm.data.sync = synced;
        self.asm.changed = false;
        let mut out = self.asm.data.clone();
        out.group_seq = self.emitted - self.pending.len() as u64;
        out.groups = self.pending.drain(..).collect();
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check bits for sixteen information bits, before the offset word is added.
    fn check_bits(info: u16) -> u16 {
        let mut reg = 0u32;
        let word = (info as u32) << 10;
        for i in (0..26).rev() {
            reg = (reg << 1) | ((word >> i) & 1);
            if reg & (1 << 10) != 0 {
                reg ^= POLY;
            }
        }
        (reg & 0x3ff) as u16
    }

    fn encode(info: u16, offset: u16) -> u32 {
        ((info as u32) << 10) | (check_bits(info) ^ offset) as u32
    }

    #[test]
    fn a_codeword_divides_cleanly_and_an_offset_is_its_own_syndrome() {
        // The property the whole block layer rests on: with no offset the
        // remainder is zero, so with one it is the offset itself. If this ever
        // stopped holding, every syndrome comparison here would need a second
        // table of constants.
        for info in [0u16, 1, 0x1234, 0xABCD, 0xFFFF] {
            assert_eq!(syndrome(encode(info, 0)), 0, "{info:04X} is not a codeword");
            for off in [OFFSET_A, OFFSET_B, OFFSET_C, OFFSET_CP, OFFSET_D] {
                assert_eq!(syndrome(encode(info, off)), off, "{info:04X}/{off:03X}");
            }
        }
    }

    #[test]
    fn the_offset_words_are_distinguishable_from_each_other() {
        let all = [OFFSET_A, OFFSET_B, OFFSET_C, OFFSET_CP, OFFSET_D];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b, "two offset words are the same value");
            }
        }
    }

    /// Every burst of exactly `len` bits, at every position it fits.
    fn bursts(len: usize) -> impl Iterator<Item = u32> {
        let interior = if len <= 2 { 1u32 } else { 1 << (len - 2) };
        (0..interior).flat_map(move |fill| {
            let pattern = if len == 1 { 1u32 } else { 1 | (fill << 1) | (1 << (len - 1)) };
            (0..=(26 - len)).map(move |pos| pattern << pos)
        })
    }

    #[test]
    fn every_burst_up_to_the_codes_stated_limit_owns_its_syndrome() {
        // Five is the Rieger bound for ten check bits and the figure the standard
        // quotes. Asserted out to five even though only two are used, because it
        // is the property that makes the correction unambiguous — if it ever
        // stopped holding, raising MAX_BURST would silently start guessing.
        let mut seen: HashMap<u16, u32> = HashMap::new();
        for len in 1..=5 {
            for err in bursts(len) {
                let syn = syndrome(err);
                assert_ne!(syn, 0, "a burst is not a codeword");
                if let Some(&other) = seen.get(&syn) {
                    assert_eq!(other, err, "two bursts share syndrome {syn:03X}");
                }
                seen.insert(syn, err);
            }
        }
        assert_eq!(seen.len(), 367, "burst patterns out to length five");
        // The table actually built covers only what MAX_BURST allows.
        assert_eq!(burst_table().len(), 51);
        assert!(!burst_table().contains_key(&0), "a zero syndrome is not an error");
    }

    #[test]
    fn correction_is_weak_enough_that_a_lost_station_is_noticed() {
        // The decoder only ever discovers it has lost sync by failing
        // MAX_BAD_RUN blocks in a row, so the fraction of the syndrome space the
        // corrector claims is a direct limit on that. See MAX_BURST for the
        // table this pins down.
        let claimed = burst_table().len() + 1; // + the zero-error syndrome
        let accept = claimed as f64 / 1024.0;
        assert!(accept < 0.06, "corrector claims {accept:.3} of the syndrome space");
        let miss = (1.0 - accept).powi(MAX_BAD_RUN as i32);
        assert!(
            miss > 0.2,
            "a run of {MAX_BAD_RUN} failures would happen only {miss:.4} of the time"
        );
    }

    #[test]
    fn the_offset_words_are_not_far_enough_apart_to_correct_between() {
        // Recorded because it is the reason the hunt refuses to correct. The
        // offset words were chosen to make a *misaligned* window fail, not to be
        // far apart in Hamming distance, and C and D sit one single-bit syndrome
        // from each other — so no burst length, not even one, keeps them
        // distinguishable once a corrector is running.
        let one_bit: Vec<u16> = bursts(1).map(syndrome).collect();
        assert!(one_bit.contains(&(OFFSET_C ^ OFFSET_D)));
    }

    #[test]
    fn the_bursts_the_corrector_takes_on_give_the_information_back() {
        let info = 0xB37Au16;
        for len in 1..=MAX_BURST {
            for err in bursts(len) {
                let word = encode(info, OFFSET_B) ^ err;
                let got = check_block(word, expected(1));
                assert_eq!(got.map(|(i, _)| i), Some(info), "burst {err:07X} was not corrected");
                assert!(got.expect("corrected above").1, "should report a correction");
            }
        }
    }

    #[test]
    fn a_burst_beyond_the_corrector_is_refused_rather_than_mis_corrected() {
        // The information bits must never come back *wrong*. Either the block is
        // refused, or — where a longer burst happens to alias onto a shorter
        // one's syndrome — the corrector's answer must still be the right word.
        let info = 0xB37Au16;
        let mut refused = 0;
        for err in bursts(MAX_BURST + 3) {
            let word = encode(info, OFFSET_B) ^ err;
            match check_block(word, expected(1)) {
                None => refused += 1,
                Some((got, _)) => {
                    assert_ne!(got, info, "an uncorrectable burst cannot land on the right answer")
                }
            }
        }
        assert!(refused > 0, "some long bursts must be refused outright");
    }

    #[test]
    fn a_clean_block_is_not_reported_as_corrected() {
        let (info, fixed) = check_block(encode(0x1234, OFFSET_A), expected(0)).expect("clean");
        assert_eq!(info, 0x1234);
        assert!(!fixed);
    }

    #[test]
    fn the_hunt_refuses_a_block_carrying_the_wrong_offset() {
        // The hunt compares syndromes outright, with no corrector in the way.
        // That is what keeps the group structure honest — block B's word cannot
        // be mistaken for block A's — and it is only true without correction.
        let hunt =
            |word: u32| OFFSETS.iter().find(|(off, _)| *off == syndrome(word)).map(|&(_, p)| p);
        assert_eq!(hunt(encode(0x1234, OFFSET_A)), Some(0));
        assert_eq!(hunt(encode(0x1234, OFFSET_B)), Some(1));
        assert_eq!(hunt(encode(0x1234, OFFSET_D)), Some(3));
        // C and C' both mark position 2 — a version-B group puts the programme
        // identification back in the third block and flags it with C'.
        assert_eq!(hunt(encode(0x1234, OFFSET_C)), Some(2));
        assert_eq!(hunt(encode(0x1234, OFFSET_CP)), Some(2));
        // A block with no offset word at all is not a block.
        assert_eq!(hunt(encode(0x1234, 0)), None);
    }

    #[test]
    fn the_position_check_accepts_c_prime_only_where_it_belongs() {
        assert!(check_block(encode(0x1234, OFFSET_CP), expected(2)).is_some());
        assert!(check_block(encode(0x1234, OFFSET_C), expected(2)).is_some());
    }

    #[test]
    fn noise_is_mostly_refused_so_a_dead_frequency_cannot_hold_sync() {
        // 26 bits of a deterministic pseudo-random stream. Some fraction is
        // always correctable by chance — the code has only ten check bits — and
        // the measured figure is what MAX_BURST is chosen against, so pin it.
        let mut x = 0x2545F491_4F6CDD1Du64;
        let mut accepted = 0;
        const TRIES: usize = 100_000;
        for _ in 0..TRIES {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            if check_block((x & 0x3ff_ffff) as u32, expected(0)).is_some() {
                accepted += 1;
            }
        }
        // 51 correctable syndromes plus the clean one, out of 1024: 5.1 %.
        let rate = accepted as f64 / TRIES as f64;
        assert!((0.03..0.08).contains(&rate), "corrector accepted {rate:.4} of pure noise");
    }

    #[test]
    fn the_modified_julian_date_lands_on_the_right_calendar_day() {
        // Both ends of the validity window, a leap day 2000 does have, and the
        // year boundary either side.
        assert_eq!(mjd_to_date(*MJD_VALID.start()), (1900, 3, 1));
        assert_eq!(mjd_to_date(*MJD_VALID.end()), (2100, 2, 28));
        assert_eq!(mjd_to_date(51_544), (2000, 1, 1));
        assert_eq!(mjd_to_date(51_603), (2000, 2, 29));
        assert_eq!(mjd_to_date(51_604), (2000, 3, 1));
        assert_eq!(mjd_to_date(61_040), (2025, 12, 31));
        assert_eq!(mjd_to_date(61_041), (2026, 1, 1));
        assert_eq!(mjd_to_date(61_269), (2026, 8, 17));

        // Every day in the window has to advance by exactly one, which is the
        // check that catches an off-by-one in the month arithmetic rather than
        // just at the anchors.
        let days_in = |y: i32, m: i32| match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            _ => {
                if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                    29
                } else {
                    28
                }
            }
        };
        let (mut y, mut m, mut d) = mjd_to_date(*MJD_VALID.start());
        for mjd in MJD_VALID.start() + 1..=*MJD_VALID.end() {
            d += 1;
            if d > days_in(y, m) {
                d = 1;
                m += 1;
            }
            if m > 12 {
                m = 1;
                y += 1;
            }
            assert_eq!(mjd_to_date(mjd), (y, m, d), "MJD {mjd}");
        }
    }

    #[test]
    fn a_date_outside_the_formulas_window_does_not_look_like_a_date() {
        // What `group4a` relies on to throw out a corrupted field, and the reason
        // the window is checked before the conversion rather than after it: two
        // days before 1900-03-01 the formula is merely *wrong*, not obviously so.
        let (_, month, _) = mjd_to_date(0);
        assert!(!(1..=12).contains(&month), "MJD 0 must not read as a month");
        assert!(!MJD_VALID.contains(&15_020), "1900-01-01 is outside the window");
        assert_eq!(mjd_to_date(15_020), (1900, 1, 3), "off by the missing leap rule");
    }

    #[test]
    fn text_reads_the_standards_code_table() {
        assert_eq!(finish_text(b"ROCK FM "), "ROCK FM");
        assert_eq!(finish_text(b"        "), "");
        // The upper half is where the accented letters live — the reason a
        // Nordic or German station's own name used to come out full of dots.
        assert_eq!(finish_text(&[0x91, 0x97, 0xF1]), "äöå");
        assert_eq!(finish_text(&[0xD1, 0xD7, 0xE1]), "ÄÖÅ");
        assert_eq!(finish_text(&[b'S', b't', b'r', b'a', 0x8D, b'e']), "Straße");
        // Each code is one character, which is what RadioText+ indexes into.
        assert_eq!(finish_text(&[0x91, b'X']).chars().count(), 2);
        // The four lower-half codes that are not their ASCII namesakes.
        assert_eq!(finish_text(&[0x24, 0xAB]), "¤$");
        // Below 0x20 there is no glyph to read, and a dot says so.
        assert_eq!(finish_text(&[0x00, b'X']), "·X");
    }

    /// One group 1A, correct in every bit as far as the block layer can tell,
    /// carrying an extended country code the station never sent.
    fn group_1a(ecc: u8) -> ([u16; 4], u8) {
        // Block B: group 1, version A. Block C: variant 0, then the code.
        ([0xD3C2, 0x1000, ecc as u16, 0], 0b1111)
    }

    #[test]
    fn a_single_country_code_does_not_relabel_the_station() {
        // Issue #173. The extended country code chooses between two entirely
        // different programme-type tables and decides whether the identity is
        // spelled out as a call sign, so it is held to the same rule as the
        // programme identification: seen twice, or not believed. Over a minute
        // of a marginal station sending nothing but E0, the decoder used to
        // publish E0, E1 and 92.
        let mut a = Assembler::new();
        let (g, valid) = group_1a(0xE0);
        a.group(g, valid);
        assert_eq!(a.data.ecc, None, "one sighting is not evidence");
        a.group(g, valid);
        assert_eq!(a.data.ecc, Some(0xE0), "the second settles it");

        // And a corrupted one arriving afterwards does not take it away.
        let (bad, valid) = group_1a(0xA5);
        a.group(bad, valid);
        assert_eq!(a.data.ecc, Some(0xE0), "a lone stray does not move a settled code");
        // Only a code that arrives twice replaces it — a station really can be
        // retuned onto another that shares the dial.
        a.group(bad, valid);
        assert_eq!(a.data.ecc, Some(0xA5));
    }

    #[test]
    fn a_country_code_needs_block_c_and_the_right_variant() {
        let mut a = Assembler::new();
        // Block C lost: nothing to read, and nothing left half-accepted either.
        let (g, _) = group_1a(0xE0);
        a.group(g, 0b1011);
        a.group(g, 0b1011);
        assert_eq!(a.data.ecc, None);
        // Variant 1 puts something else in those bits, not a country code.
        let mut other = g;
        other[2] = 0x1000 | 0xE0;
        a.group(other, 0b1111);
        a.group(other, 0b1111);
        assert_eq!(a.data.ecc, None);
    }
}
