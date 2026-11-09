//! BPC, 68.5 kHz, Shangqiu — National Time Service Center (NTSC), Chinese
//! Academy of Sciences.
//!
//! **Not from a published specification.** NTSC does not publish BPC's
//! format; what follows is the layout as described publicly (the Wikipedia
//! article on BPC and the receiver-chip literature it draws on), and has not
//! been checked against the station on the air from here. Every check the
//! layout allows is made — both parity bits, the frame number, every field's
//! range, and the day of the week against the date — so a layout that is
//! wrong decodes nothing rather than something wrong; and the receiver still
//! needs consecutive frames to agree before it believes any of them.
//!
//! Each second begins with the carrier reduced, for 100, 200, 300 or 400 ms:
//! two bits a second, the value 0 to 3 (the reduction in tenths, less one).
//! A second with no reduction marks the start of a frame. A frame is twenty
//! seconds, so three a minute, each naming the time in Beijing (UTC+8, no
//! summer time) at its own first second.
//!
//! | second of frame | value |
//! |---|---|
//! | 0 | no reduction: the frame marker |
//! | 1 | P1, the frame's place in the minute: 0, 1, 2 for seconds 0, 20, 40 |
//! | 2 | P2, reserved |
//! | 3–4 | hour on the 12-hour clock, 0–11 (four bits) |
//! | 5–7 | minute (six bits) |
//! | 8–9 | day of the week, Monday 1 to Sunday 7 (four bits) |
//! | 10 | P3: high bit afternoon (PM), low bit even parity over seconds 1–9 and the PM bit |
//! | 11–13 | day of the month (six bits) |
//! | 14–15 | month (four bits) |
//! | 16–18 | year of the century, low six bits |
//! | 19 | P4: high bit the year's seventh bit, low bit even parity over seconds 11–18 and that bit |
//!
//! Each field is binary, most significant bit first, two bits to a second.

use sdroxide_types::{TimeFrame, TimeStation, utc_ymd_hms, ymd_hms_to_unix};

use super::{Context, Format, FrameError, Sym, Template, low};
use crate::civil::{iso_weekday, valid_date};

pub(super) static FORMAT: Format = Format {
    station: TimeStation::Bpc,
    templates: &[
        Template { sym: Sym::ab(false, false), low: low(0, 1) },
        Template { sym: Sym::ab(true, false), low: low(0, 2) },
        Template { sym: Sym::ab(false, true), low: low(0, 3) },
        Template { sym: Sym::ab(true, true), low: low(0, 4) },
        Template { sym: Sym::MARKER, low: low(0, 0) },
    ],
    rising_edge: false,
    // The marker that starts the frame, its nineteen seconds, and the next
    // frame's marker, which is what tells the decoder the frame has ended.
    frame_len: 21,
    time_index: 0,
    encode,
    decode,
    fill: None,
};

/// Beijing time, minutes east of UTC.
const ZONE_MIN: i16 = 480;

/// The value 0–3 a second carries.
fn value(s: Sym) -> u32 {
    u32::from(s.a) + 2 * u32::from(s.b)
}

fn sym(v: u32) -> Sym {
    Sym::ab(v & 1 != 0, v & 2 != 0)
}

/// `n` values, most significant first, to one number.
fn field(w: &[Sym], from: usize, n: usize) -> u32 {
    w[from..from + n].iter().fold(0, |acc, &s| acc << 2 | value(s))
}

/// The ones among the bits of the values `w[range]`.
fn ones(w: &[Sym], range: std::ops::Range<usize>) -> u32 {
    w[range].iter().map(|&s| value(s).count_ones()).sum()
}

/// The twenty seconds of the frame naming local time `local` (Unix seconds as
/// if Beijing were UTC), which must fall on 0, 20 or 40 seconds.
fn frame(local: i64) -> [Sym; 20] {
    let (y, mo, d, h, mi, s) = utc_ymd_hms(local);
    let mut v = [0u32; 20];
    v[1] = s / 20;
    v[2] = 0;
    let (pm, h12) = (h >= 12, h % 12);
    v[3] = h12 >> 2;
    v[4] = h12 & 3;
    v[5] = mi >> 4;
    v[6] = (mi >> 2) & 3;
    v[7] = mi & 3;
    let dow = iso_weekday(local);
    v[8] = dow >> 2;
    v[9] = dow & 3;
    let pre: u32 = v[1..10].iter().map(|x| x.count_ones()).sum::<u32>() + u32::from(pm);
    v[10] = u32::from(pm) << 1 | (pre & 1);
    v[11] = d >> 4;
    v[12] = (d >> 2) & 3;
    v[13] = d & 3;
    v[14] = mo >> 2;
    v[15] = mo & 3;
    let yy = (y.rem_euclid(100)) as u32;
    v[16] = (yy >> 4) & 3;
    v[17] = (yy >> 2) & 3;
    v[18] = yy & 3;
    let high = yy >> 6;
    let date: u32 = v[11..19].iter().map(|x| x.count_ones()).sum::<u32>() + high;
    v[19] = high << 1 | (date & 1);
    let mut out = [Sym::MARKER; 20];
    for (o, &x) in out.iter_mut().zip(&v).skip(1) {
        *o = sym(x);
    }
    out
}

fn encode(minute_utc: i64) -> [Sym; 60] {
    let local = minute_utc + i64::from(ZONE_MIN) * 60;
    let mut out = [Sym::MARKER; 60];
    for f in 0..3 {
        out[20 * f..20 * f + 20].copy_from_slice(&frame(local + 20 * f as i64));
    }
    out
}

fn decode(w: &[Sym], _ctx: &Context) -> Result<TimeFrame, FrameError> {
    if w.len() != 21 || !w[0].marker || !w[20].marker || w[1..20].iter().any(|s| s.marker) {
        return Err(FrameError::Framing);
    }
    let p1 = value(w[1]);
    if p1 > 2 {
        return Err(FrameError::Framing);
    }
    let p3 = value(w[10]);
    let pm = p3 >> 1;
    if !(ones(w, 1..10) + pm + (p3 & 1)).is_multiple_of(2) {
        return Err(FrameError::Parity("time"));
    }
    let p4 = value(w[19]);
    let high = p4 >> 1;
    if !(ones(w, 11..19) + high + (p4 & 1)).is_multiple_of(2) {
        return Err(FrameError::Parity("date"));
    }
    let h12 = field(w, 3, 2);
    let mi = field(w, 5, 3);
    let dow = field(w, 8, 2);
    let d = field(w, 11, 3);
    let mo = field(w, 14, 2);
    let y = 2000 + i64::from(high << 6 | field(w, 16, 3));
    if h12 > 11 || mi > 59 || !valid_date(y, mo, d) {
        return Err(FrameError::Range("time or date"));
    }
    let local = ymd_hms_to_unix(y, mo, d, h12 + 12 * pm, mi, 20 * p1);
    if iso_weekday(local) != dow {
        return Err(FrameError::Range("day of week"));
    }
    Ok(TimeFrame {
        utc: local - i64::from(ZONE_MIN) * 60,
        zone_min: ZONE_MIN,
        dst: false,
        dst_change_soon: false,
        leap_second_soon: false,
        dut1_ms: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One known frame, checked value by value against the table in the
    /// module docs rather than through the encoder's arithmetic.
    #[test]
    fn the_frame_at_20_37_20_beijing_time_on_18_october_2026() {
        // 12:37:20 UTC is 20:37:20 in Beijing: the second frame of its minute.
        let utc = ymd_hms_to_unix(2026, 10, 18, 12, 37, 20);
        let s = encode(utc - 20);
        let v: Vec<u32> = s[20..40].iter().map(|&x| value(x)).collect();
        assert!(s[20].marker && s[0].marker && s[40].marker);
        assert_eq!(v[1], 1, "the second frame of the minute");
        // 20:37 is 8 PM: 8 = 0b10_00; PM set.
        assert_eq!(&v[3..5], &[2, 0]);
        // 37 = 0b10_01_01.
        assert_eq!(&v[5..8], &[2, 1, 1]);
        // Sunday = 7 = 0b01_11.
        assert_eq!(&v[8..10], &[1, 3]);
        // Ones in 1–9: 1 + 1 + 1 + 1 + 1 + 1 + 2 = 8, and PM: 9 — odd, so
        // the parity bit is 1 to make it even.
        assert_eq!(v[10], 0b11);
        // 18 = 0b01_00_10; October = 10 = 0b10_10; 26 = 0b01_10_10.
        assert_eq!(&v[11..14], &[1, 0, 2]);
        assert_eq!(&v[14..16], &[2, 2]);
        assert_eq!(&v[16..19], &[1, 2, 2]);
        // Ones in 11–18: 1 + 1 + 1 + 1 + 1 + 1 + 1 = 7; the year's seventh
        // bit is 0, so the parity bit is 1.
        assert_eq!(v[19], 0b01);
        let window: Vec<Sym> = s[20..41].to_vec();
        assert_eq!(decode(&window, &Context::default()).map(|f| f.utc), Ok(utc));
    }

    #[test]
    fn a_frame_number_past_two_is_not_a_frame() {
        let utc = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let mut w: Vec<Sym> = encode(utc)[..20].to_vec();
        w.push(Sym::MARKER);
        w[1] = sym(3);
        assert_eq!(decode(&w, &Context::default()), Err(FrameError::Framing));
    }
}
