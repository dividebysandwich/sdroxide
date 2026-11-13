//! JS8 frame layouts — what the 72 payload bits mean.
//!
//! ## Two type fields, and why
//!
//! A JS8 frame carries *two* independent classifications, which is easy to
//! conflate and expensive to get wrong:
//!
//! * **[`Js8Flags`]** lives in the three `i3bit` bits of the 87-bit information
//!   word, after the payload and before the CRC. It says where this frame sits
//!   in a message — first, last, both (a single-frame message), or neither —
//!   and whether the payload has a type header at all. This is the multi-frame
//!   framing, and it is the answer to "how does a receiver know a long message
//!   has ended".
//! * **[`Js8FrameType`]** lives in the first three bits *of the payload* and
//!   says what kind of frame this is: heartbeat, directed command, free text.
//!   It is absent when [`Js8Flags::DATA`] is set, in which case all 72 bits are
//!   payload.
//!
//! Both are three bits wide and both are called a "type" upstream; they are not
//! the same field and do not carry the same values.

use super::message::Js8Payload;
use super::varicode::{self, pack_callsign, unpack_callsign};

/// Sequencing flags — the `i3bit` field. `varicode.h:36-39`.
///
/// These are bit flags rather than an enumeration: a single-frame message sets
/// `FIRST | LAST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Js8Flags(pub u8);

impl Js8Flags {
    /// A continuation frame — neither the first nor the last.
    pub const MIDDLE: Js8Flags = Js8Flags(0);
    /// The first frame of a message.
    pub const FIRST: u8 = 1;
    /// The last frame of a message.
    pub const LAST: u8 = 2;
    /// The payload has no frame-type header; all 72 bits are data.
    pub const DATA: u8 = 4;

    /// A message that fits in one frame: both first and last.
    pub fn single() -> Self {
        Js8Flags(Self::FIRST | Self::LAST)
    }

    pub fn is_first(self) -> bool {
        self.0 & Self::FIRST != 0
    }
    pub fn is_last(self) -> bool {
        self.0 & Self::LAST != 0
    }
    /// True when the payload carries no type header.
    pub fn is_headerless_data(self) -> bool {
        self.0 & Self::DATA != 0
    }
}

/// Frame type, in the first three bits of the payload. `varicode.h:50-57`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Js8FrameType {
    Heartbeat = 0,
    Compound = 1,
    CompoundDirected = 2,
    Directed = 3,
    /// Free text. Encoded as `[10X]`, so only the top two bits are the type and
    /// the third is payload.
    Data = 4,
    /// Free text compressed against the shared dictionary, `[11X]`.
    DataCompressed = 6,
}

impl Js8FrameType {
    pub fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Js8FrameType::Heartbeat),
            1 => Some(Js8FrameType::Compound),
            2 => Some(Js8FrameType::CompoundDirected),
            3 => Some(Js8FrameType::Directed),
            // The data types swallow their low bit as payload.
            4 | 5 => Some(Js8FrameType::Data),
            6 | 7 => Some(Js8FrameType::DataCompressed),
            _ => None,
        }
    }
}

/// The directed-command table, `varicode.cpp:46`.
///
/// Entries are `(text, code)`. Several texts share a code — `"?"` is a
/// shorthand for `" SNR?"`, `" QUERY MSGS?"` for `" QUERY MSGS"` — so the first
/// entry for a code is the one used when rendering.
pub const DIRECTED_CMDS: &[(&str, i8)] = &[
    (" SNR?", 0),
    ("?", 0),
    (" DIT DIT", 1),
    (" NACK", 2),
    (" HEARING?", 3),
    (" GRID?", 4),
    (">", 5),
    (" STATUS?", 6),
    (" STATUS", 7),
    (" HEARING", 8),
    (" MSG", 9),
    (" MSG TO:", 10),
    (" QUERY", 11),
    (" QUERY MSGS", 12),
    (" QUERY MSGS?", 12),
    (" QUERY CALL", 13),
    (" ACK", 14),
    (" GRID", 15),
    (" INFO?", 16),
    (" INFO", 17),
    (" FB", 18),
    (" HW CPY?", 19),
    (" SK", 20),
    (" RR", 21),
    (" QSL?", 22),
    (" QSL", 23),
    (" CMD", 24),
    (" SNR", 25),
    (" NO", 26),
    (" YES", 27),
    (" 73", 28),
    (" HEARTBEAT SNR", 29),
    (" AGN?", 30),
    (" ", 31),
];

/// Commands that carry a signal report in their number field, `snr_cmds`.
pub const SNR_CMDS: &[i8] = &[25, 29];

/// Commands a station may answer automatically, `autoreply_cmds`.
pub const AUTOREPLY_CMDS: &[i8] = &[0, 2, 3, 4, 6, 9, 10, 11, 12, 13, 14, 16, 30];

/// Look up a command's code by its text.
pub fn cmd_code(text: &str) -> Option<i8> {
    DIRECTED_CMDS.iter().find(|(t, _)| *t == text).map(|(_, c)| *c)
}

/// Render a command code back to text, preferring the first spelling.
pub fn cmd_text(code: i8) -> Option<&'static str> {
    DIRECTED_CMDS.iter().find(|(_, c)| *c == code).map(|(t, _)| *t)
}

/// A directed command addressed to a station or group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directed {
    pub from: String,
    pub to: String,
    /// Command code; `31` is plain free text.
    pub cmd: i8,
    /// The number or signal report riding along, when the command carries one.
    pub num: Option<i32>,
    pub portable_from: bool,
    pub portable_to: bool,
}

impl Directed {
    /// Lay the frame out as 72 payload bits.
    ///
    /// `[3 type][28 from][28 to][5 cmd]` then `[1 portable_from]
    /// [1 portable_to][6 num]` — 64 + 8, exactly filling the payload.
    pub fn pack(&self) -> Option<Js8Payload> {
        let from = pack_callsign(&self.from)?;
        let to = pack_callsign(&self.to)?;
        let mut bits = [0u8; 72];
        let mut at = 0usize;
        let put = |value: u64, width: usize, bits: &mut [u8; 72], at: &mut usize| {
            for i in 0..width {
                bits[*at + i] = ((value >> (width - 1 - i)) & 1) as u8;
            }
            *at += width;
        };
        put(Js8FrameType::Directed as u64, 3, &mut bits, &mut at);
        put(u64::from(from), 28, &mut bits, &mut at);
        put(u64::from(to), 28, &mut bits, &mut at);
        put((self.cmd as u8 % 32) as u64, 5, &mut bits, &mut at);
        put(u64::from(self.portable_from), 1, &mut bits, &mut at);
        put(u64::from(self.portable_to), 1, &mut bits, &mut at);
        put(u64::from(self.num.map_or(0, varicode::pack_num)), 6, &mut bits, &mut at);
        debug_assert_eq!(at, 72);
        Some(Js8Payload { bits, frame_type: 0 })
    }

    /// Read a directed frame back, or `None` if the payload is a different type.
    pub fn unpack(payload: &Js8Payload) -> Option<Self> {
        let get = |start: usize, width: usize| -> u64 {
            payload.bits[start..start + width].iter().fold(0u64, |a, &b| (a << 1) | u64::from(b))
        };
        if get(0, 3) != Js8FrameType::Directed as u64 {
            return None;
        }
        let portable_from = get(64, 1) == 1;
        let portable_to = get(65, 1) == 1;
        let raw_num = get(66, 6) as u8;
        Some(Directed {
            from: unpack_callsign(get(3, 28) as u32, portable_from)?,
            to: unpack_callsign(get(31, 28) as u32, portable_to)?,
            cmd: get(59, 5) as i8,
            // Zero means "no number", not "report −31".
            num: (raw_num != 0).then(|| varicode::unpack_num(raw_num)),
            portable_from,
            portable_to,
        })
    }

    /// How this reads in the activity list.
    pub fn render(&self) -> String {
        let cmd = cmd_text(self.cmd).unwrap_or(" ");
        let mut s = format!("{}: {}{}", self.from, self.to, cmd);
        if let Some(n) = self.num {
            if SNR_CMDS.contains(&self.cmd) {
                s.push_str(&format!(" {n:+03}"));
            } else {
                s.push_str(&format!(" {n}"));
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_compound_directed_frame_carries_its_addressee_and_command() {
        use super::{Compound, Js8FrameType, cmd_code};
        for (to, cmd, num) in [
            ("@MAGNET", " ", None),
            ("W1AW/4", " GRID?", None),
            ("@MAGNET", " SNR", Some(-10)),
            ("N0JDS", " HEARTBEAT SNR", Some(5)),
        ] {
            let code = cmd_code(cmd).expect("known command");
            let f = Compound::directed(to, code, num).pack().expect("packs");
            let back = Compound::unpack(&f).expect("unpacks");
            assert_eq!(back.kind, Js8FrameType::CompoundDirected, "{to}{cmd}");
            assert_eq!(back.call, to);
            assert_eq!(back.command(), Some((code, num)), "{to}{cmd}");
            assert_eq!(back.grid(), None, "a command is not a grid");
        }
        // The identifying frame carries a grid and no command.
        let id =
            Compound::unpack(&Compound::ident("KN4CRD/QRP", Some("EM73")).pack().unwrap()).unwrap();
        assert_eq!(id.kind, Js8FrameType::Compound);
        assert_eq!(id.grid().as_deref(), Some("EM73"));
        assert_eq!(id.command(), None);
    }

    use super::*;

    fn directed(from: &str, to: &str, cmd: &str) -> Directed {
        Directed {
            from: from.into(),
            to: to.into(),
            cmd: cmd_code(cmd).unwrap_or_else(|| panic!("no command {cmd:?}")),
            num: None,
            portable_from: false,
            portable_to: false,
        }
    }

    #[test]
    fn a_directed_frame_round_trips() {
        for (from, to, cmd) in [
            ("KN4CRD", "N0JDS", " SNR?"),
            ("VK3ABC", "@ALLCALL", " HEARING?"),
            ("G0ABC", "@JS8NET", " STATUS?"),
            ("N0JDS", "KN4CRD", " QSL"),
        ] {
            let d = directed(from, to, cmd);
            let payload = d.pack().expect("packs");
            assert_eq!(Directed::unpack(&payload).as_ref(), Some(&d), "{from} -> {to} {cmd}");
        }
    }

    #[test]
    fn a_report_survives_the_round_trip() {
        for report in [-30, -12, -5, 0, 5, 15, 31] {
            let mut d = directed("KN4CRD", "N0JDS", " SNR");
            d.num = Some(report);
            let payload = d.pack().expect("packs");
            assert_eq!(Directed::unpack(&payload).and_then(|d| d.num), Some(report), "{report}");
        }
    }

    #[test]
    fn no_number_is_distinct_from_a_number_of_zero() {
        // The field biases by 31 so that zero can mean "absent"; conflating the
        // two would put a spurious report on every plain command.
        let plain = directed("KN4CRD", "N0JDS", " QSL").pack().expect("packs");
        assert_eq!(Directed::unpack(&plain).and_then(|d| d.num), None);

        let mut with_zero = directed("KN4CRD", "N0JDS", " SNR");
        with_zero.num = Some(0);
        let payload = with_zero.pack().expect("packs");
        assert_eq!(Directed::unpack(&payload).and_then(|d| d.num), Some(0));
    }

    #[test]
    fn portable_flags_ride_alongside_the_callsigns() {
        let mut d = directed("KN4CRD", "N0JDS", " SNR?");
        d.portable_from = true;
        d.portable_to = true;
        d.from = "KN4CRD/P".into();
        d.to = "N0JDS/P".into();
        let payload = d.pack().expect("packs");
        let back = Directed::unpack(&payload).expect("unpacks");
        assert_eq!(back.from, "KN4CRD/P");
        assert_eq!(back.to, "N0JDS/P");
    }

    #[test]
    fn the_payload_fills_exactly_seventytwo_bits() {
        let payload = directed("KN4CRD", "N0JDS", " SNR?").pack().expect("packs");
        assert_eq!(payload.bits.len(), 72);
        // And the type header is where the receiver looks for it.
        assert_eq!(&payload.bits[..3], &[0, 1, 1], "FrameDirected is 3 = 0b011");
    }

    #[test]
    fn every_command_code_maps_back_to_a_text() {
        for (_, code) in DIRECTED_CMDS {
            assert!(cmd_text(*code).is_some(), "code {code} has no text");
        }
    }

    #[test]
    fn shorthand_spellings_share_their_codes() {
        assert_eq!(cmd_code("?"), cmd_code(" SNR?"));
        assert_eq!(cmd_code(" QUERY MSGS?"), cmd_code(" QUERY MSGS"));
        // …and rendering prefers the long form.
        assert_eq!(cmd_text(0), Some(" SNR?"));
    }

    #[test]
    fn a_heartbeat_round_trips_with_and_without_a_grid() {
        for grid in [Some("EM73"), Some("QF22"), None] {
            let hb = Compound::heartbeat("KN4CRD", grid);
            let payload = hb.pack().expect("packs");
            let back = Compound::unpack(&payload).expect("unpacks");
            assert_eq!(back.call, "KN4CRD");
            assert_eq!(back.grid().as_deref(), grid, "{grid:?}");
            assert!(!back.is_cq());
        }
    }

    #[test]
    fn a_cq_is_distinguished_from_a_heartbeat_by_one_bit() {
        let hb = Compound::heartbeat("KN4CRD", Some("EM73"));
        let cq = Compound::cq("KN4CRD", Some("EM73"), 0);
        assert!(!hb.is_cq() && cq.is_cq());
        // …and the grid still reads correctly through the flag.
        assert_eq!(cq.grid().as_deref(), Some("EM73"));
        let back = Compound::unpack(&cq.pack().expect("packs")).expect("unpacks");
        assert!(back.is_cq());
        assert_eq!(back.grid().as_deref(), Some("EM73"));
    }

    #[test]
    fn the_sixteen_bit_num_survives_being_split_across_the_boundary() {
        // `num` is cut into 11 high bits and 5 low bits either side of the
        // 64/8 split. A frame that dropped the high half would still look
        // plausible — it would just report the wrong grid.
        for num in [0u16, 1, 31, 32, 1023, 1024, 32767, 0xffff] {
            let c = Compound { kind: Js8FrameType::Heartbeat, call: "KN4CRD".into(), num, sub: 5 };
            let back = Compound::unpack(&c.pack().expect("packs")).expect("unpacks");
            assert_eq!(back.num, num, "num {num}");
            assert_eq!(back.sub, 5);
        }
    }

    #[test]
    fn directed_and_data_frames_cannot_use_the_compound_layout() {
        for kind in [Js8FrameType::Directed, Js8FrameType::Data] {
            let c = Compound { kind, call: "KN4CRD".into(), num: 0, sub: 0 };
            assert!(c.pack().is_none(), "{kind:?} must not pack as compound");
        }
    }

    #[test]
    fn sequencing_flags_are_independent_bits() {
        assert!(Js8Flags::single().is_first() && Js8Flags::single().is_last());
        assert!(!Js8Flags::MIDDLE.is_first() && !Js8Flags::MIDDLE.is_last());
        assert!(Js8Flags(Js8Flags::DATA).is_headerless_data());
        assert!(!Js8Flags::single().is_headerless_data());
    }

    #[test]
    fn the_data_frame_types_swallow_their_low_bit() {
        // `[10X]` and `[11X]` — the third bit is payload, not type.
        assert_eq!(Js8FrameType::from_bits(4), Some(Js8FrameType::Data));
        assert_eq!(Js8FrameType::from_bits(5), Some(Js8FrameType::Data));
        assert_eq!(Js8FrameType::from_bits(6), Some(Js8FrameType::DataCompressed));
        assert_eq!(Js8FrameType::from_bits(7), Some(Js8FrameType::DataCompressed));
    }
}

/// Heartbeat sub-types, `varicode.cpp:297`. All render as "HB"; the flag bits
/// distinguished auto/relay/spot behaviour, deprecated as of JS8Call 2.2 but
/// still transmitted, so we decode them and keep the number.
pub const HB_SUBTYPES: usize = 8;

/// CQ sub-types, `varicode.cpp:283`.
pub const CQ_SUBTYPES: &[&str] =
    &["CQ CQ CQ", "CQ DX", "CQ QRP", "CQ CONTEST", "CQ FIELD", "CQ FD", "CQ CQ", "CQ"];

/// A heartbeat or compound-callsign frame.
///
/// `[3 type][50 callsign][11 num_high]` then `[5 num_low][3 sub]` — the 16-bit
/// `num` is split across the 64/8 boundary rather than being kept whole, which
/// is easy to miss and produces a frame that is wrong only in its top bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compound {
    pub kind: Js8FrameType,
    /// The full compound callsign, in the 50-bit encoding.
    pub call: String,
    /// Grid for a heartbeat, or a command's extra data. Bit 15 marks a CQ.
    pub num: u16,
    /// Heartbeat or CQ sub-type, three bits.
    pub sub: u8,
}

impl Compound {
    /// A plain heartbeat carrying a grid.
    pub fn heartbeat(call: &str, grid: Option<&str>) -> Self {
        Compound {
            kind: Js8FrameType::Heartbeat,
            call: call.to_string(),
            num: grid.map_or(varicode::NO_GRID, varicode::pack_grid),
            sub: 0,
        }
    }

    /// A CQ. The high bit of `num` is what distinguishes it from a heartbeat.
    pub fn cq(call: &str, grid: Option<&str>, sub: u8) -> Self {
        let mut c = Self::heartbeat(call, grid);
        c.num |= 1 << 15;
        c.sub = sub & 0b111;
        c
    }

    /// True when this frame is a CQ rather than a heartbeat.
    pub fn is_cq(&self) -> bool {
        self.num & (1 << 15) != 0
    }

    /// The grid, if one was sent.
    pub fn grid(&self) -> Option<String> {
        varicode::unpack_grid(self.num & 0x7fff)
    }

    /// The frame that opens a message whose sender or addressee does not fit
    /// a directed frame: who is speaking, and from where.
    ///
    /// `packCompoundMessage` with a grid (`varicode.cpp:1394`), as
    /// `buildMessageFrames` sends it ahead of a [`Compound::directed`]
    /// (`varicode.cpp:2126`).
    pub fn ident(call: &str, grid: Option<&str>) -> Self {
        Compound {
            kind: Js8FrameType::Compound,
            call: call.to_string(),
            num: grid.map_or(varicode::NO_GRID, varicode::pack_grid),
            sub: 0,
        }
    }

    /// The addressee and command of such a message: `FrameCompoundDirected`.
    ///
    /// The addressee is the frame's 50-bit callsign. That is what lets it be
    /// a custom group like `@EMCOMM` or a call like `W1AW/4`, neither of which
    /// fits a directed frame's 28 bits. The command rides above
    /// [`varicode::NUSERGRID`] in the grid field (`varicode.cpp:1425`).
    pub fn directed(to: &str, cmd: i8, num: Option<i32>) -> Self {
        Compound {
            kind: Js8FrameType::CompoundDirected,
            call: to.to_string(),
            num: varicode::NUSERGRID + u16::from(pack_cmd(cmd, num)),
            sub: 0,
        }
    }

    /// The command and number a compound-directed frame carries.
    pub fn command(&self) -> Option<(i8, Option<i32>)> {
        if self.kind != Js8FrameType::CompoundDirected
            || !(varicode::NUSERGRID..varicode::NO_GRID).contains(&self.num)
        {
            return None;
        }
        let value = u8::try_from(self.num - varicode::NUSERGRID).ok()?;
        let (cmd, num) = unpack_cmd(value);
        cmd_text(cmd).map(|_| (cmd, num))
    }
}

/// A command and its number in eight bits, `packCmd` (`varicode.cpp:1166`).
///
/// The report commands set the top bit and put the report in the low six,
/// with bit 6 telling ` HEARTBEAT SNR` from ` SNR`. Every other command is
/// its code alone.
fn pack_cmd(cmd: i8, num: Option<i32>) -> u8 {
    if SNR_CMDS.contains(&cmd) {
        let hb = u8::from(cmd == 29);
        // Upstream packs an absent number as zero, which reads back as none.
        let n = num.map_or(0, varicode::pack_num);
        (((1 << 1) | hb) << 6) + (n & 0x3f)
    } else {
        (cmd as u8) & 0x7f
    }
}

/// Inverse of [`pack_cmd`], `unpackCmd` (`varicode.cpp:1188`).
fn unpack_cmd(value: u8) -> (i8, Option<i32>) {
    if value & 0x80 != 0 {
        let cmd = if value & 0x40 != 0 { 29 } else { 25 };
        let n = value & 0x3f;
        (cmd, (n != 0).then(|| varicode::unpack_num(n)))
    } else {
        ((value & 0x7f) as i8, None)
    }
}

impl Compound {
    pub fn pack(&self) -> Option<Js8Payload> {
        // Directed and data frames have their own layouts and cannot ride here.
        if matches!(self.kind, Js8FrameType::Directed | Js8FrameType::Data) {
            return None;
        }
        let packed_call = varicode::pack_alphanumeric50(&self.call);
        if packed_call == 0 {
            return None;
        }
        let num_high = (self.num & 0xffe0) >> 5;
        let num_low = (self.num & 0x1f) as u8;

        let mut bits = [0u8; 72];
        let mut at = 0usize;
        let put = |value: u64, width: usize, bits: &mut [u8; 72], at: &mut usize| {
            for i in 0..width {
                bits[*at + i] = ((value >> (width - 1 - i)) & 1) as u8;
            }
            *at += width;
        };
        put(self.kind as u64, 3, &mut bits, &mut at);
        put(packed_call, 50, &mut bits, &mut at);
        put(u64::from(num_high), 11, &mut bits, &mut at);
        put(u64::from(num_low), 5, &mut bits, &mut at);
        put(u64::from(self.sub & 0b111), 3, &mut bits, &mut at);
        debug_assert_eq!(at, 72);
        Some(Js8Payload { bits, frame_type: 0 })
    }

    pub fn unpack(payload: &Js8Payload) -> Option<Self> {
        let get = |start: usize, width: usize| -> u64 {
            payload.bits[start..start + width].iter().fold(0u64, |a, &b| (a << 1) | u64::from(b))
        };
        let kind = Js8FrameType::from_bits(get(0, 3) as u8)?;
        if matches!(
            kind,
            Js8FrameType::Directed | Js8FrameType::Data | Js8FrameType::DataCompressed
        ) {
            return None;
        }
        let num = ((get(53, 11) as u16) << 5) | (get(64, 5) as u16);
        Some(Compound {
            kind,
            call: varicode::unpack_alphanumeric50(get(3, 50)),
            num,
            sub: get(69, 3) as u8,
        })
    }
}

/// Free text as a data frame.
///
/// The layout is `[1 data][1 compressed][70 payload]`. Only the leading bit is
/// needed to say "data", because every other frame type begins with a zero —
/// so the two bits that would have spelled out `[100]` are reclaimed, one as
/// the compression flag and one for payload.
///
/// Both codecs are tried and the one that carries more characters wins, which
/// is what JS8Call does; picking only one would encode the same message
/// differently from every other station.
pub fn pack_data(text: &str) -> Option<(Js8Payload, usize)> {
    let text = text.to_ascii_uppercase();

    let mut huff_bits = vec![1u8, 0];
    let huff_used = super::huff::encode_into(&mut huff_bits, &text, super::huff::FRAME_BITS);

    let mut jsc_bits = vec![1u8, 1];
    let jsc_used = super::jsc::compress_into(&mut jsc_bits, &text, super::huff::FRAME_BITS);

    // Upstream's tie-break, and it is not the obvious one: compression wins
    // whenever it is merely *equal* (`varicode.cpp:1808` tests
    // `huffChars > compressedChars`). Choosing Huffman on a tie produces a
    // longer frame that still decodes, so the mistake shows up only as
    // frames that differ from every other station's.
    let (mut bits, used) =
        if huff_used > jsc_used { (huff_bits, huff_used) } else { (jsc_bits, jsc_used) };
    if used == 0 {
        return None;
    }
    super::huff::pad(&mut bits);

    let mut payload = [0u8; 72];
    payload.copy_from_slice(&bits);
    Some((Js8Payload { bits: payload, frame_type: 0 }, used))
}

/// Read a data frame back to text, or `None` if it is not one.
pub fn unpack_data(payload: &Js8Payload) -> Option<String> {
    if payload.bits[0] != 1 {
        return None;
    }
    let compressed = payload.bits[1] == 1;
    let body = super::huff::unpad(&payload.bits[..]);
    let body = &body[2.min(body.len())..];
    Some(if compressed { super::jsc::decompress(body) } else { super::huff::decode(body) })
}

/// Free text using the whole payload, for frames whose [`Js8Flags::DATA`] bit
/// says there is no type header.
///
/// Always dictionary-compressed: upstream compiles the Huffman branch out
/// (`JS8_FAST_DATA_CAN_USE_HUFF 0`, `varicode.cpp:1853`), so a receiver would
/// not know to look for it.
pub fn pack_fast_data(text: &str) -> Option<(Js8Payload, usize)> {
    let text = text.to_ascii_uppercase();
    let mut bits = Vec::new();
    let used = super::jsc::compress_into(&mut bits, &text, super::huff::FRAME_BITS);
    if used == 0 {
        return None;
    }
    super::huff::pad(&mut bits);
    let mut payload = [0u8; 72];
    payload.copy_from_slice(&bits);
    Some((Js8Payload { bits: payload, frame_type: 0 }, used))
}

/// Read a headerless data frame.
pub fn unpack_fast_data(payload: &Js8Payload) -> String {
    super::jsc::decompress(super::huff::unpad(&payload.bits[..]))
}

/// What a frame turned out to carry.
#[derive(Debug, Clone, PartialEq)]
pub enum Js8Content {
    /// A command addressed to a station or group.
    Directed(Directed),
    /// A heartbeat, CQ, or compound callsign.
    Compound(Compound),
    /// Free text, to be appended to whatever came before it.
    Text(String),
}

/// Interpret a decoded payload, given the sequencing flags that came with it.
///
/// The flags decide *whether* there is a type header at all, so they cannot be
/// ignored: reading the first three bits of a headerless data frame yields a
/// frame type that is really the first three bits of somebody's message.
pub fn interpret(payload: &Js8Payload, flags: Js8Flags) -> Option<Js8Content> {
    if flags.is_headerless_data() {
        let text = unpack_fast_data(payload);
        return (!text.is_empty()).then_some(Js8Content::Text(text));
    }
    let kind = Js8FrameType::from_bits(payload.bits[..3].iter().fold(0u8, |a, &b| (a << 1) | b))?;
    match kind {
        Js8FrameType::Directed => Directed::unpack(payload).map(Js8Content::Directed),
        Js8FrameType::Heartbeat | Js8FrameType::Compound | Js8FrameType::CompoundDirected => {
            Compound::unpack(payload).map(Js8Content::Compound)
        }
        Js8FrameType::Data | Js8FrameType::DataCompressed => {
            unpack_data(payload).filter(|t| !t.is_empty()).map(Js8Content::Text)
        }
    }
}
