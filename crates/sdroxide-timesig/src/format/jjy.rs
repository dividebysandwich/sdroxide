//! JJY, 40 and 60 kHz — National Institute of Information and Communications
//! Technology, Japan.
//!
//! Written from NICT's description of the JJY time code ("Standard Frequency
//! and Time Signal Emission", NICT). The keying is WWVB's turned over: every
//! second *begins* at full carrier, which drops to 10 % after 800 ms for a 0,
//! 500 ms for a 1 and 200 ms for a marker, and stays down until the next
//! second starts. So a second starts on a rising edge here, where the others
//! start on a falling one. Markers stand where WWVB's do: M at second 0, and
//! P1–P5 and P0 at 9, 19, 29, 39, 49 and 59.
//!
//! The code names the minute it is sent in, in Japan Standard Time (UTC+9).
//!
//! | second | meaning |
//! |---|---|
//! | 1–3, 5–8 | minute (40, 20, 10; 8, 4, 2, 1) |
//! | 12–13, 15–18 | hour (20, 10; 8, 4, 2, 1) |
//! | 22–23, 25–28, 30–33 | day of year (200, 100; 80, 40, 20, 10; 8, 4, 2, 1) |
//! | 36 | PA1: even parity over the hour bits, 12–18 |
//! | 37 | PA2: even parity over the minute bits, 1–8 |
//! | 41–48 | year of the century (80, 40, 20, 10, 8, 4, 2, 1) |
//! | 50–52 | day of week (4, 2, 1), Sunday = 0 |
//! | 53, 54 | LS1, LS2: leap second at the end of the month |
//! | 4, 10, 11, 14, 20, 21, 24, 34, 35, 38, 40, 55–58 | always 0 |
//!
//! At minutes 15 and 45 the station keys its call sign in Morse through
//! seconds 40–48 and sends service-interruption notices in 50–55, so those two
//! minutes carry no year and no day of the week. The year is taken from the
//! previous good frame instead, and only when the result lands within the
//! hour after it — a frame with nothing to anchor it is refused rather than
//! guessed.

use sdroxide_types::{TimeFrame, TimeStation, utc_ymd_hms};

use super::wwvb::{DOY_AT, DOY_W, HOUR_AT, HOUR_W, MARKERS, MIN_AT, MIN_W, get, put};
use super::{Context, Format, FrameError, Sym, Template, bits, low};
use crate::civil::{day_of_year, from_day_of_year, iso_weekday, parity};

const TEMPLATES: &[Template] = &[
    Template { sym: Sym::ZERO, low: low(8, 10) },
    Template { sym: Sym::ONE, low: low(5, 10) },
    Template { sym: Sym::MARKER, low: low(2, 10) },
];

pub(super) static FORMAT_40: Format = Format {
    station: TimeStation::Jjy40,
    templates: TEMPLATES,
    rising_edge: true,
    frame_len: 60,
    time_index: 0,
    encode,
    decode,
};

pub(super) static FORMAT_60: Format = Format { station: TimeStation::Jjy60, ..FORMAT_40 };

const JST_S: i64 = 9 * 3600;
const YEAR_AT: [usize; 8] = [41, 42, 43, 44, 45, 46, 47, 48];
const YEAR_W: [u32; 8] = [80, 40, 20, 10, 8, 4, 2, 1];
const DOW_AT: [usize; 3] = [50, 51, 52];
const DOW_W: [u32; 3] = [4, 2, 1];
/// Zero in every minute.
const UNUSED: [usize; 10] = [4, 10, 11, 14, 20, 21, 24, 34, 35, 38];
/// Zero in every minute but the call-sign ones.
const UNUSED_PLAIN: [usize; 5] = [40, 55, 56, 57, 58];

fn is_callsign_minute(minute: u32) -> bool {
    minute == 15 || minute == 45
}

fn encode(minute_utc: i64) -> [Sym; 60] {
    let local = minute_utc + JST_S;
    let (y, _, _, h, mi, _) = utc_ymd_hms(local);
    let mut b = [false; 60];
    put(&mut b, &MIN_AT, &MIN_W, mi);
    put(&mut b, &HOUR_AT, &HOUR_W, h);
    put(&mut b, &DOY_AT, &DOY_W, day_of_year(local));
    b[36] = parity(HOUR_AT.iter().map(|&i| b[i]));
    b[37] = parity(MIN_AT.iter().map(|&i| b[i]));
    let mut out = [Sym::ZERO; 60];
    if is_callsign_minute(mi) {
        // "JJY" in Morse through seconds 40–48 lands on the decoder as an
        // arbitrary run of bits; a fixed pattern stands in for it.
        for (i, s) in (40..=48).zip([1, 0, 1, 1, 0, 1, 1, 0, 1]) {
            b[i] = s == 1;
        }
    } else {
        put(&mut b, &YEAR_AT, &YEAR_W, (y % 100) as u32);
        put(&mut b, &DOW_AT, &DOW_W, iso_weekday(local) % 7);
    }
    for (s, bit) in out.iter_mut().zip(b) {
        *s = Sym::bit(bit);
    }
    for m in MARKERS {
        out[m] = Sym::MARKER;
    }
    out
}

fn decode(w: &[Sym], ctx: &Context) -> Result<TimeFrame, FrameError> {
    if w.len() != 60 {
        return Err(FrameError::Framing);
    }
    let mi_bits = bits(w, MIN_AT);
    let callsign = is_callsign_minute(crate::civil::bcd(mi_bits.iter().copied(), &MIN_W));
    // Morse keying in 40–48 can look like anything, markers included.
    let morse = |i: usize| callsign && (40..=48).contains(&i);
    if (0..60).any(|i| !morse(i) && w[i].marker != MARKERS.contains(&i)) {
        return Err(FrameError::Framing);
    }
    if UNUSED.iter().any(|&i| w[i].a) || (!callsign && UNUSED_PLAIN.iter().any(|&i| w[i].a)) {
        return Err(FrameError::Range("unused bit set"));
    }
    if parity(HOUR_AT.iter().map(|&i| w[i].a)) != w[36].a {
        return Err(FrameError::Parity("hour"));
    }
    if parity(MIN_AT.iter().map(|&i| w[i].a)) != w[37].a {
        return Err(FrameError::Parity("minute"));
    }
    let mi = get(w, &MIN_AT, &MIN_W)?;
    let h = get(w, &HOUR_AT, &HOUR_W)?;
    let doy = get(w, &DOY_AT, &DOY_W)?;
    if mi > 59 || h > 23 {
        return Err(FrameError::Range("time"));
    }
    let utc_in = |year: i64| -> Result<i64, FrameError> {
        let day = from_day_of_year(year, doy).ok_or(FrameError::Range("day of year"))?;
        Ok(day + i64::from(h) * 3600 + i64::from(mi) * 60 - JST_S)
    };
    let utc = if callsign {
        // The year of the last good frame, in Japan — tried as it is and as
        // the year after, for the minute that crosses New Year.
        let last = ctx.last_utc.ok_or(FrameError::NeedsYear)?;
        let (y, ..) = utc_ymd_hms(last + JST_S);
        [y, y + 1]
            .into_iter()
            .filter_map(|y| utc_in(y).ok())
            .find(|&t| t > last && t - last <= 3600)
            .ok_or(FrameError::NeedsYear)?
    } else {
        let y = 2000 + i64::from(get(w, &YEAR_AT, &YEAR_W)?);
        let utc = utc_in(y)?;
        if iso_weekday(utc + JST_S) % 7 != get(w, &DOW_AT, &DOW_W)? {
            return Err(FrameError::Range("day of week"));
        }
        utc
    };
    Ok(TimeFrame {
        utc,
        zone_min: 540,
        dst: false,
        dst_change_soon: false,
        leap_second_soon: !callsign && w[53].a,
        dut1_ms: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::ymd_hms_to_unix;

    #[test]
    fn twenty_one_thirty_seven_jst_on_18_october_2026() {
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        let s = encode(t);
        let a = |i: usize| u8::from(s[i].a);
        // 21 JST: 20 10 = 1 0; 8 4 2 1 = 0 0 0 1. Two ones: PA1 = 0.
        assert_eq!(HOUR_AT.map(a), [1, 0, 0, 0, 0, 1]);
        assert_eq!(a(36), 0);
        // 37 has five ones: PA2 = 1.
        assert_eq!(a(37), 1);
        // Sunday.
        assert_eq!(DOW_AT.map(a), [0, 0, 0]);
        assert_eq!(decode(&s, &Context::default()).map(|f| f.utc), Ok(t));
    }

    #[test]
    fn a_call_sign_minute_needs_a_recent_frame_for_its_year() {
        // 15 minutes past, JST.
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 15, 0);
        let s = encode(t);
        assert_eq!(decode(&s, &Context::default()), Err(FrameError::NeedsYear));
        assert_eq!(decode(&s, &Context { last_utc: Some(t - 60) }).map(|f| f.utc), Ok(t));
        // A frame from yesterday is no anchor for today.
        assert_eq!(decode(&s, &Context { last_utc: Some(t - 86_400) }), Err(FrameError::NeedsYear));
    }

    #[test]
    fn the_call_sign_minute_after_new_year_takes_the_new_year() {
        // 00:15 JST on 1 January 2027 is 15:15 UTC on 31 December 2026.
        let t = ymd_hms_to_unix(2026, 12, 31, 15, 15, 0);
        let s = encode(t);
        let last = ymd_hms_to_unix(2026, 12, 31, 14, 59, 0);
        assert_eq!(decode(&s, &Context { last_utc: Some(last) }).map(|f| f.utc), Ok(t));
    }
}
