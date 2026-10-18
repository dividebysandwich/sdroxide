//! The mfsk-core adapter: FT8/FT4 decode and encode wrapped behind stable
//! sdroxide types. **This is the only file that touches raw mfsk-core
//! decode-result fields** — if the crate's field names change, only
//! `decode_slot` needs updating.

use std::collections::VecDeque;

use mfsk_core::msg::decode_request::DecodeRequest;
use mfsk_core::msg::hash_table::CallsignHashTable;
use mfsk_core::msg::wsjt77;
use sdroxide_types::{Decode, Mode, UvPacketFrame, UvPacketMode};

use crate::params::{AUDIO_MAX_HZ, AUDIO_MIN_HZ};

/// Whether a callsign field in a decoded 77-bit message is acceptable to
/// the whole gate — an 11 m identifier ([`sdroxide_types::is_cb_callsign`])
/// or a call the codec's own per-field verdict already passes ([`wsjt77::is_plausible_call`],
/// the `CQ`/`DE`/`QRZ` and `<...>` tags included).
///
/// Handed to mfsk-core's field-based decode hook
/// (`DecodeRequest::also_accept`, jl1nie/mfsk-core#386): the stock FT8
/// per-field gate refuses a CB-shaped call such as "26AT715", which
/// drops the whole line, so we widen it. The hook yields only the
/// callsign fields, so a grid or a report never reaches the grammar.
fn is_cb_compatible_call(call: &str, wide: bool) -> bool {
    wsjt77::is_plausible_call(call) || cb_ok(call, wide)
}

/// Whether a string is an 11 m callsign under the grammar in force: WSJT-CB's
/// own ([`sdroxide_types::is_cb_callsign`]), or the experimental wider one
/// ([`sdroxide_types::is_cb_callsign_wide`]) when the operator has opted in.
/// One place, so the decode gate and the pack ladder can never disagree about
/// which grammar a station is running.
fn cb_ok(call: &str, wide: bool) -> bool {
    sdroxide_types::is_cb_callsign_with(call, wide)
}

/// Whether a callsign may be packed or hashed — an 11 m identifier or a
/// call the codec's validator accepts. The mfsk-core fork used to widen
/// `is_valid_callsign` itself for CB identifiers; the union is that same
/// widening, kept at the call sites that have to decide.
fn is_packable_call(call: &str, wide: bool) -> bool {
    wsjt77::is_valid_callsign(call) || cb_ok(call, wide)
}

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

/// How many callsigns [`Ft8Modem::known_calls`] keeps, matching the hash
/// table's own cap (WSJT-X's `MAXHASH`) so the two evict together.
const MAX_KNOWN_CALLS: usize = 1000;

/// Encode/decode engine for one digital mode.
pub struct Ft8Modem {
    mode: Mode,
    /// Callsigns heard this session, so the `<...>` placeholders in
    /// non-standard-callsign and DXpedition messages resolve to real calls.
    hashes: CallsignHashTable,
    /// The same callsigns again, hashed the way the EU VHF contest layout
    /// needs them — see [`eu_vhf::Hashes`].
    eu_hashes: eu_vhf::Hashes,
    /// Accept the experimental wider 11 m callsign grammar
    /// ([`sdroxide_types::is_cb_callsign_wide`]) as well as WSJT-CB's. Off
    /// unless the operator opts in — see [`Ft8Modem::set_cb_wide`].
    cb_wide: bool,
    /// How hard the FT8 decoder works for weak signals — see
    /// [`sdroxide_types::Ft8Depth`]. Only the extra (post-quick) batch uses it;
    /// the plain single pass that is emitted first is always run.
    ft8_depth: sdroxide_types::Ft8Depth,
    /// The same callsigns `hashes` holds, **in the clear and newest-first**, so
    /// an operator can be shown who their hashes currently resolve to.
    ///
    /// A shadow rather than a read of `hashes`, because mfsk-core's
    /// `CallsignHashTable` is lookup-only: it offers `len22`/`capacity22` and
    /// nothing that enumerates the calls, so there is no way to ask it what it
    /// knows. Both [`Ft8Modem::remember_heard`] and [`Ft8Modem::seed_hashes`]
    /// are the *only* feeders of that table, and both see each callsign in
    /// plain text on the way in — so mirroring them here keeps the two in step
    /// by construction rather than by reverse-engineering the table.
    ///
    /// The cap and the newest-wins-a-collision order match the table's, so a
    /// callsign this list still shows is one the table can still resolve, and
    /// one it has forgotten is dropped here too. That is the whole point: a
    /// stale entry would promise a resolution the decoder can no longer make.
    known_calls: VecDeque<String>,
}

impl Ft8Modem {
    pub fn new(mode: Mode) -> Self {
        Ft8Modem {
            mode,
            hashes: CallsignHashTable::new(),
            eu_hashes: eu_vhf::Hashes::default(),
            cb_wide: false,
            ft8_depth: sdroxide_types::Ft8Depth::default(),
            known_calls: VecDeque::new(),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Choose the 11 m callsign grammar: WSJT-CB's own, or the experimental
    /// wider one. Set from [`sdroxide_types::DigiConfig::cb_wide_callsigns`] on
    /// every config change, so the decode gate, the pack ladder and the encoder
    /// all follow the operator's switch.
    pub fn set_cb_wide(&mut self, wide: bool) {
        self.cb_wide = wide;
    }

    /// How hard the FT8 decoder works — see [`sdroxide_types::Ft8Depth`].
    pub fn set_ft8_depth(&mut self, depth: sdroxide_types::Ft8Depth) {
        self.ft8_depth = depth;
    }

    pub fn ft8_depth(&self) -> sdroxide_types::Ft8Depth {
        self.ft8_depth
    }

    /// Register callsigns we already know (ours, and the station we're
    /// working), so the first hashed message naming them resolves instead of
    /// showing `<...>`.
    pub fn seed_hashes(&mut self, calls: &[String]) {
        for c in calls.iter().filter(|c| !c.is_empty()) {
            self.hashes.insert(c);
            self.eu_hashes.insert(c);
            self.remember_call(c);
        }
    }

    /// The callsigns a `<...>` in a decoded message can currently name, newest
    /// first — our own, the one being worked, and everyone heard spelled out.
    ///
    /// A **copy**, and capped by the caller: this crosses a thread boundary to
    /// a client, and the list behind it is a live ring on the decode worker.
    pub fn known_calls(&self) -> Vec<String> {
        self.known_calls.iter().cloned().collect()
    }

    /// How many callsigns the table holds, which is more than a capped reply
    /// shows — so a window can say "200 of 340" rather than imply it is all.
    pub fn known_calls_len(&self) -> usize {
        self.known_calls.len()
    }

    /// Mirror one callsign into [`Ft8Modem::known_calls`], newest first.
    ///
    /// The same normalisation and the same drops the hash tables apply, so the
    /// shadow never promises a resolution the tables cannot deliver: a token too
    /// short to be a call, a `...` placeholder and a `CQ` are all callers, not
    /// stations, and a re-heard station moves to the front rather than appearing
    /// twice. A portabled callsign is kept **whole** — `/P` and `/R` are part of
    /// the identity here, and the table hashes them whole.
    fn remember_call(&mut self, call: &str) {
        let call = eu_vhf::bare(call.trim()).to_ascii_uppercase();
        if call.len() < 3 || call == "..." || call.starts_with("CQ") {
            return;
        }
        if let Some(i) = self.known_calls.iter().position(|c| *c == call) {
            self.known_calls.remove(i);
        }
        self.known_calls.push_front(call);
        self.known_calls.truncate(MAX_KNOWN_CALLS);
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
        let (quick, extras) = self.decode_slot_staged(audio_12k, slot_utc, ap, listening_hz);
        let mut all = quick;
        all.extend(extras);
        all
    }

    /// Decode a slot and hand each result over as it is found (#667).
    ///
    /// For FT8 the quick single pass is emitted as one batch the moment it is
    /// done; the subtraction pass then emits each further decode as mfsk-core
    /// accepts it, so on a slow machine the list fills in through the deep pass
    /// rather than all at once at its end. `stop` is asked before each slower
    /// stage: answering true (a newer slot is already waiting) skips it, so a
    /// machine that cannot keep up with Deep falls behind by at most one pass,
    /// not by a growing queue of slots. Every other mode emits its whole decode
    /// once, as [`Self::decode_slot_staged`] would.
    pub fn decode_slot_streaming(
        &mut self,
        audio_12k: &[i16],
        slot_utc: i64,
        ap: &ApHints,
        listening_hz: f32,
        stop: &dyn Fn() -> bool,
        emit: &(dyn Fn(Vec<Decode>) + Sync),
    ) {
        if self.mode != Mode::Ft8 {
            let (quick, extras) = self.decode_slot_staged(audio_12k, slot_utc, ap, listening_hz);
            emit(quick);
            if !extras.is_empty() {
                emit(extras);
            }
            return;
        }
        let hint = ap.ft8();
        // **The CB widening has to be on *this* request too.**
        //
        // It is a fork-only hook (`DecodeRequest::also_accept`, mfsk-core#386):
        // upstream has no citizens'-band grammar, so the streaming rewrite above
        // rebuilt the request without it and the stock per-field gate went back
        // to refusing 11 m calls. The two halves failed differently, which is
        // what the bench saw on 2026-10-18 — *"messages come in but no CQ
        // calls"* on 27.265, where a week earlier two thirds of the decodes were
        // CQs. The report forms survived as unresolved hashes (`<19DC373>
        // <26AT715> R-12`); **every CQ form was dropped outright**, because a CQ
        // to a non-standard call is the one layout whose call cannot be carried
        // as a hash. See `a_cb_cq_survives_the_streaming_path`, which is this
        // hook's guard.
        let wide = self.cb_wide;
        let request = || {
            let req = DecodeRequest::<mfsk_core::Ft8>::new(
                audio_12k,
                AUDIO_MIN_HZ,
                AUDIO_MAX_HZ,
                SYNC_MIN,
                MAX_CAND,
            )
            .osd(true)
            .also_accept(|m| m.callsigns().all(|c| is_cb_compatible_call(c, wide)));
            match hint.as_ref() {
                Some(h) => req.ap_hint(h),
                None => req,
            }
        };
        let sent: std::sync::Mutex<Vec<Decode>> = std::sync::Mutex::new(Vec::new());
        {
            let ht = &self.hashes;
            let eu = &self.eu_hashes;
            // The quick pass, whole: a single pass reports from many threads
            // before it deduplicates, so it is taken as its result.
            let mut quick: Vec<Decode> = Vec::new();
            for r in request().decode().results {
                let Ok(bits) = <[u8; 77]>::try_from(r.message77()) else { continue };
                if let Some(d) =
                    build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
                    && !quick.iter().any(|o| same_signal(o, &d))
                {
                    quick.push(d);
                }
            }
            sent.lock().unwrap().extend(quick.iter().cloned());
            emit(quick);

            // The subtraction pass, one decode at a time as it is accepted.
            if self.ft8_depth != sdroxide_types::Ft8Depth::Fast && !stop() {
                let on_result = |r: &mfsk_core::ft8::decode::DecodeResult| {
                    let Ok(bits) = <[u8; 77]>::try_from(r.message77()) else { return };
                    let Some(d) =
                        build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
                    else {
                        return;
                    };
                    let mut s = sent.lock().unwrap();
                    if !s.iter().any(|o| same_signal(o, &d)) {
                        s.push(d.clone());
                        drop(s);
                        emit(vec![d]);
                    }
                };
                let req = request();
                let req = match self.ft8_depth {
                    sdroxide_types::Ft8Depth::Normal => req.sic_rounds(2),
                    _ => req.sic_early(),
                };
                let _ = req.on_result(&on_result).decode();
            }

            // The EU VHF contest rescue, as one more batch — see
            // `decode_slot_staged`.
            if ap.eu_vhf && !stop() {
                let mut rescued = Vec::new();
                for r in crate::ft8_eu::decode_slot(
                    audio_12k,
                    AUDIO_MIN_HZ,
                    AUDIO_MAX_HZ,
                    SYNC_MIN,
                    MAX_CAND,
                ) {
                    if let Some(d) =
                        build_decode(&r.msg77, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
                    {
                        let mut s = sent.lock().unwrap();
                        if !s.iter().any(|o| same_signal(o, &d)) {
                            s.push(d.clone());
                            rescued.push(d);
                        }
                    }
                }
                if !rescued.is_empty() {
                    emit(rescued);
                }
            }
        }
        // Remember who we heard, for the next slot's hashed messages.
        for d in sent.into_inner().unwrap() {
            for call in [d.to.as_deref(), d.from.as_deref()].into_iter().flatten() {
                self.hashes.insert(call);
                self.eu_hashes.insert(call);
            }
        }
    }

    /// Decode a slot in two stages for FT8, for a caller that must act on the
    /// quick result before the slow one lands.
    ///
    /// `.sic_early()` is one call that runs the whole checkpointed multi-pass
    /// and returns only at the end — about 1.15 s on a busy slot against FT8's
    /// 0.5 s transmit offset — so an auto-sequenced reply decided from its
    /// result goes out a cycle late. So the plain single-pass decode is returned
    /// as `quick` immediately, and the SIC pass, the CB-widened a-priori pass
    /// and everything else the existing arm does follow as `extras` (only what
    /// the quick pass did not already report). For every other mode `extras` is
    /// empty and `quick` is the whole decode.
    pub fn decode_slot_staged(
        &mut self,
        audio_12k: &[i16],
        slot_utc: i64,
        ap: &ApHints,
        listening_hz: f32,
    ) -> (Vec<Decode>, Vec<Decode>) {
        let mode = self.mode;
        let wide = self.cb_wide;
        let ht = &self.hashes;
        let eu = &self.eu_hashes;
        // FT8 fills this with the SIC extras; empty for every other mode.
        let mut staged_extras: Vec<Decode> = Vec::new();
        let mut decodes: Vec<Decode> = match mode {
            Mode::Ft4 => DecodeRequest::<mfsk_core::Ft4>::new(
                audio_12k,
                AUDIO_MIN_HZ,
                AUDIO_MAX_HZ,
                SYNC_MIN,
                MAX_CAND,
            )
            .osd(true)
            // Same CB widening as the FT8 pass below (mfsk-core#386): FT4
            // carries a message policy through the generic pipeline too.
            .also_accept(|m| m.callsigns().all(|c| is_cb_compatible_call(c, wide)))
            // Subtraction, so a weak signal inside a stronger neighbour's
            // occupied bandwidth is still decoded — FT4's whole reason for
            // multi-pass SIC, and what WSJT-X/WSJT-CB run by default. The
            // default strategy is single-pass: without this a busy slot
            // reports only the strongest signal at each offset, which is the
            // "they don't stack" report against WSJT-X. Two rounds are the
            // measured choice (mfsk-core's FT4 WSJT-X sample: 11/14 without,
            // 14/14 with `.sic_rounds(2)`, nothing more from a third; 5 ms →
            // 71 ms against a 7.5 s slot). `also_accept` above keeps the
            // codec verdict on, so a subtracted phantom cannot erase a real
            // signal underneath it (mfsk-core issue #383).
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
                // Two stages, so an auto-sequenced reply can be decided inside
                // the transmit offset: the plain single-pass decode is always
                // emitted first, and the operator's chosen depth — Fast adds
                // nothing, Normal/Deep add subtraction — follows as `extras`.
                let req = |depth: sdroxide_types::Ft8Depth| {
                    let r = DecodeRequest::<mfsk_core::Ft8>::new(
                        audio_12k,
                        AUDIO_MIN_HZ,
                        AUDIO_MAX_HZ,
                        SYNC_MIN,
                        MAX_CAND,
                    )
                    .osd(true)
                    // Widen the per-call plausibility gate to the 11 m callsign
                    // grammar (see is_cb_compatible_call) — upstream's FT8
                    // decoder has no allowlist entry for a CB-shaped call.
                    .also_accept(|m| m.callsigns().all(|c| is_cb_compatible_call(c, wide)));
                    let r = match depth {
                        sdroxide_types::Ft8Depth::Fast => r,
                        sdroxide_types::Ft8Depth::Normal => r.sic_rounds(2),
                        sdroxide_types::Ft8Depth::Deep => r.sic_early(),
                    };
                    match hint.as_ref() {
                        Some(h) => r.ap_hint(h),
                        None => r,
                    }
                };
                let run = |depth: sdroxide_types::Ft8Depth| -> Vec<Decode> {
                    req(depth)
                        .decode()
                        .results
                        .into_iter()
                        .filter_map(|r| {
                            let bits: [u8; 77] = r.message77().try_into().ok()?;
                            build_decode(&bits, r.snr_db, r.dt_sec, r.freq_hz, slot_utc, ht, eu)
                        })
                        .collect()
                };
                let quick = run(sdroxide_types::Ft8Depth::Fast);
                // The deeper pass supersedes the quick one (a recall superset),
                // so stash only what it adds and let the caller see the quick
                // batch first. It rides the shared `extras` return below, and on
                // `Fast` there is no second pass at all.
                staged_extras = if self.ft8_depth == sdroxide_types::Ft8Depth::Fast {
                    Vec::new()
                } else {
                    run(self.ft8_depth)
                        .into_iter()
                        .filter(|d| !quick.iter().any(|q| same_signal(q, d)))
                        .collect()
                };
                quick
            }
        };
        // FT4 has no wide-band a-priori pass, only a targeted one. Aim it where
        // we are listening — in a QSO that is the station whose reply the hint
        // describes — and keep whatever it finds that the wide pass missed.
        if mode == Mode::Ft4 {
            if let Some(hint) = ap.ft4() {
                let extra = DecodeRequest::<mfsk_core::Ft4>::new(
                    audio_12k,
                    // mfsk-core 0.11 made the sniper FT8-only (FT4's
                    // `decode_sniper_ap` is gone), so the targeted pass is
                    // rebuilt on the wide-band request with the sniper's
                    // ±250 Hz aperture.
                    listening_hz - 250.0,
                    listening_hz + 250.0,
                    // The threshold the old `decode_sniper_ap` entry point applied
                    // for us, and looser than the wide pass's: with the hint's
                    // bits locked the FEC can carry a candidate whose coarse sync
                    // would never qualify on its own.
                    0.5,
                    MAX_CAND,
                )
                .osd(true)
                .eq_mode(mfsk_core::engine::equalize::EqMode::Off)
                .ap_hint(&hint)
                .also_accept(|m| m.callsigns().all(|c| is_cb_compatible_call(c, wide)))
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
        // Remember who we heard, for the next slot's hashed messages. Both
        // batches, so an extras-only station is still remembered.
        self.remember_heard(decodes.iter().chain(staged_extras.iter()));
        (decodes, staged_extras)
    }

    /// Feed decodes' callsigns to the hash tables, so a hash heard later
    /// resolves.
    ///
    /// Beyond the calls the decoder already spelled out, a **bare CB call** — a
    /// one-token free-text decode that is an 11 m callsign — is remembered too.
    /// WSJT-CB answers a CQ with exactly that ("26AT715", free text, no
    /// addressing) *so that* every listener can resolve the station's hash in
    /// everything that follows: it is the only message that spells the call
    /// out, and the exchange messages that carry it are hashed (`<us> <them>`,
    /// `<us> <them> -02`). Without this the pair decodes with `from = None`,
    /// the contact is never attributed, and the exchange stalls one message in
    /// (issue #396 follow-up).
    fn remember_heard<'d>(&mut self, decodes: impl IntoIterator<Item = &'d Decode>) {
        for d in decodes {
            for call in [d.to.as_deref(), d.from.as_deref()].into_iter().flatten() {
                self.hashes.insert(call);
                self.eu_hashes.insert(call);
                self.remember_call(call);
            }
            if d.free_text
                && !d.is_cq
                && d.message.split_whitespace().count() == 1
                && sdroxide_types::is_cb_callsign_with(&d.message, self.cb_wide)
            {
                self.hashes.insert(&d.message);
                self.eu_hashes.insert(&d.message);
                self.remember_call(&d.message);
            }
        }
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
        let (msg77, sent) = pack_message(text, self.cb_wide)?;
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
/// A JT decode carries **no CRC** (72 bits, no checksum), so a weak or empty
/// slot can converge on a well-formed-looking message that was never sent. The
/// decoder's own scan collapses duplicates and orders by sync score, and the
/// first result is the real one when there is one; the caller keeps only the
/// strongest few rather than trusting every row.
pub fn decode_jt_slot(audio_12k: &[i16], mode: Mode, slot_utc: i64) -> Vec<Decode> {
    let audio: Vec<f32> = audio_12k.iter().map(|&s| f32::from(s) / 28_000.0).collect();
    match mode {
        Mode::Jt65 => mfsk_core::jt65::decode_scan_default(&audio, DECODE_RATE_U32)
            .into_iter()
            .filter_map(|r| {
                let dt = r.dt_sec;
                jt_decode(r.message, r.snr_db, dt, r.freq_hz, slot_utc)
            })
            .collect(),
        Mode::Jt9 => mfsk_core::jt9::decode_scan_default(&audio, DECODE_RATE_U32)
            .into_iter()
            .filter_map(|r| {
                // JT9 exposes only the start index; the dt is relative to the
                // slot start, which is where the scan was given the buffer.
                let dt = r.start_sample as f32 / DECODE_RATE_U32 as f32;
                jt_decode(r.message, r.snr_db, dt, r.freq_hz, slot_utc)
            })
            .collect(),
        _ => Vec::new(),
    }
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
                // FST4 carries the same 77-bit message as FT8, with no
                // CB-shape gate needed here: FST4 has no hashed-callsign
                // layout and its caller is a VHF/EME station, not an 11 m one.
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

/// Decode one 15-second MSK144 slot of 12 kHz mono i16 audio.
///
/// MSK144 is not a frame at a fixed offset: the operator transmits through the
/// whole period and the decoder slides a window across the slot looking for
/// meteor-trail bursts, so it is handed the passband centre and half-width to
/// search rather than a start time — and each [`SlotDecode`] carries the time
/// *into* the slot (`.tsec`) the burst was found at, which becomes the
/// [`Decode`]'s `dt`.
///
/// Unlike every other mode here, mfsk-core resolves the message text inside
/// the decode call, so there are no raw 77 bits for us to unpack: the text is
/// already a `String` and goes through the same parser an FT8 decode does.
pub fn decode_msk144_slot(audio_12k: &[i16], slot_utc: i64) -> Vec<Decode> {
    use mfsk_core::msk144::decode::{Depth, decode_slot};
    let fc = (AUDIO_MIN_HZ + AUDIO_MAX_HZ) / 2.0;
    let ntol = (AUDIO_MAX_HZ - AUDIO_MIN_HZ) / 2.0;
    decode_slot(audio_12k, fc, ntol, Depth::Deep)
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
                free_text: false,
                rr73_to: None,
            }
        })
        .collect()
}

/// Decode one FSK441 slot of mono f32 audio at
/// [`sdroxide_dsp::FSK441_RATE`](sdroxide_dsp::FSK441_RATE).
///
/// FSK441 is a meteor-scatter mode: the operator transmits the message over and
/// over through the whole period and the receiver hears only the short
/// reflections off ionised trails, so the decoder hunts the slot for pings.
/// Each [`sdroxide_dsp::Fsk441Ping`] carries the time into the slot it was
/// found at, which becomes the [`Decode`]'s `dt` exactly as MSK144's does.
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
    // Q65 keys one second into the slot; the scan searches around that, and
    // its own asymmetric `SearchParams` window carries the tolerance.
    const NOMINAL_START: usize = 12_000;
    let params = SearchParams::default();

    macro_rules! run {
        ($p:ty) => {
            DecodeRequest::<$p>::new(&audio, DECODE_RATE_U32, NOMINAL_START, params)
                .decode()
                .into_iter()
                .map(|r| {
                    let p = parse_text_decode(&r.message);
                    Decode {
                        slot_utc,
                        snr_db: r.snr_db.round() as i16,
                        dt: r.dt_sec,
                        audio_hz: r.freq_hz,
                        message: r.message,
                        to: p.to,
                        from: p.from,
                        grid: p.grid,
                        is_cq: p.is_cq,
                        cq_to: p.cq_to,
                        free_text: false,
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

/// mfsk-core's JT entry points take a `u32` rate; the workspace's is `f64`.
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
fn pack_message(text: &str, wide: bool) -> Option<([u8; 77], String)> {
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

    // 1b. The ARRL RTTY Roundup / generic serial exchange, `i3 = 3`:
    //     "[TU; ]<to> <from> [R] <RST><state|serial>". Both calls must be
    //     standard calls — this layout spells them with `pack28`, which a
    //     non-standard CB call cannot ride — and the exchange is a serial or
    //     a US state. Offered after EU VHF and before the everyday message so
    //     a six-digit exchange does not read as a report plus a stray token.
    if matches!(toks.len(), 3 | 4) && wsjt77::is_standard_callsign(c1) {
        let rogered = toks.get(2) == Some(&"R");
        let exch = toks.last().copied().unwrap_or("");
        // "RST" tokens look like `5NN`/`59`; the exchange is what follows.
        if let Some(rs) =
            exch.get(..2).and_then(|r| r.parse::<u8>().ok()).filter(|r| (52..=59).contains(r))
            && let Some(code) = exch.get(2..)
            && let Some(m) = roundup::pack(c1, c2, rogered, rs, code)
        {
            let r = if rogered { "R " } else { "" };
            return Some((m, format!("{c1} {c2} {r}{exch}")));
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

    // 3a. The 11 m identity opener — WSJT-CB's tx1, "<DX> MYCALL": the DX
    //     named by an explicit hash, *our* call spelled out in the clear
    //     (issue #396). Neither call fits a 28-bit field, so Type 4 carries
    //     them — the DX in its 12-bit hash field, ours in the 58-bit base-38
    //     field — which is exactly the message WSJT-CB transmits, so the DX
    //     learns who is answering even without holding our call yet. `iflip`
    //     is forced to 0 (hash first): the packer would otherwise flip it for
    //     a non-standard "standard" call, and the field order must match WSJT-CB.
    let dx = eu_vhf::bare(c1);
    if toks.len() == 2
        && c1.starts_with('<')
        && c1.ends_with('>')
        && dx != c2
        && cb_ok(dx, wide)
        && cb_ok(c2, wide)
    {
        if let Some(mut m) = wsjt77::pack77_type4(c2, dx, "", false) {
            m[70] = 0; // iflip=0: the DX's hash reads first, "<DX> MYCALL"
            return Some((m, format!("<{dx}> {c2}")));
        }
    }

    // 3b. One compound / non-standard callsign addressed with a grid or a
    //     signal report — neither of which the layout below has anywhere to
    //     put. The standard layout does, so long as that callsign travels as
    //     its hash instead of spelled out (issue #348).
    if let Some((m, sent)) = pack77_hashed(c1, c2, payload, wide) {
        return Some((m, sent));
    }

    // 3c. WSJT-CB's one-call messages (issue #396): a bare station call
    //     ("26AT715") or that call with a report or sign-off token
    //     ("26AT715 -07", "26AT715 R-07", "26AT715 RR73", "26AT715 73").
    //     The rungs below would read the status word as a second callsign —
    //     "73" and "RR73" even pass WSJT-X's own shape rules — and hash it,
    //     which is not the message anyone on the band sent. Free text carries
    //     the whole line, and that is how WSJT-CB reads it back. A genuine
    //     *pair* in the same two tokens ("26AT715 25TT304") is not this shape
    //     and stays on the two-hash ladder above.
    if !c1.is_empty()
        && cb_ok(c1, wide)
        && matches!(toks.len(), 1 | 2)
        && (toks.len() == 1 || is_cb_payload_tok(c2))
    {
        let free: String = text.chars().take(13).collect();
        if let Some(m) = wsjt77::pack77_free_text(free.trim_end()) {
            return Some((m, free.trim_end().to_string()));
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
        if is_packable_call(nonstd, wide) && wsjt77::is_standard_callsign(std_call) {
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

/// The everyday layout with one (or both) callsign(s) carried as their 22-bit
/// hash rather than spelled out — the only way FT8 can send a report to a
/// station whose callsign the 28-bit field cannot hold (issue #348), and the
/// way WSJT-CB's *addressed pairs* travel on 11 m: a station already worked
/// this session, or whose bare-call identity was heard clearly, is named by
/// hash where the two-hash one-call sequence above does not fit (issue #396).
///
/// **Single hash (the historical path):** one callsign travels as
/// `NTOKENS + hash22`, the other spelled out. The non-standard layout
/// (`i3 = 4`, [`wsjt77::pack77_type4`]) spells such a callsign out in full, but
/// it spends 58 of its 77 bits doing so and has room left for nothing but a
/// bare `RRR` / `RR73` / `73`. So a reply to `R7KJG/QRP` that should have
/// carried `R-12` went out as an acknowledgement with no report in it at all,
/// and the contact stalled there. The way through is the one WSJT-X takes:
/// send `<R7KJG/QRP> F4CYH R-12` as an ordinary `i3 = 1` message whose first
/// callsign field holds `NTOKENS + hash22`. Both ends resolve it — the far end
/// because the hash is of its own callsign, and this one because the contact
/// opened with a message that spelled the callsign out.
///
/// **Both hashed (CB pair, 11 m):** neither callsign fits a 28-bit field
/// at all — both are nonstandard — so the only `i3 = 1` form the layout can
/// carry is a grid-less pair of hashes. The status payloads (`""` / `RRR` /
/// `RR73` / `73`) ride too: on 11 m there is no third amateur station
/// listening for a spelled form to seed the hash table.
///
/// `None` for anything that layout should not carry, leaving the ladder in
/// [`pack_message`] to go on to the next rung:
///
/// * Both callsigns standard (handled earlier by rung 3).
/// * A single hash with an empty payload or a bare `RRR` / `RR73` / `73`:
///   those fit the non-standard layout whole, and spelling the callsign out is
///   worth more than the hash saves — it is what lets a third station resolve
///   the hashes in everything around it.
fn pack77_hashed(c1: &str, c2: &str, payload: &str, wide: bool) -> Option<([u8; 77], String)> {
    // A callsign the operator wrote in brackets is one they are asking to have
    // hashed; one that will not fit the 28-bit field has to be, brackets or no.
    let (b1, b2) = (eu_vhf::bare(c1), eu_vhf::bare(c2));
    let wants_hash = |tok: &str, bare: &str| {
        !bare.is_empty()
            && bare != "CQ"
            && (tok.starts_with('<') || !wsjt77::is_standard_callsign(bare))
    };
    let (h1, h2) = (wants_hash(c1, b1), wants_hash(c2, b2));
    if !h1 && !h2 {
        // Neither hashed: the everyday rung already handled the spelling.
        return None;
    }
    // Both hashed — a bracketed pair, or a callsign pair no 28-bit field can
    // hold (both ends of a CB contact, issue #396). There is no standard
    // spelling of either, so a grid-less pair of hashes is the only form the
    // everyday layout can carry them in. On 11 m the empty / RRR / RR73 / 73
    // payloads ride too: those are all a pair of hashes leaves room for anyway,
    // and no third amateur station is listening to prefer a spelled form.
    if h1 && h2 {
        if !is_packable_call(b1, wide) || !is_packable_call(b2, wide) {
            return None;
        }
        const STAND_IN: &str = "K1ABC";
        let mut msg = wsjt77::pack77(STAND_IN, STAND_IN, payload)?;
        let overlay = |msg: &mut [u8; 77], start: usize, call: &str| {
            // WSJT-X hashes the callsign whole (`save_hash_call`), so the hash
            // is taken here rather than through mfsk-core's table, which strips
            // a `/P` or `/R` suffix first — the same divergence [`eu_vhf`]
            // keeps its own table for.
            let n28 = NTOKENS + mfsk_core::msg::hash_table::ihashcall(call, 22);
            for i in 0..28 {
                msg[start + i] = ((n28 >> (27 - i)) & 1) as u8;
            }
        };
        overlay(&mut msg, 0, b1);
        overlay(&mut msg, 29, b2);
        return Some((msg, join3(&format!("<{b1}>"), &format!("<{b2}>"), payload)));
    }
    // One hashed, one spelled out — the historical single-hash path.
    if payload.is_empty() || matches!(payload, "RRR" | "RR73" | "73") {
        // Bare RRR/RR73/73 and an empty payload fit the non-standard layout
        // whole, and spelling the callsign out is worth more than the hash
        // saves — it is what lets a third station resolve the hashes in
        // everything around it.
        return None;
    }
    let (hashed, spelled) = if h1 { (b1, b2) } else { (b2, b1) };
    if !is_packable_call(hashed, wide) || !wsjt77::is_standard_callsign(spelled) {
        return None;
    }
    // Packed with a stand-in where the hash goes and then overwritten, rather
    // than assembled here: the report field alone has five shapes, and the
    // difference between a grid, a report and an R-report is mfsk-core's to
    // know. Only the one field it cannot write is written by hand.
    const STAND_IN: &str = "K1ABC";
    let mut msg =
        wsjt77::pack77(if h1 { STAND_IN } else { b1 }, if h2 { STAND_IN } else { b2 }, payload)?;
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

/// The ARRL RTTY Roundup layout, `i3 = 3` (the serial-contest shape WSJT-X
/// uses): `[tu1][h28 to][h28 from][r1 ack][s3 report][e13 exchange]`, then `i3`.
/// The 13-bit exchange is **the state index** (`nexch = 8000 + index`) for a
/// North-American station, or a **serial** (`1..=7999`) for everyone else —
/// which is exactly the generic serial exchange a CQ WPX / generic contest
/// wants, so this one layout drives both.
///
/// mfsk-core unpacks this type but packs no such message, so the pack is
/// written here against its `unpack77` `i3 = 3` arm — the bit order has to
/// match it exactly, and `a_roundup_message_round_trips_through_mfsk_core`
/// pins that against mfsk-core's own unpacker rather than against our own.
///
/// Unlike EU VHF this layout spells the callsigns with `pack28`, which the
/// resolver handles for a call it has not heard; only the rarer hashed form
/// (`unpack28_h`) needs the heard table, and the pack never writes that one.
mod roundup {
    use mfsk_core::msg::wsjt77::pack28;

    use super::eu_vhf::bare;

    /// The `i3` of this layout.
    pub const I3_RTTY_ROUNDUP: u8 = 3;

    /// The exchange codes the state index starts at. `8000` cannot be a serial
    /// (a serial is `1..=7999`), so the two ranges cannot collide.
    const STATE_BASE: u32 = 8000;

    /// The 50 US states and DC, in the order the 13-bit index numbers them —
    /// mfsk-core's own `RTTY_STATES`, mirrored here because the pack needs the
    /// index the unpack reads back. Kept in step by the round-trip test.
    pub const RTTY_STATES: &[&str] = &[
        "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA",
        "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
        "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT",
        "VA", "WA", "WV", "WI", "WY", "DC",
    ];

    /// The exchange field for a token: a 1..=3 digit serial, or a US state.
    /// Anything else cannot ride this layout.
    fn exchange_code(tok: &str) -> Option<u32> {
        let t = tok.trim().to_ascii_uppercase();
        if let Ok(n) = t.parse::<u32>() {
            return (1..=7999).contains(&n).then_some(n);
        }
        RTTY_STATES.iter().position(|s| *s == t).map(|i| STATE_BASE + i as u32 + 1)
    }

    /// Pack `<to> <from> [R] <RS exchange>` into 77 bits.
    ///
    /// `rs` is `52..=59` (the three-bit report field); `tu` opens with the
    /// `TU;` sign-off the layout carries. The calls are taken bare — angle
    /// brackets are a hash's rendering, not part of the call.
    pub fn pack(to: &str, from: &str, ack: bool, rs: u8, exchange_tok: &str) -> Option<[u8; 77]> {
        if !(52..=59).contains(&rs) {
            return None;
        }
        let exch = exchange_code(exchange_tok)?;
        let n_to = pack28(bare(to))?;
        let n_from = pack28(bare(from))?;
        let mut msg = [0u8; 77];
        let mut write = |start: usize, len: usize, val: u32| {
            for i in 0..len {
                msg[start + i] = ((val >> (len - 1 - i)) & 1) as u8;
            }
        };
        write(0, 1, 0); // tu = 0: the ordinary exchange, not the ";TU" opener
        write(1, 28, n_to);
        write(29, 28, n_from);
        write(57, 1, u32::from(ack));
        write(58, 3, u32::from(rs - 52));
        write(61, 13, exch);
        write(74, 3, u32::from(I3_RTTY_ROUNDUP));
        Some(msg)
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

/// Whether a message token is a WSJT-CB report or sign-off rather than a
/// callsign: `RRR` / `RR73` / `73`, or a signed report like `-07`, `+05`,
/// `R-07`, `R+05`. The very names are callsign-shaped to WSJT-X's validator —
/// the reason [`pack_message`] has to look at the lones at all.
fn is_cb_payload_tok(t: &str) -> bool {
    matches!(t, "RRR" | "RR73" | "73")
        || (t.starts_with("R-") || t.starts_with("R+") || t.starts_with('-') || t.starts_with('+'))
            && t.len() >= 2
            && t[1..].parse::<i16>().is_ok()
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

    /// The whole point of the fix, end to end: a station that is working an
    /// activation answers every one of these, not just the bare call. Before
    /// it, `pack_message` refused them all — the gate was the union of two
    /// narrow predicates, so a call WSJT-CB decodes had nowhere to go on the
    /// way back out.
    #[test]
    fn a_modified_cb_call_can_be_answered() {
        // The CB-to-CB sequence WSJT-CB runs, against a portable call.
        for (line, expect) in [
            ("CQ 19DC373/P", "CQ 19DC373/P"),
            ("19DC373/P 26AT715", "<19DC373/P> <26AT715>"),
            ("19DC373/P 26AT715 R+12", "<19DC373/P> <26AT715> R+12"),
            ("19DC373/P 26AT715 RR73", "<19DC373/P> <26AT715> RR73"),
            ("19DC373/P 26AT715 73", "<19DC373/P> <26AT715> 73"),
        ] {
            let (bits, sent) = pack_message(line, false).unwrap_or_else(|| {
                panic!("{line} must pack — a station answering it needs a reply")
            });
            assert_eq!(sent, expect);
            assert_ne!(bits, [0u8; 77], "{line} produced an empty transmission");
        }
        // The low-power form, which is the longest identifier the wire carries.
        let (bits, sent) =
            pack_message("19DC373/QRP 26AT715 RR73", false).expect("packs at the length limit");
        assert_eq!(sent, "<19DC373/QRP> <26AT715> RR73");
        assert_ne!(bits, [0u8; 77]);
        // …and one character past it is not a call this wire can carry.
        // One character past the field, the call no longer fits beside a
        // report, and the ladder does what it does for any call too long to
        // pair: the call goes out alone and the exchange is dropped. This is
        // the pre-existing degradation, not something the modifier introduced
        // — the 11-character bound itself is pinned in the grammar's own tests.
        let (_, sent) =
            pack_message("19DC373/QRPP 26AT715 RR73", false).expect("the call still gets out");
        assert_eq!(sent, "19DC373/QRPP", "the exchange is dropped, not silently mangled");
    }

    /// The message packer has to recognise the exchange for what it is. Before
    /// it did, the whole thing went out as thirteen characters of free text —
    /// which is a valid transmission and completely useless.
    #[test]
    fn the_packer_picks_the_contest_layout() {
        let (bits, sent) = pack_message("<PA9XYZ> <G4ABC/P> R 570007 JO22DB", false)
            .expect("packs as a contest message");
        assert_eq!(msg_kind(&bits), MsgKind::EuVhf);
        assert_eq!(sent, "<PA9XYZ> <G4ABC/P> R 570007 JO22DB");
        let (bits, _) = pack_message("<PA9XYZ> <G4ABC/P> 590003 IO91NP", false).expect("packs");
        assert_eq!(msg_kind(&bits), MsgKind::EuVhf);
    }

    /// And a decode of one is addressed, with the six-character locator read
    /// out of it — the whole reason the layout exists.
    #[test]
    fn a_contest_decode_is_addressed_and_carries_its_locator() {
        let mut eu = eu_vhf::Hashes::default();
        eu.insert("PA9XYZ");
        eu.insert("G4ABC/P");
        let (bits, _) = pack_message("<PA9XYZ> <G4ABC/P> R 570007 JO22DB", false).expect("packs");
        let d = build_decode(&bits, -5.0, 0.2, 1200.0, 0, &CallsignHashTable::new(), &eu)
            .expect("decodes");
        assert_eq!(d.to.as_deref(), Some("PA9XYZ"));
        assert_eq!(d.from.as_deref(), Some("G4ABC/P"));
        assert_eq!(d.grid.as_deref(), Some("JO22DB"));
        assert!(!d.is_cq && !d.free_text);
    }

    /// The Roundup pack, read back by **mfsk-core's own unpacker** — the one
    /// check that proves the bit order, since our packer and our unpacker
    /// agreeing with each other would prove nothing.
    #[test]
    fn a_roundup_message_round_trips_through_mfsk_core() {
        let bits = roundup::pack("K1ABC", "W9XYZ", true, 59, "1234").expect("packs");
        assert_eq!(msg_kind(&bits), MsgKind::RttyRu);
        let text = wsjt77::unpack77(&bits).expect("mfsk-core reads it");
        assert!(text.contains("K1ABC") && text.contains("W9XYZ"), "{text}");
        assert!(text.contains("1234"), "the serial survives: {text}");
    }

    /// A US state rides the same 13-bit field as a state index, and comes back
    /// as the state.
    #[test]
    fn a_roundup_state_exchange_round_trips() {
        let bits = roundup::pack("W1AW", "K5ABC", false, 59, "TX").expect("packs");
        let text = wsjt77::unpack77(&bits).expect("mfsk-core reads it");
        assert!(text.contains("TX"), "{text}");
    }

    /// A serial past the field, and an exchange that is neither a serial nor a
    /// state, are refused rather than packed into something else.
    #[test]
    fn a_roundup_exchange_out_of_range_is_refused() {
        assert!(roundup::pack("K1ABC", "W9XYZ", false, 59, "8000").is_none());
        assert!(roundup::pack("K1ABC", "W9XYZ", false, 59, "ZZ").is_none());
        assert!(roundup::pack("K1ABC", "W9XYZ", false, 59, "0").is_none());
    }

    /// The whole path a serial contest actually takes: encode a
    /// `CALL CALL 591234` line, decode the slot, and read it back as the same
    /// text through `build_decode` — the Roundup layout reached from the
    /// ordinary encode ladder.
    #[test]
    fn a_serial_contest_line_encodes_and_decodes_as_roundup() {
        let mut modem = Ft8Modem::new(Mode::Ft8);
        let (burst, sent) =
            modem.encode_burst_12k("K1ABC W9XYZ 591234", 1500.0, 0.5).expect("encode");
        assert!(sent.contains("591234"), "the exchange went out: {sent}");
        let mut slot = vec![0.0f32; (0.5 * 12_000.0) as usize];
        slot.extend_from_slice(&burst);
        slot.resize((15.0 * 12_000.0) as usize, 0.0);
        let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        let decodes = modem.decode_slot(&i16buf, 0, &ApHints::default(), 1500.0);
        assert!(
            decodes.iter().any(|d| d.message.contains("1234") && d.message.contains("K1ABC")),
            "a serial exchange decodes: {:?}",
            decodes.iter().map(|d| &d.message).collect::<Vec<_>>(),
        );
        // The callsigns and the serial both parse out of the decoded row.
        let d = decodes.iter().find(|d| d.message.contains("1234")).expect("the row");
        assert_eq!(d.to.as_deref(), Some("K1ABC"));
        assert_eq!(d.from.as_deref(), Some("W9XYZ"));
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

    /// A synthesized JT65 and JT9 message decodes back to the same
    /// `<to> <from> <grid>` — the round trip the JT controller makes. Both
    /// modes carry no CRC, so the scan may return extra rows on the noise
    /// beside the signal; the real message has to be among them, and the
    /// strongest one has to be it.
    #[test]
    fn jt_messages_round_trip() {
        for mode in [Mode::Jt65, Mode::Jt9] {
            let synth = match mode {
                Mode::Jt65 => mfsk_core::jt65::tx::synthesize_standard(
                    "CQ", "K1ABC", "FN42", 12_000, 1000.0, 0.3,
                ),
                _ => mfsk_core::jt9::tx::synthesize_standard(
                    "CQ", "K1ABC", "FN42", 12_000, 1000.0, 0.3,
                ),
            }
            .expect("synthesize");
            // The scan wants a whole 60-second slot; pad the burst into one.
            let mut slot = vec![0.0f32; 12_000]; // the one-second TX offset
            slot.extend_from_slice(&synth);
            slot.resize(60 * 12_000, 0.0);
            let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();

            let decodes = decode_jt_slot(&i16buf, mode, 0);
            let best = decodes.first().unwrap_or_else(|| panic!("{mode:?}: nothing decoded"));
            assert_eq!(best.from.as_deref(), Some("K1ABC"), "{mode:?}: {decodes:?}");
            assert!(best.is_cq, "{mode:?}: {decodes:?}");
            assert_eq!(best.grid.as_deref(), Some("FN42"), "{mode:?}: {decodes:?}");
        }
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
        }
    }

    /// A synthesized Q65 message decodes back at every sub-mode — the round trip
    /// the controller makes. Q65 shares FT8's 77-bit message, so the message is
    /// expected whole; the sub-mode decides both the slot the audio is padded
    /// into and the protocol type the decode is run as.
    ///
    /// `#[ignore]`d because a Q65 scan is tens of seconds of work even at the
    /// short sub-modes, and a 300 s slot is a minute of decode on its own; run
    /// it with `cargo test -p sdroxide-digi --release -- --ignored q65`. The
    /// cheapest sub-mode has its own unignored smoke test below.
    #[test]
    #[ignore = "Q65 scans are slow; run with --release -- --ignored"]
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
    /// the one-second TX offset, and assert it decodes back.
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
            let iping = (i / FS as usize).max(1).min(14);
            let t = (i as f32 / FS - iping as f32) / 0.2;
            let env = if (0.0..=10.0).contains(&t) { 2.718 * t * (-t).exp() } else { 0.0 };
            slot.push((s as f32 * env) as i16);
        }
        let decodes = decode_msk144_slot(&slot, 0);
        assert!(
            decodes.iter().any(|d| d.message == "K1ABC W9XYZ EN37"),
            "MSK144 did not round-trip: {decodes:?}"
        );
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
    /// Issue #223 was mfsk-core's FT8 decoder finishing every candidate with an
    /// `unpack77` that had no `i3 = 5` arm, so a contest exchange was thrown
    /// away inside the decoder. mfsk-core 0.11's field-based plausibility
    /// filter (jl1nie/mfsk-core#386) replaced the text-based one that did the
    /// dropping, so the layout decodes natively now and the `ft8_eu` rescue
    /// pass is gone; the test lives on as the regression pin.
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

        // The exchange arrives, whole, straight from the FT8 decoder. mfsk-core
        // 0.11's field-based plausibility filter no longer drops the `i3 = 5`
        // layout (jl1nie/mfsk-core#386 replaced the text-based filter that did);
        // the dedicated `ft8_eu` rescue pass this test used to rely on is gone.
        let off = rx.decode_slot(&buf, 0, &ApHints::default(), 1500.0);
        let d = off
            .iter()
            .find(|d| d.message.contains("590003"))
            .unwrap_or_else(|| panic!("the exchange did not decode; got {off:?}"));
        assert_eq!(d.message, "<PA9XYZ> <G4ABC/P> R 590003 IO91NP");
        assert_eq!(d.to.as_deref(), Some("PA9XYZ"), "and it is addressed to the right station");
        assert_eq!(d.from.as_deref(), Some("G4ABC/P"));
        assert!((d.audio_hz - 1500.0).abs() < 5.0, "at the offset it was sent on");
    }

    /// The EU VHF contest flag must not change what an ordinary slot returns.
    /// An ordinary message comes back exactly once, whether or not the contest
    /// is selected.
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
        let (bits, sent) = pack_message("TNX QSO 73 GL", false).expect("packs");
        assert_eq!(sent, "TNX QSO 73 GL");
        assert_eq!(msg_kind(&bits), MsgKind::FreeText);
        assert_eq!(wsjt77::unpack77(&bits).as_deref(), Some("TNX QSO 73 GL"));

        // Over-long text is cut to the 13 characters FT8 carries, and the
        // caller is told what actually went out.
        let (_, sent) = pack_message("THANKS FOR THE CONTACT", false).expect("packs");
        assert_eq!(sent, "THANKS FOR TH");
    }

    #[test]
    fn a_cb_one_call_messages_go_out_as_free_text() {
        // WSJT-CB's report and sign-off messages name only the DX: a single
        // call plus a payload, short enough for the thirteen characters of
        // free text — and that is how WSJT-CB sends and reads them on the air
        // (issue #396). The *opener* is a two-call pair and travels hashed
        // instead (see a_cb_pair_packs_as_two_hashes_including_the_status_payloads);
        // a lone bare call is still a legal free-text identity.
        for text in ["25TT304", "26AT715 -10", "26AT715 R-10", "26AT715 RR73", "26AT715 73"] {
            let (bits, sent) = pack_message(text, false).expect("packs");
            assert_eq!(sent, text);
            assert_eq!(msg_kind(&bits), MsgKind::FreeText, "{text}");
            assert_eq!(wsjt77::unpack77(&bits).as_deref(), Some(text));
        }
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
        let (bits, sent) = pack_message("F4CYH R7KJG/QRP -12", false).expect("packs");
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
        let (_, sent) = pack_message("R7KJG/QRP F4CYH JN18", false).expect("packs");
        assert_eq!(sent, "<R7KJG/QRP> F4CYH JN18");
    }

    /// The bare acknowledgements stay on the non-standard layout, which
    /// carries them whole *and* spells the callsign out — worth more than the
    /// hash saves, and what everything else on the band resolves from.
    #[test]
    fn an_acknowledgement_to_a_compound_call_still_spells_it_out() {
        for rpt in ["RRR", "RR73", "73"] {
            let (bits, sent) =
                pack_message(&format!("R7KJG/QRP F4CYH {rpt}"), false).expect("packs");
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

    /// 11 m opens and reports with *both* calls hashed (WSJT-CB, issue #396):
    /// neither ends up in a 28-bit field, so a grid-less pair of hashes is the
    /// only everyday form the layout carries — and the empty report (the
    /// grid-less opener) and the bare sign-offs ride with it.
    #[test]
    fn a_cb_pair_packs_as_two_hashes_including_the_status_payloads() {
        use mfsk_core::msg::hash_table::CallsignHashTable;
        let mut ht = CallsignHashTable::new();
        for call in ["26AT715", "25TT304"] {
            ht.insert(call);
        }
        for (text, resolved) in [
            ("26AT715 25TT304", "<26AT715> <25TT304>"),
            ("26AT715 25TT304 -07", "<26AT715> <25TT304> -07"),
            ("26AT715 25TT304 R-03", "<26AT715> <25TT304> R-03"),
            ("26AT715 25TT304 RRR", "<26AT715> <25TT304> RRR"),
            ("26AT715 25TT304 RR73", "<26AT715> <25TT304> RR73"),
            ("26AT715 25TT304 73", "<26AT715> <25TT304> 73"),
            ("26AT715 25TT304 JO31", "<26AT715> <25TT304> JO31"),
        ] {
            let (bits, sent) = pack_message(text, false).expect("packs");
            // Both hashed calls stay bracketed in the display text.
            let toks: Vec<&str> = text.split_whitespace().collect();
            let shown = ["<26AT715>", "<25TT304>"]
                .into_iter()
                .chain(toks.iter().skip(2).copied())
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(sent, shown);
            // The everyday layout, and a round trip resolves the hashes —
            // keeping the brackets, as `unpack77_with_hash` does for any call
            // that arrived hashed. The address parser strips them again.
            assert_eq!(msg_kind(&bits), MsgKind::Standard);
            let out = wsjt77::unpack77_with_hash(&bits, &ht).expect("unpacks");
            assert_eq!(out, resolved);
        }
    }

    /// The 11 m identity opener — WSJT-CB's tx1 — is the one message where
    /// our call leaves the *spelled* side of the pair: `<26AT715> 25TT304`.
    /// Type 4 hashes the DX into its 12-bit field and spells ours in the
    /// 58-bit base-38 field, and the hash reads first (`iflip=0`), exactly the
    /// 77 bits WSJT-CB itself transmits (verified against its `pack77`). The
    /// un-bracketed, both-hashed pair is the addressed form, and lives above.
    #[test]
    fn a_cb_opener_is_hash_then_spelled_out() {
        use mfsk_core::msg::hash_table::CallsignHashTable;
        let (bits, sent) = pack_message("<26AT715> 25TT304", false).expect("packs");
        assert_eq!(sent, "<26AT715> 25TT304");
        assert_eq!(msg_kind(&bits), MsgKind::NonStandard);
        // The exact WSJT-CB reference encoding for these two calls.
        assert_eq!(
            bits.iter().map(|b| if *b == 1 { '1' } else { '0' }).collect::<String>(),
            "01100101001100000000000000000000000010001110101000111110100001111010110000100"
        );
        // Unresolved, the DX reads as a hash; the 12-bit hash is what a WSJT-CB
        // receiver matches against its *own* call to resolve, while mfsk-core's
        // table-based unpack keeps it bracketed. Either way our call arrives
        // spelled out — that is the whole point of the form.
        assert_eq!(wsjt77::unpack77(&bits).as_deref(), Some("<...> 25TT304"));
        let mut ht = CallsignHashTable::new();
        for call in ["26AT715", "25TT304"] {
            ht.insert(call);
        }
        assert_eq!(wsjt77::unpack77_with_hash(&bits, &ht).as_deref(), Some("<26AT715> 25TT304"));
    }

    /// A CB call in an otherwise standard pair hashes alone, exactly like any
    /// other nonstandard callsign — `<26AT715> 25TT304 R-07`.
    #[test]
    fn a_cb_pair_against_a_standard_call_hashes_just_the_cb_side() {
        let (bits, sent) = pack_message("26AT715 AB1CD R-07", false).expect("packs");
        assert_eq!(sent, "<26AT715> AB1CD R-07");
        assert_eq!(msg_kind(&bits), MsgKind::Standard);
    }

    /// The experimental grammar's whole point: a three-letter CB pair packs as
    /// a two-hash contact only with the switch on. Strict, neither call is a CB
    /// one, so the pair has nowhere to go and degrades to 13 characters of free
    /// text — which no WSJT-CB station will read as two calls.
    #[test]
    fn the_wide_grammar_packs_a_three_letter_cb_pair() {
        let (strict_bits, strict_sent) =
            pack_message("26ABC715 25TT304", false).expect("packs as free text");
        assert_ne!(msg_kind(&strict_bits), MsgKind::Standard);
        assert_ne!(strict_sent, "<26ABC715> <25TT304>");

        let (bits, sent) = pack_message("26ABC715 25TT304", true).expect("packs as a CB pair");
        assert_eq!(sent, "<26ABC715> <25TT304>");
        assert_eq!(msg_kind(&bits), MsgKind::Standard);
    }

    /// A CQ from an 11 m call is spelled out on the non-standard layout, as a
    /// CQ to any callsign that cannot fit a 28-bit field is.
    #[test]
    fn a_cb_call_can_call_cq() {
        let (sent, decodes) = round_trip(Mode::Ft8, "CQ 26AT715");
        assert_eq!(sent, "CQ 26AT715");
        let d = decodes.iter().find(|d| d.message.contains("26AT715")).expect("decoded");
        assert!(d.is_cq);
        assert_eq!(d.from.as_deref(), Some("26AT715"));
    }

    /// A CB callsign is refused by upstream's stock validators, which is
    /// what this crate's mfsk-core fork pin used to patch. Since 0.11 the
    /// CB grammar lives here and widens the FT8/FT4 decode gate per call
    /// through the field-based hook (`DecodeRequest::also_accept`,
    /// mfsk-core#386): a message passes when every callsign field is
    /// acceptable to the codec's own per-field verdict or is CB-shaped.
    /// A grid or a report never reaches the grammar, because the hook
    /// yields callsign fields only.
    #[test]
    fn cb_calls_pass_the_decode_gate() {
        // 26AT715 has no ITU-amateur prefix number, so the stock validators
        // have no opinion that the fork used to supply.
        assert!(!wsjt77::is_valid_callsign("26AT715"));
        assert!(!wsjt77::is_plausible_callsign("26AT715"));
        // The union: 11 m identifiers pass, and so do everything the codec's
        // own per-field verdict already passes — the CQ/DE/QRZ and `<...>`
        // tags that ride in the callsign fields, and ordinary amateur calls.
        assert!(is_cb_compatible_call("26AT715", false));
        assert!(is_cb_compatible_call("1AT106", false));
        assert!(is_cb_compatible_call("999ZZ/ZZ", false));
        assert!(is_cb_compatible_call("CQ", false));
        assert!(is_cb_compatible_call("CQ DX", false));
        assert!(is_cb_compatible_call("<25TT304>", false));
        assert!(is_cb_compatible_call("PA3XYZ", false));
        // A callsign the ITU allowlist rejects and CB grammar refuses — the
        // crate's own documented CRC survivor "CQ G47OXF RD84" — still can't
        // get through the union.
        assert!(!is_cb_compatible_call("G47OXF", false));
        assert!(!sdroxide_types::is_cb_callsign("G47OXF"));
        // The wider grammar is a separate gate: the three-letter shape passes
        // only with it on, and nothing the strict gate refused for another
        // reason (a bad prefix, letters only) sneaks through.
        assert!(!is_cb_compatible_call("26ABC715", false));
        assert!(is_cb_compatible_call("26ABC715", true));
        assert!(!is_cb_compatible_call("G47OXF", true));
        // A modifier on a legal 11 m call is WSJT-CB's own shape, so it needs
        // neither half of the union widened — the base is already CB-legal.
        // This is the station the fork could hear and could not answer.
        assert!(is_cb_compatible_call("19DC373/P", false));
        assert!(is_cb_compatible_call("19DC373/QRP", false));
        assert!(is_cb_compatible_call("26AT715/MM", false));
        // …and the encode gate agrees, so REPLY can now pack a reply to them.
        assert!(is_packable_call("19DC373/P", false));
        assert!(is_packable_call("19DC373/QRP", false));
        assert!(!is_packable_call("19DC373/QRPP", false)); // past the 11-char field
        // A modifier cannot rescue a base that is not an 11 m call.
        assert!(!is_cb_compatible_call("ABC373/P", false));
        assert!(!is_cb_compatible_call("G47OXF/P", false));
        assert!(wsjt77::is_plausible_call("PA3XYZ"));
        assert!(!wsjt77::is_plausible_call("G47OXF"));
        assert!(!wsjt77::is_plausible_call("26AT715"));

        use mfsk_core::msg::wsjt77::Wsjt77Fields;
        // The whole gate, base verdict or our widening — the same
        // `base || predicate` the hook and the codec combine into.
        let accept = |m: &Wsjt77Fields| {
            m.is_plausible() || m.callsigns().all(|c| is_cb_compatible_call(c, false))
        };
        // A CQ to a CB call, and a CB-only pair, both hashed when sent.
        assert!(accept(&Wsjt77Fields::Standard {
            call1: "CQ".into(),
            call2: "26AT715".into(),
            exchange: String::new(),
        }));
        assert!(accept(&Wsjt77Fields::Standard {
            call1: "26AT715".into(),
            call2: "25TT304".into(),
            exchange: "R-07".into(),
        }));
        assert!(accept(&Wsjt77Fields::Nonstandard {
            call1: "26AT715".into(),
            call2: Some("25TT304".into()),
            exchange: "RR73",
        }));
        assert!(accept(&Wsjt77Fields::Nonstandard {
            call1: "CQ".into(),
            call2: Some("<25TT304>".into()),
            exchange: "RR73",
        }));
        // A stock call in the pair is the codec's own verdict, not ours.
        assert!(accept(&Wsjt77Fields::Standard {
            call1: "PA3XYZ".into(),
            call2: "26AT715".into(),
            exchange: "RR73".into(),
        }));
        // The phantom the crate's allowlist exists to catch still lands.
        assert!(!accept(&Wsjt77Fields::Standard {
            call1: "CQ".into(),
            call2: "G47OXF".into(),
            exchange: "RD84".into(),
        }));
    }

    /// A decoded message with the hash brackets taken off.
    ///
    /// WSJT-X marks a **hashed** field `<CALL>` when it resolves and `<...>`
    /// when it does not; a call spelled out in the clear carries none. An 11 m
    /// pair is packed with one end hashed, so a correct round trip comes back
    /// bracketed — comparing the brackets would be testing WSJT-X's convention
    /// rather than the chain. What has to survive is the *content*.
    fn debracketed(s: &str) -> String {
        s.replace(['<', '>'], "")
    }

    /// The forms an 11 m station actually sends, and the wider grammar's shape
    /// last. Shared by the tests below so they cannot drift apart.
    const CB_FORMS: &[(&str, bool)] = &[
        ("CQ 19DC373", false),
        ("CQ 26AT715", false),
        ("CQ 26AT715 JO22", false),
        ("CQ 19DC373/P", false),
        ("19DC373 26AT715 R-12", false),
        ("26AT715 19DC373 RR73", false),
        ("19DCG3733 26AT715 R-12", true),
    ];

    /// **A CB CQ has to survive the whole chain, not just the gate.**
    ///
    /// Everything above this tests the *predicate* — `is_cb_compatible_call` on
    /// fields — and the packing is tested from the QSO side. Nothing decoded a
    /// `CQ <11 m call>` from audio until this test, and that is the case the
    /// bench reported on 2026-10-18: on 27.265 (CB ch 26, FT8) the station heard
    /// CB stations exchanging reports and RR73 all evening and **never once a CQ
    /// from one**, while CQs from amateur calls came through. The predicate
    /// accepting `CQ 26AT715` says nothing about whether the chain can *carry*
    /// it — a CQ to a non-standard call is a different bit layout from the
    /// report forms, and this is where that difference shows.
    #[test]
    fn a_cb_cq_survives_the_whole_chain() {
        for &(text, wide) in CB_FORMS {
            let mut rx = Ft8Modem::new(Mode::Ft8);
            rx.set_cb_wide(wide);
            // A hash is only resolvable once the call has been spelled out, so
            // seed what a station that had already heard these would hold.
            rx.seed_hashes(&["19DC373".into(), "26AT715".into(), "19DC373/P".into()]);
            rx.seed_hashes(&["19DCG3733".into()]);
            let (burst, sent) = rx
                .encode_burst_12k(text, 1500.0, 0.5)
                .unwrap_or_else(|| panic!("{text}: cannot even be packed"));
            let mut slot = vec![0.0f32; 6_000];
            slot.extend_from_slice(&burst);
            slot.resize((15.0 * 12_000.0) as usize, 0.0);
            let buf: Vec<i16> = slot.iter().map(|&s| (s * 12_000.0) as i16).collect();
            let got: Vec<String> = rx
                .decode_slot(&buf, 0, &ApHints::default(), 1500.0)
                .iter()
                .map(|d| d.message.clone())
                .collect();
            assert!(
                got.iter().any(|m| debracketed(m) == debracketed(&sent)),
                "{text}: sent as {sent:?}, came back as {got:?}"
            );
        }
    }

    /// **The same forms through the path the app actually calls.**
    ///
    /// `decode_slot_streaming` is what the worker runs (`controller.rs`), and it
    /// builds its own `DecodeRequest` rather than going through
    /// `decode_slot`/`decode_slot_staged`. The CB grammar is a fork-only hook on
    /// that request (`DecodeRequest::also_accept`) — upstream has no
    /// citizens'-band calls — so a request built without it falls back to the
    /// stock per-field gate, which is the gate this fork exists to widen.
    #[test]
    fn a_cb_cq_survives_the_streaming_path() {
        let mut lost: Vec<String> = Vec::new();
        for &(text, wide) in CB_FORMS {
            let mut rx = Ft8Modem::new(Mode::Ft8);
            rx.set_cb_wide(wide);
            rx.seed_hashes(&["19DC373".into(), "26AT715".into()]);
            rx.seed_hashes(&["19DCG3733".into()]);
            let (burst, sent) = rx
                .encode_burst_12k(text, 1500.0, 0.5)
                .unwrap_or_else(|| panic!("{text}: cannot even be packed"));
            let mut slot = vec![0.0f32; 6_000];
            slot.extend_from_slice(&burst);
            slot.resize(180_000, 0.0);
            let buf: Vec<i16> = slot.iter().map(|&s| (s * 12_000.0) as i16).collect();
            let got = std::sync::Mutex::new(Vec::<String>::new());
            rx.decode_slot_streaming(&buf, 0, &ApHints::default(), 1500.0, &|| false, &|batch| {
                got.lock().unwrap().extend(batch.into_iter().map(|d| d.message));
            });
            let got = got.into_inner().unwrap();
            if !got.iter().any(|m| debracketed(m) == debracketed(&sent)) {
                lost.push(format!("{text} (sent as {sent:?}) → {got:?}"));
            }
        }
        assert!(
            lost.is_empty(),
            "the app's own decode path lost {} of {} forms:\n  {}",
            lost.len(),
            CB_FORMS.len(),
            lost.join("\n  ")
        );
    }

    // ── the sensitivity sweep ───────────────────────────────────────────────
    //
    // Not a pass/fail gate but a measurement: the noise level at which a slot
    // stops decoding, reported as SNR in the 2500 Hz reference bandwidth FT8's
    // own reports are quoted in. It exercises *our* receive chain end to end —
    // the 12 kHz path, the i16 scaling, the `AUDIO_MIN/MAX_HZ` window and the
    // candidate budget — which is the part we own; the decoder underneath is
    // mfsk-core, the same engine WSJT-X and WSJT-CB run, so a difference
    // against them can only come from the plumbing this measures.
    //
    // `#[ignore]`d because it is slow (a binary search of ~7 decode attempts
    // per mode) and because "is 2 dB worse than yesterday" is a judgement, not
    // an assertion. Run it with:
    //
    //     cargo test -p sdroxide-digi --release -- --ignored --nocapture sensitivity

    /// A deterministic Gaussian-ish noise source, so two runs report the same
    /// number. A 12-sample sum of a uniform xorshift is close enough to
    /// Gaussian for a bit-error measurement and needs no `rand` dependency.
    struct Noise(u32);

    impl Noise {
        fn next(&mut self) -> f32 {
            let mut sum = 0.0;
            for _ in 0..12 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 17;
                self.0 ^= self.0 << 5;
                sum += (self.0 as i32 as f32) / (i32::MAX as f32);
            }
            // Sum of 12 unit uniforms has variance 1; scale to unit sigma.
            sum / 12f32.sqrt()
        }
    }

    /// One encoded slot at `level` of added noise, as the i16 buffer the engine
    /// hands the decoder. Identical audio for every attempt at one seed, so the
    /// search measures the decoder and not the dice.
    fn noisy_slot(mode: Mode, text: &str, level: f32, seed: u32) -> Vec<i16> {
        let modem = Ft8Modem::new(mode);
        let (burst, _) = modem.encode_burst_12k(text, 1500.0, 0.5).expect("encode");
        let slot_s = if mode == Mode::Ft4 { 7.5 } else { 15.0 };
        let mut slot = vec![0.0f32; 6_000];
        slot.extend_from_slice(&burst);
        slot.resize((slot_s * 12_000.0) as usize, 0.0);
        let mut n = Noise(seed);
        slot.iter().map(|&s| ((s + level * n.next()) * 12_000.0) as i16).collect()
    }

    /// The smallest added-noise sigma at which `text` still decodes, found by
    /// bisection on a log scale. The bounds are widened until a clean slot
    /// decodes and a very dirty one does not, so the same routine works for
    /// every layout without a hand-tuned range per case.
    fn threshold_sigma(mode: Mode, text: &str, seed: u32) -> f32 {
        // The hash of every CB call the layout might spell as `<...>`, seeded
        // as a station that had already heard it would have resolved it. The
        // standard calls decode without any.
        let seeds: Vec<String> = ["26AT715", "25TT304"].iter().map(|s| (*s).to_string()).collect();
        let decodes = |level: f32| {
            let buf = noisy_slot(mode, text, level, seed);
            let mut rx = Ft8Modem::new(mode);
            rx.seed_hashes(&seeds);
            rx.decode_slot(&buf, 0, &ApHints::default(), 1500.0).iter().any(|d| d.message == text)
        };
        // A clean slot has to decode, or the measurement is meaningless.
        assert!(decodes(0.0), "{text}: does not decode even with no added noise");
        // Find a ceiling that loses the signal, then bisect between it and
        // silence. The ceiling is capped: past about sigma 30 the added noise
        // saturates the i16 range and no longer models a receiver, so a case
        // that survives that far is one the sweep cannot state a floor for.
        let mut hi = 1.0f32;
        while decodes(hi) {
            hi *= 2.0;
            assert!(hi <= 32.0, "{text}: still decodes at sigma {hi} — the sweep's ceiling");
        }
        let mut lo = 0.0f32;
        // Bisect until the brackets are within 5 % — enough to place the floor
        // to a tenth of a dB, which is finer than the measurement deserves.
        while hi - lo > hi * 0.05 {
            let mid = (lo + hi) / 2.0;
            if mid <= 0.0 {
                break;
            }
            if decodes(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Signal and noise power of a slot, for the SNR figure — measured from the
    /// buffers themselves rather than assumed, so the scaling in
    /// [`noisy_slot`] cannot silently skew the answer.
    fn powers(clean: &[i16], noisy: &[i16]) -> (f32, f32) {
        let to_f = |b: &[i16]| b.iter().map(|&s| f32::from(s)).collect::<Vec<f32>>();
        let (c, n) = (to_f(clean), to_f(noisy));
        let sig = c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32;
        // The noise power is the dirty slot minus the clean one: the added
        // noise alone, with the signal's own contribution removed.
        let total = n.iter().map(|s| s * s).sum::<f32>() / n.len() as f32;
        ((sig), (total - sig).max(1e-9))
    }

    /// A weak FT8 signal buried *inside* a strong neighbour's occupied
    /// bandwidth still decodes — the whole reason FT8 runs signal subtraction.
    ///
    /// This is the one thing the sensitivity sweep cannot show: it puts one
    /// signal in noise, while subtraction exists for two signals on the same
    /// tone offset. Without `.sic_early()` on the FT8 request (mfsk-core's
    /// default is a bare single pass — see its `sic_rounds` doc) the strong
    /// signal is the only one that comes back; with it, the weak one does too.
    /// The levels are the case mfsk-core's own docs name: a weak signal about
    /// 14 dB down, 10–30 Hz off a much stronger one.
    ///
    /// Deterministic, so it is a real gate rather than a measurement.
    #[test]
    fn a_weak_signal_under_a_strong_neighbour_is_recovered() {
        // One slot: the strong CQ centred at 2310, the weak one moved around
        // inside and near it.
        let slot = |weak_hz: f32| {
            let modem = Ft8Modem::new(Mode::Ft8);
            let (strong, _) = modem.encode_burst_12k("CQ AB1CD FN42", 2310.0, 0.5).unwrap();
            let weak_amp = 0.5 * 10f32.powf(-14.0 / 20.0);
            let (weak, _) = modem.encode_burst_12k("CQ W9XYZ EN52", weak_hz, weak_amp).unwrap();
            let mut out = vec![0.0f32; 1_000_000]; // > 15 s so the slot can hold both
            for (i, &x) in strong.iter().enumerate() {
                out[6_000 + i] += x;
            }
            for (i, &x) in weak.iter().enumerate() {
                out[6_000 + i] += x;
            }
            out.truncate(180_000);
            out.iter().map(|&x| (x * 12_000.0) as i16).collect::<Vec<i16>>()
        };
        let decode = |buf: &[i16]| {
            Ft8Modem::new(Mode::Ft8)
                .decode_slot(buf, 0, &ApHints::default(), 2310.0)
                .into_iter()
                .map(|d| d.message)
                .collect::<Vec<_>>()
        };

        // Weak inside the strong signal's bandwidth: both must come back.
        for weak_hz in [2300.0f32, 2320.0] {
            let got = decode(&slot(weak_hz));
            assert!(
                got.iter().any(|m| m == "CQ AB1CD FN42"),
                "the strong signal must decode at {weak_hz} Hz: {got:?}"
            );
            assert!(
                got.iter().any(|m| m == "CQ W9XYZ EN52"),
                "signal subtraction must recover the weak signal under the strong \
                 one at {weak_hz} Hz: {got:?}"
            );
        }
    }

    /// The two-stage split that lets an auto-sequenced reply be decided in
    /// time: the quick single-pass result comes back on its own, the SIC extras
    /// follow as a second batch. The quick batch holds the strong signal; the
    /// extras hold the weak one subtraction recovers.
    #[test]
    fn ft8_decodes_in_two_stages_for_an_on_time_reply() {
        let modem = Ft8Modem::new(Mode::Ft8);
        let (strong, _) = modem.encode_burst_12k("CQ AB1CD FN42", 2310.0, 0.5).unwrap();
        let weak_amp = 0.5f32 * 10f32.powf(-14.0 / 20.0);
        let (weak, _) = modem.encode_burst_12k("CQ W9XYZ EN52", 2320.0, weak_amp).unwrap();
        let mut out = vec![0.0f32; 180_000];
        for (i, &x) in strong.iter().enumerate() {
            out[6_000 + i] += x;
        }
        for (i, &x) in weak.iter().enumerate() {
            out[6_000 + i] += x;
        }
        let buf: Vec<i16> = out.iter().map(|&x| (x * 12_000.0) as i16).collect();

        let (quick, extras) =
            Ft8Modem::new(Mode::Ft8).decode_slot_staged(&buf, 0, &ApHints::default(), 2310.0);
        let q: Vec<&str> = quick.iter().map(|d| d.message.as_str()).collect();
        let e: Vec<&str> = extras.iter().map(|d| d.message.as_str()).collect();
        assert!(q.contains(&"CQ AB1CD FN42"), "the quick pass must hold the strong signal: {q:?}");
        assert!(
            e.contains(&"CQ W9XYZ EN52"),
            "the extras must hold the weak signal subtraction recovers: {e:?}"
        );
    }

    /// **The 11 m case of the test above, through the path the app actually
    /// uses — and the one that was broken on the bench.**
    ///
    /// Both calls are non-standard, so both have to survive the field gate, and
    /// the weak one is only ever found by the *subtraction* pass. That gate is a
    /// fork-only hook on the decode request (`DecodeRequest::also_accept`, the
    /// CB grammar — upstream has no citizens'-band calls), so a rework of the
    /// decode path can drop it for one pass and leave the other working. That
    /// is precisely the shape reported from the bench on 2026-10-18: on 27.265
    /// (CB ch 26, FT8) the station heard CB stations exchanging reports and
    /// RR73 all evening and **not one CQ from one**, while a week earlier (8
    /// October, before the FT8 streaming merge) **37 of its 66 decodes were
    /// CQs**. Loud signals still come through the quick pass; the ones a busy
    /// band masks are found by subtraction, and those are the CQs.
    #[test]
    fn a_weak_cb_cq_survives_the_subtraction_pass() {
        let modem = Ft8Modem::new(Mode::Ft8);
        let (strong, _) = modem.encode_burst_12k("CQ 19DC373", 2310.0, 0.5).unwrap();
        let weak_amp = 0.5f32 * 10f32.powf(-14.0 / 20.0);
        let (weak, _) = modem.encode_burst_12k("CQ 19AT168", 2320.0, weak_amp).unwrap();
        let mut out = vec![0.0f32; 180_000];
        for (i, &x) in strong.iter().enumerate() {
            out[6_000 + i] += x;
        }
        for (i, &x) in weak.iter().enumerate() {
            out[6_000 + i] += x;
        }
        let buf: Vec<i16> = out.iter().map(|&x| (x * 12_000.0) as i16).collect();

        let mut rx = Ft8Modem::new(Mode::Ft8);
        rx.seed_hashes(&["19DC373".into(), "19AT168".into()]);
        let (quick, extras) = rx.decode_slot_staged(&buf, 0, &ApHints::default(), 2310.0);
        let q: Vec<&str> = quick.iter().map(|d| d.message.as_str()).collect();
        let e: Vec<&str> = extras.iter().map(|d| d.message.as_str()).collect();
        assert!(q.contains(&"CQ 19DC373"), "the strong CB CQ must decode in the quick pass: {q:?}");
        assert!(
            e.contains(&"CQ 19AT168"),
            "the subtraction pass must recover the weak CB CQ — quick={q:?} extras={e:?}"
        );
    }

    /// Report the sensitivity floor of each FT8/FT4 message layout we carry.
    ///
    /// The reference bandwidth is 2500 Hz — the convention every FT8 SNR
    /// figure is quoted in (WSJT-X reports `snr = signal − noise in 2500 Hz`),
    /// so the numbers here read on the same scale as a decode's own `-21 dB`.
    /// Signal power is the burst's own mean square over the slot, noise power
    /// is sigma squared. The measurement is of *our chain's* floor, not
    /// mfsk-core's intrinsic one: a difference between the two would be the
    /// resample, the level scaling or the search window, and this is how you
    /// see it.
    #[test]
    #[ignore = "slow sweep; a measurement, not an assertion"]
    fn sensitivity_across_the_message_layouts() {
        const BW_HZ: f32 = 2500.0;
        const RATE: f32 = 12_000.0;
        // A fixed seed per layout: the same audio every run, so the number is
        // comparable between builds.
        // Each entry is the text *as it decodes*, which for a CB call is the
        // hashed spelling the layout actually carries.
        let cases: [(&str, MsgKind, u32); 5] = [
            ("CQ AB1CD FN42", MsgKind::Standard, 0x5eed_0001),
            ("AB1CD W9XYZ -13", MsgKind::Standard, 0x5eed_0002),
            ("CQ 26AT715", MsgKind::NonStandard, 0x5eed_0003),
            ("<26AT715> AB1CD R-07", MsgKind::Standard, 0x5eed_0004),
            ("CQ DX AB1CD FN42", MsgKind::Standard, 0x5eed_0005),
        ];
        println!("\nFT8 receive sensitivity — our chain, 2500 Hz reference bandwidth");
        println!("{:<24} {:>10} {:>12} {:>8}", "message", "sigma", "SNR (dB)", "reports");
        let report = |mode: Mode, text: &str, seed: u32| {
            let sigma = threshold_sigma(mode, text, seed);
            let clean = noisy_slot(mode, text, 0.0, seed);
            let noisy = noisy_slot(mode, text, sigma, seed);
            let (sig, noise) = powers(&clean, &noisy);
            // The decoder's own SNR estimate at the threshold, for comparison
            // with the figure we compute from the signal and noise powers.
            let reported = Ft8Modem::new(mode)
                .decode_slot(&noisy, 0, &ApHints::default(), 1500.0)
                .into_iter()
                .find(|d| d.message == text)
                .map(|d| d.snr_db);
            let snr_2500 = 10.0 * (sig / noise * BW_HZ / RATE).log10();
            println!(
                "{text:<24} {sigma:>10.4} {snr_2500:>12.1} {:>8}",
                reported.map(|s| s.to_string()).unwrap_or_else(|| "—".into())
            );
        };
        for (text, _kind, seed) in cases {
            report(Mode::Ft8, text, seed);
        }
        // FT4, whose floor is a little higher per slot because the slot is half
        // as long — same decoder family, different integration time.
        report(Mode::Ft4, "CQ AB1CD FN42", 0x5eed_0010);
        println!();
    }

    /// A CB station answers our CQ, and the exchange must not stall one message
    /// in. WSJT-CB sends its bare call first (`26AT715`, free text) *so the
    /// hash can be resolved* in everything after it — its pair opener and every
    /// report travel hashed (`<us> <them>`, `<us> <them> -02`). If that bare
    /// call is not fed to the hash table, the pair decodes with `from = None`,
    /// the contact is never attributed, and the machine sits on `CallingCq`
    /// while the operator has to make every reply by hand. This pins the seed.
    #[test]
    fn a_bare_cb_call_seeds_the_hash_a_later_pair_needs() {
        let mut rx = Ft8Modem::new(Mode::Ft8);
        rx.set_cb_wide(true);
        // Our own call, seeded as the controller does from `ap.calls()`.
        rx.seed_hashes(&["25TT304".to_string()]);

        // The bare call arrives and is remembered (the fix under test).
        let bare = Decode {
            slot_utc: 0,
            snr_db: -10,
            dt: 0.1,
            audio_hz: 1500.0,
            message: "26AT715".into(),
            to: None,
            from: None,
            grid: None,
            is_cq: false,
            cq_to: None,
            free_text: true,
            rr73_to: None,
        };
        rx.remember_heard([&bare]);

        // Now the pair opener decodes with the station resolved, not `<...>`.
        let (burst, _) = rx.encode_burst_12k("25TT304 26AT715", 1500.0, 0.5).expect("encode");
        let mut slot = vec![0.0f32; (0.5 * 12_000.0) as usize];
        slot.extend_from_slice(&burst);
        slot.resize((15.0 * 12_000.0) as usize, 0.0);
        let i16buf: Vec<i16> = slot.iter().map(|&s| (s * 20_000.0) as i16).collect();
        let decodes = rx.decode_slot(&i16buf, 0, &ApHints::default(), 1500.0);
        let pair = decodes.iter().find(|d| d.from.is_some()).expect("the pair resolves");
        assert_eq!(pair.to.as_deref(), Some("25TT304"));
        assert_eq!(pair.from.as_deref(), Some("26AT715"));
    }

    /// The shadow list an operator is shown must be exactly what the hash table
    /// can resolve — never more. A callsign that outlived the table's cap would
    /// be promised a resolution the decoder can no longer deliver, which is the
    /// one way this list could lie.
    #[test]
    fn the_known_call_list_holds_exactly_what_the_table_can_resolve() {
        let mut rx = Ft8Modem::new(Mode::Ft8);
        rx.seed_hashes(&["19DC373".to_string()]);
        let d = |message: &str, from: Option<&str>, to: Option<&str>, free_text: bool| Decode {
            slot_utc: 0,
            snr_db: -5,
            dt: 0.1,
            audio_hz: 1500.0,
            message: message.into(),
            to: to.map(String::from),
            from: from.map(String::from),
            grid: None,
            is_cq: message.starts_with("CQ"),
            cq_to: None,
            free_text,
            rr73_to: None,
        };
        // A spelled-out pair, a CQ, and a bare CB call (the 1.9.10 seed).
        let heard = [
            d("19RF410 19AT208 -08", Some("19AT208"), Some("19RF410"), false),
            d("CQ 4CB04", Some("4CB04"), None, false),
            d("109HU333", None, None, true),
        ];
        rx.remember_heard(heard.iter());

        // Newest first, our own call included, the bare CB call present.
        assert_eq!(rx.known_calls(), vec!["109HU333", "4CB04", "19AT208", "19RF410", "19DC373"]);
        assert_eq!(rx.known_calls_len(), 5);

        // A re-heard station moves to the front rather than appearing twice.
        let again = [d("CQ 4CB04", Some("4CB04"), None, false)];
        rx.remember_heard(again.iter());
        assert_eq!(rx.known_calls().first().map(String::as_str), Some("4CB04"));
        assert_eq!(rx.known_calls_len(), 5);

        // Whatever the list shows, the table really resolves it — this is the
        // invariant that stops the list promising a stale callsign.
        for call in rx.known_calls() {
            assert!(
                rx.hashes.lookup22(mfsk_core::msg::hash_table::ihashcall(&call, 22)).is_some(),
                "{call} is listed but the table cannot resolve it"
            );
        }
    }

    /// A station whose name is never spelled out is not a station the list can
    /// show, and a `CQ`/placeholder must not become one. The list is for
    /// callers, so it must not fill with the words a message happens to contain.
    #[test]
    fn the_known_call_list_never_lists_a_cq_or_a_placeholder() {
        let mut rx = Ft8Modem::new(Mode::Ft8);
        rx.remember_call("CQ");
        rx.remember_call("...");
        rx.remember_call("AB");
        rx.remember_call("<...>");
        assert!(rx.known_calls().is_empty(), "got {:?}", rx.known_calls());

        // A portabled call is kept whole: `/P` is part of who they are here,
        // and the table hashes it whole, so a stripped base would not resolve.
        rx.remember_call("r7kjg/qrp");
        assert_eq!(rx.known_calls(), vec!["R7KJG/QRP"]);
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

    /// Issue #667: streamed, the quick pass arrives as the first batch and the
    /// subtraction pass's finds after it, each once; a newer slot waiting
    /// (`stop`) skips the slow pass rather than queueing behind it.
    #[test]
    fn ft8_streams_the_quick_batch_before_subtraction() {
        let modem = Ft8Modem::new(Mode::Ft8);
        let (strong, _) = modem.encode_burst_12k("CQ AB1CD FN42", 2310.0, 0.5).unwrap();
        let weak_amp = 0.5f32 * 10f32.powf(-14.0 / 20.0);
        let (weak, _) = modem.encode_burst_12k("CQ W9XYZ EN52", 2320.0, weak_amp).unwrap();
        let mut out = vec![0.0f32; 180_000];
        for (i, &x) in strong.iter().enumerate() {
            out[6_000 + i] += x;
        }
        for (i, &x) in weak.iter().enumerate() {
            out[6_000 + i] += x;
        }
        let buf: Vec<i16> = out.iter().map(|&x| (x * 12_000.0) as i16).collect();

        let batches = std::sync::Mutex::new(Vec::<Vec<String>>::new());
        let emit = |d: Vec<Decode>| {
            batches.lock().unwrap().push(d.into_iter().map(|d| d.message).collect());
        };
        Ft8Modem::new(Mode::Ft8).decode_slot_streaming(
            &buf,
            0,
            &ApHints::default(),
            2310.0,
            &|| false,
            &emit,
        );
        let b = batches.into_inner().unwrap();
        assert!(b.len() >= 2, "the slow pass's find should be a batch of its own: {b:?}");
        assert!(b[0].iter().any(|m| m == "CQ AB1CD FN42"), "quick first: {b:?}");
        assert!(!b[0].iter().any(|m| m == "CQ W9XYZ EN52"), "weak is the slow pass's: {b:?}");
        assert!(b[1..].iter().flatten().any(|m| m == "CQ W9XYZ EN52"), "{b:?}");
        let all: Vec<&String> = b.iter().flatten().collect();
        let mut uniq = all.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(all.len(), uniq.len(), "a decode was sent twice: {b:?}");

        // A newer slot waiting: the quick batch alone.
        let batches = std::sync::Mutex::new(Vec::<Vec<String>>::new());
        let emit = |d: Vec<Decode>| {
            batches.lock().unwrap().push(d.into_iter().map(|d| d.message).collect());
        };
        Ft8Modem::new(Mode::Ft8).decode_slot_streaming(
            &buf,
            0,
            &ApHints::default(),
            2310.0,
            &|| true,
            &emit,
        );
        assert_eq!(batches.into_inner().unwrap().len(), 1);
    }
}
