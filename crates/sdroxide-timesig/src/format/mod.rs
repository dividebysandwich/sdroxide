//! The time codes: what each second's keying means, and how a minute of them
//! becomes a UTC instant.
//!
//! Every station here says one thing per second by how long something is held
//! at full level — the long-wave carrier, or the short-wave stations' 100 Hz
//! subcarrier — and every length any of them uses ends on a whole tenth of a
//! second. So a second is described to the decoder as ten 100 ms slots, each
//! either at full level or reduced, and each station is a short list of the
//! patterns it sends — its [`Template`]s. What differs is only which patterns,
//! which edge starts the second, and how the minute's sixty symbols are laid
//! out; that is a [`Format`].
//!
//! WWVB's phase code is the exception: one bit a second carried by the sign
//! of the carrier rather than a pattern of levels. It is described in
//! [`wwvb_phase`], next to the amplitude code it rides alongside. ALS162 is
//! the other: its carrier is never reduced, and a second says its bit by how
//! many swings of phase it carries — see [`als162`].
//!
//! Each format also has an encoder. The receiver never transmits, but the
//! tests and the signal generator in [`crate::synth`] need the symbols a real
//! station would send for a given minute, and an encoder written from the
//! specification is the independent half that makes a round trip mean
//! something.

pub mod als162;
mod dcf77;
mod jjy;
mod msf;
mod wwv;
mod wwvb;
pub mod wwvb_phase;

use sdroxide_types::{TimeFrame, TimeStation};

/// What one second said.
///
/// `a` is the bit for the codes with one bit a second. MSF sends two, A and
/// B, and uses both fields. `marker` is the position or minute marker, which
/// carries no bit. `hole` is WWV's second 0, which carries nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sym {
    pub a: bool,
    pub b: bool,
    pub marker: bool,
    pub hole: bool,
}

impl Sym {
    pub const ZERO: Sym = Sym { a: false, b: false, marker: false, hole: false };
    pub const ONE: Sym = Sym { a: true, ..Sym::ZERO };
    pub const MARKER: Sym = Sym { marker: true, ..Sym::ZERO };
    pub const HOLE: Sym = Sym { hole: true, ..Sym::ZERO };

    pub const fn bit(a: bool) -> Sym {
        Sym { a, ..Sym::ZERO }
    }

    pub const fn ab(a: bool, b: bool) -> Sym {
        Sym { a, b, ..Sym::ZERO }
    }

    /// The value [`sdroxide_types::TimeSigStatus::symbols`] carries for it.
    pub fn code(self) -> u8 {
        if self.marker {
            sdroxide_types::TimeSigStatus::MARKER
        } else if self.hole {
            sdroxide_types::TimeSigStatus::HOLE
        } else {
            u8::from(self.a) + 2 * u8::from(self.b)
        }
    }
}

/// One pattern a station sends: which of a second's ten 100 ms slots have the
/// carrier reduced.
#[derive(Debug, Clone, Copy)]
pub struct Template {
    pub sym: Sym,
    pub low: [bool; 10],
}

/// The slots `from..to` reduced and the rest at full carrier.
const fn low(from: usize, to: usize) -> [bool; 10] {
    let mut s = [false; 10];
    let mut i = from;
    while i < to {
        s[i] = true;
        i += 1;
    }
    s
}

/// Why a minute that was read in full did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The markers are not where this code puts them: the window is not one
    /// minute, or a symbol was misread as a marker or the other way round.
    Framing,
    /// A field failed its parity bit.
    Parity(&'static str),
    /// A field decoded to something that is not a time or a date — a BCD digit
    /// above nine, a month thirteen, a day of the week that is not the one the
    /// date falls on.
    Range(&'static str),
    /// JJY's call-sign minutes (15 and 45) carry no year, and no recent frame
    /// was at hand to take it from.
    NeedsYear,
}

/// What a decoder may know beyond the minute it is decoding.
#[derive(Debug, Clone, Copy, Default)]
pub struct Context {
    /// The UTC time of the last frame that decoded, if recent.
    pub last_utc: Option<i64>,
}

/// One station's code: the patterns it sends and how a minute of them is laid
/// out.
pub struct Format {
    pub station: TimeStation,
    pub templates: &'static [Template],
    /// True where a second begins with the carrier coming *up* (JJY, which
    /// spends the end of every second reduced); false where it begins with the
    /// carrier going down (the other long-wave stations). The short-wave
    /// stations' seconds are found by their ticks instead, and this is unused.
    pub rising_edge: bool,
    /// How many symbols [`Format::decode`] reads, ending with the marker that
    /// triggers it.
    pub frame_len: usize,
    /// Which symbol of that window starts the instant the frame names —
    /// `frame_len` meaning the second after the window, which is where a code
    /// that announces the *coming* minute (DCF77) puts it.
    pub time_index: usize,
    /// The symbols sent in the minute that begins at `minute_utc` (a multiple
    /// of sixty), second 0 first.
    pub encode: fn(minute_utc: i64) -> [Sym; 60],
    /// A window of `frame_len` symbols ending at a marker, to the frame it
    /// carries.
    pub decode: fn(window: &[Sym], ctx: &Context) -> Result<TimeFrame, FrameError>,
    /// For the codes whose checks can stand in for a second that could not
    /// be read: a window of `frame_len` seconds, `None` where unread, and how
    /// clearly each read second was read (the decoder's margin), to the whole
    /// window — or `None` where what is missing cannot be worked out safely.
    /// [`Format::decode`] still has the last word on the result.
    pub fill: Option<Fill>,
}

/// See [`Format::fill`].
pub type Fill = fn(window: &[Option<Sym>], margin: &[f32]) -> Option<Vec<Sym>>;

impl Format {
    pub fn for_station(station: TimeStation) -> &'static Format {
        match station {
            TimeStation::Dcf77 => &dcf77::FORMAT,
            TimeStation::Msf => &msf::FORMAT,
            TimeStation::Wwvb => &wwvb::FORMAT,
            TimeStation::Jjy40 => &jjy::FORMAT_40,
            TimeStation::Jjy60 => &jjy::FORMAT_60,
            TimeStation::Wwv => &wwv::FORMAT_WWV,
            TimeStation::Wwvh => &wwv::FORMAT_WWVH,
            TimeStation::Als162 => &als162::FORMAT,
        }
    }

    /// The template a symbol is sent with.
    pub fn template(&self, sym: Sym) -> Option<&'static Template> {
        self.templates.iter().find(|t| t.sym == sym)
    }
}

/// Reads bit `a` of the symbols at `positions` out of a window indexed by
/// second.
fn bits(window: &[Sym], positions: impl IntoIterator<Item = usize>) -> Vec<bool> {
    positions.into_iter().map(|i| window[i].a).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::ymd_hms_to_unix;

    /// A spread of minutes across the century: both sides of every summer-time
    /// change, leap days, year ends, and the minutes JJY sends its call sign in.
    fn minutes() -> Vec<i64> {
        let mut v = vec![
            ymd_hms_to_unix(2026, 10, 18, 12, 37, 0),
            ymd_hms_to_unix(2026, 3, 29, 0, 59, 0),
            ymd_hms_to_unix(2026, 3, 29, 1, 0, 0),
            ymd_hms_to_unix(2026, 10, 25, 0, 59, 0),
            ymd_hms_to_unix(2026, 10, 25, 1, 0, 0),
            ymd_hms_to_unix(2024, 2, 29, 23, 59, 0),
            ymd_hms_to_unix(2024, 12, 31, 23, 59, 0),
            ymd_hms_to_unix(2025, 1, 1, 0, 0, 0),
            ymd_hms_to_unix(2099, 12, 31, 14, 59, 0),
            ymd_hms_to_unix(2000, 1, 1, 0, 0, 0),
        ];
        // A deterministic scatter of the rest.
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..400 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let span = ymd_hms_to_unix(2099, 6, 1, 0, 0, 0) - ymd_hms_to_unix(2000, 6, 1, 0, 0, 0);
            v.push(ymd_hms_to_unix(2000, 6, 1, 0, 0, 0) + (x % span as u64) as i64 / 60 * 60);
        }
        v
    }

    /// The window a decoder is handed when the trigger marker of the frame
    /// that names `named` has just been heard.
    pub(crate) fn window_naming(fmt: &Format, named: i64) -> Vec<Sym> {
        // Symbols from two minutes either side, so any window lands inside.
        let mut stream = Vec::new();
        let first = named - 120;
        for m in 0..4 {
            stream.extend_from_slice(&(fmt.encode)(first + m * 60));
        }
        // `stream[i]` is second `first + i`. The named instant is the start
        // of second `named`, which is window index `time_index`.
        let named_idx = (named - first) as usize;
        let start = named_idx - fmt.time_index;
        stream[start..start + fmt.frame_len].to_vec()
    }

    #[test]
    fn every_format_round_trips_through_its_encoder() {
        for station in TimeStation::ALL {
            let fmt = Format::for_station(station);
            for t in minutes() {
                let w = window_naming(fmt, t);
                assert!(w.last().unwrap().marker, "{station:?}: the window must end at a marker");
                let ctx = Context { last_utc: Some(t - 60) };
                match (fmt.decode)(&w, &ctx) {
                    Ok(f) => assert_eq!(f.utc, t, "{station:?} at {t}"),
                    Err(e) => panic!("{station:?} failed at {t}: {e:?}"),
                }
            }
        }
    }

    #[test]
    fn one_flipped_bit_never_decodes_to_a_different_time_where_there_is_parity() {
        // DCF77, ALS162, MSF and JJY carry parity over the time fields;
        // WWVB's amplitude code and WWV's carry none, and are protected only
        // by their unused bits and range checks.
        for station in
            [TimeStation::Dcf77, TimeStation::Als162, TimeStation::Msf, TimeStation::Jjy60]
        {
            let fmt = Format::for_station(station);
            for t in minutes().into_iter().take(60) {
                let w = window_naming(fmt, t);
                for i in 0..w.len() {
                    if w[i].marker || w[i].hole {
                        continue;
                    }
                    let mut bad = w.clone();
                    bad[i].a = !bad[i].a;
                    if let Ok(f) = (fmt.decode)(&bad, &Context { last_utc: Some(t - 60) }) {
                        // Bits outside the time fields (warnings, DUT1, the
                        // weather bits) may flip freely; the time may not.
                        assert_eq!(f.utc, t, "{station:?}: bit {i} flipped changed the time");
                    }
                }
            }
        }
    }

    /// A window with some seconds unread, every read one read cleanly.
    fn erased(fmt: &Format, t: i64, gone: &[usize]) -> (Vec<Option<Sym>>, Vec<f32>) {
        let w = window_naming(fmt, t);
        let read = w.iter().enumerate().map(|(i, s)| (!gone.contains(&i)).then_some(*s)).collect();
        let margin = (0..w.len()).map(|i| if gone.contains(&i) { 0.0 } else { 1.0 }).collect();
        (read, margin)
    }

    #[test]
    fn an_unread_second_is_filled_back_to_the_minute_sent() {
        for station in [TimeStation::Dcf77, TimeStation::Als162] {
            let fmt = Format::for_station(station);
            let fill = fmt.fill.expect("a code with parity fills");
            for t in minutes().into_iter().take(80) {
                // One in each parity group, one flag, one zone bit, one fixed
                // bit — and two at once, in different groups.
                for gone in [&[25][..], &[33], &[47], &[58], &[16], &[17], &[18], &[0], &[20]]
                    .into_iter()
                    .chain([&[22, 40][..], &[30, 2]])
                {
                    let (read, margin) = erased(fmt, t, gone);
                    let w = fill(&read, &margin).unwrap_or_else(|| panic!("{station:?} {gone:?}"));
                    let f = (fmt.decode)(&w, &Context::default());
                    assert_eq!(f.map(|f| f.utc), Ok(t), "{station:?} at {t}, {gone:?} unread");
                }
            }
        }
    }

    #[test]
    fn what_cannot_be_worked_out_safely_is_not_filled() {
        let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
        for station in [TimeStation::Dcf77, TimeStation::Als162] {
            let fmt = Format::for_station(station);
            let fill = fmt.fill.expect("a code with parity fills");
            // Two in one group, three in all, both zone bits.
            for gone in [&[22, 25][..], &[40, 50], &[1, 25, 40], &[17, 18]] {
                let (read, margin) = erased(fmt, t, gone);
                assert!(fill(&read, &margin).is_none(), "{station:?}: {gone:?} filled");
            }
            // A group whose other bits were not read clearly.
            let (read, mut margin) = erased(fmt, t, &[25]);
            margin[27] = 0.1;
            assert!(fill(&read, &margin).is_none(), "{station:?}: filled past a doubtful bit");
            // A zone bit whose partner was not read clearly.
            let (read, mut margin) = erased(fmt, t, &[17]);
            margin[18] = 0.1;
            assert!(fill(&read, &margin).is_none(), "{station:?}: zone from a doubtful partner");
        }
        // ALS162's count stands in only when no group's parity did.
        let fmt = Format::for_station(TimeStation::Als162);
        let fill = fmt.fill.expect("fills");
        let (read, margin) = erased(fmt, t, &[4]);
        let w = fill(&read, &margin).expect("the count alone is worked out");
        assert_eq!((fmt.decode)(&w, &Context::default()).map(|f| f.utc), Ok(t));
        let (read, margin) = erased(fmt, t, &[4, 25]);
        assert!(fill(&read, &margin).is_none(), "the count and a group both filled");
        // The other codes have nothing to fill from.
        for station in [TimeStation::Msf, TimeStation::Wwvb, TimeStation::Jjy60, TimeStation::Wwv] {
            assert!(Format::for_station(station).fill.is_none(), "{station:?}");
        }
    }

    #[test]
    fn a_window_one_second_out_of_place_is_refused() {
        for station in TimeStation::ALL {
            let fmt = Format::for_station(station);
            let t = ymd_hms_to_unix(2026, 10, 18, 12, 37, 0);
            let mut stream = Vec::new();
            for m in 0..3 {
                stream.extend_from_slice(&(fmt.encode)(t - 60 + m * 60));
            }
            let named_idx = 60;
            let start = named_idx - fmt.time_index;
            for shift in [1usize, 2, 10, 30] {
                let w = &stream[start + shift..start + shift + fmt.frame_len];
                let ctx = Context { last_utc: Some(t - 60) };
                if let Ok(f) = (fmt.decode)(w, &ctx) {
                    panic!("{station:?} shifted by {shift} decoded as {}", f.utc);
                }
            }
        }
    }
}
