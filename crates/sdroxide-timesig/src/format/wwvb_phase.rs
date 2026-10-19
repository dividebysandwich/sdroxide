//! WWVB's phase code: one bit a second in the sign of the carrier.
//!
//! Written from NIST's "Enhanced WWVB Broadcast Format", revision 1.01
//! (John Lowe, 6 November 2013). Since 2012 the carrier is inverted for a 1
//! and left alone for a 0, binary phase-shift keying at 1 bit/s laid over the
//! amplitude code without disturbing it. Each bit begins 100 ms after the
//! amplitude drop that starts its second, and NIST recommends reading it only
//! from the second's full-power part — which is also where nearly all its
//! energy is.
//!
//! A minute frame is sixty bits, aligned with the amplitude code's minute:
//!
//! | second | meaning |
//! |---|---|
//! | 0–12 | sync word, `0 0 1 1 1 0 1 1 0 1 0 0 0` for a time frame |
//! | 13–17 | Hamming parity over the time word, `par[4]` first |
//! | 18, 20–28, 30–38, 40–46 | the time word, most significant bit first |
//! | 19 | the time word's least significant bit again |
//! | 29, 39 | reserved |
//! | 47–48, 50–52 | daylight saving time and leap second, a 5-bit code |
//! | 49 | a notice is posted on NIST's site |
//! | 53–58 | the next DST transition's schedule |
//! | 59 | always 0, and part of the sync |
//!
//! The time word is a 26-bit count of minutes since 2000-01-01 00:00 UTC
//! naming the minute the frame is sent in, protected by a Hamming (31, 26)
//! code that corrects any one error. A receiver has no absolute phase to read
//! the bits against — its reference comes from the carrier with the
//! modulation squared away, which is right or exactly upside down — so the
//! sync word decides both where the minute is and which way up the bits are.
//!
//! Twice an hour, from XX:10 and XX:40, six one-minute frames are replaced by
//! one 360-bit "extended" symbol carrying only the time of day. Those minutes
//! fail the sync and are simply not read from the phase; the amplitude code
//! carries on underneath as always. They are generated here so the synthetic
//! station sends what the real one does.

use sdroxide_types::{TimeFrame, ymd_hms_to_unix};

use super::FrameError;

/// `sync_T[12]` … `sync_T[0]`, sent in seconds 0–12 of a time frame.
pub const SYNC_T: [bool; 13] = bits13(0b0011101101000);

const fn bits13(v: u16) -> [bool; 13] {
    let mut out = [false; 13];
    let mut i = 0;
    while i < 13 {
        out[i] = v >> (12 - i) & 1 == 1;
        i += 1;
    }
    out
}

/// Seconds of the frame that hold the sync, with second 59 — always 0 —
/// appended as NIST suggests, for fourteen in all.
const SYNC_AT: [usize; 14] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 59];

fn sync_bit(i: usize) -> bool {
    if i == 59 { false } else { SYNC_T[i] }
}

/// Where each bit of the 31-bit code word is sent: `time_par[4]` … `[0]`,
/// then `time[25]` … `time[0]`.
const PAR_AT: [usize; 5] = [13, 14, 15, 16, 17];
const TIME_AT: [usize; 26] = [
    18, 20, 21, 22, 23, 24, 25, 26, 27, 28, 30, 31, 32, 33, 34, 35, 36, 37, 38, 40, 41, 42, 43, 44,
    45, 46,
];
/// Where `time[0]` is sent a second time.
const TIME_LSB_AGAIN: usize = 19;
const DST_LS_AT: [usize; 5] = [47, 48, 50, 51, 52];
const DST_NEXT_AT: [usize; 6] = [53, 54, 55, 56, 57, 58];
const NOTICE_AT: usize = 49;

/// The time bits each parity bit covers, from section 4.3.
const PARITY_TAPS: [&[u8]; 5] = [
    &[23, 21, 20, 17, 16, 15, 14, 13, 9, 8, 6, 5, 4, 2, 0],
    &[24, 22, 21, 18, 17, 16, 15, 14, 10, 9, 7, 6, 5, 3, 1],
    &[25, 23, 22, 19, 18, 17, 16, 15, 11, 10, 8, 7, 6, 4, 2],
    &[24, 21, 19, 18, 15, 14, 13, 12, 11, 7, 6, 4, 3, 2, 0],
    &[25, 22, 20, 19, 16, 15, 14, 13, 12, 8, 7, 5, 4, 3, 1],
];

/// The twelve legitimate values of the DST and leap-second word
/// `dst_ls[4..0]`, Table 4, with what each means: (`dst_on[1..0]`,
/// `leap_sec[1]`). `leap_sec[0]` — positive or negative — is not kept.
const DST_LS: [(u8, u8, bool); 12] = [
    (0b01000, 0b00, false),
    (0b10110, 0b10, false),
    (0b00011, 0b11, false),
    (0b10101, 0b01, false),
    (0b00100, 0b00, true),
    (0b10000, 0b10, true),
    (0b01101, 0b11, true),
    (0b01110, 0b01, true),
    (0b11001, 0b00, true),
    (0b11010, 0b10, true),
    (0b11111, 0b11, true),
    (0b11100, 0b01, true),
];

/// The minute counter's epoch, 2000-01-01 00:00 UTC.
fn epoch() -> i64 {
    ymd_hms_to_unix(2000, 1, 1, 0, 0, 0)
}

/// The largest count that names a minute of this century.
fn max_count() -> i64 {
    (ymd_hms_to_unix(2100, 1, 1, 0, 0, 0) - epoch()) / 60 - 1
}

fn parity_of(time: u32) -> u8 {
    let mut p = 0u8;
    for (k, taps) in PARITY_TAPS.iter().enumerate() {
        let odd = taps.iter().filter(|&&j| time >> j & 1 == 1).count() % 2 == 1;
        p |= u8::from(odd) << k;
    }
    p
}

/// The syndrome a single error in each code-word bit produces: the parity
/// bits' own, then each time bit's taps.
fn syndrome_of_par(k: usize) -> u8 {
    1 << k
}

fn syndrome_of_time(j: usize) -> u8 {
    parity_of(1 << j)
}

/// The fields of a time frame beyond the minute itself.
#[derive(Debug, Clone, Copy)]
pub struct Extras {
    /// `dst_on[1..0]`: 00 standard time, 10 DST starts today, 11 DST, 01 DST
    /// ends today.
    pub dst_on: u8,
    pub leap_soon: bool,
    pub dst_next: u8,
    pub notice: bool,
    /// The two reserved bits, 29 and 39.
    pub reserved: (bool, bool),
}

impl Default for Extras {
    fn default() -> Self {
        // Standard time, no leap second, and "the next change is the second
        // Sunday of March at 2 AM" — the schedule in force when written.
        Extras {
            dst_on: 0b00,
            leap_soon: false,
            dst_next: 0b100110,
            notice: false,
            reserved: (false, false),
        }
    }
}

/// The bits of the time frame for the minute starting at `minute_utc`.
pub fn encode(minute_utc: i64) -> [bool; 60] {
    encode_with(minute_utc, Extras::default())
}

pub fn encode_with(minute_utc: i64, x: Extras) -> [bool; 60] {
    let count = ((minute_utc - epoch()) / 60) as u32;
    let mut b = [false; 60];
    for (i, &at) in SYNC_AT.iter().enumerate().take(13) {
        b[at] = SYNC_T[i];
    }
    let par = parity_of(count);
    for (i, &at) in PAR_AT.iter().enumerate() {
        b[at] = par >> (4 - i) & 1 == 1;
    }
    for (i, &at) in TIME_AT.iter().enumerate() {
        b[at] = count >> (25 - i) & 1 == 1;
    }
    b[TIME_LSB_AGAIN] = count & 1 == 1;
    let code = DST_LS
        .iter()
        .find(|&&(_, on, leap)| on == x.dst_on && leap == x.leap_soon)
        .map_or(0b01000, |&(c, ..)| c);
    for (i, &at) in DST_LS_AT.iter().enumerate() {
        b[at] = code >> (4 - i) & 1 == 1;
    }
    for (i, &at) in DST_NEXT_AT.iter().enumerate() {
        b[at] = x.dst_next >> (5 - i) & 1 == 1;
    }
    b[NOTICE_AT] = x.notice;
    (b[29], b[39]) = x.reserved;
    b
}

/// The phase bit WWVB sends during the UTC second `sec` — a time frame's, or
/// during the half-hourly extended symbols, theirs.
pub fn bit_at(sec: i64) -> bool {
    let minute = sec.div_euclid(60) * 60;
    let into_hour = sec.rem_euclid(3600);
    let start = [600, 2400].into_iter().find(|&s| (s..s + 360).contains(&into_hour));
    match start {
        Some(s) => {
            let hour_start = sec - into_hour;
            let half_hour = (hour_start.rem_euclid(86_400) / 3600) * 2 + i64::from(s == 2400);
            extended_bit(half_hour as usize, (into_hour - s) as usize)
        }
        None => encode(minute)[(sec - minute) as usize],
    }
}

/// The 127-bit sequence #1 of the extended symbols: the LFSR of figure 8,
/// g(x) = x⁷ + x⁶ + x⁵ + x² + 1, from the state 1111111.
fn prbs() -> [bool; 127] {
    let mut a = [false; 127];
    a[..7].fill(true);
    for n in 7..127 {
        a[n] = a[n - 2] ^ a[n - 5] ^ a[n - 6] ^ a[n - 7];
    }
    a
}

/// The fixed 106-bit timing word in the middle of every extended symbol,
/// Table 12.
const TIMING_WORD: &str = "1101000111 0101100101 1001101110 0011000010 1101001110 \
                           1001010100 0010111000 1011010110 1101111111 1000000100 100100";

/// Bit `i` (0–359) of the extended symbol sent at the half hour `half_hour`
/// (0 for 00:10, 1 for 00:40, … 47 for 23:40), outside summer time — what
/// the synthetic station sends.
fn extended_bit(half_hour: usize, i: usize) -> bool {
    // Sequence #(2h + 1) is #1 rotated left by 2h (Table 11's first column).
    let shift = 2 * half_hour;
    let seq = |k: usize| prbs()[(k + shift) % 127];
    match i {
        0..127 => seq(i),
        127..233 => TIMING_WORD.chars().filter(|c| !c.is_whitespace()).nth(i - 127) == Some('1'),
        _ => seq(359 - i),
    }
}

/// How sure the decoder is of a frame it read.
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    /// The sync word's correlation, in standard deviations of noise.
    pub sync_z: f32,
    /// Which way up the bits were: +1 if a positive soft value was a 0.
    pub polarity: f32,
    /// Whether the Hamming code corrected a bit.
    pub corrected: bool,
}

/// The sync word's correlation must stand this many standard deviations
/// above noise. Fourteen bits; noise alone exceeds 5 σ about once in two
/// million tries, and a try is made once a second.
const SYNC_Z: f32 = 5.0;
/// Hard sync bits allowed wrong once the correlation clears [`SYNC_Z`].
const SYNC_ERRORS: usize = 1;
/// A single error is corrected only if the bit it names was among this many
/// least reliable — otherwise the "error" is a sign of a second one, which the
/// code cannot correct and would only make worse.
const CORRECT_AMONG: usize = 2;

/// A minute from sixty soft bits, second 0 first.
///
/// `soft[i]` is second `i`'s bit as a signed confidence in units of the
/// noise's standard deviation, positive for one phase and negative for the
/// other; which is which is found from the sync.
pub fn decode(soft: &[f32; 60]) -> Result<(TimeFrame, Reading), FrameError> {
    let c: f32 = SYNC_AT.iter().map(|&i| soft[i] * if sync_bit(i) { -1.0 } else { 1.0 }).sum();
    let sync_z = c.abs() / (SYNC_AT.len() as f32).sqrt();
    let polarity = c.signum();
    // In that polarity, positive is a 0.
    let v: Vec<f32> = soft.iter().map(|s| s * polarity).collect();
    let errors = SYNC_AT.iter().filter(|&&i| (v[i] < 0.0) != sync_bit(i)).count();
    if sync_z < SYNC_Z || errors > SYNC_ERRORS {
        return Err(FrameError::Framing);
    }
    // The 31-bit code word as reliabilities, the LSB's two copies combined.
    let mut rel_par: Vec<f32> = PAR_AT.iter().map(|&i| v[i]).collect();
    let mut rel_time: Vec<f32> = TIME_AT.iter().map(|&i| v[i]).collect();
    rel_time[25] += v[TIME_LSB_AGAIN];
    let hard = |r: f32| r < 0.0;
    let word = |rt: &[f32]| (0..26).fold(0u32, |w, i| w | u32::from(hard(rt[i])) << (25 - i));
    let par_word = |rp: &[f32]| (0..5).fold(0u8, |w, i| w | u8::from(hard(rp[i])) << (4 - i));
    let mut time = word(&rel_time);
    let syndrome = parity_of(time) ^ par_word(&rel_par);
    let mut corrected = false;
    if syndrome != 0 {
        // A perfect code: every non-zero syndrome names exactly one bit.
        let mut by_rel: Vec<f32> = rel_par.iter().chain(&rel_time).map(|r| r.abs()).collect();
        by_rel.sort_by(f32::total_cmp);
        let weak = by_rel[CORRECT_AMONG - 1];
        let flip_par = (0..5).find(|&i| syndrome_of_par(4 - i) == syndrome);
        let flip_time = (0..26).find(|&i| syndrome_of_time(25 - i) == syndrome);
        match (flip_par, flip_time) {
            (Some(i), _) if rel_par[i].abs() <= weak => rel_par[i] = -rel_par[i],
            (_, Some(i)) if rel_time[i].abs() <= weak => rel_time[i] = -rel_time[i],
            _ => return Err(FrameError::Parity("time word")),
        }
        time = word(&rel_time);
        debug_assert_eq!(parity_of(time), par_word(&rel_par));
        corrected = true;
    }
    if i64::from(time) > max_count() {
        return Err(FrameError::Range("minute count"));
    }
    // The DST and leap-second word is an error-detecting code of its own;
    // the likeliest of its twelve values is taken, since it is not what the
    // time depends on.
    let score = |code: u8| -> f32 {
        DST_LS_AT
            .iter()
            .enumerate()
            .map(|(k, &at)| if code >> (4 - k) & 1 == 1 { -v[at] } else { v[at] })
            .sum()
    };
    let &(_, dst_on, leap_soon) =
        DST_LS.iter().max_by(|a, b| score(a.0).total_cmp(&score(b.0))).expect("twelve codes");
    Ok((
        TimeFrame {
            utc: epoch() + i64::from(time) * 60,
            zone_min: 0,
            dst: dst_on == 0b11,
            dst_change_soon: dst_on == 0b10 || dst_on == 0b01,
            leap_second_soon: leap_soon,
            dut1_ms: None,
        },
        Reading { sync_z, polarity, corrected },
    ))
}

/// The bit a soft value says, in a known polarity: `None` when it is too
/// close to call for the panel to show.
pub fn bit(soft: f32, polarity: f32) -> Option<bool> {
    let v = soft * polarity;
    (v.abs() >= 1.0).then_some(v < 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Table 10's phase row: 17:30 UTC on 4 July 2012, DST in effect, the
    /// notice bit set, and the reserved bits 0 and 1.
    const TABLE_10: &str = "0011101101 0001001000 0011001000 0110001101 0011010001 0110110110";

    fn table_10() -> [bool; 60] {
        let mut b = [false; 60];
        for (i, c) in TABLE_10.chars().filter(|c| !c.is_whitespace()).enumerate() {
            b[i] = c == '1';
        }
        b
    }

    fn soft(b: &[bool; 60], level: f32, polarity: f32) -> [f32; 60] {
        b.map(|x| polarity * if x { -level } else { level })
    }

    #[test]
    fn the_nist_example_frame() {
        let t = ymd_hms_to_unix(2012, 7, 4, 17, 30, 0);
        assert_eq!((t - epoch()) / 60, 6_578_970, "the count section 6 gives");
        assert_eq!(parity_of(6_578_970), 0b10010, "the parity section 6 gives");
        let x = Extras {
            dst_on: 0b11,
            leap_soon: false,
            dst_next: 0b011011,
            notice: true,
            reserved: (false, true),
        };
        assert_eq!(encode_with(t, x), table_10());
        for polarity in [1.0, -1.0] {
            let (f, r) = decode(&soft(&table_10(), 6.0, polarity)).expect("decodes");
            assert_eq!(f.utc, t);
            assert!(f.dst && !f.dst_change_soon && !f.leap_second_soon);
            assert_eq!(r.polarity, polarity);
            assert!(!r.corrected);
        }
    }

    #[test]
    fn the_hamming_code_is_perfect() {
        // Every one of the 31 bits has its own non-zero syndrome, which is
        // what lets one error anywhere be found.
        let mut seen: Vec<u8> = (0..5).map(syndrome_of_par).collect();
        seen.extend((0..26).map(syndrome_of_time));
        let mut s = seen.clone();
        s.sort_unstable();
        s.dedup();
        assert_eq!(s.len(), 31);
        assert!(!s.contains(&0));
    }

    #[test]
    fn one_weak_error_is_corrected_and_one_strong_error_is_refused() {
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let b = encode(t);
        for &at in PAR_AT.iter().chain(&TIME_AT) {
            if at == 46 {
                // The LSB's second copy outvotes a weak error in the first.
                continue;
            }
            let mut s = soft(&b, 4.0, 1.0);
            s[at] *= -0.3;
            let (f, r) = decode(&s).unwrap_or_else(|e| panic!("bit {at}: {e:?}"));
            assert_eq!(f.utc, t, "bit {at}");
            assert!(r.corrected, "bit {at}");
            // The same error, but confidently wrong while two other bits are
            // weak: something else is going on, and nothing is believed.
            let mut s = soft(&b, 4.0, 1.0);
            s[at] = -s[at];
            let others: Vec<usize> =
                PAR_AT.iter().chain(&TIME_AT).copied().filter(|&i| i != at).take(2).collect();
            for i in others {
                s[i] *= 0.1;
            }
            assert!(decode(&s).is_err(), "bit {at} confidently wrong was corrected");
        }
    }

    #[test]
    fn every_minute_of_a_day_round_trips() {
        let day = ymd_hms_to_unix(2031, 2, 28, 0, 0, 0);
        for m in 0..1440 {
            let t = day + m * 60;
            let (f, _) = decode(&soft(&encode(t), 3.0, -1.0)).expect("decodes");
            assert_eq!(f.utc, t);
        }
    }

    #[test]
    fn the_extended_sequence_is_the_one_nist_prints() {
        const SEQ_1: &str = "1111111001101101010100010010011001111000111\
                             0111010111101001011001010011100100011000101\
                             11000010000110100000111110110000001010110";
        let want: Vec<bool> = SEQ_1.chars().map(|c| c == '1').collect();
        assert_eq!(want.len(), 127);
        assert_eq!(prbs().to_vec(), want);
        assert_eq!(TIMING_WORD.chars().filter(|c| !c.is_whitespace()).count(), 106);
        // The extended minutes never pass for a time frame, at any
        // alignment: the decoder reads them as no minute at all.
        let start = ymd_hms_to_unix(2026, 10, 18, 12, 10, 0);
        for off in 0..300 {
            let mut b = [false; 60];
            for (i, x) in b.iter_mut().enumerate() {
                *x = bit_at(start + off + i as i64);
            }
            assert!(decode(&soft(&b, 5.0, 1.0)).is_err(), "decoded at +{off} s");
            assert!(decode(&soft(&b, 5.0, -1.0)).is_err(), "decoded at +{off} s");
        }
    }

    #[test]
    fn a_time_frame_is_only_found_where_its_minute_starts() {
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        for off in 1..60 {
            let mut b = [false; 60];
            for (i, x) in b.iter_mut().enumerate() {
                *x = bit_at(t + off + i as i64);
            }
            assert!(decode(&soft(&b, 5.0, 1.0)).is_err(), "decoded {off} s late");
            assert!(decode(&soft(&b, 5.0, -1.0)).is_err(), "decoded {off} s late");
        }
    }
}
