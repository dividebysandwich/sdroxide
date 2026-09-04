//! The mfsk-core adapter: FT8/FT4 decode and encode wrapped behind stable
//! sdroxide types. **This is the only file that touches raw mfsk-core
//! decode-result fields** — if the crate's field names change, only
//! `decode_slot` needs updating.

use mfsk_core::msg::decode_request::DecodeRequest;
use mfsk_core::msg::hash_table::CallsignHashTable;
use mfsk_core::msg::wsjt77;
use sdroxide_types::{Decode, Mode};

use crate::params::{AUDIO_MAX_HZ, AUDIO_MIN_HZ};

const SYNC_MIN: f32 = 1.5;
/// How many sync candidates a slot's decode is allowed to try.
///
/// The list is sorted by sync power and cut here, so on a quiet band this is
/// never reached and on a busy one it decides which of the weak signals are
/// never looked at — which is exactly the shape of issue #307, where the gap
/// against WSJT-X grew with the number of stations on the band.
///
/// Measured on a synthetic forty-signal slot: 120 candidates found thirteen
/// messages, 300 found fourteen and 600 found fifteen, for 9.6, 10.2 and
/// 11.8 milliseconds of decode. A slot is fifteen seconds long, so the whole
/// range is free and the only question is how many are looked at.
const MAX_CAND: usize = 600;

/// Which of the 77-bit message layouts a decode came from, read straight from
/// the `i3`/`n3` type bits. Guessing this from the text can't work — free text
/// is allowed to look exactly like a standard message ("W1AW RR73" is both a
/// valid free-text string and a valid exchange), so the bits decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MsgKind {
    /// 13 characters of arbitrary text — no addressee, no sender.
    FreeText,
    /// DXpedition (Fox): `CALL1 RR73; CALL2 <fox> REPORT`.
    Fox,
    /// ARRL Field Day: `CALL1 CALL2 [FD]`.
    FieldDay,
    /// The everyday exchange: `<to> <from> <grid|report|RRR|RR73|73>`.
    Standard,
    /// ARRL RTTY Roundup: `[TU; ]<to> <from> [R] RST <state|serial>`.
    RttyRu,
    /// One non-standard (compound / long) callsign plus one hashed one.
    NonStandard,
    /// EU VHF contest: `<to> <from> [R] RSNNNN GRID6`, both calls hashed.
    EuVhf,
    /// A layout this build doesn't model.
    Other,
}

/// Read the `i3` (bits 74–76) and `n3` (bits 71–73) message-type fields.
fn msg_kind(bits77: &[u8; 77]) -> MsgKind {
    let field = |start: usize| {
        (bits77[start] & 1) << 2 | (bits77[start + 1] & 1) << 1 | (bits77[start + 2] & 1)
    };
    match (field(74), field(71)) {
        (0, 0) => MsgKind::FreeText,
        (0, 1) => MsgKind::Fox,
        (0, 3 | 4) => MsgKind::FieldDay,
        (1 | 2, _) => MsgKind::Standard,
        (3, _) => MsgKind::RttyRu,
        (4, _) => MsgKind::NonStandard,
        (5, _) => MsgKind::EuVhf,
        _ => MsgKind::Other,
    }
}

/// What we already know about the message we are hoping to decode.
///
/// Every message that advances a QSO is addressed to us, and once we are
/// working someone it comes from them as well — so 58 of the 77 bits are known
/// before the signal arrives. Handing them to the decoder as *a-priori* bits
/// lets the LDPC stage treat them as given instead of solving for them, which
/// is worth several dB on the one message we most want to hear.
///
/// It cannot mislead us: mfsk-core only attempts the AP decode after an
/// ordinary one has already failed, and the result still has to pass CRC-14. A
/// stale or wrong hint costs the attempt and nothing else.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApHints {
    /// Our own callsign — the addressee of everything we are waiting for.
    pub my_call: String,
    /// The station we are working, if any.
    pub dx_call: Option<String>,
    /// The operator has the EU VHF contest selected, so the slot is worth a
    /// second pass for the `i3 = 5` exchange mfsk-core's FT8 decoder cannot
    /// return — see [`crate::ft8_eu`] (issue #223).
    ///
    /// Here rather than in the modem's own state because it belongs to the
    /// slot: the decode worker is handed one of these per slot and holds
    /// nothing else about how the station is set up.
    pub eu_vhf: bool,
}

impl ApHints {
    /// The callsigns to seed the hash table with, so a hashed `<...>` naming
    /// either of them resolves on first sight.
    pub fn calls(&self) -> Vec<String> {
        [Some(self.my_call.as_str()), self.dx_call.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// `(call1, call2)` for the message layout: `<to> <from> …`, so our call is
    /// first and the station we're working second. `None` when we don't even
    /// know our own callsign, which is the only case with nothing to say.
    fn calls_for_hint(&self) -> Option<(&str, Option<&str>)> {
        let me = self.my_call.trim();
        if me.is_empty() {
            return None;
        }
        Some((me, self.dx_call.as_deref().map(str::trim).filter(|c| !c.is_empty())))
    }

    fn ft8(&self) -> Option<mfsk_core::ft8::decode::ApHint> {
        let (me, dx) = self.calls_for_hint()?;
        let h = mfsk_core::ft8::decode::ApHint::new().with_call1(me);
        Some(match dx {
            Some(dx) => h.with_call2(dx),
            None => h,
        })
    }

    fn ft4(&self) -> Option<mfsk_core::msg::ApHint> {
        let (me, dx) = self.calls_for_hint()?;
        let h = mfsk_core::msg::ApHint::new().with_call1(me);
        Some(match dx {
            Some(dx) => h.with_call2(dx),
            None => h,
        })
    }
}

/// Encode/decode engine for one digital mode.
pub struct Ft8Modem {
    mode: Mode,
    /// Callsigns heard this session, so the `<...>` placeholders in
    /// non-standard-callsign and DXpedition messages resolve to real calls.
    hashes: CallsignHashTable,
    /// The same callsigns again, hashed the way the EU VHF contest layout
    /// needs them — see [`eu_vhf::Hashes`].
    eu_hashes: eu_vhf::Hashes,
}

impl Ft8Modem {
    pub fn new(mode: Mode) -> Self {
        Ft8Modem { mode, hashes: CallsignHashTable::new(), eu_hashes: eu_vhf::Hashes::default() }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Register callsigns we already know (ours, and the station we're
    /// working), so the first hashed message naming them resolves instead of
    /// showing `<...>`.
    pub fn seed_hashes(&mut self, calls: &[String]) {
        for c in calls.iter().filter(|c| !c.is_empty()) {
            self.hashes.insert(c);
            self.eu_hashes.insert(c);
        }
    }

    /// Decode one full receive slot of 12 kHz mono i16 audio.
    ///
    /// FT8 and FT4 take different routes through mfsk-core — only FT8 has a
    /// wide-band a-priori pass — so each is mapped to our stable [`Decode`]
    /// inside its own arm, the only place that reads raw mfsk-core fields.
    /// (The two shared one result type as of mfsk-core 0.8; before that they
    /// were separate types, which is why the arms were split to begin with.)
    ///
    /// `ap` is what we already know about the message we are waiting for (see
    /// [`ApHints`]); `listening_hz` is where we are tuned, which is the only
    /// place FT4's a-priori pass can search. The FT8 pass applies the hint
    /// across the whole band, so it ignores `listening_hz`.
    ///
    /// Takes `&mut self` because every callsign heard is folded into the hash
    /// table, which is what lets a later `<...>` message name a real station.
    pub fn decode_slot(
        &mut self,
        audio_12k: &[i16],
        slot_utc: i64,
        ap: &ApHints,
        listening_hz: f32,
    ) -> Vec<Decode> {
        let mode = self.mode;
        let ht = &self.hashes;
        let eu = &self.eu_hashes;
        let mut decodes: Vec<Decode> = match mode {
            Mode::Ft4 => DecodeRequest::<mfsk_core::Ft4>::new(
                audio_12k,
                AUDIO_MIN_HZ,
                AUDIO_MAX_HZ,
                SYNC_MIN,
                MAX_CAND,
            )
            .osd(true)
            .decode()
            .results
            .into_iter()
            .filter_map(|r| {
                let bits: [u8; 77] = r.message77().try_into().ok()?;
                build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
            })
            .collect(),
            // FT2 is FT4 at double the symbol rate, but mfsk-core ships no FT2
            // protocol, so the pipeline is reassembled from its public generic
            // stages in [`crate::ft2`]. No a-priori pass yet — the sniper entry
            // point it would need is `pub(crate)` in mfsk-core, and FT2 is a
            // strong-signal mode where AP buys least.
            Mode::Ft2 => crate::ft2::decode::decode_slot(
                audio_12k,
                AUDIO_MIN_HZ,
                AUDIO_MAX_HZ,
                SYNC_MIN,
                MAX_CAND,
            )
            .into_iter()
            .filter_map(|r| {
                let bits: [u8; 77] = r.message77().try_into().ok()?;
                build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
            })
            .collect(),
            _ => {
                // With no hint this is bit-for-bit the plain wide-band decode;
                // with one, every candidate that fails an ordinary decode gets a
                // second attempt with our two callsigns' bits locked.
                let hint = ap.ft8();
                let req = DecodeRequest::<mfsk_core::Ft8>::new(
                    audio_12k,
                    AUDIO_MIN_HZ,
                    AUDIO_MAX_HZ,
                    SYNC_MIN,
                    MAX_CAND,
                )
                .osd(true);
                let req = match hint.as_ref() {
                    Some(h) => req.ap_hint(h),
                    None => req,
                };
                req.decode()
                    .results
                    .into_iter()
                    .filter_map(|r| {
                        let bits: [u8; 77] = r.message77().try_into().ok()?;
                        build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
                    })
                    .collect()
            }
        };
        // FT4 has no wide-band a-priori pass, only a targeted one. Aim it where
        // we are listening — in a QSO that is the station whose reply the hint
        // describes — and keep whatever it finds that the wide pass missed.
        if mode == Mode::Ft4 {
            if let Some(hint) = ap.ft4() {
                let extra =
                    DecodeRequest::<mfsk_core::Ft4>::sniper(audio_12k, listening_hz, MAX_CAND)
                        // The threshold the old `decode_sniper_ap` entry point applied
                        // for us, and looser than the sniper default: with the hint's
                        // bits locked the FEC can carry a candidate whose coarse sync
                        // would never qualify on its own.
                        .sync_min(0.5)
                        .osd(true)
                        .eq_mode(mfsk_core::engine::equalize::EqMode::Off)
                        .ap_hint(&hint)
                        .decode()
                        .results
                        .into_iter()
                        .filter_map(|r| {
                            let bits: [u8; 77] = r.message77().try_into().ok()?;
                            build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
                        })
                        .filter(|d| !decodes.iter().any(|o| same_signal(o, d)))
                        .collect::<Vec<_>>();
                decodes.extend(extra);
            }
        }
        // The EU VHF contest exchange, which mfsk-core's FT8 decoder drops
        // inside itself — see [`crate::ft8_eu`] for why there is nothing to
        // filter and no hook to catch it with (issue #223). A second pass over
        // the same slot, run only with the contest selected because it costs
        // about what the first one does, and one that can only ever return the
        // `i3 = 5` layout the first pass is structurally unable to produce.
        //
        // FT8 alone. FT4 and FT2 go through a pipeline that never unpacks and
        // so never drops the layout; that is exactly why the exchange was
        // reported working there and nowhere else.
        if ap.eu_vhf && !matches!(mode, Mode::Ft4 | Mode::Ft2) {
            let rescued = crate::ft8_eu::decode_slot(
                audio_12k,
                AUDIO_MIN_HZ,
                AUDIO_MAX_HZ,
                SYNC_MIN,
                MAX_CAND,
            );
            for r in rescued {
                if let Some(d) =
                    build_decode(&r.msg77, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
                    && !decodes.iter().any(|o| same_signal(o, &d))
                {
                    decodes.push(d);
                }
            }
        }
        // Remember who we heard, for the next slot's hashed messages.
        for d in &decodes {
            for call in [d.to.as_deref(), d.from.as_deref()].into_iter().flatten() {
                self.hashes.insert(call);
                self.eu_hashes.insert(call);
            }
        }
        decodes
    }

    /// Synthesize a message into 12 kHz mono f32 burst audio at tone offset
    /// `audio_hz`. Returns the audio and the message *as it will be received* —
    /// a form FT8 can't carry in full (a report to a compound call, an
    /// over-long free-text line) is degraded rather than dropped, and the
    /// caller logs what actually went out. `None` if nothing can be packed.
    pub fn encode_burst_12k(
        &self,
        text: &str,
        audio_hz: f32,
        amplitude: f32,
    ) -> Option<(Vec<f32>, String)> {
        let (msg77, sent) = pack_message(text)?;
        let audio = match self.mode {
            Mode::Ft4 => {
                let tones = mfsk_core::ft4::encode::message_to_tones(&msg77);
                mfsk_core::ft4::encode::tones_to_f32(&tones, audio_hz, amplitude)
            }
            Mode::Ft2 => {
                let tones = crate::ft2::encode::message_to_tones(&msg77);
                crate::ft2::encode::tones_to_f32(&tones, audio_hz, amplitude)
            }
            _ => {
                let tones = mfsk_core::ft8::wave_gen::message_to_tones(&msg77);
                mfsk_core::ft8::wave_gen::tones_to_f32(&tones, audio_hz, amplitude)
            }
        };
        Some((audio, sent))
    }
}

/// Pack `text` into a 77-bit message, degrading to the nearest layout FT8 can
/// carry, and report the message as the far end will read it.
///
/// The ladder is: the DXpedition (Fox) layout when the text is written as one,
/// then the EU VHF contest exchange, then a directed CQ, then the standard
/// exchange, then the non-standard-callsign layout (which can only carry
/// `RRR` / `RR73` / `73`, so a grid or report is dropped), then 13 characters
/// of free text.
fn pack_message(text: &str) -> Option<([u8; 77], String)> {
    let text = text.trim().to_ascii_uppercase();
    let toks: Vec<&str> = text.split_whitespace().collect();
    let (c1, c2, payload) = (
        toks.first().copied().unwrap_or(""),
        toks.get(1).copied().unwrap_or(""),
        toks.get(2).copied().unwrap_or(""),
    );

    // 0. DXpedition (Fox): "CALL1 RR73; CALL2 FOXCALL REPORT" closes one contact
    //    and reports to the next in a single transmission. Only a Fox writes
    //    this, and only ever with its own call third — the layout hashes it.
    if c2 == "RR73;" && toks.len() == 5 {
        if let Some((m, rpt)) = pack77_fox(c1, toks[2], toks[3], toks[4]) {
            return Some((m, format!("{c1} RR73; {} <{}> {rpt}", toks[2], toks[3])));
        }
    }

    // 1. The EU VHF contest exchange (issue #223):
    //    "<TO> <FROM> [R] 590003 JO22DB". Ahead of everything below it because
    //    it is the only layout that will carry a six-character locator, and
    //    nothing else here would recognise the exchange at all — it would go
    //    out as thirteen characters of free text.
    if matches!(toks.len(), 4 | 5) && c1.starts_with('<') && c2.starts_with('<') {
        let rogered = toks[2] == "R";
        let rest = &toks[if rogered { 3 } else { 2 }..];
        if let ([exch, grid6], Some((rs, serial))) =
            (rest, rest.first().and_then(|t| eu_vhf::parse_exchange(t)))
            && let Some(m) =
                eu_vhf::pack(eu_vhf::bare(c1), eu_vhf::bare(c2), rogered, rs, serial, grid6)
        {
            let r = if rogered { "R " } else { "" };
            return Some((m, format!("{c1} {c2} {r}{exch} {grid6}")));
        }
    }

    // 2. A directed CQ: "CQ EU AB1CD FN42". The modifier is not a fourth field
    //    — "CQ EU" packs as one token — so the message is still the everyday
    //    three, with the first of them two words long.
    if c1 == "CQ" && toks.len() == 4 && !c2.is_empty() {
        let directed = format!("CQ {c2}");
        if let Some(m) = wsjt77::pack77(&directed, toks[2], toks[3]) {
            return Some((m, format!("{directed} {} {}", toks[2], toks[3])));
        }
    }

    // 3. The everyday message, when both calls are standard.
    if toks.len() <= 3 {
        if let Some(m) = wsjt77::pack77(c1, c2, payload) {
            return Some((m, join3(c1, c2, payload)));
        }
    }

    // 4. One compound / non-standard callsign, the other one hashed. Only a
    //    bare RRR / RR73 / 73 fits alongside it — a grid or report is lost, so
    //    the returned text says so.
    let rpt = match payload {
        "RRR" | "RR73" | "73" => payload,
        _ => "",
    };
    if c1 == "CQ" && !c2.is_empty() && !wsjt77::is_standard_callsign(c2) {
        let m = wsjt77::pack77_type4(c2, "", "", true)?;
        return Some((m, format!("CQ {c2}")));
    }
    let nonstd_first = !c1.is_empty() && !wsjt77::is_standard_callsign(c1) && c1 != "CQ";
    let nonstd_second = !c2.is_empty() && !wsjt77::is_standard_callsign(c2);
    if nonstd_first != nonstd_second {
        let (nonstd, std_call) = if nonstd_first { (c1, c2) } else { (c2, c1) };
        if wsjt77::is_valid_callsign(nonstd) && wsjt77::is_standard_callsign(std_call) {
            if let Some(mut m) = wsjt77::pack77_type4(nonstd, std_call, rpt, false) {
                // Bit 70 (`iflip`) decides which call is read first. mfsk-core
                // always hashes-first, which is the layout for *being* the
                // non-standard station; addressing one needs the other order.
                m[70] = u8::from(nonstd_first);
                let sent = if nonstd_first {
                    join3(nonstd, &format!("<{std_call}>"), rpt)
                } else {
                    join3(&format!("<{std_call}>"), nonstd, rpt)
                };
                return Some((m, sent));
            }
        }
    }

    // 5. Free text: 13 characters, from a restricted alphabet.
    let free: String = text.chars().take(13).collect();
    let m = wsjt77::pack77_free_text(free.trim_end())?;
    Some((m, free.trim_end().to_string()))
}

/// Pack the DXpedition (Fox) layout — `i3=0, n3=1`:
/// `[c28 rr73_call][c28 work_call][h10 fox_call][r5 report]`, then `n3` and
/// `i3`. mfsk-core has no packer for it (only the unpacker), so the fields are
/// written here; the bit positions are the ones `wsjt77::unpack77` reads back.
///
/// The report field is 5 bits in 2 dB steps (`report = 2·n5 − 30`), so an odd
/// value lands on the nearest even one — that is the layout's resolution, not a
/// rounding choice of ours. The quantised report is returned alongside the bits,
/// because it is what the far end will read.
fn pack77_fox(
    rr73_call: &str,
    work_call: &str,
    fox_call: &str,
    report: &str,
) -> Option<([u8; 77], String)> {
    let n28a = wsjt77::pack28(rr73_call)?;
    let n28b = wsjt77::pack28(work_call)?;
    if !wsjt77::is_standard_callsign(rr73_call) || !wsjt77::is_standard_callsign(work_call) {
        return None;
    }
    let n10 = mfsk_core::msg::hash_table::ihashcall(fox_call, 10);
    let db: i32 = report.trim_start_matches('+').parse().ok()?;
    let n5 = ((db + 30) / 2).clamp(0, 31) as u32;

    let mut msg = [0u8; 77];
    let mut write = |start: usize, len: usize, val: u32| {
        for i in 0..len {
            msg[start + i] = ((val >> (len - 1 - i)) & 1) as u8;
        }
    };
    write(0, 28, n28a);
    write(28, 28, n28b);
    write(56, 10, n10);
    write(66, 5, n5);
    write(71, 3, 1); // n3 = 1 (DXpedition)
    write(74, 3, 0); // i3 = 0
    Some((msg, sdroxide_types::fmt_report(2 * n5 as i16 - 30)))
}

/// The EU VHF contest layout, `i3 = 5` (issue #223):
/// `[h12 to][h22 from][r1 rogered][s3 report][s11 serial][g25 grid6]`, then
/// `i3`. Both callsigns travel as *hashes* — twelve bits for the addressee,
/// twenty-two for the sender — which is what buys the room for a six-character
/// locator and a serial number in the same 77 bits.
///
/// mfsk-core carries neither half of this layout, so both are written here
/// against WSJT-X's `packjt77.f90` (`pack77_5` / the `i3.eq.5` arm of
/// `unpack77`). The hash function itself is mfsk-core's, the one it already
/// resolves type-4 messages with, so the two cannot come to disagree about what
/// a callsign hashes to.
///
/// A hash is not reversible: the far end reads `<...>` for any station it has
/// not heard spell its callsign out in an earlier message. That is the layout's
/// own bargain and not something to work around — it is why a contest exchange
/// is preceded by two ordinary messages that carry the calls in full.
mod eu_vhf {
    use std::collections::VecDeque;

    use mfsk_core::msg::hash_table::ihashcall;

    /// Strip the angle brackets a hashed callsign is written with.
    pub fn bare(tok: &str) -> &str {
        tok.strip_prefix('<').and_then(|t| t.strip_suffix('>')).unwrap_or(tok)
    }

    /// How many callsigns to keep, matching WSJT-X's own `MAXHASH`.
    const MAX_CALLS: usize = 1000;

    /// The callsigns heard, for resolving the hashes this layout travels as.
    ///
    /// Its own table rather than mfsk-core's `CallsignHashTable`, over a
    /// divergence that bites hardest in exactly this mode: that one strips a
    /// `/R` or `/P` suffix before hashing, where WSJT-X hashes the callsign
    /// whole (`packjt77.f90`, `save_hash_call`). A `/P` station is the typical
    /// entrant in a European VHF contest, so hashing the base call would put
    /// every one of them on a number no other program computes — and neither
    /// side would ever resolve the other's exchange.
    ///
    /// A ring of the calls themselves rather than a map of hashes: two widths
    /// are needed (12 bits for the addressee, 22 for the sender), the hash is
    /// cheap, and a thousand entries is what the reference implementation
    /// keeps. Newest first, so the most recently heard station wins a
    /// collision — which is the rule WSJT-X's own table follows.
    #[derive(Debug, Default, Clone)]
    pub struct Hashes {
        calls: VecDeque<String>,
    }

    impl Hashes {
        /// Remember a callsign, brackets and all if it has them. A placeholder
        /// nobody resolved, a token too short to be a call, and `CQ` are not
        /// callsigns and are dropped.
        pub fn insert(&mut self, call: &str) {
            let call = bare(call.trim()).to_ascii_uppercase();
            if call.len() < 3 || call == "..." || call.starts_with("CQ") {
                return;
            }
            if let Some(i) = self.calls.iter().position(|c| *c == call) {
                self.calls.remove(i);
            }
            self.calls.push_front(call);
            self.calls.truncate(MAX_CALLS);
        }

        /// The callsign whose `bits`-wide hash is `n`, if one has been heard.
        pub fn lookup(&self, n: u32, bits: u32) -> Option<&str> {
            self.calls.iter().find(|c| ihashcall(c, bits) == n).map(String::as_str)
        }
    }

    /// Serial numbers past this do not fit the eleven-bit field.
    pub const MAX_SERIAL: u32 = 2047;

    /// The exchange as it appears in a message: an RS of `5x` glued to a
    /// four-digit serial, e.g. `590003`.
    pub fn exchange(rs: u8, serial: u32) -> String {
        format!("{rs:02}{:04}", serial.min(MAX_SERIAL))
    }

    /// Split `590003` back into `(59, 3)`. `None` for anything that is not six
    /// digits with an RS the three-bit field can carry.
    pub fn parse_exchange(tok: &str) -> Option<(u8, u32)> {
        if tok.len() != 6 || !tok.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let rs: u8 = tok[..2].parse().ok()?;
        let serial: u32 = tok[2..].parse().ok()?;
        (52..=59).contains(&rs).then_some((rs, serial))
    }

    /// A six-character locator, in the narrower alphabet this layout allows:
    /// the sub-square letters run A..X, not A..Y as a bare Maidenhead locator
    /// would. Twenty-five bits hold exactly 18·18·10·10·24·24 of them.
    pub fn is_grid6(g: &str) -> bool {
        let b = g.as_bytes();
        b.len() == 6
            && (b'A'..=b'R').contains(&b[0])
            && (b'A'..=b'R').contains(&b[1])
            && b[2].is_ascii_digit()
            && b[3].is_ascii_digit()
            && (b'A'..=b'X').contains(&b[4])
            && (b'A'..=b'X').contains(&b[5])
    }

    /// Pack a locator into the 25-bit field.
    pub fn pack_grid6(g: &str) -> Option<u32> {
        if !is_grid6(g) {
            return None;
        }
        let b = g.as_bytes();
        let d = |i: usize, base: u8| u32::from(b[i] - base);
        Some(
            d(0, b'A') * 18 * 10 * 10 * 24 * 24
                + d(1, b'A') * 10 * 10 * 24 * 24
                + d(2, b'0') * 10 * 24 * 24
                + d(3, b'0') * 24 * 24
                + d(4, b'A') * 24
                + d(5, b'A'),
        )
    }

    /// The inverse of [`pack_grid6`]. `None` for a value outside the field's
    /// 18·18·10·10·24·24 range, which is a message that did not come from a
    /// packer following the standard.
    pub fn unpack_grid6(n: u32) -> Option<String> {
        let mut n = n;
        let mut take = |m: u32| {
            let v = n / m;
            n %= m;
            v
        };
        let j1 = take(18 * 10 * 10 * 24 * 24);
        let j2 = take(10 * 10 * 24 * 24);
        let j3 = take(10 * 24 * 24);
        let j4 = take(24 * 24);
        let j5 = take(24);
        let j6 = n;
        if j1 > 17 || j2 > 17 || j3 > 9 || j4 > 9 || j5 > 23 || j6 > 23 {
            return None;
        }
        Some(
            [
                (b'A' + j1 as u8) as char,
                (b'A' + j2 as u8) as char,
                (b'0' + j3 as u8) as char,
                (b'0' + j4 as u8) as char,
                (b'A' + j5 as u8) as char,
                (b'A' + j6 as u8) as char,
            ]
            .iter()
            .collect(),
        )
    }

    /// Pack `<to> <from> [R] RSNNNN GRID6` into 77 bits.
    ///
    /// The callsigns are taken bare: the angle brackets the message is written
    /// with are the *rendering* of a hash, not part of the callsign.
    pub fn pack(
        to: &str,
        from: &str,
        rogered: bool,
        rs: u8,
        serial: u32,
        grid6: &str,
    ) -> Option<[u8; 77]> {
        if !(52..=59).contains(&rs) {
            return None;
        }
        let igrid = pack_grid6(grid6)?;
        let mut msg = [0u8; 77];
        let mut write = |start: usize, len: usize, val: u32| {
            for i in 0..len {
                msg[start + i] = ((val >> (len - 1 - i)) & 1) as u8;
            }
        };
        write(0, 12, ihashcall(to, 12));
        write(12, 22, ihashcall(from, 22));
        write(34, 1, u32::from(rogered));
        write(35, 3, u32::from(rs - 52));
        write(38, 11, serial.min(MAX_SERIAL));
        write(49, 25, igrid);
        write(74, 3, 5); // i3 = 5
        Some(msg)
    }

    /// Read an `i3 = 5` message back, resolving both hashes where the table
    /// knows them. Returns the text exactly as WSJT-X renders it, so a decode
    /// list is comparable line for line with one from any other program.
    pub fn unpack(bits: &[u8; 77], hashes: &Hashes) -> Option<String> {
        let read = |start: usize, len: usize| -> u32 {
            (0..len).fold(0u32, |acc, i| acc << 1 | u32::from(bits[start + i] & 1))
        };
        let n12 = read(0, 12);
        let n22 = read(12, 22);
        let rogered = read(34, 1) == 1;
        let rs = 52 + read(35, 3) as u8;
        let serial = read(38, 11);
        let grid6 = unpack_grid6(read(49, 25))?;
        let name = |c: Option<&str>| format!("<{}>", c.unwrap_or("..."));
        let to = name(hashes.lookup(n12, 12));
        let from = name(hashes.lookup(n22, 22));
        let r = if rogered { "R " } else { "" };
        Some(format!("{to} {from} {r}{} {grid6}", exchange(rs, serial)))
    }
}

/// True when two decodes are the same transmission seen by two passes: same
/// text from the same place in the passband. The tolerance is a few Hz, so two
/// stations sending identical text at different offsets stay distinct.
fn same_signal(a: &Decode, b: &Decode) -> bool {
    a.message == b.message && (a.audio_hz - b.audio_hz).abs() < 5.0
}

/// Join up to three message tokens, dropping the empty ones.
fn join3(a: &str, b: &str, c: &str) -> String {
    [a, b, c].iter().filter(|t| !t.is_empty()).copied().collect::<Vec<_>>().join(" ")
}

/// Unpack 77 message bits and build a [`Decode`], or `None` if unpacking fails.
/// `hashes` resolves the `<...>` placeholders of hashed callsigns, and
/// `eu_hashes` the ones the EU VHF contest layout uses — see [`eu_vhf::Hashes`]
/// for why those are not the same table.
fn build_decode(
    bits77: &[u8; 77],
    snr_db: f32,
    dt_sec: f32,
    freq_hz: f32,
    slot_utc: i64,
    hashes: &CallsignHashTable,
    eu_hashes: &eu_vhf::Hashes,
) -> Option<Decode> {
    let kind = msg_kind(bits77);
    // mfsk-core does not carry `i3 = 5`, and answers `None` for it rather than
    // wrongly: without this the whole EU VHF exchange is silently invisible.
    let text = match kind {
        MsgKind::EuVhf => eu_vhf::unpack(bits77, eu_hashes)?,
        _ => wsjt77::unpack77_with_hash(bits77, hashes)?,
    };
    let p = parse_message(&text, kind);
    Some(Decode {
        slot_utc,
        snr_db: snr_db.round() as i16,
        dt: dt_sec,
        audio_hz: freq_hz,
        message: text,
        to: p.to,
        from: p.from,
        grid: p.grid,
        is_cq: p.is_cq,
        cq_to: p.cq_to,
        free_text: p.free_text,
        rr73_to: p.rr73_to,
    })
}

/// The addressing a decoded message carries.
#[derive(Debug, Default, PartialEq)]
struct Parsed {
    to: Option<String>,
    from: Option<String>,
    grid: Option<String>,
    is_cq: bool,
    cq_to: Option<String>,
    free_text: bool,
    rr73_to: Option<String>,
}

/// Pull the addressee, sender and grid out of a decoded message.
///
/// Every layout except free text is `<to> <from> [payload…]` once its
/// decorations are stripped, so the shared tail handles them all; the kind
/// (from the type bits, never guessed from the text) decides which decorations
/// to strip and whether there is any addressing at all.
fn parse_message(text: &str, kind: MsgKind) -> Parsed {
    // Free text carries no addressing, however much it may look like it does.
    if kind == MsgKind::FreeText {
        return Parsed { free_text: true, ..Default::default() };
    }
    let mut toks: Vec<&str> = text.split_whitespace().collect();
    // "TU; " opens an ARRL RTTY Roundup exchange; the rest is ordinary.
    if toks.first() == Some(&"TU;") {
        toks.remove(0);
    }
    if toks.is_empty() {
        return Parsed::default();
    }

    // A CQ names only its sender: "CQ [DX|EU|JA|POTA…] <from> [grid]". Anything
    // sitting between "CQ" and the callsign is the modifier — who the call is
    // aimed at, or what activity it belongs to.
    if toks[0] == "CQ" {
        return Parsed {
            from: toks.iter().skip(1).find(|t| is_callish(t)).and_then(|t| call_of(t)),
            grid: toks.last().filter(|t| is_grid(t)).map(|s| s.to_string()),
            is_cq: true,
            cq_to: toks.get(1).filter(|t| !is_callish(t)).map(|t| t.to_ascii_uppercase()),
            ..Default::default()
        };
    }

    // DXpedition (Fox): "CALL1 RR73; CALL2 <fox> REPORT" — CALL1's contact is
    // finished and CALL2 is being worked, both by the (hashed) fox. Read it as
    // the fox addressing CALL2, and carry CALL1's RR73 alongside: it is the only
    // thing that tells a Hound its own contact just completed.
    if toks.get(1) == Some(&"RR73;") {
        return Parsed {
            to: toks.get(2).and_then(|t| call_of(t)),
            from: toks.get(3).and_then(|t| call_of(t)),
            rr73_to: toks.first().and_then(|t| call_of(t)),
            ..Default::default()
        };
    }

    // The EU VHF contest exchange: "<TO> <FROM> [R] 590003 JO22DB". Addressed
    // like the rest, but its locator is the *six*-character one and sits at the
    // end rather than in the payload slot — which is the whole point of the
    // layout, so it is read out here rather than left behind as a report the
    // generic tail does not recognise.
    if kind == MsgKind::EuVhf {
        return Parsed {
            to: toks.first().and_then(|t| call_of(t)),
            from: toks.get(1).and_then(|t| call_of(t)),
            grid: toks.last().filter(|t| eu_vhf::is_grid6(t)).map(|s| s.to_string()),
            ..Default::default()
        };
    }

    // Standard, RTTY Roundup, Field Day and non-standard-callsign layouts all
    // put the addressee first and the sender second.
    Parsed {
        to: toks.first().and_then(|t| call_of(t)),
        from: toks.get(1).and_then(|t| call_of(t)),
        // A grid can arrive bare ("FN42") or rogered ("R FN42").
        grid: match toks.get(2) {
            Some(&"R") => toks.get(3),
            other => other,
        }
        .filter(|t| is_grid(t))
        .map(|s| s.to_string()),
        ..Default::default()
    }
}

/// The callsign a message token names: `<...>` is a hash nobody has resolved
/// (no callsign at all), `<W1AW>` is a resolved one, anything else is itself.
fn call_of(tok: &str) -> Option<String> {
    let inner = tok.strip_prefix('<').and_then(|t| t.strip_suffix('>')).unwrap_or(tok);
    (!inner.is_empty() && inner != "...").then(|| inner.to_string())
}

fn is_grid(t: &str) -> bool {
    // "RR73" is a sign-off, not a locator, though it is syntactically a valid
    // grid (and would plot at ~83°N/175°E). Never treat it as a position.
    if t == "RR73" {
        return false;
    }
    let b = t.as_bytes();
    b.len() == 4
        && (b'A'..=b'R').contains(&b[0].to_ascii_uppercase()) // Maidenhead fields A..R
        && (b'A'..=b'R').contains(&b[1].to_ascii_uppercase())
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
}

fn is_callish(t: &str) -> bool {
    let t = t.strip_prefix('<').and_then(|x| x.strip_suffix('>')).unwrap_or(t);
    // Letters *and* a digit: a numeric CQ modifier ("CQ 001") is all digits and
    // would otherwise be read as the calling station.
    t.len() >= 3
        && t.chars().any(|c| c.is_ascii_digit())
        && t.chars().any(|c| c.is_ascii_alphabetic())
        && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
}

#[cfg(test)]
mod eu_vhf_tests {
    use super::*;

    /// The worked example from WSJT-X's own `messages.txt`, both ways.
    ///
    /// The bit layout is written here rather than taken from a library, so the
    /// check that matters is against the reference *text* — a packer and an
    /// unpacker that agree with each other prove nothing at all.
    #[test]
    fn the_reference_exchange_survives_a_round_trip() {
        let mut ht = eu_vhf::Hashes::default();
        ht.insert("G4ABC/P");
        ht.insert("PA9XYZ");
        let bits = eu_vhf::pack("G4ABC/P", "PA9XYZ", true, 57, 7, "JO22DB").expect("packs");
        // i3 = 5, and nothing else in the type field.
        assert_eq!(&bits[74..77], &[1, 0, 1]);
        assert_eq!(msg_kind(&bits), MsgKind::EuVhf);
        assert_eq!(
            eu_vhf::unpack(&bits, &ht).as_deref(),
            Some("<G4ABC/P> <PA9XYZ> R 570007 JO22DB"),
        );
    }

    /// The other half of the same exchange: bare, not rogered.
    #[test]
    fn an_unrogered_exchange_reads_back_without_the_r() {
        let mut ht = eu_vhf::Hashes::default();
        ht.insert("PA9XYZ");
        ht.insert("G4ABC/P");
        let bits = eu_vhf::pack("PA9XYZ", "G4ABC/P", false, 59, 3, "IO91NP").expect("packs");
        assert_eq!(eu_vhf::unpack(&bits, &ht).as_deref(), Some("<PA9XYZ> <G4ABC/P> 590003 IO91NP"),);
    }

    /// A station nobody has heard spell its call out comes back as `<...>`,
    /// which is the layout's own bargain: the callsigns travel as hashes.
    #[test]
    fn an_unknown_hash_reads_as_a_placeholder() {
        let bits = eu_vhf::pack("PA9XYZ", "G4ABC/P", false, 59, 3, "IO91NP").expect("packs");
        assert_eq!(
            eu_vhf::unpack(&bits, &eu_vhf::Hashes::default()).as_deref(),
            Some("<...> <...> 590003 IO91NP"),
        );
    }

    /// Every locator the field can hold survives the trip, and the boundaries
    /// of the alphabet are in the set — the sub-square letters stop at X, and a
    /// packer that allowed Y would write a number the field cannot hold.
    #[test]
    fn every_locator_the_field_holds_round_trips() {
        for g in ["AA00AA", "RR99XX", "JN88DD", "IO91NP", "JO22DB"] {
            let n = eu_vhf::pack_grid6(g).unwrap_or_else(|| panic!("{g} did not pack"));
            assert!(n < 18 * 18 * 10 * 10 * 24 * 24, "{g} packed out of range: {n}");
            assert_eq!(eu_vhf::unpack_grid6(n).as_deref(), Some(g));
        }
        assert_eq!(eu_vhf::pack_grid6("JN88DY"), None, "Y is outside the field's alphabet");
        assert_eq!(eu_vhf::pack_grid6("JN88"), None, "a four-character grid is not one");
    }

    /// The exchange is a two-digit RS glued to a four-digit serial, and the RS
    /// is bounded by the three bits the field gives it.
    #[test]
    fn the_exchange_is_an_rs_and_a_serial() {
        assert_eq!(eu_vhf::parse_exchange("590003"), Some((59, 3)));
        assert_eq!(eu_vhf::parse_exchange("520001"), Some((52, 1)));
        assert_eq!(eu_vhf::parse_exchange("512047"), None, "RS 51 is below the field");
        assert_eq!(eu_vhf::parse_exchange("602047"), None, "RS 60 is above it");
        assert_eq!(eu_vhf::parse_exchange("59003"), None, "five digits is not an exchange");
        assert_eq!(eu_vhf::exchange(59, 3), "590003");
    }

    /// The message packer has to recognise the exchange for what it is. Before
    /// it did, the whole thing went out as thirteen characters of free text —
    /// which is a valid transmission and completely useless.
    #[test]
    fn the_packer_picks_the_contest_layout() {
        let (bits, sent) =
            pack_message("<PA9XYZ> <G4ABC/P> R 570007 JO22DB").expect("packs as a contest message");
        assert_eq!(msg_kind(&bits), MsgKind::EuVhf);
        assert_eq!(sent, "<PA9XYZ> <G4ABC/P> R 570007 JO22DB");
        let (bits, _) = pack_message("<PA9XYZ> <G4ABC/P> 590003 IO91NP").expect("packs");
        assert_eq!(msg_kind(&bits), MsgKind::EuVhf);
    }

    /// And a decode of one is addressed, with the six-character locator read
    /// out of it — the whole reason the layout exists.
    #[test]
    fn a_contest_decode_is_addressed_and_carries_its_locator() {
        let mut eu = eu_vhf::Hashes::default();
        eu.insert("PA9XYZ");
        eu.insert("G4ABC/P");
        let (bits, _) = pack_message("<PA9XYZ> <G4ABC/P> R 570007 JO22DB").expect("packs");
        let d = build_decode(&bits, -5.0, 0.2, 1200.0, 0, &CallsignHashTable::new(), &eu)
            .expect("decodes");
        assert_eq!(d.to.as_deref(), Some("PA9XYZ"));
        assert_eq!(d.from.as_deref(), Some("G4ABC/P"));
        assert_eq!(d.grid.as_deref(), Some("JO22DB"));
        assert!(!d.is_cq && !d.free_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a message, pad it into a full slot, and decode it back — the
    /// round trip every on-air message actually makes.
    fn round_trip(mode: Mode, text: &str) -> (String, Vec<Decode>) {
        let mut modem = Ft8Modem::new(mode);
        let (burst, sent) = modem.encode_burst_12k(text, 1500.0, 0.5).expect("encode");
        let slot_s = if mode == Mode::Ft4 { 7.5 } else { 15.0 };
        let mut slot = vec![0.0f32; (0.5 * 12_000.0) as usize];
        slot.extend_from_slice(&burst);
        slot.resize((slot_s * 12_000.0) as usize, 0.0);
        let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        (sent, modem.decode_slot(&i16buf, 0, &ApHints::default(), 1500.0))
    }

    #[test]
    fn parse_cq_and_qso_messages() {
        let p = parse_message("CQ AB1CD FN42", MsgKind::Standard);
        assert_eq!(p.to, None);
        assert_eq!(p.from.as_deref(), Some("AB1CD"));
        assert_eq!(p.grid.as_deref(), Some("FN42"));
        assert!(p.is_cq && p.cq_to.is_none(), "a plain CQ is aimed at nobody in particular");

        let p = parse_message("W9XYZ AB1CD -13", MsgKind::Standard);
        assert_eq!(p.to.as_deref(), Some("W9XYZ"));
        assert_eq!(p.from.as_deref(), Some("AB1CD"));
        assert_eq!(p.grid, None);
        assert!(!p.is_cq);

        let p = parse_message("AB1CD W9XYZ EM48", MsgKind::Standard);
        assert_eq!(p.to.as_deref(), Some("AB1CD"));
        assert_eq!(p.from.as_deref(), Some("W9XYZ"));
        assert_eq!(p.grid.as_deref(), Some("EM48"));

        // A rogered grid ("R FN42") still places the station.
        let p = parse_message("AB1CD W9XYZ R EM48", MsgKind::Standard);
        assert_eq!(p.grid.as_deref(), Some("EM48"));

        // "CQ DX": the caller only wants stations outside their own entity.
        let p = parse_message("CQ DX AB1CD FN42", MsgKind::Standard);
        assert_eq!(p.to, None);
        assert_eq!(p.from.as_deref(), Some("AB1CD"), "the DX modifier is not the sender");
        assert_eq!(p.grid.as_deref(), Some("FN42"));
        assert!(p.is_cq);
        assert_eq!(p.cq_to.as_deref(), Some("DX"));

        // "RR73" is a sign-off, not a locator — it must not be read as a grid.
        let p = parse_message("AB1CD W9XYZ RR73", MsgKind::Standard);
        assert_eq!(p.from.as_deref(), Some("W9XYZ"));
        assert_eq!(p.grid, None, "RR73 must not parse as a grid position");
        // Invalid Maidenhead fields (S..Z) aren't grids either.
        assert!(!is_grid("ZZ99"));
        assert!(is_grid("FN42"));
    }

    #[test]
    fn every_directed_cq_names_its_target() {
        // Continent, country prefix and activity all read the same way: the
        // token between "CQ" and the callsign.
        for (msg, want) in [
            ("CQ EU AB1CD FN42", "EU"),
            ("CQ JA AB1CD FN42", "JA"),
            ("CQ POTA AB1CD FN42", "POTA"),
            ("CQ TEST AB1CD FN42", "TEST"),
            ("CQ 001 AB1CD FN42", "001"),
        ] {
            let p = parse_message(msg, MsgKind::Standard);
            assert_eq!(p.cq_to.as_deref(), Some(want), "{msg}");
            assert_eq!(p.from.as_deref(), Some("AB1CD"), "{msg}: the modifier is not the sender");
            assert_eq!(p.grid.as_deref(), Some("FN42"), "{msg}");
        }
    }

    #[test]
    fn a_directed_cq_goes_out_as_one() {
        // "CQ EU" is a single packed token, not a fourth field, so the message
        // still fits the everyday layout.
        let (sent, decodes) = round_trip(Mode::Ft8, "CQ EU AB1CD FN42");
        assert_eq!(sent, "CQ EU AB1CD FN42");
        let d = decodes.iter().find(|d| d.is_cq).expect("decoded");
        assert_eq!(d.message, "CQ EU AB1CD FN42");
        assert_eq!(d.cq_to.as_deref(), Some("EU"));
        assert_eq!(d.from.as_deref(), Some("AB1CD"));
    }

    #[test]
    fn free_text_carries_no_addressing() {
        // Free text may look exactly like an exchange — only the type bits know
        // the difference, and reading a sender out of it would aim the QSO
        // machine at a station that never transmitted.
        let p = parse_message("W9XYZ RR73", MsgKind::FreeText);
        assert!(p.free_text);
        assert_eq!((p.to, p.from, p.grid), (None, None, None));
        assert!(!p.is_cq);
        // The same text as a real message does address someone.
        let p = parse_message("W9XYZ RR73", MsgKind::Standard);
        assert_eq!(p.to.as_deref(), Some("W9XYZ"));
    }

    #[test]
    fn hashed_and_non_standard_callsigns_parse() {
        // An unresolved hash is nobody: better no callsign than "<...>".
        let p = parse_message("<...> DL/W1AW RR73", MsgKind::NonStandard);
        assert_eq!(p.to, None);
        assert_eq!(p.from.as_deref(), Some("DL/W1AW"));

        // A resolved one names the station it stands for.
        let p = parse_message("DL/W1AW <AB1CD> 73", MsgKind::NonStandard);
        assert_eq!(p.to.as_deref(), Some("DL/W1AW"));
        assert_eq!(p.from.as_deref(), Some("AB1CD"));

        // A non-standard call can call CQ too.
        let p = parse_message("CQ DL/W1AW", MsgKind::NonStandard);
        assert!(p.is_cq);
        assert_eq!(p.from.as_deref(), Some("DL/W1AW"));
    }

    #[test]
    fn contest_and_dxpedition_layouts_parse() {
        // ARRL RTTY Roundup: the "TU;" opener is not the addressee.
        let p = parse_message("TU; K1ABC W9XYZ R 589 MA", MsgKind::RttyRu);
        assert_eq!(p.to.as_deref(), Some("K1ABC"));
        assert_eq!(p.from.as_deref(), Some("W9XYZ"));
        assert_eq!(p.grid, None);

        // Field Day.
        let p = parse_message("K1ABC W9XYZ [FD]", MsgKind::FieldDay);
        assert_eq!(p.to.as_deref(), Some("K1ABC"));
        assert_eq!(p.from.as_deref(), Some("W9XYZ"));

        // DXpedition (Fox): the fox is working W9XYZ; K1ABC's contact is done.
        let p = parse_message("K1ABC RR73; W9XYZ <DX1FOX> +03", MsgKind::Fox);
        assert_eq!(p.to.as_deref(), Some("W9XYZ"));
        assert_eq!(p.from.as_deref(), Some("DX1FOX"), "the fox sent it");
        assert_eq!(p.rr73_to.as_deref(), Some("K1ABC"), "K1ABC is the one being signed off");
    }

    #[test]
    fn a_fox_message_round_trips() {
        // One transmission closing K1ABC's contact and reporting to W9XYZ. The
        // fox's own call travels as a 10-bit hash, so a receiver that has heard
        // it resolves the name and one that hasn't sees `<...>`.
        let mut modem = Ft8Modem::new(Mode::Ft8);
        modem.seed_hashes(&["DX1FOX".to_string()]);
        let (burst, sent) =
            modem.encode_burst_12k("K1ABC RR73; W9XYZ DX1FOX +03", 1500.0, 0.5).expect("encode");
        assert_eq!(sent, "K1ABC RR73; W9XYZ <DX1FOX> +02", "the layout carries 2 dB steps");

        let mut slot = vec![0.0f32; 6_000];
        slot.extend_from_slice(&burst);
        slot.resize(15 * 12_000, 0.0);
        let buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        let d = modem
            .decode_slot(&buf, 0, &ApHints::default(), 1500.0)
            .into_iter()
            .find(|d| d.message.contains("RR73;"))
            .expect("decoded");
        assert_eq!(d.message, "K1ABC RR73; W9XYZ <DX1FOX> +02");
        assert_eq!(d.rr73_to.as_deref(), Some("K1ABC"));
        assert_eq!(d.to.as_deref(), Some("W9XYZ"));
        assert_eq!(d.from.as_deref(), Some("DX1FOX"));
        assert!(!d.free_text);
    }

    #[test]
    fn ft8_encode_decode_round_trip() {
        let (sent, decodes) = round_trip(Mode::Ft8, "CQ AB1CD FN42");
        assert_eq!(sent, "CQ AB1CD FN42");
        assert!(
            decodes.iter().any(|d| d.message == "CQ AB1CD FN42" && !d.free_text),
            "got {:?}",
            decodes.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ft4_encode_decode_round_trip() {
        let (_, decodes) = round_trip(Mode::Ft4, "CQ AB1CD FN42");
        assert!(
            decodes.iter().any(|d| d.message == "CQ AB1CD FN42"),
            "got {:?}",
            decodes.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_priori_hints_name_the_message_we_are_waiting_for() {
        // Our call is the addressee, the DX's the sender: "AB1CD W9XYZ R-13".
        let ap = ApHints {
            my_call: "AB1CD".into(),
            dx_call: Some("W9XYZ".into()),
            ..Default::default()
        };
        let h = ap.ft8().expect("a hint");
        assert_eq!(h.call1.as_deref(), Some("AB1CD"));
        assert_eq!(h.call2.as_deref(), Some("W9XYZ"));
        assert_eq!(ap.calls(), ["AB1CD", "W9XYZ"]);

        // Calling CQ we still know the addressee — anyone answering names us.
        let ap = ApHints { my_call: "AB1CD".into(), dx_call: None, ..Default::default() };
        let h = ap.ft8().expect("a hint");
        assert_eq!(h.call1.as_deref(), Some("AB1CD"));
        assert_eq!(h.call2, None);

        // An unconfigured station knows nothing to hint with.
        assert!(ApHints::default().ft8().is_none());
        assert!(ApHints::default().ft4().is_none());
        assert!(
            ApHints { my_call: "  ".into(), dx_call: Some("W9XYZ".into()), ..Default::default() }
                .ft8()
                .is_none()
        );
    }

    #[test]
    fn an_a_priori_hint_only_ever_adds_decodes() {
        // The hint is a fallback: mfsk-core attempts it only where an ordinary
        // decode has already failed, and the result still has to pass CRC-14.
        // So at any noise level the hinted pass finds everything the plain one
        // does — which is what makes it safe to leave on all the time.
        let ap = ApHints {
            my_call: "AB1CD".into(),
            dx_call: Some("W9XYZ".into()),
            ..Default::default()
        };
        let mut rng: u32 = 0x1234_5678;
        let mut noise = || {
            // xorshift32, so the comparison runs on identical audio each time.
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng as i32 as f32) / (i32::MAX as f32)
        };
        for &level in &[0.05f32, 0.2, 0.5] {
            let modem = Ft8Modem::new(Mode::Ft8);
            let (burst, _) = modem.encode_burst_12k("AB1CD W9XYZ -13", 1500.0, 0.5).unwrap();
            let mut slot = vec![0.0f32; 6_000];
            slot.extend_from_slice(&burst);
            slot.resize(15 * 12_000, 0.0);
            let buf: Vec<i16> =
                slot.iter().map(|&s| ((s + level * noise()) * 12_000.0) as i16).collect();

            let plain = Ft8Modem::new(Mode::Ft8).decode_slot(&buf, 0, &ApHints::default(), 1500.0);
            let hinted = Ft8Modem::new(Mode::Ft8).decode_slot(&buf, 0, &ap, 1500.0);
            for d in &plain {
                assert!(
                    hinted.iter().any(|h| h.message == d.message),
                    "noise {level}: the hinted pass lost {:?}",
                    d.message
                );
            }
        }
    }

    /// The exchange has to survive the air in FT8, not merely a round trip
    /// through the packer (issue #223).
    ///
    /// This is the regression the reporter found on room audio: mfsk-core's FT8
    /// decoder finishes every candidate with an `unpack77` that has no `i3 = 5`
    /// arm, so a contest exchange that decoded perfectly well was thrown away
    /// *inside* the decoder and no exchange ever reached the QSO machine. FT4
    /// and FT2 go through a pipeline that never unpacks, which is why they
    /// worked. So the assertion is specifically about FT8, and specifically
    /// about a modulated slot rather than a bitstream.
    #[test]
    fn an_ft8_contest_exchange_survives_being_transmitted() {
        let modem = Ft8Modem::new(Mode::Ft8);
        let sent = "<PA9XYZ> <G4ABC/P> R 590003 IO91NP";
        let (burst, as_sent) = modem.encode_burst_12k(sent, 1500.0, 0.5).expect("packs");
        assert_eq!(as_sent, sent, "the exchange goes out as written");
        let mut slot = vec![0.0f32; 6_000];
        slot.extend_from_slice(&burst);
        slot.resize(15 * 12_000, 0.0);
        let buf: Vec<i16> = slot.iter().map(|&s| (s * 12_000.0) as i16).collect();

        // Both callsigns spelled out already, as the two ordinary messages
        // ahead of a real exchange would have done: the layout carries hashes,
        // so without them the decode reads `<...>` however well it worked.
        let mut rx = Ft8Modem::new(Mode::Ft8);
        rx.seed_hashes(&["PA9XYZ".into(), "G4ABC/P".into()]);

        // Without the contest selected, this is what the report describes:
        // nothing at all comes back, because the decoder discards the layout.
        let off = rx.decode_slot(&buf, 0, &ApHints::default(), 1500.0);
        assert!(
            !off.iter().any(|d| d.message.contains("590003")),
            "mfsk-core's FT8 decoder is expected to drop i3 = 5 — if this ever \
             stops being true the rescue pass can go",
        );

        // With it, the exchange arrives, whole.
        let ap = ApHints { eu_vhf: true, ..Default::default() };
        let got = rx.decode_slot(&buf, 0, &ap, 1500.0);
        let d = got
            .iter()
            .find(|d| d.message.contains("590003"))
            .unwrap_or_else(|| panic!("the exchange did not decode; got {got:?}"));
        assert_eq!(d.message, "<PA9XYZ> <G4ABC/P> R 590003 IO91NP");
        assert_eq!(d.to.as_deref(), Some("PA9XYZ"), "and it is addressed to the right station");
        assert_eq!(d.from.as_deref(), Some("G4ABC/P"));
        assert!((d.audio_hz - 1500.0).abs() < 5.0, "at the offset it was sent on");
    }

    /// The rescue pass may only ever *add* the one layout the primary decode
    /// cannot return. An ordinary message must come back exactly once, and
    /// come back whether or not the contest is selected.
    #[test]
    fn the_contest_pass_does_not_disturb_ordinary_decodes() {
        let modem = Ft8Modem::new(Mode::Ft8);
        let (burst, _) = modem.encode_burst_12k("AB1CD W9XYZ -13", 1500.0, 0.5).unwrap();
        let mut slot = vec![0.0f32; 6_000];
        slot.extend_from_slice(&burst);
        slot.resize(15 * 12_000, 0.0);
        let buf: Vec<i16> = slot.iter().map(|&s| (s * 12_000.0) as i16).collect();

        let plain = Ft8Modem::new(Mode::Ft8).decode_slot(&buf, 0, &ApHints::default(), 1500.0);
        let ap = ApHints { eu_vhf: true, ..Default::default() };
        let contest = Ft8Modem::new(Mode::Ft8).decode_slot(&buf, 0, &ap, 1500.0);
        assert_eq!(
            plain.iter().filter(|d| d.message == "AB1CD W9XYZ -13").count(),
            1,
            "got {plain:?}",
        );
        assert_eq!(
            contest.iter().filter(|d| d.message == "AB1CD W9XYZ -13").count(),
            1,
            "the second pass must not report an ordinary message a second time",
        );
    }

    /// FT4's hint takes the other path — a targeted sniper decode aimed where
    /// we are listening, run *after* the wide pass and merged into it. Same
    /// guarantee as FT8's, and the one that would break silently if the sniper
    /// were ever aimed, thresholded or de-duplicated wrongly: the wide pass's
    /// decodes must all survive.
    #[test]
    fn an_ft4_a_priori_hint_only_ever_adds_decodes() {
        let ap = ApHints {
            my_call: "AB1CD".into(),
            dx_call: Some("W9XYZ".into()),
            ..Default::default()
        };
        let mut rng: u32 = 0x1234_5678;
        let mut noise = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng as i32 as f32) / (i32::MAX as f32)
        };
        for &level in &[0.05f32, 0.2, 0.5] {
            let modem = Ft8Modem::new(Mode::Ft4);
            let (burst, _) = modem.encode_burst_12k("AB1CD W9XYZ -13", 1500.0, 0.5).unwrap();
            let mut slot = vec![0.0f32; 6_000];
            slot.extend_from_slice(&burst);
            slot.resize((7.5 * 12_000.0) as usize, 0.0);
            let buf: Vec<i16> =
                slot.iter().map(|&s| ((s + level * noise()) * 12_000.0) as i16).collect();

            let plain = Ft8Modem::new(Mode::Ft4).decode_slot(&buf, 0, &ApHints::default(), 1500.0);
            let hinted = Ft8Modem::new(Mode::Ft4).decode_slot(&buf, 0, &ap, 1500.0);
            for d in &plain {
                assert!(
                    hinted.iter().any(|h| h.message == d.message),
                    "noise {level}: the hinted pass lost {:?}",
                    d.message
                );
            }
        }
    }

    #[test]
    fn free_text_goes_out_as_free_text() {
        // Not an exchange, so it can only travel as 13 characters of text.
        // Checked at the packer rather than through a decode: mfsk-core's
        // decoder drops any message whose words aren't callsign-shaped (its
        // guard against CRC-14 false decodes), which takes real free text with
        // it — so what we can verify here is that we transmit the right thing.
        let (bits, sent) = pack_message("TNX QSO 73 GL").expect("packs");
        assert_eq!(sent, "TNX QSO 73 GL");
        assert_eq!(msg_kind(&bits), MsgKind::FreeText);
        assert_eq!(wsjt77::unpack77(&bits).as_deref(), Some("TNX QSO 73 GL"));

        // Over-long text is cut to the 13 characters FT8 carries, and the
        // caller is told what actually went out.
        let (_, sent) = pack_message("THANKS FOR THE CONTACT").expect("packs");
        assert_eq!(sent, "THANKS FOR TH");
    }

    #[test]
    fn a_compound_call_is_addressable() {
        // The everyday layout can't hold "DL/W1AW", so the message degrades to
        // the non-standard form: our call travels as a hash, and the report is
        // dropped — the returned text says exactly what went out.
        let (sent, decodes) = round_trip(Mode::Ft8, "DL/W1AW AB1CD RR73");
        assert_eq!(sent, "DL/W1AW <AB1CD> RR73");
        let d = decodes.iter().find(|d| d.message.contains("DL/W1AW")).expect("decoded");
        assert_eq!(d.to.as_deref(), Some("DL/W1AW"), "the compound call is the addressee");
        assert_eq!(d.from, None, "our call is hashed, and this receiver has never heard it");

        // A receiver that has heard us resolves the hash to our callsign.
        let mut modem = Ft8Modem::new(Mode::Ft8);
        modem.seed_hashes(&["AB1CD".to_string()]);
        let (burst, _) = modem.encode_burst_12k("DL/W1AW AB1CD RR73", 1500.0, 0.5).unwrap();
        let mut slot = vec![0.0f32; 6_000];
        slot.extend_from_slice(&burst);
        slot.resize(15 * 12_000, 0.0);
        let buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        let d = modem
            .decode_slot(&buf, 0, &ApHints::default(), 1500.0)
            .into_iter()
            .find(|d| d.message.contains("DL/W1AW"))
            .expect("decoded");
        assert_eq!(d.from.as_deref(), Some("AB1CD"));
        assert_eq!(d.message, "DL/W1AW <AB1CD> RR73");
    }

    #[test]
    fn a_compound_call_can_call_cq() {
        let (sent, decodes) = round_trip(Mode::Ft8, "CQ DL/W1AW");
        assert_eq!(sent, "CQ DL/W1AW");
        let d = decodes.iter().find(|d| d.message.contains("DL/W1AW")).expect("decoded");
        assert!(d.is_cq);
        assert_eq!(d.from.as_deref(), Some("DL/W1AW"));
    }
}
