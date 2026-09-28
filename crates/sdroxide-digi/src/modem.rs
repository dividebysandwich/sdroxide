//! The mfsk-core adapter: FT8/FT4 decode and encode wrapped behind stable
//! sdroxide types. **This is the only file that touches raw mfsk-core
//! decode-result fields** — if the crate's field names change, only
//! `decode_slot` needs updating.

use mfsk_core::msg::decode_request::DecodeRequest;
use mfsk_core::msg::hash_table::CallsignHashTable;
use mfsk_core::msg::wsjt77;
use sdroxide_types::{Decode, Mode, UvPacketFrame, UvPacketMode};

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
            // Subtraction, so a weak signal inside a stronger neighbour's
            // occupied bandwidth is still decoded — FT4's whole reason for
            // multi-pass SIC, and what WSJT-X/WSJT-CB run by default. The
            // default strategy is single-pass: without this a busy slot
            // reports only the strongest signal at each audio offset, which
            // is the "they don't stack" report against WSJT-X. Two rounds are
            // the measured choice (mfsk-core's FT4 WSJT-X sample: 11/14
            // without, 14/14 with `.sic_rounds(2)`, nothing more from a
            // third; 5 ms → 71 ms against a 7.5 s slot).
            .sic_rounds(2)
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

/// Decode one full 60-second JT65 or JT9 slot of 12 kHz mono i16 audio.
///
/// The two JT modes share the 72-bit JT message and the 60-second slot, so
/// they share this mapping; only the call into mfsk-core differs. It is a free
/// function rather than a [`Ft8Modem`] method because JT keeps no hash table —
/// its 72-bit message has no hashed-callsign layout — and the JT sound card is
/// the i16 the slotted engine already carries, which mfsk-core wants as f32.
///
/// A JT decode carries **no CRC** (72 bits, no checksum), so what stands
/// between noise and an invented message is the decoder itself: JT65's
/// Reed–Solomon decode is hard-decision with no erasures, which almost never
/// converges on noise, and JT9 gates each candidate on its sync and its soft
/// symbol quality and accepts only the standard `<to> <from> <grid|report>`
/// layout. Both scans collapse duplicates. Every row they return is shown.
///
/// The scan is centred on the one-second transmit offset both modes use, and
/// `dt` is reported from there, as WSJT-X's DT column is: an on-time station
/// reads about 0.
pub fn decode_jt_slot(audio_12k: &[i16], mode: Mode, slot_utc: i64) -> Vec<Decode> {
    let audio: Vec<f32> = audio_12k.iter().map(|&s| f32::from(s) / 28_000.0).collect();
    let nominal_s = JT_TX_OFFSET_SAMPLES as f32 / DECODE_RATE_U32 as f32;
    match mode {
        Mode::Jt65 => {
            let params = mfsk_core::jt65::search::SearchParams {
                // mfsk-core's default window is 1000–2000 Hz; a JT65 signal
                // can sit anywhere a receiver passes, and its 65 tones run
                // ~180 Hz up from the sync tone the search places.
                freq_min_hz: JT_FREQ_MIN_HZ,
                freq_max_hz: AUDIO_MAX_HZ - JT65_WIDTH_HZ,
                // Every candidate costs a full decode attempt; over the whole
                // passband the default eight would cap a busy slot at eight
                // stations.
                max_candidates: 20,
                ..Default::default()
            };
            mfsk_core::jt65::decode_scan(&audio, DECODE_RATE_U32, JT_TX_OFFSET_SAMPLES, &params)
                .into_iter()
                .filter_map(|r| {
                    jt_decode(r.message, r.snr_db, r.dt_sec - nominal_s, r.freq_hz, slot_utc)
                })
                .collect()
        }
        Mode::Jt9 => mfsk_core::jt9::decode_scan(
            &audio,
            DECODE_RATE_U32,
            JT_TX_OFFSET_SAMPLES,
            // JT9's default window is already jt9's own 200–4000 Hz.
            &mfsk_core::jt9::search::SearchParams::default(),
        )
        .into_iter()
        .filter_map(|r| {
            // JT9 exposes only the start index, from the start of the buffer.
            let dt = r.start_sample as f32 / DECODE_RATE_U32 as f32 - nominal_s;
            jt_decode(r.message, r.snr_db, dt, r.freq_hz, slot_utc)
        })
        .collect(),
        _ => Vec::new(),
    }
}

/// JT65 and JT9 key one second into their 60-second slot, as WSJT-X does.
const JT_TX_OFFSET_SAMPLES: usize = 12_000;
/// The lowest sync tone the JT65 scan looks for, Hz.
const JT_FREQ_MIN_HZ: f32 = 200.0;
/// A JT65A signal's width above its sync tone: 65 tones at 2.69 Hz.
const JT65_WIDTH_HZ: f32 = 180.0;

/// mfsk-core's JT and Q65 entry points take a `u32` rate; the workspace's is
/// `f64`.
const DECODE_RATE_U32: u32 = 12_000;

/// Map one mfsk-core JT message onto the stable [`Decode`].
///
/// The 72-bit message has two shapes: the everyday `<to> <from> <grid|report>`
/// and an `Unsupported` catch-all for compound callsigns and free text, which
/// this build does not unpack — it is skipped rather than shown as a row of
/// hex, because a decode list a listener cannot read is worse than a shorter
/// one.
fn jt_decode(
    message: mfsk_core::msg::Jt72Message,
    snr_db: f32,
    dt_sec: f32,
    freq_hz: f32,
    slot_utc: i64,
) -> Option<Decode> {
    let mfsk_core::msg::Jt72Message::Standard { call1, call2, grid_or_report } = message else {
        return None;
    };
    let text = format!("{call1} {call2} {grid_or_report}");
    let p = parse_message(&text, MsgKind::Standard);
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
        free_text: false,
        rr73_to: None,
    })
}

/// Pack `text` into a 77-bit message, degrading to the nearest layout FT8 can
/// carry, and report the message as the far end will read it.
///
/// The ladder is: the DXpedition (Fox) layout when the text is written as one,
/// then the EU VHF contest exchange, then a directed CQ, then the standard
/// exchange, then that same exchange with a non-standard callsign carried as
/// its hash ([`pack77_hashed`]), then the non-standard-callsign layout (which
/// spells the callsign out but can only carry `RRR` / `RR73` / `73` beside
/// it), then 13 characters of free text.
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

    // 3b. One compound / non-standard callsign addressed with a grid or a
    //     signal report — neither of which the layout below has anywhere to
    //     put. The standard layout does, so long as that callsign travels as
    //     its hash instead of spelled out (issue #348).
    if let Some((m, sent)) = pack77_hashed(c1, c2, payload) {
        return Some((m, sent));
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

/// Where `unpack28` stops reading a 28-bit callsign field as one of the fixed
/// tokens (`DE`, `QRZ`, `CQ`, `CQ NNN`, `CQ XXXX`) and starts reading it as a
/// callsign. The 4194304 values above it are 22-bit hashes; spelled-out
/// callsigns begin above those. WSJT-X's `packjt77.f90` calls it `NTOKENS`;
/// mfsk-core knows the same number but does not export it, and only ever
/// *reads* the hash range — `pack28` has no arm that writes one.
const NTOKENS: u32 = 2_063_592;

/// The everyday layout with one callsign carried as its 22-bit hash rather
/// than spelled out, which is the only way FT8 can send a report or a grid to
/// a station whose callsign the 28-bit field cannot hold (issue #348).
///
/// The non-standard layout (`i3 = 4`, [`wsjt77::pack77_type4`]) spells such a
/// callsign out in full, but it spends 58 of its 77 bits doing so and has room
/// left for nothing but a bare `RRR` / `RR73` / `73`. So a reply to
/// `R7KJG/QRP` that should have carried `R-12` went out as an acknowledgement
/// with no report in it at all, and the contact stalled there. The way through
/// is the one WSJT-X takes, and the reason the hash range exists: send
/// `<R7KJG/QRP> F4CYH R-12` as an ordinary `i3 = 1` message whose first
/// callsign field holds `NTOKENS + hash22`. Both ends resolve it — the far end
/// because the hash is of its own callsign, and this one because the contact
/// opened with a message that spelled the callsign out.
///
/// `None` for anything that layout should not carry, leaving the ladder in
/// [`pack_message`] to go on to the next rung:
///
/// * A bare `RRR` / `RR73` / `73`, and an empty payload. Those fit the
///   non-standard layout whole, and spelling the callsign out is worth more
///   than the hash saves — it is what lets a third station resolve the hashes
///   in everything around it.
/// * Both callsigns non-standard, or neither. Hashing both would leave a
///   message that only a station which had already heard *both* spell
///   themselves out could read, and the everyday packer already handles
///   neither.
fn pack77_hashed(c1: &str, c2: &str, payload: &str) -> Option<([u8; 77], String)> {
    if payload.is_empty() || matches!(payload, "RRR" | "RR73" | "73") {
        return None;
    }
    // A callsign the operator wrote in brackets is one they are asking to have
    // hashed; one that will not fit the 28-bit field has to be, brackets or no.
    let (b1, b2) = (eu_vhf::bare(c1), eu_vhf::bare(c2));
    let wants_hash = |tok: &str, bare: &str| {
        !bare.is_empty()
            && bare != "CQ"
            && (tok.starts_with('<') || !wsjt77::is_standard_callsign(bare))
    };
    let (h1, h2) = (wants_hash(c1, b1), wants_hash(c2, b2));
    if h1 == h2 {
        return None;
    }
    let (hashed, spelled) = if h1 { (b1, b2) } else { (b2, b1) };
    if !wsjt77::is_valid_callsign(hashed) || !wsjt77::is_standard_callsign(spelled) {
        return None;
    }
    // Packed with a stand-in where the hash goes and then overwritten, rather
    // than assembled here: the report field alone has five shapes, and the
    // difference between a grid, a report and an R-report is mfsk-core's to
    // know. Only the one field it cannot write is written by hand.
    const STAND_IN: &str = "K1ABC";
    let mut msg =
        wsjt77::pack77(if h1 { STAND_IN } else { b1 }, if h2 { STAND_IN } else { b2 }, payload)?;
    // WSJT-X hashes the callsign whole, `/QRP` and all (`save_hash_call`), so
    // the hash is taken here rather than through mfsk-core's table, which
    // strips a `/P` or `/R` suffix first — the same divergence [`eu_vhf`]
    // keeps its own table for.
    let n28 = NTOKENS + mfsk_core::msg::hash_table::ihashcall(hashed, 22);
    let start = if h1 { 0 } else { 29 };
    for i in 0..28 {
        msg[start + i] = ((n28 >> (27 - i)) & 1) as u8;
    }
    let bracket = |call: &str, hash: bool| {
        if hash { format!("<{call}>") } else { call.to_string() }
    };
    Some((msg, join3(&bracket(b1, h1), &bracket(b2, h2), payload)))
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

/// How far either side of the audio cursor the MSK144 decoder searches, in Hz.
///
/// MSK144's tones sit 500 Hz either side of its centre, and the short-ping
/// search looks `2 × tolerance` either side of each tone line in the squared
/// signal's spectrum — so a tolerance of 500 Hz or more makes the high- and
/// low-tone windows overlap, and the frequency-error estimate the short-ping
/// path depends on can lock to the wrong line. The long-ping search's cost also
/// grows with it. 200 Hz covers a station a little off the cursor.
pub const MSK144_TOLERANCE_HZ: f32 = 200.0;

/// Decode one 15-second MSK144 slot of 12 kHz mono i16 audio, searching
/// [`MSK144_TOLERANCE_HZ`] either side of `audio_hz`.
///
/// MSK144 is not a frame at a fixed offset: the operator transmits through the
/// whole period and the decoder slides a window across the slot looking for
/// meteor-trail bursts, so it is handed the centre and half-width to search
/// rather than a start time — and each result carries the time *into* the
/// slot (`.tsec`) the burst was found at, which becomes the [`Decode`]'s `dt`.
/// The centre is the operator's audio cursor, as WSJT-X's is: 1500 Hz is the
/// convention everyone transmits on.
///
/// Unlike every other mode here, mfsk-core resolves the message text inside
/// the decode call, so there are no raw 77 bits for us to unpack: the text is
/// already a `String`, and [`parse_text_decode`] reads it.
pub fn decode_msk144_slot(audio_12k: &[i16], audio_hz: f32, slot_utc: i64) -> Vec<Decode> {
    use mfsk_core::msk144::decode::{Depth, decode_slot};
    decode_slot(audio_12k, audio_hz, MSK144_TOLERANCE_HZ, Depth::Deep)
        .into_iter()
        .map(|r| {
            let p = parse_text_decode(&r.message);
            Decode {
                slot_utc,
                snr_db: r.snr_db as i16,
                dt: r.tsec,
                audio_hz: r.freq_hz,
                message: r.message,
                to: p.to,
                from: p.from,
                grid: p.grid,
                is_cq: p.is_cq,
                cq_to: p.cq_to,
                free_text: p.free_text,
                rr73_to: None,
            }
        })
        .collect()
}

/// Decode one full FST4 slot of 12 kHz mono i16 audio at the chosen period.
///
/// FST4 goes through the same generic [`DecodeRequest`] as FT4 and shares its
/// 77-bit message, so the mapping to [`Decode`] is FT4's; only the protocol
/// type parameter differs, and the five periods are five types rather than a
/// runtime width. The slot length is the period the operator chose — the
/// caller must hand in that many seconds of audio, which is what
/// [`sdroxide_types::Fst4Period`]'s timing gives the controller.
pub fn decode_fst4_slot(
    audio_12k: &[i16],
    period: sdroxide_types::Fst4Period,
    slot_utc: i64,
) -> Vec<Decode> {
    use mfsk_core::msg::decode_request::DecodeRequest;
    // FST4's own example uses a sync floor of 0.8 and a small candidate cap:
    // it is a deep, sparse mode, so there are few signals and a lower bar is
    // safe. The FT8 family's 1.5 would hide the weak ones this mode exists
    // for.
    const FST4_SYNC_MIN: f32 = 0.8;
    const FST4_MAX_CAND: usize = 30;
    // The five periods are five protocol types rather than a runtime width, so
    // the request cannot be built once behind a generic closure; the macro
    // writes the arms out instead, exactly as the crate's own example does.
    macro_rules! run {
        ($p:ty) => {
            DecodeRequest::<$p>::new(
                audio_12k,
                AUDIO_MIN_HZ,
                AUDIO_MAX_HZ,
                FST4_SYNC_MIN,
                FST4_MAX_CAND,
            )
            .decode()
            .results
            .into_iter()
            .filter_map(|r| {
                let bits: [u8; 77] = r.message77().try_into().ok()?;
                // FST4 carries the same 77-bit message as FT8, hashed-callsign
                // layouts included. No session hash table is kept for it, so a
                // hashed callsign reads as `<...>` rather than being resolved.
                build_decode(
                    &bits,
                    r.snr_db,
                    r.dt_sec,
                    r.freq_hz,
                    slot_utc,
                    &CallsignHashTable::new(),
                    &eu_vhf::Hashes::default(),
                )
            })
            .collect()
        };
    }
    match period {
        sdroxide_types::Fst4Period::P15 => run!(mfsk_core::fst4::Fst4s15),
        sdroxide_types::Fst4Period::P30 => run!(mfsk_core::fst4::Fst4s30),
        sdroxide_types::Fst4Period::P60 => run!(mfsk_core::fst4::Fst4s60),
        sdroxide_types::Fst4Period::P120 => run!(mfsk_core::fst4::Fst4s120),
        sdroxide_types::Fst4Period::P300 => run!(mfsk_core::fst4::Fst4s300),
    }
}

/// Decode one full Q65 slot of 12 kHz mono i16 audio at the chosen sub-mode.
///
/// Q65 goes through mfsk-core's `q65::DecodeRequest`, which takes f32 audio
/// and a `SearchParams`; the ten sub-modes are ten protocol types, so the
/// request is built per sub-mode. The result carries the message text already
/// unpacked, so it maps through the same parser every other mode uses.
pub fn decode_q65_slot(
    audio_12k: &[i16],
    mode: sdroxide_types::Q65Mode,
    slot_utc: i64,
) -> Vec<Decode> {
    use mfsk_core::q65::DecodeRequest;
    use mfsk_core::q65::search::SearchParams;

    let audio: Vec<f32> = audio_12k.iter().map(|&s| f32::from(s) / 28_000.0).collect();
    // The scan searches around where this sub-mode keys — half a second in for
    // 15A and 30A, one second for the rest — and its own asymmetric
    // `SearchParams` window carries the tolerance. mfsk-core reports each
    // frame's start from the start of the buffer, so the nominal start comes
    // off it to give WSJT-X's DT.
    let nominal_start = (mode.start_delay_s() * f64::from(DECODE_RATE_U32)).round() as usize;
    let nominal_s = mode.start_delay_s() as f32;
    let params = SearchParams::default();

    macro_rules! run {
        ($p:ty) => {
            DecodeRequest::<$p>::new(&audio, DECODE_RATE_U32, nominal_start, params)
                .decode()
                .into_iter()
                .map(|r| {
                    let p = parse_text_decode(&r.message);
                    Decode {
                        slot_utc,
                        snr_db: r.snr_db.round() as i16,
                        dt: r.dt_sec - nominal_s,
                        audio_hz: r.freq_hz,
                        message: r.message,
                        to: p.to,
                        from: p.from,
                        grid: p.grid,
                        is_cq: p.is_cq,
                        cq_to: p.cq_to,
                        free_text: p.free_text,
                        rr73_to: None,
                    }
                })
                .collect()
        };
    }
    match mode {
        sdroxide_types::Q65Mode::A15 => run!(mfsk_core::q65::Q65a15),
        sdroxide_types::Q65Mode::A30 => run!(mfsk_core::q65::Q65a30),
        sdroxide_types::Q65Mode::A60 => run!(mfsk_core::q65::Q65a60),
        sdroxide_types::Q65Mode::B60 => run!(mfsk_core::q65::Q65b60),
        sdroxide_types::Q65Mode::C60 => run!(mfsk_core::q65::Q65c60),
        sdroxide_types::Q65Mode::D60 => run!(mfsk_core::q65::Q65d60),
        sdroxide_types::Q65Mode::E60 => run!(mfsk_core::q65::Q65e60),
        sdroxide_types::Q65Mode::D120 => run!(mfsk_core::q65::Q65d120),
        sdroxide_types::Q65Mode::E120 => run!(mfsk_core::q65::Q65e120),
        sdroxide_types::Q65Mode::A300 => run!(mfsk_core::q65::Q65a300),
    }
}

/// Decode one FSK441 slot of mono f32 audio at
/// [`sdroxide_dsp::FSK441_RATE`](sdroxide_dsp::FSK441_RATE).
///
/// FSK441 is a meteor-scatter mode: the operator transmits the message over and
/// over through the whole period and the receiver hears only the short
/// reflections off ionised trails, so the decoder hunts the slot for pings.
/// Each [`sdroxide_dsp::Fsk441Ping`] carries the time into the slot it was
/// found at, which becomes the [`Decode`]'s `dt`.
///
/// The audio is at the decoder's own rate — 441 baud × 25 samples — and not the
/// 12 kHz the other modes use: the controller resamples to it before this sees
/// a sample, because those constants are the mode and do not re-derive at
/// another rate.
pub fn decode_fsk441_slot(audio: &[f32], slot_utc: i64) -> Vec<Decode> {
    sdroxide_dsp::fsk441_find_pings(audio)
        .into_iter()
        .map(|p| {
            let parsed = parse_text_decode(&p.text);
            Decode {
                slot_utc,
                snr_db: p.snr_db.round() as i16,
                dt: p.start_s,
                audio_hz: p.audio_hz,
                message: p.text,
                to: parsed.to,
                from: parsed.from,
                grid: parsed.grid,
                is_cq: parsed.is_cq,
                cq_to: parsed.cq_to,
                free_text: parsed.free_text,
                rr73_to: parsed.rr73_to,
            }
        })
        .collect()
}

/// Decode every UVPacket frame in a window of 12 kHz mono f32 audio.
///
/// UVPacket is not a slotted WSJT mode: a frame can begin anywhere, so this
/// scans the whole window for preambles and returns whatever it finds. The
/// sub-mode comes from the winning preamble, so nothing is selected by the
/// caller; each frame's message fields are already unpacked by the decoder,
/// which is why this maps straight onto [`UvPacketFrame`] rather than through
/// the 77-bit parser.
pub fn decode_uvpacket(audio_12k: &[f32], at: i64) -> Vec<UvPacketFrame> {
    use mfsk_core::uvpacket::{AUDIO_CENTRE_HZ, rx};
    rx::decode(audio_12k, AUDIO_CENTRE_HZ)
        .into_iter()
        .map(|f| UvPacketFrame {
            at,
            mode: uv_mode(f.mode),
            app_type: f.app_type,
            sequence: f.sequence,
            block_count: f.block_count,
            snr_db: f.snr_db.round() as i16,
            payload: f.payload,
        })
        .collect()
}

/// Map mfsk-core's UVPacket sub-mode onto the stable [`UvPacketMode`].
fn uv_mode(m: mfsk_core::uvpacket::puncture::Mode) -> UvPacketMode {
    use mfsk_core::uvpacket::puncture::Mode as M;
    match m {
        M::Robust => UvPacketMode::Robust,
        M::Standard => UvPacketMode::Standard,
        M::UltraRobust => UvPacketMode::UltraRobust,
        M::Express => UvPacketMode::Express,
    }
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
/// Addressing for a decode that arrives as text alone.
///
/// MSK144 and Q65 come back from mfsk-core already unpacked, and FSK441 is
/// plain text by design, so there are no message-type bits to say whether a
/// row is a standard `<to> <from> <grid|report>` or free text. It is read as a
/// standard message and kept as one only when the calls it names look like
/// callsigns; anything else is free text and names nobody. Without that,
/// `TNX 73 GL` reads as a message from "73", which goes on the map and to PSK
/// Reporter as a station heard.
fn parse_text_decode(text: &str) -> Parsed {
    let p = parse_message(text, MsgKind::Standard);
    // An unresolved hashed call (`<...>`) is already `None` here.
    let names_a_station = |c: &Option<String>| c.as_deref().is_none_or(is_callish);
    if names_a_station(&p.to) && names_a_station(&p.from) {
        p
    } else {
        Parsed { free_text: true, ..Default::default() }
    }
}

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

    /// A synthesized MSK144 frame decodes back to its message — the round trip
    /// the controller makes. The waveform is the reference binary-FSK one
    /// mfsk-core's own sweep uses (`msk144sim`'s), not our TX path, so a decode
    /// here is the decoder working and not the modulator agreeing with itself.
    #[test]
    fn msk144_frame_round_trips() {
        use mfsk_core::FecCodec;
        use mfsk_core::msg::wsjt77::pack77;

        const FS: f32 = 12_000.0;
        // A single frame, continuous phase, 2000 baud, tone spacing = baud.
        fn frame_itone(call1: &str, call2: &str, report: &str) -> [u8; 144] {
            let msg = pack77(call1, call2, report).expect("pack77");
            let mut info = [0u8; 90];
            info[..77].copy_from_slice(&msg);
            let mut bytes = [0u8; 12];
            for (i, &b) in info[..77].iter().enumerate() {
                bytes[i / 8] |= (b & 1) << (7 - (i % 8));
            }
            let crc = mfsk_core::fec::ldpc_128_90::crc13(&bytes);
            for i in 0..13 {
                info[77 + i] = ((crc >> (12 - i)) & 1) as u8;
            }
            let mut codeword = [0u8; 128];
            mfsk_core::fec::Ldpc128_90.encode(&info, &mut codeword);
            // OQPSK frame -> the tone sequence the reference synth plays,
            // through the same differential transform mfsk-core's own sweep
            // uses (`build_i4tone`), so a decode here is not our modulator
            // agreeing with our demodulator.
            let bitseq = mfsk_core::engine::dsp::msk::build_bitseq(&codeword);
            let mut bp = [0i8; 144];
            for i in 0..144 {
                bp[i] = 2 * bitseq[i] as i8 - 1;
            }
            let mut i4 = [0i8; 144];
            for i in 1..=72usize {
                i4[2 * i - 2] = (bp[2 * i - 1] * bp[2 * i - 2] + 1) / 2;
                i4[2 * i - 1] = -((bp[2 * i - 1] * bp[(2 * i) % 144] - 1) / 2);
            }
            let mut t = [0u8; 144];
            for i in 0..144 {
                t[i] = (-i4[i] + 1) as u8;
            }
            t
        }
        fn wave(itone: &[u8; 144], freq: f32) -> Vec<i16> {
            let twopi = 2.0 * std::f32::consts::PI;
            let baud = 2000.0f32;
            let d0 = twopi * (freq - 0.25 * baud) / FS;
            let d1 = twopi * (freq + 0.25 * baud) / FS;
            let mut phi = 0.0f32;
            let mut out = Vec::with_capacity(144 * 6);
            for &tone in itone {
                let d = if tone == 0 { d0 } else { d1 };
                for _ in 0..6 {
                    out.push((phi.cos() * 12_000.0) as i16);
                    phi += d;
                    if phi >= twopi {
                        phi -= twopi;
                    }
                }
            }
            out
        }

        let itone = frame_itone("K1ABC", "W9XYZ", "EN37");
        // A meteor ping per second through a 15 s slot: the frame plays
        // continuously but is enveloped by the ionised trail's decay
        // (`2.718*t*exp(-t)`, the shape mfsk-core's own sweep uses). MSK144's
        // decoder hunts these bursts, so a bare continuous carrier is not a
        // signal it is built to find.
        let one = wave(&itone, 1500.0);
        let npts = 15 * FS as usize;
        let mut carrier = Vec::with_capacity(npts);
        while carrier.len() < npts {
            carrier.extend_from_slice(&one);
        }
        carrier.truncate(npts);
        let mut slot = Vec::with_capacity(npts);
        for (i, &s) in carrier.iter().enumerate() {
            let iping = (i / FS as usize).clamp(1, 14);
            let t = (i as f32 / FS - iping as f32) / 0.2;
            let env =
                if (0.0..=10.0).contains(&t) { std::f32::consts::E * t * (-t).exp() } else { 0.0 };
            slot.push((s as f32 * env) as i16);
        }
        let decodes = decode_msk144_slot(&slot, 1500.0, 0);
        assert!(
            decodes.iter().any(|d| d.message == "K1ABC W9XYZ EN37"),
            "MSK144 did not round-trip: {decodes:?}"
        );
        // A station a little off the cursor is still inside the search...
        let decodes = decode_msk144_slot(&slot, 1650.0, 0);
        assert!(
            decodes.iter().any(|d| d.message == "K1ABC W9XYZ EN37"),
            "150 Hz off the cursor was not found: {decodes:?}"
        );
        // ...and one well outside it is not searched for.
        let decodes = decode_msk144_slot(&slot, 2100.0, 0);
        assert!(decodes.is_empty(), "600 Hz off the cursor still decoded: {decodes:?}");
    }

    /// A synthesized JT65 and JT9 message decodes back to the same
    /// `<to> <from> <grid>` — the round trip the JT controller makes — with the
    /// DT an on-time station has in WSJT-X's column, about zero. JT65 is also
    /// placed low in the passband, where mfsk-core's default 1000–2000 Hz
    /// window would never have looked.
    #[test]
    fn jt_messages_round_trip() {
        for (mode, hz) in [(Mode::Jt65, 1000.0), (Mode::Jt65, 600.0), (Mode::Jt9, 1000.0)] {
            let synth = match mode {
                Mode::Jt65 => {
                    mfsk_core::jt65::tx::synthesize_standard("CQ", "K1ABC", "FN42", 12_000, hz, 0.3)
                }
                _ => {
                    mfsk_core::jt9::tx::synthesize_standard("CQ", "K1ABC", "FN42", 12_000, hz, 0.3)
                }
            }
            .expect("synthesize");
            // The scan wants a whole 60-second slot; pad the burst into one.
            let mut slot = vec![0.0f32; 12_000]; // the one-second TX offset
            slot.extend_from_slice(&synth);
            slot.resize(60 * 12_000, 0.0);
            let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();

            let decodes = decode_jt_slot(&i16buf, mode, 0);
            let best =
                decodes.first().unwrap_or_else(|| panic!("{mode:?} at {hz} Hz: nothing decoded"));
            assert_eq!(best.from.as_deref(), Some("K1ABC"), "{mode:?}: {decodes:?}");
            assert!(best.is_cq, "{mode:?}: {decodes:?}");
            assert_eq!(best.grid.as_deref(), Some("FN42"), "{mode:?}: {decodes:?}");
            assert!(best.dt.abs() < 0.3, "{mode:?}: an on-time signal read DT {}", best.dt);
        }
    }

    /// Noise alone decodes to nothing in either JT mode. The 72-bit message
    /// has no CRC, so this is the decoder's own gates being tested — several
    /// deterministic seeds, so a marginal gate shows up here rather than as an
    /// invented callsign on an empty band.
    #[test]
    fn jt_noise_alone_decodes_nothing() {
        for mode in [Mode::Jt65, Mode::Jt9] {
            for seed in 1..=3u32 {
                let mut rng = seed.wrapping_mul(0x9e37_79b9);
                let noise: Vec<i16> = (0..60 * 12_000)
                    .map(|_| {
                        rng ^= rng << 13;
                        rng ^= rng >> 17;
                        rng ^= rng << 5;
                        ((rng as i32 as f32 / i32::MAX as f32) * 6_000.0) as i16
                    })
                    .collect();
                let decodes = decode_jt_slot(&noise, mode, 0);
                assert!(decodes.is_empty(), "{mode:?} seed {seed}: noise decoded as {decodes:?}");
            }
        }
    }

    /// A text-only decode (MSK144, Q65, FSK441) is addressed only when its
    /// calls look like callsigns: free text names nobody, so it never reaches
    /// the map or PSK Reporter as a station heard.
    #[test]
    fn free_text_names_no_station() {
        for text in ["TNX 73 GL", "PSE QSL", "RRR", "73", "K1ABC TNX"] {
            let p = parse_text_decode(text);
            assert!(p.free_text && p.from.is_none() && p.to.is_none(), "{text}: {p:?}");
        }
        let p = parse_text_decode("K1ABC W9XYZ EN37");
        assert!(!p.free_text);
        assert_eq!((p.to.as_deref(), p.from.as_deref()), (Some("K1ABC"), Some("W9XYZ")));
        let p = parse_text_decode("CQ K1ABC FN42");
        assert!(p.is_cq && !p.free_text);
        assert_eq!(p.from.as_deref(), Some("K1ABC"));
        // An unresolved hashed call is not a reason to call the row free text.
        let p = parse_text_decode("<...> W9XYZ R-05");
        assert!(!p.free_text);
        assert_eq!(p.from.as_deref(), Some("W9XYZ"));
    }

    /// `Fst4Period` states FST4's geometry in `sdroxide-types`, which cannot
    /// depend on mfsk-core; this is where the two are held to agree — the slot,
    /// the symbol length and the transmit offset every decode's DT is measured
    /// from. FST4-15 alone keys half a second in.
    #[test]
    fn fst4_periods_match_mfsk_cores_geometry() {
        use mfsk_core::engine::{FrameLayout, ModulationParams};
        use sdroxide_types::Fst4Period;
        fn check<P: FrameLayout + ModulationParams>(p: Fst4Period) {
            assert_eq!(p.nsps(), P::NSPS as usize, "FST4-{} NSPS", p.label());
            assert_eq!(p.slot_s(), f64::from(P::T_SLOT_S), "FST4-{} slot", p.label());
            assert_eq!(
                p.start_delay_s(),
                f64::from(P::TX_START_OFFSET_S),
                "FST4-{} transmit offset",
                p.label()
            );
        }
        check::<mfsk_core::fst4::Fst4s15>(Fst4Period::P15);
        check::<mfsk_core::fst4::Fst4s30>(Fst4Period::P30);
        check::<mfsk_core::fst4::Fst4s60>(Fst4Period::P60);
        check::<mfsk_core::fst4::Fst4s120>(Fst4Period::P120);
        check::<mfsk_core::fst4::Fst4s300>(Fst4Period::P300);
    }

    /// A synthesized FST4 message decodes back at every period. FST4 shares
    /// FT8's 77-bit message with no CRC-free ambiguity, so exactly one decode
    /// of the right message is expected; the period decides the slot the audio
    /// has to be padded into.
    #[test]
    fn fst4_messages_round_trip_at_every_period() {
        use sdroxide_types::Fst4Period;
        for period in Fst4Period::ALL {
            let msg77 = mfsk_core::msg::wsjt77::pack77("CQ", "K1ABC", "FN42").expect("pack77");
            let itone = mfsk_core::fst4::encode::message_to_tones(&msg77);
            let cfg = match period {
                Fst4Period::P15 => &mfsk_core::fst4::encode::FST4_15_GFSK,
                Fst4Period::P30 => &mfsk_core::fst4::encode::FST4_30_GFSK,
                Fst4Period::P60 => &mfsk_core::fst4::encode::FST4_60A_GFSK,
                Fst4Period::P120 => &mfsk_core::fst4::encode::FST4_120_GFSK,
                Fst4Period::P300 => &mfsk_core::fst4::encode::FST4_300_GFSK,
            };
            let burst = mfsk_core::fst4::encode::tones_to_f32_with_gfsk(&itone, 1000.0, 0.3, cfg);
            // Pad into a whole slot at the period's TX offset.
            let mut slot = vec![0.0f32; (period.start_delay_s() * 12_000.0).round() as usize];
            slot.extend_from_slice(&burst);
            slot.resize((period.slot_s() * 12_000.0) as usize, 0.0);
            let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();

            let decodes = decode_fst4_slot(&i16buf, period, 0);
            let best = decodes
                .iter()
                .find(|d| d.from.as_deref() == Some("K1ABC"))
                .unwrap_or_else(|| panic!("FST4-{}: nothing decoded: {decodes:?}", period.label()));
            assert!(best.is_cq, "FST4-{}: {decodes:?}", period.label());
            assert_eq!(best.grid.as_deref(), Some("FN42"), "FST4-{}", period.label());
            // Keyed at the period's own offset, it is on time.
            assert!(best.dt.abs() < 0.3, "FST4-{}: DT {}", period.label(), best.dt);
        }
    }

    /// A synthesized Q65 message decodes back at every sub-mode — the round trip
    /// the controller makes. Q65 shares FT8's 77-bit message, so the message is
    /// expected whole; the sub-mode decides both the slot the audio is padded
    /// into and the protocol type the decode is run as.
    ///
    /// `#[ignore]`d because the ten scans take about 20 s in a test build,
    /// most of it the long sub-modes; run it with
    /// `cargo test -p sdroxide-digi --lib -- --ignored q65`. The cheapest
    /// sub-mode has its own unignored smoke test below, and
    /// `q65_sub_modes_match_mfsk_cores_geometry` checks every sub-mode's
    /// mapping on every run.
    #[test]
    #[ignore = "Q65 scans take ~20 s; run with -- --ignored"]
    fn q65_messages_round_trip_at_every_sub_mode() {
        use sdroxide_types::Q65Mode;

        q65_round_trip::<mfsk_core::q65::Q65a15>(Q65Mode::A15);
        q65_round_trip::<mfsk_core::q65::Q65a30>(Q65Mode::A30);
        q65_round_trip::<mfsk_core::q65::Q65a60>(Q65Mode::A60);
        q65_round_trip::<mfsk_core::q65::Q65b60>(Q65Mode::B60);
        q65_round_trip::<mfsk_core::q65::Q65c60>(Q65Mode::C60);
        q65_round_trip::<mfsk_core::q65::Q65d60>(Q65Mode::D60);
        q65_round_trip::<mfsk_core::q65::Q65e60>(Q65Mode::E60);
        q65_round_trip::<mfsk_core::q65::Q65d120>(Q65Mode::D120);
        q65_round_trip::<mfsk_core::q65::Q65e120>(Q65Mode::E120);
        q65_round_trip::<mfsk_core::q65::Q65a300>(Q65Mode::A300);
    }

    /// The Q65 round trip at the cheapest sub-mode, so an ordinary test run
    /// still exercises the decoder. The full sweep is `#[ignore]`d above.
    #[test]
    fn q65_message_round_trips() {
        q65_round_trip::<mfsk_core::q65::Q65a15>(sdroxide_types::Q65Mode::A15);
    }

    /// Synthesize a `CQ K1ABC FN42` at sub-mode `m`, pad it into a whole slot at
    /// the sub-mode's own TX offset, and assert it decodes back on time.
    fn q65_round_trip<P: mfsk_core::engine::ModulationParams>(m: sdroxide_types::Q65Mode) {
        let burst = mfsk_core::q65::tx::synthesize_standard_for::<P>(
            "CQ", "K1ABC", "FN42", 12_000, 1000.0, 0.3,
        )
        .expect("synthesize");
        let mut slot = vec![0.0f32; (m.start_delay_s() * 12_000.0).round() as usize];
        slot.extend_from_slice(&burst);
        slot.resize((m.slot_s() * 12_000.0) as usize, 0.0);
        let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();

        let decodes = decode_q65_slot(&i16buf, m, 0);
        let best = decodes
            .iter()
            .find(|d| d.from.as_deref() == Some("K1ABC"))
            .unwrap_or_else(|| panic!("Q65-{}: nothing decoded: {decodes:?}", m.label()));
        assert!(best.is_cq, "Q65-{}: {decodes:?}", m.label());
        assert_eq!(best.grid.as_deref(), Some("FN42"), "Q65-{}", m.label());
        // mfsk-core places a frame to a fraction of a symbol, and the long
        // sub-modes' symbols are long: 120D reads a third of a second early and
        // 300A (3.5 s symbols) a whole second. "On time" is to half a symbol
        // there; the short sub-modes are held to 0.3 s.
        let half_symbol = m.nsps() as f32 / 12_000.0 / 2.0;
        assert!(best.dt.abs() <= 0.3f32.max(half_symbol), "Q65-{}: DT {}", m.label(), best.dt);
    }

    /// `Q65Mode` states each sub-mode's geometry in `sdroxide-types`, which
    /// cannot depend on mfsk-core; this holds the two together — the slot and
    /// the symbol length that decide the burst, and so which protocol type a
    /// sub-mode is decoded as. (The transmit offset is WSJT-X's rather than
    /// mfsk-core's, which says one second for all of them.)
    #[test]
    fn q65_sub_modes_match_mfsk_cores_geometry() {
        use mfsk_core::engine::{FrameLayout, ModulationParams};
        use sdroxide_types::Q65Mode;
        fn check<P: FrameLayout + ModulationParams>(m: Q65Mode) {
            assert_eq!(m.nsps(), P::NSPS as usize, "Q65-{} NSPS", m.label());
            assert_eq!(m.slot_s(), f64::from(P::T_SLOT_S), "Q65-{} slot", m.label());
        }
        check::<mfsk_core::q65::Q65a15>(Q65Mode::A15);
        check::<mfsk_core::q65::Q65a30>(Q65Mode::A30);
        check::<mfsk_core::q65::Q65a60>(Q65Mode::A60);
        check::<mfsk_core::q65::Q65b60>(Q65Mode::B60);
        check::<mfsk_core::q65::Q65c60>(Q65Mode::C60);
        check::<mfsk_core::q65::Q65d60>(Q65Mode::D60);
        check::<mfsk_core::q65::Q65e60>(Q65Mode::E60);
        check::<mfsk_core::q65::Q65d120>(Q65Mode::D120);
        check::<mfsk_core::q65::Q65e120>(Q65Mode::E120);
        check::<mfsk_core::q65::Q65a300>(Q65Mode::A300);
    }

    /// A synthesized FSK441 ping decodes through the modem adapter into a
    /// [`Decode`] with the text and its parsed addressing. The waveform is the
    /// fork's own generator at the decoder's 11 025 Hz, with noise and a small
    /// amplitude behind it, so this exercises the adapter and the parser rather
    /// than a bare round trip of the generator against itself.
    #[test]
    fn fsk441_ping_round_trips_through_the_adapter() {
        use sdroxide_dsp::{FSK441_RATE, fsk441_encode_tones, fsk441_generate_audio};

        let msg = "W1ABC W9XYZ FN42";
        let audio = fsk441_generate_audio(&fsk441_encode_tones(msg));
        let npts = FSK441_RATE as usize * 2;
        let start = npts / 3;
        // Deterministic noise, so the test cannot flake.
        let mut slot = vec![0.0f32; npts];
        let mut state = 0x9e37_79b9u32;
        for s in slot.iter_mut() {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *s = ((state >> 8) as f32 / 8_388_608.0 - 1.0) * 0.15;
        }
        for (i, &s) in audio.iter().enumerate() {
            if start + i < npts {
                slot[start + i] += s * 0.7;
            }
        }

        let decodes = decode_fsk441_slot(&slot, 0);
        let d = decodes
            .iter()
            .find(|d| d.message == msg)
            .unwrap_or_else(|| panic!("FSK441 did not round-trip: {decodes:?}"));
        assert_eq!(d.to.as_deref(), Some("W1ABC"));
        assert_eq!(d.from.as_deref(), Some("W9XYZ"));
        assert_eq!(d.grid.as_deref(), Some("FN42"));
        // The ping is reported where it was keyed.
        assert!((d.dt - start as f32 / FSK441_RATE as f32).abs() < 0.06, "dt {}", d.dt);
    }

    /// A synthesized UVPacket frame decodes back at every sub-mode — the round
    /// trip the controller makes. The sub-mode is carried by the preamble and
    /// detected, so each frame must come back labelled with the one it was sent
    /// at, header fields and payload whole.
    #[test]
    fn uvpacket_frames_round_trip_at_every_sub_mode() {
        use mfsk_core::uvpacket::puncture::Mode as UvM;
        use sdroxide_types::UvPacketMode;

        // Exactly two 12-byte payload blocks, all printable, so the round trip
        // checks the text reading too.
        const PAYLOAD: &[u8] = b"UV packet test payload!!";
        assert_eq!(PAYLOAD.len(), 24);

        for (mode, uv) in [
            (UvPacketMode::Robust, UvM::Robust),
            (UvPacketMode::Standard, UvM::Standard),
            (UvPacketMode::UltraRobust, UvM::UltraRobust),
            (UvPacketMode::Express, UvM::Express),
        ] {
            let header = mfsk_core::uvpacket::framing::FrameHeader {
                mode: uv,
                block_count: 2,
                app_type: 5,
                sequence: 7,
            };
            let burst = mfsk_core::uvpacket::tx::encode(
                &header,
                PAYLOAD,
                mfsk_core::uvpacket::AUDIO_CENTRE_HZ,
            )
            .expect("encode");
            // Pad into a window with a second either side of the burst.
            let mut audio = vec![0.0f32; burst.len() + 24_000];
            audio[12_000..12_000 + burst.len()].copy_from_slice(&burst);

            let frames = decode_uvpacket(&audio, 0);
            let best = frames
                .iter()
                .find(|f| f.mode == mode)
                .unwrap_or_else(|| panic!("{mode:?}: nothing decoded: {frames:?}"));
            assert_eq!(best.app_type, 5, "{mode:?}");
            assert_eq!(best.sequence, 7, "{mode:?}");
            assert_eq!(best.block_count, 2, "{mode:?}");
            assert_eq!(best.payload, PAYLOAD, "{mode:?}");
            assert_eq!(best.as_text().as_deref(), Some("UV packet test payload!!"), "{mode:?}");
        }
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

    /// A weak FT4 signal buried under a strong one at the same audio offset
    /// must still decode. This is what multi-pass SIC buys and what the
    /// single-pass default cannot do: the strong signal's own occupied
    /// bandwidth (90 Hz) covers the weak one, and only subtraction of the
    /// strong decode exposes it in the residual. WSJT-X and WSJT-CB subtract
    /// by default, so without this a busy FT4 slot reports only the strongest
    /// signal at each offset — the "they don't stack" report.
    ///
    /// The two decodes are co-channel on purpose: at any wider separation the
    /// single-pass default would already find both, and the test would stop
    /// pinning subtraction at all. Noise is deterministic, so this cannot
    /// flake between runs.
    #[test]
    fn a_masked_ft4_signal_is_recovered_by_subtraction() {
        let modem = Ft8Modem::new(Mode::Ft4);
        let strong = "CQ AB1CD FN42";
        let weak = "CQ EF2GH JO22";
        let pad = (0.5 * 12_000.0) as usize;
        let (sb, _) = modem.encode_burst_12k(strong, 1500.0, 0.5).expect("encode strong");
        let (wb, _) = modem.encode_burst_12k(weak, 1500.0, 0.03).expect("encode weak");
        let mut slot = vec![0.0f32; (7.5 * 12_000.0) as usize - pad];
        for (i, &s) in sb.iter().enumerate() {
            if i < slot.len() {
                slot[i] += s;
            }
        }
        for (i, &s) in wb.iter().enumerate() {
            if i < slot.len() {
                slot[i] += s;
            }
        }
        let mut rng: u32 = 0x9e37_79b9;
        for s in slot.iter_mut() {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            *s += (rng as i32 as f32 / i32::MAX as f32) * 0.005;
        }
        let mut padded = vec![0.0f32; pad];
        padded.extend_from_slice(&slot);
        let buf: Vec<i16> = padded.iter().map(|&s| (s * 20_000.0) as i16).collect();

        let mut rx = Ft8Modem::new(Mode::Ft4);
        let got = rx.decode_slot(&buf, 0, &ApHints::default(), 1500.0);
        let messages: Vec<&str> = got.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.contains(&strong), "the strong signal decoded: {messages:?}");
        assert!(
            messages.contains(&weak),
            "the weak signal under it did not survive subtraction: {messages:?}",
        );
    }

    /// Noise alone must not decode, now that the wide-band pass subtracts:
    /// mfsk-core runs the second round at three quarters of the sync floor
    /// even when the first found nothing, so a slot of plain noise is searched
    /// deeper than it used to be and CRC-14 is all that stands between that
    /// search and an invented callsign. Several deterministic seeds, so a
    /// marginal floor shows up here rather than on the air.
    #[test]
    fn ft4_noise_alone_decodes_nothing() {
        let n = (7.5 * 12_000.0) as usize;
        for seed in 1..=8u32 {
            let mut rng = seed.wrapping_mul(0x9e37_79b9);
            let buf: Vec<i16> = (0..n)
                .map(|_| {
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    ((rng as i32 as f32 / i32::MAX as f32) * 6_000.0) as i16
                })
                .collect();
            let mut rx = Ft8Modem::new(Mode::Ft4);
            let got = rx.decode_slot(&buf, 0, &ApHints::default(), 1500.0);
            let messages: Vec<&str> = got.iter().map(|d| d.message.as_str()).collect();
            assert!(messages.is_empty(), "seed {seed}: noise decoded as {messages:?}");
        }
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

    /// Answering a non-standard callsign has to carry the report, which the
    /// non-standard layout has no room for. It goes out as an ordinary message
    /// with that callsign hashed instead, the way WSJT-X sends it — issue #348
    /// is a reply to R7KJG/QRP whose "R-12" never left the building.
    #[test]
    fn a_report_to_a_compound_call_is_carried_by_hashing_it() {
        let mut modem = Ft8Modem::new(Mode::Ft8);
        // Both ends of a real contact know the callsign: the far end because
        // it is its own, this end because the CQ that opened it spelled it out.
        modem.seed_hashes(&["R7KJG/QRP".to_string()]);
        let (burst, sent) = modem.encode_burst_12k("R7KJG/QRP F4CYH R-12", 1500.0, 0.5).unwrap();
        assert_eq!(sent, "<R7KJG/QRP> F4CYH R-12");

        let mut slot = vec![0.0f32; 6_000];
        slot.extend_from_slice(&burst);
        slot.resize(15 * 12_000, 0.0);
        let buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        let d = modem
            .decode_slot(&buf, 0, &ApHints::default(), 1500.0)
            .into_iter()
            .find(|d| d.message.contains("F4CYH"))
            .expect("decoded");
        assert_eq!(d.message, "<R7KJG/QRP> F4CYH R-12");
        assert_eq!(d.to.as_deref(), Some("R7KJG/QRP"), "the compound call is the addressee");
        assert_eq!(d.from.as_deref(), Some("F4CYH"));
    }

    /// The same both ways round: a non-standard station reporting to a
    /// standard one hashes its own callsign, in the second field.
    #[test]
    fn a_report_from_a_compound_call_hashes_the_senders_own() {
        let (bits, sent) = pack_message("F4CYH R7KJG/QRP -12").expect("packs");
        assert_eq!(sent, "F4CYH <R7KJG/QRP> -12");
        assert_eq!(msg_kind(&bits), MsgKind::Standard);
        let mut ht = CallsignHashTable::new();
        ht.insert("R7KJG/QRP");
        assert_eq!(
            wsjt77::unpack77_with_hash(&bits, &ht).as_deref(),
            Some("F4CYH <R7KJG/QRP> -12")
        );
    }

    /// ...and a grid, which the non-standard layout drops just as silently.
    #[test]
    fn a_grid_to_a_compound_call_survives_too() {
        let (_, sent) = pack_message("R7KJG/QRP F4CYH JN18").expect("packs");
        assert_eq!(sent, "<R7KJG/QRP> F4CYH JN18");
    }

    /// The bare acknowledgements stay on the non-standard layout, which
    /// carries them whole *and* spells the callsign out — worth more than the
    /// hash saves, and what everything else on the band resolves from.
    #[test]
    fn an_acknowledgement_to_a_compound_call_still_spells_it_out() {
        for rpt in ["RRR", "RR73", "73"] {
            let (bits, sent) = pack_message(&format!("R7KJG/QRP F4CYH {rpt}")).expect("packs");
            assert_eq!(sent, format!("R7KJG/QRP <F4CYH> {rpt}"));
            assert_eq!(msg_kind(&bits), MsgKind::NonStandard);
        }
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
