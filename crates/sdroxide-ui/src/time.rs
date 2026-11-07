//! Wall clock, on both targets.
//!
//! `SystemTime::now()` panics on `wasm32-unknown-unknown` — there is no clock
//! behind it — so the browser build asks JavaScript instead. Every part of the
//! UI that needs the date rather than a frame time goes through here: the
//! logbook, the waterfall's time gridlines, and the solar view, whose whole
//! scene is a function of an explicit timestamp.

use crate::i18n::tr;

/// Current Unix time (UTC seconds).
pub fn now_unix() -> i64 {
    now_unix_f64() as i64
}

/// The current UTC time as a listener reads it, `HH:MM:SS`.
pub fn utc_clock(unix: i64) -> String {
    let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(unix);
    format!("{h:02}:{mi:02}:{s:02}")
}

/// `13:08:42 UTC · 14:08 local` — the wall clock in both zones, or just the UTC
/// half when the operator is already on UTC.
///
/// Split from the reading of the platform offset so the formatting is a pure
/// function of `(unix, offset)` and can be tested without a timezone.
pub fn utc_and_local(unix: i64, offset_secs: i64) -> String {
    let utc = utc_clock(unix);
    if offset_secs == 0 {
        format!("{utc} UTC")
    } else {
        let (_, _, _, h, mi, _) = sdroxide_types::utc_ymd_hms(unix + offset_secs);
        tr!("time-utc-and-local", utc = utc, local = format!("{h:02}:{mi:02}"))
    }
}

/// [`utc_and_local`] for `unix`, with the operator's offset read from the
/// platform.
pub fn utc_and_local_now(unix: i64) -> String {
    utc_and_local(unix, local_offset_at(unix))
}

/// Current Unix time as fractional UTC seconds.
pub fn now_unix_f64() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
}

/// The operator's current offset from UTC, in seconds east of Greenwich.
pub fn local_offset_seconds() -> i64 {
    local_offset_at(now_unix())
}

/// The operator's offset from UTC at `unix`, in seconds east of Greenwich.
///
/// Both halves ask the platform rather than deriving anything: the offset is a
/// political fact about that instant — which side of a DST boundary it falls
/// on — and not something a Unix timestamp carries. Asked per instant, so a
/// satellite pass after tonight's clock change reads in tomorrow's zone.
pub fn local_offset_at(unix: i64) -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // chrono re-reads the zone at most once a second and caches it, so this
        // is cheap enough to call once a frame.
        use chrono::{Offset, TimeZone};
        chrono::Local
            .timestamp_opt(unix, 0)
            .single()
            .map_or(0, |t| i64::from(t.offset().fix().local_minus_utc()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        // getTimezoneOffset counts minutes *behind* local time, so UTC+2 — two
        // hours east — reports -120.
        let at = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(unix as f64 * 1000.0));
        let minutes = at.get_timezone_offset();
        (-minutes * 60.0) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clock carries UTC always and the local half only when the operator is
    /// off UTC.
    #[test]
    fn the_clock_shows_utc_and_local() {
        let unix = 1_700_000_000;
        let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(unix);
        let utc = format!("{h:02}:{mi:02}:{s:02}");
        assert_eq!(utc_and_local(unix, 0), format!("{utc} UTC"));
        let (_, _, _, lh, lmi, _) = sdroxide_types::utc_ymd_hms(unix + 7200);
        assert_eq!(utc_and_local(unix, 7200), format!("{utc} UTC · {lh:02}:{lmi:02} local"));
    }
}
