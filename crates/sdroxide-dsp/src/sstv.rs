//! SSTV modem: image ⇄ audio for the Scottie, Martin, and Robot modes.
//!
//! Transmit builds a per-mode timing plan and synthesises tones with a
//! continuous phase accumulator (the `psk.rs`/`rtty.rs` idiom). Receive runs an
//! FM discriminator to recover instantaneous frequency, detects the VIS
//! calibration header to pick the mode, then samples pixels line-by-line at
//! the line starts a robust fit through the 1200 Hz sync pulses gives, and
//! once the picture is complete decodes it again along the sync column of the
//! whole of it (see `sstv_slant`) — so neither fading nor a sender's clock
//! slants it.
//!
//! Timing follows the canonical N7CXI spec (as used by PySSTV/QSSTV). Colour
//! maps to frequency by black = 1500 Hz, white = 2300 Hz; sync = 1200 Hz.

use std::f64::consts::TAU;

use sdroxide_types::SstvMode;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};
use crate::sstv_slant::{
    FreqTrack, Geometry, Line, LineFit, MIN_FILL, SyncMap, measure_sync, robust_line,
};

const BLACK_HZ: f64 = 1500.0;
const WHITE_HZ: f64 = 2300.0;
const SYNC_HZ: f64 = 1200.0;
const VIS_LEADER_HZ: f64 = 1900.0;
const VIS_BIT1_HZ: f64 = 1100.0;
const VIS_BIT0_HZ: f64 = 1300.0;

// ── FSK ID: the station's callsign, sent as tones after the picture ──
//
// The format is JE3HHT's, as published with MMSSTV and implemented by every
// SSTV program and unattended repeater that reads one. It is a 45.45 baud FSK
// stream — 22 ms a bit — of six-bit symbols, most significant bit first,
// carrying ASCII $20..$5F shifted down to $00..$3F. A whole ID is
// `$2A C1..CN $01 XSUM`, where the checksum is the XOR of the characters
// alone. It is preceded by a 300 ms tone and a 100 ms tone that give a receiver
// something to arm on, and the transition out of the second of those is what
// times every bit that follows.
//
// The point of sending it as tones rather than printing the callsign into the
// picture is that a machine can read it: an SSTV repeater logs and announces
// the station that just sent, which a banner across the top of a JPEG can never
// give it (issue #287).

/// The tone a `1` bit is sent on — and the ID's start bit.
const FSKID_ONE_HZ: f64 = 1900.0;
/// The tone a `0` bit is sent on, and the 100 ms block ahead of the start bit.
const FSKID_ZERO_HZ: f64 = 2100.0;
/// The 300 ms tone that opens the ID. (MMSSTV sends 1900 Hz here in its narrow
/// mode; this is the ordinary one.)
const FSKID_LEADER_HZ: f64 = 1500.0;
const FSKID_LEADER_S: f64 = 0.300;
const FSKID_SYNC_S: f64 = 0.100;
/// 45.45 baud.
const FSKID_BIT_S: f64 = 0.022;
/// Header symbol: "an ID starts here".
const FSKID_HEAD: u8 = 0x2A;
/// Terminator symbol, ahead of the checksum.
const FSKID_END: u8 = 0x01;
/// How many characters of the ID go on the air.
///
/// Not in the specification, which gives no limit. A cap belongs here all the
/// same: every character costs 132 ms of air time after a picture that has
/// already taken a minute or two, and an ID is a callsign — anything longer is
/// a mistake in a settings field rather than something to transmit.
const FSKID_MAX_CHARS: usize = 20;

/// The six-bit symbols of an FSK ID for `text`, ready to be keyed — header,
/// characters, terminator and checksum. Empty when there is nothing to send.
///
/// Characters are folded to upper case (the code space has no lower case) and
/// anything still outside ASCII `$20..$5F` is dropped rather than mangled: a
/// callsign with a stray accent sends the rest of itself instead of a symbol
/// the far end would print as noise.
#[must_use]
pub fn fsk_id_symbols(text: &str) -> Vec<u8> {
    let chars: Vec<u8> = text
        .trim()
        .to_ascii_uppercase()
        .bytes()
        .filter(|b| (0x20..=0x5F).contains(b))
        .take(FSKID_MAX_CHARS)
        .map(|b| b - 0x20)
        .collect();
    if chars.is_empty() {
        return Vec::new();
    }
    let xsum = chars.iter().fold(0u8, |a, &c| a ^ c);
    let mut out = Vec::with_capacity(chars.len() + 3);
    out.push(FSKID_HEAD);
    out.extend_from_slice(&chars);
    out.push(FSKID_END);
    out.push(xsum);
    out
}

/// The text of a complete FSK ID symbol stream, or `None` when it is not one.
///
/// The inverse of [`fsk_id_symbols`], and the receiver's whole acceptance test:
/// the header, the terminator and the checksum all have to agree before a
/// callsign is believed. That matters more here than it looks — the hunt runs
/// on whatever is on the frequency, so the framing is the only thing standing
/// between the operator and a callsign invented out of noise.
#[must_use]
pub fn fsk_id_text(symbols: &[u8]) -> Option<String> {
    let (head, rest) = symbols.split_first()?;
    if *head != FSKID_HEAD {
        return None;
    }
    let end = rest.iter().position(|&s| s == FSKID_END)?;
    let (chars, tail) = rest.split_at(end);
    if chars.is_empty() || chars.len() > FSKID_MAX_CHARS {
        return None;
    }
    // `tail[0]` is the terminator itself; the checksum is the symbol after it.
    let &xsum = tail.get(1)?;
    if chars.iter().fold(0u8, |a, &c| a ^ c) != xsum {
        return None;
    }
    Some(chars.iter().map(|&c| (c + 0x20) as char).collect::<String>().trim().to_string())
}

/// Frequency (Hz) for an 8-bit intensity, black→white.
fn value_to_hz(v: u8) -> f64 {
    BLACK_HZ + (v as f64 / 255.0) * (WHITE_HZ - BLACK_HZ)
}

/// Inverse of [`value_to_hz`], clamped to a byte.
fn hz_to_value(hz: f64) -> u8 {
    let v = ((hz - BLACK_HZ) / (WHITE_HZ - BLACK_HZ)) * 255.0;
    v.round().clamp(0.0, 255.0) as u8
}

// ───────────────────────────── mode timing ─────────────────────────────

/// A colour channel within a scan segment.
#[derive(Clone, Copy, PartialEq)]
enum Chan {
    R,
    G,
    B,
    /// Luma.
    Y,
    /// Luma of the *second* image row a PD line carries — see
    /// [`SstvMode::rows_per_line`]. Nothing else in the family has one.
    Y2,
    /// R-Y chroma (Cr).
    Cr,
    /// B-Y chroma (Cb).
    Cb,
}

/// One timed segment of a scan line.
#[derive(Clone, Copy)]
enum Seg {
    /// Constant tone for `dur` seconds at `hz`.
    Tone { hz: f64, dur: f64 },
    /// A pixel scan of `width` samples of channel `chan`, `px` seconds each.
    Scan { chan: Chan, width: u16, px: f64 },
}

/// Per-mode parameters used to build a line plan.
struct Timing {
    sync: f64,
    sync_hz: f64,
    sep: f64,
    sep_hz: f64,
    /// Colour-channel pixel time, seconds.
    px: f64,
}

fn scottie_timing(px: f64) -> Timing {
    Timing { sync: 0.009, sync_hz: SYNC_HZ, sep: 0.0015, sep_hz: 1500.0, px }
}

fn martin_timing(px: f64) -> Timing {
    Timing { sync: 0.004_862, sync_hz: SYNC_HZ, sep: 0.000_572, sep_hz: 1500.0, px }
}

/// The PD family's pixel time, seconds. Everything else about a PD line is
/// shared: a 20 ms sync, a 2.08 ms porch, and four full-width scans.
///
/// From JL Barber N7CXI's 2000 mode specification (cross-checked against
/// `windytan/slowrx`'s `modespec.c`). Each one reproduces the published line
/// time exactly — PD90 is 20 + 2.08 + 4 × 320 × 0.532 = 703.04 ms — which is
/// the check worth making on a transcribed table, because a pixel time that is
/// out by a percent still decodes into a picture, just a sheared one.
fn pd_pixel_time(mode: SstvMode) -> f64 {
    match mode {
        SstvMode::Pd50 => 0.000_286,
        SstvMode::Pd90 => 0.000_532,
        SstvMode::Pd120 => 0.000_190,
        SstvMode::Pd160 => 0.000_382,
        SstvMode::Pd180 => 0.000_286,
        SstvMode::Pd240 => 0.000_382,
        _ => 0.000_286, // Pd290
    }
}

/// The ordered segments for one scan line of `mode` at image width `w`.
/// Robot modes carry their (per-line-varying) chroma channel via `line`.
fn line_segments(mode: SstvMode, w: u16, line: u16) -> Vec<Seg> {
    use Chan::*;
    match mode {
        SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx => {
            let px = match mode {
                SstvMode::Scottie1 => 0.000_432,
                SstvMode::Scottie2 => 0.000_275_2,
                _ => 0.001_08,
            };
            let t = scottie_timing(px);
            // Scottie order: sep · G · sep · B · SYNC · sep · R.
            vec![
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: G, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: B, width: w, px: t.px },
                Seg::Tone { hz: t.sync_hz, dur: t.sync },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: R, width: w, px: t.px },
            ]
        }
        SstvMode::Martin1 | SstvMode::Martin2 => {
            let px = if mode == SstvMode::Martin1 { 0.000_457_6 } else { 0.000_228_8 };
            let t = martin_timing(px);
            // Martin order: SYNC · porch · G · sep · B · sep · R · sep.
            //
            // Four separators, not three: a porch after the sync *and* one
            // after every scan, the last one included. Without that trailing
            // pulse the line comes to 445.874 ms against the published
            // 446.446 — 0.13 % short, which the receiver never notices because
            // it re-locks to the sync every line, and which shears a
            // transmitted picture by a third of a line by the bottom. Found by
            // pinning the line times against the published table rather than
            // by looking at the plan.
            vec![
                Seg::Tone { hz: t.sync_hz, dur: t.sync },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: G, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: B, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: R, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
            ]
        }
        SstvMode::Robot72 => {
            // Y full width; Cr, Cb half width. 300 ms/line.
            let cw = w / 2;
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.009 },
                Seg::Tone { hz: 1500.0, dur: 0.003 },
                Seg::Scan { chan: Y, width: w, px: 0.000_431_25 },
                Seg::Tone { hz: 1500.0, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan: Cr, width: cw, px: 0.000_431_25 },
                Seg::Tone { hz: 2300.0, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan: Cb, width: cw, px: 0.000_431_25 },
            ]
        }
        SstvMode::Robot36 => {
            // Y full width; one chroma per line, alternating even=Cr / odd=Cb
            // (4:2:0). 150 ms/line. Separator frequency signals which chroma.
            let cw = w / 2;
            let even = line % 2 == 0;
            let (chan, sep_hz) = if even { (Cr, 1500.0) } else { (Cb, 2300.0) };
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.009 },
                Seg::Tone { hz: 1500.0, dur: 0.003 },
                Seg::Scan { chan: Y, width: w, px: 0.000_275 },
                Seg::Tone { hz: sep_hz, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan, width: cw, px: 0.000_275 },
            ]
        }
        SstvMode::WraaseSc2_180 | SstvMode::WraaseSc2_120 => {
            // Wraase SC-2: sync, a short porch, then R, G, B at full width and
            // no separators between them — the one common family that sends
            // red first.
            // Scan time / width, derived from the published line times so the
            // total comes out exactly: SC-2 180 is three 235.000 ms scans,
            // SC-2 120 three of 156.502506 ms. (slowrx's own pixel times are
            // rounded and reproduce neither of its own line times, which is
            // the sort of thing only a test on the total ever notices.)
            let px =
                if mode == SstvMode::WraaseSc2_180 { 0.235 / 320.0 } else { 0.156_502_506 / 320.0 };
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.005_522_5 },
                Seg::Tone { hz: 1500.0, dur: 0.000_5 },
                Seg::Scan { chan: R, width: w, px },
                Seg::Scan { chan: G, width: w, px },
                Seg::Scan { chan: B, width: w, px },
            ]
        }
        SstvMode::Pd50
        | SstvMode::Pd90
        | SstvMode::Pd120
        | SstvMode::Pd160
        | SstvMode::Pd180
        | SstvMode::Pd240
        | SstvMode::Pd290 => {
            // Two image rows per sync: this row's luma, then one pair of
            // chroma scans shared between the two, then the next row's luma.
            // The chroma is full width here, not halved — PD saves its air
            // time vertically rather than horizontally.
            let px = pd_pixel_time(mode);
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.020 },
                Seg::Tone { hz: 1500.0, dur: 0.002_08 },
                Seg::Scan { chan: Y, width: w, px },
                Seg::Scan { chan: Cr, width: w, px },
                Seg::Scan { chan: Cb, width: w, px },
                Seg::Scan { chan: Y2, width: w, px },
            ]
        }
    }
}

// BT.601-ish YUV used by the Robot modes (MMSSTV coefficients).
fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let (r, g, b) = (r as f64, g as f64, b as f64);
    let y = 16.0 + (65.738 * r + 129.057 * g + 25.064 * b) / 256.0;
    let cr = 128.0 + (112.439 * r - 94.154 * g - 18.285 * b) / 256.0;
    let cb = 128.0 + (-37.945 * r - 74.494 * g + 112.439 * b) / 256.0;
    (
        y.round().clamp(0.0, 255.0) as u8,
        cr.round().clamp(0.0, 255.0) as u8,
        cb.round().clamp(0.0, 255.0) as u8,
    )
}

fn yuv_to_rgb(y: u8, cr: u8, cb: u8) -> (u8, u8, u8) {
    let y = y as f64 - 16.0;
    let cr = cr as f64 - 128.0;
    let cb = cb as f64 - 128.0;
    let r = 1.164 * y + 1.596 * cr;
    let g = 1.164 * y - 0.392 * cb - 0.813 * cr;
    let b = 1.164 * y + 2.017 * cb;
    (
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
    )
}

// ─────────────────────────────── transmit ──────────────────────────────

/// SSTV transmitter: turns an RGB image into a stream of audio samples.
pub struct SstvTx {
    rate: f64,
    /// Flattened plan of (frequency, sample-count) tone runs. Scans are expanded
    /// to one entry per pixel up front — a 320×256 image is ~250 k entries, a
    /// few MB, produced once per transmission.
    plan: Vec<(f64, u32)>,
    idx: usize,
    left: u32,
    cur_hz: f64,
    phase: f64,
    total: u64,
    done: u64,
}

impl SstvTx {
    /// Build a transmitter for `mode` from interleaved RGB (`rgb.len() == w*h*3`)
    /// at output sample `rate`, with an optional transmit clock trim `ppm`
    /// (parts-per-million; stretches/compresses the image time-scale to null out
    /// slant against a receiver whose clock differs — tone frequencies are
    /// unaffected).
    pub fn new(mode: SstvMode, rgb: &[u8], w: u16, h: u16, rate: f64, ppm: f32) -> Self {
        let mut plan: Vec<(f64, u32)> = Vec::new();
        // Cumulative-exact sample clock: derive each element's integer sample
        // count from the running fractional time, so per-element rounding never
        // accumulates into image slant (e.g. Scottie 1's 0.432 ms pixel is
        // 20.736 samples at 48 kHz — rounding each to 21 would drift +1.3%).
        // The timing rate carries the ppm trim; the phase accumulator (in
        // `next_block`) uses the true `rate`, so the tone frequencies stay exact.
        let timing_rate = rate * (1.0 + ppm as f64 / 1_000_000.0);
        let mut emitted: i64 = 0;
        let mut t_exact: f64 = 0.0;
        let mut push = |plan: &mut Vec<(f64, u32)>, hz: f64, dur: f64| {
            t_exact += dur * timing_rate;
            let target = t_exact.round() as i64;
            let n = (target - emitted).max(0);
            emitted = target;
            if n > 0 {
                plan.push((hz, n as u32));
            }
        };
        let px_at = |x: usize, y: usize| -> (u8, u8, u8) {
            let i = (y * w as usize + x) * 3;
            (rgb[i], rgb[i + 1], rgb[i + 2])
        };

        // VIS calibration header.
        push(&mut plan, VIS_LEADER_HZ, 0.300);
        push(&mut plan, SYNC_HZ, 0.010);
        push(&mut plan, VIS_LEADER_HZ, 0.300);
        push(&mut plan, SYNC_HZ, 0.030); // start bit
        let code = mode.vis_code();
        let mut parity = 0u8;
        for bit in 0..7 {
            let one = (code >> bit) & 1 == 1;
            parity ^= one as u8;
            push(&mut plan, if one { VIS_BIT1_HZ } else { VIS_BIT0_HZ }, 0.030);
        }
        push(&mut plan, if parity == 1 { VIS_BIT1_HZ } else { VIS_BIT0_HZ }, 0.030);
        push(&mut plan, SYNC_HZ, 0.030); // stop bit

        // Scottie sends a 9 ms starting sync before the very first line.
        if matches!(mode, SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx) {
            push(&mut plan, SYNC_HZ, 0.009);
        }

        // One pass per *transmitted* line, which is two image rows in the PD
        // family and one everywhere else.
        let rows = mode.rows_per_line().max(1) as usize;
        for y in (0..h as usize).step_by(rows) {
            // The second row a PD line carries; the last line of an
            // odd-height picture repeats the first rather than sending a row
            // that is not there.
            let y2 = (y + 1).min(h as usize - 1);
            for seg in line_segments(mode, w, y as u16) {
                match seg {
                    Seg::Tone { hz, dur } => push(&mut plan, hz, dur),
                    Seg::Scan { chan, width, px } => {
                        for x in 0..width as usize {
                            // Map the (possibly subsampled) scan x to a source column.
                            let sx = if width == w { x } else { (x * 2).min(w as usize - 1) };
                            let (r, g, b) = px_at(sx, y);
                            let v = match chan {
                                Chan::R => r,
                                Chan::G => g,
                                Chan::B => b,
                                Chan::Y => rgb_to_yuv(r, g, b).0,
                                Chan::Y2 => {
                                    let (r2, g2, b2) = px_at(sx, y2);
                                    rgb_to_yuv(r2, g2, b2).0
                                }
                                // A PD line's chroma belongs to both of its
                                // rows, so it is the mean of the two rather
                                // than the first one's — which is what makes
                                // the pair look like one picture instead of
                                // combed. On every other mode `rows` is 1 and
                                // this averages a row with itself.
                                Chan::Cr | Chan::Cb => {
                                    let (r2, g2, b2) = px_at(sx, y2);
                                    let a = rgb_to_yuv(r, g, b);
                                    let c = rgb_to_yuv(r2, g2, b2);
                                    let (u, v2) =
                                        if chan == Chan::Cr { (a.1, c.1) } else { (a.2, c.2) };
                                    ((u as u16 + v2 as u16) / 2) as u8
                                }
                            };
                            // Cumulative-exact per-pixel timing (no drift/slant).
                            push(&mut plan, value_to_hz(v), px);
                        }
                    }
                }
            }
        }

        let total: u64 = plan.iter().map(|&(_, n)| n as u64).sum();
        SstvTx { rate, plan, idx: 0, left: 0, cur_hz: 0.0, phase: 0.0, total, done: 0 }
    }

    /// Fill `out` with audio; returns the number of real samples written before
    /// the transmission ended (the rest of `out`, if any, is zeroed).
    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for s in out.iter_mut() {
            if self.left == 0 {
                match self.plan.get(self.idx) {
                    Some(&(hz, n)) => {
                        self.cur_hz = hz;
                        self.left = n;
                        self.idx += 1;
                    }
                    None => {
                        *s = 0.0;
                        continue;
                    }
                }
            }
            self.phase += TAU * self.cur_hz / self.rate;
            if self.phase > TAU {
                self.phase -= TAU;
            }
            *s = (self.phase.sin() as f32) * 0.5;
            self.left -= 1;
            self.done += 1;
            written += 1;
        }
        written
    }

    /// True once every planned sample has been emitted.
    pub fn done(&self) -> bool {
        self.idx >= self.plan.len() && self.left == 0
    }

    /// Total number of audio samples this transmission will emit.
    pub fn total_samples(&self) -> u64 {
        self.total
    }

    /// Transmission progress, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.total == 0 { 1.0 } else { (self.done as f32 / self.total as f32).clamp(0.0, 1.0) }
    }

    /// Append an FSK identification for `id` to the end of the transmission.
    ///
    /// Nothing is added for an empty (or unsendable) `id`, so a station with no
    /// callsign configured transmits exactly what it did before.
    ///
    /// After the picture rather than before it, which is where every program
    /// that sends one puts it: the receiver has by then finished decoding and
    /// gone back to hunting, and an unattended repeater has the whole frame in
    /// hand before it is told whose it was.
    ///
    /// The ID is *not* stretched by the transmit clock trim the picture carries.
    /// That trim exists to null out image slant against a particular receiver's
    /// sound card, which is a property of the two-dimensional picture; a bit
    /// stream a hundredth of a percent long decodes the same either way, and
    /// nothing on the far end is measuring it against the image.
    #[must_use]
    pub fn with_fsk_id(mut self, id: &str) -> Self {
        let symbols = fsk_id_symbols(id);
        if symbols.is_empty() {
            return self;
        }
        // The same cumulative-exact clock the picture plan uses, restarted for
        // the ID: each element's sample count comes from the running total, so
        // rounding 22 ms at 48 kHz (1056 samples exactly) or at 44.1 kHz
        // (970.2) cannot accumulate into a bit-clock drift the far end has to
        // chase.
        let mut emitted: i64 = 0;
        let mut t_exact: f64 = 0.0;
        let mut push = |plan: &mut Vec<(f64, u32)>, hz: f64, dur: f64| {
            t_exact += dur * self.rate;
            let target = t_exact.round() as i64;
            let n = (target - emitted).max(0);
            emitted = target;
            if n > 0 {
                plan.push((hz, n as u32));
            }
        };
        push(&mut self.plan, FSKID_LEADER_HZ, FSKID_LEADER_S);
        push(&mut self.plan, FSKID_ZERO_HZ, FSKID_SYNC_S);
        // The start bit. It is on the same tone as a `1`, so what marks it is
        // the transition out of the 100 ms block above — which is exactly what
        // the receiver times the rest of the stream from.
        push(&mut self.plan, FSKID_ONE_HZ, FSKID_BIT_S);
        for sym in symbols {
            for bit in (0..6).rev() {
                let one = (sym >> bit) & 1 == 1;
                push(&mut self.plan, if one { FSKID_ONE_HZ } else { FSKID_ZERO_HZ }, FSKID_BIT_S);
            }
        }
        self.total = self.plan.iter().map(|&(_, n)| n as u64).sum();
        self
    }
}

// ─────────────────────────────── receive ───────────────────────────────

/// A decoded output from the receiver.
pub enum SstvEvent {
    /// A VIS header identified the mode; a new image is starting.
    ModeDetected(SstvMode),
    /// A finished scan line: `rgb` is `3 * width` bytes at row `y`.
    Line { y: u16, rgb: Vec<u8> },
    /// The current image reached its last line.
    ImageComplete,
    /// A station identified itself in tones after its picture — the FSK ID
    /// every SSTV program and unattended repeater sends and reads.
    FskId(String),
    /// A header arrived, with good parity, for a mode this decoder does not
    /// have. `name` is the mode where the code is an assigned one.
    ///
    /// Worth an event rather than a discard: at that moment the receiver knows
    /// precisely what is being sent and precisely why no picture is coming,
    /// and it is the only moment anything does. Silence here is what made
    /// issue #421 read as a broken decoder rather than as an unimplemented
    /// mode.
    UnsupportedMode { code: u8, name: Option<&'static str> },
}

#[derive(PartialEq)]
enum RxPhase {
    /// Hunting for the VIS leader / decoding VIS.
    Hunt,
    /// Decoding image lines for `mode`.
    Image,
}

/// SSTV receiver. Feed audio with [`SstvRx::process`]; it emits [`SstvEvent`]s.
pub struct SstvRx {
    rate: f64,
    // Down-mix + baseband filter for the discriminator.
    mix_ph: f32,
    mix_inc: f32,
    lpf: ComplexFir,
    prev: Complex32,
    // Instantaneous frequency (Hz), lightly smoothed.
    inst_hz: f64,
    // Smoothed raw-input level (mean |audio|) for the UI activity meter.
    in_level: f32,
    have_prev: bool,

    phase: RxPhase,
    mode: SstvMode,
    // Rolling ring of recent instantaneous-frequency samples, so we can look
    // back over a whole line once its trailing sync arrives.
    hist: Vec<f64>,
    // Samples of history to guarantee (~1.2 s); `hist` runs a little past it
    // between compactions. See `push_hist`.
    hist_cap: usize,
    // Absolute sample index of hist[0].
    hist_base: u64,
    sample_idx: u64,

    // VIS bit accumulation.
    vis_state: VisState,
    // The FSK ID hunt, which runs whenever no picture is being decoded.
    fsk_state: FskIdState,

    // Image decode bookkeeping.
    line: u16,
    // Sample index where the current line is expected to start. Fractional:
    // rounding it to a sample every line is itself a slant (Scottie 1's line
    // is 20 554.56 samples at 48 kHz, and dropping the .56 every line walks
    // the picture seven pixels sideways by the bottom).
    line_start: f64,
    // Length of the current line, cached because `step_image` runs per sample.
    line_samples: f64,
    // The line through this picture's sync pulses so far.
    fit: LineFit,
    // Where each transmitted line was actually decoded from.
    used_starts: Vec<f64>,
    // Lines in a row whose sync pulse was not found.
    misses: u32,
    // The whole picture's frequency track, for the second pass at the end.
    track: FreqTrack,
    // Robot 4:2:0 chroma carried between lines.
    last_cr: Vec<u8>,
    last_cb: Vec<u8>,

    // Free-run (decode without VIS): lock onto a regular 1200 Hz sync cadence.
    // `expected` = a specific operator-selected mode, or `None` for auto (match
    // the cadence + sync length against every mode).
    expected: Option<SstvMode>,
    // Recent sync pulses as (centre sample, pulse length in samples).
    sync_hist: Vec<(u64, u32)>,
    // The picture being decoded was locked on its header, so its line
    // numbering is the sender's. A free-run lock guesses it.
    numbered: bool,
}

/// The hunt for an FSK ID: what has been seen of one so far.
///
/// Deliberately a level trigger followed by a fixed clock, not a per-bit
/// tracking loop. The whole ID is under three seconds and the transmitter's
/// bit clock is its sound card's, so there is nothing to track — and a receiver
/// that resynchronised on every edge would be one more thing to go wrong on a
/// stream that is already protected by a header, a terminator and a checksum.
struct FskIdState {
    /// Consecutive samples of the 100 ms block that arms the hunt.
    arm_run: u32,
    /// Set once that block has run long enough to be one.
    armed: bool,
    /// Sample index of the transition out of it — bit zero's leading edge, and
    /// the origin every bit below is timed from.
    start: Option<u64>,
    /// Data bits taken so far (the start bit is not among them).
    bits: Vec<bool>,
    /// How many of them have been sampled, so each is taken exactly once.
    taken: u32,
    /// Consecutive samples since arming that were on neither tone.
    gap: u32,
}

impl FskIdState {
    fn reset() -> Self {
        FskIdState { arm_run: 0, armed: false, start: None, bits: Vec::new(), taken: 0, gap: 0 }
    }
}

struct VisState {
    // Running count of leader samples, less three for every sample that was
    // not — see `step_hunt`.
    leader: u32,
    // A full (>120 ms) leader has been seen recently.
    leader_seen: bool,
    // Samples of the last `LEADER_WIN_S` near the leader tone, and of the
    // last `EDGE_WIN_S` below `EDGE_SLICE_HZ`.
    near_leader: u32,
    low: u32,
    // Samples of the last `SYNC_WIN_S` below `SYNC_SLICE_HZ`, and the sync
    // pulse that window is in, if it is in one: the first and the latest
    // sample it was.
    sync_low: u32,
    run: Option<(u64, u64)>,
    // The first sample the hunt counted. Older ones leaving the windows were
    // never added to them, so they are not taken away either.
    from: u64,
    // The last `EDGE_WIN_S` was mostly low (edge detection).
    was_low: bool,
    // Candidate start-bit sample indices awaiting a decode attempt. Both the
    // 10 ms break and the real 30 ms start bit become candidates; the break's
    // bit slots are up at the leader, so it is rejected and the real start
    // bit wins.
    cands: Vec<u64>,
}

impl VisState {
    fn reset(from: u64) -> Self {
        VisState {
            leader: 0,
            leader_seen: false,
            near_leader: 0,
            low: 0,
            sync_low: 0,
            run: None,
            from,
            was_low: false,
            cands: Vec::new(),
        }
    }
}

/// How far from a tone a sample may be and still count as on it, when a
/// window is judged by how many of its samples are.
const TONE_NEAR_HZ: f64 = 150.0;

/// The share of a window's samples that must be near the leader for the
/// window to be leader. Noise cannot be told from the leader by its middle —
/// its frequencies centre on the 1900 Hz the receiver mixes down by, which is
/// the leader's — only by its spread: it puts about a quarter of its samples
/// within [`TONE_NEAR_HZ`] of 1900, and a leader 4 dB above it half of them.
const LEADER_SHARE: f64 = 0.45;

/// The window the leader is judged over: long enough that noise does not
/// often put [`LEADER_SHARE`] of it near 1900 by chance, short against the
/// 300 ms leader.
const LEADER_WIN_S: f64 = 0.020;

/// A start bit's leading edge is where the median of the last
/// `EDGE_WIN_S` falls below `EDGE_SLICE_HZ`, midway between the leader and the
/// sync tone — the median, not the sample, because in noise single samples
/// cross any slice all the time. Short, because a VIS's timing is taken from
/// this edge.
const EDGE_WIN_S: f64 = 0.003;
const EDGE_SLICE_HZ: f64 = 1550.0;

/// The free-run hunt's sync detector: a window `SYNC_WIN_S` long is in a sync
/// pulse while at least `SYNC_SHARE` of it is below `SYNC_SLICE_HZ`.
///
/// Neither a single sample nor the median. In noise the discriminator's
/// readings of a tone are pulled towards the 1900 Hz the receiver mixes down
/// by, so at 0 dB the median of the 1200 Hz sync sits above any slice that
/// black (1500 Hz) does not reach too; but its lower readings stay low, and
/// 40 % of a 4 ms window below 1400 Hz holds on sync two thirds of the time at
/// 0 dB and nine in ten at 2 dB, on black one in twenty and on noise never.
/// Every one of the previous detector's samples had to be within 150 Hz of
/// 1200, and one that was not ended the pulse.
const SYNC_WIN_S: f64 = 0.004;
const SYNC_SHARE: f64 = 0.4;
const SYNC_SLICE_HZ: f64 = 1400.0;

/// How far a gap between two sync pulses may be off a whole number of lines:
/// this much for where each pulse was measured, and this fraction of the gap
/// for the clocks — 5000 ppm, past any sound card.
const CADENCE_TOL_S: f64 = 0.002;
const CADENCE_DRIFT: f64 = 0.005;

/// A free-run lock takes a chain of this many line gaps, one pulse to the
/// next, on the mode's cadence (see `cadence_chain`): four pulses. Three,
/// which two gaps anywhere in the list was meant to be, let noise lock.
const FREERUN_LINKS: u32 = 3;

/// A pulse's length may be this far off its mode's sync, as a fraction of it.
const SYNC_LEN_TOL: f64 = 0.4;

/// Recent sync pulses kept for the cadence: enough for a chain of
/// `FREERUN_LINKS` with a lost pulse or two and some noise between.
const SYNC_HIST_LEN: usize = 16;

/// A dip out of sync shorter than this is noise inside a pulse, not the gap
/// between two.
const SYNC_MERGE_S: f64 = 0.001;

/// Where in its readings a VIS bit is read: Open-SSTV's 30th percentile,
/// which at 4 dB decodes 25 headers in 32 against the median's 14 and has
/// not been seen to misread one more.
const VIS_QUANTILE: f64 = 0.3;

/// A start or stop bit's reading must be this close to the sync tone.
const VIS_SYNC_TOL_HZ: f64 = 150.0;

/// Start bits waiting to be decoded at once. A header produces two (the break
/// and the start bit itself); more is noise wandering across the slice, and
/// the list refuses more rather than dropping the oldest — which was the real
/// start bit, every time, whenever the noise made a few more.
const MAX_VIS_CANDS: usize = 64;

/// The median of `hz` over the middle 60 % of the `len` samples from `from`:
/// what a steady tone there reads, out of reach of the transitions either side
/// of it and of the clicks FM noise throws, which a mean would average in.
fn tone_median(hz: impl Fn(i64) -> f64, from: f64, len: f64) -> f64 {
    tone_quantile(hz, from, len, 0.5)
}

/// The `q` quantile of `hz` over the same middle 60 %, for tones below the
/// 1900 Hz the receiver mixes down by. Noise pulls a tone's readings towards
/// 1900, and the further the tone is from it the harder: at 4 dB the median of
/// the VIS's 1100 Hz bits lands above the median of its 1200 Hz start bit
/// often enough to misread one in two headers. The lower readings are the
/// ones noise has moved least.
fn tone_quantile(hz: impl Fn(i64) -> f64, from: f64, len: f64, q: f64) -> f64 {
    let lo = (from + 0.2 * len).round() as i64;
    let hi = ((from + 0.8 * len).round() as i64).max(lo + 1);
    let mut v: Vec<f64> = (lo..hi).map(hz).collect();
    let at = ((v.len() as f64 * q) as usize).min(v.len() - 1);
    *v.select_nth_unstable_by(at, f64::total_cmp).1
}

impl SstvRx {
    pub fn new(rate: f64) -> Self {
        let mix_hz = 1900.0f32;
        // Keep ~1.2 s of history (enough for the slowest line + sync search).
        let hist_cap = (rate * 1.2) as usize;
        SstvRx {
            rate,
            mix_ph: 0.0,
            mix_inc: (TAU as f32) * mix_hz / rate as f32,
            lpf: ComplexFir::new(bandpass_taps(129, -1100.0, 1100.0, rate)),
            prev: Complex32::new(0.0, 0.0),
            inst_hz: 1900.0,
            in_level: 0.0,
            have_prev: false,
            phase: RxPhase::Hunt,
            mode: SstvMode::Scottie1,
            hist: Vec::with_capacity(hist_cap + hist_cap / 4 + 1),
            hist_cap,
            hist_base: 0,
            sample_idx: 0,
            vis_state: VisState::reset(0),
            fsk_state: FskIdState::reset(),
            line: 0,
            line_start: 0.0,
            line_samples: 0.0,
            fit: LineFit::new(1.0, 0.0),
            used_starts: Vec::new(),
            misses: 0,
            track: FreqTrack::new(),
            last_cr: Vec::new(),
            last_cb: Vec::new(),
            expected: None,
            sync_hist: Vec::new(),
            numbered: false,
        }
    }

    /// Set the mode used for free-run (no-VIS) decoding, or `None` for auto
    /// (detect the mode from the sync cadence).
    pub fn set_expected(&mut self, mode: Option<SstvMode>) {
        self.expected = mode;
    }

    /// Abandon whatever is being decoded and go back to hunting for a header.
    ///
    /// An SSTV receiver that has locked on is committed for the length of the
    /// mode it locked on to, and the slow modes are long: Scottie DX is four
    /// and a half minutes, PD290 nearly five. A false VIS — or a real one for a
    /// mode the transmitting station did not actually send — therefore takes
    /// the receiver off the air for as long as it takes to run out, and on a
    /// transponder where pictures follow one another that is several missed.
    /// This is the way back (issue #397).
    ///
    /// The history goes with it, deliberately. It holds the last 1.2 s of the
    /// picture being abandoned, sync pulses and all, and the free-run hunt
    /// reads exactly that — left in place it would re-lock on the cadence of
    /// the transmission the operator has just asked to be rid of. The
    /// front-end filter, the level meter and the operator's mode selection are
    /// *not* touched: none of them is about this picture.
    pub fn restart(&mut self) {
        self.phase = RxPhase::Hunt;
        self.vis_state = VisState::reset(self.sample_idx);
        self.fsk_state = FskIdState::reset();
        self.line = 0;
        self.line_start = 0.0;
        self.line_samples = 0.0;
        self.used_starts.clear();
        self.misses = 0;
        self.track.clear();
        self.last_cr.clear();
        self.last_cb.clear();
        self.sync_hist.clear();
        self.hist.clear();
        self.hist_base = self.sample_idx;
    }

    /// The mode currently being decoded (or last detected).
    pub fn mode(&self) -> SstvMode {
        self.mode
    }

    /// Smoothed raw-input level (mean |sample|), for a UI activity meter so the
    /// operator can set their receive gain.
    pub fn level(&self) -> f32 {
        self.in_level
    }

    /// True while an image is being decoded (VIS locked).
    pub fn receiving(&self) -> bool {
        self.phase == RxPhase::Image
    }

    /// Fraction of the current image decoded, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.phase != RxPhase::Image {
            return 0.0;
        }
        let (_, h) = self.mode.dimensions();
        (self.line as f32 / h.max(1) as f32).clamp(0.0, 1.0)
    }

    /// Feed audio; push any decoded events.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<SstvEvent>) {
        // Down-mix by 1900 Hz to complex baseband, then low-pass the whole block.
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            self.in_level += 0.001 * (a.abs() - self.in_level);
            let z = Complex32::new(a * self.mix_ph.cos(), -a * self.mix_ph.sin());
            self.mix_ph += self.mix_inc;
            if self.mix_ph > std::f32::consts::TAU {
                self.mix_ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        for z in bb {
            // Instantaneous frequency via the discriminator.
            let raw_hz = if self.have_prev {
                let d = z * self.prev.conj();
                1900.0 + (d.arg() as f64) * self.rate / TAU
            } else {
                1900.0
            };
            self.prev = z;
            self.have_prev = true;
            self.inst_hz += 0.5 * (raw_hz - self.inst_hz);

            self.push_hist(self.inst_hz);
            if self.phase == RxPhase::Image {
                self.track.push(self.inst_hz);
            }
            self.sample_idx += 1;

            match self.phase {
                RxPhase::Hunt => self.step_hunt(out),
                RxPhase::Image => self.step_image(out),
            }
        }
    }

    fn push_hist(&mut self, hz: f64) {
        self.hist.push(hz);
        // Compact in blocks, never per sample. Trimming one element per push
        // memmoves the whole ~1.2 s buffer (≈460 kB at 48 kHz) on every sample
        // — several GB/s of pointless copying that dwarfs the rest of the
        // demodulator and starves the audio and display running on the same
        // thread. Letting it overshoot by a quarter amortises that away; the
        // guarantee callers rely on (at least `hist_cap` samples of history) is
        // unchanged, since the buffer only ever holds *more* than before.
        if self.hist.len() > self.hist_cap + self.hist_cap / 4 {
            let drop = self.hist.len() - self.hist_cap;
            self.hist.drain(0..drop);
            self.hist_base += drop as u64;
        }
    }

    fn hz_at(&self, idx: u64) -> f64 {
        hz_in(&self.hist, self.hist_base, idx as i64)
    }

    // ── FSK ID detection ──
    //
    // Runs alongside the VIS hunt rather than after the picture: the two cannot
    // be confused (VIS is 1100/1300 Hz around a 1200 Hz sync, the ID is
    // 1900/2100 with no sync at all), and hunting for it the whole time is what
    // lets a receiver tuned in late — or one whose picture decode never
    // started — still learn who is transmitting.
    fn step_fsk_id(&mut self, out: &mut Vec<SstvEvent>) {
        let near = |a: f64, b: f64| (a - b).abs() < 100.0;
        let bit_samples = FSKID_BIT_S * self.rate;

        // Arming: the 100 ms block on the zero tone. Two thirds of it is enough
        // — the leading edge is where the picture's last pixel ends, and on a
        // real signal that boundary is not clean.
        if self.fsk_state.start.is_none() {
            if near(self.inst_hz, FSKID_ZERO_HZ) {
                self.fsk_state.arm_run += 1;
                self.fsk_state.gap = 0;
                if self.fsk_state.arm_run as f64 > 0.066 * self.rate {
                    self.fsk_state.armed = true;
                }
                return;
            }
            if !self.fsk_state.armed {
                self.fsk_state.arm_run = 0;
                return;
            }
            // Armed and now on the one tone: this is the start bit's leading
            // edge, and the clock for everything after it.
            if near(self.inst_hz, FSKID_ONE_HZ) {
                self.fsk_state.start = Some(self.sample_idx);
                self.fsk_state.bits.clear();
                self.fsk_state.taken = 0;
                return;
            }
            // In between the two. A 200 Hz shift does not arrive instantly —
            // the discriminator and the filter ahead of it take a moment to
            // follow it — so the samples spanning the edge belong to neither
            // tone, and treating one of them as "this was not an ID after all"
            // threw the arming away a few samples before the start bit it was
            // waiting for. Anything longer than a bit or two is a real signal
            // that simply was not one.
            self.fsk_state.gap += 1;
            if self.fsk_state.gap as f64 > 3.0 * bit_samples {
                self.fsk_state = FskIdState::reset();
            }
            return;
        }

        let Some(start) = self.fsk_state.start else { return };
        // Bit 0 is the start bit and carries nothing, so the data begins at 1.
        let k = self.fsk_state.taken + 1;
        let from = start as f64 + k as f64 * bit_samples;
        // Read once the part of the bit that is read has arrived — no later,
        // since the last bit is the last thing on the air. Strictly greater:
        // `sample_idx` is one *past* the newest sample in the history (it is
        // bumped before the step runs), so waiting only for equality asks
        // `hz_at` for a sample that has not been stored yet — and its "nothing
        // here" answer is 1900 Hz, which is a perfectly good `1`. Every bit
        // read as one, and every ID decoded as $3F $3F $3F…
        if (self.sample_idx as f64) <= from + 0.8 * bit_samples + 1.0 {
            return;
        }
        self.fsk_state.taken += 1;
        // The whole bit, not the sample at its middle, which in noise is as
        // likely to be a click as the tone.
        let (hist, base) = (&self.hist, self.hist_base);
        let hz = tone_median(|i| hz_in(hist, base, i), from, bit_samples);
        let mid = (FSKID_ONE_HZ + FSKID_ZERO_HZ) / 2.0;
        if hz < mid && near(hz, FSKID_ONE_HZ) {
            self.fsk_state.bits.push(true);
        } else if hz >= mid && near(hz, FSKID_ZERO_HZ) {
            self.fsk_state.bits.push(false);
        } else {
            // Neither tone: the stream has ended (or was never one). Give up
            // and go back to arming rather than keying noise into the symbols.
            self.fsk_state = FskIdState::reset();
            return;
        }

        // A whole symbol has arrived; see whether the stream so far is an ID.
        if self.fsk_state.bits.len() % 6 != 0 {
            return;
        }
        let symbols: Vec<u8> = self
            .fsk_state
            .bits
            .chunks_exact(6)
            .map(|c| c.iter().fold(0u8, |a, &b| (a << 1) | u8::from(b)))
            .collect();
        // A header that is not the header can never become one, so a stream
        // that starts wrong is dropped at the first symbol instead of being
        // carried for the length of a callsign.
        if symbols[0] != FSKID_HEAD {
            self.fsk_state = FskIdState::reset();
            return;
        }
        if let Some(text) = fsk_id_text(&symbols) {
            out.push(SstvEvent::FskId(text));
            self.fsk_state = FskIdState::reset();
            return;
        }
        // Nothing yet, and nothing that can still become something: the
        // longest legal ID is the header, the cap, the terminator and the sum.
        if symbols.len() > FSKID_MAX_CHARS + 3 {
            self.fsk_state = FskIdState::reset();
        }
    }

    // ── VIS detection ──

    /// Slide the hunt's windows on by the sample just stored.
    fn slide_vis_windows(&mut self) {
        let lead_n = (LEADER_WIN_S * self.rate) as u64;
        let edge_n = (EDGE_WIN_S * self.rate) as u64;
        let near_leader = |hz: f64| (hz - VIS_LEADER_HZ).abs() < TONE_NEAR_HZ;
        let low = |hz: f64| hz < EDGE_SLICE_HZ;
        let newest = self.sample_idx - 1;
        // The samples leaving each window, if they were ever counted into it.
        let from = self.vis_state.from;
        let leaving = |n: u64| newest.checked_sub(n).filter(|&i| i >= from).map(|i| self.hz_at(i));
        let sync_n = (SYNC_WIN_S * self.rate) as u64;
        let sync_low = |hz: f64| hz < SYNC_SLICE_HZ;
        let (left_lead, left_edge, left_sync) = (leaving(lead_n), leaving(edge_n), leaving(sync_n));
        let vs = &mut self.vis_state;
        vs.near_leader += u32::from(near_leader(self.inst_hz));
        vs.low += u32::from(low(self.inst_hz));
        vs.sync_low += u32::from(sync_low(self.inst_hz));
        if let Some(hz) = left_sync {
            vs.sync_low -= u32::from(sync_low(hz));
        }
        if let Some(hz) = left_lead {
            vs.near_leader -= u32::from(near_leader(hz));
        }
        if let Some(hz) = left_edge {
            vs.low -= u32::from(low(hz));
        }
    }

    fn step_hunt(&mut self, out: &mut Vec<SstvEvent>) {
        self.step_fsk_id(out);
        self.slide_vis_windows();
        let lead_n = (LEADER_WIN_S * self.rate) as u64 as f64;
        let edge_n = (EDGE_WIN_S * self.rate) as u64;
        let is_leader = self.vis_state.near_leader as f64 >= LEADER_SHARE * lead_n;
        let is_low = 2 * self.vis_state.low as u64 >= edge_n;
        // Tolerant leader accumulator: brief noise glitches decrement rather than
        // reset the run, so a real ~300 ms leader still arms through hiss. No
        // amplitude gate — the discriminator is level-independent, so a clean but
        // quiet signal must still decode; the VIS code + parity check rejects
        // noise. The leader is judged a window at a time (see `LEADER_SHARE`),
        // because in noise single samples are no evidence either way.
        if is_leader {
            self.vis_state.leader = (self.vis_state.leader + 1).min((self.rate) as u32);
            if self.vis_state.leader as f64 > 0.12 * self.rate {
                self.vis_state.leader_seen = true;
            }
        } else {
            self.vis_state.leader = self.vis_state.leader.saturating_sub(3);
            // A start bit has to follow a leader that is still *recent*. The
            // flag used to be sticky for the whole hunt, so every 1200 Hz sync
            // pulse of every picture after it went on being offered as a
            // candidate header — harmless while an unrecognised code was
            // dropped in silence, and a stream of false "unsupported mode"
            // reports once one is not. The accumulator drains in about 100 ms,
            // which is well inside the 30 ms between the leader and the start
            // bit it has to survive.
            if self.vis_state.leader == 0 {
                self.vis_state.leader_seen = false;
            }
        }
        let bit = 0.030 * self.rate;
        // The fall from the leader into a start bit → candidate. The window
        // is half full of low samples half a window after the edge.
        if is_low && !self.vis_state.was_low && self.vis_state.leader_seen {
            let at = self.sample_idx.saturating_sub(edge_n / 2);
            // One candidate per edge: noise walking the median back and forth
            // across the slice on the way down makes several, all the same one
            // to a decode that reads the middle of each bit.
            let fresh =
                self.vis_state.cands.last().is_none_or(|&c| at as f64 >= c as f64 + 0.25 * bit);
            if fresh && self.vis_state.cands.len() < MAX_VIS_CANDS {
                self.vis_state.cands.push(at);
            }
        }
        self.vis_state.was_low = is_low;

        // Try the oldest candidate once its stop bit has arrived.
        if let Some(&start) = self.vis_state.cands.first()
            && (self.sample_idx as f64) >= start as f64 + 10.0 * bit
        {
            self.vis_state.cands.remove(0);
            let (hist, base) = (&self.hist, self.hist_base);
            // Slot `k` of the header, counting the start bit as 0: the lower
            // readings of its middle, see `tone_quantile`.
            let slot = |k: f64| {
                tone_quantile(|i| hz_in(hist, base, i), start as f64 + k * bit, bit, VIS_QUANTILE)
            };
            // The start bit is the reference the data bits are read
            // against: 1100 Hz is a one and 1300 a zero, and whatever the
            // noise or the tuning has done to 1200 it has done to them too.
            let reference = slot(0.0);
            let mut code = 0u8;
            let mut parity = 0u8;
            // Every VIS bit is 1100 or 1300 Hz. A candidate whose bit
            // slots are up at the 1900 Hz leader is not a header at all —
            // which is what the *break* pulse in the middle of the
            // calibration header looks like, since it is a fall into
            // 1200 Hz after a leader just like the start bit is. It
            // used to be decoded anyway, reading the leader as eight zero
            // bits with matching parity; harmless while an unknown code
            // was silently dropped, and a false "unsupported mode" report
            // the moment one is not.
            let mut looks_like_vis = (reference - SYNC_HZ).abs() < VIS_SYNC_TOL_HZ;
            for b in 0..7 {
                let hz = slot(1.0 + b as f64);
                if hz > 1600.0 {
                    looks_like_vis = false;
                }
                if hz < reference {
                    code |= 1 << b; // 1100 Hz = 1
                    parity ^= 1;
                }
            }
            let phz = slot(8.0);
            if phz > 1600.0 {
                looks_like_vis = false;
            }
            let pbit = if phz < reference { 1 } else { 0 };
            // The stop bit is back at 1200 Hz. Asked only before decoding
            // a picture, which is what a false header costs; a report of
            // an unrecognised one is cheap, and some of those (MMSSTV's
            // 16-bit codes) carry data where a 7-bit header has its stop.
            let stopped = (slot(9.0) - SYNC_HZ).abs() < VIS_SYNC_TOL_HZ;
            if looks_like_vis && parity == pbit {
                match SstvMode::from_vis(code) {
                    Some(mode) if stopped => {
                        // Image data begins after the stop bit (start + 10
                        // bits); Scottie prefixes a 9 ms starting sync
                        // before line 0.
                        let mut first = start as f64 + 10.0 * bit;
                        if matches!(
                            mode,
                            SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx
                        ) {
                            first += 0.009 * self.rate;
                        }
                        self.begin_image(mode, first as u64, true, out);
                    }
                    // A header that checks out for a mode we cannot draw.
                    // Said once per header rather than swallowed — see
                    // `SstvEvent::UnsupportedMode`. Code 0 is not a mode
                    // anyone has ever been assigned, so a candidate that
                    // reads as one is a misread and says nothing.
                    None if code != 0 && self.preceded_by_leader(start) => {
                        out.push(SstvEvent::UnsupportedMode {
                            code,
                            name: SstvMode::unsupported_name(code),
                        })
                    }
                    _ => {}
                }
            }
        }

        // No VIS yet? Try to lock onto the sync cadence of the selected mode.
        self.try_freerun(out);
    }

    /// Whether the 20 ms before `start` really is the 1900 Hz leader.
    ///
    /// Only asked before *reporting* an unrecognised code, never before
    /// decoding a recognised one: a missed report costs a line of explanation,
    /// and a missed decode costs the picture.
    ///
    /// A candidate is any fall into sync while a leader is in recent memory,
    /// and noise makes those too; some read as a code with matching parity.
    /// They are harmless as decodes (their code is not a mode) and actively
    /// wrong as reports, where the last one to arrive would overwrite the real
    /// mode's name. A start bit is preceded by the leader.
    ///
    /// Judged the way the hunt judges the leader, by the share of the window
    /// near it, so noise on a few samples does not cost the explanation.
    fn preceded_by_leader(&self, start: u64) -> bool {
        // Ending a little short of the edge, which the candidate's timing
        // only knows to within its own window.
        let to = start.saturating_sub((EDGE_WIN_S * self.rate) as u64);
        let n = (LEADER_WIN_S * self.rate) as u64;
        let near = (to.saturating_sub(n)..to)
            .filter(|&i| (self.hz_at(i) - VIS_LEADER_HZ).abs() < TONE_NEAR_HZ)
            .count();
        near as f64 >= LEADER_SHARE * n as f64
    }

    /// Total samples per scan line for `mode`, at line index `line` (only the
    /// Robot modes vary by line, and then only in which chroma they carry —
    /// the duration is the same either way).
    fn line_period_samples(&self, mode: SstvMode, line: u16) -> f64 {
        let (w, _) = mode.dimensions();
        line_segments(mode, w, line)
            .iter()
            .map(|s| match s {
                Seg::Tone { dur, .. } => *dur * self.rate,
                Seg::Scan { width, px, .. } => *width as f64 * *px * self.rate,
            })
            .sum()
    }

    /// How well the recent sync pulses fit `mode`'s cadence: the chain of
    /// them back from the newest, each a line — or two, one having been lost —
    /// before the one after it and as long as `mode`'s sync, with anything
    /// else between them passed over. Returns the chain's links, and how many
    /// of those were a single line.
    ///
    /// A chain, not a count of gaps anywhere in the list that are some whole
    /// number of lines: noise makes a pulse of a few milliseconds about once a
    /// second, and with any number of lines allowed across sixteen modes two
    /// such gaps turned up every twenty seconds or so — every one a picture's
    /// worth of receiver time lost to nothing.
    ///
    /// The tolerance is where a pulse can be measured (a couple of
    /// milliseconds in heavy noise) plus what the two stations' clocks can
    /// make of the gap between them. It was 4 % of a line, which is room for
    /// a different mode: two of Martin 2's lines are within 1.6 % of one of
    /// Martin 1's.
    fn cadence_chain(&self, mode: SstvMode) -> (u32, u32) {
        let period = self.line_period_samples(mode, 0);
        let (_, sdur) = self.sync_span(mode, mode.dimensions().0, 0);
        let fits = |len: u32| (len as f64 - sdur).abs() / sdur <= SYNC_LEN_TOL;
        let Some(&(mut at, len)) = self.sync_hist.last() else { return (0, 0) };
        if !fits(len) {
            return (0, 0);
        }
        let (mut links, mut singles) = (0, 0);
        'chain: loop {
            for &(c, l) in self.sync_hist.iter().rev().filter(|p| p.0 < at) {
                let gap = (at - c) as f64;
                let tol = CADENCE_TOL_S * self.rate + CADENCE_DRIFT * gap;
                if gap > 2.0 * period + tol {
                    break;
                }
                let k = (gap / period).round();
                if (1.0..=2.0).contains(&k) && (gap - k * period).abs() < tol && fits(l) {
                    links += 1;
                    singles += u32::from(k == 1.0);
                    at = c;
                    continue 'chain;
                }
            }
            return (links, singles);
        }
    }

    /// Free-run lock: when 1200 Hz sync pulses arrive at a regular line cadence,
    /// start decoding (no VIS needed — handles tuning into a picture already in
    /// progress). With a fixed `expected` mode it locks to that; in auto it picks
    /// the mode whose line period *and* sync length best fit the cadence.
    fn try_freerun(&mut self, out: &mut Vec<SstvEvent>) {
        let n = (SYNC_WIN_S * self.rate) as u64;
        let newest = self.sample_idx - 1;
        if self.vis_state.sync_low as f64 >= SYNC_SHARE * n as f64 {
            let first = self.vis_state.run.map_or(newest, |(first, _)| first);
            self.vis_state.run = Some((first, newest));
            return;
        }
        let Some((first, last)) = self.vis_state.run else { return };
        if (newest - last) as f64 <= SYNC_MERGE_S * self.rate {
            return;
        }
        self.vis_state.run = None;
        // The window is in a pulse from when `SYNC_SHARE` of it has come in
        // until only that much of it is left, so the run is longer than the
        // pulse by the rest of a window and centred half a window late.
        let run = ((last - first + 1) as f64 - (1.0 - 2.0 * SYNC_SHARE) * n as f64).max(0.0);
        let center = ((first + last) as f64 / 2.0 - n as f64 / 2.0).max(0.0) as u64;
        // Plausible sync length across all modes: 5.5 ms (Wraase SC-2) through
        // 9 ms (Scottie/Robot) to **20 ms** (the whole PD family), with a
        // margin either side. The old ceiling was 14 ms, which excluded every
        // PD mode — so a PD picture tuned into mid-transmission could never
        // free-run lock however long it ran (issue #421).
        if run < 0.003 * self.rate || run > 0.028 * self.rate {
            return;
        }
        self.sync_hist.push((center, run as u32));
        if self.sync_hist.len() > SYNC_HIST_LEN {
            self.sync_hist.remove(0);
        }

        // Pick the mode to lock: the fixed one, or the best auto match —
        // the one whose chain took the most single lines (Robot 72's line is
        // two of Robot 36's, so a Robot 72 picture is a Robot 36 chain too,
        // of doubles), then the one whose sync is nearest this pulse's length
        // (Scottie against Martin).
        let locked = match self.expected {
            Some(m) => (self.cadence_chain(m).0 >= FREERUN_LINKS).then_some(m),
            None => SstvMode::ALL
                .into_iter()
                .filter_map(|m| {
                    let (links, singles) = self.cadence_chain(m);
                    let (_, sdur) = self.sync_span(m, m.dimensions().0, 0);
                    (links >= FREERUN_LINKS).then_some((m, singles, (run - sdur).abs() / sdur))
                })
                .min_by(|a, b| b.1.cmp(&a.1).then(a.2.total_cmp(&b.2)))
                .map(|(m, ..)| m),
        };
        if let Some(mode) = locked {
            let (soff, sdur) = self.sync_span(mode, mode.dimensions().0, 0);
            let line_start = (center as f64 - (soff + sdur * 0.5)).max(0.0) as u64;
            self.sync_hist.clear();
            self.begin_image(mode, line_start, false, out);
        }
    }

    /// Start decoding `mode` from `first_line_start`; `numbered` if that is
    /// the picture's first line, as it is after a header, and not whichever
    /// one a free-run lock came in on.
    fn begin_image(
        &mut self,
        mode: SstvMode,
        first_line_start: u64,
        numbered: bool,
        out: &mut Vec<SstvEvent>,
    ) {
        self.mode = mode;
        self.phase = RxPhase::Image;
        self.line = 0;
        self.numbered = numbered;
        self.line_start = first_line_start as f64;
        self.line_samples = self.line_period_samples(mode, 0);
        let (w, h) = mode.dimensions();
        let (_, sync_len) = self.sync_span(mode, w, 0);
        // A pulse may sit this far off the line through the others and still
        // be counted on it: timing noise on a real signal, not a clock error,
        // which the line's slope takes care of.
        self.fit = LineFit::new(self.line_samples, (0.0015 * self.rate).max(0.25 * sync_len));
        self.used_starts.clear();
        self.misses = 0;
        // Everything from the oldest history on, so the second pass can look
        // back past the first line as far as the first pass could.
        let lines = (h / mode.rows_per_line().max(1)) as f64;
        let cap =
            (lines * self.line_samples) as usize + self.hist.len() + (2.0 * self.rate) as usize;
        self.track.start(self.hist_base, cap.min(TRACK_MAX));
        for &hz in &self.hist {
            self.track.push(hz);
        }
        self.last_cr = vec![128u8; (w / 2) as usize];
        self.last_cb = vec![128u8; (w / 2) as usize];
        // Reset free-run tracking so it re-locks cleanly for the next picture.
        self.sync_hist.clear();
        out.push(SstvEvent::ModeDetected(mode));
    }

    // ── image line decode ──
    fn step_image(&mut self, out: &mut Vec<SstvEvent>) {
        // Decode a line once its full duration of history is available. This
        // runs on every sample, so the line length comes from the cached value
        // rather than rebuilding the segment plan (and its allocation) here.
        if (self.sample_idx as f64) < self.line_start + self.line_samples + 0.02 * self.rate {
            return;
        }

        let (w, h) = self.mode.dimensions();
        if self.line == 0 && !self.numbered && self.robot36_line_is_odd() {
            // A free-run lock on an odd line. Counting it as line 0 would read
            // its Cb as Cr, and every line's after it, and paint the picture
            // in the wrong colours from top to bottom.
            self.line = 1;
        }
        let rpl = self.mode.rows_per_line().max(1);
        let k = (self.line / rpl) as f64;
        let start = self.place_line(k);
        self.used_starts.push(start);
        // One transmitted line is two picture rows in the PD family — see
        // `SstvMode::rows_per_line` — so this hands back a row at a time and
        // the caller sees the same stream of `Line` events either way.
        let (hist, base) = (&self.hist, self.hist_base);
        let rows = decode_line(
            self.mode,
            w,
            self.line,
            start,
            self.rate,
            |i| hz_in(hist, base, i),
            &mut self.last_cr,
            &mut self.last_cb,
        );
        for (n, rgb) in rows.into_iter().enumerate() {
            let y = self.line + n as u16;
            if y < h {
                out.push(SstvEvent::Line { y, rgb });
            }
        }

        self.line += rpl;
        if self.line >= h {
            self.final_pass(out);
            out.push(SstvEvent::ImageComplete);
            self.phase = RxPhase::Hunt;
            self.vis_state = VisState::reset(self.sample_idx);
            self.track.clear();
        } else {
            self.line_samples = self.line_period_samples(self.mode, self.line);
            self.line_start = self.fit.predict(k + 1.0).unwrap_or(start + self.line_samples);
        }
    }

    /// Whether the Robot 36 line starting at `line_start` is an odd one, as its
    /// separator says: 1500 Hz ahead of an even line's Cr, 2300 ahead of an
    /// odd line's Cb. Never, for any other mode.
    fn robot36_line_is_odd(&self) -> bool {
        if self.mode != SstvMode::Robot36 {
            return false;
        }
        let (w, _) = self.mode.dimensions();
        // Sync, porch and luma come before it.
        let before: f64 = line_segments(self.mode, w, 0)
            .iter()
            .take(3)
            .map(|s| match s {
                Seg::Tone { dur, .. } => *dur * self.rate,
                Seg::Scan { width, px, .. } => *width as f64 * *px * self.rate,
            })
            .sum();
        let (hist, base) = (&self.hist, self.hist_base);
        let hz =
            tone_median(|i| hz_in(hist, base, i), self.line_start + before, 0.0045 * self.rate);
        hz > 1900.0
    }

    /// Offset (in samples) from a line's start to the start of its 1200 Hz
    /// sync pulse, plus the pulse duration in samples.
    fn sync_span(&self, mode: SstvMode, w: u16, line: u16) -> (f64, f64) {
        let mut t = 0.0;
        for seg in line_segments(mode, w, line) {
            match seg {
                Seg::Tone { hz, dur } => {
                    let d = dur * self.rate;
                    if (hz - SYNC_HZ).abs() < 1.0 {
                        return (t, d);
                    }
                    t += d;
                }
                Seg::Scan { width, px, .. } => t += width as f64 * px * self.rate,
            }
        }
        (0.0, 0.009 * self.rate)
    }

    /// Where transmitted line `k` starts.
    ///
    /// Its sync pulse is looked for near where the line is expected, and if it
    /// is plainly there it joins the fit. Once the fit has enough points the
    /// line starts where the fit says, whatever this one pulse did — which is
    /// the point: a pulse lost to a fade, or one moved by noise, no longer
    /// moves the line or any line after it. Until then the pulse is used as
    /// measured, and where there is none, the line follows on from the last.
    fn place_line(&mut self, k: f64) -> f64 {
        let (w, _) = self.mode.dimensions();
        let (soff, sdur) = self.sync_span(self.mode, w, self.line);
        let centre_off = soff + sdur * 0.5;
        // Wide until there is a line to go on, then close around it, where a
        // burst of noise elsewhere cannot outbid the real pulse. Wider still
        // for every pulse missed before then: lines that follow on at the
        // nominal period drift by the sender's clock error each, and after a
        // fade long enough the real pulse is no longer where a fixed window
        // looks.
        let half_win = if self.fit.line().is_some() {
            2.0 * self.fit.tol
        } else {
            (0.012 * self.rate * (1 + self.misses) as f64).min(self.line_samples / 2.0)
        };
        let (hist, base) = (&self.hist, self.hist_base);
        let measured =
            measure_sync(|i| hz_in(hist, base, i), self.line_start + centre_off, half_win, sdur)
                .filter(|&(_, fill)| fill >= MIN_FILL)
                .map(|(c, _)| c - centre_off);
        match measured {
            Some(m) => {
                self.misses = 0;
                self.fit.add(k, m);
            }
            None => self.misses += 1,
        }
        self.fit.predict(k).or(measured).unwrap_or(self.line_start)
    }

    /// The picture's second pass, once all of it has arrived.
    ///
    /// The fit placed each line knowing only the pulses before it, and only
    /// the ones clear enough to count alone. Now the whole sync column is
    /// there: search it for the line along which the most sync tone lies
    /// ([`SyncMap::search`]), refine that through the pulses on it, and if it
    /// puts the lines somewhere other than where they were decoded — by half a
    /// pixel or more anywhere — decode every line again from the kept track
    /// and send them all, over the first pass's, before the picture completes.
    fn final_pass(&mut self, out: &mut Vec<SstvEvent>) {
        let mode = self.mode;
        let (w, h) = mode.dimensions();
        let rpl = mode.rows_per_line().max(1);
        let lines = self.used_starts.len();
        if lines < 8 || self.track.full() {
            return;
        }
        let (soff, sdur) = self.sync_span(mode, w, 0);
        let period = self.line_period_samples(mode, 0);
        let map = SyncMap::new(
            &self.track,
            Geometry { lines, period, sync_centre: soff + sdur * 0.5, sync_len: sdur },
        );
        let live = self.fit.line();
        let around = live.unwrap_or(Line { a: self.used_starts[0], b: period });
        let Some(found) = map.search(around) else { return };
        // A line through nothing but picture and noise scores what every other
        // phase does. Nothing to go on, so leave the picture as it is.
        if found.contrast < MIN_CONTRAST {
            return;
        }

        // Sharpen it on the pulses themselves, at full resolution.
        let track = &self.track;
        let pts: Vec<(f64, f64)> = (0..lines)
            .filter_map(|k| {
                let c = found.line.at(k as f64) + soff + sdur * 0.5;
                measure_sync(|i| track.hz(i), c, self.fit.tol, sdur)
                    .filter(|&(_, fill)| fill >= MIN_FILL)
                    .map(|(c, _)| (k as f64, c - soff - sdur * 0.5))
            })
            .collect();
        // Only with pulses on a good share of the lines, though. A handful is
        // what heavy noise leaves, and a line through a handful is worse than
        // the one the whole column gave.
        let mut best = Some(&pts)
            .filter(|p| p.len() >= lines / 4)
            .and_then(|p| robust_line(p, period, self.fit.tol, Some(found.line)))
            .filter(|l| map.score(*l, sdur) >= found.fill - FILL_SLACK)
            .unwrap_or(found.line);
        // Never trade a line the picture already fits better.
        if let Some(l) = live
            && map.score(l, sdur) > map.score(best, sdur) + FILL_SLACK
        {
            best = l;
        }

        let px = min_pixel_samples(mode, w, self.rate);
        let moved = self
            .used_starts
            .iter()
            .enumerate()
            .map(|(k, &s)| (best.at(k as f64) - s).abs())
            .fold(0.0, f64::max);
        if moved < 0.5 * px {
            return;
        }

        let mut cr = vec![128u8; (w / 2) as usize];
        let mut cb = vec![128u8; (w / 2) as usize];
        for k in 0..lines {
            let line = k as u16 * rpl;
            let rows = decode_line(
                mode,
                w,
                line,
                best.at(k as f64),
                self.rate,
                |i| track.hz(i),
                &mut cr,
                &mut cb,
            );
            for (n, rgb) in rows.into_iter().enumerate() {
                let y = line + n as u16;
                if y < h {
                    out.push(SstvEvent::Line { y, rgb });
                }
            }
        }
    }
}

/// How far a picture's sync column has to stand above the rest of it — as a
/// fraction of a sync window filled with sync tone — before the second pass
/// believes it found one.
const MIN_CONTRAST: f64 = 0.15;

/// Differences in fill smaller than this are noise between two lines that
/// both run down the sync column.
const FILL_SLACK: f64 = 0.01;

/// The most samples a picture's frequency track may hold: PD290 at 96 kHz,
/// with room. Past it the second pass is skipped rather than memory taken.
const TRACK_MAX: usize = 32 << 20;

/// Frequency at absolute sample `idx` of a history that starts at `base`; the
/// leader tone outside it.
fn hz_in(hist: &[f64], base: u64, idx: i64) -> f64 {
    if idx < base as i64 {
        return 1900.0;
    }
    hist.get((idx - base as i64) as usize).copied().unwrap_or(1900.0)
}

/// The shortest pixel in any of `mode`'s scans, in samples: the unit the
/// second pass measures a correction worth making in.
fn min_pixel_samples(mode: SstvMode, w: u16, rate: f64) -> f64 {
    line_segments(mode, w, 0)
        .iter()
        .filter_map(|s| match s {
            Seg::Scan { px, .. } => Some(px * rate),
            Seg::Tone { .. } => None,
        })
        .fold(f64::INFINITY, f64::min)
}

/// Decode one transmitted line, starting at sample `start`, into its picture
/// rows: one for every mode but the PD family, two for that. `hz` reads the
/// frequency track; `last_cr`/`last_cb` carry the Robot modes' chroma from one
/// line to the next.
#[allow(clippy::too_many_arguments)]
fn decode_line(
    mode: SstvMode,
    w: u16,
    line: u16,
    start: f64,
    rate: f64,
    hz: impl Fn(i64) -> f64,
    last_cr: &mut Vec<u8>,
    last_cb: &mut Vec<u8>,
) -> Vec<Vec<u8>> {
    let mut r = vec![0u8; w as usize];
    let mut g = vec![0u8; w as usize];
    let mut b = vec![0u8; w as usize];
    let mut y = vec![0u8; w as usize];
    let mut y2 = vec![0u8; w as usize];
    let mut cr = last_cr.clone();
    let mut cb = last_cb.clone();
    // A PD line's chroma is full width, so the buffers this mode's rows
    // came in with (sized for the Robot modes' half-width chroma) are the
    // wrong shape for it.
    if mode.rows_per_line() > 1 {
        cr = vec![128u8; w as usize];
        cb = vec![128u8; w as usize];
    }

    let mut t = start;
    for seg in line_segments(mode, w, line) {
        match seg {
            Seg::Tone { dur, .. } => t += dur * rate,
            Seg::Scan { chan, width, px } => {
                let step = px * rate;
                for x in 0..width as usize {
                    // Sample the centre of each pixel window.
                    let idx = (t + (x as f64 + 0.5) * step) as i64;
                    let v = hz_to_value(hz(idx));
                    let cri = x.min(cr.len().saturating_sub(1));
                    let cbi = x.min(cb.len().saturating_sub(1));
                    match chan {
                        Chan::R => r[x] = v,
                        Chan::G => g[x] = v,
                        Chan::B => b[x] = v,
                        Chan::Y => y[x] = v,
                        Chan::Y2 => y2[x] = v,
                        Chan::Cr => cr[cri] = v,
                        Chan::Cb => cb[cbi] = v,
                    }
                }
                t += width as f64 * step;
            }
        }
    }

    let robot = matches!(mode, SstvMode::Robot72 | SstvMode::Robot36);
    if robot {
        // Robot 36 sends one chroma per line and the other row's is
        // carried over; PD sends both, every line, and has nothing to
        // remember.
        last_cr.clone_from(&cr);
        last_cb.clone_from(&cb);
    }
    let pd = mode.rows_per_line() > 1;

    // One row for most modes, two for PD — where the second row is the
    // same chroma with the second luma scan over it.
    let mut rows: Vec<Vec<u8>> = Vec::with_capacity(if pd { 2 } else { 1 });
    for luma in [&y, &y2].into_iter().take(if pd { 2 } else { 1 }) {
        let mut rgb = vec![0u8; w as usize * 3];
        for x in 0..w as usize {
            let (rr, gg, bb) = if robot {
                let cx = (x / 2).min(cr.len() - 1);
                yuv_to_rgb(y[x], cr[cx], cb[cx])
            } else if pd {
                yuv_to_rgb(luma[x], cr[x], cb[x])
            } else {
                (r[x], g[x], b[x])
            };
            rgb[x * 3] = rr;
            rgb[x * 3 + 1] = gg;
            rgb[x * 3 + 2] = bb;
        }
        rows.push(rgb);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The symbol stream against the published format, character by character.
    ///
    /// Worth spelling out rather than only round-tripping: a codec tested
    /// against nothing but its own inverse agrees with itself whatever it does,
    /// and what has to be true here is that it agrees with MMSSTV. `$2A`, then
    /// ASCII less `$20`, then `$01`, then the XOR of the characters alone.
    #[test]
    fn the_symbols_are_the_published_ones() {
        assert_eq!(
            fsk_id_symbols("OE1XYZ"),
            vec![
                0x2A, // header
                0x2F, // O
                0x25, // E
                0x11, // 1
                0x38, // X
                0x39, // Y
                0x3A, // Z
                0x01, // terminator
                0x2F ^ 0x25 ^ 0x11 ^ 0x38 ^ 0x39 ^ 0x3A,
            ]
        );
        // Lower case has no code of its own and is folded, not dropped...
        assert_eq!(fsk_id_symbols("oe1xyz"), fsk_id_symbols("OE1XYZ"));
        // ...and a character with no code at all is left out rather than
        // mangled into one that would print as noise at the far end.
        assert_eq!(fsk_id_symbols("OE1XYZ\u{00fc}"), fsk_id_symbols("OE1XYZ"));
        // Nothing to identify with is nothing to send.
        assert!(fsk_id_symbols("").is_empty());
        assert!(fsk_id_symbols("   ").is_empty());
    }

    #[test]
    fn a_stream_that_does_not_check_out_is_not_a_callsign() {
        let good = fsk_id_symbols("OE1XYZ");
        assert_eq!(fsk_id_text(&good).as_deref(), Some("OE1XYZ"));
        // One bit of one character wrong: the checksum is the whole point.
        let mut bad = good.clone();
        bad[3] ^= 1;
        assert_eq!(fsk_id_text(&bad), None);
        // No header, no terminator, nothing after the terminator: each on its
        // own is enough to refuse.
        assert_eq!(fsk_id_text(&good[1..]), None);
        assert_eq!(fsk_id_text(&good[..good.len() - 2]), None);
        assert_eq!(fsk_id_text(&good[..good.len() - 1]), None);
    }

    /// The whole of issue #287: a picture, then the callsign in tones, decoded
    /// off the audio by the receiver rather than by the encoder's own inverse.
    #[test]
    fn the_id_comes_back_off_the_air() {
        let rate = 48_000.0;
        let mode = SstvMode::Robot36;
        let (w, h) = mode.dimensions();
        let rgb = vec![128u8; w as usize * h as usize * 3];
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0).with_fsk_id("OE1XYZ");
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut heard = None;
        let mut guard = 0;
        while !tx.done() && guard < 40_000 {
            let n = tx.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            for e in events.drain(..) {
                if let SstvEvent::FskId(id) = e {
                    heard = Some(id);
                }
            }
            guard += 1;
        }
        assert_eq!(heard.as_deref(), Some("OE1XYZ"));
    }

    /// A station with nothing to identify with transmits exactly what it used
    /// to — no leader, no tail, not one sample.
    #[test]
    fn no_callsign_adds_no_air_time() {
        let rate = 48_000.0;
        let mode = SstvMode::Robot36;
        let (w, h) = mode.dimensions();
        let rgb = vec![64u8; w as usize * h as usize * 3];
        let plain = SstvTx::new(mode, &rgb, w, h, rate, 0.0).total_samples();
        assert_eq!(SstvTx::new(mode, &rgb, w, h, rate, 0.0).with_fsk_id("").total_samples(), plain);
        // And one that has something to send costs the leader plus six bits a
        // character, which is about two and a half seconds for a callsign.
        let with = SstvTx::new(mode, &rgb, w, h, rate, 0.0).with_fsk_id("OE1XYZ").total_samples();
        let added = (with - plain) as f64 / rate;
        let want = FSKID_LEADER_S + FSKID_SYNC_S + FSKID_BIT_S * (1.0 + 6.0 * 9.0);
        assert!((added - want).abs() < 0.01, "the ID added {added:.3} s, expected {want:.3} s");
    }

    /// Issue #397: a receiver committed to a four-minute mode has to be able
    /// to let go of it.
    ///
    /// Locked on to Scottie DX and then restarted, it must be hunting again —
    /// and it must *stay* hunting on the audio that is still arriving from the
    /// transmission it abandoned, because the free-run detector reads the
    /// history buffer and the history buffer was full of that picture's sync
    /// pulses. Then a new header on the same receiver has to start a picture,
    /// which is the half that says the restart re-armed rather than merely
    /// stopped.
    #[test]
    fn a_restart_lets_go_of_the_picture_and_hunts_again() {
        let rate = 48_000.0;
        let (w, h) = SstvMode::ScottieDx.dimensions();
        let rgb = vec![96u8; w as usize * h as usize * 3];
        let mut tx = SstvTx::new(SstvMode::ScottieDx, &rgb, w, h, rate, 0.0);
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];

        // Far enough in to be decoding lines, not merely to have seen the VIS.
        while !rx.receiving() || rx.progress() < 0.02 {
            let n = tx.next_block(&mut block);
            assert!(n > 0, "the transmission ran out before the picture started");
            rx.process(&block[..n], &mut events);
            events.clear();
        }
        assert_eq!(rx.mode(), SstvMode::ScottieDx);

        rx.restart();
        assert!(!rx.receiving(), "the picture was not let go of");
        assert_eq!(rx.progress(), 0.0);

        // Another second of the abandoned transmission: a receiver that kept
        // its history would lock straight back on to the cadence it is hearing.
        for _ in 0..12 {
            let n = tx.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            events.clear();
        }
        assert!(!rx.receiving(), "it re-locked on the transmission it was told to abandon");

        // ...and the next station's header still starts a picture.
        let (w2, h2) = SstvMode::Robot36.dimensions();
        let rgb2 = vec![32u8; w2 as usize * h2 as usize * 3];
        let mut next = SstvTx::new(SstvMode::Robot36, &rgb2, w2, h2, rate, 0.0);
        let mut detected = None;
        let mut guard = 0;
        while detected.is_none() && !next.done() && guard < 2_000 {
            let n = next.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            for e in events.drain(..) {
                if let SstvEvent::ModeDetected(m) = e {
                    detected = Some(m);
                }
            }
            guard += 1;
        }
        assert_eq!(detected, Some(SstvMode::Robot36), "the restarted receiver heard nothing");
    }

    /// End-to-end: encode a small gradient, decode it back, and check the VIS
    /// mode was recovered and the image roughly matches.
    #[test]
    fn scottie1_loopback_recovers_mode() {
        let rate = 48_000.0;
        let mode = SstvMode::Scottie1;
        let (w, h) = mode.dimensions();
        // Simple vertical gradient so a rough decode is easy to sanity-check.
        let mut rgb = vec![0u8; w as usize * h as usize * 3];
        for yy in 0..h as usize {
            for xx in 0..w as usize {
                let i = (yy * w as usize + xx) * 3;
                let v = (xx * 255 / w as usize) as u8;
                rgb[i] = v;
                rgb[i + 1] = v;
                rgb[i + 2] = v;
            }
        }
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut detected = None;
        let mut lines = 0;
        let mut guard = 0;
        while !tx.done() && guard < 20_000 {
            let n = tx.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            for e in events.drain(..) {
                match e {
                    SstvEvent::ModeDetected(m) => detected = Some(m),
                    SstvEvent::Line { .. } => lines += 1,
                    SstvEvent::ImageComplete
                    | SstvEvent::FskId(_)
                    | SstvEvent::UnsupportedMode { .. } => {}
                }
            }
            guard += 1;
        }
        // Flush any tail.
        rx.process(&[0.0; 48_000], &mut events);
        for e in events.drain(..) {
            if let SstvEvent::ModeDetected(m) = e {
                detected = Some(m);
            } else if let SstvEvent::Line { .. } = e {
                lines += 1;
            }
        }
        assert_eq!(detected, Some(mode), "VIS mode should be recovered");
        assert!(lines > (h as usize) / 2, "should decode most lines, got {lines}");
    }

    /// Every mode, encoder into decoder: the VIS is read back, the picture
    /// comes out the right height, and the pixels are roughly what went in.
    ///
    /// One test over `SstvMode::ALL` rather than one per mode, because what
    /// has to hold is a property of the table: a mode added to it with a pixel
    /// time transcribed wrongly still produces a picture, just a sheared one,
    /// and the only thing that catches that is decoding it back and looking at
    /// where the colours landed. It is also what proves the PD family's
    /// two-rows-per-line plumbing, which nothing else in the file exercises.
    #[test]
    fn every_mode_round_trips_through_its_own_decoder() {
        let rate = 48_000.0;
        for mode in SstvMode::ALL {
            let (w, h) = mode.dimensions();
            // Three vertical bars — red, green, blue — so a decode that has
            // the channels or the timing wrong cannot pass by accident.
            let mut rgb = vec![0u8; w as usize * h as usize * 3];
            for yy in 0..h as usize {
                for xx in 0..w as usize {
                    let i = (yy * w as usize + xx) * 3;
                    let band = xx * 3 / w as usize;
                    rgb[i + band.min(2)] = 220;
                }
            }
            let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
            let mut rx = SstvRx::new(rate);
            let mut events = Vec::new();
            let mut block = vec![0.0f32; 8192];
            let mut detected = None;
            let mut got = vec![0u8; w as usize * h as usize * 3];
            let mut lines = 0usize;
            let mut guard = 0;
            while !tx.done() && guard < 400_000 {
                let n = tx.next_block(&mut block);
                rx.process(&block[..n], &mut events);
                for e in events.drain(..) {
                    match e {
                        SstvEvent::ModeDetected(m) => detected = Some(m),
                        SstvEvent::Line { y, rgb: row } => {
                            lines += 1;
                            let at = y as usize * w as usize * 3;
                            if at + row.len() <= got.len() {
                                got[at..at + row.len()].copy_from_slice(&row);
                            }
                        }
                        _ => {}
                    }
                }
                guard += 1;
            }
            rx.process(&vec![0.0f32; 48_000], &mut events);
            for e in events.drain(..) {
                if let SstvEvent::Line { y, rgb: row } = e {
                    lines += 1;
                    let at = y as usize * w as usize * 3;
                    if at + row.len() <= got.len() {
                        got[at..at + row.len()].copy_from_slice(&row);
                    }
                }
            }
            assert_eq!(detected, Some(mode), "{} VIS not recovered", mode.label());
            assert!(
                lines > (h as usize) / 2,
                "{}: only {lines} of {h} lines decoded",
                mode.label()
            );
            // Sample the middle of each colour bar, a third of the way down,
            // and check the right channel is the dominant one.
            let yy = h as usize / 3;
            for (band, chan) in [(0usize, 0usize), (1, 1), (2, 2)] {
                let xx = (band * 2 + 1) * w as usize / 6;
                let i = (yy * w as usize + xx) * 3;
                let px = [got[i], got[i + 1], got[i + 2]];
                let others = (0..3).filter(|&c| c != chan).map(|c| px[c]).max().unwrap();
                assert!(
                    px[chan] > 100 && px[chan] as i32 > others as i32 + 40,
                    "{}: bar {band} decoded as {px:?}, expected channel {chan} to dominate",
                    mode.label()
                );
            }
        }
    }

    /// The published line times, recomputed from the segment plan. A pixel
    /// time transcribed with a digit out still makes a picture; it is the
    /// *total* that gives it away, so that is what is pinned.
    #[test]
    fn the_line_times_are_the_published_ones() {
        let rx = SstvRx::new(1_000_000.0); // µs per sample: read the plan directly
        for (mode, ms) in [
            (SstvMode::Scottie1, 428.22),
            (SstvMode::Scottie2, 277.692),
            (SstvMode::ScottieDx, 1_050.3),
            (SstvMode::Martin1, 446.446),
            (SstvMode::Martin2, 226.798),
            (SstvMode::Robot72, 300.0),
            (SstvMode::Robot36, 150.0),
            (SstvMode::WraaseSc2_180, 711.0225),
            (SstvMode::WraaseSc2_120, 475.53),
            (SstvMode::Pd50, 388.16),
            (SstvMode::Pd90, 703.04),
            (SstvMode::Pd120, 508.48),
            (SstvMode::Pd160, 804.416),
            (SstvMode::Pd180, 754.24),
            (SstvMode::Pd240, 1_000.0),
            (SstvMode::Pd290, 937.28),
        ] {
            let got = rx.line_period_samples(mode, 0) / 1000.0;
            assert!(
                (got - ms).abs() < 0.05,
                "{}: line is {got:.4} ms, published {ms} ms",
                mode.label()
            );
        }
    }

    /// The PD family is the only one that carries two picture rows per sync.
    #[test]
    fn only_the_pd_modes_carry_two_rows_a_line() {
        for m in SstvMode::ALL {
            let want = matches!(
                m,
                SstvMode::Pd50
                    | SstvMode::Pd90
                    | SstvMode::Pd120
                    | SstvMode::Pd160
                    | SstvMode::Pd180
                    | SstvMode::Pd240
                    | SstvMode::Pd290
            );
            assert_eq!(m.rows_per_line() == 2, want, "{}", m.label());
        }
    }

    /// A header for a mode this build does not have is reported, not
    /// swallowed — the whole of issue #421's "the decoder isn't starting
    /// reception" with a perfect signal on the waterfall.
    #[test]
    fn an_unimplemented_mode_says_so_instead_of_going_quiet() {
        let rate = 48_000.0;
        // Pasokon P3's VIS, sent by hand: leader, break, leader, start bit,
        // seven data bits LSB first, parity, stop.
        let code = 0x71u8;
        let mut audio: Vec<f32> = Vec::new();
        let mut phase = 0.0f64;
        let mut tone = |hz: f64, dur: f64, audio: &mut Vec<f32>| {
            for _ in 0..(dur * rate) as usize {
                phase += TAU * hz / rate;
                audio.push((phase.sin() as f32) * 0.5);
            }
        };
        tone(1900.0, 0.300, &mut audio);
        tone(1200.0, 0.010, &mut audio);
        tone(1900.0, 0.300, &mut audio);
        tone(1200.0, 0.030, &mut audio);
        let mut parity = 0u8;
        for bit in 0..7 {
            let one = (code >> bit) & 1 == 1;
            parity ^= one as u8;
            tone(if one { 1100.0 } else { 1300.0 }, 0.030, &mut audio);
        }
        tone(if parity == 1 { 1100.0 } else { 1300.0 }, 0.030, &mut audio);
        tone(1200.0, 0.030, &mut audio);
        tone(1500.0, 0.500, &mut audio);

        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        for chunk in audio.chunks(4096) {
            rx.process(chunk, &mut events);
        }
        let reported: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                SstvEvent::UnsupportedMode { code, name } => Some((*code, *name)),
                _ => None,
            })
            .collect();
        // Exactly one report: the break pulse in the middle of the header is
        // also a rising edge into 1200 Hz after a leader, and must not be
        // mistaken for a second header.
        assert_eq!(reported, vec![(0x71, Some("Pasokon P3"))]);
        assert!(
            !events.iter().any(|e| matches!(e, SstvEvent::ModeDetected(_))),
            "nothing may be decoded for a mode we do not have"
        );
    }

    #[test]
    fn vis_codes_roundtrip() {
        for m in SstvMode::ALL {
            assert_eq!(SstvMode::from_vis(m.vis_code()), Some(m));
        }
    }

    #[test]
    fn tx_ppm_scales_duration() {
        let rate = 48_000.0;
        let mode = SstvMode::Martin1;
        let (w, h) = mode.dimensions();
        let rgb = vec![128u8; w as usize * h as usize * 3];
        let base = SstvTx::new(mode, &rgb, w, h, rate, 0.0).total_samples() as f64;
        // +10 000 ppm = +1% longer transmission.
        let trimmed = SstvTx::new(mode, &rgb, w, h, rate, 10_000.0).total_samples() as f64;
        assert!((trimmed / base - 1.01).abs() < 0.0005, "ratio {}", trimmed / base);
    }

    /// Free-run: feed the RX the picture audio *after* the VIS header (as if we
    /// tuned in mid-transmission). With the mode pre-selected it should lock onto
    /// the sync cadence and decode.
    #[test]
    fn freerun_decodes_without_vis() {
        let rate = 48_000.0;
        let mode = SstvMode::Scottie1;
        let (w, h) = mode.dimensions();
        let mut rgb = vec![0u8; w as usize * h as usize * 3];
        for yy in 0..h as usize {
            for xx in 0..w as usize {
                let i = (yy * w as usize + xx) * 3;
                rgb[i] = (xx * 255 / w as usize) as u8;
            }
        }
        // Render the whole transmission to a buffer.
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
        let mut audio = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut guard = 0;
        while !tx.done() && guard < 20_000 {
            let n = tx.next_block(&mut block);
            audio.extend_from_slice(&block[..n]);
            guard += 1;
        }
        // Skip past the VIS (~1.1 s) so only image data is fed → forces free-run.
        // Use auto (`None`): the RX must identify the mode from the sync cadence.
        let skip = (rate * 1.1) as usize;
        let mut rx = SstvRx::new(rate);
        rx.set_expected(None);
        let mut events = Vec::new();
        let mut detected = None;
        let mut lines = 0;
        for chunk in audio[skip.min(audio.len())..].chunks(4096) {
            rx.process(chunk, &mut events);
            for e in events.drain(..) {
                match e {
                    SstvEvent::ModeDetected(m) => detected = Some(m),
                    SstvEvent::Line { .. } => lines += 1,
                    SstvEvent::ImageComplete
                    | SstvEvent::FskId(_)
                    | SstvEvent::UnsupportedMode { .. } => {}
                }
            }
        }
        assert_eq!(detected, Some(mode), "free-run should lock the selected mode");
        assert!(lines > 40, "free-run should decode many lines, got {lines}");
    }

    /// A deterministic noise source, so a noisy test fails the same way twice.
    struct Noise(u64);

    impl Noise {
        /// Roughly unit-variance Gaussian: the sum of twelve uniforms.
        fn next(&mut self) -> f64 {
            let mut sum = 0.0;
            for _ in 0..12 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                sum += (self.0 >> 11) as f64 / (1u64 << 53) as f64;
            }
            sum - 6.0
        }
    }

    /// Send a picture black on the left, white on the right through a channel:
    /// the sender's clock `ppm` off the receiver's, `gain(t)` on the signal, and
    /// noise of standard deviation `noise` once the header is over (a header
    /// lost in the noise would test the VIS detector, not the timing). Returns
    /// the edge's slant over the picture's height and its row-to-row jitter,
    /// both in pixels, fitted over the rows sent while `gain` was 1.
    fn slant_through(
        mode: SstvMode,
        ppm: f32,
        noise: f64,
        gain: impl Fn(f64) -> f64,
    ) -> (f64, f64) {
        let rate = 48_000.0;
        let (w, h) = mode.dimensions();
        let (wu, hu) = (w as usize, h as usize);
        let mut rgb = vec![0u8; wu * hu * 3];
        for y in 0..hu {
            rgb[(y * wu + wu / 2) * 3..(y + 1) * wu * 3].fill(230);
        }
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, ppm);
        let mut rx = SstvRx::new(rate);
        let mut rng = Noise(0x9E37_79B9_7F4A_7C15);
        let mut got = vec![0u8; rgb.len()];
        let mut complete = false;
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut n_sent = 0usize;
        let mut take = |events: &mut Vec<SstvEvent>, got: &mut Vec<u8>| {
            for e in events.drain(..) {
                match e {
                    SstvEvent::Line { y, rgb: row } => {
                        let at = y as usize * wu * 3;
                        got[at..at + row.len()].copy_from_slice(&row);
                    }
                    SstvEvent::ImageComplete => complete = true,
                    _ => {}
                }
            }
        };
        while !tx.done() {
            let n = tx.next_block(&mut block);
            for s in &mut block[..n] {
                let t = n_sent as f64 / rate;
                let nz = if t > 1.0 { noise * rng.next() } else { 0.0 };
                *s = (*s as f64 * 0.3 * gain(t) + nz) as f32;
                n_sent += 1;
            }
            rx.process(&block[..n], &mut events);
            take(&mut events, &mut got);
        }
        let tail: Vec<f32> = (0..2 * rate as usize).map(|_| (noise * rng.next()) as f32).collect();
        rx.process(&tail, &mut events);
        take(&mut events, &mut got);
        assert!(complete, "{}: the picture never completed", mode.label());

        let period = rx.line_period_samples(mode, 0) / rate * (1.0 + ppm as f64 * 1e-6);
        let rpl = mode.rows_per_line() as usize;
        // Where the edge is: how many of the row's pixels are dark.
        let edge = |y: usize| {
            (0..wu)
                .filter(|&x| {
                    got[(y * wu + x) * 3..][..3].iter().map(|&v| v as u32).sum::<u32>() < 345
                })
                .count() as f64
        };
        let rows: Vec<(f64, f64)> = (0..hu)
            .filter(|&y| {
                let t = 0.92 + (y / rpl) as f64 * period;
                [t - 0.2, t, t + period].iter().all(|&t| gain(t) == 1.0)
            })
            .map(|y| (y as f64, edge(y)))
            .collect();
        let n = rows.len() as f64;
        let (my, me) =
            (rows.iter().map(|r| r.0).sum::<f64>() / n, rows.iter().map(|r| r.1).sum::<f64>() / n);
        let sxx: f64 = rows.iter().map(|r| (r.0 - my).powi(2)).sum();
        let slope = rows.iter().map(|r| (r.0 - my) * (r.1 - me)).sum::<f64>() / sxx;
        let jitter =
            (rows.iter().map(|r| (r.1 - me - slope * (r.0 - my)).powi(2)).sum::<f64>() / n).sqrt();
        (slope * hu as f64, jitter)
    }

    /// A long fade before enough sync pulses have come in to fit a line
    /// through. The receiver used to walk on at the nominal period through
    /// it, came out of the fade with its ±12 ms window somewhere the pulses no
    /// longer were, and never found them again: 268 pixels of slant on Robot 36
    /// with the sender's clock 0.25 % slow. Now the window widens for every
    /// pulse missed, the fit forms once the signal is back, and the picture
    /// below the fade is straight.
    #[test]
    fn a_fade_before_the_fit_forms_does_not_slant_the_picture() {
        let fade = |t: f64| if t > 1.0 && t < 15.0 { 0.02 } else { 1.0 };
        let (slant, jitter) = slant_through(SstvMode::Robot36, 2500.0, 0.05, fade);
        assert!(slant.abs() < 1.5, "slant {slant:.1} px");
        assert!(jitter < 1.0, "row jitter {jitter:.1} px");
    }

    /// Short deep fades every few seconds, the way HF QSB takes a signal. The
    /// fit carries the lines through each one; a receiver that re-locked on
    /// every pulse would come out of each one wherever the noise put it.
    #[test]
    fn repeated_fades_do_not_move_the_lines() {
        let qsb = |t: f64| if t % 6.0 > 4.5 { 0.03 } else { 1.0 };
        let (slant, jitter) = slant_through(SstvMode::Martin1, 1500.0, 0.05, qsb);
        assert!(slant.abs() < 1.0, "slant {slant:.1} px");
        assert!(jitter < 1.0, "row jitter {jitter:.1} px");
    }

    /// So much noise that almost no sync pulse is clear enough to count on
    /// its own, and the running fit never forms: the lines follow on at the
    /// nominal period, 0.3 % short, and the picture shears by dozens of
    /// pixels. The second pass finds the sync column in the whole of the
    /// picture at once and decodes it again along it. The bound is loose
    /// because the edge itself is noisy here; the shear it catches is not
    /// subtle.
    #[test]
    fn the_second_pass_straightens_a_picture_too_noisy_to_fit_as_it_came() {
        let (slant, _) = slant_through(SstvMode::Pd50, 3000.0, 0.25, |_| 1.0);
        assert!(slant.abs() < 6.0, "slant {slant:.1} px");
    }

    // ── Sensitivity ──
    //
    // What the receiver makes of a picture in white noise, as a function of
    // signal-to-noise ratio. The sweep below is the yardstick every change to
    // the demodulator is measured against: a change that does not move these
    // numbers is not a sensitivity improvement, however reasonable it looks.

    /// The bandwidth the signal-to-noise ratios here are quoted in: the SSB
    /// voice channel SSTV is received through.
    const SNR_BANDWIDTH_HZ: f64 = 3000.0;

    /// The most rows a free-run lock may number wrongly: the pulses it takes
    /// to be sure of the cadence, and the line it came in partway through.
    const FREERUN_MAX_ROWS_LATE: usize = 16;

    /// What one reception in noise came to.
    struct Reception {
        /// The mode the receiver locked on to, if any.
        mode: Option<SstvMode>,
        /// Whether it locked before the picture began — off the VIS, that is,
        /// rather than off the picture's sync pulses once the VIS was missed.
        /// A free-run lock rescues the picture but not its first lines, and on
        /// Robot 36 not always its colours.
        by_vis: bool,
        /// Picture rows delivered.
        rows: usize,
        /// Mean absolute error of the delivered rows against what was sent,
        /// in levels (0–255) per channel.
        err: f64,
    }

    /// A picture with something in every channel and at every scale: a ramp
    /// across in red, a ramp down in green, and blocks in blue — so a decoder
    /// that swaps channels, loses the timing or smears the detail cannot pass
    /// by accident, and the colour-difference modes get edges in chroma.
    fn test_card(w: usize, h: usize) -> Vec<u8> {
        let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 3;
                rgb[i] = (x * 255 / w.max(1)) as u8;
                rgb[i + 1] = (y * 255 / h.max(1)) as u8;
                rgb[i + 2] = if (x / 16 + y / 16) % 2 == 0 { 40 } else { 215 };
            }
        }
        rgb
    }

    /// Send `mode` through white Gaussian noise at `snr_db` (in
    /// [`SNR_BANDWIDTH_HZ`]) and see what comes out. With `header` the whole
    /// transmission is sent and the receiver has to find the VIS; without it
    /// the header is cut off and it has to lock on the sync cadence alone.
    /// Noise runs throughout, including half a second before the signal and
    /// two seconds after it, so the hunt sees noise before it sees a picture.
    fn receive_in_noise(mode: SstvMode, snr_db: f64, seed: u64, header: bool) -> Reception {
        // The calibration header and VIS are 0.94 s; past 1.1 s is picture.
        receive_joining(mode, snr_db, seed, if header { 0.0 } else { 1.1 })
    }

    /// [`receive_in_noise`], joining the transmission `skip_s` seconds in:
    /// with the header if that is none of it, without it otherwise.
    fn receive_joining(mode: SstvMode, snr_db: f64, seed: u64, skip_s: f64) -> Reception {
        let header = skip_s == 0.0;
        let rate = 48_000.0;
        let (w, h) = mode.dimensions();
        let (wu, hu) = (w as usize, h as usize);
        let sent = test_card(wu, hu);
        // The transmitter's tone is 0.5 peak, so 0.125 of power; white noise
        // of variance σ² puts σ²·B/(rate/2) of it in B.
        let noise_power = 0.125 / 10f64.powf(snr_db / 10.0);
        let sigma = (noise_power * (rate / 2.0) / SNR_BANDWIDTH_HZ).sqrt();

        let mut tx = SstvTx::new(mode, &sent, w, h, rate, 0.0);
        let mut audio = Vec::new();
        let mut block = vec![0.0f32; 8192];
        while !tx.done() {
            let n = tx.next_block(&mut block);
            audio.extend_from_slice(&block[..n]);
        }
        let skip = (skip_s * rate) as usize;
        let lead = (0.5 * rate) as usize;
        let tail = (2.0 * rate) as usize;
        let mut rng = Noise(seed | 1);
        let air: Vec<f32> = std::iter::repeat_n(0.0, lead)
            .chain(audio[skip.min(audio.len())..].iter().copied())
            .chain(std::iter::repeat_n(0.0, tail))
            .map(|s| (s as f64 + sigma * rng.next()) as f32)
            .collect();

        let mut rx = SstvRx::new(rate);
        rx.set_expected(None);
        let mut got = vec![0u8; sent.len()];
        let mut have = vec![false; hu];
        let mut locked = None;
        let mut by_vis = false;
        // The VIS is decoded before its stop bit, 30 ms ahead of the picture;
        // the quickest free-run lock takes two whole lines.
        let picture = lead as f64 + if header { 0.94 * rate } else { 0.0 };
        let mut fed = 0usize;
        let mut events = Vec::new();
        // Fed in small blocks so a lock is timed to within one of them.
        for chunk in air.chunks(512) {
            rx.process(chunk, &mut events);
            fed += chunk.len();
            for e in events.drain(..) {
                match e {
                    // The first lock is the one that counts: a receiver that
                    // locks on noise first has missed the picture.
                    SstvEvent::ModeDetected(m) if locked.is_none() => {
                        locked = Some(m);
                        by_vis = header && (fed as f64) < picture + 0.05 * rate;
                    }
                    SstvEvent::Line { y, rgb } if locked == Some(mode) => {
                        let at = y as usize * wu * 3;
                        if at + rgb.len() <= got.len() {
                            got[at..at + rgb.len()].copy_from_slice(&rgb);
                            have[y as usize] = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        let rows = have.iter().filter(|&&b| b).count();
        // Error of the delivered rows against the rows sent `off` further
        // down. A receiver that locks on the sync cadence cannot know which
        // line it came in on, and numbers the first one it decodes row 0 — so
        // without the header the picture is compared where it fits best. With
        // it the rows are where they are.
        let err_at = |off: usize| {
            let (mut total, mut n) = (0u64, 0u64);
            for y in (0..hu.saturating_sub(off)).filter(|&y| have[y]) {
                let (g, s) = (&got[y * wu * 3..][..wu * 3], &sent[(y + off) * wu * 3..][..wu * 3]);
                total += g.iter().zip(s).map(|(&a, &b)| a.abs_diff(b) as u64).sum::<u64>();
                n += (wu * 3) as u64;
            }
            if n == 0 { f64::NAN } else { total as f64 / n as f64 }
        };
        let err = if header {
            err_at(0)
        } else {
            (0..=FREERUN_MAX_ROWS_LATE)
                .map(err_at)
                .filter(|e| e.is_finite())
                .fold(f64::NAN, f64::min)
        };
        Reception { mode: locked, by_vis, rows, err }
    }

    /// The harness itself, on a signal far above the noise: every row comes
    /// back, close to what was sent, by either route in. (Close, not exact:
    /// Robot 36 halves its chroma, and the blocks' colour edges cost it about
    /// 8 levels with no noise at all.) If this fails the
    /// sweep below is measuring the harness, not the receiver.
    #[test]
    fn a_strong_signal_comes_through_the_noise_harness_intact() {
        let mode = SstvMode::Robot36;
        let (_, h) = mode.dimensions();
        for header in [true, false] {
            let r = receive_in_noise(mode, 60.0, 1, header);
            assert_eq!(r.mode, Some(mode), "header {header}: locked {:?}", r.mode);
            assert_eq!(r.by_vis, header, "header {header}: locked off the VIS {}", r.by_vis);
            assert!(r.rows > h as usize * 9 / 10, "header {header}: {} rows", r.rows);
            assert!(r.err < 12.0, "header {header}: error {:.1}", r.err);
        }
    }

    /// The header read at 10 dB, where every sample of it is noisy enough to
    /// be misread. It used to be read one sample per bit, and one sample per
    /// sample for the edge the bits are timed from; noise wandering across
    /// that edge filled the candidate list and pushed the real start bit off
    /// it, and nothing below about 50 dB was ever decoded off its VIS.
    #[test]
    fn the_vis_is_read_through_the_noise() {
        for mode in [SstvMode::Robot36, SstvMode::Martin1, SstvMode::Scottie1, SstvMode::Pd120] {
            for seed in 1..=2 {
                let r = receive_in_noise(mode, 10.0, seed, true);
                assert!(
                    r.mode == Some(mode) && r.by_vis,
                    "{} seed {seed}: locked {:?}, by VIS {}",
                    mode.label(),
                    r.mode,
                    r.by_vis
                );
            }
        }
    }

    /// A picture joined partway through, at 4 dB, locked on its sync pulses
    /// alone. That took 14–20 dB while a pulse was a run of samples every one
    /// of which had to be within 150 Hz of 1200: in noise one of them never
    /// is, and the pulse ended there. It is now a window mostly below the
    /// slice, which holds through the noise.
    #[test]
    fn a_picture_joined_late_locks_through_the_noise() {
        for mode in [SstvMode::Robot36, SstvMode::Martin1, SstvMode::Scottie1, SstvMode::Pd120] {
            for seed in 1..=2 {
                let r = receive_in_noise(mode, 4.0, seed, false);
                let (_, h) = mode.dimensions();
                assert_eq!(r.mode, Some(mode), "{} seed {seed}", mode.label());
                assert!(
                    r.rows > h as usize * 9 / 10,
                    "{} seed {seed}: {} rows",
                    mode.label(),
                    r.rows
                );
            }
        }
    }

    /// Robot 36 sends Cr on even lines and Cb on odd ones, and says which in
    /// the separator between luma and chroma. A free-run lock does not know
    /// which line it came in on; locked on an odd one and counting it as line
    /// 0, the receiver read every line's chroma as the other one and painted
    /// the whole picture in the wrong colours. Joined on each in turn, the
    /// colours come out right either way.
    #[test]
    fn robot36_joined_on_either_line_has_its_colours() {
        let line_s = 0.150;
        for extra in 0..4 {
            let r = receive_joining(SstvMode::Robot36, 40.0, 1, 1.1 + extra as f64 * line_s);
            assert_eq!(r.mode, Some(SstvMode::Robot36), "joined {extra} lines later");
            assert!(r.err < 12.0, "joined {extra} lines later: error {:.1}", r.err);
        }
    }

    /// One signal-to-noise ratio of a sweep, and every reception at it.
    type SweepPoint = (f64, Vec<Reception>);

    /// The sensitivity sweep: for a few representative modes, by each route
    /// in, the share of receptions that locked on the right mode, the rows
    /// they delivered and how far those rows were from what was sent, from
    /// well below the point where anything decodes to well above it.
    ///
    /// Slow, so not part of the ordinary run:
    ///
    /// ```text
    /// cargo test --release -p sdroxide-dsp sstv::tests::sensitivity_sweep -- --ignored --nocapture
    /// ```
    ///
    /// `SSTV_SWEEP_MODES` (labels, comma-separated) and `SSTV_SWEEP_SEEDS`
    /// narrow or widen it.
    #[test]
    #[ignore = "slow: a sensitivity measurement, run by hand"]
    fn sensitivity_sweep() {
        let modes: Vec<SstvMode> = match std::env::var("SSTV_SWEEP_MODES") {
            Ok(list) => SstvMode::ALL
                .into_iter()
                .filter(|m| list.split(',').any(|l| l.trim().eq_ignore_ascii_case(m.label())))
                .collect(),
            Err(_) => {
                vec![SstvMode::Robot36, SstvMode::Martin1, SstvMode::Scottie1, SstvMode::Pd120]
            }
        };
        let seeds: u64 =
            std::env::var("SSTV_SWEEP_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
        let snrs: Vec<f64> = (0..=20).map(|k| k as f64 * 2.0).collect();

        let jobs: Vec<(SstvMode, bool)> =
            modes.iter().flat_map(|&m| [(m, true), (m, false)]).collect();
        let results: Vec<(SstvMode, bool, Vec<SweepPoint>)> = std::thread::scope(|s| {
            let handles: Vec<_> = jobs
                .iter()
                .map(|&(mode, header)| {
                    let snrs = &snrs;
                    s.spawn(move || {
                        let rows = snrs
                            .iter()
                            .map(|&snr| {
                                let runs = (0..seeds)
                                    .map(|k| receive_in_noise(mode, snr, 0x5EED + 7919 * k, header))
                                    .collect();
                                (snr, runs)
                            })
                            .collect();
                        (mode, header, rows)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        for (mode, header, rows) in results {
            let (_, h) = mode.dimensions();
            let route = if header { "VIS" } else { "free-run" };
            println!("\n{} via {route}  (SNR in {SNR_BANDWIDTH_HZ} Hz)", mode.label());
            println!("  SNR dB   locked   by VIS   rows %   error");
            for (snr, runs) in rows {
                let n = runs.len() as f64;
                let locked = runs.iter().filter(|r| r.mode == Some(mode)).count();
                let by_vis = runs.iter().filter(|r| r.mode == Some(mode) && r.by_vis).count();
                let rows_pct =
                    runs.iter().map(|r| r.rows as f64).sum::<f64>() / n / h as f64 * 100.0;
                let errs: Vec<f64> = runs.iter().map(|r| r.err).filter(|e| e.is_finite()).collect();
                let err = if errs.is_empty() {
                    "    -".to_string()
                } else {
                    format!("{:5.1}", errs.iter().sum::<f64>() / errs.len() as f64)
                };
                let vis =
                    if header { format!("{by_vis:>2}/{:<2}", runs.len()) } else { "-".into() };
                println!(
                    "  {snr:6.1}   {locked:>2}/{:<2}    {vis:>6}   {rows_pct:5.1}   {err}",
                    runs.len()
                );
            }
        }
    }
}
