//! Wire format for Icom's IP-remote protocol — the one RS-BA1 speaks, and the
//! one every LAN or WiFi Icom listens for on UDP 50001.
//!
//! Pure functions over byte slices: builders return a `Vec<u8>` ready to send,
//! parsers borrow a datagram and return `Option<T>`. Nothing here touches a
//! socket, so all of it is unit-testable without a radio.
//!
//! # Shape of the protocol
//!
//! Three UDP streams — control, CI-V and audio — each an independent instance
//! of the same reliability layer, and each with its own 16-bit sequence space.
//! Every packet opens with the same 16-byte header:
//!
//! ```text
//! 0x00  len     u32   total length including this header
//! 0x04  type    u16   see `ctrl` below; 0 on a payload packet
//! 0x06  seq     u16   sender's sequence, echoed in retransmit requests
//! 0x08  sentid  u32   sender's session id
//! 0x0C  rcvdid  u32   peer's session id, 0 before it is known
//! ```
//!
//! Everything in the header is little-endian. Fields *inside* the longer
//! packets are a mixture — Icom sends the negotiated sample rates, the port
//! numbers and the inner sequence counters big-endian while leaving the token
//! and the request ids little-endian — so each accessor states which it is
//! rather than leaving it to a rule that does not exist.
//!
//! # Discriminating a packet
//!
//! By length first, then by `type`. The fixed sizes below are unique, which is
//! what makes a switch on `len` sufficient — and why a payload packet is
//! recognised as "not one of these lengths" rather than by a type code.

use std::net::Ipv4Addr;

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

/// Every packet starts with this much header.
pub const HEADER: usize = 0x10;

pub const CONTROL_SIZE: usize = 0x10;
pub const WATCHDOG_SIZE: usize = 0x14;
pub const PING_SIZE: usize = 0x15;
pub const OPENCLOSE_SIZE: usize = 0x16;
pub const TOKEN_SIZE: usize = 0x40;
pub const STATUS_SIZE: usize = 0x50;
pub const LOGIN_RESPONSE_SIZE: usize = 0x60;
pub const LOGIN_SIZE: usize = 0x80;
pub const CONNINFO_SIZE: usize = 0x90;
pub const CAPABILITIES_SIZE: usize = 0x42;
pub const RADIO_CAP_SIZE: usize = 0x66;

/// Header length of a CI-V payload packet; the CI-V frame follows.
pub const CIV_HEADER: usize = 0x15;
/// Header length of an audio payload packet; PCM follows.
pub const AUDIO_HEADER: usize = 0x18;

/// Control packet `type` codes.
pub mod ctrl {
    /// Idle — also the acknowledgement carrying a sequence number.
    pub const IDLE: u16 = 0x00;
    /// Retransmit request. `len == 0x10` asks for the single sequence in the
    /// header; anything longer carries `(first, last)` u16 pairs from 0x10.
    pub const RETRANSMIT: u16 = 0x01;
    /// "Are you there?" — the first packet of any stream.
    pub const ARE_YOU_THERE: u16 = 0x03;
    /// "I am here" — the reply, and where we learn the peer's session id.
    pub const I_AM_HERE: u16 = 0x04;
    /// Disconnect.
    pub const DISCONNECT: u16 = 0x05;
    /// "Are you ready?" / "I am ready".
    pub const READY: u16 = 0x06;
    /// Ping, both request and reply (distinguished by the byte at 0x10).
    pub const PING: u16 = 0x07;
}

/// Timings, all in milliseconds. These are the radio's expectations, not
/// preferences: idle any longer than [`IDLE_PERIOD`] and the link is torn down
/// at the far end.
pub mod timing {
    pub const IDLE_PERIOD: u64 = 100;
    pub const PING_PERIOD: u64 = 500;
    pub const ARE_YOU_THERE_PERIOD: u64 = 500;
    pub const RETRANSMIT_PERIOD: u64 = 100;
    pub const TOKEN_RENEWAL: u64 = 60_000;
    /// Nothing heard for this long means the link is gone.
    pub const STALE_MS: u64 = 15_000;
    /// How long a *named* data port gets to answer its handshake probes. A
    /// radio that is really there answers the first one in milliseconds; a port
    /// that never answers is wedged — commonly by an earlier session the radio
    /// is still holding — and the session should fail with a reason rather than
    /// sit half-open behind a healthy control stream, so it can tear down
    /// (returning the token) and reconnect clean.
    ///
    /// Five seconds, not three: the probes go out every
    /// [`ARE_YOU_THERE_PERIOD`], so this is ten attempts, and a lossy WiFi link
    /// that drops the first few must not be mistaken for a wedged radio and
    /// bounced when it was about to come up (a field report of an IC-R8600 that
    /// "won't connect reliably every time"). A genuinely wedged port answers
    /// none of the ten and is still caught, two seconds later.
    pub const HANDSHAKE_MS: u64 = 5_000;
    /// How many sent packets to keep for retransmit.
    pub const TX_BUFFER: usize = 500;
    /// More missing than this is a broken network, not a lost packet — drop the
    /// backlog rather than melting the link with retransmit requests.
    pub const MAX_MISSING: usize = 50;
}

// ---------------------------------------------------------------------------
// Little scalar helpers
// ---------------------------------------------------------------------------

fn put_u16(buf: &mut [u8], at: usize, v: u16) {
    buf[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn put_u32(buf: &mut [u8], at: usize, v: u32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_u16_be(buf: &mut [u8], at: usize, v: u16) {
    buf[at..at + 2].copy_from_slice(&v.to_be_bytes());
}
fn put_u32_be(buf: &mut [u8], at: usize, v: u32) {
    buf[at..at + 4].copy_from_slice(&v.to_be_bytes());
}

pub fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
pub fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}
pub fn u16_be_at(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}
pub fn u32_be_at(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Copy a string into a fixed-width field, truncating rather than overflowing.
/// Icom's fields are NUL-padded, never NUL-terminated-then-garbage, so the
/// caller's buffer must already be zeroed — every builder here zeroes it.
fn put_str(buf: &mut [u8], at: usize, width: usize, s: &str) {
    let b = s.as_bytes();
    let n = b.len().min(width);
    buf[at..at + n].copy_from_slice(&b[..n]);
}

/// Read a NUL-padded fixed-width field back as a `String`, dropping anything
/// that is not valid UTF-8 — a radio name is cosmetic and never worth failing a
/// connection over.
pub fn read_str(b: &[u8], at: usize, width: usize) -> String {
    let end = (at + width).min(b.len());
    if at >= end {
        return String::new();
    }
    let field = &b[at..end];
    let n = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..n]).trim().to_string()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// The 16 bytes at the front of every packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub len: u32,
    pub kind: u16,
    pub seq: u16,
    pub sent_id: u32,
    pub rcvd_id: u32,
}

impl Header {
    pub fn parse(b: &[u8]) -> Option<Header> {
        if b.len() < HEADER {
            return None;
        }
        Some(Header {
            len: u32_at(b, 0),
            kind: u16_at(b, 4),
            seq: u16_at(b, 6),
            sent_id: u32_at(b, 8),
            rcvd_id: u32_at(b, 12),
        })
    }
}

/// Start a packet of `size` bytes with the header filled in.
fn packet(size: usize, kind: u16, sent_id: u32, rcvd_id: u32) -> Vec<u8> {
    let mut p = vec![0u8; size];
    put_u32(&mut p, 0, size as u32);
    put_u16(&mut p, 4, kind);
    put_u32(&mut p, 8, sent_id);
    put_u32(&mut p, 12, rcvd_id);
    p
}

/// Our session id, as the radio expects it to be derived: the low two octets of
/// our own address in the top half, our local port in the bottom.
///
/// It is an identifier, not an address — the radio never routes on it — but it
/// has to be stable for the life of a stream and distinct between the three
/// streams, and binding it to the port gives both for free.
pub fn session_id(local_ip: Ipv4Addr, local_port: u16) -> u32 {
    let o = local_ip.octets();
    (u32::from(o[2]) << 24) | (u32::from(o[3]) << 16) | u32::from(local_port)
}

// ---------------------------------------------------------------------------
// Fixed-size packets
// ---------------------------------------------------------------------------

/// A bare 16-byte control packet: idle, are-you-there, ready, disconnect, or a
/// single-sequence retransmit request.
pub fn control(kind: u16, seq: u16, sent_id: u32, rcvd_id: u32) -> Vec<u8> {
    let mut p = packet(CONTROL_SIZE, kind, sent_id, rcvd_id);
    put_u16(&mut p, 6, seq);
    p
}

/// A retransmit request naming more than one sequence. Each wanted sequence is
/// written as a `(first, last)` pair, which for a single missing packet means
/// the same number twice — the form the radio's own client uses.
pub fn retransmit_request(seqs: &[u16], sent_id: u32, rcvd_id: u32) -> Vec<u8> {
    let mut p = packet(CONTROL_SIZE + seqs.len() * 4, ctrl::RETRANSMIT, sent_id, rcvd_id);
    for (i, &s) in seqs.iter().enumerate() {
        put_u16(&mut p, HEADER + i * 4, s);
        put_u16(&mut p, HEADER + i * 4 + 2, s);
    }
    let n = p.len() as u32;
    put_u32(&mut p, 0, n);
    p
}

/// A ping. `reply` distinguishes our probe from our answer to theirs; `time` is
/// echoed verbatim on the way back, so the peer can measure its own round trip.
pub fn ping(seq: u16, reply: bool, time: u32, sent_id: u32, rcvd_id: u32) -> Vec<u8> {
    let mut p = packet(PING_SIZE, ctrl::PING, sent_id, rcvd_id);
    put_u16(&mut p, 6, seq);
    p[0x10] = u8::from(reply);
    put_u32(&mut p, 0x11, time);
    p
}

/// The parsed body of a ping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ping {
    pub seq: u16,
    pub reply: bool,
    pub time: u32,
}

pub fn parse_ping(b: &[u8]) -> Option<Ping> {
    let h = Header::parse(b)?;
    if b.len() != PING_SIZE || h.kind != ctrl::PING {
        return None;
    }
    Some(Ping { seq: h.seq, reply: b[0x10] != 0, time: u32_at(b, 0x11) })
}

/// Open (or close) a data stream. Sent on the CI-V and audio sockets once their
/// control handshake is done; without it the radio connects but stays silent.
pub fn open_close(open: bool, inner_seq: u16, sent_id: u32, rcvd_id: u32) -> Vec<u8> {
    let mut p = packet(OPENCLOSE_SIZE, 0, sent_id, rcvd_id);
    put_u16(&mut p, 0x10, 0x01c0);
    put_u16_be(&mut p, 0x13, inner_seq);
    p[0x15] = if open { 0x04 } else { 0x00 };
    p
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

/// Icom's username/password obfuscation: a fixed substitution over the
/// printable ASCII range, with the character's own position folded in so a
/// repeated letter does not encode to a repeated byte.
///
/// This is obfuscation and not encryption — it is reversible without a key and
/// offers no secrecy. It is implemented because the radio requires it, and the
/// credential travels in clear over the LAN either way.
pub fn passcode(s: &str) -> [u8; 16] {
    const TABLE: [u8; 159] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x47, 0x5d, 0x4c, 0x42, 0x66, 0x20, 0x23, 0x46, 0x4e, 0x57, 0x45, 0x3d, 0x67,
        0x76, 0x60, 0x41, 0x62, 0x39, 0x59, 0x2d, 0x68, 0x7e, 0x7c, 0x65, 0x7d, 0x49, 0x29, 0x72,
        0x73, 0x78, 0x21, 0x6e, 0x5a, 0x5e, 0x4a, 0x3e, 0x71, 0x2c, 0x2a, 0x54, 0x3c, 0x3a, 0x63,
        0x4f, 0x43, 0x75, 0x27, 0x79, 0x5b, 0x35, 0x70, 0x48, 0x6b, 0x56, 0x6f, 0x34, 0x32, 0x6c,
        0x30, 0x61, 0x6d, 0x7b, 0x2f, 0x4b, 0x64, 0x38, 0x2b, 0x2e, 0x50, 0x40, 0x3f, 0x55, 0x33,
        0x37, 0x25, 0x77, 0x24, 0x26, 0x74, 0x6a, 0x28, 0x53, 0x4d, 0x69, 0x22, 0x5c, 0x44, 0x31,
        0x36, 0x58, 0x3b, 0x7a, 0x51, 0x5f, 0x52, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let mut out = [0u8; 16];
    for (i, &c) in s.as_bytes().iter().take(16).enumerate() {
        // Position is folded into the character, so a repeated letter does not
        // encode to a repeated byte. The fold can land as high as 158, which is
        // why the table runs past the printable range rather than stopping at
        // 127 — the tail is zeroes, and Icom's own client indexes into them.
        let mut p = usize::from(c) + i;
        if p > 126 {
            p = 32 + p % 127;
        }
        out[i] = TABLE[p];
    }
    out
}

/// The login packet: obfuscated credentials plus the name we want to appear
/// under in the radio's connection list.
#[allow(clippy::too_many_arguments)]
pub fn login(
    username: &str,
    password: &str,
    client_name: &str,
    inner_seq: u16,
    tok_request: u16,
    sent_id: u32,
    rcvd_id: u32,
) -> Vec<u8> {
    let mut p = packet(LOGIN_SIZE, 0, sent_id, rcvd_id);
    put_u32_be(&mut p, 0x10, (LOGIN_SIZE - HEADER) as u32);
    p[0x14] = 0x01; // request
    p[0x15] = 0x00; // login
    put_u16_be(&mut p, 0x16, inner_seq);
    put_u16(&mut p, 0x1a, tok_request);
    p[0x40..0x50].copy_from_slice(&passcode(username));
    p[0x50..0x60].copy_from_slice(&passcode(password));
    put_str(&mut p, 0x60, 16, client_name);
    p
}

/// What the radio said about our login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginResponse {
    pub token_request: u16,
    pub token: u32,
    pub error: u32,
    /// Free-text link description ("FTTH", …). Only wfview's own server ever
    /// says `WFVIEW`, which is how a real radio is told apart from a relay.
    pub connection: String,
}

impl LoginResponse {
    /// The radio's way of saying the username or password was wrong.
    pub const BAD_CREDENTIALS: u32 = 0xfeff_ffff;

    pub fn parse(b: &[u8]) -> Option<LoginResponse> {
        let h = Header::parse(b)?;
        if b.len() != LOGIN_RESPONSE_SIZE || h.kind == 0x01 {
            return None;
        }
        Some(LoginResponse {
            token_request: u16_at(b, 0x1a),
            token: u32_at(b, 0x1c),
            error: u32_at(b, 0x30),
            connection: read_str(b, 0x40, 16),
        })
    }
}

/// Token request/renewal. `magic` is the sub-request: `0x02` claims the token
/// the login handed us, `0x05` renews it, `0x01` gives it back.
pub fn token(
    magic: u8,
    inner_seq: u16,
    tok_request: u16,
    token: u32,
    sent_id: u32,
    rcvd_id: u32,
) -> Vec<u8> {
    let mut p = packet(TOKEN_SIZE, 0, sent_id, rcvd_id);
    put_u32_be(&mut p, 0x10, (TOKEN_SIZE - HEADER) as u32);
    p[0x14] = 0x01;
    p[0x15] = magic;
    put_u16_be(&mut p, 0x16, inner_seq);
    put_u16(&mut p, 0x1a, tok_request);
    put_u32(&mut p, 0x1c, token);
    put_u16_be(&mut p, 0x24, 0x0798);
    p
}

/// A reply to one of our token requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenReply {
    pub request_type: u8,
    pub request_reply: u8,
    pub token_request: u16,
    pub token: u32,
    pub response: u32,
}

impl TokenReply {
    /// The radio refusing a renewal — the session must be rebuilt.
    pub const REJECTED: u32 = 0xffff_ffff;

    pub fn parse(b: &[u8]) -> Option<TokenReply> {
        let h = Header::parse(b)?;
        if b.len() != TOKEN_SIZE || h.kind == 0x01 {
            return None;
        }
        Some(TokenReply {
            request_reply: b[0x14],
            request_type: b[0x15],
            token_request: u16_at(b, 0x1a),
            token: u32_at(b, 0x1c),
            response: u32_at(b, 0x30),
        })
    }
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

/// One radio as the far end describes it. A LAN transceiver reports exactly
/// one; an RS-BA1 server PC can front several.
///
/// This is where the CI-V address comes from, which is why the backend needs no
/// per-model configuration to talk to a rig it has never seen: the radio names
/// itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioCap {
    pub name: String,
    pub audio: String,
    /// CI-V address — `0xB6` on an IC-7300MK2, `0xA4` on an IC-705, …
    pub civ_address: u8,
    /// Bit set of offered receive sample rates, as the radio encodes it.
    pub rx_sample: u16,
    /// Same for transmit. `<= 1` means the far end will not accept TX audio.
    pub tx_sample: u16,
    pub baud_rate: u32,
    pub common_cap: u16,
    pub mac: [u8; 6],
    pub guid: [u8; 16],
}

impl RadioCap {
    /// `0x8010` means "identify me by MAC"; anything else means the 16-byte
    /// GUID at the same offset is the identifier.
    pub const CAP_MAC: u16 = 0x8010;

    pub fn uses_mac(&self) -> bool {
        self.common_cap == RadioCap::CAP_MAC
    }

    /// Whether the radio offered a transmit stream — the wire fact, verbatim.
    ///
    /// What the stream request is built from, and only that: a session has to
    /// ask for what the radio advertised, whether or not the radio makes sense.
    /// Whether there is a *transmitter* on the other end is
    /// [`Self::has_transmitter`], which is a different question.
    pub fn can_transmit(&self) -> bool {
        self.tx_sample > 1
    }

    /// Whether this radio can actually put something on the air.
    ///
    /// Not the same as [`Self::can_transmit`], and the difference is not
    /// theoretical: a field report from an IC-R8600 came back with the transmit
    /// controls offered and without the "receive only" note, so its capability
    /// block advertises a transmit stream on a set that has no transmitter in
    /// it. The negotiation still asks for exactly what the radio offered — a
    /// session that works must not be changed over this — but nothing above it
    /// should show a PTT, an SWR meter or a drive control.
    ///
    /// Nothing is guessed about the protocol here. `IC-R…` is Icom's own naming
    /// for its receivers, and a receiver has nothing to modulate however it
    /// fills this field.
    pub fn has_transmitter(&self) -> bool {
        self.can_transmit() && !is_receiver(&self.name)
    }

    fn parse(b: &[u8]) -> Option<RadioCap> {
        if b.len() < RADIO_CAP_SIZE {
            return None;
        }
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&b[0x0a..0x10]);
        let mut guid = [0u8; 16];
        guid.copy_from_slice(&b[0x00..0x10]);
        Some(RadioCap {
            name: read_str(b, 0x10, 32),
            audio: read_str(b, 0x30, 32),
            civ_address: b[0x52],
            rx_sample: u16_be_at(b, 0x53),
            tx_sample: u16_be_at(b, 0x55),
            baud_rate: u32_be_at(b, 0x5a),
            common_cap: u16_be_at(b, 0x07),
            mac,
            guid,
        })
    }
}

/// Parse a capabilities packet and the radio descriptions trailing it.
///
/// Returns `None` when the length is not a capabilities header plus a whole
/// number of radios — which on this protocol is how an oversized retransmit
/// request is told apart from a capabilities reply, since neither has a type
/// code of its own.
pub fn parse_capabilities(b: &[u8]) -> Option<Vec<RadioCap>> {
    if b.len() < CAPABILITIES_SIZE || !(b.len() - CAPABILITIES_SIZE).is_multiple_of(RADIO_CAP_SIZE)
    {
        return None;
    }
    let mut out = Vec::new();
    let mut at = CAPABILITIES_SIZE;
    while at + RADIO_CAP_SIZE <= b.len() {
        out.push(RadioCap::parse(&b[at..at + RADIO_CAP_SIZE])?);
        at += RADIO_CAP_SIZE;
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Stream request and status
// ---------------------------------------------------------------------------

/// Audio format, as the byte the radio negotiates on.
///
/// Only [`AudioCodec::Lpcm16Mono`] is ever selected. Icom transceivers reject
/// Opus outright — the reference client forces LPCM the moment it sees a
/// connection type other than its own server's — and the compressed formats
/// buy nothing on a LAN where 48 kHz mono is 768 kbit/s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AudioCodec {
    ULaw8Mono = 1,
    Lpcm8Mono = 2,
    Lpcm16Mono = 4,
    Lpcm8Stereo = 8,
    Lpcm16Stereo = 16,
    ULaw8Stereo = 32,
}

impl AudioCodec {
    pub fn byte(self) -> u8 {
        self as u8
    }
    /// Bytes per frame on the wire.
    pub fn frame_bytes(self) -> usize {
        match self {
            AudioCodec::ULaw8Mono | AudioCodec::Lpcm8Mono => 1,
            AudioCodec::Lpcm16Mono | AudioCodec::Lpcm8Stereo | AudioCodec::ULaw8Stereo => 2,
            AudioCodec::Lpcm16Stereo => 4,
        }
    }
}

/// Everything the stream request needs that the radio told us first.
#[derive(Debug, Clone)]
pub struct StreamRequest<'a> {
    pub radio: &'a RadioCap,
    pub username: &'a str,
    pub rx_codec: AudioCodec,
    /// `None` when the radio cannot receive audio from us.
    pub tx_codec: Option<AudioCodec>,
    pub rx_sample_rate: u32,
    pub tx_sample_rate: u32,
    pub civ_local_port: u16,
    pub audio_local_port: u16,
    pub tx_buffer_ms: u32,
    pub inner_seq: u16,
    pub token_request: u16,
    pub token: u32,
    pub sent_id: u32,
    pub rcvd_id: u32,
}

/// Ask the radio to open the CI-V and audio streams, telling it which local
/// ports to send them to. The radio answers with a status packet naming *its*
/// ports — the two halves cannot be learned in one exchange, which is why the
/// local ports have to be bound before this is sent.
pub fn stream_request(r: &StreamRequest<'_>) -> Vec<u8> {
    let mut p = packet(CONNINFO_SIZE, 0, r.sent_id, r.rcvd_id);
    put_u32_be(&mut p, 0x10, (CONNINFO_SIZE - HEADER) as u32);
    p[0x14] = 0x01;
    p[0x15] = 0x03; // stream request
    put_u16_be(&mut p, 0x16, r.inner_seq);
    put_u16(&mut p, 0x1a, r.token_request);
    put_u32(&mut p, 0x1c, r.token);
    if r.radio.uses_mac() {
        put_u16_be(&mut p, 0x27, RadioCap::CAP_MAC);
        p[0x2a..0x30].copy_from_slice(&r.radio.mac);
    } else {
        p[0x20..0x30].copy_from_slice(&r.radio.guid);
    }
    put_str(&mut p, 0x40, 32, &r.radio.name);
    p[0x60..0x70].copy_from_slice(&passcode(r.username));
    p[0x70] = 1; // rx enable
    if let Some(tx) = r.tx_codec {
        p[0x71] = 1;
        p[0x73] = tx.byte();
    }
    p[0x72] = r.rx_codec.byte();
    put_u32_be(&mut p, 0x74, r.rx_sample_rate);
    put_u32_be(&mut p, 0x78, if r.tx_codec.is_some() { r.tx_sample_rate } else { 0 });
    put_u32_be(&mut p, 0x7c, u32::from(r.civ_local_port));
    put_u32_be(&mut p, 0x80, u32::from(r.audio_local_port));
    put_u32_be(&mut p, 0x84, r.tx_buffer_ms);
    p[0x88] = 1; // convert
    p
}

/// The radio's answer to a stream request, and its running status thereafter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub error: u32,
    /// Set when the radio is telling us it dropped the connection.
    pub disconnected: bool,
    pub civ_port: u16,
    pub audio_port: u16,
}

impl Status {
    /// "I could not do that" — in practice a radio that needs a power cycle,
    /// or one already streaming to somebody else.
    pub const FAILED: u32 = 0xffff_ffff;

    pub fn parse(b: &[u8]) -> Option<Status> {
        let h = Header::parse(b)?;
        if b.len() != STATUS_SIZE || h.kind == 0x01 {
            return None;
        }
        Some(Status {
            error: u32_at(b, 0x30),
            disconnected: b[0x40] == 0x01,
            civ_port: u16_be_at(b, 0x42),
            audio_port: u16_be_at(b, 0x46),
        })
    }
}

/// A connection-info broadcast: who is using the radio, if anyone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnInfo {
    pub name: String,
    pub busy: bool,
    pub computer: String,
}

impl ConnInfo {
    pub fn parse(b: &[u8]) -> Option<ConnInfo> {
        let h = Header::parse(b)?;
        if b.len() != CONNINFO_SIZE || h.kind == 0x01 {
            return None;
        }
        Some(ConnInfo {
            name: read_str(b, 0x40, 32),
            busy: u32_at(b, 0x60) != 0,
            computer: read_str(b, 0x64, 16),
        })
    }
}

// ---------------------------------------------------------------------------
// Payload streams
// ---------------------------------------------------------------------------

/// Wrap a CI-V frame for the CI-V stream.
pub fn civ_data(frame: &[u8], inner_seq: u16, sent_id: u32, rcvd_id: u32) -> Vec<u8> {
    let mut p = packet(CIV_HEADER + frame.len(), 0, sent_id, rcvd_id);
    p[0x10] = 0xc1;
    put_u16(&mut p, 0x11, frame.len() as u16);
    put_u16_be(&mut p, 0x13, inner_seq);
    p[CIV_HEADER..].copy_from_slice(frame);
    p
}

/// Shortest possible CI-V frame: `FE FE <to> <from> <cmd> FD`.
const CIV_MIN_FRAME: usize = 6;

/// Pull the CI-V frame out of a CI-V stream packet.
///
/// Three things are checked, and all three earn their place:
///
/// * the inner length agrees with the datagram — a truncated frame handed to
///   the CI-V parser reads as junk between two good frames, not as an error;
/// * the payload is long enough to be a frame at all;
/// * it starts `FE FE`. Without that last check an *open/close* packet is
///   accepted as CI-V: it is 0x16 bytes, its type is 0, and the two bytes at
///   0x11 happen to read as an inner length of 1, which is self-consistent.
///   Every real CI-V frame begins with the preamble, so requiring it separates
///   the two at no cost.
pub fn parse_civ_data(b: &[u8]) -> Option<&[u8]> {
    let h = Header::parse(b)?;
    if b.len() <= CIV_HEADER || h.kind == 0x01 {
        return None;
    }
    let data_len = usize::from(u16_at(b, 0x11));
    if data_len < CIV_MIN_FRAME
        || data_len + CIV_HEADER != h.len as usize
        || b.len() < CIV_HEADER + data_len
    {
        return None;
    }
    let frame = &b[CIV_HEADER..CIV_HEADER + data_len];
    (frame[0] == 0xfe && frame[1] == 0xfe).then_some(frame)
}

/// The largest audio payload that fits one datagram without fragmenting.
pub const AUDIO_CHUNK: usize = 1364;

/// Wrap PCM for the audio stream.
pub fn audio_data(pcm: &[u8], inner_seq: u16, sent_id: u32, rcvd_id: u32) -> Vec<u8> {
    let mut p = packet(AUDIO_HEADER + pcm.len(), 0, sent_id, rcvd_id);
    // The reference client sends 0x9781 for one specific chunk size and 0x0080
    // otherwise; radios accept either, so use the general one throughout.
    put_u16(&mut p, 0x10, 0x0080);
    put_u16_be(&mut p, 0x12, inner_seq);
    put_u16_be(&mut p, 0x16, pcm.len() as u16);
    p[AUDIO_HEADER..].copy_from_slice(pcm);
    p
}

/// Pull PCM out of an audio stream packet.
pub fn parse_audio_data(b: &[u8]) -> Option<&[u8]> {
    let h = Header::parse(b)?;
    if b.len() <= AUDIO_HEADER || h.kind == 0x01 || h.len < 0x20 {
        return None;
    }
    Some(&b[AUDIO_HEADER..])
}

/// Decode a PCM payload to mono `f32` in ±1.
///
/// Stereo formats are folded to mono by taking the left channel: on every Icom
/// that offers stereo the two channels are main and sub receiver, and this
/// backend drives one receiver.
pub fn decode_pcm(codec: AudioCodec, payload: &[u8], out: &mut Vec<f32>) {
    match codec {
        AudioCodec::Lpcm16Mono => {
            for c in payload.chunks_exact(2) {
                out.push(f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0);
            }
        }
        AudioCodec::Lpcm16Stereo => {
            for c in payload.chunks_exact(4) {
                out.push(f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0);
            }
        }
        AudioCodec::Lpcm8Mono => {
            for &b in payload {
                out.push((f32::from(b) - 128.0) / 128.0);
            }
        }
        AudioCodec::Lpcm8Stereo => {
            for c in payload.chunks_exact(2) {
                out.push((f32::from(c[0]) - 128.0) / 128.0);
            }
        }
        AudioCodec::ULaw8Mono => {
            for &b in payload {
                out.push(ulaw_to_f32(b));
            }
        }
        AudioCodec::ULaw8Stereo => {
            for c in payload.chunks_exact(2) {
                out.push(ulaw_to_f32(c[0]));
            }
        }
    }
}

/// Encode mono `f32` for transmit. Only the mono formats can be sent — the
/// radio has one modulator.
pub fn encode_pcm(codec: AudioCodec, samples: &[f32], out: &mut Vec<u8>) {
    match codec {
        AudioCodec::Lpcm16Mono | AudioCodec::Lpcm16Stereo => {
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        AudioCodec::Lpcm8Mono | AudioCodec::Lpcm8Stereo => {
            for &s in samples {
                out.push(((s.clamp(-1.0, 1.0) * 127.0) + 128.0) as u8);
            }
        }
        AudioCodec::ULaw8Mono | AudioCodec::ULaw8Stereo => {
            for &s in samples {
                out.push(f32_to_ulaw(s));
            }
        }
    }
}

fn ulaw_to_f32(b: u8) -> f32 {
    let u = !b;
    let sign = u & 0x80;
    let exponent = (u >> 4) & 0x07;
    let mantissa = u & 0x0f;
    let mut v = (i32::from(mantissa) << 3) + 0x84;
    v <<= exponent;
    v -= 0x84;
    let v = if sign != 0 { -v } else { v };
    v as f32 / 32768.0
}

fn f32_to_ulaw(s: f32) -> u8 {
    const BIAS: i32 = 0x84;
    const CLIP: i32 = 32635;
    let mut v = (s.clamp(-1.0, 1.0) * 32767.0) as i32;
    let sign = if v < 0 {
        v = -v;
        0x80u8
    } else {
        0
    };
    let v = v.min(CLIP) + BIAS;
    let mut exponent = 7u8;
    let mut mask = 0x4000;
    while exponent > 0 && v & mask == 0 {
        exponent -= 1;
        mask >>= 1;
    }
    let mantissa = ((v >> (i32::from(exponent) + 3)) & 0x0f) as u8;
    !(sign | (exponent << 4) | mantissa)
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

/// The handful of facts that are *not* common across the LAN Icoms.
///
/// Frequency, mode, PTT, meters, the CW keyer and the scope all use the same
/// CI-V commands on every model, so they need no entry here. The `1A 05` menu
/// items do: Icom renumbers them between models, and the IC-7300MK2 moved
/// several relative to the original IC-7300. Writing the wrong `1A 05` index is
/// not a harmless no-op — the numbers around the modulation-input block include
/// the calibration marker — so an unknown model gets `None` and the operator is
/// asked to set the menu item on the radio instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    pub civ_address: u8,
    pub name: &'static str,
    /// Bins in one `27 00` scope sweep.
    pub scope_bins: usize,
    /// Top of the `27 00` amplitude scale. `160` on the IC-7300 generation,
    /// `200` on the IC-7760 — the sweep is finished magnitude bins with no
    /// documented dB per step, so this is only what "full scale" means.
    pub scope_full_scale: u8,
    /// Every `1A 05 nn nn` sub-command that picks a modulation source, and the
    /// value that means LAN on this model.
    ///
    /// One item per DATA slot the radio has, DATA-OFF first: two on an
    /// IC-7300MK2 or an IC-705, four on an IC-7760, which carries DATA1
    /// through DATA3. All of them are written, because the operator may reach
    /// any of them from the front panel and a slot still pointing at the
    /// microphone transmits silence.
    pub lan_mod_input: Option<(&'static [u16], u8)>,
    /// `1A 05 nn nn` sub-command for LAN AF/IF Output → Output Select.
    pub lan_afif_select: Option<u16>,
    /// `1A nn` sub-command carrying the DATA switch — the mode's `-D` variant.
    ///
    /// `0x06` on everything modern; the IC-7200's is `0x04` and an IC-7000 has
    /// none, but neither has a network port so neither reaches this backend.
    ///
    /// It matters because the plain mode command *clears* the switch: a rig
    /// left in FM-D by hand drops back to FM the moment anything sets the
    /// mode, which is before every over. Without the switch a digital over
    /// goes out through the microphone path with the rig's speech processing
    /// and — on FM — its pre-emphasis, which tilts a Bell 202 tone pair about
    /// 6 dB and makes packet unreadable while still sounding like packet.
    pub data_mode_sub: Option<u8>,
    /// Whether this model's 12 kHz IF output comes out mirrored about its
    /// centre, so the mix down to baseband has to run the other way.
    ///
    /// The IF is a *real* signal at 12 kHz, which means it carries the wanted
    /// spectrum at +12 kHz and its own mirror at -12 kHz. Which of the two the
    /// mixer lands on DC decides which way up the result is, and getting it
    /// wrong is not a subtle defect: SSB comes out on the opposite sideband to
    /// the one it was sent on, so the operator has to select USB where the band
    /// runs LSB to make anybody intelligible.
    ///
    /// Icom built this output for DRM and Dream reads it with its
    /// `--flipspectrum` off, so `false` is the convention to expect and what
    /// every model here is given until a radio says otherwise. The IC-7760 is
    /// the one that has: a field report of exactly the sideband swap above,
    /// against a session on the 12 kHz IF over the RF deck's LAN port. Icom
    /// documents no such difference — neither the CI-V reference guide nor the
    /// I/Q output supplement says which way either stream runs — so this is the
    /// radio's behaviour as reported, not a fact off a page, and
    /// `IcomNetConfig::invert_if` is there to overrule it in either direction.
    ///
    /// Deliberately not extended to the IC-7610, which shares the IC-7760's
    /// 689-bin scope and might well share this too: nobody has reported it, and
    /// guessing here would break a radio that works today.
    pub if_inverted: bool,
    /// How many aerial sockets this model's `0x12` selector switches between.
    ///
    /// Mirrors `sdroxide_types::IcomModel::antenna_sockets`, which carries the
    /// same fact for the serial CAT path and explains why it cannot simply be
    /// asked. `1` means the command is not a selector here at all: on an
    /// IC-7300MK2 it switches the receiving antenna in and out, and nothing
    /// else.
    pub antenna_sockets: usize,
}

/// Everything a model we do not recognise still gets: the common command set,
/// a 475-bin scope, and no menu writes.
pub const UNKNOWN_MODEL: Model = Model {
    civ_address: 0,
    name: "Icom (LAN)",
    scope_bins: 475,
    scope_full_scale: 160,
    lan_mod_input: None,
    lan_afif_select: None,
    // The one field an unrecognised model still gets a value for. `1A 06` is
    // the DATA switch on every Icom with a network port, and a rig that does
    // not have the sub-command answers NG and is left as it was — a cheaper
    // failure than a digital mode that quietly transmits through the
    // microphone path.
    data_mode_sub: Some(0x06),
    if_inverted: false,
    antenna_sockets: 2,
};

/// Models whose `1A 05` numbering has been read out of the manufacturer's CI-V
/// reference — or, on the sets old enough to predate those being published
/// separately, out of the command table at the back of the instruction manual.
/// Extending this is a documentation exercise, not a protocol one.
///
/// Every Icom that speaks this protocol belongs here, because the alternative
/// is not a smaller table but an operator being sent to the radio's own menus
/// for something sdroxide could have written for them. What keeps a model out
/// is only ever a number nobody has read.
pub const MODELS: &[Model] = &[
    Model {
        civ_address: 0xB6,
        name: "IC-7300MK2",
        scope_bins: 475,
        scope_full_scale: 160,
        // IC-7300MK2 CI-V reference: 1A 05 00 84 = DATA OFF MOD, 00 85 = DATA MOD,
        // both `00=MIC … 05=LAN`.
        lan_mod_input: Some((&[0x0084, 0x0085], 0x05)),
        // 1A 05 00 79 = LAN AF/IF Output → Output Select, `00=AF, 01=IF`.
        lan_afif_select: Some(0x0079),
        data_mode_sub: Some(0x06),
        if_inverted: false,
        // One aerial socket. Its `0x12` is not a selector: the guide lists
        // sub-command `00` alone, `00/01` switching the receiving antenna.
        antenna_sockets: 1,
    },
    Model {
        civ_address: 0xA4,
        name: "IC-705",
        scope_bins: 475,
        scope_full_scale: 160,
        // IC-705 CI-V reference guide, SET > Connectors: 1A 05 01 18 = MOD
        // Input > DATA OFF MOD, 01 19 = DATA MOD, both
        // `00=MIC, 01=USB, 02=MIC,USB, 03=WLAN`. Note the value: the radio has
        // no LAN socket, so the modulation source is `03`, not the `05` an
        // IC-7300MK2 takes.
        lan_mod_input: Some((&[0x0118, 0x0119], 0x03)),
        // 1A 05 01 14 = WLAN AF/IF Output > Output Select, `00=AF, 01=IF`.
        // 01 09 is the *USB* one — the same setting on the other port, and the
        // wrong one to write for a network session.
        lan_afif_select: Some(0x0114),
        data_mode_sub: Some(0x06),
        if_inverted: false,
        antenna_sockets: 2,
    },
    Model {
        civ_address: 0xB2,
        name: "IC-7760",
        // The two-box IC-7760 is the first LAN Icom that does not send 475
        // bins: its CI-V reference gives the LAN sweep as one 704-byte
        // division carrying 689 points on a 0 ~ C8 scale. The USB form of the
        // same sweep is split into 15, which is why the division field has to
        // be read rather than assumed.
        scope_bins: 689,
        scope_full_scale: 200,
        // IC-7760 CI-V reference guide, SET > Connectors > MOD Input:
        // 1A 05 01 29 = DATA OFF MOD, 01 30 / 01 31 / 01 32 = DATA1 / DATA2 /
        // DATA3 MOD. The list runs `00=MIC … 09=LAN`, so LAN is `09` here —
        // neither the IC-7300MK2's `05` nor the IC-705's `03`.
        lan_mod_input: Some((&[0x0129, 0x0130, 0x0131, 0x0132], 0x09)),
        // 1A 05 01 23 = LAN AF/IF Output > Output Select, `00=AF, 01=IF`.
        // 01 03 is the [USB B] port's copy and 01 10 the LINE-OUT one; both
        // are the same setting on a socket this session is not using.
        lan_afif_select: Some(0x0123),
        data_mode_sub: Some(0x06),
        // Field report: on the 12 kHz IF this radio's stream is mirrored, and
        // SSB lands on the wrong sideband until the mix runs the other way.
        // See `Model::if_inverted` for what is and is not known about that.
        if_inverted: true,
        antenna_sockets: 2,
    },
    Model {
        civ_address: 0x98,
        name: "IC-7610",
        // The IC-7610 sweeps the IC-7760's shape, not the IC-7300's, two
        // generations before it: its CI-V reference gives the LAN sweep as one
        // 704-byte division carrying 689 points on a 0 ~ C8 scale, and splits
        // the USB form of the same sweep into 15.
        scope_bins: 689,
        scope_full_scale: 200,
        // IC-7610 CI-V reference guide, Connectors > MOD Input: 1A 05 00 91 =
        // DATA OFF MOD, 00 92 / 00 93 / 00 94 = DATA1 / DATA2 / DATA3 MOD.
        // Four slots, like the IC-7760, but the list is the IC-7300MK2's —
        // `00=MIC, 01=ACC, 02=MIC,ACC, 03=USB, 04=MIC,USB, 05=LAN` — so LAN is
        // `05` here and `09` on the radio whose slot count it shares.
        lan_mod_input: Some((&[0x0091, 0x0092, 0x0093, 0x0094], 0x05)),
        // 1A 05 00 86 = Connectors > LAN AF/IF Output > Output Select,
        // `00=AF, 01=IF`. 00 80 is the [USB] port's copy of the same setting,
        // six items earlier — near enough to reach by a typo, and the wrong
        // one to write for a network session.
        lan_afif_select: Some(0x0086),
        data_mode_sub: Some(0x06),
        if_inverted: false,
        antenna_sockets: 2,
    },
    Model {
        civ_address: 0xA2,
        name: "IC-9700",
        scope_bins: 475,
        scope_full_scale: 160,
        // IC-9700 CI-V reference guide, SET > Connectors > MOD Input:
        // 1A 05 01 15 = DATA OFF MOD, 01 16 = DATA MOD. One DATA slot, as on
        // an IC-7300MK2, and the same `00=MIC … 05=LAN` list.
        lan_mod_input: Some((&[0x0115, 0x0116], 0x05)),
        // 1A 05 01 10 = SET > Connectors > LAN AF/IF Output > Output Select,
        // `00=AF, 01=IF`. 01 05 is the [USB] port's copy.
        lan_afif_select: Some(0x0110),
        data_mode_sub: Some(0x06),
        if_inverted: false,
        antenna_sockets: 2,
    },
    Model {
        civ_address: 0xAC,
        name: "IC-905",
        scope_bins: 475,
        scope_full_scale: 160,
        // IC-905 CI-V reference guide, SET > Connectors > MOD Input:
        // 1A 05 01 26 = DATA OFF MOD, 01 27 = DATA MOD, both
        // `00=MIC, 01=USB, 02=MIC,USB, 03=LAN` — the IC-705's short list, so
        // LAN is `03` and not the `05` of the models either side of it here.
        //
        // Not 01 29: that is ATV MOD, which has a *longer* list of its own
        // (`00=MIC, 01=AV-IN, … 05=LAN`) because the ATV picture carries the
        // AV-IN socket. Writing `03` there selects USB.
        lan_mod_input: Some((&[0x0126, 0x0127], 0x03)),
        // 1A 05 01 22 = LAN AF/IF Output > Output Select, `00=AF, 01=IF`.
        // 01 17 is the USB/AV-OUT copy.
        lan_afif_select: Some(0x0122),
        data_mode_sub: Some(0x06),
        if_inverted: false,
        antenna_sockets: 2,
    },
    Model {
        civ_address: 0x96,
        name: "IC-R8600",
        scope_bins: 475,
        scope_full_scale: 160,
        // A receiver: no modulation input to select, which is the one field in
        // this table that does not exist rather than merely being unknown.
        // `RadioCap::can_transmit` already suppresses the note that would
        // otherwise ask the operator to go and set a menu item that is not on
        // their radio; this is the other half of that.
        lan_mod_input: None,
        // IC-R8600 CI-V reference guide: 1A 05 00 89 = the signal output from
        // [LAN], `00=AF, 01=IF`. Icom words it per socket rather than as a
        // "LAN AF/IF Output" heading on this set, and there are two more of
        // them — 00 68 for the [AF/IF] jack and 00 73 for the front [USB].
        lan_afif_select: Some(0x0089),
        // The `1A` command on this set has sub-commands `00` and `05` and
        // nothing else. There is no DATA switch because there is no transmit,
        // so the family default would be a frame the receiver can only answer
        // NG to, once per mode change.
        data_mode_sub: None,
        if_inverted: false,
        antenna_sockets: 2,
    },
    Model {
        civ_address: 0x8E,
        name: "IC-7851",
        // The scope fields are the family fallback, not figures read out of a
        // manual. This set has no separate CI-V reference guide — its command
        // table is a chapter of the instruction manual — and that table
        // documents no `27 00` at all, though the firmware release notes for
        // the sweep say otherwise. So the shape is simply unread. It is what
        // an unrecognised radio would have been given anyway, which is why the
        // gap is not a reason to leave the model out and lose the menu numbers
        // below with it.
        scope_bins: 475,
        scope_full_scale: 160,
        // IC-7850/IC-7851 instruction manual, command table: 1A 05 00 63 =
        // MOD input connector during DATA OFF, 00 64 / 00 65 / 00 66 = DATA1 /
        // DATA2 / DATA3. The flagship has the longest list of the lot —
        // `00=MIC, 01=ACC-A, 02=ACC-B, 03=MIC/ACC-A, 04=MIC/ACC-B,
        // 05=ACC-A/ACC-B, 06=MIC/ACC-A/ACC-B, 07=S/P DIF, 08=USB, 09=LAN` —
        // so LAN is `09`, as on the IC-7760, for an entirely different reason.
        lan_mod_input: Some((&[0x0063, 0x0064, 0x0065, 0x0066], 0x09)),
        // 1A 05 00 56 = AF/IF signal output to LAN, `00=AF, 01=IF`. This set
        // has five of these, one per output socket — 00 32 ACC-A, 00 38 ACC-B,
        // 00 44 S/P DIF, 00 50 USB B — and only the last of the five is ours.
        lan_afif_select: Some(0x0056),
        data_mode_sub: Some(0x06),
        if_inverted: false,
        antenna_sockets: 2,
    },
];

/// Whether a model name is one of Icom's receivers.
///
/// The `IC-R` prefix is the whole rule and it is Icom's, not ours: IC-R8600,
/// IC-R30, IC-R6 and the rest of the line receive only, and no transceiver is
/// named that way. Used to overrule a capability block that offers a transmit
/// stream a radio cannot possibly honour — see [`RadioCap::can_transmit`].
pub fn is_receiver(name: &str) -> bool {
    let name = name.trim().to_ascii_uppercase();
    name.starts_with("IC-R") || name.starts_with("ICR")
}

/// Look a radio up by the CI-V address it reported.
pub fn model_for(civ_address: u8) -> Model {
    MODELS
        .iter()
        .copied()
        .find(|m| m.civ_address == civ_address)
        .unwrap_or(Model { civ_address, ..UNKNOWN_MODEL })
}

/// Look a radio up by the name *and* the address its capability block gave.
///
/// Two pieces of evidence, both of which the operator can move:
/// SET > Connectors > CI-V > CI-V Address is a menu item on all of these
/// radios, and so is the network radio name the capability block carries. So
/// the name is consulted — a rig moved off its factory address would otherwise
/// fall through to [`UNKNOWN_MODEL`] and be told to set its own menu items by
/// hand, in the one case where sdroxide knows perfectly well which radio it is
/// talking to — but only where it does not contradict a known address.
///
/// A name matching one model while the address is another model's factory
/// value is a nickname somebody typed, not a radio: the address wins, because
/// writing a menu index for the wrong Icom is the failure this whole table
/// exists to avoid.
///
/// Whichever way it goes, the address the radio actually reported is what the
/// session addresses its frames to.
pub fn model_for_radio(name: &str, civ_address: u8) -> Model {
    let want = name.trim().to_ascii_uppercase();
    let by_name = MODELS.iter().copied().find(|m| m.name == want);
    let by_address = MODELS.iter().copied().find(|m| m.civ_address == civ_address);
    match (by_name, by_address) {
        (Some(m), Some(a)) if m.name != a.name => a,
        (Some(m), _) => Model { civ_address, ..m },
        _ => model_for(civ_address),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trips() {
        let p = control(ctrl::ARE_YOU_THERE, 0, 0x1234_5678, 0x9abc_def0);
        assert_eq!(p.len(), CONTROL_SIZE);
        let h = Header::parse(&p).unwrap();
        assert_eq!(h.len, CONTROL_SIZE as u32);
        assert_eq!(h.kind, ctrl::ARE_YOU_THERE);
        assert_eq!(h.sent_id, 0x1234_5678);
        assert_eq!(h.rcvd_id, 0x9abc_def0);
    }

    #[test]
    fn session_id_folds_address_and_port() {
        // The low two octets sit above the port, so two clients on the same
        // host never collide and the same client on two subnets does not
        // either.
        let id = session_id(Ipv4Addr::new(192, 168, 1, 42), 0xbeef);
        assert_eq!(id, 0x012a_beef);
    }

    #[test]
    fn ping_round_trips_and_keeps_the_timestamp() {
        let p = ping(7, false, 0x0102_0304, 1, 2);
        let got = parse_ping(&p).unwrap();
        assert_eq!(got, Ping { seq: 7, reply: false, time: 0x0102_0304 });
        let reply = ping(got.seq, true, got.time, 2, 1);
        assert!(parse_ping(&reply).unwrap().reply);
    }

    #[test]
    fn passcode_matches_the_published_table() {
        // Position is folded in, so a repeated character does not repeat.
        // 'a' is 0x61: index 97 at position 0, index 98 at position 1.
        assert_eq!(&passcode("aa")[..2], &[0x38, 0x2b]);
        // Empty stays all-zero, and 16 characters is the hard limit.
        assert_eq!(passcode(""), [0u8; 16]);
        let long = passcode("abcdefghijklmnopqrstuvwxyz");
        assert_eq!(long.len(), 16);
        assert_ne!(long[15], 0);
    }

    #[test]
    fn passcode_folds_characters_that_run_off_the_table() {
        // 'z' (0x7a) at index 15 would index 0x89, past the printable range;
        // it has to wrap rather than panic.
        let p = passcode("               z");
        assert_ne!(p[15], 0);
    }

    #[test]
    fn login_puts_credentials_where_the_radio_looks() {
        let p = login("bob", "secret", "sdroxide", 3, 0xabcd, 1, 2);
        assert_eq!(p.len(), LOGIN_SIZE);
        assert_eq!(&p[0x40..0x43], &passcode("bob")[..3]);
        assert_eq!(&p[0x50..0x56], &passcode("secret")[..6]);
        assert_eq!(&p[0x60..0x68], b"sdroxide");
        assert_eq!(u32_be_at(&p, 0x10), (LOGIN_SIZE - HEADER) as u32);
        assert_eq!(u16_be_at(&p, 0x16), 3);
        assert_eq!(u16_at(&p, 0x1a), 0xabcd);
    }

    #[test]
    fn a_login_response_is_parsed_and_bad_credentials_are_recognised() {
        let mut b = vec![0u8; LOGIN_RESPONSE_SIZE];
        put_u32(&mut b, 0, LOGIN_RESPONSE_SIZE as u32);
        put_u16(&mut b, 0x1a, 0x1234);
        put_u32(&mut b, 0x1c, 0xdead_beef);
        put_u32(&mut b, 0x30, LoginResponse::BAD_CREDENTIALS);
        b[0x40..0x44].copy_from_slice(b"FTTH");
        let r = LoginResponse::parse(&b).unwrap();
        assert_eq!(r.token_request, 0x1234);
        assert_eq!(r.token, 0xdead_beef);
        assert_eq!(r.error, LoginResponse::BAD_CREDENTIALS);
        assert_eq!(r.connection, "FTTH");
    }

    fn a_radio_cap(civ: u8, tx_sample: u16) -> Vec<u8> {
        let mut c = vec![0u8; RADIO_CAP_SIZE];
        put_u16_be(&mut c, 0x07, RadioCap::CAP_MAC);
        c[0x0a..0x10].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        c[0x10..0x1a].copy_from_slice(b"IC-7300MK2");
        c[0x30..0x33].copy_from_slice(b"ICO");
        c[0x52] = civ;
        put_u16_be(&mut c, 0x53, 0x0f);
        put_u16_be(&mut c, 0x55, tx_sample);
        put_u32_be(&mut c, 0x5a, 115_200);
        c
    }

    #[test]
    fn capabilities_yield_the_radios_own_civ_address() {
        let mut b = vec![0u8; CAPABILITIES_SIZE];
        put_u16(&mut b, 0x40, 1);
        b.extend_from_slice(&a_radio_cap(0xB6, 0x0f));
        let radios = parse_capabilities(&b).unwrap();
        assert_eq!(radios.len(), 1);
        assert_eq!(radios[0].civ_address, 0xB6);
        assert_eq!(radios[0].name, "IC-7300MK2");
        assert_eq!(radios[0].baud_rate, 115_200);
        assert!(radios[0].uses_mac());
        assert!(radios[0].can_transmit());
    }

    #[test]
    fn a_receive_only_radio_says_so_through_its_tx_sample_field() {
        let mut b = vec![0u8; CAPABILITIES_SIZE];
        b.extend_from_slice(&a_radio_cap(0x96, 0));
        assert!(!parse_capabilities(&b).unwrap()[0].can_transmit());
    }

    /// The IC-R8600 in the field advertised a transmit stream. The negotiation
    /// still has to ask for what the radio offered — that session works — but
    /// nothing above it should put a PTT on a receiver.
    #[test]
    fn a_receiver_that_offers_a_transmit_stream_still_has_no_transmitter() {
        let mut b = vec![0u8; CAPABILITIES_SIZE];
        put_u16(&mut b, 0x40, 1);
        let mut cap = a_radio_cap(0x96, 0x0f);
        cap[0x10..0x1a].copy_from_slice(b"IC-R8600  ");
        b.extend_from_slice(&cap);
        let radio = &parse_capabilities(&b).unwrap()[0];
        assert_eq!(radio.name, "IC-R8600");
        assert!(radio.can_transmit(), "the wire offer is reported as it stands");
        assert!(!radio.has_transmitter(), "an IC-R has nothing to modulate");
    }

    #[test]
    fn the_receiver_rule_is_the_ic_r_prefix_and_nothing_else() {
        assert!(is_receiver("IC-R8600"));
        assert!(is_receiver("IC-R30"));
        assert!(is_receiver(" ic-r6 "));
        // Every transceiver, including the ones with an R further along.
        assert!(!is_receiver("IC-7300MK2"));
        assert!(!is_receiver("IC-705"));
        assert!(!is_receiver("IC-9700"));
    }

    #[test]
    fn a_length_that_is_not_whole_radios_is_not_a_capabilities_packet() {
        // This is the only thing separating a capabilities reply from an
        // oversized retransmit request, so it has to hold.
        let mut b = vec![0u8; CAPABILITIES_SIZE + 3];
        put_u16(&mut b, 0x40, 1);
        assert!(parse_capabilities(&b).is_none());
    }

    #[test]
    fn a_stream_request_carries_ports_and_codecs_big_endian() {
        let radio = RadioCap {
            name: "IC-7300MK2".into(),
            audio: "ICOM".into(),
            civ_address: 0xB6,
            rx_sample: 0x0f,
            tx_sample: 0x0f,
            baud_rate: 115_200,
            common_cap: RadioCap::CAP_MAC,
            mac: [1, 2, 3, 4, 5, 6],
            guid: [0; 16],
        };
        let p = stream_request(&StreamRequest {
            radio: &radio,
            username: "bob",
            rx_codec: AudioCodec::Lpcm16Mono,
            tx_codec: Some(AudioCodec::Lpcm16Mono),
            rx_sample_rate: 48_000,
            tx_sample_rate: 48_000,
            civ_local_port: 40_001,
            audio_local_port: 40_002,
            tx_buffer_ms: 150,
            inner_seq: 4,
            token_request: 0x1111,
            token: 0x2222_3333,
            sent_id: 1,
            rcvd_id: 2,
        });
        assert_eq!(p.len(), CONNINFO_SIZE);
        assert_eq!(p[0x15], 0x03);
        assert_eq!(&p[0x40..0x4a], b"IC-7300MK2");
        assert_eq!(p[0x70], 1);
        assert_eq!(p[0x71], 1);
        assert_eq!(p[0x72], AudioCodec::Lpcm16Mono.byte());
        assert_eq!(u32_be_at(&p, 0x74), 48_000);
        assert_eq!(u32_be_at(&p, 0x7c), 40_001);
        assert_eq!(u32_be_at(&p, 0x80), 40_002);
        assert_eq!(u16_be_at(&p, 0x27), RadioCap::CAP_MAC);
        assert_eq!(&p[0x2a..0x30], &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn a_receive_only_stream_request_asks_for_no_tx_rate() {
        let radio = RadioCap {
            name: "IC-R8600".into(),
            audio: String::new(),
            civ_address: 0x96,
            rx_sample: 0x0f,
            tx_sample: 0,
            baud_rate: 115_200,
            common_cap: RadioCap::CAP_MAC,
            mac: [0; 6],
            guid: [0; 16],
        };
        let p = stream_request(&StreamRequest {
            radio: &radio,
            username: "bob",
            rx_codec: AudioCodec::Lpcm16Mono,
            tx_codec: None,
            rx_sample_rate: 48_000,
            tx_sample_rate: 48_000,
            civ_local_port: 1,
            audio_local_port: 2,
            tx_buffer_ms: 150,
            inner_seq: 0,
            token_request: 0,
            token: 0,
            sent_id: 0,
            rcvd_id: 0,
        });
        assert_eq!(p[0x71], 0);
        assert_eq!(p[0x73], 0);
        assert_eq!(u32_be_at(&p, 0x78), 0);
    }

    #[test]
    fn status_reports_the_radios_own_ports_big_endian() {
        let mut b = vec![0u8; STATUS_SIZE];
        put_u32(&mut b, 0, STATUS_SIZE as u32);
        put_u16_be(&mut b, 0x42, 50_002);
        put_u16_be(&mut b, 0x46, 50_003);
        let s = Status::parse(&b).unwrap();
        assert_eq!(s.civ_port, 50_002);
        assert_eq!(s.audio_port, 50_003);
        assert!(!s.disconnected);
        assert_eq!(s.error, 0);
    }

    #[test]
    fn civ_data_round_trips_and_a_truncated_one_is_refused() {
        let frame = [0xfe, 0xfe, 0xe0, 0xb6, 0x03, 0xfd];
        let p = civ_data(&frame, 9, 1, 2);
        assert_eq!(parse_civ_data(&p).unwrap(), frame);
        assert_eq!(u16_be_at(&p, 0x13), 9);
        // A datagram cut short mid-frame must not reach the CI-V parser.
        let mut bad = p.clone();
        bad.truncate(bad.len() - 2);
        assert!(parse_civ_data(&bad).is_none());
    }

    #[test]
    fn an_open_close_packet_is_not_mistaken_for_a_civ_frame() {
        // Same type code, and its bytes at 0x11 read as a self-consistent
        // inner length — only the missing FE FE preamble tells them apart.
        let oc = open_close(true, 1, 1, 2);
        assert_eq!(oc.len(), OPENCLOSE_SIZE);
        assert!(parse_civ_data(&oc).is_none());
    }

    #[test]
    fn audio_data_round_trips() {
        let pcm = vec![0x11u8; 320];
        let p = audio_data(&pcm, 5, 1, 2);
        assert_eq!(u16_be_at(&p, 0x16), 320);
        assert_eq!(u16_be_at(&p, 0x12), 5);
        assert_eq!(parse_audio_data(&p).unwrap(), &pcm[..]);
    }

    #[test]
    fn lpcm16_survives_a_round_trip() {
        let src: Vec<f32> = (0..64).map(|i| (i as f32 / 32.0) - 1.0).collect();
        let mut bytes = Vec::new();
        encode_pcm(AudioCodec::Lpcm16Mono, &src, &mut bytes);
        assert_eq!(bytes.len(), src.len() * 2);
        let mut back = Vec::new();
        decode_pcm(AudioCodec::Lpcm16Mono, &bytes, &mut back);
        assert_eq!(back.len(), src.len());
        for (a, b) in src.iter().zip(&back) {
            assert!((a - b).abs() < 1e-4, "{a} vs {b}");
        }
    }

    #[test]
    fn stereo_payloads_fold_to_the_left_channel() {
        // Main and sub receiver, and this backend drives one of them.
        let mut bytes = Vec::new();
        for i in 0..4i16 {
            bytes.extend_from_slice(&(i * 1000).to_le_bytes()); // left
            bytes.extend_from_slice(&(-i * 1000).to_le_bytes()); // right
        }
        let mut out = Vec::new();
        decode_pcm(AudioCodec::Lpcm16Stereo, &bytes, &mut out);
        assert_eq!(out.len(), 4);
        assert!(out[3] > 0.0);
    }

    #[test]
    fn ulaw_survives_a_round_trip_within_its_own_resolution() {
        for &s in &[-0.9f32, -0.25, -0.01, 0.0, 0.01, 0.25, 0.9] {
            let mut bytes = Vec::new();
            encode_pcm(AudioCodec::ULaw8Mono, &[s], &mut bytes);
            let mut back = Vec::new();
            decode_pcm(AudioCodec::ULaw8Mono, &bytes, &mut back);
            assert!((back[0] - s).abs() < 0.05, "{s} came back as {}", back[0]);
        }
    }

    #[test]
    fn a_retransmit_request_names_each_sequence_as_a_range() {
        let p = retransmit_request(&[0x0102, 0x0304], 1, 2);
        assert_eq!(p.len(), CONTROL_SIZE + 8);
        assert_eq!(u32_at(&p, 0), p.len() as u32);
        assert_eq!(u16_at(&p, 4), ctrl::RETRANSMIT);
        assert_eq!(u16_at(&p, 0x10), 0x0102);
        assert_eq!(u16_at(&p, 0x12), 0x0102);
        assert_eq!(u16_at(&p, 0x14), 0x0304);
    }

    #[test]
    fn open_close_flips_only_the_magic_byte() {
        let open = open_close(true, 1, 1, 2);
        let close = open_close(false, 1, 1, 2);
        assert_eq!(open.len(), OPENCLOSE_SIZE);
        assert_eq!(open[0x15], 0x04);
        assert_eq!(close[0x15], 0x00);
        assert_eq!(&open[..0x15], &close[..0x15]);
    }

    #[test]
    fn the_known_models_are_known_and_anything_else_degrades_safely() {
        let mk2 = model_for(0xB6);
        assert_eq!(mk2.name, "IC-7300MK2");
        assert_eq!(mk2.scope_bins, 475);
        assert!(mk2.lan_mod_input.is_some());
        assert!(mk2.lan_afif_select.is_some());

        // The IC-705 numbers its Connectors block a hundred items further on
        // than the MK2 does, and takes a different value for the LAN source —
        // the two facts that make this a per-model table rather than a
        // constant.
        let ic705 = model_for(0xA4);
        assert_eq!(ic705.name, "IC-705");
        assert_eq!(ic705.lan_mod_input, Some((&[0x0118u16, 0x0119][..], 0x03)));
        assert_eq!(ic705.lan_afif_select, Some(0x0114));

        // An unknown rig still works — it just does not get menu writes, which
        // is the whole point of the table. `88h` is the IC-7100, which has no
        // network port and so can never be in here.
        let other = model_for(0x88);
        assert_eq!(other.civ_address, 0x88);
        assert!(other.lan_mod_input.is_none());
        assert!(other.lan_afif_select.is_none());
    }

    #[test]
    fn the_ic7760_is_the_odd_one_in_every_field_that_has_one() {
        let r = model_for(0xB2);
        assert_eq!(r.name, "IC-7760");
        // A wider sweep on a taller scale than the IC-7300 generation: 689
        // points, 0 ~ 200. Reading either from a neighbouring model draws a
        // trace that is the wrong width *and* 20 dB out.
        assert_eq!(r.scope_bins, 689);
        assert_eq!(r.scope_full_scale, 200);
        // Four DATA slots, and LAN is 09 — a value that means "MIC, USB, ACC"
        // on the model whose numbering is nearest.
        assert_eq!(r.lan_mod_input, Some((&[0x0129u16, 0x0130, 0x0131, 0x0132][..], 0x09)));
        assert_eq!(r.lan_afif_select, Some(0x0123));
    }

    #[test]
    fn the_ic7610_sweeps_the_ic7760s_shape_on_numbering_of_its_own() {
        // Issue #190: the model an operator was told sdroxide did not know.
        let r = model_for(0x98);
        assert_eq!(r.name, "IC-7610");
        // 689 points on 0 ~ 200, like the IC-7760 and unlike everything else
        // of its generation.
        assert_eq!(r.scope_bins, 689);
        assert_eq!(r.scope_full_scale, 200);
        // Four DATA slots like the IC-7760's, but LAN is `05` and not `09`:
        // the two facts travel separately, so neither model's numbering can be
        // guessed from the other's.
        assert_eq!(r.lan_mod_input, Some((&[0x0091u16, 0x0092, 0x0093, 0x0094][..], 0x05)));
        assert_eq!(r.lan_afif_select, Some(0x0086));
    }

    #[test]
    fn every_lan_transceiver_writes_its_data_off_slot_first() {
        // The DATA-OFF entry leads each list because it is the one a rig sits
        // on outside a digital mode; a list that began anywhere else would
        // leave the plain-SSB modulation source pointing at the microphone.
        for m in MODELS {
            let Some((items, lan)) = m.lan_mod_input else {
                assert!(is_receiver(m.name), "{} has no MOD input and is not a receiver", m.name);
                continue;
            };
            assert!(!items.is_empty(), "{} has an empty MOD input list", m.name);
            // One DATA-OFF plus one, two or three DATA slots.
            assert!((2..=4).contains(&items.len()), "{} has {} slots", m.name, items.len());
            // Consecutive, and in the order Icom numbers them.
            assert!(items.windows(2).all(|w| w[1] > w[0]), "{} is out of order", m.name);
            // The value is an index into a per-model list, never a bit field:
            // the longest of them stops at `09`.
            assert!(lan <= 0x09, "{} takes {lan:#04x} for LAN", m.name);
        }
    }

    #[test]
    fn a_receiver_gets_an_output_select_and_nothing_that_implies_transmit() {
        let r = model_for(0x96);
        assert_eq!(r.name, "IC-R8600");
        assert!(is_receiver(r.name));
        // The 12 kHz IF is the whole reason a receive-only set is in the table.
        assert_eq!(r.lan_afif_select, Some(0x0089));
        assert!(r.lan_mod_input.is_none());
        // …and no DATA switch, which on this set is a sub-command that does
        // not exist rather than one that does nothing.
        assert!(r.data_mode_sub.is_none());
    }

    #[test]
    fn a_radio_moved_off_its_factory_address_is_still_recognised_by_name() {
        // CI-V > CI-V Address is a menu item, and a radio on a shared bus is
        // routinely moved off its factory value. The name still names it.
        let moved = model_for_radio("IC-7760", 0x7A);
        assert_eq!(moved.name, "IC-7760");
        assert_eq!(moved.scope_bins, 689);
        // …and frames still go to where the radio said it is listening.
        assert_eq!(moved.civ_address, 0x7A);

        // The capability block is a fixed-width field with whatever the far end
        // put in it, so neither the padding nor the case is worth trusting.
        assert_eq!(model_for_radio(" ic-7760 ", 0xB2).name, "IC-7760");

        // A name nobody knows falls back to the address, which is how the
        // table behaved before names were consulted at all.
        assert_eq!(model_for_radio("IC-7100", 0xB6).name, "IC-7300MK2");
        assert_eq!(model_for_radio("IC-7100", 0x88).name, UNKNOWN_MODEL.name);

        // And a name that names one model while the address names another is a
        // nickname somebody typed into the network settings. The address wins:
        // an IC-705's menu numbers written into an IC-7760 land in the wrong
        // block entirely.
        let renamed = model_for_radio("IC-705", 0xB2);
        assert_eq!(renamed.name, "IC-7760");
        assert_eq!(renamed.lan_afif_select, Some(0x0123));
    }
}
