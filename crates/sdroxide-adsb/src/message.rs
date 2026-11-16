//! What a Mode S reply says: the field layouts, from the published standard.
//!
//! Written from ICAO Annex 10 Volume IV (3.1.2.6 for the surveillance replies,
//! 3.1.2.8.6 for the extended squitter) and RTCA DO-260B Appendix A (the ME
//! payloads), which between them fix every bit position below. Nothing here is
//! ported from an existing decoder.
//!
//! # The shape of a reply
//!
//! Five bits of downlink format, then a body whose layout that format chooses,
//! then 24 bits of parity ([`crate::crc`]). Short replies are 56 bits, long ones
//! 112; the format decides which, and formats 16 and above are the long ones.
//!
//! Two field encodings recur and are handled once here:
//!
//! * **AC** — an altitude, 13 bits in a surveillance reply and 12 in a position
//!   squitter (the same field without its metric flag).
//! * **ID** — a Mode A identity, 13 bits, interleaved as C1 A1 C2 A2 C4 A4 X
//!   B1 D1 B2 D2 B4 D4 for reasons that date from pulse-position Mode A radar.
//!
//! # Deliberately not decoded
//!
//! **The 100-foot Gillham altitude encoding.** An AC field with its Q bit clear
//! is a Gray-coded altitude in hundred-foot steps, used above 50 175 feet and by
//! a few older transponders. It is not implemented, so an aircraft using it
//! shows no altitude rather than a wrong one; everything civil below the
//! flight levels where it matters uses the 25-foot encoding that is.
//!
//! **The metric altitude encoding** (AC with its M bit set), for the same
//! reason: it is provided for in the standard and used by nothing.
//!
//! **Target state and status (TC 29) and operational status (TC 31).** They
//! carry selected altitude, autopilot modes and integrity figures — real
//! information, but none of it belongs on a target list, and every field would
//! be another thing claimed without a way to check it.

/// The 56 bits of an extended squitter's message field, decoded.
#[derive(Debug, Clone, PartialEq)]
pub enum Es {
    /// TC 1–4: who it is.
    Identification { callsign: String, category: Option<&'static str> },
    /// TC 5–8: where it is on the ground.
    Surface { movement_kt: Option<f32>, track_deg: Option<f32>, cpr: Cpr },
    /// TC 9–18 (barometric) and 20–22 (geometric): where it is in the air.
    Airborne { altitude_ft: Option<i32>, geometric: bool, cpr: Cpr },
    /// TC 19: how fast, which way, and climbing or descending.
    Velocity {
        ground_speed_kt: Option<f32>,
        track_deg: Option<f32>,
        vertical_rate_fpm: Option<i32>,
    },
    /// TC 28: the squawk and whether anything is wrong.
    Status { squawk: Option<u16>, emergency: Option<&'static str> },
    /// TC 19, subtypes 3 and 4: what the aircraft does through the air — its
    /// heading and an airspeed, indicated or true — sent when it has no
    /// velocity over the ground to give.
    Airspeed {
        heading_deg: Option<f32>,
        airspeed_kt: Option<f32>,
        /// The airspeed is true airspeed; otherwise indicated.
        tas: bool,
        vertical_rate_fpm: Option<i32>,
    },
    /// TC 29, subtype 1 (ADS-B version 2): what the crew has set the aircraft
    /// to do — selected altitude and heading, the barometer setting, and the
    /// autopilot's modes.
    TargetState(TargetState),
    /// TC 31: the transponder's ADS-B version, and from version 1 on the
    /// accuracy and integrity of its position.
    OpStatus { version: u8, nac_p: Option<u8>, sil: Option<u8> },
    /// A type code this decoder does not read. Kept as a variant rather than
    /// dropped so the frame still counts as heard, which is what keeps an
    /// aircraft on the list between position reports.
    Other,
}

/// The fields of a target state and status message (TC 29, subtype 1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TargetState {
    /// Selected altitude, feet.
    pub selected_alt_ft: Option<i32>,
    /// The selected altitude comes from the flight management system rather
    /// than the mode control panel.
    pub selected_alt_fms: bool,
    /// Barometric pressure setting, hectopascals.
    pub qnh_hpa: Option<f32>,
    /// Selected heading, degrees.
    pub selected_heading_deg: Option<f32>,
    /// Autopilot and mode flags, when the aircraft says they are valid.
    pub modes: Option<Modes>,
    /// Navigation accuracy category for position.
    pub nac_p: u8,
    /// Source integrity level.
    pub sil: u8,
}

/// The mode bits of a target state message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modes {
    pub autopilot: bool,
    pub vnav: bool,
    pub alt_hold: bool,
    pub approach: bool,
    pub tcas: bool,
    pub lnav: bool,
}

/// One half of a position, as broadcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cpr {
    pub lat: u32,
    pub lon: u32,
    pub odd: bool,
}

/// A reply, by what it carries.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// DF 17/18: an extended squitter, with the address it announced.
    Squitter(Es),
    /// DF 11: an all-call reply. Says only that the aircraft is there, which is
    /// still how a target with no ADS-B at all gets a row.
    AllCall,
    /// DF 0/4/16/20: an answer carrying an altitude.
    Altitude { altitude_ft: Option<i32>, on_ground: bool },
    /// DF 5/21: an answer carrying the squawk.
    Identity { squawk: u16, on_ground: bool },
}

/// A decoded reply. The parity has already been checked by [`crate::frame`];
/// this is only the layout.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub df: u8,
    pub icao: u32,
    pub body: Body,
}

/// `len` bits starting at bit `start`, counting from the message's first bit.
fn bits(msg: &[u8], start: usize, len: usize) -> u32 {
    debug_assert!(len <= 32);
    let mut v = 0u32;
    for i in 0..len {
        let b = start + i;
        let byte = msg.get(b / 8).copied().unwrap_or(0);
        v = (v << 1) | u32::from(byte >> (7 - b % 8) & 1);
    }
    v
}

/// The five-bit downlink format every reply begins with.
pub fn downlink_format(msg: &[u8]) -> u8 {
    (msg.first().copied().unwrap_or(0) >> 3) & 0x1f
}

/// How many bytes a reply of this format is.
pub fn message_len(df: u8) -> usize {
    if df >= 16 { 14 } else { 7 }
}

/// Decode a reply whose parity has already been accepted.
///
/// `icao` is the address the caller established — announced in the frame for a
/// squitter, recovered from the overlaid parity for a reply — because those are
/// two different facts and only the caller knows which one it has.
pub fn decode(msg: &[u8], icao: u32) -> Message {
    let df = downlink_format(msg);
    // Clamped rather than assumed: `crate::frame::accept` has already checked
    // the length, but this is public and a caller with a truncated buffer
    // should get an `Es::Other` rather than a panic.
    let me = msg.get(4..11).unwrap_or(&[]);
    let body = match df {
        17 | 18 => Body::Squitter(decode_es(me)),
        11 => Body::AllCall,
        // Every one of these puts its 13-bit field at the same place: five bits
        // of format and fourteen of status ahead of it.
        0 | 4 | 16 | 20 => Body::Altitude {
            altitude_ft: ac13(bits(msg, 19, 13)),
            on_ground: on_ground(df, bits(msg, 5, 3)),
        },
        5 | 21 => Body::Identity {
            squawk: id13(bits(msg, 19, 13)),
            on_ground: on_ground(df, bits(msg, 5, 3)),
        },
        _ => Body::AllCall,
    };
    Message { df, icao, body }
}

/// Whether the reply says the aircraft is on the ground.
///
/// DF4/5/20/21 carry a three-bit flight status where codes 1, 3 and 5 mean
/// on-ground. DF0/16's first bit is a vertical status where 1 means the same
/// thing; the bits fetched for it are not an FS field at all, so it is read
/// separately rather than run through the same table.
fn on_ground(df: u8, field: u32) -> bool {
    match df {
        0 | 16 => field & 0b100 != 0,
        _ => matches!(field, 1 | 3 | 5),
    }
}

/// Decode the 56-bit message field of an extended squitter.
fn decode_es(me: &[u8]) -> Es {
    if me.len() < 7 {
        return Es::Other;
    }
    let tc = bits(me, 0, 5) as u8;
    match tc {
        1..=4 => {
            let ca = bits(me, 5, 3) as u8;
            let mut call = String::with_capacity(8);
            for i in 0..8 {
                call.push(charset(bits(me, 8 + i * 6, 6) as u8));
            }
            Es::Identification {
                callsign: call.trim_matches(|c| c == '#' || c == ' ').to_string(),
                category: category(tc, ca),
            }
        }
        5..=8 => {
            let mov = bits(me, 5, 7);
            let trk_valid = bits(me, 12, 1) == 1;
            let trk = bits(me, 13, 7);
            Es::Surface {
                movement_kt: movement(mov),
                track_deg: trk_valid.then(|| trk as f32 * 360.0 / 128.0),
                cpr: Cpr {
                    odd: bits(me, 21, 1) == 1,
                    lat: bits(me, 22, 17),
                    lon: bits(me, 39, 17),
                },
            }
        }
        9..=18 | 20..=22 => Es::Airborne {
            altitude_ft: ac12(bits(me, 8, 12)),
            geometric: tc >= 20,
            cpr: Cpr { odd: bits(me, 21, 1) == 1, lat: bits(me, 22, 17), lon: bits(me, 39, 17) },
        },
        19 => velocity(me),
        29 => target_state(me),
        31 => {
            // Version 0 transponders lay the rest of this message out
            // differently, so only the version is read from them.
            let version = bits(me, 40, 3) as u8;
            let v1 = version >= 1;
            Es::OpStatus {
                version,
                nac_p: v1.then(|| bits(me, 44, 4) as u8),
                sil: v1.then(|| bits(me, 50, 2) as u8),
            }
        }
        28 => {
            // Subtype 1 is the emergency/priority status; the rest are ACAS
            // resolution advisories, which say what a collision-avoidance box
            // is telling the crew rather than anything about the aircraft.
            if bits(me, 5, 3) != 1 {
                return Es::Other;
            }
            let sq = id13(bits(me, 11, 13));
            Es::Status {
                squawk: (sq != 0).then_some(sq),
                emergency: match bits(me, 8, 3) {
                    1 => Some("general emergency"),
                    2 => Some("lifeguard / medical"),
                    3 => Some("minimum fuel"),
                    4 => Some("no communication"),
                    5 => Some("unlawful interference"),
                    6 => Some("downed aircraft"),
                    _ => None,
                },
            }
        }
        _ => Es::Other,
    }
}

/// TC 19, subtypes 1 and 2: velocity over the ground.
///
/// Subtypes 3 and 4 give an airspeed and a heading instead, which are what the
/// aircraft is doing through the air rather than over the map. They go to
/// [`airspeed`] and into fields of their own: a leader line drawn from an
/// airspeed points somewhere the aircraft is not going, and the two cannot be
/// told apart once they are in the same field.
fn velocity(me: &[u8]) -> Es {
    let st = bits(me, 5, 3);
    // The supersonic subtypes carry the same fields with four times the scale.
    let scale = match st {
        1 => 1.0f32,
        2 => 4.0,
        3 | 4 => return airspeed(me, if st == 4 { 4.0 } else { 1.0 }),
        _ => return Es::Other,
    };
    let ew_west = bits(me, 13, 1) == 1;
    let ew = bits(me, 14, 10);
    let ns_south = bits(me, 24, 1) == 1;
    let ns = bits(me, 25, 10);

    // Zero means "no information"; one means zero. Everything is offset by one
    // so that the all-zeroes field can mean absent.
    let (speed, track) = if ew == 0 || ns == 0 {
        (None, None)
    } else {
        let vx = (ew as f32 - 1.0) * scale * if ew_west { -1.0 } else { 1.0 };
        let vy = (ns as f32 - 1.0) * scale * if ns_south { -1.0 } else { 1.0 };
        let deg = vx.atan2(vy).to_degrees();
        (Some(vx.hypot(vy)), Some(if deg < 0.0 { deg + 360.0 } else { deg }))
    };

    Es::Velocity { ground_speed_kt: speed, track_deg: track, vertical_rate_fpm: vertical_rate(me) }
}

/// The vertical rate field of TC 19, all subtypes alike: 64 ft/min steps,
/// zero meaning absent.
fn vertical_rate(me: &[u8]) -> Option<i32> {
    let raw = bits(me, 37, 9);
    (raw != 0).then(|| {
        let mag = (raw as i32 - 1) * 64;
        if bits(me, 36, 1) == 1 { -mag } else { mag }
    })
}

/// TC 19, subtypes 3 and 4: heading and airspeed.
///
/// Kept apart from the ground velocity on purpose: the heading is where the
/// nose points and the airspeed is through the air, so neither may feed the
/// track or the leader line (see [`velocity`]).
fn airspeed(me: &[u8], scale: f32) -> Es {
    let heading_deg = (bits(me, 13, 1) == 1).then(|| bits(me, 14, 10) as f32 * 360.0 / 1024.0);
    let raw = bits(me, 25, 10);
    Es::Airspeed {
        heading_deg,
        airspeed_kt: (raw != 0).then_some((raw as f32 - 1.0) * scale),
        tas: bits(me, 24, 1) == 1,
        vertical_rate_fpm: vertical_rate(me),
    }
}

/// TC 29: target state and status. Only subtype 1 (DO-260B, ADS-B version 2)
/// is read; subtype 0 is the older, differently laid out message.
fn target_state(me: &[u8]) -> Es {
    if bits(me, 5, 2) != 1 {
        return Es::Other;
    }
    let alt = bits(me, 9, 11);
    let baro = bits(me, 20, 9);
    // Heading: a sign bit and eight bits, 180/256 of a degree each — the same
    // as nine bits of 360/512 read unsigned.
    let hdg = (bits(me, 29, 1) == 1).then(|| bits(me, 30, 9) as f32 * 360.0 / 512.0);
    let modes = (bits(me, 46, 1) == 1).then(|| Modes {
        autopilot: bits(me, 47, 1) == 1,
        vnav: bits(me, 48, 1) == 1,
        alt_hold: bits(me, 49, 1) == 1,
        approach: bits(me, 51, 1) == 1,
        tcas: bits(me, 52, 1) == 1,
        lnav: bits(me, 53, 1) == 1,
    });
    Es::TargetState(TargetState {
        selected_alt_ft: (alt != 0).then(|| (alt as i32 - 1) * 32),
        selected_alt_fms: bits(me, 8, 1) == 1,
        qnh_hpa: (baro != 0).then_some(800.0 + (baro as f32 - 1.0) * 0.8),
        selected_heading_deg: hdg,
        modes,
        nac_p: bits(me, 39, 4) as u8,
        sil: bits(me, 44, 2) as u8,
    })
}

/// The surface movement code, which is a speed on a piecewise scale that gets
/// coarser as it goes up (DO-260B A.2.3.2.5).
fn movement(mov: u32) -> Option<f32> {
    Some(match mov {
        0 => return None,
        1 => 0.0,
        2..=8 => 0.125 + (mov - 2) as f32 * 0.125,
        9..=12 => 1.0 + (mov - 9) as f32 * 0.25,
        13..=38 => 2.0 + (mov - 13) as f32 * 0.5,
        39..=93 => 15.0 + (mov - 39) as f32,
        94..=108 => 70.0 + (mov - 94) as f32 * 2.0,
        109..=123 => 100.0 + (mov - 109) as f32 * 5.0,
        124 => 175.0,
        _ => return None,
    })
}

/// The six-bit character set the identification field uses: 1–26 are the
/// letters, 48–57 the digits, 32 a space, and everything else is unassigned.
fn charset(v: u8) -> char {
    match v {
        1..=26 => (b'A' + v - 1) as char,
        48..=57 => (b'0' + v - 48) as char,
        32 => ' ',
        _ => '#',
    }
}

/// Emitter category: four sets of eight, selected by the type code. Set A (type
/// code 4) is the aeroplanes by weight, B (3) is everything else that flies,
/// C (2) is things on the ground, and D (1) is reserved.
fn category(tc: u8, ca: u8) -> Option<&'static str> {
    match (tc, ca) {
        (_, 0) => None,
        (4, 1) => Some("light"),
        (4, 2) => Some("small"),
        (4, 3) => Some("large"),
        (4, 4) => Some("high-vortex large"),
        (4, 5) => Some("heavy"),
        (4, 6) => Some("high performance"),
        (4, 7) => Some("rotorcraft"),
        (3, 1) => Some("glider"),
        (3, 2) => Some("lighter than air"),
        (3, 3) => Some("parachutist"),
        (3, 4) => Some("ultralight"),
        (3, 6) => Some("unmanned"),
        (3, 7) => Some("space vehicle"),
        (2, 1) => Some("emergency vehicle"),
        (2, 2) => Some("service vehicle"),
        (2, 3..=5) => Some("obstacle"),
        _ => None,
    }
}

/// The 12-bit altitude in a position squitter: the AC field without its metric
/// flag. Q set means 25-foot steps; see the module note for why Q clear is not
/// decoded.
pub fn ac12(v: u32) -> Option<i32> {
    if v == 0 {
        return None;
    }
    if v & 0x10 == 0 {
        return None;
    }
    let n = ((v & 0x0FE0) >> 1) | (v & 0x000F);
    let ft = n as i32 * 25 - 1000;
    (ft > -1000).then_some(ft)
}

/// The 13-bit altitude in a surveillance reply: the same field with a metric
/// flag inserted, which is why the halves are gathered differently.
pub fn ac13(v: u32) -> Option<i32> {
    if v == 0 || v == 0x1FFF {
        return None;
    }
    // M set is a metric altitude, which nothing transmits.
    if v & 0x0040 != 0 {
        return None;
    }
    if v & 0x0010 == 0 {
        return None;
    }
    let n = ((v & 0x1F80) >> 2) | ((v & 0x0020) >> 1) | (v & 0x000F);
    let ft = n as i32 * 25 - 1000;
    (ft > -1000).then_some(ft)
}

/// The 13-bit Mode A identity, as the four digits an operator reads.
///
/// The interleaving is Mode A's pulse order — C1 A1 C2 A2 C4 A4 X B1 D1 B2 D2
/// B4 D4 — carried into Mode S unchanged, which is why it looks arbitrary.
pub fn id13(v: u32) -> u16 {
    let bit = |n: u32| (v >> n) & 1;
    let a = bit(7) * 4 + bit(9) * 2 + bit(11);
    let b = bit(1) * 4 + bit(3) * 2 + bit(5);
    let c = bit(8) * 4 + bit(10) * 2 + bit(12);
    let d = bit(0) * 4 + bit(2) * 2 + bit(4);
    (a * 1000 + b * 100 + c * 10 + d) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap()).collect()
    }

    fn es_of(s: &str) -> Es {
        let b = hex(s);
        match decode(&b, 0).body {
            Body::Squitter(es) => es,
            other => panic!("not a squitter: {other:?}"),
        }
    }

    /// The published identification squitter. Both halves matter: getting the
    /// six-bit character set wrong gives a callsign that is plausible and
    /// wrong, which is worse than none.
    #[test]
    fn an_identification_squitter_reads_back_its_callsign() {
        let Es::Identification { callsign, category } = es_of("8D4840D6202CC371C32CE0576098")
        else {
            panic!("not an identification");
        };
        assert_eq!(callsign, "KLM1023");
        assert_eq!(category, None, "category 0 is 'no information', not a category");
    }

    /// The published velocity squitter, against the figures quoted with it:
    /// 322.197 degrees, 417.655 knots, level.
    #[test]
    fn a_velocity_squitter_reads_back_its_track_and_speed() {
        let Es::Velocity { ground_speed_kt, track_deg, vertical_rate_fpm } =
            es_of("8DA3D42599250129780484712C50")
        else {
            panic!("not a velocity");
        };
        let gs = ground_speed_kt.expect("a ground speed");
        let trk = track_deg.expect("a track");
        assert!((gs - 417.655).abs() < 0.01, "ground speed {gs}");
        assert!((trk - 322.197).abs() < 0.01, "track {trk}");
        assert_eq!(vertical_rate_fpm, Some(0));
    }

    /// The published airspeed squitter (TC 19 subtype 3), against the figures
    /// quoted with it: heading 243.98°, true airspeed 375 kt, 2304 ft/min down.
    #[test]
    fn an_airspeed_squitter_reads_back_heading_and_true_airspeed() {
        let Es::Airspeed { heading_deg, airspeed_kt, tas, vertical_rate_fpm } =
            es_of("8DA05F219B06B6AF189400CBC33F")
        else {
            panic!("not an airspeed message");
        };
        let h = heading_deg.expect("a heading");
        assert!((h - 243.98).abs() < 0.01, "heading {h}");
        assert_eq!(airspeed_kt, Some(375.0));
        assert!(tas);
        assert_eq!(vertical_rate_fpm, Some(-2304));
    }

    /// The published target state squitter (TC 29 subtype 1): selected
    /// altitude 16 992 ft from the MCP, 1012.8 hPa, selected heading 66.8°,
    /// autopilot, VNAV, LNAV and TCAS on, NACp 9, SIL 3 — the figures the
    /// vendored `xng-mode-adsb` tests expect for it (rs1090 rounds the
    /// altitude to 17 000).
    #[test]
    fn a_target_state_squitter_reads_back_what_the_crew_has_set() {
        let Es::TargetState(t) = es_of("8DA05629EA21485CBF3F8CADAEEB") else {
            panic!("not a target state");
        };
        assert_eq!(t.selected_alt_ft, Some(16_992));
        assert!(!t.selected_alt_fms);
        let q = t.qnh_hpa.expect("a baro setting");
        assert!((q - 1012.8).abs() < 0.05, "qnh {q}");
        let h = t.selected_heading_deg.expect("a selected heading");
        assert!((h - 66.8).abs() < 0.1, "selected heading {h}");
        let m = t.modes.expect("modes");
        assert!(m.autopilot && m.vnav && m.lnav && m.tcas);
        assert!(!m.alt_hold && !m.approach);
        assert_eq!((t.nac_p, t.sil), (9, 3));
    }

    /// An operational status message names its version; a version 0 one gives
    /// nothing else.
    #[test]
    fn an_operational_status_gives_the_version() {
        let mut me = [0u8; 7];
        me[0] = 31 << 3;
        let es = |me: &[u8; 7]| decode_es(me);
        assert_eq!(es(&me), Es::OpStatus { version: 0, nac_p: None, sil: None });
        me[5] = 0b0100_1010; // version 2, NIC-A 0, NACp 10
        me[6] = 0b0011_0000; // SIL 3
        assert_eq!(es(&me), Es::OpStatus { version: 2, nac_p: Some(10), sil: Some(3) });
    }

    /// The published position pair, at 38 000 feet, one even and one odd.
    #[test]
    fn a_position_squitter_reads_back_its_altitude_and_parity() {
        let Es::Airborne { altitude_ft, geometric, cpr } = es_of("8D40621D58C382D690C8AC2863A7")
        else {
            panic!("not a position");
        };
        assert_eq!(altitude_ft, Some(38_000));
        assert!(!geometric);
        assert!(!cpr.odd);
        assert_eq!(cpr.lat, 93_000);
        assert_eq!(cpr.lon, 51_372);

        let Es::Airborne { cpr, .. } = es_of("8D40621D58C386435CC412692AD6") else {
            panic!("not a position");
        };
        assert!(cpr.odd);
        assert_eq!(cpr.lat, 74_158);
        assert_eq!(cpr.lon, 50_194);
    }

    /// The layout functions are public, so a truncated buffer has to come back
    /// as "nothing recognised" rather than as a panic.
    #[test]
    fn a_short_buffer_decodes_to_nothing_rather_than_panicking() {
        // An empty buffer has no downlink format either, so it reads as DF0;
        // from one byte on it is the squitter it claims to be, with nothing in
        // it until there is a whole message field to read.
        assert!(matches!(decode(&[], 0).body, Body::Altitude { .. }));
        for len in 1..14 {
            let msg = vec![17u8 << 3; len];
            let m = decode(&msg, 0);
            assert_eq!(m.icao, 0);
            if len < 11 {
                assert_eq!(m.body, Body::Squitter(Es::Other), "len {len}");
            }
        }
    }

    /// The identity field's interleaving is the one piece of this format that
    /// cannot be guessed. Squawk 7000 is the European VFR conspicuity code and
    /// 1200 the American one; both are things an operator will recognise
    /// immediately if they are right and immediately if they are not.
    #[test]
    fn the_identity_field_unpicks_into_the_digits_it_is_read_as() {
        // C1 A1 C2 A2 C4 A4 X B1 D1 B2 D2 B4 D4, built digit by digit.
        let pack = |a: u32, b: u32, c: u32, d: u32| -> u32 {
            let bit = |v: u32, n: u32| (v >> n) & 1;
            bit(c, 0) << 12
                | bit(a, 0) << 11
                | bit(c, 1) << 10
                | bit(a, 1) << 9
                | bit(c, 2) << 8
                | bit(a, 2) << 7
                | bit(b, 0) << 5
                | bit(d, 0) << 4
                | bit(b, 1) << 3
                | bit(d, 1) << 2
                | bit(b, 2) << 1
                | bit(d, 2)
        };
        assert_eq!(id13(pack(7, 0, 0, 0)), 7000);
        assert_eq!(id13(pack(1, 2, 0, 0)), 1200);
        assert_eq!(id13(pack(7, 7, 0, 0)), 7700);
        assert_eq!(id13(pack(0, 0, 2, 1)), 21);
    }

    /// A twenty-five-foot altitude round-trips, and the encodings this decoder
    /// declines are declined rather than answered with a wrong number.
    #[test]
    fn the_altitude_field_decodes_the_encoding_everything_uses() {
        // Build the 25-foot form for 38 000 ft: n = (38000 + 1000) / 25.
        let n = ((38_000 + 1000) / 25) as u32;
        let v = ((n & 0x7F0) << 1) | 0x10 | (n & 0x00F);
        assert_eq!(ac12(v), Some(38_000));
        assert_eq!(ac12(0), None, "an all-zero field is 'no altitude'");
        assert_eq!(ac12(v & !0x10), None, "Gillham is declined, not guessed at");
        assert_eq!(ac13(0x0040), None, "and so is metric");
    }

    /// A surveillance reply's altitude sits at the same offset whatever the
    /// format, and the ground flag comes from a different field in DF0 than in
    /// DF4 — the same three bits mean different things there.
    #[test]
    fn a_surveillance_reply_yields_an_altitude_where_the_standard_puts_it() {
        // The 25-foot encoding of 5 000 ft, packed into the 13-bit AC field:
        // n[10:5] at v[12:7], n[4] at v[5], n[3:0] at v[3:0], Q set, M clear.
        let n = ((5_000 + 1000) / 25) as u32;
        let ac = ((n & 0x7E0) << 2) | 0x10 | ((n & 0x010) << 1) | (n & 0x00F);

        let build = |df: u8, status: u32, field: u32| {
            let mut msg = [0u8; 7];
            msg[0] = df << 3;
            for i in 0..3 {
                if status >> (2 - i) & 1 == 1 {
                    msg[(5 + i) / 8] |= 0x80 >> ((5 + i) % 8);
                }
            }
            for i in 0..13 {
                if field >> (12 - i) & 1 == 1 {
                    msg[(19 + i) / 8] |= 0x80 >> ((19 + i) % 8);
                }
            }
            msg
        };

        let Body::Altitude { altitude_ft, on_ground } = decode(&build(4, 0, ac), 0).body else {
            panic!("not an altitude reply");
        };
        assert_eq!(altitude_ft, Some(5_000));
        assert!(!on_ground);

        // Flight status 1 is on the ground...
        let Body::Altitude { on_ground, .. } = decode(&build(4, 1, ac), 0).body else {
            panic!("not an altitude reply");
        };
        assert!(on_ground);

        // ...but in DF0 those bits are a vertical status, and it is the first
        // of them that says so.
        let Body::Altitude { on_ground, .. } = decode(&build(0, 1, ac), 0).body else {
            panic!("not an altitude reply");
        };
        assert!(!on_ground, "flight status 1 is not a DF0 vertical status");
        let Body::Altitude { on_ground, .. } = decode(&build(0, 4, ac), 0).body else {
            panic!("not an altitude reply");
        };
        assert!(on_ground);
    }
}
