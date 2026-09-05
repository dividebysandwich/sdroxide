//! What a data field means: the message layouts of ITU-R M.1371-5.
//!
//! # Why this is a struct of options rather than an enum of messages
//!
//! Twenty-seven message types share about six pieces of information between
//! them, and no two carry quite the same set. A position arrives in types 1, 2,
//! 3, 9, 18, 19, 21 and 27; a name in 5, 19, 21 and 24; dimensions in 5, 19, 21
//! and 24. Message 19 carries a position *and* a name *and* dimensions in one
//! go, and message 24 comes in two halves several seconds apart.
//!
//! An enum with a variant per type would make [`crate::track`] a
//! twenty-seven-arm match in which the same four assignments appear over and
//! over. So the layouts are read *here*, once per type, into the small set of
//! things a vessel actually has, and the tracker folds whatever turned up. What
//! the type was is kept in [`Message::kind`], because an operator reading a row
//! wants to know which message told them.
//!
//! # Coordinates
//!
//! Longitude and latitude are signed, in ten-thousandths of a minute — so a
//! division by 600 000 — and each has a reserved "not available" value which is
//! a position off the earth: 181° and 91°. Message 27 states them in tenths of
//! a minute instead, because it is meant for a satellite and every bit counts.
//! Both are range-checked afterwards rather than compared against the reserved
//! values alone: a transmitter with a broken receiver sends whatever its GNSS
//! set last handed it, and a ship drawn at 200° east is worse than one not
//! drawn.
//!
//! # Rate of turn is not degrees a minute on the wire
//!
//! Message 1's turn field is `4.733·√(rate)` signed, rounded — a square-root
//! companding that gives fine resolution near zero and coarse resolution at a
//! hard turn. Squaring it back is the only way to get a number anybody can
//! read, and the two saturating values (±127) mean "turning faster than 5° in
//! 30 seconds" rather than 720°/min, so they are reported as the limit rather
//! than as the square.
//!
//! Source: ITU-R M.1371-5 Annex 8 (the message tables), Table 45
//! (navigational status), Table 50 (ship type) and Table 51 (aid type).

use sdroxide_types::AisKind;

use crate::sixbit::{read, read_signed, text};

/// Where a station said it was, and what it was doing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fix {
    /// Degrees. `None` where the transmitter said "not available", or where
    /// what it said was not on the earth.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// A differentially corrected fix, better than 10 m.
    pub accuracy: bool,
    /// The transmitter's own integrity monitoring was in use.
    pub raim: bool,
    /// Speed over ground, knots.
    pub sog_kt: Option<f32>,
    /// Course over ground, degrees true.
    pub cog_deg: Option<f32>,
    /// True heading, degrees.
    pub heading_deg: Option<f32>,
    /// Rate of turn, degrees a minute, negative to port.
    pub turn_deg_min: Option<f32>,
    /// Navigational status, 0–14 — see [`sdroxide_types::nav_status_label`].
    pub nav_status: Option<u8>,
    /// Altitude in metres, for a search-and-rescue aircraft.
    pub altitude_m: Option<i32>,
}

/// What a station said it is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Statics {
    pub name: Option<String>,
    pub call_sign: Option<String>,
    pub imo: Option<u32>,
    pub ship_type: Option<u8>,
    /// Distance from the reported position to bow, stern, port and starboard,
    /// metres.
    pub dim: Option<(u16, u16, u16, u16)>,
}

/// What a station said it is doing this voyage.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Voyage {
    pub destination: Option<String>,
    pub draught_m: Option<f32>,
    /// `MM-DD HH:MM` in UTC, as stated. Empty fields in the message mean "not
    /// available" and produce `None` rather than a date of `00-00`.
    pub eta: Option<String>,
}

/// An aid to navigation — a buoy, a beacon, a lighthouse, or a mark that exists
/// only as this transmission.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Aid {
    pub aid_type: u8,
    pub name: String,
    /// Nothing is physically there; the mark is broadcast by a shore station.
    pub virtual_aid: bool,
    /// The aid says it has drifted off its charted position — which is the one
    /// thing about a buoy anybody needs to be told.
    pub off_position: bool,
    pub dim: Option<(u16, u16, u16, u16)>,
}

/// One decoded message, as whatever pieces of the picture it carried.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// The message type, 1–27.
    pub kind: u8,
    /// Repeat indicator: how many times this has been relayed. 3 means "do not
    /// repeat again".
    pub repeat: u8,
    pub mmsi: u32,
    /// What sort of station the *message type* implies, where it implies
    /// anything. The MMSI's own format outranks it — see
    /// [`AisKind::from_mmsi`].
    pub class: Option<AisKind>,
    pub fix: Option<Fix>,
    pub statics: Option<Statics>,
    pub voyage: Option<Voyage>,
    pub aid: Option<Aid>,
    /// UTC as a base station stated it, `HH:MM:SS`.
    pub utc: Option<String>,
    /// The layout was one this decoder reads. False for the types it does not
    /// — binary broadcasts, safety text, interrogations, assignment commands —
    /// which still produce a row, because "this MMSI is out there" is worth
    /// knowing and the alternative is a decoder that looks deaf.
    pub known: bool,
}

/// "Not available" for a longitude in ten-thousandths of a minute: 181°.
const LON_NA: i64 = 181 * 600_000;
/// ...and for a latitude: 91°.
const LAT_NA: i64 = 91 * 600_000;

/// Read a message from a data field, most significant bit first.
///
/// `None` only when there is not even a header — every field beyond that is
/// optional, because a message truncated by a fade in its last slot still says
/// who sent it.
pub fn parse(bits: &[bool]) -> Option<Message> {
    if bits.len() < 38 {
        return None;
    }
    let kind = read(bits, 0, 6) as u8;
    // A message shorter than its own type defines is not that message. The
    // fields past the end would be read as zeros — see [`read`] — and zeros are
    // a perfectly plausible navigational status, speed and position, so a frame
    // like that does not decode into an incomplete vessel but into a confident
    // and entirely invented one.
    //
    // Nothing is lost by being strict: these frames are checked by a 16-bit
    // FCS before they arrive here, and a transmission cut short by a fade fails
    // that check rather than reaching this function. What does reach it is the
    // occasional burst of interference whose CRC happens to come out right, and
    // this is what stops one of those from becoming a ship.
    if bits.len() < payload_bits(kind)? {
        return None;
    }
    let repeat = read(bits, 6, 2) as u8;
    let mmsi = read(bits, 8, 30) as u32;
    // An MMSI is nine decimal digits. The field is thirty bits wide and so can
    // hold half as much again, and a number that cannot be an identity is not
    // one — it is the same interference, one field further on.
    if mmsi > 999_999_999 {
        return None;
    }
    let mut m = Message {
        kind,
        repeat,
        mmsi,
        class: None,
        fix: None,
        statics: None,
        voyage: None,
        aid: None,
        utc: None,
        known: true,
    };

    match kind {
        // Position report, Class A — scheduled, assigned and interrogated all
        // have the same layout, which is why the standard gives them three
        // numbers and one table.
        1..=3 => {
            m.class = Some(AisKind::ClassA);
            m.fix = Some(Fix {
                nav_status: nav_status(read(bits, 38, 4) as u8),
                turn_deg_min: turn_rate(read_signed(bits, 42, 8)),
                sog_kt: sog_tenths(read(bits, 50, 10)),
                accuracy: bits.get(60).copied().unwrap_or(false),
                lon: lon(read_signed(bits, 61, 28)),
                lat: lat(read_signed(bits, 89, 27)),
                cog_deg: cog_tenths(read(bits, 116, 12)),
                heading_deg: heading(read(bits, 128, 9)),
                raim: bits.get(148).copied().unwrap_or(false),
                altitude_m: None,
            });
        }
        // Base station report, and the UTC/date response that shares its
        // layout.
        4 | 11 => {
            m.class = Some(AisKind::BaseStation);
            m.utc = utc(read(bits, 61, 5), read(bits, 66, 6), read(bits, 72, 6));
            m.fix = Some(Fix {
                accuracy: bits.get(78).copied().unwrap_or(false),
                lon: lon(read_signed(bits, 79, 28)),
                lat: lat(read_signed(bits, 107, 27)),
                raim: bits.get(148).copied().unwrap_or(false),
                ..Fix::default()
            });
        }
        // Static and voyage related data — the message that names the ship.
        5 => {
            m.class = Some(AisKind::ClassA);
            let imo = read(bits, 40, 30) as u32;
            m.statics = Some(Statics {
                // An IMO number is seven digits. Anything shorter is a unit
                // that has never been given one and is filling the field.
                imo: (1_000_000..=9_999_999).contains(&imo).then_some(imo),
                call_sign: some_text(text(bits, 70, 7)),
                name: some_text(text(bits, 112, 20)),
                ship_type: ship_type(read(bits, 232, 8) as u8),
                dim: dim(bits, 240),
            });
            m.voyage = Some(Voyage {
                eta: eta(
                    read(bits, 274, 4),
                    read(bits, 278, 5),
                    read(bits, 283, 5),
                    read(bits, 288, 6),
                ),
                draught_m: draught(read(bits, 294, 8)),
                destination: some_text(text(bits, 302, 20)),
            });
        }
        // Search and rescue aircraft position report: an altitude where a ship
        // has a rate of turn, and speed in whole knots rather than tenths.
        9 => {
            m.class = Some(AisKind::SarAircraft);
            m.fix = Some(Fix {
                altitude_m: altitude(read(bits, 38, 12)),
                sog_kt: sog_whole(read(bits, 50, 10), 1023),
                accuracy: bits.get(60).copied().unwrap_or(false),
                lon: lon(read_signed(bits, 61, 28)),
                lat: lat(read_signed(bits, 89, 27)),
                cog_deg: cog_tenths(read(bits, 116, 12)),
                raim: bits.get(147).copied().unwrap_or(false),
                ..Fix::default()
            });
        }
        // Standard Class B position report.
        18 => {
            m.class = Some(AisKind::ClassB);
            m.fix = Some(Fix {
                sog_kt: sog_tenths(read(bits, 46, 10)),
                accuracy: bits.get(56).copied().unwrap_or(false),
                lon: lon(read_signed(bits, 57, 28)),
                lat: lat(read_signed(bits, 85, 27)),
                cog_deg: cog_tenths(read(bits, 112, 12)),
                heading_deg: heading(read(bits, 124, 9)),
                raim: bits.get(147).copied().unwrap_or(false),
                ..Fix::default()
            });
        }
        // Extended Class B report: the position message and the static one in
        // a single 312-bit transmission.
        19 => {
            m.class = Some(AisKind::ClassB);
            m.fix = Some(Fix {
                sog_kt: sog_tenths(read(bits, 46, 10)),
                accuracy: bits.get(56).copied().unwrap_or(false),
                lon: lon(read_signed(bits, 57, 28)),
                lat: lat(read_signed(bits, 85, 27)),
                cog_deg: cog_tenths(read(bits, 112, 12)),
                heading_deg: heading(read(bits, 124, 9)),
                raim: bits.get(305).copied().unwrap_or(false),
                ..Fix::default()
            });
            m.statics = Some(Statics {
                name: some_text(text(bits, 143, 20)),
                ship_type: ship_type(read(bits, 263, 8) as u8),
                dim: dim(bits, 271),
                ..Statics::default()
            });
        }
        // Aid to navigation report.
        21 => {
            m.class = Some(AisKind::AidToNavigation);
            // The name may run on past the fixed field, in a variable-length
            // extension of up to fourteen more characters. Whether it does is
            // decided by how long the message is, which is the only place in
            // AIS where that matters.
            let extra = bits.len().saturating_sub(272) / 6;
            let mut name = text(bits, 43, 20);
            if extra > 0 {
                name.push_str(&text(bits, 272, extra.min(14)));
            }
            m.aid = Some(Aid {
                aid_type: read(bits, 38, 5) as u8,
                name: name.trim_end().to_string(),
                off_position: bits.get(259).copied().unwrap_or(false),
                virtual_aid: bits.get(269).copied().unwrap_or(false),
                dim: dim(bits, 219),
            });
            m.fix = Some(Fix {
                accuracy: bits.get(163).copied().unwrap_or(false),
                lon: lon(read_signed(bits, 164, 28)),
                lat: lat(read_signed(bits, 192, 27)),
                raim: bits.get(268).copied().unwrap_or(false),
                ..Fix::default()
            });
        }
        // Static data report: Class B's answer to message 5, sent as two
        // halves that arrive seconds apart and are folded onto one row by the
        // tracker.
        24 => {
            m.class = Some(AisKind::ClassB);
            match read(bits, 38, 2) {
                0 => {
                    m.statics =
                        Some(Statics { name: some_text(text(bits, 40, 20)), ..Statics::default() });
                }
                _ => {
                    // On a craft associated with a parent ship the dimension
                    // block is the mothership's MMSI instead, and reading it as
                    // metres would put a 300 m hull on a lifeboat.
                    let auxiliary = (980_000_000..=989_999_999).contains(&mmsi);
                    m.statics = Some(Statics {
                        ship_type: ship_type(read(bits, 40, 8) as u8),
                        call_sign: some_text(text(bits, 90, 7)),
                        dim: (!auxiliary).then(|| dim(bits, 132)).flatten(),
                        ..Statics::default()
                    });
                }
            }
        }
        // Long-range position report, for reception by satellite: coarser
        // units on every field, and no name attached to any of it.
        27 => {
            m.fix = Some(Fix {
                accuracy: bits.first().is_some_and(|_| bits.get(38).copied().unwrap_or(false)),
                raim: bits.get(39).copied().unwrap_or(false),
                nav_status: nav_status(read(bits, 40, 4) as u8),
                lon: lon_coarse(read_signed(bits, 44, 18)),
                lat: lat_coarse(read_signed(bits, 62, 17)),
                sog_kt: sog_whole(read(bits, 79, 6), 63),
                cog_deg: cog_whole(read(bits, 85, 9)),
                ..Fix::default()
            });
        }
        _ => m.known = false,
    }
    Some(m)
}

/// The shortest a message of each type is allowed to be, in bits, per
/// ITU-R M.1371 — and `None` for the type numbers the standard does not define.
///
/// Several types are variable-length (the binary and safety-text messages carry
/// as much as their payload needs, and an aid-to-navigation report carries as
/// much of its name as it has), so this is a floor rather than an exact length.
/// A floor is all that is needed: what it rules out is a frame too short to
/// contain the fields [`parse`] is about to read out of it.
fn payload_bits(kind: u8) -> Option<usize> {
    Some(match kind {
        1..=4 => 168,
        5 => 424,
        6 => 88,
        7 => 72,
        8 => 56,
        9 => 168,
        10 => 72,
        11 => 168,
        12 => 72,
        13 => 72,
        14 => 40,
        15 => 88,
        16 => 96,
        17 => 80,
        18 => 168,
        19 => 312,
        20 => 72,
        21 => 272,
        22 => 168,
        23 => 160,
        // Part A stops at 160; part B runs to 168.
        24 => 160,
        25 => 40,
        26 => 60,
        // Sent as 96 bits, though a few transmitters pad it to a full slot.
        27 => 96,
        // 0 is not a message number, and there is nothing above 27.
        _ => return None,
    })
}

fn some_text(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}

fn nav_status(v: u8) -> Option<u8> {
    (v < 15).then_some(v)
}

fn ship_type(v: u8) -> Option<u8> {
    (v > 0).then_some(v)
}

/// Speed in tenths of a knot. 1023 is "not available" and 1022 is "102.2 knots
/// or more", which is reported as the figure it stands for rather than thrown
/// away — nothing afloat reaches it, but a hydrofoil in a following sea has
/// been known to try.
fn sog_tenths(v: u64) -> Option<f32> {
    (v != 1023).then(|| v as f32 / 10.0)
}

/// Speed in whole knots, with the type's own "not available" value.
fn sog_whole(v: u64, na: u64) -> Option<f32> {
    (v != na).then_some(v as f32)
}

fn cog_tenths(v: u64) -> Option<f32> {
    (v < 3600).then(|| v as f32 / 10.0)
}

fn cog_whole(v: u64) -> Option<f32> {
    (v < 360).then_some(v as f32)
}

fn heading(v: u64) -> Option<f32> {
    (v < 360).then_some(v as f32)
}

fn altitude(v: u64) -> Option<i32> {
    // 4095 is "not available" and 4094 is "4094 metres or more".
    (v != 4095).then_some(v as i32)
}

fn draught(v: u64) -> Option<f32> {
    (v > 0).then(|| v as f32 / 10.0)
}

/// Undo the square-root companding on the rate of turn.
fn turn_rate(v: i64) -> Option<f32> {
    // −128 is "not available"; ±127 is "turning faster than 5° in 30 s", which
    // is 10°/min and is reported as that rather than as 720.
    if v == -128 {
        return None;
    }
    if v.abs() >= 127 {
        return Some(if v < 0 { -10.0 } else { 10.0 });
    }
    let r = (v as f32 / 4.733).powi(2);
    Some(if v < 0 { -r } else { r })
}

fn lon(v: i64) -> Option<f64> {
    if v == LON_NA {
        return None;
    }
    let d = v as f64 / 600_000.0;
    (d.abs() <= 180.0).then_some(d)
}

fn lat(v: i64) -> Option<f64> {
    if v == LAT_NA {
        return None;
    }
    let d = v as f64 / 600_000.0;
    (d.abs() <= 90.0).then_some(d)
}

fn lon_coarse(v: i64) -> Option<f64> {
    if v == 181 * 600 {
        return None;
    }
    let d = v as f64 / 600.0;
    (d.abs() <= 180.0).then_some(d)
}

fn lat_coarse(v: i64) -> Option<f64> {
    if v == 91 * 600 {
        return None;
    }
    let d = v as f64 / 600.0;
    (d.abs() <= 90.0).then_some(d)
}

/// The four distances from the reported position to the hull's edges, where any
/// of them was given.
fn dim(bits: &[bool], at: usize) -> Option<(u16, u16, u16, u16)> {
    let bow = read(bits, at, 9) as u16;
    let stern = read(bits, at + 9, 9) as u16;
    let port = read(bits, at + 18, 6) as u16;
    let stbd = read(bits, at + 24, 6) as u16;
    (bow | stern | port | stbd != 0).then_some((bow, stern, port, stbd))
}

fn utc(hour: u64, minute: u64, second: u64) -> Option<String> {
    (hour < 24 && minute < 60 && second < 60).then(|| format!("{hour:02}:{minute:02}:{second:02}"))
}

fn eta(month: u64, day: u64, hour: u64, minute: u64) -> Option<String> {
    // Zero in the month or day is the "not available" the standard defines;
    // hour 24 and minute 60 are its equivalents for the time of day.
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }
    let hour = if hour > 23 { 24 } else { hour };
    let minute = if minute > 59 { 60 } else { minute };
    if hour == 24 || minute == 60 {
        return Some(format!("{month:02}-{day:02}"));
    }
    Some(format!("{month:02}-{day:02} {hour:02}:{minute:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::Payload;

    /// Issue #330: a frame too short for the message it claims to be would have
    /// the rest of its fields read out of bits that never arrived — and zero is
    /// a valid speed, a valid navigational status and a valid position, so what
    /// came out was not a partial vessel but a whole invented one.
    #[test]
    fn a_frame_too_short_for_its_type_is_not_that_type() {
        // The header of a Class A position report and nothing after it.
        let short = Payload::new(48).put(0, 6, 1).put(8, 30, 244_660_000).bits();
        assert!(parse(&short).is_none(), "48 bits is not a 168-bit position report");

        // The same message at its proper length still decodes.
        let full = Payload::new(168).put(0, 6, 1).put(8, 30, 244_660_000).bits();
        assert_eq!(parse(&full).expect("a header").mmsi, 244_660_000);
    }

    /// The message numbers the standard leaves undefined are not messages, and
    /// neither is 0.
    #[test]
    fn an_undefined_message_number_is_refused() {
        for kind in [0u8, 28, 40, 63] {
            let bits = Payload::new(1_008).put(0, 6, u64::from(kind)).put(8, 30, 1).bits();
            assert!(parse(&bits).is_none(), "message number {kind} is not one");
        }
    }

    /// An MMSI is nine digits; the field it travels in holds ten. A number in
    /// the gap is interference that got through the check sequence, not a ship.
    #[test]
    fn an_mmsi_too_large_to_be_one_is_refused() {
        let bits = Payload::new(168).put(0, 6, 1).put(8, 30, 1_060_322_817).bits();
        assert!(parse(&bits).is_none());

        // The largest that can be an identity is still accepted.
        let bits = Payload::new(168).put(0, 6, 1).put(8, 30, 999_999_999).bits();
        assert_eq!(parse(&bits).expect("a header").mmsi, 999_999_999);
    }

    /// A Class A position report reads back the way it was written, field for
    /// field — the offsets in the standard's Table 18 against the offsets in
    /// this file.
    #[test]
    fn a_class_a_position_report_reads_back() {
        let bits = Payload::new(168)
            .put(0, 6, 1) // message type
            .put(6, 2, 0) // repeat
            .put(8, 30, 244_660_000) // MMSI
            .put(38, 4, 0) // under way using engine
            .put_signed(42, 8, 24) // rate of turn, companded
            .put(50, 10, 123) // 12.3 knots
            .put(60, 1, 1) // DGPS
            .put_signed(61, 28, (4.8973 * 600_000.0) as i64)
            .put_signed(89, 27, (52.3791 * 600_000.0) as i64)
            .put(116, 12, 2_734) // 273.4 degrees
            .put(128, 9, 271)
            .put(148, 1, 1)
            .bits();

        let m = parse(&bits).expect("a header");
        assert!(m.known);
        assert_eq!(m.kind, 1);
        assert_eq!(m.mmsi, 244_660_000);
        assert_eq!(m.class, Some(AisKind::ClassA));
        let f = m.fix.expect("a position report carries a fix");
        assert_eq!(f.nav_status, Some(0));
        assert!((f.lon.expect("lon") - 4.8973).abs() < 1e-5, "{:?}", f.lon);
        assert!((f.lat.expect("lat") - 52.3791).abs() < 1e-5, "{:?}", f.lat);
        assert_eq!(f.sog_kt, Some(12.3));
        assert_eq!(f.cog_deg, Some(273.4));
        assert_eq!(f.heading_deg, Some(271.0));
        assert!(f.accuracy && f.raim);
        // 24 companded is (24/4.733)² ≈ 25.7 degrees a minute, to starboard.
        let rot = f.turn_deg_min.expect("a rate of turn");
        assert!((rot - 25.7).abs() < 0.5, "{rot}");
    }

    /// A longitude west of Greenwich and a turn to port are both negative, and
    /// both have to survive the two's complement round trip. A sign error here
    /// puts every ship in the Atlantic on the wrong side of it.
    #[test]
    fn west_and_port_are_negative() {
        let bits = Payload::new(168)
            .put(0, 6, 1)
            .put(8, 30, 366_000_001)
            .put_signed(42, 8, -24)
            .put_signed(61, 28, (-74.0060 * 600_000.0) as i64)
            .put_signed(89, 27, (40.7128 * 600_000.0) as i64)
            .bits();
        let f = parse(&bits).unwrap().fix.unwrap();
        assert!((f.lon.unwrap() + 74.0060).abs() < 1e-5, "{:?}", f.lon);
        assert!((f.lat.unwrap() - 40.7128).abs() < 1e-5, "{:?}", f.lat);
        assert!(f.turn_deg_min.unwrap() < 0.0);
    }

    /// The reserved "not available" positions are off the earth and must not be
    /// drawn — nor must anything else that is.
    #[test]
    fn a_position_that_is_not_available_is_not_a_position() {
        let bits = Payload::new(168)
            .put(0, 6, 1)
            .put(8, 30, 1)
            .put_signed(61, 28, LON_NA)
            .put_signed(89, 27, LAT_NA)
            .put(50, 10, 1023)
            .put(116, 12, 3600)
            .put(128, 9, 511)
            .put_signed(42, 8, -128)
            .bits();
        let f = parse(&bits).unwrap().fix.unwrap();
        assert_eq!((f.lat, f.lon), (None, None));
        assert_eq!(f.sog_kt, None);
        assert_eq!(f.cog_deg, None);
        assert_eq!(f.heading_deg, None);
        assert_eq!(f.turn_deg_min, None);
    }

    /// A static and voyage report names the ship, and its dimensions are what
    /// make a tanker draw larger than a pilot boat.
    #[test]
    fn a_static_report_names_the_ship() {
        let mut p = Payload::new(424)
            .put(0, 6, 5)
            .put(8, 30, 244_660_000)
            .put(40, 30, 9_876_543)
            .put(232, 8, 80) // tanker
            .put(240, 9, 250)
            .put(249, 9, 50)
            .put(258, 6, 20)
            .put(264, 6, 25)
            .put(274, 4, 6)
            .put(278, 5, 14)
            .put(283, 5, 9)
            .put(288, 6, 30)
            .put(294, 8, 124); // 12.4 m
        p = p.put_text(70, 7, "PBCD").put_text(112, 20, "NORDIC LIGHT").put_text(
            302,
            20,
            "ROTTERDAM",
        );
        let m = parse(&p.bits()).unwrap();
        let s = m.statics.expect("a static report carries statics");
        assert_eq!(s.name.as_deref(), Some("NORDIC LIGHT"));
        assert_eq!(s.call_sign.as_deref(), Some("PBCD"));
        assert_eq!(s.imo, Some(9_876_543));
        assert_eq!(s.ship_type, Some(80));
        assert_eq!(s.dim, Some((250, 50, 20, 25)));
        let v = m.voyage.expect("...and a voyage");
        assert_eq!(v.destination.as_deref(), Some("ROTTERDAM"));
        assert_eq!(v.draught_m, Some(12.4));
        assert_eq!(v.eta.as_deref(), Some("06-14 09:30"));
    }

    /// Class B says the same things at different offsets, and getting one of
    /// them wrong would put a whole class of vessel in the wrong ocean. Message
    /// 19 carries the position and the name together.
    #[test]
    fn class_b_reports_read_at_their_own_offsets() {
        let bits = Payload::new(168)
            .put(0, 6, 18)
            .put(8, 30, 244_123_456)
            .put(46, 10, 62) // 6.2 knots
            .put_signed(57, 28, (11.9746 * 600_000.0) as i64)
            .put_signed(85, 27, (57.7089 * 600_000.0) as i64)
            .put(112, 12, 1_800)
            .put(124, 9, 179)
            .bits();
        let m = parse(&bits).unwrap();
        assert_eq!(m.class, Some(AisKind::ClassB));
        let f = m.fix.unwrap();
        assert_eq!(f.sog_kt, Some(6.2));
        assert_eq!(f.cog_deg, Some(180.0));
        assert_eq!(f.heading_deg, Some(179.0));
        assert!((f.lon.unwrap() - 11.9746).abs() < 1e-5);

        let mut p = Payload::new(312)
            .put(0, 6, 19)
            .put(8, 30, 244_123_456)
            .put(46, 10, 62)
            .put_signed(57, 28, (11.9746 * 600_000.0) as i64)
            .put_signed(85, 27, (57.7089 * 600_000.0) as i64)
            .put(263, 8, 37)
            .put(271, 9, 8)
            .put(280, 9, 4)
            .put(289, 6, 2)
            .put(295, 6, 2);
        p = p.put_text(143, 20, "SEA BREEZE");
        let m = parse(&p.bits()).unwrap();
        assert_eq!(m.statics.unwrap().name.as_deref(), Some("SEA BREEZE"));
        assert!(m.fix.unwrap().lat.is_some());
    }

    /// An aid to navigation is a mark, not a ship: it has a type, a name that
    /// may run past its fixed field, and it may not physically exist.
    #[test]
    fn an_aid_to_navigation_can_be_virtual_and_can_have_a_long_name() {
        let mut p = Payload::new(360)
            .put(0, 6, 21)
            .put(8, 30, 992_111_840)
            .put(38, 5, 30) // special mark
            .put_signed(164, 28, (3.0 * 600_000.0) as i64)
            .put_signed(192, 27, (51.5 * 600_000.0) as i64)
            .put(269, 1, 1)
            .put(259, 1, 1);
        p = p.put_text(43, 20, "MAAS APPROACH BUOY N").put_text(272, 14, "O 3");
        let m = parse(&p.bits()).unwrap();
        let a = m.aid.expect("an aid report");
        assert_eq!(a.aid_type, 30);
        assert_eq!(a.name, "MAAS APPROACH BUOY NO 3");
        assert!(a.virtual_aid && a.off_position);
        assert_eq!(m.class, Some(AisKind::AidToNavigation));
        assert!(m.fix.unwrap().lat.is_some());
    }

    /// The long-range report states everything in coarser units, and reading
    /// it with message 1's scaling would put a ship a thousand times too far
    /// from where it is.
    #[test]
    fn the_long_range_report_uses_its_own_units() {
        let bits = Payload::new(96)
            .put(0, 6, 27)
            .put(8, 30, 244_660_000)
            .put(40, 4, 1) // at anchor
            .put_signed(44, 18, (4.9 * 600.0) as i64)
            .put_signed(62, 17, (52.4 * 600.0) as i64)
            .put(79, 6, 12)
            .put(85, 9, 275)
            .bits();
        let f = parse(&bits).unwrap().fix.unwrap();
        assert!((f.lon.unwrap() - 4.9).abs() < 0.01, "{:?}", f.lon);
        assert!((f.lat.unwrap() - 52.4).abs() < 0.01, "{:?}", f.lat);
        assert_eq!(f.sog_kt, Some(12.0));
        assert_eq!(f.cog_deg, Some(275.0));
        assert_eq!(f.nav_status, Some(1));
    }

    /// The two halves of a Class B static report land on different fields, and
    /// a lifeboat's "dimensions" are its mothership's MMSI rather than a hull.
    #[test]
    fn the_static_data_report_comes_in_two_halves() {
        let a = Payload::new(168).put(0, 6, 24).put(8, 30, 2_441).put(38, 2, 0);
        let a = a.put_text(40, 20, "MERLIN").bits();
        assert_eq!(parse(&a).unwrap().statics.unwrap().name.as_deref(), Some("MERLIN"));

        let b = Payload::new(168)
            .put(0, 6, 24)
            .put(8, 30, 244_123_456)
            .put(38, 2, 1)
            .put(40, 8, 37)
            .put(132, 9, 12)
            .put(141, 9, 3);
        let b = b.put_text(90, 7, "PDXY").bits();
        let s = parse(&b).unwrap().statics.unwrap();
        assert_eq!(s.call_sign.as_deref(), Some("PDXY"));
        assert_eq!(s.ship_type, Some(37));
        assert_eq!(s.dim, Some((12, 3, 0, 0)));

        // The same message from a craft associated with a parent ship: no hull.
        let c = Payload::new(168)
            .put(0, 6, 24)
            .put(8, 30, 982_123_456)
            .put(38, 2, 1)
            .put(132, 9, 12)
            .put(141, 9, 3)
            .bits();
        assert_eq!(parse(&c).unwrap().statics.unwrap().dim, None);
    }

    /// A type this decoder does not read still produces a row, because "this
    /// MMSI is out there" is worth knowing and silence looks like deafness.
    #[test]
    fn an_unread_type_still_says_who_sent_it() {
        let bits = Payload::new(168).put(0, 6, 8).put(8, 30, 2_442_000).bits();
        let m = parse(&bits).unwrap();
        assert!(!m.known);
        assert_eq!(m.kind, 8);
        assert_eq!(m.mmsi, 2_442_000);
        assert!(m.fix.is_none());
    }
}
