//! DCF77, 77.5 kHz, Mainflingen — Physikalisch-Technische Bundesanstalt.
//!
//! Written from PTB's description of the service ("DCF77", PTB-Mitteilungen
//! and the PTB web pages on the time code): at the start of every second
//! except the fifty-ninth the carrier drops to about 15 % of its amplitude,
//! for 100 ms to send a 0 and 200 ms to send a 1. The missing drop in second
//! 59 is the minute marker, and the next drop starts the minute.
//!
//! What the 59 bits say is the *coming* minute: the time and date, in the
//! German civil zone (CET or CEST), valid at the start of the second that
//! follows the marker.
//!
//! | second | meaning |
//! |---|---|
//! | 0 | start of minute, always 0 |
//! | 1–14 | weather and civil-protection data (encrypted; not read) |
//! | 15 | call bit: transmitter abnormal |
//! | 16 | A1: CET/CEST change at the end of this hour |
//! | 17, 18 | Z1, Z2: CEST, CET in force (exactly one set) |
//! | 19 | A2: leap second at the end of this hour |
//! | 20 | S: start of time code, always 1 |
//! | 21–27, 28 | minute (1, 2, 4, 8, 10, 20, 40), even parity |
//! | 29–34, 35 | hour (1, 2, 4, 8, 10, 20), even parity |
//! | 36–41 | day of month (1, 2, 4, 8, 10, 20) |
//! | 42–44 | day of week (1, 2, 4), Monday = 1 |
//! | 45–49 | month (1, 2, 4, 8, 10) |
//! | 50–57 | year of the century (1, 2, 4, 8, 10, 20, 40, 80) |
//! | 58 | even parity over 36–57 |
//! | 59 | no modulation: the minute marker |
//!
//! A leap second makes second 59 a 0 bit and moves the marker to second 60.
//! That minute does not decode here, which costs one minute every few years.
//!
//! # Seconds that could not be read
//!
//! A minute with a second or two unread is not given up on where its checks
//! can stand in for them — see [`fill_layout`], which ALS162 shares.

use sdroxide_types::{TimeFrame, TimeStation, utc_ymd_hms, ymd_hms_to_unix};

use super::{Context, Format, FrameError, Sym, Template, bits, low};
use crate::civil::{bcd, bcd_digits_ok, eu_summer, iso_weekday, parity, to_bcd, valid_date};

pub(super) static FORMAT: Format = Format {
    station: TimeStation::Dcf77,
    templates: &[
        Template { sym: Sym::ZERO, low: low(0, 1) },
        Template { sym: Sym::ONE, low: low(0, 2) },
        Template { sym: Sym::MARKER, low: low(0, 0) },
    ],
    rising_edge: false,
    frame_len: 60,
    time_index: 60,
    encode,
    decode,
    fill: Some(fill),
};

const MIN_W: [u32; 7] = [1, 2, 4, 8, 10, 20, 40];
const HOUR_W: [u32; 6] = [1, 2, 4, 8, 10, 20];
const DAY_W: [u32; 6] = [1, 2, 4, 8, 10, 20];
const DOW_W: [u32; 3] = [1, 2, 4];
const MON_W: [u32; 5] = [1, 2, 4, 8, 10];
const YEAR_W: [u32; 8] = [1, 2, 4, 8, 10, 20, 40, 80];

fn encode(minute_utc: i64) -> [Sym; 60] {
    let named = minute_utc + 60;
    let summer = eu_summer(named);
    let zone_s = if summer { 7200 } else { 3600 };
    let (y, mo, d, h, mi, _) = utc_ymd_hms(named + zone_s);
    let mut b = [false; 59];
    b[16] = eu_summer(named) != eu_summer(named + 3600);
    b[17] = summer;
    b[18] = !summer;
    b[20] = true;
    put(&mut b, 21, &to_bcd(mi, &MIN_W));
    b[28] = parity(b[21..28].iter().copied());
    put(&mut b, 29, &to_bcd(h, &HOUR_W));
    b[35] = parity(b[29..35].iter().copied());
    put(&mut b, 36, &to_bcd(d, &DAY_W));
    put(&mut b, 42, &to_bcd(iso_weekday(named + zone_s), &DOW_W));
    put(&mut b, 45, &to_bcd(mo, &MON_W));
    put(&mut b, 50, &to_bcd((y % 100) as u32, &YEAR_W));
    b[58] = parity(b[36..58].iter().copied());
    let mut out = [Sym::MARKER; 60];
    for (s, bit) in out.iter_mut().zip(b) {
        *s = Sym::bit(bit);
    }
    out
}

/// The most unread seconds a minute may have and still be filled.
const MAX_FILLS: usize = 2;
/// How clearly every other bit of a parity group must have been read for
/// the group's parity to stand in for one that was not, in the decoder's
/// margin — where a clean second reads at about 1 and a coin toss at 0.
const FILL_MIN_MARGIN: f32 = 0.3;
/// The even-parity groups: minute, hour, date.
const GROUPS: [std::ops::RangeInclusive<usize>; 3] = [21..=28, 29..=35, 36..=58];

fn fill(w: &[Option<Sym>], margin: &[f32]) -> Option<Vec<Sym>> {
    fill_layout(w, margin, false)
}

/// The unread seconds of a minute in DCF77's layout, worked out from the
/// rest of it.
///
/// - In a parity group (minute, hour, date): what makes the group's parity
///   even — when it is the group's only unread second, and every other bit of
///   the group was read clearly. One misread bit beside it would otherwise
///   pass as the right parity.
/// - Bit 0 is always 0 and bit 20 always 1; of 17 and 18 exactly one is set,
///   so either says what the other is.
/// - Every other bit is a flag or data that is 0 in all but a few minutes a
///   year: it is taken to be 0. A missed announcement is the cost.
/// - `count` (ALS162): bits 3–6 count the 1s in 21–58, and can be worked out
///   from them — but only when no group needed its parity, so that every
///   filled minute keeps at least one check it did not use up.
///
/// At most [`MAX_FILLS`] seconds are filled; the frame is then decoded and
/// checked as any other.
pub(super) fn fill_layout(w: &[Option<Sym>], margin: &[f32], count: bool) -> Option<Vec<Sym>> {
    if w.len() != 60 || margin.len() != 60 || !w[59].is_some_and(|s| s.marker) {
        return None;
    }
    let unread: Vec<usize> = (0..59).filter(|&i| w[i].is_none()).collect();
    if unread.len() > MAX_FILLS || w[..59].iter().flatten().any(|s| s.marker) {
        return None;
    }
    let mut b: Vec<Option<bool>> = w[..59].iter().map(|s| s.map(|s| s.a)).collect();
    let mut grouped = false;
    let mut counted = false;
    for &i in &unread {
        if let Some(g) = GROUPS.iter().find(|g| g.contains(&i)) {
            let others: Vec<usize> = g.clone().filter(|&j| j != i).collect();
            if others.iter().any(|&j| b[j].is_none() || margin[j] < FILL_MIN_MARGIN) {
                return None;
            }
            b[i] = Some(others.iter().filter(|&&j| b[j] == Some(true)).count() % 2 == 1);
            grouped = true;
        } else if i == 17 || i == 18 {
            // A misread partner would move the time by an hour.
            let other = if i == 17 { 18 } else { 17 };
            if margin[other] < FILL_MIN_MARGIN {
                return None;
            }
            b[i] = Some(!b[other]?);
        } else if count && (3..=6).contains(&i) {
            counted = true;
        } else {
            b[i] = Some(i == 20);
        }
    }
    if counted {
        if grouped {
            return None;
        }
        let ones = b[21..59].iter().filter(|&&x| x == Some(true)).count();
        for (k, i) in (3..=6).enumerate() {
            b[i] = Some(ones & (2 << k) != 0);
        }
    }
    let mut out: Vec<Sym> = b.into_iter().map(|x| x.map(Sym::bit)).collect::<Option<_>>()?;
    out.push(Sym::MARKER);
    Some(out)
}

fn put(b: &mut [bool], at: usize, field: &[bool]) {
    b[at..at + field.len()].copy_from_slice(field);
}

fn decode(w: &[Sym], _ctx: &Context) -> Result<TimeFrame, FrameError> {
    if w.len() != 60 || !w[59].marker || w[..59].iter().any(|s| s.marker) {
        return Err(FrameError::Framing);
    }
    let b = bits(w, 0..59);
    if b[0] || !b[20] {
        return Err(FrameError::Framing);
    }
    let field = |r: std::ops::Range<usize>| &b[r];
    if parity(b[21..29].iter().copied()) {
        return Err(FrameError::Parity("minute"));
    }
    if parity(b[29..36].iter().copied()) {
        return Err(FrameError::Parity("hour"));
    }
    if parity(b[36..59].iter().copied()) {
        return Err(FrameError::Parity("date"));
    }
    let (z1, z2) = (b[17], b[18]);
    if z1 == z2 {
        return Err(FrameError::Range("time zone"));
    }
    for (r, w) in [
        (21..28, &MIN_W[..]),
        (29..35, &HOUR_W[..]),
        (36..42, &DAY_W[..]),
        (45..50, &MON_W[..]),
        (50..58, &YEAR_W[..]),
    ] {
        if !bcd_digits_ok(field(r), w) {
            return Err(FrameError::Range("BCD digit"));
        }
    }
    let mi = bcd(field(21..28).iter().copied(), &MIN_W);
    let h = bcd(field(29..35).iter().copied(), &HOUR_W);
    let d = bcd(field(36..42).iter().copied(), &DAY_W);
    let dow = bcd(field(42..45).iter().copied(), &DOW_W);
    let mo = bcd(field(45..50).iter().copied(), &MON_W);
    let y = 2000 + i64::from(bcd(field(50..58).iter().copied(), &YEAR_W));
    if mi > 59 || h > 23 || !valid_date(y, mo, d) {
        return Err(FrameError::Range("time or date"));
    }
    let local = ymd_hms_to_unix(y, mo, d, h, mi, 0);
    if iso_weekday(local) != dow {
        return Err(FrameError::Range("day of week"));
    }
    let zone_min: i16 = if z1 { 120 } else { 60 };
    Ok(TimeFrame {
        utc: local - i64::from(zone_min) * 60,
        zone_min,
        dst: z1,
        dst_change_soon: b[16],
        leap_second_soon: b[19],
        dut1_ms: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bits for one known minute, checked against the table in the module
    /// docs position by position rather than through the encoder's own field
    /// helpers.
    #[test]
    fn the_minute_before_14_37_cest_on_18_october_2026() {
        // 12:37 UTC is 14:37 CEST; the frame naming it is sent 12:36–12:37.
        let named = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let s = encode(named - 60);
        let a: Vec<u8> = s.iter().map(|s| u8::from(s.a)).collect();
        assert!(s[59].marker);
        assert_eq!(a[17], 1, "CEST");
        assert_eq!(a[18], 0);
        assert_eq!(a[20], 1);
        // 37 = 1 + 2 + 4 + 10 + 20.
        assert_eq!(&a[21..28], &[1, 1, 1, 0, 1, 1, 0]);
        assert_eq!(a[28], 1, "five ones: parity 1");
        // 14 = 4 + 10.
        assert_eq!(&a[29..35], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(a[35], 0);
        // 18 = 8 + 10; Sunday = 7; October = 10; 26 = 2 + 4 + 20.
        assert_eq!(&a[36..42], &[0, 0, 0, 1, 1, 0]);
        assert_eq!(&a[42..45], &[1, 1, 1]);
        assert_eq!(&a[45..50], &[0, 0, 0, 0, 1]);
        assert_eq!(&a[50..58], &[0, 1, 1, 0, 0, 1, 0, 0]);
        // Ones in 36–57: 2 + 3 + 1 + 3 = 9, so the parity bit is 1.
        assert_eq!(a[58], 1);
        assert_eq!(decode(&s, &Context::default()).map(|f| f.utc), Ok(named));
    }

    #[test]
    fn both_zone_bits_or_neither_is_refused() {
        let named = ymd_hms_to_unix(2026, 1, 10, 9, 0, 0);
        let mut s = encode(named - 60);
        assert!(s[18].a && !s[17].a, "CET in January");
        s[17].a = true;
        assert_eq!(decode(&s, &Context::default()), Err(FrameError::Range("time zone")));
    }
}
