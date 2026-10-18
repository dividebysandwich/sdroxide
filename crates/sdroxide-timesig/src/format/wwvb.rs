//! WWVB, 60 kHz, Fort Collins — National Institute of Standards and
//! Technology.
//!
//! Written from NIST Special Publication 432 and NIST's "WWVB Time Code
//! Format" page. At the start of every second the carrier power drops by
//! 17 dB: for 200 ms to send a 0, 500 ms for a 1, and 800 ms for a marker.
//! Markers stand at seconds 0, 9, 19, 29, 39, 49 and 59, so the minute is
//! found where two of them are adjacent.
//!
//! The code names the minute it is sent in, in UTC, at its second 0.
//!
//! | second | meaning |
//! |---|---|
//! | 1–3, 5–8 | minute (40, 20, 10; 8, 4, 2, 1) |
//! | 12–13, 15–18 | hour (20, 10; 8, 4, 2, 1) |
//! | 22–23, 25–28, 30–33 | day of year (200, 100; 80, 40, 20, 10; 8, 4, 2, 1) |
//! | 36–38 | DUT1 sign: 1 0 1 positive, 0 1 0 negative |
//! | 40–43 | DUT1 magnitude (0.8, 0.4, 0.2, 0.1 s) |
//! | 45–48, 50–53 | year of the century (80, 40, 20, 10; 8, 4, 2, 1) |
//! | 55 | leap year |
//! | 56 | leap second at the end of the month |
//! | 57–58 | daylight saving time status |
//! | 4, 10, 11, 14, 20, 21, 24, 34, 35, 44, 54 | unused, always 0 |
//!
//! There is no parity. What stands guard instead is everything that has to be
//! true of a real minute: the unused bits, the sign pattern, BCD digits, a day
//! of the year that exists, and the leap-year bit agreeing with the year.
//!
//! Since 2012 WWVB also phase-modulates its carrier with a second, more
//! robust code. That code is not read here; the amplitude code above is
//! unchanged by it.

use sdroxide_types::{TimeFrame, TimeStation, utc_ymd_hms};

use super::{Context, Format, FrameError, Sym, Template, bits, low};
use crate::civil::{bcd, bcd_digits_ok, day_of_year, from_day_of_year, is_leap, to_bcd};

pub(super) static FORMAT: Format = Format {
    station: TimeStation::Wwvb,
    templates: &[
        Template { sym: Sym::ZERO, low: low(0, 2) },
        Template { sym: Sym::ONE, low: low(0, 5) },
        Template { sym: Sym::MARKER, low: low(0, 8) },
    ],
    rising_edge: false,
    frame_len: 60,
    time_index: 0,
    encode,
    decode,
};

pub(super) const MARKERS: [usize; 7] = [0, 9, 19, 29, 39, 49, 59];
const UNUSED: [usize; 11] = [4, 10, 11, 14, 20, 21, 24, 34, 35, 44, 54];

pub(super) const MIN_AT: [usize; 7] = [1, 2, 3, 5, 6, 7, 8];
pub(super) const MIN_W: [u32; 7] = [40, 20, 10, 8, 4, 2, 1];
pub(super) const HOUR_AT: [usize; 6] = [12, 13, 15, 16, 17, 18];
pub(super) const HOUR_W: [u32; 6] = [20, 10, 8, 4, 2, 1];
pub(super) const DOY_AT: [usize; 10] = [22, 23, 25, 26, 27, 28, 30, 31, 32, 33];
pub(super) const DOY_W: [u32; 10] = [200, 100, 80, 40, 20, 10, 8, 4, 2, 1];
const YEAR_AT: [usize; 8] = [45, 46, 47, 48, 50, 51, 52, 53];
const YEAR_W: [u32; 8] = [80, 40, 20, 10, 8, 4, 2, 1];

/// Sets `bits` at `at` to `value` in the weights given. Day of the year has a
/// hundreds digit, which [`to_bcd`] does not take, so it is split off here.
pub(super) fn put(bits: &mut [bool; 60], at: &[usize], weights: &[u32], value: u32) {
    let split = weights.iter().take_while(|&&w| w >= 100).count();
    let mut hundreds = value / 100;
    let mut field = Vec::with_capacity(weights.len());
    for &w in &weights[..split] {
        let set = hundreds >= w / 100;
        if set {
            hundreds -= w / 100;
        }
        field.push(set);
    }
    field.extend(to_bcd(value % 100, &weights[split..]));
    for (&i, v) in at.iter().zip(field) {
        bits[i] = v;
    }
}

/// Reads a field written by [`put`].
pub(super) fn get(w: &[Sym], at: &[usize], weights: &[u32]) -> Result<u32, FrameError> {
    let b = bits(w, at.iter().copied());
    let split = weights.iter().take_while(|&&w| w >= 100).count();
    if !bcd_digits_ok(&b[split..], &weights[split..]) {
        return Err(FrameError::Range("BCD digit"));
    }
    Ok(bcd(b, weights))
}

fn encode(minute_utc: i64) -> [Sym; 60] {
    let (y, _, _, h, mi, _) = utc_ymd_hms(minute_utc);
    let mut b = [false; 60];
    put(&mut b, &MIN_AT, &MIN_W, mi);
    put(&mut b, &HOUR_AT, &HOUR_W, h);
    put(&mut b, &DOY_AT, &DOY_W, day_of_year(minute_utc));
    // DUT1 of +0.0 s: the positive sign and a zero magnitude.
    b[36] = true;
    b[38] = true;
    put(&mut b, &YEAR_AT, &YEAR_W, (y % 100) as u32);
    b[55] = is_leap(y);
    let mut out = [Sym::ZERO; 60];
    for (s, bit) in out.iter_mut().zip(b) {
        *s = Sym::bit(bit);
    }
    for m in MARKERS {
        out[m] = Sym::MARKER;
    }
    out
}

fn decode(w: &[Sym], _ctx: &Context) -> Result<TimeFrame, FrameError> {
    if w.len() != 60 || (0..60).any(|i| w[i].marker != MARKERS.contains(&i)) {
        return Err(FrameError::Framing);
    }
    if UNUSED.iter().any(|&i| w[i].a) {
        return Err(FrameError::Range("unused bit set"));
    }
    let sign = (w[36].a, w[37].a, w[38].a);
    let negative = match sign {
        (true, false, true) => false,
        (false, true, false) => true,
        _ => return Err(FrameError::Range("DUT1 sign")),
    };
    let mi = get(w, &MIN_AT, &MIN_W)?;
    let h = get(w, &HOUR_AT, &HOUR_W)?;
    let doy = get(w, &DOY_AT, &DOY_W)?;
    let y = 2000 + i64::from(get(w, &YEAR_AT, &YEAR_W)?);
    if mi > 59 || h > 23 {
        return Err(FrameError::Range("time"));
    }
    if w[55].a != is_leap(y) {
        return Err(FrameError::Range("leap-year bit"));
    }
    let day = from_day_of_year(y, doy).ok_or(FrameError::Range("day of year"))?;
    let dut1 = bcd(bits(w, 40..44), &[8, 4, 2, 1]) as i16 * 100;
    let (dst_today, dst_tomorrow) = (w[57].a, w[58].a);
    Ok(TimeFrame {
        utc: day + i64::from(h) * 3600 + i64::from(mi) * 60,
        zone_min: 0,
        dst: dst_today && dst_tomorrow,
        dst_change_soon: dst_today != dst_tomorrow,
        leap_second_soon: w[56].a,
        dut1_ms: Some(if negative { -dut1 } else { dut1 }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::ymd_hms_to_unix;

    #[test]
    fn day_291_of_2026_at_12_37_utc() {
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let s = encode(t);
        let a = |i: usize| u8::from(s[i].a);
        for m in MARKERS {
            assert!(s[m].marker, "marker at {m}");
        }
        // 37: 40 20 10 = 0 1 1, then 8 4 2 1 = 0 1 1 1.
        assert_eq!([1, 2, 3, 5, 6, 7, 8].map(a), [0, 1, 1, 0, 1, 1, 1]);
        // 12: 20 10 = 0 1, 8 4 2 1 = 0 0 1 0.
        assert_eq!([12, 13, 15, 16, 17, 18].map(a), [0, 1, 0, 0, 1, 0]);
        // 291: 200 100 = 1 0; 80 40 20 10 = 1 0 0 1; 8 4 2 1 = 0 0 0 1.
        assert_eq!(DOY_AT.map(a), [1, 0, 1, 0, 0, 1, 0, 0, 0, 1]);
        assert_eq!(decode(&s, &Context::default()).map(|f| f.utc), Ok(t));
    }

    #[test]
    fn the_leap_year_bit_has_to_agree_with_the_year() {
        let t = ymd_hms_to_unix(2026, 5, 1, 0, 0, 0);
        let mut s = encode(t);
        s[55].a = true;
        assert_eq!(decode(&s, &Context::default()), Err(FrameError::Range("leap-year bit")));
    }
}
