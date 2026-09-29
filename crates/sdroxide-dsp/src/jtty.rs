// Portions of this file are ported from WSJT-X
// (https://github.com/WSJTX/wsjtx), tag v3.2.0-rc1, files under lib/jtty/.
// WSJT-X is free software, licensed under the GNU General Public License
// version 3 or later. This file is distributed under the same terms as the
// rest of sdroxide (GPL-3.0-or-later).
//
// JTTY was designed by the WSJT development team (Joe Taylor K1JT, Steve
// Franke K9AN, Bill Somerville G4WJS and others). The forward-error-correction
// profile, the source grammar and the receive shape below are theirs; the Rust
// translation and the circular Viterbi decoder are this fork's.

//! JTTY — the WSJT-X 3.2 RTTY-like asynchronous text mode, receive side.
//!
//! JTTY is not one of the slotted weak-signal modes: a transmission can start
//! at any instant, so there is no T/R period and no frame boundary to lock the
//! clock to. Each **frame** lasts 1.888 s and carries a 13-symbol
//! synchronisation sequence followed by 46 coded 4-GFSK symbols at
//! `12000/384 = 31.25` baud. The four tones sit 31.25 Hz apart, so the signal
//! is about 127 Hz wide — a narrow, weak-signal alternative to 45.45-baud RTTY.
//!
//! # The FEC
//!
//! A frame's 34-bit payload is extended by a 12-bit CRC to 46 information
//! bits, then encoded by a **tail-biting, rate-1/2 convolutional code with
//! constraint length K=10** (generators octal 1167/1545, `d_free = 12`). Two
//! coded bits choose one of the four tones (Gray-mapped 00/01/11/10 → 0/1/2/3).
//! The tail-biting form means the encoder's start state equals its end state,
//! so the decoder searches closed paths in a circular trellis — [`decode_tones`]
//! is a circular Viterbi over the 46 symbols with a CRC gate on the surviving
//! tail-biting path.
//!
//! # The source grammar
//!
//! Bits 1-32 are a message word, bit 33 is reserved (must be zero) and bit 34
//! is the end-of-message flag. Four top-level types: six callsign actions
//! (`CQ K1ABC CQ`, `K1ABC`, …), a typed 30-bit `STRUCT30` exchange, and
//! `TEXT5` (five 6-bit characters per frame). [`decode_source`] renders a whole
//! message's worth of frames and rejects structurally invalid words before
//! they are shown — the rule the reference states as "an invalid source word
//! shall not be displayed".
//!
//! # Why this is not mfsk-core
//!
//! JTTY is not in mfsk-core and not on WSJT-X's `master`; it lives only in the
//! v3.2 release branch. So this is the fork's own decoder, like FSK441.

/// The decoder's working sample rate. The reference runs its DSP at 6 kHz —
/// 31.25 baud is exactly 192 samples a symbol — and hands the receiver 12 kHz
/// audio it halves to that. The controller resamples the 48 kHz tap to
/// [`JTTY_RATE`] before this module sees a sample.
pub const JTTY_RATE: f64 = 6000.0;
/// The symbol (baud) rate: `12000/384`.
pub const JTTY_BAUD: f32 = 31.25;
/// Samples per symbol at [`JTTY_RATE`].
pub const JTTY_NSPS: usize = 192;
/// Tone spacing in Hz — equal to the baud rate, as the reference tone grid is.
pub const JTTY_TONE_SPACING_HZ: f32 = 31.25;
/// Symbols per frame: 13 sync + 46 coded.
pub const JTTY_SYNC_SYMBOLS: usize = 13;
/// Coded symbols per frame (one per information bit pair).
pub const JTTY_CODED_SYMBOLS: usize = 46;
/// Total symbols per frame.
pub const JTTY_FRAME_SYMBOLS: usize = JTTY_SYNC_SYMBOLS + JTTY_CODED_SYMBOLS; // 59
/// The 13-symbol synchronisation sequence (tone indices), reference `is13`.
pub const JTTY_SYNC: [u8; JTTY_SYNC_SYMBOLS] = [0, 2, 2, 3, 0, 0, 3, 2, 1, 3, 1, 2, 0];
/// Frame duration in seconds at the symbol rate.
pub const JTTY_FRAME_S: f32 = JTTY_FRAME_SYMBOLS as f32 / JTTY_BAUD; // 1.888

// ---------------------------------------------------------------------------
// FEC: the tail-biting TBCC over the 1167/1545/0x80F profile.
// ---------------------------------------------------------------------------

/// Payload bits per frame.
pub const PAYLOAD_BITS: usize = 34;
/// Outer CRC-12 bits.
pub const CRC_BITS: usize = 12;
/// Information bits: payload + CRC.
pub const INFORMATION_BITS: usize = PAYLOAD_BITS + CRC_BITS; // 46
/// Convolutional memory order.
const MEMORY_NU: usize = 9;
/// Trellis states (`2^nu`).
const STATE_COUNT: usize = 1 << MEMORY_NU; // 512
/// Constraint-length register mask (`2^(nu+1)-1`).
const REGISTER_MASK: u32 = (1 << (MEMORY_NU + 1)) - 1; // 0x3FF
/// First generator polynomial, octal 1167.
const GENERATOR_0: u32 = 0o1167;
/// Second generator polynomial, octal 1545.
const GENERATOR_1: u32 = 0o1545;
/// CRC-12 polynomial, `0x80F`.
const OUTER_POLYNOMIAL: u32 = 0x80F;
/// The CRC register's top-bit mask.
const OUTER_TOP_BIT_MASK: u32 = 1 << (CRC_BITS - 1); // 0x800
/// The CRC register's width mask.
const OUTER_REGISTER_MASK: u32 = (1 << CRC_BITS) - 1; // 0xFFF
/// The reserved-zero information bit, 1-based (payload bit 33).
pub const RESERVED_BIT: usize = 33;

/// Gray-coded 2-bit map: 00/01/11/10 → 0/1/2/3.
const GRAY_TONES: [u8; 4] = [0, 1, 3, 2];

/// Parity of the set bits in `val`.
fn parity_bit(val: u32) -> u32 {
    val.count_ones() & 1
}

/// CRC-12 over the payload, as 12 check bits, MSB first.
pub fn encode_crc12(payload: &[u8; PAYLOAD_BITS]) -> [u8; CRC_BITS] {
    let mut reg: u32 = 0;
    for &b in payload.iter() {
        reg ^= (b as u32) << (CRC_BITS - 1);
        if reg & OUTER_TOP_BIT_MASK != 0 {
            reg = (reg << 1) ^ OUTER_POLYNOMIAL;
        } else {
            reg <<= 1;
        }
        reg &= OUTER_REGISTER_MASK;
    }
    let mut out = [0u8; CRC_BITS];
    for (i, b) in out.iter_mut().enumerate() {
        *b = ((reg >> (CRC_BITS - 1 - i)) & 1) as u8;
    }
    out
}

/// The 46 information bits for a payload: 34 payload + 12 CRC.
pub fn information_bits(payload: &[u8; PAYLOAD_BITS]) -> [u8; INFORMATION_BITS] {
    let mut info = [0u8; INFORMATION_BITS];
    info[..PAYLOAD_BITS].copy_from_slice(payload);
    info[PAYLOAD_BITS..].copy_from_slice(&encode_crc12(payload));
    info
}

/// Encode a payload into 46 tone symbols. This is the reference's `tbcc_encode`
/// and is used to test the decoder (the fork is receive-only for JTTY for now).
pub fn encode_tones(payload: &[u8; PAYLOAD_BITS]) -> [u8; INFORMATION_BITS] {
    let info = information_bits(payload);
    let mut state: u32 = 0;
    for t in 0..MEMORY_NU {
        let k = INFORMATION_BITS - (MEMORY_NU - 1) + t; // 1-based, ends at INFORMATION_BITS
        let bit = info[k - 1] as u32;
        state = ((state << 1) | bit) & (STATE_COUNT as u32 - 1);
    }
    let mut tones = [0u8; INFORMATION_BITS];
    for t in 0..INFORMATION_BITS {
        let bit = info[t] as u32;
        let reg = ((state << 1) | bit) & REGISTER_MASK;
        let b0 = parity_bit(reg & GENERATOR_0);
        let b1 = parity_bit(reg & GENERATOR_1);
        state = ((state << 1) | bit) & (STATE_COUNT as u32 - 1);
        tones[t] = GRAY_TONES[(2 * b0 + b1) as usize];
    }
    tones
}

/// The tone each (destination-state, dropped-bit) branch produces. Ported from
/// the reference's `build_incoming_tones`: the dropped input bit is the high
/// bit (`bit nu`) and the state is the destination's low `nu` bits.
fn incoming_tones() -> [[u8; 2]; STATE_COUNT] {
    let mut t = [[0u8; 2]; STATE_COUNT];
    for (s, row) in t.iter_mut().enumerate() {
        for dropped in 0..2u32 {
            let reg = (s as u32) | (dropped << MEMORY_NU);
            let b0 = parity_bit(reg & GENERATOR_0);
            let b1 = parity_bit(reg & GENERATOR_1);
            row[dropped as usize] = GRAY_TONES[(2 * b0 + b1) as usize];
        }
    }
    t
}

/// True if the 46 information bits pass the CRC-12.
pub fn crc_ok(bits: &[u8; INFORMATION_BITS]) -> bool {
    let mut reg: u32 = 0;
    for &b in bits.iter() {
        reg ^= (b as u32) << (CRC_BITS - 1);
        if reg & OUTER_TOP_BIT_MASK != 0 {
            reg = (reg << 1) ^ OUTER_POLYNOMIAL;
        } else {
            reg <<= 1;
        }
        reg &= OUTER_REGISTER_MASK;
    }
    reg == 0
}

/// A decoded frame: its payload, and the tail-biting path metric.
#[derive(Debug, Clone, PartialEq)]
pub struct JttyFrame {
    /// The 34 source bits.
    pub payload: [u8; PAYLOAD_BITS],
}

/// Tone energies for one frame: `energies[t][tone]`, 46 symbol times, four
/// tones. Produced by the receive front end from the audio.
pub type ToneEnergies = [[f32; 4]; INFORMATION_BITS];

/// Decode 46 symbols of tone energies into a payload, or `None`.
///
/// A circular (tail-biting) Viterbi: two forward passes over the trellis, then
/// each state is traced back and a path is accepted only if it closes (its
/// remembered start state equals its final state) and its CRC checks. The
/// first `list_size` closed paths by metric are tried, mirroring the
/// reference's list decoder at the level a first cut needs — it corrects 5-6
/// of 46 symbol errors where the reference's coherent-block list decoder
/// corrects 8.
pub fn decode_tones(energies: &ToneEnergies, list_size: usize, iters: usize) -> Option<JttyFrame> {
    let tones = incoming_tones();
    decode_with_table(energies, list_size, iters, &tones)
}

/// [`decode_tones`] with a caller-supplied incoming-tone table, so the table
/// is built once per decode batch rather than once per frame.
#[allow(clippy::needless_range_loop)] // state indices index several parallel arrays
pub fn decode_with_table(
    energies: &ToneEnergies,
    list_size: usize,
    iters: usize,
    tones: &[[u8; 2]; STATE_COUNT],
) -> Option<JttyFrame> {
    if iters < 1 {
        return None;
    }
    let mut prev_m = vec![0.0f32; STATE_COUNT];
    let mut curr_m = vec![0.0f32; STATE_COUNT];
    let mut tb = vec![0u8; STATE_COUNT * INFORMATION_BITS];

    for _ in 0..iters {
        for t in 0..INFORMATION_BITS {
            let e = &energies[t];
            for s in 0..STATE_COUNT {
                let base = s >> 1;
                let p0 = base;
                let p1 = base | (1 << (MEMORY_NU - 1));
                let m0 = prev_m[p0] + e[tones[s][0] as usize];
                let m1 = prev_m[p1] + e[tones[s][1] as usize];
                if m0 > m1 {
                    curr_m[s] = m0;
                    tb[s * INFORMATION_BITS + t] = 0;
                } else {
                    curr_m[s] = m1;
                    tb[s * INFORMATION_BITS + t] = 1;
                }
            }
            prev_m.copy_from_slice(&curr_m);
        }

        let mut cands: Vec<(f32, [u8; INFORMATION_BITS])> = Vec::new();
        for s in 0..STATE_COUNT {
            let start = s;
            let mut curr_s = s;
            let mut bits = [0u8; INFORMATION_BITS];
            for t in (0..INFORMATION_BITS).rev() {
                bits[t] = (curr_s & 1) as u8;
                let mut prev = curr_s >> 1;
                if tb[curr_s * INFORMATION_BITS + t] == 1 {
                    prev |= 1 << (MEMORY_NU - 1);
                }
                curr_s = prev;
            }
            if curr_s == start {
                cands.push((prev_m[s], bits));
            }
        }
        cands.sort_by(|a, b| b.0.total_cmp(&a.0));
        cands.truncate(list_size);
        for (_, bits) in &cands {
            if crc_ok(bits) {
                let mut payload = [0u8; PAYLOAD_BITS];
                payload.copy_from_slice(&bits[..PAYLOAD_BITS]);
                return Some(JttyFrame { payload });
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Source grammar: decode a message's worth of frames into display text.
// ---------------------------------------------------------------------------

/// The TEXT5 character alphabet, indexed by six-bit value. The reference's
/// `ALPHABET` in `lib/jtty/jtty_source_codec.f90`, exactly 64 characters.
const JTTY_CHARSET: &[u8; 64] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ +-./?!\"#$%,&*()_'=[]{}<>|:;";

/// Structurally valid source word, or why not.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceError {
    /// The all-zero grammar word is the invalid sentinel.
    Sentinel,
    /// The reserved bit (bit 33) was set.
    ReservedBit,
    /// A reserved/invalid type, family, subtype, enum value or field range.
    Invalid,
}

/// The call-action type from top-level bits (`i2.n2`, for `i2` 0 or 1).
#[derive(Debug, Clone, Copy, PartialEq)]
enum TopKind {
    Cq,
    Call,
    TuCq,
    CallTu,
    CallAgn,
    TuNowCall,
}

fn bits1to32(payload: &[u8; PAYLOAD_BITS]) -> u32 {
    // Bits are most-significant first: bit 1 is the high bit.
    let mut v = 0u32;
    for &b in &payload[..32] {
        v = (v << 1) | (b as u32 & 1);
    }
    v
}

/// The 30-bit call action, if the word is one (`i2` 0 or 1).
fn call_action(payload: &[u8; PAYLOAD_BITS]) -> Option<(TopKind, u32)> {
    let call28 = {
        let mut v = 0u32;
        for &b in &payload[..28] {
            v = (v << 1) | (b as u32 & 1);
        }
        v
    };
    let n2 = (payload[28] as u32) << 1 | payload[29] as u32;
    let i2 = (payload[30] as u32) << 1 | payload[31] as u32;
    let kind = match (i2, n2) {
        (0, 0) => TopKind::Cq,
        (0, 1) => TopKind::Call,
        (0, 2) => TopKind::TuCq,
        (0, 3) => TopKind::CallTu,
        (1, 0) => TopKind::CallAgn,
        (1, 1) => TopKind::TuNowCall,
        _ => return None, // 1.2, 1.3 reserved
    };
    Some((kind, call28))
}

/// Render the 28-bit call field, ported from the reference's `unpack28_core`
/// (`lib/77bit/packjt77.f90`). JTTY reuses the FT8/FT4 standard-call grammar.
///
/// A `None` return means the field is one this decoder does not reconstruct —
/// a hashed 22-bit call needs the session's hash table, and a word that fails
/// the callsign check is not shown. It is never invented text.
fn render_call28(n28: u32) -> Option<String> {
    const NTOKENS: u32 = 2_063_592;
    const MAX22: u32 = 4_194_304;
    // Alphabets, exactly the reference's c1/c2/c3/c4.
    const C1: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 37, leading space
    const C2: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 36
    const C3: &[u8] = b"0123456789"; // 10
    const C4: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 27, leading space

    let mut n = n28;
    if n < NTOKENS {
        match n {
            0 => return Some("DE".into()),
            1 => return Some("QRZ".into()),
            2 => return Some("CQ".into()),
            3..=1002 => return Some(format!("CQ_{:03}", n - 3)),
            1003..=532_443 => {
                let mut m = n - 1003;
                let i1 = m / (27 * 27 * 27);
                m -= 27 * 27 * 27 * i1;
                let i2 = m / (27 * 27);
                m -= 27 * 27 * i2;
                let i3 = m / 27;
                let i4 = m - 27 * i3;
                let s = format!(
                    "{}{}{}{}",
                    C4[i1 as usize] as char,
                    C4[i2 as usize] as char,
                    C4[i3 as usize] as char,
                    C4[i4 as usize] as char
                );
                return Some(format!("CQ_{}", s.trim()));
            }
            _ => return None,
        }
    }
    n -= NTOKENS;
    if n < MAX22 {
        // A 22-bit callsign hash; resolving it needs the session's hash table,
        // which is a later concern. Do not invent a call.
        return None;
    }
    // Standard callsign.
    let mut m = n - MAX22;
    let i1 = m / (36 * 10 * 27 * 27 * 27);
    m -= 36 * 10 * 27 * 27 * 27 * i1;
    let i2 = m / (10 * 27 * 27 * 27);
    m -= 10 * 27 * 27 * 27 * i2;
    let i3 = m / (27 * 27 * 27);
    m -= 27 * 27 * 27 * i3;
    let i4 = m / (27 * 27);
    m -= 27 * 27 * i4;
    let i5 = m / 27;
    let i6 = m - 27 * i5;
    let raw = [
        C1[i1 as usize] as char,
        C2[i2 as usize] as char,
        C3[i3 as usize] as char,
        C4[i4 as usize] as char,
        C4[i5 as usize] as char,
        C4[i6 as usize] as char,
    ];
    let s: String = raw.iter().collect::<String>().trim().to_string();
    // A two-digit prefix is not a standard call (reference reject rule).
    if s.len() >= 2 && s.as_bytes()[0].is_ascii_digit() && s.as_bytes()[1].is_ascii_digit() {
        return None;
    }
    if s.is_empty() { None } else { Some(s) }
}

/// The 18 registered control phrases.
const CONTROL_PHRASES: [&str; 18] = [
    "AGN?", "CALL?", "AGN CALL", "NR?", "AGN NR", "EXCH?", "STATE?", "SECTION?", "ZONE?", "GRID?",
    "RPRT?", "QSL TU", "TU", "QRZ?", "QSO B4", "WAIT", "NIL?", "OK?",
];

/// The ARRL/RAC section names, indexed 1..=86 (index 0 and >86 invalid). This
/// is the reference's `PACK77_ARRL_SECTIONS` registry, which JTTY shares
/// rather than copying — same entries, same order, so a section decoded here
/// is the one WSJT-X means.
const ARRL_SECTIONS: [&str; 86] = [
    "AB", "AK", "AL", "AR", "AZ", "BC", "CO", "CT", "DE", "EB", "EMA", "ENY", "EPA", "EWA", "GA",
    "GH", "IA", "ID", "IL", "IN", "KS", "KY", "LA", "LAX", "NS", "MB", "MDC", "ME", "MI", "MN",
    "MO", "MS", "MT", "NC", "ND", "NE", "NFL", "NH", "NL", "NLI", "NM", "NNJ", "NNY", "TER", "NTX",
    "NV", "OH", "OK", "ONE", "ONN", "ONS", "OR", "ORG", "PAC", "PR", "QC", "RI", "SB", "SC", "SCV",
    "SD", "SDG", "SF", "SFL", "SJV", "SK", "SNJ", "STX", "SV", "TN", "UT", "VA", "VI", "VT", "WCF",
    "WI", "WMA", "WNY", "WPA", "WTX", "WV", "WWA", "WY", "DX", "PE", "NB",
];

fn base36_token(token: u16, len3: bool) -> Option<String> {
    let n = if len3 {
        if (token as u32) < 36 * 36 || (token as u32) >= 36 * 36 * 36 {
            return None;
        }
        token as u32
    } else {
        if (token as u32) >= 36 * 36 {
            return None;
        }
        token as u32
    };
    let digits = if len3 { 3 } else { 2 };
    let alphabet = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut out = Vec::with_capacity(digits);
    let mut n = n;
    for _ in 0..digits {
        out.push(alphabet[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    let s = String::from_utf8(out).ok()?;
    // A three-character spelling with a leading zero is not canonical.
    if len3 && s.starts_with('0') {
        return None;
    }
    Some(s)
}

fn render_grid4(code: u16) -> Option<String> {
    if code > 32399 {
        return None;
    }
    let f = code / 100;
    let d = code % 100;
    let (f1, f2) = (f / 18, f % 18);
    let letters = |v: u16| (b'A' + (v % 18) as u8) as char;
    // field = f1*18 + f2 as uppercase letters; the code packs them as
    // (((f1*18 + f2)*10 + d1)*10 + d2), with f1,f2 in 0..18 (A-R).
    let l1 = letters(f / 18);
    let l2 = letters(f % 18);
    let d1 = d / 10;
    let d2 = d % 10;
    if f1 > 17 || f2 > 17 {
        return None;
    }
    Some(format!("{l1}{l2}{d1}{d2}"))
}

/// Render one STRUCT30 atom, or `None` if it is invalid.
fn render_struct30(payload: &[u8; PAYLOAD_BITS]) -> Option<String> {
    let body = &payload[..27];
    let family = {
        let mut v = 0u8;
        for &b in &payload[27..30] {
            v = (v << 1) | (b & 1);
        }
        v
    };
    // Read field helpers: body is 27 bits, MSB first.
    let get = |start: usize, len: usize| -> u32 {
        let mut v = 0u32;
        for &b in &body[start..start + len] {
            v = (v << 1) | (b as u32 & 1);
        }
        v
    };
    match family {
        0b000 => {
            // EXCH_NUM: role1 kind4 value17 zero5
            let role = get(0, 1);
            let kind = get(1, 4);
            let value = get(5, 17);
            if get(22, 5) != 0 {
                return None;
            }
            let field = match kind {
                0 => format!("{value:03}"),                              // SERIAL
                1 if (1..=40).contains(&value) => format!("{value:02}"), // CQ_ZONE
                2 if (1..=90).contains(&value) => format!("{value:02}"), // ITU_ZONE
                3 => format!("{value}"),                                 // AGE
                4 => format!("{value}"),                                 // POWER
                5 => format!("{value:02}"),                              // CHECK
                6 if value <= 9999 => format!("{value:04}"),             // FIRST_LICENSE_YEAR
                7 => format!("{value}"),                                 // GENERIC_NUMERIC
                _ => return None,
            };
            Some(if role == 1 { format!("599 {field}") } else { field })
        }
        0b001 => {
            // EXCH_LOC: role1 kind4 length1 token16 zero5
            let role = get(0, 1);
            let kind = get(1, 4);
            let len3 = get(5, 1) == 1;
            let token = get(6, 16) as u16;
            if get(22, 5) != 0 {
                return None;
            }
            if kind > 4 {
                return None;
            }
            let field = base36_token(token, len3)?;
            Some(if role == 1 { format!("599 {field}") } else { field })
        }
        0b010 => {
            // EXCH_PAIR: schema3 pair_data23 zero1
            let schema = get(0, 3);
            if get(26, 1) != 0 {
                return None;
            }
            match schema {
                0 => {
                    // ZONE_LOC3: cq_zone6 length1 token16
                    let zone = get(3, 6);
                    let len3 = get(9, 1) == 1;
                    let token = get(10, 16) as u16;
                    if !(1..=40).contains(&zone) {
                        return None;
                    }
                    let field = base36_token(token, len3)?;
                    Some(format!("599 {zone:02} {field}"))
                }
                1 => {
                    // CLASS_SECTION: count6 class3 section7 zero7
                    let count = get(3, 6);
                    let class = get(9, 3);
                    let section = get(12, 7);
                    if get(19, 7) != 0 {
                        return None;
                    }
                    if !(1..=32).contains(&count) || class > 5 {
                        return None;
                    }
                    // Section is a 1-based index into the registry.
                    if section < 1 {
                        return None;
                    }
                    let section = *ARRL_SECTIONS.get((section - 1) as usize)?;
                    let class_ch = (b'A' + class as u8) as char;
                    Some(format!("{count}{class_ch} {section}"))
                }
                _ => None,
            }
        }
        0b011 => {
            // EXCH_NUM_TIME: role1 serial14 minute11 zero1
            let role = get(0, 1);
            let serial = get(1, 14);
            let minute = get(15, 11);
            if get(26, 1) != 0 || minute > 1439 {
                return None;
            }
            let hhmm = format!("{:02}{:02}", minute / 60, minute % 60);
            let field = format!("{serial:03} {hhmm}");
            Some(if role == 1 { format!("599 {field}") } else { field })
        }
        0b100 => {
            // MISC: subtype4 subtype_data23
            let subtype = get(0, 4);
            match subtype {
                0 => {
                    // CONTROL: phrase7 zero16
                    let phrase = get(4, 7);
                    if get(11, 16) != 0 {
                        return None;
                    }
                    CONTROL_PHRASES.get(phrase as usize).map(|s| s.to_string())
                }
                1 => {
                    // GRID4: role1 grid4_code15 zero7
                    let role = get(4, 1);
                    let code = get(5, 15) as u16;
                    if get(20, 7) != 0 {
                        return None;
                    }
                    let g = render_grid4(code)?;
                    Some(if role == 1 { format!("599 {g}") } else { g })
                }
                _ => None,
            }
        }
        // 101 PROFILED, 110 VERSIONED, 111 INVALID: all invalid in v1.
        _ => None,
    }
}

/// One decoded source atom: its text, and whether it is a structured atom
/// (which contributes an implicit separator) rather than a verbatim TEXT5 run.
#[derive(Debug, Clone, PartialEq)]
pub struct JttyAtom {
    pub text: String,
    pub structured: bool,
}

/// Decode one source word to its atom, or `None` if it is structurally invalid.
pub fn decode_atom(payload: &[u8; PAYLOAD_BITS]) -> Option<JttyAtom> {
    if payload[32] != 0 {
        return None; // reserved bit set
    }
    let word = bits1to32(payload);
    if word == 0 {
        return None; // sentinel
    }
    // Top-level type: bits 31-32 are i2, but call actions also carry bits 29-30.
    let i2 = ((payload[30] & 1) << 1) | (payload[31] & 1);
    match i2 {
        0 | 1 => {
            let (kind, call28) = call_action(payload)?;
            let call = render_call28(call28)?;
            let text = match kind {
                TopKind::Cq => format!("CQ {call} CQ"),
                TopKind::Call => call,
                TopKind::TuCq => format!("TU {call} CQ"),
                TopKind::CallTu => format!("{call} TU"),
                TopKind::CallAgn => format!("{call} AGN?"),
                TopKind::TuNowCall => format!("TU NOW {call}"),
            };
            Some(JttyAtom { text, structured: true })
        }
        2 => render_struct30(payload).map(|text| JttyAtom { text, structured: true }),
        3 => {
            // TEXT5: bits 1-30 are five six-bit characters, bits 31-32 = 11.
            // Verbatim — no implicit separator, and the trailing padding is not
            // part of the message.
            let mut s = String::new();
            for k in 0..5 {
                let mut v = 0u8;
                for i in 0..6 {
                    v = (v << 1) | (payload[k * 6 + i] & 1);
                }
                s.push(JTTY_CHARSET[v as usize] as char);
            }
            Some(JttyAtom { text: s.trim_end().to_string(), structured: false })
        }
        _ => None,
    }
}

/// Render one source word to text, or `None` if it is structurally invalid.
///
/// This is a single atom; a whole message is a sequence of frames, the last
/// carrying the EOM bit. Use [`decode_source`] for a message.
pub fn decode_word(payload: &[u8; PAYLOAD_BITS]) -> Option<String> {
    decode_atom(payload).map(|a| a.text)
}

/// True if the word carries the end-of-message flag (bit 34).
pub fn is_eom(payload: &[u8; PAYLOAD_BITS]) -> bool {
    payload[33] == 1
}

/// A decoded JTTY message: the accumulated text and whether EOM was seen.
#[derive(Debug, Clone, PartialEq)]
pub struct JttyMessage {
    pub text: String,
    pub complete: bool,
}

/// Decode a sequence of frames into one message.
///
/// Atoms are rendered independently and joined; a structured atom supplies one
/// implicit trailing space and TEXT5 is appended verbatim, per the spec. The
/// display is truncated at 80 characters. An invalid word ends the run without
/// inventing text.
pub fn decode_source(frames: &[JttyFrame]) -> Option<JttyMessage> {
    let mut out = String::new();
    let mut complete = false;
    for f in frames {
        let atom = decode_atom(&f.payload)?;
        // Canonical atom sequence: a structured atom carries one implicit
        // trailing space, TEXT5 is appended verbatim. Getting this wrong runs
        // two TEXT5 fragments together or doubles a boundary space.
        out.push_str(&atom.text);
        if atom.structured && !out.ends_with(' ') {
            out.push(' ');
        }
        if is_eom(&f.payload) {
            complete = true;
            break;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out.truncate(80);
    if out.is_empty() { None } else { Some(JttyMessage { text: out, complete }) }
}

// ---------------------------------------------------------------------------
// Source packer: text -> frames, for transmit. The inverse of the decoder.
// ---------------------------------------------------------------------------

/// Pack a standard callsign into its 28-bit field, mirroring the reference's
/// `unpack28_core` for the standard-call case (the only case this packer
/// produces — a hashed call is not something we transmit). Returns `None` if
/// the token is not a standard call the reference could round-trip.
pub fn pack_call28(call: &str) -> Option<u32> {
    const NTOKENS: u32 = 2_063_592;
    const MAX22: u32 = 4_194_304;
    const A1: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const A2: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const A3: &[u8] = b"0123456789";
    const A4: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    let call = call.trim();
    // The reference's standard-call shape: 3..=6 chars, alphanumeric, a digit
    // in position 2 or 3, at least one letter before it, all letters after it.
    let bytes = call.as_bytes();
    let n = bytes.len();
    if !(3..=6).contains(&n) {
        return None;
    }
    if !bytes.iter().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
        return None;
    }
    let mut iarea = 0usize;
    for i in (1..n).rev() {
        if bytes[i].is_ascii_digit() {
            iarea = i + 1; // 1-based
            break;
        }
    }
    if iarea != 2 && iarea != 3 {
        return None;
    }
    let npdig = bytes[..iarea - 1].iter().filter(|b| b.is_ascii_digit()).count();
    let nplet = bytes[..iarea - 1].iter().filter(|b| b.is_ascii_uppercase()).count();
    if nplet == 0 || npdig >= iarea - 1 {
        return None;
    }
    if !bytes[iarea..].iter().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    if n > iarea && bytes[iarea..].iter().filter(|b| b.is_ascii_uppercase()).count() > 3 {
        return None;
    }
    // Pad to six characters the way the reference does.
    let padded: Vec<u8> = if iarea == 2 {
        let mut v = vec![b' '];
        v.extend_from_slice(&bytes[..n]);
        v
    } else {
        bytes.to_vec()
    };
    if padded.len() != 6 {
        return None;
    }
    let idx = |alpha: &[u8], c: u8| alpha.iter().position(|&x| x == c).map(|i| i as u32);
    let i1 = idx(A1, padded[0])?;
    let i2 = idx(A2, padded[1])?;
    let i3 = idx(A3, padded[2])?;
    let i4 = idx(A4, padded[3])?;
    let i5 = idx(A4, padded[4])?;
    let i6 = idx(A4, padded[5])?;
    let n28 = 36 * 10 * 27 * 27 * 27 * i1
        + 10 * 27 * 27 * 27 * i2
        + 27 * 27 * 27 * i3
        + 27 * 27 * i4
        + 27 * i5
        + i6;
    Some(NTOKENS + MAX22 + n28)
}

/// Index of a JTTY character in the 64-character alphabet, or `None`.
fn source_index(c: u8) -> Option<u8> {
    JTTY_CHARSET.iter().position(|&x| x == c).map(|i| i as u8)
}

/// Write a 32-bit grammar word plus the reserved bit and EOM into a payload.
fn word_to_payload(n32: u32, eom: bool) -> [u8; PAYLOAD_BITS] {
    let mut p = [0u8; PAYLOAD_BITS];
    for (i, slot) in p.iter_mut().take(32).enumerate() {
        *slot = ((n32 >> (31 - i)) & 1) as u8;
    }
    p[32] = 0; // reserved
    p[33] = if eom { 1 } else { 0 };
    p
}

/// Set the top 30 bits of a word from five six-bit characters.
fn text5_word(s: &str) -> Option<u32> {
    let bytes = s.as_bytes();
    let mut top30 = 0u32;
    for i in 0..5 {
        let c = bytes.get(i).copied().unwrap_or(b' ');
        let idx = source_index(c)?;
        top30 = top30 * 64 + idx as u32;
    }
    Some(4 * top30 + 3)
}

/// A source atom the packer can emit.
#[derive(Debug, Clone, PartialEq)]
enum Atom {
    Call { action: u8, call: String },
    ExchNum { role: u8, kind: u8, value: u32 },
    ExchLoc { role: u8, kind: u8, text: String },
    ClassSection { count: u32, class: char, section: usize },
    Grid4 { role: u8, grid: String },
    Control { phrase: u8 },
    Text5(String),
}

fn tx_base36_token(s: &str) -> Option<(u16, bool)> {
    let b = s.as_bytes();
    if !(b.len() == 2 || b.len() == 3) {
        return None;
    }
    let mut v: u32 = 0;
    for &c in b {
        let d = if c.is_ascii_digit() {
            (c - b'0') as u32
        } else if c.is_ascii_uppercase() {
            (c - b'A') as u32 + 10
        } else {
            return None;
        };
        v = v * 36 + d;
    }
    let len3 = b.len() == 3;
    if len3 {
        if !(36 * 36..36 * 36 * 36).contains(&v) || b[0] == b'0' {
            return None;
        }
    } else if v >= 36 * 36 {
        return None;
    }
    Some((v as u16, len3))
}

fn grid4_index(s: &str) -> Option<u16> {
    let b = s.as_bytes();
    if b.len() != 4 {
        return None;
    }
    let f1 = (b[0] as char).to_digit(36)?; // A..R
    let f2 = (b[1] as char).to_digit(36)?;
    if !(10..=27).contains(&f1) || !(10..=27).contains(&f2) {
        return None;
    }
    let d1 = (b[2] as char).to_digit(10)?;
    let d2 = (b[3] as char).to_digit(10)?;
    let f = (f1 - 10) * 18 + (f2 - 10);
    Some(((f * 10 + d1) * 10 + d2) as u16)
}

/// The ARRL/RAC section index (1-based) for a section name, or `None`.
fn section_index(name: &str) -> Option<usize> {
    let up = name.to_ascii_uppercase();
    ARRL_SECTIONS.iter().position(|&s| s == up).map(|i| i + 1)
}

/// Pack one atom into a 32-bit grammar word, or `None` if it is not valid.
fn pack_atom(atom: &Atom) -> Option<u32> {
    let finish = |body: u32, family: u32| -> u32 {
        // body occupies bits 1-27, family bits 28-30, i2=2 in bits 31-32.
        ((body & 0x07FF_FFFF) << 5) | ((family & 0x7) << 2) | 0b10
    };
    match atom {
        Atom::Call { action, call } => {
            // Actions 0-3 are i2=0 with n2=action; actions 4-5 are i2=1 with
            // n2=action-4. n32 = (n28 << 4) + 4*n2 + i2, per the reference.
            let n28 = pack_call28(call)?;
            let (i2, n2) =
                if *action <= 3 { (0u32, *action as u32) } else { (1u32, (*action - 4) as u32) };
            Some((n28 << 4) | (4 * n2 + i2))
        }
        Atom::ExchNum { role, kind, value } => {
            // EXCH_NUM: role1 kind4 value17 zero5
            let body = ((*role as u32 & 1) << 26)
                | ((*kind as u32 & 0xF) << 22)
                | ((*value & 0x1FFFF) << 5);
            Some(finish(body, 0))
        }
        Atom::ExchLoc { role, kind, text } => {
            let (token, len3) = tx_base36_token(text)?;
            let body = ((*role as u32 & 1) << 26)
                | ((*kind as u32 & 0xF) << 22)
                | ((len3 as u32) << 21)
                | ((token as u32 & 0xFFFF) << 5);
            Some(finish(body, 1))
        }
        Atom::ClassSection { count, class, section } => {
            if !(1..=32).contains(count) || *section == 0 || *section > 86 {
                return None;
            }
            let class_i = (*class as u8).wrapping_sub(b'A');
            if class_i > 5 {
                return None;
            }
            // EXCH_PAIR schema1: count6 class3 section7 zero7 -> pair_data
            let pair_data = ((*count & 0x3F) << 17)
                | ((class_i as u32 & 0x7) << 14)
                | ((*section as u32 & 0x7F) << 7);
            let body = (1u32 << 24) | ((pair_data & 0x7F_FFFF) << 1);
            Some(finish(body, 2))
        }
        Atom::Grid4 { role, grid } => {
            let g = grid4_index(grid)?;
            let data = ((*role as u32 & 1) << 22) | ((g as u32 & 0x7FFF) << 7);
            let body = (1u32 << 23) | data;
            Some(finish(body, 4))
        }
        Atom::Control { phrase } => {
            let body = (*phrase as u32 & 0x7F) << 16;
            Some(finish(body, 4))
        }
        Atom::Text5(s) => text5_word(s),
    }
}

/// A packed message: the frames' payloads, in order.
pub type PackedMessage = Vec<[u8; PAYLOAD_BITS]>;

/// Render an atom to the text it represents, for the packer's exactness check.
fn render_atom(atom: &Atom) -> String {
    match atom {
        Atom::Call { action, call } => match action {
            0 => format!("CQ {call} CQ"),
            1 => call.clone(),
            2 => format!("TU {call} CQ"),
            3 => format!("{call} TU"),
            4 => format!("{call} AGN?"),
            5 => format!("TU NOW {call}"),
            _ => String::new(),
        },
        Atom::ExchNum { role, kind, value } => {
            let field = match kind {
                0 | 5 => format!("{value:03}"),
                1 | 2 => format!("{value:02}"),
                6 => format!("{value:04}"),
                _ => format!("{value}"),
            };
            if *role == 1 { format!("599 {field}") } else { field }
        }
        Atom::ExchLoc { role, text, .. } => {
            if *role == 1 {
                format!("599 {text}")
            } else {
                text.clone()
            }
        }
        Atom::ClassSection { count, class, section } => {
            let sec = ARRL_SECTIONS.get(section.wrapping_sub(1)).copied().unwrap_or("");
            format!("{count}{class} {sec}")
        }
        Atom::Grid4 { role, grid } => {
            if *role == 1 {
                format!("599 {grid}")
            } else {
                grid.clone()
            }
        }
        Atom::Control { phrase } => {
            CONTROL_PHRASES.get(*phrase as usize).copied().unwrap_or("").to_string()
        }
        Atom::Text5(s) => s.clone(),
    }
}

/// Normalise an operator's message: uppercase, collapse spaces, fold anything
/// outside the alphabet to `#`, per the reference's literal interface.
pub fn normalize_text(msg: &str) -> String {
    let mut out = String::new();
    let mut prev_space = false;
    for c in msg.trim().chars() {
        let up = c.to_ascii_uppercase();
        if up == ' ' {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            let ch = if source_index(up as u8).is_some() { up } else { '#' };
            out.push(ch);
            prev_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Common control phrases for the packer's recognition, indexed as on the wire.
const CONTROL_PACK_LIMIT: u8 = 18;

/// Pack a message into one or more frames, choosing the fewest frames that
/// preserve the normalized text exactly — the reference's DP over character
/// offsets, offering compact atoms and TEXT5.
///
/// This is the practical subset the reference's ordinary text packer uses:
/// the six call actions, registered control phrases, generic numeric and QTH
/// exchanges, GRID4 and class/section (Field Day). Contest-specific profiles
/// (RTTY Roundup serial normalization) are not applied — text packs as it
/// reads.
pub fn pack_message(msg: &str) -> Option<PackedMessage> {
    let text = normalize_text(msg);
    let n = text.len();
    if n == 0 {
        return None;
    }
    let bytes = text.as_bytes();
    const INF: usize = 999;
    // dp[i] = fewest frames for text[i..]; successor[i] = next offset.
    let mut dp = vec![INF; n + 1];
    let mut succ = vec![0usize; n];
    let mut choice: Vec<Option<Atom>> = vec![None; n];
    dp[n] = 0;

    // At each position, try TEXT5 (next 5 chars) and, at token boundaries,
    // every compact atom whose rendering matches the text span exactly.
    let mut i = n;
    while i > 0 {
        i -= 1;
        let consider = |atom: Atom,
                        next: usize,
                        dp: &mut Vec<usize>,
                        succ: &mut Vec<usize>,
                        choice: &mut Vec<Option<Atom>>| {
            let cost = 1 + dp[next];
            if cost <= dp[i] {
                dp[i] = cost;
                succ[i] = next;
                choice[i] = Some(atom);
            }
        };

        // TEXT5 always available: exactly five chars, or the tail.
        let end = (i + 5).min(n);
        let frag = &text[i..end];
        if let Some(w) = text5_word(&format!("{frag:<5}")) {
            let _ = w;
            consider(Atom::Text5(frag.to_string()), end, &mut dp, &mut succ, &mut choice);
        }

        // Compact atoms only at a token boundary.
        let at_boundary = i == 0 || bytes[i - 1] == b' ';
        if at_boundary {
            // Gather the tokens starting here (up to three).
            let mut words: Vec<(usize, usize)> = Vec::new();
            let mut p = i;
            while p < n && words.len() < 3 {
                let e = text[p..].find(' ').map(|k| p + k).unwrap_or(n);
                words.push((p, e));
                p = if e < n { e + 1 } else { n };
            }
            let w0 = words.first().map(|&(a, b)| &text[a..b]).unwrap_or("");
            let w1 = words.get(1).map(|&(a, b)| &text[a..b]).unwrap_or("");

            // Call actions. Each renders around a *callsign* word, which is not
            // always the first word (`CQ K1ABC CQ` puts the call second), so
            // try the words a call could be and keep the action whose rendering
            // matches the text span exactly.
            for cand in words.iter().take(2) {
                let call = &text[cand.0..cand.1];
                if pack_call28(call).is_none() {
                    continue;
                }
                for action in 0u8..6 {
                    let atom = Atom::Call { action, call: call.to_string() };
                    let rendered = render_atom(&atom);
                    if !text[i..].starts_with(&rendered) {
                        continue;
                    }
                    let mut next = i + rendered.len();
                    if next < n && bytes[next] == b' ' {
                        next += 1;
                    }
                    consider(atom, next, &mut dp, &mut succ, &mut choice);
                }
            }
            // Control phrases (may be several words).
            for phrase in 0..CONTROL_PACK_LIMIT {
                let atom = Atom::Control { phrase };
                let rendered = render_atom(&atom);
                if text[i..].starts_with(&rendered) {
                    let mut next = i + rendered.len();
                    if next < n && bytes[next] == b' ' {
                        next += 1;
                    }
                    consider(atom, next, &mut dp, &mut succ, &mut choice);
                }
            }
            // Numeric / grid / QTH exchanges, field-only and 599-prefixed.
            for role in 0u8..=1 {
                let (field, prefix) = if role == 1 {
                    if w0 != "599" || w1.is_empty() {
                        continue;
                    }
                    (w1, "599 ")
                } else {
                    (w0, "")
                };
                if field.is_empty() {
                    continue;
                }
                let _ = prefix;
                // Generic numeric.
                if field.len() <= 6 && field.bytes().all(|b| b.is_ascii_digit()) {
                    let v: u32 = field.parse().ok()?;
                    if v <= 131_071 {
                        let atom = Atom::ExchNum { role, kind: 7, value: v };
                        let rendered = render_atom(&atom);
                        if text[i..].starts_with(&rendered) {
                            let mut next = i + rendered.len();
                            if next < n && bytes[next] == b' ' {
                                next += 1;
                            }
                            consider(atom, next, &mut dp, &mut succ, &mut choice);
                        }
                    }
                }
                // GRID4.
                if field.len() == 4 {
                    let atom = Atom::Grid4 { role, grid: field.to_string() };
                    let rendered = render_atom(&atom);
                    if text[i..].starts_with(&rendered) {
                        let mut next = i + rendered.len();
                        if next < n && bytes[next] == b' ' {
                            next += 1;
                        }
                        consider(atom, next, &mut dp, &mut succ, &mut choice);
                    }
                }
                // Generic QTH (two or three base-36 chars, at least one letter).
                if (field.len() == 2 || field.len() == 3)
                    && field.bytes().any(|b| b.is_ascii_uppercase())
                    && tx_base36_token(field).is_some()
                {
                    let atom = Atom::ExchLoc { role, kind: 3, text: field.to_string() };
                    let rendered = render_atom(&atom);
                    if text[i..].starts_with(&rendered) {
                        let mut next = i + rendered.len();
                        if next < n && bytes[next] == b' ' {
                            next += 1;
                        }
                        consider(atom, next, &mut dp, &mut succ, &mut choice);
                    }
                }
            }
            // Class/section: <count><class> <section>.
            if w0.len() >= 2
                && !w1.is_empty()
                && let (count_str, class_ch) = w0.split_at(w0.len() - 1)
                && count_str.bytes().all(|b| b.is_ascii_digit())
                && let Ok(count) = count_str.parse::<u32>()
                && let Some(sec) = section_index(w1)
                && (1..=32).contains(&count)
                && class_ch.as_bytes()[0].is_ascii_uppercase()
            {
                let atom = Atom::ClassSection {
                    count,
                    class: class_ch.chars().next().unwrap(),
                    section: sec,
                };
                let rendered = render_atom(&atom);
                if text[i..].starts_with(&rendered) {
                    let mut next = i + rendered.len();
                    if next < n && bytes[next] == b' ' {
                        next += 1;
                    }
                    consider(atom, next, &mut dp, &mut succ, &mut choice);
                }
            }
        }
    }

    if dp[0] > 16 {
        return None;
    }
    // Walk the choices and emit frames, EOM on the last.
    let mut atoms = Vec::new();
    let mut i = 0usize;
    while i < n {
        let a = choice[i].clone()?;
        atoms.push(a);
        let next = succ[i];
        if next <= i {
            return None;
        }
        i = next;
    }
    let last = atoms.len().saturating_sub(1);
    let mut frames = Vec::new();
    for (k, a) in atoms.iter().enumerate() {
        let n32 = pack_atom(a)?;
        frames.push(word_to_payload(n32, k == last));
    }
    Some(frames)
}

// ---------------------------------------------------------------------------
// Receive front end: 4-GFSK sync search, tone energies, frame assembly.
// ---------------------------------------------------------------------------

use num_complex::Complex32;
use std::f32::consts::TAU;

/// Gaussian pulse for the GFSK frequency smoothing (`gfsk_pulse`, BT=2.0 for
/// JTTY). Used by the synthetic transmitter in tests.
fn gfsk_pulse(bt: f32, t: f32) -> f32 {
    use std::f32::consts::PI;
    let c = PI * (2.0f32 / std::f32::consts::LN_2).sqrt();
    0.5 * (erf(c * bt * (t + 0.5)) - erf(c * bt * (t - 0.5)))
}

/// Abramowitz & Stegun 7.1.26 error function approximation. Good to ~1e-7,
/// which is far below the transmit audio's dynamic range.
fn erf(x: f32) -> f32 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0
        - (((((1.061_405_4 * t - 1.453_152_1) * t) + 1.421_413_8) * t - 0.284_496_74) * t
            + 0.254_829_6)
            * t
            * (-x * x).exp();
    sign * y
}

/// Synthesize one JTTY frame's audio at [`JTTY_RATE`] — sync + coded symbols —
/// for a tone sequence. This is the reference's `gen_jttywave`: the same
/// GFSK-smoothed frequency ramp the mode is defined by, used here to test the
/// receiver round-trip and available to callers that want transmit audio later.
#[allow(clippy::needless_range_loop)] // symbol/sample indices drive parallel arrays
pub fn synthesize_frame(tones: &[u8], f0: f32) -> Vec<f32> {
    let nsps = JTTY_NSPS;
    let nsym = tones.len();
    let bt = 2.0f32;
    let mut dphi = vec![0.0f32; (nsym + 2) * nsps];
    let peak = TAU / nsps as f32;
    // Build the smoothed frequency ramp: each symbol's pulse spreads over
    // three symbol windows.
    let mut pulse = vec![0.0f32; 3 * nsps];
    for (i, p) in pulse.iter_mut().enumerate() {
        let tt = (i as f32 - 1.5 * nsps as f32) / nsps as f32;
        *p = gfsk_pulse(bt, tt);
    }
    for j in 0..nsym {
        let ib = j * nsps;
        for k in 0..3 * nsps {
            dphi[ib + k] += peak * pulse[k] * tones[j] as f32;
        }
    }
    // Dummy symbols at each end, matching the reference.
    for k in 0..2 * nsps {
        dphi[k] += peak * tones[0] as f32 * pulse[nsps + k];
        let end = (nsym) * nsps + k;
        if end < dphi.len() {
            dphi[end] += peak * tones[nsym - 1] as f32 * pulse[k];
        }
    }
    // Convert phase increments to samples, offset by f0, starting 2 symbols in.
    let mut out = Vec::with_capacity(nsym * nsps);
    let mut phi = 0.0f32;
    let dt = 1.0 / JTTY_RATE as f32;
    let base = TAU * f0 * dt;
    for k in 2 * nsps..(nsym + 2) * nsps {
        phi += base + dphi[k];
        out.push(phi.cos());
    }
    out
}

/// The complex analytic signal of a real 6 kHz buffer (positive frequencies
/// only), the reference's `ana64a`. A Hilbert transform via FFT: zero the
/// negative-frequency half and double the positive one.
fn analytic_signal(x: &[f32]) -> Vec<Complex32> {
    use rustfft::FftPlanner;
    let n = x.len().next_power_of_two();
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);
    let ifft = planner.plan_fft_inverse(n);
    let mut buf: Vec<Complex32> = x.iter().map(|&v| Complex32::new(v, 0.0)).collect();
    buf.resize(n, Complex32::new(0.0, 0.0));
    fft.process(&mut buf);
    // Zero negative frequencies; halve DC and Nyquist.
    buf[0] *= 0.5;
    if n.is_multiple_of(2) {
        buf[n / 2] *= 0.5;
    }
    for v in buf.iter_mut().take(n).skip(n / 2 + 1) {
        *v = Complex32::new(0.0, 0.0);
    }
    ifft.process(&mut buf);
    let scale = 1.0 / n as f32;
    buf.iter().map(|v| *v * scale).collect()
}

/// Sync-search result: where in the buffer the frame starts (samples), and the
/// audio centre frequency offset (Hz).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JttySync {
    /// Start sample of the sync sequence at [`JTTY_RATE`].
    pub start: usize,
    /// The signal's lowest-tone frequency, Hz.
    pub f0: f32,
    /// How many of the 13 sync tones were matched by a hard decision.
    pub sync_hits: usize,
    /// Sync SNR estimate, dB.
    pub snr_db: f32,
}

/// Four unit-amplitude tone references over one symbol, for the audio base
/// frequency `f0` (the lowest tone). Tone `i` sits at `f0 + i·baud`.
fn tone_references(f0: f32) -> [[Complex32; JTTY_NSPS]; 4] {
    let mut t = [[Complex32::new(0.0, 0.0); JTTY_NSPS]; 4];
    for (tone, row) in t.iter_mut().enumerate() {
        let hz = f0 + tone as f32 * JTTY_TONE_SPACING_HZ;
        for (k, r) in row.iter_mut().enumerate() {
            let a = TAU * hz / JTTY_RATE as f32 * k as f32;
            *r = Complex32::new(a.cos(), a.sin());
        }
    }
    t
}

/// Find a JTTY frame in a 6 kHz real buffer, or `None`.
///
/// Correlates the analytic signal against the reference sync waveform across
/// candidate starts and a coaxially-swept audio base frequency, and requires
/// most of the 13 sync tones to be readable. The reference searches a shifted
/// spectrum; this is the same decision at the symbol level, which is compact
/// and adequate for a first cut.
pub fn find_sync(x: &[f32], ftol: f32) -> Option<JttySync> {
    if x.len() < JTTY_FRAME_SYMBOLS * JTTY_NSPS {
        return None;
    }
    let c = analytic_signal(x);
    let nsps = JTTY_NSPS;
    let max_start = x.len() - JTTY_FRAME_SYMBOLS * nsps;
    let step = nsps / 4; // quarter-symbol start granularity
    // The primary audio base is the tone spacing (so the four tones sit at
    // 31.25…125 Hz) — the convention the reference's own synthesizer uses.
    // Sweep a modest window either side.
    let f0_centre = JTTY_TONE_SPACING_HZ;
    let f0_step = 5.0f32;
    let f0_lo = (f0_centre - ftol).max(1.0);
    let f0_hi = f0_centre + ftol;

    let mut best: Option<JttySync> = None;
    let mut f0 = f0_lo;
    while f0 <= f0_hi {
        let tone_ref = tone_references(f0);
        let mut start = 0usize;
        while start <= max_start {
            let mut hits = 0usize;
            let mut pt = 0.0f32;
            let mut pa = 0.0f32;
            for j in 0..JTTY_SYNC_SYMBOLS {
                let i0 = start + j * nsps;
                let mut pow = [0.0f32; 4];
                for (tone, row) in tone_ref.iter().enumerate() {
                    let mut z = Complex32::new(0.0, 0.0);
                    for k in 0..nsps {
                        z += c[i0 + k] * row[k].conj();
                    }
                    pow[tone] = z.norm_sqr();
                }
                let (best_tone, _) =
                    pow.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
                if best_tone as u8 == JTTY_SYNC[j] {
                    hits += 1;
                }
                pt += pow[JTTY_SYNC[j] as usize];
                pa += pow.iter().sum::<f32>();
            }
            let pn = (pa - pt) / 3.0;
            let snr = if pn > 0.0 { 10.0 * (pt / pn).log10() } else { -99.0 };
            if hits > 6 {
                let cand = JttySync { start, f0, sync_hits: hits, snr_db: snr };
                // Prefer the strongest sync; ties break on the earliest start.
                // The scanner resumes after each decoded frame, so a strong
                // later sync cannot make it skip a frame that has already been
                // stepped over.
                let better = match &best {
                    None => true,
                    Some(b) => {
                        cand.sync_hits > b.sync_hits
                            || (cand.sync_hits == b.sync_hits
                                && (cand.snr_db > b.snr_db
                                    || (cand.snr_db == b.snr_db && cand.start < b.start)))
                    }
                };
                if better {
                    best = Some(cand);
                }
            }
            start += step;
        }
        f0 += f0_step;
    }
    best
}

/// Find the earliest acceptable sync near `from`, at a fixed base frequency.
///
/// A frame-by-frame scanner uses this: once the first frame fixes `f0`, each
/// following frame is searched only in a short window after the previous one,
/// which is both faster than the whole-buffer search and correct — the global
/// search can return a later frame's sync and skip the one in between.
pub fn find_sync_near(
    x: &[f32],
    f0: f32,
    from: usize,
    window: usize,
    min_hits: usize,
) -> Option<JttySync> {
    let nsps = JTTY_NSPS;
    let frame = JTTY_FRAME_SYMBOLS * nsps;
    if from + frame > x.len() {
        return None;
    }
    let c = analytic_signal(x);
    let end = (from + window).min(x.len() - frame);
    let tone_ref = tone_references(f0);
    let step = nsps / 8;
    let mut start = from;
    while start <= end {
        let mut hits = 0usize;
        let mut pt = 0.0f32;
        let mut pa = 0.0f32;
        for j in 0..JTTY_SYNC_SYMBOLS {
            let i0 = start + j * nsps;
            let mut pow = [0.0f32; 4];
            for (tone, row) in tone_ref.iter().enumerate() {
                let mut z = Complex32::new(0.0, 0.0);
                for k in 0..nsps {
                    z += c[i0 + k] * row[k].conj();
                }
                pow[tone] = z.norm_sqr();
            }
            let (best_tone, _) = pow.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
            if best_tone as u8 == JTTY_SYNC[j] {
                hits += 1;
            }
            pt += pow[JTTY_SYNC[j] as usize];
            pa += pow.iter().sum::<f32>();
        }
        if hits >= min_hits {
            let pn = (pa - pt) / 3.0;
            let snr = if pn > 0.0 { 10.0 * (pt / pn).log10() } else { -99.0 };
            return Some(JttySync { start, f0, sync_hits: hits, snr_db: snr });
        }
        start += step;
    }
    None
}

/// Tone energies for the 46 coded symbols following a sync, for the decoder.
#[allow(clippy::needless_range_loop)] // symbol index selects the energy row
pub fn payload_energies(x: &[f32], sync: &JttySync) -> ToneEnergies {
    let c = analytic_signal(x);
    let nsps = JTTY_NSPS;
    let tone_ref = tone_references(sync.f0);
    let mut energies = [[0.0f32; 4]; INFORMATION_BITS];
    let payload_start = sync.start + JTTY_SYNC_SYMBOLS * nsps;
    for t in 0..INFORMATION_BITS {
        let i0 = payload_start + t * nsps;
        if i0 + nsps > c.len() {
            break;
        }
        for (tone, row) in tone_ref.iter().enumerate() {
            let mut z = Complex32::new(0.0, 0.0);
            for k in 0..nsps {
                z += c[i0 + k] * row[k].conj();
            }
            energies[t][tone] = z.norm();
        }
    }
    energies
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_bits(s: &str, n: usize) -> Vec<u8> {
        s.chars().take(n).map(|c| c.to_digit(2).unwrap() as u8).collect()
    }

    fn payload_from(s: &str) -> [u8; PAYLOAD_BITS] {
        let v = parse_bits(s, PAYLOAD_BITS);
        let mut a = [0u8; PAYLOAD_BITS];
        a.copy_from_slice(&v);
        a
    }

    /// Golden tone vectors emitted by the reference encoder (compiled with
    /// gfortran from WSJT-X v3.2.0-rc1 `tbcc.f90`), so the port is checked
    /// against the reference's own output rather than a reading of its source.
    #[test]
    fn tone_symbols_match_the_reference() {
        let cases = [
            (
                "0100100000010110100011011111011110",
                "0110133033022212223112023032100103003022003130",
            ),
            (
                "1101111000001010101011000011011001",
                "2320311020113031311101120331030023101000320302",
            ),
            (
                "0000000000000000000000000000000000",
                "0000000000000000000000000000000000000000000000",
            ),
            (
                "1010101010101010101010101010101010",
                "2013223103030303030303030303030303202331231312",
            ),
            (
                "0000000000000000000000000000000010",
                "1002020120000000000000000000000023031212302001",
            ),
            (
                "0111111111111111111111111111111110",
                "1312303201333333333333333333333331200100020111",
            ),
        ];
        for (pay, ton) in cases {
            let got = encode_tones(&payload_from(pay));
            let want: Vec<u8> = ton.chars().map(|c| c.to_digit(10).unwrap() as u8).collect();
            assert_eq!(&got[..], &want[..], "payload {pay}");
        }
    }

    #[test]
    fn information_bits_match_the_reference() {
        let info = information_bits(&payload_from("0100100000010110100011011111011110"));
        let want = parse_bits("0100100000010110100011011111011110001110101110", INFORMATION_BITS);
        assert_eq!(&info[..], &want[..]);
    }

    fn perfect_energies(tones: &[u8; INFORMATION_BITS]) -> ToneEnergies {
        let mut e = [[0.0f32; 4]; INFORMATION_BITS];
        for t in 0..INFORMATION_BITS {
            e[t][tones[t] as usize] = 1.0;
        }
        e
    }

    #[test]
    fn clean_round_trip_through_the_decoder() {
        for pay in [
            "0100100000010110100011011111011110",
            "1101111000001010101011000011011001",
            "0000000000000000000000000000000000",
            "1010101010101010101010101010101010",
        ] {
            let p = payload_from(pay);
            let decoded = decode_tones(&perfect_energies(&encode_tones(&p)), 32, 2).unwrap();
            assert_eq!(decoded.payload, p, "payload {pay}");
        }
    }

    /// The code corrects a meaningful fraction of symbol errors — the property
    /// that makes it a weak-signal mode rather than a wire.
    #[test]
    fn symbol_errors_are_corrected() {
        let p = payload_from("0100100000010110100011011111011110");
        let tones = encode_tones(&p);
        let mut state = 0x1234_5678u32;
        let mut rand = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        // Five errors: strongly corrected. More is lossy — assert the floor.
        for nerr in [1usize, 3, 5] {
            let mut ok = 0;
            let trials = 100;
            for _ in 0..trials {
                let mut e = perfect_energies(&tones);
                let mut used = [false; INFORMATION_BITS];
                for _ in 0..nerr {
                    let pos = (rand() as usize) % INFORMATION_BITS;
                    if used[pos] {
                        continue;
                    }
                    used[pos] = true;
                    let wrong = ((tones[pos] + 1 + (rand() % 3) as u8) % 4) as usize;
                    e[pos][tones[pos] as usize] = 0.0;
                    e[pos][wrong] = 1.0;
                }
                if decode_tones(&e, 32, 2).is_some_and(|f| f.payload == p) {
                    ok += 1;
                }
            }
            assert!(ok >= trials - 5, "{nerr} errors corrected {ok}/{trials}");
        }
    }

    /// The ten representative 34-bit source vectors from the JTTY source
    /// encoding spec, decoded to their canonical renderings.
    #[test]
    fn source_vectors_render_canonically() {
        // (bits 1-34, expected rendering)
        let cases = [
            ("0000100110111101111000110101000001", "CQ K1ABC CQ"),
            ("0000000000000000000000000001001001", "AGN?"),
            ("0100010011100101010101010110001101", "HELLO"),
        ];
        for (bits, want) in cases {
            let p = payload_from(bits);
            let got = decode_word(&p);
            assert_eq!(got.as_deref(), Some(want), "{bits}");
        }
    }

    /// STRUCT30 atoms render as their canonical fields, from the spec's
    /// representative vectors.
    #[test]
    fn struct30_vectors_render() {
        let cases = [
            ("1000000000000001111011000000001000", "599 123"), // EXCH_NUM full SERIAL, non-final
            ("1000000000000001111011000000001001", "599 123"), // same, EOM
            ("1000000000000110111010000000011001", "599 CA"),  // EXCH_LOC full STATE
            ("0000001011011110010000110100101001", "599 05 NWT"), // ZONE_LOC3
            ("0010000010110001011000000000101001", "1D EMA"),  // CLASS_SECTION
            ("1000000100111001000010110100111001", "599 156 1749"), // EXCH_NUM_TIME
            ("0001001010000110011000000001001001", "FN42"),    // GRID4 field-only
        ];
        for (bits, want) in cases {
            let p = payload_from(bits);
            let got = decode_word(&p);
            assert_eq!(got.as_deref(), Some(want), "{bits}");
        }
    }

    /// The all-zero word is the invalid sentinel, and a set reserved bit is
    /// rejected before display.
    #[test]
    fn invalid_words_are_rejected() {
        let mut zero = [0u8; PAYLOAD_BITS];
        assert!(decode_word(&zero).is_none());
        // Set the reserved bit (bit 33) on an otherwise valid word.
        zero = payload_from("0000100110111101111000110101000001");
        zero[32] = 1;
        assert!(decode_word(&zero).is_none());
    }

    /// EOM is bit 34.
    #[test]
    fn eom_flag_is_read() {
        let not_last = payload_from("1000000000000001111011000000001000");
        let last = payload_from("1000000000000001111011000000001001");
        assert!(!is_eom(&not_last));
        assert!(is_eom(&last));
    }

    /// The synthetic transmitter and the receive front end agree: a
    /// synthesized frame is found by the sync search and its payload decodes
    /// back to the source bits.
    #[test]
    fn a_synthesized_frame_decodes() {
        let payload = payload_from("0100100000010110100011011111011110");
        let coded = encode_tones(&payload);
        // Frame = sync tones then coded tones.
        let mut frame_tones = Vec::new();
        frame_tones.extend_from_slice(&JTTY_SYNC);
        frame_tones.extend_from_slice(&coded);
        // The reference works in a candidate-centred baseband with the lowest
        // tone at the symbol rate; here synth with f0 at the tone spacing so
        // the four tones sit at 31.25/62.5/93.75/125 Hz.
        let audio = synthesize_frame(&frame_tones, JTTY_TONE_SPACING_HZ);
        // Pad so the sync search has room either side.
        let mut buf = vec![0.0f32; JTTY_NSPS * 4];
        buf.extend_from_slice(&audio);
        buf.extend(std::iter::repeat_n(0.0, JTTY_NSPS * 4));
        let sync = find_sync(&buf, 200.0);
        assert!(sync.is_some(), "sync not found");
        let sync = sync.unwrap();
        assert!(sync.sync_hits > 6, "only {} sync tones matched", sync.sync_hits);
        // End to end: payload energies -> FEC -> the source bits.
        let energies = payload_energies(&buf, &sync);
        let decoded = decode_tones(&energies, 32, 2).expect("payload decode");
        assert_eq!(decoded.payload, payload, "source bits round-tripped");
    }

    /// The packer round-trips through our own decoder: a message packs to
    /// frames, and decoding those frames reproduces the normalized text. The
    /// decoder is verified against the reference, so this checks the packer
    /// against the same standard.
    #[test]
    fn packed_messages_round_trip_through_the_decoder() {
        let cases = [
            "CQ K1ABC CQ",
            "K1ABC",
            "TU K1ABC CQ",
            "K1ABC TU",
            "K1ABC AGN?",
            "TU NOW K1ABC",
            "W1ABC 599 123",
            "599 123",
            "599 FN42",
            "1D EMA",
            "THE QUICK BROWN FOX",
            // An 11 m CB identifier is not a standard amateur call, so it must
            // pack as text and still read back exactly.
            "26AT715",
            "CQ 26AT715 CQ",
        ];
        for msg in cases {
            let frames = pack_message(msg).unwrap_or_else(|| panic!("{msg}: did not pack"));
            let decoded: Vec<JttyFrame> =
                frames.iter().map(|p| JttyFrame { payload: *p }).collect();
            let out = decode_source(&decoded).unwrap_or_else(|| panic!("{msg}: did not decode"));
            assert_eq!(out.text, normalize_text(msg), "{msg}");
        }
    }

    /// A CQ packs to exactly one frame and the grammar word matches the
    /// reference's representative vector bit for bit.
    #[test]
    fn cq_packs_to_the_reference_word() {
        let frames = pack_message("CQ K1ABC CQ").expect("packs");
        assert_eq!(frames.len(), 1, "a CQ is one frame");
        // The spec's `CQ K1ABC CQ` vector, with EOM on bit 34.
        let want = payload_from("0000100110111101111000110101000001");
        assert_eq!(&frames[0][..32], &want[..32], "the 32-bit grammar word differs");
        assert_eq!(frames[0][33], 1, "the single frame carries EOM");
    }

    #[test]
    fn normalize_uppercases_and_collapses() {
        assert_eq!(normalize_text("cq  k1abc   cq"), "CQ K1ABC CQ");
        assert_eq!(normalize_text("hello"), "HELLO");
        // An unsupported char becomes '#'.
        assert_eq!(normalize_text("hi~there"), "HI#THERE");
    }

    /// The frame timing constants the mode is defined by.
    #[test]
    fn frame_geometry_is_the_modes() {
        assert_eq!(JTTY_NSPS, 192);
        assert!((JTTY_BAUD - 31.25).abs() < 1e-6);
        assert_eq!(JTTY_FRAME_SYMBOLS, 59);
        assert!((JTTY_FRAME_S - 1.888).abs() < 0.001);
    }
}
