//! MSF, 60 kHz, Anthorn — National Physical Laboratory.
//!
//! Written from NPL's "MSF 60 kHz time and date code" specification. Every
//! second begins with the carrier switched off for 100 ms. Two bits follow, A
//! in the next 100 ms and B in the one after — carrier off for a 1 — and the
//! carrier is on for the rest of the second. Second 0 is the minute marker:
//! carrier off for 500 ms.
//!
//! The code sent during a minute names the minute that *follows* it, in UK
//! civil time (GMT or BST), valid at the start of the next second 0.
//!
//! | second | A | B |
//! |---|---|---|
//! | 1–16 | 0 | DUT1: 1–8 positive, 9–16 negative, 100 ms each |
//! | 17–24 | year (80, 40, 20, 10, 8, 4, 2, 1) | 0 |
//! | 25–29 | month (10, 8, 4, 2, 1) | 0 |
//! | 30–35 | day of month (20, 10, 8, 4, 2, 1) | 0 |
//! | 36–38 | day of week (4, 2, 1), Sunday = 0 | 0 |
//! | 39–44 | hour (20, 10, 8, 4, 2, 1) | 0 |
//! | 45–51 | minute (40, 20, 10, 8, 4, 2, 1) | 0 |
//! | 52 | 0 | 0 |
//! | 53 | 1 | summer time changes within the hour |
//! | 54 | 1 | odd parity over 17A–24A |
//! | 55 | 1 | odd parity over 25A–35A |
//! | 56 | 1 | odd parity over 36A–38A |
//! | 57 | 1 | odd parity over 39A–51A |
//! | 58 | 1 | BST in force |
//! | 59 | 0 | 0 |
//!
//! The A bits of seconds 52–59 spell `01111110`, the minute identifier, which
//! is checked here as well as the parity: it is the one part of the code that
//! is the same every minute, and the cheapest proof that the window is where
//! it is supposed to be.

use sdroxide_types::{TimeFrame, TimeStation, utc_ymd_hms, ymd_hms_to_unix};

use super::{Context, Format, FrameError, Sym, Template, low};
use crate::civil::{bcd, bcd_digits_ok, eu_summer, iso_weekday, parity, to_bcd, valid_date};

pub(super) static FORMAT: Format = Format {
    station: TimeStation::Msf,
    templates: &[
        Template { sym: Sym::ab(false, false), low: low(0, 1) },
        Template { sym: Sym::ab(true, false), low: low(0, 2) },
        Template {
            sym: Sym::ab(false, true),
            low: [true, false, true, false, false, false, false, false, false, false],
        },
        Template { sym: Sym::ab(true, true), low: low(0, 3) },
        Template { sym: Sym::MARKER, low: low(0, 5) },
    ],
    rising_edge: false,
    frame_len: 60,
    time_index: 59,
    encode,
    decode,
    fill: None,
};

const YEAR_W: [u32; 8] = [80, 40, 20, 10, 8, 4, 2, 1];
const MON_W: [u32; 5] = [10, 8, 4, 2, 1];
const DAY_W: [u32; 6] = [20, 10, 8, 4, 2, 1];
const DOW_W: [u32; 3] = [4, 2, 1];
const HOUR_W: [u32; 6] = [20, 10, 8, 4, 2, 1];
const MIN_W: [u32; 7] = [40, 20, 10, 8, 4, 2, 1];

fn encode(minute_utc: i64) -> [Sym; 60] {
    let named = minute_utc + 60;
    let summer = eu_summer(named);
    let zone_s = if summer { 3600 } else { 0 };
    let local = named + zone_s;
    let (y, mo, d, h, mi, _) = utc_ymd_hms(local);
    // Indexed by second; [0] is the marker and carries no bits.
    let mut a = [false; 60];
    let mut b = [false; 60];
    put(&mut a, 17, &to_bcd((y % 100) as u32, &YEAR_W));
    put(&mut a, 25, &to_bcd(mo, &MON_W));
    put(&mut a, 30, &to_bcd(d, &DAY_W));
    put(&mut a, 36, &to_bcd(iso_weekday(local) % 7, &DOW_W));
    put(&mut a, 39, &to_bcd(h, &HOUR_W));
    put(&mut a, 45, &to_bcd(mi, &MIN_W));
    a[53..=58].fill(true);
    b[53] = eu_summer(named) != eu_summer(named + 3600);
    b[54] = !parity(a[17..=24].iter().copied());
    b[55] = !parity(a[25..=35].iter().copied());
    b[56] = !parity(a[36..=38].iter().copied());
    b[57] = !parity(a[39..=51].iter().copied());
    b[58] = summer;
    let mut out = [Sym::MARKER; 60];
    for s in 1..60 {
        out[s] = Sym::ab(a[s], b[s]);
    }
    out
}

fn put(bits: &mut [bool], at: usize, field: &[bool]) {
    bits[at..at + field.len()].copy_from_slice(field);
}

fn decode(w: &[Sym], _ctx: &Context) -> Result<TimeFrame, FrameError> {
    // Window index i is second i + 1; index 59 is the next minute's marker.
    if w.len() != 60 || !w[59].marker || w[..59].iter().any(|s| s.marker) {
        return Err(FrameError::Framing);
    }
    let a = |s: usize| w[s - 1].a;
    let b = |s: usize| w[s - 1].b;
    let identifier = [false, true, true, true, true, true, true, false];
    if (52..=59).map(a).ne(identifier) {
        return Err(FrameError::Framing);
    }
    let field = |r: std::ops::RangeInclusive<usize>| r.map(a).collect::<Vec<bool>>();
    // Odd parity: the field and its parity bit hold an odd number of ones.
    let odd = |r: std::ops::RangeInclusive<usize>, p: bool| parity(field(r)) != p;
    if !odd(17..=24, b(54)) {
        return Err(FrameError::Parity("year"));
    }
    if !odd(25..=35, b(55)) {
        return Err(FrameError::Parity("date"));
    }
    if !odd(36..=38, b(56)) {
        return Err(FrameError::Parity("day of week"));
    }
    if !odd(39..=51, b(57)) {
        return Err(FrameError::Parity("time"));
    }
    for (r, wts) in [
        (17..=24, &YEAR_W[..]),
        (25..=29, &MON_W[..]),
        (30..=35, &DAY_W[..]),
        (39..=44, &HOUR_W[..]),
        (45..=51, &MIN_W[..]),
    ] {
        if !bcd_digits_ok(&field(r), wts) {
            return Err(FrameError::Range("BCD digit"));
        }
    }
    let y = 2000 + i64::from(bcd(field(17..=24), &YEAR_W));
    let mo = bcd(field(25..=29), &MON_W);
    let d = bcd(field(30..=35), &DAY_W);
    let dow = bcd(field(36..=38), &DOW_W);
    let h = bcd(field(39..=44), &HOUR_W);
    let mi = bcd(field(45..=51), &MIN_W);
    if mi > 59 || h > 23 || !valid_date(y, mo, d) {
        return Err(FrameError::Range("time or date"));
    }
    let local = ymd_hms_to_unix(y, mo, d, h, mi, 0);
    if iso_weekday(local) % 7 != dow {
        return Err(FrameError::Range("day of week"));
    }
    let bst = b(58);
    let zone_min: i16 = if bst { 60 } else { 0 };
    let plus = (1..=8).filter(|&s| b(s)).count() as i16;
    let minus = (9..=16).filter(|&s| b(s)).count() as i16;
    Ok(TimeFrame {
        utc: local - i64::from(zone_min) * 60,
        zone_min,
        dst: bst,
        dst_change_soon: b(53),
        leap_second_soon: false,
        dut1_ms: Some((plus - minus) * 100),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_minute_before_13_37_bst_on_18_october_2026() {
        let named = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let s = encode(named - 60);
        let a = |sec: usize| u8::from(s[sec].a);
        let b = |sec: usize| u8::from(s[sec].b);
        assert!(s[0].marker);
        // 26 = 20 + 4 + 2, MSB first.
        assert_eq!((17..=24).map(a).collect::<Vec<_>>(), [0, 0, 1, 0, 0, 1, 1, 0]);
        // October = 10; the 18th = 10 + 8; Sunday = 0.
        assert_eq!((25..=29).map(a).collect::<Vec<_>>(), [1, 0, 0, 0, 0]);
        assert_eq!((30..=35).map(a).collect::<Vec<_>>(), [0, 1, 1, 0, 0, 0]);
        assert_eq!((36..=38).map(a).collect::<Vec<_>>(), [0, 0, 0]);
        // 13:37 BST.
        assert_eq!((39..=44).map(a).collect::<Vec<_>>(), [0, 1, 0, 0, 1, 1]);
        assert_eq!((45..=51).map(a).collect::<Vec<_>>(), [0, 1, 1, 0, 1, 1, 1]);
        assert_eq!((52..=59).map(a).collect::<Vec<_>>(), [0, 1, 1, 1, 1, 1, 1, 0]);
        assert_eq!(b(58), 1, "BST");
        // Year has three ones: already odd, so the parity bit is 0.
        assert_eq!(b(54), 0);
        // Day of week is all zeros: the parity bit makes it odd.
        assert_eq!(b(56), 1);
        // Shift to the decoder's window: seconds 1–59, then the next marker.
        let mut w: Vec<Sym> = s[1..].to_vec();
        w.push(Sym::MARKER);
        assert_eq!(decode(&w, &Context::default()).map(|f| f.utc), Ok(named));
    }
}
