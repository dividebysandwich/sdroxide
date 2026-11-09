//! The seconds-only stations: BPM and RWM. Each marks every second and every
//! minute, and says nothing about which minute it is.
//!
//! So there is no time code to decode. A second is read only to tell the
//! minute's mark from an ordinary second; [`crate::Decoder`] reports the mark
//! as [`crate::Event::Minute`], and whoever holds the computer's clock names
//! it (see `TimeSigController`). The formats here exist for the templates the
//! reader matches against and for the encoder the signal generator uses.
//!
//! # BPM
//!
//! Pucheng, NTSC, on 2.5, 5, 10 and 15 MHz: an AM broadcast whose seconds
//! are 10 ms of 1000 Hz and whose minute is 300 ms of it. The reader listens
//! to that tone, so a second is "high" while the tone is on: an ordinary
//! second reads as nothing over its slots (its tick ends inside the first
//! slot's guard) and the minute as three slots high. UT1 seconds, sent in
//! part of every half hour, are 100 ms long and read as neither.
//!
//! # RWM
//!
//! Moscow, VNIIFTRI, on 4996, 9996 and 14996 kHz: the carrier itself is
//! keyed, on for 100 ms at each second and for 500 ms at the minute, off in
//! between. Its seconds are found by the carrier coming up, like JJY's. In
//! the parts of the hour where it sends ten pulses a second the second's own
//! pulse is the longer one, which the edge detector prefers two to one; the
//! minute's 500 ms still reads as the minute.

use sdroxide_types::{TimeFrame, TimeStation};

use super::{Context, Format, FrameError, Sym, Template, low};

pub(super) static FORMAT_BPM: Format = Format {
    station: TimeStation::Bpm,
    templates: &[
        Template { sym: Sym::ZERO, low: low(0, 10) },
        Template { sym: Sym::MARKER, low: low(3, 10) },
    ],
    rising_edge: true,
    frame_len: 1,
    time_index: 0,
    encode,
    decode,
    fill: None,
};

pub(super) static FORMAT_RWM: Format = Format {
    station: TimeStation::Rwm,
    templates: &[
        Template { sym: Sym::ZERO, low: low(1, 10) },
        Template { sym: Sym::MARKER, low: low(5, 10) },
    ],
    ..FORMAT_BPM
};

/// The minute mark at second 0, an ordinary second everywhere else.
fn encode(_minute_utc: i64) -> [Sym; 60] {
    let mut out = [Sym::ZERO; 60];
    out[0] = Sym::MARKER;
    out
}

/// Never a frame: these stations send no time of day.
fn decode(_w: &[Sym], _ctx: &Context) -> Result<TimeFrame, FrameError> {
    Err(FrameError::Framing)
}
