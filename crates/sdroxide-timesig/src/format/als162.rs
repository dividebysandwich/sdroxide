//! ALS162, 162 kHz, Allouis — the French legal time, kept by LNE-SYRTE and
//! transmitted by ANFR (TDF until 2017, when the France Inter programme it
//! once carried left long wave).
//!
//! Written from LNE-SYRTE's description of the signal ("Mise à disposition du
//! temps légal par le signal ALS162", and B. Dubois, "L'émetteur français de
//! fréquence étalon et de signaux de temps codé", Bulletin BNM 63–64, 1986),
//! with the bit table as tabulated on Wikipedia's ALS162 page.
//!
//! # The modulation
//!
//! The carrier's amplitude never changes; its phase does. Each second except
//! the fifty-ninth carries one *element*: the phase ramps up 1 rad over 25 ms,
//! down 2 rad over 50 ms and up 1 rad over 25 ms, back where it started. The
//! instant the middle ramp passes through zero is the start of the second, so
//! an element runs from 50 ms before the second to 50 ms after it. A 0 is one
//! element; a 1 is two, the second following straight on from the first. The
//! hundred milliseconds before each element are left unmodulated, and the
//! fifty-ninth second has no element at all — the minute marker.
//!
//! From 150 ms on, the rest of each second also carries a pseudo-random phase
//! modulation of the same ±1 rad, for time transfer by correlation. Its
//! sequence is not published and is not read here; it is the reason the
//! decoder looks at nothing past 150 ms.
//!
//! # The code
//!
//! What the 59 bits say is the *coming* minute, in French civil time (CET or
//! CEST), valid at the start of the second that follows the marker — DCF77's
//! layout from second 15 on.
//!
//! | second | meaning |
//! |---|---|
//! | 0 | start of minute, always 0 |
//! | 1 | positive leap second at the end of this hour |
//! | 2 | negative leap second at the end of this hour |
//! | 3–6 | how many of 21–58 are 1, weights 2, 4, 8, 16 (always even) |
//! | 7–12 | unused, 0 |
//! | 13 | tomorrow is a public holiday |
//! | 14 | today is a public holiday |
//! | 15 | reserved for abnormal operation; not read |
//! | 16 | CET/CEST change at the end of this hour |
//! | 17, 18 | CEST, CET in force (exactly one set) |
//! | 19 | unused, 0 |
//! | 20 | start of time code, always 1 |
//! | 21–27, 28 | minute (1, 2, 4, 8, 10, 20, 40), even parity |
//! | 29–34, 35 | hour (1, 2, 4, 8, 10, 20), even parity |
//! | 36–41 | day of month (1, 2, 4, 8, 10, 20) |
//! | 42–44 | day of week (1, 2, 4), Monday = 1 |
//! | 45–49 | month (1, 2, 4, 8, 10) |
//! | 50–57 | year of the century (1, 2, 4, 8, 10, 20, 40, 80) |
//! | 58 | even parity over 36–57 |
//! | 59 | no element: the minute marker |
//!
//! The count in 3–6 is a check DCF77 does not have: on top of the three
//! parity bits, a misread bit anywhere in 21–58 changes it.
//!
//! A leap second is an extra 0 between seconds 2 and 3, which moves
//! everything after it a second later; that minute does not decode here.
//!
//! A minute with a second or two unread is filled from its checks as
//! DCF77's is ([`super::dcf77::fill_layout`]); the count is one of them.

use sdroxide_types::{TimeFrame, TimeStation, utc_ymd_hms, ymd_hms_to_unix};

use super::{Context, Format, FrameError, Sym, Template, bits, low};
use crate::civil::{bcd, bcd_digits_ok, eu_summer, iso_weekday, parity, to_bcd, valid_date};

/// The carrier is never reduced: every template is full level throughout, and
/// the decoder reads ALS162's seconds by their phase elements instead (see
/// [`elements`]). The templates are here for what they name.
pub(super) static FORMAT: Format = Format {
    station: TimeStation::Als162,
    templates: &[
        Template { sym: Sym::ZERO, low: low(0, 0) },
        Template { sym: Sym::ONE, low: low(0, 0) },
        Template { sym: Sym::MARKER, low: low(0, 0) },
    ],
    rising_edge: false,
    frame_len: 60,
    time_index: 60,
    encode,
    decode,
    fill: Some(fill),
};

/// Each ramp of an element, seconds.
const RAMP_S: f64 = 0.025;
/// One element, seconds.
pub const ELEMENT_S: f64 = 4.0 * RAMP_S;

/// The patterns a second can carry, as how many elements it sends: the
/// marker none, a 0 one, a 1 two.
pub const SYMBOLS: [(Sym, usize); 3] = [(Sym::MARKER, 0), (Sym::ZERO, 1), (Sym::ONE, 2)];

/// How many elements a second carries.
pub fn elements(sym: Sym) -> usize {
    if sym.marker {
        0
    } else if sym.a {
        2
    } else {
        1
    }
}

/// One element's phase, radians, `t` seconds from the second it marks:
/// 0 → +1 → −1 → 0 over −50 … +50 ms, and nothing outside that.
pub fn element(t: f64) -> f64 {
    let x = t / RAMP_S;
    if !(-2.0..2.0).contains(&x) {
        0.0
    } else if x < -1.0 {
        x + 2.0
    } else if x < 1.0 {
        -x
    } else {
        x - 2.0
    }
}

const MIN_W: [u32; 7] = [1, 2, 4, 8, 10, 20, 40];
const HOUR_W: [u32; 6] = [1, 2, 4, 8, 10, 20];
const DAY_W: [u32; 6] = [1, 2, 4, 8, 10, 20];
const DOW_W: [u32; 3] = [1, 2, 4];
const MON_W: [u32; 5] = [1, 2, 4, 8, 10];
const YEAR_W: [u32; 8] = [1, 2, 4, 8, 10, 20, 40, 80];
/// The count of ones in 21–58, which is always even, without its lowest bit.
const COUNT_W: [u32; 4] = [2, 4, 8, 16];

fn fill(w: &[Option<Sym>], margin: &[f32]) -> Option<Vec<Sym>> {
    super::dcf77::fill_layout(w, margin, true)
}

fn ones(b: &[bool]) -> u32 {
    b[21..59].iter().filter(|&&x| x).count() as u32
}

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
    // Plain binary, not BCD: the weights are powers of two.
    let n = ones(&b);
    for (i, w) in COUNT_W.iter().enumerate() {
        b[3 + i] = n & w != 0;
    }
    let mut out = [Sym::MARKER; 60];
    for (s, bit) in out.iter_mut().zip(b) {
        *s = Sym::bit(bit);
    }
    out
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
    let count: u32 = COUNT_W.iter().enumerate().filter(|&(i, _)| b[3 + i]).map(|(_, w)| w).sum();
    if count != ones(&b) {
        return Err(FrameError::Parity("count of ones"));
    }
    // Unused, and 0 in every minute sent — so a 1 there is a misread.
    if b[7..13].iter().any(|&x| x) || b[19] {
        return Err(FrameError::Range("unused bit set"));
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
        leap_second_soon: b[1] || b[2],
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
        assert_eq!(&a[0..3], &[0, 0, 0]);
        assert_eq!(&a[7..17], &[0; 10]);
        assert_eq!(a[17], 1, "CEST");
        assert_eq!(a[18], 0);
        assert_eq!(a[19], 0);
        assert_eq!(a[20], 1);
        // 37 = 1 + 2 + 4 + 10 + 20, and five ones make its parity 1.
        assert_eq!(&a[21..29], &[1, 1, 1, 0, 1, 1, 0, 1]);
        // 14 = 4 + 10.
        assert_eq!(&a[29..36], &[0, 0, 1, 0, 1, 0, 0]);
        // 18 = 8 + 10; Sunday = 7; October = 10; 26 = 2 + 4 + 20; nine ones.
        assert_eq!(&a[36..42], &[0, 0, 0, 1, 1, 0]);
        assert_eq!(&a[42..45], &[1, 1, 1]);
        assert_eq!(&a[45..50], &[0, 0, 0, 0, 1]);
        assert_eq!(&a[50..58], &[0, 1, 1, 0, 0, 1, 0, 0]);
        assert_eq!(a[58], 1);
        // Ones in 21–58: 6 + 2 + 10 = 18 = 2 + 16.
        assert_eq!(&a[3..7], &[1, 0, 0, 1]);
        assert_eq!(decode(&s, &Context::default()).map(|f| f.utc), Ok(named));
    }

    #[test]
    fn a_count_that_disagrees_is_refused() {
        let named = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let mut s = encode(named - 60);
        s[4].a = !s[4].a;
        assert_eq!(decode(&s, &Context::default()), Err(FrameError::Parity("count of ones")));
        // Two flips in the minute's field keep its parity but not the count.
        let mut s = encode(named - 60);
        s[21].a = !s[21].a;
        s[22].a = !s[22].a;
        assert!(decode(&s, &Context::default()).is_err());
    }

    #[test]
    fn the_holiday_and_leap_bits_do_not_disturb_the_time() {
        let named = ymd_hms_to_unix(2026, 7, 14, 10, 0, 0);
        let mut s = encode(named - 60);
        s[14].a = true;
        s[1].a = true;
        let f = decode(&s, &Context::default()).expect("decodes");
        assert_eq!(f.utc, named);
        assert!(f.leap_second_soon);
    }

    #[test]
    fn the_element_is_the_published_shape() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(element(-0.05), 0.0));
        assert!(close(element(-0.025), 1.0));
        assert!(close(element(0.0), 0.0));
        assert!(close(element(0.025), -1.0));
        assert!(element(0.0499999).abs() < 1e-5);
        assert!(close(element(0.06), 0.0));
        assert!(close(element(-0.06), 0.0));
        // Falling through zero on the second at 40 rad/s.
        assert!(close(element(0.001), -0.04));
    }
}
