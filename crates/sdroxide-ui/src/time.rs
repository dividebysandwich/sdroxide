//! Wall clock, on both targets.
//!
//! `SystemTime::now()` panics on `wasm32-unknown-unknown` — there is no clock
//! behind it — so the browser build asks JavaScript instead. Every part of the
//! UI that needs the date rather than a frame time goes through here: the
//! logbook, the waterfall's time gridlines, and the solar view, whose whole
//! scene is a function of an explicit timestamp.

/// Current Unix time (UTC seconds).
/// The current UTC time as a listener reads it, `HH:MM:SS`.
pub fn utc_clock(unix: i64) -> String {
    let (_, _, _, h, mi, s) = sdroxide_types::utc_ymd_hms(unix);
    format!("{h:02}:{mi:02}:{s:02}")
}

pub fn now_unix() -> i64 {
    now_unix_f64() as i64
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
///
/// Both halves ask the platform rather than deriving anything: the offset is a
/// political fact about this instant — which side of a DST boundary it falls on
/// — and not something a Unix timestamp carries.
pub fn local_offset_seconds() -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // chrono re-reads the zone at most once a second and caches it, so this
        // is cheap enough to call once a frame.
        use chrono::{Offset, TimeZone};
        chrono::Local
            .timestamp_opt(now_unix(), 0)
            .single()
            .map_or(0, |t| i64::from(t.offset().fix().local_minus_utc()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        // getTimezoneOffset counts minutes *behind* local time, so UTC+2 — two
        // hours east — reports -120.
        let minutes = js_sys::Date::new_0().get_timezone_offset();
        (-minutes * 60.0) as i64
    }
}
