//! The SmartSDR wire format: the ASCII command/status protocol on TCP 4992 and
//! the VITA-49 data packets on UDP.
//!
//! Both halves of this crate share it — the client parses what a radio sends and
//! builds what a client sends; the simulator does exactly the reverse — so every
//! codec here is written as a matched pair and tested by round-tripping.
//!
//! # Control protocol
//!
//! Every line is ASCII, `\n`-terminated, and identified by its first character:
//!
//! | Tag | Direction | Shape | Meaning |
//! |-----|-----------|-------|---------|
//! | `V` | radio → client | `V1.4.0.0` | firmware/protocol version, sent first |
//! | `H` | radio → client | `H1A2B3C4D` | this client's handle, in hex |
//! | `C` | client → radio | `C<seq>\|<command>` | a sequenced command |
//! | `R` | radio → client | `R<seq>\|<hex code>\|<body>` | reply to command `seq` |
//! | `S` | radio → client | `S<handle>\|<object> <k>=<v> …` | status update |
//! | `M` | radio → client | `M<hex id>\|<text>` | log/notice message |
//!
//! A result code of `0` is success. Codes are *hexadecimal* and unsigned: fatal
//! ones such as `F3000001` have the high bit set, so parsing them as a signed
//! decimal integer silently turns a hard rejection into an apparent success.
//!
//! # VITA-49
//!
//! Data rides UDP as VITA-49 packets with a fixed 28-byte (7-word) header, all
//! big-endian. Word 3's low 16 bits hold the *packet class code*, which is what
//! actually says whether the payload is IQ, audio, FFT bins or meter readings —
//! the stream ID in word 1 only says which of our streams it belongs to.
//!
//! One payload does **not** follow the header's big-endian rule: `dax_iq`
//! carries little-endian float32 (the radio reports `payload_endian=little` for
//! that stream type alone). Reading it big-endian byte-reverses every float into
//! a denormal ≈ 0, which reads as a dead-silent receiver rather than as an
//! error — so it is called out at [`decode_dax_iq`].

use std::collections::BTreeMap;

use sdroxide_types::{AtuState, AtuStatus};

/// The TCP command port, and the UDP port discovery broadcasts arrive on. The
/// radio uses the same number for both.
pub const RADIO_PORT: u16 = 4992;

/// FlexRadio's IEEE OUI, in the VITA-49 class ID (word 2).
pub const FLEX_OUI: u32 = 0x00_1C_2D;
/// FlexRadio's VITA-49 information class code (word 3, upper 16 bits).
pub const FLEX_INFO_CLASS: u16 = 0x534C;

/// VITA-49 header length: 7 words.
pub const VITA_HEADER_BYTES: usize = 28;
/// VITA-49 header length in 32-bit words.
pub const VITA_HEADER_WORDS: usize = 7;

/// Packet class codes seen on the wire (from FlexLib's `VitaFlex.cs`).
pub mod pcc {
    /// RX audio, uncompressed: float32 stereo, big-endian, 24 kHz.
    pub const IF_NARROW: u16 = 0x03E3;
    /// RX audio, reduced bandwidth: int16 mono, big-endian.
    pub const IF_NARROW_REDUCED: u16 = 0x0123;
    /// RX audio, Opus-compressed.
    pub const OPUS: u16 = 0x8005;
    /// Meter readings: N × (uint16 id, int16 raw), big-endian.
    pub const METER: u16 = 0x8002;
    /// Panadapter FFT bins.
    pub const FFT: u16 = 0x8003;
    /// Waterfall tiles.
    pub const WATERFALL: u16 = 0x8004;

    /// DAX IQ, float32 interleaved I/Q, **little**-endian. The rate is encoded
    /// in the class code rather than carried as a field, so these are four
    /// distinct codes — see [`super::dax_iq_rate`].
    pub const DAX_IQ_24K: u16 = 0x02E3;
    pub const DAX_IQ_48K: u16 = 0x02E4;
    pub const DAX_IQ_96K: u16 = 0x02E5;
    pub const DAX_IQ_192K: u16 = 0x02E6;
}

/// The sample rate a DAX IQ packet class code stands for, or `None` if the code
/// is not a DAX IQ one.
///
/// Worth checking against the rate we asked for: the radio creates every
/// `dax_iq` stream at 48 kHz and only moves to the requested rate once the
/// follow-up `stream set … daxiq_rate=` lands, so packets at the wrong rate are
/// a normal transient — and a permanent one if that command was rejected.
pub fn dax_iq_rate(class_code: u16) -> Option<u32> {
    match class_code {
        pcc::DAX_IQ_24K => Some(24_000),
        pcc::DAX_IQ_48K => Some(48_000),
        pcc::DAX_IQ_96K => Some(96_000),
        pcc::DAX_IQ_192K => Some(192_000),
        _ => None,
    }
}

/// The DAX IQ packet class code for a rate (the simulator's side of
/// [`dax_iq_rate`]). Unknown rates fall back to the 48 kHz code, which is what
/// the radio itself defaults a fresh stream to.
pub fn dax_iq_class_code(rate_hz: u32) -> u16 {
    match rate_hz {
        24_000 => pcc::DAX_IQ_24K,
        96_000 => pcc::DAX_IQ_96K,
        192_000 => pcc::DAX_IQ_192K,
        _ => pcc::DAX_IQ_48K,
    }
}

// ─── Control protocol ────────────────────────────────────────────────────────

/// A parsed control line.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// `V<version>` — the radio's firmware version.
    Version(String),
    /// `H<hex>` — the handle the radio assigned to this client.
    Handle(u32),
    /// `R<seq>|<hex code>|<body>` — a reply to command `seq`.
    Response { seq: u32, code: u32, body: String },
    /// `S<hex handle>|<object> <k>=<v> …` — a status update. `object` keeps its
    /// spaces (`"slice 0"`, `"display pan 0x40000000"`); `kvs` is everything
    /// after it.
    Status { handle: u32, object: String, kvs: Kvs },
    /// `M<hex id>|<text>` — a log message. The id's bits 24–25 are the severity.
    Message { severity: Severity, text: String },
    /// A line whose tag we don't know. Kept rather than dropped so the trace log
    /// shows what a radio actually sent.
    Unknown(String),
}

/// Severity encoded in bits 24–25 of an `M` line's message id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    fn from_id(id: u32) -> Severity {
        match (id >> 24) & 0x3 {
            0 => Severity::Info,
            1 => Severity::Warning,
            2 => Severity::Error,
            _ => Severity::Fatal,
        }
    }
}

/// Key/value pairs from a status line or a response body.
///
/// Ordered so a rendered trace line is stable between runs — a diagnostic log a
/// user mails in is only comparable against another if the same status prints
/// the same way twice.
pub type Kvs = BTreeMap<String, String>;

/// Parse `k=v` tokens separated by `sep`. A bare token with no `=` is kept with
/// an empty value, so callers can still test for its presence.
///
/// `sep` is a parameter because meter definitions are the one status object that
/// does *not* use spaces: they arrive as
/// `meter 7.src=SLC#7.num=0#7.nam=LEVEL#…`, and splitting those on space yields
/// a single unusable token.
pub fn parse_kvs_with(body: &str, sep: char) -> Kvs {
    let mut out = Kvs::new();
    for token in body.split(sep).filter(|t| !t.is_empty()) {
        match token.split_once('=') {
            Some((k, v)) => {
                out.insert(k.to_string(), v.to_string());
            }
            None => {
                out.insert(token.to_string(), String::new());
            }
        }
    }
    out
}

/// Parse space-separated `k=v` tokens (the usual case).
pub fn parse_kvs(body: &str) -> Kvs {
    parse_kvs_with(body, ' ')
}

/// Parse one control line (without its trailing newline).
pub fn parse_line(raw: &str) -> Line {
    let raw = raw.trim();
    if raw.is_empty() {
        return Line::Unknown(String::new());
    }
    let mut chars = raw.chars();
    let tag = chars.next().unwrap();
    let body = chars.as_str();

    match tag {
        'V' => Line::Version(body.to_string()),
        'H' => Line::Handle(u32::from_str_radix(body.trim(), 16).unwrap_or(0)),
        'R' => {
            let mut parts = body.splitn(3, '|');
            let seq = parts.next().unwrap_or("").trim().parse().unwrap_or(0);
            // Hex, and unsigned: `F3000001` is a real fatal code and must not
            // overflow into a "success" 0.
            let code =
                parts.next().map(|c| u32::from_str_radix(c.trim(), 16).unwrap_or(0)).unwrap_or(0);
            let body = parts.next().unwrap_or("").to_string();
            Line::Response { seq, code, body }
        }
        'S' => {
            let Some((handle_s, rest)) = body.split_once('|') else {
                return Line::Unknown(raw.to_string());
            };
            let handle = u32::from_str_radix(handle_s.trim(), 16).unwrap_or(0);
            let (object, kv_body) = split_object(rest);
            // Meter definitions are the `#`-separated exception; everything else
            // is space-separated. Decided by the object, not by whether the
            // body has spaces in it: a real radio's meter descriptions do
            // (`8.desc=RF Power Forward`), and split on those every meter came
            // out nameless — no SWR and no forward power, ever.
            let sep = if object == "meter" || (kv_body.contains('#') && !kv_body.contains(' ')) {
                '#'
            } else {
                ' '
            };
            Line::Status { handle, object, kvs: parse_kvs_with(kv_body, sep) }
        }
        'M' => {
            let (id_s, text) = body.split_once('|').unwrap_or((body, ""));
            let id = u32::from_str_radix(id_s.trim(), 16).unwrap_or(0);
            Line::Message { severity: Severity::from_id(id), text: text.to_string() }
        }
        _ => Line::Unknown(raw.to_string()),
    }
}

/// Split a status body into its object name and its key/value remainder.
///
/// Object names are multi-word (`slice 0`, `display pan 0x40000000`) and KV
/// tokens always contain `=`, so the boundary is the last space *before* the
/// first `=`. Anchoring on the first `=` rather than on a word count is what
/// makes this work for every object shape without a table of them.
fn split_object(body: &str) -> (String, &str) {
    let Some(first_eq) = body.find('=') else {
        return (body.trim().to_string(), "");
    };
    match body[..first_eq].rfind(' ') {
        Some(cut) => (body[..cut].trim().to_string(), &body[cut + 1..]),
        // The body starts with a KV: no object prefix at all.
        None => (String::new(), body),
    }
}

/// Build a `C<seq>|<command>\n` command line.
pub fn build_command(seq: u32, command: &str) -> String {
    format!("C{seq}|{command}\n")
}

/// Format a stream/object id the way the radio does (`0x40000000`).
pub fn hex_id(id: u32) -> String {
    format!("0x{id:08X}")
}

/// Parse a `0x`-prefixed (or bare) hex id. Used for stream ids in both status
/// keys and `stream create` reply bodies.
pub fn parse_hex_id(s: &str) -> Option<u32> {
    let t = s.trim();
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    u32::from_str_radix(t, 16).ok()
}

// ─── VITA-49 ─────────────────────────────────────────────────────────────────

/// The fields of a VITA-49 header this crate acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VitaHeader {
    /// Packet type, from bits 31–28. Radios send `3` (ExtDataWithStream) for
    /// everything; a client's DAX TX audio goes out as `1` (IFDataWithStream).
    pub packet_type: u8,
    /// Whether a 4-byte trailer follows the payload (bit 26).
    pub has_trailer: bool,
    /// The 4-bit modulo-16 packet counter (bits 19–16). Gaps in it are the only
    /// packet-loss signal the protocol offers.
    pub count: u8,
    /// Declared packet length in 32-bit words, including the header.
    pub words: u16,
    /// Which stream this packet belongs to (word 1).
    pub stream_id: u32,
    /// What the payload *is* (word 3, low 16 bits) — see [`pcc`].
    pub class_code: u16,
    /// Whether word 0 declares a Class ID (bit 27), two words of it.
    pub has_class_id: bool,
    /// Integer-timestamp selector (bits 23–22): non-zero means one word of it.
    pub tsi: u8,
    /// Fractional-timestamp selector (bits 21–20): non-zero means two words.
    pub tsf: u8,
}

impl VitaHeader {
    /// How long this packet's header actually is, in 32-bit words, from the
    /// indicator bits in word 0 — rather than the [`VITA_HEADER_WORDS`] a FLEX
    /// is assumed to send.
    ///
    /// They agree on every packet this backend has been shown: a FLEX sends the
    /// full shape, stream ID and class ID and both timestamps, which is seven
    /// words. They are worth being able to compare because the cost of them
    /// disagreeing is invisible and total — the payload would start one float
    /// late, so every I would be read as a Q, and a mirrored spectrum is
    /// unintelligible on both sidebands while the radio's own panadapter and
    /// waterfall, which do not come from this stream, go on looking perfect
    /// (issue #368).
    #[must_use]
    pub fn declared_header_words(&self) -> usize {
        // Odd packet types carry a stream ID; even ones do not. A FLEX sends 3
        // for everything it streams.
        1 + usize::from(self.packet_type & 1 == 1)
            + if self.has_class_id { 2 } else { 0 }
            + usize::from(self.tsi != 0)
            + if self.tsf != 0 { 2 } else { 0 }
    }
}

/// Parse a VITA-49 header, or `None` if the datagram is too short to hold one.
pub fn parse_vita_header(buf: &[u8]) -> Option<VitaHeader> {
    if buf.len() < VITA_HEADER_BYTES {
        return None;
    }
    let w = |i: usize| u32::from_be_bytes(buf[i * 4..i * 4 + 4].try_into().unwrap());
    let word0 = w(0);
    Some(VitaHeader {
        packet_type: ((word0 >> 28) & 0xF) as u8,
        has_trailer: word0 & 0x0400_0000 != 0,
        count: ((word0 >> 16) & 0x0F) as u8,
        words: (word0 & 0xFFFF) as u16,
        stream_id: w(1),
        class_code: (w(3) & 0xFFFF) as u16,
        has_class_id: word0 & 0x0800_0000 != 0,
        tsi: ((word0 >> 22) & 0x3) as u8,
        tsf: ((word0 >> 20) & 0x3) as u8,
    })
}

/// The payload slice of a datagram: everything after the header, minus the
/// trailer when the header declares one.
pub fn vita_payload<'a>(buf: &'a [u8], h: &VitaHeader) -> &'a [u8] {
    let end = if h.has_trailer { buf.len().saturating_sub(4) } else { buf.len() };
    &buf[VITA_HEADER_BYTES.min(end)..end]
}

/// How many packets were lost between `last` and `now`, given the 4-bit counter.
///
/// A gap of 15 is indistinguishable from 15 lost or 31 lost; the counter simply
/// cannot say. It is still worth reporting, because any nonzero answer means the
/// stream dropped something.
pub fn count_gap(last: u8, now: u8) -> u8 {
    (now.wrapping_sub(last) & 0x0F).wrapping_sub(1) & 0x0F
}

/// Build a VITA-49 header into `out` (7 words, big-endian).
#[allow(clippy::too_many_arguments)]
pub fn write_vita_header(
    out: &mut Vec<u8>,
    packet_type: u8,
    stream_id: u32,
    class_code: u16,
    count: u8,
    payload_words: usize,
) {
    let words = (payload_words + VITA_HEADER_WORDS) as u32;
    let mut word0 = 0u32;
    word0 |= (packet_type as u32 & 0xF) << 28;
    word0 |= 1 << 27; // C — class ID present
    // T = 0: no trailer.
    word0 |= 0x3 << 22; // TSI = Other
    word0 |= 0x1 << 20; // TSF = SampleCount
    word0 |= (count as u32 & 0xF) << 16;
    word0 |= words & 0xFFFF;

    out.extend_from_slice(&word0.to_be_bytes());
    out.extend_from_slice(&stream_id.to_be_bytes());
    out.extend_from_slice(&FLEX_OUI.to_be_bytes());
    out.extend_from_slice(&(((FLEX_INFO_CLASS as u32) << 16) | class_code as u32).to_be_bytes());
    out.extend_from_slice(&[0u8; 12]); // integer + fractional timestamps
}

/// Frames of stereo float32 audio per DAX TX packet — what SmartSDR itself
/// sends, and small enough to stay well inside any path MTU.
pub const TX_FRAMES_PER_PACKET: usize = 128;
/// DAX TX audio sample rate.
pub const TX_RATE_HZ: u32 = 24_000;

/// Build one DAX TX audio packet: mono `samples` duplicated to stereo, float32
/// **big**-endian, as `IFDataWithStream` (packet type 1).
///
/// Mono in, stereo out, because that is the shape of both ends: the engine
/// modulates one channel, and the radio's TX stream format is stereo regardless.
pub fn build_dax_tx_audio(stream_id: u32, count: u8, samples: &[f32]) -> Vec<u8> {
    let payload_words = samples.len() * 2;
    let mut out = Vec::with_capacity(VITA_HEADER_BYTES + payload_words * 4);
    write_vita_header(&mut out, 1, stream_id, pcc::IF_NARROW, count, payload_words);
    for &s in samples {
        let be = s.to_be_bytes();
        out.extend_from_slice(&be);
        out.extend_from_slice(&be);
    }
    out
}

/// Where 0 dBFS sits in a `dax_iq` payload: the floats are full-scale at
/// **2^15**, not at 1.0, so a client has to divide by this to get the ±1.0 a
/// receive chain expects.
///
/// It is a float carrying an integer's scale — FlexRadio's own description of
/// DAX IQ is "32-bit fixed point, left justified", and what crosses the wire is
/// that number in a float. Nothing in the protocol says so; the constant is
/// `hb9fxq/flexlib-go`'s `ONE_OVER_ZERO_DBFS = 1/2^15`, applied in its
/// `ParseFData` to exactly these packet classes.
///
/// Missing it is 90.3 dB of gain that nothing downstream can undo: a FLEX-6600
/// showed its band noise at the top of the panadapter, the waterfall saturated
/// end to end and the S-meter pinned at S9+109 with the overload light on
/// (issue #184). Auto-fit cannot rescue that — the picture is not badly
/// *scaled*, it is genuinely at +0 dBFS and clipping.
pub const DAX_IQ_FULL_SCALE: f32 = 32_768.0;

/// Decode a `dax_iq` payload into interleaved I/Q floats appended to `out`,
/// normalised to ±1.0 full scale.
///
/// **Little**-endian, alone among the stream types — see the module docs. A
/// truncated final sample pair is dropped rather than half-decoded. The scale
/// is [`DAX_IQ_FULL_SCALE`].
pub fn decode_dax_iq(payload: &[u8], out: &mut Vec<f32>) {
    for chunk in payload.chunks_exact(4) {
        out.push(f32::from_le_bytes(chunk.try_into().unwrap()) / DAX_IQ_FULL_SCALE);
    }
    // Keep the interleave aligned: a partial I with no Q would swap the two for
    // the whole rest of the stream.
    if out.len() % 2 == 1 {
        out.pop();
    }
}

/// Build a `dax_iq` payload from interleaved I/Q floats (the simulator's side of
/// [`decode_dax_iq`]), taking ±1.0 full scale and putting the radio's own
/// [`DAX_IQ_FULL_SCALE`] on the wire.
pub fn encode_dax_iq(samples: &[f32], out: &mut Vec<u8>) {
    for &s in samples {
        out.extend_from_slice(&(s * DAX_IQ_FULL_SCALE).to_le_bytes());
    }
}

/// One meter reading straight off the wire, before scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawMeter {
    pub id: u16,
    pub raw: i16,
}

/// Decode a meter packet payload: N × (uint16 id, int16 raw), big-endian.
pub fn decode_meters(payload: &[u8], out: &mut Vec<RawMeter>) {
    for chunk in payload.chunks_exact(4) {
        out.push(RawMeter {
            id: u16::from_be_bytes(chunk[0..2].try_into().unwrap()),
            raw: i16::from_be_bytes(chunk[2..4].try_into().unwrap()),
        });
    }
}

/// Encode meter readings as a packet payload (the simulator's side of
/// [`decode_meters`]).
pub fn encode_meters(meters: &[RawMeter], out: &mut Vec<u8>) {
    for m in meters {
        out.extend_from_slice(&m.id.to_be_bytes());
        out.extend_from_slice(&m.raw.to_be_bytes());
    }
}

/// Scale a raw meter reading by its declared unit.
///
/// The divisor is a property of the unit, not of the meter: the radio sends
/// fixed-point integers and the exponent differs per unit (FlexLib `Meter.cs`).
/// An unknown unit is passed through unscaled rather than guessed at — a wrong
/// divisor would produce a plausible number, which is worse than an odd one.
pub fn scale_meter(raw: i16, unit: &str) -> f32 {
    let v = raw as f32;
    match unit.to_ascii_lowercase().as_str() {
        "dbm" | "db" | "dbfs" | "swr" => v / 128.0,
        "volts" | "amps" => v / 1024.0,
        "degf" | "degc" => v / 64.0,
        // "rpm", "watts" and anything a future firmware adds.
        _ => v,
    }
}

/// Watts from dBm — the unit a FLEX reports its forward power meter in, as
/// FlexLib converts it.
pub fn dbm_to_watts(dbm: f32) -> f32 {
    10f32.powf((dbm - 30.0) / 10.0)
}

/// Whether the `info` reply says the radio has a built-in antenna tuner, or
/// `None` where it does not say.
///
/// The reply is the protocol's one comma-separated body, with quoted strings:
/// `model="FLEX-8400",…,atu_present=1,num_scu=1,…`.
pub fn info_atu_present(body: &str) -> Option<bool> {
    parse_kvs_with(body, ',').get("atu_present").map(|v| v.trim().trim_matches('"') != "0")
}

/// The tuner's state after an `atu` status object, applied over `prev`, or
/// `None` if the object says nothing about the tuner.
///
/// An update carries only what it carries: a FLEX-8400 sends
/// `status=TUNE_SUCCESSFUL atu_enabled=1 memories_enabled=1 using_mem=1` to
/// every client and, to the one that asked for the tune, the same without
/// `atu_enabled`. Reading an absent field as `0` turned a tuner that had just
/// matched into one that looked bypassed, so a field that is not there keeps
/// what it was. The status names are FlexLib's; one this code has never seen
/// reads as [`AtuStatus::Idle`], which claims nothing.
pub fn update_atu(prev: AtuState, kvs: &Kvs) -> Option<AtuState> {
    let flag = |k: &str| kvs.get(k).map(|v| v.trim() != "0");
    let status = kvs.get("status").map(|s| match s.as_str() {
        "TUNE_IN_PROGRESS" => AtuStatus::Tuning,
        "TUNE_SUCCESSFUL" | "TUNE_OK" => AtuStatus::Tuned,
        "TUNE_BYPASS" | "TUNE_MANUAL_BYPASS" => AtuStatus::Bypass,
        "TUNE_FAIL" | "TUNE_FAIL_BYPASS" => AtuStatus::Failed,
        "TUNE_ABORTED" => AtuStatus::Aborted,
        _ => AtuStatus::Idle,
    });
    let (enabled, using_memory) = (flag("atu_enabled"), flag("using_mem"));
    if status.is_none() && enabled.is_none() && using_memory.is_none() {
        return None;
    }
    Some(AtuState {
        status: status.unwrap_or(prev.status),
        enabled: enabled.unwrap_or(prev.enabled),
        using_memory: using_memory.unwrap_or(prev.using_memory),
    })
}

/// Whether an `interlock` status says the radio is on the air, or `None` if it
/// carries no `state` — the interlock's configuration, which arrives under the
/// same object name.
///
/// `PTT_REQUESTED` counts: the radio has committed to keying and its relays are
/// already moving, which is the click the receiver would otherwise play.
/// `UNKEY_REQUESTED` does not, so an over of sdroxide's own — which ends in
/// exactly that state — never reads as the radio going on the air by itself.
pub fn interlock_on_air(kvs: &Kvs) -> Option<bool> {
    kvs.get("state").map(|s| matches!(s.as_str(), "PTT_REQUESTED" | "TRANSMITTING"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The meter definitions as a FLEX-8400 sends them, descriptions with
    /// spaces and all.
    #[test]
    fn reads_meter_definitions_with_spaces_in_them() {
        let Line::Status { object, kvs, .. } = parse_line(
            "S0|meter 8.src=TX-#8.num=1#8.nam=FWDPWR#8.low=0.0#8.hi=53.0#8.desc=RF Power \
             Forward#8.unit=dBm#8.fps=20#",
        ) else {
            panic!("not a status");
        };
        assert_eq!(object, "meter");
        assert_eq!(kvs.get("8.nam").map(String::as_str), Some("FWDPWR"));
        assert_eq!(kvs.get("8.unit").map(String::as_str), Some("dBm"));
        assert_eq!(kvs.get("8.desc").map(String::as_str), Some("RF Power Forward"));
    }

    #[test]
    fn converts_forward_power_to_watts() {
        assert!((dbm_to_watts(50.0) - 100.0).abs() < 0.01);
        assert!((dbm_to_watts(30.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn reads_whether_the_radio_has_a_tuner() {
        let reply = r#"model="FLEX-8400",chassis_serial="1725",name="Marco",gps="Locked",atu_present=1,num_scu=1,software_ver=4.2.20.41343,"#;
        assert_eq!(info_atu_present(reply), Some(true));
        assert_eq!(info_atu_present(r#"model="FLEX-6400",atu_present=0,"#), Some(false));
        assert_eq!(info_atu_present(r#"model="FLEX-6600",num_scu=2"#), None);
    }

    #[test]
    fn reads_the_tuners_state() {
        let Line::Status { kvs, .. } = parse_line(
            "S0|atu status=TUNE_SUCCESSFUL atu_enabled=1 memories_enabled=1 using_mem=1",
        ) else {
            panic!("not a status");
        };
        let tuned = AtuState { status: AtuStatus::Tuned, enabled: true, using_memory: true };
        assert_eq!(update_atu(AtuState::default(), &kvs), Some(tuned));
        let in_progress = parse_kvs("status=TUNE_IN_PROGRESS atu_enabled=1 using_mem=0");
        assert_eq!(update_atu(tuned, &in_progress).map(|a| a.status), Some(AtuStatus::Tuning));
        let bypass = parse_kvs("status=TUNE_BYPASS atu_enabled=0");
        assert_eq!(
            update_atu(tuned, &bypass),
            Some(AtuState { status: AtuStatus::Bypass, enabled: false, using_memory: true })
        );
        // The copy a FLEX-8400 sends the client that asked for the tune, with
        // no `atu_enabled`: the tuner is still in circuit.
        let to_us = parse_kvs("status=TUNE_SUCCESSFUL memories_enabled=1 using_mem=1");
        assert_eq!(update_atu(tuned, &to_us), Some(tuned));
        // Nothing about the tuner at all.
        assert_eq!(update_atu(tuned, &parse_kvs("memories_enabled=0")), None);
    }

    #[test]
    fn reads_whether_the_radio_is_on_the_air() {
        let on = |s: &str| interlock_on_air(&parse_kvs(s));
        // The sequence a FLEX-8400 sent through an ATU tune.
        assert_eq!(on("tx_client_handle=0x3D499A76 state=PTT_REQUESTED source=SW"), Some(true));
        assert_eq!(on("tx_client_handle=0x3D499A76 state=TRANSMITTING source=SW"), Some(true));
        assert_eq!(on("tx_client_handle=0x3D499A76 state=UNKEY_REQUESTED source="), Some(false));
        assert_eq!(on("tx_client_handle=0x00000000 state=READY source="), Some(false));
        assert_eq!(on("state=RECEIVE tx_allowed=0"), Some(false));
        // The interlock's configuration, under the same object name.
        assert_eq!(on("acc_txreq_enable=0 rca_txreq_enable=0 tx_delay=0"), None);
    }

    #[test]
    fn parses_the_five_line_tags() {
        assert_eq!(parse_line("V3.3.28.0"), Line::Version("3.3.28.0".into()));
        assert_eq!(parse_line("H1A2B3C4D"), Line::Handle(0x1A2B3C4D));
        assert_eq!(
            parse_line("R7|0|0x04000001"),
            Line::Response { seq: 7, code: 0, body: "0x04000001".into() }
        );
        assert!(matches!(
            parse_line("M10000001|Some notice"),
            Line::Message { severity: Severity::Info, .. }
        ));
        assert!(matches!(parse_line("Q?????"), Line::Unknown(_)));
    }

    #[test]
    fn result_codes_are_unsigned_hex() {
        // The fatal codes have the high bit set. Parsed as a signed decimal they
        // come back 0, turning a hard rejection into an apparent success.
        let Line::Response { code, .. } = parse_line("R3|F3000001|too many clients") else {
            panic!("not a response");
        };
        assert_eq!(code, 0xF300_0001);
        assert_ne!(code, 0);

        let Line::Response { code, .. } = parse_line("R4|50001001|No Such Object") else {
            panic!("not a response");
        };
        assert_eq!(code, 0x5000_1001);
    }

    #[test]
    fn splits_multi_word_status_objects() {
        let Line::Status { handle, object, kvs } =
            parse_line("S1A2B3C4D|display pan 0x40000000 center=14.100 bandwidth=0.192")
        else {
            panic!("not a status");
        };
        assert_eq!(handle, 0x1A2B3C4D);
        assert_eq!(object, "display pan 0x40000000");
        assert_eq!(kvs["center"], "14.100");
        assert_eq!(kvs["bandwidth"], "0.192");

        let Line::Status { object, kvs, .. } =
            parse_line("S0|slice 0 RF_frequency=14.074000 mode=USB in_use=1")
        else {
            panic!("not a status");
        };
        assert_eq!(object, "slice 0");
        assert_eq!(kvs["RF_frequency"], "14.074000");
        assert_eq!(kvs["in_use"], "1");
    }

    #[test]
    fn meter_definitions_use_hash_separators() {
        // Meter status is the one object that does not separate KVs with spaces.
        let Line::Status { object, kvs, .. } =
            parse_line("S1A2B3C4D|meter 7.src=SLC#7.num=0#7.nam=LEVEL#7.unit=dBm#7.low=-150.0")
        else {
            panic!("not a status");
        };
        assert_eq!(object, "meter");
        assert_eq!(kvs["7.src"], "SLC");
        assert_eq!(kvs["7.nam"], "LEVEL");
        assert_eq!(kvs["7.unit"], "dBm");
    }

    #[test]
    fn status_with_no_object_prefix_is_all_kvs() {
        let Line::Status { object, kvs, .. } = parse_line("S1|freq=14.2 mode=USB") else {
            panic!("not a status");
        };
        assert_eq!(object, "");
        assert_eq!(kvs["mode"], "USB");
    }

    #[test]
    fn bare_tokens_survive_kv_parsing() {
        let kvs = parse_kvs("removed in_use=0");
        assert!(kvs.contains_key("removed"));
        assert_eq!(kvs["in_use"], "0");
    }

    #[test]
    fn vita_header_round_trips() {
        let mut buf = Vec::new();
        write_vita_header(&mut buf, 1, 0x0400_0001, pcc::IF_NARROW, 5, 4);
        let h = parse_vita_header(&buf).expect("header");
        assert_eq!(h.packet_type, 1);
        assert_eq!(h.stream_id, 0x0400_0001);
        assert_eq!(h.class_code, pcc::IF_NARROW);
        assert_eq!(h.count, 5);
        assert_eq!(h.words, 11);
        assert!(!h.has_trailer);
    }

    /// The seven-word header this backend takes the payload from is what the
    /// indicator bits in word 0 describe — stream ID, class ID and both
    /// timestamps. A packet that says otherwise has its payload read from the
    /// wrong offset, and one word out is one float out: I read as Q for the
    /// whole packet, which is a mirrored spectrum (issue #368).
    #[test]
    fn the_header_length_the_bits_declare_is_the_one_the_payload_is_taken_from() {
        let mut buf = Vec::new();
        write_vita_header(&mut buf, 3, 0x2000_0000, pcc::DAX_IQ_192K, 0, 4);
        let h = parse_vita_header(&buf).expect("header");
        assert!(h.has_class_id);
        assert_ne!(h.tsi, 0);
        assert_ne!(h.tsf, 0);
        assert_eq!(h.declared_header_words(), VITA_HEADER_WORDS);
        assert_eq!(VITA_HEADER_WORDS * 4, VITA_HEADER_BYTES);

        // And the shapes that would not be: each is shorter than the seven
        // words assumed, so the payload would start that many words late.
        let reparse = |mask: u32, set: u32| {
            let mut b = buf.clone();
            let w0 = (u32::from_be_bytes(b[..4].try_into().unwrap()) & !mask) | set;
            b[..4].copy_from_slice(&w0.to_be_bytes());
            parse_vita_header(&b).expect("header").declared_header_words()
        };
        assert_eq!(reparse(1 << 27, 0), VITA_HEADER_WORDS - 2, "no class ID");
        assert_eq!(reparse(0x3 << 22, 0), VITA_HEADER_WORDS - 1, "no integer timestamp");
        assert_eq!(reparse(0x3 << 20, 0), VITA_HEADER_WORDS - 2, "no fractional timestamp");
        // An even packet type carries no stream ID at all.
        assert_eq!(reparse(0xF << 28, 2 << 28), VITA_HEADER_WORDS - 1, "no stream ID");
    }

    #[test]
    fn short_datagram_is_not_a_header() {
        assert!(parse_vita_header(&[0u8; 27]).is_none());
    }

    #[test]
    fn dax_iq_is_little_endian() {
        // Reading this payload big-endian yields denormals ≈ 0 — a silent
        // receiver rather than a visible error, which is why it has its own test.
        let samples = [1.0f32, -0.5, 0.25, 0.0];
        let mut wire = Vec::new();
        encode_dax_iq(&samples, &mut wire);
        assert_eq!(&wire[0..4], &DAX_IQ_FULL_SCALE.to_le_bytes());

        let mut back = Vec::new();
        decode_dax_iq(&wire, &mut back);
        assert_eq!(back, samples);
    }

    /// The radio's floats are full scale at 2^15, and a client that takes them
    /// at face value is 90 dB hot — a saturated waterfall and a pinned S-meter
    /// (issue #184). Pinned as a number rather than left to the round trip
    /// above, which would pass just as happily with no scaling at either end.
    #[test]
    fn dax_iq_full_scale_is_two_to_the_fifteenth() {
        assert_eq!(DAX_IQ_FULL_SCALE, 32_768.0);
        let mut back = Vec::new();
        // One I/Q pair at the radio's own full scale reads as ±1.0 here.
        let wire: Vec<u8> = [32_768.0f32, -32_768.0].iter().flat_map(|s| s.to_le_bytes()).collect();
        decode_dax_iq(&wire, &mut back);
        assert_eq!(back, vec![1.0, -1.0]);
    }

    #[test]
    fn dax_iq_drops_a_dangling_i_sample() {
        // A truncated pair would swap I and Q for the rest of the stream.
        let mut wire = Vec::new();
        encode_dax_iq(&[1.0, 0.5, 0.25], &mut wire);
        let mut back = Vec::new();
        decode_dax_iq(&wire, &mut back);
        assert_eq!(back, vec![1.0, 0.5]);
    }

    #[test]
    fn tx_audio_packet_is_stereo_big_endian() {
        let pkt = build_dax_tx_audio(0x0B00_0002, 3, &[0.5, -0.5]);
        let h = parse_vita_header(&pkt).expect("header");
        assert_eq!(h.packet_type, 1); // IFDataWithStream, not ExtData
        assert_eq!(h.class_code, pcc::IF_NARROW);
        let pay = vita_payload(&pkt, &h);
        assert_eq!(pay.len(), 2 * 2 * 4); // 2 frames × stereo × f32
        assert_eq!(&pay[0..4], &0.5f32.to_be_bytes());
        assert_eq!(&pay[4..8], &0.5f32.to_be_bytes()); // duplicated to R
        assert_eq!(&pay[8..12], &(-0.5f32).to_be_bytes());
    }

    #[test]
    fn dax_iq_class_codes_carry_the_rate() {
        for rate in [24_000, 48_000, 96_000, 192_000] {
            assert_eq!(dax_iq_rate(dax_iq_class_code(rate)), Some(rate));
        }
        assert_eq!(dax_iq_rate(pcc::IF_NARROW), None);
        // An unsupported rate falls back to the radio's own default.
        assert_eq!(dax_iq_class_code(11_025), pcc::DAX_IQ_48K);
    }

    #[test]
    fn meters_round_trip() {
        let ms = [RawMeter { id: 7, raw: -9000 }, RawMeter { id: 12, raw: 192 }];
        let mut wire = Vec::new();
        encode_meters(&ms, &mut wire);
        let mut back = Vec::new();
        decode_meters(&wire, &mut back);
        assert_eq!(back, ms);
    }

    #[test]
    fn meter_scaling_follows_the_unit() {
        assert_eq!(scale_meter(-9000, "dBm"), -70.3125);
        assert_eq!(scale_meter(192, "SWR"), 1.5);
        assert_eq!(scale_meter(1024, "Volts"), 1.0);
        assert_eq!(scale_meter(64, "degC"), 1.0);
        // Unknown units pass through rather than being guessed at.
        assert_eq!(scale_meter(50, "RPM"), 50.0);
    }

    #[test]
    fn packet_counter_gap_wraps_at_four_bits() {
        assert_eq!(count_gap(3, 4), 0); // in sequence
        assert_eq!(count_gap(3, 6), 2); // two lost
        assert_eq!(count_gap(15, 0), 0); // wrapped, in sequence
        assert_eq!(count_gap(14, 1), 2); // wrapped, two lost
    }

    #[test]
    fn hex_ids_round_trip() {
        assert_eq!(hex_id(0x4000_0000), "0x40000000");
        assert_eq!(parse_hex_id("0x40000000"), Some(0x4000_0000));
        assert_eq!(parse_hex_id("40000000"), Some(0x4000_0000));
        assert_eq!(parse_hex_id("nope"), None);
    }

    #[test]
    fn command_lines_are_sequenced_and_newline_terminated() {
        assert_eq!(build_command(12, "slice tune 0 14.074"), "C12|slice tune 0 14.074\n");
    }
}
