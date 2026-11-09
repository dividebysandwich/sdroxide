//! Calendar arithmetic the time codes need and Unix time does not give for
//! free: day of the year, day of the week, and the European summer-time rule.
//!
//! Everything is in whole seconds of Unix time. The proleptic Gregorian
//! conversions are `sdroxide_types::utc_ymd_hms` and its inverse; what is here
//! is built on them.

use sdroxide_types::{utc_ymd_hms, ymd_hms_to_unix};

pub(crate) fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub(crate) fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Whether `day` is a real day of `month` in `year`.
pub(crate) fn valid_date(year: i64, month: u32, day: u32) -> bool {
    (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

/// The day of the week, ISO numbering: Monday 1 … Sunday 7.
pub(crate) fn iso_weekday(unix: i64) -> u32 {
    // 1970-01-01 was a Thursday.
    (unix.div_euclid(86_400) + 3).rem_euclid(7) as u32 + 1
}

/// Day of the year, 1 for 1 January.
pub(crate) fn day_of_year(unix: i64) -> u32 {
    let (y, ..) = utc_ymd_hms(unix);
    (unix.div_euclid(86_400) - ymd_hms_to_unix(y, 1, 1, 0, 0, 0).div_euclid(86_400)) as u32 + 1
}

/// Midnight starting day `doy` (1-based) of `year`, or `None` past the end of
/// that year.
pub(crate) fn from_day_of_year(year: i64, doy: u32) -> Option<i64> {
    let len = if is_leap(year) { 366 } else { 365 };
    (doy >= 1 && doy <= len)
        .then(|| ymd_hms_to_unix(year, 1, 1, 0, 0, 0) + i64::from(doy - 1) * 86_400)
}

/// The last Sunday of `month` in `year`, at 01:00 UTC — the instant the
/// European Union (and the United Kingdom, on the same rule) changes between
/// winter and summer time. Directive 2000/84/EC, articles 2 and 3.
fn last_sunday_0100_utc(year: i64, month: u32) -> i64 {
    let last = ymd_hms_to_unix(year, month, days_in_month(year, month), 1, 0, 0);
    // ISO Sunday is 7; step back to it.
    last - i64::from(iso_weekday(last) % 7) * 86_400
}

/// Whether European summer time (CEST, BST) is in force at `utc`.
pub(crate) fn eu_summer(utc: i64) -> bool {
    let (y, ..) = utc_ymd_hms(utc);
    utc >= last_sunday_0100_utc(y, 3) && utc < last_sunday_0100_utc(y, 10)
}

/// Binary-coded decimal: the sum of `weights` over the bits that are set.
pub(crate) fn bcd(bits: impl IntoIterator<Item = bool>, weights: &[u32]) -> u32 {
    bits.into_iter().zip(weights).filter(|(b, _)| *b).map(|(_, w)| w).sum()
}

/// The bits that encode `value` against `weights`, greedily from the largest
/// weight — which is what BCD is when the weights are 80, 40, 20, 10, 8, 4, 2,
/// 1 in any order.
pub(crate) fn to_bcd(value: u32, weights: &[u32]) -> Vec<bool> {
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(weights[i]));
    let mut left = value;
    let mut bits = vec![false; weights.len()];
    // Tens and units separately, so 19 is 10 + 9 and never 10 + 8 + 1 by
    // accident of ordering: the units digit's weights are all below ten.
    let (mut tens, mut units) = (left / 10 * 10, left % 10);
    for i in order {
        let w = weights[i];
        let pool = if w >= 10 { &mut tens } else { &mut units };
        if *pool >= w {
            *pool -= w;
            bits[i] = true;
        }
    }
    left = tens + units;
    debug_assert_eq!(left, 0, "{value} does not fit the weights {weights:?}");
    bits
}

/// Whether a BCD field's units digit (the weights below ten) is a digit.
pub(crate) fn bcd_digits_ok(bits: &[bool], weights: &[u32]) -> bool {
    let units = bcd(
        bits.iter().copied(),
        &weights.iter().map(|&w| if w < 10 { w } else { 0 }).collect::<Vec<_>>(),
    );
    let tens = bcd(
        bits.iter().copied(),
        &weights.iter().map(|&w| if w >= 10 { w / 10 } else { 0 }).collect::<Vec<_>>(),
    );
    units <= 9 && tens <= 9
}

/// Even parity: true when an odd number of `bits` are set, which is the value
/// a parity bit takes to make the count even.
pub(crate) fn parity(bits: impl IntoIterator<Item = bool>) -> bool {
    bits.into_iter().filter(|b| *b).count() % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays_and_ordinal_days() {
        // 2026-10-18 is a Sunday, the 291st day of the year.
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 0, 0);
        assert_eq!(iso_weekday(t), 7);
        assert_eq!(day_of_year(t), 291);
        assert_eq!(from_day_of_year(2026, 291), Some(ymd_hms_to_unix(2026, 10, 18, 0, 0, 0)));
        assert_eq!(day_of_year(ymd_hms_to_unix(2024, 12, 31, 0, 0, 0)), 366);
        assert_eq!(from_day_of_year(2025, 366), None);
    }

    #[test]
    fn summer_time_changes_at_one_utc_on_the_last_sundays() {
        // 2026: 29 March and 25 October.
        let on = ymd_hms_to_unix(2026, 3, 29, 1, 0, 0);
        let off = ymd_hms_to_unix(2026, 10, 25, 1, 0, 0);
        assert!(!eu_summer(on - 1));
        assert!(eu_summer(on));
        assert!(eu_summer(off - 1));
        assert!(!eu_summer(off));
    }

    #[test]
    fn bcd_round_trips_whatever_the_weight_order() {
        let msb_first = [80, 40, 20, 10, 8, 4, 2, 1];
        let lsb_first = [1, 2, 4, 8, 10, 20, 40, 80];
        for v in 0..100 {
            assert_eq!(bcd(to_bcd(v, &msb_first), &msb_first), v);
            assert_eq!(bcd(to_bcd(v, &lsb_first), &lsb_first), v);
            assert!(bcd_digits_ok(&to_bcd(v, &lsb_first), &lsb_first));
        }
        // 1+2+8 = 11 in the units digit is not a digit.
        assert!(!bcd_digits_ok(&[true, true, false, true, false, false, false, false], &lsb_first));
    }
}
