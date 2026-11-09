//! WWV and WWVH, the short-wave services of the National Institute of
//! Standards and Technology — the same time code from Fort Collins and from
//! Kauaʻi.
//!
//! Written from NIST Special Publication 432 (appendix B, "WWV/WWVH Time
//! Code") and NIST's "WWV and WWVH Digital Time Code and Broadcast Format"
//! page, with the bit positions as tabulated from NIST's time-code figure.
//!
//! The code rides a 100 Hz subcarrier, one symbol a second. The subcarrier is
//! held at its high level from 30 ms after the second — the first 30 ms are
//! kept clear for the seconds tick — until 200 ms for a 0, 500 ms for a 1 and
//! 800 ms for a position marker, and at a level 15 dB lower for the rest of the
//! second. Second 0 carries no pulse at all: the hole is what marks the minute.
//! Markers stand at seconds 9, 19, 29, 39, 49 and 59.
//!
//! Every number is binary-coded decimal sent *least* significant bit first,
//! the reverse of WWVB, and the frame names the minute it begins.
//!
//! | second | meaning |
//! |---|---|
//! | 2 | DST status at 00:00 UTC today |
//! | 3 | leap second at the end of the month |
//! | 4–7 | year, units (1, 2, 4, 8) |
//! | 10–13, 15–17 | minute (1, 2, 4, 8; 10, 20, 40) |
//! | 20–23, 25–26 | hour (1, 2, 4, 8; 10, 20) |
//! | 30–33, 35–38, 40–41 | day of year (1, 2, 4, 8; 10, 20, 40, 80; 100, 200) |
//! | 50 | DUT1 sign, 1 for positive |
//! | 51–54 | year, tens (10, 20, 40, 80) |
//! | 55 | DST status at 24:00 UTC today |
//! | 56–58 | DUT1 magnitude (0.1, 0.2, 0.4 s) |
//! | 1, 8, 14, 18, 24, 27, 28, 34, 42–48 | unused, always 0 |
//!
//! There is no parity, as on WWVB. What stands guard is the hole and the
//! markers, the fifteen unused bits, BCD digits, and a day of the year that
//! exists — and, as everywhere, consecutive minutes agreeing.
//!
//! The seconds themselves are not in the code: they are the ticks, a 5 ms
//! burst of 1000 Hz (WWV) or 1200 Hz (WWVH) starting exactly on each second
//! but the 29th and 59th, which the decoder times its seconds from.

use sdroxide_types::{TimeFrame, TimeStation, utc_ymd_hms};

use super::{Context, Format, FrameError, Sym, Template, bits, low};
use crate::civil::{bcd, bcd_digits_ok, day_of_year, from_day_of_year, to_bcd};

/// Both stations send the same patterns. "Reduced" here is the subcarrier at
/// its low level; slot 0 is high from 30 ms on, which the slot reading sees
/// as mostly high, for every symbol alike.
const TEMPLATES: &[Template] = &[
    Template { sym: Sym::ZERO, low: low(2, 10) },
    Template { sym: Sym::ONE, low: low(5, 10) },
    Template { sym: Sym::MARKER, low: low(8, 10) },
    Template { sym: Sym::HOLE, low: low(0, 10) },
];

pub(super) static FORMAT_WWV: Format = Format {
    station: TimeStation::Wwv,
    templates: TEMPLATES,
    rising_edge: false,
    frame_len: 60,
    time_index: 0,
    encode,
    decode,
    fill: None,
};

pub(super) static FORMAT_WWVH: Format = Format { station: TimeStation::Wwvh, ..FORMAT_WWV };

const MARKERS: [usize; 6] = [9, 19, 29, 39, 49, 59];
const UNUSED: [usize; 15] = [1, 8, 14, 18, 24, 27, 28, 34, 42, 43, 44, 45, 46, 47, 48];

const MIN_AT: [usize; 7] = [10, 11, 12, 13, 15, 16, 17];
const MIN_W: [u32; 7] = [1, 2, 4, 8, 10, 20, 40];
const HOUR_AT: [usize; 6] = [20, 21, 22, 23, 25, 26];
const HOUR_W: [u32; 6] = [1, 2, 4, 8, 10, 20];
const DOY_AT: [usize; 8] = [30, 31, 32, 33, 35, 36, 37, 38];
const DOY_W: [u32; 8] = [1, 2, 4, 8, 10, 20, 40, 80];
const DOY_HUNDREDS_AT: [usize; 2] = [40, 41];
const YEAR_AT: [usize; 8] = [4, 5, 6, 7, 51, 52, 53, 54];
const YEAR_W: [u32; 8] = [1, 2, 4, 8, 10, 20, 40, 80];
const DUT1_AT: [usize; 3] = [56, 57, 58];

fn put(b: &mut [bool; 60], at: &[usize], weights: &[u32], value: u32) {
    for (&i, v) in at.iter().zip(to_bcd(value, weights)) {
        b[i] = v;
    }
}

fn get(w: &[Sym], at: &[usize], weights: &[u32]) -> Result<u32, FrameError> {
    let b = bits(w, at.iter().copied());
    if !bcd_digits_ok(&b, weights) {
        return Err(FrameError::Range("BCD digit"));
    }
    Ok(bcd(b, weights))
}

fn encode(minute_utc: i64) -> [Sym; 60] {
    let (y, _, _, h, mi, _) = utc_ymd_hms(minute_utc);
    let mut b = [false; 60];
    put(&mut b, &MIN_AT, &MIN_W, mi);
    put(&mut b, &HOUR_AT, &HOUR_W, h);
    let doy = day_of_year(minute_utc);
    put(&mut b, &DOY_AT, &DOY_W, doy % 100);
    b[40] = doy / 100 % 2 == 1;
    b[41] = doy / 200 == 1;
    put(&mut b, &YEAR_AT, &YEAR_W, (y % 100) as u32);
    // DUT1 of +0.0 s: the positive sign and a zero magnitude.
    b[50] = true;
    let mut out = [Sym::ZERO; 60];
    for (s, bit) in out.iter_mut().zip(b) {
        *s = Sym::bit(bit);
    }
    out[0] = Sym::HOLE;
    for m in MARKERS {
        out[m] = Sym::MARKER;
    }
    out
}

fn decode(w: &[Sym], _ctx: &Context) -> Result<TimeFrame, FrameError> {
    if w.len() != 60
        || (0..60).any(|i| w[i].marker != MARKERS.contains(&i) || w[i].hole != (i == 0))
    {
        return Err(FrameError::Framing);
    }
    if UNUSED.iter().any(|&i| w[i].a) {
        return Err(FrameError::Range("unused bit set"));
    }
    let mi = get(w, &MIN_AT, &MIN_W)?;
    let h = get(w, &HOUR_AT, &HOUR_W)?;
    let doy = get(w, &DOY_AT, &DOY_W)? + bcd(bits(w, DOY_HUNDREDS_AT), &[100, 200]);
    let y = 2000 + i64::from(get(w, &YEAR_AT, &YEAR_W)?);
    if mi > 59 || h > 23 {
        return Err(FrameError::Range("time"));
    }
    // No leap-year bit to check, unlike WWVB: the year's own length decides
    // whether day 366 exists.
    let day = from_day_of_year(y, doy).ok_or(FrameError::Range("day of year"))?;
    let dut1 = bcd(bits(w, DUT1_AT), &[1, 2, 4]) as i16 * 100;
    let (dst_today, dst_tonight) = (w[2].a, w[55].a);
    Ok(TimeFrame {
        utc: day + i64::from(h) * 3600 + i64::from(mi) * 60,
        zone_min: 0,
        // The status at the start of the UTC day, which is what holds until
        // the change at 02:00 local time on the day the two bits differ.
        dst: dst_today,
        dst_change_soon: dst_today != dst_tonight,
        leap_second_soon: w[3].a,
        dut1_ms: Some(if w[50].a { dut1 } else { -dut1 }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::ymd_hms_to_unix;

    /// The worked example tabulated from NIST's time-code figure: day 86 of
    /// 2009, 21:30 UTC, DUT1 +0.3 s, no summer time, no leap second.
    #[test]
    fn the_nist_example_frame() {
        let ones = [4, 7, 15, 16, 20, 26, 31, 32, 38, 50, 56, 57];
        let mut w = [Sym::ZERO; 60];
        for i in ones {
            w[i] = Sym::ONE;
        }
        w[0] = Sym::HOLE;
        for m in MARKERS {
            w[m] = Sym::MARKER;
        }
        let f = decode(&w, &Context::default()).expect("the example decodes");
        assert_eq!(f.utc, ymd_hms_to_unix(2009, 3, 27, 21, 30, 0));
        assert_eq!(f.dut1_ms, Some(300));
        assert!(!f.dst && !f.dst_change_soon && !f.leap_second_soon);
    }

    #[test]
    fn day_291_of_2026_at_12_37_utc() {
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let s = encode(t);
        let a = |i: usize| u8::from(s[i].a);
        assert!(s[0].hole);
        // 37, least significant first: 7 = 1 1 1 0, then 30 = 1 1 0.
        assert_eq!(MIN_AT.map(a), [1, 1, 1, 0, 1, 1, 0]);
        // 291: 1 = 1 0 0 0; 90 = 1 0 0 1; 200 = 0 1.
        assert_eq!(DOY_AT.map(a), [1, 0, 0, 0, 1, 0, 0, 1]);
        assert_eq!(DOY_HUNDREDS_AT.map(a), [0, 1]);
        // 26: 6 = 0 1 1 0, 20 = 0 1 0 0.
        assert_eq!(YEAR_AT.map(a), [0, 1, 1, 0, 0, 1, 0, 0]);
        assert_eq!(decode(&s, &Context::default()).map(|f| f.utc), Ok(t));
    }

    #[test]
    fn a_pulse_where_the_hole_should_be_is_not_a_minute() {
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let mut s = encode(t);
        s[0] = Sym::ZERO;
        assert_eq!(decode(&s, &Context::default()), Err(FrameError::Framing));
    }

    #[test]
    fn day_366_only_in_a_leap_year() {
        let mut s = encode(ymd_hms_to_unix(2024, 12, 31, 10, 0, 0));
        assert!(decode(&s, &Context::default()).is_ok());
        // The same day number in 2025: year units 4 → 5 sets bit 4.
        s[4].a = true;
        assert_eq!(decode(&s, &Context::default()), Err(FrameError::Range("day of year")));
    }
}
