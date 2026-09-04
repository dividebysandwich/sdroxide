//! VDL Mode 2 domain types, shared by the native engine, the wire protocol and
//! the UI (native + WASM). Pure data + serde — the demodulator, the link layer
//! and the ACARS/XID parsers live in the native `sdroxide-vdl2` crate.
//!
//! # Why a message log and not a table of aircraft
//!
//! ADS-B next door is a room full of transmitters repeating themselves, so
//! [`crate::AdsbStatus`] folds every squitter into one row per aeroplane and
//! re-sends the table. VDL Mode 2 is the opposite: it is a *conversation*. An
//! aircraft sends a position report once and never sends it again, a ground
//! station answers, a link is established, a clearance goes up. Folding that
//! into "latest state per station" would throw away the thing an operator came
//! to read.
//!
//! So the decoder keeps two views and sends both. [`Vdl2Message`] is the log,
//! newest last, bounded by [`Vdl2Settings::max_messages`] — what was said.
//! [`Vdl2Station`] is one row per 24-bit AVLC address — who is out there, which
//! a scrolling log makes surprisingly hard to see on a busy channel. The
//! station's address is the *same* 24-bit ICAO number
//! [`crate::AdsbAircraft::icao`] carries, so an aircraft heard on both bands is
//! recognisably one aircraft.
//!
//! # The channels
//!
//! Fourteen of them, 25 kHz apart, and the decoder listens to all it can reach
//! at once — one downconverter each inside a single receiver window, the way the
//! ISM decoder covers its channel plan. 136.975 MHz is the Common Signalling
//! Channel, the one frequency in use worldwide; the rest carry the traffic that
//! does not fit on it, and which of them are busy depends entirely on where the
//! aerial is.
//!
//! # Sources
//!
//! ETSI EN 301 841-1 (the VDL Mode 2 SARPs, published as ICAO Annex 10 Volume
//! III Part I Chapter 6) for the physical and link layers; ARINC 618 for the
//! ACARS message structure carried over them. Cited per module in
//! `sdroxide-vdl2`, where the arithmetic is.

use serde::{Deserialize, Serialize};

/// The VDL Mode 2 channels, ascending.
///
/// Every 25 kHz slot from 136.650 to 136.975 MHz. Not every other one: the
/// datalink allocation *is* the 25 kHz raster, and which slots carry traffic is
/// a regional matter — 136.700 to 136.975 is the twelve-channel European plan,
/// 136.650 is the North American on-airport channel, and 136.675 runs alongside
/// the European twelve until 2027. Skipping the slots between them was this
/// decoder's own mistake, reported from Brittany where the local ground station
/// is on 136.900 (issue #265).
///
/// Whole plan is 350 kHz wide including the outer half-channels, which is why
/// one ordinary receiver window still covers the lot.
///
/// Source: the European ICAO Frequency Management Manual (EUR Doc 011) for the
/// twelve; the Aeronautical Frequency Committee's USA VDL Mode 2 spectral plan
/// for 136.650.
pub const VDL2_CHANNELS_HZ: [f64; 14] = [
    136_650_000.0,
    136_675_000.0,
    136_700_000.0,
    136_725_000.0,
    136_750_000.0,
    136_775_000.0,
    136_800_000.0,
    136_825_000.0,
    136_850_000.0,
    136_875_000.0,
    136_900_000.0,
    136_925_000.0,
    136_950_000.0,
    136_975_000.0,
];

/// What each channel of [`VDL2_CHANNELS_HZ`] is assigned to, in the same order.
///
/// "Airspace" and "airport" say which kind of *ground station* the channel is
/// for — one serving an area from a remote ground station, or one serving the
/// aeroplanes on and around an aerodrome. Both carry aircraft too, so the label
/// is not a direction of travel. The published tables mark these AIR and GND,
/// which reads exactly backwards to an operator hearing an airport's ground
/// station on an "AIR" channel, so this spells it out instead.
///
/// Here rather than in the decoder because the panel says it and the panel also
/// runs in a browser, where the decoder does not.
pub const VDL2_CHANNEL_LABELS: [&str; 14] = [
    "airport station (USA)",
    "airspace station (to 2027)",
    "airspace station",
    "airspace station",
    "airspace station",
    "airspace station",
    "airspace station",
    "airport station",
    "airspace station",
    "airport station",
    "airspace station",
    "airport station",
    "airspace station",
    "common signalling channel",
];

/// The Common Signalling Channel — the one VDL2 frequency in use worldwide.
///
/// A receiver that can only reach one channel should be pointed here: every
/// ground station announces itself on it and every link starts on it.
pub const VDL2_CSC_HZ: f64 = 136_975_000.0;

/// Where the window wants to sit to reach the whole plan: the midpoint of the
/// outer channels' outer edges. Half a slot off the raster, because the plan
/// has an even number of channels in it.
pub const VDL2_PLAN_CENTER_HZ: f64 = 136_812_500.0;

/// How wide one VDL2 channel's slot is — the VHF air-ground raster, which is
/// also the spacing between the entries of [`VDL2_CHANNELS_HZ`]. Two channels
/// in use next to each other are 25 kHz apart with no guard slot between them,
/// which is what the receive filter has to earn its keep against.
pub const VDL2_CHANNEL_SPACING_HZ: f64 = 25_000.0;

/// Symbols a second. D8PSK, three bits each, so 31.5 kbit/s.
pub const VDL2_SYMBOL_RATE: f64 = 10_500.0;

/// The narrowest stream that holds the whole fourteen-channel plan.
///
/// 350 kHz of plan, divided by the fraction of a front end's span that is
/// usable — the outer edges of any receiver's window are where its own
/// anti-alias filter is already rolling off, and a channel sitting in the roll
/// off decodes badly or not at all. Same three-quarters figure the ISM plan
/// uses, for the same reason.
pub const VDL2_PLAN_RATE_HZ: f64 = 466_667.0;

/// The narrowest stream that holds one channel with its shoulders.
///
/// A D8PSK channel at α = 0.6 occupies 10500 × 1.6 = 16.8 kHz, and a window
/// this wide leaves room for it plus the front end's roll-off. Below this
/// nothing can be decoded and the honest answer is to say so.
pub const VDL2_MIN_RATE_HZ: f64 = 34_000.0;

/// Samples per symbol below which the timing estimate has too little to work
/// with. Not a refusal — a "this will do badly" sentence.
pub const VDL2_GOOD_SPS: f64 = 3.0;

/// Longest message log the decoder will keep, whatever the settings say.
pub const VDL2_MESSAGE_MAX: u16 = 2000;
/// Longest station table the decoder will keep, whatever the settings say.
pub const VDL2_STATION_MAX: u16 = 2000;

/// Default for [`Vdl2Settings::max_messages`].
pub const VDL2_MESSAGES: u16 = 500;
/// Default for [`Vdl2Settings::max_stations`].
pub const VDL2_STATIONS: u16 = 300;
/// Default for [`Vdl2Settings::drop_list_s`]. Ground stations beacon about once
/// a minute and an aircraft may say nothing for a long cruise, so half an hour
/// is deliberately generous — this is a "who is out there" list, not a radar.
pub const VDL2_DROP_LIST_S: u16 = 1800;
/// Default for [`Vdl2Settings::threshold_db`]: how far above the learned noise
/// floor a burst has to rise before the decoder looks at it.
pub const VDL2_THRESHOLD_DB: u8 = 9;
/// Every channel enabled — the fourteen low bits of
/// [`Vdl2Settings::channels`].
pub const VDL2_ALL_CHANNELS: u16 = 0x3fff;

/// What kind of station an AVLC address names.
///
/// Three bits of the 28-bit address field. Kept as an enum rather than reduced
/// to "aircraft or not" because the two ground-station kinds mean different
/// things — an administrative address is the ground station itself, a delegated
/// one is a service it speaks for — and because an operator reading a log wants
/// to know which end of a conversation they are looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Vdl2AddrKind {
    /// Type 1: an aircraft, addressed by its 24-bit ICAO number — the same
    /// number its ADS-B squitters carry.
    #[default]
    Aircraft,
    /// Type 4: a ground station, administrative address.
    GroundAdmin,
    /// Type 5: a ground station, delegated address.
    GroundDelegated,
    /// Type 7: all stations. What a ground station's broadcast beacon is
    /// addressed to.
    AllStations,
    /// Types 0, 2, 3 and 6, which the standard reserves. Kept rather than
    /// dropped: a frame arriving with one is either a decode error or something
    /// new, and silently calling it an aircraft would hide both.
    Reserved,
}

impl Vdl2AddrKind {
    /// The three address-type bits, as the standard numbers them.
    pub fn from_bits(bits: u8) -> Vdl2AddrKind {
        match bits & 0x7 {
            1 => Vdl2AddrKind::Aircraft,
            4 => Vdl2AddrKind::GroundAdmin,
            5 => Vdl2AddrKind::GroundDelegated,
            7 => Vdl2AddrKind::AllStations,
            _ => Vdl2AddrKind::Reserved,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Vdl2AddrKind::Aircraft => "aircraft",
            Vdl2AddrKind::GroundAdmin => "ground",
            Vdl2AddrKind::GroundDelegated => "ground (delegated)",
            Vdl2AddrKind::AllStations => "all stations",
            Vdl2AddrKind::Reserved => "reserved",
        }
    }

    /// Short form for a table column, where there is no room for the above.
    pub fn short(self) -> &'static str {
        match self {
            Vdl2AddrKind::Aircraft => "AIR",
            Vdl2AddrKind::GroundAdmin => "GND",
            Vdl2AddrKind::GroundDelegated => "GND*",
            Vdl2AddrKind::AllStations => "ALL",
            Vdl2AddrKind::Reserved => "?",
        }
    }

    pub fn is_ground(self) -> bool {
        matches!(self, Vdl2AddrKind::GroundAdmin | Vdl2AddrKind::GroundDelegated)
    }
}

/// The AVLC link control field, already read.
///
/// AVLC is HDLC's frame taxonomy with the modulo-128 half left out, so this is
/// the familiar three families: numbered information, supervisory flow control,
/// and unnumbered link management. Variants are appended only — postcard
/// numbers them by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vdl2Frame {
    /// Information, with send and receive sequence numbers and the poll bit.
    I { ns: u8, nr: u8, p: bool },
    /// Receive ready.
    Rr { nr: u8, pf: bool },
    /// Receive not ready.
    Rnr { nr: u8, pf: bool },
    /// Reject.
    Rej { nr: u8, pf: bool },
    /// Selective reject.
    Srej { nr: u8, pf: bool },
    /// Unnumbered information — what a broadcast rides in.
    Ui { pf: bool },
    /// Disconnected mode.
    Dm { pf: bool },
    /// Disconnect.
    Disc { pf: bool },
    /// Unnumbered acknowledgement.
    Ua { pf: bool },
    /// Frame reject.
    Frmr { pf: bool },
    /// Exchange identification — how VDL2 negotiates a link and how a ground
    /// station announces itself.
    Xid { pf: bool },
    /// Test.
    Test { pf: bool },
    /// A control byte the standard does not define. Kept whole so the log can
    /// show it rather than pretend the frame was something else.
    Unknown(u8),
}

impl Vdl2Frame {
    /// How the frame reads in a log line.
    pub fn label(self) -> String {
        match self {
            Vdl2Frame::I { ns, nr, p } => {
                format!("I {ns}/{nr}{}", if p { " P" } else { "" })
            }
            Vdl2Frame::Rr { nr, pf } => format!("RR {nr}{}", pf_suffix(pf)),
            Vdl2Frame::Rnr { nr, pf } => format!("RNR {nr}{}", pf_suffix(pf)),
            Vdl2Frame::Rej { nr, pf } => format!("REJ {nr}{}", pf_suffix(pf)),
            Vdl2Frame::Srej { nr, pf } => format!("SREJ {nr}{}", pf_suffix(pf)),
            Vdl2Frame::Ui { pf } => format!("UI{}", pf_suffix(pf)),
            Vdl2Frame::Dm { pf } => format!("DM{}", pf_suffix(pf)),
            Vdl2Frame::Disc { pf } => format!("DISC{}", pf_suffix(pf)),
            Vdl2Frame::Ua { pf } => format!("UA{}", pf_suffix(pf)),
            Vdl2Frame::Frmr { pf } => format!("FRMR{}", pf_suffix(pf)),
            Vdl2Frame::Xid { pf } => format!("XID{}", pf_suffix(pf)),
            Vdl2Frame::Test { pf } => format!("TEST{}", pf_suffix(pf)),
            Vdl2Frame::Unknown(b) => format!("?{b:02X}"),
        }
    }

    /// True for the frames that carry a payload worth parsing.
    pub fn carries_data(self) -> bool {
        matches!(self, Vdl2Frame::I { .. } | Vdl2Frame::Ui { .. } | Vdl2Frame::Xid { .. })
    }
}

fn pf_suffix(pf: bool) -> &'static str {
    if pf { " P/F" } else { "" }
}

/// An ACARS message carried over AVLC.
///
/// The fields are ARINC 618's, in the order they arrive. `msn` and `flight` are
/// present on downlinks (aircraft to ground) and empty on uplinks, which is how
/// the two are told apart without a separate flag.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Vdl2Acars {
    /// The mode character. `2` for most air-ground traffic.
    pub mode: char,
    /// Aircraft registration, seven characters with the leading pad trimmed —
    /// `.OE-LWA` arrives as `OE-LWA`.
    pub registration: String,
    /// The technical acknowledgement: the block id being acknowledged, or NAK
    /// when there is nothing to acknowledge.
    pub ack: char,
    /// Two-character label. `H1` is a free-text company message, `_d` a
    /// handshake, `5Z` a general request — the label is what tells an operator
    /// what kind of message they are looking at before reading a word of it.
    pub label: String,
    /// Block identification, one character.
    pub block_id: char,
    /// Message sequence number, four characters. Downlink only.
    pub msn: String,
    /// Flight identification, six characters. Downlink only.
    pub flight: String,
    /// The message text, parity stripped.
    pub text: String,
    /// The block ended ETB rather than ETX: this is one part of a longer
    /// message and more blocks follow.
    pub more: bool,
    /// The ACARS message's own CRC checked out.
    ///
    /// Reported and never used as a filter: by the time this is read the AVLC
    /// frame check sequence has already passed over the same bytes, so a
    /// disagreement says something about this decoder's understanding of the
    /// ACARS framing rather than about the radio path.
    pub crc_ok: bool,
    /// Characters whose odd parity bit did not agree. A quality figure for the
    /// same reason: the frame has already been checked as a whole.
    pub parity_errors: u16,
}

impl Vdl2Acars {
    /// A one-line summary for the log: the label, then whatever identifies the
    /// message.
    pub fn summary(&self) -> String {
        let who = if !self.flight.trim().is_empty() {
            self.flight.trim().to_string()
        } else {
            self.registration.trim().to_string()
        };
        let text = self.text.trim();
        let head = if who.is_empty() {
            format!("[{}]", self.label)
        } else {
            format!("[{}] {who}", self.label)
        };
        if text.is_empty() { head } else { format!("{head} {text}") }
    }
}

/// An XID exchange, decoded as far as this pass goes.
///
/// XID is how VDL2 establishes and hands over links, and how a ground station
/// broadcasts what it is and where. The parameters are a TLV list whose
/// vocabulary is large; what is decoded here is named, and what is not is
/// counted rather than dropped.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Vdl2Xid {
    /// What the exchange is: `GSIF` (a ground station broadcasting its
    /// identity), `CMD_LE` (link establishment), `RSP_LE`, `CMD_HO`
    /// (handoff), and so on, as the connection-management parameter says.
    pub kind: String,
    /// Everything decoded, as name/value text the panel prints as it stands.
    /// Held this way rather than as a struct per parameter because the set is
    /// open and an operator reading an XID wants to see what arrived, not a
    /// subset somebody chose in advance.
    pub params: Vec<(String, String)>,
    /// Where the station said it is, when it said.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Frequencies the ground station advertises, MHz — the alternate channels
    /// it will hand a link over to.
    pub frequencies: Vec<f64>,
    /// Parameters whose identifier this decoder does not know. Shown as a count
    /// so an operator can tell "nothing else was sent" from "there was more and
    /// sdroxide could not read it".
    pub unknown: u16,
}

/// What an AVLC frame was carrying.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Vdl2Payload {
    /// Nothing — a supervisory frame, or an information frame with an empty
    /// payload.
    #[default]
    None,
    Acars(Vdl2Acars),
    Xid(Box<Vdl2Xid>),
    /// X.25 (ISO 8208), CLNP (ISO 8473), ES-IS, IDRP and the ASN.1 applications
    /// above them — out of scope for this pass.
    ///
    /// Named as far as the first octets allow and shown as hex rather than
    /// dropped: an operator who sees "CLNP, 84 octets" between two addresses
    /// they recognise has learned something, and one who sees nothing has not.
    Other {
        note: String,
        hex: String,
    },
}

/// One decoded AVLC frame, as the log holds it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vdl2Message {
    /// Unix seconds when the burst was decoded.
    pub at: i64,
    /// Which channel it arrived on, absolute Hz.
    pub freq_hz: f64,
    /// The 24-bit source address and what kind of station it names.
    pub src: u32,
    pub src_kind: Vdl2AddrKind,
    pub dst: u32,
    pub dst_kind: Vdl2AddrKind,
    /// Command rather than response — the destination octets' command/response
    /// bit. Which way round a frame is going is not otherwise visible, because
    /// both ends use the same frame types.
    pub command: bool,
    pub frame: Vdl2Frame,
    pub payload: Vdl2Payload,
    /// Signal-to-noise of the burst, dB above the channel's learned floor.
    pub snr_db: f32,
    /// Level of the burst, dBFS — negative.
    pub rssi_dbfs: f32,
    /// Mean residual phase error after the symbol decision, degrees. A D8PSK
    /// decision sector is 45° wide, so anything over about 15° is a frame that
    /// only just arrived.
    pub evm_deg: f32,
    /// Carrier offset measured on this burst, Hz. All frames on a channel
    /// sitting at the same offset is the receiver's clock, not the aircraft's.
    pub freq_err_hz: f32,
    /// Reed–Solomon symbols the decoder had to fix. Zero on a clean frame; near
    /// the limit on every frame means the channel is at the edge.
    pub rs_corrected: u16,
    /// The AVLC frame as hex, addresses through payload. What identifies
    /// something the decoder only half understands.
    pub raw_hex: String,
}

impl Vdl2Message {
    pub fn src_hex(&self) -> String {
        format!("{:06X}", self.src & 0xff_ffff)
    }

    pub fn dst_hex(&self) -> String {
        format!("{:06X}", self.dst & 0xff_ffff)
    }

    /// The one line the log shows before anything is clicked.
    pub fn summary(&self) -> String {
        match &self.payload {
            Vdl2Payload::Acars(a) => a.summary(),
            Vdl2Payload::Xid(x) => {
                if x.kind.is_empty() {
                    "XID".to_string()
                } else {
                    format!("XID {}", x.kind)
                }
            }
            Vdl2Payload::Other { note, .. } => note.clone(),
            Vdl2Payload::None => self.frame.label(),
        }
    }
}

/// One station heard, keyed on its 24-bit AVLC address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vdl2Station {
    /// The 24-bit address. For an aircraft this is its ICAO number — the same
    /// one [`crate::AdsbAircraft::icao`] holds, so a station heard on both
    /// bands is recognisably the same aeroplane.
    pub addr: u32,
    pub kind: Vdl2AddrKind,
    /// Filled once an ACARS message reveals it. Empty is normal rather than
    /// broken: an aircraft exchanging link control has not said its
    /// registration and may not for minutes.
    pub registration: String,
    /// Likewise, from the flight identification field of a downlink.
    pub flight: String,
    /// Where a ground station said it is, from an XID location parameter.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Unix seconds first and last heard this session.
    pub first_at: i64,
    pub last_at: i64,
    /// Frames from this address this session.
    pub messages: u32,
    /// The channel it was last heard on.
    pub last_freq_hz: f64,
    /// Signal-to-noise of the last frame, dB.
    pub last_snr_db: f32,
    /// The last ACARS label, or the last frame type — something to read in a
    /// column that would otherwise be empty for a station that only ever sends
    /// supervisory frames.
    pub last_label: String,
}

impl Vdl2Station {
    pub fn new(addr: u32, kind: Vdl2AddrKind, now: i64) -> Vdl2Station {
        Vdl2Station {
            addr,
            kind,
            registration: String::new(),
            flight: String::new(),
            lat: None,
            lon: None,
            first_at: now,
            last_at: now,
            messages: 0,
            last_freq_hz: 0.0,
            last_snr_db: 0.0,
            last_label: String::new(),
        }
    }

    pub fn hex(&self) -> String {
        format!("{:06X}", self.addr & 0xff_ffff)
    }

    /// What to call it on screen: the flight, else the registration, else the
    /// address. Never empty.
    pub fn label(&self) -> String {
        let flight = self.flight.trim();
        if !flight.is_empty() {
            return flight.to_string();
        }
        let reg = self.registration.trim();
        if !reg.is_empty() {
            return reg.to_string();
        }
        self.hex()
    }
}

/// One channel of the plan, as the panel's channel strip draws it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Vdl2ChannelStatus {
    pub freq_hz: f64,
    /// A downconverter is open on it and it is being decoded.
    pub live: bool,
    /// Why it is not, when it is not: "outside the receiver's window" and
    /// "switched off" are two very different problems that produce the same
    /// empty column.
    pub reason: Option<String>,
    /// Bursts the gate opened on this channel, and frames that came out.
    pub bursts: u64,
    pub frames: u64,
    /// The learned noise floor, dBFS. The number the burst threshold is
    /// measured against, and the one that says whether a channel is being
    /// deafened by something next to it.
    pub floor_dbfs: f32,
}

/// Everything the decoder has, re-sent whole a couple of times a second.
///
/// One message rather than separate log and table updates, for the reason
/// [`crate::AdsbStatus`] is one: a snapshot carrying the messages without the
/// stations, or the counters without either, would be describing two different
/// instants.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Vdl2Status {
    /// Every channel of the plan, whether or not it is being decoded.
    pub channels: Vec<Vdl2ChannelStatus>,
    /// Every station still on the list, in no particular order — the panel
    /// sorts.
    pub stations: Vec<Vdl2Station>,
    /// The message log, oldest first, bounded by
    /// [`Vdl2Settings::max_messages`].
    pub messages: Vec<Vdl2Message>,
    /// Bursts the gates opened, across every live channel.
    pub bursts: u64,
    /// Bursts whose synchronisation word was found. A large `bursts` with no
    /// `syncs` means the channel is busy with something that is not VDL2.
    pub syncs: u64,
    /// Headers that passed their own error-correcting code, and those that did
    /// not. Syncs without headers is a decoder problem, not a propagation one.
    pub headers: u64,
    pub header_bad: u64,
    /// Reed–Solomon blocks the decoder could not repair, and symbols it did.
    ///
    /// On real traffic `rs_fail` is currently *every* block: the code's
    /// parameters as implemented do not fit what is on the air, and the frame
    /// check sequence is what admits a frame instead — see
    /// `sdroxide_vdl2::channel` and issue #265. So a large number here is
    /// expected and is not, on its own, evidence of a weak signal.
    pub rs_fail: u64,
    pub rs_corrected: u64,
    /// Data fields that did not unwrap as an HDLC frame: no flags, an abort, or
    /// a destuffed length that is not a whole number of octets. Headers being
    /// read while this climbs is a decoder problem one layer in, not a
    /// propagation one.
    pub hdlc_bad: u64,
    /// Frames the Reed–Solomon layer refused and the frame check sequence then
    /// accepted. Every frame, at the moment; the day it is not, the FEC has
    /// started earning its place.
    pub fec_bypassed: u64,
    /// Frames whose Reed–Solomon came out clean and whose frame check sequence
    /// then did not. The sharpest single signal that something above the FEC is
    /// wrong rather than the radio path.
    pub fcs_bad: u64,
    /// Frames delivered.
    pub frames: u64,
    /// Frames long enough to need more than one Reed–Solomon block, and how
    /// many of those decoded. The interleaving across blocks is the one part of
    /// this decoder that has never been checked against a real long frame, so
    /// the two are counted separately rather than hidden in the totals.
    pub multiblock: u64,
    pub multiblock_ok: u64,
    /// Where the decoder's own window is, and how wide, in Hz.
    pub window_center_hz: f64,
    pub window_rate_hz: f64,
    /// Samples per symbol the per-channel chains settled on. Under
    /// [`VDL2_GOOD_SPS`] the timing estimate is working with very little.
    pub samples_per_symbol: f32,
    /// Why nothing is running, when nothing is running. Filled by the engine,
    /// not by the decoder: the worker knows what it is decoding, and only the
    /// engine knows what the receiver could have been decoding.
    pub unavailable: Option<String>,
    /// Why the decoder will do badly here even though it is running — a window
    /// that reaches only some of the plan, or one so narrow that the symbol
    /// timing has nothing to work with. Also engine-filled.
    pub degraded: Option<String>,
    /// Where the operator would have to tune. `None` when the dial is already
    /// somewhere the decoder can work.
    pub suggest_center_hz: Option<f64>,
}

/// How the VDL2 decoder behaves. Persisted to `vdl2.json`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Vdl2Settings {
    /// One bit per entry of [`VDL2_CHANNELS_HZ`], ascending, low bit first.
    ///
    /// A mask rather than a list of frequencies because the plan is fixed: a
    /// channel is either one of the fourteen or it is not something this decoder
    /// knows how to listen to.
    ///
    /// Stored under a different name from the seven-channel mask that came
    /// before it. Every bit moved when the plan filled in the slots between the
    /// old channels, so a `vdl2.json` written by an older build would now read
    /// as a *different* set of channels rather than the same one — and switching
    /// half the plan off silently is worse than the one-off cost of an operator
    /// re-making their choice. An old file falls back to all channels on.
    #[serde(rename = "channel_mask")]
    pub channels: u16,
    /// How far above the learned noise floor a burst has to rise, dB.
    pub threshold_db: u8,
    /// A station with nothing heard from it for this long leaves the list.
    pub drop_list_s: u16,
    /// Ceilings, so a busy channel cannot grow the status message without
    /// bound. The oldest go first.
    pub max_stations: u16,
    pub max_messages: u16,
    /// Show frames whose payload is neither ACARS nor XID, as hex. On by
    /// default: they are real traffic between real stations, and an operator
    /// who cannot see them has no way to know how much is being left unread.
    pub show_other: bool,
}

impl Default for Vdl2Settings {
    fn default() -> Self {
        Vdl2Settings {
            channels: VDL2_ALL_CHANNELS,
            threshold_db: VDL2_THRESHOLD_DB,
            drop_list_s: VDL2_DROP_LIST_S,
            max_stations: VDL2_STATIONS,
            max_messages: VDL2_MESSAGES,
            show_other: true,
        }
    }
}

impl Vdl2Settings {
    /// The decoder switched off, for a front end that cannot run it.
    ///
    /// A separate value rather than `Default` with an empty mask, for the reason
    /// [`crate::AdsbSettings::OFF`] is one: the engine forces this into the live
    /// state on a source that hands over demodulated audio, and that must not be
    /// mistaken for what the operator chose.
    pub const OFF: Vdl2Settings = Vdl2Settings {
        channels: 0,
        threshold_db: VDL2_THRESHOLD_DB,
        drop_list_s: VDL2_DROP_LIST_S,
        max_stations: VDL2_STATIONS,
        max_messages: VDL2_MESSAGES,
        show_other: true,
    };

    /// The settings with every field inside the range the decoder can honour.
    ///
    /// Applied where they arrive rather than where they are used, for the same
    /// reason [`crate::AdsbSettings::sane`] is: they come from a config file an
    /// operator may have edited and from a remote client.
    pub fn sane(mut self) -> Vdl2Settings {
        self.channels &= VDL2_ALL_CHANNELS;
        self.threshold_db = self.threshold_db.clamp(3, 40);
        self.drop_list_s = self.drop_list_s.clamp(30, 21_600);
        self.max_stations = self.max_stations.clamp(10, VDL2_STATION_MAX);
        self.max_messages = self.max_messages.clamp(10, VDL2_MESSAGE_MAX);
        self
    }

    /// Whether channel `i` of [`VDL2_CHANNELS_HZ`] is switched on.
    pub fn channel_enabled(self, i: usize) -> bool {
        i < VDL2_CHANNELS_HZ.len() && self.channels & (1 << i) != 0
    }

    /// Any channel at all.
    pub fn any_enabled(self) -> bool {
        self.channels & VDL2_ALL_CHANNELS != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plan's centre really is the midpoint of the outer channels' outer
    /// edges, and the whole plan really does fit in the rate named for it.
    #[test]
    fn the_plan_centre_and_rate_describe_the_same_channels() {
        let lo = VDL2_CHANNELS_HZ[0] - VDL2_CHANNEL_SPACING_HZ / 2.0;
        let hi = VDL2_CHANNELS_HZ[VDL2_CHANNELS_HZ.len() - 1] + VDL2_CHANNEL_SPACING_HZ / 2.0;
        assert_eq!((lo + hi) / 2.0, VDL2_PLAN_CENTER_HZ);
        // Three quarters of the plan rate has to cover the span, or the outer
        // channels sit in the front end's roll-off.
        assert!(VDL2_PLAN_RATE_HZ * 0.75 >= hi - lo, "the plan does not fit its own rate");
    }

    /// The channels are the whole 25 kHz raster, ascending and with nothing
    /// skipped — the thing every window calculation assumes, and the thing that
    /// was wrong when a French operator's local ground station on 136.900 was
    /// nowhere in the panel (issue #265).
    #[test]
    fn the_channels_are_the_whole_raster() {
        for w in VDL2_CHANNELS_HZ.windows(2) {
            let gap = w[1] - w[0];
            assert_eq!(gap, VDL2_CHANNEL_SPACING_HZ, "{gap} is not one raster slot");
        }
        assert!(VDL2_CHANNELS_HZ.contains(&VDL2_CSC_HZ));
        // The CSC is the top of the plan, and every channel below it is a slot
        // of the same raster.
        assert_eq!(VDL2_CHANNELS_HZ[VDL2_CHANNELS_HZ.len() - 1], VDL2_CSC_HZ);
        // An even number of channels puts the centre half a slot off the
        // raster, which is legal for a window and would not be for a channel.
        assert_eq!(VDL2_CHANNELS_HZ.len() % 2, 0);
        assert!(!VDL2_CHANNELS_HZ.contains(&VDL2_PLAN_CENTER_HZ));
    }

    /// A `vdl2.json` from a build with the seven-channel plan in it does not
    /// come back as seven of the fourteen: the key it was written under is not
    /// the key this reads, so it falls back to every channel on.
    #[test]
    fn an_old_channel_mask_does_not_silently_mean_something_else() {
        let old: Vdl2Settings =
            serde_json::from_str(r#"{"channels":127,"threshold_db":11}"#).expect("parse");
        assert_eq!(old.channels, VDL2_ALL_CHANNELS);
        // ...while everything else in the file is still read.
        assert_eq!(old.threshold_db, 11);
    }

    /// Every channel of the plan has a bit, and no bit outside it survives a
    /// hand-edited config.
    #[test]
    fn the_channel_mask_covers_the_plan_and_nothing_else() {
        let all = Vdl2Settings::default();
        for i in 0..VDL2_CHANNELS_HZ.len() {
            assert!(all.channel_enabled(i), "channel {i} is off by default");
        }
        assert!(!all.channel_enabled(VDL2_CHANNELS_HZ.len()));
        let wild = Vdl2Settings { channels: 0xffff, ..Vdl2Settings::default() }.sane();
        assert_eq!(wild.channels, VDL2_ALL_CHANNELS);
        assert!(!Vdl2Settings::OFF.any_enabled());
    }

    /// A hand-edited config cannot ask for a log longer than the decoder keeps.
    #[test]
    fn the_ceilings_are_clamped() {
        let s = Vdl2Settings {
            max_messages: 60_000,
            max_stations: 60_000,
            threshold_db: 200,
            drop_list_s: 1,
            ..Vdl2Settings::default()
        }
        .sane();
        assert_eq!(s.max_messages, VDL2_MESSAGE_MAX);
        assert_eq!(s.max_stations, VDL2_STATION_MAX);
        assert_eq!(s.threshold_db, 40);
        assert_eq!(s.drop_list_s, 30);
    }

    /// A station always has something to call it, from the first frame.
    #[test]
    fn a_station_is_never_unlabelled() {
        let mut s = Vdl2Station::new(0x44_0F_31, Vdl2AddrKind::Aircraft, 0);
        assert_eq!(s.label(), "440F31");
        s.registration = "OE-LWA ".to_string();
        assert_eq!(s.label(), "OE-LWA");
        s.flight = "AUA123".to_string();
        assert_eq!(s.label(), "AUA123");
    }

    /// The address-type bits map to the kinds the standard names, and every
    /// other value is reserved rather than quietly an aircraft.
    #[test]
    fn reserved_address_types_are_not_aircraft() {
        assert_eq!(Vdl2AddrKind::from_bits(1), Vdl2AddrKind::Aircraft);
        assert_eq!(Vdl2AddrKind::from_bits(4), Vdl2AddrKind::GroundAdmin);
        assert_eq!(Vdl2AddrKind::from_bits(5), Vdl2AddrKind::GroundDelegated);
        assert_eq!(Vdl2AddrKind::from_bits(7), Vdl2AddrKind::AllStations);
        for bits in [0u8, 2, 3, 6] {
            assert_eq!(Vdl2AddrKind::from_bits(bits), Vdl2AddrKind::Reserved, "{bits}");
        }
    }
}
